# The date and time picker, run 1: typed dates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** While a date, a time or a timestamp is edited, in the grid or in the row panel, a line under the editor says what the text will be stored as, or why it will not be. What is typed is read in many forms (ISO 8601, a month's name, the locale's order of numbers, times, `tomorrow 9am`), each type's limits and its zone's changes of the clocks are held to, and a commit stores one literal written the same way every time: the one the line said.

**Architecture:** A module with no egui in it, `src/dates.rs`, knows each engine's kinds of dates and times (matched on `Dialect`), reads a text for a column at a `Clock` (the time it is, the zone, the order of a day's numbers), and gives a `Reading` (the moment, its literal, the change of the clocks it falls in) or a `Refusal`. The editor keeps the reading of its text (`Editor::moment`), made where its problem is made: when it opens and on each keystroke (`Action::EditorTyped`). A commit stores the reading's literal, so what was said is what is stored, whenever the commit comes. A refusal is a `Problem` like a number's, so Enter is refused as it is for numbers. `src/ui/date_line.rs` words the reading and draws it in a popover under the cell or the row panel's field (a float headed by the column in the Omarchy look), with the buttons a change of the clocks offers; a click on one rewrites the field's text, as typing would. A MySQL session is asked its `time_zone` once it is up, on the backend, which reads it from the zone database: a `TIMESTAMP` is read in it.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, jiff (already a dependency: its zone database and civil arithmetic), headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-25-date-time-picker-design.md`: run 1 of the three at its top.

---

## Before Task 1

Commit the spec and this plan:

```bash
git add docs/superpowers/specs/2026-10-25-date-time-picker-design.md docs/superpowers/plans/2026-10-25-date-time-picker-1-typed-dates.md
git commit -m "Plan the date and time picker, run 1: typed dates"
```

## Where this sits

The prompt asked for one pull request; you chose three runs, a pull request each. This is run 1, a branch of `main` at `d18267a7`. Run 2 (the calendar: the macOS popover's month and time fields, the Omarchy float's month, clicks, Now and Today, the footer's NULL, DEFAULT and ∞, the first weekday, the setting) and run 3 (the picker's keys) build on it, each from what the one before merged.

## Decisions and departures, for a yes or a no before code

| The prompt or the boards | Built | Why |
|---|---|---|
| "matched on `Driver` with every variant named" | Matched on `Dialect`, every variant named, no `_` arm | The type mapping the editors use (`column_class`, `Table`) takes the `Dialect`, which is `Driver`'s one to one |
| `now` typed: "stored as the resolved value, never as text" | `now` alone is the time by the clock, stored as it is said; `now()` and `current_timestamp` stay the database's time of the save (`NewValue::Now`), as the now button beside the cell does | Until now `now` was the database's too; the prompt makes it a relative word |
| The boards' grid cell: `2026-01-15 09:30` (no offset) | A `timestamptz` opens on its value in the display zone with its offset, as it does today (`2026-01-15 09:30:00-05`); typed without one, it is the display zone's | The text then says which of two moments a repeated hour is, and it opened so before |
| (an offset other than the zone's typed into a `timestamptz`) | The same moment, sent as the display zone shows it: `09:30+01` is sent as `03:30:00-05` | The prompt's canonical literal; before, it was sent as typed |
| Omarchy float: a line `> 2026-01-15 09:30` under its head | No second copy of the typed text: the cell (or the row panel's field) is the line; the float has the head (`published_at · timestamptz`, the zone) and the parsed line | The cell is the field the keys type in; a copy would only repeat it |
| Omarchy: `tab` takes the offered time, and cycles a repeated hour | In this run the choices are buttons in both looks, clicked with the pointer; `tab` comes with run 3's keys | Run 3 holds every key the picker adds, with the keymap's checks |
| "The weekday and month names use the system language"; "the date order comes from the system locale" | The order of a day's numbers is the locale's (`LC_ALL`, `LC_TIME`, then the system's languages; day first where none says), and reads `15/01/2026`; the line writes a day as the boards do, `Thu 15 Jan 2026`, in English through `gettext` | English is the app's only catalog; the line's form is the boards' |
| PostgreSQL `date` to 5874897 AD, `timestamp` to 294276 AD | Years after 9999 are refused: "Tabletist reads years up to 9999." The lower limits are held: 24 November 4714 BC (the prompt's "4713 BC" is the documentation's round figure; `4714-11-24 BC` is the first day the server takes) | jiff's dates end at 9999 |
| A repeated hour asks which one | On PostgreSQL. A MySQL `TIMESTAMP` is sent with no offset, so nothing would tell the server which: no choice is offered there, and the server takes the one it takes. A skipped time is refused on both | A choice that changes nothing sent would mislead |
| "using the connection's session `time_zone`" | Asked once the session is up (`@@session.time_zone`, or `@@global.system_time_zone` where it is `SYSTEM`) and read on the backend: `UTC`, an offset, or a zone's name. An abbreviation (`CET`, `EST`) names no rules of the clocks: the value is then read as the server's clock writes it, no change of the clocks is found, and the limits are taken as UTC | An abbreviation is not the zone of the database by that name |
| MySQL `TIME`: "time fields only" | A time of day as for the other engines; a span past a day or below zero (`100:30:00`, `-12:00`) is kept as written, to ±838:59:59, the line saying "a span of time, not a time of day" | MySQL keeps spans in `TIME`; refusing them would take away what can be typed today |
| SQLite: "If existing values are integers, write seconds since 1970. If they are text, write ISO 8601" | As said, and REAL values are Julian days (`julianday()`'s), written as one; a `TIME` kept as a number is seconds of the day | A REAL in a date column is a Julian day; reading it as seconds would store another day |
| (PostgreSQL's and SQLite's `24:00:00`) | Read, said as "the end of the day"; a time rounded up past `23:59:59` is it | PostgreSQL rounds `23:59:59.7` into `time(0)` to it |
| (a MySQL `TIMESTAMP` before its session's zone is known, or with one named by an abbreviation) | A day counted from today, `now`, a time without its day, or an offset is refused: "The server's zone is not known: type the day and the time as the server keeps them, with no offset." | Today there is the server's, and the text goes to it as its own clock |
| (a `timestamptz` while the settings' named zone is not read yet, or one the zone database does not know) | Typed in UTC and sent with `+00`, as the cells show the values as sent | It is sent with an offset all the same, which the server reads as it is |
| (a time typed into a date, a day into a time, an offset into a `timestamp`) | Refused, with why: "A date keeps no time of day.", "A time keeps no day.", "timestamp keeps no zone: leave the offset out." SQLite keeps an offset in its text | PostgreSQL drops a time or an offset without a word, MySQL moves one |
| "NULL, DEFAULT and empty follow the existing rules" | NULL and DEFAULT do (typed where the column takes them). An empty text in a date column is refused as no date, and the line says what to type | It was sent as `''`, which no server takes as a date |
| (what the servers read that is not listed: a zone's name after a time, `epoch`, `J2451187`, `20260115`) | Refused as no date or time: write the moment | The line says what is stored; a word only the server reads it could not say |
| (a value the server holds that the reader does not read: a SQLite text of another form, say) | Opens as it is, red, with the line saying it is no date; Enter and Esc as for any refused text, and unchanged it closes with no change | Only a typed change is checked against the column |
| (paste, a duplicated row, the filter bar) | Not read: what they put in a date cell is sent as it is, as before | Only the editor has a line to say what a text is |
| "sent as `…`" in a monospaced face | Said where the literal is not what was typed, in the monospaced secondary role: `09:30:00 · sent as 09:30:00` for `9:30`, as the board's time state has it. A fraction is said to the digits the column keeps (`09:30:12.250000` for `timestamp(6)`, the board's), and sent as typed (`.250`) | The line otherwise repeats the text |
| A gap "shows as a warning" | The line in the warning's ink, and the field's ring not red (a MySQL zero date likewise); Enter is refused | As asked |
| "Fri 30 Oct 2026 · in 6 days" (a day typed as ISO on the board) | How far the day is from today is said within a week, and always for a relative word | The board says it for an ISO day six days away |
| Open question 3, the display zone | The setting exists: Settings › General › Time zone for timestamptz (`data.tz`, system by default); it is used | |
| Open question 4, the repeated hour | Summer time first | As proposed |

Open questions 1 (the first weekday), 2 (the setting) and 5 (the keys) are for runs 2 and 3. For 2: `[editor.typing]` is the SQL editor's table today (`src/settings/editor.rs`), and Omarchy has no `:set` yet. For 5: `ctrl+h` cannot be a key of the cell editor on Omarchy, since vim's insert mode deletes with it (`omarchy_binds_no_super_and_nothing_over_vims_insert_keys`); `alt+h`, `alt+l`, `alt+j` and `alt+k` are free (`alt+1` to `alt+9` switch tabs).

A reviewer given the draft broke it with tests of its own: nine defects and four smaller ones, three of them checked against the PostgreSQL server. The draft fixes them all, and each has a test in it:

- A relative count past what a span holds (`+20000y`, `+99999999d`) panicked jiff on the UI thread while it was typed. The span is built with jiff's `try_` methods now, and such a count is refused (`a_count_past_every_day_is_refused_not_a_crash`).
- A date column that is also a foreign key said nothing when its text was refused: the line is not drawn under the key's list, and the note under the field had given way to the line. The note stays while the list is up (`a_date_that_is_a_foreign_key_says_why_its_text_is_refused`).
- A `timetz` offset past ±15:59:59 (`10:00+16`) was taken, and the save failed: PostgreSQL refuses it. It is refused with the limit.
- An offset's minutes were not held to 59: `+05:99` read as +06:39. Such an offset is no offset now.
- A BC `timestamptz`, and a fraction written with more zeros than the server wrote (`.250` for `.25`, `.000`), typed back as they opened, were a change. Whether a typed date is a change is now asked of the two moments, not of their texts (`a_date_typed_back_as_another_writing_of_it_is_no_change`).
- A SQLite `TIME` kept as seconds opened on a day and a time, which its own reader refused; it opens on its time now and is written as seconds of the day.
- SQLite values kept as REAL were taken for seconds since 1970; they are Julian days, as `julianday()` writes them, and are read and written as such (`sqlite_times_and_julian_days_follow_their_columns`).
- A time rounded up past `23:59:59` wrapped to `00:00:00`. jiff's own rounding of a civil time wraps at midnight, which also moved a `DATETIME(0)` at `23:59:59.7` back to the start of its day; the rounding is the module's own now, carried into the next second and day. `24:00:00`, which PostgreSQL and SQLite times hold, is read, and a MySQL span's fraction is rounded as a time's is (`a_time_rounded_past_its_day_is_the_days_end`).
- **Use 03:30** and the two choices of a repeated hour dropped the time's fraction (`a_choice_keeps_the_fraction_of_the_time_it_moves`).
- Smaller: a MySQL `TIMESTAMP` whose session zone is not known took `now`, `tomorrow` or a time alone in the app's zone and sent it as the server's clock; such a text is refused there now, with why. An era was taken after a day with no written year (`today bc`, `15 jan bc`) and after a time alone; only after a year written now. The terminal look's choices of a repeated hour were not in its case.

What held, among what it tried: every name of the spec's types (odd spacing, case, sizes, arrays, `interval`); the accepted forms and the refused ones (`2026-01-15 9`, `1/2/3`, `15 jan 2026 25:00`, trailing words, full-width digits, a text of a hundred thousand digits, year 10000); the New York gap and fold; MySQL `TIMESTAMP`'s limits in the session's zone; BC leap years and PostgreSQL's first day; infinity; a kept text reopened; a new row's cell; the row panel; and the session's zone read from a live MySQL server.

A second reviewer read this plan against its diffs and the spec, and found more, which the draft and the plan now take in:

- A `timestamptz` whose named zone was not read yet (or is unknown to the database) fell into the path of a MySQL session with no known zone: its value opened red, and what was typed was sent with no offset. It is typed in UTC now and sent with `+00` (`a_timestamptz_whose_zone_is_not_read_yet_is_sent_in_utc`).
- A time the clocks skip turned the field red, though the spec has it as a warning: `Problem::warns` keeps the ring as it is (`a_time_the_clocks_skip_is_a_warning_and_no_day_is_a_fault`).
- An offset typed for a MySQL `TIMESTAMP` with no known zone said "keeps no zone", which the type does not: it is that zone's refusal now.
- The line's live region was made again at each change, which a screen reader need not read out; it is one node now.
- Canada writes its days year first (CLDR's `y-MM-dd`), not month first; `PgOffset`'s words give the seconds; a SQLite time kept as a Julian day is one of 1 January 2000, as `julianday('09:30')` is.
- `fn date_order` had come between `system_zone`'s comment and its function.
- The website page said less than the behaviour (Julian days, times kept as numbers, the refusals, a time alone, spans, `24:00:00`, today being the settings' zone's), and the table above said too little of what is now refused or opens red.

## Files

| File | Task | What |
|---|---|---|
| `src/dates.rs` (new) | 1 | Kinds, reading, limits, zones, literals |
| `src/lib.rs` | 1 | The module |
| `src/edit.rs` | 2, 3 | `Problem::Moment`, `Editor::moment`, `Table::dates`, `edit::moment`; `keeps_offset` goes; the session's zone |
| `src/app.rs` | 2, 3 | `date_clock`, `stopped_clock`, `date_order`; the reading on each keystroke; the session's zone asked and kept |
| `src/app/editing.rs` | 2 | A commit stores the reading's literal; the editor opens with its reading, and a SQLite seconds value as its day |
| `src/ui/format.rs` | 2 | `with_offset` goes (`dates` writes the offset now) |
| `src/ui/date_line.rs` (new) | 2 | The line's words and its popover |
| `src/ui/cell_editor.rs` | 2 | `Target::dates`; a refusal's words; the note under the field gives way to the line |
| `src/ui/data_view.rs`, `src/ui/row_form.rs` | 2 | The line under the cell and the field |
| `src/ui/mod.rs` | 2 | The modules |
| `src/ui/date_tests.rs` (new) | 2, 3 | Behaviour, headless |
| `src/ui/zone_tests.rs` | 2 | An offset typed is the same moment in the zone |
| `crates/tabletist-db/src/{adapter,lib,mysql,pg,sqlite}.rs` | 3 | `Connection::session_zone` |
| `crates/tabletist-db/tests/{mysql,postgres,sqlite}.rs` | 3 | Against the servers |
| `src/backend.rs` | 3 | `Command::SessionZone`, `Event::SessionZone` |
| `src/zones.rs` | 3 | A server's zone read from the database |
| `src/model.rs` | 3 | `Workspace::session_zone` |
| `src/key_picker.rs`, `src/review.rs` | 3 | Tests' `Table`s |
| `docs/_guide/editing-data.md` | 3 | Dates and times |

The tests run as before: `cargo test --locked --workspace --all-targets`, with `TABLETIST_TEST_PG_URL` and `TABLETIST_TEST_MYSQL_URL` for the servers (AGENTS.md). Use `~/.cargo/bin/cargo` where the mise shim fails.

---

### Task 1: Read dates and times

**Files:**
- Create: `src/dates.rs`
- Modify: `src/lib.rs`

`dates` has no egui and reads no clock. `kind` names each engine's types: PostgreSQL's by their names, MySQL's by the first word (`TIMESTAMP` is zoned: the server moves it to and from the session's zone), SQLite's by what the declared name holds (`DATE` without `TIME` a date, `TIMESTAMP` or both a timestamp, `TIME` a time). A `Column` adds the digits of a second the type keeps, how SQLite's values are kept (`Stored`: text with its separator, or seconds), and the zone of a zoned value. `read` takes a text apart (an era; a day as a word, ISO, numbers in the locale's `Order`, or a month's name; a time; an offset), checks it (the month, the day, the time of day, the type's range), resolves a zoned one in its zone (a gap is a `Refusal::Gap` with the time after it offered; a fold takes summer time and says both) and writes the literal (SQLite's in the form its column keeps: text, seconds, or a Julian day). `rewritten` writes the text a choice puts in the field; `opened` the day a SQLite seconds value opens as.

- [ ] **Step 1: Write the failing tests**

The tests are the module's `tests`, at its end. Create `src/dates.rs` with them alone, and declare the module:

`src/dates.rs`:

```diff
diff --git a/src/dates.rs b/src/dates.rs
new file mode 100644
--- /dev/null
+++ b/src/dates.rs
@@ -0,0 +1,699 @@
+use jiff::civil::Date;
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use jiff::civil::date;
+
+    fn new_york() -> TimeZone {
+        TimeZone::get("America/New_York").expect("the zone")
+    }
+
+    /// Saturday 24 October 2026, 10:00 in New York, as the boards have it.
+    fn clock() -> Clock {
+        let now = date(2026, 10, 24)
+            .at(10, 0, 0, 0)
+            .to_zoned(new_york())
+            .unwrap()
+            .timestamp();
+        Clock {
+            now,
+            zone: new_york(),
+            order: Order::DayMonth,
+        }
+    }
+
+    fn column(dialect: Dialect, type_name: &str) -> Column {
+        Column::of(dialect, type_name, Some(&new_york()), []).expect("a column of dates")
+    }
+
+    fn literal(dialect: Dialect, type_name: &str, typed: &str) -> String {
+        match read(typed, &column(dialect, type_name), &clock()) {
+            Ok(reading) => reading.literal,
+            Err(refusal) => panic!("{typed:?} refused: {refusal:?}"),
+        }
+    }
+
+    fn refusal(dialect: Dialect, type_name: &str, typed: &str) -> Refusal {
+        read(typed, &column(dialect, type_name), &clock()).expect_err(typed)
+    }
+
+    #[test]
+    fn each_engine_names_its_kinds() {
+        use Dialect::{MySql, Postgres, Sqlite};
+        let zoned = Some(Kind::Zoned);
+        let stamp = Some(Kind::Timestamp);
+        let day = Some(Kind::Date);
+        let time = Some(Kind::Time { offset: false });
+        for (dialect, name, expected) in [
+            (Postgres, "date", day),
+            (Postgres, "timestamp without time zone", stamp),
+            (Postgres, "timestamp(3) without time zone", stamp),
+            (Postgres, "timestamp", stamp),
+            (Postgres, "timestamp with time zone", zoned),
+            (Postgres, "timestamptz", zoned),
+            (Postgres, "time without time zone", time),
+            (
+                Postgres,
+                "time(3) with time zone",
+                Some(Kind::Time { offset: true }),
+            ),
+            (Postgres, "timetz", Some(Kind::Time { offset: true })),
+            (Postgres, "interval", None),
+            (Postgres, "date[]", None),
+            (Postgres, "daterange", None),
+            (Postgres, "text", None),
+            (MySql, "date", day),
+            (MySql, "datetime", stamp),
+            (MySql, "datetime(6)", stamp),
+            (MySql, "timestamp", zoned),
+            (MySql, "timestamp(3)", zoned),
+            (MySql, "time", time),
+            (MySql, "year", None),
+            (MySql, "varchar(20)", None),
+            (Sqlite, "DATE", day),
+            (Sqlite, "DATETIME", stamp),
+            (Sqlite, "TIMESTAMP", stamp),
+            (Sqlite, "timestamptz", stamp),
+            (Sqlite, "DATE_TIME", stamp),
+            (Sqlite, "TIME", time),
+            (Sqlite, "TEXT", None),
+            (Sqlite, "", None),
+        ] {
+            assert_eq!(kind(dialect, name), expected, "{dialect:?} {name}");
+        }
+    }
+
+    #[test]
+    fn each_written_form_reads_as_its_day() {
+        let pg = Dialect::Postgres;
+        for typed in [
+            "2026-01-15",
+            "2026-1-15",
+            "15 Jan 2026",
+            "15 january 2026",
+            "Jan 15 2026",
+            "January 15, 2026",
+            "jan 15th 2026",
+            "15/01/2026",
+            "15.01.2026",
+            "2026/01/15",
+            " 2026-01-15 ",
+        ] {
+            assert_eq!(literal(pg, "date", typed), "2026-01-15", "{typed}");
+        }
+        // A month's name with no year: this year's.
+        assert_eq!(literal(pg, "date", "15 jan"), "2026-01-15");
+    }
+
+    #[test]
+    fn the_locale_orders_a_day_written_in_numbers() {
+        let column = column(Dialect::Postgres, "date");
+        let in_order = |order: Order| Clock { order, ..clock() };
+        let read_in = |order| {
+            read("02/03/2026", &column, &in_order(order))
+                .unwrap()
+                .literal
+        };
+        assert_eq!(read_in(Order::DayMonth), "2026-03-02");
+        assert_eq!(read_in(Order::MonthDay), "2026-02-03");
+        assert_eq!(Order::of_region(Some("us")), Order::MonthDay);
+        assert_eq!(Order::of_region(Some("ca")), Order::YearMonth);
+        assert_eq!(Order::of_region(Some("gb")), Order::DayMonth);
+        assert_eq!(Order::of_region(Some("jp")), Order::YearMonth);
+        assert_eq!(Order::of_region(None), Order::DayMonth);
+    }
+
+    #[test]
+    fn each_written_form_reads_as_its_time() {
+        let pg = Dialect::Postgres;
+        for (typed, expected) in [
+            ("9:30", "09:30:00"),
+            ("9.30", "09:30:00"),
+            ("9am", "09:00:00"),
+            ("9 am", "09:00:00"),
+            ("9:30pm", "21:30:00"),
+            ("12am", "00:00:00"),
+            ("12pm", "12:00:00"),
+            ("21:30", "21:30:00"),
+            ("21:30:15", "21:30:15"),
+            ("21:30:15.25", "21:30:15.25"),
+        ] {
+            assert_eq!(literal(pg, "time", typed), expected, "{typed}");
+        }
+    }
+
+    #[test]
+    fn a_day_and_a_time_read_together_in_every_form() {
+        let pg = Dialect::Postgres;
+        let stamp = "timestamp without time zone";
+        for typed in [
+            "2026-01-15 09:30",
+            "2026-01-15T09:30",
+            "2026-01-15t09:30:00",
+            "15 Jan 2026 9:30",
+            "Jan 15 2026, 9:30am",
+            "15/01/2026 9.30",
+        ] {
+            assert_eq!(literal(pg, stamp, typed), "2026-01-15 09:30:00", "{typed}");
+        }
+        assert_eq!(
+            literal(pg, stamp, "2026-01-15 09:30:12.250"),
+            "2026-01-15 09:30:12.250"
+        );
+        // A day alone is its midnight; a time alone today's.
+        assert_eq!(literal(pg, stamp, "2026-01-15"), "2026-01-15 00:00:00");
+        assert_eq!(literal(pg, stamp, "9:30"), "2026-10-24 09:30:00");
+    }
+
+    #[test]
+    fn words_count_from_today_by_the_clock() {
+        let pg = Dialect::Postgres;
+        // Saturday 24 October 2026.
+        for (typed, expected) in [
+            ("today", "2026-10-24"),
+            ("Tomorrow", "2026-10-25"),
+            ("yesterday", "2026-10-23"),
+            ("now", "2026-10-24"),
+            ("+7d", "2026-10-31"),
+            ("-2w", "2026-10-10"),
+            ("+1m", "2026-11-24"),
+            ("+1y", "2027-10-24"),
+            ("next fri", "2026-10-30"),
+            ("fri", "2026-10-30"),
+            ("next saturday", "2026-10-31"),
+            ("last mon", "2026-10-19"),
+        ] {
+            assert_eq!(literal(pg, "date", typed), expected, "{typed}");
+        }
+        let stamp = "timestamp without time zone";
+        assert_eq!(literal(pg, stamp, "tomorrow 9am"), "2026-10-25 09:00:00");
+        assert_eq!(
+            literal(pg, stamp, "tomorrow at 9:30"),
+            "2026-10-25 09:30:00"
+        );
+        assert_eq!(literal(pg, stamp, "now"), "2026-10-24 10:00:00");
+        assert_eq!(literal(pg, "time", "now"), "10:00:00");
+        let reading = read("+7d", &column(pg, "date"), &clock()).unwrap();
+        assert!(reading.relative);
+        assert_eq!(reading.days, Some(7));
+        assert_eq!(reading.typed, "+7d");
+        let reading = read("yesterday", &column(pg, "date"), &clock()).unwrap();
+        assert_eq!(reading.days, Some(-1));
+        let reading = read("2026-10-30", &column(pg, "date"), &clock()).unwrap();
+        assert!(!reading.relative);
+    }
+
+    #[test]
+    fn a_month_moved_past_its_end_is_held_to_its_last_day() {
+        let clock = Clock {
+            now: date(2026, 1, 31)
+                .at(12, 0, 0, 0)
+                .to_zoned(new_york())
+                .unwrap()
+                .timestamp(),
+            ..clock()
+        };
+        let column = column(Dialect::Postgres, "date");
+        assert_eq!(read("+1m", &column, &clock).unwrap().literal, "2026-02-28");
+    }
+
+    #[test]
+    fn a_day_or_a_time_that_is_none_is_refused_with_why() {
+        let pg = Dialect::Postgres;
+        assert_eq!(
+            refusal(pg, "date", "2026-02-30"),
+            Refusal::NoDay { day: 30, month: 2 }
+        );
+        assert_eq!(refusal(pg, "date", "2026-13-01"), Refusal::NoMonth(13));
+        assert_eq!(
+            refusal(pg, "time", "25:00"),
+            Refusal::NoTime("25:00".into())
+        );
+        assert_eq!(refusal(pg, "time", "9:61"), Refusal::NoTime("9:61".into()));
+        assert_eq!(refusal(pg, "time", "13pm"), Refusal::NoTime("13pm".into()));
+        assert_eq!(refusal(pg, "date", "soon"), UNREAD);
+        assert_eq!(refusal(pg, "date", ""), UNREAD);
+        assert_eq!(refusal(pg, "date", "2026-01-15 9:30"), Refusal::DayOnly);
+        assert_eq!(refusal(pg, "time", "2026-01-15 9:30"), Refusal::TimeOnly);
+        assert_eq!(refusal(pg, "date", "20260-01-15"), Refusal::Years);
+        // 2028 has a 29 February; 2026 has none.
+        assert_eq!(literal(pg, "date", "2028-02-29"), "2028-02-29");
+    }
+
+    #[test]
+    fn a_moment_is_sent_in_the_zone_with_its_offset() {
+        let pg = Dialect::Postgres;
+        let tz = "timestamp with time zone";
+        assert_eq!(
+            literal(pg, tz, "2026-01-15 09:30"),
+            "2026-01-15 09:30:00-05"
+        );
+        assert_eq!(
+            literal(pg, tz, "2026-07-15 09:30"),
+            "2026-07-15 09:30:00-04"
+        );
+        // Typed with another offset: the same moment, in the zone.
+        assert_eq!(
+            literal(pg, tz, "2026-01-15 09:30+01"),
+            "2026-01-15 03:30:00-05"
+        );
+        assert_eq!(
+            literal(pg, tz, "2026-01-15T14:30:00Z"),
+            "2026-01-15 09:30:00-05"
+        );
+        // As the server writes one, with its fraction: what the column
+        // keeps.
+        assert_eq!(
+            literal(pg, tz, "2026-01-12 09:14:03.482915+00"),
+            "2026-01-12 04:14:03.482915-05"
+        );
+        let reading = read("2026-01-15 09:30", &column(pg, tz), &clock()).unwrap();
+        assert_eq!(reading.zone_name.as_deref(), Some("EST"));
+        assert_eq!(reading.fold, None);
+    }
+
+    #[test]
+    fn a_time_the_clocks_skip_is_refused_and_the_time_after_offered() {
+        let column = column(Dialect::Postgres, "timestamptz");
+        let refused = read("2026-03-08 02:30", &column, &clock()).unwrap_err();
+        assert_eq!(
+            refused,
+            Refusal::Gap {
+                at: date(2026, 3, 8).at(2, 30, 0, 0),
+                place: "New York".into(),
+                offered: date(2026, 3, 8).at(3, 30, 0, 0),
+            }
+        );
+        let offered = rewritten(date(2026, 3, 8).at(3, 30, 0, 0), None, &column);
+        assert_eq!(offered, "2026-03-08 03:30");
+        assert_eq!(
+            read(&offered, &column, &clock()).unwrap().literal,
+            "2026-03-08 03:30:00-04"
+        );
+    }
+
+    #[test]
+    fn a_time_the_clocks_go_back_over_takes_summer_time_first_or_the_one_chosen() {
+        let column = column(Dialect::Postgres, "timestamptz");
+        let first = read("2026-11-01 01:30", &column, &clock()).unwrap();
+        assert_eq!(first.literal, "2026-11-01 01:30:00-04");
+        let fold = first.fold.expect("a fold");
+        assert_eq!(fold.earlier.1, "EDT");
+        assert_eq!(fold.later.1, "EST");
+        assert_eq!(fold.place, "New York");
+        let chosen = rewritten(fold.at, Some(fold.later.0), &column);
+        assert_eq!(chosen, "2026-11-01 01:30-05");
+        let second = read(&chosen, &column, &clock()).unwrap();
+        assert_eq!(second.literal, "2026-11-01 01:30:00-05");
+        assert!(second.fold.is_some(), "the choice is still offered");
+    }
+
+    #[test]
+    fn a_timestamp_without_a_zone_is_stored_as_typed_in_any_zone() {
+        for zone in [
+            new_york(),
+            TimeZone::UTC,
+            TimeZone::get("Asia/Kolkata").unwrap(),
+        ] {
+            let clock = Clock {
+                zone: zone.clone(),
+                ..clock()
+            };
+            let column = Column::of(Dialect::Postgres, "timestamp", Some(&zone), []).unwrap();
+            let reading = read("2026-03-08 02:30", &column, &clock).unwrap();
+            assert_eq!(reading.literal, "2026-03-08 02:30:00");
+            assert_eq!(reading.fold, None);
+        }
+        assert_eq!(
+            refusal(Dialect::Postgres, "timestamp", "2026-03-08 02:30-05"),
+            Refusal::NoZone
+        );
+        assert_eq!(
+            literal(Dialect::MySql, "datetime", "2026-03-08 02:30"),
+            "2026-03-08 02:30:00"
+        );
+    }
+
+    #[test]
+    fn each_type_holds_its_own_range() {
+        let my = Dialect::MySql;
+        assert_eq!(
+            refusal(my, "timestamp", "2040-05-01 08:00"),
+            Refusal::Range(Range::MySqlTimestamp)
+        );
+        assert_eq!(
+            refusal(my, "timestamp", "1969-12-31 18:00"),
+            Refusal::Range(Range::MySqlTimestamp)
+        );
+        // 2038-01-19 03:14:07 UTC is 22:14:07 the day before in New York.
+        assert_eq!(
+            literal(my, "timestamp", "2038-01-18 22:14:07"),
+            "2038-01-18 22:14:07"
+        );
+        assert_eq!(
+            refusal(my, "timestamp", "2038-01-18 22:14:08"),
+            Refusal::Range(Range::MySqlTimestamp)
+        );
+        assert_eq!(
+            refusal(my, "date", "0999-12-31"),
+            Refusal::Range(Range::MySqlDate)
+        );
+        assert_eq!(literal(my, "date", "1000-01-01"), "1000-01-01");
+        assert_eq!(literal(my, "date", "9999-12-31"), "9999-12-31");
+        assert_eq!(
+            refusal(my, "datetime", "0999-12-31 23:59"),
+            Refusal::Range(Range::MySqlDatetime)
+        );
+        assert_eq!(refusal(my, "date", "0000-00-00"), Refusal::Zero);
+        assert_eq!(
+            refusal(my, "datetime", "0000-00-00 00:00:00"),
+            Refusal::Zero
+        );
+        let pg = Dialect::Postgres;
+        assert_eq!(literal(pg, "date", "4714-11-24 BC"), "4714-11-24 BC");
+        assert_eq!(
+            refusal(pg, "date", "4714-11-23 BC"),
+            Refusal::Range(Range::PgDate)
+        );
+        assert_eq!(refusal(pg, "date", "0000-01-01"), Refusal::YearZero);
+        assert_eq!(literal(Dialect::Sqlite, "DATE", "0000-01-01"), "0000-01-01");
+    }
+
+    #[test]
+    fn a_mysql_timestamp_is_sent_as_the_sessions_clock_writes_it() {
+        let my = Dialect::MySql;
+        let berlin = TimeZone::get("Europe/Berlin").unwrap();
+        let column = Column::of(my, "timestamp", Some(&berlin), []).unwrap();
+        // Berlin's clocks go back that night: the literal has no offset to
+        // say which 02:30, and none is asked.
+        let reading = read("2026-10-25 02:30", &column, &clock()).unwrap();
+        assert_eq!(reading.literal, "2026-10-25 02:30:00");
+        assert_eq!(reading.fold, None);
+        // They go forward in March, over 02:30.
+        let refused = read("2026-03-29 02:30", &column, &clock()).unwrap_err();
+        assert!(matches!(refused, Refusal::Gap { .. }), "{refused:?}");
+        // The limits are UTC's: 03:14:07 UTC is 04:14:07 in Berlin.
+        assert!(read("2038-01-19 04:14:07", &column, &clock()).is_ok());
+        assert_eq!(
+            read("2038-01-19 04:14:08", &column, &clock()).unwrap_err(),
+            Refusal::Range(Range::MySqlTimestamp)
+        );
+        // A zone the app cannot name: the server's clock, no change found.
+        let unnamed = Column::of(my, "timestamp", None, []).unwrap();
+        let reading = read("2026-03-29 02:30", &unnamed, &clock()).unwrap();
+        assert_eq!(reading.literal, "2026-03-29 02:30:00");
+    }
+
+    #[test]
+    fn infinity_and_days_before_christ_go_back_as_they_came() {
+        let pg = Dialect::Postgres;
+        for typed in ["infinity", "-infinity"] {
+            for type_name in ["date", "timestamp", "timestamptz"] {
+                assert_eq!(literal(pg, type_name, typed), typed);
+            }
+        }
+        assert_eq!(literal(pg, "date", "+Infinity"), "infinity");
+        assert_eq!(
+            refusal(Dialect::MySql, "date", "infinity"),
+            Refusal::NotRead(Kind::Date)
+        );
+        assert_eq!(literal(pg, "date", "0044-03-15 BC"), "0044-03-15 BC");
+        assert_eq!(
+            literal(pg, "timestamp", "0044-03-15 12:00:00 BC"),
+            "0044-03-15 12:00:00 BC"
+        );
+        // As the server writes a moment before Christ: its offset, then
+        // the era. New York kept its own clock then.
+        assert_eq!(
+            literal(pg, "timestamptz", "0044-03-15 12:00:00+00 BC"),
+            "0044-03-15 07:03:58-04:56:02 BC"
+        );
+        let reading = read("0044-03-15 BC", &column(pg, "date"), &clock()).unwrap();
+        assert_eq!(
+            reading.moment,
+            Moment::At {
+                date: Some(date(-43, 3, 15)),
+                time: None,
+                offset: None
+            }
+        );
+        assert_eq!(
+            refusal(Dialect::Sqlite, "DATE", "0044-03-15 BC"),
+            Refusal::NotRead(Kind::Date)
+        );
+    }
+
+    #[test]
+    fn sqlite_writes_the_form_its_column_already_keeps() {
+        let lite = Dialect::Sqlite;
+        let of = |type_name: &str, values: &[Value]| {
+            Column::of(lite, type_name, None, values.iter()).unwrap()
+        };
+        let text = [Value::Null, Value::Text("2026-10-01".into())];
+        let seconds = [Value::Int(1_790_000_000)];
+        let stamps = [Value::Text("2026-10-01T08:00:00".into())];
+        assert_eq!(Stored::of(&[]), Stored::Fresh);
+        assert_eq!(Stored::of(&text), Stored::Text { separator: ' ' });
+        assert_eq!(Stored::of(&seconds), Stored::Seconds);
+        assert_eq!(Stored::of(&stamps), Stored::Text { separator: 'T' });
+        let read_as = |column: &Column, typed: &str| read(typed, column, &clock()).unwrap().literal;
+        assert_eq!(read_as(&of("DATE", &text), "24 Oct 2026"), "2026-10-24");
+        assert_eq!(read_as(&of("DATE", &[]), "24 Oct 2026"), "2026-10-24");
+        assert_eq!(read_as(&of("DATE", &seconds), "24 Oct 2026"), "1792800000");
+        assert_eq!(
+            read_as(&of("DATETIME", &seconds), "2026-10-24 08:00:00.5"),
+            "1792828801"
+        );
+        assert_eq!(
+            read_as(&of("DATETIME", &stamps), "2026-10-24 08:00"),
+            "2026-10-24T08:00:00"
+        );
+        assert_eq!(
+            read_as(&of("DATETIME", &[]), "2026-10-24 08:00"),
+            "2026-10-24 08:00:00"
+        );
+        // A SQLite column keeps what it is given: an offset too.
+        assert_eq!(
+            read_as(&of("DATETIME", &[]), "2026-10-24 08:00+02"),
+            "2026-10-24 08:00:00+02"
+        );
+        // Seconds read back, and open as their day.
+        let column = of("DATE", &seconds);
+        assert_eq!(read_as(&column, "1792800000"), "1792800000");
+        assert_eq!(
+            opened(&Value::Int(1_792_800_000), &column).as_deref(),
+            Some("2026-10-24")
+        );
+        assert_eq!(opened(&Value::Text("2026-10-24".into()), &column), None);
+    }
+
+    #[test]
+    fn a_fraction_is_kept_to_the_digits_the_column_keeps() {
+        let my = Dialect::MySql;
+        assert_eq!(
+            literal(my, "datetime", "2026-01-15 09:30:12.6"),
+            "2026-01-15 09:30:13"
+        );
+        assert_eq!(
+            literal(my, "datetime(3)", "2026-01-15 09:30:12.123456"),
+            "2026-01-15 09:30:12.123"
+        );
+        assert_eq!(
+            literal(Dialect::Postgres, "time(0)", "09:30:59.7"),
+            "09:31:00"
+        );
+        // Rounded up into the next day, and at the last there is not.
+        assert_eq!(
+            literal(my, "datetime", "2026-01-15 23:59:59.7"),
+            "2026-01-16 00:00:00"
+        );
+        assert_eq!(
+            literal(my, "datetime", "9999-12-31 23:59:59.7"),
+            "9999-12-31 23:59:59"
+        );
+    }
+
+    #[test]
+    fn a_mysql_time_past_a_day_or_below_zero_is_a_span_of_time() {
+        let my = Dialect::MySql;
+        assert_eq!(literal(my, "time", "100:30:00"), "100:30:00");
+        assert_eq!(literal(my, "time", "-12:00"), "-12:00:00");
+        assert_eq!(literal(my, "time", "838:59:59"), "838:59:59");
+        assert_eq!(
+            refusal(my, "time", "839:00:00"),
+            Refusal::Range(Range::MySqlTime)
+        );
+        assert_eq!(
+            refusal(my, "time", "100:61:00"),
+            Refusal::NoTime("100:61:00".into())
+        );
+        // A time of day is read as one; PostgreSQL's time keeps no span.
+        assert_eq!(literal(my, "time", "9:30"), "09:30:00");
+        assert_eq!(
+            refusal(Dialect::Postgres, "time", "100:30:00"),
+            Refusal::NotRead(Kind::Time { offset: false })
+        );
+    }
+
+    #[test]
+    fn a_time_with_an_offset_keeps_it_where_the_type_does() {
+        let pg = Dialect::Postgres;
+        assert_eq!(literal(pg, "timetz", "09:30-05"), "09:30:00-05");
+        assert_eq!(literal(pg, "timetz", "09:30"), "09:30:00");
+        assert_eq!(refusal(pg, "time", "09:30-05"), Refusal::NoZone);
+    }
+
+    #[test]
+    fn a_count_past_every_day_is_refused_not_a_crash() {
+        let pg = Dialect::Postgres;
+        assert_eq!(refusal(pg, "date", "+20000y"), Refusal::Years);
+        assert_eq!(refusal(pg, "date", "+99999999d"), Refusal::Years);
+        assert_eq!(refusal(pg, "timestamp", "-9999999w 9am"), Refusal::Years);
+        assert_eq!(refusal(pg, "date", "+9999999999999999999d"), UNREAD);
+    }
+
+    #[test]
+    fn an_offset_is_read_only_where_its_minutes_and_its_range_are_ones() {
+        let pg = Dialect::Postgres;
+        assert_eq!(
+            refusal(pg, "timestamptz", "2026-01-15 10:00+05:99"),
+            Refusal::NotRead(Kind::Zoned)
+        );
+        assert_eq!(
+            refusal(pg, "timestamptz", "2026-01-15 10:00+0599"),
+            Refusal::NotRead(Kind::Zoned)
+        );
+        assert_eq!(
+            refusal(pg, "timetz", "10:00+16"),
+            Refusal::Range(Range::PgOffset)
+        );
+        assert_eq!(
+            refusal(pg, "timetz", "10:00+25:00"),
+            Refusal::Range(Range::PgOffset)
+        );
+        assert_eq!(literal(pg, "timetz", "10:00+15:59"), "10:00:00+15:59");
+    }
+
+    #[test]
+    fn a_time_rounded_past_its_day_is_the_days_end() {
+        let pg = Dialect::Postgres;
+        assert_eq!(literal(pg, "time(0)", "23:59:59.7"), "24:00:00");
+        assert_eq!(literal(pg, "time", "23:59:59.9999999"), "24:00:00");
+        assert_eq!(literal(pg, "time", "24:00"), "24:00:00");
+        assert_eq!(literal(pg, "time", "24:00:00.000"), "24:00:00");
+        assert_eq!(literal(Dialect::Sqlite, "TIME", "24:00:00"), "24:00:00");
+        assert_eq!(
+            refusal(pg, "time", "24:00:01"),
+            Refusal::NoTime("24:00:01".into())
+        );
+        let my = Dialect::MySql;
+        assert_eq!(literal(my, "time", "23:59:59.7"), "24:00:00");
+        // A span is rounded as a time of day is.
+        assert_eq!(literal(my, "time", "100:30:00.9"), "100:30:01");
+        assert_eq!(literal(my, "time", "12:00:00.9"), "12:00:01");
+        assert_eq!(literal(my, "time(2)", "-12:00:00.125"), "-12:00:00.13");
+        assert_eq!(
+            refusal(my, "time", "838:59:59.5"),
+            Refusal::Range(Range::MySqlTime)
+        );
+    }
+
+    #[test]
+    fn a_choice_keeps_the_fraction_of_the_time_it_moves() {
+        let column = column(Dialect::Postgres, "timestamptz");
+        let Err(Refusal::Gap { offered, .. }) = read("2026-03-08 02:30:15.5", &column, &clock())
+        else {
+            panic!("a gap");
+        };
+        let text = rewritten(offered, None, &column);
+        assert_eq!(text, "2026-03-08 03:30:15.5");
+        assert_eq!(
+            read(&text, &column, &clock()).unwrap().literal,
+            "2026-03-08 03:30:15.5-04"
+        );
+    }
+
+    #[test]
+    fn an_era_goes_only_after_a_year_that_is_written() {
+        let pg = Dialect::Postgres;
+        for typed in ["today bc", "15 jan bc", "+7d bc"] {
+            assert_eq!(refusal(pg, "date", typed), UNREAD, "{typed}");
+        }
+        assert_eq!(
+            refusal(pg, "timestamp", "9:30 bc"),
+            Refusal::NotRead(Kind::Timestamp)
+        );
+        assert_eq!(literal(pg, "date", "15 jan 2026 bc"), "2026-01-15 BC");
+    }
+
+    #[test]
+    fn a_mysql_session_whose_zone_is_not_known_takes_no_day_counted_from_today() {
+        let unnamed = Column::of(Dialect::MySql, "timestamp", None, []).unwrap();
+        for typed in ["now", "tomorrow 9am", "9:30", "+1d"] {
+            assert_eq!(
+                read(typed, &unnamed, &clock()),
+                Err(Refusal::ZoneUnknown),
+                "{typed}"
+            );
+        }
+        assert!(read("2026-01-15 09:30", &unnamed, &clock()).is_ok());
+    }
+
+    #[test]
+    fn sqlite_times_and_julian_days_follow_their_columns() {
+        let lite = Dialect::Sqlite;
+        let of = |type_name: &str, values: &[Value]| {
+            Column::of(lite, type_name, None, values.iter()).unwrap()
+        };
+        // A time kept as seconds: of the day, opened as its time.
+        let times = of("TIME", &[Value::Int(3_600)]);
+        assert_eq!(
+            opened(&Value::Int(3_600), &times).as_deref(),
+            Some("01:00:00")
+        );
+        assert_eq!(read("01:00:00", &times, &clock()).unwrap().literal, "3600");
+        assert_eq!(read("2:00", &times, &clock()).unwrap().literal, "7200");
+        assert_eq!(read("7200", &times, &clock()).unwrap().literal, "7200");
+        // Julian days, as `julianday()` keeps them.
+        let days = of("DATE", &[Value::Float(2_461_337.5)]);
+        assert_eq!(days.stored, Stored::Days);
+        assert_eq!(
+            opened(&Value::Float(2_461_337.5), &days).as_deref(),
+            Some("2026-10-24")
+        );
+        assert_eq!(
+            read("25 Oct 2026", &days, &clock()).unwrap().literal,
+            "2461338.5"
+        );
+        assert_eq!(
+            read("2461337.5", &days, &clock()).unwrap().literal,
+            "2461337.5"
+        );
+        // A Julian day begins at noon: .5 is midnight, .75 six in the
+        // morning.
+        let stamps = of("DATETIME", &[Value::Float(2_461_337.75)]);
+        assert_eq!(
+            opened(&Value::Float(2_461_337.75), &stamps).as_deref(),
+            Some("2026-10-24 06:00:00")
+        );
+        assert_eq!(
+            read("2026-10-24 06:00", &stamps, &clock()).unwrap().literal,
+            "2461337.75"
+        );
+    }
+
+    #[test]
+    fn a_zones_place_is_the_last_part_of_its_name() {
+        assert_eq!(place(&new_york()), "New York");
+        assert_eq!(place(&TimeZone::UTC), "UTC");
+        assert_eq!(
+            written_offset(Offset::from_seconds(-17_762).unwrap()),
+            "-04:56:02"
+        );
+        assert_eq!(
+            written_offset(Offset::from_seconds(19_800).unwrap()),
+            "+05:30"
+        );
+    }
+}
```

`src/lib.rs`:

```diff
diff --git a/src/lib.rs b/src/lib.rs
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -6,6 +6,7 @@
 pub mod clock;
 pub mod completion;
 pub mod connections;
+pub mod dates;
 pub mod desktop_theme;
 pub mod edit;
 pub mod endpoint;
```


- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib dates::`
Expected: no build, with errors such as:

```
error[E0425]: cannot find function `read` in this scope
error[E0425]: cannot find function `kind` in this scope
error[E0433]: cannot find type `Refusal` in this scope
error: could not compile `tabletist` (lib test) due to 209 previous errors; 2 warnings emitted
```

- [ ] **Step 3: Write the module**

Put the code before the tests, so that the file reads:

`src/dates.rs`:

```diff
diff --git a/src/dates.rs b/src/dates.rs
--- a/src/dates.rs
+++ b/src/dates.rs
@@ -1,4 +1,1472 @@
-use jiff::civil::Date;
+//! Dates and times as an editor reads them: what a column of dates or
+//! times keeps (its kind, the digits of its seconds, its range), the text
+//! typed into it read as a moment (ISO 8601, a day with a month's name,
+//! the numbers of a day in the locale's order, a time of day, words such
+//! as `tomorrow`), and the literal a save sends for it. The text is the
+//! person's; what is sent is written here, the same way every time.
+//!
+//! Nothing here draws, and nothing reads a clock: the time it is now and
+//! the person's zone come in a [`Clock`].
+
+use jiff::civil::{Date, DateTime, Time, Weekday};
+use jiff::tz::{AmbiguousOffset, Offset, TimeZone};
+use jiff::{Timestamp, Unit};
+use tabletist_db::{Dialect, Value};
+
+/// What a column of dates or times keeps, which decides what its editor
+/// reads and shows.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Kind {
+    /// A day.
+    Date,
+    /// A day and a time, with no zone: stored as typed.
+    Timestamp,
+    /// A moment, shown and typed in a zone: PostgreSQL's `timestamptz`,
+    /// and MySQL's `TIMESTAMP`, which the server moves to and from the
+    /// session's zone.
+    Zoned,
+    /// A time of day. `offset` where the type keeps one (`timetz`).
+    Time { offset: bool },
+}
+
+impl Kind {
+    /// Whether a value of the kind has a day.
+    pub fn has_date(self) -> bool {
+        match self {
+            Self::Date | Self::Timestamp | Self::Zoned => true,
+            Self::Time { .. } => false,
+        }
+    }
+
+    /// Whether a value of the kind has a time of day.
+    pub fn has_time(self) -> bool {
+        match self {
+            Self::Timestamp | Self::Zoned | Self::Time { .. } => true,
+            Self::Date => false,
+        }
+    }
+}
+
+/// The type's name without what is in its parentheses: `timestamp(3)
+/// with time zone` is `timestamp with time zone`.
+fn unsized_name(type_name: &str) -> String {
+    let name = type_name.trim().to_ascii_lowercase();
+    let mut plain = String::with_capacity(name.len());
+    let mut depth = 0_u32;
+    for letter in name.chars() {
+        match letter {
+            '(' => depth += 1,
+            ')' => depth = depth.saturating_sub(1),
+            letter if depth == 0 => plain.push(letter),
+            _ => {}
+        }
+    }
+    plain.split_whitespace().collect::<Vec<_>>().join(" ")
+}
+
+/// The kind of the column whose type the catalog names `type_name`, by
+/// the engine's own names. `None` for every other type: `interval`,
+/// MySQL's `YEAR`, an array of dates (its editor is the array's).
+pub fn kind(dialect: Dialect, type_name: &str) -> Option<Kind> {
+    if type_name.trim().ends_with("[]") {
+        return None;
+    }
+    let name = unsized_name(type_name);
+    match dialect {
+        Dialect::Postgres => match name.as_str() {
+            "date" => Some(Kind::Date),
+            "timestamp" | "timestamp without time zone" => Some(Kind::Timestamp),
+            "timestamptz" | "timestamp with time zone" => Some(Kind::Zoned),
+            "time" | "time without time zone" => Some(Kind::Time { offset: false }),
+            "timetz" | "time with time zone" => Some(Kind::Time { offset: true }),
+            _ => None,
+        },
+        // The first word: `datetime(6)`, `timestamp`, and whatever the
+        // catalog writes after them.
+        Dialect::MySql => match name.split(' ').next().unwrap_or_default() {
+            "date" => Some(Kind::Date),
+            "datetime" => Some(Kind::Timestamp),
+            "timestamp" => Some(Kind::Zoned),
+            "time" => Some(Kind::Time { offset: false }),
+            _ => None,
+        },
+        // SQLite has no such types: the declared name says what the
+        // column is meant to keep.
+        Dialect::Sqlite => {
+            let date = name.contains("date");
+            let time = name.contains("time");
+            if name.contains("timestamp") || (date && time) {
+                Some(Kind::Timestamp)
+            } else if date {
+                Some(Kind::Date)
+            } else if time {
+                Some(Kind::Time { offset: false })
+            } else {
+                None
+            }
+        }
+    }
+}
+
+/// The digits of a second's fraction a column keeps: those its type
+/// states, else the engine's own. SQLite keeps what it is given.
+fn kept_digits(dialect: Dialect, type_name: &str) -> u8 {
+    let stated = type_name
+        .split_once('(')
+        .and_then(|(_, rest)| rest.split(')').next())
+        .and_then(|digits| digits.trim().parse::<u8>().ok());
+    match (dialect, stated) {
+        (Dialect::Postgres | Dialect::MySql, Some(digits)) => digits.min(6),
+        (Dialect::Postgres, None) => 6,
+        (Dialect::MySql, None) => 0,
+        (Dialect::Sqlite, _) => 9,
+    }
+}
+
+/// How the values a SQLite column already holds are kept, which a new
+/// value follows. Other engines keep a type, and are written as theirs.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Stored {
+    /// No value to follow: ISO 8601 text.
+    Fresh,
+    /// Text, its day and time apart by `separator` (` ` or `T`).
+    Text { separator: char },
+    /// Whole seconds since 1970 (UTC).
+    Seconds,
+    /// Julian day numbers (REAL), as SQLite's `julianday()` gives them:
+    /// days and their fraction since noon UTC on 24 November 4714 BC.
+    Days,
+}
+
+impl Stored {
+    /// How the first value of `values` that is not NULL is kept.
+    pub fn of<'a>(values: impl IntoIterator<Item = &'a Value>) -> Self {
+        for value in values {
+            match value {
+                Value::Null => {}
+                Value::Int(_) => return Self::Seconds,
+                Value::Float(_) => return Self::Days,
+                Value::Text(text) => {
+                    let separator = match text.as_bytes().get(10) {
+                        Some(b'T') => 'T',
+                        _ => ' ',
+                    };
+                    return Self::Text { separator };
+                }
+                Value::Bool(_) | Value::Bytes(_) => return Self::Fresh,
+            }
+        }
+        Self::Fresh
+    }
+}
+
+/// A column of dates or times, as an editor reads text for it.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Column {
+    pub dialect: Dialect,
+    pub kind: Kind,
+    /// The digits of a second's fraction it keeps.
+    pub digits: u8,
+    /// How SQLite's values of it are kept ([`Stored::Fresh`] elsewhere).
+    pub stored: Stored,
+    /// The zone a [`Kind::Zoned`] value is shown and typed in: the
+    /// settings' for PostgreSQL, the session's for MySQL. `None` where a
+    /// MySQL session's is not known (not answered yet, or named in a way
+    /// the database of zones does not know): the value is read as the
+    /// server's clock writes it, and no change of the clocks is found.
+    pub zone: Option<TimeZone>,
+}
+
+impl Column {
+    /// The column of `type_name`, where it keeps dates or times. `values`
+    /// are those of it loaded, which a SQLite value follows.
+    pub fn of<'a>(
+        dialect: Dialect,
+        type_name: &str,
+        zone: Option<&TimeZone>,
+        values: impl IntoIterator<Item = &'a Value>,
+    ) -> Option<Self> {
+        let kind = kind(dialect, type_name)?;
+        let stored = match dialect {
+            Dialect::Sqlite => Stored::of(values),
+            Dialect::Postgres | Dialect::MySql => Stored::Fresh,
+        };
+        Some(Self {
+            dialect,
+            kind,
+            digits: kept_digits(dialect, type_name),
+            stored,
+            zone: match kind {
+                Kind::Zoned => zone.cloned(),
+                Kind::Date | Kind::Timestamp | Kind::Time { .. } => None,
+            },
+        })
+    }
+}
+
+/// The order a day written in numbers alone is read in (`15/01/2026`), by
+/// the region of the system's locale.
+#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
+pub enum Order {
+    /// `15/01/2026`.
+    #[default]
+    DayMonth,
+    /// `01/15/2026`.
+    MonthDay,
+    /// `2026/01/15`.
+    YearMonth,
+}
+
+impl Order {
+    /// The order of the region `region` (`us`, `de`, `jp`: lower case, as
+    /// `fastframe_i18n::LanguageTag` gives it), by CLDR's short date
+    /// patterns. Day first where it is not known.
+    pub fn of_region(region: Option<&str>) -> Self {
+        match region.unwrap_or_default() {
+            "us" | "ph" | "fm" | "mh" | "pw" | "as" | "gu" | "mp" | "pr" | "um" | "vi" | "bz" => {
+                Self::MonthDay
+            }
+            "cn" | "jp" | "kr" | "kp" | "tw" | "hu" | "lt" | "mn" | "ir" | "bt" | "se" | "za"
+            | "ca" => Self::YearMonth,
+            _ => Self::DayMonth,
+        }
+    }
+}
+
+/// What a reading needs of the moment it is read at: the time it is, the
+/// zone the person's days are counted in, and how they write a day.
+#[derive(Debug, Clone)]
+pub struct Clock {
+    pub now: Timestamp,
+    pub zone: TimeZone,
+    pub order: Order,
+}
+
+impl Clock {
+    /// Today, in `zone`.
+    pub fn today_in(&self, zone: &TimeZone) -> Date {
+        self.now.to_zoned(zone.clone()).date()
+    }
+
+    /// The time it is in `zone`, to the second.
+    fn now_in(&self, zone: &TimeZone) -> DateTime {
+        let at = self.now.to_zoned(zone.clone()).datetime();
+        at.with().subsec_nanosecond(0).build().unwrap_or(at)
+    }
+}
+
+/// A value read: a day, a time, or both, with the offset a zoned value
+/// has, or one of the values that are no day.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Moment {
+    At {
+        date: Option<Date>,
+        time: Option<Time>,
+        offset: Option<Offset>,
+    },
+    /// PostgreSQL's `infinity`: later than every day.
+    Infinity,
+    /// PostgreSQL's `-infinity`: earlier than every day.
+    MinusInfinity,
+    /// `24:00:00`: the end of a day, which a PostgreSQL or SQLite time
+    /// holds.
+    EndOfDay,
+    /// A MySQL `TIME` past a day or below zero, which the type keeps as a
+    /// span of time: written as it is sent (`-12:00:00`, `100:30:00`).
+    Span(String),
+}
+
+/// The two moments a time the clocks go back over is, in the zone.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Fold {
+    /// The time, as the zone's clock shows it twice.
+    pub at: DateTime,
+    /// Where it is, as the zone's name says it ("New York").
+    pub place: String,
+    /// The first, before the clocks go back (summer time), with the
+    /// zone's abbreviation then ("EDT").
+    pub earlier: (Offset, String),
+    /// The second, after it ("EST").
+    pub later: (Offset, String),
+}
+
+/// What a text is as a value of its column.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Reading {
+    /// The text read.
+    pub typed: String,
+    pub moment: Moment,
+    /// The digits of the second's fraction it has, as it will be stored.
+    pub digits: u8,
+    /// What a save sends.
+    pub literal: String,
+    /// The day was named by a word or a count from today.
+    pub relative: bool,
+    /// How many days from today the day is (in the zone it is counted
+    /// in): `1` tomorrow, `-1` yesterday. `None` for a value with no day.
+    pub days: Option<i64>,
+    /// The zone's abbreviation for a zoned value then ("EST").
+    pub zone_name: Option<String>,
+    /// The time is one the clocks go back over: the value is the moment
+    /// of its offset, the first one where none was typed.
+    pub fold: Option<Fold>,
+}
+
+/// The limits a type sets, named by what holds them.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Range {
+    /// PostgreSQL's `date`: 4714-11-24 BC to 5874897 AD.
+    PgDate,
+    /// PostgreSQL's `timestamp` and `timestamptz`: 4714-11-24 BC to 294276
+    /// AD.
+    PgTimestamp,
+    /// MySQL's `DATE`: 1000-01-01 to 9999-12-31.
+    MySqlDate,
+    /// MySQL's `DATETIME`: 1000-01-01 to 9999-12-31.
+    MySqlDatetime,
+    /// MySQL's `TIMESTAMP`: 1970-01-01 00:00:01 to 2038-01-19 03:14:07
+    /// UTC.
+    MySqlTimestamp,
+    /// MySQL's `TIME`, a span of time: -838:59:59 to 838:59:59.
+    MySqlTime,
+    /// PostgreSQL's `timetz` offsets: -15:59:59 to +15:59:59.
+    PgOffset,
+}
+
+/// Why a text is no value of its column.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Refusal {
+    /// Nothing here reads as a value of the kind.
+    NotRead(Kind),
+    /// A month that is none: `13`.
+    NoMonth(i64),
+    /// A day the month does not have: 30 February.
+    NoDay { day: i64, month: i8 },
+    /// A time of day that is none, as it was typed: `25:00`.
+    NoTime(String),
+    /// Outside what the type holds.
+    Range(Range),
+    /// A year of more than four digits, which the app does not read.
+    Years,
+    /// PostgreSQL counts no year 0: 1 BC is just before 1 AD.
+    YearZero,
+    /// A time the zone's clocks skip. `offered` is the time the same
+    /// moment is once they have gone forward: 03:30 for 02:30.
+    Gap {
+        at: DateTime,
+        place: String,
+        offered: DateTime,
+    },
+    /// MySQL's zero date, `0000-00-00`, which a server may hold and
+    /// nothing should be given.
+    Zero,
+    /// An offset typed for a column that keeps no zone.
+    NoZone,
+    /// A time typed for a column that keeps a day only.
+    DayOnly,
+    /// A day typed for a column that keeps a time only.
+    TimeOnly,
+    /// A day counted from today, a time without its day, or an offset, for
+    /// a MySQL `TIMESTAMP` whose session's zone is not known: what today is
+    /// there, and what the offset is in its clock, are the server's to
+    /// say.
+    ZoneUnknown,
+}
+
+/// The refusal of a text that reads as nothing, before [`read`] puts the
+/// column's kind in.
+const UNREAD: Refusal = Refusal::NotRead(Kind::Date);
+
+/// The place a zone is for, as its name says it: `America/New_York` is
+/// "New York". The zone's name where it has no place in it ("UTC"), and
+/// nothing where it has no name.
+pub fn place(zone: &TimeZone) -> String {
+    let name = zone.iana_name().unwrap_or_default();
+    name.rsplit('/')
+        .next()
+        .unwrap_or_default()
+        .replace('_', " ")
+}
+
+/// An offset as PostgreSQL writes one: `+00`, `-05`, `+05:30`,
+/// `-04:56:02`.
+pub fn written_offset(offset: Offset) -> String {
+    let seconds = offset.seconds();
+    let sign = if seconds < 0 { '-' } else { '+' };
+    let seconds = seconds.unsigned_abs();
+    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
+    match (minutes, seconds) {
+        (0, 0) => format!("{sign}{hours:02}"),
+        (_, 0) => format!("{sign}{hours:02}:{minutes:02}"),
+        _ => format!("{sign}{hours:02}:{minutes:02}:{seconds:02}"),
+    }
+}
+
+/// `time` as `HH:MM:SS`, with `digits` of its fraction.
+fn written_time(time: Time, digits: u8) -> String {
+    let mut out = format!(
+        "{:02}:{:02}:{:02}",
+        time.hour(),
+        time.minute(),
+        time.second()
+    );
+    if digits > 0 {
+        let nanos = format!("{:09}", time.subsec_nanosecond());
+        out.push('.');
+        out.push_str(&nanos[..usize::from(digits.min(9))]);
+    }
+    out
+}
+
+/// Whether the engine writes a year before 1 AD with its era (`0044 BC`):
+/// PostgreSQL alone. SQLite's year 0 is `0000`, and MySQL holds none.
+fn eras(dialect: Dialect) -> bool {
+    match dialect {
+        Dialect::Postgres => true,
+        Dialect::MySql | Dialect::Sqlite => false,
+    }
+}
+
+/// `date` as `YYYY-MM-DD`, its year as written: before 1 AD, where the
+/// engine writes an era, the year counted back (1 BC is `0001`), which
+/// `bc` then follows.
+fn written_date(date: Date, dialect: Dialect) -> (String, bool) {
+    let year = i32::from(date.year());
+    let bc = year < 1 && eras(dialect);
+    let shown = if bc { 1 - year } else { year };
+    (
+        format!("{shown:04}-{:02}-{:02}", date.month(), date.day()),
+        bc,
+    )
+}
+
+/// The value `typed` reads as for `column`, at `clock`, or why it reads
+/// as none.
+pub fn read(typed: &str, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
+    let mut reading = reading(typed, column, clock).map_err(|refusal| match refusal {
+        Refusal::NotRead(_) => Refusal::NotRead(column.kind),
+        refusal => refusal,
+    })?;
+    reading.typed = typed.to_owned();
+    let today = clock.today_in(column.zone.as_ref().unwrap_or(&clock.zone));
+    reading.days = match reading.moment {
+        Moment::At {
+            date: Some(date), ..
+        } => today
+            .until((Unit::Day, date))
+            .ok()
+            .map(|span| i64::from(span.get_days())),
+        Moment::At { date: None, .. }
+        | Moment::Infinity
+        | Moment::MinusInfinity
+        | Moment::Span(_)
+        | Moment::EndOfDay => None,
+    };
+    Ok(reading)
+}
+
+/// [`read`], before the text and the count of days are put in.
+fn reading(typed: &str, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
+    let text = typed.trim().to_ascii_lowercase();
+    if let Some(moment) = special(&text, column)? {
+        let literal = match &moment {
+            Moment::Infinity => "infinity".to_owned(),
+            Moment::MinusInfinity => "-infinity".to_owned(),
+            Moment::Span(written) => written.clone(),
+            Moment::EndOfDay => "24:00:00".to_owned(),
+            // SQLite's seconds: the day and time they are, written as the
+            // column keeps them.
+            Moment::At { date, time, .. } => {
+                let parts = Parts {
+                    day: date.map(Day::Known),
+                    time: time.map(|time| (time, 0)),
+                    ..Parts::default()
+                };
+                return resolve(parts, column, clock);
+            }
+        };
+        return Ok(Reading {
+            typed: String::new(),
+            moment,
+            digits: 0,
+            literal,
+            relative: false,
+            days: None,
+            zone_name: None,
+            fold: None,
+        });
+    }
+    let parts = scan(&text, column, clock)?;
+    resolve(parts, column, clock)
+}
+
+/// The words a value that is no day is typed as, and SQLite's seconds.
+fn special(text: &str, column: &Column) -> Result<Option<Moment>, Refusal> {
+    match column.dialect {
+        Dialect::Postgres if column.kind.has_time() && !column.kind.has_date() => {
+            Ok(end_of_day(text, column))
+        }
+        Dialect::Postgres => Ok(match text {
+            "infinity" | "+infinity" => Some(Moment::Infinity),
+            "-infinity" => Some(Moment::MinusInfinity),
+            _ => None,
+        }),
+        Dialect::MySql => {
+            let zero = text.strip_prefix("0000-00-00").is_some_and(|rest| {
+                let rest = rest.trim_start_matches([' ', 't']);
+                rest.is_empty()
+                    || rest
+                        .strip_prefix("00:00:00")
+                        .is_some_and(|fraction| fraction.trim_start_matches(['.', '0']).is_empty())
+            });
+            if zero && column.kind.has_date() {
+                return Err(Refusal::Zero);
+            }
+            match column.kind {
+                Kind::Time { .. } => span(text, column.digits),
+                Kind::Date | Kind::Timestamp | Kind::Zoned => Ok(None),
+            }
+        }
+        Dialect::Sqlite => {
+            if let Some(end) = end_of_day(text, column) {
+                return Ok(Some(end));
+            }
+            let Some(moment) = numbered(text, column.stored) else {
+                return Ok(None);
+            };
+            let at = moment?.to_zoned(TimeZone::UTC).datetime();
+            Ok(Some(Moment::At {
+                date: column.kind.has_date().then(|| at.date()),
+                time: column.kind.has_time().then(|| at.time()),
+                offset: None,
+            }))
+        }
+    }
+}
+
+/// The moment SQLite's number `text` is, where the column keeps its values
+/// as numbers. `None` for any other text.
+fn numbered(text: &str, stored: Stored) -> Option<Result<Timestamp, Refusal>> {
+    let moment = match stored {
+        Stored::Seconds => Timestamp::from_second(text.parse::<i64>().ok()?),
+        Stored::Days => {
+            let days = text.parse::<f64>().ok().filter(|days| days.is_finite())?;
+            let millis = ((days - JULIAN_EPOCH) * MILLIS_A_DAY).round();
+            if millis.abs() > 1e15 {
+                return Some(Err(Refusal::Years));
+            }
+            Timestamp::from_millisecond(millis as i64)
+        }
+        Stored::Fresh | Stored::Text { .. } => return None,
+    };
+    Some(moment.map_err(|_| Refusal::Years))
+}
+
+/// The Julian day number of 1970-01-01 00:00 UTC, and a day's
+/// milliseconds.
+const JULIAN_EPOCH: f64 = 2_440_587.5;
+const MILLIS_A_DAY: f64 = 86_400_000.0;
+
+/// `24:00` (with seconds or a fraction of nothing), the end of a day, which
+/// a time of PostgreSQL's or SQLite's holds. `None` for any other text.
+fn end_of_day(text: &str, column: &Column) -> Option<Moment> {
+    let rest = text.strip_prefix("24:00")?;
+    let rest = rest.strip_prefix(":00").unwrap_or(rest);
+    let rest = match rest.strip_prefix('.') {
+        Some(fraction) if fraction.bytes().all(|byte| byte == b'0') => "",
+        Some(_) => return None,
+        None => rest,
+    };
+    (rest.is_empty() && !column.kind.has_date()).then_some(Moment::EndOfDay)
+}
+
+/// A MySQL `TIME` that is no time of day: below zero, or of a day and
+/// more (`-12:00`, `100:30:00`), with `places` of its fraction kept. `None`
+/// for any other text: a time of day is read as one.
+fn span(text: &str, places: u8) -> Result<Option<Moment>, Refusal> {
+    let (negative, rest) = match text.strip_prefix('-') {
+        Some(rest) => (true, rest),
+        None => (false, text),
+    };
+    let (hours, rest) = digits(rest);
+    let Some(rest) = rest.strip_prefix(':') else {
+        return Ok(None);
+    };
+    let (minutes, rest) = digits(rest);
+    let (seconds, rest) = match rest.strip_prefix(':') {
+        Some(rest) => digits(rest),
+        None => ("00", rest),
+    };
+    let (fraction, rest) = match rest.strip_prefix('.') {
+        Some(rest) => digits(rest),
+        None => ("", rest),
+    };
+    let formed = (1..=3).contains(&hours.len())
+        && minutes.len() == 2
+        && seconds.len() == 2
+        && rest.is_empty();
+    let number = |part: &str| part.parse::<u32>().unwrap_or(u32::MAX);
+    let (hour, minute, second) = (number(hours), number(minutes), number(seconds));
+    if !formed || !(negative || hour >= 24) {
+        return Ok(None);
+    }
+    if minute > 59 || second > 59 {
+        return Err(Refusal::NoTime(text.to_owned()));
+    }
+    if fraction.len() > 9 {
+        return Ok(None);
+    }
+    // Rounded to the places kept, as the server rounds: half up, carried
+    // into the seconds and on.
+    let nanos: u64 = format!("{fraction:0<9}").parse().unwrap_or_default();
+    let step = 10_u64.pow(9 - u32::from(places.min(9)));
+    let whole = (u64::from(hour) * 3_600 + u64::from(minute) * 60 + u64::from(second))
+        * 1_000_000_000
+        + nanos;
+    let whole = (whole + step / 2) / step * step;
+    if whole > 838 * 3_600 * 1_000_000_000 + 3_599 * 1_000_000_000 {
+        return Err(Refusal::Range(Range::MySqlTime));
+    }
+    let (seconds, nanos) = (whole / 1_000_000_000, whole % 1_000_000_000);
+    let sign = if negative && whole > 0 { "-" } else { "" };
+    let mut written = format!(
+        "{sign}{:02}:{:02}:{:02}",
+        seconds / 3_600,
+        seconds / 60 % 60,
+        seconds % 60
+    );
+    if places > 0 && nanos > 0 {
+        let nanos = format!("{nanos:09}");
+        written.push('.');
+        written.push_str(&nanos[..usize::from(places.min(9))]);
+    }
+    Ok(Some(Moment::Span(written)))
+}
+
+/// A day as it was typed, before its era and its month are checked.
+#[derive(Debug, Clone, Copy)]
+enum Day {
+    Numbers {
+        year: i64,
+        month: i64,
+        day: i64,
+    },
+    /// Named by a word or a count from today.
+    Known(Date),
+    /// `now`: today, and the time it is.
+    Now,
+}
+
+/// What a text was made of.
+#[derive(Debug, Default)]
+struct Parts {
+    day: Option<Day>,
+    time: Option<(Time, u8)>,
+    offset: Option<Offset>,
+    bc: bool,
+    relative: bool,
+}
+
+/// Splits the run of ASCII digits at the start of `text` off it.
+fn digits(text: &str) -> (&str, &str) {
+    let end = text.bytes().take_while(u8::is_ascii_digit).count();
+    text.split_at(end)
+}
+
+/// Splits the run of ASCII letters at the start of `text` off it.
+fn letters(text: &str) -> (&str, &str) {
+    let end = text.bytes().take_while(u8::is_ascii_alphabetic).count();
+    text.split_at(end)
+}
+
+/// The month `word` begins the English name of, three letters at least.
+fn month(word: &str) -> Option<i64> {
+    const MONTHS: [&str; 12] = [
+        "january",
+        "february",
+        "march",
+        "april",
+        "may",
+        "june",
+        "july",
+        "august",
+        "september",
+        "october",
+        "november",
+        "december",
+    ];
+    if word.len() < 3 {
+        return None;
+    }
+    let at = MONTHS.iter().position(|name| name.starts_with(word))?;
+    i64::try_from(at).ok().map(|at| at + 1)
+}
+
+/// The weekday `word` begins the English name of, three letters at least.
+fn weekday(word: &str) -> Option<Weekday> {
+    const DAYS: [(&str, Weekday); 7] = [
+        ("monday", Weekday::Monday),
+        ("tuesday", Weekday::Tuesday),
+        ("wednesday", Weekday::Wednesday),
+        ("thursday", Weekday::Thursday),
+        ("friday", Weekday::Friday),
+        ("saturday", Weekday::Saturday),
+        ("sunday", Weekday::Sunday),
+    ];
+    if word.len() < 3 {
+        return None;
+    }
+    DAYS.iter()
+        .find(|(name, _)| name.starts_with(word))
+        .map(|(_, day)| *day)
+}
+
+/// A year after a day and its month: four digits or more, not the hour
+/// of a time that follows.
+fn year(text: &str) -> Option<(i64, &str)> {
+    let text = text.trim_start_matches([' ', ',']);
+    let (year, rest) = digits(text);
+    if year.len() < 4 || rest.starts_with([':', '.']) {
+        return None;
+    }
+    Some((year.parse().ok()?, rest))
+}
+
+/// `2026-01-15`.
+fn iso_day(text: &str) -> Option<(Day, &str)> {
+    let (year, rest) = digits(text);
+    if year.len() < 4 {
+        return None;
+    }
+    let (month, rest) = digits(rest.strip_prefix('-')?);
+    let (day, rest) = digits(rest.strip_prefix('-')?);
+    if !(1..=2).contains(&month.len()) || !(1..=2).contains(&day.len()) {
+        return None;
+    }
+    let numbers = Day::Numbers {
+        year: year.parse().ok()?,
+        month: month.parse().ok()?,
+        day: day.parse().ok()?,
+    };
+    Some((numbers, rest))
+}
+
+/// `15/01/2026`, `15.01.2026` or `15-01-2026` in the locale's order, or
+/// `2026/01/15` with its year first in any.
+fn numeric_day(text: &str, order: Order) -> Option<(Day, &str)> {
+    let (first, rest) = digits(text);
+    let separator = rest
+        .chars()
+        .next()
+        .filter(|c| ['/', '.', '-'].contains(c))?;
+    let (second, rest) = digits(&rest[1..]);
+    let (third, rest) = digits(rest.strip_prefix(separator)?);
+    if first.is_empty() || !(1..=2).contains(&second.len()) || third.is_empty() {
+        return None;
+    }
+    let number = |part: &str| part.parse::<i64>().ok();
+    let (year, month, day) = if first.len() >= 4 {
+        (first, second, third)
+    } else if third.len() >= 4 && first.len() <= 2 {
+        match order {
+            Order::DayMonth | Order::YearMonth => (third, second, first),
+            Order::MonthDay => (third, first, second),
+        }
+    } else {
+        return None;
+    };
+    let numbers = Day::Numbers {
+        year: number(year)?,
+        month: number(month)?,
+        day: number(day)?,
+    };
+    Some((numbers, rest))
+}
+
+/// `15 Jan 2026`, `Jan 15 2026`, `January 15, 2026`; this year's where no
+/// year follows.
+fn named_day(text: &str, this_year: i64) -> Option<(Day, &str, bool)> {
+    let (number, rest) = digits(text);
+    let (day, month_number, rest) = if (1..=2).contains(&number.len()) {
+        let (word, rest) = letters(rest.trim_start());
+        (number, month(word)?, rest)
+    } else {
+        let (word, rest) = letters(text);
+        let month_number = month(word)?;
+        let (day, rest) = digits(rest.trim_start_matches([' ', '.']));
+        if !(1..=2).contains(&day.len()) {
+            return None;
+        }
+        let rest = ["st", "nd", "rd", "th"]
+            .iter()
+            .find_map(|ending| rest.strip_prefix(ending))
+            .unwrap_or(rest);
+        (day, month_number, rest)
+    };
+    let rest = rest.strip_prefix('.').unwrap_or(rest);
+    let typed = year(rest);
+    let named = typed.is_some();
+    let (year, rest) = typed.unwrap_or((this_year, rest));
+    let numbers = Day::Numbers {
+        year,
+        month: month_number,
+        day: day.parse().ok()?,
+    };
+    Some((numbers, rest, named))
+}
+
+/// `today`, `tomorrow`, `yesterday`, `now`, `next fri`, `fri`, `last mon`,
+/// `+7d`, `-2w`, `+1m`, `+1y`.
+fn relative_day(text: &str, today: Date) -> Result<Option<(Day, &str)>, Refusal> {
+    let (word, rest) = letters(text);
+    let known = |date: Result<Date, jiff::Error>| date.map(Day::Known).map_err(|_| Refusal::Years);
+    let found = match word {
+        "today" => Day::Known(today),
+        "tomorrow" => known(today.tomorrow())?,
+        "yesterday" => known(today.yesterday())?,
+        "now" => Day::Now,
+        "next" | "last" => {
+            let (name, after) = letters(rest.trim_start());
+            let Some(day) = weekday(name) else {
+                return Ok(None);
+            };
+            let nth = if word == "next" { 1 } else { -1 };
+            return Ok(Some((known(today.nth_weekday(nth, day))?, after)));
+        }
+        "" => {
+            let sign = match text.as_bytes().first() {
+                Some(b'+') => 1,
+                Some(b'-') => -1,
+                _ => return Ok(None),
+            };
+            let (count, rest) = digits(&text[1..]);
+            let (unit, rest) = letters(rest);
+            let Ok(count) = count.parse::<i64>() else {
+                return Ok(None);
+            };
+            let count = sign * count;
+            // A count past what a span holds is past every day there is.
+            let span = jiff::Span::new();
+            let span = match unit {
+                "d" | "day" | "days" => span.try_days(count),
+                "w" | "week" | "weeks" => span.try_weeks(count),
+                "m" | "month" | "months" => span.try_months(count),
+                "y" | "year" | "years" => span.try_years(count),
+                _ => return Ok(None),
+            }
+            .map_err(|_| Refusal::Years)?;
+            return Ok(Some((known(today.checked_add(span))?, rest)));
+        }
+        name => match weekday(name) {
+            Some(day) => known(today.nth_weekday(1, day))?,
+            None => return Ok(None),
+        },
+    };
+    Ok(Some((found, rest)))
+}
+
+/// What comes between a day and its time: `T`, a space, a comma, `at`.
+fn separator(text: &str) -> &str {
+    if let Some(rest) = text.strip_prefix('t')
+        && rest.starts_with(|c: char| c.is_ascii_digit())
+    {
+        return rest;
+    }
+    let rest = text.trim_start_matches([' ', ',']);
+    rest.strip_prefix("at ").unwrap_or(rest).trim_start()
+}
+
+/// `9:30`, `21:30:15.25`, `9.30`, `9am`, `9:30 pm`, with the digits of
+/// its fraction. `None` where `text` begins with no time.
+fn time_of_day(text: &str) -> Result<Option<(Time, u8, &str)>, Refusal> {
+    let (hour, mut rest) = digits(text);
+    if !(1..=2).contains(&hour.len()) {
+        return Ok(None);
+    }
+    let (mut minute, mut second, mut nanos, mut fraction) = ("0", "0", 0_i32, 0_u8);
+    let mut written = false;
+    if let Some(after) = rest.strip_prefix(':') {
+        let (part, after) = digits(after);
+        if part.len() != 2 {
+            return Err(UNREAD);
+        }
+        (minute, rest, written) = (part, after, true);
+        if let Some(after) = rest.strip_prefix(':') {
+            let (part, after) = digits(after);
+            if part.len() != 2 {
+                return Err(UNREAD);
+            }
+            (second, rest) = (part, after);
+            if let Some(after) = rest.strip_prefix('.') {
+                let (part, after) = digits(after);
+                if !(1..=9).contains(&part.len()) {
+                    return Err(UNREAD);
+                }
+                fraction = part.len() as u8;
+                nanos = format!("{part:0<9}").parse().unwrap_or_default();
+                rest = after;
+            }
+        }
+    } else if let Some(after) = rest.strip_prefix('.') {
+        let (part, after) = digits(after);
+        if part.len() == 2 {
+            (minute, rest, written) = (part, after, true);
+        }
+    }
+    let after = rest.trim_start();
+    let meridiem = ["am", "pm", "a.m.", "p.m."]
+        .iter()
+        .find_map(|word| Some((word.starts_with('p'), after.strip_prefix(word)?)));
+    if !written && meridiem.is_none() {
+        return Ok(None);
+    }
+    if let Some((_, after)) = meridiem {
+        rest = after;
+    }
+    let piece = text[..text.len() - rest.len()].trim();
+    let refused = || Refusal::NoTime(piece.to_owned());
+    let number = |part: &str| part.parse::<i8>().map_err(|_| refused());
+    let mut hour = number(hour)?;
+    if let Some((afternoon, _)) = meridiem {
+        if !(1..=12).contains(&hour) {
+            return Err(refused());
+        }
+        hour = hour % 12 + if afternoon { 12 } else { 0 };
+    }
+    let time = Time::new(hour, number(minute)?, number(second)?, nanos).map_err(|_| refused())?;
+    Ok(Some((time, fraction, rest)))
+}
+
+/// `Z`, `UTC`, `+05`, `-05:00`, `+0530`, `-04:56:02`.
+fn offset(text: &str) -> Option<(Offset, &str)> {
+    for word in ["z", "utc", "gmt"] {
+        if let Some(rest) = text.strip_prefix(word)
+            && !rest.starts_with(|c: char| c.is_ascii_alphabetic())
+        {
+            return Some((Offset::UTC, rest));
+        }
+    }
+    let sign = match text.as_bytes().first()? {
+        b'+' => 1,
+        b'-' => -1,
+        _ => return None,
+    };
+    let (first, mut rest) = digits(&text[1..]);
+    let (hours, mut minutes, mut seconds) = match first.len() {
+        2 => (first, "0", "0"),
+        4 => (&first[..2], &first[2..], "0"),
+        _ => return None,
+    };
+    if first.len() == 2
+        && let Some(after) = rest.strip_prefix(':')
+    {
+        let (part, after) = digits(after);
+        if part.len() != 2 {
+            return None;
+        }
+        (minutes, rest) = (part, after);
+        if let Some(after) = rest.strip_prefix(':') {
+            let (part, after) = digits(after);
+            if part.len() != 2 {
+                return None;
+            }
+            (seconds, rest) = (part, after);
+        }
+    }
+    let number = |part: &str| part.parse::<i32>().ok();
+    let (minutes, seconds) = (number(minutes)?, number(seconds)?);
+    if minutes > 59 || seconds > 59 {
+        return None;
+    }
+    let total = number(hours)? * 3_600 + minutes * 60 + seconds;
+    Some((Offset::from_seconds(sign * total).ok()?, rest))
+}
+
+/// The pieces of `text`, lower case and trimmed.
+fn scan(text: &str, column: &Column, clock: &Clock) -> Result<Parts, Refusal> {
+    let mut parts = Parts::default();
+    let mut rest = text;
+    // PostgreSQL writes an era after the whole value.
+    if eras(column.dialect) && column.kind.has_date() {
+        if let Some(before) = rest.strip_suffix(" bc") {
+            (parts.bc, rest) = (true, before.trim_end());
+        } else if let Some(before) = rest.strip_suffix(" ad") {
+            rest = before.trim_end();
+        }
+    }
+    let zone = column.zone.as_ref().unwrap_or(&clock.zone);
+    let today = clock.today_in(zone);
+    // Whether the day's year was written: an era is of a year typed.
+    let mut year_typed = true;
+    let day = match relative_day(rest, today)? {
+        Some(found) => {
+            parts.relative = true;
+            Some(found)
+        }
+        None => iso_day(rest)
+            .or_else(|| numeric_day(rest, clock.order))
+            .or_else(|| {
+                let (day, after, named) = named_day(rest, i64::from(today.year()))?;
+                year_typed = named;
+                Some((day, after))
+            }),
+    };
+    if let Some((day, after)) = day {
+        parts.day = Some(day);
+        rest = separator(after);
+    }
+    let numbers = matches!(parts.day, Some(Day::Numbers { .. }));
+    if parts.bc && !(numbers && year_typed) {
+        return Err(UNREAD);
+    }
+    if !matches!(parts.day, Some(Day::Now))
+        && let Some((time, fraction, after)) = time_of_day(rest)?
+    {
+        parts.time = Some((time, fraction));
+        rest = after;
+    }
+    let after = rest.trim_start();
+    if let Some((found, after)) = offset(after) {
+        parts.offset = Some(found);
+        rest = after;
+    }
+    if !rest.trim().is_empty() || (parts.day.is_none() && parts.time.is_none()) {
+        return Err(UNREAD);
+    }
+    Ok(parts)
+}
+
+/// The day the numbers name, in the era typed, as the column's engine
+/// counts years.
+fn checked_day(
+    column: &Column,
+    year: i64,
+    month: i64,
+    day: i64,
+    bc: bool,
+) -> Result<Date, Refusal> {
+    if year > 9999 {
+        return Err(Refusal::Years);
+    }
+    if eras(column.dialect) && year == 0 {
+        return Err(Refusal::YearZero);
+    }
+    let year = if bc { 1 - year } else { year };
+    let month = i8::try_from(month)
+        .ok()
+        .filter(|month| (1..=12).contains(month))
+        .ok_or(Refusal::NoMonth(month))?;
+    let first = Date::new(i16::try_from(year).map_err(|_| Refusal::Years)?, month, 1)
+        .map_err(|_| Refusal::Years)?;
+    let day = i8::try_from(day)
+        .ok()
+        .filter(|day| (1..=first.days_in_month()).contains(day))
+        .ok_or(Refusal::NoDay { day, month })?;
+    first.with().day(day).build().map_err(|_| UNREAD)
+}
+
+/// `at` rounded to `digits` of a second, as the databases round a value
+/// they keep fewer digits of.
+fn rounded(at: DateTime, digits: u8) -> DateTime {
+    if digits >= 9 {
+        return at;
+    }
+    // Half up, carried into the next second (and day): jiff's own
+    // rounding of a civil time wraps at midnight.
+    let step = 10_i32.pow(9 - u32::from(digits));
+    let kept = (at.subsec_nanosecond() + step / 2) / step * step;
+    let whole = at.with().subsec_nanosecond(0).build().unwrap_or(at);
+    if kept >= 1_000_000_000 {
+        whole
+            .checked_add(jiff::SignedDuration::from_secs(1))
+            .unwrap_or(at)
+    } else {
+        whole.with().subsec_nanosecond(kept).build().unwrap_or(at)
+    }
+}
+
+/// The two moments of a time the clocks of `zone` go back over.
+fn fold(zone: &TimeZone, at: DateTime) -> Option<Fold> {
+    let AmbiguousOffset::Fold { before, after } = zone.to_ambiguous_timestamp(at).offset() else {
+        return None;
+    };
+    let named = |offset: Offset| {
+        let name = offset
+            .to_timestamp(at)
+            .map(|moment| zone.to_offset_info(moment).abbreviation().to_owned())
+            .unwrap_or_default();
+        (offset, name)
+    };
+    Some(Fold {
+        at,
+        place: place(zone),
+        earlier: named(before),
+        later: named(after),
+    })
+}
+
+/// The first and the last moment MySQL's `TIMESTAMP` holds.
+fn mysql_timestamps() -> (Timestamp, Timestamp) {
+    let first = Timestamp::from_second(1).unwrap_or(Timestamp::UNIX_EPOCH);
+    let last = Timestamp::new(2_147_483_647, 999_999_000).unwrap_or(Timestamp::MAX);
+    (first, last)
+}
+
+/// The first day PostgreSQL keeps: 24 November 4714 BC.
+fn pg_first_day() -> Date {
+    Date::new(-4713, 11, 24).unwrap_or(Date::MIN)
+}
+
+/// Whether the column's type holds `at` (the moment `instant` is, for a
+/// zoned value).
+fn within(column: &Column, at: DateTime, instant: Option<Timestamp>) -> Result<(), Refusal> {
+    let date = at.date();
+    let thousand = Date::new(1000, 1, 1).unwrap_or(Date::MIN);
+    let refused = match (column.dialect, column.kind) {
+        (Dialect::Postgres, Kind::Date) => (date < pg_first_day()).then_some(Range::PgDate),
+        (Dialect::Postgres, Kind::Timestamp) => {
+            (date < pg_first_day()).then_some(Range::PgTimestamp)
+        }
+        (Dialect::Postgres, Kind::Zoned) => {
+            let first = pg_first_day()
+                .to_zoned(TimeZone::UTC)
+                .map(|zoned| zoned.timestamp())
+                .unwrap_or(Timestamp::MIN);
+            let instant = instant.unwrap_or_else(|| {
+                at.to_zoned(TimeZone::UTC)
+                    .map_or(Timestamp::MIN, |zoned| zoned.timestamp())
+            });
+            (instant < first).then_some(Range::PgTimestamp)
+        }
+        (Dialect::MySql, Kind::Date) => (date < thousand).then_some(Range::MySqlDate),
+        (Dialect::MySql, Kind::Timestamp) => (date < thousand).then_some(Range::MySqlDatetime),
+        (Dialect::MySql, Kind::Zoned) => {
+            // Where the session's zone is not known, as UTC: the server's
+            // own limits are within a day of it.
+            let instant = instant.unwrap_or_else(|| {
+                at.to_zoned(TimeZone::UTC)
+                    .map_or(Timestamp::MIN, |zoned| zoned.timestamp())
+            });
+            let (first, last) = mysql_timestamps();
+            (instant < first || instant > last).then_some(Range::MySqlTimestamp)
+        }
+        // SQLite's dates run from 0000 to 9999, and the years read do.
+        (Dialect::Sqlite, _) | (Dialect::Postgres | Dialect::MySql, Kind::Time { .. }) => None,
+    };
+    refused.map_or(Ok(()), |range| Err(Refusal::Range(range)))
+}
+
+/// The value `parts` are for `column`, and what is sent for it.
+fn resolve(parts: Parts, column: &Column, clock: &Clock) -> Result<Reading, Refusal> {
+    let here = column.zone.as_ref().unwrap_or(&clock.zone);
+    let now = matches!(parts.day, Some(Day::Now));
+    let date = match parts.day {
+        Some(Day::Numbers { year, month, day }) => {
+            Some(checked_day(column, year, month, day, parts.bc)?)
+        }
+        Some(Day::Known(date)) => Some(date),
+        Some(Day::Now) => Some(clock.today_in(here)),
+        None => None,
+    };
+    let (time, typed_digits) = match (parts.time, now) {
+        (Some((time, digits)), _) => (Some(time), digits),
+        (None, true) => (Some(clock.now_in(here).time()), 0),
+        (None, false) => (None, 0),
+    };
+    let digits = typed_digits.min(column.digits);
+    let mut reading = Reading {
+        typed: String::new(),
+        moment: Moment::Infinity,
+        digits,
+        literal: String::new(),
+        relative: parts.relative,
+        days: None,
+        zone_name: None,
+        fold: None,
+    };
+    match column.kind {
+        Kind::Date => {
+            if parts.time.is_some() {
+                return Err(Refusal::DayOnly);
+            }
+            if parts.offset.is_some() {
+                return Err(Refusal::NoZone);
+            }
+            let date = date.ok_or(UNREAD)?;
+            within(column, date.to_datetime(Time::midnight()), None)?;
+            reading.moment = Moment::At {
+                date: Some(date),
+                time: None,
+                offset: None,
+            };
+            reading.literal = match column.stored {
+                Stored::Seconds | Stored::Days => {
+                    number_of(date.to_datetime(Time::midnight()), None, column.stored)
+                }
+                Stored::Fresh | Stored::Text { .. } => era(written_date(date, column.dialect)),
+            };
+        }
+        Kind::Time { offset: keeps } => {
+            if date.is_some() && !now {
+                return Err(Refusal::TimeOnly);
+            }
+            if parts.offset.is_some() && !keeps {
+                return Err(Refusal::NoZone);
+            }
+            let postgres = match column.dialect {
+                Dialect::Postgres => true,
+                Dialect::MySql | Dialect::Sqlite => false,
+            };
+            if let Some(offset) = parts.offset
+                && postgres
+                && offset.seconds().abs() > PG_OFFSETS
+            {
+                return Err(Refusal::Range(Range::PgOffset));
+            }
+            let time = time.ok_or(UNREAD)?;
+            let day = Date::default();
+            let at = rounded(day.to_datetime(time), digits);
+            let digits = match column.stored {
+                Stored::Seconds => 0,
+                Stored::Days => digits.min(3),
+                Stored::Fresh | Stored::Text { .. } => digits,
+            };
+            reading.digits = digits;
+            if at.date() != day {
+                // Rounded up past the day's last moment: its end, which
+                // MySQL keeps as a span of a day.
+                reading.moment = match column.dialect {
+                    Dialect::MySql => Moment::Span("24:00:00".to_owned()),
+                    Dialect::Postgres | Dialect::Sqlite => Moment::EndOfDay,
+                };
+                reading.literal = "24:00:00".to_owned();
+            } else {
+                let time = at.time();
+                reading.moment = Moment::At {
+                    date: None,
+                    time: Some(time),
+                    offset: parts.offset,
+                };
+                reading.literal = match column.stored {
+                    // Seconds of the day; a Julian day of 1 January 2000,
+                    // the day SQLite's `julianday('09:30')` gives a time.
+                    Stored::Seconds | Stored::Days => {
+                        let (year, stored) = match column.stored {
+                            Stored::Days => (2000, Stored::Days),
+                            _ => (1970, Stored::Seconds),
+                        };
+                        let epoch = Date::new(year, 1, 1).unwrap_or_default();
+                        number_of(epoch.to_datetime(time), None, stored)
+                    }
+                    Stored::Fresh | Stored::Text { .. } => written_time(time, digits),
+                };
+            }
+            if let Some(offset) = parts.offset {
+                reading.literal.push_str(&written_offset(offset));
+            }
+        }
+        Kind::Timestamp => {
+            // SQLite keeps the text it is given, an offset too.
+            let keeps_any = match column.dialect {
+                Dialect::Sqlite => true,
+                Dialect::Postgres | Dialect::MySql => false,
+            };
+            if parts.offset.is_some() && !keeps_any {
+                return Err(Refusal::NoZone);
+            }
+            let date = date.unwrap_or_else(|| clock.today_in(&clock.zone));
+            let digits = match column.stored {
+                Stored::Seconds => 0,
+                Stored::Days => digits.min(3),
+                Stored::Fresh | Stored::Text { .. } => digits,
+            };
+            reading.digits = digits;
+            let at = rounded(date.to_datetime(time.unwrap_or(Time::midnight())), digits);
+            within(column, at, None)?;
+            reading.moment = Moment::At {
+                date: Some(at.date()),
+                time: Some(at.time()),
+                offset: parts.offset,
+            };
+            let separator = match column.stored {
+                Stored::Text { separator } => separator,
+                Stored::Fresh | Stored::Seconds | Stored::Days => ' ',
+            };
+            reading.literal = match column.stored {
+                Stored::Seconds | Stored::Days => number_of(at, parts.offset, column.stored),
+                Stored::Fresh | Stored::Text { .. } => {
+                    let (day, bc) = written_date(at.date(), column.dialect);
+                    let mut written =
+                        format!("{day}{separator}{}", written_time(at.time(), digits));
+                    if let Some(offset) = parts.offset {
+                        written.push_str(&written_offset(offset));
+                    }
+                    era((written, bc))
+                }
+            };
+        }
+        Kind::Zoned => zoned(&mut reading, column, clock, (date, time, parts.offset))?,
+    }
+    Ok(reading)
+}
+
+/// `written` with ` BC` after it where its year is before 1 AD.
+fn era((written, bc): (String, bool)) -> String {
+    if bc { format!("{written} BC") } else { written }
+}
+
+/// The furthest offset PostgreSQL's `timetz` keeps, in seconds: 15:59:59.
+const PG_OFFSETS: i32 = 15 * 3_600 + 59 * 60 + 59;
+
+/// The number SQLite keeps for `at` (read as UTC where no offset was
+/// typed): its seconds since 1970, or its Julian day.
+fn number_of(at: DateTime, offset: Option<Offset>, stored: Stored) -> String {
+    let Ok(moment) = offset.unwrap_or(Offset::UTC).to_timestamp(at) else {
+        return String::new();
+    };
+    match stored {
+        Stored::Days => (moment.as_millisecond() as f64 / MILLIS_A_DAY + JULIAN_EPOCH).to_string(),
+        Stored::Seconds | Stored::Fresh | Stored::Text { .. } => moment.as_second().to_string(),
+    }
+}
+
+/// A zoned value: the day and time in the column's zone, the offset the
+/// zone has then, and the moment's literal.
+fn zoned(
+    reading: &mut Reading,
+    column: &Column,
+    clock: &Clock,
+    (date, time, typed): (Option<Date>, Option<Time>, Option<Offset>),
+) -> Result<(), Refusal> {
+    let Some(zone) = column.zone.as_ref() else {
+        // A zone not known: the value as the server's clock writes it. What
+        // today is there is the server's to say.
+        if typed.is_some() || reading.relative || date.is_none() {
+            return Err(Refusal::ZoneUnknown);
+        }
+        let date = date.unwrap_or_else(|| clock.today_in(&clock.zone));
+        let at = rounded(
+            date.to_datetime(time.unwrap_or(Time::midnight())),
+            reading.digits,
+        );
+        within(column, at, None)?;
+        reading.moment = Moment::At {
+            date: Some(at.date()),
+            time: Some(at.time()),
+            offset: None,
+        };
+        let (day, bc) = written_date(at.date(), column.dialect);
+        reading.literal = era((
+            format!("{day} {}", written_time(at.time(), reading.digits)),
+            bc,
+        ));
+        return Ok(());
+    };
+    let date = date.unwrap_or_else(|| clock.today_in(zone));
+    let typed_at = rounded(
+        date.to_datetime(time.unwrap_or(Time::midnight())),
+        reading.digits,
+    );
+    let instant = match typed {
+        Some(offset) => offset.to_timestamp(typed_at).map_err(|_| Refusal::Years)?,
+        None => match zone.to_ambiguous_timestamp(typed_at).offset() {
+            AmbiguousOffset::Unambiguous { offset }
+            | AmbiguousOffset::Fold { before: offset, .. } => {
+                offset.to_timestamp(typed_at).map_err(|_| Refusal::Years)?
+            }
+            AmbiguousOffset::Gap { before, .. } => {
+                let moment = before.to_timestamp(typed_at).map_err(|_| Refusal::Years)?;
+                return Err(Refusal::Gap {
+                    at: typed_at,
+                    place: place(zone),
+                    offered: moment.to_zoned(zone.clone()).datetime(),
+                });
+            }
+        },
+    };
+    let shown = instant.to_zoned(zone.clone());
+    let at = shown.datetime();
+    if at.year() > 9999 {
+        return Err(Refusal::Years);
+    }
+    within(column, at, Some(instant))?;
+    let offset = shown.offset();
+    reading.moment = Moment::At {
+        date: Some(at.date()),
+        time: Some(at.time()),
+        offset: Some(offset),
+    };
+    reading.zone_name = Some(zone.to_offset_info(instant).abbreviation().to_owned());
+    // A MySQL literal has no offset to choose one of the two moments by:
+    // the server takes the one it takes, and nothing is offered.
+    reading.fold = match column.dialect {
+        Dialect::Postgres | Dialect::Sqlite => fold(zone, at),
+        Dialect::MySql => None,
+    };
+    let (day, bc) = written_date(at.date(), column.dialect);
+    let time = written_time(at.time(), reading.digits);
+    reading.literal = match column.dialect {
+        // The server reads a MySQL value in the session's zone, which is
+        // this one: it is sent as that zone's clock writes it.
+        Dialect::MySql => format!("{day} {time}"),
+        Dialect::Postgres | Dialect::Sqlite => {
+            era((format!("{day} {time}{}", written_offset(offset)), bc))
+        }
+    };
+    Ok(())
+}
+
+/// The text a value read is rewritten to where a choice of the editor's
+/// changes it (the time offered across a change of the clocks, one of the
+/// two a fold has): its day and time as the zone's clock shows them, and
+/// the offset where one is chosen.
+pub fn rewritten(at: DateTime, offset: Option<Offset>, column: &Column) -> String {
+    let (day, bc) = written_date(at.date(), column.dialect);
+    let mut time = format!("{:02}:{:02}", at.hour(), at.minute());
+    if at.second() != 0 || at.subsec_nanosecond() != 0 {
+        time = written_time(at.time(), fraction_digits(at.time()));
+    }
+    let mut written = format!("{day} {time}");
+    if let Some(offset) = offset {
+        written.push_str(&written_offset(offset));
+    }
+    era((written, bc))
+}
+
+/// The digits a time's fraction needs, its trailing zeros left off.
+fn fraction_digits(time: Time) -> u8 {
+    let nanos = time.subsec_nanosecond();
+    if nanos == 0 {
+        return 0;
+    }
+    let zeros = (0..9)
+        .take_while(|at| nanos % 10_i32.pow(at + 1) == 0)
+        .count();
+    9 - zeros as u8
+}
+
+/// The text an editor opens on for a SQLite value kept as a number
+/// (seconds since 1970, or a Julian day): the day, its time, or both, as
+/// it is in UTC. `None` for any other.
+pub fn opened(value: &Value, column: &Column) -> Option<String> {
+    let number = match (value, column.stored) {
+        (Value::Int(seconds), Stored::Seconds) => seconds.to_string(),
+        (Value::Float(days), Stored::Days) => days.to_string(),
+        (Value::Int(days), Stored::Days) => days.to_string(),
+        _ => return None,
+    };
+    let at = numbered(&number, column.stored)?
+        .ok()?
+        .to_zoned(TimeZone::UTC)
+        .datetime();
+    let (day, _) = written_date(at.date(), column.dialect);
+    let time = written_time(at.time(), fraction_digits(at.time()));
+    Some(match column.kind {
+        Kind::Date => day,
+        Kind::Time { .. } => time,
+        Kind::Timestamp | Kind::Zoned => format!("{day} {time}"),
+    })
+}
 
 #[cfg(test)]
 mod tests {
```


- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --lib dates::`
Expected: clean, and `test result: ok. 27 passed; 0 failed`

- [ ] **Step 5: Commit**

```bash
git add src/dates.rs src/lib.rs
git commit -m "Read typed dates and times for each engine's types"
```

---

### Task 2: The editor reads what is typed, and the line says what is stored

**Files:**
- Create: `src/ui/date_line.rs`, `src/ui/date_tests.rs`
- Modify: `src/edit.rs`, `src/app.rs`, `src/app/editing.rs`, `src/ui/format.rs`, `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/row_form.rs`, `src/ui/mod.rs`, `src/ui/zone_tests.rs`

The app keeps the clock a reading needs (`App::date_clock`: the time it is or, in a test, `stopped_clock`; the settings' zone; the locale's `date_order`, found at the start). `Table::dates` is a page's column as `dates` reads it; `edit::moment` reads a text typed into it, but for a keyword whose value the database makes (`now()`, NULL, DEFAULT). The editor's reading is made with its problem: at open and on each keystroke. A commit stores the literal of the reading of its text, or reads the text again where none is kept. The line's words are `date_line::line`; `date_line::under` draws them under the cell or the field, where no foreign key's list hangs, and a click on an offer rewrites the text. Where the line can be drawn (a date column with no foreign key's list up), the note that would say what a text fails under the field is not drawn; a cell scrolled out of sight shows neither until it is back. A problem that warns (`Problem::warns`: a gap, MySQL's zero date) does not make the field's ring red.

`format::with_offset` and `edit::keeps_offset` go: `dates` sends a zoned value with its offset now.

- [ ] **Step 1: Write the failing tests**

The behaviour tests, a new file, declared in `src/ui/mod.rs`, with the test the zone's file changes. The line's own unit tests, at the end of the new `src/ui/date_line.rs`, come with it in Step 3. A click away is `Action::LeaveEdit`, which the grid sends for one; a press on the popover keeping the edit open is what **Use 03:30** and `01:30 EST` do in their tests.

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -18975,8 +18975,10 @@
             type_into(&mut harness, tab, id, at(row, 4), "DEFAULT");
             let edits = &object(&harness, tab, id).edits;
             assert!(!edits.cells.contains_key(&(row, 4)));
-            // The time of the save goes out in the row's INSERT.
-            type_into(&mut harness, tab, id, at(row, 4), "now");
+            // The time of the save goes out in the row's INSERT. `now`
+            // alone is the time by the clock, as the line under the editor
+            // says: the database's is `now()`.
+            type_into(&mut harness, tab, id, at(row, 4), "now()");
             type_into(&mut harness, tab, id, at(row, 1), "9100000000000000004");
             let before = harness.app.backend.sent.len();
             harness.app.apply(Action::WriteEdits { tab, id });
@@ -21163,6 +21165,7 @@
                         top: false,
                         touched: true,
                         problem: None,
+                        moment: None,
                         picker: None,
                         select: false,
                     });
```

`src/ui/cell_editor.rs`:

```diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -1797,6 +1797,7 @@
             top: false,
             touched: true,
             problem: None,
+            moment: None,
             picker: None,
             select: false,
         };
@@ -1808,6 +1809,7 @@
             json: false,
             hold: true,
             now: false,
+            dates: None,
             keys: Keys::of(&Keymap::default(), Layout::of(look)),
             search: search.map(|(count, refused)| Search {
                 table: "publishers".into(),
```

`src/ui/date_tests.rs`:

```diff
diff --git a/src/ui/date_tests.rs b/src/ui/date_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/date_tests.rs
@@ -0,0 +1,603 @@
+//! The line under the editor of a date or a time: which columns have it,
+//! what it says, what a commit stores, and the choices a change of the
+//! clocks offers. On the Bookshop's `book_covers.created_at`, its type
+//! changed for each test, with the clock stopped on Saturday 24 October
+//! 2026 at 10:00 in New York.
+//!
+//! Only what is said and what is stored are checked. No colour, size or
+//! picture of the design is.
+
+use egui::accesskit::Role;
+use egui::{Key, Modifiers};
+use tabletist_db::{Driver, NewValue, Value, ValueKind};
+
+use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
+use crate::settings::{Settings, Zone};
+use crate::testing::{Harness, labels};
+
+/// `created_at`, by its place in the page.
+const MADE: usize = 4;
+
+/// Saturday 24 October 2026, 10:00 in New York.
+fn saturday() -> jiff::Timestamp {
+    jiff::civil::date(2026, 10, 24)
+        .at(10, 0, 0, 0)
+        .to_zoned(jiff::tz::TimeZone::get("America/New_York").unwrap())
+        .unwrap()
+        .timestamp()
+}
+
+fn in_zone(harness: &mut Harness, name: &str) {
+    let settings = Settings {
+        tz: Zone::Named(name.into()),
+        ..harness.app.settings.clone()
+    };
+    harness.app.apply_settings(settings);
+    harness.settle();
+}
+
+/// The covers' tab on `driver`, `created_at` of the type `type_name` and
+/// holding `values` (the first row's first), shown in New York, with the
+/// cursor on the first row's `created_at`.
+fn covers_with(driver: Driver, type_name: &str, values: &[Value]) -> (Harness, ConnTabId, TabId) {
+    let mut harness = Harness::new();
+    harness.app.stopped_clock = Some(saturday());
+    let tab = harness.connect_fake_as(false);
+    harness.app.workspace_mut(tab).unwrap().driver = driver;
+    harness.app.apply(Action::OpenObject {
+        tab,
+        object: tabletist_db::ObjectRef::new("main", "book_covers"),
+        kind: tabletist_db::ObjectKind::Table,
+        pin: true,
+    });
+    let mut structure = crate::testing::book_covers_structure();
+    structure.columns[MADE].type_name = type_name.into();
+    harness.answer_structure(structure);
+    let mut page = crate::testing::book_covers_page(values.len().max(1));
+    page.columns[MADE].type_name = type_name.into();
+    page.columns[MADE].kind = ValueKind::Temporal;
+    for (row, value) in page.rows.iter_mut().zip(values) {
+        row[MADE] = value.clone();
+    }
+    harness.answer_rows(page);
+    let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
+    harness.app.workspace_mut(tab).unwrap().pane = Pane::Grid;
+    in_zone(&mut harness, "America/New_York");
+    harness.app.apply(Action::SelectCell {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+    });
+    harness.settle();
+    (harness, tab, id)
+}
+
+/// [`covers_with`], `created_at` a foreign key to the Bookshop's
+/// `orders.placed_at`.
+fn keyed_covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
+    let (mut harness, tab, id) = covers(driver, type_name, value);
+    let object = harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .object_tab_mut(id)
+        .unwrap();
+    let structure = object.structure.value.as_mut().unwrap();
+    structure.foreign_keys = vec![tabletist_db::ForeignKeyInfo {
+        columns: vec!["created_at".into()],
+        ref_schema: "main".into(),
+        ref_table: "orders".into(),
+        ref_columns: vec!["placed_at".into()],
+        ..tabletist_db::ForeignKeyInfo::default()
+    }];
+    harness.settle();
+    (harness, tab, id)
+}
+
+fn covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
+    covers_with(driver, type_name, &[Value::Text(value.into())])
+}
+
+fn open(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+    harness.app.apply(Action::EditCell {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+        start: EditStart::Value,
+    });
+    harness.settle();
+}
+
+/// Opens the editor and types `text` in the place of its own.
+fn type_in(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
+    open(harness, tab, id);
+    editor_mut(harness, tab, id).text = text.to_owned();
+    harness.app.apply(Action::EditorTyped { tab, id });
+    harness.settle();
+}
+
+fn editor_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut crate::edit::Editor {
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let object = workspace.object_tab_mut(id).unwrap();
+    object.edits.editor.as_mut().expect("an editor")
+}
+
+fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
+    let object = harness.app.workspace(tab)?.object_tab(id)?;
+    Some(object.edits.editor.as_ref()?.text.clone())
+}
+
+/// The cell's pending value, where it has one.
+fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<NewValue> {
+    let object = harness.app.workspace(tab)?.object_tab(id)?;
+    Some(object.edits.cells.get(&(0, MADE))?.new.clone())
+}
+
+fn pending_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
+    match pending(harness, tab, id)? {
+        NewValue::Text(text) => Some(text),
+        other => panic!("not a text: {other:?}"),
+    }
+}
+
+fn says(harness: &mut Harness, words: &str) -> bool {
+    harness.has(words)
+}
+
+#[test]
+fn each_kind_of_date_or_time_column_has_the_line_and_no_other_does() {
+    use Driver::{MySql, Postgres, Sqlite};
+    for (driver, type_name, value, named) in [
+        (Postgres, "date", "2026-01-15", Some("created_at, date")),
+        (
+            Postgres,
+            "timestamp without time zone",
+            "2026-01-15 09:30:00",
+            Some("created_at, date and time"),
+        ),
+        (
+            Postgres,
+            "timestamp with time zone",
+            "2026-01-15 14:30:00+00",
+            Some("created_at, date and time"),
+        ),
+        (
+            Postgres,
+            "time without time zone",
+            "09:30:00",
+            Some("created_at, time"),
+        ),
+        (
+            Postgres,
+            "time with time zone",
+            "09:30:00-05",
+            Some("created_at, time"),
+        ),
+        (MySql, "date", "2026-01-15", Some("created_at, date")),
+        (
+            MySql,
+            "datetime",
+            "2026-01-15 09:30:00",
+            Some("created_at, date and time"),
+        ),
+        (
+            MySql,
+            "timestamp",
+            "2026-01-15 09:30:00",
+            Some("created_at, date and time"),
+        ),
+        (MySql, "time", "09:30:00", Some("created_at, time")),
+        (Sqlite, "DATE", "2026-01-15", Some("created_at, date")),
+        (
+            Sqlite,
+            "DATETIME",
+            "2026-01-15 09:30:00",
+            Some("created_at, date and time"),
+        ),
+        (Sqlite, "TIME", "09:30:00", Some("created_at, time")),
+        (Postgres, "interval", "1 day", None),
+        (Postgres, "date[]", "{2026-01-15}", None),
+        (MySql, "year", "2026", None),
+    ] {
+        let (mut harness, tab, id) = covers(driver, type_name, value);
+        open(&mut harness, tab, id);
+        let tree = harness.settle();
+        let dialog = tree
+            .nodes
+            .iter()
+            .filter(|(_, node)| node.role() == Role::Dialog)
+            .find_map(|(_, node)| {
+                node.label()
+                    .filter(|label| label.starts_with("created_at, "))
+            });
+        assert_eq!(dialog, named, "{driver:?} {type_name}");
+    }
+}
+
+#[test]
+fn the_line_says_what_a_commit_stores() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    open(&mut harness, tab, id);
+    // Opened on the value as the cell shows it, in New York, with its
+    // offset: what is sent.
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-15 09:30:00-05")
+    );
+    assert!(says(&mut harness, "Thu 15 Jan 2026, 09:30 EST"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    type_in(&mut harness, tab, id, "tomorrow 9am");
+    let said = "Sun 25 Oct 2026, 09:00 EDT · tomorrow · sent as 2026-10-25 09:00:00-04";
+    assert!(says(&mut harness, said), "{:?}", labels(&harness.settle()));
+    // The clock moves on before the commit: what is stored is what was said.
+    harness.app.stopped_clock = Some(saturday() + jiff::SignedDuration::from_hours(15));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-25 09:00:00-04")
+    );
+}
+
+#[test]
+fn now_alone_is_the_time_by_the_clock_and_now_with_brackets_the_databases() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    type_in(&mut harness, tab, id, "now");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-24 10:00:00-04")
+    );
+    type_in(&mut harness, tab, id, "now()");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), Some(NewValue::Now));
+}
+
+#[test]
+fn a_text_that_is_no_date_is_refused_and_esc_keeps_the_cell() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    type_in(&mut harness, tab, id, "2026-02-30");
+    assert!(says(&mut harness, "There is no 30 February."));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-02-30")
+    );
+    assert_eq!(pending(&harness, tab, id), None);
+    harness.press(Key::Escape, Modifiers::NONE);
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert_eq!(pending(&harness, tab, id), None);
+    let (mut harness, tab, id) = covers(Driver::Postgres, "time", "09:30:00");
+    type_in(&mut harness, tab, id, "25:00");
+    assert!(says(&mut harness, "25:00 is not a time of day."));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(editor_text(&harness, tab, id).is_some());
+}
+
+#[test]
+fn enter_applies_and_a_click_away_applies_too() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    type_in(&mut harness, tab, id, "15 Feb 2026");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-02-15")
+    );
+    type_in(&mut harness, tab, id, "16 Feb 2026");
+    harness.app.apply(Action::LeaveEdit {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+        place: crate::edit::EditorPlace::Grid,
+    });
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-02-16")
+    );
+    // The day it was, written another way, is no change.
+    type_in(&mut harness, tab, id, "Jan 15 2026");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+}
+
+#[test]
+fn a_time_the_clocks_skip_is_refused_until_the_time_after_is_taken() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    type_in(&mut harness, tab, id, "2026-03-08 02:30");
+    assert!(says(
+        &mut harness,
+        "02:30 doesn't exist on 8 Mar in New York; clocks go forward."
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    harness.click("Use 03:30");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-03-08 03:30")
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-03-08 03:30:00-04")
+    );
+}
+
+#[test]
+fn a_time_the_clocks_go_back_over_takes_summer_time_unless_the_other_is_chosen() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    type_in(&mut harness, tab, id, "2026-11-01 01:30");
+    assert!(says(
+        &mut harness,
+        "01:30 happens twice on 1 Nov in New York; clocks go back. · sent as 2026-11-01 01:30:00-04"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30:00-04")
+    );
+    type_in(&mut harness, tab, id, "2026-11-01 01:30");
+    harness.click("01:30 EST");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30-05")
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30:00-05")
+    );
+}
+
+#[test]
+fn a_timestamp_without_a_zone_is_stored_as_typed_whatever_the_zone() {
+    for zone in ["America/New_York", "Asia/Kolkata"] {
+        let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:00");
+        in_zone(&mut harness, zone);
+        type_in(&mut harness, tab, id, "2026-03-08 02:30");
+        assert!(says(
+            &mut harness,
+            "Sun 8 Mar 2026, 02:30 · no zone · sent as 2026-03-08 02:30:00"
+        ));
+        harness.press(Key::Enter, Modifiers::NONE);
+        assert_eq!(
+            pending_text(&harness, tab, id).as_deref(),
+            Some("2026-03-08 02:30:00"),
+            "{zone}"
+        );
+    }
+}
+
+#[test]
+fn mysql_refuses_what_its_types_do_not_hold() {
+    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
+    type_in(&mut harness, tab, id, "2040-05-01 08:00");
+    assert!(says(
+        &mut harness,
+        "MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    let (mut harness, tab, id) = covers(Driver::MySql, "date", "2026-01-15");
+    type_in(&mut harness, tab, id, "0999-12-31");
+    assert!(says(
+        &mut harness,
+        "MySQL DATE holds 1000-01-01 to 9999-12-31."
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+}
+
+#[test]
+fn a_mysql_zero_date_is_said_and_kept_until_another_is_typed() {
+    let (mut harness, tab, id) = covers(Driver::MySql, "date", "0000-00-00");
+    open(&mut harness, tab, id);
+    assert!(says(
+        &mut harness,
+        "MySQL zero date · pick a date or set NULL"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert_eq!(pending(&harness, tab, id), None);
+}
+
+#[test]
+fn infinity_and_days_before_christ_go_back_as_they_came() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "infinity");
+    open(&mut harness, tab, id);
+    assert!(says(&mut harness, "No end date · stored as infinity"));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    type_in(&mut harness, tab, id, "-infinity");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("-infinity")
+    );
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "0044-03-15 BC");
+    type_in(&mut harness, tab, id, "0044-03-15 BC");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    type_in(&mut harness, tab, id, "0044-03-16 bc");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("0044-03-16 BC")
+    );
+}
+
+#[test]
+fn sqlite_writes_seconds_where_its_values_are_seconds_and_text_elsewhere() {
+    // 24 and 23 October 2026, at midnight UTC.
+    let seconds = [Value::Int(1_792_800_000), Value::Int(1_792_713_600)];
+    let (mut harness, tab, id) = covers_with(Driver::Sqlite, "DATE", &seconds);
+    open(&mut harness, tab, id);
+    // Opened on the day the seconds are: closed untouched, no change.
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-10-24")
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    type_in(&mut harness, tab, id, "25 Oct 2026");
+    assert!(says(
+        &mut harness,
+        "Sun 25 Oct 2026 · tomorrow · stored as seconds since 1970, like the other rows · sent as 1792886400"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("1792886400")
+    );
+    // The values nobody edited keep theirs.
+    let object = harness.app.workspace(tab).unwrap().object_tab(id).unwrap();
+    assert_eq!(object.edits.cells.len(), 1);
+    let text = [Value::Text("2026-10-24".into())];
+    let (mut harness, tab, id) = covers_with(Driver::Sqlite, "DATE", &text);
+    type_in(&mut harness, tab, id, "25 Oct 2026");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-25")
+    );
+}
+
+#[test]
+fn null_and_default_are_typed_only_where_the_column_takes_them() {
+    // `created_at` has a default, and takes no NULL.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    type_in(&mut harness, tab, id, "DEFAULT");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(matches!(
+        pending(&harness, tab, id),
+        Some(NewValue::Default { .. })
+    ));
+    type_in(&mut harness, tab, id, "NULL");
+    assert!(says(
+        &mut harness,
+        "Not a date. Try 2026-01-15, 15 Jan 2026 or tomorrow."
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(editor_text(&harness, tab, id).is_some());
+}
+
+#[test]
+fn the_row_panel_has_the_line_under_its_field() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    harness.app.workspace_mut(tab).unwrap().row_panel = true;
+    harness.settle();
+    harness.app.apply(Action::EditField {
+        tab,
+        id,
+        cell: CellPos { row: 0, col: MADE },
+        start: EditStart::Value,
+    });
+    harness.settle();
+    editor_mut(&mut harness, tab, id).text = "+6d".into();
+    harness.app.apply(Action::EditorTyped { tab, id });
+    assert!(says(
+        &mut harness,
+        "Fri 30 Oct 2026 · in 6 days · sent as 2026-10-30"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-30")
+    );
+}
+
+#[test]
+fn the_terminal_look_heads_its_float_with_the_column_and_the_zone() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    harness.set_look(crate::theme::Look::omarchy());
+    type_in(&mut harness, tab, id, "2026-02-30 09:30");
+    assert!(says(&mut harness, "! there is no 30 february."));
+    assert!(harness.painted_color("america/new_york").is_some());
+}
+
+#[test]
+fn a_date_typed_back_as_another_writing_of_it_is_no_change() {
+    // BC, with the offset the server wrote: the editor opens on it as
+    // sent, and the zone shows it with New York's own clock then.
+    let (mut harness, tab, id) =
+        covers(Driver::Postgres, "timestamptz", "0044-03-15 14:52:04+00 BC");
+    type_in(&mut harness, tab, id, "0044-03-15 14:52:04+00 BC");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    // A fraction of zeros, or one with more of them than the server wrote.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:12.25");
+    type_in(&mut harness, tab, id, "2026-01-15 09:30:12.250");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp", "2026-01-15 09:30:00");
+    type_in(&mut harness, tab, id, "2026-01-15 09:30:00.000");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(pending(&harness, tab, id), None);
+}
+
+#[test]
+fn a_date_that_is_a_foreign_key_says_why_its_text_is_refused() {
+    let (mut harness, tab, id) = keyed_covers(Driver::Postgres, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    // The picker asks for the columns of the table the key points at.
+    let column = |name: &str, type_name: &str| tabletist_db::ColumnInfo {
+        name: name.into(),
+        type_name: type_name.into(),
+        ..tabletist_db::ColumnInfo::default()
+    };
+    harness.answer_structure(tabletist_db::Structure {
+        columns: vec![column("id", "integer"), column("placed_at", "date")],
+        primary_key: vec!["id".into()],
+        ..tabletist_db::Structure::default()
+    });
+    harness.answer_lookups(&[]);
+    editor_mut(&mut harness, tab, id).text = "2026-02-30".into();
+    harness.app.apply(Action::EditorTyped { tab, id });
+    harness.settle();
+    // The text is a search until it is taken: no order holds that day,
+    // and Enter takes the text.
+    harness.answer_lookups(&[]);
+    harness.settle();
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert!(editor_text(&harness, tab, id).is_some());
+    assert!(
+        says(&mut harness, "There is no 30 February."),
+        "{:?}",
+        labels(&harness.settle())
+    );
+}
+
+#[test]
+fn a_timestamptz_whose_zone_is_not_read_yet_is_sent_in_utc() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    // A named zone the database does not know: values are shown as sent.
+    in_zone(&mut harness, "Mars/Olympus");
+    type_in(&mut harness, tab, id, "2026-01-15 09:30");
+    assert!(says(
+        &mut harness,
+        "Thu 15 Jan 2026, 09:30 UTC · sent as 2026-01-15 09:30:00+00"
+    ));
+    type_in(&mut harness, tab, id, "tomorrow 9am");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-25 09:00:00+00")
+    );
+}
+
+#[test]
+fn a_time_the_clocks_skip_is_a_warning_and_no_day_is_a_fault() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    type_in(&mut harness, tab, id, "2026-03-08 02:30");
+    let problem = editor_mut(&mut harness, tab, id)
+        .problem
+        .clone()
+        .expect("refused");
+    assert!(problem.warns());
+    type_in(&mut harness, tab, id, "2026-02-30 02:30");
+    let problem = editor_mut(&mut harness, tab, id)
+        .problem
+        .clone()
+        .expect("refused");
+    assert!(!problem.warns());
+}
```

`src/ui/format.rs`:

```diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -1255,50 +1255,6 @@
     }
 
     #[test]
-    fn a_date_and_time_typed_with_no_offset_gets_the_zones() {
-        let zone = new_york();
-        assert_eq!(
-            with_offset("2026-01-12 04:14:03", &zone).as_deref(),
-            Some("2026-01-12 04:14:03-05")
-        );
-        assert_eq!(
-            with_offset(" 2026-07-01 08:00 ", &zone).as_deref(),
-            Some("2026-07-01 08:00-04")
-        );
-        assert_eq!(
-            with_offset("2026-01-12T04:14:03.5", &zone).as_deref(),
-            Some("2026-01-12T04:14:03.5-05")
-        );
-        // A date alone is its midnight there.
-        assert_eq!(
-            with_offset("2026-01-12", &zone).as_deref(),
-            Some("2026-01-12 00:00:00-05")
-        );
-        // The hour the clocks skip, and the one they have twice, take the
-        // offset from before the change.
-        assert_eq!(
-            with_offset("2026-03-08 02:30:00", &zone).as_deref(),
-            Some("2026-03-08 02:30:00-05")
-        );
-        assert_eq!(
-            with_offset("2026-11-01 01:30:00", &zone).as_deref(),
-            Some("2026-11-01 01:30:00-04")
-        );
-        // Typed with an offset, or as no date: as typed.
-        for typed in [
-            "2026-01-12 04:14:03+02",
-            "2026-01-12T04:14:03Z",
-            "soon",
-            "2026-01-12T04:14[America/New_York]",
-            "-002026-01-12",
-            "+002026-01-12",
-            "04:14",
-        ] {
-            assert_eq!(with_offset(typed, &zone), None, "{typed}");
-        }
-    }
-
-    #[test]
     fn what_is_no_moment_with_an_offset_is_left_as_it_is() {
         let zone = new_york();
         for text in [
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -23,6 +23,8 @@
 pub mod conflict_prompt;
 pub mod connect_dialog;
 pub mod data_view;
+#[cfg(test)]
+mod date_tests;
 #[cfg(test)]
 mod delete_many_tests;
 #[cfg(test)]
```

`src/ui/zone_tests.rs`:

```diff
diff --git a/src/ui/zone_tests.rs b/src/ui/zone_tests.rs
--- a/src/ui/zone_tests.rs
+++ b/src/ui/zone_tests.rs
@@ -291,12 +291,13 @@
         pending(&harness, tab, id).as_deref(),
         Some("2026-01-12 06:00:00-05")
     );
-    // One typed with an offset is sent as typed.
+    // One typed with another offset is the same moment, sent as the zone
+    // shows it.
     open_editor(&mut harness, tab, id);
     commit(&mut harness, tab, id, "2026-01-12 06:00:00+01");
     assert_eq!(
         pending(&harness, tab, id).as_deref(),
-        Some("2026-01-12 06:00:00+01")
+        Some("2026-01-12 00:00:00-05")
     );
 }
 
```


- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib`
Expected: no build, with errors such as:

```
error[E0609]: no field `stopped_clock` on type `app::App`
error[E0560]: struct `edit::Editor` has no field named `moment`
error[E0560]: struct `cell_editor::Target` has no field named `dates`
error[E0599]: no method named `warns` found for enum `edit::Problem` in the current scope
error: could not compile `tabletist` (lib test) due to 7 previous errors
```

- [ ] **Step 3: Write the code**

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -100,6 +100,12 @@
     /// The computer's time zone as the app found it at its start: what the
     /// System zone of the settings is.
     pub system_zone: jiff::tz::TimeZone,
+    /// The order the person writes a day's numbers in (`15/01/2026`), by
+    /// the region of the system's locale, as the app found it at its start.
+    pub date_order: crate::dates::Order,
+    /// A time the clock is stopped at, for a test: `None` reads the
+    /// system's clock. What `today` and `now` typed into a date are.
+    pub stopped_clock: Option<jiff::Timestamp>,
     /// The time zone database, once the backend has read it
     /// (`Command::LoadZones`): the zone menu's names, and each zone.
     pub zones: Option<std::sync::Arc<crate::zones::Zones>>,
@@ -322,6 +328,8 @@
             settings,
             settings_file,
             system_zone: system_zone(),
+            date_order: date_order(),
+            stopped_clock: None,
             named_zone,
             zones: None,
             locale: Locale::default(),
@@ -701,6 +709,17 @@
             .resolve(&self.system_zone, self.zones.as_deref())
     }
 
+    /// What a date typed into an editor is read at: the time it is, the
+    /// zone of the settings (the system's until a named one is read), and
+    /// the order the person writes a day's numbers in.
+    pub fn date_clock(&self) -> crate::dates::Clock {
+        crate::dates::Clock {
+            now: self.stopped_clock.unwrap_or_else(jiff::Timestamp::now),
+            zone: self.zone().unwrap_or_else(|| self.system_zone.clone()),
+            order: self.date_order,
+        }
+    }
+
     /// Gives every workspace the zone of the settings, and has the row
     /// panels' texts made again in it.
     fn refresh_zone(&mut self) {
@@ -1586,10 +1605,11 @@
                 }
             }
             Action::EditorTyped { tab, id } => {
-                let problem = self.editor_problem(tab, id);
+                let (problem, moment) = self.editor_problem(tab, id);
                 let delay = self.lookup_delay;
                 if let Some(editor) = self.editor_mut(tab, id) {
                     editor.problem = problem;
+                    editor.moment = moment;
                     editor.touched = true;
                     // The row panel's field wraps, and Enter commits it: a
                     // line break can only be pasted into it. Its text is
@@ -5620,6 +5640,24 @@
     }
 }
 
+/// The order the system's locale writes a day's numbers in, by its region:
+/// that of the locale for times where one is set (`LC_ALL`, `LC_TIME`),
+/// else of the first language the system prefers that names one. Day
+/// first in a test, whatever the machine's.
+fn date_order() -> crate::dates::Order {
+    if cfg!(test) {
+        return crate::dates::Order::DayMonth;
+    }
+    let times = ["LC_ALL", "LC_TIME"]
+        .into_iter()
+        .filter_map(|name| std::env::var(name).ok());
+    let region = times
+        .chain(fastframe_i18n::system_languages().iter().cloned())
+        .filter_map(|tag| fastframe_i18n::LanguageTag::parse(&tag))
+        .find_map(|tag| tag.region);
+    crate::dates::Order::of_region(region.as_deref())
+}
+
 /// The computer's time zone, or UTC where it cannot be found. Tests have
 /// UTC whatever the computer they run on says, so what they show does not
 /// depend on it: a test of the System zone sets `App::system_zone`.
@@ -7453,7 +7491,8 @@
                 None => {
                     workspace.status = SessionStatus::Idle;
                     // The version it was asked for, where it had not
-                    // answered, is asked of the next session.
+                    // answered, is asked of the next session, and so is
+                    // its zone.
                     workspace.server_version.pending = None;
                     due.push(workspace.session);
                 }
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -6,6 +6,7 @@
 
 use super::App;
 use crate::backend::{Command, RequestId, SessionId};
+use crate::dates::{Reading, Refusal};
 use crate::edit::{
     Answer, Duplicate, Editor, EditorPlace, Effect, Holder, Lock, NEW_ROWS, Note, Order, Pending,
     Place, Problem, Saved, Saving, Sent, State, Sweep, Table, Taken, Told, change_set, check,
@@ -144,7 +145,7 @@
 
 /// What closing the tab's editor makes of its text: `None` where no editor
 /// is open, it was not typed into, or its cell is no longer on the page.
-fn typed(table: &Table<'_>, object: &ObjectTab) -> Option<Typed> {
+fn typed(table: &Table<'_>, object: &ObjectTab, clock: &crate::dates::Clock) -> Option<Typed> {
     let editor = object.edits.editor.as_ref()?;
     let cell = editor.cell;
     let column = table.column(cell.col)?;
@@ -161,14 +162,29 @@
     }
     // What was typed, as the column reads it: a keyword where it is one.
     let mut new = crate::edit::typed_as(table.dialect, column, &editor.text);
-    // A moment typed with no offset is the zone's, where the column keeps
-    // one: the server would read it in its own.
-    if let NewValue::Text(text) = &new
-        && crate::edit::keeps_offset(table.dialect, &column.type_name)
-        && let Some(zone) = table.zone
-        && let Some(sent) = crate::ui::format::with_offset(text, zone)
-    {
-        new = NewValue::Text(sent);
+    // A date or a time is stored as the line under its editor said: the
+    // literal of what the text read as when it was typed (a `now` is that
+    // moment, not this one), or what it is read as now where the editor
+    // kept no reading of this text.
+    let moment = match &editor.moment {
+        Some(Ok(reading)) if reading.typed == editor.text => Some(Ok(reading.clone())),
+        _ => crate::edit::moment(table, cell.col, &editor.text, clock),
+    };
+    let mut refused = None;
+    // A day or a time typed back as the cell has it, however each is
+    // written (`.250` for `.25`, another zone, BC after an offset), is
+    // the moment that was loaded.
+    let mut same = false;
+    match moment {
+        Some(Ok(reading)) => {
+            same = loaded.is_some_and(|loaded| {
+                let was = crate::edit::moment(table, cell.col, &start_text(loaded, class), clock);
+                matches!(was, Some(Ok(was)) if was.moment == reading.moment)
+            });
+            new = NewValue::Text(reading.literal);
+        }
+        Some(Err(refusal)) => refused = Some(Problem::Moment(refusal)),
+        None => {}
     }
     // A moment typed back as it was, in the zone it is shown in or in any
     // other, is the moment that was loaded: in a column of dates and
@@ -178,15 +194,18 @@
     let changed = match (&new, loaded) {
         // A new row's unset cell is its default already.
         (NewValue::Default { .. }, None) => false,
-        (new, loaded) => loaded.is_none_or(|loaded| {
-            is_change(loaded, new, class)
-                && !(temporal
-                    && crate::edit::shows_in_zone(table.dialect)
-                    && crate::edit::same_moment_typed(loaded, new))
-        }),
+        (new, loaded) => {
+            !same
+                && loaded.is_none_or(|loaded| {
+                    is_change(loaded, new, class)
+                        && !(temporal
+                            && crate::edit::shows_in_zone(table.dialect)
+                            && crate::edit::same_moment_typed(loaded, new))
+                })
+        }
     };
     let problem = changed
-        .then(|| crate::edit::problem(table.dialect, column, &editor.text))
+        .then(|| refused.or_else(|| crate::edit::problem(table.dialect, column, &editor.text)))
         .flatten();
     Some(Typed {
         cell,
@@ -353,8 +372,9 @@
         };
         // On a row of the page only: what is typed into a new row is part
         // of that row, which is counted as one whatever it holds.
+        let clock = self.date_clock();
         let closed = self
-            .table(tab, id, typed)
+            .table(tab, id, |table, object| typed(table, object, &clock))
             .flatten()
             .filter(|typed| new_id(typed.cell.row).is_none());
         let pending = closed
@@ -736,14 +756,30 @@
         self.object_tab_mut(tab, id)?.edits.editor.as_mut()
     }
 
-    /// What the open editor's text fails, if anything.
-    pub(super) fn editor_problem(&self, tab: ConnTabId, id: TabId) -> Option<Problem> {
+    /// What the open editor's text fails, if anything, and what it reads
+    /// as where its column keeps dates or times.
+    pub(super) fn editor_problem(
+        &self,
+        tab: ConnTabId,
+        id: TabId,
+    ) -> (Option<Problem>, Option<Result<Reading, Refusal>>) {
+        let clock = self.date_clock();
         self.table(tab, id, |table, object| {
-            let editor = object.edits.editor.as_ref()?;
-            let column = table.column(editor.cell.col)?;
-            crate::edit::problem(table.dialect, column, &editor.text)
+            let Some(editor) = object.edits.editor.as_ref() else {
+                return (None, None);
+            };
+            let col = editor.cell.col;
+            let moment = crate::edit::moment(table, col, &editor.text, &clock);
+            let problem = match &moment {
+                Some(Err(refusal)) => Some(Problem::Moment(refusal.clone())),
+                Some(Ok(_)) => None,
+                None => table
+                    .column(col)
+                    .and_then(|column| crate::edit::problem(table.dialect, column, &editor.text)),
+            };
+            (problem, moment)
         })
-        .flatten()
+        .unwrap_or_default()
     }
 
     /// The column of the tab's open editor, when the row panel draws it.
@@ -842,6 +878,7 @@
         };
         // Opened from its value, to be read before it is changed.
         let from_value = matches!(start, EditStart::Value);
+        let clock = self.date_clock();
         let opened = self.table(tab, id, |table, object| {
             if let Some(lock) = table.lock(cell) {
                 return Err(lock);
@@ -867,10 +904,16 @@
                     // A new row's cell starts from nothing.
                     // A moment with an offset as the cell shows it: in
                     // the zone, and edited there, where the engine sends
-                    // one.
+                    // one. SQLite's seconds since 1970 as the day they are.
                     None => table
                         .loaded(cell)
                         .map(|loaded| {
+                            let seconds = table
+                                .dates(cell.col)
+                                .and_then(|dates| crate::dates::opened(loaded, &dates));
+                            if let Some(day) = seconds {
+                                return day;
+                            }
                             let start = start_text(loaded, class);
                             let temporal =
                                 table.page.columns.get(cell.col).map(|column| column.kind)
@@ -893,10 +936,19 @@
             // character typed on a cell that does not take it. Not the
             // empty text a NULL opens with, or an edit from nothing: that
             // one nobody typed.
-            let problem = table
-                .column(cell.col)
-                .filter(|_| kept || !text.is_empty())
-                .and_then(|column| crate::edit::problem(table.dialect, column, &text));
+            let typed = kept || !text.is_empty();
+            let moment = crate::edit::moment(table, cell.col, &text, &clock);
+            let problem = match &moment {
+                // MySQL's zero date, as the cell holds it: said, and
+                // refused only once it is typed.
+                Some(Err(Refusal::Zero)) if !kept => None,
+                Some(Err(refusal)) => typed.then(|| Problem::Moment(refusal.clone())),
+                Some(Ok(_)) => None,
+                None => table
+                    .column(cell.col)
+                    .filter(|_| typed)
+                    .and_then(|column| crate::edit::problem(table.dialect, column, &text)),
+            };
             Ok(Editor {
                 cell,
                 place,
@@ -908,6 +960,7 @@
                 top: large && from_value,
                 touched: touched || kept,
                 problem,
+                moment,
                 picker: None,
                 select: false,
             })
@@ -1496,7 +1549,8 @@
     /// as a cell to fix, so typing is never lost. Says whether the editor
     /// closed.
     pub(super) fn close_editor(&mut self, tab: ConnTabId, id: TabId, left: bool) -> bool {
-        let verdict = self.table(tab, id, typed);
+        let clock = self.date_clock();
+        let verdict = self.table(tab, id, |table, object| typed(table, object, &clock));
         // Whether the text is the key of an option taken from the
         // picker's list: read before the editor goes.
         let taken = self
```

`src/edit.rs`:

```diff
diff --git a/src/edit.rs b/src/edit.rs
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -14,6 +14,9 @@
 
 use crate::backend::RequestId;
 use crate::model::{CellPos, ObjectTab, Workspace};
+
+/// UTC, for a `timestamptz` whose zone is not known yet.
+static UTC: jiff::tz::TimeZone = jiff::tz::TimeZone::UTC;
 
 /// The largest value an editor opens, and the largest it keeps, in bytes
 /// of its text: a field that held megabytes would be laid out every frame.
@@ -181,6 +184,22 @@
             .columns
             .iter()
             .find(|column| column.name == *name)
+    }
+
+    /// The page's column `col` as an editor reads dates and times for it,
+    /// where it keeps them: the zone a PostgreSQL `timestamptz` is shown
+    /// in, and the values SQLite's follow.
+    pub fn dates(&self, col: usize) -> Option<crate::dates::Column> {
+        let column = self.column(col)?;
+        let values = self.page.rows.iter().filter_map(|row| row.get(col));
+        // A `timestamptz` in UTC while the settings' zone is not read yet
+        // (or is one the database does not know): it is sent with an
+        // offset all the same, which the server reads as it is.
+        let zone = match self.dialect {
+            Dialect::Postgres => Some(self.zone.unwrap_or(&UTC)),
+            Dialect::MySql | Dialect::Sqlite => None,
+        };
+        crate::dates::Column::of(self.dialect, &column.type_name, zone, values)
     }
 
     /// The column's class, by the type the structure gives it.
@@ -682,6 +701,20 @@
     TooLong { max: u32 },
     /// Over `MAX_EDIT_BYTES`: more than an editor opens.
     TooLarge,
+    /// No day or time the column takes, or one it does not hold.
+    Moment(crate::dates::Refusal),
+}
+
+impl Problem {
+    /// Whether the problem is a warning more than a fault: a time the
+    /// zone's clocks skip, which the line offers a way out of, or MySQL's
+    /// zero date. The text is not taken all the same.
+    pub fn warns(&self) -> bool {
+        matches!(
+            self,
+            Self::Moment(crate::dates::Refusal::Gap { .. } | crate::dates::Refusal::Zero)
+        )
+    }
 }
 
 /// Whether `text` is a value `column` takes, as far as the app can tell
@@ -997,21 +1030,6 @@
     }
 }
 
-/// Whether a column of `type_name` keeps a moment with its offset, so that
-/// a value typed into it without one is read in the server session's zone
-/// (PostgreSQL's `timestamptz`). MySQL's `TIMESTAMP` and SQLite's dates
-/// keep no offset: what is typed is what is kept.
-pub fn keeps_offset(dialect: Dialect, type_name: &str) -> bool {
-    match dialect {
-        Dialect::Postgres => {
-            let name = type_name.trim().to_ascii_lowercase();
-            name.starts_with("timestamptz")
-                || (name.starts_with("timestamp") && name.ends_with("with time zone"))
-        }
-        Dialect::MySql | Dialect::Sqlite => false,
-    }
-}
-
 /// What `text`, typed into a cell of `column`, is as the cell's new value.
 /// A word is a keyword only where the column is not text, and only where
 /// the column can take it: NULL where it may be NULL, DEFAULT where it has
@@ -1053,6 +1071,27 @@
     NewValue::Text(text.to_owned())
 }
 
+/// What `text`, typed into the cell of the page's column `col`, reads as
+/// where the column keeps dates or times: `None` for any other column, and
+/// for a keyword whose value the database makes (NULL, DEFAULT, `now()`).
+/// The word `now` alone is read here, as the time by `clock`: the
+/// database's own time is `now()`.
+pub fn moment(
+    table: &Table<'_>,
+    col: usize,
+    text: &str,
+    clock: &crate::dates::Clock,
+) -> Option<Result<crate::dates::Reading, crate::dates::Refusal>> {
+    let column = table.column(col)?;
+    let dates = table.dates(col)?;
+    match typed_as(table.dialect, column, text) {
+        NewValue::Text(_) => {}
+        NewValue::Now if text.trim().eq_ignore_ascii_case("now") => {}
+        NewValue::Null | NewValue::Default { .. } | NewValue::Now => return None,
+    }
+    Some(crate::dates::read(text, &dates, clock))
+}
+
 /// What stands against `text` as a value typed into a cell of `column`.
 /// Only text has a check: a keyword is the database's own.
 pub fn problem(dialect: Dialect, column: &ColumnInfo, text: &str) -> Option<Problem> {
@@ -1452,6 +1491,10 @@
     pub touched: bool,
     /// What the text fails, kept up to date by `Action::EditorTyped`.
     pub problem: Option<Problem>,
+    /// What the text reads as, where the column keeps dates or times:
+    /// what the line under the editor says, and, read once, what a commit
+    /// stores. Kept up to date with `problem`.
+    pub moment: Option<Result<crate::dates::Reading, crate::dates::Refusal>>,
     /// The search of the table a foreign key cell points at, while the
     /// editor is the small field of such a cell.
     pub picker: Option<crate::key_picker::Picker>,
@@ -4293,6 +4336,7 @@
             top: false,
             touched: false,
             problem: None,
+            moment: None,
             picker: None,
             select: false,
         });
@@ -5683,6 +5727,7 @@
             top: false,
             touched: true,
             problem: None,
+            moment: None,
             picker: None,
             select: false,
         });
```

`src/ui/cell_editor.rs`:

```diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -69,6 +69,10 @@
     pub hold: bool,
     /// The column keeps a date or a time: the editor offers now.
     pub now: bool,
+    /// The column as its dates and times are read, where it keeps them:
+    /// the line under the editor says what the text is stored as, and
+    /// says what it fails in the place of the note under the field.
+    pub dates: Option<crate::dates::Column>,
     /// The keys that end the edit, as the keymap has them.
     pub keys: Keys,
     /// The cell is a foreign key's, and its editor searches the table the
@@ -598,7 +602,13 @@
     // foreign key's text is a search until a commit takes it: what its
     // column would say of it shows only once it was refused.
     let said = target.search.as_ref().is_none_or(|search| search.refused);
-    let failing = editor.problem.is_some() && said;
+    // A warning is no fault: a time the clocks skip has the line's way
+    // out, and the field is not red for it.
+    let failing = editor
+        .problem
+        .as_ref()
+        .is_some_and(|problem| !problem.warns())
+        && said;
     if look.terminal {
         // The cursor's line, as the grid draws it round the cell its keys
         // are on, and no halo: the grid marks its own cell.
@@ -619,7 +629,10 @@
     } else {
         focus::hint(ui, &response, rect, Ring::Field { radius: 0 });
     }
-    if let (Some(problem), true, true) = (&editor.problem, target.hold, said) {
+    // A date's line says what it fails, where it is drawn: not under a
+    // foreign key's list.
+    let lined = target.dates.is_some() && editor.picker.is_none();
+    if let (Some(problem), true, true, false) = (&editor.problem, target.hold, said, lined) {
         let message = problem_text(problem, &target.type_name, Some(&editor.text), locale);
         // The list is under a foreign key's cell: the note goes over it.
         let above = target.search.is_some();
@@ -783,7 +796,13 @@
     // A foreign key's text is a search until a commit takes it: what its
     // column would say of it shows only once it was refused.
     let said = target.search.as_ref().is_none_or(|search| search.refused);
-    let failing = editor.problem.is_some() && said;
+    // A warning is no fault: a time the clocks skip has the line's way
+    // out, and the field is not red for it.
+    let failing = editor
+        .problem
+        .as_ref()
+        .is_some_and(|problem| !problem.warns())
+        && said;
     // A field's own ring, red while the text fails its check. The ring
     // shows once the keyboard is in use; the red border does not wait for
     // that, since a pasted text can fail before any key is pressed.
@@ -810,7 +829,7 @@
     let problem = editor
         .problem
         .as_ref()
-        .filter(|_| said)
+        .filter(|_| said && (target.dates.is_none() || editor.picker.is_some()))
         .map(|problem| problem_text(problem, &target.type_name, Some(&editor.text), locale));
     // While the text is a search it is no value, and its length is none
     // of the column's business.
@@ -1448,6 +1467,7 @@
         Problem::TooLarge => {
             say("Over 256 KiB: values this large cannot be edited yet").into_owned()
         }
+        Problem::Moment(refusal) => crate::ui::date_line::refusal_text(refusal, type_name, locale),
     }
 }
 
```

`src/ui/data_view.rs`:

```diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1750,6 +1750,14 @@
                 pick = Some(Action::PickerNull { tab, id });
             }
         }
+        // A date or time cell's line, under it, while it is in sight.
+        if !large
+            && let (Some(editor), Some(target)) = (editor.as_deref_mut(), &target)
+            && let Some(anchor) = hangs
+        {
+            let skin = (&look, &palette, locale);
+            crate::ui::date_line::under(ui, anchor, editor, target, &mut outcome, skin);
+        }
         if large
             && let (Some(editor), Some(target)) = (editor, &target)
             && let Some(anchor) = output.editing_rect
@@ -1973,6 +1981,7 @@
         now: table
             .column(cell.col)
             .is_some_and(|column| tabletist_db::temporal(&column.type_name).is_some()),
+        dates: table.dates(cell.col),
         keys: keys.clone(),
         search: object
             .edits
```

`src/ui/date_line.rs`:

```diff
diff --git a/src/ui/date_line.rs b/src/ui/date_line.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/date_line.rs
@@ -0,0 +1,723 @@
+//! The line under the editor of a date or time: what the text will be
+//! stored as, or why it will not be. A popover under the cell, or under
+//! the row panel's field, on macOS; a float under it in the terminal look,
+//! headed by the column and its zone. It never takes the keyboard: the
+//! field keeps it, and every choice it offers rewrites the field's text.
+
+use egui::accesskit::{Live, Role};
+use egui::{Id, Rect, Sense, Ui, pos2, vec2};
+use jiff::civil::{Date, Time, Weekday};
+
+use crate::dates::{Column, Kind, Moment, Range, Reading, Refusal, Stored};
+use crate::i18n::{Locale, gettext};
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::{sql_complete, widgets};
+
+/// `text` in the person's language.
+fn tr(locale: Locale, text: &'static str) -> String {
+    gettext(locale, text).into_owned()
+}
+
+/// How the line reads.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Tone {
+    /// What will be stored.
+    Plain,
+    /// Stored only once a choice is made (a change of the clocks), or a
+    /// value the cell holds that nothing should be given.
+    Warning,
+    /// Not stored: the text is no value of the column.
+    Danger,
+}
+
+/// A choice the line offers, as a button: what it says, the text it
+/// rewrites the field to, and whether the value is that one now.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Offer {
+    pub label: String,
+    pub text: String,
+    pub chosen: bool,
+}
+
+/// The line's words.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Line {
+    pub tone: Tone,
+    pub words: String,
+    /// The literal a save sends, said after the words where it is not
+    /// what was typed: "sent as `2026-01-15 09:30:00-05`".
+    pub sent: Option<String>,
+    pub offers: Vec<Offer>,
+}
+
+impl Line {
+    /// All of it as one text: what a screen reader is told, and what a
+    /// test reads.
+    pub fn said(&self, look: &Look, locale: Locale) -> String {
+        match &self.sent {
+            Some(sent) => format!(
+                "{} · {sent}",
+                self.words,
+                sent = sent_words(sent, look, locale)
+            ),
+            None => self.words.clone(),
+        }
+    }
+}
+
+/// "sent as" before `literal`, in the look's case: the literal as it is.
+fn sent_words(literal: &str, look: &Look, locale: Locale) -> String {
+    format!("{} {literal}", look.label(&tr(locale, "sent as")))
+}
+
+const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
+const MONTHS: [&str; 12] = [
+    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
+];
+const FULL_MONTHS: [&str; 12] = [
+    "January",
+    "February",
+    "March",
+    "April",
+    "May",
+    "June",
+    "July",
+    "August",
+    "September",
+    "October",
+    "November",
+    "December",
+];
+
+fn weekday_name(day: Weekday, locale: Locale) -> String {
+    let at = usize::try_from(day.to_monday_zero_offset()).unwrap_or_default();
+    tr(locale, WEEKDAYS[at % 7])
+}
+
+fn month_name(month: i8, locale: Locale) -> String {
+    let at = usize::try_from(month - 1).unwrap_or_default();
+    tr(locale, MONTHS[at % 12])
+}
+
+/// The year as a person reads it: `2026`, and `44 BC` before 1 AD.
+fn year_words(year: i16, locale: Locale) -> String {
+    if year < 1 {
+        format!("{} {}", 1 - i32::from(year), tr(locale, "BC"))
+    } else {
+        year.to_string()
+    }
+}
+
+/// "Thu 15 Jan 2026".
+pub fn day_words(date: Date, locale: Locale) -> String {
+    format!(
+        "{} {} {} {}",
+        weekday_name(date.weekday(), locale),
+        date.day(),
+        month_name(date.month(), locale),
+        year_words(date.year(), locale)
+    )
+}
+
+/// "15 Jan" or "8 Mar": a day of the year the line is about.
+fn short_day(date: Date, locale: Locale) -> String {
+    format!("{} {}", date.day(), month_name(date.month(), locale))
+}
+
+/// "09:30", with its seconds where they are not nothing (always for a
+/// time of day, `seconds`), and its fraction to `digits` where it has one.
+fn time_words(time: Time, digits: u8, seconds: bool) -> String {
+    let mut out = format!("{:02}:{:02}", time.hour(), time.minute());
+    let fraction = time.subsec_nanosecond() != 0 && digits > 0;
+    if seconds || time.second() != 0 || fraction {
+        out.push_str(&format!(":{:02}", time.second()));
+    }
+    if fraction {
+        let nanos = format!("{:09}", time.subsec_nanosecond());
+        out.push('.');
+        out.push_str(&nanos[..usize::from(digits.min(9))]);
+    }
+    out
+}
+
+/// "today", "tomorrow", "in 6 days", "3 days ago".
+fn days_words(days: i64, locale: Locale) -> String {
+    match days {
+        0 => tr(locale, "today"),
+        1 => tr(locale, "tomorrow"),
+        -1 => tr(locale, "yesterday"),
+        days if days > 0 => tr(locale, "in {count} days").replace("{count}", &days.to_string()),
+        days => tr(locale, "{count} days ago").replace("{count}", &days.abs().to_string()),
+    }
+}
+
+/// What the type a column of `kind` keeps is called, in a sentence:
+/// "date", "time", "date and time".
+pub fn kind_words(kind: Kind, locale: Locale) -> String {
+    match kind {
+        Kind::Date => tr(locale, "date"),
+        Kind::Time { .. } => tr(locale, "time"),
+        Kind::Timestamp | Kind::Zoned => tr(locale, "date and time"),
+    }
+}
+
+/// Why `refusal` keeps a text from its column, of the type `type_name`
+/// (as the grid's header names it).
+pub fn refusal_text(refusal: &Refusal, type_name: &str, locale: Locale) -> String {
+    let say = |text: &'static str| tr(locale, text);
+    match refusal {
+        Refusal::NotRead(Kind::Date) => say("Not a date. Try 2026-01-15, 15 Jan 2026 or tomorrow."),
+        Refusal::NotRead(Kind::Time { .. }) => say("Not a time. Try 9:30, 9am or 21:30."),
+        Refusal::NotRead(Kind::Timestamp | Kind::Zoned) => {
+            say("Not a date and time. Try 2026-01-15 9:30 or tomorrow 9am.")
+        }
+        Refusal::NoMonth(month) => {
+            say("There is no month {month}.").replace("{month}", &month.to_string())
+        }
+        Refusal::NoDay { day, month } => {
+            let at = usize::try_from(month - 1).unwrap_or_default();
+            say("There is no {day} {month}.")
+                .replace("{day}", &day.to_string())
+                .replace("{month}", &tr(locale, FULL_MONTHS[at % 12]))
+        }
+        Refusal::NoTime(typed) => say("{time} is not a time of day.").replace("{time}", typed),
+        Refusal::Range(range) => match range {
+            Range::PgDate => say("PostgreSQL date holds 4714-11-24 BC to 5874897 AD."),
+            Range::PgTimestamp => say("PostgreSQL timestamp holds 4714-11-24 BC to 294276 AD."),
+            Range::MySqlDate => say("MySQL DATE holds 1000-01-01 to 9999-12-31."),
+            Range::MySqlDatetime => say("MySQL DATETIME holds 1000-01-01 to 9999-12-31."),
+            Range::MySqlTimestamp => say("MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."),
+            Range::MySqlTime => say("MySQL TIME holds -838:59:59 to 838:59:59."),
+            Range::PgOffset => say("PostgreSQL keeps offsets from -15:59:59 to +15:59:59."),
+        },
+        Refusal::Years => say("Tabletist reads years up to 9999."),
+        Refusal::YearZero => say("There is no year 0: 1 BC comes just before 1 AD."),
+        Refusal::Gap { at, place, .. } => {
+            let words = match place.is_empty() {
+                true => say("{time} doesn't exist on {day} in this zone; clocks go forward."),
+                false => say("{time} doesn't exist on {day} in {place}; clocks go forward."),
+            };
+            words
+                .replace("{time}", &time_words(at.time(), 0, false))
+                .replace("{day}", &short_day(at.date(), locale))
+                .replace("{place}", place)
+        }
+        Refusal::Zero => say("MySQL zero date · pick a date or set NULL"),
+        Refusal::NoZone => {
+            say("{type} keeps no zone: leave the offset out.").replace("{type}", type_name)
+        }
+        Refusal::DayOnly => say("A date keeps no time of day."),
+        Refusal::TimeOnly => say("A time keeps no day."),
+        Refusal::ZoneUnknown => say(
+            "The server's zone is not known: type the day and the time as the server keeps them, with no offset.",
+        ),
+    }
+}
+
+/// What a value read is said as: the day, its time and zone, how far it is
+/// from today, and how it is kept.
+fn reading_words(reading: &Reading, column: &Column, locale: Locale) -> String {
+    let say = |text: &'static str| tr(locale, text);
+    let (date, time, offset) = match &reading.moment {
+        Moment::Infinity => return say("No end date · stored as infinity"),
+        Moment::MinusInfinity => return say("No start date · stored as -infinity"),
+        Moment::EndOfDay => {
+            return format!("24:00:00 · {}", say("the end of the day"));
+        }
+        Moment::Span(written) => {
+            return format!("{written} · {}", say("a span of time, not a time of day"));
+        }
+        Moment::At { date, time, offset } => (*date, *time, *offset),
+    };
+    let digits = column.digits;
+    let mut head = match (date, time) {
+        (Some(date), Some(time)) if column.kind.has_time() => {
+            format!(
+                "{}, {}",
+                day_words(date, locale),
+                time_words(time, digits, false)
+            )
+        }
+        (Some(date), _) => day_words(date, locale),
+        (None, Some(time)) => {
+            let mut words = time_words(time, digits, true);
+            if let Some(offset) = offset {
+                words.push_str(&crate::dates::written_offset(offset));
+            }
+            words
+        }
+        (None, None) => String::new(),
+    };
+    if let Some(zone) = &reading.zone_name {
+        head.push(' ');
+        head.push_str(zone);
+    }
+    let mut parts = vec![head];
+    if let Some(days) = reading
+        .days
+        .filter(|days| reading.relative || days.abs() <= 7)
+    {
+        parts.push(days_words(days, locale));
+    }
+    if column.kind == Kind::Timestamp && offset.is_none() {
+        parts.push(say("no zone"));
+    }
+    let sqlite = match column.dialect {
+        tabletist_db::Dialect::Sqlite => column.kind.has_date(),
+        tabletist_db::Dialect::Postgres | tabletist_db::Dialect::MySql => false,
+    };
+    if sqlite {
+        parts.push(match column.stored {
+            Stored::Fresh => say("stored as text"),
+            Stored::Text { .. } => say("stored as text, like the other rows"),
+            Stored::Seconds => say("stored as seconds since 1970, like the other rows"),
+            Stored::Days => say("stored as a Julian day, like the other rows"),
+        });
+    }
+    parts.join(" · ")
+}
+
+/// The line for the text `typed`, which reads as `moment`, in a column
+/// `column` of the type `type_name` (as the grid's header names it).
+pub fn line(
+    typed: &str,
+    moment: &Result<Reading, Refusal>,
+    column: &Column,
+    type_name: &str,
+    (look, locale): (&Look, Locale),
+) -> Line {
+    let plain = |tone: Tone, words: String| Line {
+        tone,
+        words: look.label(&words),
+        sent: None,
+        offers: Vec::new(),
+    };
+    if typed.trim().is_empty() {
+        let hint = match column.kind {
+            Kind::Date => tr(locale, "Type a date: 2026-01-15, 15 Jan 2026 or tomorrow."),
+            Kind::Time { .. } => tr(locale, "Type a time: 9:30, 9am or 21:30."),
+            Kind::Timestamp | Kind::Zoned => tr(
+                locale,
+                "Type a date and time: 2026-01-15 9:30 or tomorrow 9am.",
+            ),
+        };
+        return plain(Tone::Plain, hint);
+    }
+    match moment {
+        Ok(reading) => {
+            let words = reading_words(reading, column, locale);
+            let sent = (reading.literal != typed.trim()).then(|| reading.literal.clone());
+            let Some(fold) = &reading.fold else {
+                return Line {
+                    tone: Tone::Plain,
+                    words: look.label(&words),
+                    sent,
+                    offers: Vec::new(),
+                };
+            };
+            let words = match fold.place.is_empty() {
+                true => tr(
+                    locale,
+                    "{time} happens twice on {day} in this zone; clocks go back.",
+                ),
+                false => tr(
+                    locale,
+                    "{time} happens twice on {day} in {place}; clocks go back.",
+                ),
+            }
+            .replace("{time}", &time_words(fold.at.time(), 0, false))
+            .replace("{day}", &short_day(fold.at.date(), locale))
+            .replace("{place}", &fold.place);
+            let chosen = match &reading.moment {
+                Moment::At { offset, .. } => *offset,
+                Moment::Infinity | Moment::MinusInfinity | Moment::Span(_) | Moment::EndOfDay => {
+                    None
+                }
+            };
+            let offers = [&fold.earlier, &fold.later]
+                .into_iter()
+                .map(|(offset, name)| Offer {
+                    label: look.label(&format!("{} {name}", time_words(fold.at.time(), 0, false))),
+                    text: crate::dates::rewritten(fold.at, Some(*offset), column),
+                    chosen: chosen == Some(*offset),
+                })
+                .collect();
+            Line {
+                tone: Tone::Warning,
+                words: look.label(&words),
+                sent,
+                offers,
+            }
+        }
+        Err(Refusal::Gap { offered, .. }) => {
+            let refusal = moment.as_ref().err();
+            let words = refusal.map(|refusal| refusal_text(refusal, type_name, locale));
+            let time = time_words(offered.time(), 0, false);
+            Line {
+                tone: Tone::Warning,
+                words: look.label(&words.unwrap_or_default()),
+                sent: None,
+                offers: vec![Offer {
+                    label: look.label(&tr(locale, "Use {time}").replace("{time}", &time)),
+                    text: crate::dates::rewritten(*offered, None, column),
+                    chosen: false,
+                }],
+            }
+        }
+        Err(Refusal::Zero) => plain(
+            Tone::Warning,
+            refusal_text(&Refusal::Zero, type_name, locale),
+        ),
+        Err(refusal) => {
+            let words = refusal_text(refusal, type_name, locale);
+            // The terminal marks what is refused, as its status line does.
+            let words = if look.terminal {
+                format!("! {words}")
+            } else {
+                words
+            };
+            plain(Tone::Danger, words)
+        }
+    }
+}
+
+/// What the pointer did to the line this frame.
+#[derive(Default)]
+pub struct Shown {
+    /// The line was pressed or clicked: the press landed outside the
+    /// field, which keeps the keyboard.
+    pub pressed: bool,
+    /// The text an offer rewrites the field to, once clicked.
+    pub picked: Option<String>,
+}
+
+/// The widths of the macOS popover and of the terminal's float.
+const WIDTH: f32 = 330.0;
+const TERMINAL_WIDTH: f32 = 420.0;
+/// The room inside the frame.
+const PAD: egui::Vec2 = vec2(10.0, 8.0);
+/// The gap between the cell and the line.
+const GAP: f32 = 4.0;
+
+/// The float's head in the terminal look: the column and its type, and the
+/// zone a zoned value is in.
+pub struct Head<'a> {
+    pub column: &'a str,
+    pub type_name: &'a str,
+    pub zone: Option<String>,
+}
+
+/// Draws `line` under `anchor`, the cell or field the editor `field` is
+/// on (over it where the window has no room below), named after the
+/// column as a dialog is. `head` heads the terminal look's float.
+pub fn show(
+    ui: &Ui,
+    anchor: Rect,
+    field: Id,
+    (line, head, kind): (&Line, &Head<'_>, Kind),
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Shown {
+    let mut shown = Shown::default();
+    let screen = ui.ctx().content_rect();
+    let width = (if look.terminal { TERMINAL_WIDTH } else { WIDTH })
+        .max(anchor.width().min(WIDTH * 1.5))
+        .min(screen.width() - 16.0);
+    let inner = width - 2.0 * PAD.x;
+    let small = widgets::secondary(look);
+    let color = match line.tone {
+        Tone::Plain => palette.secondary,
+        Tone::Warning => palette.warning,
+        Tone::Danger => palette.danger,
+    };
+    let mut words = Text::one(look, small, &line.words, color);
+    if let Some(sent) = &line.sent {
+        let mono = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
+        words = words
+            .add(
+                small,
+                &format!(" · {} ", look.label(&tr(locale, "sent as"))),
+                color,
+            )
+            .add(mono, sent, color);
+    }
+    let laid = words.wrap(inner).layout(ui.ctx());
+    let head_laid = look.terminal.then(|| {
+        let name = format!("{} · {}", head.column, head.type_name);
+        Text::one(look, small, &name, palette.dim).layout(ui.ctx())
+    });
+    let zone_laid = head
+        .zone
+        .as_ref()
+        .filter(|_| look.terminal)
+        .map(|zone| Text::one(look, small, &zone.to_lowercase(), palette.dim).layout(ui.ctx()));
+    let head_height = head_laid
+        .as_ref()
+        .map_or(0.0, |laid| laid.height() + 2.0 * PAD.y);
+    let offers_height = if line.offers.is_empty() {
+        0.0
+    } else {
+        look.control_height + PAD.y
+    };
+    let height = head_height + laid.height() + 2.0 * PAD.y + offers_height;
+    let below = anchor.bottom() + GAP;
+    let top = if below + height > screen.bottom() && anchor.top() - GAP - height >= screen.top() {
+        anchor.top() - GAP - height
+    } else {
+        below
+    };
+    let left = anchor.left().min(screen.right() - width).max(screen.left());
+    let rect = Rect::from_min_size(pos2(left, top), vec2(width, height));
+    // One area for the lines of every editor: only one editor is open.
+    let id = Id::new("date-line");
+    egui::Area::new(id)
+        .order(egui::Order::Foreground)
+        .fade_in(false)
+        .constrain(false)
+        .fixed_pos(rect.min)
+        .show(ui.ctx(), |ui| {
+            let (rect, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
+            let whole = ui.interact(rect, id.with("whole"), Sense::CLICK);
+            let name = format!("{}, {}", head.column, kind_words(kind, locale));
+            whole.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &name));
+            ui.ctx().accesskit_node_builder(whole.id, |node| {
+                node.set_role(Role::Dialog);
+            });
+            // The dialog is the field's: it describes what is typed there.
+            ui.ctx().accesskit_node_builder(field, |node| {
+                node.push_controlled(whole.id.accesskit_id());
+            });
+            let clicked_over =
+                ui.input(|input| input.pointer.any_click()) && ui.rect_contains_pointer(rect);
+            shown.pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over;
+            let corner = if look.terminal { 0 } else { look.dialog_radius };
+            let border = sql_complete::panel(ui, rect, corner, look, palette);
+            let mut y = rect.top();
+            if let Some(head_laid) = head_laid {
+                let center = y + head_height / 2.0;
+                head_laid.paint_left(ui.painter(), rect.left() + PAD.x, center);
+                if let Some(zone) = zone_laid {
+                    zone.paint_right(ui.painter(), rect.right() - PAD.x, center);
+                }
+                y += head_height;
+                ui.painter()
+                    .hline(rect.x_range(), y, egui::Stroke::new(1.0, palette.border));
+            }
+            let words_rect = Rect::from_min_size(
+                pos2(rect.left() + PAD.x, y + PAD.y),
+                vec2(inner, laid.height()),
+            );
+            let said = line.said(look, locale);
+            // A live region: a screen reader reads the line out as it
+            // changes, at once where the text is refused.
+            // One node whatever it says: a screen reader reads a region
+            // out as it changes, not one that is made again.
+            let live = ui.interact(words_rect, id.with("said"), Sense::hover());
+            live.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
+            ui.ctx().accesskit_node_builder(live.id, |node| {
+                node.set_live(match line.tone {
+                    Tone::Danger => Live::Assertive,
+                    Tone::Plain | Tone::Warning => Live::Polite,
+                });
+            });
+            laid.paint_left(ui.painter(), words_rect.left(), words_rect.center().y);
+            y = words_rect.bottom() + PAD.y;
+            let mut x = rect.left() + PAD.x;
+            for (at, offer) in line.offers.iter().enumerate() {
+                let spec = widgets::ButtonSpec::new(&offer.label)
+                    .padding(8.0)
+                    .role(small);
+                // The one the value is now stands out as the default does.
+                let spec = if offer.chosen { spec.primary() } else { spec };
+                let button = Rect::from_min_size(
+                    pos2(x, y),
+                    vec2(spec.width(ui, look), look.control_height),
+                );
+                let mut child = ui.new_child(
+                    egui::UiBuilder::new()
+                        .max_rect(button)
+                        .id_salt(("offer", at)),
+                );
+                let response = spec.show_at(&mut child, button, look, palette);
+                if response.clicked() {
+                    shown.picked = Some(offer.text.clone());
+                }
+                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
+                x = button.right() + 6.0;
+            }
+            ui.painter().add(border);
+        });
+    shown
+}
+
+/// The line of `editor`, where its column keeps dates or times and no
+/// foreign key's list hangs from it, under `anchor`. A press on it is the
+/// editor's: the field `target.id` keeps the keyboard, and the frame does
+/// not read as an edit that was left. An offer clicked rewrites the text,
+/// as typing would.
+pub fn under(
+    ui: &Ui,
+    anchor: Rect,
+    editor: &mut crate::edit::Editor,
+    target: &crate::ui::cell_editor::Target,
+    outcome: &mut crate::ui::cell_editor::Outcome,
+    skin: (&Look, &Palette, Locale),
+) {
+    let (Some(column), Some(moment), None) = (&target.dates, &editor.moment, &editor.picker) else {
+        return;
+    };
+    let (look, _, locale) = skin;
+    let said = line(
+        &editor.text,
+        moment,
+        column,
+        &target.type_name,
+        (look, locale),
+    );
+    let zone = column
+        .zone
+        .as_ref()
+        .and_then(|zone| zone.iana_name().map(str::to_owned));
+    let head = Head {
+        column: &target.name,
+        type_name: &target.type_name,
+        zone,
+    };
+    let shown = show(ui, anchor, target.id, (&said, &head, column.kind), skin);
+    if shown.pressed {
+        ui.memory_mut(|memory| memory.request_focus(target.id));
+        outcome.left = false;
+        outcome.kept = false;
+    }
+    if let Some(text) = shown.picked {
+        editor.text = text;
+        outcome.changed = true;
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+    use crate::dates::{Clock, Order, read};
+    use jiff::civil::date;
+    use jiff::tz::TimeZone;
+    use tabletist_db::Dialect;
+
+    fn new_york() -> TimeZone {
+        TimeZone::get("America/New_York").unwrap()
+    }
+
+    fn clock() -> Clock {
+        let now = date(2026, 10, 24)
+            .at(10, 0, 0, 0)
+            .to_zoned(new_york())
+            .unwrap()
+            .timestamp();
+        Clock {
+            now,
+            zone: new_york(),
+            order: Order::DayMonth,
+        }
+    }
+
+    fn line_of(dialect: Dialect, type_name: &str, typed: &str, look: &Look) -> Line {
+        let column = Column::of(dialect, type_name, Some(&new_york()), []).unwrap();
+        let moment = read(typed, &column, &clock());
+        line(typed, &moment, &column, type_name, (look, Locale::English))
+    }
+
+    fn said(dialect: Dialect, type_name: &str, typed: &str) -> String {
+        let look = Look::macos();
+        line_of(dialect, type_name, typed, &look).said(&look, Locale::English)
+    }
+
+    #[test]
+    fn the_line_says_what_is_stored_as_the_boards_word_it() {
+        let pg = Dialect::Postgres;
+        assert_eq!(
+            said(pg, "timestamptz", "2026-01-15 09:30"),
+            "Thu 15 Jan 2026, 09:30 EST · sent as 2026-01-15 09:30:00-05"
+        );
+        assert_eq!(said(pg, "date", "2026-01-15"), "Thu 15 Jan 2026");
+        assert_eq!(
+            said(pg, "date", "2026-10-30"),
+            "Fri 30 Oct 2026 · in 6 days"
+        );
+        assert_eq!(
+            said(pg, "date", "+6d"),
+            "Fri 30 Oct 2026 · in 6 days · sent as 2026-10-30"
+        );
+        assert_eq!(said(pg, "time", "9:30"), "09:30:00 · sent as 09:30:00");
+        assert_eq!(
+            said(pg, "timestamp(6)", "2026-01-15 09:30:12.250"),
+            "Thu 15 Jan 2026, 09:30:12.250000 · no zone"
+        );
+        assert_eq!(
+            said(pg, "date", "infinity"),
+            "No end date · stored as infinity"
+        );
+        assert_eq!(
+            said(Dialect::MySql, "time", "100:30:00"),
+            "100:30:00 · a span of time, not a time of day"
+        );
+        assert_eq!(said(pg, "date", "0044-03-15 BC"), "Fri 15 Mar 44 BC");
+        assert_eq!(
+            said(Dialect::Sqlite, "DATE", "2026-10-24"),
+            "Sat 24 Oct 2026 · today · stored as text"
+        );
+    }
+
+    #[test]
+    fn a_refused_text_says_why() {
+        let pg = Dialect::Postgres;
+        let refused = line_of(pg, "date", "2026-02-30", &Look::macos());
+        assert_eq!(refused.tone, Tone::Danger);
+        assert_eq!(refused.words, "There is no 30 February.");
+        assert_eq!(said(pg, "time", "25:00"), "25:00 is not a time of day.");
+        assert_eq!(
+            said(Dialect::MySql, "timestamp", "2040-05-01 08:00"),
+            "MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."
+        );
+        let terminal = line_of(pg, "date", "2026-02-30", &Look::omarchy());
+        assert_eq!(terminal.words, "! there is no 30 february.");
+    }
+
+    #[test]
+    fn a_change_of_the_clocks_offers_its_way_out() {
+        let pg = Dialect::Postgres;
+        let gap = line_of(pg, "timestamptz", "2026-03-08 02:30", &Look::macos());
+        assert_eq!(gap.tone, Tone::Warning);
+        assert_eq!(
+            gap.words,
+            "02:30 doesn't exist on 8 Mar in New York; clocks go forward."
+        );
+        assert_eq!(
+            gap.offers,
+            vec![Offer {
+                label: "Use 03:30".into(),
+                text: "2026-03-08 03:30".into(),
+                chosen: false
+            }]
+        );
+        let fold = line_of(pg, "timestamptz", "2026-11-01 01:30", &Look::macos());
+        assert_eq!(
+            fold.words,
+            "01:30 happens twice on 1 Nov in New York; clocks go back."
+        );
+        let labels: Vec<_> = fold
+            .offers
+            .iter()
+            .map(|offer| (offer.label.as_str(), offer.chosen))
+            .collect();
+        assert_eq!(labels, [("01:30 EDT", true), ("01:30 EST", false)]);
+        assert_eq!(fold.offers[1].text, "2026-11-01 01:30-05");
+        // The terminal look's offers are in its case, as its words are.
+        let terminal = line_of(pg, "timestamptz", "2026-11-01 01:30", &Look::omarchy());
+        let labels: Vec<_> = terminal
+            .offers
+            .iter()
+            .map(|offer| offer.label.as_str())
+            .collect();
+        assert_eq!(labels, ["01:30 edt", "01:30 est"]);
+    }
+}
```

`src/ui/format.rs`:

```diff
diff --git a/src/ui/format.rs b/src/ui/format.rs
--- a/src/ui/format.rs
+++ b/src/ui/format.rs
@@ -523,20 +523,6 @@
     jiff::tz::Offset::from_seconds(sign * seconds).ok()
 }
 
-/// An offset as [`offset_of`] reads it: the minutes and the seconds only
-/// where they are not nothing.
-fn written_offset(offset: jiff::tz::Offset) -> String {
-    let seconds = offset.seconds();
-    let sign = if seconds < 0 { '-' } else { '+' };
-    let seconds = seconds.unsigned_abs();
-    let (hours, minutes, seconds) = (seconds / 3_600, seconds / 60 % 60, seconds % 60);
-    match (minutes, seconds) {
-        (0, 0) => format!("{sign}{hours:02}"),
-        (_, 0) => format!("{sign}{hours:02}:{minutes:02}"),
-        _ => format!("{sign}{hours:02}:{minutes:02}:{seconds:02}"),
-    }
-}
-
 /// `at` written as a [`Stamp`] is, with `digits` of its fraction.
 fn written_stamp(at: jiff::civil::DateTime, separator: char, digits: usize) -> String {
     let mut out = format!(
@@ -575,7 +561,7 @@
     Some(format!(
         "{}{}",
         written_stamp(shown.datetime(), stamp.separator, stamp.digits),
-        written_offset(shown.offset())
+        crate::dates::written_offset(shown.offset())
     ))
 }
 
@@ -591,44 +577,6 @@
         written.or_else(|| text.parse::<jiff::Timestamp>().ok())
     };
     matches!((moment(a), moment(b)), (Some(a), Some(b)) if a == b)
-}
-
-/// `text`, a date and a time typed with no offset, with the offset `zone`
-/// has at that time: `2026-01-12 04:14` in New York is sent as
-/// `2026-01-12 04:14-05`, so the server reads it in the zone the person
-/// sees and not in its session's. A date alone is its midnight. A time the
-/// zone skips or has twice (a change of the clocks) takes the offset from
-/// before the change. `None` for anything else: a text with an offset
-/// already, or no date and time.
-pub fn with_offset(text: &str, zone: &jiff::tz::TimeZone) -> Option<String> {
-    let typed = text.trim();
-    // One with an offset, or a `Z`, reads as a moment: it is sent as typed.
-    if typed.parse::<jiff::Timestamp>().is_ok() {
-        return None;
-    }
-    // Only a date of four digits for its year, as PostgreSQL reads one,
-    // and no zone's name in brackets, which it does not: anything else is
-    // the person's to send as typed.
-    let bytes = typed.as_bytes();
-    let plain = bytes.len() >= 10
-        && bytes[..4].iter().all(u8::is_ascii_digit)
-        && bytes[4] == b'-'
-        && !typed.contains('[');
-    if !plain {
-        return None;
-    }
-    let at: jiff::civil::DateTime = typed.parse().ok()?;
-    let offset = match zone.to_ambiguous_timestamp(at).offset() {
-        jiff::tz::AmbiguousOffset::Unambiguous { offset } => offset,
-        jiff::tz::AmbiguousOffset::Gap { before, .. }
-        | jiff::tz::AmbiguousOffset::Fold { before, .. } => before,
-    };
-    let typed = if typed.contains(':') {
-        typed.to_owned()
-    } else {
-        written_stamp(at, ' ', 0)
-    };
-    Some(format!("{typed}{}", written_offset(offset)))
 }
 
 /// `text` cut in the middle with "…" to fit `max` points as measured by
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -23,6 +23,7 @@
 pub mod conflict_prompt;
 pub mod connect_dialog;
 pub mod data_view;
+pub mod date_line;
 #[cfg(test)]
 mod date_tests;
 #[cfg(test)]
```

`src/ui/row_form.rs`:

```diff
diff --git a/src/ui/row_form.rs b/src/ui/row_form.rs
--- a/src/ui/row_form.rs
+++ b/src/ui/row_form.rs
@@ -264,6 +264,14 @@
                 self.ending.push(asked.action(tab, id));
             }
         }
+        // A date or time field's line, under the field while it is in
+        // sight.
+        if !editor.large
+            && let Some(anchor) = outcome.rect
+            && let Some(anchor) = crate::ui::key_picker::hung_from(anchor, ui.clip_rect())
+        {
+            crate::ui::date_line::under(ui, anchor, editor, target, &mut outcome, skin);
+        }
         // The text is noted as typed before anything ends the edit, as for
         // a cell's field.
         if outcome.changed {
```


- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: clean; the library's tests `test result: ok. 3351 passed; 0 failed; 17 ignored`, and every other target as before

- [ ] **Step 5: Commit**

```bash
git add src/edit.rs src/app.rs src/app/editing.rs src/ui/format.rs src/ui/date_line.rs src/ui/date_tests.rs src/ui/cell_editor.rs src/ui/data_view.rs src/ui/row_form.rs src/ui/mod.rs src/ui/zone_tests.rs
git commit -m "Say under a date's editor what it stores, and store that"
```

---

### Task 3: A MySQL TIMESTAMP is read in its session's zone

**Files:**
- Modify: `crates/tabletist-db/src/adapter.rs`, `crates/tabletist-db/src/lib.rs`, `crates/tabletist-db/src/mysql.rs`, `crates/tabletist-db/src/pg.rs`, `crates/tabletist-db/src/sqlite.rs`, `crates/tabletist-db/tests/mysql.rs`, `crates/tabletist-db/tests/postgres.rs`, `crates/tabletist-db/tests/sqlite.rs`, `src/backend.rs`, `src/zones.rs`, `src/model.rs`, `src/app.rs`, `src/edit.rs`, `src/key_picker.rs`, `src/review.rs`, `src/ui/date_tests.rs`, `docs/_guide/editing-data.md`

`Connection::session_zone` names the zone MySQL moves a `TIMESTAMP` to and from (the session's, or the server's where it follows the system); PostgreSQL and SQLite name none. The app asks it once a MySQL session is up (`App::ask_session_zone`, from `after_connect` and from a session woken after it was closed for being idle; the test goes through `after_connect`); the backend reads the name from the zone database (`zones::session`), and the workspace keeps the answer, dropped with the session. `Table::dates` gives it to a MySQL column. The website says how dates and times are typed.

- [ ] **Step 1: Write the failing tests**

`crates/tabletist-db/tests/mysql.rs`:

```diff
diff --git a/crates/tabletist-db/tests/mysql.rs b/crates/tabletist-db/tests/mysql.rs
--- a/crates/tabletist-db/tests/mysql.rs
+++ b/crates/tabletist-db/tests/mysql.rs
@@ -1914,6 +1914,18 @@
     let next = run(&connection, "SELECT 1", 1).await.unwrap();
     assert!(!next.was_cancelled());
     assert_connect_time_settings(&connection).await;
+}
+
+#[tokio::test]
+async fn the_session_names_the_zone_its_timestamps_are_kept_in() {
+    let Some(connection) = connect().await else {
+        return;
+    };
+    // The session's, or the server's where it follows the system's: never
+    // the word SYSTEM.
+    let zone = within(connection.session_zone()).await.unwrap();
+    let zone = zone.expect("a MySQL session has a zone");
+    assert!(!zone.is_empty() && zone != "SYSTEM", "{zone}");
 }
 
 #[tokio::test]
```

`crates/tabletist-db/tests/postgres.rs`:

```diff
diff --git a/crates/tabletist-db/tests/postgres.rs b/crates/tabletist-db/tests/postgres.rs
--- a/crates/tabletist-db/tests/postgres.rs
+++ b/crates/tabletist-db/tests/postgres.rs
@@ -1604,6 +1604,14 @@
         &outcome.results[0].outcome,
         StatementOutcome::Error { error: Error::Query { code: Some(code), .. }, .. } if code == "42601"
     ));
+}
+
+#[tokio::test]
+async fn postgres_sends_its_moments_with_their_offsets_and_names_no_zone() {
+    let Some(connection) = connect().await else {
+        return;
+    };
+    assert_eq!(within(connection.session_zone()).await.unwrap(), None);
 }
 
 #[tokio::test]
```

`crates/tabletist-db/tests/sqlite.rs`:

```diff
diff --git a/crates/tabletist-db/tests/sqlite.rs b/crates/tabletist-db/tests/sqlite.rs
--- a/crates/tabletist-db/tests/sqlite.rs
+++ b/crates/tabletist-db/tests/sqlite.rs
@@ -1485,6 +1485,12 @@
 }
 
 #[tokio::test]
+async fn sqlite_keeps_no_zone_for_its_moments() {
+    let (connection, _dir) = fixture().await;
+    assert_eq!(within(connection.session_zone()).await.unwrap(), None);
+}
+
+#[tokio::test]
 async fn the_server_version_names_sqlite() {
     let (connection, _dir) = fixture().await;
     let version = within(connection.server_version()).await.unwrap();
```

`src/ui/date_tests.rs`:

```diff
diff --git a/src/ui/date_tests.rs b/src/ui/date_tests.rs
--- a/src/ui/date_tests.rs
+++ b/src/ui/date_tests.rs
@@ -11,6 +11,7 @@
 use egui::{Key, Modifiers};
 use tabletist_db::{Driver, NewValue, Value, ValueKind};
 
+use crate::backend::{Command, Event};
 use crate::model::{Action, CellPos, ConnTabId, EditStart, Pane, TabId};
 use crate::settings::{Settings, Zone};
 use crate::testing::{Harness, labels};
@@ -601,3 +602,57 @@
         .expect("refused");
     assert!(!problem.warns());
 }
+
+#[test]
+fn a_mysql_session_is_asked_its_zone_once_it_is_up_and_no_other_is() {
+    for (driver, asked) in [
+        (Driver::MySql, true),
+        (Driver::Postgres, false),
+        (Driver::Sqlite, false),
+    ] {
+        let (mut harness, tab, _) = covers(driver, "date", "2026-01-15");
+        harness.app.after_connect(tab);
+        let asks = harness
+            .app
+            .backend
+            .sent
+            .iter()
+            .any(|command| matches!(command, Command::SessionZone { .. }));
+        assert_eq!(asks, asked, "{driver:?}");
+    }
+}
+
+#[test]
+fn a_mysql_timestamp_is_read_in_the_zone_the_session_keeps_it_in() {
+    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
+    let session = harness.app.workspace(tab).unwrap().session;
+    let request = crate::backend::RequestId(9_000);
+    harness
+        .app
+        .workspace_mut(tab)
+        .unwrap()
+        .session_zone
+        .start(request);
+    let berlin = jiff::tz::TimeZone::get("Europe/Berlin").unwrap();
+    harness.app.apply(Action::Backend(Event::SessionZone {
+        session,
+        request,
+        result: Ok(Some(berlin)),
+    }));
+    type_in(&mut harness, tab, id, "2026-03-29 02:30");
+    assert!(says(
+        &mut harness,
+        "02:30 doesn't exist on 29 Mar in Berlin; clocks go forward."
+    ));
+    // Sent as the session's clock writes it, which is what the cell shows.
+    type_in(&mut harness, tab, id, "2026-01-15 10:30");
+    assert!(says(
+        &mut harness,
+        "Thu 15 Jan 2026, 10:30 CET · sent as 2026-01-15 10:30:00"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-01-15 10:30:00")
+    );
+}
```

`src/zones.rs`:

```diff
diff --git a/src/zones.rs b/src/zones.rs
--- a/src/zones.rs
+++ b/src/zones.rs
@@ -56,6 +56,23 @@
     use super::*;
 
     #[test]
+    fn a_servers_zone_is_read_where_it_names_rules_or_an_offset() {
+        assert_eq!(session("UTC"), Some(TimeZone::UTC));
+        let berlin = session("Europe/Berlin").expect("a zone");
+        assert_eq!(berlin.iana_name(), Some("Europe/Berlin"));
+        let offset = |seconds| {
+            Some(TimeZone::fixed(
+                jiff::tz::Offset::from_seconds(seconds).unwrap(),
+            ))
+        };
+        assert_eq!(session("+00:00"), offset(0));
+        assert_eq!(session("-05:30"), offset(-19_800));
+        assert_eq!(session("CET"), None);
+        assert_eq!(session("EST"), None);
+        assert_eq!(session("Mars/Olympus"), None);
+    }
+
+    #[test]
     fn the_database_is_read_whole_and_found_in_any_case() {
         let zones = Zones::load();
         assert!(zones.names().len() > 100);
```


- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib` and `cargo test --locked -p tabletist-db --tests`
Expected: no build of either, with errors such as:

```
error[E0599]: no variant named `SessionZone` found for enum `backend::Command`
error[E0599]: no variant named `SessionZone` found for enum `backend::Event`
error[E0609]: no field `session_zone` on type `&mut Workspace`
error[E0425]: cannot find function `session` in this scope
error: could not compile `tabletist` (lib test) due to 10 previous errors
error[E0599]: no method named `session_zone` found for struct `tabletist_db::Connection` in the current scope
error: could not compile `tabletist-db` (test "mysql") due to 1 previous error
```

- [ ] **Step 3: Write the code and the page**

`crates/tabletist-db/src/adapter.rs`:

```diff
diff --git a/crates/tabletist-db/src/adapter.rs b/crates/tabletist-db/src/adapter.rs
--- a/crates/tabletist-db/src/adapter.rs
+++ b/crates/tabletist-db/src/adapter.rs
@@ -18,6 +18,8 @@
     fn cancel_handle(&self) -> CancelHandle;
 
     async fn server_version(&self) -> Result<String>;
+
+    async fn session_zone(&self) -> Result<Option<String>>;
 
     async fn list_databases(&self) -> Result<Vec<String>>;
 
```

`crates/tabletist-db/src/lib.rs`:

```diff
diff --git a/crates/tabletist-db/src/lib.rs b/crates/tabletist-db/src/lib.rs
--- a/crates/tabletist-db/src/lib.rs
+++ b/crates/tabletist-db/src/lib.rs
@@ -312,6 +312,18 @@
         }
     }
 
+    /// The zone the server moves a moment it keeps without an offset to
+    /// and from, as the server names it (`UTC`, `+00:00`, `Europe/Berlin`,
+    /// or the system's abbreviation, `CET`): MySQL's `TIMESTAMP`. `None`
+    /// for an engine that keeps no such moment.
+    pub async fn session_zone(&self) -> Result<Option<String>> {
+        match &self.inner {
+            Inner::Sqlite(conn) => conn.session_zone().await,
+            Inner::Postgres(conn) => conn.session_zone().await,
+            Inner::MySql(conn) => conn.session_zone().await,
+        }
+    }
+
     /// Databases the session can switch to. Empty when switching does not
     /// apply (SQLite, and MySQL where databases are listed as schemas).
     pub async fn list_databases(&self) -> Result<Vec<String>> {
```

`crates/tabletist-db/src/mysql.rs`:

```diff
diff --git a/crates/tabletist-db/src/mysql.rs b/crates/tabletist-db/src/mysql.rs
--- a/crates/tabletist-db/src/mysql.rs
+++ b/crates/tabletist-db/src/mysql.rs
@@ -201,6 +201,19 @@
     /// See [`crate::Connection::server_version`]. Asked at connect.
     async fn server_version(&self) -> Result<String> {
         Ok(version_name(&self.version))
+    }
+
+    /// See [`crate::Connection::session_zone`]: the session's zone, or the
+    /// server's own where the session follows it (`SYSTEM`).
+    async fn session_zone(&self) -> Result<Option<String>> {
+        let zones: Vec<String> = self
+            .catalog(
+                "SELECT IF(@@session.time_zone = 'SYSTEM', @@global.system_time_zone, \
+                 @@session.time_zone)",
+                (),
+            )
+            .await?;
+        Ok(zones.into_iter().next())
     }
 
     /// None: MySQL's databases are listed as its schemas.
```

`crates/tabletist-db/src/pg.rs`:

```diff
diff --git a/crates/tabletist-db/src/pg.rs b/crates/tabletist-db/src/pg.rs
--- a/crates/tabletist-db/src/pg.rs
+++ b/crates/tabletist-db/src/pg.rs
@@ -498,6 +498,12 @@
             token: self.cancel.clone(),
             tls: self.tls.clone(),
         })
+    }
+
+    /// None: a `timestamptz` comes with its offset, and the app shows it in
+    /// the zone of its own settings.
+    async fn session_zone(&self) -> Result<Option<String>> {
+        Ok(None)
     }
 
     /// See [`crate::Connection::server_version`].
```

`crates/tabletist-db/src/sqlite.rs`:

```diff
diff --git a/crates/tabletist-db/src/sqlite.rs b/crates/tabletist-db/src/sqlite.rs
--- a/crates/tabletist-db/src/sqlite.rs
+++ b/crates/tabletist-db/src/sqlite.rs
@@ -948,6 +948,11 @@
             Ok(format!("SQLite {version}"))
         })
         .await
+    }
+
+    /// None: SQLite keeps a moment as the text or the number it is given.
+    async fn session_zone(&self) -> Result<Option<String>> {
+        Ok(None)
     }
 
     /// None: a file is one database.
```

`docs/_guide/editing-data.md`:

```diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -43,14 +43,102 @@
 and the syntax of JSON. A foreign key you type is also looked up in the
 table it points at, as a [warning](#foreign-keys).
 
-### Dates and times with a zone
+### Dates and times
+
+A date, a time or a timestamp is typed, and a line under the editor says
+what will be stored as you type: `Thu 15 Jan 2026, 09:30 EST · sent as
+2026-01-15 09:30:00-05`. What is sent is always written the same way for
+the column's type, whatever you typed. Enter is refused while the text is
+no value of the column, and the line says why: `There is no 30 February.`
+
+| Column | PostgreSQL | MySQL | SQLite (declared type) |
+| --- | --- | --- | --- |
+| Date | `date` | `DATE` | holds `DATE` and not `TIME` |
+| Date and time | `timestamp` | `DATETIME` | `DATETIME`, `TIMESTAMP`, or holds `DATE` and `TIME` |
+| Date and time in a zone | `timestamptz` | `TIMESTAMP` | |
+| Time | `time`, `timetz` | `TIME` | `TIME` |
+
+An `interval`, MySQL's `YEAR` and an array of dates are edited as text.
+
+You can type:
+
+- ISO 8601, with a space or `T`, with or without seconds, a fraction and
+  an offset: `2026-01-15 09:30`, `2026-01-15T09:30:00.25Z`;
+- a day with its month's name: `15 Jan 2026`, `Jan 15 2026`,
+  `January 15, 2026`, or without a year for this year's;
+- a day in numbers, in your system's order: `15/01/2026` or `01/15/2026`,
+  and `2026/01/15` anywhere;
+- a time: `9:30`, `9.30`, `9am`, `9:30 pm`, `21:30:15`;
+- a day counted from today: `today`, `tomorrow`, `yesterday`, `now`,
+  `+7d`, `-2w`, `+1m`, `+1y`, `next fri`, `last mon`, with a time after
+  it: `tomorrow 9am`.
+
+A day counted from today is stored as the day it is, never as the word:
+the line says it, `Fri 30 Oct 2026 · in 6 days`. Today is the one of the
+zone of [Time zone for timestamptz]({% link _reference/settings-and-files.md %}#the-settings-window),
+for every date column. A month moved past its end stops at the last day:
+31 January and `+1m` is 28 February. A day alone in a timestamp is its
+midnight, and a time alone today's. `now()` and `current_timestamp` are
+still the database's time of the save; `now` alone is the time it is as
+you type it.
+
+What a column cannot keep is refused, with why: a time typed into a
+date, a day into a time, an offset into a `timestamp` (SQLite keeps one
+in its text). Words the servers read themselves, such as `epoch` or a
+zone's name after a time, are not read here: write the moment.
+
+#### Zones
 
 A PostgreSQL `timestamptz` is shown, and opens to be edited, in the zone
 of [Time zone for timestamptz]({% link _reference/settings-and-files.md %}#the-settings-window),
-with that zone's offset. A value you type without an offset is sent with
-the zone's, so the server reads it as the time you see; one with an
-offset is sent as you typed it. Typing back the same moment, in any zone,
-changes nothing.
+and is sent with that zone's offset. One typed with another offset is the
+same moment, sent as that zone shows it. Typing back the same moment, in
+any zone, changes nothing.
+
+A MySQL `TIMESTAMP` is shown and typed in the zone of the session, which
+the server moves it to and from, and is sent as that zone's clock writes
+it. A `timestamp` with no zone, and MySQL's `DATETIME`, are stored as
+typed, whatever the zone.
+
+Where the clocks change, the line says so:
+
+- a time the clocks skip, `02:30 doesn't exist on 8 Mar in New York;
+  clocks go forward.`, is refused until you take the time after it with
+  **Use 03:30**;
+- a time the clocks go back over is two moments: summer time is taken
+  unless you choose the other, `01:30 EDT` or `01:30 EST`. A MySQL value
+  is sent with no offset, so there it is the server's to choose.
+
+Until a MySQL session's zone is known, or where the server names it by an
+abbreviation such as `CET`, which names no rules, a `TIMESTAMP` takes only
+a day and a time written out, with no offset: today there is the
+server's. A `timestamptz` whose zone is not read yet is shown as the
+server sent it and typed in UTC.
+
+#### What a type holds
+
+The line says the limits of a type, and refuses what is outside them:
+
+- MySQL `TIMESTAMP` holds 1970-01-01 00:00:01 to 2038-01-19 03:14:07
+  UTC; `DATE` and `DATETIME` hold 1000-01-01 to 9999-12-31;
+- PostgreSQL holds days from 24 November 4714 BC, typed `4714-11-24 BC`.
+  Years after 9999 are not read;
+- PostgreSQL's `infinity` and `-infinity` are typed as they are, and said
+  as `No end date · stored as infinity`;
+- PostgreSQL's and SQLite's `24:00:00` is the end of a day, and a time
+  rounded up past `23:59:59` is it; a MySQL `TIME` past a day or below
+  zero (`100:30:00`, `-12:00`) is a span of time, kept to ±838:59:59;
+- a MySQL zero date, `0000-00-00`, is shown as it is stored and cannot be
+  typed: give a date or set NULL.
+
+SQLite keeps a date as text or as a number. A new value is written the
+way the column's values already are: seconds since 1970 where they are
+whole numbers, a Julian day (as `julianday()` gives it) where they are
+real numbers, and ISO 8601 text otherwise or where there are none
+(`2026-01-15` or `2026-01-15 09:30:00`). A time kept as a number is
+seconds of the day, or the Julian day of its time on 1 January 2000. A
+value kept as a number opens as the day, the time, or both, that it is.
+The values you do not edit keep theirs.
 
 ### Words that are not text
 
@@ -61,7 +149,7 @@
 | --- | --- |
 | `NULL` | NULL, where the column takes it |
 | `DEFAULT` | The column's default, where it has one |
-| `now()`, `now`, `current_timestamp` | The moment of the save by the database's own clock, in a date or time column |
+| `now()`, `current_timestamp` | The moment of the save by the database's own clock, in a date or time column |
 
 The cell shows the word until you save, and what the database stored
 after. Review SQL shows the keyword as it is sent.
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -3753,6 +3753,7 @@
                             .is_some_and(|workspace| std::mem::take(&mut workspace.woken));
                         if woken {
                             self.ask_server_version(tab);
+                            self.ask_session_zone(tab);
                         } else {
                             self.after_connect(tab);
                         }
@@ -4412,6 +4413,17 @@
                     workspace.server_version.finish(request, result);
                 }
             }
+            Event::SessionZone {
+                session,
+                request,
+                result,
+            } => {
+                if let Some(tab) = self.tab_for_session(session)
+                    && let Some(workspace) = self.workspace_mut(tab)
+                {
+                    workspace.session_zone.finish(request, result);
+                }
+            }
             Event::Written {
                 session,
                 request,
@@ -5513,6 +5525,27 @@
         }
     }
 
+    /// The zone a MySQL session moves its `TIMESTAMP` values to and from:
+    /// asked for once a session is up, where it is not known. The other
+    /// engines keep no such moment.
+    fn ask_session_zone(&mut self, tab: ConnTabId) {
+        let Some(workspace) = self.workspace(tab) else {
+            return;
+        };
+        let asks = match workspace.driver {
+            Driver::MySql => workspace.session_zone.needs_load(),
+            Driver::Postgres | Driver::Sqlite => false,
+        };
+        if asks {
+            let session = workspace.session;
+            let request = RequestId(self.next_id());
+            if let Some(workspace) = self.workspace_mut(tab) {
+                workspace.session_zone.start(request);
+            }
+            self.backend.send(Command::SessionZone { session, request });
+        }
+    }
+
     /// After a (re)connect: load the tree, restart everything that waited on
     /// the old session, and open anything queued for this connection.
     pub fn after_connect(&mut self, tab: ConnTabId) {
@@ -5532,6 +5565,7 @@
             }
         }
         self.ask_server_version(tab);
+        self.ask_session_zone(tab);
         self.refresh_tree(tab);
         let Some(workspace) = self.workspace_mut(tab) else {
             return;
@@ -7494,6 +7528,7 @@
                     // answered, is asked of the next session, and so is
                     // its zone.
                     workspace.server_version.pending = None;
+                    workspace.session_zone.pending = None;
                     due.push(workspace.session);
                 }
             }
```

`src/backend.rs`:

```diff
diff --git a/src/backend.rs b/src/backend.rs
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -163,6 +163,13 @@
         session: SessionId,
         request: RequestId,
     },
+    /// The zone the server moves MySQL's `TIMESTAMP` values to and from,
+    /// for the editor of one: its changes of the clocks, and its limits in
+    /// UTC.
+    SessionZone {
+        session: SessionId,
+        request: RequestId,
+    },
     /// Writes a state file atomically. Saves to one file are written in
     /// order, and a burst of them writes only the newest.
     Save {
@@ -373,6 +380,14 @@
         request: RequestId,
         result: Result<String, Error>,
     },
+    /// The session's zone, read from the zone database: `None` where the
+    /// engine has none, or the server names it in a way the database does
+    /// not know (an abbreviation such as `CET`).
+    SessionZone {
+        session: SessionId,
+        request: RequestId,
+        result: Result<Option<jiff::tz::TimeZone>, Error>,
+    },
     /// A `Save` was written, or why it was not.
     Saved {
         path: PathBuf,
@@ -2113,7 +2128,8 @@
         | Command::Write { session, .. }
         | Command::ListDatabases { session, .. }
         | Command::RunSql { session, .. }
-        | Command::ServerVersion { session, .. } => *session,
+        | Command::ServerVersion { session, .. }
+        | Command::SessionZone { session, .. } => *session,
         Command::Test { .. }
         | Command::LoadSecret { .. }
         | Command::StoreSecret { .. }
@@ -2144,7 +2160,8 @@
         | Command::Write { request, .. }
         | Command::ListDatabases { request, .. }
         | Command::RunSql { request, .. }
-        | Command::ServerVersion { request, .. } => Some(*request),
+        | Command::ServerVersion { request, .. }
+        | Command::SessionZone { request, .. } => Some(*request),
         Command::Connect { .. }
         | Command::Test { .. }
         | Command::Close { .. }
@@ -2233,6 +2250,11 @@
             cancel: None,
         },
         Command::ServerVersion { session, request } => Event::ServerVersion {
+            session,
+            request,
+            result: Err(error),
+        },
+        Command::SessionZone { session, request } => Event::SessionZone {
             session,
             request,
             result: Err(error),
@@ -2519,6 +2541,18 @@
                 let result = connection.server_version().await;
                 let lost = lost_error(&result);
                 outbox.emit(Event::ServerVersion {
+                    session: *session,
+                    request: *request,
+                    result,
+                });
+                lost
+            }
+            Command::SessionZone { session, request } => {
+                let result = connection.session_zone().await;
+                let lost = lost_error(&result);
+                // The zone database is read here, on the backend.
+                let result = result.map(|named| named.as_deref().and_then(crate::zones::session));
+                outbox.emit(Event::SessionZone {
                     session: *session,
                     request: *request,
                     result,
```

`src/edit.rs`:

```diff
diff --git a/src/edit.rs b/src/edit.rs
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -128,6 +128,9 @@
     /// The zone a moment with an offset is shown and typed in, where the
     /// engine sends one and the zone is known (`Workspace::zone_shown`).
     pub zone: Option<&'a jiff::tz::TimeZone>,
+    /// The zone a MySQL session moves its `TIMESTAMP` values to and from,
+    /// once it is known (`Workspace::session_zone`).
+    pub session_zone: Option<&'a jiff::tz::TimeZone>,
 }
 
 impl<'a> Table<'a> {
@@ -148,6 +151,11 @@
             added: &object.edits.added,
             deleted: &object.edits.deleted,
             zone: workspace.zone_shown(),
+            session_zone: workspace
+                .session_zone
+                .value
+                .as_ref()
+                .and_then(Option::as_ref),
         })
     }
 }
@@ -188,7 +196,8 @@
 
     /// The page's column `col` as an editor reads dates and times for it,
     /// where it keeps them: the zone a PostgreSQL `timestamptz` is shown
-    /// in, and the values SQLite's follow.
+    /// in, the one a MySQL `TIMESTAMP` is kept in, and the values SQLite's
+    /// follow.
     pub fn dates(&self, col: usize) -> Option<crate::dates::Column> {
         let column = self.column(col)?;
         let values = self.page.rows.iter().filter_map(|row| row.get(col));
@@ -197,7 +206,8 @@
         // offset all the same, which the server reads as it is.
         let zone = match self.dialect {
             Dialect::Postgres => Some(self.zone.unwrap_or(&UTC)),
-            Dialect::MySql | Dialect::Sqlite => None,
+            Dialect::MySql => self.session_zone,
+            Dialect::Sqlite => None,
         };
         crate::dates::Column::of(self.dialect, &column.type_name, zone, values)
     }
@@ -3243,6 +3253,7 @@
             added: &edits.added,
             deleted: &edits.deleted,
             zone: Some(&crate::testing::UTC),
+            session_zone: None,
         }
     }
 
@@ -4874,6 +4885,7 @@
             added: &[],
             deleted: &NONE_DELETED,
             zone: Some(&crate::testing::UTC),
+            session_zone: None,
         }
     }
 
```

`src/key_picker.rs`:

```diff
diff --git a/src/key_picker.rs b/src/key_picker.rs
--- a/src/key_picker.rs
+++ b/src/key_picker.rs
@@ -1364,6 +1364,7 @@
                     added: &edits.added,
                     deleted: &edits.deleted,
                     zone: Some(&crate::testing::UTC),
+                    session_zone: None,
                 },
                 &own,
                 edits,
@@ -1461,6 +1462,7 @@
             added: &edits.added,
             deleted: &edits.deleted,
             zone: Some(&crate::testing::UTC),
+            session_zone: None,
         };
         assert!(unmatched(&table, &own, &edits).is_empty());
         // Another table's key of the same number is no row of this one.
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -1143,6 +1143,10 @@
     pub next_query: u32,
     /// "PostgreSQL 17.2", asked for when the first SQL editor opens.
     pub server_version: Fetch<String>,
+    /// The zone a MySQL session moves its `TIMESTAMP` values to and from,
+    /// asked for once the session is up: `Some(None)` where the server
+    /// names one the zone database does not know.
+    pub session_zone: Fetch<Option<jiff::tz::TimeZone>>,
     /// The columns of tables and views, for the SQL editors' completion:
     /// fetched for the tables a statement names, and filled by a table
     /// tab's structure. A fetch that failed stays failed, so it is not
@@ -5376,6 +5380,7 @@
             tabs: Vec::new(),
             next_query: 1,
             server_version: Fetch::default(),
+            session_zone: Fetch::default(),
             columns: HashMap::new(),
             catalog_generation: 0,
             active_tab: None,
@@ -5734,6 +5739,7 @@
             object.edits.holders.clear();
         }
         self.server_version = Fetch::default();
+        self.session_zone = Fetch::default();
         // Fetches the old session will never answer, and names that may
         // not be the new one's.
         self.columns.clear();
```

`src/review.rs`:

```diff
diff --git a/src/review.rs b/src/review.rs
--- a/src/review.rs
+++ b/src/review.rs
@@ -762,6 +762,7 @@
             added: &[],
             deleted: &crate::edit::NONE_DELETED,
             zone: Some(&crate::testing::UTC),
+            session_zone: None,
         }
     }
 
```

`src/zones.rs`:

```diff
diff --git a/src/zones.rs b/src/zones.rs
--- a/src/zones.rs
+++ b/src/zones.rs
@@ -51,6 +51,31 @@
     }
 }
 
+/// The zone a database server names `name` (MySQL's session zone):
+/// `UTC`, an offset such as `+00:00` or `-05:30`, or a zone of the
+/// database by its name. `None` for anything else, an abbreviation such
+/// as `CET` or `EST` above all: it names no rules of the clocks, and the
+/// zone of the database by that name is not the server's. Disk work, for
+/// the backend.
+pub fn session(name: &str) -> Option<TimeZone> {
+    let name = name.trim();
+    if name.eq_ignore_ascii_case("utc") || name.eq_ignore_ascii_case("gmt") {
+        return Some(TimeZone::UTC);
+    }
+    if let Some(sign) = name.strip_prefix(['+', '-']).map(|_| &name[..1]) {
+        let (hours, minutes) = name[1..].split_once(':')?;
+        let (hours, minutes) = (hours.parse::<i32>().ok()?, minutes.parse::<i32>().ok()?);
+        let seconds = (hours * 60 + minutes) * 60 * if sign == "-" { -1 } else { 1 };
+        return Some(TimeZone::fixed(
+            jiff::tz::Offset::from_seconds(seconds).ok()?,
+        ));
+    }
+    if !name.contains('/') {
+        return None;
+    }
+    jiff::tz::db().get(name).ok()
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```


- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`, then `cd docs && bin/check`.
Expected: clean; the library's tests `test result: ok. 3354 passed; 0 failed; 17 ignored`; with the servers, `the_session_names_the_zone_its_timestamps_are_kept_in`, `postgres_sends_its_moments_with_their_offsets_and_names_no_zone` and `sqlite_keeps_no_zone_for_its_moments` pass.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-db src/backend.rs src/zones.rs src/model.rs src/app.rs src/edit.rs src/key_picker.rs src/review.rs src/ui/date_tests.rs docs/_guide/editing-data.md
git commit -m "Read a MySQL TIMESTAMP in the zone of its session"
```

---

## After Task 3

Push the branch and open the pull request. Its description has what the prompt's Delivery asks: the summary, the type mapping per driver (the table at the top of `docs/_guide/editing-data.md`'s Dates and times), zones and clock changes with jiff, the SQLite rule, no setting or key in this run (runs 2 and 3), the open questions with what this plan says of each, and the departures above. No screenshot.

## Checks by hand

Off the record, on the demo database (Bookshop) on each engine and in both looks: open `books.published` (a date) and `orders.placed_at` (a timestamp, `DATETIME` on MySQL and SQLite); type `tomorrow 9am`, `15/01/2026`, `2026-02-30`, `+7d`. The popover sits under the cell, and under the row panel's field, clear of the window's edge; Esc, Enter and a click away do what they did; a refused text keeps the field red. The Bookshop has no `timestamptz`: the zones are the tests'. Nothing of it is a test.
