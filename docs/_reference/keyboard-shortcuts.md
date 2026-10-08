---
title: Keyboard Shortcuts
description: The keys of the macOS and Windows looks, and where the Omarchy look's keys are.
nav_order: 1
---

Press `?` in the app for a shorter version of this list, with the keys of
the look you are using. It leaves out the keys of the dialogs and of the
lists that open in an editor, which are here.

In this page, **Mod** is Cmd on macOS and Ctrl on Windows and Linux. Keys
written with Ctrl are Ctrl on every system.

The same keys are written out for one system in
[Tabletist on macOS]({% link _guide/macos.md %}#the-keyboard) and
[The Omarchy Look]({% link _guide/omarchy.md %}#the-keyboard).

## Connections

| Keys | Do |
| --- | --- |
| Mod+O | Open the connection picker |
| Mod+N | New connection. With a table's rows in front, [a new row]({% link _guide/editing-data.md %}#add-a-row) |
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
| Esc | Cancel |

## The window

| Keys | Do |
| --- | --- |
| Mod+P | Quick open. In it, Up and Down move, Enter opens and Esc closes |
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

These are the keys of the macOS and Windows looks. The Omarchy look has
[its own]({% link _guide/omarchy.md %}#editing).

| Keys | Do |
| --- | --- |
| Enter, F2 | Edit the cell. Typing on a selected cell starts an edit too, and so does a double-click |
| Mod+N | Add a row, at the top of the grid |
| Delete | Drop a new row that is not saved yet |
| Mod+I | Edit the row in the row panel |
| Enter, Tab, Shift+Tab | Keep the edit and move down, right or left |
| Alt+Enter | Open the large editor. In it, Mod+Enter applies and Esc cancels |
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
| Up, Down | Move in the completion list |
| Tab | Accept the completion |
| Enter | Accept the completion, except when it stays a line break: [the SQL Editor guide says when]({% link _guide/sql-editor.md %}#completion) |
| Esc | Close the completion list |

## The Omarchy look

The Omarchy look keeps every key above except the editing ones, and adds
vim keys, a `:` prompt and letters for its dialogs. All of them are listed
in [The Omarchy Look]({% link _guide/omarchy.md %}#the-keyboard).
