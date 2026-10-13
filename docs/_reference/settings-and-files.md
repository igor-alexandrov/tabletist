---
title: Settings and Files
description: The Settings window, every key of settings.toml and of connections.toml, where Tabletist keeps its files, and its command-line options.
nav_order: 2
---

## The Settings window

Cmd/Ctrl+, opens Settings. Changes apply right away.

{% include shot.html file="macos-settings" alt="The Settings window with its four options" %}

| Option | Choices | Default |
| --- | --- | --- |
| **Rows per page** | 100, 300, 500, 1,000 or 5,000 | 300 |
| **Timestamps** | To the second, or full precision | To the second |
| **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
| **Value tags** | Colors for enum, CHECK and boolean columns | On |

On macOS and Windows the window shows where the settings file is, and can
reveal it in your file manager, export it, or reset every option. In the
Omarchy look the file is shown beside the options.

{% include shot.html file="omarchy-settings" alt="The Settings screen in the Omarchy look, with settings.toml beside the options" width="2880" height="1800" %}

## The settings file

Settings are kept in `settings.toml`, which is safe to edit by hand.
Tabletist watches the file and applies your edits while it runs.

```toml
[data]
page_size = 300
timestamps = "second"
group_digits = false
value_tags = true

[sidebar]
show_system_schemas = false

[editor]
sql_limit = 1000
sql_timeout_secs = 30
```

| Key | Values | Default |
| --- | --- | --- |
| `data.page_size` | 10 to 10000 | `300` |
| `data.timestamps` | `"second"` or `"full"` | `"second"` |
| `data.group_digits` | `true` or `false` | `false` |
| `data.value_tags` | `true` or `false` | `true` |
| `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
| `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
| `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
| `appearance.theme` | The name of a palette file in the `themes` folder | Not set |

`show_system_schemas` and `theme` are set only in the file. The SQL
editor's **Limit** and **Timeout** menus set the two `editor` keys.

A line that cannot be read is ignored, keeps its default, and is reported
in the Settings window. When you change an option in the app, the file is
written again in a standard form, which drops comments you added.

## Files

Tabletist keeps settings and state in two folders.

| System | Settings | State |
| --- | --- | --- |
| Linux | `~/.config/tabletist` | `~/.local/state/tabletist` |
| macOS | `~/Library/Application Support/dev.tabletist.Tabletist` | The same folder |
| Windows | `%APPDATA%\tabletist\Tabletist\config` | `%LOCALAPPDATA%\tabletist\Tabletist\data` |

On Linux, `XDG_CONFIG_HOME` and `XDG_STATE_HOME` move the two folders.

| File | Folder | Holds |
| --- | --- | --- |
| `settings.toml` | Settings | The options above |
| `connections.toml` | Settings | Your saved connections. It never holds a password |
| `known_hosts.json` | Settings | The SSH host keys you trusted |
| `themes/` | Settings | Palette files |
| `last_used.json` | State | When each saved connection last connected |
| `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
| `tabletist.log`, `panic.log` | State | Logs |

Passwords are kept in the system keyring, never in these files.
[Read about passwords]({% link _reference/connections.md %}#passwords).

## The connections file

Saved connections are kept in `connections.toml`, one `[[connections]]`
table for each.

```toml
version = 3

[[connections]]
id          = "6a0c1f0e-5a52-4b7b-9c56-3f0c8f2f7a11"
name        = "Bookshop production"
environment = "production"
read_only   = true
password    = "keyring"
ssh_secret  = "ask"
driver      = "postgres"
host        = "db.example.com"
port        = 5432
user        = "app"
database    = "bookshop_production"
tls         = "verify-full"
ca_file     = "/etc/ssl/bookshop-ca.pem"

[connections.ssh]
host = "bastion.example.com"
port = 2222
user = "deploy"
auth = "key-file"
key  = "/home/ada/.ssh/id_ed25519"

[[connections]]
name        = "Bookshop on disk"
environment = "local"
driver      = "sqlite"
sqlite_path = "/home/ada/bookshop.db"
```

| Key | Values | Without it |
| --- | --- | --- |
| `id` | Any text that no other connection has | Tabletist gives the connection one at the next start |
| `name` | Text | Required |
| `environment` | `local`, `dev`, `staging`, `production`, `none` | `none` |
| `read_only` | `true`, `false` | As the environment has it: read-only on production |
| `password` | `none`, `keyring`, `ask` | `none` |
| `ssh_secret` | `none`, `keyring`, `ask` | `none` |
| `driver` | `postgres`, `mysql`, `sqlite` | Required |
| `host`, `user`, `database` | Text | Empty |
| `port` | A number up to 65535 | 5432 for PostgreSQL, 3306 for MySQL |
| `tls` | `disable`, `prefer`, `require`, `verify-ca`, `verify-full` | `prefer` |
| `ca_file` | The path of a certificate file | None |
| `sqlite_path` | The path of a SQLite file | None |

A tunnel is the connection's `[connections.ssh]` table: `host`, and
optionally `port` and `user`, `auth` (`password`, `key-file` or `agent`),
and with `key-file` the `key` to use.

Edit the file while Tabletist is closed. Tabletist writes the whole file
again when you change a connection, so a comment you add is not kept.

Tabletist reads the file strictly. A key it does not know, or a value a key
cannot have, is an error and not something it skips: a line such as
`read_onyl = true` must not pass for a connection that may write. When the
file cannot be read, Tabletist starts with no connections, says which line
is wrong, and keeps the file beside itself as `connections.toml.bad`. Fix
the line and rename the file back.

Versions before this one kept the connections in `connections.json`.
Tabletist reads that file once, at the first start that finds no
`connections.toml`, writes the two files above from it, and leaves it
where it is.

## Command-line options

| Option | Does |
| --- | --- |
| `--demo` | Runs with sample data in a throwaway profile |
| `--verbose` | Logs at debug level |
| `--version` | Prints the version |
| `--help` | Lists the options |

`--help` also lists `--demo-shot`, which saves a picture of the demo's
window to a file, and `--demo-size`, which sets that window's size.
