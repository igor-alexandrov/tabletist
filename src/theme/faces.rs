//! The typefaces each look draws with, on top of fastframe's Inter set-up.
//!
//! - Standard: Inter, with egui's monospace for data.
//! - macOS: IBM Plex Sans (400, 500, 600) for the interface and IBM Plex
//!   Mono (400, 500) for data, both bundled.
//! - Omarchy: the desktop's monospace font for everything, at 400, 500 and
//!   700, or the bundled JetBrains Mono when fontconfig finds none.
//!
//! Views name faces through [`super::regular`], [`super::medium`],
//! [`super::semibold`], [`super::mono`], [`super::mono_medium`] and
//! [`super::mono_bold`]; this module decides what each family holds.

use std::sync::Arc;

#[cfg(any(target_os = "macos", target_os = "linux", test, feature = "shots"))]
use egui::epaint::text::VariationCoords;
use egui::{FontData, FontDefinitions, FontFamily};

use super::{Faces, Look};

/// The family of medium-weight monospace text.
pub const MONO_MEDIUM: &str = "mono-medium";
/// The family of bold monospace text (medium where the face has no bold).
pub const MONO_BOLD: &str = "mono-bold";

/// IBM Plex Sans, variable in weight and width (SIL OFL 1.1).
#[cfg(any(target_os = "macos", test, feature = "shots"))]
const PLEX_SANS: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Variable.ttf");
#[cfg(any(target_os = "macos", test, feature = "shots"))]
const PLEX_MONO: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf");
#[cfg(any(target_os = "macos", test, feature = "shots"))]
const PLEX_MONO_MEDIUM: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf");
/// JetBrains Mono, variable in weight (SIL OFL 1.1): Omarchy's default, for
/// desktops without a monospace font of their own.
#[cfg(any(target_os = "linux", test, feature = "shots"))]
const JETBRAINS_MONO: &[u8] = include_bytes!("../../assets/fonts/JetBrainsMono-Variable.ttf");

/// fastframe's families for Inter's named weights.
const MEDIUM: &str = "inter-medium";
const SEMIBOLD: &str = "inter-semibold";
const BOLD: &str = "inter-bold";

/// egui draws IBM Plex larger than browsers do at the same size; these
/// bring it back to the design's measure.
#[cfg(any(target_os = "macos", test, feature = "shots"))]
const PLEX_SANS_SCALE: f32 = 0.887;
#[cfg(any(target_os = "macos", test, feature = "shots"))]
const PLEX_MONO_SCALE: f32 = 0.92;

/// `data` drawn at `scale` of its size.
#[cfg(any(target_os = "macos", test, feature = "shots"))]
fn scaled(mut data: FontData, scale: f32) -> Arc<FontData> {
    data.tweak.scale = scale;
    Arc::new(data)
}

/// A variable face at `weight`.
#[cfg(any(target_os = "linux", test, feature = "shots"))]
fn weighted(bytes: &'static [u8], weight: f32) -> Arc<FontData> {
    let mut data = FontData::from_static(bytes);
    data.tweak.coords = VariationCoords::new([(b"wght", weight)]);
    Arc::new(data)
}

/// Puts `face` first in `family`, ahead of what it held (Inter and the
/// fallbacks stay behind it for glyphs the face lacks).
fn lead(fonts: &mut FontDefinitions, family: FontFamily, face: &str) {
    let list = fonts.families.entry(family).or_default();
    list.retain(|name| name != face);
    list.insert(0, face.to_owned());
}

/// The fonts behind the first in a family: its fallbacks.
fn tail(fonts: &FontDefinitions, family: &FontFamily) -> Vec<String> {
    fonts
        .families
        .get(family)
        .map(|list| list.iter().skip(1).cloned().collect())
        .unwrap_or_default()
}

/// Replaces the faces fastframe set up with the look's. `desktop` allows
/// reading the desktop's own monospace font (never in tests or screenshots).
pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: bool) {
    // Every look has the monospace weight families, so views can name them.
    for family in [MONO_MEDIUM, MONO_BOLD] {
        let list = fonts
            .families
            .get(&FontFamily::Monospace)
            .cloned()
            .unwrap_or_default();
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
    match look.faces {
        Faces::Inter => {}
        Faces::Plex => plex(fonts),
        Faces::Terminal => terminal(fonts, desktop),
    }
}

#[cfg(any(target_os = "macos", test, feature = "shots"))]
fn plex(fonts: &mut FontDefinitions) {
    for (name, weight) in [
        ("plex-sans", 400.0),
        ("plex-sans-medium", 500.0),
        ("plex-sans-semibold", 600.0),
    ] {
        let mut data = FontData::from_static(PLEX_SANS);
        data.tweak.coords = VariationCoords::new([(b"wght", weight)]);
        fonts
            .font_data
            .insert(name.to_owned(), scaled(data, PLEX_SANS_SCALE));
    }
    lead(fonts, FontFamily::Proportional, "plex-sans");
    lead(fonts, FontFamily::Name(MEDIUM.into()), "plex-sans-medium");
    lead(
        fonts,
        FontFamily::Name(SEMIBOLD.into()),
        "plex-sans-semibold",
    );
    // Only the three weights the design allows: bold is semibold.
    lead(fonts, FontFamily::Name(BOLD.into()), "plex-sans-semibold");
    fonts.font_data.insert(
        "plex-mono".into(),
        scaled(FontData::from_static(PLEX_MONO), PLEX_MONO_SCALE),
    );
    lead(fonts, FontFamily::Monospace, "plex-mono");
    fonts.font_data.insert(
        "plex-mono-medium".into(),
        scaled(FontData::from_static(PLEX_MONO_MEDIUM), PLEX_MONO_SCALE),
    );
    let fallbacks = tail(fonts, &FontFamily::Monospace);
    for family in [MONO_MEDIUM, MONO_BOLD] {
        let mut list = vec!["plex-mono-medium".to_owned()];
        list.extend(fallbacks.iter().cloned());
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
}

#[cfg(not(any(target_os = "macos", test, feature = "shots")))]
fn plex(_fonts: &mut FontDefinitions) {}

/// The desktop's monospace at 400, 500 and 700, when fontconfig has it.
fn desktop_faces() -> Option<[Arc<FontData>; 3]> {
    let regular = super::desktop_font::monospace("monospace")?;
    let medium = super::desktop_font::monospace("monospace:medium");
    let bold = super::desktop_font::monospace("monospace:bold");
    let regular = Arc::new(regular);
    let medium = medium.map_or_else(|| regular.clone(), Arc::new);
    let bold = bold.map_or_else(|| medium.clone(), Arc::new);
    Some([regular, medium, bold])
}

#[cfg(any(target_os = "linux", test, feature = "shots"))]
fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
    Some([
        weighted(JETBRAINS_MONO, 400.0),
        weighted(JETBRAINS_MONO, 500.0),
        weighted(JETBRAINS_MONO, 700.0),
    ])
}

#[cfg(not(any(target_os = "linux", test, feature = "shots")))]
fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
    None
}

/// The terminal face's scale: egui draws a monospace face larger than a
/// terminal or browser does at the same size, and the design sets it a
/// little tighter still.
const TERMINAL_SCALE: f32 = 0.82;

/// Omarchy: one monospace face for the interface and the data alike.
fn terminal(fonts: &mut FontDefinitions, desktop: bool) {
    let faces = desktop
        .then(desktop_faces)
        .flatten()
        .or_else(bundled_terminal_faces);
    let Some([regular, medium, bold]) = faces else {
        return;
    };
    for (name, data) in [
        ("terminal", regular),
        ("terminal-medium", medium),
        ("terminal-bold", bold),
    ] {
        let mut data = Arc::unwrap_or_clone(data);
        data.tweak.scale = TERMINAL_SCALE;
        fonts.font_data.insert(name.to_owned(), Arc::new(data));
    }
    lead(fonts, FontFamily::Monospace, "terminal");
    lead(fonts, FontFamily::Proportional, "terminal");
    lead(fonts, FontFamily::Name(MEDIUM.into()), "terminal-medium");
    lead(fonts, FontFamily::Name(SEMIBOLD.into()), "terminal-bold");
    lead(fonts, FontFamily::Name(BOLD.into()), "terminal-bold");
    for (family, face) in [
        (MONO_MEDIUM, "terminal-medium"),
        (MONO_BOLD, "terminal-bold"),
    ] {
        let mut list = vec![face.to_owned()];
        list.extend(tail(fonts, &FontFamily::Monospace));
        fonts.families.insert(FontFamily::Name(family.into()), list);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn configured(look: &Look) -> FontDefinitions {
        let mut fonts = fastframe_fonts::FontSetup::default()
            .system_fallbacks(false)
            .definitions();
        configure(&mut fonts, look, false);
        fonts
    }

    fn first(fonts: &FontDefinitions, family: FontFamily) -> String {
        fonts.families[&family][0].clone()
    }

    #[test]
    fn macos_draws_plex_and_omarchy_one_monospace_face() {
        let mac = configured(&Look::macos());
        assert_eq!(first(&mac, FontFamily::Proportional), "plex-sans");
        assert_eq!(first(&mac, FontFamily::Monospace), "plex-mono");
        assert_eq!(
            first(&mac, FontFamily::Name(MONO_BOLD.into())),
            "plex-mono-medium"
        );
        let omarchy = configured(&Look::omarchy());
        assert_eq!(first(&omarchy, FontFamily::Proportional), "terminal");
        assert_eq!(first(&omarchy, FontFamily::Monospace), "terminal");
        assert_eq!(
            first(&omarchy, FontFamily::Name(SEMIBOLD.into())),
            "terminal-bold"
        );
        let standard = configured(&Look::standard());
        assert_eq!(first(&standard, FontFamily::Proportional), "inter");
        assert!(
            standard
                .families
                .contains_key(&FontFamily::Name(MONO_BOLD.into()))
        );
    }
}
