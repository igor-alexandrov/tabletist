//! The installed fonts as CoreText lists them, on the Mac that runs the
//! tests: Menlo, which every macOS has had since 10.6, stands for the rest.
//! Nothing elsewhere. CoreText may be asked on any thread: the harness's
//! own.

#![cfg(target_os = "macos")]

use tabletist_appkit::{faces_of, monospace_families};

#[test]
fn menlo_is_listed_monospace_and_helvetica_is_not() {
    let families = monospace_families();
    assert!(
        families.iter().any(|family| family == "Menlo"),
        "{families:?}"
    );
    assert!(!families.iter().any(|family| family == "Helvetica"));
    // Each once.
    let mut once = families.clone();
    once.sort();
    once.dedup();
    assert_eq!(once.len(), families.len());
}

#[test]
fn menlos_faces_are_in_its_file_at_their_weights() {
    let faces = faces_of("Menlo");
    let face = |name: &str| {
        faces
            .iter()
            .find(|face| face.postscript_name == name)
            .unwrap_or_else(|| panic!("{name} in {faces:?}"))
    };
    let regular = face("Menlo-Regular");
    assert!(regular.path.is_file(), "{regular:?}");
    assert!(regular.weight.abs() < 0.01, "{regular:?}");
    assert!(regular.width.abs() < 0.01, "{regular:?}");
    assert!(!regular.italic);
    let bold = face("Menlo-Bold");
    assert!(bold.weight > 0.3, "{bold:?}");
    assert!(face("Menlo-Italic").italic);
    assert!(faces_of("No Such Family Of Fonts").is_empty());
}
