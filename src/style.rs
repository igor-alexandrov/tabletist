//! The theme's look set up on an egui context: the fonts behind the text
//! roles, the image loaders and the icons ([`install`]), and the palette
//! and look on egui's own widgets and text styles ([`apply`]). The palette
//! and the looks themselves are [`crate::theme`]'s, which knows no context.

use egui::{CornerRadius, Stroke, Vec2};

use crate::font_files::TerminalFaces;
use crate::theme::{DataFace, DialogStyle, Look, Palette, Typeface};
use crate::typography::Kind;

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
    let _ = install_with(ctx, system_fallbacks, look, None);
}

/// [`install`], with `desktop`, the terminal look's font as fontconfig gave
/// it (never in tests or screenshots): the bundled one is drawn without.
/// Returns the fonts it set, for [`set_read_faces`].
pub fn install_with(
    ctx: &egui::Context,
    system_fallbacks: bool,
    look: &Look,
    desktop: Option<&TerminalFaces>,
) -> egui::FontDefinitions {
    let mut fonts = fastframe_fonts::FontSetup::default()
        .system_fallbacks(system_fallbacks)
        .definitions();
    crate::typography::configure(&mut fonts, look, desktop);
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts.clone());
    egui_extras::install_image_loaders(ctx);
    fastframe_icons::install::<Icon>(ctx);
    fonts
}

/// The fonts of `look` and of `next` in one set, made and set as
/// [`install_with`] sets one look's: what a look switch sets, which egui
/// puts in place at the next frame's start, a frame that may draw either.
pub fn install_both(
    ctx: &egui::Context,
    system_fallbacks: bool,
    (look, next): (&Look, &Look),
    desktop: Option<&TerminalFaces>,
) -> egui::FontDefinitions {
    let mut fonts = fastframe_fonts::FontSetup::default()
        .system_fallbacks(system_fallbacks)
        .definitions();
    crate::typography::configure(&mut fonts, look, desktop);
    crate::typography::configure(&mut fonts, next, desktop);
    text_rendering().apply_to(&mut fonts);
    ctx.set_fonts(fonts.clone());
    fonts
}

/// Draws `desktop`, the font read for the settings' name: the terminal
/// look in the one fontconfig gave, or in the bundled one for none; macOS's
/// data in an installed one, or in Plex Mono for none. `fonts`, the ones
/// set, with the faces put in. Nothing is read from the disk here.
pub fn set_read_faces(
    ctx: &egui::Context,
    fonts: &mut egui::FontDefinitions,
    look: &Look,
    desktop: Option<&TerminalFaces>,
) {
    crate::typography::configure(fonts, look, desktop);
    // The others have the rendering's hinting already.
    let terminal = [400, 500, 700]
        .map(|weight| crate::typography::family_name(Typeface::Terminal, Kind::Mono, weight));
    let installed =
        [400, 500].map(|weight| crate::typography::data_family_name(DataFace::Installed, weight));
    for name in terminal.iter().chain(&installed) {
        if let Some(data) = fonts.font_data.get_mut(name) {
            text_rendering().tweak(&mut std::sync::Arc::make_mut(data).tweak);
        }
    }
    ctx.set_fonts(fonts.clone());
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
        Download => "download",
        Funnel => "funnel",
        Image => "image",
        KeyRound => "key-round",
        List => "list",
        ListTree => "list-tree",
        LogIn => "log-in",
        PanelRight => "panel-right",
        Server => "server",
        ShieldAlert => "shield-alert",
        SquareTerminal => "square-terminal",
        Table => "table-2",
        TriangleAlert => "triangle-alert",
        WifiOff => "wifi-off",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
