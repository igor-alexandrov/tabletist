//! Palette, shape, icons, and the mapping onto egui's style. Fonts and text
//! styles live in [`crate::typography`].

use egui::{Color32, CornerRadius, Stroke, Vec2};

/// A palette file from the themes directory.
pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
/// The palette files, the shared presets, and Omarchy's live palette.
pub type Catalog = fastframe_theme::Catalog<Palette>;

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
    /// The terminal's second value tag.
    pub info: Color32,
    /// The terminal's first value tag, JSON numbers.
    pub orange: Color32,
    /// The local environment, the terminal's third value tag.
    pub magenta: Color32,
    /// The terminal's fourth value tag.
    pub blue: Color32,
    /// The theme's bright red: a value tag only when it is a rose, apart
    /// from the red that marks production.
    pub rose: Color32,
    /// The theme's bright green: a value tag only when it is an olive,
    /// apart from the green that marks dev.
    pub olive: Color32,
}

/// The colours Tabletist adds to fastframe's base ones.
pub const EXTRA_COLORS: [&str; 10] = [
    "faint",
    "border",
    "selection",
    "success",
    "info",
    "orange",
    "magenta",
    "blue",
    "rose",
    "olive",
];

impl Palette {
    /// The macOS design's dark colours. The content is a step lighter than
    /// the bars round it, as the light content is whiter than its bars.
    pub fn dark() -> Self {
        Self {
            dark: true,
            window: Color32::from_rgb(0x26, 0x26, 0x24),
            panel: Color32::from_rgb(0x22, 0x22, 0x20),
            surface: Color32::from_rgb(0x30, 0x2f, 0x2c),
            surface_hover: Color32::from_rgb(0x2d, 0x2c, 0x29),
            // Lighter than the surface: what is raised or pressed comes
            // forward in the dark.
            surface_active: Color32::from_rgb(0x3a, 0x39, 0x36),
            outline: Color32::from_rgb(0x3a, 0x39, 0x36),
            text: Color32::from_rgb(0xec, 0xeb, 0xe6),
            secondary: Color32::from_rgb(0xc4, 0xc2, 0xbc),
            dim: Color32::from_rgb(0xa3, 0xa1, 0x9b),
            // Accent, danger and warning are read as text: 4.5:1 or better.
            accent: Color32::from_rgb(0x7a, 0x9d, 0xf5),
            accent_hover: Color32::from_rgb(0xa9, 0xc1, 0xfa),
            on_accent: Color32::from_rgb(0x1c, 0x1c, 0x1a),
            danger: Color32::from_rgb(0xf0, 0x8a, 0x82),
            warning: Color32::from_rgb(0xe0, 0xb2, 0x5a),
            // The content's tone, as in the light palette: a dialog is a
            // piece of content over a dimmed window.
            overlay: Color32::from_rgb(0x26, 0x26, 0x24),
            shadow: Color32::from_black_alpha(150),
            faint: Color32::from_rgb(0x8a, 0x88, 0x82),
            border: Color32::from_rgb(0x44, 0x43, 0x3f),
            selection: Color32::from_rgb(0x2b, 0x35, 0x50),
            success: Color32::from_rgb(0x8f, 0xd1, 0x9e),
            info: Color32::from_rgb(0x9f, 0xbd, 0xf0),
            orange: Color32::from_rgb(0xf0, 0xa8, 0x68),
            magenta: Color32::from_rgb(0xb9, 0xa0, 0xf5),
            blue: Color32::from_rgb(0x9f, 0xbd, 0xf0),
            rose: Color32::from_rgb(0xed, 0xa6, 0xc6),
            olive: Color32::from_rgb(0xbc, 0xcb, 0x88),
        }
    }

    /// The fill of a control under the pointer: a step further from the
    /// window than the control at rest, so darker on a light palette and
    /// lighter on a dark one.
    pub fn hover_fill(&self) -> Color32 {
        if self.dark {
            self.surface_active
        } else {
            self.surface_hover
        }
    }

    /// The fill of a control while it is pressed: a step past
    /// [`Palette::hover_fill`].
    pub fn pressed_fill(&self) -> Color32 {
        if self.dark {
            self.surface_active.lerp_to_gamma(self.text, 0.08)
        } else {
            self.surface_active
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
            blue: Color32::from_rgb(0x1e, 0x4f, 0x8a),
            rose: Color32::from_rgb(0x8a, 0x2e, 0x5a),
            olive: Color32::from_rgb(0x4c, 0x5a, 0x1e),
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
            "blue" => self.blue = color,
            "rose" => self.rose = color,
            "olive" => self.olive = color,
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
        // Value tags follow a file's own colours: its accent as the blue,
        // and its red and green where it names no rose or olive (which
        // then fall back to muted).
        if given.contains("accent") && !given.contains("blue") {
            self.blue = self.accent;
        }
        if given.contains("danger") && !given.contains("rose") {
            self.rose = self.danger;
        }
        if given.contains("success") && !given.contains("olive") {
            self.olive = self.success;
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

/// The typefaces a look draws with (see [`crate::typography`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
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
    /// Menus and the disconnected banner.
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
    pub data_font: DataFont,
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
            data_font: DataFont::Proportional,
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
            tree_row: 24.0,
            grid_row: 37.0,
            bordered_controls: false,
            selection: Selection::Pill,
            tabs: TabStyle::Raised,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            data_font: DataFont::Monospace,
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
            tree_row: 24.0,
            grid_row: 30.0,
            bordered_controls: true,
            selection: Selection::Bar,
            tabs: TabStyle::Underline,
            dialog: DialogStyle::AccentBorder,
            capsule_search: false,
            sidebar_tinted: false,
            panel_separators: true,
            data_font: DataFont::Monospace,
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

    /// `text` as this look labels things: lower case in the terminal look.
    pub fn label(&self, text: &str) -> String {
        if self.terminal {
            text.to_lowercase()
        } else {
            text.to_owned()
        }
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
    crate::typography::configure(&mut fonts, look, system_fallbacks);
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts);
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
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
    visuals.widgets.hovered.bg_fill = palette.hover_fill();
    visuals.widgets.hovered.weak_bg_fill = palette.hover_fill();
    visuals.widgets.active.bg_fill = palette.pressed_fill();
    visuals.widgets.active.weak_bg_fill = palette.pressed_fill();
    visuals.widgets.open.bg_fill = palette.hover_fill();
    visuals.widgets.open.weak_bg_fill = palette.hover_fill();
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

    style.text_styles = crate::typography::text_styles(look.faces);
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
        Check => lucide "check",
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
        Play => lucide "play",
        Plus => lucide "plus",
        RefreshCw => lucide "refresh-cw",
        Search => lucide "search",
        Settings => lucide "settings",
        Trash2 => lucide "trash-2",
        X => lucide "x",
        ArrowDown => "arrow-down",
        ArrowUp => "arrow-up",
        ChevronsUpDown => "chevrons-up-down",
        Code => "code",
        Database => "database",
        Funnel => "funnel",
        Image => "image",
        KeyRound => "key-round",
        List => "list",
        ListTree => "list-tree",
        LogIn => "log-in",
        PanelRight => "panel-right",
        Server => "server",
        ShieldAlert => "shield-alert",
        Table => "table-2",
        WifiOff => "wifi-off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastframe_theme::Palette as _;

    #[test]
    fn data_is_monospace_on_macos_and_omarchy() {
        use crate::typography::{Kind, TextRole};
        use crate::ui::grid::data_role;
        assert_eq!(data_role(&Look::omarchy()), TextRole::OBody);
        assert_eq!(data_role(&Look::macos()), TextRole::GridCell);
        assert_eq!(data_role(&Look::macos()).spec().kind, Kind::Mono);
        assert_eq!(data_role(&Look::standard()).spec().kind, Kind::Sans);
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
                      orange\t#ff9e64\nmagenta\t#bb9af7\nblue\t#7aa2f7\n\
                      bright_red\t#f7768e\nbright_green\t#9ece6a\n";
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
    fn value_tags_take_a_themes_rose_and_olive_only_when_they_stand_apart() {
        // Tokyo Night's bright red and green are its red and green.
        let palette = tokyo_night();
        assert_eq!(palette.blue, egui::Color32::from_rgb(0x7a, 0xa2, 0xf7));
        let slots = crate::ui::value_tags::terminal_slots(&palette);
        assert_eq!(slots[3], palette.blue);
        assert_eq!(slots[6], palette.dim);
        assert_eq!(slots[7], palette.dim);
        // A file with its own colours but no rose or olive: muted too.
        let file = r##"{"base": "dark", "colors": {"danger": "#ff0000", "success": "#00ff00"}}"##;
        let palette: Palette = fastframe_theme::parse_palette(file).unwrap();
        let slots = crate::ui::value_tags::terminal_slots(&palette);
        assert_eq!(slots[6], palette.dim);
        assert_eq!(slots[7], palette.dim);
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
    fn a_control_under_the_pointer_stands_further_off_the_window() {
        for (theme, palette) in [("light", Palette::light()), ("dark", Palette::dark())] {
            let off = |color| contrast(color, palette.window);
            assert!(off(palette.hover_fill()) > off(palette.surface), "{theme}");
            assert!(
                off(palette.pressed_fill()) > off(palette.hover_fill()),
                "{theme}"
            );
        }
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
