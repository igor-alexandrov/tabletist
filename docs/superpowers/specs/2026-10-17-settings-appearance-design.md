# Settings: the Appearance tab

Date: 2026-10-17. Status: for planning. The first five decisions below
were put to the user and answered on 2026-10-17; the last three are the
prompt's stated defaults or this spec's, named in the plan for a yes.

## Intent

Tabletist draws its grid, its header and its theme one way for each look,
with nothing to change. This adds the Appearance tab of Settings, in both
looks, in the same `settings.toml` that General writes: the theme, the
text, how dense and how striped the grid is, whether the header shows
column types, how the table list starts, and how strongly an environment
colours the connection bar.

The designs are the "Settings · Appearance" boards of the design canvas
Artifact (`macOS – Settings, Appearance`, `Omarchy – Settings, Appearance`),
with the Components, Dark mode, Table view and Two connections boards for
reference. They are not copied into the repository.

Success: a user opens Settings, picks Appearance, and every open window
follows each change at once: denser rows with the same top row, stripes,
a header of one line, a flat table list, a connection bar with only its
stripe. On Omarchy the change shows in the file pane beside the options,
and an edit in `$EDITOR` comes back live.

## Decisions

| Question | Decision |
|---|---|
| Writing the file | Today's writer: every key, in the app's fixed form, as General and both Appearance boards have it. The prompt asked for non-default values only and comments kept; that would change General's and Agents' file too. Not done. |
| Delivery | One plan for the tab in four steps, each its own pull request: 1 Grid and Environment, 2 Theme, 3 Text and the font size keys, 4 Preview. |
| Font size keys | egui's own zoom (Cmd/Ctrl + `=` `-` `0`, the whole window) is turned off. The keys change `appearance.font_size` through the keymap and write it (step 3). |
| macOS data font menu | The bundled fonts first (IBM Plex Mono, JetBrains Mono). Installed fonts through CoreText come later, in `tabletist-appkit`. |
| Table list per connection | A state file by connection id (`table_lists.json`, beside `last_used.json`), not `connections.toml`: that file refuses unknown keys, and a new key would make an older Tabletist refuse it whole. |
| Alternate rows default | The prompt's default, as drawn: off on macOS, on on Omarchy (open question 1). `zebra` is written once it is set; a file without it takes the look's default. |
| Interface size on Omarchy | The prompt's default: none, one font and size for everything (open question 2). |
| Row heights | This spec's: Regular keeps today's rows (37 pt on macOS, the board's 36 and its 1 pt rule; 30 on Omarchy, 1.5 lines). Compact and Roomy are 8 pt either side on macOS (the board's 28 and 44, with the rule), and 1 and 2 lines (20, 40) on Omarchy. Nobody's grid changes on upgrade. |

## Where the prompt, the boards and the app differ

| What is asked | What is built | Why |
|---|---|---|
| Only non-default values written, comments and unknown keys kept | Today's canonical writer | The user's choice (above). |
| The table list choice "stored with that connection" | A state file by connection id | The user's choice (above). |
| Data font: a menu of installed monospace fonts | The two bundled monospace fonts (step 3) | The user's choice (above). |
| The macOS board's subtitle, "Theme, type and how dense the grid is. The preview updates as you change things." | Shown from step 4, when there is a preview | Until then it says what the pane does not have. Step 1 draws the title alone. |
| The Omarchy board's file pane writes every key, `zebra` among them | `zebra` is written once it is set | Its default depends on the look and the writer does not know the look. |
| Every section of the boards at once | Grid and Environment in step 1, the rest in steps 2 to 4 | Delivery in steps (above). |

## The file

Step 1 adds two tables under `[appearance]`. `appearance.theme` stays what
it is today (a palette file name) until step 2.

    [appearance.grid]
    row_height   = "regular"  # compact | regular | roomy
    zebra        = true
    column_types = true
    table_list   = "tree"  # tree | flat

    [appearance.env]
    bar = "tint"  # tint | label

| Key | Type | Default | Notes |
|---|---|---|---|
| `appearance.grid.row_height` | `"compact"`, `"regular"`, `"roomy"` | `"regular"` | |
| `appearance.grid.zebra` | boolean | absent | Absent is the look's default: off on macOS, on on Omarchy. |
| `appearance.grid.column_types` | boolean | true | The header's second line. |
| `appearance.grid.table_list` | `"tree"`, `"flat"` | `"tree"` | The default for a connection with no choice of its own. |
| `appearance.env.bar` | `"tint"`, `"label"` | `"tint"` | Production is tinted whatever it says. |

Reading follows General's rules: a value the key cannot have is ignored by
its line and the key keeps its default; an unknown key is ignored without
a word. Keys are found in nested tables by their dotted path.

## Step 1: Grid and Environment

### The pane

`SettingsPane::Appearance` comes between General and Agents in both looks.

macOS, from the board, with its words:

- **Grid**
  - Row density: segments Compact · 28, Regular · 36, Roomy · 44.
  - Alternate row shading: a switch.
  - Show column types: a switch. "Second line in headers: int8, varchar, jsonb".
  - Table list: segments Grouped by prefix, Flat. "Default for new connections; the sidebar toggle overrides it".
- **Environment color**
  - Window header: segments Tint + stripe, Stripe only. "Production always uses the full tint".

Omarchy, from the board, with its words:

- **grid**
  - row height: compact, regular, roomy. "1 · 1.5 · 2 lines".
  - alternate rows: `[x]`.
  - column types: `[x]`. "second header line".
  - table list: tree, flat. "default; t toggles".
- **environment**
  - connection bar: tint + label, label only. "PROD always tinted".

The keys are General's: `j/k` move, `h/l` change, `space` toggles,
`ctrl+e` opens the file, `R` resets the option, `ctrl+h/l` move between
panes, `esc` closes. Reset to defaults on macOS resets the pane's options.
The prompt asks for segmented controls as radio groups and every control
described by its helper text. General's and Agents' controls are neither
today (the Omarchy choices are radio buttons, the macOS segments buttons
marked selected, and nothing has a description), so both are left to one
change of the shared widgets for every pane, after this step.

### What each setting does

- **Row density** sets the row height of the table grid and of the SQL
  editor's results. The grid keeps the same top row: the scroll offset is
  carried from the old height to the new.
- **Alternate rows** shade odd rows in the panel colour, in both grids,
  in both looks. The selected and hovered rows keep their fills.
- **Column types** off draws the header with the name alone, one line high
  (31 pt on macOS, 27 on Omarchy, each with its rule).
- **Table list** is the sidebar's starting form for a connection with no
  choice of its own. The sidebar toggle (Omarchy `t`) flips the open
  connection's list and saves the choice for it, by id, in
  `table_lists.json` in the state directory. A change of the setting moves
  every open connection that has no choice of its own. Deleting a
  connection forgets its choice.
- **Environment bar** Label (Stripe only) draws the connection bar in the
  panel colour with the outline under it, and keeps the stripe (macOS) or
  the coloured label and chips (Omarchy). Production keeps the full tint.

All of it is applied from `Settings` by `Look::with_grid` (row height,
stripes, header lines) for the two grids, and by `env::bar_colors` for the
bar. Colours come from the palettes there are: nothing new.

### Tests (headless, no pixels)

1. With no file, each look has its defaults: rows 37 and 30, stripes off
   on macOS and on on Omarchy, two header lines, a tree, a tinted bar.
2. Each key round-trips through the file; a bad value is ignored by its
   line.
3. Row density changes the grid's row height and keeps its top row.
4. Alternate rows paint the odd rows' fill in both looks, and none with
   the setting off.
5. Column types off makes the header one line high and draws no type.
6. The table list default applies only to a connection with no choice;
   the toggle saves a choice that a restart reads back.
7. With Stripe only, a dev connection's bar is the panel colour with its
   stripe or label still drawn, and a production one keeps its tint.
8. The pane's rows change with the keys on Omarchy and with clicks on
   macOS, and every change is written to the file.

## Step 2: Theme

- macOS: three tiles, System, Light, Dark, a radio group named "Theme";
  "System follows macOS, including Auto." System follows egui's system
  theme live (it does today); Light and Dark force a palette. Only
  `Palette::light()` and `Palette::dark()`. Values `"system"`, `"light"`,
  `"dark"`.
- Omarchy: follow omarchy, or pin a theme by name or path; "now: {name}"
  and the read-only palette swatches, "read from the omarchy theme". A pin
  that cannot be loaded falls back to following, said in the status line.
  Values `"omarchy"`, a theme name, a path.
- Open for the step's plan: today `appearance.theme` names a palette file
  in the themes directory on every look. The step decides how such a value
  reads on macOS (a value meant for the other look is ignored and the file
  left as it is).

## Step 3: Text

- macOS: Interface size (Small · 12, Default · 13, Large · 14) scales the
  chrome's roles; Data font (IBM Plex Mono, JetBrains Mono) and size (11 to
  16) change grid cells, the SQL editor and JSON. The OFL notice of the
  bundled fonts is shown with the app's licence.
- Omarchy: one font (system monospace through fontconfig, "now: jetbrains
  mono, from fontconfig", or a fontconfig name) and one size (9 to 24).
- A missing font falls back to the default and says so.
- Keys: ⌘= / ⌘- / ⌘0 (macOS) and `ctrl+=` / `ctrl+-` (Omarchy) change and
  reset `font_size` and write it, through new keymap commands, with
  egui's zoom off.

## Step 4: Preview

The real grid and header components with two Bookshop rows of
`book_covers`, and the connection bar, drawn from the settings as they
are. It never asks the backend for anything.

## Out of scope

- Custom colour editing or theme creation.
- Per-connection font or density.
- The other Settings tabs.
