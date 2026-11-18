---
title: Translating
description: How Tabletist's interface is translated, and how to add or improve a language.
nav_order: 4
---

Tabletist's interface is in English and Русский. Each language is one plain
text file, a gettext catalog, in
[`assets/i18n`](https://github.com/igor-alexandrov/tabletist/tree/main/assets/i18n):
`ru.po` for Russian. Every string the interface draws is looked up there,
and one the catalog lacks shows in English, so a catalog can be finished a
part at a time.

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

To try a change, build Tabletist ([Build from Source](../build-from-source/))
and choose the language in Settings › General, or set it in
[settings.toml](../settings-and-files/#the-settings-file):

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
msginit --no-translator --locale=pt_BR.UTF-8 \
  --input=assets/i18n/tabletist.pot --output-file=assets/i18n/pt-BR.po
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

The Omarchy look writes every label in lower case, so write ordinary
sentence case.

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

## After a change to the code

When a change adds, changes or takes out a message, run
`assets/i18n/update.sh` from the repository. It writes the template,
`assets/i18n/tabletist.pot`, adds the new messages to every catalog
untranslated, and checks each catalog. It needs GNU gettext 0.24 or
later, which reads Rust: `brew install gettext` on macOS, or your
distribution's `gettext` where it is that new. CI runs it with `--check`
and fails when the template or a catalog is out of date.
