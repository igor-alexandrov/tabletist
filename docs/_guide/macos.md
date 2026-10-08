---
title: Tabletist on macOS
description: How Tabletist looks and works on a Mac, the keys of the macOS look, the Keychain, and where its files are.
nav_order: 7
---

On a Mac, Tabletist has the macOS look: rounded controls, IBM Plex Sans
for the interface and IBM Plex Mono for data. There is no setting that
switches to another look.

{% include shot.html file="macos-table" alt="Tabletist in its macOS look, with two connections in the bar at the top" %}

## Install

Tabletist needs macOS 11 or later. One download runs on both Apple
Silicon and Intel. The steps, and what to do when macOS blocks the first
launch, are on the
[Download page]({% link _guide/download.md %}#macos).

## The window

- **The title bar holds your connections.** There is no separate title:
  the chips of the open connections sit beside the window's buttons, as
  tabs do in Safari. In full screen the window's buttons are hidden and
  the bar takes the whole width.
- **Light and dark follow the system.** Tabletist changes with the
  Appearance setting while it runs.
  [Your own colors]({% link _guide/omarchy.md %}#your-own-colors) work on a
  Mac too.
- **The app menu has the usual two.** **Tabletist → About Tabletist**
  shows the version, and **Tabletist → Settings…** opens Settings.

{% include shot.html file="macos-table-dark" alt="The same window in the dark appearance" %}

## The keyboard

This page lists the keys of the macOS look, grouped by where they work.
The app's own list (`?`) is shorter: it leaves out the keys of the
dialogs and of the lists that open in an editor.

Keys written with Ctrl are the Control key, not Cmd. Backspace is the key
a Mac keyboard labels Delete.

### Everywhere

| Keys | Do |
| --- | --- |
| Cmd+O | Open the connection picker |
| Cmd+N | New connection. With a table's rows in front, a new row |
| Cmd+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
| Cmd+Shift+W | Close the connection |
| Cmd+P | Quick open |
| Cmd+T | New SQL editor |
| Cmd+Shift+[ and ] | Previous and next tab |
| Cmd+W | Close the tab |
| Cmd+B | Show or hide the sidebar |
| F6, Shift+F6 | Next and previous part of the window |
| Cmd+, | Settings |
| `?` | The list of keys |

### Quick open

| Keys | Do |
| --- | --- |
| Cmd+P | Open it, then type to find a table or view |
| Up, Down | Move in the list |
| Return | Open the one selected |
| Esc | Close |

### The picker

| Keys | Do |
| --- | --- |
| Arrows | Move |
| Return | Connect, or show the connection if it is open already |
| Shift+Return | Open it again |
| Cmd+E | Edit |
| Cmd+D | Duplicate |
| Cmd+Backspace | Delete |
| Esc | Cancel connecting |

### The connection dialog

| Keys | Do |
| --- | --- |
| Cmd+T | Test |
| Cmd+S | Save |
| Cmd+Return | Save and connect |
| Esc | Cancel |

### The sidebar

| Keys | Do |
| --- | --- |
| Arrows, Home, End | Move |
| Return | Open the table or view |
| Cmd+R | Refresh the list |

### A table

| Keys | Do |
| --- | --- |
| Arrows, Page Up, Page Down, Home, End | Move in the grid |
| Cmd+Option+Left and Right | Previous and next page |
| Cmd+F | Filter bar |
| Cmd+C | Copy the cell |
| Cmd+Shift+C | Copy the row |
| Cmd+R | Refresh |
| Cmd+. | Cancel the running query |
| Space, Cmd+Shift+R | Show or hide the row panel. On a boolean cell that can be edited, Space flips the cell instead |

### Editing

| Keys | Do |
| --- | --- |
| Return, F2 | Edit the cell. Typing on a selected cell starts an edit too, and so does a double-click |
| Cmd+N | Add a row, at the top of the grid |
| Backspace, Forward Delete | Drop a new row that is not saved yet |
| Cmd+I | Edit the row in the row panel |
| Return, Tab, Shift+Tab | Keep the edit and move down, right or left |
| Esc | Cancel the edit |
| Option+Return | Open the large editor. In it, Cmd+Return applies and Esc cancels |
| Cmd+Backspace | Set NULL |
| Cmd+' | Set DEFAULT, where the column has a default |
| Space | Flip a boolean cell: true, false, NULL |
| Cmd+Z | Revert the cell |
| Cmd+S | Save all pending changes |
| Cmd+Option+Backspace | Discard all pending changes |
| Cmd+Shift+D | Show or hide the SQL of the pending changes |

### The SQL editor

| Keys | Do |
| --- | --- |
| Cmd+Return | Run the statement at the cursor |
| Cmd+Shift+Return | Run all |
| Cmd+. | Cancel the run |
| Cmd+Shift+F | Format |
| Cmd+Shift+M | Switch between read-only and read-write runs |
| Cmd+I, Ctrl+Space | Complete |

While the completion list is open, Up and Down move in it, Tab accepts
and Esc closes it. Return accepts too, except when it stays a line break:
[the SQL Editor guide says when]({% link _guide/sql-editor.md %}#completion).

macOS may keep Ctrl+Space for switching input sources. Cmd+I always
reaches Tabletist.

## The Keychain

A password field in the connection dialog has a **Keychain** checkbox.
Ticked, the password is kept in your keychain under the name
`dev.tabletist.Tabletist`. Not ticked, Tabletist asks for the password
when you connect.

{% include shot.html file="macos-connection-edit" alt="The connection dialog, with a Keychain checkbox beside the password and beside the key's passphrase" width="2880" height="1800" %}

[Read about passwords]({% link _reference/connections.md %}#passwords).

## SSH agents

For an SSH tunnel that signs in with your agent, Tabletist uses the agent
`~/.ssh/config` names for the host with `IdentityAgent`, which is how
1Password's agent is found. Without one it uses `SSH_AUTH_SOCK`.
[Read about SSH tunnels]({% link _reference/connections.md %}#ssh-tunnels).

## Files

Everything Tabletist keeps is in one folder:

```text
~/Library/Application Support/dev.tabletist.Tabletist
```

It holds `settings.toml`, `connections.json`, `known_hosts.json`, the
`themes` folder and the logs. **Settings** shows where the settings file
is, and **Reveal in Finder** takes you there.

{% include shot.html file="macos-settings" alt="The Settings window, with the path of the settings file at the bottom" %}

[Every file is described in Settings and Files]({% link _reference/settings-and-files.md %}#files).
