---
title: Settings and Files
description: The Settings window, every key of settings.toml, where Tabletist keeps its files, and its command-line options.
nav_order: 2
---

## The Settings window

Cmd/Ctrl+, opens Settings. Changes apply right away.

| Option | Choices | Default |
| --- | --- | --- |
| **Rows per page** | 100, 300, 500, 1,000 or 5,000 | 300 |
| **Timestamps** | To the second, or full precision | To the second |
| **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
| **Value tags** | Colors for enum, CHECK and boolean columns | On |

On macOS and Windows the window shows where the settings file is, and can
reveal it in your file manager, export it, or reset every option. In the
Omarchy look the file is shown beside the options.

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
| `connections.json` | Settings | Your saved connections. It never holds a password |
| `known_hosts.json` | Settings | The SSH host keys you trusted |
| `themes/` | Settings | Palette files |
| `tabletist.log`, `panic.log` | State | Logs |

Passwords are kept in the system keyring, never in these files.
[Read about passwords]({% link _reference/connections.md %}#passwords).

## Command-line options

| Option | Does |
| --- | --- |
| `--demo` | Runs with sample data in a throwaway profile |
| `--verbose` | Logs at debug level |
| `--version` | Prints the version |
| `--help` | Lists the options |

`--help` also lists `--demo-shot`, which saves a picture of the demo's
window to a file, and `--demo-size`, which sets that window's size.
