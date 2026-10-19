//! The installed fonts macOS can draw data in, beside the two it bundles:
//! the monospace families CoreText lists, for the data font's menu
//! ([`families`]), and the faces of the one the settings name, read from
//! their files ([`read`]).
//!
//! Only the CoreText calls are macOS's (in `tabletist-appkit`). What is
//! made of their answers is the same everywhere, and tested with fonts of
//! the tests' own.

use std::path::PathBuf;
use std::sync::Arc;

use egui::FontData;
use egui::epaint::text::VariationCoords;
use skrifa::raw::FileRef;
use skrifa::string::StringId;
use skrifa::{FontRef, MetadataProvider, Tag};

use super::desktop_font::Read;
use crate::theme::DataFace;

/// One face of an installed family, as CoreText describes it.
#[derive(Clone, Debug, PartialEq)]
pub struct Face {
    /// What tells the face from the others in a collection file.
    pub postscript_name: String,
    pub path: PathBuf,
    /// CoreText's weight, from -1 to 1: 0 is regular, 0.23 medium.
    pub weight: f64,
    /// CoreText's width, from -1 to 1: 0 is normal.
    pub width: f64,
    pub italic: bool,
}

/// CoreText's weights of the two the data is drawn at (400 and 500), and
/// of the heaviest a medium may be (semibold, 600).
const REGULAR: f64 = 0.0;
const MEDIUM: f64 = 0.23;
const SEMIBOLD: f64 = 0.3;

/// CoreText's weights against CSS's, as AppKit's `NSFontWeight` constants
/// pair them (ultralight to black).
const WEIGHTS: [(f64, f32); 9] = [
    (-0.8, 100.0),
    (-0.6, 200.0),
    (-0.4, 300.0),
    (0.0, 400.0),
    (0.23, 500.0),
    (0.3, 600.0),
    (0.4, 700.0),
    (0.56, 800.0),
    (0.62, 900.0),
];

/// CoreText's `weight` as a CSS weight, between the constants' by a
/// straight line: what a variable font's `wght` axis is set to.
pub fn css_weight(weight: f64) -> f32 {
    let (first, last) = (WEIGHTS[0], WEIGHTS[WEIGHTS.len() - 1]);
    if weight <= first.0 {
        return first.1;
    }
    WEIGHTS
        .windows(2)
        .find(|pair| weight <= pair[1].0)
        .map_or(last.1, |pair| {
            let ((from, low), (to, high)) = (pair[0], pair[1]);
            let along = ((weight - from) / (to - from)) as f32;
            low + along * (high - low)
        })
}

/// The installed monospace families the menu lists after the bundled
/// faces: as [`listed`] has them. None off macOS.
pub fn families() -> Vec<String> {
    listed(installed_families())
}

/// `families` as the menu lists them: each once, whatever the case, sorted
/// whatever the case, and without the bundled faces' names (they are
/// listed first), `monospace`, and the system's hidden families, whose
/// names begin with a full stop (`.SF NS Mono`).
pub fn listed(families: Vec<String>) -> Vec<String> {
    let mut listed: Vec<String> = families
        .into_iter()
        .filter(|family| {
            let family = family.trim();
            !family.is_empty() && !family.starts_with('.') && DataFace::of(Some(family)).is_err()
        })
        .collect();
    listed.sort_by_cached_key(|family| family.to_lowercase());
    listed.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
    listed
}

/// The face of `faces` nearest CoreText's `weight`: upright and of the
/// normal width, where the family has such a face.
pub fn nearest(faces: &[Face], weight: f64) -> Option<&Face> {
    let upright = faces.iter().any(|face| !face.italic);
    let normal = |face: &Face| face.width.abs() < 0.01;
    let candidates = faces.iter().filter(|face| !(upright && face.italic));
    let any_normal = candidates.clone().any(normal);
    candidates
        .filter(|face| !any_normal || normal(face))
        .min_by(|a, b| {
            let off = |face: &Face| (face.weight - weight).abs();
            off(a)
                .total_cmp(&off(b))
                .then_with(|| a.postscript_name.cmp(&b.postscript_name))
        })
}

/// The faces the data is drawn in, at 400 and 500: the regular, and a
/// medium or semibold heavier than it, or the regular again where the
/// family has none (Menlo has a regular and a bold).
pub fn regular_and_medium(faces: &[Face]) -> Option<(&Face, &Face)> {
    let regular = nearest(faces, REGULAR)?;
    let medium = nearest(faces, MEDIUM)
        .filter(|medium| medium.weight > regular.weight && medium.weight <= SEMIBOLD + 1e-6)
        .unwrap_or(regular);
    Some((regular, medium))
}

/// Whether a face has outlines (TrueType `glyf` or CFF), which egui draws:
/// a bitmap-only one parses, and would draw nothing.
pub fn outlined(font: &FontRef) -> bool {
    [b"glyf", b"CFF ", b"CFF2"]
        .into_iter()
        .any(|tag| font.table_data(Tag::new(tag)).is_some())
}

/// The face's index in the font file `bytes`: the face whose PostScript
/// name is `postscript_name` in a collection, the one face in a single
/// font (a variable font's named instance has another name than the file's
/// face).
fn index_of(bytes: &[u8], postscript_name: &str) -> Option<u32> {
    match FileRef::new(bytes).ok()? {
        FileRef::Font(_) => Some(0),
        FileRef::Collection(collection) => (0..collection.len()).find(|&index| {
            collection.get(index).is_ok_and(|font| {
                font.localized_strings(StringId::POSTSCRIPT_NAME)
                    .english_or_first()
                    .is_some_and(|name| name.to_string() == postscript_name)
            })
        }),
    }
}

/// `face`, read from `bytes`, its file, for egui: at its weight where the
/// font is variable in weight. `None` where the file has no such face, or
/// one without outlines.
pub fn load(bytes: Vec<u8>, face: &Face) -> Option<FontData> {
    let index = index_of(&bytes, &face.postscript_name)?;
    let font = FontRef::from_index(&bytes, index).ok()?;
    if !outlined(&font) {
        return None;
    }
    let weight = font
        .axes()
        .get_by_tag(Tag::new(b"wght"))
        .map(|axis| css_weight(face.weight).clamp(axis.min_value(), axis.max_value()));
    let mut data = FontData::from_owned(bytes);
    data.index = index;
    if let Some(weight) = weight {
        data.tweak.coords = VariationCoords::new([(b"wght", weight)]);
    }
    Some(data)
}

/// `face` read from its file, or `None` with one warning.
fn load_file(face: &Face) -> Option<FontData> {
    let data = std::fs::read(&face.path)
        .ok()
        .and_then(|bytes| load(bytes, face));
    if data.is_none() {
        log::warn!(
            "could not read the font {} in {}, or it has no outlines",
            face.postscript_name,
            face.path.display()
        );
    }
    data
}

/// The installed font the settings name for the data, read at 400 and 500
/// (the third face, at 700, which the data is never drawn at, is the
/// 500's). Nothing is read for none, for `monospace` and for a bundled
/// face's name. A name that no installed family has is `missing`: the
/// default is drawn in its place. CoreText's answers, then the font files:
/// at the start (as the fonts always were) and on the backend after.
pub fn read(asked: Option<&str>) -> Read {
    read_with(asked, &installed_faces, &installed_families)
}

/// [`read`], with the installed fonts as `faces_of` and `families` give
/// them.
pub fn read_with(
    asked: Option<&str>,
    faces_of: &dyn Fn(&str) -> Vec<Face>,
    families: &dyn Fn() -> Vec<String>,
) -> Read {
    let none = Read {
        asked: asked.map(str::to_owned),
        ..Read::default()
    };
    let named = asked
        .map(str::trim)
        .filter(|name| !name.is_empty() && DataFace::of(Some(name)).is_err());
    let Some(name) = named else {
        return none;
    };
    let mut family = name.to_owned();
    let mut faces = faces_of(name);
    if faces.is_empty() {
        // A name written by hand, in another case than CoreText's, where
        // CoreText's own lookup does not find it whatever the case.
        if let Some(found) = families()
            .into_iter()
            .find(|found| found.eq_ignore_ascii_case(name))
        {
            faces = faces_of(&found);
            family = found;
        }
    }
    let loaded = regular_and_medium(&faces).and_then(|(regular, medium)| {
        let drawn = Arc::new(load_file(regular)?);
        let heavier = if medium == regular {
            drawn.clone()
        } else {
            load_file(medium).map_or_else(|| drawn.clone(), Arc::new)
        };
        Some([drawn, heavier.clone(), heavier])
    });
    match loaded {
        Some(faces) => Read {
            faces: Some(faces),
            family: Some(family),
            ..none
        },
        None => {
            log::warn!("the font {name:?} is not installed: IBM Plex Mono is drawn");
            Read {
                missing: true,
                ..none
            }
        }
    }
}

#[cfg(target_os = "macos")]
fn installed_families() -> Vec<String> {
    tabletist_appkit::monospace_families()
}

#[cfg(not(target_os = "macos"))]
fn installed_families() -> Vec<String> {
    Vec::new()
}

#[cfg(target_os = "macos")]
fn installed_faces(family: &str) -> Vec<Face> {
    tabletist_appkit::faces_of(family)
        .into_iter()
        .map(|face| Face {
            postscript_name: face.postscript_name,
            path: face.path,
            weight: face.weight,
            width: face.width,
            italic: face.italic,
        })
        .collect()
}

#[cfg(not(target_os = "macos"))]
fn installed_faces(_family: &str) -> Vec<Face> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A face of a family of the test's own.
    fn face(name: &str, weight: f64) -> Face {
        Face {
            postscript_name: name.into(),
            path: PathBuf::from(format!("/fonts/{name}.otf")),
            weight,
            width: 0.0,
            italic: false,
        }
    }

    fn names<'a>(faces: Option<(&'a Face, &'a Face)>) -> Option<(&'a str, &'a str)> {
        faces.map(|(regular, medium)| {
            (
                regular.postscript_name.as_str(),
                medium.postscript_name.as_str(),
            )
        })
    }

    /// A bundled font's file, as an installed face of `name`.
    fn bundled(name: &str, file: &str, weight: f64) -> Face {
        Face {
            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets/fonts")
                .join(file),
            ..face(name, weight)
        }
    }

    #[test]
    fn the_menu_lists_each_family_once_and_not_the_bundled_ones() {
        let families = [
            "Menlo",
            ".SF NS Mono",
            "fira code",
            "JetBrains Mono",
            "Fira Code",
            "Andale Mono",
            "IBM Plex Mono",
            "  ",
            "Monospace",
        ];
        let listed = listed(families.map(str::to_owned).to_vec());
        assert_eq!(listed, ["Andale Mono", "fira code", "Menlo"]);
    }

    #[test]
    fn the_data_is_drawn_in_the_regular_and_a_medium_where_there_is_one() {
        let family = [
            face("Mono-Bold", 0.4),
            face("Mono-Light", -0.4),
            face("Mono-Regular", 0.0),
            face("Mono-Medium", 0.23),
        ];
        assert_eq!(
            names(regular_and_medium(&family)),
            Some(("Mono-Regular", "Mono-Medium"))
        );
        // Menlo: a bold is no medium, the regular is drawn at 500 too.
        let menlo = [face("Menlo-Regular", 0.0), face("Menlo-Bold", 0.4)];
        assert_eq!(
            names(regular_and_medium(&menlo)),
            Some(("Menlo-Regular", "Menlo-Regular"))
        );
        // A semibold is one.
        let semibold = [face("S-Regular", 0.0), face("S-Semibold", 0.3)];
        assert_eq!(
            names(regular_and_medium(&semibold)),
            Some(("S-Regular", "S-Semibold"))
        );
        // A family of one face draws it at both.
        let light = [face("L-Light", -0.4)];
        assert_eq!(
            names(regular_and_medium(&light)),
            Some(("L-Light", "L-Light"))
        );
        assert_eq!(names(regular_and_medium(&[])), None);
    }

    #[test]
    fn an_upright_face_of_normal_width_is_taken_over_a_nearer_one() {
        let italic = Face {
            italic: true,
            ..face("Mono-Italic", 0.0)
        };
        let condensed = Face {
            width: -0.2,
            ..face("Mono-Condensed", 0.0)
        };
        let family = [italic.clone(), condensed.clone(), face("Mono-Book", 0.1)];
        assert_eq!(
            nearest(&family, 0.0).map(|face| face.postscript_name.as_str()),
            Some("Mono-Book")
        );
        // Where the family has no other, it is drawn.
        assert_eq!(nearest(std::slice::from_ref(&italic), 0.0), Some(&italic));
        assert_eq!(nearest(&[italic, condensed.clone()], 0.0), Some(&condensed));
    }

    #[test]
    fn coretext_weights_are_css_weights() {
        for (weight, css) in [
            (-1.0, 100.0),
            (-0.4, 300.0),
            (0.0, 400.0),
            (0.115, 450.0),
            (0.23, 500.0),
            (0.4, 700.0),
            (1.0, 900.0),
        ] {
            assert!((css_weight(weight) - css).abs() < 0.01, "{weight}: {css}");
        }
    }

    #[test]
    fn a_variable_face_is_read_at_its_weight() {
        let medium = bundled("JetBrainsMono-Medium", "JetBrainsMono-Variable.ttf", 0.23);
        let bytes = std::fs::read(&medium.path).unwrap();
        let data = load(bytes, &medium).expect("JetBrains Mono reads");
        assert_eq!(data.index, 0);
        let expected = VariationCoords::new([(b"wght", 500.0)]);
        assert_eq!(data.tweak.coords, expected);
        // A face of fixed weight is read as it is.
        let plex = bundled("IBMPlexMono", "IBMPlexMono-Regular.ttf", 0.0);
        let data = load(std::fs::read(&plex.path).unwrap(), &plex).unwrap();
        assert_eq!(data.tweak.coords, VariationCoords::default());
        // Bytes that are no font are none.
        assert!(load(b"not a font".to_vec(), &plex).is_none());
    }

    #[test]
    fn the_font_the_settings_name_is_read_from_its_files() {
        let faces_of = |family: &str| match family {
            "Fira Code" => vec![
                bundled("Fira-Regular", "IBMPlexMono-Regular.ttf", 0.0),
                bundled("Fira-Medium", "IBMPlexMono-Medium.ttf", 0.23),
            ],
            "Gone" => vec![face("Gone-Regular", 0.0)],
            _ => Vec::new(),
        };
        let families = || vec!["Fira Code".to_owned(), "Gone".to_owned()];
        let read = read_with(Some("Fira Code"), &faces_of, &families);
        assert_eq!(read.asked.as_deref(), Some("Fira Code"));
        assert_eq!(read.family.as_deref(), Some("Fira Code"));
        assert!(!read.missing);
        let faces = read.faces.expect("read");
        let plex_mono_medium = include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf");
        assert_eq!(faces[0].font.len(), super::super::fonts::PLEX_MONO.len());
        assert_eq!(faces[1].font.len(), plex_mono_medium.len());
        assert!(Arc::ptr_eq(&faces[1], &faces[2]));
        // Written by hand in another case: CoreText's name is drawn.
        let read = read_with(Some(" fira CODE "), &faces_of, &families);
        assert_eq!(read.family.as_deref(), Some("Fira Code"));
        assert!(read.faces.is_some());
        // Not installed, or its file not there: the default, said.
        for name in ["Iosevka", "Gone"] {
            let read = read_with(Some(name), &faces_of, &families);
            assert!(read.missing, "{name}");
            assert!(read.faces.is_none() && read.family.is_none(), "{name}");
            assert_eq!(read.asked.as_deref(), Some(name));
        }
    }

    /// The Mac that runs the tests has Menlo, in a collection: CoreText's
    /// answers, read. Nothing elsewhere.
    #[cfg(target_os = "macos")]
    #[test]
    fn menlo_is_read_from_the_mac() {
        assert!(families().iter().any(|family| family == "Menlo"));
        for asked in ["Menlo", "menlo"] {
            let read = read(Some(asked));
            assert!(!read.missing, "{asked}");
            // CoreText finds a family whatever the case it is asked in:
            // the name stays as the settings write it.
            assert_eq!(read.family.as_deref(), Some(asked));
            let faces = read.faces.expect("Menlo's faces");
            // The regular, found in the collection by its name; Menlo has
            // no medium, so the regular again at 500.
            let font = FontRef::from_index(&faces[0].font, faces[0].index).unwrap();
            let name = font
                .localized_strings(StringId::POSTSCRIPT_NAME)
                .english_or_first()
                .map(|name| name.to_string());
            assert_eq!(name.as_deref(), Some("Menlo-Regular"));
            assert!(Arc::ptr_eq(&faces[0], &faces[1]));
        }
        assert!(read(Some("No Such Mono Of Ours")).missing);
    }

    #[test]
    fn nothing_is_read_for_the_bundled_faces() {
        let faces_of = |family: &str| -> Vec<Face> { panic!("{family} looked up") };
        let families = || -> Vec<String> { panic!("families listed") };
        for asked in [
            None,
            Some("JetBrains Mono"),
            Some("ibm plex mono"),
            Some("monospace"),
            Some(" "),
        ] {
            let read = read_with(asked, &faces_of, &families);
            assert_eq!(read.asked.as_deref(), asked);
            assert!(read.faces.is_none() && !read.missing, "{asked:?}");
        }
    }
}
