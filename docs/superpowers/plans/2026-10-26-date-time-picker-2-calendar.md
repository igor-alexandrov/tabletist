# The date and time picker, run 2: the calendar Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Under the line that says what a date, a time or a timestamp is stored as, a calendar: on macOS a popover with the month the text is in (the new value filled, the value before the edit marked with a dot, today outlined, days the type does not hold disabled), the time fields with the zone and **Now**, and a footer with NULL, DEFAULT and PostgreSQL's infinities; in the Omarchy look the month and the time painted under the float's line. On macOS a click picks a day, Now or a value, and every pick rewrites the field's text, as typing would; the Omarchy calendar is drawn for the eye, and its keys come with run 3. The weeks start on the locale's first day. A setting, on by default, shows it: **Show a calendar when editing dates**, and `:set calendar` / `:set nocalendar` in the Omarchy look.

**Architecture:** `src/dates.rs` (no egui) gains what a calendar needs of a column: the first weekday of a region (CLDR's `weekData`), the six weeks a month is shown in, the month a text that does not read names so far (`so_far`), the text a picked day or Now writes (`picked`, `now_text`), and whether the type holds a day (`holds`, asked of the reader itself). `src/ui/calendar.rs` makes a `Model` of the editor's text each frame (selected, was, today, dimmed, the time a pick keeps, the time fields, the zone, what the footer offers) and draws it inside run 1's popover (`date_line::show`), under the line: on macOS with day buttons in an AccessKit grid, spin buttons and buttons; in the terminal look painted only. The month shown is the view's own (egui's memory, by the field and the cell), following each new text unless the arrows took it elsewhere since. A pick only rewrites `Editor::text` (and NULL or DEFAULT commits it), so the reducer reads it as it reads typing. The editor keeps the value before the edit (`Editor::was`). The cell editor's `Target` carries the calendar (`cell_editor::Calendar`: the clock and the week's first day) where `Settings::date_picker` is on, and then has no database "now" button. `data.date_picker` is a key of `settings.toml`, a row of Settings › General, and two lines of the Omarchy prompt.

**Tech Stack:** Rust, egui (crmne fork) with AccessKit, jiff (its civil dates and zones), headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-25-date-time-picker-design.md`: run 2 of the three at its top.

---

## Before Task 1

Commit this plan:

```bash
git add docs/superpowers/plans/2026-10-26-date-time-picker-2-calendar.md
git commit -m "Plan the date and time picker, run 2: the calendar"
```

## Where this sits

The prompt asked for one pull request; you chose three runs, a pull request each. Run 1 (typed dates, igor-alexandrov/tabletist#224) merged on 2026-10-26. This is run 2, a branch of `main` at `a7ca990b`. Run 3 (the picker's keys: macOS Tab into the month and the time fields, Page Up and Down, T; Omarchy's chords, Ctrl+T, Ctrl+A and Ctrl+X, Tab for the clock changes; the keymap and the pages) builds on what this one merges.

## Decisions and departures, for a yes or a no before code

The first three are the ones the run was asked to put to you; the plan is written with the recommendation of each.

| # | The prompt or the boards | Built (recommended) | Why, and the other way |
|---|---|---|---|
| 1 | The setting is `editor.typing.date_picker`, in Settings › Editor › Typing | `data.date_picker`, in Settings › General › Data, after Value tags: **Show a calendar when editing dates**, on by default | `[editor.typing]` and the Editor pane are the SQL editor's ("The SQL editor: how queries run, how typing behaves"); General › Data is where how the grid's values are shown and edited lives, beside the zone `timestamptz` is typed in. The prompt's way is the key's table (`Key::path`) and the section the row is listed in (`OptionId::DATA` for `TYPING`, `SettingsPane::rows`): a few lines |
| 2 | `:set nocalendar` / `:set calendar`, saved to the same key | Two lines of the keymap, `:set calendar` and `:set nocalendar` (Omarchy, from any tab, as `:keys default` is), run by `run_command` as Settings' switch is (`Action::SetOption`), so they are saved the same way. The terminal Settings screen's row shows them as its hint | Omarchy has no `:set` yet. A `:set` that reads any option by its name is a feature of its own (names, values, completion, errors); these two are what the prompt asks for, and the keymap's checks hold them |
| 3 | (the **now** button beside a date cell's field, run 1's: the database's time of the save) | Not shown while the calendar is: the calendar's **Now** writes the time it is into the text (a date takes today by its day in the month), and `now()` typed is still the database's. With the setting off, the button is as before | Two buttons called now that store different things would be one too many. The other ways: keep both, or keep the button and leave Now out of the time row (the boards draw it in the row) |
| 4 | Boards: a month of five weeks where it fits (January 2026), six where it needs them (March 2026) | Six weeks always, as the prompt says ("a 6-row grid") | The prompt wins; the popover keeps its height as ‹ and › move it |
| 5 | Footer: "On PostgreSQL date and timestamp, −∞ and ∞ are on the left"; the Special values board has them in the hints' place; the grid board's `timestamptz` has none | On PostgreSQL date, `timestamp` and `timestamptz`, −∞ and ∞ on the left in the place of ↩ Apply and esc Cancel | A `timestamptz` takes `infinity` as the others do, and run 1 reads it there. 330 wide holds the hints, ∞, NULL and DEFAULT only without the hints, as the board draws it |
| 6 | Boards: **NULL ⌘⌫**, DEFAULT's tooltip `⌘'` | NULL and DEFAULT with no key beside them | While the field has the keyboard ⌘⌫ deletes to the line's start. The picker's keys are run 3's, with the keymap's checks |
| 7 | (what NULL, DEFAULT and ∞ do once clicked) | NULL and DEFAULT rewrite the text to `NULL` or `DEFAULT` and commit it at once, the editor closing, as the foreign key list's Set NULL does. −∞ and ∞ rewrite it to `-infinity` or `infinity` and leave the edit open: the line says "No end date · stored as infinity" and ↩ applies | NULL and DEFAULT are no day the calendar could show; the infinities are values of the column the line describes |
| 8 | The gap board: **Use 03:30** in the time row, in the place of the zone and Now | Under the line, where run 1 put it, the time row as ever | One place for it, whether the calendar shows or not, and the Omarchy float has no time row for a button |
| 9 | "The month dims while the text does not read" | Dimmed, its days still take a click: an empty text (a new row's cell), `infinity`, and MySQL's zero date (whose line says "pick a date or set NULL") take a day so. The time a pick keeps is the one typed so far (`2026-02-30 09:30` and a click on 15 is `2026-02-15 09:30`), else midnight. A day the type does not hold is disabled and takes none | A dimmed month that took no click would leave an empty cell no way to its first day by the pointer |
| 10 | Time fields: ↑ and ↓ step the focused part; the board draws a focused one | Drawn, and spin buttons with their range and value for a screen reader; nothing steps them in this run | Stepping is by keys (↑, ↓, digits, Ctrl+A and Ctrl+X): run 3's. A field focused in the month or the time is too |
| 11 | "Fractional seconds show when the precision is above 0 or the value has them" | Where the type states a precision above 0 (`timestamp(6)`, `datetime(3)`) or the value has a fraction | A PostgreSQL `timestamp` keeps six digits unless it says otherwise; read as "the precision", every one would show `.000000`, which the grid board's `timestamptz` does not |
| 12 | Zone chip: the city, or "no zone"; the MySQL TIMESTAMP card draws neither chip nor Now | The zone's place ("New York"), "no zone" for a `timestamp`, a `timetz`'s offset, and for a MySQL `TIMESTAMP` whose session's zone is not known "the server's zone" and no Now | What time it is there is the server's to say: run 1 refuses `now` for such a column |
| 13 | Omarchy float: a `> 2026-01-15 09:30` line, the month, `time 09:30:00 est · utc−05  ctrl+t now`, and a status row of keys (`-- INSERT --  ctrl+h/l day …`) | The head and the line (run 1's), the month, and for a timestamp `time  09:30:00  est · utc-05`; no copy of the typed text (run 1's departure), no key hints and no status row; a time has no time row, as its board has none. The value before the edit is on the line: `was 15 jan` | The keys are run 3's, and the window's own mode line says the mode. `utc-05` is written with run 1's offsets' minus |
| 14 | "Today in the header shows today's month without changing the value" | ‹, › and Today move the month shown; it stays there until the text names another day. A text that names no month (`soon`) leaves the month where it was | The month a person moved to stays theirs while they type nothing that says otherwise |
| 15 | The popover is 330 wide | 330 while the calendar shows; run 1's width (as wide as the cell, to 495) with the line alone | The board's |
| 16 | The design system's tokens | Those the palette has; where it has no slot: `ink-disabled` the palette's `faint` (its disabled text), `accent-border-strong` the window drawn toward the accent (0.45, as the row panel's dashes), `warning-mark` `warning` (the pending stripe's), `accent-strong` `accent_hover`, `surface-sunken` `panel`, `surface-muted` `surface` | The palette is the app's mapping of the tokens; a new slot is a change of every theme |
| 17 | "The first weekday … from the system locale" | The region of the locale for times (`LC_ALL`, `LC_TIME`, then the system's languages, as run 1's day order), by CLDR's `weekData` (Sunday in the US, Saturday in Egypt, Friday in the Maldives, Monday where none says) | macOS's own First day of week (System Settings) and Windows's are not read: that needs AppKit or the Windows API, behind their crates. Monday in tests |
| 18 | Test 3: "month moves clamp" | Not in this run | No pointer action moves the value by a month: ‹ and › move the month shown, as the prompt says Today does. The clamp (31 Jan and a month is 28 Feb) is Alt+H and Alt+L's, run 3's, and `+1m` typed already holds it (run 1) |
| 19 | (a window too short for the popover) | As run 1: under the cell, or over it where there is room above; a window too short for either cuts its foot | Over the field it would hide what is typed |
| 20 | "Pointer picking" (and run 1's offers are buttons in both looks) | The Omarchy month and time row take no click: they are painted, as the board's "the calendar never takes focus and has no cursor of its own". Run 1's offers (Use 03:30, the two of a repeated hour) stay buttons there | The Omarchy picker is driven by its keys, run 3's; a pointer there would be a second way the board does not draw. The other way: day cells that take a click, with no node |
| 21 | (the time row's room) | The zone chip gives way where the row has no room for it beside the fields and Now (a stated fraction and a long place name); the line says the zone either way | 330 wide holds six fields of a `timestamptz(6)`, Now and a short place |
| 22 | Other months' days in `ink-disabled` | Drawn so, and they take a click as the month's own do: a pick of 2 February from January's last week writes it and shows February | They are days like the others; only those the type does not hold are disabled. A MySQL `TIME` past a day (a span) shows `--` in its fields |
| 23 | "the value from before this edit marked with a small `warning-mark` dot" | The value the cell showed when the edit began: its pending value where it has one, else the one it loaded. The Omarchy line says it as `was 15 jan` | What the person saw in the cell is what "before" means to them |

The weekday and month names go through `gettext` ("Mo", "Monday", "January"); English is the app's only catalog. Spec tests 3 (a day keeps the time, Now), 10, 11 and 12 are headless, in `src/ui/calendar_tests.rs`; no colour, size or picture is checked anywhere.

A reviewer given the draft broke it with tests of its own, in a copy: four defects, which the draft now fixes, each with a test that fails without the fix:

- **Now**, pressed in the second 01:30 of 1 November (EST), wrote `2026-11-01 01:30`, which reads as the first (EDT): an hour off. A day picked on a text whose repeated hour was chosen as EST dropped the choice the same way. `picked` keeps the text's offset where the time is a repeated hour's and the offset one of its two, and `now_text` writes the clock's (`the_second_of_a_repeated_hour_stays_the_second_when_picked_or_now`, and `a_pick_writes_its_day_and_keeps_the_time`).
- SQLite offered the days before 0000 (‹ from January of year 0): a click wrote `-001-12-06`, which no reader takes. `holds` now asks that the picked text reads (or is a time the clocks skip), not only that no limit refuses it (`a_day_the_type_does_not_hold_cannot_be_picked`).
- After the arrows, a new text in the month the calendar had been on (or Now there) left the month where the arrows took it, the selected day out of sight. The month follows each new text that names one (`the_month_follows_each_new_text_and_is_each_cells_own`).
- The month the arrows reached was kept by the tab's one field, so an editor opened on another cell in the next frame (a new row's, by **Add row**) started on it. It is kept by the field and the cell (same test, a NULL cell).

What held, among what it tried: picks across 1 BC and 1 AD, PostgreSQL's first day, December 9999 (› disabled, no panic), the last `timestamptz` days of 9999 in New York; MySQL `TIMESTAMP`'s edges with and without a known session zone; SQLite seconds and Julian days written as the column keeps them; a pick from a time the clocks skip, with a `timestamp(3)` fraction, with an offset typed; `timetz` Now in winter; NULL and DEFAULT in the row panel and a new row's cell; no ∞ on MySQL or SQLite; the setting turned off under an open editor; `:set` from a SQL tab; the Settings rows in both looks; no node of the Omarchy calendar; a press on any control leaving the keyboard in the field; `so_far` and `month_days` on odd, huge and non-ASCII input.

A second reviewer read this plan against its diffs and the spec. Besides the first three above, which it found too, the plan now takes in:

- The Omarchy float takes no click, which the table did not say (row 20), and the Goal said "a click picks" of both looks.
- The guide said the weeks start on "your system's first day of the week" (it is the locale's region's), that a fraction shows "where the type keeps one" (where it states one, or the value has one), that every click rewrites the text (NULL and DEFAULT apply at once), and gave a time a month. Rewritten.
- Untested behaviour now has tests: a pick in a dimmed month keeping the time typed so far (row 9), the server's zone with no Now and a `timetz`'s offset (row 12), the line alone after `:set nocalendar`, and the keymap's own test of the two lines.
- Rows for what the diffs did unsaid: the chip giving way where the time row has no room (21), the other months' days taking clicks (22), the dot marking a pending value (23).
- A press on a calendar control reads as the editor's on its release frame too, as run 1's offers do; the day and time fields' ids are salted by the editor's cell; the fraction is a flag of its field, not its name compared; ‹ and › at the last month say why they are disabled; `SoFar`; `opened_text` read once.

## Files

| File | Task | What |
|---|---|---|
| `src/dates.rs` | 1 | `locale_region`, `first_weekday`, `month_days`, `so_far`, `picked`, `now_text`, `holds`, `Column::fraction` |
| `src/ui/calendar.rs` (new) | 2 | The model and the drawing of the month, the time row and the footer |
| `src/ui/date_line.rs` | 2 | The calendar under the line; `was` on the terminal's line; picks that commit |
| `src/ui/cell_editor.rs` | 2 | `Calendar`, `Target::{calendar, nullable, default}`; now only without the calendar |
| `src/ui/data_view.rs`, `src/ui/row_form.rs`, `src/ui/row_panel.rs` | 2 | The calendar given to the target |
| `src/edit.rs`, `src/app/editing.rs` | 2 | `Editor::was`, made where the editor opens (`opened_text`) |
| `src/app.rs` | 2 | `App::week_start`, `App::calendar` |
| `src/settings.rs` | 2, 3 | `data.date_picker`; then its option and value |
| `src/ui/calendar_tests.rs` (new), `src/ui/date_tests.rs`, `src/ui/mod.rs` | 2, 3 | Behaviour, headless; the now button's test with the calendar off |
| `src/keymap.rs`, `src/app/editing.rs` | 3 | `:set calendar`, `:set nocalendar` |
| `src/model.rs`, `src/ui/settings/{mod,sheet,terminal}.rs`, `src/ui/settings_access_tests.rs` | 3 | The row in Settings › General |
| `docs/_guide/editing-data.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md` | 2, 3 | The calendar, the setting, the lines |

The tests run as before: `cargo test --locked --workspace --all-targets` (no server is needed by this run's). Use `~/.cargo/bin/cargo` where the mise shim fails.

---

### Task 1: Days for a calendar

**Files:**
- Modify: `src/dates.rs`

`locale_region` is run 1's region of the locale, now its own function: `Order::of_locale` and the new `first_weekday` both read it. `month_days` gives the 42 days a month is shown in, from the week of its first day (`None` past what a date holds, so the weeks keep their places). `so_far` reads the month a text that does not read names (`2026-02-30 09:30`, `2026-02`, `feb 2027`, `15 jan`) and the time typed with it. `picked` writes the text a day picked puts in the field, keeping a time and, in a repeated hour, the offset that chose one of its two; `now_text` the text Now puts there, with the offset in the second of a repeated hour (`None` where the zone is the server's); `holds` asks the reader whether it takes a picked day (a time the clocks skip is a day all the same). `Column::fraction` says the type states digits of a second.

- [ ] **Step 1: Write the failing tests**

`src/dates.rs`:

```diff
diff --git a/src/dates.rs b/src/dates.rs
--- a/src/dates.rs
+++ b/src/dates.rs
@@ -2511,4 +2511,153 @@ mod tests {
             "+05:30"
         );
     }
+
+    #[test]
+    fn a_week_starts_on_the_day_its_region_starts_it() {
+        assert_eq!(first_weekday(Some("gb")), Weekday::Monday);
+        assert_eq!(first_weekday(Some("us")), Weekday::Sunday);
+        assert_eq!(first_weekday(Some("eg")), Weekday::Saturday);
+        assert_eq!(first_weekday(Some("mv")), Weekday::Friday);
+        assert_eq!(first_weekday(None), Weekday::Monday);
+        let languages = ["en-US".to_owned()];
+        let region = |time| locale_region(None, time, &languages);
+        assert_eq!(first_weekday(region(None).as_deref()), Weekday::Sunday);
+        assert_eq!(
+            first_weekday(region(Some("de_DE.UTF-8")).as_deref()),
+            Weekday::Monday
+        );
+    }
+
+    #[test]
+    fn a_month_is_six_weeks_from_the_week_of_its_first_day() {
+        let january = month_days(date(2026, 1, 15), Weekday::Monday);
+        assert_eq!(january.len(), 42);
+        assert_eq!(january[0], Some(date(2025, 12, 29)));
+        assert_eq!(january[3], Some(date(2026, 1, 1)));
+        assert_eq!(january[41], Some(date(2026, 2, 8)));
+        let sunday = month_days(date(2026, 1, 15), Weekday::Sunday);
+        assert_eq!(sunday[0], Some(date(2025, 12, 28)));
+        // March 2026 begins on a Sunday: its last day is in the sixth week.
+        let march = month_days(date(2026, 3, 1), Weekday::Monday);
+        assert_eq!(march[6], Some(date(2026, 3, 1)));
+        assert_eq!(march[36], Some(date(2026, 3, 31)));
+        // Past the last day a date holds, the weeks keep their places.
+        let last = month_days(date(9999, 12, 1), Weekday::Monday);
+        assert_eq!(last.iter().flatten().last(), Some(&date(9999, 12, 31)));
+        assert_eq!(last[41], None);
+    }
+
+    #[test]
+    fn a_text_that_does_not_read_names_the_month_typed_so_far() {
+        let at = |type_name, typed| so_far(typed, &column(Dialect::Postgres, type_name), &clock());
+        let february = date(2026, 2, 1);
+        assert_eq!(
+            at("timestamptz", "2026-02-30 09:30"),
+            Some(SoFar {
+                month: february,
+                time: Some(Time::constant(9, 30, 0, 0))
+            })
+        );
+        let month = |type_name, typed| at(type_name, typed).map(|sofar| sofar.month);
+        assert_eq!(month("date", "2026-02"), Some(february));
+        assert_eq!(month("date", "30 feb"), Some(february));
+        assert_eq!(month("date", "feb 2027"), Some(date(2027, 2, 1)));
+        assert_eq!(month("date", "0044-02-30 BC"), Some(date(-43, 2, 1)));
+        assert_eq!(month("date", "2026-13-01"), None);
+        assert_eq!(month("date", "soon"), None);
+        assert_eq!(month("time", "25:00"), None);
+    }
+
+    #[test]
+    fn a_pick_writes_its_day_and_keeps_the_time() {
+        let pg = Dialect::Postgres;
+        let half_past = Some(Time::constant(9, 30, 0, 0));
+        let day = date(2026, 1, 20);
+        assert_eq!(
+            picked(day, half_past, None, &column(pg, "timestamptz")),
+            "2026-01-20 09:30"
+        );
+        assert_eq!(
+            picked(day, None, None, &column(pg, "timestamp")),
+            "2026-01-20 00:00"
+        );
+        assert_eq!(
+            picked(day, half_past, None, &column(pg, "date")),
+            "2026-01-20"
+        );
+        assert_eq!(
+            picked(date(-43, 3, 15), None, None, &column(pg, "date")),
+            "0044-03-15 BC"
+        );
+        let fraction = Some(Time::constant(9, 30, 12, 250_000_000));
+        assert_eq!(
+            picked(day, fraction, None, &column(pg, "timestamp(6)")),
+            "2026-01-20 09:30:12.25"
+        );
+        let clock = clock();
+        let now = |type_name| now_text(&column(pg, type_name), &clock);
+        assert_eq!(now("timestamptz").as_deref(), Some("2026-10-24 10:00"));
+        assert_eq!(now("date").as_deref(), Some("2026-10-24"));
+        assert_eq!(now("timetz").as_deref(), Some("10:00:00-04"));
+        let unknown = Column::of(Dialect::MySql, "timestamp", None, []).unwrap();
+        assert_eq!(now_text(&unknown, &clock), None);
+        // The second 01:30 of 1 November keeps its offset, picked or now;
+        // another day's 01:30 has no second to keep it for.
+        let zoned = column(pg, "timestamptz");
+        let est = Offset::from_hours(-5).ok();
+        let fold = date(2026, 11, 1);
+        assert_eq!(
+            picked(fold, Some(Time::constant(1, 30, 0, 0)), est, &zoned),
+            "2026-11-01 01:30-05"
+        );
+        assert_eq!(
+            picked(day, Some(Time::constant(1, 30, 0, 0)), est, &zoned),
+            "2026-01-20 01:30"
+        );
+        let second = Clock {
+            now: "2026-11-01T06:30:00Z".parse().unwrap(),
+            ..clock.clone()
+        };
+        assert_eq!(
+            now_text(&zoned, &second).as_deref(),
+            Some("2026-11-01 01:30-05")
+        );
+    }
+
+    #[test]
+    fn a_type_that_states_a_fraction_of_a_second_says_so() {
+        let pg = Dialect::Postgres;
+        assert!(column(pg, "timestamp(3)").fraction);
+        assert!(!column(pg, "timestamp").fraction);
+        assert!(!column(pg, "timestamp(0)").fraction);
+        assert!(column(Dialect::MySql, "datetime(6)").fraction);
+    }
+
+    #[test]
+    fn a_calendar_offers_only_the_days_the_type_holds() {
+        let clock = clock();
+        let mysql = |type_name| column(Dialect::MySql, type_name);
+        assert!(!holds(date(999, 12, 31), None, &mysql("date"), &clock));
+        assert!(holds(date(1000, 1, 1), None, &mysql("date"), &clock));
+        // TIMESTAMP's first second is 00:00:01 UTC, 19:00:01 the evening
+        // before in New York.
+        let stamps = mysql("timestamp");
+        assert!(holds(date(1970, 1, 1), None, &stamps, &clock));
+        assert!(!holds(date(1969, 12, 31), None, &stamps, &clock));
+        let evening = Some(Time::constant(20, 0, 0, 0));
+        assert!(holds(date(1969, 12, 31), evening, &stamps, &clock));
+        assert!(!holds(date(2038, 1, 20), None, &stamps, &clock));
+        // SQLite's years run from 0000.
+        let sqlite = column(Dialect::Sqlite, "DATE");
+        assert!(holds(date(0, 1, 1), None, &sqlite, &clock));
+        assert!(!holds(date(-1, 12, 31), None, &sqlite, &clock));
+        let days = column(Dialect::Postgres, "date");
+        assert!(holds(date(-4713, 11, 24), None, &days, &clock));
+        assert!(!holds(date(-4713, 11, 23), None, &days, &clock));
+        // A time the clocks skip is no limit of the type: the line offers
+        // the time after it.
+        let skipped = Some(Time::constant(2, 30, 0, 0));
+        let zoned = column(Dialect::Postgres, "timestamptz");
+        assert!(holds(date(2026, 3, 8), skipped, &zoned, &clock));
+    }
 }
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib dates`
Expected: no build, 39 errors, such as:

```
error[E0425]: cannot find function `first_weekday` in this scope
error[E0425]: cannot find function `month_days` in this scope
error[E0422]: cannot find struct, variant or union type `SoFar` in this scope
error[E0425]: cannot find function `picked` in this scope
error[E0425]: cannot find function `holds` in this scope
error[E0609]: no field `fraction` on type `dates::Column`
```

- [ ] **Step 3: Write the code**

`src/dates.rs`:

```diff
diff --git a/src/dates.rs b/src/dates.rs
--- a/src/dates.rs
+++ b/src/dates.rs
@@ -108,13 +108,18 @@ pub fn kind(dialect: Dialect, type_name: &str) -> Option<Kind> {
     }
 }
 
+/// The digits of a second's fraction a type states: `timestamp(3)`'s.
+fn stated_digits(type_name: &str) -> Option<u8> {
+    type_name
+        .split_once('(')
+        .and_then(|(_, rest)| rest.split(')').next())
+        .and_then(|digits| digits.trim().parse::<u8>().ok())
+}
+
 /// The digits of a second's fraction a column keeps: those its type
 /// states, else the engine's own. SQLite keeps what it is given.
 fn kept_digits(dialect: Dialect, type_name: &str) -> u8 {
-    let stated = type_name
-        .split_once('(')
-        .and_then(|(_, rest)| rest.split(')').next())
-        .and_then(|digits| digits.trim().parse::<u8>().ok());
+    let stated = stated_digits(type_name);
     match (dialect, stated) {
         (Dialect::Postgres | Dialect::MySql, Some(digits)) => digits.min(6),
         (Dialect::Postgres, None) => 6,
@@ -234,6 +239,9 @@ pub struct Column {
     pub kind: Kind,
     /// The digits of a second's fraction it keeps.
     pub digits: u8,
+    /// Its type states a fraction of a second (`timestamp(3)`): a
+    /// calendar's time fields show one whatever the value.
+    pub fraction: bool,
     /// How SQLite's values of it are kept ([`Stored::Fresh`] elsewhere).
     pub stored: Stored,
     /// The zone a [`Kind::Zoned`] value is shown and typed in: the
@@ -275,6 +283,7 @@ impl Column {
             dialect,
             kind,
             digits,
+            fraction: stated_digits(type_name).is_some_and(|stated| stated > 0),
             stored,
             zone: match kind {
                 Kind::Zoned => zone.cloned(),
@@ -303,17 +312,7 @@ impl Order {
     /// `LC_TIME`; else the first of the system's `languages` that names a
     /// region.
     pub fn of_locale(all: Option<&str>, time: Option<&str>, languages: &[String]) -> Self {
-        let region = |tag: &str| fastframe_i18n::LanguageTag::parse(tag).and_then(|tag| tag.region);
-        let (all, time) = (
-            all.filter(|all| !all.is_empty()),
-            time.filter(|time| !time.is_empty()),
-        );
-        let found = match (all, time) {
-            (Some(all), _) => region(all),
-            (None, Some(time)) => region(time),
-            (None, None) => languages.iter().find_map(|tag| region(tag)),
-        };
-        Self::of_region(found.as_deref())
+        Self::of_region(locale_region(all, time, languages).as_deref())
     }
 
     /// The order of the region `region` (`us`, `de`, `jp`: lower case, as
@@ -331,6 +330,44 @@ impl Order {
     }
 }
 
+/// The region of the locale for times, as POSIX decides it: `LC_ALL`
+/// where it is set, whatever region it names or none (`C`); else
+/// `LC_TIME`; else the first of the system's `languages` that names a
+/// region. Lower case, as `fastframe_i18n::LanguageTag` gives it.
+pub fn locale_region(
+    all: Option<&str>,
+    time: Option<&str>,
+    languages: &[String],
+) -> Option<String> {
+    let region = |tag: &str| fastframe_i18n::LanguageTag::parse(tag).and_then(|tag| tag.region);
+    let (all, time) = (
+        all.filter(|all| !all.is_empty()),
+        time.filter(|time| !time.is_empty()),
+    );
+    match (all, time) {
+        (Some(all), _) => region(all),
+        (None, Some(time)) => region(time),
+        (None, None) => languages.iter().find_map(|tag| region(tag)),
+    }
+}
+
+/// The day the weeks of the region `region` start on (lower case, as
+/// [`locale_region`] gives it), by CLDR's `weekData`. Monday where it is
+/// not known, as the boards draw it.
+pub fn first_weekday(region: Option<&str>) -> Weekday {
+    match region.unwrap_or_default() {
+        "ag" | "as" | "bd" | "br" | "bs" | "bt" | "bw" | "bz" | "ca" | "cn" | "co" | "dm"
+        | "do" | "et" | "gt" | "gu" | "hk" | "hn" | "id" | "il" | "in" | "jm" | "jp" | "ke"
+        | "kh" | "kr" | "la" | "mh" | "mm" | "mo" | "mt" | "mx" | "mz" | "ni" | "np" | "pa"
+        | "pe" | "ph" | "pk" | "pr" | "pt" | "py" | "sa" | "sg" | "sv" | "th" | "tt" | "tw"
+        | "um" | "us" | "ve" | "vi" | "ws" | "ye" | "za" | "zw" => Weekday::Sunday,
+        "ae" | "af" | "bh" | "dj" | "dz" | "eg" | "iq" | "ir" | "jo" | "kw" | "ly" | "om"
+        | "qa" | "sd" | "sy" => Weekday::Saturday,
+        "mv" => Weekday::Friday,
+        _ => Weekday::Monday,
+    }
+}
+
 /// What a reading needs of the moment it is read at: the time it is, the
 /// zone the person's days are counted in, and how they write a day.
 #[derive(Debug, Clone)]
@@ -1636,6 +1673,146 @@ pub fn opened(value: &Value, column: &Column) -> Option<String> {
     })
 }
 
+/// The days a calendar shows for the month `month` is in: six weeks, each
+/// beginning on `first`, from the week of the month's first day. A day
+/// before or past what a date holds is `None`, in its place.
+pub fn month_days(month: Date, first: Weekday) -> Vec<Option<Date>> {
+    let start = month.first_of_month();
+    let back = i64::from(start.weekday().since(first));
+    (0..42)
+        .map(|at| start.checked_add(jiff::Span::new().days(at - back)).ok())
+        .collect()
+}
+
+/// What a text that does not read names so far: the month (its first
+/// day) and the time typed, where it has one.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct SoFar {
+    pub month: Date,
+    pub time: Option<Time>,
+}
+
+/// `2026-02` (a day may follow its `-`).
+fn iso_month(text: &str) -> Option<(i64, i64)> {
+    let (year, rest) = digits(text);
+    let (month, rest) = digits(rest.strip_prefix('-')?);
+    let formed = year.len() >= 4 && (1..=2).contains(&month.len()) && matches!(rest, "" | "-");
+    formed.then_some((year.parse().ok()?, month.parse().ok()?))
+}
+
+/// `jan`, `january 2026`: this year's where no year follows.
+fn named_month(text: &str, this_year: i64) -> Option<(i64, i64)> {
+    let (word, rest) = letters(text);
+    let number = month(word)?;
+    if rest.trim().is_empty() {
+        return Some((this_year, number));
+    }
+    let (year, rest) = year(rest)?;
+    rest.trim().is_empty().then_some((year, number))
+}
+
+/// The month a text names so far, whether it reads or not, for a column
+/// that keeps a day: `2026-02-30 09:30` is February 2026 at 09:30,
+/// `2026-02` February, `15 jan` this year's January. `None` where it
+/// names none the column's engine holds.
+pub fn so_far(typed: &str, column: &Column, clock: &Clock) -> Option<SoFar> {
+    if !column.kind.has_date() {
+        return None;
+    }
+    let text = typed.trim().to_ascii_lowercase();
+    let (text, bc) = match text.strip_suffix(" bc") {
+        Some(before) if eras(column.dialect) => (before.trim_end(), true),
+        _ => (text.as_str(), false),
+    };
+    let zone = column.zone.as_ref().unwrap_or(&clock.zone);
+    let this_year = i64::from(clock.today_in(zone).year());
+    let day = iso_day(text)
+        .or_else(|| numeric_day(text, clock.order))
+        .or_else(|| named_day(text, this_year).map(|(day, rest, _)| (day, rest)));
+    let (year, month, time) = match day {
+        Some((Day::Numbers { year, month, .. }, rest)) => {
+            let time = time_of_day(separator(rest))
+                .ok()
+                .flatten()
+                .map(|(time, _, _)| time);
+            (year, month, time)
+        }
+        Some((Day::Known(_) | Day::Now, _)) => return None,
+        None => {
+            let (year, month) = iso_month(text).or_else(|| named_month(text, this_year))?;
+            (year, month, None)
+        }
+    };
+    let month = checked_day(column, year, month, 1, bc).ok()?;
+    Some(SoFar { month, time })
+}
+
+/// The text a day picked in a calendar puts in the field, as it would be
+/// typed: the day, and for a kind with a time of day `time` (midnight
+/// where there is none), written as [`rewritten`] writes one. `offset` is
+/// the one the text had, kept where the time is one the clocks go back
+/// over and it is one of the two: the same of the two moments.
+pub fn picked(date: Date, time: Option<Time>, offset: Option<Offset>, column: &Column) -> String {
+    let at = date.to_datetime(time.unwrap_or(Time::midnight()));
+    match column.kind {
+        Kind::Date => era(written_date(date, column.dialect)),
+        Kind::Timestamp | Kind::Time { .. } => rewritten(at, None, column),
+        Kind::Zoned => rewritten(at, folded(at, offset, column), column),
+    }
+}
+
+/// `offset`, where `at` is a time the column's zone's clocks go back over
+/// and the offset is one of its two moments' (on an engine that is sent
+/// one: a MySQL value has none to choose by).
+fn folded(at: DateTime, offset: Option<Offset>, column: &Column) -> Option<Offset> {
+    let offset = offset?;
+    let two = column.zone.as_ref().and_then(|zone| fold(zone, at))?;
+    let sent = match column.dialect {
+        Dialect::Postgres | Dialect::Sqlite => true,
+        Dialect::MySql => false,
+    };
+    (sent && (two.earlier.0 == offset || two.later.0 == offset)).then_some(offset)
+}
+
+/// The text **Now** puts in the field: the time it is, to the second, in
+/// the zone the column's values are typed in, as it would be typed (today
+/// for a date, and a `timetz` with the zone's offset). `None` for a MySQL
+/// `TIMESTAMP` whose session's zone is not known: the time there is the
+/// server's to say.
+pub fn now_text(column: &Column, clock: &Clock) -> Option<String> {
+    let zone = match (&column.zone, column.kind) {
+        (Some(zone), _) => zone,
+        (None, Kind::Zoned) => return None,
+        (None, Kind::Date | Kind::Timestamp | Kind::Time { .. }) => &clock.zone,
+    };
+    let at = clock.now_in(zone);
+    let offset = zone.to_offset(clock.now);
+    Some(match column.kind {
+        Kind::Date => picked(at.date(), None, None, column),
+        Kind::Timestamp => rewritten(at, None, column),
+        // The second of a repeated hour is written with its offset.
+        Kind::Zoned => rewritten(at, folded(at, Some(offset), column), column),
+        Kind::Time { offset } => {
+            let mut text = written_time(at.time(), 0);
+            if offset {
+                text.push_str(&written_offset(zone.to_offset(clock.now)));
+            }
+            text
+        }
+    })
+}
+
+/// Whether the column takes the day `date` at `time` (midnight where
+/// there is none), as a pick would write it: a calendar offers no day it
+/// does not. A time the clocks skip is a day all the same: the line offers
+/// the time after it.
+pub fn holds(date: Date, time: Option<Time>, column: &Column, clock: &Clock) -> bool {
+    matches!(
+        read(&picked(date, time, None, column), column, clock),
+        Ok(_) | Err(Refusal::Gap { .. })
+    )
+}
+
 #[cfg(test)]
 mod tests {
     use super::*;
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --workspace --all-targets`
Expected: clean; the library's tests `test result: ok. 3463 passed; 0 failed; 17 ignored`, `dates::tests` 37 of them.

- [ ] **Step 5: Commit**

```bash
git add src/dates.rs
git commit -m "Know the days a calendar shows, and the text a pick writes"
```

---

### Task 2: The calendar under the line

**Files:**
- Create: `src/ui/calendar.rs`, `src/ui/calendar_tests.rs`
- Modify: `src/ui/date_line.rs`, `src/ui/cell_editor.rs`, `src/ui/data_view.rs`, `src/ui/row_form.rs`, `src/ui/row_panel.rs`, `src/edit.rs`, `src/app/editing.rs`, `src/app.rs`, `src/settings.rs`, `src/ui/mod.rs`, `src/ui/date_tests.rs`, `docs/_guide/editing-data.md`, `docs/_reference/settings-and-files.md`

`calendar::model` reads the editor's reading (a day and a time, a gap's, infinity, or a refusal and what `so_far` finds) into what the calendar shows; `calendar::show` draws it from the bottom of run 1's popover, which is 330 wide and taller by `calendar::height`. On macOS: the month's head (‹, ›, Today), the weekdays, a `Grid` of day buttons named "Thursday 15 January 2026" with `selected`, and "today" or "current value" as their description, disabled where `holds` says no; the time row's spin buttons, zone chip and Now; the footer. In the terminal look the month and the time row are painted, and no node is made. The month shown is kept in egui's memory by the field's id and the cell, with the text it was shown for and the frame: a new text that names a month takes it there, the arrows and Today take it elsewhere, and an editor drawn after a frame without one, or on another cell, starts again. `date_line::under` makes the model where the target has a calendar, rewrites the text with a pick (and commits NULL and DEFAULT, `Advance::Stay`), and gives the terminal's line the value before the edit. `edit_cell` keeps that value (`Editor::was`): the cell's pending text where it has one, else what its loaded value opens as (`opened_text`, run 1's code moved). The setting is a key of the file here (`data.date_picker`, on by default, reloaded live with the file); its row in Settings is Task 3's.

- [ ] **Step 1: Write the failing tests**

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -3266,6 +3266,7 @@ mod tests {
             tz: Zone::Named("America/New_York".into()),
             group_digits: Some(true),
             value_tags: false,
+            date_picker: false,
             production_read_only: false,
             confirm_close_edits: false,
             idle_disconnect: Idle::Hours4,
@@ -3324,6 +3325,7 @@ timestamps = \"full\"  # second | full
 tz = \"America/New_York\"  # system | utc | \"Area/City\"
 group_digits = true
 value_tags = false
+date_picker = false
 
 [safety]
 production_read_only = false
@@ -3443,37 +3445,38 @@ bar = \"label\"  # tint | label
                 (Key::Tz, 9),
                 (Key::GroupDigits, 10),
                 (Key::ValueTags, 11),
-                (Key::ProductionReadOnly, 14),
-                (Key::ConfirmCloseEdits, 15),
-                (Key::IdleDisconnect, 16),
-                (Key::ShowSystemSchemas, 19),
-                (Key::NewTab, 22),
-                (Key::SqlLimit, 23),
-                (Key::SqlTimeoutSecs, 24),
-                (Key::RunScope, 25),
-                (Key::ConfirmUnsafeWrites, 26),
-                (Key::EditorKeys, 29),
-                (Key::Indent, 30),
-                (Key::Autocomplete, 31),
-                (Key::ClosePairs, 32),
-                (Key::LineNumbers, 33),
-                (Key::HighlightLine, 34),
-                (Key::Wrap, 35),
-                (Key::Keywords, 38),
-                (Key::Commas, 39),
-                (Key::HistoryKeep, 42),
-                (Key::McpEnabled, 45),
-                (Key::McpLogDays, 46),
-                (Key::McpNotify, 47),
-                (Key::Theme, 50),
-                (Key::InterfaceSize, 51),
-                (Key::Font, 52),
-                (Key::FontSize, 53),
-                (Key::RowHeight, 56),
-                (Key::Zebra, 57),
-                (Key::ColumnTypes, 58),
-                (Key::TableList, 59),
-                (Key::EnvBar, 62),
+                (Key::DatePicker, 12),
+                (Key::ProductionReadOnly, 15),
+                (Key::ConfirmCloseEdits, 16),
+                (Key::IdleDisconnect, 17),
+                (Key::ShowSystemSchemas, 20),
+                (Key::NewTab, 23),
+                (Key::SqlLimit, 24),
+                (Key::SqlTimeoutSecs, 25),
+                (Key::RunScope, 26),
+                (Key::ConfirmUnsafeWrites, 27),
+                (Key::EditorKeys, 30),
+                (Key::Indent, 31),
+                (Key::Autocomplete, 32),
+                (Key::ClosePairs, 33),
+                (Key::LineNumbers, 34),
+                (Key::HighlightLine, 35),
+                (Key::Wrap, 36),
+                (Key::Keywords, 39),
+                (Key::Commas, 40),
+                (Key::HistoryKeep, 43),
+                (Key::McpEnabled, 46),
+                (Key::McpLogDays, 47),
+                (Key::McpNotify, 48),
+                (Key::Theme, 51),
+                (Key::InterfaceSize, 52),
+                (Key::Font, 53),
+                (Key::FontSize, 54),
+                (Key::RowHeight, 57),
+                (Key::Zebra, 58),
+                (Key::ColumnTypes, 59),
+                (Key::TableList, 60),
+                (Key::EnvBar, 63),
             ]
         );
     }
```

`src/ui/calendar_tests.rs`:

```diff
diff --git a/src/ui/calendar_tests.rs b/src/ui/calendar_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/calendar_tests.rs
@@ -0,0 +1,412 @@
+//! The calendar under the line of a date's editor: the month the text is
+//! in, the days a pointer picks, the time fields, Now and Today, the
+//! footer's NULL, DEFAULT and infinities, the first weekday, and the
+//! setting that shows it. On the Bookshop's `book_covers.created_at`, its
+//! type changed for each test, with the clock stopped on Saturday 24
+//! October 2026 at 10:00 in New York.
+//!
+//! What is read is the tree a screen reader is given, the field's text
+//! and what a commit stores. No colour, size or picture of the design is.
+
+use egui::accesskit::{Node, Role};
+use egui::{Key, Modifiers};
+use tabletist_db::{Driver, NewValue};
+
+use super::date_tests::{
+    MADE, covers, covers_with, editor_text, open, pending, pending_text, says, type_in,
+};
+use crate::model::Action;
+use crate::testing::{Harness, labels};
+
+/// The node labelled `label` in the role `role`, where it is drawn.
+fn node(harness: &mut Harness, label: &str, role: Role) -> Option<Node> {
+    let tree = harness.settle();
+    tree.nodes
+        .iter()
+        .find(|(_, node)| node.role() == role && node.label() == Some(label))
+        .map(|(_, node)| node.clone())
+}
+
+/// The month the calendar shows, by its grid's name.
+fn month(harness: &mut Harness) -> Option<String> {
+    let tree = harness.settle();
+    tree.nodes
+        .iter()
+        .find(|(_, node)| node.role() == Role::Grid)
+        .and_then(|(_, node)| node.label().map(str::to_owned))
+}
+
+fn day(harness: &mut Harness, name: &str) -> Node {
+    node(harness, name, Role::Button)
+        .unwrap_or_else(|| panic!("no day {name:?}: {:?}", labels(&harness.settle())))
+}
+
+/// The days of the month that are selected.
+fn selected(harness: &mut Harness) -> Vec<String> {
+    let tree = harness.settle();
+    tree.nodes
+        .iter()
+        .filter(|(_, node)| node.role() == Role::Button && node.is_selected() == Some(true))
+        .filter_map(|(_, node)| node.label().map(str::to_owned))
+        .collect()
+}
+
+/// `created_at` takes NULL.
+fn nullable(harness: &mut Harness, tab: crate::model::ConnTabId, id: crate::model::TabId) {
+    let workspace = harness.app.workspace_mut(tab).unwrap();
+    let object = workspace.object_tab_mut(id).unwrap();
+    object.structure.value.as_mut().unwrap().columns[MADE].nullable = true;
+}
+
+#[test]
+fn the_month_follows_the_text_and_marks_the_value_before_and_today() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
+    assert_eq!(selected(&mut harness), ["Thursday 15 January 2026"]);
+    // The value before the edit, once the text is another day.
+    type_in(&mut harness, tab, id, "2026-01-20 09:30");
+    assert_eq!(selected(&mut harness), ["Tuesday 20 January 2026"]);
+    let was = day(&mut harness, "Thursday 15 January 2026");
+    assert_eq!(was.description(), Some("current value"));
+    // A cell with a pending value marks that one: what it shows.
+    harness.press(Key::Enter, Modifiers::NONE);
+    type_in(&mut harness, tab, id, "2026-01-25 09:30");
+    let pending = day(&mut harness, "Tuesday 20 January 2026");
+    assert_eq!(pending.description(), Some("current value"));
+    assert_eq!(
+        day(&mut harness, "Thursday 15 January 2026").description(),
+        None
+    );
+    // Today, in the month a relative word names.
+    type_in(&mut harness, tab, id, "+6d");
+    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
+    assert_eq!(selected(&mut harness), ["Friday 30 October 2026"]);
+    let today = day(&mut harness, "Saturday 24 October 2026");
+    assert_eq!(today.description(), Some("today"));
+    // A text that does not read shows the month typed so far, with
+    // nothing selected.
+    type_in(&mut harness, tab, id, "2026-02-30 09:30");
+    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
+    assert_eq!(selected(&mut harness), Vec::<String>::new());
+    // And one that names none stays where it was.
+    type_in(&mut harness, tab, id, "soon");
+    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
+    // BC is the year counted back.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "0044-03-15 BC");
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness).as_deref(), Some("March 44 BC"));
+}
+
+#[test]
+fn a_day_picked_keeps_the_time_and_rewrites_the_text() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    open(&mut harness, tab, id);
+    harness.click("Tuesday 20 January 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-20 09:30")
+    );
+    // The line says what the pick is, and the field keeps the keyboard.
+    assert!(says(
+        &mut harness,
+        "Tue 20 Jan 2026, 09:30 EST · sent as 2026-01-20 09:30:00-05"
+    ));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-01-20 09:30:00-05")
+    );
+    // A text that does not read dims the month, which still takes a
+    // click: the time typed so far is kept.
+    type_in(&mut harness, tab, id, "2026-02-30 09:30");
+    harness.click("Sunday 15 February 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-02-15 09:30")
+    );
+    // A text with no time picks the day at midnight, in the month shown
+    // last; a date keeps none.
+    type_in(&mut harness, tab, id, "");
+    harness.click("Monday 26 January 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-26 00:00")
+    );
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    harness.click("Friday 16 January 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-16")
+    );
+    // Another month's day, from the weeks the month shares.
+    harness.click("Monday 2 February 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-02-02")
+    );
+    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
+}
+
+#[test]
+fn now_sets_the_clocks_time_and_today_only_shows_its_month() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    open(&mut harness, tab, id);
+    // The arrows and Today move the month, never the value.
+    harness.click("Next month");
+    assert_eq!(month(&mut harness).as_deref(), Some("February 2026"));
+    harness.click("Today");
+    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
+    harness.click("Previous month");
+    assert_eq!(month(&mut harness).as_deref(), Some("September 2026"));
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-15 09:30:00-05")
+    );
+    harness.click("Now");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-10-24 10:00")
+    );
+    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-10-24 10:00:00-04")
+    );
+    // With the calendar the database's now is not beside the field: it is
+    // typed, `now()`.
+    open(&mut harness, tab, id);
+    assert!(node(&mut harness, "now", Role::Button).is_none());
+}
+
+#[test]
+fn the_second_of_a_repeated_hour_stays_the_second_when_picked_or_now() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    type_in(&mut harness, tab, id, "2026-11-01 01:30");
+    harness.click("01:30 EST");
+    harness.click("Sunday 1 November 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30-05")
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30:00-05")
+    );
+    // Now, in the second 01:30.
+    harness.app.stopped_clock = Some("2026-11-01T06:30:00Z".parse().unwrap());
+    open(&mut harness, tab, id);
+    harness.click("Now");
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-11-01 01:30:00-05")
+    );
+}
+
+#[test]
+fn the_month_follows_each_new_text_and_is_each_cells_own() {
+    // Moved off with the arrows, the month comes back with a new text in
+    // the month it was in, and with Now.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    harness.click("Next month");
+    type_in(&mut harness, tab, id, "2026-01-20");
+    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-10-15 14:30:00+00");
+    open(&mut harness, tab, id);
+    harness.click("Previous month");
+    harness.click("Now");
+    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
+    // Another cell's editor, opened from this one's, starts on its own
+    // month: a NULL's, today's.
+    let days = [
+        tabletist_db::Value::Text("2026-01-15".into()),
+        tabletist_db::Value::Null,
+    ];
+    let (mut harness, tab, id) = covers_with(Driver::Postgres, "date", &days);
+    open(&mut harness, tab, id);
+    harness.click("Next month");
+    harness.app.apply(Action::EditCell {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 1, col: MADE },
+        start: crate::model::EditStart::Value,
+    });
+    assert_eq!(month(&mut harness).as_deref(), Some("October 2026"));
+}
+
+#[test]
+fn a_day_the_type_does_not_hold_cannot_be_picked() {
+    let (mut harness, tab, id) = covers(Driver::MySql, "date", "1000-01-15");
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness).as_deref(), Some("January 1000"));
+    let before = day(&mut harness, "Tuesday 31 December 999");
+    assert!(before.is_disabled());
+    harness.click("Tuesday 31 December 999");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("1000-01-15")
+    );
+    assert!(!day(&mut harness, "Wednesday 1 January 1000").is_disabled());
+    // SQLite's years run from 0000: the year before is shown, and none of
+    // its days taken.
+    let (mut harness, tab, id) = covers(Driver::Sqlite, "DATE", "0000-01-15");
+    open(&mut harness, tab, id);
+    harness.click("Previous month");
+    assert!(day(&mut harness, "Monday 6 December 2 BC").is_disabled());
+}
+
+#[test]
+fn the_time_fields_are_spin_buttons_with_their_range() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "time", "09:30:00");
+    open(&mut harness, tab, id);
+    // A time has no month.
+    assert_eq!(month(&mut harness), None);
+    let hour = node(&mut harness, "Hour", Role::SpinButton).expect("the hour");
+    assert_eq!(hour.numeric_value(), Some(9.0));
+    assert_eq!(hour.max_numeric_value(), Some(23.0));
+    let minute = node(&mut harness, "Minute", Role::SpinButton).expect("the minute");
+    assert_eq!(minute.numeric_value(), Some(30.0));
+    assert!(node(&mut harness, "Fraction of a second", Role::SpinButton).is_none());
+    // A type that keeps a fraction shows it, to its digits.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamp(6)", "2026-01-15 09:30:12.25");
+    open(&mut harness, tab, id);
+    let fraction =
+        node(&mut harness, "Fraction of a second", Role::SpinButton).expect("the fraction");
+    assert_eq!(fraction.numeric_value(), Some(250_000.0));
+    assert_eq!(fraction.max_numeric_value(), Some(999_999.0));
+    // The chip says the timestamp keeps no zone.
+    assert!(harness.has("no zone"));
+    // A `timetz`'s is its offset.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timetz", "09:30:00-05");
+    open(&mut harness, tab, id);
+    assert!(harness.has("-05"));
+    // A MySQL `TIMESTAMP` whose session's zone is not known: the server's,
+    // and no Now, whose time is the server's to say.
+    let (mut harness, tab, id) = covers(Driver::MySql, "timestamp", "2026-01-15 09:30:00");
+    open(&mut harness, tab, id);
+    assert!(harness.has("the server's zone"));
+    assert!(node(&mut harness, "Now", Role::Button).is_none());
+}
+
+#[test]
+fn null_and_default_are_offered_only_where_the_column_takes_them() {
+    // `created_at` has a default, and takes no NULL.
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    assert!(node(&mut harness, "NULL", Role::Button).is_none());
+    harness.click("DEFAULT");
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert!(matches!(
+        pending(&harness, tab, id),
+        Some(NewValue::Default { .. })
+    ));
+    nullable(&mut harness, tab, id);
+    open(&mut harness, tab, id);
+    harness.click("NULL");
+    assert_eq!(editor_text(&harness, tab, id), None);
+    assert_eq!(pending(&harness, tab, id), Some(NewValue::Null));
+    // The infinities are PostgreSQL's, and pick as a day does.
+    open(&mut harness, tab, id);
+    harness.click("Infinity");
+    assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("infinity"));
+    assert!(says(&mut harness, "No end date · stored as infinity"));
+    harness.click("Minus infinity");
+    assert_eq!(editor_text(&harness, tab, id).as_deref(), Some("-infinity"));
+    let (mut harness, tab, id) = covers(Driver::MySql, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    assert!(node(&mut harness, "Infinity", Role::Button).is_none());
+    assert!(node(&mut harness, "DEFAULT", Role::Button).is_some());
+}
+
+#[test]
+fn the_setting_shows_the_line_alone_and_reloads_live() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    // The file changes while the app runs.
+    let text = "[data]\ndate_picker = false\n".to_owned();
+    let file = crate::backend::Event::SettingsFile { text, own: false };
+    harness.app.apply(Action::Backend(file));
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness), None);
+    assert!(says(&mut harness, "Thu 15 Jan 2026"));
+    // The database's now is back beside the field.
+    assert!(node(&mut harness, "now", Role::Button).is_some());
+}
+
+#[test]
+fn the_weeks_start_on_the_locales_first_day() {
+    for (first, starts, before) in [
+        (
+            jiff::civil::Weekday::Monday,
+            "Monday 29 December 2025",
+            "Sunday 28 December 2025",
+        ),
+        (
+            jiff::civil::Weekday::Sunday,
+            "Sunday 28 December 2025",
+            "Saturday 27 December 2025",
+        ),
+    ] {
+        let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+        harness.app.week_start = first;
+        open(&mut harness, tab, id);
+        let tree = harness.settle();
+        let first_day = crate::testing::bounds(&tree, starts, Role::Button).expect(starts);
+        let thursday = crate::testing::bounds(&tree, "Thursday 1 January 2026", Role::Button)
+            .expect("the first of January");
+        // The first day is the first of its row.
+        assert!(first_day.left() < thursday.left(), "{first:?}");
+        assert!(
+            node(&mut harness, before, Role::Button).is_none(),
+            "{first:?}"
+        );
+    }
+}
+
+#[test]
+fn the_terminal_look_paints_its_month_for_the_eye_and_says_the_value_before_on_its_line() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "timestamptz", "2026-01-15 14:30:00+00");
+    harness.set_look(crate::theme::Look::omarchy());
+    type_in(&mut harness, tab, id, "2026-01-20 09:30");
+    // Its line carries everything: the calendar is no part of the tree.
+    assert!(says(
+        &mut harness,
+        "tue 20 jan 2026, 09:30 est · was 15 jan · sent as 2026-01-20 09:30:00-05"
+    ));
+    assert_eq!(month(&mut harness), None);
+    assert!(node(&mut harness, "Hour", Role::SpinButton).is_none());
+    assert!(harness.painted_color("january 2026").is_some());
+    assert!(harness.painted_color("09:30:00").is_some());
+    assert!(harness.painted_color("est · utc-05").is_some());
+}
+
+#[test]
+fn the_row_panel_has_the_calendar_under_its_field() {
+    let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
+    harness.app.workspace_mut(tab).unwrap().row_panel = true;
+    harness.settle();
+    harness.app.apply(Action::EditField {
+        tab,
+        id,
+        cell: crate::model::CellPos { row: 0, col: MADE },
+        start: crate::model::EditStart::Value,
+    });
+    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
+    harness.click("Friday 16 January 2026");
+    assert_eq!(
+        editor_text(&harness, tab, id).as_deref(),
+        Some("2026-01-16")
+    );
+    harness.press(Key::Enter, Modifiers::NONE);
+    assert_eq!(
+        pending_text(&harness, tab, id).as_deref(),
+        Some("2026-01-16")
+    );
+}
```

`src/ui/date_tests.rs`:

```diff
diff --git a/src/ui/date_tests.rs b/src/ui/date_tests.rs
--- a/src/ui/date_tests.rs
+++ b/src/ui/date_tests.rs
@@ -17,7 +17,7 @@ use crate::settings::{Settings, Zone};
 use crate::testing::{Harness, labels};
 
 /// `created_at`, by its place in the page.
-const MADE: usize = 4;
+pub(super) const MADE: usize = 4;
 
 /// Saturday 24 October 2026, 10:00 in New York.
 fn saturday() -> jiff::Timestamp {
@@ -40,7 +40,11 @@ fn in_zone(harness: &mut Harness, name: &str) {
 /// The covers' tab on `driver`, `created_at` of the type `type_name` and
 /// holding `values` (the first row's first), shown in New York, with the
 /// cursor on the first row's `created_at`.
-fn covers_with(driver: Driver, type_name: &str, values: &[Value]) -> (Harness, ConnTabId, TabId) {
+pub(super) fn covers_with(
+    driver: Driver,
+    type_name: &str,
+    values: &[Value],
+) -> (Harness, ConnTabId, TabId) {
     let mut harness = Harness::new();
     harness.app.stopped_clock = Some(saturday());
     let tab = harness.connect_fake_as(false);
@@ -95,11 +99,11 @@ fn keyed_covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnT
     (harness, tab, id)
 }
 
-fn covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
+pub(super) fn covers(driver: Driver, type_name: &str, value: &str) -> (Harness, ConnTabId, TabId) {
     covers_with(driver, type_name, &[Value::Text(value.into())])
 }
 
-fn open(harness: &mut Harness, tab: ConnTabId, id: TabId) {
+pub(super) fn open(harness: &mut Harness, tab: ConnTabId, id: TabId) {
     harness.app.apply(Action::EditCell {
         tab,
         id,
@@ -110,7 +114,7 @@ fn open(harness: &mut Harness, tab: ConnTabId, id: TabId) {
 }
 
 /// Opens the editor and types `text` in the place of its own.
-fn type_in(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
+pub(super) fn type_in(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
     open(harness, tab, id);
     editor_mut(harness, tab, id).text = text.to_owned();
     harness.app.apply(Action::EditorTyped { tab, id });
@@ -123,25 +127,25 @@ fn editor_mut(harness: &mut Harness, tab: ConnTabId, id: TabId) -> &mut crate::e
     object.edits.editor.as_mut().expect("an editor")
 }
 
-fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
+pub(super) fn editor_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
     let object = harness.app.workspace(tab)?.object_tab(id)?;
     Some(object.edits.editor.as_ref()?.text.clone())
 }
 
 /// The cell's pending value, where it has one.
-fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<NewValue> {
+pub(super) fn pending(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<NewValue> {
     let object = harness.app.workspace(tab)?.object_tab(id)?;
     Some(object.edits.cells.get(&(0, MADE))?.new.clone())
 }
 
-fn pending_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
+pub(super) fn pending_text(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<String> {
     match pending(harness, tab, id)? {
         NewValue::Text(text) => Some(text),
         other => panic!("not a text: {other:?}"),
     }
 }
 
-fn says(harness: &mut Harness, words: &str) -> bool {
+pub(super) fn says(harness: &mut Harness, words: &str) -> bool {
     harness.has(words)
 }
 
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -15,6 +15,8 @@ mod agents_tests;
 mod agent_settings_tests;
 #[cfg(test)]
 mod appearance_settings_tests;
+#[cfg(test)]
+mod calendar_tests;
 pub mod cell_editor;
 #[cfg(test)]
 mod comment_tests;
@@ -13617,6 +13619,9 @@ value_tags   = true
         use egui::accesskit::Role;
         for look in desktop_looks() {
             let (mut harness, tab, id) = stamped_in(look);
+            // The database's now, where no calendar is under the field:
+            // a calendar's Now is the clock's (`date_tests`).
+            harness.app.settings.date_picker = false;
             // In the grid, and in the row panel's field.
             for panel in [false, true] {
                 harness.app.workspace_mut(tab).unwrap().row_panel = panel;
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib`
Expected: no build, 4 errors:

```
error[E0560]: struct `settings::Settings` has no field named `date_picker`
error[E0599]: no variant, associated function, or constant named `DatePicker` found for enum `settings::Key` in the current scope
error[E0609]: no field `date_picker` on type `settings::Settings`
error[E0609]: no field `week_start` on type `app::App`
```

- [ ] **Step 3: Write the code**

`docs/_guide/editing-data.md`:

```diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -88,6 +88,32 @@ date, a day into a time, an offset into a `timestamp` (SQLite keeps one
 in its text). Words the servers read themselves, such as `epoch` or a
 zone's name after a time, are not read here: write the moment.
 
+#### The calendar
+
+Under the line of a date or a timestamp, a calendar shows the month the
+text is in: the day it reads as filled, the day the cell showed before
+the edit marked with a dot, and today outlined. The weeks start on the
+first day of the week of your locale's region. While the text reads as
+no day the month is dimmed, on the month typed so far. A click on a day,
+of the month or of the weeks it shares, writes it into the field and
+keeps the time typed (midnight where there is none); a day the column's
+type does not hold cannot be picked. ‹ and › show the month before and
+after, and **Today** today's, without changing the value.
+
+A timestamp has its hour, minute and second under the month, and a time
+has them alone, with a fraction where the type states one
+(`timestamp(3)`) or the value has one. Then the zone (`no zone` for a
+`timestamp`, the offset for a `timetz`) and **Now**, which writes the time
+it is. The foot offers **NULL** where the column takes it and **DEFAULT**
+where it has one, which apply at once, and on PostgreSQL **−∞** and **∞**
+for a date or a timestamp. A pick rewrites the field's text, as typing
+would: what the line says is still what is stored.
+
+The Omarchy look draws the month, and a timestamp's time, under its
+line, which says the day before the edit too (`was 15 jan`); they take no
+click, and their keys come later. Turn the calendar off with
+`date_picker = false` under `[data]` in `settings.toml`: the line stays.
+
 #### Zones
 
 A PostgreSQL `timestamptz` is shown, and opens to be edited, in the zone
@@ -170,9 +196,10 @@ after. Review SQL shows the keyword as it is sent.
   default, `now()` outside date and time columns.
 - A date column gets the date and a time column the time of day. On
   SQLite the time is UTC, to the second.
-- While a date or time value is being edited, the **now** button beside
-  it sets the same thing. The Omarchy look has no button: type `now()`
-  there.
+- While a date or time value is being edited with the calendar off, the
+  **now** button beside it sets the same thing. The calendar's **Now** is
+  the time it is as you click it. The Omarchy look has no button: type
+  `now()` there.
 - In a new row, a cell you leave alone is its default already, so
   `DEFAULT` there takes the value out again.
 
```

`docs/_reference/settings-and-files.md`:

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -120,6 +120,7 @@ page_size = 1000
 timestamps = "second"
 tz = "system"
 value_tags = true
+date_picker = true
 
 [safety]
 production_read_only = true
@@ -175,6 +176,7 @@ bar = "tint"
 | `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A PostgreSQL `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
+| `data.date_picker` | `true` or `false`. The calendar (the month and the time fields) under the editor of a date, a time or a timestamp; the line that says what is stored shows either way | `true` |
 | `safety.production_read_only` | `true` or `false`. Where the **Open read-only** box of a new production connection starts; the connection is saved with its box set, so a change here changes no saved connection | `true` |
 | `safety.confirm_close_edits` | `true` asks before pending changes are dropped; `false` discards them without asking. While a save runs, what would drop them does nothing until it ends, and closing the window asks either way | `true` |
 | `safety.idle_disconnect` | `"never"`, `"5m"`, `"15m"`, `"30m"`, `"1h"` or `"4h"`. A connection left unused that long has its server session closed; its tabs stay as they were, and it connects again when you next refresh, page, open a table, run or save. Never while changes are pending or something is still loading, running or saving | `"30m"` |
```

`src/app.rs`:

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -105,6 +105,9 @@ pub struct App {
     /// The order the person writes a day's numbers in (`15/01/2026`), by
     /// the region of the system's locale, as the app found it at its start.
     pub date_order: crate::dates::Order,
+    /// The day the system's locale starts its weeks on, as the app found
+    /// it at its start: the first column of the calendar.
+    pub week_start: jiff::civil::Weekday,
     /// A time the clock is stopped at, for a test: `None` reads the
     /// system's clock. What `today` and `now` typed into a date are.
     pub stopped_clock: Option<jiff::Timestamp>,
@@ -334,6 +337,7 @@ impl App {
             settings_file,
             system_zone: system_zone(),
             date_order: date_order(),
+            week_start: week_start(),
             stopped_clock: None,
             named_zone,
             zones: None,
@@ -715,6 +719,17 @@ impl App {
             .resolve(&self.system_zone, self.zones.as_deref())
     }
 
+    /// The calendar under the editor of a date, where the settings show
+    /// one: its days read at [`App::date_clock`], its weeks the locale's.
+    pub fn calendar(&self) -> Option<crate::ui::cell_editor::Calendar> {
+        self.settings
+            .date_picker
+            .then(|| crate::ui::cell_editor::Calendar {
+                clock: self.date_clock(),
+                week: self.week_start,
+            })
+    }
+
     /// What a date typed into an editor is read at: the time it is, the
     /// zone of the settings (the system's until a named one is read), and
     /// the order the person writes a day's numbers in.
@@ -5835,6 +5850,22 @@ fn date_order() -> crate::dates::Order {
     )
 }
 
+/// The day the system's locale starts its weeks on
+/// ([`crate::dates::first_weekday`]). Monday in a test, whatever the
+/// machine's.
+fn week_start() -> jiff::civil::Weekday {
+    if cfg!(test) {
+        return jiff::civil::Weekday::Monday;
+    }
+    let variable = |name: &str| std::env::var(name).ok();
+    let region = crate::dates::locale_region(
+        variable("LC_ALL").as_deref(),
+        variable("LC_TIME").as_deref(),
+        fastframe_i18n::system_languages(),
+    );
+    crate::dates::first_weekday(region.as_deref())
+}
+
 /// The computer's time zone, or UTC where it cannot be found. Tests have
 /// UTC whatever the computer they run on says, so what they show does not
 /// depend on it: a test of the System zone sets `App::system_zone`.
@@ -21495,6 +21526,7 @@ masked = ["orders.email"]
                         touched: true,
                         problem: None,
                         moment: None,
+                        was: None,
                         picker: None,
                         select: false,
                     });
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -215,6 +215,31 @@ fn typed(table: &Table<'_>, object: &ObjectTab, clock: &crate::dates::Clock) ->
     })
 }
 
+/// The text an editor opens on for the value the page's cell `cell`
+/// loaded, of the class `class`: `None` for a new row's cell, which loaded
+/// nothing. A moment with an offset as the cell shows it: in the zone, and
+/// edited there, where the engine sends one. SQLite's seconds since 1970
+/// as the day they are.
+fn opened_text(table: &Table<'_>, cell: CellPos, class: ColumnClass) -> Option<String> {
+    let loaded = table.loaded(cell)?;
+    let seconds = table
+        .dates(cell.col)
+        .and_then(|dates| crate::dates::opened(loaded, &dates));
+    if let Some(day) = seconds {
+        return Some(day);
+    }
+    let start = start_text(loaded, class);
+    let temporal = table.page.columns.get(cell.col).map(|column| column.kind)
+        == Some(tabletist_db::ValueKind::Temporal);
+    Some(
+        temporal
+            .then_some(table.zone)
+            .flatten()
+            .and_then(|zone| crate::ui::format::in_zone(&start, zone))
+            .unwrap_or(start),
+    )
+}
+
 impl App {
     /// What editing may know of a table tab: `None` while it has no page.
     pub(super) fn table<T>(
@@ -889,43 +914,22 @@ impl App {
             // holds: its check holds for it, empty or not, and Enter does
             // not close on it while its column refuses it.
             let mut kept = false;
+            // What the cell shows before this edit, as text: its pending
+            // text, or the value it loaded (none for a new row's cell).
+            let pending = object.edits.cells.get(&(cell.row, cell.col));
+            let before = match pending.map(|pending| &pending.new) {
+                Some(NewValue::Text(text)) => Some(text.clone()),
+                Some(NewValue::Null | NewValue::Default { .. } | NewValue::Now) => None,
+                None => opened_text(table, cell, class),
+            };
             let text = match start {
                 EditStart::Replace(text) | EditStart::Typed(text) => text,
-                EditStart::Value => match object.edits.cells.get(&(cell.row, cell.col)) {
-                    Some(pending) => match &pending.new {
-                        NewValue::Text(text) => {
-                            kept = true;
-                            text.clone()
-                        }
-                        // As a pending NULL: nothing of it is text
-                        // to go on from.
-                        NewValue::Null | NewValue::Default { .. } | NewValue::Now => String::new(),
-                    },
-                    // A new row's cell starts from nothing.
-                    // A moment with an offset as the cell shows it: in
-                    // the zone, and edited there, where the engine sends
-                    // one. SQLite's seconds since 1970 as the day they are.
-                    None => table
-                        .loaded(cell)
-                        .map(|loaded| {
-                            let seconds = table
-                                .dates(cell.col)
-                                .and_then(|dates| crate::dates::opened(loaded, &dates));
-                            if let Some(day) = seconds {
-                                return day;
-                            }
-                            let start = start_text(loaded, class);
-                            let temporal =
-                                table.page.columns.get(cell.col).map(|column| column.kind)
-                                    == Some(tabletist_db::ValueKind::Temporal);
-                            temporal
-                                .then_some(table.zone)
-                                .flatten()
-                                .and_then(|zone| crate::ui::format::in_zone(&start, zone))
-                                .unwrap_or(start)
-                        })
-                        .unwrap_or_default(),
-                },
+                // As a pending NULL: nothing of it is text to go on from.
+                // A new row's cell starts from nothing.
+                EditStart::Value => {
+                    kept = matches!(pending.map(|pending| &pending.new), Some(NewValue::Text(_)));
+                    before.clone().unwrap_or_default()
+                }
             };
             // A value of several lines, a long one or a document is edited
             // in the large editor, where its edit was asked for: a popover
@@ -938,6 +942,12 @@ impl App {
             // one nobody typed.
             let typed = kept || !text.is_empty();
             let moment = crate::edit::moment(table, cell.col, &text, &clock);
+            // The value the cell shows before this edit, where it reads
+            // as a moment: what the calendar marks.
+            let was = before
+                .and_then(|text| crate::edit::moment(table, cell.col, &text, &clock))
+                .and_then(Result::ok)
+                .map(|reading| reading.moment);
             let problem = match &moment {
                 // MySQL's zero date, as the cell holds it: said, and
                 // refused only once it is typed.
@@ -961,6 +971,7 @@ impl App {
                 touched: touched || kept,
                 problem,
                 moment,
+                was,
                 picker: None,
                 select: false,
             })
```

`src/edit.rs`:

```diff
diff --git a/src/edit.rs b/src/edit.rs
--- a/src/edit.rs
+++ b/src/edit.rs
@@ -1505,6 +1505,10 @@ pub struct Editor {
     /// what the line under the editor says, and, read once, what a commit
     /// stores. Kept up to date with `problem`.
     pub moment: Option<Result<crate::dates::Reading, crate::dates::Refusal>>,
+    /// The value the cell showed before this edit (its pending one, where
+    /// it has one), where its column keeps dates or times and it reads as
+    /// one: what the calendar marks.
+    pub was: Option<crate::dates::Moment>,
     /// The search of the table a foreign key cell points at, while the
     /// editor is the small field of such a cell.
     pub picker: Option<crate::key_picker::Picker>,
@@ -4348,6 +4352,7 @@ mod tests {
             touched: false,
             problem: None,
             moment: None,
+            was: None,
             picker: None,
             select: false,
         });
@@ -5740,6 +5745,7 @@ mod tests {
             touched: true,
             problem: None,
             moment: None,
+            was: None,
             picker: None,
             select: false,
         });
```

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -593,6 +593,9 @@ pub struct Settings {
     pub group_digits: Option<bool>,
     /// Enum, CHECK and boolean values drawn as coloured tags.
     pub value_tags: bool,
+    /// A calendar under the editor of a date, a time or a timestamp, with
+    /// time fields, beside the line that says what the text is stored as.
+    pub date_picker: bool,
     /// Where a new connection to production starts its "Open read-only"
     /// switch, which it is saved with. A saved connection whose switch was
     /// never set keeps the built-in rule: production is read-only.
@@ -710,6 +713,7 @@ pub enum Key {
     Tz,
     GroupDigits,
     ValueTags,
+    DatePicker,
     ProductionReadOnly,
     ConfirmCloseEdits,
     IdleDisconnect,
@@ -753,7 +757,7 @@ impl Key {
     /// In the order the file has them: a table's keys together.
     /// An old key comes before the one that took its place: the new one
     /// is read over it.
-    pub const ALL: [Key; 40] = [
+    pub const ALL: [Key; 41] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -761,6 +765,7 @@ impl Key {
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::DatePicker,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
@@ -806,6 +811,7 @@ impl Key {
             Self::Tz => ("data", "tz"),
             Self::GroupDigits => ("data", "group_digits"),
             Self::ValueTags => ("data", "value_tags"),
+            Self::DatePicker => ("data", "date_picker"),
             Self::ProductionReadOnly => ("safety", "production_read_only"),
             Self::ConfirmCloseEdits => ("safety", "confirm_close_edits"),
             Self::IdleDisconnect => ("safety", "idle_disconnect"),
@@ -2367,6 +2373,7 @@ impl Settings {
             Key::Tz => toml_string(self.tz.name()),
             Key::GroupDigits => self.group_digits?.to_string(),
             Key::ValueTags => self.value_tags.to_string(),
+            Key::DatePicker => self.date_picker.to_string(),
             Key::ProductionReadOnly => self.production_read_only.to_string(),
             Key::ConfirmCloseEdits => self.confirm_close_edits.to_string(),
             Key::IdleDisconnect => toml_string(self.idle_disconnect.name()),
@@ -2676,6 +2683,7 @@ impl Settings {
             Key::Tz => text.and_then(Zone::from_name).map(|zone| self.tz = zone),
             Key::GroupDigits => flag.map(|flag| self.group_digits = Some(flag)),
             Key::ValueTags => flag.map(|flag| self.value_tags = flag),
+            Key::DatePicker => flag.map(|flag| self.date_picker = flag),
             Key::ProductionReadOnly => flag.map(|flag| self.production_read_only = flag),
             Key::ConfirmCloseEdits => flag.map(|flag| self.confirm_close_edits = flag),
             Key::IdleDisconnect => text
@@ -2885,6 +2893,7 @@ impl Settings {
             Key::Tz => self.tz = other.tz.clone(),
             Key::GroupDigits => self.group_digits = other.group_digits,
             Key::ValueTags => self.value_tags = other.value_tags,
+            Key::DatePicker => self.date_picker = other.date_picker,
             Key::ProductionReadOnly => self.production_read_only = other.production_read_only,
             Key::ConfirmCloseEdits => self.confirm_close_edits = other.confirm_close_edits,
             Key::IdleDisconnect => self.idle_disconnect = other.idle_disconnect,
@@ -2970,6 +2979,7 @@ impl Default for Settings {
             tz: Zone::System,
             group_digits: None,
             value_tags: true,
+            date_picker: true,
             production_read_only: true,
             confirm_close_edits: true,
             idle_disconnect: Idle::default(),
```

`src/ui/calendar.rs`:

```diff
diff --git a/src/ui/calendar.rs b/src/ui/calendar.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/calendar.rs
@@ -0,0 +1,930 @@
+//! The calendar under the line of a date's editor: the month the text is
+//! in, with its time fields, its zone and Now, and on macOS a footer that
+//! offers NULL, DEFAULT and PostgreSQL's infinities. It never takes the
+//! keyboard: the field keeps it, and every pick rewrites the field's text,
+//! as typing would. In the terminal look it is painted under the float's
+//! line, which says all it shows, and no screen reader hears it.
+
+use egui::accesskit::Role;
+use egui::{CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+use jiff::civil::{Date, Time, Weekday};
+
+use crate::dates::{Column, Kind, Moment, Refusal};
+use crate::i18n::Locale;
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::cell_editor::Calendar;
+use crate::ui::date_line::{full_month_name, tr, year_words};
+use crate::ui::widgets;
+
+/// What the calendar shows of the editor's text, made each frame.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Model {
+    /// The text it is of: a new one takes the month to the one it names.
+    pub typed: String,
+    /// The month the text is in (its first day), where the column keeps a
+    /// day and the text names one. The calendar shows it unless its
+    /// arrows took it to another since the text last named it.
+    pub follows: Option<Date>,
+    /// The month shown where the text has named none yet: the value's
+    /// before the edit, else today's.
+    pub start: Date,
+    /// The day the text reads as: filled.
+    pub selected: Option<Date>,
+    /// The day the cell had before this edit: marked with a dot.
+    pub was: Option<Date>,
+    /// Today, in the zone the column's days are counted in: outlined.
+    pub today: Date,
+    /// The text reads as no day: the month is dimmed.
+    pub dim: bool,
+    /// The time a day picked keeps, and the offset the text has, which a
+    /// pick keeps where the time is a repeated hour's.
+    pub keeps: Option<Time>,
+    pub offset: Option<jiff::tz::Offset>,
+    /// The time fields, where the column keeps a time of day.
+    pub parts: Vec<Part>,
+    /// What the zone chip says, where the time row has one.
+    pub zone: Option<Zone>,
+    /// The zone's abbreviation then and its offset, as the terminal's time
+    /// row says them: `EST · UTC-05`.
+    pub abbreviation: Option<String>,
+    /// The text Now writes, where the time row has Now.
+    pub now: Option<String>,
+    /// What the footer offers: NULL where the column takes it, DEFAULT
+    /// where it has one, and PostgreSQL's infinities for a day.
+    pub null: bool,
+    pub default: bool,
+    pub infinity: bool,
+    /// The day of the week the weeks start on.
+    pub week: Weekday,
+    pub kind: Kind,
+}
+
+/// What the zone chip of the time row says.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum Zone {
+    /// Where the zone a moment is shown in is: "New York".
+    Place(String),
+    /// A timestamp with no zone, stored as typed.
+    None,
+    /// A MySQL `TIMESTAMP` whose session's zone is not known.
+    Server,
+    /// A `timetz`'s offset, as typed: `-05`.
+    Offset(String),
+}
+
+impl Zone {
+    /// The chip's words.
+    fn words(&self, locale: Locale) -> String {
+        match self {
+            Self::Place(place) | Self::Offset(place) => place.clone(),
+            Self::None => tr(locale, "no zone"),
+            Self::Server => tr(locale, "the server's zone"),
+        }
+    }
+}
+
+/// One of the time fields: what it is, what it shows, and its range.
+#[derive(Debug, Clone, PartialEq)]
+pub struct Part {
+    pub name: &'static str,
+    /// The fraction of a second, after a point: the others are after a
+    /// colon.
+    pub fraction: bool,
+    /// As the field shows it: `09`, or `--` where the text reads as no
+    /// time.
+    pub shown: String,
+    pub value: Option<f64>,
+    pub most: f64,
+}
+
+/// The time fields of `time` for `column`: hour, minute, second, and the
+/// fraction where the type states one or the time has one.
+fn parts(time: Option<Time>, column: &Column) -> Vec<Part> {
+    let part = |name, value: Option<i64>, most: f64, width: usize| Part {
+        name,
+        fraction: false,
+        shown: value.map_or_else(|| "-".repeat(width), |value| format!("{value:0width$}")),
+        value: value.map(|value| value as f64),
+        most,
+    };
+    let mut parts = vec![
+        part("Hour", time.map(|time| i64::from(time.hour())), 23.0, 2),
+        part("Minute", time.map(|time| i64::from(time.minute())), 59.0, 2),
+        part("Second", time.map(|time| i64::from(time.second())), 59.0, 2),
+    ];
+    let digits = usize::from(column.digits.min(9));
+    let fraction = time.is_some_and(|time| time.subsec_nanosecond() != 0);
+    if digits > 0 && (column.fraction || fraction) {
+        let value = time.map(|time| {
+            let nanos = i64::from(time.subsec_nanosecond());
+            nanos / 10_i64.pow(9 - digits as u32)
+        });
+        let most = 10_f64.powi(digits as i32) - 1.0;
+        parts.push(Part {
+            fraction: true,
+            ..part("Fraction of a second", value, most, digits)
+        });
+    }
+    parts
+}
+
+/// The calendar of the editor whose text is `text`, which reads as
+/// `moment`, in the column `column`, whose cell had `was` before the
+/// edit. `offers` is what the footer may offer: NULL, then DEFAULT.
+pub fn model(
+    (text, moment, was): (
+        &str,
+        &Result<crate::dates::Reading, Refusal>,
+        Option<&Moment>,
+    ),
+    column: &Column,
+    calendar: &Calendar,
+    offers: (bool, bool),
+) -> Model {
+    let clock = &calendar.clock;
+    let today = clock.today_in(column.zone.as_ref().unwrap_or(&clock.zone));
+    let was = match was {
+        Some(Moment::At { date, .. }) => *date,
+        _ => None,
+    };
+    let (mut selected, mut keeps, mut dim, mut follows) = (None, None, true, None);
+    let (mut offset, mut abbreviation) = (None, None);
+    match moment {
+        Ok(reading) => match &reading.moment {
+            Moment::At {
+                date,
+                time,
+                offset: typed,
+            } => {
+                (selected, keeps, dim) = (*date, *time, false);
+                follows = *date;
+                offset = *typed;
+                abbreviation = reading
+                    .zone_name
+                    .as_ref()
+                    .zip(*typed)
+                    .map(|(name, offset)| {
+                        format!("{name} · UTC{}", crate::dates::written_offset(offset))
+                    });
+            }
+            // Dimmed, on today's month.
+            Moment::Infinity | Moment::MinusInfinity => follows = Some(today),
+            Moment::EndOfDay | Moment::Span(_) => dim = false,
+        },
+        // A time the clocks skip is a day all the same: the line offers
+        // the time after it.
+        Err(Refusal::Gap { at, .. }) => {
+            (selected, keeps, dim) = (Some(at.date()), Some(at.time()), false);
+            follows = selected;
+        }
+        Err(_) => {
+            if let Some(sofar) = crate::dates::so_far(text, column, clock) {
+                (follows, keeps) = (Some(sofar.month), sofar.time);
+            }
+        }
+    }
+    let zone = match column.kind {
+        Kind::Zoned => Some(match &column.zone {
+            Some(zone) => Zone::Place(crate::dates::place(zone)),
+            None => Zone::Server,
+        }),
+        Kind::Timestamp => Some(Zone::None),
+        Kind::Time { offset: true } => {
+            offset.map(|offset| Zone::Offset(crate::dates::written_offset(offset)))
+        }
+        Kind::Time { offset: false } | Kind::Date => None,
+    };
+    let postgres = match column.dialect {
+        tabletist_db::Dialect::Postgres => true,
+        tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => false,
+    };
+    Model {
+        typed: text.to_owned(),
+        offset,
+        follows: follows.map(Date::first_of_month),
+        start: was.unwrap_or(today).first_of_month(),
+        selected,
+        was,
+        today,
+        dim,
+        keeps,
+        parts: if column.kind.has_time() {
+            parts(keeps, column)
+        } else {
+            Vec::new()
+        },
+        zone,
+        abbreviation,
+        now: column
+            .kind
+            .has_time()
+            .then(|| crate::dates::now_text(column, clock))
+            .flatten(),
+        null: offers.0,
+        default: offers.1,
+        infinity: postgres && column.kind.has_date(),
+        week: calendar.week,
+        kind: column.kind,
+    }
+}
+
+/// What a frame of the calendar asks for.
+#[derive(Debug, Default, PartialEq)]
+pub struct Asked {
+    /// The text a pick rewrites the field to.
+    pub text: Option<String>,
+    /// The pick ends the edit: NULL and DEFAULT, which are no day.
+    pub commit: bool,
+    /// A press on one of its controls.
+    pub pressed: bool,
+}
+
+/// The heights of the macOS calendar's parts.
+const HEAD: f32 = 32.0;
+const WEEKDAYS: f32 = 22.0;
+const DAY: f32 = 30.0;
+const DAY_GAP: f32 = 2.0;
+const TIME_ROW: f32 = 42.0;
+const FOOTER: f32 = 38.0;
+/// The terminal's rows, and its days' columns.
+const TERMINAL_ROW: f32 = 22.0;
+const TERMINAL_DAY: f32 = 32.0;
+/// The room at the sides.
+const PAD: f32 = 10.0;
+
+/// How tall the calendar of `model` is, under the line.
+pub fn height(model: &Model, look: &Look) -> f32 {
+    let month = model.kind.has_date();
+    let time = model.kind.has_time();
+    if look.terminal {
+        // The terminal's time row is a timestamp's: a time's line says
+        // it, as its board has it.
+        let month = if month {
+            2.0 * TERMINAL_ROW + 6.0 * TERMINAL_ROW + 8.0
+        } else {
+            0.0
+        };
+        let time = if time && model.kind.has_date() {
+            TERMINAL_ROW + 4.0
+        } else {
+            0.0
+        };
+        return month + time;
+    }
+    let month = if month {
+        HEAD + WEEKDAYS + 6.0 * DAY + 5.0 * DAY_GAP + 8.0
+    } else {
+        0.0
+    };
+    let time = if time { TIME_ROW } else { 0.0 };
+    month + time + FOOTER
+}
+
+/// Where the calendar of the editor `memory` names (its field and its
+/// cell) keeps which month it shows: the text it last showed it for, the
+/// month, and the frame it was drawn in.
+fn shown_id(memory: Id) -> Id {
+    memory.with("calendar-month")
+}
+
+/// The month to show: the one the text names, unless the arrows moved off
+/// it since the text last changed. A text that names none leaves it where
+/// it is. An editor drawn after a frame without one, or on another cell,
+/// starts again.
+fn month_shown(ui: &Ui, memory: Id, model: &Model) -> Date {
+    let frame = ui.ctx().cumulative_frame_nr();
+    let kept: Option<(String, Date, u64)> = ui.ctx().data(|data| data.get_temp(shown_id(memory)));
+    let shown = match kept {
+        Some((typed, shown, at)) if frame.saturating_sub(at) <= 1 => {
+            match model.follows.filter(|_| typed != model.typed) {
+                Some(follows) => follows,
+                None => shown,
+            }
+        }
+        _ => model.follows.unwrap_or(model.start),
+    };
+    remember(ui, memory, model, shown);
+    shown
+}
+
+fn remember(ui: &Ui, memory: Id, model: &Model, shown: Date) {
+    let frame = ui.ctx().cumulative_frame_nr();
+    let kept = (model.typed.clone(), shown, frame);
+    ui.ctx()
+        .data_mut(|data| data.insert_temp(shown_id(memory), kept));
+}
+
+/// The first day of the month `by` months from `month`, where a date
+/// holds it.
+fn moved(month: Date, by: i64) -> Option<Date> {
+    month.checked_add(jiff::Span::new().months(by)).ok()
+}
+
+const INITIALS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
+const WEEKDAY_NAMES: [&str; 7] = [
+    "Monday",
+    "Tuesday",
+    "Wednesday",
+    "Thursday",
+    "Friday",
+    "Saturday",
+    "Sunday",
+];
+
+/// The weekdays in the order the weeks run, from `first`.
+fn weekdays(first: Weekday) -> [Weekday; 7] {
+    std::array::from_fn(|at| first.wrapping_add(at as i64))
+}
+
+fn initial(day: Weekday, locale: Locale) -> String {
+    let at = usize::try_from(day.to_monday_zero_offset()).unwrap_or_default();
+    tr(locale, INITIALS[at % 7])
+}
+
+/// "January 2026", "March 44 BC".
+pub fn month_words(month: Date, locale: Locale) -> String {
+    format!(
+        "{} {}",
+        full_month_name(month.month(), locale),
+        year_words(month.year(), locale)
+    )
+}
+
+/// "Thursday 15 January 2026": a day as a screen reader says it.
+pub fn day_name(date: Date, locale: Locale) -> String {
+    let at = usize::try_from(date.weekday().to_monday_zero_offset()).unwrap_or_default();
+    format!(
+        "{} {} {} {}",
+        tr(locale, WEEKDAY_NAMES[at % 7]),
+        date.day(),
+        full_month_name(date.month(), locale),
+        year_words(date.year(), locale)
+    )
+}
+
+/// Draws the calendar of `model` from `top`, `width` wide, for the editor
+/// `memory` names (its field and its cell) in `column`: on macOS with the
+/// controls that pick, in the terminal look painted. Says what was picked.
+pub fn show(
+    ui: &mut Ui,
+    (top, width): (egui::Pos2, f32),
+    memory: Id,
+    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Asked {
+    // Its controls' ids are the editor's: one cell's are no other's.
+    let salted = egui::UiBuilder::new().id_salt(memory);
+    ui.scope_builder(salted, |ui| {
+        drawn(
+            ui,
+            (top, width),
+            memory,
+            (model, column, clock),
+            (look, palette, locale),
+        )
+    })
+    .inner
+}
+
+/// [`show`], in the editor's own scope.
+fn drawn(
+    ui: &mut Ui,
+    (top, width): (egui::Pos2, f32),
+    memory: Id,
+    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> Asked {
+    let mut asked = Asked::default();
+    let skin = (look, palette, locale);
+    let mut y = top.y;
+    let left = top.x;
+    if model.kind.has_date() {
+        let shown = month_shown(ui, memory, model);
+        if look.terminal {
+            y = terminal_month(ui, (left, y, width), shown, model, skin);
+        } else {
+            let at = (left, y, width);
+            y = month(
+                ui,
+                (at, memory),
+                shown,
+                (model, column, clock),
+                skin,
+                &mut asked,
+            );
+        }
+    }
+    if look.terminal {
+        if model.kind.has_time() && model.kind.has_date() {
+            terminal_time(ui, (left, y, width), model, skin);
+        }
+        return asked;
+    }
+    if model.kind.has_time() {
+        y = time_row(ui, (left, y, width), model, skin, &mut asked);
+    }
+    footer(ui, (left, y, width), model, skin, &mut asked);
+    asked
+}
+
+/// The macOS month: its head, its weekdays and its six weeks. Returns
+/// where it ends.
+fn month(
+    ui: &mut Ui,
+    ((left, top, width), memory): ((f32, f32, f32), Id),
+    shown: Date,
+    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
+    (look, palette, locale): (&Look, &Palette, Locale),
+    asked: &mut Asked,
+) -> f32 {
+    let skin = (look, palette, locale);
+    let right = left + width;
+    widgets::hline(ui, egui::Rangef::new(left, right), top, palette.border);
+    let middle = top + 4.0 + 24.0 / 2.0 + 4.0;
+    let title = month_words(shown, locale);
+    let laid = Text::one(look, TextRole::UiBodySemibold, &title, palette.text).layout(ui.ctx());
+    laid.paint_left(ui.painter(), left + PAD, middle);
+    // ‹ › and Today, at the right.
+    let today_label = tr(locale, "Today");
+    let today = widgets::ButtonSpec::new(&today_label)
+        .link()
+        .padding(6.0)
+        .role(widgets::secondary(look));
+    let today_width = today.width(ui, look);
+    let today_rect = Rect::from_min_size(
+        pos2(right - PAD - today_width, middle - 12.0),
+        vec2(today_width, 24.0),
+    );
+    let mut go = None;
+    let response = today.show_at(ui, today_rect, look, palette);
+    asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
+    if response.clicked() {
+        go = Some(model.today.first_of_month());
+    }
+    let mut x = today_rect.left() - 6.0;
+    for (by, glyph, name, last) in [
+        (
+            1,
+            "›",
+            tr(locale, "Next month"),
+            tr(locale, "No date comes later"),
+        ),
+        (
+            -1,
+            "‹",
+            tr(locale, "Previous month"),
+            tr(locale, "No date comes earlier"),
+        ),
+    ] {
+        let place = Rect::from_min_size(pos2(x - 24.0, middle - 12.0), vec2(24.0, 24.0));
+        x -= 28.0;
+        let target = moved(shown, by);
+        let spec = widgets::ButtonSpec::new(glyph)
+            .quiet()
+            .label(&name)
+            .padding(0.0);
+        let spec = match target {
+            Some(_) => spec,
+            None => spec.disabled(&last),
+        };
+        let response = spec.show_at(ui, place, look, palette);
+        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
+        if response.clicked() {
+            go = target.or(go);
+        }
+    }
+    if let Some(month) = go {
+        remember(ui, memory, model, month);
+    }
+    let shown = go.unwrap_or(shown);
+    let mut y = top + HEAD;
+    let column_width = (width - 16.0) / 7.0;
+    let first = left + 8.0;
+    for (at, day) in weekdays(model.week).into_iter().enumerate() {
+        let center = pos2(first + column_width * (at as f32 + 0.5), y + WEEKDAYS / 2.0);
+        Text::one(
+            look,
+            TextRole::FieldLabel,
+            &initial(day, locale),
+            palette.faint,
+        )
+        .layout(ui.ctx())
+        .paint_center(ui.painter(), center);
+    }
+    y += WEEKDAYS;
+    // The month is a grid of its days, each a button.
+    let grid_id = Id::new("calendar-grid");
+    ui.scope_builder(egui::UiBuilder::new().id_salt(grid_id), |ui| {
+        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
+            node.set_role(Role::Grid);
+            node.set_label(month_words(shown, locale));
+        });
+        for (at, day) in crate::dates::month_days(shown, model.week)
+            .into_iter()
+            .enumerate()
+        {
+            let (row, col) = (at / 7, at % 7);
+            let cell = Rect::from_min_size(
+                pos2(
+                    first + column_width * col as f32,
+                    y + (DAY + DAY_GAP) * row as f32,
+                ),
+                vec2(column_width, DAY),
+            );
+            let Some(day) = day else {
+                continue;
+            };
+            if let Some(text) = day_cell(ui, cell, day, shown, (model, column, clock), skin, asked)
+            {
+                asked.text = Some(text);
+            }
+        }
+    });
+    y + 6.0 * DAY + 5.0 * DAY_GAP + 8.0
+}
+
+/// One day of the macOS month in `cell`: its number, its marks, and a
+/// click that picks it. Returns the text a click rewrites the field to.
+fn day_cell(
+    ui: &mut Ui,
+    cell: Rect,
+    day: Date,
+    shown: Date,
+    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
+    (look, palette, locale): (&Look, &Palette, Locale),
+    asked: &mut Asked,
+) -> Option<String> {
+    let holds = crate::dates::holds(day, model.keeps, column, clock);
+    let selected = model.selected == Some(day);
+    let today = model.today == day;
+    let was = model.was == Some(day) && !selected;
+    let id = ui
+        .id()
+        .with(("calendar-day", day.year(), day.month(), day.day()));
+    let sense = if holds {
+        Sense::click()
+    } else {
+        Sense::hover()
+    };
+    let response = ui.interact(cell, id, sense);
+    let name = day_name(day, locale);
+    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, holds, &name));
+    let said: Vec<String> = [(today, "today"), (was, "current value")]
+        .into_iter()
+        .filter(|(on, _)| *on)
+        .map(|(_, words)| tr(locale, words))
+        .collect();
+    ui.ctx().accesskit_node_builder(response.id, |node| {
+        node.set_selected(selected);
+        if !said.is_empty() {
+            node.set_description(said.join(", "));
+        }
+    });
+    let inside = day.year() == shown.year() && day.month() == shown.month();
+    // The month dims while the text reads as no day; a day the type does
+    // not hold, and another month's, are drawn as disabled.
+    let fade = |color: egui::Color32| {
+        if model.dim {
+            color.gamma_multiply(0.45)
+        } else {
+            color
+        }
+    };
+    let painter = ui.painter();
+    let radius = CornerRadius::same(7);
+    if selected && !model.dim {
+        painter.rect_filled(cell.shrink2(vec2(1.0, 0.0)), radius, palette.accent);
+    } else if response.hovered() && holds {
+        painter.rect_filled(cell.shrink2(vec2(1.0, 0.0)), radius, palette.hover_fill());
+    }
+    if today && !(selected && !model.dim) {
+        let outline = palette.window.lerp_to_gamma(palette.accent, 0.45);
+        painter.rect_stroke(
+            cell.shrink(3.0),
+            CornerRadius::same(6),
+            Stroke::new(1.0, fade(outline)),
+            StrokeKind::Inside,
+        );
+    }
+    let (role, color) = if selected && !model.dim {
+        (TextRole::UiBodySemibold, palette.on_accent)
+    } else if !holds || !inside {
+        (TextRole::UiBody, palette.faint)
+    } else if today {
+        (TextRole::UiBodySemibold, palette.accent_hover)
+    } else {
+        (TextRole::UiBody, palette.text)
+    };
+    Text::one(look, role, &day.day().to_string(), fade(color))
+        .layout(ui.ctx())
+        .paint_center(ui.painter(), cell.center());
+    if was {
+        ui.painter().circle_filled(
+            pos2(cell.center().x, cell.bottom() - 6.0),
+            2.0,
+            fade(palette.warning),
+        );
+    }
+    asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
+    (response.clicked() && holds)
+        .then(|| crate::dates::picked(day, model.keeps, model.offset, column))
+}
+
+/// The macOS time row: the fields, then the zone chip and Now. Returns
+/// where it ends.
+fn time_row(
+    ui: &mut Ui,
+    (left, top, width): (f32, f32, f32),
+    model: &Model,
+    (look, palette, locale): (&Look, &Palette, Locale),
+    asked: &mut Asked,
+) -> f32 {
+    widgets::hline(
+        ui,
+        egui::Rangef::new(left, left + width),
+        top,
+        palette.border,
+    );
+    let middle = top + TIME_ROW / 2.0;
+    let mono = TextRole::GridCell;
+    let mut x = left + PAD;
+    for (at, part) in model.parts.iter().enumerate() {
+        if at > 0 {
+            let between = if part.fraction { "." } else { ":" };
+            let laid = Text::one(look, mono, between, palette.faint).layout(ui.ctx());
+            x += 3.0;
+            x += laid.paint_left(ui.painter(), x, middle) + 3.0;
+        }
+        let text_width = mono.width(ui.ctx(), look.faces, &part.shown);
+        let field = Rect::from_min_size(
+            pos2(x, middle - 13.0),
+            vec2((text_width + 8.0).max(30.0), 26.0),
+        );
+        x = field.right();
+        // A spin button with its range: the keys that step it come with
+        // the picker's keys.
+        let id = ui.id().with(("calendar-time", at));
+        let response = ui.interact(field, id, Sense::hover());
+        let name = tr(locale, part.name);
+        response
+            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &name));
+        ui.ctx().accesskit_node_builder(response.id, |node| {
+            node.set_min_numeric_value(0.0);
+            node.set_max_numeric_value(part.most);
+            if let Some(value) = part.value {
+                node.set_numeric_value(value);
+            }
+            node.set_value(part.shown.clone());
+        });
+        ui.painter().rect_stroke(
+            field,
+            CornerRadius::same(6),
+            Stroke::new(1.0, palette.border),
+            StrokeKind::Inside,
+        );
+        Text::one(look, mono, &part.shown, palette.text)
+            .layout(ui.ctx())
+            .paint_center(ui.painter(), field.center());
+    }
+    let mut right = left + width - PAD;
+    if let Some(now) = &model.now {
+        let label = tr(locale, "Now");
+        let spec = widgets::ButtonSpec::new(&label)
+            .padding(8.0)
+            .role(widgets::secondary(look));
+        let place = Rect::from_min_size(
+            pos2(right - spec.width(ui, look), middle - 12.0),
+            vec2(spec.width(ui, look), 24.0),
+        );
+        let response = spec.show_at(ui, place, look, palette);
+        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
+        if response.clicked() {
+            asked.text = Some(now.clone());
+        }
+        right = place.left() - 6.0;
+    }
+    if let Some(zone) = &model.zone {
+        let words = zone.words(locale);
+        let laid = Text::one(look, TextRole::Shortcut, &words, palette.secondary).layout(ui.ctx());
+        let chip = Rect::from_min_size(
+            pos2(right - laid.width() - 14.0, middle - 11.0),
+            vec2(laid.width() + 14.0, 22.0),
+        );
+        if chip.left() > x + 6.0 {
+            ui.painter()
+                .rect_filled(chip, CornerRadius::same(11), palette.surface);
+            laid.paint_center(ui.painter(), chip.center());
+            widgets::announce(ui, chip, &words);
+        }
+    }
+    top + TIME_ROW
+}
+
+/// The macOS footer: the keys that apply and cancel, or PostgreSQL's
+/// infinities, and NULL and DEFAULT where the column takes them.
+fn footer(
+    ui: &mut Ui,
+    (left, top, width): (f32, f32, f32),
+    model: &Model,
+    (look, palette, locale): (&Look, &Palette, Locale),
+    asked: &mut Asked,
+) {
+    let rect = Rect::from_min_size(pos2(left, top), vec2(width, FOOTER));
+    ui.painter().rect_filled(
+        rect.shrink(1.0),
+        CornerRadius {
+            nw: 0,
+            ne: 0,
+            sw: look.dialog_radius,
+            se: look.dialog_radius,
+        },
+        palette.panel,
+    );
+    widgets::hline(ui, rect.x_range(), top, palette.border);
+    let middle = rect.center().y;
+    let mut button = |ui: &mut Ui, x: f32, text: &str, name: &str, right: bool| {
+        let spec = widgets::ButtonSpec::new(text)
+            .label(name)
+            .padding(8.0)
+            .role(widgets::secondary(look));
+        let width = spec.width(ui, look);
+        let left = if right { x - width } else { x };
+        let place = Rect::from_min_size(pos2(left, middle - 12.0), vec2(width, 24.0));
+        let response = spec.show_at(ui, place, look, palette);
+        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
+        (response.clicked(), place)
+    };
+    if model.infinity {
+        // The keys' hints give way to the infinities, as the board has it.
+        let mut x = left + PAD;
+        for (text, written, name) in [
+            ("−∞", "-infinity", tr(locale, "Minus infinity")),
+            ("∞", "infinity", tr(locale, "Infinity")),
+        ] {
+            let (clicked, place) = button(ui, x, text, &name, false);
+            if clicked {
+                asked.text = Some(written.to_owned());
+            }
+            x = place.right() + 6.0;
+        }
+    } else {
+        let apply = crate::ui::keys::written_in(
+            ui.ctx(),
+            look,
+            crate::keymap::Command::CommitDown,
+            crate::keymap::Scope::CellEditor,
+        );
+        let cancel = crate::ui::keys::written_in(
+            ui.ctx(),
+            look,
+            crate::keymap::Command::CancelEdit,
+            crate::keymap::Scope::CellEditor,
+        );
+        let (apply_words, cancel_words) = (tr(locale, "Apply"), tr(locale, "Cancel"));
+        let hints = [
+            (&apply, apply_words.as_str(), true),
+            (&cancel, cancel_words.as_str(), true),
+        ];
+        widgets::key_hints(ui, (left + PAD, middle), &hints, 10.0, look, palette);
+    }
+    let mut x = left + width - PAD;
+    for (offered, text) in [(model.default, "DEFAULT"), (model.null, "NULL")] {
+        if !offered {
+            continue;
+        }
+        let (clicked, place) = button(ui, x, text, text, true);
+        if clicked {
+            asked.text = Some(text.to_owned());
+            asked.commit = true;
+        }
+        x = place.left() - 6.0;
+    }
+}
+
+/// The terminal's month: its name and its six weeks, painted. Returns
+/// where it ends.
+fn terminal_month(
+    ui: &Ui,
+    (left, top, width): (f32, f32, f32),
+    shown: Date,
+    model: &Model,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) -> f32 {
+    widgets::hline(
+        ui,
+        egui::Rangef::new(left, left + width),
+        top,
+        palette.border,
+    );
+    let role = TextRole::OBody;
+    let mut y = top + TERMINAL_ROW / 2.0;
+    Text::one(
+        look,
+        TextRole::OGroup,
+        &look.label(&month_words(shown, locale)),
+        palette.text,
+    )
+    .layout(ui.ctx())
+    .paint_left(ui.painter(), left + PAD, y);
+    y += TERMINAL_ROW;
+    let fade = |color: egui::Color32| {
+        if model.dim {
+            color.gamma_multiply(0.45)
+        } else {
+            color
+        }
+    };
+    for (at, day) in weekdays(model.week).into_iter().enumerate() {
+        let center = pos2(left + PAD + TERMINAL_DAY * (at as f32 + 0.5), y);
+        Text::one(
+            look,
+            role,
+            &look.label(&initial(day, locale)),
+            fade(palette.dim),
+        )
+        .layout(ui.ctx())
+        .paint_center(ui.painter(), center);
+    }
+    y += TERMINAL_ROW / 2.0 + 2.0;
+    for (at, day) in crate::dates::month_days(shown, model.week)
+        .into_iter()
+        .enumerate()
+    {
+        let Some(day) = day else {
+            continue;
+        };
+        let (row, col) = (at / 7, at % 7);
+        let cell = Rect::from_min_size(
+            pos2(
+                left + PAD + TERMINAL_DAY * col as f32,
+                y + TERMINAL_ROW * row as f32,
+            ),
+            vec2(TERMINAL_DAY, TERMINAL_ROW),
+        );
+        let inside = day.year() == shown.year() && day.month() == shown.month();
+        let selected = model.selected == Some(day) && !model.dim;
+        let number = format!("{:>2}", day.day());
+        let (role, color) = if selected {
+            ui.painter()
+                .rect_filled(cell, CornerRadius::ZERO, palette.accent);
+            (TextRole::OGroup, palette.on_accent)
+        } else if !inside {
+            (role, palette.faint)
+        } else if model.today == day {
+            (role, palette.accent)
+        } else {
+            (role, palette.text)
+        };
+        let laid = Text::one(look, role, &number, fade(color)).layout(ui.ctx());
+        laid.paint_center(ui.painter(), cell.center());
+        // Today is underlined, as the terminal marks a link.
+        if model.today == day && !selected {
+            let under = cell.center().y + 8.0;
+            let half = laid.width() / 2.0;
+            ui.painter().hline(
+                egui::Rangef::new(cell.center().x - half, cell.center().x + half),
+                under,
+                Stroke::new(1.0, fade(palette.accent)),
+            );
+        }
+    }
+    y + 6.0 * TERMINAL_ROW + 6.0
+}
+
+/// The terminal's time row of a timestamp: the time, and its zone.
+fn terminal_time(
+    ui: &Ui,
+    (left, top, width): (f32, f32, f32),
+    model: &Model,
+    (look, palette, locale): (&Look, &Palette, Locale),
+) {
+    widgets::hline(
+        ui,
+        egui::Rangef::new(left, left + width),
+        top,
+        palette.border,
+    );
+    let role = TextRole::OBody;
+    let middle = top + (TERMINAL_ROW + 4.0) / 2.0;
+    let time: Vec<&str> = model.parts.iter().map(|part| part.shown.as_str()).collect();
+    let mut time = time[..time.len().min(3)].join(":");
+    if let Some(fraction) = model.parts.get(3) {
+        time.push('.');
+        time.push_str(&fraction.shown);
+    }
+    let zone = match (&model.abbreviation, &model.zone) {
+        (Some(abbreviation), _) => abbreviation.clone(),
+        (None, Some(zone)) => zone.words(locale),
+        (None, None) => String::new(),
+    };
+    let mut x = left + PAD;
+    for (text, color) in [
+        (look.label(&tr(locale, "time")), palette.dim),
+        (time, palette.text),
+        (look.label(&zone), palette.dim),
+    ] {
+        let laid = Text::one(look, role, &text, color).layout(ui.ctx());
+        x += laid.paint_left(ui.painter(), x, middle) + 10.0;
+    }
+}
```

`src/ui/cell_editor.rs`:

```diff
diff --git a/src/ui/cell_editor.rs b/src/ui/cell_editor.rs
--- a/src/ui/cell_editor.rs
+++ b/src/ui/cell_editor.rs
@@ -67,12 +67,20 @@ pub struct Target {
     pub json: bool,
     /// No dialog is up: an editor that is open has the keyboard.
     pub hold: bool,
-    /// The column keeps a date or a time: the editor offers now.
+    /// The column keeps a date or a time and the calendar is not shown:
+    /// the editor offers the database's now.
     pub now: bool,
     /// The column as its dates and times are read, where it keeps them:
     /// the line under the editor says what the text is stored as, and
     /// says what it fails in the place of the note under the field.
     pub dates: Option<crate::dates::Column>,
+    /// The column takes NULL, and it has a default: what the calendar's
+    /// footer offers.
+    pub nullable: bool,
+    pub default: bool,
+    /// The calendar under the line of a date's editor, where the settings
+    /// show one.
+    pub calendar: Option<Calendar>,
     /// The keys that end the edit, as the keymap has them.
     pub keys: Keys,
     /// The cell is a foreign key's, and its editor searches the table the
@@ -95,6 +103,17 @@ impl Target {
     }
 }
 
+/// What the calendar under a date's editor reads its days at, where the
+/// settings show one.
+#[derive(Clone, Debug)]
+pub struct Calendar {
+    /// The time it is, the zone a day is counted in, and how a day's
+    /// numbers are written.
+    pub clock: crate::dates::Clock,
+    /// The day the locale's weeks start on.
+    pub week: jiff::civil::Weekday,
+}
+
 /// What a foreign key cell's field says of its search.
 pub struct Search {
     /// The table that is searched.
@@ -1818,6 +1837,7 @@ mod tests {
             touched: true,
             problem: None,
             moment: None,
+            was: None,
             picker: None,
             select: false,
         };
@@ -1830,6 +1850,9 @@ mod tests {
             hold: true,
             now: false,
             dates: None,
+            nullable: false,
+            default: false,
+            calendar: None,
             keys: Keys::of(&Keymap::default(), Layout::of(look)),
             search: search.map(|(count, refused)| Search {
                 table: "publishers".into(),
```

`src/ui/data_view.rs`:

```diff
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -1519,6 +1519,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
     // An editor that is open has the keyboard, unless a dialog has it.
     let hold = app.dialog.is_none();
     let editor_keys = cell_editor::Keys::of(&app.keymap, app.layout());
+    let calendar = app.calendar();
     if look.terminal {
         error_line(app, ui, tab, object_tab);
     }
@@ -1547,7 +1548,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId)
         .and_then(|key| key.into_iter().min());
     let (unset, lacking) = new_row_columns(workspace, object);
     let target = object.edits.editor.as_ref().and_then(|editor| {
-        let how = (hold, &editor_keys);
+        let how = (hold, &editor_keys, calendar.as_ref());
         editor_target(workspace, object, tab, editor.cell, how)
     });
     // The cells whose foreign key names no row, with what each says.
@@ -2115,7 +2116,7 @@ pub(super) fn editor_target(
     object: &ObjectTab,
     tab: ConnTabId,
     cell: CellPos,
-    (hold, keys): (bool, &cell_editor::Keys),
+    (hold, keys, calendar): (bool, &cell_editor::Keys, Option<&cell_editor::Calendar>),
 ) -> Option<cell_editor::Target> {
     let table = Table::of(workspace, object)?;
     let column = table.page.columns.get(cell.col)?;
@@ -2126,6 +2127,12 @@ pub(super) fn editor_target(
     };
     // The type as the header names it, not as the structure does.
     let type_name = format::type_label(&column.type_name, column.kind);
+    let dates = table.dates(cell.col);
+    let info = table.column(cell.col);
+    // The calendar under a date's line, where the settings show one: its
+    // Now is the clock's, and takes the place of the database's now
+    // beside the field.
+    let calendar = calendar.filter(|_| dates.is_some()).cloned();
     Some(cell_editor::Target {
         id: cell_editor::field_id(tab, object.id),
         name: column.name.clone(),
@@ -2133,10 +2140,12 @@ pub(super) fn editor_target(
         max_chars,
         json: class == Some(tabletist_db::ColumnClass::Json),
         hold,
-        now: table
-            .column(cell.col)
-            .is_some_and(|column| tabletist_db::temporal(&column.type_name).is_some()),
-        dates: table.dates(cell.col),
+        now: calendar.is_none()
+            && info.is_some_and(|column| tabletist_db::temporal(&column.type_name).is_some()),
+        dates,
+        nullable: info.is_some_and(|column| column.nullable),
+        default: info.is_some_and(|column| column.default.is_some()),
+        calendar,
         keys: keys.clone(),
         search: object
             .edits
```

`src/ui/date_line.rs`:

```diff
diff --git a/src/ui/date_line.rs b/src/ui/date_line.rs
--- a/src/ui/date_line.rs
+++ b/src/ui/date_line.rs
@@ -15,7 +15,7 @@ use crate::typography::{Text, TextRole};
 use crate::ui::{sql_complete, widgets};
 
 /// `text` in the person's language.
-fn tr(locale: Locale, text: &'static str) -> String {
+pub(super) fn tr(locale: Locale, text: &'static str) -> String {
     gettext(locale, text).into_owned()
 }
 
@@ -100,8 +100,14 @@ fn month_name(month: i8, locale: Locale) -> String {
     tr(locale, MONTHS[at % 12])
 }
 
+/// "January".
+pub(super) fn full_month_name(month: i8, locale: Locale) -> String {
+    let at = usize::try_from(month - 1).unwrap_or_default();
+    tr(locale, FULL_MONTHS[at % 12])
+}
+
 /// The year as a person reads it: `2026`, and `44 BC` before 1 AD.
-fn year_words(year: i16, locale: Locale) -> String {
+pub(super) fn year_words(year: i16, locale: Locale) -> String {
     if year < 1 {
         format!("{} {}", 1 - i32::from(year), tr(locale, "BC"))
     } else {
@@ -175,12 +181,9 @@ pub fn refusal_text(refusal: &Refusal, type_name: &str, locale: Locale) -> Strin
         Refusal::NoMonth(month) => {
             say("There is no month {month}.").replace("{month}", &month.to_string())
         }
-        Refusal::NoDay { day, month } => {
-            let at = usize::try_from(month - 1).unwrap_or_default();
-            say("There is no {day} {month}.")
-                .replace("{day}", &day.to_string())
-                .replace("{month}", &tr(locale, FULL_MONTHS[at % 12]))
-        }
+        Refusal::NoDay { day, month } => say("There is no {day} {month}.")
+            .replace("{day}", &day.to_string())
+            .replace("{month}", &full_month_name(*month, locale)),
         Refusal::NoTime(typed) => say("{time} is not a time of day.").replace("{time}", typed),
         Refusal::Range(range) => match range {
             Range::PgDate => say("PostgreSQL date holds 4714-11-24 BC to 5874897 AD."),
@@ -285,11 +288,13 @@ fn reading_words(reading: &Reading, column: &Column, locale: Locale) -> String {
 
 /// The line for the text `typed`, which reads as `moment`, in a column
 /// `column` of the type `type_name` (as the grid's header names it).
+/// `was` is the day the cell had before the edit, where the line names it
+/// (the terminal look's, under its calendar): said where the text reads
+/// as another.
 pub fn line(
     typed: &str,
     moment: &Result<Reading, Refusal>,
-    column: &Column,
-    type_name: &str,
+    (column, type_name, was): (&Column, &str, Option<Date>),
     (look, locale): (&Look, Locale),
 ) -> Line {
     let plain = |tone: Tone, words: String| Line {
@@ -311,7 +316,29 @@ pub fn line(
     }
     match moment {
         Ok(reading) => {
-            let words = reading_words(reading, column, locale);
+            let mut words = reading_words(reading, column, locale);
+            let day = match &reading.moment {
+                Moment::At { date, .. } => *date,
+                Moment::Infinity | Moment::MinusInfinity | Moment::Span(_) | Moment::EndOfDay => {
+                    None
+                }
+            };
+            if let Some(was) = was.filter(|was| Some(*was) != day) {
+                let same_year = day.is_none_or(|day| day.year() == was.year());
+                let said = if same_year {
+                    short_day(was, locale)
+                } else {
+                    format!(
+                        "{} {}",
+                        short_day(was, locale),
+                        year_words(was.year(), locale)
+                    )
+                };
+                words.push_str(&format!(
+                    " · {}",
+                    tr(locale, "was {day}").replace("{day}", &said)
+                ));
+            }
             let sent = (reading.literal != typed.trim()).then(|| reading.literal.clone());
             let Some(fold) = &reading.fold else {
                 return Line {
@@ -395,6 +422,8 @@ pub struct Shown {
     pub pressed: bool,
     /// The text an offer rewrites the field to, once clicked.
     pub picked: Option<String>,
+    /// The pick ends the edit: the calendar's NULL or DEFAULT.
+    pub commit: bool,
 }
 
 /// The widths of the macOS popover and of the terminal's float.
@@ -405,6 +434,15 @@ const PAD: egui::Vec2 = vec2(10.0, 8.0);
 /// The gap between the cell and the line.
 const GAP: f32 = 4.0;
 
+/// The calendar under the line, and what it is read at.
+pub struct Sheet<'a> {
+    pub model: &'a crate::ui::calendar::Model,
+    /// What its month is kept by: the field and the cell it edits.
+    pub memory: Id,
+    pub column: &'a Column,
+    pub clock: &'a crate::dates::Clock,
+}
+
 /// The float's head in the terminal look: the column and its type, and the
 /// zone a zoned value is in.
 pub struct Head<'a> {
@@ -421,13 +459,18 @@ pub fn show(
     anchor: Rect,
     field: Id,
     (line, head, kind): (&Line, &Head<'_>, Kind),
+    sheet: Option<Sheet<'_>>,
     (look, palette, locale): (&Look, &Palette, Locale),
 ) -> Shown {
     let mut shown = Shown::default();
     let screen = ui.ctx().content_rect();
-    let width = (if look.terminal { TERMINAL_WIDTH } else { WIDTH })
-        .max(anchor.width().min(WIDTH * 1.5))
-        .min(screen.width() - 16.0);
+    // The macOS calendar is as wide as its board draws it.
+    let width = match (&sheet, look.terminal) {
+        (Some(_), false) => WIDTH,
+        (_, true) => TERMINAL_WIDTH.max(anchor.width().min(WIDTH * 1.5)),
+        (None, false) => WIDTH.max(anchor.width().min(WIDTH * 1.5)),
+    }
+    .min(screen.width() - 16.0);
     let inner = width - 2.0 * PAD.x;
     let small = widgets::secondary(look);
     let color = match line.tone {
@@ -464,7 +507,10 @@ pub fn show(
     } else {
         look.control_height + PAD.y
     };
-    let height = head_height + laid.height() + 2.0 * PAD.y + offers_height;
+    let calendar_height = sheet
+        .as_ref()
+        .map_or(0.0, |sheet| crate::ui::calendar::height(sheet.model, look));
+    let height = head_height + laid.height() + 2.0 * PAD.y + offers_height + calendar_height;
     let below = anchor.bottom() + GAP;
     let top = if below + height > screen.bottom() && anchor.top() - GAP - height >= screen.top() {
         anchor.top() - GAP - height
@@ -550,6 +596,21 @@ pub fn show(
                 shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                 x = button.right() + 6.0;
             }
+            if let Some(sheet) = &sheet {
+                let top = pos2(rect.left(), rect.bottom() - calendar_height);
+                let asked = crate::ui::calendar::show(
+                    ui,
+                    (top, rect.width()),
+                    sheet.memory,
+                    (sheet.model, sheet.column, sheet.clock),
+                    (look, palette, locale),
+                );
+                shown.pressed |= asked.pressed;
+                if let Some(text) = asked.text {
+                    shown.picked = Some(text);
+                    shown.commit = asked.commit;
+                }
+            }
             ui.painter().add(border);
         });
     shown
@@ -572,11 +633,21 @@ pub fn under(
         return;
     };
     let (look, _, locale) = skin;
+    let model = target.calendar.as_ref().map(|calendar| {
+        let offers = (target.nullable, target.default);
+        let was = editor.was.as_ref();
+        crate::ui::calendar::model((&editor.text, moment, was), column, calendar, offers)
+    });
+    // The terminal look names the day before the edit on its line, where
+    // macOS marks it in the month.
+    let was = model
+        .as_ref()
+        .filter(|_| look.terminal)
+        .and_then(|model| model.was);
     let said = line(
         &editor.text,
         moment,
-        column,
-        &target.type_name,
+        (column, &target.type_name, was),
         (look, locale),
     );
     let zone = column
@@ -588,7 +659,23 @@ pub fn under(
         type_name: &target.type_name,
         zone,
     };
-    let shown = show(ui, anchor, target.id, (&said, &head, column.kind), skin);
+    let sheet = model
+        .as_ref()
+        .zip(target.calendar.as_ref())
+        .map(|(model, calendar)| Sheet {
+            model,
+            memory: target.id.with((editor.cell.row, editor.cell.col)),
+            column,
+            clock: &calendar.clock,
+        });
+    let shown = show(
+        ui,
+        anchor,
+        target.id,
+        (&said, &head, column.kind),
+        sheet,
+        skin,
+    );
     if shown.pressed {
         ui.memory_mut(|memory| memory.request_focus(target.id));
         outcome.left = false;
@@ -597,6 +684,9 @@ pub fn under(
     if let Some(text) = shown.picked {
         editor.text = text;
         outcome.changed = true;
+        if shown.commit {
+            outcome.commit = Some(crate::model::Advance::Stay);
+        }
     }
 }
 
@@ -628,7 +718,12 @@ mod tests {
     fn line_of(dialect: Dialect, type_name: &str, typed: &str, look: &Look) -> Line {
         let column = Column::of(dialect, type_name, Some(&new_york()), []).unwrap();
         let moment = read(typed, &column, &clock());
-        line(typed, &moment, &column, type_name, (look, Locale::English))
+        line(
+            typed,
+            &moment,
+            (&column, type_name, None),
+            (look, Locale::English),
+        )
     }
 
     fn said(dialect: Dialect, type_name: &str, typed: &str) -> String {
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -15,6 +15,7 @@ mod agents_tests;
 mod agent_settings_tests;
 #[cfg(test)]
 mod appearance_settings_tests;
+pub mod calendar;
 #[cfg(test)]
 mod calendar_tests;
 pub mod cell_editor;
```

`src/ui/row_form.rs`:

```diff
diff --git a/src/ui/row_form.rs b/src/ui/row_form.rs
--- a/src/ui/row_form.rs
+++ b/src/ui/row_form.rs
@@ -104,7 +104,12 @@ impl<'a> Form<'a> {
         tab: ConnTabId,
         row: usize,
         editor: Option<&'a mut Editor>,
-        (hold, terminal, keys): (bool, bool, &crate::ui::cell_editor::Keys),
+        (hold, terminal, keys, calendar): (
+            bool,
+            bool,
+            &crate::ui::cell_editor::Keys,
+            Option<&crate::ui::cell_editor::Calendar>,
+        ),
     ) -> Self {
         // Only an editor on this row is the panel's to draw.
         let editor = editor.filter(|editor| editor.cell.row == row);
@@ -153,7 +158,7 @@ impl<'a> Form<'a> {
             })
             .collect();
         let target = editor.as_ref().and_then(|editor| {
-            let how = (hold, keys);
+            let how = (hold, keys, calendar);
             let mut target =
                 crate::ui::data_view::editor_target(workspace, object, tab, editor.cell, how)?;
             // The panel's editor is out of the tab while the panel draws:
```

`src/ui/row_panel.rs`:

```diff
diff --git a/src/ui/row_panel.rs b/src/ui/row_panel.rs
--- a/src/ui/row_panel.rs
+++ b/src/ui/row_panel.rs
@@ -398,6 +398,7 @@ fn draw(
     // An editor that is open has the keyboard, unless a dialog has it.
     let hold = app.dialog.is_none();
     let editor_keys = crate::ui::cell_editor::Keys::of(&app.keymap, app.layout());
+    let calendar = app.calendar();
     // `za` asked to fold the documents.
     let fold = app.workspace_mut(tab).is_some_and(|workspace| {
         let asked = workspace.fold_documents == Some(id);
@@ -502,7 +503,7 @@ fn draw(
             focus::region(ui, Region::Panel, full);
             let mut form = match object {
                 Some(object) => {
-                    let how = (hold, look.terminal, &editor_keys);
+                    let how = (hold, look.terminal, &editor_keys, calendar.as_ref());
                     Form::of(workspace, object, tab, cell.row, editor, how)
                 }
                 None => Form::none(),
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`, then `cd docs && bin/check`.
Expected: clean; the library's tests `test result: ok. 3475 passed; 0 failed; 17 ignored`, `calendar_tests` 12 of them, and run 1's `date_tests` as before (they run with the calendar shown).

- [ ] **Step 5: Commit**

```bash
git add src/ui/calendar.rs src/ui/calendar_tests.rs src/ui/date_line.rs src/ui/cell_editor.rs src/ui/data_view.rs src/ui/row_form.rs src/ui/row_panel.rs src/edit.rs src/app/editing.rs src/app.rs src/settings.rs src/ui/mod.rs src/ui/date_tests.rs docs/_guide/editing-data.md docs/_reference/settings-and-files.md
git commit -m "Show a calendar under the line of a date's editor"
```

---

### Task 3: The switch in Settings, and `:set calendar`

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/keymap.rs`, `src/app/editing.rs`, `src/ui/settings_access_tests.rs`, `src/ui/mod.rs`, `src/ui/calendar_tests.rs`, `docs/_guide/editing-data.md`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md`

`OptionId::DatePicker` and `OptionValue::DatePicker` make the key a row of Settings › General › Data: a switch on macOS and Windows, with its small print, and on and off in the terminal screen, whose hint names the prompt's two lines. `Command::ShowCalendar` and `Command::HideCalendar` are the keymap's `:set calendar` and `:set nocalendar`, Omarchy's only, from any tab; `run_command` sends them as the switch's `SetOption`, which saves the file. The General pane's controls are one more: the sheet's Tab test goes back eight.

- [ ] **Step 1: Write the failing tests**

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -4808,6 +4808,14 @@ mod tests {
             ex(Scope::SqlEditor, "keys cheatsheet"),
             command(Command::KeysCheatsheet)
         );
+        assert_eq!(
+            ex(Scope::Grid, "set calendar"),
+            command(Command::ShowCalendar)
+        );
+        assert_eq!(
+            ex(Scope::SqlEditor, "set nocalendar"),
+            command(Command::HideCalendar)
+        );
         // Each in its own scope only, and `:w` never anything but a save.
         assert_eq!(ex(Scope::Grid, "rw"), Nothing);
         assert_eq!(ex(Scope::SqlEditor, "w"), command(Command::WriteNothing));
```

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -4581,6 +4581,7 @@ bar = \"label\"  # tint | label
                 Key::Tz,
                 Key::GroupDigits,
                 Key::ValueTags,
+                Key::DatePicker,
                 Key::ProductionReadOnly,
                 Key::ConfirmCloseEdits,
                 Key::IdleDisconnect,
```

`src/ui/calendar_tests.rs`:

```diff
diff --git a/src/ui/calendar_tests.rs b/src/ui/calendar_tests.rs
--- a/src/ui/calendar_tests.rs
+++ b/src/ui/calendar_tests.rs
@@ -51,6 +51,24 @@ fn selected(harness: &mut Harness) -> Vec<String> {
         .collect()
 }
 
+/// What the settings file was last saved as.
+fn saved(harness: &Harness) -> String {
+    harness
+        .app
+        .backend
+        .sent
+        .iter()
+        .rev()
+        .find_map(|command| match command {
+            crate::backend::Command::Save {
+                file: crate::backend::StateFile::Settings(text),
+                ..
+            } => Some(text.clone()),
+            _ => None,
+        })
+        .unwrap_or_default()
+}
+
 /// `created_at` takes NULL.
 fn nullable(harness: &mut Harness, tab: crate::model::ConnTabId, id: crate::model::TabId) {
     let workspace = harness.app.workspace_mut(tab).unwrap();
@@ -324,7 +342,7 @@ fn null_and_default_are_offered_only_where_the_column_takes_them() {
 }
 
 #[test]
-fn the_setting_shows_the_line_alone_and_reloads_live() {
+fn the_setting_and_its_lines_show_the_line_alone_and_reload_live() {
     let (mut harness, tab, id) = covers(Driver::Postgres, "date", "2026-01-15");
     open(&mut harness, tab, id);
     assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
@@ -338,6 +356,26 @@ fn the_setting_shows_the_line_alone_and_reloads_live() {
     assert!(says(&mut harness, "Thu 15 Jan 2026"));
     // The database's now is back beside the field.
     assert!(node(&mut harness, "now", Role::Button).is_some());
+    harness.press(Key::Escape, Modifiers::NONE);
+    // The terminal's prompt sets it, and saves it as Settings does.
+    let run = |harness: &mut Harness, line: &str| {
+        harness.app.workspace_mut(tab).unwrap().command = Some(line.into());
+        harness.app.apply(Action::RunCommand(tab));
+    };
+    run(&mut harness, "set calendar");
+    assert!(harness.app.settings.date_picker);
+    // On, the default, the file does not name it.
+    assert!(!saved(&harness).contains("date_picker"));
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness).as_deref(), Some("January 2026"));
+    harness.press(Key::Escape, Modifiers::NONE);
+    run(&mut harness, "set nocalendar");
+    assert!(!harness.app.settings.date_picker);
+    open(&mut harness, tab, id);
+    assert_eq!(month(&mut harness), None);
+    assert!(says(&mut harness, "Thu 15 Jan 2026"));
+    let written = saved(&harness);
+    assert!(written.contains("date_picker = false"), "{written}");
 }
 
 #[test]
```

`src/ui/mod.rs`:

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -1905,7 +1905,7 @@ value_tags   = true
             let lead = harness.painted_rect(lead).expect("the tab's lead");
             assert!(control.top() > lead.bottom(), "{control:?} {lead:?}");
             // And back up with it: the first row's menu.
-            for _ in 0..7 {
+            for _ in 0..8 {
                 harness.press(egui::Key::Tab, egui::Modifiers::SHIFT);
             }
             let tree = harness.settle();
```

`src/ui/settings_access_tests.rs`:

```diff
diff --git a/src/ui/settings_access_tests.rs b/src/ui/settings_access_tests.rs
--- a/src/ui/settings_access_tests.rs
+++ b/src/ui/settings_access_tests.rs
@@ -296,6 +296,11 @@ fn the_terminal_generals_rows_are_described_by_their_hints() {
             (OptionId::GroupDigits, Some("grouping off")),
             // Two tags, as the grid shows them.
             (OptionId::ValueTags, Some("print  ebook")),
+            // The prompt's lines that set it.
+            (
+                OptionId::DatePicker,
+                Some(":set calendar · :set nocalendar"),
+            ),
             (
                 OptionId::ProductionReadOnly,
                 Some("default for new connections"),
```

- [ ] **Step 2: Run them to see them fail**

Run: `cargo test --locked --lib`
Expected: no build, 3 errors:

```
error[E0599]: no variant, associated function, or constant named `DatePicker` found for enum `settings::OptionId` in the current scope
error[E0599]: no variant, associated function, or constant named `HideCalendar` found for enum `keymap::Command` in the current scope
error[E0599]: no variant, associated function, or constant named `ShowCalendar` found for enum `keymap::Command` in the current scope
```

With the two tests that name them set aside (`src/ui/settings_access_tests.rs`'s and the keymap's `ex` test), it builds and these fail: `settings::tests::every_option_is_stored_under_its_own_key_and_starts_at_the_default`, `ui::calendar_tests::the_setting_and_its_lines_show_the_line_alone_and_reload_live` (`:set calendar` is no line yet) and `ui::tests::tab_brings_a_control_the_rows_scrolled_away_into_view`.

- [ ] **Step 3: Write the code**

`docs/_guide/editing-data.md`:

```diff
diff --git a/docs/_guide/editing-data.md b/docs/_guide/editing-data.md
--- a/docs/_guide/editing-data.md
+++ b/docs/_guide/editing-data.md
@@ -111,8 +111,9 @@ would: what the line says is still what is stored.
 
 The Omarchy look draws the month, and a timestamp's time, under its
 line, which says the day before the edit too (`was 15 jan`); they take no
-click, and their keys come later. Turn the calendar off with
-`date_picker = false` under `[data]` in `settings.toml`: the line stays.
+click, and their keys come later. Turn the calendar off with **Show a
+calendar when editing dates** in Settings › General, or `:set
+nocalendar` in the Omarchy look: the line stays.
 
 #### Zones
 
```

`docs/_guide/omarchy.md`:

```diff
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -48,6 +48,7 @@ second Esc gives the keyboard back.
 | `:history clear` | Clear this connection's history, once you answer `y` |
 | `:keys default` | Open every key in force in your editor, as `settings.toml` spells them, commented: a file to copy lines from, written again each time and never read |
 | `:keys cheatsheet` | Save the keys in force as a Markdown cheat sheet, where you say |
+| `:set calendar`, `:set nocalendar` | Show or hide the [calendar]({% link _guide/editing-data.md %}#the-calendar) under a date's editor, saved as its switch in Settings is |
 | `?` | The list of keys |
 
 Ctrl+P opens quick open from any pane: a picker at the top of the
```

`docs/_reference/settings-and-files.md`:

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -19,6 +19,7 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Time zone for timestamptz** | System, UTC or any zone. A PostgreSQL `timestamptz` is shown in this zone, a copy gives the value as the server sent it, and a value you type without an offset is sent with this zone's. In the Omarchy look `h`/`l` choose System, UTC or the last zone the file named; any other zone is set in the file | System |
 | **Numbers** | `1,240.50` or `1240.50`. Grouping is for display only: a copy gives the raw value | `1,240.50` on macOS and Windows, `1240.50` in the Omarchy look |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
+| **Show a calendar when editing dates** | On or off. On, the month and the time fields stand under the editor of a date, a time or a timestamp (a date has no time fields, a time no month), with the line that says what is stored; off, the line alone. In the Omarchy look `:set calendar` and `:set nocalendar` set it too | On |
 | **New production connections open read-only** | On or off: where the **Open read-only** box of a new production connection starts. A connection you saved keeps its own box | On |
 | **Ask before closing with unsaved edits** | On or off. Off, closing a table, its connection or the window, or loading another page of it, discards its pending changes without asking | On |
 | **Disconnect when idle** | Never, or after 5, 15 or 30 minutes, 1 hour or 4 hours unused. The server session is closed; the tabs stay, and the connection connects again when something next needs it: a refresh, a page, a table opened, a run or a save | After 30 min |
@@ -176,7 +177,7 @@ bar = "tint"
 | `data.tz` | `"system"`, `"utc"` or a zone such as `"America/New_York"`. A name the zone database does not know is a line that cannot be read. A PostgreSQL `timestamptz` value is shown and edited in this zone, a copy gives it as the server sent it, and one you type without an offset is sent with this zone's | `"system"` |
 | `data.group_digits` | `true` or `false` | Not set: grouped on macOS and Windows, not in the Omarchy look |
 | `data.value_tags` | `true` or `false` | `true` |
-| `data.date_picker` | `true` or `false`. The calendar (the month and the time fields) under the editor of a date, a time or a timestamp; the line that says what is stored shows either way | `true` |
+| `data.date_picker` | `true` or `false`. The calendar (the month and the time fields) under the editor of a date, a time or a timestamp; the line that says what is stored shows either way. `:set calendar` and `:set nocalendar` set it in the Omarchy look | `true` |
 | `safety.production_read_only` | `true` or `false`. Where the **Open read-only** box of a new production connection starts; the connection is saved with its box set, so a change here changes no saved connection | `true` |
 | `safety.confirm_close_edits` | `true` asks before pending changes are dropped; `false` discards them without asking. While a save runs, what would drop them does nothing until it ends, and closing the window asks either way | `true` |
 | `safety.idle_disconnect` | `"never"`, `"5m"`, `"15m"`, `"30m"`, `"1h"` or `"4h"`. A connection left unused that long has its server session closed; its tabs stay as they were, and it connects again when you next refresh, page, open a table, run or save. Never while changes are pending or something is still loading, running or saving | `"30m"` |
```

`src/app/editing.rs`:

```diff
diff --git a/src/app/editing.rs b/src/app/editing.rs
--- a/src/app/editing.rs
+++ b/src/app/editing.rs
@@ -1942,6 +1942,14 @@ impl App {
             // sheet where the person says. Neither touches the settings.
             (Typed::Command(Command::KeysDefault), _) => Action::KeysDefault,
             (Typed::Command(Command::KeysCheatsheet), _) => Action::ExportCheatSheet,
+            // The calendar under a date's editor, saved as its switch in
+            // Settings saves it.
+            (Typed::Command(Command::ShowCalendar), _) => {
+                Action::SetOption(crate::settings::OptionValue::DatePicker(true))
+            }
+            (Typed::Command(Command::HideCalendar), _) => {
+                Action::SetOption(crate::settings::OptionValue::DatePicker(false))
+            }
             (Typed::Command(Command::ClearHistory), _) => Action::ClearHistory(tab),
             // From any pane: of what the tree's cursor is on, or the tab in
             // front.
```

`src/keymap.rs`:

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -505,6 +505,8 @@ commands! {
     Help = "help",
     KeysDefault = "keys_default",
     KeysCheatsheet = "keys_cheatsheet",
+    ShowCalendar = "show_calendar",
+    HideCalendar = "hide_calendar",
     // Grid and editing.
     MoveRow = "move_row",
     MoveColumn = "move_column",
@@ -859,6 +861,8 @@ impl Command {
             C::Help => info("Help, all keys", "Help, all keys", Navigation),
             C::KeysDefault => info("Write the full list of keys", "", Navigation),
             C::KeysCheatsheet => info("Write a cheat sheet of the keys", "", Navigation),
+            C::ShowCalendar => info("Show a calendar when editing dates", "", Editing),
+            C::HideCalendar => info("Edit dates without the calendar", "", Editing),
             C::MoveRow => info("Move by row", "", Editing),
             C::MoveColumn => info("Move by column", "", Editing),
             C::MovePage => info("Move by page", "", Editing),
@@ -1512,6 +1516,10 @@ pub const BINDINGS: &[Binding] = {
             NONE,
             keys(&[k(":keys cheatsheet")]),
         ),
+        // The setting of the calendar under a date's editor, as Settings
+        // has it.
+        bind(C::ShowCalendar, GLOBAL, NONE, keys(&[k(":set calendar")])),
+        bind(C::HideCalendar, GLOBAL, NONE, keys(&[k(":set nocalendar")])),
         // Grid and editing.
         bind(
             C::MoveRow,
```

`src/model.rs`:

```diff
diff --git a/src/model.rs b/src/model.rs
--- a/src/model.rs
+++ b/src/model.rs
@@ -3094,6 +3094,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::Tz),
                 SettingsRow::Option(OptionId::GroupDigits),
                 SettingsRow::Option(OptionId::ValueTags),
+                SettingsRow::Option(OptionId::DatePicker),
                 SettingsRow::Option(OptionId::ProductionReadOnly),
                 SettingsRow::Option(OptionId::ConfirmCloseEdits),
                 SettingsRow::Option(OptionId::IdleDisconnect),
```

`src/settings.rs`:

```diff
diff --git a/src/settings.rs b/src/settings.rs
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -912,6 +912,8 @@ pub enum OptionId {
     Tz,
     GroupDigits,
     ValueTags,
+    /// The calendar under a date's editor: `data.date_picker`.
+    DatePicker,
     /// Where a new production connection's read-only switch starts:
     /// `safety.production_read_only`.
     ProductionReadOnly,
@@ -967,7 +969,7 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's, the Editor
     /// pane's and then the Agents pane's.
-    pub const ALL: [OptionId; 37] = [
+    pub const ALL: [OptionId; 38] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -975,6 +977,7 @@ impl OptionId {
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::DatePicker,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
@@ -1007,7 +1010,7 @@ impl OptionId {
         Self::AgentsNotify,
     ];
     /// The General pane's, in the order it has them.
-    pub const GENERAL: [OptionId; 10] = [
+    pub const GENERAL: [OptionId; 11] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -1015,18 +1018,20 @@ impl OptionId {
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::DatePicker,
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
     ];
     /// The General pane's sections, in order: Startup, Data and Safety.
     pub const STARTUP: [OptionId; 2] = [Self::OnLaunch, Self::RestoreTabs];
-    pub const DATA: [OptionId; 5] = [
+    pub const DATA: [OptionId; 6] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::DatePicker,
     ];
     pub const SAFETY: [OptionId; 3] = [
         Self::ProductionReadOnly,
@@ -1125,6 +1130,7 @@ impl OptionId {
             Self::Tz => Key::Tz,
             Self::GroupDigits => Key::GroupDigits,
             Self::ValueTags => Key::ValueTags,
+            Self::DatePicker => Key::DatePicker,
             Self::ProductionReadOnly => Key::ProductionReadOnly,
             Self::ConfirmCloseEdits => Key::ConfirmCloseEdits,
             Self::IdleDisconnect => Key::IdleDisconnect,
@@ -1175,6 +1181,7 @@ pub enum OptionValue {
     /// `None` is the look's default.
     GroupDigits(Option<bool>),
     ValueTags(bool),
+    DatePicker(bool),
     ProductionReadOnly(bool),
     ConfirmCloseEdits(bool),
     IdleDisconnect(Idle),
@@ -1251,6 +1258,7 @@ impl OptionValue {
             Self::Tz(_) => OptionId::Tz,
             Self::GroupDigits(_) => OptionId::GroupDigits,
             Self::ValueTags(_) => OptionId::ValueTags,
+            Self::DatePicker(_) => OptionId::DatePicker,
             Self::ProductionReadOnly(_) => OptionId::ProductionReadOnly,
             Self::ConfirmCloseEdits(_) => OptionId::ConfirmCloseEdits,
             Self::IdleDisconnect(_) => OptionId::IdleDisconnect,
@@ -1294,6 +1302,7 @@ impl OptionValue {
             Self::Tz(zone) => settings.tz = zone,
             Self::GroupDigits(on) => settings.group_digits = on,
             Self::ValueTags(on) => settings.value_tags = on,
+            Self::DatePicker(on) => settings.date_picker = on,
             Self::ProductionReadOnly(on) => settings.production_read_only = on,
             Self::ConfirmCloseEdits(on) => settings.confirm_close_edits = on,
             Self::IdleDisconnect(choice) => settings.idle_disconnect = choice,
@@ -1959,6 +1968,7 @@ impl Settings {
             OptionId::Tz => OptionValue::Tz(self.tz.clone()),
             OptionId::GroupDigits => OptionValue::GroupDigits(self.group_digits),
             OptionId::ValueTags => OptionValue::ValueTags(self.value_tags),
+            OptionId::DatePicker => OptionValue::DatePicker(self.date_picker),
             OptionId::ProductionReadOnly => {
                 OptionValue::ProductionReadOnly(self.production_read_only)
             }
@@ -2044,6 +2054,7 @@ impl Settings {
             // Grouped is the left of the two.
             OptionId::GroupDigits => OptionValue::GroupDigits(Some(!forward)),
             OptionId::ValueTags => OptionValue::ValueTags(forward),
+            OptionId::DatePicker => OptionValue::DatePicker(forward),
             OptionId::ProductionReadOnly => OptionValue::ProductionReadOnly(forward),
             OptionId::ConfirmCloseEdits => OptionValue::ConfirmCloseEdits(forward),
             OptionId::RestoreTabs => OptionValue::RestoreTabs(forward),
@@ -2298,6 +2309,7 @@ impl Settings {
                 !self.group_digits.unwrap_or_default(),
             ))),
             OptionId::ValueTags => Some(OptionValue::ValueTags(!self.value_tags)),
+            OptionId::DatePicker => Some(OptionValue::DatePicker(!self.date_picker)),
             OptionId::ProductionReadOnly => {
                 Some(OptionValue::ProductionReadOnly(!self.production_read_only))
             }
```

`src/ui/settings/mod.rs`:

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -41,6 +41,7 @@ pub(crate) fn label(option: OptionId) -> &'static str {
         OptionId::Tz => "Time zone for timestamptz",
         OptionId::GroupDigits => "Numbers",
         OptionId::ValueTags => "Value tags",
+        OptionId::DatePicker => "Show a calendar when editing dates",
         OptionId::ProductionReadOnly => "New production connections open read-only",
         OptionId::ConfirmCloseEdits => "Ask before closing with unsaved edits",
         OptionId::IdleDisconnect => "Disconnect when idle",
@@ -106,6 +107,7 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         OptionId::Tz => "timestamptz zone",
         OptionId::ProductionReadOnly => "production opens read-only",
         OptionId::ConfirmCloseEdits => "confirm close with edits",
+        OptionId::DatePicker => "calendar for dates",
         OptionId::RowHeight => "Row height",
         OptionId::Zebra => "Alternate rows",
         OptionId::ColumnTypes => "Column types",
@@ -154,6 +156,7 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::OnLaunch => Some("Or: show Connections · open a chosen connection"),
         OptionId::PageSize => Some("Table view; the SQL editor has its own limit"),
         OptionId::ValueTags => Some("Colors for enum, CHECK (…IN…) and boolean columns"),
+        OptionId::DatePicker => Some("Under date and time cells; typing always works"),
         OptionId::LogDays => Some("Every call and its SQL, never the rows. On this computer only"),
         OptionId::Tz => Some("Values are shown in this zone and sent with their offset"),
         OptionId::ProductionReadOnly => Some("Default for the switch in each connection"),
@@ -297,6 +300,7 @@ pub(super) fn is_on(value: &OptionValue) -> bool {
         value,
         OptionValue::RestoreTabs(true)
             | OptionValue::ValueTags(true)
+            | OptionValue::DatePicker(true)
             | OptionValue::ProductionReadOnly(true)
             | OptionValue::ConfirmCloseEdits(true)
             | OptionValue::AgentsOn(true)
@@ -434,6 +438,10 @@ pub(super) fn choices(
             (Word("off"), OptionValue::ValueTags(false)),
             (Word("on"), OptionValue::ValueTags(true)),
         ],
+        OptionId::DatePicker => vec![
+            (Word("off"), OptionValue::DatePicker(false)),
+            (Word("on"), OptionValue::DatePicker(true)),
+        ],
         OptionId::ProductionReadOnly => vec![
             (Word("off"), OptionValue::ProductionReadOnly(false)),
             (Word("on"), OptionValue::ProductionReadOnly(true)),
```

`src/ui/settings/sheet.rs`:

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -852,6 +852,7 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         | OptionId::Wrap => SWITCH,
         OptionId::RestoreTabs
         | OptionId::ValueTags
+        | OptionId::DatePicker
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1150,6 +1151,7 @@ fn control(
         }
         OptionId::RestoreTabs
         | OptionId::ValueTags
+        | OptionId::DatePicker
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1512,6 +1514,7 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         | OptionId::RestoreTabs
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
+        | OptionId::DatePicker
         | OptionId::IdleDisconnect
         | OptionId::AgentsOn
         | OptionId::LogDays
```

`src/ui/settings/terminal.rs`:

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -1111,6 +1111,7 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         }
         OptionId::RestoreTabs
         | OptionId::ValueTags
+        | OptionId::DatePicker
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1393,6 +1394,7 @@ fn option_row(
         }
         OptionId::RestoreTabs
         | OptionId::ValueTags
+        | OptionId::DatePicker
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
@@ -1569,6 +1571,11 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
                 .space(role, "  ")
                 .add(role, "ebook", second)
         }
+        // The prompt's lines that set it, from the keymap.
+        OptionId::DatePicker => Text::new(look)
+            .add(role, &key(ui, look, Command::ShowCalendar), palette.accent)
+            .add(role, " · ", palette.dim)
+            .add(role, &key(ui, look, Command::HideCalendar), palette.accent),
         OptionId::RestoreTabs => dim(&skin.say("per connection")),
         OptionId::OnLaunch => dim(&skin.say("or connections · a chosen connection")),
         OptionId::ProductionReadOnly => dim(&skin.say("default for new connections")),
```

- [ ] **Step 4: Run the checks**

Run: `cargo fmt --all --check && cargo clippy --locked --workspace --all-targets -- -D warnings && RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps && cargo test --locked --workspace --all-targets`, then `cd docs && bin/check`.
Expected: clean; the library's tests `test result: ok. 3475 passed; 0 failed; 17 ignored`; `tests/keys.rs` and `tests/engines.rs` pass.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs src/model.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/keymap.rs src/app/editing.rs src/ui/settings_access_tests.rs src/ui/mod.rs src/ui/calendar_tests.rs docs/_guide/editing-data.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md
git commit -m "Set the calendar in Settings, and with :set calendar"
```

---

## After Task 3

Push the branch and open the pull request. Its description has what the prompt's Delivery asks of this run: the summary; the setting (`data.date_picker`, Settings › General, `:set calendar` and `:set nocalendar`) and that the keys are run 3's; the first weekday by the locale's region; the keymap check (`:set calendar` and `:set nocalendar` are Omarchy's, from any tab, and the keymap's generated tests find no other command on them); the open questions 1 (the locale's first weekday, as proposed) and 2 (the setting: on by default, where the table above says); and the departures above. No screenshot.

## Checks by hand

Off the record, on the demo database (Bookshop) on each engine and in both looks: open `books.published` (a date) and `orders.placed_at` (a timestamp). The popover sits under the cell and under the row panel's field; a click on a day, on ‹, ›, Today and Now leaves the keyboard in the field; Enter, Esc and a click away do what they did; NULL and DEFAULT show only where the column takes them. Compare with the boards by eye: `macOS – Date and time picker`, `… by type and state` and `Omarchy – Date and time picker`. Nothing of it is a test.
