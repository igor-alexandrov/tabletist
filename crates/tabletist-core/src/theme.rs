//! Palette, shape and faces: what each look draws with, as data. They are
//! set up on egui's style, with the icons, in `style`; the fonts and text
//! styles live in `typography`.

use egui::Color32;

use crate::settings::{FontSize, FontSizes, InterfaceSize, Mode, RowHeight, Settings, Theme};

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
    /// The cells of a selected range, a step quieter than the selected row.
    pub accent_faint: Color32,
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
            accent_faint: Color32::from_rgb(0x2a, 0x32, 0x48),
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
            accent_faint: Color32::from_rgb(0xee, 0xf2, 0xfc),
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
            "accent_faint" => self.accent_faint = color,
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
        if given.contains("accent") && !given.contains("accent_faint") {
            self.accent_faint = self.window.lerp_to_gamma(self.accent, 0.1);
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

/// The typefaces a look draws with (see `typography`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Typeface {
    /// Inter, with egui's monospace.
    Inter,
    /// IBM Plex Sans and IBM Plex Mono.
    Plex,
    /// One monospace face for everything.
    Terminal,
}

/// The monospace face macOS draws data in: one of the two it bundles, or
/// the installed font the settings name, once it is read
/// ([`crate::font_files::installed_font`]).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum DataFace {
    #[default]
    PlexMono,
    JetBrainsMono,
    Installed,
}

impl DataFace {
    /// The bundled faces, in the order the menu lists them, first.
    pub const BUNDLED: [DataFace; 2] = [Self::PlexMono, Self::JetBrainsMono];

    /// A bundled face's name, as the menu and the file write it. An
    /// installed one has the name the settings give it.
    pub fn name(self) -> Option<&'static str> {
        match self {
            Self::PlexMono => Some("IBM Plex Mono"),
            Self::JetBrainsMono => Some("JetBrains Mono"),
            Self::Installed => None,
        }
    }

    /// The bundled face `font` names, whatever its case: the default for
    /// none, and for `monospace`, which names the desktop's own (the
    /// terminal look's default). A name of no bundled face is given back:
    /// an installed font, drawn once it is read, else the default, which
    /// says so.
    pub fn of(font: Option<&str>) -> Result<Self, &str> {
        let font = font.map(str::trim);
        let Some(font) = font.filter(|font| !font.eq_ignore_ascii_case("monospace")) else {
            return Ok(Self::default());
        };
        Self::BUNDLED
            .into_iter()
            .find(|face| {
                face.name()
                    .is_some_and(|name| name.eq_ignore_ascii_case(font))
            })
            .ok_or(font)
    }
}

/// A look's typefaces at the sizes the settings give them: what every
/// text role is drawn in. The terminal look keeps its roles' sizes and
/// draws the whole window larger or smaller
/// ([`Settings::font_size_in`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Faces {
    pub typeface: Typeface,
    /// The interface's own text, on macOS and Windows.
    pub interface: InterfaceSize,
    /// The data's face, on macOS.
    pub data: DataFace,
    /// The data's size, on macOS and Windows.
    pub data_size: FontSize,
}

impl Faces {
    /// `typeface` at the sizes it has before the settings say any.
    pub const fn of(typeface: Typeface) -> Self {
        Self {
            typeface,
            interface: InterfaceSize::Default,
            data: DataFace::PlexMono,
            data_size: FontSizes::of(false).default,
        }
    }

    /// The faces at the sizes `settings` give, and the data in the face
    /// they name: an installed one where `installed` (it is read), else
    /// the default. The terminal look's stay as they are.
    pub fn with_text(self, settings: &Settings, installed: bool) -> Self {
        if self.typeface == Typeface::Terminal {
            return Self::of(self.typeface);
        }
        let data = match DataFace::of(settings.font.as_deref()) {
            Ok(face) => face,
            Err(_) if installed => DataFace::Installed,
            Err(_) => DataFace::default(),
        };
        Self {
            typeface: self.typeface,
            interface: settings.interface_size,
            data,
            data_size: settings.font_size_in(false),
        }
    }
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
    /// Grid rows at the regular density: [`Look::row`] is the height a
    /// grid draws them at.
    pub grid_row: f32,
    /// How dense the grids are: the Appearance setting, regular until a
    /// look is given the settings ([`Look::with_grid`]).
    pub density: RowHeight,
    /// Odd grid rows shaded: the look's default until it is given the
    /// settings.
    pub zebra: bool,
    /// The grid header's second line, the columns' types.
    pub type_line: bool,
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
            density: RowHeight::Regular,
            zebra: false,
            type_line: true,
            bordered_controls: false,
            selection: Selection::Tint,
            tabs: TabStyle::Outlined,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            data_font: DataFont::Proportional,
            faces: Faces::of(Typeface::Inter),
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
            density: RowHeight::Regular,
            zebra: false,
            type_line: true,
            bordered_controls: false,
            selection: Selection::Pill,
            tabs: TabStyle::Raised,
            dialog: DialogStyle::Shadow,
            capsule_search: false,
            sidebar_tinted: true,
            panel_separators: true,
            data_font: DataFont::Monospace,
            faces: Faces::of(Typeface::Plex),
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
            density: RowHeight::Regular,
            zebra: true,
            type_line: true,
            bordered_controls: true,
            selection: Selection::Bar,
            tabs: TabStyle::Underline,
            dialog: DialogStyle::AccentBorder,
            capsule_search: false,
            sidebar_tinted: false,
            panel_separators: true,
            data_font: DataFont::Monospace,
            faces: Faces::of(Typeface::Terminal),
            primary: PrimaryStyle::Outline,
            terminal: true,
        }
    }

    /// The look with its text as `settings` have it: the interface's size,
    /// and the data's face and size, the installed face where `installed`
    /// ([`Faces::with_text`]).
    pub fn with_text(self, settings: &Settings, installed: bool) -> Self {
        Self {
            faces: self.faces.with_text(settings, installed),
            ..self
        }
    }

    /// The look with the grids as `settings` have them: how dense their
    /// rows are, whether odd ones are shaded, and whether the header has
    /// the types' line.
    pub fn with_grid(self, settings: &crate::settings::Settings) -> Self {
        Self {
            density: settings.row_height,
            zebra: settings.zebra_in(self.terminal),
            type_line: settings.column_types,
            ..self
        }
    }

    /// A grid row's height at the look's density. The terminal counts in
    /// lines, a regular row being one and a half: one or two. The others
    /// step 8 either side, the regular row's rule kept in each, and are at
    /// least as tall as a line of their data: a large data size makes the
    /// short rows taller.
    pub fn row(&self) -> f32 {
        let regular = self.grid_row;
        let row = match (self.density, self.terminal) {
            (RowHeight::Regular, _) => regular,
            (RowHeight::Compact, true) => regular * 2.0 / 3.0,
            (RowHeight::Roomy, true) => regular * 4.0 / 3.0,
            (RowHeight::Compact, false) => regular - 8.0,
            (RowHeight::Roomy, false) => regular + 8.0,
        };
        if self.terminal {
            return row;
        }
        // A line of Inter and of Plex Mono is at most 1.3 times the data's
        // size. JetBrains Mono's is 1.32, where the rows of macOS are
        // taller than that needs anyway.
        let line = (self.faces.data_size.points() * 1.3).ceil();
        row.max(line)
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

/// Where a pinned theme is read from, as its name is written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinFrom<'a> {
    /// `Nord.json`: a palette file in the themes folder, as older versions
    /// named one.
    File(&'a str),
    /// `/…`, `~/…` or another absolute path of the platform (`C:\…`): a
    /// palette file, or an Omarchy theme's folder.
    Path(&'a str),
    /// `Tokyo Night`: an Omarchy theme, else a palette of the themes folder
    /// by its name.
    Name(&'a str),
}

impl<'a> PinFrom<'a> {
    pub fn of(name: &'a str) -> Self {
        let absolute = std::path::Path::new(name).is_absolute();
        if absolute || name.starts_with('/') || name.starts_with("~/") {
            Self::Path(name)
        } else if !name.contains(['/', '\\'])
            && name
                .rsplit_once('.')
                .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("json"))
        {
            Self::File(name)
        } else {
            Self::Name(name)
        }
    }

    /// The palette file the themes folder is to read first, for a pin
    /// that names one.
    pub fn file(self) -> Option<&'a str> {
        match self {
            Self::File(file) => Some(file),
            Self::Path(_) | Self::Name(_) => None,
        }
    }
}

/// The palette of the themes folder a pin names: its file, or a palette
/// called `name` whatever its case (`tokyo night` is `Tokyo Night.json`).
/// Never Omarchy's live palette, which is no theme of its own.
pub fn catalog_pin(catalog: &Catalog, name: &str) -> Option<Palette> {
    let named = |theme: &&CustomTheme| {
        theme.filename != fastframe_theme::omarchy::FILENAME
            && (theme.filename == name
                || fastframe_theme::display_name(&theme.filename).to_lowercase()
                    == name.to_lowercase())
    };
    catalog
        .themes()
        .iter()
        .find(named)
        .map(|theme| theme.palette)
}

/// Picks the palette to draw with. macOS and Windows draw light or dark:
/// the system's, or the one `theme` names. The terminal look draws the
/// pinned theme where it was read (`pinned`), else Omarchy's live palette
/// when the desktop is Omarchy, else the system's light or dark.
pub fn resolve(
    look: &Look,
    theme: &Theme,
    pinned: Option<Palette>,
    catalog: &Catalog,
    system: Option<egui::Theme>,
) -> Palette {
    let system = match system {
        Some(egui::Theme::Light) => Palette::light(),
        _ => Palette::dark(),
    };
    if !look.terminal {
        return match theme.mode() {
            Mode::System => system,
            Mode::Light => Palette::light(),
            Mode::Dark => Palette::dark(),
        };
    }
    if let Some(palette) = pinned {
        return palette.with_readable_labels();
    }
    if catalog.follows_omarchy()
        && let Some(theme) = catalog.system_theme()
    {
        return theme.palette.with_readable_labels();
    }
    system
}

/// Adds the shared presets and, on Omarchy, the desktop's live palette.
pub fn enable_desktop_themes(catalog: &mut Catalog) {
    catalog.enable_desktop_themes(fastframe_theme::DesktopThemes {
        slug: "tabletist",
        omarchy_template: include_str!("../../../contrib/omarchy/tabletist.json.tpl"),
        // 0.1 mapped only the base colours; an untouched copy is replaced.
        omarchy_previous_templates: &[include_str!(
            "../../../contrib/omarchy/tabletist-0.1.json.tpl"
        )],
        presets: true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use fastframe_theme::Palette as _;

    /// The settings with the interface's size, the data font and its size
    /// as given.
    fn text(interface: InterfaceSize, font: Option<&str>, size: f64) -> Settings {
        Settings {
            interface_size: interface,
            font: font.map(str::to_owned),
            font_size: FontSize::of(size),
            ..Settings::default()
        }
    }

    #[test]
    fn a_data_font_is_one_of_the_bundled_by_its_name() {
        assert_eq!(DataFace::of(None), Ok(DataFace::PlexMono));
        assert_eq!(
            DataFace::of(Some("jetbrains mono")),
            Ok(DataFace::JetBrainsMono)
        );
        assert_eq!(DataFace::of(Some("IBM Plex Mono")), Ok(DataFace::PlexMono));
        // The terminal look's default names the desktop's own: the default
        // here too, and nothing to say.
        assert_eq!(DataFace::of(Some("monospace")), Ok(DataFace::PlexMono));
        assert_eq!(DataFace::of(Some(" monospace ")), Ok(DataFace::PlexMono));
        assert_eq!(
            DataFace::of(Some(" JetBrains Mono ")),
            Ok(DataFace::JetBrainsMono)
        );
        assert_eq!(DataFace::of(Some("Fira Code")), Err("Fira Code"));
        // An installed font is drawn once it is read, the default before.
        let fira = text(InterfaceSize::Default, Some("Fira Code"), 12.5);
        assert_eq!(
            Look::macos().with_text(&fira, false).faces.data,
            DataFace::PlexMono
        );
        assert_eq!(
            Look::macos().with_text(&fira, true).faces.data,
            DataFace::Installed
        );
        // A bundled face's name is the bundled face, read or not.
        let jetbrains = text(InterfaceSize::Default, Some("JetBrains Mono"), 12.5);
        for installed in [false, true] {
            let look = Look::macos().with_text(&jetbrains, installed);
            assert_eq!(look.faces.data, DataFace::JetBrainsMono);
        }
        let plex = text(InterfaceSize::Default, None, 12.5);
        assert_eq!(
            Look::macos().with_text(&plex, true).faces.data,
            DataFace::PlexMono
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

    /// What the terminal look draws with `theme`, the pin read from the
    /// catalog as the app reads one of the themes folder.
    fn terminal(catalog: &Catalog, theme: &Theme, system: Option<egui::Theme>) -> Palette {
        let pinned = theme.pin().and_then(|name| catalog_pin(catalog, name));
        resolve(&Look::omarchy(), theme, pinned, catalog, system)
    }

    #[test]
    fn without_a_choice_the_os_theme_decides() {
        let catalog = Catalog::preview(vec![nord()], false);
        for look in Look::ALL {
            let at = |system| resolve(&look, &Theme::Unset, None, &catalog, system);
            assert_eq!(at(Some(egui::Theme::Light)), Palette::light());
            assert_eq!(at(Some(egui::Theme::Dark)), Palette::dark());
            assert_eq!(at(None), Palette::dark());
        }
    }

    #[test]
    fn macos_and_windows_draw_the_mode_the_settings_name() {
        let catalog = Catalog::preview(vec![nord(), omarchy()], true);
        for look in [Look::macos(), Look::standard()] {
            let at = |theme: Theme, system| resolve(&look, &theme, None, &catalog, system);
            let light = Some(egui::Theme::Light);
            assert_eq!(at(Theme::Dark, light), Palette::dark());
            assert_eq!(at(Theme::Light, Some(egui::Theme::Dark)), Palette::light());
            assert_eq!(at(Theme::System, light), Palette::light());
            // The terminal look's values are the system's here, Omarchy
            // followed or not, a palette there or not.
            assert_eq!(at(Theme::Omarchy, light), Palette::light());
            assert_eq!(
                at(Theme::Pinned("Nord.json".into()), light),
                Palette::light()
            );
        }
    }

    #[test]
    fn omarchy_wins_over_the_os_theme_when_followed() {
        let catalog = Catalog::preview(vec![omarchy()], true);
        let dark = Some(egui::Theme::Dark);
        for theme in [Theme::Unset, Theme::Omarchy, Theme::Light, Theme::Dark] {
            assert_eq!(
                terminal(&catalog, &theme, dark),
                omarchy().palette,
                "{theme:?}"
            );
        }
    }

    #[test]
    fn a_pinned_theme_wins_over_everything_in_the_terminal_look() {
        let catalog = Catalog::preview(vec![nord(), omarchy()], true);
        let light = Some(egui::Theme::Light);
        for name in ["Nord.json", "Nord", "nord"] {
            let theme = Theme::Pinned(name.into());
            assert_eq!(terminal(&catalog, &theme, light), nord().palette, "{name}");
        }
        // Omarchy's live palette is no theme to pin by its file's name.
        let theme = Theme::Pinned("omarchy".into());
        assert_eq!(catalog_pin(&catalog, "Omarchy"), None);
        assert_eq!(terminal(&catalog, &theme, light), omarchy().palette);
    }

    #[test]
    fn an_unknown_pin_falls_back_to_the_desktop() {
        let catalog = Catalog::preview(vec![nord()], false);
        let theme = Theme::Pinned("Missing.json".into());
        assert_eq!(
            terminal(&catalog, &theme, Some(egui::Theme::Light)),
            Palette::light()
        );
    }

    #[test]
    fn a_pin_is_read_from_where_its_name_says() {
        assert_eq!(PinFrom::of("Nord.json"), PinFrom::File("Nord.json"));
        assert_eq!(PinFrom::of("my.JSON"), PinFrom::File("my.JSON"));
        assert_eq!(PinFrom::of("Tokyo Night"), PinFrom::Name("Tokyo Night"));
        assert_eq!(PinFrom::of("/x/Nord.json"), PinFrom::Path("/x/Nord.json"));
        assert_eq!(
            PinFrom::of("~/themes/sunset"),
            PinFrom::Path("~/themes/sunset")
        );
        // Neither a file of the folder nor a path the app can find.
        assert_eq!(
            PinFrom::of("themes/Nord.json"),
            PinFrom::Name("themes/Nord.json")
        );
        #[cfg(windows)]
        assert_eq!(
            PinFrom::of("C:\\themes\\Nord.json"),
            PinFrom::Path("C:\\themes\\Nord.json")
        );
        assert_eq!(PinFrom::of("Nord.json").file(), Some("Nord.json"));
        assert_eq!(PinFrom::of("Nord").file(), None);
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
    fn a_themes_faint_labels_are_lightened_until_they_read() {
        let palette = crate::fixtures::tokyo_night();
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
    fn dialogs_take_the_themes_panel_when_it_names_no_overlay() {
        let palette = crate::fixtures::tokyo_night();
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
        let template = include_str!("../../../contrib/omarchy/tabletist.json.tpl");
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
    fn the_default_palettes_are_warm() {
        // Warm neutrals lean red over blue (the light content is white).
        for palette in [Palette::light(), Palette::dark()] {
            for color in [palette.panel, palette.surface] {
                assert!(color.r() > color.b(), "{color:?} is not warm");
            }
        }
    }
}
