//! Palette, typography, icons, and the mapping onto egui's style.

pub mod desktop_font;

use egui::{Color32, CornerRadius, FontId, Stroke, Vec2};

/// A palette file from the themes directory.
pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
/// The palette files, the shared presets, and Omarchy's live palette.
pub type Catalog = fastframe_theme::Catalog<Palette>;

// The type scale. Every text size in the interface comes from here.
/// Body text: tree, grid, tabs, fields, buttons.
pub const TEXT: f32 = 13.0;
/// Secondary text: types, footer, hints, counts.
pub const TEXT_SMALL: f32 = 11.5;
/// Dialog and panel titles.
pub const TEXT_TITLE: f32 = 15.0;
/// The picker's heading.
pub const TEXT_HEADING: f32 = 20.0;
/// JSON, hex and raw SQL.
pub const TEXT_MONO: f32 = 12.0;

// Spacing, on a 4 pt grid.
/// Padding inside panels and bars.
pub const PAD: i8 = 12;

/// Every colour the interface draws with. The names match fastframe's
/// sixteen base colours so palette files and Omarchy can set them.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Palette {
    pub dark: bool,
    pub window: Color32,
    pub panel: Color32,
    pub surface: Color32,
    pub surface_hover: Color32,
    pub surface_active: Color32,
    pub outline: Color32,
    pub text: Color32,
    pub secondary: Color32,
    pub dim: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub on_accent: Color32,
    pub danger: Color32,
    pub warning: Color32,
    pub overlay: Color32,
    pub shadow: Color32,
}

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
            dim: Color32::from_rgb(0x85, 0x81, 0x7b),
            accent: Color32::from_rgb(0x5a, 0x9f, 0xf8),
            accent_hover: Color32::from_rgb(0x7a, 0xb2, 0xf9),
            on_accent: Color32::from_rgb(0x0b, 0x13, 0x20),
            danger: Color32::from_rgb(0xf2, 0x70, 0x7a),
            warning: Color32::from_rgb(0xea, 0xb3, 0x5a),
            // The panel tone: fields and buttons (surface) must stand off
            // a dialog as they do off the sidebar.
            overlay: Color32::from_rgb(0x26, 0x25, 0x23),
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
            // Accent, danger and warning are read as text: 4.5:1 or better.
            accent: Color32::from_rgb(0x0a, 0x64, 0xcc),
            accent_hover: Color32::from_rgb(0x0a, 0x55, 0xad),
            on_accent: Color32::WHITE,
            danger: Color32::from_rgb(0xb8, 0x2a, 0x36),
            warning: Color32::from_rgb(0x8a, 0x5a, 0x00),
            overlay: Color32::from_rgb(0xff, 0xff, 0xff),
            shadow: Color32::from_black_alpha(40),
        }
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
            _ => return false,
        }
        true
    }

    /// Dialogs and popups take the panel colour when a file (such as the
    /// Omarchy template) sets the panel but not the overlay, so they follow
    /// the theme rather than keeping the default's.
    fn derive(&mut self, given: &std::collections::BTreeSet<&str>) {
        if given.contains("panel") && !given.contains("overlay") {
            self.overlay = self.panel;
        }
    }
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
    /// Bars (top bar, object tabs, filter bar, footer) are separated from
    /// the content by 1 px lines. The sidebar and the row panel always are:
    /// one tone off the content is too faint to mark a pane's edge.
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

    /// macOS 26: rounder, roomier, borderless controls, an inset pill
    /// selection, and tabs in a track.
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
            // Tones and the raised tabs set the bars apart.
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

/// Picks the palette to draw with: the user's chosen file, else Omarchy's
/// live palette when the desktop is Omarchy, else the OS light/dark setting.
pub fn resolve(catalog: &Catalog, custom: Option<&str>, system: Option<egui::Theme>) -> Palette {
    if let Some(theme) = custom.and_then(|name| catalog.find(name)) {
        return theme.palette;
    }
    if catalog.follows_omarchy()
        && let Some(theme) = catalog.system_theme()
    {
        return theme.palette;
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
        omarchy_previous_templates: &[],
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

#[cfg(test)]
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

/// The WCAG contrast ratio of two opaque colours.
#[cfg(test)]
pub(crate) fn contrast(a: egui::Color32, b: egui::Color32) -> f64 {
    let (x, y) = (luminance(a), luminance(b));
    let (hi, lo) = if x > y { (x, y) } else { (y, x) };
    (hi + 0.05) / (lo + 0.05)
}

fastframe_icons::icons! {
    /// Every icon the interface draws. All are shared Lucide icons from
    /// fastframe-icons, so no SVG files live in this repository yet.
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastframe_theme::Palette as _;

    #[test]
    fn data_is_monospace_only_in_the_omarchy_look() {
        assert_eq!(data(&Look::omarchy()).family, egui::FontFamily::Monospace);
        assert_eq!(data(&Look::macos()).family, egui::FontFamily::Proportional);
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
        let colors = "mode\tdark\nbackground\t#1a1b26\nforeground\t#a9b1d6\n\
                      accent\t#7aa2f7\nred\t#f7768e\nyellow\t#e0af68\n";
        let rendered = fastframe_theme::omarchy::render_seed::<Palette>(
            include_str!("../contrib/omarchy/tabletist.json.tpl"),
            colors,
        )
        .unwrap();
        fastframe_theme::parse_palette(&rendered).unwrap()
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
                fastframe_theme::BASE_COLORS.contains(&name),
                "{name} is not a base colour"
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
        // Warm neutrals lean red over blue.
        for palette in [Palette::light(), Palette::dark()] {
            for color in [palette.window, palette.panel, palette.surface] {
                assert!(color.r() > color.b(), "{color:?} is not warm");
            }
        }
    }
}
