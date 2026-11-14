# Look switch: Classic or Omarchy on every OS

## Goal

Today the look is fixed by the OS the build targets: `Look::for_platform()`
(`crates/tabletist-core/src/theme.rs`), read once in `App::new`
(`src/app.rs`), gives macOS the macOS look, Windows the standard look and
Linux the Omarchy look. This adds a choice on every OS between its classic
look and the Omarchy experience, made in Settings › Appearance or in
`settings.toml`, and applied live.

## Decisions

| Question | Decision | Why |
|---|---|---|
| What is it for | A user-facing setting: a control in Settings › Appearance on both settings screens, and a key in `settings.toml`. | Igor: "A user-facing setting". |
| Which looks, where | Every OS chooses between two: its classic look and the Omarchy experience. | Igor: "Linux version should have a traditional (MacOs/Windows) UI, plus Mac and Windows should have an Omarchy-Experience." |
| Linux's classic look | The macOS look with Ctrl keys written out (Windows' layout), never ⌘. | Igor chose it over the Windows look: it is the fully designed one. |
| When it applies | Live, at once: style, fonts and keymap change, and Settings redraws in the new look. | Igor: "Live, right away". |
| Windows' menu strip in the Omarchy look | Taken off the window; put back in the classic look. macOS keeps its global menu bar in both looks. | Igor: "Hide it". Omarchy has no menu bar. |
| The key and its values | `[appearance] look = "classic" \| "omarchy"`; absent is the OS's look of today. | Approved with the design's part 1. Nobody's app changes until they choose. |
| The control's place | First in Settings › Appearance, above Theme, in both screens. A departure from the design canvas, which has no look picker. | Approved with the design's part 1. |
| Architecture | The look is chosen from the setting and carries its key layout (`Look::keys`); `Layout::of` stops inferring it from the Plex typeface. | Approved with part 2. The rejected way (an OS check in `Layout::of`) would make the macOS look's screenshots, rendered on Linux, depend on the host. |
| Delivery | Two runs, a PR each: run 1 the setting, the live switch, both controls and the docs; run 2 the macOS and Windows parts. | Approved with part 2 (see the memory on shipping in steps). |

## What the person sees

### The setting

```toml
[appearance]
look = "omarchy"   # classic | omarchy
```

| OS | absent | `classic` | `omarchy` |
|---|---|---|---|
| macOS | classic | macOS look, Mac keys (⌘, glyphs) | Omarchy look, Omarchy keys |
| Windows | classic | standard look, Windows keys | Omarchy look, Omarchy keys, no menu strip |
| Linux | omarchy | macOS look, Windows keys (Ctrl, written out) | Omarchy look, Omarchy keys (today's) |

`look` is written only once it is set. A value other than the two is marked
red in the settings file pane and ignored, as any invalid line is: the OS's
default is drawn.

### The control

- The sheet (macOS look, standard look): a section "Interface" first in
  Appearance, above Theme, with one row "Interface" and a segmented control
  **Classic · Omarchy**. The row's note: "Omarchy: keyboard-first, square,
  monospace."
- The terminal board (Omarchy look): a section `interface` first in
  appearance, one choice row `interface` with `classic` and `omarchy`,
  changed with Tab and Enter/Space as the board's other choice rows are. The
  file pane shows the `look` line in `[appearance]`.

### Switching

Choosing the other value, from either screen or by editing the file:

- restyles the whole window, installs the new look's fonts, and makes its
  keymap, all before the next frame is drawn;
- keeps the Settings dialog open on Appearance, now drawn by the other
  screen, with the Interface row focused (the sheet's control, or the
  board's row selected);
- keeps every tab, query and pending edit as they were.

Settings whose defaults differ by look keep doing so: alternate rows, line
numbers, keywords, editor keys and the text size follow the look drawn
unless they are set. A theme value meant for the other look is kept in the
file and drawn as this look's default, as today. In the Omarchy look on
macOS or Windows, `theme = "omarchy"` finds no Omarchy theme and falls back
as on a Linux desktop that is not Omarchy. The terminal look's font is the
desktop's monospace on Linux and the bundled JetBrains Mono elsewhere, unless
the settings name an installed one.

## How it is built

### Core (`tabletist-core`)

- `settings.rs`: `Settings` gains `pub look: Option<LookChoice>` beside
  `theme` and `zebra`, `enum LookChoice { Classic, Omarchy }`, read and written as `look` in
  `[appearance]`, with its key line kept for the file pane's marks.
- `theme.rs`: `Look::chosen(choice: Option<LookChoice>, os: Os) -> Look`,
  with `Os` the three platforms and `Os::current()`. `for_platform()` becomes
  `chosen(None, Os::current())`.
- `Look` gains `pub keys: Layout`: `macos()` has `Mac`, `standard()`
  `Windows`, `omarchy()` `Omarchy`; `with_keys(layout)` gives another.
  Linux's classic is `Look::macos().with_keys(Layout::Windows)`.
- `keymap.rs`: `Layout::of(look)` returns `look.keys`.

### App

- `App::new` takes `Look::chosen(settings.look, Os::current())`.
- `App::apply_settings`, where every change arrives (the screens and the
  file), calls a new `App::switch_look` when the chosen look differs from
  the one drawn. It does what start-up does for a look: the look with the
  text and grid settings (`with_text`, `with_grid`); the desktop font read
  where the new look draws one (`reads_font`); fonts installed
  (`style::install_with`) and the style applied (`apply_style`); the palette
  resolved again from the new look's theme value; the font list asked for
  where the macOS look draws an installed face (`list_fonts`).
- The keymap is made again for the new layout by the existing check in the
  frame (`keymap_layout != layout()`), and the window buttons' line already
  follows the look (`ui::window_buttons_line`).
- The Settings dialog's state is one for both screens; the pane stays
  Appearance and the focus goes to the Interface control.

### Menu bar (run 2)

- Windows: `menu::Bar` takes the strip off the window when the look turns
  terminal and puts it back when it turns classic (`tabletist-menubar`'s
  `detach` and `attach`).
- macOS: the global menu stays; items show `keymap.menu_key(command,
  layout)` for the Omarchy layout, which has no key for an item whose
  Omarchy binding is a sequence (`cc`, `:`). The unified title bar stays in
  both looks; the Omarchy top bar leaves the window buttons their room.

## Testing

Headless, through `src/testing.rs`:

1. With no `look`, each OS resolves to its look today (`Look::chosen` for
   the three `Os` values).
2. Choosing Omarchy in the sheet's Interface control: `look.terminal`, the
   Omarchy layout, the board drawn on Appearance with `interface` selected,
   and `look = "omarchy"` written.
3. Choosing classic on the board: the classic look of the OS, the sheet
   drawn, `look = "classic"` written.
4. The file changed to the other value switches the look live.
5. An unknown value: its line marked, the OS's default drawn.
6. Linux's classic look writes Ctrl keys and never ⌘
   (`Look::macos().with_keys(Layout::Windows)`).
7. Settings that follow the look (alternate rows, text size) take the new
   look's default when unset, and keep a set value.

The Windows and macOS menu code is type-checked in a scratch crate (the
workspace does not cross-compile here). Renders are compared by eye in a
throwaway scene, never in a test.

## Delivery

1. **Run 1:** the setting, `Look::chosen` and `Look::keys`, the live switch,
   both controls, and the docs (`docs/_guide/macos.md`,
   `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md`).
   After it every OS can switch, and Linux has the whole feature.
2. **Run 2:** the menu strip on Windows, the macOS menu with Omarchy keys,
   the Omarchy top bar beside macOS's window buttons, and fonts and the
   Updates pane off Linux. Until it lands, Windows' Omarchy look still shows
   the strip.

## Out of scope

- A GNOME look (the canvas' "Linux – GNOME draft").
- The standard (Windows) look on Linux.
- A command-line flag or environment variable for the look.
- Changing any OS's default look.
