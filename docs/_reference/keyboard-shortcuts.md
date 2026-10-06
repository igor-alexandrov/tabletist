---
title: Keyboard Shortcuts
description: Every key in Tabletist, for the desktop looks and for the Omarchy look.
nav_order: 1
---

Press `?` in the app for this list. It shows the keys of the look you are
using.

In this page, **Mod** is Cmd on macOS and Ctrl on Windows and Linux. Keys
written with Ctrl are Ctrl on every system.

## Connections

| Keys | Do |
| --- | --- |
| Mod+O | Open the connection picker |
| Mod+N | New connection |
| Mod+1 to 9, Ctrl+Tab, Ctrl+Shift+Tab | Switch connection |
| Mod+Shift+W | Close the connection |
| Esc | Cancel connecting |

In the picker:

| Keys | Do |
| --- | --- |
| Arrows, Enter | Pick a connection |
| Shift+Enter | Open it again |
| Mod+E | Edit |
| Mod+D | Duplicate |
| Mod+Backspace | Delete |

In the connection dialog:

| Keys | Do |
| --- | --- |
| Mod+S | Save |
| Mod+T | Test |
| Mod+Enter | Save and connect |

## The window

| Keys | Do |
| --- | --- |
| Mod+P | Quick open |
| Mod+B | Show or hide the sidebar |
| Mod+W | Close the tab |
| Mod+Shift+[ and ] | Previous and next tab |
| F6, Shift+F6 | Next and previous part of the window |
| Mod+, | Settings |
| ? | This list |

## Tables

| Keys | Do |
| --- | --- |
| Arrows, Home, End, Enter | Move in the sidebar |
| Arrows, Page Up, Page Down, Home, End | Move in the grid |
| Mod+Alt+Left and Right | Previous and next page |
| Mod+F | Filter bar |
| Mod+R | Refresh |
| Mod+. | Cancel the running query |
| Space, Mod+Shift+R | Show or hide the row panel |
| Mod+C | Copy the cell |
| Mod+Shift+C | Copy the row |

## Editing

These are the keys of the desktop looks. The Omarchy look has
[its own](#the-omarchy-look).

| Keys | Do |
| --- | --- |
| Enter, F2 | Edit the cell |
| Mod+I | Edit the row in the row panel |
| Tab, Shift+Tab | Keep the edit and move right or left |
| Esc | Cancel the edit |
| Mod+Backspace | Set NULL |
| Mod+Z | Revert the cell |
| Mod+S | Save all pending changes |
| Mod+Alt+Backspace | Discard all pending changes |
| Mod+Shift+D | Show or hide the SQL of the pending changes |

## SQL editor

| Keys | Do |
| --- | --- |
| Mod+T | New SQL editor |
| Mod+Return | Run the statement at the cursor |
| Mod+Shift+Return | Run all |
| Mod+. | Cancel the run |
| Mod+Shift+F | Format |
| Mod+Shift+M | Switch between read-only and read-write runs |
| Ctrl+Space, Mod+I | Complete |

## The Omarchy look

The Omarchy look keeps every key above except the editing ones, and adds
vim keys. The status line shows the ones that work where you are.

| Keys | Do |
| --- | --- |
| `j` `k` | Move by row |
| `h` `l` | Move by column |
| `[` `]` | Previous and next row |
| Ctrl+H, Ctrl+L | Move between the parts of the window |
| `1` to `9` | Go to that tab |
| `t` | Switch the sidebar between tree and flat |
| `/` | Filter with a WHERE line |
| `s`, `d` | Structure view, Data view |
| Space | Show or hide the row panel |
| `gd` | Follow the selected foreign key |
| `za` | Fold a document in the row panel |
| `y` | Copy the cell |

Editing:

| Keys | Do |
| --- | --- |
| `i`, Enter | Edit the cell |
| `cc` | Edit the cell from nothing |
| `e`, Mod+I | Edit the row in the row panel |
| Tab, Shift+Tab | Keep the edit and move right or left |
| Esc | Leave the editor and keep the edit |
| Ctrl+C | Drop the edit |
| `x` | Set NULL |
| `u` | Revert the cell |
| `:w`, Mod+S | Save all pending changes |
| `:e!` | Discard all pending changes |
| `:diff`, Mod+Shift+D | Show the SQL of the pending changes |
| `Y` | Copy the SQL of the pending changes |

In the picker, `j` and `k` move, `n` is a new connection, `e` edits, `yy`
duplicates, `dd` deletes and `/` filters.
