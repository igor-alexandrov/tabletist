# The date and time picker

The prompt this work was given, as it was given. It asked for one pull
request; asked on 2026-10-25, Igor chose three runs, a pull request each:

1. **Typed dates** (`docs/superpowers/plans/2026-10-25-date-time-picker-1-typed-dates.md`):
   which columns are dates and times, reading what is typed (ISO 8601,
   names of months, the locale's order of numbers, times, relative words),
   zones with their changes of the clocks, each type's limits, BC,
   infinity, MySQL's zero date and session zone, the literal a save sends
   (SQLite keeping the form its column has), and the line under the editor
   that says what is stored, in the grid and the row panel, in both looks.
2. **The calendar**: the macOS popover's month and time fields, the
   Omarchy float's month, clicks, Now and Today, the footer's NULL,
   DEFAULT and ∞, the locale's first weekday, accessibility of the month,
   and the setting with `:set calendar`.
3. **The picker's keys**: macOS Tab into the month and the time fields,
   Omarchy's chords, Ctrl+A and Ctrl+X, the keymap and the pages.

Each run's plan says where it departs from what follows.

---

# Prompt: build the date and time picker

You are working on Tabletist, a native database client written in Rust (PostgreSQL, MySQL, SQLite), with a macOS look and an Omarchy look. Build the **date and time picker**: a calendar and time fields under the editor while you edit a date, time or timestamp value, in the grid and in the inspector.

Typing always comes first. The text field keeps focus, the picker follows what you type, and every pick rewrites the text. The text is what gets stored.

## Designs

Design canvas: **[Tabletist](https://claude.ai/artifact/4FtP3GnsXevWfpBsotjVb6)**, section **Editing · Values**, at the end of the row, after `Foreign key label, notes`:

| Board | File | Shows |
|---|---|---|
| `macOS – Date and time picker` | `MacDatePicker.dc.html` | The picker under a grid cell (`editions.published_at`, timestamptz) and under an inspector field (`released_on`, date): parsed line, month, time fields with the zone and **Now**, footer with Apply, Cancel, NULL, DEFAULT, and the rules |
| `macOS – Date and time picker, by type and state` | `MacDatePickerStates.dc.html` | **date**, **time**, **timestamp, no zone**, **Doesn't parse**, **Clock change**, **Out of range** (MySQL TIMESTAMP), **Special values** (PostgreSQL infinity), **SQLite** |
| `Omarchy – Date and time picker` | `OmarchyDatePicker.dc.html` | The float under a grid cell, the keys, and **date**, **time**, **doesn't parse**, **clock change** |

Also read:

- `macOS – Editing values, editors by type` and `Omarchy – Editing values, editors by type`, card 7. The one-week strip drawn there is replaced by this picker. The other cards (NULL, DEFAULT, invalid input staying red) still apply.
- `settings-editor-prompt.md`: where the new setting goes.
- `settings-keyboard-prompt.md`: new keys go in the keymap under the Cell scope.
- The design system **[Tabletist](https://claude.ai/artifact/C44ZLtrGKqYek9WfvpSBoq)** (`project/README.md`, `tokens.json`). Use its tokens only.

If a board and this prompt disagree, this prompt wins. List each case in the PR.

## Which columns get it

The picker is chosen from the column type, through the type mapping the editors already use, matched on `Driver` with every variant named and no `_` arm.

| Kind | PostgreSQL | MySQL | SQLite (declared type) | Picker shows |
|---|---|---|---|---|
| date | `date` | `DATE` | contains `DATE` and not `TIME` | month |
| timestamp | `timestamp` | `DATETIME` | `DATETIME`, `TIMESTAMP`, or contains both `DATE` and `TIME` | month and time fields, no zone |
| timestamp with zone | `timestamptz` | `TIMESTAMP` | none | month, time fields, zone |
| time | `time`, `timetz` | `TIME` | `TIME` | time fields only, no calendar |

- `interval`, MySQL `YEAR` and anything else keep the text editor.
- MySQL `TIMESTAMP` is converted to and from the session zone by the server. Treat it as a timestamp with a zone, using the connection's session `time_zone`.
- Arrays of these types use the array editor. Each element doesn't get its own picker in this version.

## Opening and closing

- It opens with the editor: `↩`, double-click, or typing into the cell on macOS; `i`, `cc` or `enter` on Omarchy. The same in the inspector.
- It closes with the editor. On macOS `↩` applies and `esc` cancels. On Omarchy `enter` applies, `esc` goes back to normal mode, `ctrl+c` drops the edit.
- A click outside applies, the way it does for other cell editors today.
- Locked columns (identity, generated, no UPDATE privilege) never open it.
- **Setting:** `editor.typing.date_picker = true` in Settings › Editor › Typing, labelled "Show a calendar when editing dates" (Omarchy: `:set nocalendar` / `:set calendar`, saved to the same key). When it is off, the editor is plain text with the parsed line underneath.

## Layout (macOS)

A popover anchored under the cell or field, 330px wide, `radius-card`, `shadow-popover`. The cell or field is the text field; the popover has no field of its own.

1. **Parsed line:** what will be stored, for example "Thu 15 Jan 2026, 09:30 EST · sent as `2026-01-15 09:30:00-05`". Errors and warnings replace it, in `danger-ink` or `warning-ink`.
2. **Month**, for date and timestamp kinds:
   - a header with the month and year, ‹ and ›, and **Today**;
   - a 6-row grid with weekday initials; days from the next or previous month in `ink-disabled`;
   - the new value filled with `accent`; the value from before this edit marked with a small `warning-mark` dot; today outlined with `accent-border-strong`.
3. **Time row**, for timestamp and time kinds: hour, minute, and second fields. Seconds show when the column keeps them; fractional seconds show when the precision is above 0 or the value has them. Then the zone chip (the city, for example "New York"; a timestamp without zone shows "no zone") and **Now**.
4. **Footer:** `↩` Apply, `esc` Cancel, and on the right **NULL** (only when the column allows it) and **DEFAULT** (only when it has one). On PostgreSQL date and timestamp, **−∞** and **∞** are on the left.

## Typing and parsing

- Accept at least:
  - ISO 8601 with a space or `T`, with or without seconds, fractional seconds and an offset;
  - `15 Jan 2026`, `Jan 15 2026`, `15/01/2026` in the system's date order;
  - `9:30`, `9.30`, `9am`, `21:30`;
  - `today`, `tomorrow`, `yesterday`, `now`, `+7d`, `-2w`, `next fri`, plus a time (`tomorrow 9am`).
- Relative words are shown resolved on the parsed line ("Fri 30 Oct 2026 · in 6 days") and stored as the resolved value, never as text.
- The month follows the text as soon as it parses. While it doesn't parse, the month dims and shows the month typed so far, if any, with nothing selected.
- **Doesn't parse:** the field turns red; the parsed line says why ("There is no 30 February."); `↩` and `enter` are refused, as for numbers.
- **Write format:** the editor writes a canonical literal for the column type (`2026-01-15 09:30:00-05` for timestamptz), not the typed text.
  - SQLite: the format the column already uses. If existing values are integers, write seconds since 1970. If they are text, write ISO 8601 (`YYYY-MM-DD` or `YYYY-MM-DD HH:MM:SS`). With no existing values, write ISO 8601 text.
  - Never change the format of values the user didn't edit.

## Picking

- Clicking a day sets the date part and keeps the time part (or keeps 00:00 when there was none).
- **Now** sets the current time (date and time for timestamps, today for dates). **Today** in the header shows today's month without changing the value.
- **macOS keyboard:**
  - `⇥` moves focus from the field into the month: arrows by day, `Page Up` and `Page Down` by month, `⇧` with them by year, `T` today, `↩` picks.
  - `⇥` again goes to the time fields, where `↑` `↓` step the focused part, and digits type.
  - `⇧⇥` goes back. `esc` from anywhere cancels.
- **Omarchy keys**, all in insert mode, all rewriting the typed line; the calendar never takes focus:
  - `ctrl+h` / `ctrl+l` day back and forward, `ctrl+j` / `ctrl+k` week forward and back;
  - `alt+h` / `alt+l` month, `alt+j` / `alt+k` year;
  - `ctrl+t` now (date: today);
  - `ctrl+a` / `ctrl+x` +1 / −1 on the part under the cursor (year, month, day, hour, minute, second);
  - `tab` takes the offered time in a clock change.
- Moving past the end of a month clamps (31 Jan + 1 month = 28 Feb).

## Time zones

- **timestamptz** (and MySQL TIMESTAMP) is shown and typed in the display zone, sent with its offset. The display zone is your system zone by default. Use the existing display zone setting if there is one; if there isn't, use the system zone and say so in the PR.
- **Clock change, gap:** a local time that doesn't exist ("02:30 doesn't exist on 8 Mar in New York; clocks go forward.") shows as a warning with **Use 03:30** (Omarchy: `tab`). `↩` is refused until it's resolved.
- **Clock change, repeated hour:** an ambiguous time asks which one, with two buttons: `01:30 EDT` and `01:30 EST`. The default is the first (daylight time). Omarchy cycles them with `tab`.
- **timestamp without zone** and **datetime** are stored as typed, with no conversion. The chip says "no zone".
- Use the IANA zone database (for example `chrono-tz` or `jiff`), not fixed offsets.

## Limits and special values

- Ranges come from the type, and days outside them can't be picked (dimmed, and refused when typed, with the limit on the parsed line):
  - MySQL `TIMESTAMP`: 1970-01-01 00:00:01 to 2038-01-19 03:14:07 UTC. `DATE` and `DATETIME`: 1000-01-01 to 9999-12-31.
  - PostgreSQL `date`: 4713 BC to 5874897 AD. `timestamp`: 4713 BC to 294276 AD.
  - SQLite: 0000 to 9999.
- BC years are typed as `0044-03-15 BC` on PostgreSQL. The month shows them, labelled "44 BC".
- PostgreSQL `infinity` and `-infinity` for date and timestamp: the ∞ buttons and typed `infinity`, `-infinity`. The parsed line says "No end date · stored as infinity". The month dims.
- MySQL zero dates (`0000-00-00`) already stored are shown as they are and can't be picked. The parsed line says "MySQL zero date · pick a date or set NULL".
- NULL, DEFAULT and empty follow the existing rules: NULL only on nullable columns, and DEFAULT only when there is one.

## Week start and locale

- The first weekday and the date order come from the system locale (Monday on the boards). The weekday and month names use the system language.
- Stored values are never locale formatted.

## Accessibility

- The popover is a dialog named after the column ("published_at, date and time"), attached to the field without taking focus from it.
- The month is a grid with day buttons named "Thursday 15 January 2026", and the selected day has `aria-selected`. Today is announced as "today"; the previous value as "current value".
- The parsed line is a live region (polite), and its errors are assertive.
- Time fields are spin buttons with their range.
- Omarchy: the line and the parsed line carry everything; the calendar is decoration and is hidden from screen readers.

## Tests (headless, no pixels)

Use only the Bookshop demo data in fixtures and tests. Pixel or design snapshots must never be part of any test run, and snapshot material isn't committed.

1. Type mapping: every listed type opens the right picker on each driver; `interval` and `YEAR` open text; the match names every `Driver`.
2. Parsing: each accepted form gives the expected value; relative words resolve against a fixed clock in tests; invalid text (`2026-02-30`, `25:00`) is refused with the message.
3. Picking a day keeps the time; Now and `ctrl+t` set the current value; `ctrl+a/x` step each part; month moves clamp.
4. Every pick and key rewrites the text, and the stored literal matches the parsed line.
5. timestamptz in `America/New_York`: a gap time is refused and Use 03:30 resolves it; a repeated hour asks and stores the chosen offset.
6. timestamp without zone stores exactly what was typed, whatever the display zone.
7. MySQL TIMESTAMP outside 1970 to 2038 is refused; DATE outside 1000 to 9999 is refused.
8. PostgreSQL `infinity` and `-infinity` round trip; BC dates round trip.
9. SQLite writes integers when existing values are integers, and ISO text otherwise; untouched values keep their format.
10. NULL and DEFAULT show only where allowed.
11. `editor.typing.date_picker = false` and `:set nocalendar` show the text editor with the parsed line; the setting reloads live.
12. Week start and names follow the locale in tests (Monday-first and Sunday-first).
13. `esc` cancels and the cell keeps its value; `↩` applies; a click outside applies.

## Out of scope

- Date ranges and range types (`daterange`, `tstzrange`).
- Interval editing.
- Pickers inside arrays.
- Filtering by date with the picker (the filter bar stays text).
- Calendars other than Gregorian.

## Open questions

Use the stated default, and note it in the PR.

1. **Week start:** default the system locale's first weekday. The alternative is always Monday.
2. **The setting** `editor.typing.date_picker`: default on. Confirm the label and where it sits in Settings › Editor.
3. **Display zone:** default the system zone if no display zone setting exists yet; propose where the setting would go.
4. **Repeated hour default:** daylight time first. Confirm.
5. **Keys:** Omarchy `alt+h/l` and `alt+j/k` are proposals; check them against the keymap, since `alt+N` switches tabs.

## Delivery

- One PR. Every commit passes `cargo fmt --check`, `cargo clippy -- -D warnings` and the headless tests.
- No em dashes in UI copy.
- PR description:
  - a summary;
  - the type mapping per driver;
  - how zones and clock changes are handled, and which zone library;
  - the SQLite write format rule;
  - the setting and keys added, and the keymap check;
  - the open questions and the defaults chosen;
  - any place the build differs from the boards.
- Screenshots only if blurred. No unblurred screenshot goes to GitHub.
