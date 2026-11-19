# Translations, run 1: the Language setting and Russian

Issue igor-alexandrov/tabletist#344. Every user-facing string already goes
through `gettext` (`crates/tabletist-core/src/i18n.rs`), but `Locale` has one
variant, English, and `assets/i18n` holds no catalog. This run adds a
language setting, Russian as the first catalog, the tooling a translator
needs, and the contributor guide.

The design is the canvas's Settings page, "Language (#344)":
`MacSettingsLanguage`, `OmarchySettingsLanguage`, `MacSettingsGeneral`,
`OmarchySettingsGeneral`, `MacTranslated`, `OmarchyTranslated` and
`LanguageNotes` (version 196).

## Runs

1. **This run:**
   - the Language setting (Settings › General on every look, and
     `[interface] language` in settings.toml)
   - reading the system's language
   - a live change, including the macOS and Windows menu bar
   - the catalog tooling, with a CI check
   - whole phrases for the strings the code builds from pieces today
     (about 85, such as a count, "rows", "in" and a table's name), so each
     is one message with named placeholders and its plural through
     `ngettext`; the English drawn stays the same
   - the Russian catalog for every string `xgettext` finds
   - the contributor guide
2. **Run 2:**
   - the strings handed to `gettext` through a variable, which no
     extractor sees:
     - the keymap's command descriptions, so the menu bar's items too
       (in run 1 only File, Edit and View have Russian titles)
     - the settings panes' names and labels (the nav's «Основные»)
     - `skin.say` words and local `say` closures: among them the review's
       SQL comments (`-- row id 2 · blocked: fix … first`)
     - the English written directly in `cell_editor`, `sql_complete` and
       `date_line`
   - their Russian
   - interface numbers and dates in the language's format (12 480,
     17 нояб. 2026 г.)
   - grouped grid numbers with the language's separators
3. **Later runs:** Português (Brasil), then 简体中文 (zh-Hans, the system's
   CJK font, nothing bundled).

After this run a Russian user sees every string `xgettext` finds today in
Russian, and the rest in English until run 2.

## Decided with the user (2026-11-17)

- **First languages:** Russian, then Brazilian Portuguese and Simplified
  Chinese. Only the app is translated; the website, the contributor guide
  included, stays English.
- **System's first language only:** "System" follows the system's first
  language and nothing after it. With a catalog for it, that language;
  without one, English, never the next language in the list (French then
  Russian gives English).
- **No catalog means the value reads "English":** when the first language
  has no catalog, the value reads just "English" (`english` on Omarchy),
  not "System (English)".
- **Stays English:**
  - keys
  - `:` commands
  - SQL and its keywords
  - names and data
  - type names
  - settings.toml
  - server errors
  - environment names (dev, production, DEV, PROD)

  What a key does is translated; the key itself is not.
- **Omarchy case:** lower case is fine for Russian (the look lower-cases
  every label as today).
- **Omarchy keys:** h/l stays on the Omarchy Settings screen.
- **Pieced-together strings:** rewritten as whole phrases in this run,
  so the first Russian build reads correctly wherever it is translated.
- **Grid values:** grid dates are always ISO. Grouped grid numbers follow
  the language (in run 2).

## Departures and readings, approved

The user said yes to the first three on 2026-11-17; the fourth is the
user's check of the catalog.

| What | The canvas or the reading | This run builds | Why |
|---|---|---|---|
| The menu when the system language has no catalog | My reading, drawn on the canvas: no "System" entry, ✓ on English, and the stored value stays `system` | That reading | Once a catalog for the person's language ships, the app follows it without a visit to Settings |
| Omarchy's list | Enter opens a filterable list of languages | h/l steps through the values; Enter does nothing on this row | Settings has no list picker on Omarchy today, and this run has three values (System, English, Русский). The list comes with the later languages |
| General's "Reset all" | Not drawn | Leaves the language as it is, as Appearance's reset leaves the look | A reset that changes the whole window's language under the person is the same surprise the look's exception avoids. R on the row (Omarchy) and the menu's System entry put it back to System. Where the system's language has no catalog the menu has no System entry: English is what System gives then, and only the file (or R on Omarchy) writes `system` back |
| The Russian wording | `MacTranslated`, `OmarchyTranslated` | `assets/i18n/ru.po`, drafted and listed for the user's check | The user is a native speaker |

## The language

`crates/tabletist-core/src/i18n.rs`:

- `Locale { English, Russian }` with `ALL`, `tag()` (`en`, `ru`) and
  `native_name()` (`English`, `Русский`). Its English name, for the menu's
  grey line, is a msgid in the settings UI (`English`, `Russian`).
- `Locale::from_tag(&LanguageTag)`: `en` → English, `ru` → Russian,
  anything else `None`.
- `Locale::catalog()`: `Russian` → `Some(&ru::Translator)`.
- `pub fn first_system_language() -> Option<LanguageTag>`: the first of
  `fastframe_i18n::system_languages()`, and only that one. `None` when it
  does not parse or is the C locale (`C`, `POSIX`, `C.UTF-8`), which names
  no language (`language_of`). `fastframe_i18n::detect` walks the list, so
  it is not used.
- `pub fn endonym(tag: &LanguageTag) -> Option<&'static str>`: a language's
  name in itself, for the "not translated yet" line. A table of about forty
  common languages (Français, Deutsch, Español, Italiano, 日本語…); an
  unknown one gives `None`, and the line names its language subtag (`xx`).
- `pub fn fill(text, &[(placeholder, value)])`: every placeholder put in
  in one pass, so a name that holds another placeholder's name (a table
  called `{columns}`) is put in as it is.

`App` keeps `system_language: Option<LanguageTag>`, read once in
`App::new` (`None` under `cfg!(test)`, as the system zone is UTC there), and
`App::locale` is `settings.language.resolve(system_language)`.

## The setting

`crates/tabletist-core/src/settings.rs`:

- `enum Language { #[default] System, Fixed(Locale) }`
  - `name()`: `system`, `en`, `ru`.
  - `from_name()`: `system` in any case, or a tag `Locale::from_tag` knows.
    Anything else is an invalid line, shown red in the file pane and
    ignored, as an unknown zone is.
  - `resolve(system: Option<&LanguageTag>) -> Locale`: `Fixed(locale)` is
    that locale; `System` is the system's first language where it has a
    catalog, else English.
- `Key::Language`: path `("interface", "language")`, note
  `system | "en" | "ru"` (the canvas's note lists all four; each later run
  adds its language). First in `Key::ALL`, so a new file writes
  `[interface]` first.
- `OptionId::Language`, `OptionValue::Language(Language)`, first in
  `OptionId::GENERAL`, in a new section `OptionId::LANGUAGE`; the General
  pane's rows start with it.
- Stepping (h/l on Omarchy) goes through the menu's entries in order and
  stops at either end (`Settings::stepped_language`). A step that stays
  leaves System as it is.

## The menu's entries

- **First language has a catalog** (Russian, or English):
  `System (Русский)` / `System (English)`, a separator, then `English`,
  `Русский`.
- **First language has no catalog** (French, or none read): `English`,
  `Русский`. The stored `system` shows as English, with ✓ on English.
  Choosing English writes `en`.
- **Names:** each language is written in its own name. Every entry but
  the current language's carries the language's name in the current
  language, in grey: `Русский` with "Russian" beside it in English, and
  `English` with «Английский» beside it in Russian.
- **"System" entry:** `System (…)` is translated (`System ({language})`),
  with the native name inside.

## Settings › General

**The classic sheet** (`src/ui/settings/sheet.rs`), as
`MacSettingsGeneral` draws it:

- **Section:** a first section, "Language", with one row.
- **Label:** "Language". In any language but English, the English word
  follows in grey: «Язык · Language». The English word is not translated:
  it is the way back.
- **Small print:** "Menus, labels and messages; data and SQL stay as they
  are". In its place, while the value is System and the first language
  has no catalog: "{language} is not translated yet", with the endonym
  (or the language subtag). It is left out of the label column's width,
  as History's long small print is.
- **Control:** a dropdown (`menu::Dropdown`, as `launch_menu`) with the
  entries above, and a "Help translate" link beside it. The link opens
  `https://igor.works/tabletist/translating/` with `ctx.open_url`.

**The Omarchy screen** (`src/ui/settings/terminal.rs`), as
`OmarchySettingsGeneral` and `OmarchySettingsLanguage` draw it:

- **Section:** the heading `language` and its row before `startup`.
- **Value:**
  - `system (english)`, `system (русский)`, `english` or `русский`, in the
    text's colour, as the zone's `system (america/new_york)` is
  - `‹ ›` while the cursor is on the row
  - h/l step; R puts back System
- **Hint:**
  - `from $LANG · help translate: assets/i18n`
  - with no catalog for the first language:
    `français is not translated yet · help translate: assets/i18n`

## A live change

`App::apply_settings` sets `self.locale` again from the settings and the
system's language read at start; the window redraws in the next frame, as every
string is looked up each frame. Tabs, edits and the editor's text are kept.

`src/menu/bar.rs` remembers the locale it last described. `Bar::sync`
rebuilds the menu bar when `app.locale` differs, through the same
`describe` + `rebuild` path a changed shortcut takes.

Text already said in the old language (a toast on screen, a status line
message) stays as it was said.

## The catalog tooling

- **`assets/i18n/`:**
  - `tabletist.pot` is the template, from every `.rs` file git has under
    `src/` and `crates/`. References name the file, not the line, so
    moving code changes nothing.
  - `ru.po` is the Russian catalog. Its header carries
    `Plural-Forms: nplurals=3; plural=(n%10==1 && n%100!=11 ? 0 : n%10>=2 && n%10<=4 && (n%100<10 || n%100>=20) ? 1 : 2);`
- **`assets/i18n/update.sh`:** runs `xgettext` with the keywords fastframe-i18n
  expects (`gettext:2`, `ngettext:2,3`, `pgettext:2c,3`), `msgmerge`s every
  catalog without fuzzy matching, and checks each with `msgfmt --check`.
  With `--check` it changes nothing and fails when the template or a
  catalog is out of date, `xgettext` warns, or a catalog fails `msgfmt`.
  It needs GNU gettext 0.24 or later (Rust support).
- **CI:** a `translations` job on macOS (Ubuntu's gettext is older than
  0.24) installs Homebrew's `gettext` and runs `assets/i18n/update.sh
  --check`.
- **Message clashes:** `xgettext` warns about a message used both alone
  and as a plural. `row` takes a context where it stands alone
  (`pgettext("row panel title", …)`), and `is required` is reworded into
  whole phrases. `counted(locale, n, one, many)` is a keyword too
  (`counted:3,4`), which brings `filter`, `new row` and `column` in as
  plurals: their lone uses take the context `label`.
- **Obsolete entries:** a message the code no longer has is dropped from
  every catalog (`msgattrib --no-obsolete`), not kept. A new `.rs` file
  is read before it is added to git.
- **Build:** the build compiles each `.po` (fastframe-i18n's `build.rs`), so
  a catalog that does not parse fails the build.

## The contributor guide

- **The new page:** `docs/_reference/translating.md` ("Translating", nav
  order 4).
  - One `.po` per language in `assets/i18n`, edited in a text editor or
    Poedit.
  - `assets/i18n/update.sh` after a code change.
  - What stays English.
  - Placeholders and plurals.
  - That a missing string shows in English.
  - How to try a catalog: set `[interface] language`.
  - A pull request.
- **`docs/_reference/settings-and-files.md`:** the Language row in the
  General table, `[interface] language` in the full TOML block and the key
  table.
- **`docs/_guide/omarchy.md`:** the Settings section names the new row.

## Testing

Harness tests (`crate::testing::Harness`), headless, no design
conformance:

- **Settings file:**
  - `interface.language` reads `system`, `en`, `ru` and `RU`.
  - `fr` and `xx` are invalid lines, kept and ignored.
  - `Fixed(Russian)` writes `ru`; the default writes nothing.
- **`Language::resolve`:**
  - `ru-RU` → Russian, `en_GB.UTF-8` → English, `fr-FR` → English, none →
    English, `Fixed` over any system.
- **The classic sheet:**
  - the Language row is the General pane's first.
  - "System (English)" with an English system.
  - "English" and the "Français is not translated yet" line with a French
    system.
  - choosing Русский redraws the window in Russian: the sheet's title
    «Настройки» and the label «Язык · Language» (the nav's pane names are
    run 2's).
  - the file gets `language = "ru"`.
- **The Omarchy screen:**
  - h/l steps System → English → Русский, and stops there.
  - R puts back System.
  - the row's words.
- **Reset all:** General's reset keeps the language.
- **Menu bar:** after the change, `bar::describe` gives Russian titles, and
  `Bar::sync`'s check says a rebuild is due (the native swap itself is not
  exercised headless).
- **Catalog:**
  - `Locale::Russian` translates through the compiled catalog, with
    Russian's three plural forms (1, 3, 5, 11, 21).
  - Every msgid with a placeholder has the same placeholders in its
    `ru.po` msgstr (a test reading `assets/i18n/ru.po`).
- **CI:** `assets/i18n/update.sh --check` runs in the `translations` job.
- **AGENTS.md:** the Checks section says to run `assets/i18n/update.sh`
  after a string changes, and that a message is a whole phrase.
