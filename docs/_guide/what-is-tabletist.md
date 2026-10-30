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
- **Browses tables.** A sidebar of one schema's tables and views, quick
  open to find any of them by name, a data grid with sorting, filters and
  paging done by the server, a row panel that shows every field of a row,
  and Structure and DDL views.
  [Browse a table]({% link _guide/browsing-data.md %}).
- **Edits data safely.** Change values, and add, duplicate and delete
  rows. Dates have a calendar, and a foreign key searches the table it
  points at as you type. Changes stay pending until you save. A save is
  one transaction. When someone else changed the same row, the save
  writes nothing and asks you what to do.
  [See how editing works]({% link _guide/editing-data.md %}).
- **Runs SQL.** An editor per connection with completion, a formatter,
  Vim keys, a row limit and a timeout, read-only or read-write, and a
  History tab of every statement run on the connection.
  [Open the SQL editor guide]({% link _guide/sql-editor.md %}).
- **Lets agents read your databases.** Claude Code, Cursor and other
  agents ask Tabletist over MCP. They read only the connections you
  allow, never get the connection's password, and are kept from what is
  in the tables you hide and the values of the columns you mask. You can
  read a log of their calls.
  [Connect an agent]({% link _guide/agents.md %}).
- **Is careful with production.** A production connection opens read-only
  unless you turn that off, and a save to production asks first, with the
  statements on screen.
- **Looks at home.** A macOS look in IBM Plex, and on Linux the Omarchy
  look: square, keyboard first, with vim keys. **Settings › Keyboard**
  changes the keys.
  Read about [the Omarchy look]({% link _guide/omarchy.md %}) and
  [Tabletist on macOS]({% link _guide/macos.md %}).
- **Keeps itself up to date.** It checks GitHub for a new version. On
  macOS from Applications, and on Linux from the AppImage or the unpacked
  archive, it installs the new version when you restart; on Windows it
  links to the release.
  [See the download page]({% link _guide/download.md %}).

## What it does not do yet

Tabletist is young. These are the limits you are most likely to meet:

- You can change values in existing rows, and add, duplicate and delete
  rows. Pasting rows is not built yet.
- Binary values, values over 256 KiB, views, and tables without a primary
  key or unique index cannot be edited.
- The SQL editor does not run read-write on a production connection.
- SSH tunnels do not go through `ProxyJump` or `ProxyCommand`.
- The app is in English only.

## Open source

Tabletist is free and open source under the MIT license. The code, the
issues and the releases are on
[GitHub](https://github.com/igor-alexandrov/tabletist).
