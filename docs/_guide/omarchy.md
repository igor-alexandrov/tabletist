---
title: The Omarchy Look
description: How Tabletist looks and works on Linux, how it follows the Omarchy theme, and how to use your own colors.
nav_order: 6
---

On Linux, Tabletist takes the Omarchy look: square corners, the keyboard
first, vim keys, and the desktop's monospace font throughout. Every Linux
desktop gets it. On macOS and Windows the app has a desktop look instead,
and there is no setting that switches between them.

![Tabletist in its Omarchy look, with key hints along the bottom]({{ '/assets/images/omarchy.png' | relative_url }})

## The keyboard

The status line along the bottom shows the keys that work where you are.
The ones you will use most:

| Keys | Do |
| --- | --- |
| `j` `k` | Move by row |
| `h` `l` | Move by column |
| `/` | Filter with a WHERE line |
| `s`, `d` | Structure view, Data view |
| Space | Show or hide the row panel |
| `gd` | Follow the selected foreign key |
| `y` | Copy the cell |
| `i` | Edit the cell |
| `:w` | Save all pending changes |
| `:diff` | Show the SQL of the pending changes |
| `:e!` | Discard all pending changes |
| `1` to `9` | Go to that tab |

The whole list is in
[Keyboard Shortcuts]({% link _reference/keyboard-shortcuts.md %}#the-omarchy-look).

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
