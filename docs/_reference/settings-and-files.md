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

A second pane, **Agents**, sets what the `[mcp]` keys
hold, and shows how to connect an agent:
[Agents]({% link _guide/agents.md %}) is the guide.

| Option | Choices | Default |
| --- | --- | --- |
| **Let agents use Tabletist** | On or off. Off ends every agent's session | Off |
| **Keep the activity log** | 7, 30, 90 or 365 days | 30 days |
| **Notify when an agent connects** | On or off | On |

On macOS and Windows the window shows where the settings file is, and can
reveal it in your file manager, export it, or reset the options of the
pane it shows. On the Agents pane, a reset leaves the server's switch as
it is. In the Omarchy look the file is shown beside the options.

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

[mcp]
enabled = false
log_days = 30
notify = true
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
| `mcp.enabled` | `true` lets agents reach the connections you open to them, through `tabletist mcp` | `false` |
| `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
| `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |

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
| `connections.moved` | State | A note that your connections are kept in `connections.toml` |
| `labels.json` | State | The column you chose to label each table's rows in a foreign key's list, by saved connection and, on PostgreSQL, by database |
| `tabletist.log`, `panic.log` | State | Logs |
| `agents.sqlite3` | State | Every call agents made: the tool, the statement and its values, and how it ended. Never a row |
| `agents_shown.json` | State | What agents may do as Tabletist last showed it: whether the server is on, and each connection's access |
| `mcp.sock`, `mcp.key`, `mcp.lock` | Runtime, else State | Where `tabletist mcp` reaches the app, the key each proves it holds before either says more, and the lock the app holds while it listens. The socket and the key are there while agents are on |
| `mcp.pipe` | State | On Windows, in the socket's place: the name of the pipe `tabletist mcp` reaches the app by, new each time agents are turned on |

The runtime folder is `/run/user/<your user id>/tabletist` on Linux, where
the system makes `/run/user` for you, and `~/.local/state/tabletist` where
it does not, whatever `XDG_STATE_HOME` says: an agent may start
`tabletist mcp` without your variables. macOS has none, and uses the state
folder, or `.tabletist` in your home folder when the state folder's path is
too long for a socket. Windows has none either, and uses the state folder:
the app listens on a named pipe, whose name is new each time agents are
turned on, and writes the name there for `tabletist mcp` to find.

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

What agents may do on a connection is its `[connections.agents]` table:
`access` (`off` or `read`), `rows`, the most rows of an answer (1 to 5000),
and `timeout_secs`, how long a statement may run (1 to 60). Without the
table, or a key of it, a connection is `off`, with 500 rows and 10 seconds.

The table can also say which tables agents may not read, with one of two
lists:

```toml
[connections.agents]
access = "read"
hidden = ["customers", "sales.orders"]
```

`hidden` names the tables agents may not read, and every other one they
may. `only` names the tables they may read, and no others, the ones made
later included: `only = []` hides every table. A name without a schema is
that table in every schema, and a name with a dot or a double quote in it
is written in double quotes, with the quote doubled: `sales."q1.orders"`.
A view over a hidden table is hidden with it, and so are a partition and a
child table of one. While anything is hidden, agents may not read the
system catalogs either (`information_schema`, `pg_catalog`, `mysql`,
`sqlite_schema` and the rest). If a table that `hidden` names is renamed
or dropped, agents may not use the connection at all until you take the
name out of the list or put the right one in.

A file in which a connection hides a table says `version = 5`, one in
which a connection has the table and hides nothing says `version = 4`,
and one in which none has it stays `version = 3`. Agents use the table only while
`[mcp] enabled` is `true` in `settings.toml`. **Stop agents here**, in a
connection's Agents tab, stops the agents working on it and sets its
`access` to `off`, so that an agent that connects again does not see it.

### Agent access changed outside Tabletist

A change to either file that lets agents do more than Tabletist last
showed you is not applied at once: turning `[mcp] enabled` on, a
connection's `access` up from `off`, or a connection hiding less: a name
taken out of `hidden` or put into `only`, or a `hidden` list made an
`only` one. Tabletist says what
changed in a bar under the top of the window, and until you answer,
agents may do what they could before. **Keep** applies the change. **Put
back** undoes it in the files: it turns `[mcp] enabled` off, sets the
connection's `access` back to `off`, or puts its tables back as they were,
and leaves your other settings and connections as they are. A change that
lets agents do less applies at once.

Turning agents on in Tabletist itself, with **Turn on** in a connection's
Agents tab or the switch in **Settings › Agents**, is never held. Tabletist keeps what it showed you in
`agents_shown.json`. Without that file, the first time a version with this
check starts, whatever you turned on by hand is held once.

Edit the file while Tabletist is closed. Tabletist writes the whole file
again when you change a connection, so a comment you add is not kept.

Tabletist reads the file strictly. A key it does not know, or a value a key
cannot have, is an error and not something it skips: a line such as
`read_onyl = true` must not pass for a connection that may write. When the
file cannot be read, Tabletist starts with no connections, says which line
is wrong, and keeps the file beside itself as `connections.toml.bad` (or
`.bad.1` when that name is taken: the notice says which). Fix the line and
rename the file back. When each connection last connected is kept. What
Tabletist says of the line never repeats a value written on it.

Until you rename the file back or remove it, every start says that it is
there, and loads no connection.

Versions before this one kept the connections in `connections.json`.
Tabletist reads that file once, at the first start that finds no
`connections.toml`, writes the two files above from it, tells you, and
leaves it where it is. If the old file itself cannot be read, it is kept
aside in the same way and nothing is written for it, so it is read again
once it is mended.

At the next start, which finds `connections.toml`, Tabletist notes in its
state folder that the connections have moved (`connections.moved`). From
then on `connections.json` is not read again, whatever becomes of the new
file: the connections of the day they moved do not come back in place of a
`connections.toml` that was kept aside or removed. To have the old file
read once more, remove `connections.moved` as well.

## Command-line options

| Option | Does |
| --- | --- |
| `--demo` | Runs with sample data in a throwaway profile |
| `--verbose` | Logs at debug level |
| `--version` | Prints the version |
| `--help` | Lists the options |
| `mcp` | Connects an agent to the running Tabletist, over its stdin and stdout. An agent starts it. When Tabletist is not running, or agents are off in it, it says so to the agent and ends |

`--help` also lists `--demo-shot`, which saves a picture of the demo's
window to a file, and `--demo-size`, which sets that window's size.
