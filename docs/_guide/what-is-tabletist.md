---
title: What is Tabletist?
description: What Tabletist does, which databases and systems it works with, and what it does not do yet.
nav_order: 0
---

## Why Tabletist

**A fast, native database client.** Tabletist connects to PostgreSQL, MySQL
and SQLite. It is written in Rust and draws its own window, so there is no
browser inside. It runs on Linux (Omarchy and Hyprland first), macOS and
Windows.

{% include shot.html file="macos-table" alt="Tabletist with a table's data grid, the sidebar of tables on the left and the row panel on the right" %}

## What it does

- **Keeps your connections.** Saved connections sit in a picker, each tagged
  with its environment (local, dev, staging, production) and when it was
  last used. Open more than one and each is a chip in the window's header.
- **Connects the way your servers need.** Direct connections, TLS, and SSH
  tunnels with a password, a key file or your agent. Passwords live in the
  system keyring, or are asked for when you connect.
  [Read about connections]({% link _reference/connections.md %}).
- **Browses tables.** A sidebar of one schema's tables and views, a data
  grid with sorting, filters and paging done by the server, a row panel
  that shows every field of a row, and a Structure view.
  [Browse a table]({% link _guide/browsing-data.md %}).
- **Edits values safely.** Changes stay pending until you save. A save is
  one transaction. When someone else changed the same row, the save
  writes nothing and asks you what to do.
  [See how editing works]({% link _guide/editing-data.md %}).
- **Runs SQL.** An editor per connection with completion, a formatter, a
  row limit and a timeout.
  [Open the SQL editor guide]({% link _guide/sql-editor.md %}).
- **Is careful with production.** A production connection opens read-only
  unless you turn that off, and a save to production asks first, with the
  statements on screen.
- **Looks at home.** A macOS look in IBM Plex, and on Linux the Omarchy
  look: square, keyboard first, with vim keys.
  Read about [the Omarchy look]({% link _guide/omarchy.md %}) and
  [Tabletist on macOS]({% link _guide/macos.md %}).

## What it does not do yet

Tabletist is young. These are the limits you are most likely to meet:

- You can change values in existing rows. Adding, duplicating and deleting
  rows are not built yet.
- Binary values, values over 256 KiB, views, and tables without a primary
  key or unique index cannot be edited.
- The SQL editor does not run read-write on a production connection.
- SSH tunnels do not go through `ProxyJump` or `ProxyCommand`.
- The app is in English only.

## Open source

Tabletist is free and open source under the MIT license. The code, the
issues and the releases are on
[GitHub](https://github.com/igor-alexandrov/tabletist).
