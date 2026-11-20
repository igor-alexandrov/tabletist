# Translations, run 3: Brazilian Portuguese: Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Tabletist's interface ships a third language, Brazilian Portuguese (`pt-BR`, «Português (Brasil)»). It is listed in Settings › General, chosen by a `pt-BR` system, and every message is translated. Numbers and interface dates are written the Brazilian way.

**Architecture:** Runs 1 and 2 already route every interface string through gettext (`Msgid`, `pgettext`, `ngettext`) and keep a language's marks in `i18n::numbers`. This run adds a catalog, `assets/i18n/pt-BR.po`, which the build compiles to the `pt_br` module. It also adds one `Locale` variant and the few `match locale` arms that name a language: tag, name, catalog, marks and the menu's aside. `Locale::from_tag` stops comparing `tag()` with the language subtag, because `pt-BR`'s tag has a region. It matches language and region explicitly instead. Dates need no code: their order lives in the catalog's whole-phrase messages (`{month} {day}` becomes `{day} de {month}`).

**Tech Stack:**
- Rust and egui 0.36.
- fastframe-i18n v0.16.1. Its build compiles every `assets/i18n/*.po`, so `pt-BR.po` becomes `pt_br::Translator`. Its plural rules take gettext's C expression; `n > 1` parses.
- GNU gettext 1.0: `msginit`, `msgmerge`, `msggrep`, `msgattrib`, `msgcat`.
- The app's `testing::Harness`.

**Spec:** run 1's spec, `docs/superpowers/specs/2026-11-17-translations-1-russian-design.md` ("Adding a language"), and the contributor guide, `docs/_reference/translating.md`. Issue #344.

**Base:** `main` at e3097bff (PR #360 merged). Branch `claude/translations-3a-portuguese`, and later `-3b`, `-3c` and `-3d`, each cut from main once the one before it is merged. If main moves before a PR, rebase onto it (never a merge of main). Then run `assets/i18n/update.sh` again and translate what main added.

---

## What the survey found (2026-11-19)

The template, `tabletist.pot`, has 2,377 entries and about 8,400 English words. Each entry is counted by the file of its first `#:` reference:

| Area | Entries | Words |
|---|---|---|
| `src/ui/settings/**` | 419 | 1,427 |
| keymap (`crates/tabletist-core/src/keymap*`), menus, command palette, `src/app*` | 90 by first reference; 3a's slice takes every entry any of these files uses, 929 in all | 270 |
| `src/ui` views, `about.rs` to `export_toast.rs` (alphabetical) | 671 | ~2,400 |
| `src/ui` views, `file_list.rs` to `write_prompts.rs` | 660 | ~2,270 |
| `crates/**` (format, clock, numbers, export, connections, MCP) | 537 | 2,020 |

The code that names a language, outside tests:
- `i18n.rs`: `Locale`, `ALL`, `tag`, `native_name`, `from_tag`, `catalog`.
- `i18n/numbers.rs`: `marks`.
- `ui/settings/mod.rs`: `language_aside`, the language's name in the window's language (`gettext(locale, "Russian")`).

Nothing else matches on a `Locale` variant. The calendar's first weekday is a setting of its own (`calendar.week`), not the language's, and it stays so.

## Decisions for Igor (before code)

| What | This plan builds | Why |
|---|---|---|
| **Which tags choose it** | **Igor chose: every Portuguese tag.** `pt-BR`, `pt_BR.UTF-8`, plain `pt`, `pt-PT` and `pt-AO` all read Brazilian Portuguese, as `ru_UA` reads Russian | Most European Portuguese readers read pt-BR far more easily than English. `from_tag` still compares the language subtag, but with each locale's own language subtag (`pt`), not with `tag()` (`pt-BR`) |
| **Its tag in the file** | `language = "pt-BR"`; read in any case, with `-` or `_` (`pt_br`, `PT-BR`) | BCP 47, as `tag()` writes it and as the catalog file is named |
| **Menu order** | **Igor chose:** native names, Latin script first: English, Português (Brasil), Русский. Omarchy's h/l steps through them in that order | macOS and Windows list languages by their own names, so a person finds theirs. Appending would put it after Русский |
| **Its name in other languages** | A new message, `Brazilian Portuguese`, for the menu's aside: «Португальский (Бразилия)» in `ru.po`. `pt-BR.po` writes `English` as «Inglês» and `Russian` as «Russo» | `language_aside` names each language in the window's language beside its own name |
| **Plural-Forms** | `nplurals=2; plural=(n > 1);`, as msginit writes for `pt_BR` | CLDR `pt`: 0 and 1 are singular («0 linha», «1 linha», «2 linhas») |
| **Number marks** | Group `.`, decimal `,` (CLDR `pt`): «12.480 linhas», «1.240,50», «1,2 mil» | The task's marks. `group_number` already treats a non-`.` decimal mark as Russian's is treated: a decimal grid number always takes the comma, so a column reads alike |
| **Short counts** | `{n} mil`, `{n} mi`, `{n} bi` (CLDR `pt` short) | In the catalog's `thousands`, `millions` and `billions` contexts |
| **Interface dates** | «13 de out.», «13 de out. de 2025», «qui. 15 de jan. de 2026», «Janeiro de 2026», «quinta-feira, 15 de janeiro de 2026». Months and weekdays are in lower case inside a date and capitalised alone (sentence case) | Brazilian order and the preposition `de`. Each is a whole-phrase message, so no code changes |
| **Calendar's weekday initials** | One letter: S T Q Q S S D (Monday first; the calendar orders them by its own setting) | What Brazilian calendars print. Two letters would be made up («Se», «Te», «Qu», «Qu») and still collide |
| **Time** | 24-hour `10:42:18`, as in every language | Brazil writes 24-hour time |
| **Grid values and dates** | As the database wrote them; dates stay ISO | Decided in run 1 |
| **Register** | Buttons and menu items take the infinitive («Salvar», «Excluir linha»). Instructions take the imperative («Escolha uma tabela»). The person is «você», used only where a sentence needs it | Apple's and Microsoft's pt-BR style |
| **Kept English** | What the guide keeps: keys, `:` commands, SQL, names, product names, environment names, placeholders, `NULL`, `DEFAULT`. Also the loanwords Brazilian developers use as they are: `commit`, `rollback`, `log`, `view` | `translating.md`; a native reviewer may prefer «registro» for log |
| **One PR or four** | **Igor chose four PRs, one plan.** 3a: the code, docs, the catalog's start, and Settings, the keymap, menus and palette (929 entries: an entry any of their files uses). 3b: views A to E (~670). 3c: views F to W (~660). 3d: the crates (~540) and the "nothing untranslated" test | ~2,400 entries is too many to read at once. Each PR is mergeable alone: an untranslated entry shows in English, and 3a already makes the language choosable, as run 1 did for Russian |

Each PR's description says that the Portuguese needs a native speaker's review, and lists the wording I was least sure of.

## Glossary (every slice uses it)

| English | pt-BR |
|---|---|
| table / view / column / row / cell | tabela / view / coluna / linha / célula |
| schema / database / connection | esquema / banco de dados / conexão |
| query / result / run | consulta / resultado / executar |
| filter / sort / page | filtro (filtrar) / ordenar / página |
| primary key / foreign key / index | chave primária / chave estrangeira / índice |
| value / type / default | valor / tipo / padrão |
| save / cancel / discard / apply | salvar / cancelar / descartar / aplicar |
| delete / remove / clear | excluir / remover / limpar |
| copy / paste / cut / undo / redo | copiar / colar / recortar / desfazer / refazer |
| pending changes / review | alterações pendentes / revisar (revisão) |
| read-only / write | somente leitura / gravar (gravação) |
| export / import | exportar / importar |
| history / log | histórico / log |
| tab / pane / sidebar / window / sheet | aba / painel / barra lateral / janela / painel |
| Settings / General / Appearance / Editor / Keyboard / Agents / Updates | Configurações / Geral / Aparência / Editor / Teclado / Agentes / Atualizações |
| System / Language / Help translate | Sistema / Idioma / Ajudar a traduzir |
| shortcut / command / command palette | atalho / comando / paleta de comandos |
| file / folder | arquivo / pasta |
| structure | estrutura |
| agent / access | agente / acesso |
| update / install / go back (an update) | atualização / instalar / voltar à versão anterior |

### Terms 3a settled (3b to 3d keep them)

| English | pt-BR |
|---|---|
| grid | grade |
| statement (SQL) | instrução |
| label (a foreign key's) / label chooser | rótulo / seletor de rótulo |
| toast | notificação |
| prompt (a question the app asks) / the `:` prompt | pergunta / linha de comando |
| Keybindings (vim or standard) | Modo de teclado |
| Autocomplete (the setting) / a completion | Preenchimento automático / sugestão |
| reset | redefinir |
| skip / overwrite / timeout | ignorar / sobrescrever / tempo limite |
| switch (database, tab) | trocar de |
| fold / open (a group) | recolher / expandir |
| cheat sheet | referência rápida |
| Agents Log | Log de agentes |
| Files connection | conexão de arquivos |
| Off / Read (an agent's access) | Desativado / Leitura |
| All (feminine) / Show all | Todas / Mostrar tudo |
| record (a shortcut) / keyboard row | gravar / linha de atalho |
| release / build / release notes | versão / compilação / notas da versão |
| Commit cell (an edit, not SQL's) | Confirmar a célula |
| View (the object kind, context `object kind`) / View (the menu) | View / Visualizar |
| Keychain / keyring / Credential Manager | Acesso às Chaves / chaveiro / Gerenciador de Credenciais |

## The PR split

An entry belongs to the slice of its first `#:` reference. A slice is cut from what is still untranslated, so an entry shared between areas is translated once, by the first slice that reaches it.

| PR | Slice (msggrep `-N` patterns) |
|---|---|
| 3a | `src/ui/settings/*`, `crates/tabletist-core/src/keymap*`, `crates/tabletist-core/src/keymap/*`, `src/menu.rs`, `src/app.rs`, `src/app/*` |
| 3b | `src/ui/[a-e]*` (`about.rs` to `export_toast.rs`, `connect_dialog/` among them) |
| 3c | `src/ui/[f-z]*` and anything else left in `src/` |
| 3d | `crates/*`, and whatever is still empty |

## Files

- Create: `assets/i18n/pt-BR.po`: the catalog, filled a slice at a time.
- Modify: `crates/tabletist-core/src/i18n.rs`: `Locale::BrazilianPortuguese`, `ALL`, `tag`, `native_name`, `from_tag`, `catalog`; the PO tests generalised to every catalog.
- Modify: `crates/tabletist-core/src/i18n/numbers.rs`: `marks`, and the tests.
- Modify: `crates/tabletist-core/src/settings.rs`: the tests (`from_name`, `choices`, `stepped_language`). `Language` itself needs no change.
- Modify: `src/ui/settings/mod.rs:301-307`: `language_aside`.
- Modify: `src/ui/language_tests.rs`: the Omarchy step test, the fr_FR test, and new pt-BR cases.
- Modify: `assets/i18n/ru.po`: «Португальский (Бразилия)».
- Modify: `assets/i18n/tabletist.pot`: written by `update.sh`.
- Modify: `docs/_reference/translating.md`, `docs/_reference/settings-and-files.md` (rows at lines 15 and 278), `docs/_guide/omarchy.md:394`.
- Test: `crates/tabletist-core/src/clock.rs`, `src/ui/date_line.rs`: one pt-BR assertion each, beside the Russian ones (in 3d and 3c, once their slices are translated).

---

## PR 3a: the language, Settings and menus

### Task 1: The locale and its tags

**Files:**
- Modify: `crates/tabletist-core/src/i18n.rs`
- Create: `assets/i18n/pt-BR.po`

- [ ] **Step 1: Start the catalog from the template**

```bash
msginit --no-translator --locale=pt_BR.UTF-8 \
  --input=assets/i18n/tabletist.pot --output-file=assets/i18n/pt-BR.po
```

Check that the header says `"Language: pt_BR\n"` and `"Plural-Forms: nplurals=2; plural=(n > 1);\n"`. Then set its first comment line to `# Brazilian Portuguese translations for Tabletist package.`, as `ru.po` names Russian.

- [ ] **Step 2: Write the failing tests**

In `i18n.rs`'s tests, replace `a_tag_names_a_shipped_language_by_its_language_alone` with:

```rust
#[test]
fn a_tag_names_a_shipped_language() {
    assert_eq!(Locale::from_tag(&tag("ru")), Some(Locale::Russian));
    assert_eq!(Locale::from_tag(&tag("ru-RU")), Some(Locale::Russian));
    assert_eq!(Locale::from_tag(&tag("ru_UA.UTF-8")), Some(Locale::Russian));
    assert_eq!(Locale::from_tag(&tag("en_GB.UTF-8")), Some(Locale::English));
    let brazilian = Some(Locale::BrazilianPortuguese);
    assert_eq!(Locale::from_tag(&tag("pt-BR")), brazilian);
    assert_eq!(Locale::from_tag(&tag("pt_BR.UTF-8")), brazilian);
    assert_eq!(Locale::from_tag(&tag("PT-br")), brazilian);
    // Every Portuguese reads Brazilian: the only Portuguese shipped.
    assert_eq!(Locale::from_tag(&tag("pt")), brazilian);
    assert_eq!(Locale::from_tag(&tag("pt-PT")), brazilian);
    assert_eq!(Locale::from_tag(&tag("pt_AO.UTF-8")), brazilian);
    assert_eq!(Locale::from_tag(&tag("fr-FR")), None);
    assert_eq!(Locale::from_tag(&tag("C")), None);
}

#[test]
fn every_shipped_language_reads_back_from_its_tag() {
    for locale in Locale::ALL {
        assert_eq!(Locale::from_tag(&tag(locale.tag())), Some(locale), "{locale:?}");
    }
}
```

Also add, after `russian_has_a_catalog_and_english_none`:

```rust
#[test]
fn brazilian_portuguese_has_a_catalog_with_two_forms() {
    use fastframe_i18n::Locale as _;
    let pt = Locale::BrazilianPortuguese;
    assert!(pt.catalog().is_some());
    assert_eq!(gettext(pt, "Settings"), "Configurações");
    let rows = |count: u64| ngettext(pt, "{n} row", "{n} rows", count);
    assert_eq!(rows(0), "{n} linha");
    assert_eq!(rows(1), "{n} linha");
    assert_eq!(rows(2), "{n} linhas");
}
```

- [ ] **Step 3: Run them and see them fail**

Run: `~/.cargo/bin/cargo test -p tabletist-core i18n::tests`
Expected: does not compile: no variant `BrazilianPortuguese`.

- [ ] **Step 4: Add the variant**

```rust
pub enum Locale {
    #[default]
    English,
    BrazilianPortuguese,
    Russian,
}

impl Locale {
    /// Every language the app ships, in the order the language menu lists
    /// them: by their own names, Latin script first.
    pub const ALL: [Locale; 3] = [Self::English, Self::BrazilianPortuguese, Self::Russian];

    /// The tag the settings file writes: `en`, `pt-BR`, `ru`.
    pub fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::BrazilianPortuguese => "pt-BR",
            Self::Russian => "ru",
        }
    }

    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::BrazilianPortuguese => "Português (Brasil)",
            Self::Russian => "Русский",
        }
    }

    /// The language subtag that chooses it: `en`, `pt`, `ru`.
    fn language(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::BrazilianPortuguese => "pt",
            Self::Russian => "ru",
        }
    }

    /// The language `tag` names, where the app ships it: its language
    /// subtag alone decides (`ru-RU`, `ru_UA.UTF-8` are Russian; `pt-PT`
    /// is Brazilian Portuguese, the only Portuguese shipped).
    pub fn from_tag(tag: &LanguageTag) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|locale| locale.language() == tag.language)
    }
}
```

In the `fastframe_i18n::Locale` impl, add `Self::BrazilianPortuguese => Some(&pt_br::Translator),`. The module doc's "(`ru.po`)" becomes "(`ru.po`, `pt-BR.po`)".

`LanguageTag::parse` lowercases its subtags (`endonym` already matches `Some("br")`), so `PT-br` matches.

- [ ] **Step 5: Give the catalog the two test entries**

The new test needs «Configurações» and «{n} linha» / «{n} linhas» in `pt-BR.po`. They are part of 3a's slice (Task 4), so translate them now:

```po
msgid "Settings"
msgstr "Configurações"

msgid "{n} row"
msgid_plural "{n} rows"
msgstr[0] "{n} linha"
msgstr[1] "{n} linhas"
```

- [ ] **Step 6: Run the tests and see them pass**

Run: `~/.cargo/bin/cargo test -p tabletist-core i18n`
Expected: PASS. A non-exhaustive match elsewhere fails to compile: `numbers::marks` and `settings/mod.rs::language_aside`. Tasks 2 and 3 cover them, so build only `-p tabletist-core` until then. `marks` is in this crate, so add its arm (Task 2, Step 3) before running.

- [ ] **Step 7: Commit**

```bash
git add assets/i18n/pt-BR.po crates/tabletist-core/src/i18n.rs
git commit -m "Ship Brazilian Portuguese as a third language, chosen by a pt-BR tag"
```

### Task 2: Its numbers

**Files:**
- Modify: `crates/tabletist-core/src/i18n/numbers.rs`

- [ ] **Step 1: Write the failing test**

In `numbers.rs`'s tests, `use crate::i18n::Locale::{BrazilianPortuguese as Brazilian, English, Russian};`, and add:

```rust
#[test]
fn brazilian_portuguese_groups_by_points_with_a_decimal_comma() {
    assert_eq!(marks(Brazilian), ('.', ','));
    assert_eq!(number(Brazilian, 12_480_u64), "12.480");
    assert_eq!(number(Brazilian, 999_u64), "999");
    assert_eq!(group_number(Brazilian, "1240.50"), "1.240,50");
    assert_eq!(group_number(Brazilian, "-1234567"), "-1.234.567");
    // A decimal number takes its comma even with nothing to group.
    assert_eq!(group_number(Brazilian, "12.5"), "12,5");
    assert_eq!(group_number(Brazilian, "1e5"), "1e5");
}
```

The cases for sizes and short counts («1,2 mil», «5,0 MB») need the catalog's units, so they go in 3d with that slice (Task 11).

- [ ] **Step 2: Run it and see it fail**

Run: `~/.cargo/bin/cargo test -p tabletist-core numbers`
Expected: does not compile (non-exhaustive `match` in `marks`).

- [ ] **Step 3: Add the arm**

```rust
pub fn marks(locale: Locale) -> (char, char) {
    match locale {
        Locale::English => (',', '.'),
        // CLDR's `pt`: a point between groups, a decimal comma.
        Locale::BrazilianPortuguese => ('.', ','),
        // CLDR's: a no-break space, so a wrap never splits a number.
        Locale::Russian => ('\u{a0}', ','),
    }
}
```

Also update the doc of `number`: "A count as the interface writes it: `12,480`, «12.480», «12 480»."

- [ ] **Step 4: Run it and see it pass**

Run: `~/.cargo/bin/cargo test -p tabletist-core numbers`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/i18n/numbers.rs
git commit -m "Write Brazilian Portuguese numbers with a point between groups and a decimal comma"
```

### Task 3: Settings lists it

**Files:**
- Modify: `src/ui/settings/mod.rs:301-307`
- Modify: `crates/tabletist-core/src/settings.rs` (tests)
- Modify: `src/ui/language_tests.rs`
- Modify: `assets/i18n/ru.po`

- [ ] **Step 1: Write the failing core tests**

In `settings.rs`, `the_language_is_read_by_its_tag_and_anything_else_is_a_wrong_line` adds:

```rust
let brazilian = Language::Fixed(Locale::BrazilianPortuguese);
for name in ["pt-BR", "pt_BR", "PT-BR", "pt_BR.UTF-8"] {
    assert_eq!(read(name).settings.language, brazilian, "{name:?}");
}
let written = Settings {
    language: brazilian,
    ..Settings::default()
};
assert!(written.to_toml().contains("[interface]\nlanguage = \"pt-BR\""));
```

`"pt"` and `"pt-PT"` read as Brazilian too, and are written back as `pt-BR`.

`the_menu_lists_system_only_where_its_language_has_a_catalog` lists three languages:

```rust
let brazilian = Language::Fixed(Locale::BrazilianPortuguese);
assert_eq!(
    Language::choices(Some(&tag("pt-BR"))),
    [Language::System, english, brazilian, russian]
);
assert_eq!(Language::choices(Some(&tag("fr-FR"))), [english, brazilian, russian]);
assert_eq!(Language::choices(None), [english, brazilian, russian]);
```

The other `choices` asserts get `brazilian` between `english` and `russian`.

`a_step_moves_through_the_menu_and_stops_at_its_ends` steps `english → brazilian → russian`, and back:

```rust
assert_eq!(step(english, Some(&ru), true), OptionValue::Language(brazilian));
assert_eq!(step(brazilian, Some(&ru), true), OptionValue::Language(russian));
assert_eq!(step(russian, Some(&ru), true), OptionValue::Language(russian));
assert_eq!(step(russian, Some(&ru), false), OptionValue::Language(brazilian));
```

`system_follows_the_first_language_only_and_else_english` adds `assert_eq!(system.resolve(Some(&tag("pt_BR.UTF-8"))), Locale::BrazilianPortuguese);`.

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test -p tabletist-core settings::tests::`
Expected: PASS. `Language` reads `ALL` and `from_tag`, so Task 1 already made these true. Fix any assert whose order Step 1 missed.

- [ ] **Step 3: Write the failing UI tests**

In `src/ui/language_tests.rs`:

`the_omarchy_row_steps_through_the_languages_and_r_puts_back_system`: after English, `l` reaches Português, then Russian, which stays:

```rust
harness.press(Key::L, Modifiers::NONE);
assert_eq!(
    harness.app.settings.language,
    Language::Fixed(Locale::BrazilianPortuguese)
);
assert_eq!(harness.app.locale, Locale::BrazilianPortuguese);
assert!(painted(&harness, "‹ português (brasil) ›"));
harness.press(Key::L, Modifiers::NONE);
assert_eq!(harness.app.settings.language, Language::Fixed(Locale::Russian));
assert!(painted(&harness, "‹ русский ›"));
// The last: it stays.
harness.press(Key::L, Modifiers::NONE);
assert_eq!(harness.app.settings.language, Language::Fixed(Locale::Russian));
harness.press(Key::H, Modifiers::NONE);
assert_eq!(
    harness.app.settings.language,
    Language::Fixed(Locale::BrazilianPortuguese)
);
harness.press(Key::H, Modifiers::NONE);
assert_eq!(harness.app.settings.language, Language::Fixed(Locale::English));
harness.press(Key::R, Modifiers::SHIFT);
assert_eq!(harness.app.settings.language, Language::System);
```

`the_omarchy_row_says_a_system_language_is_not_translated`: from fr_FR's English, `l` reaches `Locale::BrazilianPortuguese` (it was Russian).

New:

```rust
#[test]
fn a_brazilian_system_reads_portuguese() {
    let harness = general(Look::macos(), Some("pt_BR.UTF-8"));
    assert_eq!(harness.app.locale, Locale::BrazilianPortuguese);
    assert_eq!(harness.app.settings.language, Language::System);
    assert!(painted(&harness, "Sistema (Português (Brasil))"));
    assert!(painted(&harness, "Idioma · Language"));
}

#[test]
fn choosing_portuguese_writes_its_tag_and_names_the_others_in_portuguese() {
    let mut harness = general(Look::macos(), Some("en-US"));
    open_the_menu(&mut harness, "Language");
    harness.click("Language Português (Brasil)");
    assert_eq!(harness.app.locale, Locale::BrazilianPortuguese);
    assert!(painted(&harness, "Configurações"));
    assert!(painted(&harness, "Geral"));
    let saved = saved_settings(&harness);
    assert!(saved.contains("[interface]\nlanguage = \"pt-BR\""), "{saved}");
    // The menu names each other language in Portuguese beside its own name.
    open_the_menu(&mut harness, "Idioma");
    assert!(painted(&harness, "Inglês"));
    assert!(painted(&harness, "Russo"));
}

#[test]
fn a_russian_window_names_portuguese_in_russian() {
    let mut harness = general(Look::macos(), Some("ru-RU"));
    open_the_menu(&mut harness, "Язык");
    assert!(painted(&harness, "Português (Brasil)"));
    assert!(painted(&harness, "Португальский (Бразилия)"));
}
```

Pull the "last Save command's text" loop out of `choosing_russian_redraws_the_window_and_writes_the_file` into `fn saved_settings(harness: &Harness) -> String`, and use it in both tests. Add a case where a `pt-PT` system reads Brazilian Portuguese, with System listed.

- [ ] **Step 4: Run them and see them fail**

Run: `~/.cargo/bin/cargo test --bin tabletist language_tests`
Expected: does not compile at first (`language_aside`'s match). After Step 5, the painted-text asserts fail until the catalog has «Sistema ({language})», «Idioma», «Configurações», «Geral», «Inglês» and «Russo». They come with Task 4.

- [ ] **Step 5: Name it in the window's language**

```rust
pub(super) fn language_aside(locale: Locale, of: Locale) -> Option<String> {
    let name = match of {
        Locale::English => gettext(locale, "English"),
        Locale::BrazilianPortuguese => gettext(locale, "Brazilian Portuguese"),
        Locale::Russian => gettext(locale, "Russian"),
    };
    (of != locale).then(|| name.into_owned())
}
```

Run `assets/i18n/update.sh`. It adds `Brazilian Portuguese` to the template and to both catalogs. In `ru.po`, translate it as «Португальский (Бразилия)», since `every_russian_entry_is_translated_without_an_em_dash` requires it.

- [ ] **Step 6: Commit (the UI tests pass after Task 4)**

```bash
git add src/ui/settings/mod.rs crates/tabletist-core/src/settings.rs src/ui/language_tests.rs assets/i18n
git commit -m "List Português (Brasil) in the language menu and on Omarchy's row"
```

### Task 4: Translate the 3a slice

**Files:**
- Modify: `assets/i18n/pt-BR.po`

- [ ] **Step 1: Cut the slice**

```bash
scratch=$(mktemp -d -p "$TMPDIR")
msgattrib --untranslated assets/i18n/pt-BR.po \
  | msggrep -N 'src/ui/settings/*' -N 'src/keymap*' -N 'src/menu*' -N 'src/app*' \
  -o "${scratch}/3a.po"
grep -c '^msgid ' "${scratch}/3a.po"   # ~510
```

- [ ] **Step 2: Translate it**

Fill every `msgstr` in `3a.po`, in chunks of about 150 entries. Follow the glossary, the register row and the guide's "What to keep as it is". Read the `#:` file when a short message could mean more than one thing. Note each entry I am unsure of in `${scratch}/unsure-3a.md`, with the English, my Portuguese and why; the PR description lists them.

- [ ] **Step 3: Merge it into the catalog**

```bash
msgcat --use-first "${scratch}/3a.po" assets/i18n/pt-BR.po -o "${scratch}/merged.po" \
  && mv "${scratch}/merged.po" assets/i18n/pt-BR.po \
  && assets/i18n/update.sh
```

`update.sh` puts the catalog back in the template's order and checks it with `msgfmt --check`.

- [ ] **Step 4: Check the slice is done and keeps its placeholders**

```bash
msgattrib --untranslated assets/i18n/pt-BR.po \
  | msggrep -N 'src/ui/settings/*' -N 'src/keymap*' -N 'src/menu*' -N 'src/app*' \
  | grep -c '^msgid '   # 1: the header alone
grep -n '—' assets/i18n/pt-BR.po   # nothing
```

Run: `~/.cargo/bin/cargo test --bin tabletist language_tests && ~/.cargo/bin/cargo test -p tabletist-core i18n`
Expected: PASS, the placeholder test of Task 5 included.

- [ ] **Step 5: Commit**

```bash
git add assets/i18n/pt-BR.po
git commit -m "Translate Settings, the menus and the command palette into Brazilian Portuguese"
```

### Task 5: The catalog tests cover every catalog

**Files:**
- Modify: `crates/tabletist-core/src/i18n.rs` (tests)

- [ ] **Step 1: Generalise the placeholder and em-dash tests**

Russian's two tests become one table of catalogs, each with its minimum entry count:

```rust
/// Every catalog the app ships, by file.
const CATALOGS: [(&str, &str); 2] = [
    ("ru.po", include_str!("../../../assets/i18n/ru.po")),
    ("pt-BR.po", include_str!("../../../assets/i18n/pt-BR.po")),
];

#[test]
fn every_translation_keeps_its_placeholders() {
    for (file, text) in CATALOGS {
        let entries = entries(text);
        assert!(entries.len() > 1400, "{file}: {} entries", entries.len());
        for (ids, strs) in entries {
            let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
                continue;
            };
            let wanted = placeholders(id);
            for translated in strs.iter().filter(|text| !text.is_empty()) {
                assert_eq!(placeholders(translated), wanted, "{file}: {id:?} → {translated:?}");
            }
        }
    }
}

#[test]
fn no_translation_has_an_em_dash_or_is_fuzzy() {
    for (file, text) in CATALOGS {
        assert!(!text.contains("#, fuzzy"), "{file}: a fuzzy entry shows in English");
        for (ids, strs) in entries(text) {
            let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
                continue;
            };
            for translated in &strs {
                assert!(!translated.contains('—'), "{file}: {id:?} → {translated:?}");
            }
        }
    }
}

#[test]
fn every_russian_entry_is_translated() {
    assert_every_entry_is_translated(CATALOGS[0]);
}

fn assert_every_entry_is_translated((file, text): (&str, &str)) {
    for (ids, strs) in entries(text) {
        let Some(id) = ids.first().filter(|id| !id.is_empty()) else {
            continue;
        };
        assert!(
            !strs.is_empty() && strs.iter().all(|text| !text.is_empty()),
            "{file}: {id:?} is not translated"
        );
    }
}
```

The task names a pt-BR placeholder test. `every_translation_keeps_its_placeholders` is that test, and it covers `pt-BR.po` from 3a on. PR 3d adds `every_brazilian_portuguese_entry_is_translated` (Task 11).

- [ ] **Step 2: Run them**

Run: `~/.cargo/bin/cargo test -p tabletist-core i18n::tests`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add crates/tabletist-core/src/i18n.rs
git commit -m "Check every catalog's placeholders and dashes, Brazilian Portuguese's among them"
```

### Task 6: Docs

**Files:**
- Modify: `docs/_reference/translating.md`, `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`

- [ ] **Step 1: Name the language**

- `translating.md`:
  - The first paragraph says "in English, Português (Brasil) and Русский" and "`ru.po` for Russian, `pt-BR.po` for Brazilian Portuguese".
  - The `msginit` example becomes `--locale=de_DE.UTF-8 … --output-file=assets/i18n/de.po`, since pt-BR exists now.
  - The Plurals section adds: «Brazilian Portuguese has two, and counts 0 as one: «0 linha», «2 linhas».»
  - The marks paragraph adds «12.480 linhas» and «1,2 mil».
  - The Dates section adds «13 de out. de 2025».
- `settings-and-files.md:15`: "System, English, Português (Brasil) or Русский."
- `settings-and-files.md:278`: `"system"`, `"en"`, `"pt-BR"` or `"ru"`, in any case. Then: "a regional tag of one of them, such as `"ru-RU"`, reads as that language, and any Portuguese tag (`"pt"`, `"pt-PT"`) reads as Brazilian Portuguese."
- `omarchy.md:394`: "System, English, Português (Brasil), then Русский".

- [ ] **Step 2: Commit**

```bash
git add docs/_reference docs/_guide/omarchy.md
git commit -m "Name Português (Brasil) in the translating guide and the settings reference"
```

### Task 7: Verify and open PR 3a

- [ ] **Step 1: The checks the task's "done" names**

```bash
assets/i18n/update.sh --check
~/.cargo/bin/cargo fmt --all -- --check
~/.cargo/bin/cargo clippy --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --workspace --all-targets
```

Run the tests in the background: they take more than two minutes.

- [ ] **Step 2: Push and open the PR**

Title: "Add Brazilian Portuguese, run 3a: the language, Settings and menus (#344)". The body says:
- what 3a makes true;
- what is still English until 3b to 3d;
- that every Portuguese tag reads Brazilian Portuguese;
- **the Portuguese needs a native speaker's review**, followed by the unsure list.

---

## PR 3b: views, `about.rs` to `export_toast.rs`

### Task 8: Translate the 3b slice

Branch `claude/translations-3b-portuguese` from main once 3a is merged.

- [ ] **Step 1: Cut, translate, merge, check.** Same as Task 4 Steps 1 to 4, with `-N 'src/ui/[a-e]*'` (about 670 entries). Use the same glossary, and read 3a's catalog for terms already chosen.
- [ ] **Step 2: Commit, verify as Task 7, open the PR** ("run 3b"), with its unsure list. The date line and calendar assertions wait for 3d (Task 11): their month and weekday names come from `crates/tabletist-core/src/i18n/numbers.rs`, which 3b does not translate.

## PR 3c: views, `file_list.rs` to `write_prompts.rs`

### Task 9: Translate the 3c slice

- [ ] **Step 1: Cut, translate, merge, check.** As Task 4, with `-N 'src/*'` on what is still untranslated (about 660 entries).
- [ ] **Step 2: Commit, verify, open the PR** ("run 3c"). The history duration assertion waits for 3d (Task 11), since its units come from `numbers.rs`.

## PR 3d: the crates, and nothing left untranslated

### Task 10: Translate the 3d slice

- [ ] **Step 1: Cut, translate, merge, check.** As Task 4, with no `-N`: every entry still untranslated (about 540, from `crates/`).

### Task 11: Nothing untranslated, and the units

- [ ] **Step 1: The completeness test**

```rust
#[test]
fn every_brazilian_portuguese_entry_is_translated() {
    assert_every_entry_is_translated(CATALOGS[1]);
}
```

- [ ] **Step 2: The units**, in `numbers.rs`'s tests. 3a already translated the short counts («1,2 mil», «3,4 mi», «7 bi») and checks them in `brazilian_portuguese_groups_by_points_with_a_decimal_comma`, so the translating guide's example holds. The rest:

```rust
assert_eq!(file_size(Brazilian, 5 * 1024 * 1024), "5,0 MB");
assert_eq!(
    range_label(Brazilian, 0, 300, false, None, Some(1_234_567)).as_deref(),
    Some("1–300 de 1.234.567")
);
```

and in `clock.rs` beside line 147: `before.words(Locale::BrazilianPortuguese)` is «11 de out.», and `last_year`'s is «17 de dez. de 2025».

In `src/ui/date_line.rs:736-752`, beside the Russian `day_words` and `calendar` checks:

```rust
assert_eq!(day_words(day, Locale::BrazilianPortuguese), "qui. 15 de jan. de 2026");
assert_eq!(
    calendar(Locale::BrazilianPortuguese),
    ("Janeiro de 2026".into(), "quinta-feira, 15 de janeiro de 2026".into())
);
```

And beside `src/ui/history.rs:1583`'s Russian durations: `duration_words(1_500, Locale::BrazilianPortuguese)` is «1 s». `duration_words` divides by 1,000 before it formats, so seconds are whole.

- [ ] **Step 3: Run them**

Run: `~/.cargo/bin/cargo test -p tabletist-core`
Expected: PASS; `msgattrib --untranslated assets/i18n/pt-BR.po | grep -c '^msgid '` prints 1.

- [ ] **Step 4: Commit, verify as Task 7, open the PR** ("run 3d"). It says that the catalog is complete and that it still needs a native speaker's review.

---

## Not in this run

- European Portuguese (`pt-PT`) as a catalog of its own.
- Simplified Chinese: the next run.
- Screenshots in Portuguese. If a PR wants one, it uses the Bookshop demo data only and stays local, unblurred shots never pushed.
