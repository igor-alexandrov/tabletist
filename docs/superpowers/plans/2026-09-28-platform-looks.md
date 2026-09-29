# Platform Looks Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give Tabletist a modern look native to each OS: a Tahoe-era macOS look, an Omarchy look on Linux, and the standard look on Windows. Each OS also gets a warm default palette.

**Architecture:** A `Look` struct of shape and density tokens sits next to `Palette` in `src/theme.rs`.
- `Look::for_platform()` picks the look at startup. Tests and screenshots can set any look.
- `theme::apply` maps palette and look onto `egui::Style`.
- A few shared painters in `src/ui/widgets.rs` (selection, tab, card, search field, primary button, modal) draw with the look. Views call them instead of painting inline.

**Tech Stack:** Rust 2024, egui/eframe 0.36 (crmne fork), fastframe-fonts/-theme v0.1.7, skrifa 0.44 (already in the tree through epaint), AccessKit headless tests, egui_kittest + wgpu screenshots.

**Spec:** `docs/superpowers/specs/2026-09-28-platform-looks-design.md`

## Global Constraints

- Behaviour does not change. Every existing test passes unchanged, apart from the harness constructor signatures this plan changes.
- `unsafe_code = "forbid"`; clippy `-D warnings`; `unwrap_used` warns (tests are exempt through `clippy.toml`).
- Never use em dashes in code, comments, docs, or commit messages.
- Platform code sits behind `cfg`. Linux, macOS and Windows keep compiling.
- Every new crate in `Cargo.toml` gets a comment explaining it. Prefer crates already in the tree.
- Palette files and the Omarchy template (`contrib/omarchy/tabletist.json.tpl`) keep working unchanged. No new palette field.
- Text meets contrast: `text` and `secondary` 4.5:1 and `dim` 3:1 on window, panel and surface, plus `on_accent` 4.5:1 on accent (the `palette_text_meets_contrast` test).
- Checks before every commit:
  - `cargo fmt --all --check`
  - `cargo clippy --locked --workspace --all-targets -- -D warnings`
  - `cargo test --locked --workspace --all-targets`
  - `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- Use `~/.cargo/bin/cargo` if `cargo` resolves to a broken mise shim.
- Commit messages: imperative sentence subject, a short body, ending with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.

### Token names (refines the spec's table)

The spec's `radius` and `radius_small` become three tokens, so the standard look keeps today's metrics exactly:

| Token | Used for | Standard | macOS | Omarchy |
|---|---|---|---|---|
| `radius` | controls, tree rows, object tabs, icon-button hover, cards | 4 | 8 | 0 |
| `tab_radius` | connection tabs | 6 | 8 | 0 |
| `dialog_radius` | dialogs, popups | 8 | 12 | 0 |
| `control_height` | fields, buttons | 26 | 28 | 28 |
| `tree_row` | sidebar rows | 24 | 26 | 24 |
| `bordered_controls` | Omarchy fg-alpha fills and 1 px borders | false | false | true |
| `selection` | `Tint` (full width, today) / `Pill` (inset 6, rounded) / `Bar` (full width, 2 px accent edge, accent text) | Tint | Pill | Bar |
| `tabs` | `Outlined` (today) / `Raised` (active: window fill and soft shadow, no stroke) / `Underline` (square, 2 px accent under active) | Outlined | Raised | Underline |
| `dialog` | `Shadow` / `AccentBorder` | Shadow | Shadow | AccentBorder |
| `capsule_search` | search fields fully rounded | false | true | false |
| `sidebar_tinted` | sidebar fill `panel` (else `window`) | true | true | false |
| `panel_separators` | 1 px lines between panels | true | false | true |
| `data_font` | grid cells and row-panel values | Proportional | Proportional | Monospace |

## Review Focus

1. **Font file egui cannot parse.** `fc-match` may name a `.pcf` bitmap, a `.ttc` face index, or a broken file. epaint panics on unparsable font data. Expected: the file is validated with skrifa first, and on any failure the app starts with the bundled monospace. Task 7 pins this with `an_unparsable_desktop_font_is_ignored` and `fc_match_output_parses_path_and_index`.
2. **Minimum window (720x480) with the taller macOS and Omarchy metrics.** Expected: the tab bar, sidebar filter and grid are all still present, and nothing panics. Task 3 pins this with `every_look_lays_out_at_small_and_large_sizes`.
3. **An Omarchy light theme** (rose-pine, catppuccin-latte, flexoki-light, white). Expected: control borders are still visible on light backgrounds, because the borders derive from `text`, not a fixed colour. Task 2 pins this with `omarchy_borders_follow_the_text_colour_in_light_themes`.
4. **Keyboard focus under the Omarchy look.** Hover and focus share one fill. Expected: a focused text field still shows the accent focus stroke. Task 2 pins this with `focused_fields_keep_the_accent_stroke_in_every_look`.
5. **Activating tabs, tree rows and picker cards by keyboard or screen reader.** After the move to shared painters, these must still expose the same AccessKit names and roles. Expected: the existing tests (`the_close_button_closes_its_tab`, `the_connect_button_on_a_row_connects_in_this_tab`, and the sidebar tests) pass unchanged. Tasks 4 to 6 run the whole suite.

---

## File structure

- `src/theme.rs`: `Look` and its enums, the palette values, `apply(ctx, palette, look)`, `install(ctx, system_fallbacks, look)`, `data(look)`. The desktop monospace lookup goes in a new submodule, `src/theme/desktop_font.rs`, declared with `mod desktop_font;` in `theme.rs` (a file module may keep its children in a same-named directory).
- `src/app.rs`: `App.look`, passed to `install` and `apply`.
- `src/testing.rs`: the harness starts in `Look::standard()`. It adds `set_look`, and `for_shots` takes a look.
- `src/ui/widgets.rs`: `single(text, look)`, `icon_button(ui, icon, label, look, palette)`, `selection`, `tab`, `card`, `search_field`, `primary_button`, `modal`.
- Views: `conn_tabs.rs`, `object_tabs.rs`, `sidebar.rs`, `picker.rs`, `quick_open.rs`, `grid.rs`, `row_panel.rs`, `data_view.rs`, `filter_bar.rs`, `workspace.rs`, `connect_dialog.rs`, `password_prompt.rs`, `host_key_prompt.rs`, `help.rs`, `ui/mod.rs`.
- `src/shots.rs`: every scene under every look.
- `Cargo.toml`: `skrifa` for Linux only.
- `README.md` and `docs/screenshots/`: refreshed screenshots.

---

### Task 1: `Look` threaded through the app with no visual change

**Files:**
- Modify: `src/theme.rs` (add `Look`; change `apply` and `install`; tests)
- Modify: `src/app.rs:52-105` (field), `src/app.rs:1900-1945` (`attach`, `logic`)
- Modify: `src/testing.rs:324-377` (harness)
- Modify: `src/shots.rs:145-152` (`both`)
- Modify: `src/ui/widgets.rs` (`single`, `icon_button` signatures), plus every call site

**Interfaces:**
- Produces:
  - `theme::Look` (the `Copy` struct below), with its enums `Selection`, `TabStyle`, `DialogStyle`, `DataFont`
  - `Look::standard()`, `Look::macos()`, `Look::omarchy()`, `Look::for_platform()`, `Look::ALL: [Look; 3]`
  - `theme::apply(ctx: &egui::Context, palette: &Palette, look: &Look)`
  - `theme::install(ctx: &egui::Context, system_fallbacks: bool, look: &Look)`
  - `App.look: Look`
  - `Harness::set_look(&mut self, look: Look)`
  - `Harness::for_shots(size, scale, light: bool, look: Look)`
  - `widgets::single(text: &mut String, look: &Look) -> egui::TextEdit<'_>`
  - `widgets::icon_button(ui, icon, label: &str, look: &Look, palette: &Palette) -> Response`

- [ ] **Step 1: Write the failing tests** in `src/theme.rs` `mod tests`:

```rust
    #[test]
    fn the_standard_look_keeps_todays_metrics() {
        let ctx = egui::Context::default();
        apply(&ctx, &Palette::light(), &Look::standard());
        let style = ctx.global_style();
        assert_eq!(style.visuals.widgets.inactive.corner_radius, CornerRadius::same(4));
        assert_eq!(style.visuals.window_corner_radius, CornerRadius::same(8));
        assert_eq!(style.visuals.menu_corner_radius, CornerRadius::same(6));
        assert_eq!(style.spacing.interact_size.y, 26.0);
        assert_eq!(style.visuals.widgets.inactive.bg_stroke, Stroke::NONE);
    }

    #[test]
    fn the_platform_look_matches_the_host() {
        let expected = if cfg!(target_os = "macos") {
            Look::macos()
        } else if cfg!(target_os = "linux") {
            Look::omarchy()
        } else {
            Look::standard()
        };
        assert_eq!(Look::for_platform(), expected);
    }

    #[test]
    fn looks_have_distinct_names() {
        let names: Vec<_> = Look::ALL.iter().map(|look| look.name).collect();
        assert_eq!(names, ["standard", "macos", "omarchy"]);
    }
```

Update the two existing tests that call `apply(&ctx, &palette)` to `apply(&ctx, &palette, &Look::standard())`.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib theme::tests`
Expected: compile errors (`Look` not found, `apply` takes 2 arguments).

- [ ] **Step 3: Add `Look`** to `src/theme.rs`. Put it after the `Palette` impls. Delete `RADIUS`, `RADIUS_SMALL` and `CONTROL_HEIGHT` at the end of this task, once nothing reads them (Step 5).

```rust
/// How a selected row shows it is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Selection {
    /// A full-width rounded tint.
    Tint,
    /// An inset, rounded pill (macOS sidebars).
    Pill,
    /// A full-width bar with an accent edge and accent text (Omarchy).
    Bar,
}

/// How tabs show which one is active.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabStyle {
    /// The active tab takes the window colour and an outline.
    Outlined,
    /// The active tab takes the window colour and a soft shadow.
    Raised,
    /// Square tabs; the active one is underlined in the accent.
    Underline,
}

/// How dialogs and popups stand off the window.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DialogStyle {
    /// A soft shadow and a dimmed backdrop.
    Shadow,
    /// A 2 px accent border, no shadow, and a scrim (Omarchy).
    AccentBorder,
}

/// The face data is drawn in: grid cells and row-panel values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DataFont {
    Proportional,
    Monospace,
}

/// Shape and density: everything about the interface that follows the
/// platform rather than the colour theme. Any palette draws with any look.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    /// Names screenshots and tests.
    pub name: &'static str,
    /// Controls, tree rows, object tabs, icon-button hover, cards.
    pub radius: u8,
    /// Connection tabs.
    pub tab_radius: u8,
    /// Dialogs and popups.
    pub dialog_radius: u8,
    /// Fields and buttons share one height.
    pub control_height: f32,
    /// Sidebar tree rows.
    pub tree_row: f32,
    /// Omarchy's controls: a faint foreground fill and a 1 px border.
    pub bordered_controls: bool,
    pub selection: Selection,
    pub tabs: TabStyle,
    pub dialog: DialogStyle,
    /// Search fields are fully rounded.
    pub capsule_search: bool,
    /// The sidebar takes the panel colour rather than the window's.
    pub sidebar_tinted: bool,
    /// Panels are separated by 1 px lines.
    pub panel_separators: bool,
    pub data_font: DataFont,
}

impl Look {
    pub const ALL: [Look; 3] = [Self::standard(), Self::macos(), Self::omarchy()];

    /// Windows, and today's metrics.
    pub const fn standard() -> Self {
        Self {
            name: "standard",
            radius: 4,
            tab_radius: 6,
            dialog_radius: 8,
            control_height: 26.0,
            tree_row: 24.0,
            bordered_controls: false,
            selection: Selection::Tint,
            tabs: TabStyle::Outlined,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            data_font: DataFont::Proportional,
        }
    }

    /// macOS 26: rounder, roomier, borderless, with an inset pill selection.
    pub const fn macos() -> Self {
        Self {
            name: "macos",
            radius: 8,
            tab_radius: 8,
            dialog_radius: 12,
            control_height: 28.0,
            tree_row: 26.0,
            bordered_controls: false,
            selection: Selection::Pill,
            tabs: TabStyle::Raised,
            dialog: DialogStyle::Shadow,
            capsule_search: true,
            sidebar_tinted: true,
            panel_separators: false,
            data_font: DataFont::Proportional,
        }
    }

    /// Omarchy's shell: square, 1 px borders, monospace data.
    pub const fn omarchy() -> Self {
        Self {
            name: "omarchy",
            radius: 0,
            tab_radius: 0,
            dialog_radius: 0,
            control_height: 28.0,
            tree_row: 24.0,
            bordered_controls: true,
            selection: Selection::Bar,
            tabs: TabStyle::Underline,
            dialog: DialogStyle::AccentBorder,
            capsule_search: false,
            sidebar_tinted: false,
            panel_separators: true,
            data_font: DataFont::Monospace,
        }
    }

    /// The look of the OS this build runs on. Every Linux desktop gets the
    /// Omarchy look.
    pub const fn for_platform() -> Self {
        if cfg!(target_os = "macos") {
            Self::macos()
        } else if cfg!(target_os = "linux") {
            Self::omarchy()
        } else {
            Self::standard()
        }
    }
}
```

- [ ] **Step 4: Thread the look through `apply` and `install`.** In `src/theme.rs`:

```rust
pub fn install(ctx: &egui::Context, system_fallbacks: bool, _look: &Look) {
    // (body unchanged; Task 7 uses the look)
}

pub fn apply(ctx: &egui::Context, palette: &Palette, look: &Look) {
    let mut style = (*ctx.global_style()).clone();
    apply_to_style(&mut style, palette, look);
    // ... unchanged
}

fn apply_to_style(style: &mut egui::Style, palette: &Palette, look: &Look) {
```

Inside `apply_to_style`, replace the constants with look tokens:
- `visuals.window_corner_radius = CornerRadius::same(look.dialog_radius);`
- `visuals.menu_corner_radius = CornerRadius::same(look.tab_radius);` (6 for standard, as today)
- `let corner = CornerRadius::same(look.radius);`
- `style.spacing.interact_size = Vec2::new(40.0, look.control_height);`

- [ ] **Step 5: Add `App.look` and update the call sites.**

`src/app.rs`:
- Add the field `pub look: crate::theme::Look,` after `palette` in the struct, and `look: crate::theme::Look::for_platform(),` in `App::new`.
- In `attach`: `theme::install(ctx, follow_desktop, &self.look);` and `theme::apply(ctx, &self.palette, &self.look);`.
- In `logic`: `theme::apply(ctx, &palette, &self.look);`.

`src/testing.rs` `with_backend`: make the binding `let mut app`, and before `app.attach(&ctx, false);` add `app.look = crate::theme::Look::standard();`. Tests stay deterministic on every OS. Add to the plain `impl Harness`:

```rust
    /// Draws with `look` from the next frame on.
    pub fn set_look(&mut self, look: crate::theme::Look) {
        self.app.look = look;
        crate::theme::install(&self.ctx, false, &look);
        crate::theme::apply(&self.ctx, &self.app.palette, &look);
    }
```

In `for_shots`, take `look: crate::theme::Look`, and replace the `theme::apply` line with `harness.set_look(look);`, placed after the palette is set.

In `src/shots.rs`, change `both` to render every look. The macOS look also gets the measured title bar:

```rust
fn both(name: &str, scene: impl Fn(&mut Harness)) {
    for look in crate::theme::Look::ALL {
        for (light, suffix) in [(true, "light"), (false, "dark")] {
            let mut harness = Harness::for_shots(SIZE, 2.0, light, look);
            if look == crate::theme::Look::macos() {
                // Measured on a macOS 26 window with a unified compact toolbar.
                harness.app.titlebar = crate::app::TitleBar {
                    height: 38.0,
                    inset: 80.0,
                };
            }
            scene(&mut harness);
            harness.shot(&out(&format!("{name}-{}-{suffix}.png", look.name)));
        }
    }
}
```

Delete the `workspace-macos` scene in `shots()`, since the macOS look now covers it. In `sidebar_width_over_time`, pass `crate::theme::Look::standard()`.

`src/ui/widgets.rs`: `single(text, look)` uses `look.control_height`; `icon_button(ui, icon, label, look, palette)` uses `CornerRadius::same(look.radius)` for the hover fill.

Update every call site with `rg -n 'widgets::single\(|icon_button\(' src`. Each view has `app.look` available: add `let look = app.look;` next to `let palette = app.palette;`, and pass `&look`. Replace `crate::theme::CONTROL_HEIGHT` in `sidebar.rs` with `look.control_height`. Replace the remaining `theme::RADIUS` and `theme::RADIUS_SMALL` uses with the equivalent tokens:
- `conn_tabs.rs` (`RADIUS`): `look.tab_radius`
- `workspace.rs` banner and `data_view.rs` error (`RADIUS`): `look.tab_radius`
- `sidebar.rs` and `object_tabs.rs` (`RADIUS_SMALL`): `look.radius`
- `picker.rs` (`RADIUS`): `look.tab_radius`

Every value stays the same under the standard look. Delete the three constants.

- [ ] **Step 6: Run the full suite**

Run: `cargo test --locked --workspace --all-targets`
Expected: PASS, with the three new theme tests included.
Run: `cargo build --features shots --tests` (compiles shots.rs)
Expected: builds.

- [ ] **Step 7: Run all checks and commit**

```bash
git add -A src
git commit -m "Add Look, the per-platform shape tokens, with today's metrics as standard

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Warm palette and the per-look egui style

**Files:**
- Modify: `src/theme.rs` (`Palette::light`, `Palette::dark`, `apply_to_style`, tests)

**Interfaces:**
- Consumes: `Look`, `apply(ctx, palette, look)` (Task 1).
- Produces: egui visuals per look. Later tasks rely on the following:
  - Omarchy: `widgets.*.corner_radius == 0`; inactive `bg_stroke` 1 px at `text` 40%; `window_shadow == Shadow::NONE`; `window_stroke == 2 px accent`.
  - macOS: widget corners 8; window corners 12.

- [ ] **Step 1: Write the failing tests** in `src/theme.rs` `mod tests`:

```rust
    #[test]
    fn the_omarchy_look_is_square_with_bordered_controls() {
        let ctx = egui::Context::default();
        let palette = Palette::dark();
        apply(&ctx, &palette, &Look::omarchy());
        let style = ctx.global_style();
        let visuals = &style.visuals;
        assert_eq!(visuals.widgets.inactive.corner_radius, CornerRadius::ZERO);
        assert_eq!(visuals.window_corner_radius, CornerRadius::ZERO);
        assert_eq!(visuals.widgets.inactive.bg_stroke.width, 1.0);
        assert_eq!(visuals.window_shadow, egui::epaint::Shadow::NONE);
        assert_eq!(visuals.window_stroke, Stroke::new(2.0, palette.accent));
        assert_eq!(style.spacing.interact_size.y, 28.0);
    }

    #[test]
    fn omarchy_borders_follow_the_text_colour_in_light_themes() {
        let ctx = egui::Context::default();
        let palette = Palette::light();
        apply(&ctx, &palette, &Look::omarchy());
        let stroke = ctx.global_style().visuals.widgets.inactive.bg_stroke;
        assert_eq!(stroke.color, palette.text.gamma_multiply(0.4));
        assert!(stroke.color.a() > 0);
    }

    #[test]
    fn the_macos_look_rounds_borderless_controls() {
        let ctx = egui::Context::default();
        apply(&ctx, &Palette::light(), &Look::macos());
        let style = ctx.global_style();
        assert_eq!(style.visuals.widgets.inactive.corner_radius, CornerRadius::same(8));
        assert_eq!(style.visuals.window_corner_radius, CornerRadius::same(12));
        assert_eq!(style.visuals.widgets.inactive.bg_stroke, Stroke::NONE);
        assert_eq!(style.spacing.interact_size.y, 28.0);
    }

    #[test]
    fn focused_fields_keep_the_accent_stroke_in_every_look() {
        for look in Look::ALL {
            let ctx = egui::Context::default();
            let palette = Palette::dark();
            apply(&ctx, &palette, &look);
            let selection = ctx.global_style().visuals.selection;
            assert_eq!(selection.stroke, Stroke::new(1.0, palette.accent), "{}", look.name);
        }
    }

    #[test]
    fn the_default_palettes_are_warm() {
        // Warm neutrals lean red over blue.
        for palette in [Palette::light(), Palette::dark()] {
            for color in [palette.window, palette.panel, palette.surface] {
                assert!(color.r() > color.b(), "{color:?} is not warm");
            }
        }
    }
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib theme::tests`
Expected: FAIL on the Omarchy, macOS and warm-palette tests. The focus test passes already, and it guards the change that follows.

- [ ] **Step 3: Set the warm palettes.** These values were checked against the contrast test while planning. The lowest ratios are light dim on surface at 3.54 and dark dim on surface at 3.49.

```rust
    pub fn dark() -> Self {
        Self {
            dark: true,
            window: Color32::from_rgb(0x1f, 0x1e, 0x1d),
            panel: Color32::from_rgb(0x26, 0x25, 0x23),
            surface: Color32::from_rgb(0x30, 0x2e, 0x2c),
            surface_hover: Color32::from_rgb(0x39, 0x37, 0x34),
            surface_active: Color32::from_rgb(0x43, 0x40, 0x3d),
            outline: Color32::from_rgb(0x3a, 0x38, 0x35),
            text: Color32::from_rgb(0xec, 0xeb, 0xe8),
            secondary: Color32::from_rgb(0xb0, 0xad, 0xa7),
            dim: Color32::from_rgb(0x85, 0x81, 0x7b),
            accent: Color32::from_rgb(0x3b, 0x8c, 0xf7),
            accent_hover: Color32::from_rgb(0x5a, 0x9f, 0xf8),
            on_accent: Color32::from_rgb(0x0b, 0x13, 0x20),
            danger: Color32::from_rgb(0xf2, 0x70, 0x7a),
            warning: Color32::from_rgb(0xea, 0xb3, 0x5a),
            overlay: Color32::from_rgb(0x2d, 0x2c, 0x2a),
            shadow: Color32::from_black_alpha(150),
        }
    }

    pub fn light() -> Self {
        Self {
            dark: false,
            window: Color32::from_rgb(0xfd, 0xfc, 0xfa),
            panel: Color32::from_rgb(0xf4, 0xf2, 0xee),
            surface: Color32::from_rgb(0xeb, 0xe8, 0xe3),
            surface_hover: Color32::from_rgb(0xe4, 0xe1, 0xdb),
            surface_active: Color32::from_rgb(0xd9, 0xd5, 0xce),
            outline: Color32::from_rgb(0xe3, 0xe0, 0xda),
            text: Color32::from_rgb(0x1f, 0x1e, 0x1c),
            secondary: Color32::from_rgb(0x5c, 0x59, 0x53),
            // 3:1 or better on every light surface (large-text minimum).
            dim: Color32::from_rgb(0x7d, 0x79, 0x73),
            accent: Color32::from_rgb(0x0a, 0x6f, 0xe0),
            accent_hover: Color32::from_rgb(0x0a, 0x5f, 0xc2),
            on_accent: Color32::WHITE,
            danger: Color32::from_rgb(0xd1, 0x3b, 0x45),
            warning: Color32::from_rgb(0xb0, 0x74, 0x10),
            overlay: Color32::from_rgb(0xff, 0xff, 0xff),
            shadow: Color32::from_black_alpha(40),
        }
    }
```

- [ ] **Step 4: Map the look onto the style.** At the end of the visuals section of `apply_to_style`, after the per-state fills and before `visuals.text_cursor`, add:

```rust
    if look.bordered_controls {
        // Omarchy's shell: foreground alpha over the background, 1 px borders.
        let fg = palette.text;
        let states = [
            (&mut visuals.widgets.inactive, 0.04, 0.40),
            (&mut visuals.widgets.hovered, 0.08, 0.25),
            (&mut visuals.widgets.active, 0.22, 0.25),
            (&mut visuals.widgets.open, 0.08, 0.25),
        ];
        for (widget, fill, border) in states {
            widget.bg_fill = fg.gamma_multiply(fill);
            widget.weak_bg_fill = fg.gamma_multiply(fill);
            widget.bg_stroke = Stroke::new(1.0, fg.gamma_multiply(border));
        }
        visuals.extreme_bg_color = fg.gamma_multiply(0.04);
    }
    match look.dialog {
        DialogStyle::Shadow => {}
        DialogStyle::AccentBorder => {
            visuals.window_stroke = Stroke::new(2.0, palette.accent);
            visuals.window_shadow = egui::epaint::Shadow::NONE;
            visuals.popup_shadow = egui::epaint::Shadow::NONE;
        }
    }
```

- [ ] **Step 5: Run the tests**

Run: `cargo test --lib theme::tests`
Expected: PASS, with `palette_text_meets_contrast` included.
Run: `cargo test --locked --workspace --all-targets`
Expected: PASS.

- [ ] **Step 6: Run all checks and commit**

```bash
git add src/theme.rs
git commit -m "Warm the default palettes and style controls per look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Shared painters and a layout test across looks

**Files:**
- Modify: `src/ui/widgets.rs`
- Modify: `src/ui/mod.rs` (tests)

**Interfaces:**
- Consumes: `Look` and its enums (Task 1).
- Produces:
  - `widgets::selection(ui: &egui::Ui, rect: Rect, selected: bool, hovered: bool, look: &Look, palette: &Palette)`: paints only, allocates nothing.
  - `widgets::selection_text(selected: bool, look: &Look, palette: &Palette) -> Color32`: the text colour for a row.
  - `widgets::tab(ui: &egui::Ui, rect: Rect, active: bool, hovered: bool, radius: u8, look: &Look, palette: &Palette)`: paints only.
  - `widgets::card(ui: &egui::Ui, rect: Rect, hovered: bool, look: &Look, palette: &Palette)`: paints only.
  - `widgets::search_field<'t>(text: &'t mut String, hint: &str, look: &Look) -> egui::TextEdit<'t>`
  - `widgets::primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -> Response`
  - `widgets::modal(id: egui::Id, look: &Look, palette: &Palette) -> egui::Modal`
  - `widgets::add_search(ui: &mut Ui, field: egui::TextEdit<'_>, look: &Look) -> Response`: adds a search field, rounded into a capsule on macOS.
  - `Harness::frame_with(&mut self, add: impl FnOnce(&mut egui::Ui)) -> TreeUpdate`

- [ ] **Step 1: Write the failing tests.** In `src/ui/mod.rs` `mod tests`:

```rust
    #[test]
    fn every_look_lays_out_at_small_and_large_sizes() {
        for look in crate::theme::Look::ALL {
            for size in [egui::vec2(720.0, 480.0), egui::vec2(2560.0, 1440.0)] {
                let mut harness = Harness::with_size(size);
                harness.set_look(look);
                let tab = harness.connect_fake();
                harness.app.apply(crate::model::Action::OpenObject {
                    tab,
                    object: tabletist_db::ObjectRef::new("main", "users"),
                    kind: tabletist_db::ObjectKind::Table,
                    pin: true,
                });
                harness.answer_rows(crate::testing::page(3, false));
                let tree = harness.settle();
                for (label, role) in [
                    ("New connection tab", egui::accesskit::Role::Button),
                    ("Refresh objects", egui::accesskit::Role::Button),
                    ("Row 1", egui::accesskit::Role::Button),
                ] {
                    assert!(
                        crate::testing::node(&tree, label, role).is_some(),
                        "{label} missing in {} at {size:?}",
                        look.name
                    );
                }
            }
        }
    }
```

In `src/ui/widgets.rs` `mod tests`:

```rust
    #[test]
    fn the_primary_button_is_announced_by_its_text() {
        let mut harness = crate::testing::Harness::new();
        let look = crate::theme::Look::omarchy();
        let palette = crate::theme::Palette::dark();
        let tree = harness.frame_with(|ui| {
            super::primary_button(ui, "Save & Connect", &look, &palette);
        });
        assert!(crate::testing::node(&tree, "Save & Connect", egui::accesskit::Role::Button).is_some());
    }

    #[test]
    fn omarchy_dialogs_use_a_scrim_and_an_accent_border() {
        let palette = crate::theme::Palette::dark();
        let modal = super::modal(egui::Id::new("t"), &crate::theme::Look::omarchy(), &palette);
        assert_eq!(modal.backdrop_color.a(), 128);
        let shadowed = super::modal(egui::Id::new("t"), &crate::theme::Look::macos(), &palette);
        assert!(shadowed.backdrop_color.a() > 0);
    }
```

This needs a small harness helper in `src/testing.rs` that runs one frame of an arbitrary UI and returns the AccessKit tree:

```rust
impl Harness {
    /// Runs one frame that draws only `add` in a central panel, for
    /// testing a widget on its own.
    pub fn frame_with(&mut self, add: impl FnOnce(&mut egui::Ui)) -> TreeUpdate {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.scale);
        let mut add = Some(add);
        let mut output = self.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                if let Some(add) = add.take() {
                    add(ui);
                }
            });
        });
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("AccessKit is enabled")
    }
}
```

This mirrors `Harness::frame` (`src/testing.rs:44`) without the app, the clipboard, or the shots renderer.

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test --lib every_look_lays_out primary_button omarchy_dialogs`
Expected: `every_look_lays_out...` may already pass, since it guards the later view changes. The widget tests fail to compile because `primary_button` and `modal` don't exist yet.

- [ ] **Step 3: Implement the painters** in `src/ui/widgets.rs`. Import `crate::theme::{DialogStyle, Look, Selection, TabStyle}` and `egui::{Color32, CornerRadius, Rect, Stroke, StrokeKind}`.

```rust
/// The highlight behind a selected or hovered row: sidebar, picker, quick
/// open. Paints only; the caller allocated `rect`.
pub fn selection(
    ui: &Ui,
    rect: Rect,
    selected: bool,
    hovered: bool,
    look: &Look,
    palette: &Palette,
) {
    let painter = ui.painter();
    let corner = CornerRadius::same(look.radius);
    match look.selection {
        Selection::Tint => {
            if selected {
                painter.rect_filled(rect, corner, palette.accent.gamma_multiply(0.2));
            } else if hovered {
                painter.rect_filled(rect, corner, palette.surface_hover);
            }
        }
        Selection::Pill => {
            let pill = rect.shrink2(vec2(6.0, 1.0));
            if selected {
                painter.rect_filled(pill, corner, palette.accent.gamma_multiply(0.22));
            } else if hovered {
                painter.rect_filled(pill, corner, palette.surface_hover);
            }
        }
        Selection::Bar => {
            if selected {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.18));
                let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
                painter.rect_filled(edge, CornerRadius::ZERO, palette.accent);
            } else if hovered {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
            }
        }
    }
}

/// The text colour of a row: Omarchy marks the selected row in the accent.
pub fn selection_text(selected: bool, look: &Look, palette: &Palette) -> Color32 {
    if selected && look.selection == Selection::Bar {
        palette.accent
    } else {
        palette.text
    }
}

/// A tab's background: connection tabs (`radius` = `look.tab_radius`) and
/// object tabs (`look.radius`).
pub fn tab(
    ui: &Ui,
    rect: Rect,
    active: bool,
    hovered: bool,
    radius: u8,
    look: &Look,
    palette: &Palette,
) {
    let painter = ui.painter();
    let corner = CornerRadius::same(radius);
    match look.tabs {
        TabStyle::Outlined | TabStyle::Raised => {
            if active {
                if look.tabs == TabStyle::Raised {
                    let shadow = egui::epaint::Shadow {
                        offset: [0, 1],
                        blur: 4,
                        spread: 0,
                        color: palette.shadow.gamma_multiply(0.6),
                    };
                    painter.add(shadow.as_shape(rect, corner));
                }
                painter.rect_filled(rect, corner, palette.window);
                if look.tabs == TabStyle::Outlined {
                    painter.rect_stroke(
                        rect,
                        corner,
                        Stroke::new(1.0, palette.outline),
                        StrokeKind::Inside,
                    );
                }
            } else if hovered {
                painter.rect_filled(rect, corner, palette.surface_hover);
            }
        }
        TabStyle::Underline => {
            if active {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
                let line = Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - 2.0),
                    rect.max,
                );
                painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
            } else if hovered {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
            }
        }
    }
}

/// A card: the picker's saved connections.
pub fn card(ui: &Ui, rect: Rect, hovered: bool, look: &Look, palette: &Palette) {
    let painter = ui.painter();
    // Cards round a little more than rows: 6 standard (today), 10 macOS, 0 Omarchy.
    let corner = CornerRadius::same(if look.radius == 0 { 0 } else { look.radius + 2 });
    let fill = if hovered {
        palette.surface_hover
    } else {
        palette.panel
    };
    painter.rect_filled(rect, corner, fill);
    if look.bordered_controls {
        let border = if hovered { 0.40 } else { 0.25 };
        painter.rect_stroke(
            rect,
            corner,
            Stroke::new(1.0, palette.text.gamma_multiply(border)),
            StrokeKind::Inside,
        );
    }
}

/// A search field: a capsule on macOS, square on Omarchy.
pub fn search_field<'t>(text: &'t mut String, hint: &str, look: &Look) -> egui::TextEdit<'t> {
    let field = single(text, look).hint_text(hint);
    if look.capsule_search {
        field.margin(egui::Margin::symmetric(12, 4))
    } else {
        field
    }
}

/// The one accent-filled button in a dialog or view.
pub fn primary_button(ui: &mut Ui, text: &str, look: &Look, palette: &Palette) -> Response {
    let button = egui::Button::new(
        egui::RichText::new(text)
            .font(crate::theme::medium(crate::theme::TEXT))
            .color(palette.on_accent),
    )
    .fill(palette.accent)
    .corner_radius(CornerRadius::same(look.radius))
    .min_size(vec2(0.0, look.control_height));
    ui.add(button)
}

/// A dialog: a soft shadow over a dimmed window, or Omarchy's accent border
/// over a scrim.
pub fn modal(id: egui::Id, look: &Look, palette: &Palette) -> egui::Modal {
    let frame = egui::Frame::new()
        .fill(palette.overlay)
        .corner_radius(CornerRadius::same(look.dialog_radius))
        .inner_margin(egui::Margin::same(20));
    match look.dialog {
        DialogStyle::Shadow => egui::Modal::new(id)
            .frame(
                frame
                    .stroke(Stroke::new(1.0, palette.outline))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 10],
                        blur: 36,
                        spread: 0,
                        color: palette.shadow,
                    }),
            )
            .backdrop_color(Color32::from_black_alpha(if palette.dark { 120 } else { 70 })),
        DialogStyle::AccentBorder => {
            let [r, g, b, _] = palette.window.to_array();
            egui::Modal::new(id)
                .frame(frame.stroke(Stroke::new(2.0, palette.accent)))
                .backdrop_color(Color32::from_rgba_unmultiplied(r, g, b, 128))
        }
    }
}
```

`TextEdit` takes its corner radius from `visuals.widgets.*.corner_radius`, so the capsule comes from a scoped style in `add_search`:

```rust
/// Adds a search field; macOS rounds it into a capsule.
pub fn add_search(ui: &mut Ui, field: egui::TextEdit<'_>, look: &Look) -> Response {
    ui.scope(|ui| {
        if look.capsule_search {
            let corner = CornerRadius::same((look.control_height / 2.0) as u8);
            let widgets = &mut ui.visuals_mut().widgets;
            for state in [&mut widgets.inactive, &mut widgets.hovered, &mut widgets.active] {
                state.corner_radius = corner;
            }
        }
        ui.add(field)
    })
    .inner
}
```

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: PASS. Clippy may flag the painters as unused until Task 4. If it does, add them to Task 4 in the same session, or run clippy after Task 4. Do not add `allow(dead_code)`.

- [ ] **Step 5: Run all checks and commit** (clippy at the latest after Task 4, as above)

```bash
git add src/ui/widgets.rs src/ui/mod.rs src/testing.rs
git commit -m "Add shared painters for selection, tabs, cards, search, buttons and dialogs

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Sidebar, picker and quick open on the shared painters

**Files:**
- Modify: `src/ui/sidebar.rs:14,66-100,130-140,170-245`
- Modify: `src/ui/picker.rs` (whole body)
- Modify: `src/ui/quick_open.rs:22-70`
- Modify: `src/ui/mod.rs:28-45` (let the picker background be `window`)

**Interfaces:**
- Consumes: `selection`, `selection_text`, `card`, `search_field`, `add_search`, `primary_button`, `modal` (Task 3); `look.tree_row`, `look.sidebar_tinted`, `look.panel_separators`.

- [ ] **Step 1: Sidebar.**
  - Delete `const ROW_HEIGHT` and use `look.tree_row` everywhere it was used. Pass the look into `tree_row` by adding a `look: &crate::theme::Look` parameter after `palette`.
  - Panel frame: `.fill(if look.sidebar_tinted { palette.panel } else { palette.window })` and `.show_separator_line(look.panel_separators)`.
  - Filter: replace `crate::ui::widgets::single(&mut workspace.tree.filter).hint_text(...)` inside `ui.add_sized` with:

```rust
let field = crate::ui::widgets::search_field(
    &mut workspace.tree.filter,
    &gettext(locale, "Filter"),
    &look,
);
ui.allocate_ui(size, |ui| {
    ui.set_min_size(size);
    crate::ui::widgets::add_search(ui, field.desired_width(size.x), &look)
});
```

  - In `tree_row`, replace the selected and hovered `rect_filled` block with `crate::ui::widgets::selection(ui, rect, selected, response.hovered(), look, palette);`. Keep the cursor stroke, using `CornerRadius::same(look.radius)`.
  - Object text colour: `TreeNode::Object(..) | TreeNode::Empty(_) => (theme::regular(theme::TEXT), crate::ui::widgets::selection_text(selected, look, palette))`.

- [ ] **Step 2: Picker.** Rewrite `picker::show` around cards:
  - `const ROW_HEIGHT: f32 = 56.0;` and `const LIST_WIDTH: f32 = 560.0;`
  - Title: `ui.add_space(64.0);` then the heading as today, then `ui.add_space(4.0);` and a subtitle, `RichText::new(gettext(locale, "Choose a saved connection or create a new one")).color(palette.secondary)`.
  - Search row: `search_field(&mut picker.search, &gettext(locale, "Search connections"), &look).desired_width(LIST_WIDTH - 170.0)` added with `add_search`. The New connection button becomes `primary_button(ui, &gettext(locale, "New connection"), &look, &palette)`.
  - Each row: allocate `vec2(LIST_WIDTH, ROW_HEIGHT)`, call `card(ui, rect, response.hovered(), &look, &palette)`, then `ui.add_space(8.0)` between cards. The inner row uses `rect.shrink2(vec2(14.0, 0.0))`. The name uses `theme::medium(theme::TEXT)` and the summary `theme::regular(theme::TEXT_SMALL)`, both as today. Keep every `widget_info`, context menu and icon-button label unchanged, because tests find them by name.
  - The new string "Choose a saved connection or create a new one" goes through `gettext`. Check `assets/i18n/` for a catalogue (today only `.gitkeep`). If one exists, add the msgid; if not, nothing more to do.

- [ ] **Step 3: Quick open.**
  - `egui::Modal::new(egui::Id::new("quick-open"))` becomes `crate::ui::widgets::modal(egui::Id::new("quick-open"), &look, &palette)`.
  - The query field becomes `search_field(&mut open.query, &gettext(locale, "Name"), &look).desired_width(f32::INFINITY)`, added with `add_search`.
  - Replace each `ui.selectable_label(index == open.selected, label)` with an allocated row:

```rust
let (rect, response) = ui.allocate_exact_size(
    egui::vec2(ui.available_width(), look.tree_row),
    egui::Sense::click(),
);
response.widget_info(|| {
    egui::WidgetInfo::selected(egui::WidgetType::SelectableLabel, true, index == open.selected, &label)
});
crate::ui::widgets::selection(ui, rect, index == open.selected, response.hovered(), &look, &palette);
ui.painter().text(
    egui::pos2(rect.left() + 10.0, rect.center().y),
    egui::Align2::LEFT_CENTER,
    &label,
    theme::regular(theme::TEXT),
    crate::ui::widgets::selection_text(index == open.selected, &look, &palette),
);
```

  Keep `scroll_to_me` and `clicked` handling on `response`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: PASS. The quick open, picker, sidebar and every-look tests all run.

- [ ] **Step 5: Render and review.** Run `cargo test --features shots --lib shots::shots -- --ignored`, then open `target/shots/picker-{macos,omarchy,standard}-{light,dark}.png` and `workspace-*.png` and check:
  - the Omarchy selection is a full-width bar with an accent edge and accent text
  - the macOS selection is an inset pill
  - picker cards are rounded on macOS and square with a border on Omarchy

- [ ] **Step 6: Run all checks and commit**

```bash
git add src/ui
git commit -m "Draw the sidebar, picker and quick open with the platform look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 5: Connection tabs, object tabs, top bar and footer

**Files:**
- Modify: `src/ui/conn_tabs.rs:25-60,86-160`
- Modify: `src/ui/object_tabs.rs:14-120`
- Modify: `src/ui/workspace.rs:11,90-100,190-200`
- Modify: `src/ui/data_view.rs:44-52`

**Interfaces:**
- Consumes: `widgets::tab` (Task 3), `look.tab_radius`, `look.radius`, `look.control_height`, `look.panel_separators`.

- [ ] **Step 1: Connection tabs.** In `tab()`:
  - Delete the fill/`rect_fill`/`rect_stroke` block and call `crate::ui::widgets::tab(ui, rect, active, response.hovered(), look.tab_radius, &look, &palette);`, with `let look = app.look;` at the top.
  - The panel frame fill stays `palette.panel`.
  - Close buttons use `icon_button(..., &look, &palette)`, already done in Task 1.
  - The `Underline` style needs the tab flush with the bar's bottom edge, so its underline touches the content below. In `show`, build the frame margin per style, keeping the tab height `HEIGHT - 8.0`:

```rust
let margin = if look.tabs == crate::theme::TabStyle::Underline {
    Margin { left: 6, right: 6, top: 2 * vertical, bottom: 0 }
} else {
    Margin::symmetric(6, vertical)
};
```

    Then use `.inner_margin(margin)`. The macOS title-bar centring only applies to the `Raised` style, so it is unaffected.

- [ ] **Step 2: Object tabs.** Same change: `crate::ui::widgets::tab(ui, rect, is_active, response.hovered(), look.radius, &look, &palette);`, and the panel `.show_separator_line(look.panel_separators)`.

- [ ] **Step 3: Workspace top bar and footer.**
  - Delete `TOP_BAR_HEIGHT` and use `.exact_size(app.look.control_height + 8.0)`, capturing `let look = app.look;` before the panel.
  - Top bar: `.show_separator_line(look.panel_separators)`.
  - The banner's Reconnect button becomes `crate::ui::widgets::primary_button(ui, &gettext(locale, "Reconnect"), &look, &palette).clicked()`.
  - `data_view::footer`: `.exact_size(look.control_height + 8.0)` and `.show_separator_line(look.panel_separators)`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: PASS. In particular the tab tests in `ui/mod.rs`, `a_disconnected_workspace_offers_reconnect`, and `every_look_lays_out_at_small_and_large_sizes`.

- [ ] **Step 5: Render and review** `workspace-*`, `state-disconnected-*` and `structure-*`:
  - the macOS active tab is raised with no outline, and sits on the traffic lights' line
  - the Omarchy active tab is underlined, square and flush with the bar's bottom
  - the standard look is unchanged apart from the palette

- [ ] **Step 6: Run all checks and commit**

```bash
git add src/ui
git commit -m "Draw tabs, top bar and footer with the platform look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 6: Dialogs, primary buttons and the filter bar

**Files:**
- Modify: `src/ui/connect_dialog.rs:40,244-262`
- Modify: `src/ui/password_prompt.rs:17,45-55`
- Modify: `src/ui/host_key_prompt.rs:17,45-55`
- Modify: `src/ui/help.rs:18`
- Modify: `src/ui/filter_bar.rs:150-155`
- Modify: `src/ui/workspace.rs:40-55` (filter bar panel)
- Modify: `src/ui/row_panel.rs:53-58` (field filter)

**Interfaces:**
- Consumes: `modal`, `primary_button`, `search_field`, `add_search` (Task 3).

- [ ] **Step 1: Write the failing test** in `src/ui/mod.rs` `mod tests`. It checks that the dialog still works end to end under the Omarchy look, which is the one that changes frames most:

```rust
    #[test]
    fn dialogs_open_and_close_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.press(Key::N, Modifiers::COMMAND);
            assert!(harness.has("Save & Connect"), "{}", look.name);
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(harness.app.dialog.is_none(), "{}", look.name);
        }
    }
```

- [ ] **Step 2: Run it**

Run: `cargo test --lib dialogs_open_and_close_in_every_look`
Expected: PASS already. It guards the refactor that follows.

- [ ] **Step 3: Replace the modals and buttons.**
  - `connect_dialog.rs:40`: `egui::Modal::new(egui::Id::new("connection-dialog"))` becomes `crate::ui::widgets::modal(egui::Id::new("connection-dialog"), &look, &palette)`. Same change in `password_prompt.rs:17` (`"password-prompt"`), `host_key_prompt.rs:17` (`"host-key-prompt"`) and `help.rs:18` (`"help"`). Capture `let look = app.look;` next to `palette` in each.
  - `connect_dialog.rs:248-256`: the hand-built accent button becomes `if crate::ui::widgets::primary_button(ui, &gettext(locale, "Save & Connect"), &look, &palette).clicked() {`.
  - `password_prompt.rs`: "Connect" becomes `primary_button`. Keep `|| entered`.
  - `host_key_prompt.rs`: "Trust and connect" becomes `primary_button`.
  - `filter_bar.rs:150`: the Apply button becomes `primary_button(ui, &gettext(locale, "Apply"), &look, &palette)`.
  - `workspace.rs` filter-bar panel: `.show_separator_line(look.panel_separators)`.
  - `row_panel.rs:53`: the field filter becomes `search_field(&mut filter, &gettext(locale, "Filter fields"), &look).desired_width(f32::INFINITY)`, added with `add_search`.

- [ ] **Step 4: Run the tests**

Run: `cargo test --lib`
Expected: PASS, with the connect dialog, password prompt, host key and filter bar tests included.

- [ ] **Step 5: Render and review** `dialog-*` and `filter-*`:
  - Omarchy: square dialog, 2 px accent border, window-tinted scrim
  - macOS: 12 pt radius and a soft wide shadow

- [ ] **Step 6: Run all checks and commit**

```bash
git add src/ui
git commit -m "Draw dialogs and primary buttons with the platform look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 7: Monospace data and the desktop monospace font on Linux

**Files:**
- Create: `src/theme/desktop_font.rs`
- Modify: `src/theme.rs` (`mod desktop_font;`, `install`, `data`)
- Modify: `Cargo.toml` (Linux-only `skrifa`)
- Modify: `src/ui/grid.rs:180-250` (cell font, row hover)
- Modify: `src/ui/row_panel.rs:115-125` (value font)

**Interfaces:**
- Consumes: `Look.data_font` (Task 1).
- Produces:
  - `theme::data(look: &Look) -> FontId`
  - `desktop_font::parse_fc_match(output: &str) -> Option<(PathBuf, u32)>`
  - `desktop_font::load(path: &Path, index: u32) -> Option<egui::FontData>`
  - `desktop_font::monospace() -> Option<egui::FontData>`, which returns `None` outside Linux

- [ ] **Step 1: Add the dependency.** In `Cargo.toml` under `[target.'cfg(target_os = "linux")'.dependencies]`:

```toml
# Checks that the desktop's monospace font parses before egui loads it (egui
# panics on a font it cannot read). The version epaint already uses.
skrifa = { version = "0.44", default-features = false, features = ["std"] }
```

- [ ] **Step 2: Write the failing tests** in `src/theme/desktop_font.rs`:

```rust
//! The desktop's monospace font on Linux, as fontconfig resolves it
//! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it).

use std::path::{Path, PathBuf};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fc_match_output_parses_path_and_index() {
        assert_eq!(
            parse_fc_match("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf\n0"),
            Some((PathBuf::from("/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf"), 0))
        );
        assert_eq!(
            parse_fc_match("/usr/share/fonts/noto/NotoSansMono.ttc\n2\n"),
            Some((PathBuf::from("/usr/share/fonts/noto/NotoSansMono.ttc"), 2))
        );
        assert_eq!(parse_fc_match(""), None);
        assert_eq!(parse_fc_match("relative.ttf\n0"), None);
        assert_eq!(parse_fc_match("/x.ttf\nnot-a-number"), None);
    }

    #[test]
    fn an_unparsable_desktop_font_is_ignored() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.ttf");
        std::fs::write(&path, b"not a font").unwrap();
        assert!(load(&path, 0).is_none());
        assert!(load(&dir.path().join("missing.ttf"), 0).is_none());
    }

    #[test]
    fn a_real_font_loads_with_its_index() {
        // egui's bundled monospace (Hack) is a valid TTF.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hack.ttf");
        let hack = egui::FontDefinitions::default().font_data["Hack"].font.to_vec();
        std::fs::write(&path, hack).unwrap();
        let data = load(&path, 0).expect("loads");
        assert_eq!(data.index, 0);
        assert!(load(&path, 5).is_none(), "no face 5 in a single-face file");
    }
}
```

The `load` tests use skrifa, a Linux-only dependency. Gate the module's tests with `#[cfg(all(test, target_os = "linux"))]` and `load` with `#[cfg(target_os = "linux")]`. `parse_fc_match` is plain string parsing, so give it its own test module with no gate.

- [ ] **Step 3: Run the tests to verify they fail**

Run: `cargo test --lib desktop_font`
Expected: compile error, because the functions are not defined.

- [ ] **Step 4: Implement it** in `src/theme/desktop_font.rs`, above the tests:

```rust
/// Reads `fc-match -f '%{file}\n%{index}'`: an absolute path and a face index.
pub fn parse_fc_match(output: &str) -> Option<(PathBuf, u32)> {
    let mut lines = output.lines();
    let path = PathBuf::from(lines.next()?.trim());
    let index = lines.next()?.trim().parse().ok()?;
    path.is_absolute().then_some((path, index))
}

/// The face at `index` in `path`, if it exists and parses.
#[cfg(target_os = "linux")]
pub fn load(path: &Path, index: u32) -> Option<egui::FontData> {
    let bytes = std::fs::read(path).ok()?;
    skrifa::FontRef::from_index(&bytes, index).ok()?;
    let mut data = egui::FontData::from_owned(bytes);
    data.index = index;
    Some(data)
}

/// The desktop's monospace font, or `None` (logged) when fontconfig is
/// missing or names a file egui could not read.
pub fn monospace() -> Option<egui::FontData> {
    #[cfg(target_os = "linux")]
    {
        let output = std::process::Command::new("fc-match")
            .args(["-f", "%{file}\n%{index}", "monospace"])
            .output()
            .ok()
            .filter(|output| output.status.success())?;
        let text = String::from_utf8_lossy(&output.stdout);
        let Some((path, index)) = parse_fc_match(&text) else {
            log::warn!("fc-match gave no monospace font");
            return None;
        };
        let font = load(&path, index);
        if font.is_none() {
            log::warn!("could not read the monospace font {}", path.display());
        }
        font
    }
    #[cfg(not(target_os = "linux"))]
    {
        None
    }
}
```

On non-Linux targets `Path` is unused. Import it only under `#[cfg(target_os = "linux")]` so clippy stays clean on macOS and Windows.

- [ ] **Step 5: Use it in `theme.rs`.** Add `mod desktop_font;` near the top. In `install`, choose the monospace face:

```rust
pub fn install(ctx: &egui::Context, system_fallbacks: bool, look: &Look) {
    let mut setup = fastframe_fonts::FontSetup::default().system_fallbacks(system_fallbacks);
    // Tests and screenshots (no system fallbacks) stay reproducible.
    if system_fallbacks
        && look.data_font == DataFont::Monospace
        && let Some(font) = desktop_font::monospace()
    {
        setup = setup.monospace(fastframe_fonts::Monospace::Font {
            name: "desktop-monospace".into(),
            data: std::sync::Arc::new(font),
        });
    }
    let mut fonts = setup.definitions();
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
}

/// The font data is drawn in: monospace on Omarchy, Inter elsewhere.
pub fn data(look: &Look) -> FontId {
    match look.data_font {
        DataFont::Monospace => mono(TEXT_MONO),
        DataFont::Proportional => regular(TEXT),
    }
}
```

Add a test to `theme::tests`:

```rust
    #[test]
    fn data_is_monospace_only_in_the_omarchy_look() {
        assert_eq!(data(&Look::omarchy()).family, egui::FontFamily::Monospace);
        assert_eq!(data(&Look::macos()).family, egui::FontFamily::Proportional);
        assert_eq!(data(&Look::standard()).family, egui::FontFamily::Proportional);
    }
```

- [ ] **Step 6: Grid and row panel.**
  - `grid.rs`: the `grid::show` signature gains `look: &crate::theme::Look`. The only caller is `data_view::show`, which passes `&app.look`.
  - Replace `let font = theme::regular(theme::TEXT);` in the cell loop with `let font = theme::data(look);`.
  - Column widths are measured with the same `font` where `initial_widths` is called. Find the `let font =` feeding `initial_widths` near `grid.rs:145` and change it to `theme::data(look)` too, so mono columns size to mono text.
  - Row hover: after the `selected_row` branch, add `else if response.hovered() { painter.rect_filled(rect, CornerRadius::ZERO, palette.surface_hover.gamma_multiply(0.6)); }` before the zebra branch.
  - `row_panel.rs:120-124`: the non-JSON value font `theme::regular(theme::TEXT)` becomes `theme::data(&look)`.
  - `structure.rs` already draws type names in `theme::mono`, so it needs no change. The spec listed it; this is a deliberate no-op.

- [ ] **Step 7: Run the tests**

Run: `cargo test --lib`
Expected: PASS, with the grid tests (`grid::tests`, copy and selection tests) included.

- [ ] **Step 8: Try it for real on this Omarchy machine.**
  - Run `cargo run -- --demo`. The grid and row panel should draw in JetBrainsMono Nerd Font, and the chrome in Inter.
  - Run `FONTCONFIG_FILE=/dev/null cargo run -- --demo` to make fc-match fail. The app should start with the bundled monospace and log one warning.

- [ ] **Step 9: Run all checks and commit**

```bash
git add Cargo.toml Cargo.lock src
git commit -m "Draw data in the desktop monospace font in the Omarchy look

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 8: Screenshot review, README, and platform checks

**Files:**
- Modify: `src/shots.rs` (only if a scene needs adjusting)
- Modify: `docs/screenshots/workspace-light.png`, `docs/screenshots/workspace-dark.png`
- Create: `docs/screenshots/workspace-omarchy.png`
- Modify: `README.md`

- [ ] **Step 1: Render everything**

Run: `cargo test --features shots --lib shots -- --ignored`
Expected: 9 scenes x 3 looks x 2 themes = 54 PNGs in `target/shots/`.

- [ ] **Step 2: Review by eye.** For each look and theme, open `workspace`, `picker`, `dialog`, `filter`, `state-*` and `structure`, and check:
  - no clipped text in bars at 28 pt controls
  - tabs sit centred on the title-bar line in macOS
  - the Omarchy underline is flush with the bar
  - no rounded corners anywhere in Omarchy
  - borders are visible in Omarchy light
  - dim text is readable in the warm palettes

  Fix any defect in the task's own file and commit it as `Fix <what> in the <look> look`.

- [ ] **Step 3: Refresh the README screenshots.** Copy the new shots:
  - `target/shots/workspace-macos-light.png` to `docs/screenshots/workspace-light.png`
  - `target/shots/workspace-macos-dark.png` to `docs/screenshots/workspace-dark.png`
  - `target/shots/workspace-omarchy-dark.png` to `docs/screenshots/workspace-omarchy.png`

  Downscale each to 1000 px wide as before, with `magick in.png -resize 1000x out.png` if ImageMagick is installed, otherwise copy them unscaled. In `README.md`:
  - replace the "What it does" bullet "Follows the Omarchy theme live on Omarchy, and the system light/dark setting elsewhere." with "Looks native on each platform: a macOS look, and on Linux the Omarchy look (square, bordered, monospace data) following the Omarchy theme live."
  - add `![Tabletist on Omarchy](docs/screenshots/workspace-omarchy.png)` after the dark screenshot.

- [ ] **Step 4: Platform compile checks**

Run: `rustup target add aarch64-apple-darwin x86_64-pc-windows-msvc && cargo check --workspace --target aarch64-apple-darwin && cargo check --workspace --target x86_64-pc-windows-msvc`
Expected: both check cleanly. If a target can't be checked here, for example because a C dependency needs the platform SDK, record which one and why for the final report. Do not claim it compiled.

- [ ] **Step 5: Final checks and commit**

Run all four checks from Global Constraints.

```bash
git add README.md docs/screenshots
git commit -m "Refresh the README screenshots for the platform looks

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```
