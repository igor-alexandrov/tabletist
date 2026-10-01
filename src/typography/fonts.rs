//! The font files behind each look, one egui family per weight.
//!
//! - Standard: Inter (fastframe's set-up), with egui's monospace for data.
//! - macOS: IBM Plex Sans (400, 500, 600) and IBM Plex Mono (400, 500),
//!   bundled: `plex-sans-400` ... `plex-mono-500`.
//! - Omarchy: the desktop's monospace font at 400, 500 and 700, or the
//!   bundled JetBrains Mono when fontconfig finds none: `omarchy-mono-400`
//!   ... `omarchy-mono-700`.
//!
//! egui's own Proportional and Monospace families (and fastframe's Inter
//! weight families, which the edit connection dialog still names) lead with
//! the look's regular faces, so text egui draws by itself matches.

use std::sync::Arc;

#[cfg(any(target_os = "macos", target_os = "linux", test))]
use egui::epaint::text::VariationCoords;
use egui::{FontData, FontDefinitions, FontFamily};

use super::{Kind, family_name};
use crate::theme::{Faces, Look};

/// IBM Plex Sans, variable in weight and width (SIL OFL 1.1).
#[cfg(any(target_os = "macos", test))]
pub(crate) const PLEX_SANS: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Variable.ttf");
#[cfg(any(target_os = "macos", test))]
pub(crate) const PLEX_MONO: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf");
#[cfg(any(target_os = "macos", test))]
pub(crate) const PLEX_MONO_MEDIUM: &[u8] =
    include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf");
/// Noto Sans Symbols 2 cut down to the keyboard symbols Plex lacks (⌘ ⌥ ⇧
/// ⌫ ⌦ ⏎), for the macOS shortcuts (SIL OFL 1.1).
#[cfg(any(target_os = "macos", test))]
pub(crate) const KEYS: &[u8] = include_bytes!("../../assets/fonts/NotoSansSymbols2-Keys.ttf");
/// JetBrains Mono, variable in weight (SIL OFL 1.1): Omarchy's default, for
/// desktops without a monospace font of their own.
#[cfg(any(target_os = "linux", test))]
pub(crate) const JETBRAINS_MONO: &[u8] =
    include_bytes!("../../assets/fonts/JetBrainsMono-Variable.ttf");

/// A face's line box in ems: what epaint places its glyphs by.
#[cfg(any(target_os = "macos", test))]
#[derive(Clone, Copy)]
struct Metrics {
    ascent: f32,
    descent: f32,
}

#[cfg(any(target_os = "macos", test))]
impl Metrics {
    /// The baseline, below the middle of the line box.
    fn baseline(self) -> f32 {
        (self.ascent - self.descent) / 2.0
    }
}

/// IBM Plex's line box, Sans and Mono alike.
#[cfg(any(target_os = "macos", test))]
const PLEX_METRICS: Metrics = Metrics {
    ascent: 1.025,
    descent: 0.275,
};
/// The key symbols' line box: it reaches far deeper below the baseline.
#[cfg(any(target_os = "macos", test))]
const KEYS_METRICS: Metrics = Metrics {
    ascent: 1.069,
    descent: 0.63,
};
/// The height of Plex's capitals in ems, Sans and Mono alike.
const PLEX_CAPITALS: f32 = 0.698;

/// The height of `faces`' capitals in ems, where the line box does not
/// centre on them: Plex's centres a little above its capitals. Inter's and
/// JetBrains Mono's centre on theirs, and a desktop font's are not known.
pub fn capitals(faces: Faces) -> Option<f32> {
    // Off macOS the Plex look draws in Inter.
    (faces == Faces::Plex && cfg!(any(target_os = "macos", test))).then_some(PLEX_CAPITALS)
}

/// fastframe's families for Inter's named weights.
const MEDIUM: &str = "inter-medium";
const SEMIBOLD: &str = "inter-semibold";
const BOLD: &str = "inter-bold";

/// A variable face at `weight`.
#[cfg(any(target_os = "macos", target_os = "linux", test))]
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

/// A family of its own for `face`, with `family`'s fallbacks behind it.
fn own(fonts: &mut FontDefinitions, name: &str, face: &str, fallbacks: &FontFamily) {
    let mut list = vec![face.to_owned()];
    list.extend(tail(fonts, fallbacks));
    fonts.families.insert(FontFamily::Name(name.into()), list);
}

/// A family named `name` holding what `from` holds.
fn alias(fonts: &mut FontDefinitions, name: &str, from: &FontFamily) {
    let list = fonts.families.get(from).cloned().unwrap_or_default();
    fonts.families.insert(FontFamily::Name(name.into()), list);
}

/// Replaces the faces fastframe set up with the look's. `desktop` allows
/// reading the desktop's own monospace font (never in tests or screenshots).
pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: bool) {
    match look.faces {
        Faces::Inter => inter(fonts),
        Faces::Plex => plex(fonts),
        Faces::Terminal => terminal(fonts, desktop),
    }
}

/// Inter's weights under the role families' names.
fn inter(fonts: &mut FontDefinitions) {
    for (weight, from) in [
        (400, FontFamily::Proportional),
        (500, FontFamily::Name(MEDIUM.into())),
        (600, FontFamily::Name(SEMIBOLD.into())),
        (700, FontFamily::Name(BOLD.into())),
    ] {
        alias(fonts, &family_name(Faces::Inter, Kind::Sans, weight), &from);
    }
    alias(
        fonts,
        &family_name(Faces::Inter, Kind::Mono, 400),
        &FontFamily::Monospace,
    );
}

#[cfg(any(target_os = "macos", test))]
fn plex(fonts: &mut FontDefinitions) {
    for weight in [400, 500, 600] {
        let name = family_name(Faces::Plex, Kind::Sans, weight);
        fonts
            .font_data
            .insert(name.clone(), weighted(PLEX_SANS, f32::from(weight)));
        own(fonts, &name, &name, &FontFamily::Proportional);
    }
    for (weight, bytes) in [(400, PLEX_MONO), (500, PLEX_MONO_MEDIUM)] {
        let name = family_name(Faces::Plex, Kind::Mono, weight);
        fonts
            .font_data
            .insert(name.clone(), Arc::new(FontData::from_static(bytes)));
        own(fonts, &name, &name, &FontFamily::Monospace);
    }
    let sans = |weight| family_name(Faces::Plex, Kind::Sans, weight);
    lead(fonts, FontFamily::Proportional, &sans(400));
    lead(fonts, FontFamily::Name(MEDIUM.into()), &sans(500));
    lead(fonts, FontFamily::Name(SEMIBOLD.into()), &sans(600));
    // Only the three weights the design allows: bold is semibold.
    lead(fonts, FontFamily::Name(BOLD.into()), &sans(600));
    lead(
        fonts,
        FontFamily::Monospace,
        &family_name(Faces::Plex, Kind::Mono, 400),
    );
    // The keyboard symbols right behind each face, ahead of the fallbacks.
    // epaint centres a fallback face's line box on the leading face's
    // instead of sharing its baseline, which would stand the symbols well
    // above the letters: the shift puts them on Plex's baseline.
    let mut keys = FontData::from_static(KEYS);
    keys.tweak.y_offset_factor = PLEX_METRICS.baseline() - KEYS_METRICS.baseline();
    fonts.font_data.insert("keys".into(), Arc::new(keys));
    for list in fonts.families.values_mut() {
        if list.first().is_some_and(|face| face.starts_with("plex-")) {
            list.retain(|face| face != "keys");
            list.insert(1, "keys".into());
        }
    }
}

#[cfg(not(any(target_os = "macos", test)))]
fn plex(fonts: &mut FontDefinitions) {
    inter(fonts);
}

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

#[cfg(any(target_os = "linux", test))]
fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
    Some([
        weighted(JETBRAINS_MONO, 400.0),
        weighted(JETBRAINS_MONO, 500.0),
        weighted(JETBRAINS_MONO, 700.0),
    ])
}

#[cfg(not(any(target_os = "linux", test)))]
fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
    None
}

/// Omarchy: one monospace face for the interface and the data alike.
fn terminal(fonts: &mut FontDefinitions, desktop: bool) {
    let faces = desktop
        .then(desktop_faces)
        .flatten()
        .or_else(bundled_terminal_faces);
    let Some(faces) = faces else {
        // No monospace of its own: the families still exist, on Inter.
        for weight in [400, 500, 700] {
            let from = if weight == 400 {
                FontFamily::Monospace
            } else {
                FontFamily::Name(if weight == 500 { MEDIUM } else { BOLD }.into())
            };
            alias(
                fonts,
                &family_name(Faces::Terminal, Kind::Mono, weight),
                &from,
            );
        }
        return;
    };
    for (weight, data) in [400, 500, 700].into_iter().zip(faces) {
        let name = family_name(Faces::Terminal, Kind::Mono, weight);
        fonts.font_data.insert(name.clone(), data);
        own(fonts, &name, &name, &FontFamily::Monospace);
    }
    let face = |weight| family_name(Faces::Terminal, Kind::Mono, weight);
    lead(fonts, FontFamily::Monospace, &face(400));
    lead(fonts, FontFamily::Proportional, &face(400));
    lead(fonts, FontFamily::Name(MEDIUM.into()), &face(500));
    lead(fonts, FontFamily::Name(SEMIBOLD.into()), &face(700));
    lead(fonts, FontFamily::Name(BOLD.into()), &face(700));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::typography::{Text, TextRole};

    #[test]
    fn macos_draws_the_keys_its_shortcuts_name() {
        let mut harness = crate::testing::Harness::new();
        harness.set_look(Look::macos());
        harness.settle();
        for role in [
            crate::typography::TextRole::Secondary,
            crate::typography::TextRole::Shortcut,
        ] {
            let font = role.font_id(Faces::Plex);
            assert!(
                harness
                    .ctx
                    .fonts_mut(|fonts| fonts.has_glyphs(&font, "⌘⌫↩")),
                "{}: a replacement box instead of a key",
                role.id()
            );
        }
    }

    /// A context drawing in the macOS faces at `pixels_per_point`.
    fn macos_at(pixels_per_point: f32) -> egui::Context {
        let ctx = egui::Context::default();
        ctx.set_pixels_per_point(pixels_per_point);
        ctx.set_fonts(configured(&Look::macos()));
        ctx.run_ui(egui::RawInput::default(), |_| {})
            .textures_delta
            .clear();
        ctx
    }

    /// The bottom of `character`'s texture as `galley` paints it, the font's
    /// tweak included (`glyph.pos` alone misses it): the baseline, for a
    /// glyph that stands on it.
    fn painted_bottom(galley: &egui::Galley, character: char) -> f32 {
        let row = &galley.rows[0];
        let glyph = row
            .glyphs
            .iter()
            .find(|glyph| glyph.chr == character)
            .expect("the character is laid out");
        let first = glyph.first_vertex as usize;
        row.visuals.mesh.vertices[first..first + 4]
            .iter()
            .map(|vertex| vertex.pos.y)
            .fold(f32::NEG_INFINITY, f32::max)
    }

    #[test]
    fn macos_key_symbols_stand_on_the_letters_baseline() {
        for pixels_per_point in [1.0, 2.0] {
            let ctx = macos_at(pixels_per_point);
            for role in [
                TextRole::Secondary,
                TextRole::Shortcut,
                TextRole::MonoSecondary,
            ] {
                // Each of these symbols stands on the baseline, like the N.
                let laid = Text::in_faces(Faces::Plex)
                    .add(role, "N⌘⌫⌦", egui::Color32::WHITE)
                    .layout(&ctx);
                let letter = painted_bottom(&laid.galley, 'N');
                for key in ['⌘', '⌫', '⌦'] {
                    // Glyphs and their offsets snap to pixels separately.
                    let apart = (painted_bottom(&laid.galley, key) - letter).abs();
                    assert!(
                        apart * pixels_per_point <= 1.01,
                        "{} at {pixels_per_point}x: {key} is {apart} pt off the baseline",
                        role.id()
                    );
                }
            }
        }
    }

    /// An icon is centred on the line its text is painted on, and reads as
    /// centred when the capitals beside it are.
    #[test]
    fn macos_text_is_painted_with_its_capitals_centred() {
        for pixels_per_point in [1.0, 2.0] {
            let ctx = macos_at(pixels_per_point);
            let plex = TextRole::ALL
                .into_iter()
                .filter(|role| !role.id().starts_with("o-"));
            for role in plex {
                for text in ["H", "⌘H"] {
                    let laid = Text::in_faces(Faces::Plex)
                        .add(role, text, egui::Color32::WHITE)
                        .layout(&ctx);
                    for line in [32.0, 100.3] {
                        // epaint snaps the text's corner to a pixel.
                        let top =
                            ((line - laid.middle()) * pixels_per_point).round() / pixels_per_point;
                        let baseline = top + painted_bottom(&laid.galley, 'H');
                        let middle = baseline - PLEX_CAPITALS * role.size() / 2.0;
                        let off = (middle - line).abs() * pixels_per_point;
                        assert!(
                            off <= 0.51,
                            "{} {text:?} at {pixels_per_point}x: {off} px off the line",
                            role.id()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn only_plex_centres_text_by_its_capitals() {
        assert_eq!(capitals(Faces::Plex), Some(PLEX_CAPITALS));
        assert_eq!(capitals(Faces::Inter), None);
        assert_eq!(capitals(Faces::Terminal), None);
    }

    /// The metrics above are the bundled files' own.
    #[cfg(target_os = "linux")]
    #[test]
    fn the_metrics_are_the_bundled_fonts() {
        use skrifa::MetadataProvider as _;
        let same = |read: f32, expected: f32| (read - expected).abs() < 1e-6;
        for (bytes, expected, capitals) in [
            (PLEX_SANS, PLEX_METRICS, Some(PLEX_CAPITALS)),
            (PLEX_MONO, PLEX_METRICS, Some(PLEX_CAPITALS)),
            (PLEX_MONO_MEDIUM, PLEX_METRICS, Some(PLEX_CAPITALS)),
            (KEYS, KEYS_METRICS, None),
        ] {
            let font = skrifa::FontRef::new(bytes).expect("a bundled font parses");
            let metrics = font.metrics(
                skrifa::instance::Size::unscaled(),
                skrifa::instance::LocationRef::default(),
            );
            let units = f32::from(metrics.units_per_em);
            assert!(same(metrics.leading, 0.0), "no line gap to count");
            assert!(same(metrics.ascent / units, expected.ascent));
            assert!(same(-metrics.descent / units, expected.descent));
            if let Some(capitals) = capitals {
                let read = metrics.cap_height.expect("Plex has a cap height");
                assert!(same(read / units, capitals));
            }
        }
    }

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
        assert_eq!(first(&mac, FontFamily::Proportional), "plex-sans-400");
        assert_eq!(first(&mac, FontFamily::Monospace), "plex-mono-400");
        assert_eq!(
            first(&mac, FontFamily::Name("plex-mono-500".into())),
            "plex-mono-500"
        );
        let omarchy = configured(&Look::omarchy());
        assert_eq!(
            first(&omarchy, FontFamily::Proportional),
            "omarchy-mono-400"
        );
        assert_eq!(first(&omarchy, FontFamily::Monospace), "omarchy-mono-400");
        assert_eq!(
            first(&omarchy, FontFamily::Name(SEMIBOLD.into())),
            "omarchy-mono-700"
        );
        let standard = configured(&Look::standard());
        assert_eq!(first(&standard, FontFamily::Proportional), "inter");
        assert_eq!(
            first(&standard, FontFamily::Name("inter-600".into())),
            first(&standard, FontFamily::Name(SEMIBOLD.into()))
        );
    }
}
