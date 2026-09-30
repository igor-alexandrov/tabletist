//! Palette, typography, icons, and the mapping onto egui's style.

pub mod desktop_font;
pub mod faces;

use egui::{Color32, CornerRadius, FontId, Stroke, Vec2};

/// A palette file from the themes directory.
pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
/// The palette files, the shared presets, and Omarchy's live palette.
pub type Catalog = fastframe_theme::Catalog<Palette>;

// The type scale, in points at 1x. Every text size in the interface comes
// from here or from the look ([`Look::title`], [`Look::heading`]).
/// Body text: tree, grid, tabs, fields, buttons.
pub const TEXT: f32 = 13.0;
/// Secondary text: counts, subtitles, the status bar, key hints.
pub const TEXT_SMALL: f32 = 12.0;
/// Field labels and captions.
pub const TEXT_LABEL: f32 = 11.5;
/// Section labels, type lines under column names, shortcut hints.
pub const TEXT_CAPTION: f32 = 11.0;
/// Dialog titles.
pub const TEXT_TITLE: f32 = 17.0;
/// The picker's heading.
pub const TEXT_HEADING: f32 = 20.0;
/// Raw SQL and hex.
pub const TEXT_MONO: f32 = 12.0;

// Spacing, on a 4 pt grid.
/// Padding inside panels and bars.
pub const PAD: i8 = 12;

/// Every colour the interface draws with. The first sixteen names match
/// fastframe's base colours so palette files and Omarchy can set them; the
/// rest are Tabletist's own, derived from those when a file leaves them out.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Palette {
    pub dark: bool,
    /// The content: grid, row panel, object header.
    pub window: Color32,
    /// Sidebar, status bar, table headers.
    pub panel: Color32,
    /// Segmented tracks, chips, grid row dividers.
    pub surface: Color32,
    /// The object tab strip, disabled buttons, minor dividers.
    pub surface_hover: Color32,
    pub surface_active: Color32,
    /// Pane dividers and major borders.
    pub outline: Color32,
    pub text: Color32,
    /// Icons, inactive tabs.
    pub secondary: Color32,
    /// Labels, counts, types.
    pub dim: Color32,
    pub accent: Color32,
    /// Selected text, hovered links.
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub overlay: Color32,
    pub shadow: Color32,
    /// Name prefixes, the "view" tag, disabled text.
    pub faint: Color32,
    /// Fields and secondary buttons.
    pub border: Color32,
    /// The selected row or item.
    pub selection: Color32,
    /// Verified TLS, JSON strings, the dev environment.
    pub success: Color32,
    /// Cool value tags.
    pub info: Color32,
    /// Warm value tags, JSON numbers.
    pub orange: Color32,
    /// The local environment.
    pub magenta: Color32,
}

/// The colours Tabletist adds to fastframe's base ones.
pub const EXTRA_COLORS: [&str; 7] = [
    "faint",
    "border",
    "selection",
    "success",
    "info",
    "orange",
    "magenta",
];

impl Palette {
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
            dim: Color32::from_rgb(0x9a, 0x97, 0x91),
            accent: Color32::from_rgb(0x5a, 0x9f, 0xf8),
            accent_hover: Color32::from_rgb(0x7a, 0xb2, 0xf9),
            on_accent: Color32::from_rgb(0x0b, 0x13, 0x20),
            danger: Color32::from_rgb(0xf2, 0x70, 0x7a),
            warning: Color32::from_rgb(0xea, 0xb3, 0x5a),
            // The panel tone: fields and buttons (surface) must stand off
            // a dialog as they do off the sidebar.
            overlay: Color32::from_rgb(0x26, 0x25, 0x23),
            shadow: Color32::from_black_alpha(150),
            faint: Color32::from_rgb(0x85, 0x81, 0x7b),
            border: Color32::from_rgb(0x48, 0x45, 0x41),
            selection: Color32::from_rgb(0x28, 0x34, 0x57),
            success: Color32::from_rgb(0x9e, 0xce, 0x6a),
            info: Color32::from_rgb(0x7d, 0xcf, 0xff),
            orange: Color32::from_rgb(0xff, 0x9e, 0x64),
            magenta: Color32::from_rgb(0xbb, 0x9a, 0xf7),
        }
    }

    /// The macOS design's light colours.
    pub fn light() -> Self {
        Self {
            dark: false,
            window: Color32::WHITE,
            panel: Color32::from_rgb(0xfb, 0xfa, 0xf8),
            surface: Color32::from_rgb(0xf0, 0xef, 0xeb),
            surface_hover: Color32::from_rgb(0xeb, 0xe9, 0xe4),
            surface_active: Color32::from_rgb(0xe3, 0xe1, 0xdc),
            outline: Color32::from_rgb(0xe3, 0xe1, 0xdc),
            text: Color32::from_rgb(0x1c, 0x1c, 0x1a),
            secondary: Color32::from_rgb(0x4d, 0x4c, 0x48),
            // 4.5:1 on every light surface.
            dim: Color32::from_rgb(0x6b, 0x6a, 0x65),
            // Accent, danger and warning are read as text: 4.5:1 or better.
            accent: Color32::from_rgb(0x2c, 0x55, 0xc9),
            accent_hover: Color32::from_rgb(0x1e, 0x3f, 0x9e),
            on_accent: Color32::WHITE,
            danger: Color32::from_rgb(0xa3, 0x23, 0x1b),
            warning: Color32::from_rgb(0x8a, 0x5a, 0x00),
            overlay: Color32::WHITE,
            shadow: Color32::from_rgba_unmultiplied(28, 28, 26, 71),
            faint: Color32::from_rgb(0x8a, 0x89, 0x84),
            border: Color32::from_rgb(0xdc, 0xda, 0xd4),
            selection: Color32::from_rgb(0xe4, 0xeb, 0xfb),
            success: Color32::from_rgb(0x14, 0x6b, 0x3a),
            info: Color32::from_rgb(0x1e, 0x4f, 0x8a),
            orange: Color32::from_rgb(0x9a, 0x4a, 0x0b),
            magenta: Color32::from_rgb(0x5b, 0x3a, 0xa8),
        }
    }

    /// Lightens (or darkens) the label colours until they read against the
    /// content: a theme's muted colour can be too faint (Tokyo Night's is).
    pub fn with_readable_labels(mut self) -> Self {
        for (color, minimum) in [(&mut self.secondary, 4.5), (&mut self.dim, 4.5)] {
            let target = if self.dark {
                Color32::WHITE
            } else {
                Color32::BLACK
            };
            let mut step = 0.0;
            while contrast(*color, self.window) < minimum && step < 1.0 {
                step += 0.02;
                *color = color.lerp_to_gamma(target, 0.02);
            }
        }
        self
    }
}

impl fastframe_theme::Palette for Palette {
    fn base(base: fastframe_theme::Base) -> Self {
        match base {
            fastframe_theme::Base::Dark => Self::dark(),
            fastframe_theme::Base::Light => Self::light(),
        }
    }

    fn set(&mut self, name: &str, color: Color32) -> bool {
        match name {
            "window" => self.window = color,
            "panel" => self.panel = color,
            "surface" => self.surface = color,
            "surface_hover" => self.surface_hover = color,
            "surface_active" => self.surface_active = color,
            "outline" => self.outline = color,
            "text" => self.text = color,
            "secondary" => self.secondary = color,
            "dim" => self.dim = color,
            "accent" => self.accent = color,
            "accent_hover" => self.accent_hover = color,
            "on_accent" => self.on_accent = color,
            "danger" => self.danger = color,
            "warning" => self.warning = color,
            "overlay" => self.overlay = color,
            "shadow" => self.shadow = color,
            "faint" => self.faint = color,
            "border" => self.border = color,
            "selection" => self.selection = color,
            "success" => self.success = color,
            "info" => self.info = color,
            "orange" => self.orange = color,
            "magenta" => self.magenta = color,
            _ => return false,
        }
        true
    }

    /// Dialogs and popups take the panel colour when a file (such as the
    /// Omarchy template) sets the panel but not the overlay, so they follow
    /// the theme rather than keeping the default's. A file that sets only
    /// the base colours gets Tabletist's own from them.
    fn derive(&mut self, given: &std::collections::BTreeSet<&str>) {
        if given.contains("panel") && !given.contains("overlay") {
            self.overlay = self.panel;
        }
        if given.contains("dim") && !given.contains("faint") {
            self.faint = self.dim;
        }
        if given.contains("outline") && !given.contains("border") {
            self.border = self.outline;
        }
        if given.contains("accent") && !given.contains("selection") {
            self.selection = self.window.lerp_to_gamma(self.accent, 0.2);
        }
    }
}

/// The WCAG contrast ratio of two opaque colours.
pub fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

fn luminance(color: egui::Color32) -> f64 {
    let channel = |c: u8| {
        let c = f64::from(c) / 255.0;
        if c <= 0.039_28 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * channel(color.r()) + 0.7152 * channel(color.g()) + 0.0722 * channel(color.b())
}

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

/// The typefaces a look draws with (see [`faces`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Faces {
    /// Inter, with egui's monospace.
    Inter,
    /// IBM Plex Sans and IBM Plex Mono.
    Plex,
    /// One monospace face for everything.
    Terminal,
}

/// How the one primary button in a view draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryStyle {
    /// Filled with the accent.
    Accent,
    /// Filled with the text colour (macOS: near-black on light).
    Ink,
    /// Outlined in the accent, accent text on a faint accent fill.
    Outline,
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
    /// Grid rows.
    pub grid_row: f32,
    /// Omarchy's controls: a faint foreground fill and a 1 px border.
    pub bordered_controls: bool,
    pub selection: Selection,
    pub tabs: TabStyle,
    pub dialog: DialogStyle,
    /// Search fields are fully rounded.
    pub capsule_search: bool,
    /// The sidebar takes the panel colour rather than the window's.
    pub sidebar_tinted: bool,
    /// Bars (top bar, object tabs, filter bar, footer) are separated from
    /// the content by 1 px lines. The sidebar and the row panel always are:
    /// one tone off the content is too faint to mark a pane's edge.
    pub panel_separators: bool,
    /// Pop-up buttons (combo boxes) stand off their background with a
    /// raised fill and a soft shadow, and show up and down chevrons.
    pub raised_popups: bool,
    pub data_font: DataFont,
    /// Grid cells and row-panel values.
    pub data_size: f32,
    /// JSON in the row panel.
    pub json_size: f32,
    /// The open object's name over the grid.
    pub title: f32,
    /// Screen titles (Connections).
    pub heading: f32,
    pub faces: Faces,
    pub primary: PrimaryStyle,
    /// A keyboard-first terminal interface (Omarchy): lower-case labels
    /// that lead with their keys, and key hints in bars.
    pub terminal: bool,
}

impl Look {
    pub const ALL: [Look; 3] = [Self::standard(), Self::macos(), Self::omarchy()];

    /// Windows.
    pub const fn standard() -> Self {
        Self {
            name: "standard",
            radius: 4,
            tab_radius: 6,
            dialog_radius: 8,
            control_height: 26.0,
            tree_row: 24.0,
            grid_row: 26.0,
            bordered_controls: false,
            selection: Selection::Tint,
            tabs: TabStyle::Outlined,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            raised_popups: false,
            data_font: DataFont::Proportional,
            data_size: TEXT,
            json_size: TEXT_MONO,
            title: 17.0,
            heading: TEXT_HEADING,
            faces: Faces::Inter,
            primary: PrimaryStyle::Accent,
            terminal: false,
        }
    }

    /// macOS 26: Plex type, bordered buttons, an inset pill selection and
    /// ink primary buttons.
    pub const fn macos() -> Self {
        Self {
            name: "macos",
            radius: 8,
            tab_radius: 8,
            dialog_radius: 12,
            control_height: 28.0,
            tree_row: 22.0,
            grid_row: 34.0,
            bordered_controls: false,
            selection: Selection::Pill,
            tabs: TabStyle::Raised,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            raised_popups: true,
            data_font: DataFont::Monospace,
            data_size: 12.5,
            json_size: 11.5,
            title: 17.0,
            heading: TEXT_HEADING,
            faces: Faces::Plex,
            primary: PrimaryStyle::Ink,
            terminal: false,
        }
    }

    /// Omarchy's shell: square, 1 px borders, monospace everything, keys
    /// first.
    pub const fn omarchy() -> Self {
        Self {
            name: "omarchy",
            radius: 0,
            tab_radius: 0,
            dialog_radius: 0,
            control_height: 24.0,
            tree_row: 17.0,
            grid_row: 21.0,
            bordered_controls: true,
            selection: Selection::Bar,
            tabs: TabStyle::Underline,
            dialog: DialogStyle::AccentBorder,
            capsule_search: false,
            sidebar_tinted: false,
            panel_separators: true,
            raised_popups: false,
            data_font: DataFont::Monospace,
            data_size: TEXT,
            json_size: TEXT_SMALL,
            title: 15.0,
            heading: 14.0,
            faces: Faces::Terminal,
            primary: PrimaryStyle::Outline,
            terminal: true,
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

    /// The command modifier as shortcuts spell it: `⌘` on macOS, `Ctrl+`
    /// elsewhere.
    pub fn command_key(&self) -> &'static str {
        if self.faces == Faces::Plex {
            "⌘"
        } else {
            "Ctrl+"
        }
    }

    /// `text` as this look labels things: lower case in the terminal look.
    pub fn label(&self, text: &str) -> String {
        if self.terminal {
            text.to_lowercase()
        } else {
            text.to_owned()
        }
    }
}

/// How an environment draws: its colour (the bar's stripe and tint), and
/// its badge's fill and text.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EnvColors {
    pub color: Color32,
    pub badge: Color32,
    pub badge_text: Color32,
}

/// An environment's colours: fixed on light palettes (the macOS design's),
/// from the theme's own colours on dark ones, with a solid badge.
pub fn env_colors(env: crate::connections::Environment, palette: &Palette) -> EnvColors {
    use crate::connections::Environment;
    let rgb = Color32::from_rgb;
    if !palette.dark {
        let (color, badge, badge_text) = match env {
            Environment::Dev => (
                rgb(0x2c, 0x7a, 0x4b),
                rgb(0xe3, 0xf1, 0xe6),
                rgb(0x1f, 0x6b, 0x35),
            ),
            Environment::Staging => (
                rgb(0xd0, 0x8a, 0x12),
                rgb(0xfb, 0xef, 0xd6),
                rgb(0x8a, 0x5a, 0x00),
            ),
            Environment::Production => (
                rgb(0xc2, 0x26, 0x1f),
                rgb(0xfb, 0xe3, 0xe1),
                rgb(0xa3, 0x23, 0x1b),
            ),
            Environment::Local => (
                rgb(0x7c, 0x4d, 0xdb),
                rgb(0xef, 0xea, 0xf9),
                rgb(0x5b, 0x3a, 0xa8),
            ),
            Environment::Test => (palette.accent, palette.selection, palette.accent_hover),
            Environment::None => (
                rgb(0xb5, 0xb3, 0xad),
                rgb(0xef, 0xee, 0xe9),
                rgb(0x4d, 0x4c, 0x48),
            ),
        };
        return EnvColors {
            color,
            badge,
            badge_text,
        };
    }
    let color = match env {
        Environment::Dev => palette.success,
        Environment::Staging => palette.warning,
        Environment::Production => palette.danger,
        Environment::Local => palette.magenta,
        Environment::Test => palette.accent,
        Environment::None => palette.dim,
    };
    EnvColors {
        color,
        badge: color,
        badge_text: palette.window,
    }
}

/// `amount` of `color` mixed into `base` (CSS color-mix).
pub fn mix(base: Color32, color: Color32, amount: f32) -> Color32 {
    base.lerp_to_gamma(color, amount)
}

/// Picks the palette to draw with: the user's chosen file, else Omarchy's
/// live palette when the desktop is Omarchy, else the OS light/dark setting.
pub fn resolve(catalog: &Catalog, custom: Option<&str>, system: Option<egui::Theme>) -> Palette {
    if let Some(theme) = custom.and_then(|name| catalog.find(name)) {
        return theme.palette.with_readable_labels();
    }
    if catalog.follows_omarchy()
        && let Some(theme) = catalog.system_theme()
    {
        return theme.palette.with_readable_labels();
    }
    match system {
        Some(egui::Theme::Light) => Palette::light(),
        _ => Palette::dark(),
    }
}

/// Adds the shared presets and, on Omarchy, the desktop's live palette.
pub fn enable_desktop_themes(catalog: &mut Catalog) {
    catalog.enable_desktop_themes(fastframe_theme::DesktopThemes {
        slug: "tabletist",
        omarchy_template: include_str!("../contrib/omarchy/tabletist.json.tpl"),
        // 0.1 mapped only the base colours; an untouched copy is replaced.
        omarchy_previous_templates: &[include_str!("../contrib/omarchy/tabletist-0.1.json.tpl")],
        presets: true,
    });
}

pub fn regular(size: f32) -> FontId {
    fastframe_fonts::Weight::Regular.font_id(size)
}

pub fn medium(size: f32) -> FontId {
    fastframe_fonts::Weight::Medium.font_id(size)
}

pub fn semibold(size: f32) -> FontId {
    fastframe_fonts::Weight::SemiBold.font_id(size)
}

pub fn mono(size: f32) -> FontId {
    FontId::monospace(size)
}

pub fn mono_medium(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name(faces::MONO_MEDIUM.into()))
}

/// Bold monospace: 700 where the face has it, else medium (Plex Mono).
pub fn mono_bold(size: f32) -> FontId {
    FontId::new(size, egui::FontFamily::Name(faces::MONO_BOLD.into()))
}

/// How the desktop renders text, read once per process. Tests use the
/// platform default so they never wait on D-Bus.
fn text_rendering() -> fastframe_text::TextRendering {
    static RENDERING: std::sync::OnceLock<fastframe_text::TextRendering> =
        std::sync::OnceLock::new();
    *RENDERING.get_or_init(|| {
        if cfg!(test) {
            fastframe_text::TextRendering::platform_default()
        } else {
            fastframe_text::detect()
        }
    })
}

/// Installs fonts, image loaders and icons once per egui context.
/// `system_fallbacks` is off in tests and screenshots for reproducible output.
pub fn install(ctx: &egui::Context, system_fallbacks: bool, look: &Look) {
    let mut fonts = fastframe_fonts::FontSetup::default()
        .system_fallbacks(system_fallbacks)
        .definitions();
    // Tests and screenshots (no system fallbacks) stay reproducible.
    faces::configure(&mut fonts, look, system_fallbacks);
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
}

/// The font data is drawn in: monospace on macOS and Omarchy, Inter elsewhere.
pub fn data(look: &Look) -> FontId {
    match look.data_font {
        DataFont::Monospace => mono(look.data_size),
        DataFont::Proportional => regular(look.data_size),
    }
}

/// Applies the palette and look to egui's own widgets and text styles.
pub fn apply(ctx: &egui::Context, palette: &Palette, look: &Look) {
    let mut style = (*ctx.global_style()).clone();
    apply_to_style(&mut style, palette, look);
    // egui keeps a style per theme and swaps them when the OS switches
    // between light and dark. The palette decides dark or light itself, so
    // both slots get the same style.
    let style = std::sync::Arc::new(style);
    ctx.set_style_of(egui::Theme::Dark, std::sync::Arc::clone(&style));
    ctx.set_style_of(egui::Theme::Light, style);
}

fn apply_to_style(style: &mut egui::Style, palette: &Palette, look: &Look) {
    let visuals = &mut style.visuals;
    *visuals = if palette.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };
    visuals.dark_mode = palette.dark;
    text_rendering().apply_to_visuals(visuals);
    visuals.panel_fill = palette.panel;
    visuals.window_fill = palette.overlay;
    visuals.extreme_bg_color = palette.surface;
    visuals.faint_bg_color = palette.surface;
    visuals.code_bg_color = palette.surface;
    visuals.override_text_color = Some(palette.text);
    visuals.weak_text_color = Some(palette.secondary);
    visuals.hyperlink_color = palette.accent;
    visuals.selection.bg_fill = palette.accent.gamma_multiply(0.35);
    visuals.selection.stroke = Stroke::new(1.0, palette.accent);
    visuals.window_stroke = Stroke::new(1.0, palette.outline);
    visuals.window_corner_radius = CornerRadius::same(look.dialog_radius);
    visuals.menu_corner_radius = CornerRadius::same(look.tab_radius);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: [0, 6],
        blur: 24,
        spread: 0,
        color: palette.shadow,
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: [0, 4],
        blur: 16,
        spread: 0,
        color: palette.shadow,
    };
    let corner = CornerRadius::same(look.radius);
    for widget in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = corner;
        widget.bg_stroke = Stroke::NONE;
        widget.fg_stroke = Stroke::new(1.0, palette.text);
        widget.expansion = 0.0;
    }
    visuals.widgets.noninteractive.corner_radius = corner;
    visuals.widgets.noninteractive.bg_fill = palette.panel;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, palette.outline);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, palette.text);
    visuals.widgets.inactive.bg_fill = palette.surface;
    visuals.widgets.inactive.weak_bg_fill = palette.surface;
    visuals.widgets.hovered.bg_fill = palette.surface_hover;
    visuals.widgets.hovered.weak_bg_fill = palette.surface_hover;
    visuals.widgets.active.bg_fill = palette.surface_active;
    visuals.widgets.active.weak_bg_fill = palette.surface_active;
    visuals.widgets.open.bg_fill = palette.surface_hover;
    visuals.widgets.open.weak_bg_fill = palette.surface_hover;
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
    visuals.text_cursor.stroke = Stroke::new(2.0, palette.accent);
    visuals.striped = false;

    use egui::FontFamily::{Monospace, Proportional};
    use egui::TextStyle;
    style.text_styles = [
        (TextStyle::Small, FontId::new(TEXT_SMALL, Proportional)),
        (TextStyle::Body, FontId::new(TEXT, Proportional)),
        (TextStyle::Button, FontId::new(TEXT, Proportional)),
        (TextStyle::Heading, FontId::new(TEXT_HEADING, Proportional)),
        (TextStyle::Monospace, FontId::new(TEXT_MONO, Monospace)),
    ]
    .into();
    style.spacing.item_spacing = Vec2::new(8.0, 6.0);
    style.spacing.button_padding = Vec2::new(10.0, 4.0);
    style.spacing.interact_size = Vec2::new(40.0, look.control_height);
    style.spacing.combo_height = 240.0;
    style.spacing.menu_margin = egui::Margin::same(6);
    style.spacing.window_margin = egui::Margin::same(16);
    style.interaction.selectable_labels = false;
    style.interaction.tooltip_delay = 0.4;
    style.animation_time = 0.12;
}

fastframe_icons::icons! {
    /// Every icon the interface draws: shared Lucide icons from
    /// fastframe-icons, and Lucide icons of Tabletist's own in
    /// `assets/icons/` (ISC, see its LICENSE.txt).
    pub enum Icon {
        prefix: "tabletist-icon-",
        directory: "../assets/icons/",
        ChevronDown => lucide "chevron-down",
        ChevronLeft => lucide "chevron-left",
        ChevronRight => lucide "chevron-right",
        ChevronUp => lucide "chevron-up",
        CircleAlert => lucide "circle-alert",
        CircleCheck => lucide "circle-check",
        CircleX => lucide "circle-x",
        Clock => lucide "clock",
        Copy => lucide "copy",
        Ellipsis => lucide "ellipsis",
        Eye => lucide "eye",
        Info => lucide "info",
        Lock => lucide "lock",
        LogOut => lucide "log-out",
        PanelLeft => lucide "panel-left",
        Pencil => lucide "pencil",
        Pin => lucide "pin",
        Plus => lucide "plus",
        RefreshCw => lucide "refresh-cw",
        Search => lucide "search",
        Settings => lucide "settings",
        Trash2 => lucide "trash-2",
        X => lucide "x",
        ArrowDown => "arrow-down",
        ArrowUp => "arrow-up",
        ChevronsUpDown => "chevrons-up-down",
        Funnel => "funnel",
        Image => "image",
        KeyRound => "key-round",
        List => "list",
        ListTree => "list-tree",
        LogIn => "log-in",
        PanelRight => "panel-right",
        Table => "table-2",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastframe_theme::Palette as _;

    #[test]
    fn data_is_monospace_on_macos_and_omarchy() {
        assert_eq!(data(&Look::omarchy()).family, egui::FontFamily::Monospace);
        assert_eq!(data(&Look::macos()).family, egui::FontFamily::Monospace);
        assert_eq!(
            data(&Look::standard()).family,
            egui::FontFamily::Proportional
        );
    }

    fn nord() -> CustomTheme {
        let mut palette = Palette::dark();
        palette.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
        CustomTheme {
            filename: "Nord.json".into(),
            palette,
        }
    }

    fn omarchy() -> CustomTheme {
        let mut palette = Palette::light();
        palette.accent = egui::Color32::from_rgb(0x90, 0x7a, 0xa9);
        CustomTheme {
            filename: fastframe_theme::omarchy::FILENAME.into(),
            palette,
        }
    }

    #[test]
    fn every_base_colour_can_be_set_by_name() {
        let mut palette = Palette::dark();
        for name in fastframe_theme::BASE_COLORS {
            assert!(
                palette.set(name, egui::Color32::RED),
                "{name} is not settable"
            );
        }
        assert!(!palette.set("no_such_colour", egui::Color32::RED));
    }

    #[test]
    fn without_a_choice_the_os_theme_decides() {
        let catalog = Catalog::preview(vec![nord()], false);
        assert_eq!(
            resolve(&catalog, None, Some(egui::Theme::Light)),
            Palette::light()
        );
        assert_eq!(
            resolve(&catalog, None, Some(egui::Theme::Dark)),
            Palette::dark()
        );
        assert_eq!(resolve(&catalog, None, None), Palette::dark());
    }

    #[test]
    fn omarchy_wins_over_the_os_theme_when_followed() {
        let catalog = Catalog::preview(vec![omarchy()], true);
        assert_eq!(
            resolve(&catalog, None, Some(egui::Theme::Dark)),
            omarchy().palette
        );
    }

    #[test]
    fn a_chosen_theme_wins_over_everything() {
        let catalog = Catalog::preview(vec![nord(), omarchy()], true);
        assert_eq!(
            resolve(&catalog, Some("Nord.json"), Some(egui::Theme::Light)),
            nord().palette
        );
    }

    #[test]
    fn an_unknown_custom_theme_falls_back_to_the_system_theme() {
        let catalog = Catalog::preview(vec![nord()], false);
        assert_eq!(
            resolve(&catalog, Some("Missing.json"), Some(egui::Theme::Light)),
            Palette::light()
        );
    }

    #[test]
    fn applying_a_palette_sets_egui_visuals() {
        let ctx = egui::Context::default();
        let palette = Palette::light();
        apply(&ctx, &palette, &Look::standard());
        let style = ctx.global_style();
        assert_eq!(style.visuals.panel_fill, palette.panel);
        assert!(!style.visuals.dark_mode);
        assert_eq!(style.visuals.override_text_color, Some(palette.text));
    }

    #[test]
    fn a_palette_survives_an_os_theme_switch() {
        // egui keeps one style per theme; an OS light/dark switch must not
        // bring back egui's stock style while the palette stays the same.
        let ctx = egui::Context::default();
        let palette = Palette::dark();
        apply(&ctx, &palette, &Look::standard());
        ctx.set_theme(egui::Theme::Light);
        assert_eq!(ctx.global_style().visuals.panel_fill, palette.panel);
    }

    #[test]
    fn the_standard_look_keeps_todays_metrics() {
        let ctx = egui::Context::default();
        apply(&ctx, &Palette::light(), &Look::standard());
        let style = ctx.global_style();
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            CornerRadius::same(4)
        );
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

    /// The Omarchy template rendered for Tokyo Night.
    fn tokyo_night() -> Palette {
        let colors = "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
                      lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
                      accent\t#7aa2f7\nselection\t#283457\nred\t#f7768e\n\
                      green\t#9ece6a\nyellow\t#e0af68\ncyan\t#7dcfff\n\
                      orange\t#ff9e64\nmagenta\t#bb9af7\n";
        let rendered = fastframe_theme::omarchy::render_seed::<Palette>(
            include_str!("../contrib/omarchy/tabletist.json.tpl"),
            colors,
        )
        .unwrap();
        fastframe_theme::parse_palette(&rendered).unwrap()
    }

    #[test]
    fn a_themes_faint_labels_are_lightened_until_they_read() {
        let palette = tokyo_night();
        assert!(
            contrast(palette.secondary, palette.window) < 4.5,
            "Tokyo Night's muted fails"
        );
        let readable = palette.with_readable_labels();
        assert!(contrast(readable.secondary, readable.window) >= 4.5);
        assert!(contrast(readable.dim, readable.window) >= 4.5);
        assert_eq!(readable.success, egui::Color32::from_rgb(0x9e, 0xce, 0x6a));
    }

    #[test]
    fn environments_take_the_designs_colours_on_light_and_the_themes_on_dark() {
        use crate::connections::Environment;
        let light = env_colors(Environment::Production, &Palette::light());
        assert_eq!(light.badge, egui::Color32::from_rgb(0xfb, 0xe3, 0xe1));
        let dark = tokyo_night();
        let colors = env_colors(Environment::Dev, &dark);
        assert_eq!(colors.badge, dark.success);
        assert_eq!(colors.badge_text, dark.window);
    }

    #[test]
    fn dialogs_take_the_themes_panel_when_it_names_no_overlay() {
        let palette = tokyo_night();
        assert_eq!(palette.window, egui::Color32::from_rgb(0x1a, 0x1b, 0x26));
        assert_eq!(palette.overlay, palette.panel);
        assert_ne!(palette.overlay, Palette::dark().overlay);
    }

    #[test]
    fn a_palette_file_that_sets_overlay_keeps_it() {
        let file = r##"{"base": "dark", "colors": {"panel": "#101010", "overlay": "#202020"}}"##;
        let palette: Palette = fastframe_theme::parse_palette(file).unwrap();
        assert_eq!(palette.overlay, egui::Color32::from_rgb(0x20, 0x20, 0x20));
        let file = r##"{"base": "light", "colors": {"accent": "#ff0000"}}"##;
        let palette: Palette = fastframe_theme::parse_palette(file).unwrap();
        assert_eq!(palette.overlay, Palette::light().overlay, "no panel given");
    }

    #[test]
    fn looks_have_distinct_names() {
        let names: Vec<_> = Look::ALL.iter().map(|look| look.name).collect();
        assert_eq!(names, ["standard", "macos", "omarchy"]);
    }

    #[test]
    fn the_omarchy_template_names_only_known_colours() {
        let template = include_str!("../contrib/omarchy/tabletist.json.tpl");
        for line in template.lines().filter(|line| line.contains("\": \"{{")) {
            let name = line.trim().trim_start_matches('"');
            let name = &name[..name.find('"').unwrap()];
            if name == "base" {
                continue;
            }
            assert!(
                fastframe_theme::BASE_COLORS.contains(&name) || EXTRA_COLORS.contains(&name),
                "{name} is not a colour the palette has"
            );
        }
    }

    #[test]
    fn palette_text_meets_contrast() {
        let mut failures = Vec::new();
        for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
            for (surface, background) in [
                ("window", palette.window),
                ("panel", palette.panel),
                ("surface", palette.surface),
            ] {
                for (role, color, minimum) in [
                    ("text", palette.text, 4.5),
                    ("secondary", palette.secondary, 4.5),
                    // Dim text is decoration (NULL, row numbers): the large-text minimum.
                    ("dim", palette.dim, 3.0),
                    // Warnings and errors are read, not decoration.
                    ("warning", palette.warning, 4.5),
                    ("danger", palette.danger, 4.5),
                    ("accent", palette.accent, 4.5),
                ] {
                    let ratio = contrast(color, background);
                    if ratio < minimum {
                        failures.push(format!("{theme} {role} on {surface}: {ratio:.2}"));
                    }
                }
            }
            let ratio = contrast(palette.on_accent, palette.accent);
            if ratio < 4.5 {
                failures.push(format!("{theme} on_accent on accent: {ratio:.2}"));
            }
        }
        assert!(failures.is_empty(), "{failures:#?}");
    }

    #[test]
    fn dialog_controls_stand_off_the_dialog() {
        for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
            let ratio = contrast(palette.surface, palette.overlay);
            assert!(ratio >= 1.1, "{theme} surface on overlay: {ratio:.3}");
        }
    }

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
        assert_eq!(style.spacing.interact_size.y, 24.0);
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
        assert_eq!(
            style.visuals.widgets.inactive.corner_radius,
            CornerRadius::same(8)
        );
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
            assert_eq!(
                selection.stroke,
                Stroke::new(1.0, palette.accent),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn the_default_palettes_are_warm() {
        // Warm neutrals lean red over blue (the light content is white).
        for palette in [Palette::light(), Palette::dark()] {
            for color in [palette.panel, palette.surface] {
                assert!(color.r() > color.b(), "{color:?} is not warm");
            }
        }
    }
}
