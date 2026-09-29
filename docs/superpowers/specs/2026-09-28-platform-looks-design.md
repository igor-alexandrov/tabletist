# Platform looks: a modern, per-OS visual refresh

Date: 2026-09-28. Status: approved in conversation, awaiting spec review.

## Intent

The interface looks dated, most of all on macOS. Tabletist should look like a
current native app on each platform:

- **macOS:** a Tahoe-era Mac app with the feel of Claude Desktop (warm
  neutral surfaces, soft hairlines, rounded inset selection, capsule fields).
- **Linux:** at home on Omarchy, following the Omarchy shell's own style
  tokens (square corners, 1 px borders, foreground-tinted states, monospace
  data).
- **Windows:** the shared modern base, no dedicated pass.

Success: the screenshot harness renders every scene under each look, light
and dark, and the result reads as native on macOS and Omarchy. Behaviour does
not change; every existing UI test passes unchanged.

## Decisions

| Question | Decision |
|---|---|
| macOS layout | Keep the connection tabs in the unified title bar beside the window buttons. Restyle; no structural move. |
| Omarchy typeface | Chrome stays Inter. Data (grid cells, row panel values, raw SQL, JSON) uses the desktop's `monospace` font. |
| Default palette | Warm neutrals with a system-blue accent. |
| Architecture | A `Look` struct of shape and density tokens, chosen per OS at runtime and overridable in tests and screenshots. |

Rejected: `cfg` constants (only the host's look could be tested or
screenshotted), separate views per platform (duplicated layout, drift).

## Out of scope

- macOS vibrancy or Liquid Glass (egui draws opaque fills; fastframe has no
  support). The sidebar tone stands in for glass.
- The macOS system accent colour.
- A Windows 11 Fluent pass.
- Any layout change beyond spacing: the tab strip, the workspace info bar,
  and the object tab strip keep their places.
- New features.

## The looks

| Token | macOS | Omarchy (all Linux) | Standard (Windows) |
|---|---|---|---|
| `radius` (controls, rows, tabs) | 8 | 0 | 4 (connection tabs 6) |
| `radius_small` (chips, icon buttons) | 6 | 0 | 4 |
| `radius_dialog` | 12 | 0 | 8 |
| `control_height` | 28 | 28 | 26 |
| `tree_row` (sidebar row height) | 26 | 24 | 24 |
| `grid_row` | 24 | 24 | 24 |
| `bordered_controls` | false | true | false |
| `selection` | `Pill` (inset, rounded, accent tint) | `Bar` (full width, fg @ 18%, accent text, 2 px accent left edge) | `Tint` (full width, rounded, accent tint) |
| `tabs` | `Capsule` (active raised, soft shadow) | `Underline` (square, 2 px accent under the active tab) | `Outlined` (active takes the window colour and a 1 px outline) |
| `dialog` | `Shadow` (large soft shadow, dimmed backdrop) | `AccentBorder` (2 px accent border, no shadow, 50% scrim) | `Shadow` |
| `search_field` | capsule (radius = height / 2), search icon | square, 1 px border, search icon | rounded, search icon |
| `sidebar_tinted` | true (one tone off the content, no separator) | false (window tone, 1 px border) | true |
| `panel_separators` | false | true | true (1 px lines between panels) |
| `data_font` | `Proportional` | `Monospace` | `Proportional` |

Standard keeps today's metrics; the plan refined radius/radius_small into radius, tab_radius and dialog_radius.

Omarchy control states come from the shell's `Style.qml`, as foreground alpha
over the background:

| State | Fill | Border |
|---|---|---|
| normal | fg @ 4% | 1 px fg @ 40% |
| hover, keyboard focus | fg @ 8% | 1 px fg @ 25% |
| selected | fg @ 18% | none |
| pressed | fg @ 22% | none |
| text selection | accent @ 35% | none |

egui draws keyboard focus with the active (pressed) visuals, so a focused control shows the pressed fill and border.

Everywhere:

- The grid gains a row hover tint.
- Primary buttons (Save & Connect, New connection, Reconnect) fill with the
  accent.
- The picker has a larger title, connections as cards (row height 56, driver
  and summary on the second line), and a primary New connection button.
- Top bar and footer padding sits on the 4 pt grid; the workspace info bar and
  footer drop to `control_height + 8`.

## Palette

`Palette::light()` and `Palette::dark()` become warm neutrals. The final values
are tuned while reviewing screenshots and must pass the contrast test:

| Role | Light | Dark |
|---|---|---|
| window (content) | #fdfcfa | #1f1e1d |
| panel (bars, sidebar) | #f4f2ee | #262523 |
| surface | #ebe8e3 | #302e2c |
| outline | #e2dfd9 | #3a3835 |
| text | #1f1e1c | #ecebe8 |
| accent | #0a6fe0 | #3b8cf7 |

No new palette field. Palette files and the Omarchy template keep working
unchanged, so an Omarchy theme or a chosen palette file draws any look.

## Architecture

### `src/theme.rs`

- `pub struct Look { ... }` with the tokens above, plus enums `Selection`,
  `TabStyle`, `DialogStyle`, `DataFont`.
- `Look::macos()`, `Look::omarchy()`, `Look::standard()`, and
  `Look::for_platform()`, which picks one with `cfg!(target_os)`. Linux maps to
  Omarchy whether or not the desktop is Omarchy (the user asked for the Omarchy
  style on Linux).
- `theme::apply(ctx, &palette, &look)` maps both onto `egui::Style`:
  - widget and window corner radii
  - Omarchy's fill and border states on the widget visuals
  - the window stroke and shadow per `dialog`
  - the text styles, with `Monospace` bound to the data font
- The constants `RADIUS`, `RADIUS_SMALL`, `CONTROL_HEIGHT` and the per-view
  `ROW_HEIGHT` values are removed once views read the look.

### `App`

`App` gains `pub look: Look`, which is `Look::for_platform()` at startup. Every
place that calls `theme::apply` passes it.

### The data font

`theme::install` takes the look. With `DataFont::Monospace`, on Linux:

1. It resolves the desktop's monospace font file with
   `fc-match -f '%{file}' monospace`.
2. It reads the file.
3. It passes it to fastframe-fonts as `Monospace::Font`.

If any step fails, it logs one warning and uses egui's bundled monospace. Tests
and screenshots (`system_fallbacks == false`) skip the lookup for
reproducible output. Views that draw data switch from `theme::regular` to
`theme::data(size, look)`.

### Shared painters, `src/ui/widgets.rs`

- `selection(ui, rect, selected, hovered, look, palette)`: sidebar tree,
  picker rows, quick open.
- `tab(ui, rect, active, hovered, look, palette)`: connection tabs and object
  tabs.
- `search_field(ui, text, hint, look, palette) -> Response`: the sidebar
  filter, picker search, row panel filter, quick open.
- `primary_button(ui, text, look, palette) -> Response`.
- `icon_button` takes the look for its hover radius.

Views call these instead of painting rectangles inline.

### Views touched

- `conn_tabs.rs`
- `object_tabs.rs`
- `sidebar.rs`
- `grid.rs` (hover, header, data font)
- `row_panel.rs` (data font, field filter)
- `picker.rs` (cards, title, primary button)
- `workspace.rs` (bar heights, banner radius)
- `data_view.rs` (footer, error radius)
- `filter_bar.rs`
- `quick_open.rs`
- `connect_dialog.rs` (primary button, dialog style)
- `password_prompt.rs`
- `host_key_prompt.rs`
- `help.rs`
- `structure.rs` (data font for types)

## Testing

- **Unit (`theme.rs`):**
  - Each look's style mapping: Omarchy has radius 0 and bordered inactive
    widgets; macOS has radius 8 and no widget border.
  - `for_platform` matches the host.
  - The existing contrast test runs over the new palette.
- **UI:**
  - Every existing AccessKit test passes unchanged.
  - A new test lays the window out at 720x480 and 2560x1440 under all three
    looks, and finds the tab bar, the sidebar filter and the grid.
- **Screenshots:**
  - `Harness::for_shots(size, scale, light, look)`.
  - `shots.rs` writes every scene as `<scene>-<look>-<light|dark>.png` (the
    macOS scenes with the measured title bar).
  - Before and after pairs are reviewed by eye.
- **Checks:** `cargo fmt --check`, `clippy -D warnings`, `cargo test`, and
  `cargo doc`.
- **macOS:** `cargo check --target aarch64-apple-darwin` if the target
  installs; otherwise it is reported as rendered by the harness only.
- **Coverage report:** macOS and Windows are rendered by the harness and
  compiled, not run on those systems.

## Commits

One topic per commit, each passing the checks:

1. Add `Look` and thread it through `App`, `apply`, and the harness (no visual
   change: `standard` equals today's values).
2. Warm palette.
3. Shared painters, then the views moved onto them, one or two views per
   commit.
4. macOS look values.
5. Omarchy look values and the monospace data font.
6. Screenshots per look, then refreshed README screenshots.
