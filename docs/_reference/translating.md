---
title: Translating
description: How Tabletist's interface is translated, and how to add or improve a language.
nav_order: 4
---

Tabletist's interface is in English, Português (Brasil) and Русский. Each
language is one plain text file, a gettext catalog, in
[`assets/i18n`](https://github.com/igor-alexandrov/tabletist/tree/main/assets/i18n):
`pt-BR.po` for Brazilian Portuguese, `ru.po` for Russian. The interface
looks its strings up there, and one the catalog lacks shows in English, so
a catalog can be finished a part at a time. If you find a word of the
interface that stays English whatever the catalog says, it needs a change
to the code, not to the catalog: tell us in an
[issue](https://github.com/igor-alexandrov/tabletist/issues).

Only the app is translated. This website and its guide stay in English.

## Improving a language

Open the language's `.po` file in a text editor or in
[Poedit](https://poedit.net). Each entry is an English message (`msgid`)
and its translation (`msgstr`):

```po
#: src/ui/settings/mod.rs
msgid "Help translate"
msgstr "Помочь с переводом"
```

The `#:` line says which file uses the message: read the code around it
when a short message could mean more than one thing.

To try a change, build Tabletist ([Build from Source]({% link _reference/build-from-source.md %}))
and choose the language in Settings › General, or set it in
[settings.toml]({% link _reference/settings-and-files.md %}#the-settings-file):

```toml
[interface]
language = "ru"
```

Then open a pull request with the file.

## Adding a language

Open an [issue](https://github.com/igor-alexandrov/tabletist/issues) first:
a new language also needs a few lines of code, which name it in the
language menu and tie it to its catalog.
Then start its catalog from the template:

```sh
msginit --no-translator --locale=de_DE.UTF-8 \
  --input=assets/i18n/tabletist.pot --output-file=assets/i18n/de.po
```

## What to keep as it is

- Keys and chords (`⌘S`, `Ctrl+Shift+P`, `j/k`, `ctrl+b`), and `:`
  commands (`:w`, `:set calendar`). What a key does is translated; the key
  is not.
- SQL and its keywords, names of tables, columns and types, and data.
- Product names: Tabletist, Omarchy, PostgreSQL, MySQL, SQLite, Redis,
  DuckDB, MCP.
- Environment names: dev, staging, production, DEV, PROD.
- Placeholders in braces, `{n}`, `{name}`, `{table}`, exactly as written:
  the app puts a number or a name in their place. Their order in your
  sentence is yours to choose.
- A leading or trailing space, and a trailing colon or `…`.
- What the grid shows of the data: its values and its dates, which stay
  as the database wrote them (dates in ISO, `2026-01-12`), and the type
  and size it writes for a binary value (`bytea · 48.2 KB`).
- The window's title, which names the connection, its environment and its
  database.

The Omarchy look writes every label in lower case, so write ordinary
sentence case.

Use no em dashes. Where English puts a key before what it does
(`Esc: cancel`), keep the colon: «Esc: отмена». Elsewhere, use a full stop,
a comma, a colon or parentheses.

## Plurals

A message with a number has an English singular (`msgid`) and plural
(`msgid_plural`), and as many translations as the language has forms,
which the catalog's `Plural-Forms` header counts. Russian has three:

```po
msgid "{n} row"
msgid_plural "{n} rows"
msgstr[0] "{n} строка"
msgstr[1] "{n} строки"
msgstr[2] "{n} строк"
```

Brazilian Portuguese has two, and counts 0 as one: «0 linha», «1 linha»,
«2 linhas».

The app writes the number with the language's own marks: «12 480 строк»,
with a no-break space between the thousands, and «1,2 тыс.» with a decimal
comma; «12.480 linhas» and «1,2 mil» in Brazilian Portuguese. These marks
are set in the code, not in the catalog.

## Dates

Months and weekdays are messages of their own, each with a context that
says how it is used: `month` is a name on its own («Январь»), `month in a
date` is how a date says it («13 января»), and `short month in a date` is
its short form («окт.»). A whole date is one message, such as `{month}
{day}, {year}`, so you choose the order and add what the language writes
around it: «{day} {month} {year} г.», «{day} de {month} de {year}».

## After a change to the code

When a change adds, changes or takes out a message, run
`assets/i18n/update.sh` from the repository. It writes the template,
`assets/i18n/tabletist.pot`, adds the new messages to every catalog
untranslated, and checks each catalog. It needs GNU gettext 1.0 or
later: `brew install gettext` on macOS, or your distribution's `gettext`
where it is that new. CI runs it with `--check`
and fails when the template or a catalog is out of date.
