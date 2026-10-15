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
| Ctrl+Shift+C | Open the connection picker. While you type in a field it copies what is selected instead |
| Ctrl+Shift+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
| Ctrl+Shift+W | Close the connection |
| Ctrl+T | New SQL editor |
| Alt+1 to 9 | Go to that tab |
| Ctrl+Shift+[ and ] | Previous and next tab |
| Ctrl+W | Close the tab. While you type in a field it deletes a word instead |
| Ctrl+B | Show or hide the sidebar |
| Ctrl+H, Ctrl+L | Move between the parts of the window |
| F6, Shift+F6 | Next and previous part of the window |
| Ctrl+, | Settings |
| `:agents` | Show the Agents tab |
| `?` | The list of keys |

Quick open has no key in the Omarchy look. `/` in the sidebar filters its
list instead.

### The picker

| Keys | Do |
| --- | --- |
| `j` `k`, arrows | Move |
| Enter | Connect, or show the connection if it is open already |
| Shift+Enter | Open it again |
| `n` | New connection |
| `e` | Edit |
| `yy` then `p` | Duplicate |
| `dd` | Delete |
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
| `/` | Filter the list |
| `t` | Switch between tree and flat |
| `R` | Reload the list |
| `ga` | Show the Agents tab |
| Ctrl+B | Hide the sidebar |

### A table

| Keys | Do |
| --- | --- |
| `j` `k` | Move by row |
| `h` `l` | Move by column |
| Arrows, Page Up, Page Down, Home, End | Move in the grid |
| Ctrl+Alt+Left and Right | Previous and next page |
| `/` | Filter with a WHERE line |
| Ctrl+F | Filter bar |
| `gd` | Follow the selected foreign key. On a row whose delete a foreign key refused, open the rows that hold it |
| `ga` | Show the Agents tab |
| `v` then `y` | Copy the cell |
| `R` | Reload |
| Ctrl+C | Cancel the running query |

The Structure and Data views are clicked: switching between them has no
key. Nor has copying a whole row.

### The Agents tab

| Keys | Do |
| --- | --- |
| `y` | Copy the command that connects an agent |
| `o` | Turn the agent server on, while it is off |
| `j` / `k` | Move among the agents' sessions: those here, then those elsewhere |
| `S` | Stop the selected session, wherever it works |
| `X` | Stop every agent here, and turn this connection's access off |
| Enter | Show the window of the connection the selected agent works on |

### The Structure view

Ctrl+L brings the keyboard to its columns, as it does to the rows.

| Keys | Do |
| --- | --- |
| `j` `k`, Up, Down | Move by column |
| `gl` | Use the column under the cursor as the label for foreign keys |
| `:label` | Choose that label from a list. In it, `j` `k` move, Enter chooses and Esc closes |
| `:label auto` | Back to the app's own pick |
| `:label none` | No label: rows are listed by their keys alone |
| `:label` and a column's name | Use that column |

The label is the column that the table's rows are read by in the list of a
foreign key that points at it:
[Editing Data]({% link _guide/editing-data.md %}#the-label-column) says
what it does. A key or a column of bytes is not taken, and the status line
says why.

### The row panel

| Keys | Do |
| --- | --- |
| Enter | Open the row panel, with the keyboard on its fields. Ctrl+L goes to it too |
| Ctrl+Shift+R | Show or hide the row panel |
| `j` `k` | Move between its fields |
| Enter, `i` | Edit the field |
| `[` `]` | Previous and next row, while the panel shows |
| `za` | Fold the documents in it |
| `cc`, `x`, `D`, `u` | Work on the field as they do on its cell |
| Ctrl+H | Back to the grid, on that field's cell |
| Esc | From its fields, back to the rows. From the rows, close it |

### Editing

| Keys | Do |
| --- | --- |
| `i` | Edit the cell |
| `cc`, `s` | Edit the cell from nothing |
| `o`, `O` | Add a row below the cursor's row, or above it |
| `yy` | Yank the cursor's row, and put it on the clipboard |
| `p`, `P` | Put a copy of the yanked row below the cursor's row, or above it |
| `dd` | Delete the row: mark it for deletion, or restore a marked row. A new row that is not saved yet is dropped |
| `3dd` | The same for the cursor's row and the two after it. Any count works, up to 1,000 rows. The cursor moves to the row after them |
| `V` | Start a selection of rows on the cursor's row. `j`, `k` and the other moves extend it. The status line reads `-- VISUAL --` and counts its rows |
| `d` | In a selection of rows: delete its rows, or restore them where all are marked. New rows among them are dropped |
| Esc, `V` | Leave the selection of rows |
| Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
| Esc | Leave the editor and keep the edit |
| Ctrl+C | Drop the edit |
| Ctrl+N, Ctrl+P, Down, Up, Page Down, Page Up | Move in a foreign key cell's list |
| Enter, Tab | In that list: set the cell to the highlighted row |
| Esc | In that list: close it and leave the cell as it was |
| Ctrl+L | In that list: choose the column that labels it. In the chooser, `j` `k`, Ctrl+N, Ctrl+P and the arrows move, Enter chooses and Esc closes |
| Alt+Enter | Open the large editor. In it, Ctrl+Enter applies, Esc keeps the text, Ctrl+C drops it and Ctrl+Shift+F formats JSON |
| `x` | Set NULL. On a column that cannot be NULL, the status line says so |
| `D` | Set DEFAULT, where the column has a default |
| Space | Flip a boolean cell: true, false, NULL |
| `u` | Revert the cell |
| `:w`, Ctrl+S | Save all pending changes |
| `:e!` | Discard all pending changes |
| `:diff` | Show the SQL of the pending changes |
| `Y` | Copy that SQL, while it shows |
| Esc | Close that SQL |

A date or time cell has no key for the time of the save: type `now()`
into it.

The `:` prompt takes `w`, `e!` and `diff` on a table, the `label` lines in
its Structure view, `ro` and `rw` in a SQL editor, and nothing else.

{% include shot.html file="omarchy-edit-review" alt="The :diff panel above the status line, showing the statement a save will run" %}

A save can ask you something first. Each question has its own keys:

| Question | Keys |
| --- | --- |
| A save to production | Type the database's name, then Enter. Where the connection names no database, its own name. Esc goes back |
| A row that changed on the server | `o` overwrite, `s` use the server's values, `k` or Esc keep mine and reload. PgUp and PgDn scroll |
| A row that was deleted on the server | `d` discard my changes, `k` or Esc keep them pending |
| Leaving with pending changes | `w` write, `d` discard, Esc stay |

{% include shot.html file="omarchy-edit-conflict" alt="The conflict box in the Omarchy look, with a letter for each answer" %}

### The SQL editor

| Keys | Do |
| --- | --- |
| Ctrl+Enter | Run the statement at the cursor |
| Ctrl+Shift+Enter | Run all |
| Ctrl+C | Cancel the run, while one runs. With none running it copies |
| Esc | Leave the editor: the letters below are read from then on |
| `=` | Format, with the keyboard out of the text |
| `:ro`, `:rw` | Read-only and read-write runs, with the keyboard out of the text |
| Ctrl+Space | Complete |

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
| `j` `k`, `h` `l` | Move in the result |
| Enter | Show the selected row in the row panel |
| Ctrl+Shift+R | Show or hide the row panel |
| `[` `]` | Previous and next row, while the panel shows |

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
