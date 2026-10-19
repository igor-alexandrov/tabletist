//! The fonts installed on the Mac, as CoreText lists them: the monospace
//! families, for the data font's menu, and the faces of one family, with
//! the file each is in, for the app to read.
//!
//! CoreText's font collections and descriptors may be used on any thread:
//! these are called on the backend's, which drain no autorelease pool of
//! their own, so each call drains one.

// CoreText's attribute keys are C statics, and its arrays and dictionaries
// hold untyped values: reading either is a step the compiler cannot check.
#![allow(unsafe_code)]

use std::path::PathBuf;

use objc2::rc::autoreleasepool;
use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CFURL};
use objc2_core_text::{
    CTFontCollection, CTFontDescriptor, CTFontSymbolicTraits, kCTFontFamilyNameAttribute,
    kCTFontNameAttribute, kCTFontSymbolicTrait, kCTFontTraitsAttribute, kCTFontURLAttribute,
    kCTFontWeightTrait, kCTFontWidthTrait,
};

/// One face of an installed family.
#[derive(Clone, Debug, PartialEq)]
pub struct FontFace {
    /// The face's PostScript name, which tells it from the others in a
    /// collection file (`Menlo-Bold` in `Menlo.ttc`).
    pub postscript_name: String,
    /// The file the face is in.
    pub path: PathBuf,
    /// CoreText's weight, from -1 to 1: 0 is regular, 0.23 medium, 0.4
    /// bold.
    pub weight: f64,
    /// CoreText's width, from -1 to 1: 0 is normal, below condensed.
    pub width: f64,
    pub italic: bool,
}

/// The names of the installed families CoreText marks monospace, each
/// once, in no order. The system's hidden ones (`.SF NS Mono`) are among
/// them: the caller leaves out what it does not list.
pub fn monospace_families() -> Vec<String> {
    autoreleasepool(|_| monospace_families_in_pool())
}

fn monospace_families_in_pool() -> Vec<String> {
    let mut families = Vec::new();
    for descriptor in descriptors(None) {
        if !symbolic(&descriptor).contains(CTFontSymbolicTraits::MonoSpaceTrait) {
            continue;
        }
        // SAFETY: the key is CoreText's own constant, read after the
        // framework is loaded (it is linked).
        let family = string(&descriptor, unsafe { kCTFontFamilyNameAttribute });
        if let Some(family) = family
            && !families.contains(&family)
        {
            families.push(family);
        }
    }
    families
}

/// The faces of the installed family named `family`, as CoreText writes
/// it (its case too): none where no family has the name. A face whose
/// file CoreText does not give is left out.
pub fn faces_of(family: &str) -> Vec<FontFace> {
    autoreleasepool(|_| faces_of_in_pool(family))
}

fn faces_of_in_pool(family: &str) -> Vec<FontFace> {
    descriptors(Some(family))
        .iter()
        .filter_map(|descriptor| {
            // SAFETY: as in `monospace_families`.
            let (name, url) = unsafe { (kCTFontNameAttribute, kCTFontURLAttribute) };
            let postscript_name = string(descriptor, name)?;
            let path = attribute(descriptor, url)?
                .downcast::<CFURL>()
                .ok()?
                .to_file_path()?;
            let traits = traits(descriptor);
            // SAFETY: as in `monospace_families`.
            let (weight, width) = unsafe { (kCTFontWeightTrait, kCTFontWidthTrait) };
            Some(FontFace {
                postscript_name,
                path,
                weight: number(traits.as_deref(), weight).and_then(|n| n.as_f64())?,
                width: number(traits.as_deref(), width)
                    .and_then(|n| n.as_f64())
                    .unwrap_or(0.0),
                italic: symbolic(descriptor).contains(CTFontSymbolicTraits::ItalicTrait),
            })
        })
        .collect()
}

/// The descriptors of every installed face, or of `family`'s alone.
fn descriptors(family: Option<&str>) -> Vec<CFRetained<CTFontDescriptor>> {
    // SAFETY: no options: the generics of a dictionary passed are
    // CoreText's to check, and none is passed.
    let collection = unsafe { CTFontCollection::from_available_fonts(None) };
    let array = match family {
        // SAFETY: as above, no options.
        Some(family) => unsafe {
            collection.matching_font_descriptors_for_family(&CFString::from_str(family), None)
        },
        // SAFETY: a collection of every font, queried as it was made.
        None => unsafe { collection.matching_font_descriptors() },
    };
    let Some(array) = array else {
        return Vec::new();
    };
    // SAFETY: CoreText documents the array as one of font descriptors.
    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CTFontDescriptor>>(array) };
    array.to_vec()
}

/// The value `descriptor` has for `key`, of whatever type.
fn attribute(descriptor: &CTFontDescriptor, key: &CFString) -> Option<CFRetained<CFType>> {
    // SAFETY: `key` is one of CoreText's attribute names, and the value
    // comes back as an untyped object, checked before it is used.
    unsafe { descriptor.attribute(key) }
}

/// The string `descriptor` has for `key`, where it has a string.
fn string(descriptor: &CTFontDescriptor, key: &CFString) -> Option<String> {
    let value = attribute(descriptor, key)?.downcast::<CFString>().ok()?;
    Some(value.to_string())
}

/// The traits dictionary of `descriptor`, its values left untyped.
fn traits(descriptor: &CTFontDescriptor) -> Option<CFRetained<CFDictionary<CFString, CFType>>> {
    // SAFETY: as in `monospace_families`.
    let key = unsafe { kCTFontTraitsAttribute };
    let traits = attribute(descriptor, key)?
        .downcast::<CFDictionary>()
        .ok()?;
    // SAFETY: CoreText documents the traits' keys as strings
    // (`kCTFontWeightTrait` ...); the values are typed as any object and
    // checked where they are read.
    Some(unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(traits) })
}

/// The number the traits dictionary has for `key`.
fn number(
    traits: Option<&CFDictionary<CFString, CFType>>,
    key: &CFString,
) -> Option<CFRetained<CFNumber>> {
    traits?.get(key)?.downcast::<CFNumber>().ok()
}

/// The symbolic traits of `descriptor`: none where it gives none.
fn symbolic(descriptor: &CTFontDescriptor) -> CTFontSymbolicTraits {
    // SAFETY: as in `monospace_families`.
    let key = unsafe { kCTFontSymbolicTrait };
    let bits = number(traits(descriptor).as_deref(), key).and_then(|n| n.as_i64());
    // The class of the face is in the top bits, which a signed number
    // keeps: only the low 32 are traits.
    let bits = bits.map_or(0, |bits| (bits & 0xFFFF_FFFF) as u32);
    CTFontSymbolicTraits(bits)
}
