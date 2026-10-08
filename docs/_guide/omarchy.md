---
title: The Omarchy Look
description: How Tabletist looks and works on Linux, the keys of the Omarchy look, how it follows the Omarchy theme, and how to use your own colors.
nav_order: 6
---

On Linux, Tabletist takes the Omarchy look: square corners, the keyboard
first, vim keys, and the desktop's monospace font throughout. Every Linux
desktop gets it. On macOS and Windows the app has a desktop look instead,
and there is no setting that switches between them.

{% include shot.html file="omarchy-table" alt="Tabletist in its Omarchy look: a table filtered with a WHERE line, a row open beside it, and key hints along the bottom" width="2880" height="1800" %}

## The keyboard

This page lists the keys of the Omarchy look, grouped by where they
work. The status line along the bottom shows the ones that work where you
are. The app's own list (`?`) is shorter: it leaves out the keys of the
dialogs and of the lists that open in an editor.

A letter does its job when no text field has the keyboard. While a field
has it, letters are typed into the field, and Esc gives the keyboard
back.

### Everywhere

| Keys | Do |
| --- | --- |
| Ctrl+O | Open the connection picker |
| Ctrl+N | New connection |
| Ctrl+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
| Ctrl+Shift+W | Close the connection |
| Ctrl+P | Quick open |
| Ctrl+T | New SQL editor |
| `1` to `9` | Go to that tab |
| Ctrl+Shift+[ and ] | Previous and next tab |
| Ctrl+W | Close the tab |
| Ctrl+B | Show or hide the sidebar |
| Ctrl+H, Ctrl+L | Move between the parts of the window |
| F6, Shift+F6 | Next and previous part of the window |
| Ctrl+, | Settings |
| `?` | The list of keys |

### Quick open

| Keys | Do |
| --- | --- |
| Ctrl+P | Open it, then type to find a table or view |
| Up, Down | Move in the list |
| Enter | Open the one selected |
| Esc | Close |

### The picker

| Keys | Do |
| --- | --- |
| `j` `k`, arrows | Move |
| Enter | Connect, or show the connection if it is open already |
| Shift+Enter | Open it again |
| `n` | New connection |
| `e`, Ctrl+E | Edit |
| `yy`, Ctrl+D | Duplicate |
| `dd`, Ctrl+Backspace | Delete |
| `/` | Filter the list |
| Esc | Cancel connecting |

{% include shot.html file="omarchy-connections" alt="The connection picker in the Omarchy look, with its keys along the bottom" %}

### The connection form

| Keys | Do |
| --- | --- |
| Tab | Next field |
| `u` | Paste a URL |
| Ctrl+T | Test |
| Ctrl+S | Save |
| Ctrl+Enter | Save and connect |
| Esc | Cancel |

{% include shot.html file="omarchy-connection-edit" alt="The connection form in the Omarchy look, for a production database behind an SSH tunnel" width="2200" height="2032" %}

### The sidebar

| Keys | Do |
| --- | --- |
| `j` `k`, arrows, Home, End | Move |
| Enter | Open the table or view |
| `t` | Switch between tree and flat |
| Ctrl+B | Hide the sidebar |
| Ctrl+R | Refresh the list |

### A table

| Keys | Do |
| --- | --- |
| `j` `k` | Move by row |
| `h` `l` | Move by column |
| `[` `]` | Previous and next row |
| Arrows, Page Up, Page Down, Home, End | Move in the grid |
| Ctrl+Alt+Left and Right | Previous and next page |
| `/` | Filter with a WHERE line |
| Ctrl+F | Filter bar |
| `s`, `d` | Structure view, Data view |
| `gd` | Follow the selected foreign key |
| `y`, Ctrl+C | Copy the cell |
| Ctrl+Shift+C | Copy the row |
| Ctrl+R | Refresh |
| Ctrl+. | Cancel the running query |

### The row panel

| Keys | Do |
| --- | --- |
| Space, Ctrl+Shift+R | Show or hide the row panel |
| Esc | Close it |
| `[` `]` | Previous and next row |
| `za` | Fold the documents in it |
| `e`, Ctrl+I | Edit the row in it |

### Editing

| Keys | Do |
| --- | --- |
| `i`, Enter | Edit the cell |
| `cc` | Edit the cell from nothing |
| `o`, `O` | Add a row below the cursor's row, or above it |
| `dd` | Drop a new row that is not saved yet |
| `e`, Ctrl+I | Edit the row in the row panel |
| Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
| Esc | Leave the editor and keep the edit |
| Ctrl+C | Drop the edit |
| Alt+Enter | Open the large editor. In it, Ctrl+Enter applies and Esc keeps the text |
| `x` | Set NULL |
| `u` | Revert the cell |
| `:w`, Ctrl+S | Save all pending changes |
| `:e!` | Discard all pending changes |
| `:diff`, Ctrl+Shift+D | Show the SQL of the pending changes |
| `Y` | Copy that SQL, while it shows |
| Esc | Close that SQL |

The `:` prompt takes `w`, `e!` and `diff`, and nothing else.

{% include shot.html file="omarchy-edit-review" alt="The :diff panel above the status line, showing the statement a save will run" %}

A save can ask you something first. Each question has its own keys:

| Question | Keys |
| --- | --- |
| A save to production | Type `write` to confirm |
| A row that changed on the server | `o` overwrite, `s` use the server's values, `k` or Esc keep mine and reload. PgUp and PgDn scroll |
| A row that was deleted on the server | `d` discard my changes, `k` or Esc keep them pending |
| Leaving with pending changes | `w` write, `d` discard, Esc stay |

{% include shot.html file="omarchy-edit-conflict" alt="The conflict box in the Omarchy look, with a letter for each answer" %}

### The SQL editor

| Keys | Do |
| --- | --- |
| Ctrl+Enter | Run the statement at the cursor |
| Ctrl+Shift+Enter | Run all |
| Ctrl+. | Cancel the run |
| Ctrl+Shift+F | Format |
| Ctrl+Shift+M | Switch between read-only and read-write runs |
| Ctrl+Space, Ctrl+I | Complete |
| Esc | Leave the editor |

{% include shot.html file="omarchy-sql" alt="A SQL editor in the Omarchy look, with its result under it and the run keys in the status line" %}

While the completion list is open:

| Keys | Do |
| --- | --- |
| Up, Down, Ctrl+P, Ctrl+N | Move in the list |
| Tab | Accept |
| Enter | Accept, except when it stays a line break: [the SQL Editor guide says when]({% link _guide/sql-editor.md %}#completion) |
| Esc | Close the list |

On a result, with the keyboard out of the editor:

| Keys | Do |
| --- | --- |
| `j` `k`, `h` `l`, `[` `]` | Move in the result |
| Enter | Show the selected row in the row panel |
| `i`, Space | Show or hide the row panel |

When a read-only run refused a write, the card in Messages has its own
letters: `e` edits the connection, and `w` allows writes in this tab or
runs the statements again in a read-write transaction, whichever the card
offers.

### Settings

| Keys | Do |
| --- | --- |
| `j` `k` | Move |
| `h` `l` | Change the option |
| Space | Toggle the option |
| Ctrl+E | Open `settings.toml` in your editor |
| `R` | Reset the option |
| Esc | Close |

The Mac's keys are in [Tabletist on macOS]({% link _guide/macos.md %}#the-keyboard),
and the keys of the macOS and Windows looks together in
[Keyboard Shortcuts]({% link _reference/keyboard-shortcuts.md %}).

## Themes

On Omarchy, Tabletist follows the current theme and recolors when you
switch themes, while it runs. On another Linux desktop it follows the
system's light or dark setting.

Tabletist maps an Omarchy theme onto its own palette with a template. The
one it ships is
[`contrib/omarchy/tabletist.json.tpl`](https://github.com/igor-alexandrov/tabletist/blob/main/contrib/omarchy/tabletist.json.tpl)
in the source.

### Your own colors

A palette file in the `themes` folder of Tabletist's settings folder can
replace the desktop's colors. Name the file in `settings.toml`:

```toml
[appearance]
theme = "my-theme.json"
```

A named file wins over the Omarchy theme and over the system's light or
dark setting. The folder and the file are described in
[Settings and Files]({% link _reference/settings-and-files.md %}).

## Settings

Cmd/Ctrl+, opens Settings. In the Omarchy look it is a full screen with
`settings.toml` shown beside the options. Edits made to the file in your
editor are applied while Tabletist runs.

{% include shot.html file="omarchy-settings" alt="The Settings screen in the Omarchy look: the options on the left and settings.toml on the right" width="2880" height="1800" %}
