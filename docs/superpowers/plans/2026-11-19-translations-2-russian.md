# Translations, run 2: the strings in variables, numbers and dates: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Russian window says in Russian every string run 1 left English, because it reached `gettext` through a variable. That covers the menu bar's items and its Query, Window and Help menus, the keymap's command names, the Settings nav and labels, the `say` words and the glued SQL comments. Counts and interface dates follow the language, and grouped grid numbers use its separators.

**Architecture:** One new type, `i18n::Msgid`. A marked English message is built with `msgid("…")` or `pmsgid("context", "…")`, two `const fn`s that `update.sh` gives xgettext as keywords. Every wrapper that translated a `&'static str` it was handed (`skin.say`, the local `say` closures, `Info::name`, `SettingsPane::name`, `menu::title`) takes or returns a `Msgid` instead. So a bare literal no longer compiles where a translated one is meant, and the compiler lists every site. Tabletist gets its own `ngettext`, which takes the count as any integer and hands the catalog a `u64`. Numbers and dates get a small locale-aware module, `i18n::numbers`, used by `format.rs`, `clock.rs` and `date_line.rs`.

**Tech Stack:**
- Rust and egui 0.36.
- fastframe-i18n v0.16.1. Its `Translator::ntranslate` takes `n: u64`; only its free `ngettext` takes `u32`.
- GNU gettext 1.0 (`xgettext`, `msgmerge`, `msgfmt`, `msgcomm`).
- jiff.
- The app's `testing::Harness`.

**Spec:** run 1's spec, `docs/superpowers/specs/2026-11-17-translations-1-russian-design.md`, section "Runs", item 2. This plan adds the decisions below. Run 1's plan, `docs/superpowers/plans/2026-11-18-translations-1-russian.md`, lists what stayed English ("Not in this run").

**Base:** `main` at 73e6913f (PR #352 merged). Branch `claude/translations-2-russian`. If main moves before the PR, rebase onto it (never a merge of main), then run `assets/i18n/update.sh` again and translate what main added.

---

## What the survey found (2026-11-19)

The template has 721 msgids. Run 2 reaches about 1,000 more:

| Source | Strings | Not in the template |
|---|---|---|
| `keymap.rs` `Command::info` names and rows (320 commands) | ~330 | ~305 |
| `Group::name` / `Scope::name` | 41 | 30 |
| `menu::title` and the menus' titles | 61 | 50 |
| Command palette (`app/commands.rs` `RUNNABLE`) | 36 | 26 |
| `say` / `skin.say` / `word` / `tr` / `label` wrappers in `src/ui` | ~720 | ~630 |
| Settings panes, row labels, notes, hints | ~60 | ~50 |
| `date_line.rs` tables, `clock.rs`, `history.rs` durations, `format.rs` | ~45 | ~45 |

There are 195 `gettext`/`ngettext` sites whose msgid is not a literal. Each one becomes one of:
- a `Msgid` call;
- a whole phrase;
- a name that stays English: a product name, `env!("CARGO_PKG_DESCRIPTION")` (the About box's description), a server's own text.

xgettext 1.0 was tried on the call shapes the code uses. It extracts `msgid("…")` and `pmsgid("c", "…")` as plain calls, as method arguments (`skin.say(msgid("delete"))`), inside `format!`, in match arms, in struct fields and in `const` arrays. It does not extract a method call such as `skin.say("delete")`, so a keyword on the wrappers themselves cannot work. The marker goes on the literal.

## Decisions for Igor (before code)

| What | This plan builds | Why |
|---|---|---|
| **The marker** | `msgid("…")` / `pmsgid("ctx", "…")` returning `Msgid`, a newtype. Not a `N_` that returns `&str` | Wrappers take `Msgid`, so an unmarked literal fails to compile instead of silently staying English. `N_` would also need `#[allow(non_snake_case)]` |
| **Counts in English** | Interface counts are grouped in every language: "12,480 rows", «12 480 строк» | One rule. Some English counts are already grouped ("Exported 31,400 rows", "1–300 of 1,234,567"); `counted` and the `{n}` fills are not ("12480 rows") |
| **Russian separators** | Group: U+00A0 no-break space (CLDR `ru`); decimal mark: comma | «12 480», «1 240,50». A no-break space keeps a number on one line |
| **Grouped grid numbers in Russian** | Both separators follow the language: `1240.50` shows `1 240,50`. Ungrouped grid numbers, copy, export and the cell editor keep the value as the database wrote it (`1240.50`) | "The language's separators". Grouping is display only today and stays that way |
| **Grid dates** | Always ISO, in every language | Decided in run 1 |
| **Interface dates** | History's day headings and moments, the date line, the calendar's month: «13 окт.», «13 окт. 2025 г.», «чт 15 янв. 2026», «Январь 2026». English is as today ("Oct 13", "Thu 15 Jan 2026") | The language's order and month forms, through whole-phrase messages (`{month} {day}` → `{day} {month}`) |
| **Time** | 24-hour `10:42:18` in both languages | Already 24-hour; Russian writes it the same way |
| **Completion kind letters** (`sql_complete` K/S/T/V/C) | Stay as they are. The spoken "{label}, {kind}" is already translated | They act as icons. Russian letters would clash (С for both схема and столбец) |
| **History's "Query N"** | Recorded as English `Query 7`. When History draws a detail, it reads `Query N` in any shipped language (run 1 entries made in Russian hold «Запрос 7») and says it in the window's language | History stores what ran, so the label is read back, not rewritten in the file |
| **Plural counts** | Tabletist's own `ngettext`, `counted` and `ngettext_n` take the count as `usize`, `u64`, `u32` or `i64`, and hand the catalog a `u64`. The 69 `u32::try_from(…).unwrap_or(u32::MAX)` lines go | Russian's form for 4,294,967,301 is «строка». At the cap (4,294,967,295) it was «строк» |
| **Glued sentences behind `say`** | Rewritten as whole phrases with `fill`, as run 1 did for the rest. Examples: review.rs's `-- row {row} · blocked: fix {columns} first`, and cell_editor's `{type} holds {min} to {max}` | AGENTS.md: never a sentence glued from translated pieces |
| **One PR or three** | **Recommended: three PRs, one plan.** 2a covers the marker, plurals, keymap, menus and Settings (tasks 1–4). 2b covers the views' `say` words and the whole phrases (task 5). 2c covers numbers, dates and History (tasks 6–7). Task 8 runs in each one for its own strings | About 1,000 Russian strings to read. Three PRs give about 450, 450 and 100, each mergeable alone. One PR is possible if you'd rather |

## Files

| File | Task | What changes |
|---|---|---|
| `crates/tabletist-core/src/i18n.rs` | 1, 6 | `Msgid`, `msgid`, `pmsgid`; Tabletist's `ngettext`; `counted` grouped; tests |
| `crates/tabletist-core/src/i18n/numbers.rs` (new) | 6 | Separators, `number`, `group_number`, month and weekday tables |
| `assets/i18n/update.sh`, `update_test.sh` | 1 | Keywords `msgid:1`, `pmsgid:1c,2` |
| `AGENTS.md`, `docs/_reference/translating.md` | 1, 8 | The marker rule; the guide no longer says menu items stay English |
| ~31 files with `u32::try_from(…).unwrap_or(u32::MAX)` | 1 | The count passed as it is |
| `crates/tabletist-core/src/keymap.rs` | 2 | `Info.name`, `Info.row`, `Group::name`, `Scope::name` → `Msgid` |
| `src/menu.rs`, `src/menu/bar.rs`, `src/app/menu.rs`, `src/menu/tests.rs` | 2 | Titles are `Msgid`s |
| `src/app/commands.rs`, `crates/tabletist-core/src/model.rs` (`Runnable`) | 2 | Palette names are `Msgid`s |
| `src/ui/help.rs`, `src/ui/settings/keyboard.rs`, `src/app/keyboard.rs`, `src/ui/pane_hints.rs`, `src/ui/mod.rs` | 2 | Callers of the above |
| `crates/tabletist-core/src/model.rs` (`SettingsPane`), `crates/tabletist-core/src/settings.rs` | 3 | Pane names, notes and hints → `Msgid` |
| `src/ui/settings/{mod,sheet,terminal,agents,file_pane,keyboard,updates,updates_terminal}.rs`, `src/app/quick_open.rs`, `src/app.rs` | 3 | `label`, `terminal_label`, `Said::Word`, `skin.say` take `Msgid` |
| `src/ui/*.rs`, `src/ui/connect_dialog/*.rs` (the 9 `say` methods and ~40 closures) | 5 | Wrappers take `Msgid`; glued sentences become whole phrases |
| `crates/tabletist-core/src/format.rs`, `crates/tabletist-core/src/clock.rs` | 6 | Locale-aware grouping, compact counts, sizes, ranges, days |
| `src/ui/data_view.rs`, `src/ui/date_line.rs`, `src/ui/calendar.rs`, `src/ui/history.rs` | 6, 7 | Callers; durations; the Query label |
| `src/app.rs` (History recording) | 7 | `Query {n}` recorded in English |
| `assets/i18n/tabletist.pot`, `assets/i18n/ru.po` | every task | Extracted; translated |
| `src/ui/language_tests.rs`, `crates/tabletist-core/src/i18n.rs` tests | 1–7 | Russian window tests; catalog completeness |

Every task's checks, the same each time:

```bash
export CARGO_TARGET_DIR=$PWD/target
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
~/.cargo/bin/cargo test --locked --workspace --all-targets
assets/i18n/update.sh --check
```

The server tests need the two URLs while PostgreSQL (55432) and MySQL (53306) are up (`ss -ltn`), as AGENTS.md describes.

---

### Task 1: `Msgid`, the keywords, and a count past `u32`

**Files:**
- Modify: `crates/tabletist-core/src/i18n.rs`
- Modify: `assets/i18n/update.sh:57-61`, `assets/i18n/update_test.sh`
- Modify: `AGENTS.md:102-107`
- Modify: every file with a plural `u32::try_from(…).unwrap_or(u32::MAX)` (`rg -l 'unwrap_or\(u32::MAX\)' src crates`)

- [ ] **Step 1: Write the failing tests** in `i18n.rs`'s `mod tests`:

```rust
#[test]
fn a_marked_message_is_said_in_the_language() {
    const SETTINGS: Msgid = msgid("Settings");
    assert_eq!(SETTINGS.english(), "Settings");
    assert_eq!(SETTINGS.say(Locale::Russian), "Настройки");
    assert_eq!(SETTINGS.say(Locale::English), "Settings");
    // A context picks its own entry; with none it is the English.
    let row = pmsgid("row panel title", "Row");
    assert_eq!(row.english(), "Row");
    assert_eq!(row.say(Locale::English), "Row");
}

#[test]
fn a_count_past_u32_keeps_its_russian_form() {
    let rows = |count: u64| ngettext(Locale::Russian, "{n} row", "{n} rows", count);
    // 4,294,967,301 ends in 1: one; the old cap, 4,294,967,295, said many.
    assert_eq!(rows(4_294_967_301), "{n} строка");
    assert_eq!(rows(4_294_967_302), "{n} строки");
    assert_eq!(ngettext(Locale::Russian, "{n} row", "{n} rows", 21_usize), "{n} строка");
    assert_eq!(ngettext(Locale::English, "{n} row", "{n} rows", 1_u32), "{n} row");
    assert_eq!(ngettext(Locale::English, "{n} row", "{n} rows", 0_i64), "{n} rows");
}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-core i18n::tests`
Expected: compile errors, `msgid`, `pmsgid` and `Msgid` not found, and `ngettext` expecting `u32`.

- [ ] **Step 3: Write the implementation.** In `i18n.rs`, replace the `pub use` line and `counted`:

```rust
pub use fastframe_i18n::{LanguageTag, gettext, pgettext};

use std::borrow::Cow;

/// An English message marked for the catalog where it is written, and
/// said in a language where it is drawn. `update.sh` extracts every
/// `msgid("…")` and `pmsgid("context", "…")`; a function that translates
/// what it is handed takes a `Msgid`, so a bare `&str` cannot reach it
/// unextracted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Msgid {
    context: Option<&'static str>,
    text: &'static str,
}

/// `text`, marked for the catalog.
pub const fn msgid(text: &'static str) -> Msgid {
    Msgid { context: None, text }
}

/// `text` in `context`, marked for the catalog: for a word spelled alike
/// with two meanings (`May` the month and the short month).
pub const fn pmsgid(context: &'static str, text: &'static str) -> Msgid {
    Msgid {
        context: Some(context),
        text,
    }
}

impl Msgid {
    /// The message as the code writes it: the settings file's and the
    /// tests' word, never drawn in another language.
    pub const fn english(self) -> &'static str {
        self.text
    }

    /// The message in `locale`.
    pub fn say(self, locale: Locale) -> Cow<'static, str> {
        match self.context {
            Some(context) => pgettext(locale, context, self.text),
            None => gettext(locale, self.text),
        }
    }
}

/// A count as the catalog's plural rules read it: any integer, kept
/// whole, so a count past `u32::MAX` keeps its form. A negative one
/// (never a count) reads as many.
pub trait Count: Copy {
    fn as_u64(self) -> u64;
}

impl Count for u64 {
    fn as_u64(self) -> u64 {
        self
    }
}

impl Count for u32 {
    fn as_u64(self) -> u64 {
        self.into()
    }
}

impl Count for usize {
    fn as_u64(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
}

impl Count for i64 {
    fn as_u64(self) -> u64 {
        u64::try_from(self).unwrap_or(u64::MAX)
    }
}

/// The whole phrase for `count`, by the catalog's plural rules; English's
/// without a catalog (`one` for 1, `many` otherwise, 0 among them). The
/// catalog is handed the count as a `u64`, not fastframe's `u32`.
pub fn ngettext(
    locale: Locale,
    one: &'static str,
    many: &'static str,
    count: impl Count,
) -> Cow<'static, str> {
    use fastframe_i18n::Locale as _;
    let count = count.as_u64();
    match locale.catalog() {
        Some(catalog) => catalog.ntranslate(count, one, many, None),
        None => Cow::Borrowed(if count == 1 { one } else { many }),
    }
}

/// `count` with its noun: "1 change", "3 changes".
pub fn counted(locale: Locale, count: usize, one: &'static str, many: &'static str) -> String {
    format!("{count} {}", ngettext(locale, one, many, count))
}
```

`counted` gets its grouped number in task 6, not here.

- [ ] **Step 4: Change the plural call sites.** For each file `rg -l 'unwrap_or\(u32::MAX\)' src crates` lists, wherever the `u32` feeds an `ngettext`:
  - drop the `let plural = u32::try_from(x).unwrap_or(u32::MAX);` line and pass `x` itself;
  - in `ddl.rs:250`, drop the `plural` closure the same way;
  - leave any `unwrap_or(u32::MAX)` that does not feed an `ngettext` alone.

  For example, `src/ui/review.rs`, `Line::NewRequired`, becomes:

```rust
Line::NewRequired(columns) => {
    let blocked = ngettext(
        locale,
        "blocked: {columns} is required",
        "blocked: {columns} are required",
        columns.len(),
    );
```

  Then fix the hand-chosen plural at `src/ui/sql_editor.rs:1174` (`gettext(locale, if count == 1 { one } else { many })`) to `ngettext(locale, one, many, count)` with literal arguments. Read the site: its `one`/`many` must become literals at the call, so they extract.

- [ ] **Step 5: Add the keywords.** In `assets/i18n/update.sh`, the xgettext call:

```bash
# `counted(locale, n, "row", "rows")` (crate::i18n) is a plural too, and
# `msgid("…")` / `pmsgid("context", "…")` mark a message translated later,
# where it is drawn.
xgettext --language=Rust --from-code=UTF-8 \
    --keyword= --keyword=gettext:2 --keyword=ngettext:2,3 --keyword=pgettext:2c,3 \
    --keyword=counted:3,4 --keyword=msgid:1 --keyword=pmsgid:1c,2 \
```

  In `update_test.sh`, add a file to its scratch repository holding `const A: Msgid = msgid("Marked");` and `pmsgid("month", "May")`, and assert the template has `msgid "Marked"` and `msgctxt "month"`. Follow the file's existing pattern of one `grep -q` per assertion.

- [ ] **Step 6: AGENTS.md rule.** Replace "A string `gettext` is handed in a variable is not extracted." with:

```markdown
A message drawn later than it is written (a table of names, a
command's description, a word handed to `say`) is marked where it is
written with `i18n::msgid("…")`, or `pmsgid("context", "…")` for a word
with two meanings, and is a `Msgid` until it is drawn with `.say(locale)`.
A function that translates text it is handed takes a `Msgid`, never a
`&str`. Pass a count to `ngettext` as it is (`rows.len()`), never through
`u32`.
```

- [ ] **Step 7: Update the i18n.rs module comment.** It said strings in variables stay English. Rewrite it to say they are marked with `msgid`.

- [ ] **Step 8: Run the tests and the checks.** The first two tests pass. `update.sh --check` fails, because `sql_editor`'s two literals are new. Run `assets/i18n/update.sh`, translate the new entries in `ru.po` (task 8's rules), then run the checks again.

- [ ] **Step 9: Commit**

```bash
git add -A crates/tabletist-core/src/i18n.rs assets/i18n AGENTS.md src crates
git commit -m "Mark messages drawn later with msgid, and pass a plural's count whole"
```

---

### Task 2: The keymap, the menu bar and the palette

**Files:**
- Modify: `crates/tabletist-core/src/keymap.rs:110-140, 794-846, 834-1294, 4265-4285`
- Modify: `src/menu.rs:55-270`, `src/menu/bar.rs:35-70, 130-140, 355-365`, `src/app/menu.rs:14`
- Modify: `src/app/commands.rs:82-460`, `crates/tabletist-core/src/model.rs:~3525` (`Runnable`)
- Modify: `src/ui/help.rs:22-60, 81-82, 182, 217, 223, 329, 384, 426, 584`, `src/ui/settings/keyboard.rs:38-142`, `src/app/keyboard.rs:302`, `src/ui/mod.rs:1600`, `src/ui/pane_hints.rs:99, 141`
- Test: `src/ui/language_tests.rs`, `src/menu/tests.rs`

- [ ] **Step 1: Write the failing test** in `src/ui/language_tests.rs`:

```rust
/// Every menu of the bar and every item but the app's own name and the
/// system's items is in Russian: none reads as its English.
#[test]
fn the_russian_menu_bar_is_in_russian_throughout() {
    use crate::menu::{Item, Platform, bar, title};
    let mut harness = Harness::new();
    harness
        .app
        .change_settings(|settings| settings.language = Language::Fixed(Locale::Russian));
    for platform in [Platform::MacOs, Platform::Windows] {
        let described = crate::menu::bar::describe(platform, &harness.app);
        let titles: Vec<String> = described
            .menus
            .iter()
            .map(|menu| menu.title.trim_start_matches('&').to_owned())
            .collect();
        let wanted: &[&str] = match platform {
            Platform::MacOs => &["Tabletist", "Файл", "Изменить", "Вид", "Запрос", "Окно", "Справка"],
            Platform::Windows => &["Файл", "Изменить", "Вид", "Запрос", "Окно", "Справка"],
        };
        assert_eq!(titles, wanted, "{platform:?}");
        for menu in bar(platform) {
            for item in menu.items {
                if matches!(item, Item::Separator) {
                    continue;
                }
                let english = title(item, false).english();
                let said = title(item, false).say(Locale::Russian);
                // The app's name stays, as every product name does.
                if english.contains("Tabletist") {
                    continue;
                }
                assert_ne!(said, english, "{item:?} is still English");
            }
        }
    }
}
```

  Check the exact Windows menu list in `src/menu.rs::bar` before fixing `wanted`. Keep `Tabletist` exempt, and any item whose English contains it ("About Tabletist" reads «О программе Tabletist»: that is translated, so the exemption only skips an exact product name). If `describe` or `bar` is `pub(super)`, add the test in `src/menu/tests.rs` instead, next to `the_bar_is_built_again_in_a_language_chosen_in_settings`.

  In the same file, add a test that the Keyboard settings and the help overlay say a command in Russian. `Command::Reload` is «Обновить схему» in both, through `harness.has(...)` on the open sheet and overlay. Follow `help_tests.rs`'s `grouped` helper to open the overlay.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib language_tests::the_russian_menu_bar`
Expected: a compile error (`english` on `&str`), then, once task 2 compiles, failures naming English items.

- [ ] **Step 3: Make the keymap's words `Msgid`s.** In `keymap.rs`:

```rust
use crate::i18n::{Msgid, msgid};

pub struct Info {
    /// The command's own name.
    pub name: Msgid,
    /// The row of the settled keymap it belongs to, by the row's name. …
    pub row: Msgid,
    pub group: Group,
    pub built: bool,
}

const fn info(name: Msgid, row: Msgid, group: Group) -> Info { … }
```

  Each arm becomes, for example, `C::Connections => info(msgid("Connections"), msgid("Connections"), Window)`, and the shared rows `const PANES: Msgid = msgid("Move between panes");`. The empty row (`""`, "a key the app had before") becomes `msgid("")`. Never draw an empty one: check with `row.english().is_empty()`. xgettext skips the empty msgid, since it is the header. `Group::name` and `Scope::name` return `Msgid` with each literal wrapped. Where the Omarchy and Mac words differ only in case ("window" / "Window and connections"), keep both as written: they are different messages.

  Every comparison and lookup that uses these as identifiers uses `.english()`: `src/app/keyboard.rs:302`, `src/ui/mod.rs:1600`, `keymap.rs` tests (4950, 4972, 5660), `keymap/overrides.rs` where a row is matched, and settings.toml, whose keys stay English. Let the compiler list them.

- [ ] **Step 4: The menu bar.** In `src/menu.rs`, `Menu.title: Msgid`, and `title(item, on) -> Msgid`, with every literal wrapped. `"Tabletist"` becomes `msgid("Tabletist")`; the Russian catalog gives it back unchanged. In `bar.rs`:
  - `describe`'s `text` becomes `|msgid: Msgid| msgid.say(app.locale).into_owned()`;
  - `ItemState.title` and `Change::Title` hold a `Msgid`;
  - line 360 is `self.menu.set_title(id, &msgid.say(app.locale))`;
  - `src/app/menu.rs:14` does the same.

  A native item's title (`Native::Minimize`…) goes through `title` too, and is translated the same way.

- [ ] **Step 5: The palette.** In `src/app/commands.rs`, each `run_of` / `window_run` tuple's name becomes `msgid("Reload schema")`. `model::Runnable.name: Msgid`, and `Runnable::named` says `self.name.say(locale)`. The quick-open search matches the drawn (translated) name. Read `quick_open.rs:925` and `1228`: when it matches against `name`, it uses the said text, so a Russian user finds «Обновить» by typing Russian. Add a test: in Russian, typing «схем» in the palette lists «Обновить схему».

- [ ] **Step 6: Callers.**
  - `help.rs`: `GROUPS` and `heading()` become `Msgid`s, and each `gettext(locale, command.info().name)` becomes `command.info().name.say(locale)`.
  - `settings/keyboard.rs:38-142`: `why.text(layout)` returns `Msgid`; its literals live in `keymap/overrides.rs` or `keymap.rs`, so find them there.
  - `pane_hints.rs:99, 141`: the `word` closures take `Msgid`, and each call wraps its literal: `word(msgid("move"))`.

- [ ] **Step 7: Extract and translate.** Run `assets/i18n/update.sh`. Translate every new entry in `ru.po`. The `Info.row` names are often the same text as `name`, so that entry is shared. Menu items follow macOS Russian usage: «Обновить схему», «Сочетания клавиш», «Свернуть», «Масштаб», «Все окна на передний план». See task 8 for the glossary.

- [ ] **Step 8: Run the tests and the checks.** The new tests pass, and so do `src/menu/tests.rs` and `help_tests.rs`. Any test that compared `info().name` to a `&str` compares `.english()`.

- [ ] **Step 9: Commit**

```bash
git add -A src crates assets/i18n
git commit -m "Say the keymap's commands and the menu bar in the window's language"
```

---

### Task 3: Settings: the nav, labels, notes and the Omarchy words

**Files:**
- Modify: `crates/tabletist-core/src/model.rs:3797` (`SettingsPane::name`)
- Modify: `crates/tabletist-core/src/settings.rs` (`theme_note`, the `name()` helpers that are drawn)
- Modify: `src/ui/settings/mod.rs:37, 88-110, 376-454, 615`
- Modify: `src/ui/settings/sheet.rs:127, 373, 724, 2091`, `src/ui/settings/terminal.rs:133-148, 316-525, 2042, 2152`
- Modify: `src/ui/settings/{agents,file_pane,keyboard,updates,updates_terminal}.rs`, `src/app/quick_open.rs:190-213`, `src/app.rs:8764, 8783, 9165`, `src/app/updates.rs:842`
- Test: `src/ui/language_tests.rs`

- [ ] **Step 1: Write the failing test**

```rust
/// The nav names every pane in Russian, in both looks, and the Omarchy
/// screen's own words are Russian too.
#[test]
fn the_russian_settings_name_every_pane() {
    for look in [Look::macos(), Look::omarchy()] {
        let mut harness = general(look, Some("ru-RU"));
        for pane in ["Основные", "Оформление", "Редактор", "Клавиатура", "Агенты", "Обновления"] {
            // The Omarchy look lower-cases what it draws.
            let pane = if look.terminal { pane.to_lowercase() } else { pane.to_owned() };
            assert!(harness.has(&pane) || painted(&harness, &pane), "{look:?}: {pane}");
        }
    }
}
```

  Before fixing the list, read the canvas's `MacTranslated` / `OmarchyTranslated` artboards (memory: design canvas Artifact) for its words for the panes. Use its words where it has them. The canvas has «Основные». The list above is the draft for the rest.

  Also check one row label per pane in Russian, by its label on the sheet: General's "On launch" is «При запуске».

  Replace the comment at `language_tests.rs:117-118` ("the panes' names are handed to `gettext` in a variable") with an assertion that «Основные» is drawn after choosing Русский.

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked --lib language_tests::the_russian_settings`
Expected: FAIL, `Основные` not found (the nav says "General").

- [ ] **Step 3: Make the words `Msgid`s.**
  - `SettingsPane::name(self) -> Msgid`.
  - `settings::label(OptionId) -> Msgid` and `terminal_label`.
  - `Said::Word(Msgid)`.
  - The skin's `say(&self, text: Msgid) -> String` (`terminal.rs:133`, `sheet.rs:127`) is `self.look.label(&text.say(self.locale))`.
  - Each `skin.say("…")` becomes `skin.say(msgid("…"))`. `rg -n 'say\("' src/ui/settings` lists them; the compiler confirms.
  - The notes and hints the survey found as variables become `Msgid` consts: `NOTE_TEXT`, `NO_WINDOW`, `agent_name`/`agent_hint`, `pin_missing`, `check_text`, `failure.said()`, `theme_hint`, `reset_question`, `sample_timestamp`, and `theme_note`.

  A note that glues words ("{run}", "{format}" filled by `said`) keeps its placeholders: it is already a whole phrase.

- [ ] **Step 4: Extract, translate, and run the tests and checks.** The same as task 2's steps 7 and 8.

- [ ] **Step 5: Commit**

```bash
git add -A src crates assets/i18n
git commit -m "Say Settings' panes, labels and notes in the window's language"
```

---

### Task 4: The PR for 2a (if three PRs)

- [ ] Run the checks. Write the PR body by task 8's recipe, open it against `main`, then go on to task 5 on a new branch from it (`claude/translations-2b-russian`). Rebase that branch onto main once 2a merges.

---

### Task 5: The views' `say` words and the whole phrases

**Files:**
- Modify: the 9 `say` methods (`connect_dialog/mod.rs:145`, `export.rs:40`, `history.rs:170`, `quick_open.rs:130`, `review.rs:485`, `start.rs:215`, `write_prompts.rs:70`, `sql_results.rs:59`, and those already done in task 3)
- Modify: the local closures the survey listed (`agent_access.rs:92`, `agents.rs:1644`, `cell_editor.rs:1301-1534`, `conflict_prompt.rs:138`, `copy_as.rs:294`, `data_view.rs:1151, 2604, 2723, 3003`, `help.rs:182`, `label_chooser.rs:163, 432`, `pending_bar.rs:186`, `picker.rs:1051`, `review.rs:61`, `row_panel.rs:173, 311, 2251, 2282, 2478, 2718, 3030`, `sidebar.rs:1389`, `sql_editor.rs:562`, `workspace.rs:537, 691, 944, 2131, 3872`), and the consts (`COPIED`, `COPIED_SQL`, `COPIED_COMMAND`, `GONE`, `TYPING`, `WRITING`, `SAVING`, `CANCELLED`, `INTERCEPT_WARNING`)
- Modify: `src/ui/workspace.rs:3140-3214` (`.map(|word| gettext(locale, word))` over literal arrays), `ddl.rs`, `structure.rs`, `json_view.rs`, `key_picker.rs`, `filter_bar.rs` (`op_label`), `connect_dialog/{agents,reading,sheet,terminal}.rs` (`kind.label()`, `ssh_auth.label()`, `closed_summary`, `skin.keyring()`)
- Test: `src/ui/language_tests.rs`, `src/ui/review.rs`'s tests

- [ ] **Step 1: Write the failing tests.** The review's comments in Russian, built from `review::comment`:

```rust
#[test]
fn the_review_s_comments_are_whole_russian_phrases() {
    use crate::ui::review::comment;
    let blocked = Line::Blocked { row: "id 2".into(), columns: vec!["kind".into()] };
    assert_eq!(
        comment(&blocked, Locale::English).as_deref(),
        Some("-- row id 2 · blocked: fix kind first")
    );
    assert_eq!(
        comment(&blocked, Locale::Russian).as_deref(),
        Some("-- строка id 2 · заблокирована: сначала исправьте kind")
    );
    let run = Line::DeleteRun(3);
    assert_eq!(comment(&run, Locale::English).as_deref(), Some("-- delete 3 rows"));
    assert_eq!(comment(&run, Locale::Russian).as_deref(), Some("-- удалить 3 строки"));
}
```

  Use `Line`'s real field types from `review.rs`. Igor's check of `ru.po` decides the Russian; the test follows the catalog. In the same task, add one Harness test per look for the cell editor's limit line in Russian: `{type} holds {min} to {max}`, with an INTEGER cell open, «INTEGER: от {min} до {max}». Follow `cell_editor`'s existing tests for opening a cell.

- [ ] **Step 2: Run them to see them fail.** Expected: the Russian is English pieces, and `DeleteRun` says "rows" for 3 with no plural form.

- [ ] **Step 3: Change each wrapper and closure to take `Msgid`.** Wrap each literal at its call. Where a closure assembles a sentence from several `say`s, the sentence becomes one message with placeholders, filled with `i18n::fill`. Its count goes through `ngettext`. `review.rs` becomes, for example:

```rust
pub fn comment(line: &Line, locale: Locale) -> Option<String> {
    let say = |text: Msgid| text.say(locale);
    Some(match line {
        Line::Row(row) => format!("-- {}", fill(&gettext(locale, "row {row}"), &[("{row}", row)])),
        Line::Blocked { row, columns } => format!(
            "-- {}",
            fill(
                &gettext(locale, "row {row} · blocked: fix {columns} first"),
                &[("{row}", row), ("{columns}", &columns.join(", "))],
            )
        ),
        Line::DeleteRun(rows) => format!(
            "-- {}",
            fill(
                &ngettext(locale, "delete {n} row", "delete {n} rows", *rows),
                &[("{n}", &rows.to_string())],
            )
        ),
        // …each arm one message the same way…
    })
}
```

  `rows.to_string()` becomes `numbers::number(locale, rows)` in task 6. If 2c lands after 2b, task 6 switches it.

  A list joined by ", " or " and " keeps its join as a message (`" and "` → `pgettext("list join", " and ")`), since the parts are names. Lone words, such as a column header or a button, keep a lone `msgid`. When xgettext warns that a word is used alone and as a plural (`row` / `{n} row`), give the lone use a context: `pmsgid("label", "row")`, as run 1 did.

  In `cell_editor.rs`:
  - the local `counted` closure (1411) and `count` (1301) go; their phrases call `ngettext` with literals (`"{n} char"`, `"{n} chars"`; `"{n} line"`, `"{n} lines"`; `"{n} decimal"`; `"{n} digit"`; `"{n} character"`), so the keyword reads the right arguments;
  - the `"{} / {max}"` counters (530, 863) stay as they are: two numbers and a slash, no words;
  - `"{type_name} {} {min} {} {max}"` (1424), `"{} 1{} {}."` (1441) and `"{table}{}" … "'s key cannot be matched…"` (1510) become whole phrases.

  `date_line.rs`'s `"24:00:00 · {}"` (234) and `"{} {name}"` (381) become whole phrases. Its month and weekday tables are task 6's.

- [ ] **Step 4: Sweep for what is left.** `rg -n 'gettext\(\s*(self\.)?locale,\s*[a-z_]' src crates` lists every `gettext` whose msgid is not a literal. Each remaining site is:
  - a `Msgid`'s `say`;
  - inside `i18n.rs`;
  - `about.rs:93`'s package description (Cargo's English, which stays: say so in a comment).

  Note any other site in the PR as staying English, with its reason.

- [ ] **Step 5: Extract, translate, and run the tests and checks.** The same as task 2's steps 7 and 8. Expect a few xgettext "used alone and as a plural" failures from `update.sh`: give each lone use a context.

- [ ] **Step 6: Commit**, in two if the diff is very large: wrappers and lone words first, whole phrases second.

```bash
git add -A src crates assets/i18n
git commit -m "Say the views' words in the window's language, each sentence whole"
```

---

### Task 6: Numbers and dates in the language

**Files:**
- Create: `crates/tabletist-core/src/i18n/numbers.rs` (and `mod numbers;` plus `pub use numbers::{…}` in `i18n.rs`, with `i18n.rs` moved to `i18n/mod.rs` only if the crate's layout needs it: a sibling `i18n/numbers.rs` next to `i18n.rs` works as is)
- Modify: `crates/tabletist-core/src/format.rs:790-880` (`file_size`, `compact_count`, `range_label`, `group_digits`, `group_number`)
- Modify: `crates/tabletist-core/src/clock.rs` (`day`, `moment`, `day_of`'s `Day::On`)
- Modify: `crates/tabletist-core/src/i18n.rs` (`counted`)
- Modify: `src/ui/data_view.rs:2930-2962`, `src/ui/date_line.rs:74-140`, `src/ui/calendar.rs` (month heading), `src/ui/history.rs:1460-1495`, and the callers of `group_digits` / `compact_count` / `range_label` / `file_size` (`rg -l 'group_digits|compact_count|range_label|file_size' src crates`)
- Test: `numbers.rs`'s own tests, `src/ui/language_tests.rs`

- [ ] **Step 1: Write the failing unit tests** in `numbers.rs`:

```rust
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
    }

    #[test]
    fn a_grid_number_takes_its_language_s_marks() {
        assert_eq!(group_number(English, "-1240.50"), "-1,240.50");
        assert_eq!(group_number(Russian, "-1240.50"), "-1\u{a0}240,50");
        // A short one keeps its point as the database wrote it, grouped
        // or not: only a grouped number is redrawn.
        assert_eq!(group_number(Russian, "12.5"), "12,5");
        assert_eq!(group_number(Russian, "1e9"), "1e9");
        assert_eq!(group_number(Russian, "NaN"), "NaN");
    }

    #[test]
    fn a_short_count_says_its_unit() {
        assert_eq!(compact_count(English, 1_250), "1.3K");
        assert_eq!(compact_count(Russian, 1_250), "1,3 тыс.");
        assert_eq!(compact_count(Russian, 3_400_000), "3,4 млн");
        assert_eq!(compact_count(Russian, 999), "999");
    }

    #[test]
    fn a_day_is_written_in_its_language() {
        assert_eq!(short_day(English, 10, 13), "Oct 13");
        assert_eq!(short_day(Russian, 10, 13), "13 окт.");
        assert_eq!(dated_day(English, 2025, 10, 13), "Oct 13, 2025");
        assert_eq!(dated_day(Russian, 2025, 10, 13), "13 окт. 2025 г.");
        assert_eq!(month_name(Russian, 1).as_ref(), "Январь");
    }
}
```

  Decide `group_number(Russian, "12.5")` with Igor's answer to the separators question. As drafted, a value with nothing to group still shows a comma in Russian when grouping is on, so a column reads alike. Today `group_number` returns a short number untouched; English keeps that.

- [ ] **Step 2: Run them to see them fail.** Expected: `numbers` not found.

- [ ] **Step 3: Write `numbers.rs`.**

```rust
//! Numbers and dates as a language writes them: its group and decimal
//! marks, its month names and its day's order. Values in the grid that are
//! not grouped, and every date in the grid, stay as the database wrote
//! them (ISO).

use std::borrow::Cow;

use super::{Count, Locale, Msgid, fill, gettext, pmsgid};

/// The mark between groups of three digits, and the decimal mark.
pub fn marks(locale: Locale) -> (char, char) {
    match locale {
        Locale::English => (',', '.'),
        // CLDR's: a no-break space, so a number is never split by a wrap.
        Locale::Russian => ('\u{a0}', ','),
    }
}

/// A count as the interface writes it: `12,480`, «12 480».
pub fn number(locale: Locale, count: impl Count) -> String {
    grouped(&count.as_u64().to_string(), marks(locale).0)
}

/// A run of digits with `mark` between every three, from the end.
pub(crate) fn grouped(digits: &str, mark: char) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3 * mark.len_utf8());
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(mark);
        }
        out.push(digit);
    }
    out
}
```

  Move `format.rs`'s `group_number` body here as `pub fn group_number(locale: Locale, text: &str) -> Cow<'_, str>`, with the language's group mark, and its decimal mark in place of `.`. For English a number with nothing to group comes back borrowed, as today. For Russian, a decimal one is redrawn with its comma. `format::group_digits(n)` and `format::group_number(text)` become thin wrappers that take a `Locale` (or are replaced at their callers, which the compiler lists).

  `compact_count(locale, n)`: the value rounds as today. Its decimal mark is the language's, and its unit is a whole phrase: `ngettext`-free, since the number is a decimal:

```rust
// Translators: a count shortened to thousands: "1.2K".
let thousands = gettext(locale, "{n}K");
// Translators: a count shortened to millions: "3.4M".
let millions = gettext(locale, "{n}M");
// Translators: a count shortened to billions: "7B".
let billions = gettext(locale, "{n}B");
```

  Russian: `{n} тыс.`, `{n} млн`, `{n} млрд`. `range_label` becomes the whole phrases `"{first}–{last} of {total}"` and `"{first}–{last} of ~{total}"` (and `"{first}–{last}"` with no total), with `number` for each count. `file_size`'s `B`, `KB`, `MB` become messages `"{n} B"`, `"{n} KB"`, `"{n} MB"`: «{n} Б», «{n} КБ», «{n} МБ», with the decimal mark.

  Dates:

```rust
const SHORT_MONTHS: [Msgid; 12] = [
    pmsgid("short month", "Jan"), pmsgid("short month", "Feb"), pmsgid("short month", "Mar"),
    pmsgid("short month", "Apr"), pmsgid("short month", "May"), pmsgid("short month", "Jun"),
    pmsgid("short month", "Jul"), pmsgid("short month", "Aug"), pmsgid("short month", "Sep"),
    pmsgid("short month", "Oct"), pmsgid("short month", "Nov"), pmsgid("short month", "Dec"),
];
const MONTHS: [Msgid; 12] = [ /* pmsgid("month", "January") … "December" */ ];
const WEEKDAYS: [Msgid; 7] = [ /* pmsgid("short weekday", "Mon") … "Sun" */ ];

/// A short month by its number, 1 to 12: "Oct", «окт.» (as a date says
/// it: Russian's genitive).
pub fn short_month(locale: Locale, month: i8) -> Cow<'static, str> { … }
/// A month's name alone, as a calendar's heading: "October", «Октябрь».
pub fn month_name(locale: Locale, month: i8) -> Cow<'static, str> { … }
/// A short weekday, Monday first: "Thu", «чт».
pub fn weekday(locale: Locale, monday_zero: usize) -> Cow<'static, str> { … }

/// "Oct 13", «13 окт.».
pub fn short_day(locale: Locale, month: i8, day: i8) -> String {
    // Translators: a day of this year, as History's heading.
    fill(&gettext(locale, "{month} {day}"), &[("{month}", &short_month(locale, month)), ("{day}", &day.to_string())])
}
/// "Oct 13, 2025", «13 окт. 2025 г.».
pub fn dated_day(locale: Locale, year: i16, month: i8, day: i8) -> String {
    // Translators: a day of another year.
    fill(&gettext(locale, "{month} {day}, {year}"), &[…])
}
```

  Write out all 12 and 7 entries in full in the code. The comments above only shorten the plan.

  `clock.rs`: `day_in`'s `"%b %-d"` / `"%b %-d, %Y"` become the zoned date's parts handed to `short_day` / `dated_day`. `Day::On` holds the date's parts (year, month, day), not a string, so History says it at draw time in the window's language. `clock::moment` and `clock::day` take a `Locale`. `time` and `hour` stay `%H:%M:%S` / `%H:%M`.

  `date_line.rs`: `WEEKDAYS`, `MONTHS` and `FULL_MONTHS` go, in favour of `numbers::{weekday, short_month, month_name}`. `day_words` is the whole phrase `"{weekday} {day} {month} {year}"` («{weekday} {day} {month} {year}», Igor to say if «г.» follows); `short_day` there is `"{day} {month}"`. `calendar.rs`'s month heading uses `month_name` and `"{month} {year}"`.

  `i18n::counted` writes `number(locale, count)`. Every `fill(…, &[("{n}", &x.to_string())])` of a count becomes `&number(locale, x)`. List them with `rg -n '"\{n\}", &[a-z_.()]*to_string' src crates` and do each. A count that is an identifier (a query's number, "Query 7"; a line number; a port) stays `to_string()`, never grouped.

  `data_view.rs:2940, 2958`: `format::group_number(text)` becomes `numbers::group_number(app.locale, text)`. Read how `locale` reaches `data_view`'s cell drawing (a `Locale` argument or the app). Copy and export read the value, not the drawn text. Confirm this with a test: copy a grouped Russian cell, and the clipboard holds `1240.50`.

  `history.rs:1484 duration_words`: `"{n} ms"`, `"{n} s"`, `"{n} min"`, `"{n} min {s} s"` as messages: «{n} мс», «{n} с», «{n} мин», «{n} мин {s} с».

- [ ] **Step 4: Write the failing Harness test** in `language_tests.rs`:

```rust
/// In Russian, counts and grouped grid numbers take Russian's marks, and
/// the grid's dates stay ISO.
#[test]
fn russian_numbers_take_its_marks_and_grid_dates_stay_iso() {
    let mut page = crate::testing::book_covers_page(3);
    page.rows[0][1] = tabletist_db::Value::Text("1240.50".into());
    let mut harness = Harness::with_size(egui::vec2(1400.0, 1200.0));
    harness.app.settings.group_digits = Some(true);
    harness
        .app
        .change_settings(|settings| settings.language = Language::Fixed(Locale::Russian));
    // As export_ways_tests' bookshop_with: book_covers open on `page`.
    let tab = open_book_covers(&mut harness, page);
    assert!(painted(&harness, "1\u{a0}240,50"), "{:?}", harness.painted);
    assert!(painted(&harness, "9\u{a0}100\u{a0}000\u{a0}000\u{a0}000\u{a0}000\u{a0}002"));
    assert!(painted(&harness, "2026-01-12 09:14:03"), "grid dates stay ISO");
    // The status line's count, Russian's plural and marks.
    let _ = tab;
}
```

  `open_book_covers` is a helper in `language_tests.rs`, copied from `export_ways_tests.rs::bookshop_with`: connect `fixture_connection`, open `main.book_covers`, answer the structure and `page`, and return the `ConnTabId`. Set `rows[0][1]` to a `Value` the grid draws as numeric text. If `Value::Text` in a `Numeric` column is not how a decimal reaches the grid, use the variant `data_view.rs:2955`'s `other` arm reads (for example `Value::Float` or `Value::Decimal`); read `tabletist_db::Value` first. Add an assertion for one interface count in Russian that the grid's status line or the export toast draws ("Exported 31,400 rows" → «Экспортировано 31 400 строк», in `export_ways_tests` style), and one for History's day heading («13 окт.» upper-cased on the classic look: «13 ОКТ.»).

- [ ] **Step 5: Run everything.** English tests that drew an ungrouped count now see it grouped ("12480 rows" → "12,480 rows"). Fix each expected string; do not change the code back. If one count turns out to be an identifier, make it `to_string()` and say why in a comment.

- [ ] **Step 6: Extract, translate, and run the checks.** The same as task 2's steps 7 and 8.

- [ ] **Step 7: Commit**

```bash
git add -A src crates assets/i18n
git commit -m "Write numbers and interface dates as the window's language does"
```

---

### Task 7: History's "Query N" in the window's language

**Files:**
- Modify: `src/app.rs:5258`
- Modify: `src/ui/history.rs:163` (`detail_of`) and its callers (789, 955, `source_words` 1324)
- Test: `src/ui/language_tests.rs` (or `src/ui/history_tests.rs` if History has its own)

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_query_s_number_is_said_in_the_window_s_language() {
    use crate::ui::history::detail_words;
    assert_eq!(detail_words("Query 7", Locale::Russian), "Запрос 7");
    // A run 1 entry recorded in Russian reads back in English too.
    assert_eq!(detail_words("Запрос 7", Locale::English), "Query 7");
    // Anything else is what was recorded: a table's name, a file.
    assert_eq!(detail_words("Query 7 draft", Locale::Russian), "Query 7 draft");
    assert_eq!(detail_words("book_covers", Locale::Russian), "book_covers");
}
```

  Add a Harness test that runs a query in the SQL editor in Russian and reads the stored `history::Entry.detail`: it is `"Query 1"`. Follow History's existing tests for a run that records. Then, with the window in Russian, History draws «Запрос 1».

- [ ] **Step 2: Run them to see them fail.**

- [ ] **Step 3: Implement.** `src/app.rs:5258` records English:

```rust
detail: fill("Query {n}", &[("{n}", &sql.number.to_string())]),
```

  `history.rs`:

```rust
/// An entry's detail as the window says it: a query's own label
/// ("Query 7"), recorded in any language the app ships, in `locale`; any
/// other detail as it was recorded.
pub fn detail_words(detail: &str, locale: Locale) -> Cow<'_, str> {
    let number = Locale::ALL.into_iter().find_map(|recorded| {
        let label = gettext(recorded, "Query {n}");
        let (before, after) = around(&label, "{n}");
        let digits = detail.strip_prefix(before)?.strip_suffix(after)?;
        (!digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())).then_some(digits)
    });
    match number {
        Some(n) => Cow::Owned(fill(&gettext(locale, "Query {n}"), &[("{n}", n)])),
        None => Cow::Borrowed(detail),
    }
}
```

  `detail_of` returns `detail_words(&entry.detail, locale)`, and its callers take the locale they already have.

- [ ] **Step 4: Run the tests and the checks. Commit.**

```bash
git add -A src assets/i18n
git commit -m "Record a query's History label in English and say it as History draws"
```

---

### Task 8: The catalog, the guide, and the PR

**Files:**
- Modify: `assets/i18n/ru.po`, `crates/tabletist-core/src/i18n.rs` tests
- Modify: `docs/_reference/translating.md`

- [ ] **Step 1: Write the failing completeness tests** in `i18n.rs`, using `entries()`:

```rust
#[test]
fn every_russian_entry_is_translated_without_an_em_dash() {
    let text = include_str!("../../../assets/i18n/ru.po");
    for (ids, strs) in entries(text) {
        let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
            continue;
        };
        assert!(
            !strs.is_empty() && strs.iter().all(|text| !text.is_empty()),
            "{id:?} is not translated"
        );
        for translated in &strs {
            assert!(!translated.contains('—'), "{id:?} → {translated:?}");
        }
    }
}
```

  `entries()` keeps a `#, fuzzy` entry as translated. `update.sh` runs msgmerge with `--no-fuzzy-matching`, so none appear. Make the test also fail on a `#, fuzzy` line in the text.

  Raise `every_russian_translation_keeps_its_placeholders`'s floor (`entries.len() > 600`) to the new count less a margin.

- [ ] **Step 2: Translate.** For each task, list the new entries with `msgattrib --untranslated assets/i18n/ru.po`, and translate each with its `#:` file open, so its meaning is read in the code. The rules are run 1's (`docs/_reference/translating.md`), plus:
  - No em dash. Placeholders as written. Leading and trailing spaces kept.
  - Keys, `:` commands, SQL, names, types, product and environment names stay English.
  - Commands and menu items are verbs in the infinitive, as macOS and Windows Russian menus write them: «Обновить схему», «Закрыть вкладку».
  - Omarchy's lower-case words are written in ordinary case: the look lower-cases them.
  - Reuse run 1's words. Run 1's draft and Igor's review fixed «Сбросить» (Discard), «Чтение» (Read), «подкл.», «Стр.», «стлб». `msgcat` and `grep` in `ru.po` find a word's existing translation.
  - Add a `// Translators:` comment in the code for every short word whose meaning the file does not make plain (`row` the noun, `Query` the menu).

- [ ] **Step 3: The guide.** `docs/_reference/translating.md`: drop "A few strings are not in the catalog yet, such as the window's title and some menu items…". Under "What to keep as it is", add "the window's title" if it still is. Add to "Plurals" that a number in a message is written with the language's own marks: «12 480».

- [ ] **Step 4: Run the full checks**, all of the "Every task's checks" list.

- [ ] **Step 5: The PR body's string list.** List the new Russian strings against main:

```bash
scratch=/tmp/claude-1000/…/scratchpad   # this session's scratchpad
git show origin/main:assets/i18n/ru.po > "$scratch/main.po"
# Entries in this branch's catalog and not in main's.
msgcomm --unique --no-wrap --no-location assets/i18n/ru.po "$scratch/main.po" \
    | msgcomm --more-than=1 --no-wrap --no-location - assets/i18n/ru.po \
    > "$scratch/new.po"
msgattrib --no-wrap --no-location "$scratch/new.po" | grep -c '^msgid '
```

  Turn `new.po` into a Markdown table (`| English | Русский | where |`), grouped by file, with a short awk over the `msgid` / `msgstr` / `#:` lines. Run it again with locations to get the `where` column. Put the entries the draft was least sure of at the top of the body. If the list is longer than GitHub's 65,536-character body, post the rest as PR comments, one per area (menus and keymap; Settings; views; numbers and dates).

- [ ] **Step 6: Commit and open the PR**

```bash
git add -A assets/i18n crates/tabletist-core/src/i18n.rs docs/_reference/translating.md
git commit -m "Translate run 2's messages into Russian"
git push -u origin claude/translations-2-russian
gh pr create --base main --title "Translations, run 2: menus, Settings, the views' words, numbers and dates in Russian" --body-file "$scratch/body.md"
```

  The body has:
  - what changed, by task;
  - what still stays English, and why;
  - the decisions table above, with Igor's answers;
  - "Refs #344";
  - the string list.

  Then bind it with `ccd_pr`.

## Self-review

- **Scope coverage:**
  - The keymap and the menus: task 2.
  - `say` words: tasks 3 and 5.
  - The Settings nav: task 3.
  - Local `say` closures and the review's comments: task 5.
  - `cell_editor`, `sql_complete` and the date line: tasks 5 and 6. `sql_complete`'s letters stay, as decided above.
  - The marker in `update.sh` and AGENTS.md: task 1.
  - Numbers and dates: task 6.
  - Grid dates stay ISO: task 6's test.
  - The `u32::MAX` cap: task 1.
  - History's "Query N": task 7.
  - `update.sh --check`, a complete `ru.po`, and Harness tests for the menu bar, the Settings nav and the numbers: tasks 2, 3, 6 and 8.
  - fmt, clippy and tests: every task.
  - The PR's string list: task 8.
- **Types:**
  - `Msgid`, `msgid`, `pmsgid` and `Msgid::{english, say}`: task 1, used in 2–7.
  - `Count` and `ngettext(…, impl Count)`: task 1, used in 6.
  - `numbers::{marks, number, group_number, compact_count, short_month, month_name, weekday, short_day, dated_day}`: task 6.
  - `history::detail_words`: task 7.
