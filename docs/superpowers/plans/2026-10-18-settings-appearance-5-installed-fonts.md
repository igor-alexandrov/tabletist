# Settings › Appearance, step 5: Installed Data Fonts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** On macOS, Settings › Appearance › Data font offers the installed monospace fonts after the two bundled ones (IBM Plex Mono, JetBrains Mono), and draws grid cells, the SQL editor and JSON in the one picked.

**Architecture:** `crates/tabletist-appkit`, the only crate where `unsafe` may go, lists the installed families CoreText marks monospace and a family's faces (file, PostScript name, weight, width, italic), behind a safe API with SAFETY notes. The app (`src/typography/installed_font.rs`, compiled and tested on every platform) picks a family's upright regular and a medium, reads their files with skrifa (a collection's face by its PostScript name, a variable font at its weight) and gives back the `desktop_font::Read` the Omarchy font already uses. So the named font is read the way the Omarchy font is: at the start, before the first frame, and on the backend after a change (`Command::ReadFont`). Its faces fill two new egui families, `installed-mono-400` and `-500`, which hold Plex Mono while none is read, and `DataFace::Installed` names them. The menu's list is read on the backend each time Settings opens (`Command::ListFonts`).

**Tech Stack:** Rust, egui 0.36 (crmne fork), objc2-core-text 0.3.2 and objc2-core-foundation 0.3.2 (new: CoreText's crate), skrifa 0.44 (epaint's own, now a dependency on every platform).

**Spec:** `docs/superpowers/specs/2026-10-17-settings-appearance-design.md`, "macOS data font menu" and "Step 5: Installed data fonts on macOS". Step 3's plan (`2026-10-18-settings-appearance-3-text.md`) built the data font menu this extends.

---

## Decisions and departures

| What | Decision |
|---|---|
| Where the list comes from | CoreText: every available font's descriptor, kept where its symbolic traits have `kCTFontMonoSpaceTrait`, by family name, each once. The menu leaves out the bundled faces' names (listed first), `monospace`, and the system's hidden families (`.SF NS Mono`), and sorts the rest whatever the case. |
| When the list is read | On the backend each time Settings opens (`Action::ShowSettings`, and `Action::ShowAgentSettings` from the Agents tab, Appearance being a click away), on macOS outside tests and the demo. A font installed while the app runs is listed the next time Settings opens. |
| Which faces are drawn | The upright regular of normal width (nearest CoreText weight 0), and at 500 a medium or semibold heavier than it (CoreText weight up to 0.3), else the regular again: Menlo's bold is not drawn at 500. The data is never drawn at 700; the third face of `TerminalFaces` is the 500's. |
| A variable font | CoreText lists its named instances with one file. The face is read at the CSS weight of the instance (`wght` axis), so the regular and the medium differ. |
| A name in the file in another case | CoreText finds the family whatever the case (seen on CI's macOS job), so the read asks for the one family and nothing more: listing every installed font to compare names, as the first draft did, cost a full listing on the UI thread at each start for a name not installed (review of #173). |
| A name that is not a monospace family | Drawn all the same where CoreText has the family: the file names it on purpose. The menu lists monospace ones only. |
| While a read is on its way | The face drawn stays, a bundled one or an installed one read before (`App::text_look`): no flash of Plex Mono on the way to an installed font, and nothing said. |
| A name nothing draws | IBM Plex Mono, and the row's small print says "“name” is not installed: IBM Plex Mono is used" (step 3 said "is not bundled"). In tests and the demo nothing is read, so every installed name is said not installed. |
| What is only compiled | The CoreText calls and their callers in `installed_font.rs` are type-checked for `aarch64-apple-darwin` here (the appkit crate directly with `--target`, the app module in a scratch crate), never run. Three macOS-only tests (Menlo, which every macOS has: two in the appkit crate, one in `installed_font.rs`) call them for real on CI's macOS job; none has run here. |

## Files

- Create `crates/tabletist-appkit/src/fonts.rs`: `monospace_families()`, `faces_of(family)`, `FontFace`. All the `unsafe` of this step, each use with a SAFETY note.
- Create `crates/tabletist-appkit/tests/fonts.rs`: Menlo through CoreText, on macOS only.
- Modify `crates/tabletist-appkit/{Cargo.toml,src/lib.rs}`, `Cargo.toml`, `Cargo.lock`: the two CoreText crates; skrifa for every platform.
- Create `src/typography/installed_font.rs`: the listing, the faces' pick, the files' read, `read()`.
- Modify `src/typography.rs`, `src/typography/{fonts,desktop_font}.rs`, `src/theme.rs`: `DataFace::Installed`, its families, `read_font()`, `Read::has()`, `with_text(settings, installed)`, `set_read_faces()`.
- Modify `src/app.rs`, `src/backend.rs`, `src/testing.rs`, `src/ui/grid.rs`: the reads, the list, the look after a read, the grid's marks forgotten.
- Modify `src/ui/settings/{sheet,mod}.rs`: the menu and the small print.
- Tests: `src/ui/text_settings_tests.rs`, `src/app.rs`'s tests, and the modules' own.
- Docs: `docs/_guide/macos.md`, `docs/_reference/settings-and-files.md`.

Every task ends with the checks of `AGENTS.md` (`$C` is `~/.cargo/bin/cargo`, the mise shim fails here):

```bash
$C fmt --all --check
$C clippy --locked --workspace --all-targets -- -D warnings
$C test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' $C doc --locked --workspace --no-deps
CARGO_TARGET_DIR=target/xcheck $C clippy --locked -p tabletist-appkit --all-targets --target aarch64-apple-darwin -- -D warnings
```

The last one works for the appkit crate alone (its dependencies are pure Rust); the workspace does not cross-compile here (ring's C build). From task 2 on, `installed_font.rs` is type-checked for macOS and Windows in a scratch crate outside the repository that includes it with `#[path]` and stubs `crate::theme::DataFace`, `super::desktop_font::Read` and (for its tests) `super::fonts::PLEX_MONO`, with egui (the fork's revision, patched), skrifa, log, and the appkit crate on macOS:

```bash
(cd <scratch>/xcheck && CARGO_TARGET_DIR=<worktree>/target/xcheck $C clippy --offline --all-targets --target aarch64-apple-darwin -- -D warnings)
(cd <scratch>/xcheck && CARGO_TARGET_DIR=<worktree>/target/xcheck $C clippy --offline --all-targets --target x86_64-pc-windows-msvc -- -D warnings)
```

Prove each such check real once with a deliberate type error in the macOS-only code, then take it out.

---

### Task 1: List the installed fonts through CoreText (`tabletist-appkit`)

**Files:**
- Create: `crates/tabletist-appkit/src/fonts.rs`, `crates/tabletist-appkit/tests/fonts.rs`
- Modify: `crates/tabletist-appkit/Cargo.toml`, `crates/tabletist-appkit/src/lib.rs`, `Cargo.toml` (the comment on the crate), `Cargo.lock`

- [ ] **Step 1: Add the dependencies.** Let cargo resolve once to learn the lock entry, then put `Cargo.lock` back and add only the new package and the crate's two names by hand: a re-resolve also moves `windows-sys` for six unrelated crates. Confirm with `$C metadata --locked --format-version 1 > /dev/null`.

```diff
diff --git a/Cargo.lock b/Cargo.lock
index ed2b8ce..445aca7 100644
--- a/Cargo.lock
+++ b/Cargo.lock
@@ -3661,6 +3661,16 @@ dependencies = [
  "objc2-foundation 0.2.2",
 ]
 
+[[package]]
+name = "objc2-core-text"
+version = "0.3.2"
+source = "registry+https://github.com/rust-lang/crates.io-index"
+checksum = "0cde0dfb48d25d2b4862161a4d5fcc0e3c24367869ad306b0c9ec0073bfed92d"
+dependencies = [
+ "bitflags 2.13.2",
+ "objc2-core-foundation",
+]
+
 [[package]]
 name = "objc2-encode"
 version = "4.1.0"
@@ -5723,6 +5733,8 @@ version = "0.1.0"
 dependencies = [
  "objc2 0.6.4",
  "objc2-app-kit 0.3.2",
+ "objc2-core-foundation",
+ "objc2-core-text",
  "objc2-foundation 0.3.2",
 ]
 
diff --git a/Cargo.toml b/Cargo.toml
index 09ca58c..cbd0e00 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -144,8 +144,9 @@ security-framework = "3.7"
 objc2 = "0.6"
 objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSArray", "NSGeometry"] }
 objc2-app-kit = { version = "0.3", default-features = false, features = ["std", "NSApplication", "NSResponder", "NSView", "NSWindow", "NSToolbar", "NSControl", "NSButton", "objc2-core-foundation"] }
-# The app menu's About item: the one AppKit call that needs unsafe code, kept
-# in a crate of its own because this one forbids it.
+# The app menu's About and Settings items, and the installed fonts CoreText
+# lists for the data font: the AppKit and CoreText calls that need unsafe
+# code, kept in a crate of their own because this one forbids it.
 tabletist-appkit = { path = "crates/tabletist-appkit" }
 # Quit (Cmd+Q, the app menu, the Dock) reaches the app as a close request for
 # its window, which a tab with pending changes holds back to ask first
diff --git a/crates/tabletist-appkit/Cargo.toml b/crates/tabletist-appkit/Cargo.toml
index 6246830..e8e5bb1 100644
--- a/crates/tabletist-appkit/Cargo.toml
+++ b/crates/tabletist-appkit/Cargo.toml
@@ -1,17 +1,18 @@
 [package]
 name = "tabletist-appkit"
 version = "0.1.0"
-description = "The AppKit calls Tabletist cannot make without unsafe code"
+description = "The AppKit and CoreText calls Tabletist cannot make without unsafe code"
 edition.workspace = true
 rust-version.workspace = true
 publish = false
 
 # The workspace forbids unsafe code. Giving a menu item a target and an action
 # means defining an Objective-C class and sending messages the compiler cannot
-# check, so this crate carries the workspace's lints with `unsafe_code` lowered
-# to `deny`, allowed only in `about.rs`, `settings.rs`, `target.rs` and the
-# tests, each use with a SAFETY note. The app and the database layer keep
-# `forbid`.
+# check, and listing the installed fonts means reading CoreText's C statics and
+# untyped collections, so this crate carries the workspace's lints with
+# `unsafe_code` lowered to `deny`, allowed only in `about.rs`, `fonts.rs`,
+# `settings.rs`, `target.rs` and the tests, each use with a SAFETY note. The
+# app and the database layer keep `forbid`.
 [lints.rust]
 unsafe_code = "deny"
 
@@ -23,6 +24,11 @@ unwrap_used = "warn"
 objc2 = "0.6"
 objc2-foundation = { version = "0.3", default-features = false, features = ["std", "NSArray", "NSString"] }
 objc2-app-kit = { version = "0.3", default-features = false, features = ["std", "NSApplication", "NSResponder", "NSMenu", "NSMenuItem"] }
+# CoreText's list of the installed fonts, for the data font's menu
+# (`fonts.rs`), and the CoreFoundation types it answers in. The versions
+# objc2-app-kit builds with; CoreText's crate is the one new crate.
+objc2-core-foundation = { version = "0.3", default-features = false, features = ["std", "CFArray", "CFDictionary", "CFNumber", "CFSet", "CFString", "CFURL"] }
+objc2-core-text = { version = "0.3", default-features = false, features = ["std", "CTFontCollection", "CTFontDescriptor", "CTFontTraits"] }
 
 # AppKit's menus belong to the main thread, which the test harness keeps for
 # itself: these tests have their own `main`.
```

- [ ] **Step 2: Write the macOS tests.** They run on CI's macOS job only (nothing is compiled from the file elsewhere): Menlo is listed monospace and Helvetica is not, each family once; Menlo's regular is in a file, at weight 0, upright and of normal width; its bold is heavier; its italic is italic; a family nobody has has no faces.

```diff
diff --git a/crates/tabletist-appkit/tests/fonts.rs b/crates/tabletist-appkit/tests/fonts.rs
new file mode 100644
index 0000000..a820903
--- /dev/null
+++ b/crates/tabletist-appkit/tests/fonts.rs
@@ -0,0 +1,43 @@
+//! The installed fonts as CoreText lists them, on the Mac that runs the
+//! tests: Menlo, which every macOS has had since 10.6, stands for the rest.
+//! Nothing elsewhere. CoreText may be asked on any thread: the harness's
+//! own.
+
+#![cfg(target_os = "macos")]
+
+use tabletist_appkit::{faces_of, monospace_families};
+
+#[test]
+fn menlo_is_listed_monospace_and_helvetica_is_not() {
+    let families = monospace_families();
+    assert!(
+        families.iter().any(|family| family == "Menlo"),
+        "{families:?}"
+    );
+    assert!(!families.iter().any(|family| family == "Helvetica"));
+    // Each once.
+    let mut once = families.clone();
+    once.sort();
+    once.dedup();
+    assert_eq!(once.len(), families.len());
+}
+
+#[test]
+fn menlos_faces_are_in_its_file_at_their_weights() {
+    let faces = faces_of("Menlo");
+    let face = |name: &str| {
+        faces
+            .iter()
+            .find(|face| face.postscript_name == name)
+            .unwrap_or_else(|| panic!("{name} in {faces:?}"))
+    };
+    let regular = face("Menlo-Regular");
+    assert!(regular.path.is_file(), "{regular:?}");
+    assert!(regular.weight.abs() < 0.01, "{regular:?}");
+    assert!(regular.width.abs() < 0.01, "{regular:?}");
+    assert!(!regular.italic);
+    let bold = face("Menlo-Bold");
+    assert!(bold.weight > 0.3, "{bold:?}");
+    assert!(face("Menlo-Italic").italic);
+    assert!(faces_of("No Such Family Of Fonts").is_empty());
+}
```

- [ ] **Step 3: Check that they fail for macOS.** `CARGO_TARGET_DIR=target/xcheck $C clippy --locked -p tabletist-appkit --all-targets --target aarch64-apple-darwin -- -D warnings`. Expected: `error[E0432]: unresolved imports tabletist_appkit::faces_of, tabletist_appkit::monospace_families`.

- [ ] **Step 4: Write the module.** `descriptors()` takes every available font's descriptor, or one family's; `attribute()`, `string()`, `traits()`, `number()` and `symbolic()` read them, each value checked by `downcast` before it is used. The two casts (`CFArray` to one of descriptors, the traits' `CFDictionary` to one keyed by strings) are what CoreText documents. Each public call drains an autorelease pool of its own: the backend's blocking threads have none.

```diff
diff --git a/crates/tabletist-appkit/src/fonts.rs b/crates/tabletist-appkit/src/fonts.rs
new file mode 100644
index 0000000..0730b0d
--- /dev/null
+++ b/crates/tabletist-appkit/src/fonts.rs
@@ -0,0 +1,162 @@
+//! The fonts installed on the Mac, as CoreText lists them: the monospace
+//! families, for the data font's menu, and the faces of one family, with
+//! the file each is in, for the app to read.
+//!
+//! CoreText's font collections and descriptors may be used on any thread:
+//! these are called on the backend's, which drain no autorelease pool of
+//! their own, so each call drains one.
+
+// CoreText's attribute keys are C statics, and its arrays and dictionaries
+// hold untyped values: reading either is a step the compiler cannot check.
+#![allow(unsafe_code)]
+
+use std::path::PathBuf;
+
+use objc2::rc::autoreleasepool;
+use objc2_core_foundation::{CFArray, CFDictionary, CFNumber, CFRetained, CFString, CFType, CFURL};
+use objc2_core_text::{
+    CTFontCollection, CTFontDescriptor, CTFontSymbolicTraits, kCTFontFamilyNameAttribute,
+    kCTFontNameAttribute, kCTFontSymbolicTrait, kCTFontTraitsAttribute, kCTFontURLAttribute,
+    kCTFontWeightTrait, kCTFontWidthTrait,
+};
+
+/// One face of an installed family.
+#[derive(Clone, Debug, PartialEq)]
+pub struct FontFace {
+    /// The face's PostScript name, which tells it from the others in a
+    /// collection file (`Menlo-Bold` in `Menlo.ttc`).
+    pub postscript_name: String,
+    /// The file the face is in.
+    pub path: PathBuf,
+    /// CoreText's weight, from -1 to 1: 0 is regular, 0.23 medium, 0.4
+    /// bold.
+    pub weight: f64,
+    /// CoreText's width, from -1 to 1: 0 is normal, below condensed.
+    pub width: f64,
+    pub italic: bool,
+}
+
+/// The names of the installed families CoreText marks monospace, each
+/// once, in no order. The system's hidden ones (`.SF NS Mono`) are among
+/// them: the caller leaves out what it does not list.
+pub fn monospace_families() -> Vec<String> {
+    autoreleasepool(|_| monospace_families_in_pool())
+}
+
+fn monospace_families_in_pool() -> Vec<String> {
+    let mut families = Vec::new();
+    for descriptor in descriptors(None) {
+        if !symbolic(&descriptor).contains(CTFontSymbolicTraits::MonoSpaceTrait) {
+            continue;
+        }
+        // SAFETY: the key is CoreText's own constant, read after the
+        // framework is loaded (it is linked).
+        let family = string(&descriptor, unsafe { kCTFontFamilyNameAttribute });
+        if let Some(family) = family
+            && !families.contains(&family)
+        {
+            families.push(family);
+        }
+    }
+    families
+}
+
+/// The faces of the installed family named `family`, as CoreText writes
+/// it (its case too): none where no family has the name. A face whose
+/// file CoreText does not give is left out.
+pub fn faces_of(family: &str) -> Vec<FontFace> {
+    autoreleasepool(|_| faces_of_in_pool(family))
+}
+
+fn faces_of_in_pool(family: &str) -> Vec<FontFace> {
+    descriptors(Some(family))
+        .iter()
+        .filter_map(|descriptor| {
+            // SAFETY: as in `monospace_families`.
+            let (name, url) = unsafe { (kCTFontNameAttribute, kCTFontURLAttribute) };
+            let postscript_name = string(descriptor, name)?;
+            let path = attribute(descriptor, url)?
+                .downcast::<CFURL>()
+                .ok()?
+                .to_file_path()?;
+            let traits = traits(descriptor);
+            // SAFETY: as in `monospace_families`.
+            let (weight, width) = unsafe { (kCTFontWeightTrait, kCTFontWidthTrait) };
+            Some(FontFace {
+                postscript_name,
+                path,
+                weight: number(traits.as_deref(), weight).and_then(|n| n.as_f64())?,
+                width: number(traits.as_deref(), width)
+                    .and_then(|n| n.as_f64())
+                    .unwrap_or(0.0),
+                italic: symbolic(descriptor).contains(CTFontSymbolicTraits::ItalicTrait),
+            })
+        })
+        .collect()
+}
+
+/// The descriptors of every installed face, or of `family`'s alone.
+fn descriptors(family: Option<&str>) -> Vec<CFRetained<CTFontDescriptor>> {
+    // SAFETY: no options: the generics of a dictionary passed are
+    // CoreText's to check, and none is passed.
+    let collection = unsafe { CTFontCollection::from_available_fonts(None) };
+    let array = match family {
+        // SAFETY: as above, no options.
+        Some(family) => unsafe {
+            collection.matching_font_descriptors_for_family(&CFString::from_str(family), None)
+        },
+        // SAFETY: a collection of every font, queried as it was made.
+        None => unsafe { collection.matching_font_descriptors() },
+    };
+    let Some(array) = array else {
+        return Vec::new();
+    };
+    // SAFETY: CoreText documents the array as one of font descriptors.
+    let array = unsafe { CFRetained::cast_unchecked::<CFArray<CTFontDescriptor>>(array) };
+    array.to_vec()
+}
+
+/// The value `descriptor` has for `key`, of whatever type.
+fn attribute(descriptor: &CTFontDescriptor, key: &CFString) -> Option<CFRetained<CFType>> {
+    // SAFETY: `key` is one of CoreText's attribute names, and the value
+    // comes back as an untyped object, checked before it is used.
+    unsafe { descriptor.attribute(key) }
+}
+
+/// The string `descriptor` has for `key`, where it has a string.
+fn string(descriptor: &CTFontDescriptor, key: &CFString) -> Option<String> {
+    let value = attribute(descriptor, key)?.downcast::<CFString>().ok()?;
+    Some(value.to_string())
+}
+
+/// The traits dictionary of `descriptor`, its values left untyped.
+fn traits(descriptor: &CTFontDescriptor) -> Option<CFRetained<CFDictionary<CFString, CFType>>> {
+    // SAFETY: as in `monospace_families`.
+    let key = unsafe { kCTFontTraitsAttribute };
+    let traits = attribute(descriptor, key)?
+        .downcast::<CFDictionary>()
+        .ok()?;
+    // SAFETY: CoreText documents the traits' keys as strings
+    // (`kCTFontWeightTrait` ...); the values are typed as any object and
+    // checked where they are read.
+    Some(unsafe { CFRetained::cast_unchecked::<CFDictionary<CFString, CFType>>(traits) })
+}
+
+/// The number the traits dictionary has for `key`.
+fn number(
+    traits: Option<&CFDictionary<CFString, CFType>>,
+    key: &CFString,
+) -> Option<CFRetained<CFNumber>> {
+    traits?.get(key)?.downcast::<CFNumber>().ok()
+}
+
+/// The symbolic traits of `descriptor`: none where it gives none.
+fn symbolic(descriptor: &CTFontDescriptor) -> CTFontSymbolicTraits {
+    // SAFETY: as in `monospace_families`.
+    let key = unsafe { kCTFontSymbolicTrait };
+    let bits = number(traits(descriptor).as_deref(), key).and_then(|n| n.as_i64());
+    // The class of the face is in the top bits, which a signed number
+    // keeps: only the low 32 are traits.
+    let bits = bits.map_or(0, |bits| (bits & 0xFFFF_FFFF) as u32);
+    CTFontSymbolicTraits(bits)
+}
diff --git a/crates/tabletist-appkit/src/lib.rs b/crates/tabletist-appkit/src/lib.rs
index 7d23a85..20f44f4 100644
--- a/crates/tabletist-appkit/src/lib.rs
+++ b/crates/tabletist-appkit/src/lib.rs
@@ -1,4 +1,5 @@
-//! The AppKit calls Tabletist cannot make without `unsafe` code.
+//! The AppKit and CoreText calls Tabletist cannot make without `unsafe`
+//! code.
 //!
 //! The rest of the workspace forbids `unsafe`, so the few calls that need it
 //! live here behind an API that does not. Empty off macOS.
@@ -6,6 +7,9 @@
 #[cfg(target_os = "macos")]
 mod about;
 
+#[cfg(target_os = "macos")]
+mod fonts;
+
 #[cfg(target_os = "macos")]
 mod settings;
 
@@ -15,5 +19,8 @@ mod target;
 #[cfg(target_os = "macos")]
 pub use about::AboutItem;
 
+#[cfg(target_os = "macos")]
+pub use fonts::{FontFace, faces_of, monospace_families};
+
 #[cfg(target_os = "macos")]
 pub use settings::SettingsItem;
```

- [ ] **Step 5: Check.** The macOS clippy of step 3 passes; prove it real with a deliberate `let _: u8 = "x";` in `fonts.rs`, then take it out. The Linux checks pass (nothing changed there).

- [ ] **Step 6: Commit.** `git add Cargo.lock Cargo.toml crates/tabletist-appkit && git commit -m "List the installed monospace fonts through CoreText in tabletist-appkit"`

### Task 2: Draw data in an installed face (`typography`, `theme`)

**Files:**
- Create: `src/typography/installed_font.rs`
- Modify: `Cargo.toml` (skrifa), `src/typography.rs`, `src/typography/fonts.rs`, `src/typography/desktop_font.rs`, `src/theme.rs`
- Callers of the changed signatures: `src/app.rs`, `src/testing.rs`, `src/ui/text_settings_tests.rs`, `src/ui/settings/sheet.rs`

- [ ] **Step 1: Move skrifa to the dependencies of every platform.** epaint builds it everywhere already; the lock does not change.

```diff
diff --git a/Cargo.toml b/Cargo.toml
index cbd0e00..895e5c7 100644
--- a/Cargo.toml
+++ b/Cargo.toml
@@ -50,6 +50,11 @@ directories = "6"
 # to the headless tests.
 eframe = { version = "0.36", default-features = false, features = ["accesskit", "default_fonts", "glow", "persistence", "wayland", "x11"] }
 egui = "0.36"
+# Checks that a font read from the system parses before egui loads it (egui
+# panics on a font it cannot read): the desktop's monospace on Linux, an
+# installed data font on macOS, whose face in a collection it finds by name.
+# The version epaint already uses on every platform.
+skrifa = { version = "0.44", default-features = false, features = ["std"] }
 # SVG loading for the Lucide icons.
 egui_extras = { version = "0.36", features = ["svg"] }
 # Writes --demo-shot screenshots as PNG.
@@ -129,9 +134,6 @@ zbus-secret-service-keyring-store = { version = "1.0.1", features = ["rt-tokio-c
 # before it is read, which the store's read would answer with a prompt to
 # unlock it (`secrets::linux_locked`), for a read an agent's call made.
 secret-service = { version = "5", features = ["rt-tokio-crypto-rust"] }
-# Checks that the desktop's monospace font parses before egui loads it (egui
-# panics on a font it cannot read). The version epaint already uses.
-skrifa = { version = "0.44", default-features = false, features = ["std"] }
 
 [target.'cfg(target_os = "macos")'.dependencies]
 apple-native-keyring-store = { version = "1.0.2", features = ["keychain"] }
```

- [ ] **Step 2: Write the code with its tests.** The tests: in `installed_font.rs` (whole below, its tests at its end): the menu's list (each family once whatever the case, sorted, no bundled face, no hidden family, no `monospace`); the regular and the medium of a family of the test's own (a medium, Menlo's regular twice, a semibold, a family of one face, none); an upright face of normal width over a nearer one; CoreText's weights as CSS's; a variable face read at its weight from the bundled JetBrains Mono, a fixed one as it is, bytes that are no font as none; `read_with()` over a list of the test's own whose files are the bundled Plex Mono (found, in another case, not installed, its file gone); nothing looked up for a bundled face's name, `monospace` or nothing; and on macOS only, Menlo read from the Mac. In `theme.rs`, an installed name draws `Installed` once read and the default before, a bundled name is the bundled face read or not. In `fonts.rs`, the families: Plex Mono in them before a read, the read faces leading them after, Plex Mono again for none.

The module's tests and its code are one new file, and the new `with_text` signature breaks every caller at once, so the blocks below hold tests and code together. To see the tests fail first, add the test modules of `theme.rs`, `fonts.rs` and `installed_font.rs` (with `pub mod installed_font;`) before anything else: expected `error[E0599]: no variant or associated item named Installed found for enum DataFace` and `error[E0061]: this method takes 1 argument but 2 arguments were supplied`.

The code: `DataFace::Installed`, `DataFace::BUNDLED` and `name() -> Option<&str>` in `theme.rs` (above); the module; `read_font()`, which reads through CoreText on macOS and fontconfig elsewhere (`cfg!`, so both are compiled everywhere: they are public and nothing is dead on any platform); `data_family_name(Installed)` and no capitals shift for an installed face; `Read::has()`; the outline check shared with the Linux loader; the families in `plex()`, whose signature takes the faces read. Callers pass `false` for now.

```diff
diff --git a/src/theme.rs b/src/theme.rs
index 1723b05..7b7806f 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -339,38 +339,47 @@ pub enum Typeface {
     Terminal,
 }
 
-/// The monospace face macOS draws data in: one of the two it bundles.
+/// The monospace face macOS draws data in: one of the two it bundles, or
+/// the installed font the settings name, once it is read
+/// ([`crate::typography::installed_font`]).
 #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
 pub enum DataFace {
     #[default]
     PlexMono,
     JetBrainsMono,
+    Installed,
 }
 
 impl DataFace {
-    /// In the order the menu lists them.
-    pub const ALL: [DataFace; 2] = [Self::PlexMono, Self::JetBrainsMono];
+    /// The bundled faces, in the order the menu lists them, first.
+    pub const BUNDLED: [DataFace; 2] = [Self::PlexMono, Self::JetBrainsMono];
 
-    /// The face's name, as the menu and the file write it.
-    pub fn name(self) -> &'static str {
+    /// A bundled face's name, as the menu and the file write it. An
+    /// installed one has the name the settings give it.
+    pub fn name(self) -> Option<&'static str> {
         match self {
-            Self::PlexMono => "IBM Plex Mono",
-            Self::JetBrainsMono => "JetBrains Mono",
+            Self::PlexMono => Some("IBM Plex Mono"),
+            Self::JetBrainsMono => Some("JetBrains Mono"),
+            Self::Installed => None,
         }
     }
 
-    /// The face `font` names, whatever its case: the default for none, and
-    /// for `monospace`, which names the desktop's own (the terminal
-    /// look's default). A name of no bundled face is given back: it falls
-    /// back to the default, which says so.
+    /// The bundled face `font` names, whatever its case: the default for
+    /// none, and for `monospace`, which names the desktop's own (the
+    /// terminal look's default). A name of no bundled face is given back:
+    /// an installed font, drawn once it is read, else the default, which
+    /// says so.
     pub fn of(font: Option<&str>) -> Result<Self, &str> {
         let font = font.map(str::trim);
         let Some(font) = font.filter(|font| !font.eq_ignore_ascii_case("monospace")) else {
             return Ok(Self::default());
         };
-        Self::ALL
+        Self::BUNDLED
             .into_iter()
-            .find(|face| face.name().eq_ignore_ascii_case(font))
+            .find(|face| {
+                face.name()
+                    .is_some_and(|name| name.eq_ignore_ascii_case(font))
+            })
             .ok_or(font)
     }
 }
@@ -402,15 +411,21 @@ impl Faces {
     }
 
     /// The faces at the sizes `settings` give, and the data in the face
-    /// they name. The terminal look's stay as they are.
-    pub fn with_text(self, settings: &Settings) -> Self {
+    /// they name: an installed one where `installed` (it is read), else
+    /// the default. The terminal look's stay as they are.
+    pub fn with_text(self, settings: &Settings, installed: bool) -> Self {
         if self.typeface == Typeface::Terminal {
             return Self::of(self.typeface);
         }
+        let data = match DataFace::of(settings.font.as_deref()) {
+            Ok(face) => face,
+            Err(_) if installed => DataFace::Installed,
+            Err(_) => DataFace::default(),
+        };
         Self {
             typeface: self.typeface,
             interface: settings.interface_size,
-            data: DataFace::of(settings.font.as_deref()).unwrap_or_default(),
+            data,
             data_size: settings.font_size_in(false),
         }
     }
@@ -562,10 +577,11 @@ impl Look {
     }
 
     /// The look with its text as `settings` have it: the interface's size,
-    /// and the data's face and size.
-    pub fn with_text(self, settings: &Settings) -> Self {
+    /// and the data's face and size, the installed face where `installed`
+    /// ([`Faces::with_text`]).
+    pub fn with_text(self, settings: &Settings, installed: bool) -> Self {
         Self {
-            faces: self.faces.with_text(settings),
+            faces: self.faces.with_text(settings, installed),
             ..self
         }
     }
@@ -990,7 +1006,7 @@ mod tests {
         use crate::typography::TextRole;
         for look in [Look::macos(), Look::standard()] {
             let faces = look
-                .with_text(&text(InterfaceSize::Large, None, 16.0))
+                .with_text(&text(InterfaceSize::Large, None, 16.0), false)
                 .faces;
             // The interface's roles keep to the body's 14 as the design's
             // to 13, the data's to 16 as the design's to 12.5.
@@ -1009,7 +1025,7 @@ mod tests {
         for faces in [
             Look::macos().faces,
             Look::omarchy()
-                .with_text(&text(InterfaceSize::Large, Some("Fira Code"), 20.0))
+                .with_text(&text(InterfaceSize::Large, Some("Fira Code"), 20.0), true)
                 .faces,
         ] {
             for role in TextRole::ALL {
@@ -1035,11 +1051,27 @@ mod tests {
             Ok(DataFace::JetBrainsMono)
         );
         assert_eq!(DataFace::of(Some("Fira Code")), Err("Fira Code"));
-        let look = Look::macos().with_text(&text(InterfaceSize::Default, Some("Fira Code"), 12.5));
-        assert_eq!(look.faces.data, DataFace::PlexMono);
-        let look =
-            Look::macos().with_text(&text(InterfaceSize::Default, Some("JetBrains Mono"), 12.5));
-        assert_eq!(look.faces.data, DataFace::JetBrainsMono);
+        // An installed font is drawn once it is read, the default before.
+        let fira = text(InterfaceSize::Default, Some("Fira Code"), 12.5);
+        assert_eq!(
+            Look::macos().with_text(&fira, false).faces.data,
+            DataFace::PlexMono
+        );
+        assert_eq!(
+            Look::macos().with_text(&fira, true).faces.data,
+            DataFace::Installed
+        );
+        // A bundled face's name is the bundled face, read or not.
+        let jetbrains = text(InterfaceSize::Default, Some("JetBrains Mono"), 12.5);
+        for installed in [false, true] {
+            let look = Look::macos().with_text(&jetbrains, installed);
+            assert_eq!(look.faces.data, DataFace::JetBrainsMono);
+        }
+        let plex = text(InterfaceSize::Default, None, 12.5);
+        assert_eq!(
+            Look::macos().with_text(&plex, true).faces.data,
+            DataFace::PlexMono
+        );
     }
 
     fn nord() -> CustomTheme {
```

```diff
diff --git a/src/typography.rs b/src/typography.rs
index e166193..6f46a4e 100644
--- a/src/typography.rs
+++ b/src/typography.rs
@@ -18,9 +18,21 @@
 
 pub mod desktop_font;
 mod fonts;
+pub mod installed_font;
 
 pub use fonts::{TerminalFaces, configure};
 
+/// The font the settings name, read from the system: on macOS the data's,
+/// an installed one through CoreText; elsewhere the terminal look's,
+/// through fontconfig.
+pub fn read_font(asked: Option<&str>) -> desktop_font::Read {
+    if cfg!(target_os = "macos") {
+        installed_font::read(asked)
+    } else {
+        desktop_font::read(asked)
+    }
+}
+
 use std::sync::Arc;
 
 use egui::text::LayoutJob;
@@ -417,10 +429,12 @@ impl TextRole {
     }
 
     /// The height of the role's capitals in points, in faces whose line box
-    /// does not centre on them (Plex, and not JetBrains Mono).
+    /// does not centre on them (Plex, and not JetBrains Mono, nor an
+    /// installed face, whose are not known).
     fn capitals(self, faces: Faces) -> Option<f32> {
-        if self.data_face(faces) == Some(DataFace::JetBrainsMono) {
-            return None;
+        match self.data_face(faces) {
+            Some(DataFace::JetBrainsMono | DataFace::Installed) => return None,
+            Some(DataFace::PlexMono) | None => {}
         }
         fonts::capitals(faces.typeface).map(|ems| ems * self.size(faces))
     }
@@ -466,11 +480,13 @@ pub fn family_name(typeface: Typeface, kind: Kind, weight: u16) -> String {
 }
 
 /// The name of the family holding the data face `face` at `weight`: each
-/// is bundled at 400 and 500, as Plex Mono is.
+/// is at 400 and 500, as Plex Mono is. An installed face's hold the one
+/// the settings name, read.
 pub fn data_family_name(face: DataFace, weight: u16) -> String {
     match face {
         DataFace::PlexMono => family_name(Typeface::Plex, Kind::Mono, weight),
         DataFace::JetBrainsMono => format!("jetbrains-mono-{weight}"),
+        DataFace::Installed => format!("installed-mono-{weight}"),
     }
 }
 
diff --git a/src/typography/desktop_font.rs b/src/typography/desktop_font.rs
index 25b7674..ec25f6f 100644
--- a/src/typography/desktop_font.rs
+++ b/src/typography/desktop_font.rs
@@ -1,6 +1,7 @@
 //! The desktop's monospace font on Linux, as fontconfig resolves it
 //! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it), or
-//! the font the settings name by its fontconfig name.
+//! the font the settings name by its fontconfig name. macOS reads the data
+//! font it names into the same [`Read`] ([`super::installed_font`]).
 
 use super::fonts::TerminalFaces;
 
@@ -69,22 +70,31 @@ pub fn pattern(name: &str) -> String {
     pattern
 }
 
-/// What fontconfig gave the terminal look for the font the settings name.
+/// What fontconfig gave the terminal look for the font the settings name,
+/// or CoreText macOS's data for it.
 #[derive(Clone, Debug, Default)]
 pub struct Read {
     /// The font asked for, as the settings name it: `None` is the
     /// system's monospace.
     pub asked: Option<String>,
     /// The faces at 400, 500 and 700: `None` where fontconfig gave none
-    /// that can be drawn (the bundled one is drawn then).
+    /// that can be drawn (the bundled one is drawn then), and on macOS
+    /// where nothing was read (a bundled face is drawn).
     pub faces: Option<TerminalFaces>,
-    /// The family drawn, as fontconfig names it.
+    /// The family drawn, as fontconfig or CoreText names it.
     pub family: Option<String>,
     /// Whether the font asked for is not there: the system's monospace is
-    /// drawn in its place.
+    /// drawn in its place, or on macOS the default data face.
     pub missing: bool,
 }
 
+impl Read {
+    /// Whether the faces read are those of `font`, as the settings name it.
+    pub fn has(&self, font: Option<&str>) -> bool {
+        self.faces.is_some() && self.asked.as_deref() == font
+    }
+}
+
 /// The font named `asked` (the system's monospace for `None`), at 400, 500
 /// and 700, or the system's monospace in its place where fontconfig does
 /// not have it. Each `fc-match` is bounded to half a second; a font found
@@ -195,10 +205,7 @@ fn stop(child: &mut std::process::Child) {
 pub fn load(path: &Path, index: u32) -> Option<egui::FontData> {
     let bytes = std::fs::read(path).ok()?;
     let face = skrifa::FontRef::from_index(&bytes, index).ok()?;
-    let outlined = [b"glyf", b"CFF ", b"CFF2"]
-        .into_iter()
-        .any(|tag| face.table_data(skrifa::Tag::new(tag)).is_some());
-    if !outlined {
+    if !super::installed_font::outlined(&face) {
         return None;
     }
     let mut data = egui::FontData::from_owned(bytes);
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
index 3d92bea..de9b5d8 100644
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -2,9 +2,12 @@
 //!
 //! - Standard: Inter (fastframe's set-up), with egui's monospace for data.
 //! - macOS: IBM Plex Sans (400, 500, 600) and IBM Plex Mono (400, 500),
-//!   bundled: `plex-sans-400` ... `plex-mono-500`; and JetBrains Mono
-//!   (400, 500), the other face data can be drawn in: `jetbrains-mono-400`
-//!   and `jetbrains-mono-500`.
+//!   bundled: `plex-sans-400` ... `plex-mono-500`; JetBrains Mono (400,
+//!   500), the other bundled face data can be drawn in: `jetbrains-mono-400`
+//!   and `jetbrains-mono-500`; and the installed font the settings name for
+//!   the data, where one was read ([`super::installed_font`]):
+//!   `installed-mono-400` and `installed-mono-500`, holding Plex Mono
+//!   without one.
 //! - Omarchy: the desktop's monospace font at 400, 500 and 700 (fontconfig
 //!   reads it, [`super::desktop_font`]), or the bundled JetBrains Mono when
 //!   it found none: `omarchy-mono-400` ... `omarchy-mono-700`.
@@ -128,16 +131,19 @@ fn alias(fonts: &mut FontDefinitions, name: &str, from: &FontFamily) {
     fonts.families.insert(FontFamily::Name(name.into()), list);
 }
 
-/// The terminal look's monospace at 400, 500 and 700.
+/// The terminal look's monospace at 400, 500 and 700, or the installed
+/// font macOS draws data in at the same.
 pub type TerminalFaces = [Arc<FontData>; 3];
 
 /// Replaces the faces fastframe set up with the look's. `desktop` is the
-/// desktop's own monospace that fontconfig gave the terminal look (never in
-/// tests or screenshots), which has the bundled one drawn without it.
+/// font read from the system for the settings' name (never in tests or
+/// screenshots): the desktop's own monospace that fontconfig gave the
+/// terminal look, which has the bundled one drawn without it, or the
+/// installed font macOS draws data in.
 pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: Option<&TerminalFaces>) {
     match look.faces.typeface {
         Typeface::Inter => inter(fonts),
-        Typeface::Plex => plex(fonts),
+        Typeface::Plex => plex(fonts, desktop),
         Typeface::Terminal => terminal(fonts, desktop),
     }
 }
@@ -164,7 +170,7 @@ fn inter(fonts: &mut FontDefinitions) {
 }
 
 #[cfg(any(target_os = "macos", test))]
-fn plex(fonts: &mut FontDefinitions) {
+fn plex(fonts: &mut FontDefinitions, installed: Option<&TerminalFaces>) {
     for weight in [400, 500, 600] {
         let name = family_name(Typeface::Plex, Kind::Sans, weight);
         fonts
@@ -189,6 +195,22 @@ fn plex(fonts: &mut FontDefinitions) {
             .insert(name.clone(), weighted(JETBRAINS_MONO, f32::from(weight)));
         own(fonts, &name, &name, &FontFamily::Monospace);
     }
+    // The installed font the settings name, where one was read. Without
+    // one its families hold Plex Mono, so a look that names them draws.
+    for (weight, face) in [(400, 0), (500, 1)] {
+        let name = data_family_name(DataFace::Installed, weight);
+        match installed {
+            Some(faces) => {
+                fonts.font_data.insert(name.clone(), faces[face].clone());
+                own(fonts, &name, &name, &FontFamily::Monospace);
+            }
+            None => {
+                fonts.font_data.remove(&name);
+                let plex = family_name(Typeface::Plex, Kind::Mono, weight);
+                own(fonts, &name, &plex, &FontFamily::Monospace);
+            }
+        }
+    }
     let sans = |weight| family_name(Typeface::Plex, Kind::Sans, weight);
     lead(fonts, FontFamily::Proportional, &sans(400));
     lead(fonts, FontFamily::Name(MEDIUM.into()), &sans(500));
@@ -216,7 +238,7 @@ fn plex(fonts: &mut FontDefinitions) {
 }
 
 #[cfg(not(any(target_os = "macos", test)))]
-fn plex(fonts: &mut FontDefinitions) {
+fn plex(fonts: &mut FontDefinitions, _installed: Option<&TerminalFaces>) {
     inter(fonts);
 }
 
@@ -471,7 +493,7 @@ mod tests {
             font: Some("JetBrains Mono".into()),
             ..Settings::default()
         };
-        let jetbrains = Look::macos().with_text(&settings).faces;
+        let jetbrains = Look::macos().with_text(&settings, false).faces;
         assert_eq!(family(jetbrains, TextRole::GridCell), "jetbrains-mono-400");
         assert_eq!(
             family(jetbrains, TextRole::CompletionMatch),
@@ -486,6 +508,41 @@ mod tests {
         assert_eq!(laid.capitals, None);
     }
 
+    #[test]
+    fn macos_draws_data_in_the_installed_font_read() {
+        use crate::settings::Settings;
+        let settings = Settings {
+            font: Some("Fira Code".into()),
+            ..Settings::default()
+        };
+        let faces = Look::macos().with_text(&settings, true).faces;
+        let family = |weight: u16| FontFamily::Name(format!("installed-mono-{weight}").into());
+        assert_eq!(TextRole::GridCell.font_id(faces).family, family(400));
+        assert_eq!(TextRole::CompletionMatch.font_id(faces).family, family(500));
+        // Its capitals are not known: no shift for them.
+        let laid = Text::in_faces(faces).add(TextRole::GridCell, "H", egui::Color32::WHITE);
+        assert_eq!(laid.capitals, None);
+        // Before one is read, its families hold Plex Mono.
+        let fonts = configured(&Look::macos());
+        assert_eq!(first(&fonts, family(400)), "plex-mono-400");
+        assert_eq!(first(&fonts, family(500)), "plex-mono-500");
+        // Read: they lead with its faces, the fallbacks behind.
+        let read = |bytes: &'static [u8]| Arc::new(FontData::from_static(bytes));
+        let installed = [read(JETBRAINS_MONO), read(PLEX_MONO_MEDIUM), read(KEYS)];
+        let mut fonts = configured(&Look::macos());
+        configure(&mut fonts, &Look::macos(), Some(&installed));
+        for (weight, bytes) in [(400, JETBRAINS_MONO), (500, PLEX_MONO_MEDIUM)] {
+            let name = first(&fonts, family(weight));
+            assert_eq!(name, format!("installed-mono-{weight}"));
+            assert_eq!(fonts.font_data[&name].font.len(), bytes.len());
+            assert!(fonts.families[&family(weight)].len() > 1);
+        }
+        // And back to Plex Mono when none is.
+        configure(&mut fonts, &Look::macos(), None);
+        assert_eq!(first(&fonts, family(400)), "plex-mono-400");
+        assert!(!fonts.font_data.contains_key("installed-mono-400"));
+    }
+
     #[test]
     fn macos_draws_plex_and_omarchy_one_monospace_face() {
         let mac = configured(&Look::macos());
diff --git a/src/typography/installed_font.rs b/src/typography/installed_font.rs
new file mode 100644
index 0000000..8652d06
--- /dev/null
+++ b/src/typography/installed_font.rs
@@ -0,0 +1,485 @@
+//! The installed fonts macOS can draw data in, beside the two it bundles:
+//! the monospace families CoreText lists, for the data font's menu
+//! ([`families`]), and the faces of the one the settings name, read from
+//! their files ([`read`]).
+//!
+//! Only the CoreText calls are macOS's (in `tabletist-appkit`). What is
+//! made of their answers is the same everywhere, and tested with fonts of
+//! the tests' own.
+
+use std::path::PathBuf;
+use std::sync::Arc;
+
+use egui::FontData;
+use egui::epaint::text::VariationCoords;
+use skrifa::raw::FileRef;
+use skrifa::string::StringId;
+use skrifa::{FontRef, MetadataProvider, Tag};
+
+use super::desktop_font::Read;
+use crate::theme::DataFace;
+
+/// One face of an installed family, as CoreText describes it.
+#[derive(Clone, Debug, PartialEq)]
+pub struct Face {
+    /// What tells the face from the others in a collection file.
+    pub postscript_name: String,
+    pub path: PathBuf,
+    /// CoreText's weight, from -1 to 1: 0 is regular, 0.23 medium.
+    pub weight: f64,
+    /// CoreText's width, from -1 to 1: 0 is normal.
+    pub width: f64,
+    pub italic: bool,
+}
+
+/// CoreText's weights of the two the data is drawn at (400 and 500), and
+/// of the heaviest a medium may be (semibold, 600).
+const REGULAR: f64 = 0.0;
+const MEDIUM: f64 = 0.23;
+const SEMIBOLD: f64 = 0.3;
+
+/// CoreText's weights against CSS's, as AppKit's `NSFontWeight` constants
+/// pair them (ultralight to black).
+const WEIGHTS: [(f64, f32); 9] = [
+    (-0.8, 100.0),
+    (-0.6, 200.0),
+    (-0.4, 300.0),
+    (0.0, 400.0),
+    (0.23, 500.0),
+    (0.3, 600.0),
+    (0.4, 700.0),
+    (0.56, 800.0),
+    (0.62, 900.0),
+];
+
+/// CoreText's `weight` as a CSS weight, between the constants' by a
+/// straight line: what a variable font's `wght` axis is set to.
+pub fn css_weight(weight: f64) -> f32 {
+    let (first, last) = (WEIGHTS[0], WEIGHTS[WEIGHTS.len() - 1]);
+    if weight <= first.0 {
+        return first.1;
+    }
+    WEIGHTS
+        .windows(2)
+        .find(|pair| weight <= pair[1].0)
+        .map_or(last.1, |pair| {
+            let ((from, low), (to, high)) = (pair[0], pair[1]);
+            let along = ((weight - from) / (to - from)) as f32;
+            low + along * (high - low)
+        })
+}
+
+/// The installed monospace families the menu lists after the bundled
+/// faces: as [`listed`] has them. None off macOS.
+pub fn families() -> Vec<String> {
+    listed(installed_families())
+}
+
+/// `families` as the menu lists them: each once, whatever the case, sorted
+/// whatever the case, and without the bundled faces' names (they are
+/// listed first), `monospace`, and the system's hidden families, whose
+/// names begin with a full stop (`.SF NS Mono`).
+pub fn listed(families: Vec<String>) -> Vec<String> {
+    let mut listed: Vec<String> = families
+        .into_iter()
+        .filter(|family| {
+            let family = family.trim();
+            !family.is_empty() && !family.starts_with('.') && DataFace::of(Some(family)).is_err()
+        })
+        .collect();
+    listed.sort_by_cached_key(|family| family.to_lowercase());
+    listed.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
+    listed
+}
+
+/// The face of `faces` nearest CoreText's `weight`: upright and of the
+/// normal width, where the family has such a face.
+pub fn nearest(faces: &[Face], weight: f64) -> Option<&Face> {
+    let upright = faces.iter().any(|face| !face.italic);
+    let normal = |face: &Face| face.width.abs() < 0.01;
+    let candidates = faces.iter().filter(|face| !(upright && face.italic));
+    let any_normal = candidates.clone().any(normal);
+    candidates
+        .filter(|face| !any_normal || normal(face))
+        .min_by(|a, b| {
+            let off = |face: &Face| (face.weight - weight).abs();
+            off(a)
+                .total_cmp(&off(b))
+                .then_with(|| a.postscript_name.cmp(&b.postscript_name))
+        })
+}
+
+/// The faces the data is drawn in, at 400 and 500: the regular, and a
+/// medium or semibold heavier than it, or the regular again where the
+/// family has none (Menlo has a regular and a bold).
+pub fn regular_and_medium(faces: &[Face]) -> Option<(&Face, &Face)> {
+    let regular = nearest(faces, REGULAR)?;
+    let medium = nearest(faces, MEDIUM)
+        .filter(|medium| medium.weight > regular.weight && medium.weight <= SEMIBOLD + 1e-6)
+        .unwrap_or(regular);
+    Some((regular, medium))
+}
+
+/// Whether a face has outlines (TrueType `glyf` or CFF), which egui draws:
+/// a bitmap-only one parses, and would draw nothing.
+pub fn outlined(font: &FontRef) -> bool {
+    [b"glyf", b"CFF ", b"CFF2"]
+        .into_iter()
+        .any(|tag| font.table_data(Tag::new(tag)).is_some())
+}
+
+/// The face's index in the font file `bytes`: the face whose PostScript
+/// name is `postscript_name` in a collection, the one face in a single
+/// font (a variable font's named instance has another name than the file's
+/// face).
+fn index_of(bytes: &[u8], postscript_name: &str) -> Option<u32> {
+    match FileRef::new(bytes).ok()? {
+        FileRef::Font(_) => Some(0),
+        FileRef::Collection(collection) => (0..collection.len()).find(|&index| {
+            collection.get(index).is_ok_and(|font| {
+                font.localized_strings(StringId::POSTSCRIPT_NAME)
+                    .english_or_first()
+                    .is_some_and(|name| name.to_string() == postscript_name)
+            })
+        }),
+    }
+}
+
+/// `face`, read from `bytes`, its file, for egui: at its weight where the
+/// font is variable in weight. `None` where the file has no such face, or
+/// one without outlines.
+pub fn load(bytes: Vec<u8>, face: &Face) -> Option<FontData> {
+    let index = index_of(&bytes, &face.postscript_name)?;
+    let font = FontRef::from_index(&bytes, index).ok()?;
+    if !outlined(&font) {
+        return None;
+    }
+    let weight = font
+        .axes()
+        .get_by_tag(Tag::new(b"wght"))
+        .map(|axis| css_weight(face.weight).clamp(axis.min_value(), axis.max_value()));
+    let mut data = FontData::from_owned(bytes);
+    data.index = index;
+    if let Some(weight) = weight {
+        data.tweak.coords = VariationCoords::new([(b"wght", weight)]);
+    }
+    Some(data)
+}
+
+/// `face` read from its file, or `None` with one warning.
+fn load_file(face: &Face) -> Option<FontData> {
+    let data = std::fs::read(&face.path)
+        .ok()
+        .and_then(|bytes| load(bytes, face));
+    if data.is_none() {
+        log::warn!(
+            "could not read the font {} in {}, or it has no outlines",
+            face.postscript_name,
+            face.path.display()
+        );
+    }
+    data
+}
+
+/// The installed font the settings name for the data, read at 400 and 500
+/// (the third face, at 700, which the data is never drawn at, is the
+/// 500's). Nothing is read for none, for `monospace` and for a bundled
+/// face's name. A name that no installed family has is `missing`: the
+/// default is drawn in its place. CoreText's answers, then the font files:
+/// at the start (as the fonts always were) and on the backend after.
+pub fn read(asked: Option<&str>) -> Read {
+    read_with(asked, &installed_faces, &installed_families)
+}
+
+/// [`read`], with the installed fonts as `faces_of` and `families` give
+/// them.
+pub fn read_with(
+    asked: Option<&str>,
+    faces_of: &dyn Fn(&str) -> Vec<Face>,
+    families: &dyn Fn() -> Vec<String>,
+) -> Read {
+    let none = Read {
+        asked: asked.map(str::to_owned),
+        ..Read::default()
+    };
+    let named = asked
+        .map(str::trim)
+        .filter(|name| !name.is_empty() && DataFace::of(Some(name)).is_err());
+    let Some(name) = named else {
+        return none;
+    };
+    let mut family = name.to_owned();
+    let mut faces = faces_of(name);
+    if faces.is_empty() {
+        // A name written by hand, in another case than CoreText's.
+        if let Some(found) = families()
+            .into_iter()
+            .find(|found| found.eq_ignore_ascii_case(name))
+        {
+            faces = faces_of(&found);
+            family = found;
+        }
+    }
+    let loaded = regular_and_medium(&faces).and_then(|(regular, medium)| {
+        let drawn = Arc::new(load_file(regular)?);
+        let heavier = if medium == regular {
+            drawn.clone()
+        } else {
+            load_file(medium).map_or_else(|| drawn.clone(), Arc::new)
+        };
+        Some([drawn, heavier.clone(), heavier])
+    });
+    match loaded {
+        Some(faces) => Read {
+            faces: Some(faces),
+            family: Some(family),
+            ..none
+        },
+        None => {
+            log::warn!("the font {name:?} is not installed: IBM Plex Mono is drawn");
+            Read {
+                missing: true,
+                ..none
+            }
+        }
+    }
+}
+
+#[cfg(target_os = "macos")]
+fn installed_families() -> Vec<String> {
+    tabletist_appkit::monospace_families()
+}
+
+#[cfg(not(target_os = "macos"))]
+fn installed_families() -> Vec<String> {
+    Vec::new()
+}
+
+#[cfg(target_os = "macos")]
+fn installed_faces(family: &str) -> Vec<Face> {
+    tabletist_appkit::faces_of(family)
+        .into_iter()
+        .map(|face| Face {
+            postscript_name: face.postscript_name,
+            path: face.path,
+            weight: face.weight,
+            width: face.width,
+            italic: face.italic,
+        })
+        .collect()
+}
+
+#[cfg(not(target_os = "macos"))]
+fn installed_faces(_family: &str) -> Vec<Face> {
+    Vec::new()
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A face of a family of the test's own.
+    fn face(name: &str, weight: f64) -> Face {
+        Face {
+            postscript_name: name.into(),
+            path: PathBuf::from(format!("/fonts/{name}.otf")),
+            weight,
+            width: 0.0,
+            italic: false,
+        }
+    }
+
+    fn names<'a>(faces: Option<(&'a Face, &'a Face)>) -> Option<(&'a str, &'a str)> {
+        faces.map(|(regular, medium)| {
+            (
+                regular.postscript_name.as_str(),
+                medium.postscript_name.as_str(),
+            )
+        })
+    }
+
+    /// A bundled font's file, as an installed face of `name`.
+    fn bundled(name: &str, file: &str, weight: f64) -> Face {
+        Face {
+            path: PathBuf::from(env!("CARGO_MANIFEST_DIR"))
+                .join("assets/fonts")
+                .join(file),
+            ..face(name, weight)
+        }
+    }
+
+    #[test]
+    fn the_menu_lists_each_family_once_and_not_the_bundled_ones() {
+        let families = [
+            "Menlo",
+            ".SF NS Mono",
+            "fira code",
+            "JetBrains Mono",
+            "Fira Code",
+            "Andale Mono",
+            "IBM Plex Mono",
+            "  ",
+            "Monospace",
+        ];
+        let listed = listed(families.map(str::to_owned).to_vec());
+        assert_eq!(listed, ["Andale Mono", "fira code", "Menlo"]);
+    }
+
+    #[test]
+    fn the_data_is_drawn_in_the_regular_and_a_medium_where_there_is_one() {
+        let family = [
+            face("Mono-Bold", 0.4),
+            face("Mono-Light", -0.4),
+            face("Mono-Regular", 0.0),
+            face("Mono-Medium", 0.23),
+        ];
+        assert_eq!(
+            names(regular_and_medium(&family)),
+            Some(("Mono-Regular", "Mono-Medium"))
+        );
+        // Menlo: a bold is no medium, the regular is drawn at 500 too.
+        let menlo = [face("Menlo-Regular", 0.0), face("Menlo-Bold", 0.4)];
+        assert_eq!(
+            names(regular_and_medium(&menlo)),
+            Some(("Menlo-Regular", "Menlo-Regular"))
+        );
+        // A semibold is one.
+        let semibold = [face("S-Regular", 0.0), face("S-Semibold", 0.3)];
+        assert_eq!(
+            names(regular_and_medium(&semibold)),
+            Some(("S-Regular", "S-Semibold"))
+        );
+        // A family of one face draws it at both.
+        let light = [face("L-Light", -0.4)];
+        assert_eq!(
+            names(regular_and_medium(&light)),
+            Some(("L-Light", "L-Light"))
+        );
+        assert_eq!(names(regular_and_medium(&[])), None);
+    }
+
+    #[test]
+    fn an_upright_face_of_normal_width_is_taken_over_a_nearer_one() {
+        let italic = Face {
+            italic: true,
+            ..face("Mono-Italic", 0.0)
+        };
+        let condensed = Face {
+            width: -0.2,
+            ..face("Mono-Condensed", 0.0)
+        };
+        let family = [italic.clone(), condensed.clone(), face("Mono-Book", 0.1)];
+        assert_eq!(
+            nearest(&family, 0.0).map(|face| face.postscript_name.as_str()),
+            Some("Mono-Book")
+        );
+        // Where the family has no other, it is drawn.
+        assert_eq!(nearest(std::slice::from_ref(&italic), 0.0), Some(&italic));
+        assert_eq!(nearest(&[italic, condensed.clone()], 0.0), Some(&condensed));
+    }
+
+    #[test]
+    fn coretext_weights_are_css_weights() {
+        for (weight, css) in [
+            (-1.0, 100.0),
+            (-0.4, 300.0),
+            (0.0, 400.0),
+            (0.115, 450.0),
+            (0.23, 500.0),
+            (0.4, 700.0),
+            (1.0, 900.0),
+        ] {
+            assert!((css_weight(weight) - css).abs() < 0.01, "{weight}: {css}");
+        }
+    }
+
+    #[test]
+    fn a_variable_face_is_read_at_its_weight() {
+        let medium = bundled("JetBrainsMono-Medium", "JetBrainsMono-Variable.ttf", 0.23);
+        let bytes = std::fs::read(&medium.path).unwrap();
+        let data = load(bytes, &medium).expect("JetBrains Mono reads");
+        assert_eq!(data.index, 0);
+        let expected = VariationCoords::new([(b"wght", 500.0)]);
+        assert_eq!(data.tweak.coords, expected);
+        // A face of fixed weight is read as it is.
+        let plex = bundled("IBMPlexMono", "IBMPlexMono-Regular.ttf", 0.0);
+        let data = load(std::fs::read(&plex.path).unwrap(), &plex).unwrap();
+        assert_eq!(data.tweak.coords, VariationCoords::default());
+        // Bytes that are no font are none.
+        assert!(load(b"not a font".to_vec(), &plex).is_none());
+    }
+
+    #[test]
+    fn the_font_the_settings_name_is_read_from_its_files() {
+        let faces_of = |family: &str| match family {
+            "Fira Code" => vec![
+                bundled("Fira-Regular", "IBMPlexMono-Regular.ttf", 0.0),
+                bundled("Fira-Medium", "IBMPlexMono-Medium.ttf", 0.23),
+            ],
+            "Gone" => vec![face("Gone-Regular", 0.0)],
+            _ => Vec::new(),
+        };
+        let families = || vec!["Fira Code".to_owned(), "Gone".to_owned()];
+        let read = read_with(Some("Fira Code"), &faces_of, &families);
+        assert_eq!(read.asked.as_deref(), Some("Fira Code"));
+        assert_eq!(read.family.as_deref(), Some("Fira Code"));
+        assert!(!read.missing);
+        let faces = read.faces.expect("read");
+        let plex_mono_medium = include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf");
+        assert_eq!(faces[0].font.len(), super::super::fonts::PLEX_MONO.len());
+        assert_eq!(faces[1].font.len(), plex_mono_medium.len());
+        assert!(Arc::ptr_eq(&faces[1], &faces[2]));
+        // Written by hand in another case: CoreText's name is drawn.
+        let read = read_with(Some(" fira CODE "), &faces_of, &families);
+        assert_eq!(read.family.as_deref(), Some("Fira Code"));
+        assert!(read.faces.is_some());
+        // Not installed, or its file not there: the default, said.
+        for name in ["Iosevka", "Gone"] {
+            let read = read_with(Some(name), &faces_of, &families);
+            assert!(read.missing, "{name}");
+            assert!(read.faces.is_none() && read.family.is_none(), "{name}");
+            assert_eq!(read.asked.as_deref(), Some(name));
+        }
+    }
+
+    /// The Mac that runs the tests has Menlo, in a collection: CoreText's
+    /// answers, read. Nothing elsewhere.
+    #[cfg(target_os = "macos")]
+    #[test]
+    fn menlo_is_read_from_the_mac() {
+        assert!(families().iter().any(|family| family == "Menlo"));
+        for asked in ["Menlo", "menlo"] {
+            let read = read(Some(asked));
+            assert!(!read.missing, "{asked}");
+            assert_eq!(read.family.as_deref(), Some("Menlo"));
+            let faces = read.faces.expect("Menlo's faces");
+            // The regular, found in the collection by its name; Menlo has
+            // no medium, so the regular again at 500.
+            let font = FontRef::from_index(&faces[0].font, faces[0].index).unwrap();
+            let name = font
+                .localized_strings(StringId::POSTSCRIPT_NAME)
+                .english_or_first()
+                .map(|name| name.to_string());
+            assert_eq!(name.as_deref(), Some("Menlo-Regular"));
+            assert!(Arc::ptr_eq(&faces[0], &faces[1]));
+        }
+        assert!(read(Some("No Such Mono Of Ours")).missing);
+    }
+
+    #[test]
+    fn nothing_is_read_for_the_bundled_faces() {
+        let faces_of = |family: &str| -> Vec<Face> { panic!("{family} looked up") };
+        let families = || -> Vec<String> { panic!("families listed") };
+        for asked in [
+            None,
+            Some("JetBrains Mono"),
+            Some("ibm plex mono"),
+            Some("monospace"),
+            Some(" "),
+        ] {
+            let read = read_with(asked, &faces_of, &families);
+            assert_eq!(read.asked.as_deref(), asked);
+            assert!(read.faces.is_none() && !read.missing, "{asked:?}");
+        }
+    }
+}
```

```diff
diff --git a/src/app.rs b/src/app.rs
index f064126..f13d51d 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -270,7 +270,7 @@ impl App {
             crate::settings::Zone::System | crate::settings::Zone::Utc => None,
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
-        let look = crate::theme::Look::for_platform().with_text(&settings);
+        let look = crate::theme::Look::for_platform().with_text(&settings, false);
         let mut app = Self {
             dirs,
             settings,
@@ -4201,7 +4201,7 @@ impl App {
         }
         // The text's sizes and the data's face: every role drawn from here
         // on has them, and egui's own styles are set again in `logic`.
-        let look = self.look.with_text(&self.settings);
+        let look = self.look.with_text(&self.settings, false);
         if look != self.look {
             self.look = look;
             self.restyle = true;
diff --git a/src/testing.rs b/src/testing.rs
index 89e5b84..3888c0f 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -1586,7 +1586,7 @@ impl Harness {
 
     /// Draws with `look` from the next frame on.
     pub fn set_look(&mut self, look: crate::theme::Look) {
-        let look = look.with_text(&self.app.settings);
+        let look = look.with_text(&self.app.settings, false);
         self.app.look = look;
         self.app.fonts = crate::theme::install_with(&self.ctx, false, &look, None);
         crate::theme::apply(&self.ctx, &self.app.palette, &look);
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index e03ee32..bb39fac 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -905,16 +905,18 @@ fn font_menu(
 ) {
     let Skin { look, palette, .. } = *skin;
     let drawn = DataFace::of(settings.font.as_deref()).unwrap_or_default();
-    let dropdown = menu::Dropdown::new("data-font", drawn.name(), width);
+    let shown = drawn.name().unwrap_or_default();
+    let dropdown = menu::Dropdown::new("data-font", shown, width);
     let menu = dropdown.show(ui, look, palette, |ui| {
-        for face in DataFace::ALL {
-            let entry_name = format!("{name} {}", face.name());
-            let entry = menu::Item::choice(face.name(), face == drawn)
+        for face in DataFace::BUNDLED {
+            let text = face.name().unwrap_or_default();
+            let entry_name = format!("{name} {text}");
+            let entry = menu::Item::choice(text, face == drawn)
                 .name(&entry_name)
                 .show(ui, look, palette);
             if entry.clicked() {
                 // The default is no value: the file leaves it out.
-                let font = (face != DataFace::default()).then(|| face.name().to_owned());
+                let font = (face != DataFace::default()).then(|| text.to_owned());
                 actions.push(Action::SetOption(OptionValue::Font(font)));
                 ui.close();
             }
@@ -922,7 +924,7 @@ fn font_menu(
     });
     menu.response.widget_info(|| {
         let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
-        info.current_text_value = Some(drawn.name().to_owned());
+        info.current_text_value = Some(shown.to_owned());
         info
     });
 }
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index 25b0ccf..f40463c 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -92,7 +92,7 @@ fn a_grid_row_holds_a_line_of_its_data_at_every_size() {
                     row_height: density,
                     ..Default::default()
                 };
-                let look = look.with_text(&settings).with_grid(&settings);
+                let look = look.with_text(&settings, false).with_grid(&settings);
                 let line = crate::ui::grid::data_role(&look).row_height(&harness.ctx, look.faces);
                 assert!(line <= look.row(), "{} {halves} {density:?}", look.name);
             }
```

- [ ] **Step 3: Run them.** `$C test --locked -p tabletist --lib -- installed_font theme:: fonts::`. Expected: PASS (the Menlo test is compiled on macOS only).

- [ ] **Step 4: Check.** Every check above, and the scratch crate for macOS and Windows (prove it real once).

- [ ] **Step 5: Commit.** `git add -A Cargo.toml src && git commit -m "Draw macOS data in an installed font once it is read"`

### Task 3: Read the font at the start and after a change, and list the families

**Files:**
- Modify: `src/app.rs`, `src/backend.rs`, `src/theme.rs` (`set_terminal_faces` becomes `set_read_faces`), `src/testing.rs`, `src/ui/grid.rs`
- Test: `src/app.rs` (its tests), `src/ui/text_settings_tests.rs`

- [ ] **Step 1: Write the tests.** In `src/app.rs`'s tests (in step 3's block of that file, beside the code): the app asks for a read on macOS for an installed name, never for a bundled face's or none, nor on Windows or in tests (`a_new_font_is_read_where_the_desktops_fonts_are`); Settings asks for the list each time it opens on macOS, from the Agents tab too, and the answer is the menu's list. In `text_settings_tests.rs` (below): a read drawn, the face drawn kept while another is on its way, the default for one not installed, and the families after each; a read that ends before a newer one, for a name the settings no longer hold, is dropped (Menlo, Iosevka, Menlo again: Iosevka's missing read must not put Plex Mono under Menlo's families); JetBrains Mono stays drawn while an installed font's read is on its way.

```diff
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index f40463c..d74c399 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -358,3 +358,98 @@ fn the_screen_says_the_font_fontconfig_drew_and_one_it_has_not() {
     assert_eq!(harness.app.settings.font, None);
     assert!(!file(&harness).contains("font ="));
 }
+
+/// What CoreText would give for `name`: Hack's faces (egui's own), or
+/// none where it is not installed.
+fn installed_read(name: &str, found: bool) -> Read {
+    let hack = egui::FontDefinitions::default().font_data["Hack"].clone();
+    Read {
+        asked: Some(name.into()),
+        faces: found.then(|| [hack.clone(), hack.clone(), hack]),
+        family: found.then(|| name.into()),
+        missing: !found,
+    }
+}
+
+/// The backend's answer to the font read numbered `asked`, then the
+/// logic's pass that draws it.
+fn answer(harness: &mut Harness, asked: u64, read: Read) {
+    harness
+        .app
+        .apply(Action::Backend(crate::backend::Event::Font {
+            asked,
+            read: Box::new(read),
+        }));
+    logic(harness);
+}
+
+#[test]
+fn an_installed_font_is_drawn_once_read_and_the_default_where_it_is_not() {
+    let mut harness = sheet(Look::macos());
+    // Where the desktop's fonts are read, as the app does outside tests.
+    harness.app.follow_desktop = true;
+    harness.app.change_settings(|settings| {
+        settings.font = Some("Fira Code".into());
+    });
+    // On its way: the default until it ends.
+    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    answer(&mut harness, 1, installed_read("Fira Code", true));
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+    // Its faces lead the installed families.
+    let fonts = &harness.app.fonts;
+    for weight in [400, 500] {
+        let family = egui::FontFamily::Name(format!("installed-mono-{weight}").into());
+        assert_eq!(
+            fonts.families[&family][0],
+            format!("installed-mono-{weight}")
+        );
+    }
+    // Another on its way: the one drawn stays until it ends.
+    harness.app.change_settings(|settings| {
+        settings.font = Some("Gone Mono".into());
+    });
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+    // Not installed: the default.
+    answer(&mut harness, 2, installed_read("Gone Mono", false));
+    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    let fonts = &harness.app.fonts;
+    let family = egui::FontFamily::Name("installed-mono-400".into());
+    assert_eq!(fonts.families[&family][0], "plex-mono-400");
+}
+
+#[test]
+fn a_read_for_a_name_since_changed_does_not_replace_the_face_drawn() {
+    let mut harness = sheet(Look::macos());
+    harness.app.follow_desktop = true;
+    let font = |harness: &mut Harness, name: &str| {
+        harness.app.change_settings(|settings| {
+            settings.font = Some(name.into());
+        });
+    };
+    let leads = |harness: &Harness| {
+        let family = egui::FontFamily::Name("installed-mono-400".into());
+        harness.app.fonts.families[&family][0].clone()
+    };
+    font(&mut harness, "Menlo");
+    answer(&mut harness, 1, installed_read("Menlo", true));
+    // Iosevka, then Menlo again: Iosevka's read ends first, and finds
+    // nothing. Menlo's faces stay drawn until its own read ends.
+    font(&mut harness, "Iosevka");
+    font(&mut harness, "Menlo");
+    answer(&mut harness, 2, installed_read("Iosevka", false));
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+    assert_eq!(leads(&harness), "installed-mono-400");
+    answer(&mut harness, 3, installed_read("Menlo", true));
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+    // The newest read is taken whatever it read.
+    font(&mut harness, "Gone Mono");
+    answer(&mut harness, 4, installed_read("Gone Mono", false));
+    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    assert_eq!(leads(&harness), "plex-mono-400");
+    // From a bundled face too: it stays until the read ends.
+    font(&mut harness, "JetBrains Mono");
+    font(&mut harness, "Fira Code");
+    assert_eq!(harness.app.look.faces.data, DataFace::JetBrainsMono);
+    answer(&mut harness, 5, installed_read("Fira Code", true));
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+}
```

- [ ] **Step 2: Run them.** `$C test --locked -p tabletist --lib -- font`. Expected, with the block above alone: `error[E0616]: field follow_desktop of struct App is private`. With `src/app.rs`'s test hunks in too: also `no variant named ListFonts` and `Fonts` on `Command` and `Event`, and `no field installed_fonts on type App`.

- [ ] **Step 3: Write the code.** `reads_font()` (the terminal look, or macOS with a name of no bundled face, where the desktop is followed); `installed_drawn()` (the name's faces read) and `text_look()`, which keeps the face drawn while a read is on its way; the read in `attach` and the look after it; the read asked for before the look is set again in `apply_settings`, so that `installed_drawn()` sees it on its way; a read is dropped where it is older than the newest taken, or older than the newest asked and for a name since changed; after a read lands, the faces set (`set_read_faces` hints the installed families as the terminal's), the look set again, and the grid's marks forgotten for every look and size (`forget_marks` by type: every installed font has the same `Faces`, so a key by faces cannot tell them apart); `Command::ListFonts` and `Event::Fonts`; `installed_fonts`; `follow_desktop` visible to the tests.

```diff
diff --git a/src/app.rs b/src/app.rs
index f13d51d..7e181f2 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -141,7 +141,7 @@ pub struct App {
     /// The OS theme seen last frame, to notice light/dark switches.
     system_theme: Option<egui::Theme>,
     /// Whether the desktop's themes are followed (not in tests or the demo).
-    follow_desktop: bool,
+    pub(crate) follow_desktop: bool,
     /// The settings named another theme since the last `logic`.
     theme_changed: bool,
     /// The light or dark the window's own parts were last told to take:
@@ -160,8 +160,13 @@ pub struct App {
     /// once.
     pin_told: Option<String>,
     /// What fontconfig gave the terminal look for the font the settings
-    /// name: nothing in tests, the demo and the other looks.
+    /// name, or CoreText macOS's data: nothing in tests, the demo and
+    /// Windows.
     pub desktop_font: crate::typography::desktop_font::Read,
+    /// The installed monospace families macOS's data font menu lists after
+    /// the bundled faces: read each time Settings opens, none in tests and
+    /// the demo.
+    pub installed_fonts: Vec<String>,
     /// The number of the last font read asked for, and of the newest one
     /// taken, as for the theme's.
     font_asked: u64,
@@ -270,6 +275,7 @@ impl App {
             crate::settings::Zone::System | crate::settings::Zone::Utc => None,
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
+        // An installed data font is read once the window exists (`attach`).
         let look = crate::theme::Look::for_platform().with_text(&settings, false);
         let mut app = Self {
             dirs,
@@ -310,6 +316,7 @@ impl App {
             theme_taken: 0,
             pin_told: None,
             desktop_font: crate::typography::desktop_font::Read::default(),
+            installed_fonts: Vec::new(),
             font_asked: 0,
             font_taken: 0,
             font_read: false,
@@ -2035,6 +2042,7 @@ impl App {
                 // for while it is open, it stays as it is.
                 if matches!(self.dialog, None | Some(Dialog::Help)) {
                     self.dialog = Some(Dialog::Settings(Box::default()));
+                    self.list_fonts();
                 }
             }
             Action::ShowAgentSettings => {
@@ -2045,6 +2053,8 @@ impl App {
                         pane: crate::model::SettingsPane::Agents,
                         ..Default::default()
                     })));
+                    // Appearance is a click away.
+                    self.list_fonts();
                 }
             }
             Action::ShowSettingsPane(pane) => {
@@ -3472,13 +3482,18 @@ impl App {
             Event::SettingsWatch { live } => self.settings_file.live = live,
             Event::Font { asked, read } => {
                 // An older read that ends later read a font since renamed.
-                if asked < self.font_taken {
+                // So did one that ends before a newer read, for a name the
+                // settings no longer hold: the faces drawn stay until the
+                // newer one ends.
+                let renamed = read.asked != self.settings.font;
+                if asked < self.font_taken || (asked < self.font_asked && renamed) {
                     return;
                 }
                 self.font_taken = asked;
                 self.desktop_font = *read;
                 self.font_read = true;
             }
+            Event::Fonts(families) => self.installed_fonts = families,
             Event::Theme { asked, read } => {
                 // Reads end in any order: one older than the newest taken
                 // saw a theme of Omarchy's that is gone, and one that read
@@ -4199,16 +4214,18 @@ impl App {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
         }
+        // Asked for first: an installed face drawn stays while the read of
+        // another is on its way.
+        if old.font != self.settings.font {
+            self.read_desktop_font();
+        }
         // The text's sizes and the data's face: every role drawn from here
         // on has them, and egui's own styles are set again in `logic`.
-        let look = self.look.with_text(&self.settings, false);
+        let look = self.text_look();
         if look != self.look {
             self.look = look;
             self.restyle = true;
         }
-        if old.font != self.settings.font {
-            self.read_desktop_font();
-        }
         // An edit of the file that turns the server on is held.
         self.check_agents();
         if self.serves_agents {
@@ -5863,9 +5880,10 @@ impl App {
         ctx.options_mut(|options| options.zoom_with_keyboard = false);
         // Read here, before the first frame, as the desktop's monospace
         // always was: a font that arrived later would move every line.
-        if follow_desktop && self.look.terminal {
-            self.desktop_font =
-                crate::typography::desktop_font::read(self.settings.font.as_deref());
+        if self.reads_font() {
+            self.desktop_font = crate::typography::read_font(self.settings.font.as_deref());
+            // macOS: the data in the installed font read.
+            self.look = self.text_look();
         }
         let desktop = self.desktop_font.faces.as_ref();
         self.fonts = theme::install_with(ctx, follow_desktop, &self.look, desktop);
@@ -5915,10 +5933,48 @@ impl App {
         });
     }
 
-    /// Asks the backend for the terminal look's font as the settings name
-    /// it, where the desktop's fonts are read (not in tests or the demo).
+    /// Whether the font the settings name is read from the system: the
+    /// terminal look's, and on macOS an installed one for the data (no
+    /// bundled face's name), where the desktop's fonts are read (not in
+    /// tests or the demo).
+    fn reads_font(&self) -> bool {
+        let installed = self.look.faces.typeface == crate::theme::Typeface::Plex
+            && crate::theme::DataFace::of(self.settings.font.as_deref()).is_err();
+        self.follow_desktop && (self.look.terminal || installed)
+    }
+
+    /// Whether macOS draws its data in an installed font: the one the
+    /// settings name, read.
+    pub(crate) fn installed_drawn(&self) -> bool {
+        self.desktop_font.has(self.settings.font.as_deref())
+    }
+
+    /// The look with its text as the settings have it ([`Look::with_text`]).
+    /// While the read of an installed font is on its way, the data's face
+    /// drawn stays (a bundled one, or the installed one read before) until
+    /// it ends.
+    fn text_look(&self) -> crate::theme::Look {
+        let mut look = self.look.with_text(&self.settings, self.installed_drawn());
+        let reading = self.font_asked > self.font_taken;
+        if reading && crate::theme::DataFace::of(self.settings.font.as_deref()).is_err() {
+            look.faces.data = self.look.faces.data;
+        }
+        look
+    }
+
+    /// Asks the backend for the installed monospace families the data
+    /// font's menu lists, where macOS draws data in a face of its own and
+    /// the desktop's fonts are read (not in tests or the demo).
+    fn list_fonts(&mut self) {
+        if self.follow_desktop && self.look.faces.typeface == crate::theme::Typeface::Plex {
+            self.backend.send(Command::ListFonts);
+        }
+    }
+
+    /// Asks the backend for the font the settings name, where it is read
+    /// from the system ([`Self::reads_font`]).
     fn read_desktop_font(&mut self) {
-        if !self.look.terminal || !self.follow_desktop {
+        if !self.reads_font() {
             return;
         }
         self.font_asked += 1;
@@ -6039,8 +6095,15 @@ impl App {
         self.tell_missing_pin(ctx);
         if std::mem::take(&mut self.font_read) {
             let desktop = self.desktop_font.faces.as_ref();
-            theme::set_terminal_faces(ctx, &mut self.fonts, &self.look, desktop);
-            crate::ui::grid::forget_marks(ctx, &self.look);
+            theme::set_read_faces(ctx, &mut self.fonts, &self.look, desktop);
+            crate::ui::grid::forget_marks(ctx);
+            // macOS: the data in the installed font read, or in the default
+            // where it is missing.
+            let look = self.text_look();
+            if look != self.look {
+                self.look = look;
+                self.restyle = true;
+            }
         }
         if std::mem::take(&mut self.restyle) {
             theme::apply(ctx, &self.palette, &self.look);
@@ -7121,15 +7184,66 @@ mod tests {
             .app
             .change_settings(|settings| settings.page_size = 500);
         assert_eq!(fonts_asked(&mut harness), vec![]);
-        // Not in tests and the demo, nor in the other looks, whose fonts
-        // are bundled.
+        // Not in tests and the demo.
         harness.app.follow_desktop = false;
         font(&mut harness, Some("Iosevka"));
         assert_eq!(fonts_asked(&mut harness), vec![]);
+        // macOS: an installed font for its data, not a bundled face.
         harness.app.follow_desktop = true;
         harness.app.look = crate::theme::Look::macos();
         font(&mut harness, Some("JetBrains Mono"));
+        font(&mut harness, None);
         assert_eq!(fonts_asked(&mut harness), vec![]);
+        font(&mut harness, Some("Menlo"));
+        assert_eq!(fonts_asked(&mut harness), vec![(Some("Menlo".into()), 3)]);
+        // Windows draws its data in Inter, which has no face to read.
+        harness.app.look = crate::theme::Look::standard();
+        font(&mut harness, Some("Consolas"));
+        assert_eq!(fonts_asked(&mut harness), vec![]);
+    }
+
+    #[test]
+    fn the_installed_fonts_are_listed_each_time_settings_opens_on_a_mac() {
+        let listed = |harness: &mut Harness| {
+            std::mem::take(&mut harness.app.backend.sent)
+                .into_iter()
+                .filter(|command| matches!(command, Command::ListFonts))
+                .count()
+        };
+        let mut harness = Harness::new();
+        harness.app.look = crate::theme::Look::macos();
+        harness.app.follow_desktop = true;
+        harness.app.apply(Action::ShowSettings);
+        assert_eq!(listed(&mut harness), 1);
+        // Asked for while it is open, it stays as it is.
+        harness.app.apply(Action::ShowSettings);
+        assert_eq!(listed(&mut harness), 0);
+        harness.app.dialog = None;
+        harness.app.apply(Action::ShowSettings);
+        assert_eq!(listed(&mut harness), 1);
+        // Opened from the Agents tab, where agents can reach the app.
+        harness.app.dialog = None;
+        harness.app.apply(Action::ShowAgentSettings);
+        let agents = usize::from(crate::endpoint::AVAILABLE);
+        assert_eq!(listed(&mut harness), agents);
+        // The answer is the menu's list.
+        harness
+            .app
+            .apply(Action::Backend(Event::Fonts(vec!["Menlo".into()])));
+        assert_eq!(harness.app.installed_fonts, ["Menlo"]);
+        // Not in tests and the demo, nor in the looks that draw no data
+        // in a face of their own.
+        for (look, follow) in [
+            (crate::theme::Look::macos(), false),
+            (crate::theme::Look::omarchy(), true),
+            (crate::theme::Look::standard(), true),
+        ] {
+            harness.app.dialog = None;
+            harness.app.look = look;
+            harness.app.follow_desktop = follow;
+            harness.app.apply(Action::ShowSettings);
+            assert_eq!(listed(&mut harness), 0, "{}", look.name);
+        }
     }
 
     #[test]
diff --git a/src/backend.rs b/src/backend.rs
index cf6de10..0bfc37f 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -196,13 +196,18 @@ pub enum Command {
         pin: Option<String>,
         asked: u64,
     },
-    /// Reads the terminal look's font through fontconfig: the one `font`
-    /// names, or the system's monospace. Answered with [`Event::Font`],
-    /// which carries `asked`: reads end in any order.
+    /// Reads the font `font` names ([`crate::typography::read_font`]): the
+    /// terminal look's through fontconfig, or the system's monospace; on
+    /// macOS an installed one the data is drawn in, through CoreText.
+    /// Answered with [`Event::Font`], which carries `asked`: reads end in
+    /// any order.
     ReadFont {
         font: Option<String>,
         asked: u64,
     },
+    /// Lists the installed monospace families the data font's menu offers
+    /// on macOS. Answered with [`Event::Fonts`].
+    ListFonts,
     /// Signals `done` once every save sent before it is on disk.
     Flush {
         done: mpsc::Sender<()>,
@@ -370,6 +375,9 @@ pub enum Event {
         asked: u64,
         read: Box<crate::typography::desktop_font::Read>,
     },
+    /// The installed monospace families ([`Command::ListFonts`]), as the
+    /// menu lists them.
+    Fonts(Vec<String>),
     /// The time zone database, read whole ([`Command::LoadZones`]).
     Zones(Arc<crate::zones::Zones>),
     /// The settings file changed on disk: its text. `own` when it is, byte
@@ -2007,13 +2015,19 @@ impl Worker {
             Command::ReadFont { font, asked } => {
                 let outbox = self.outbox.clone();
                 tokio::task::spawn_blocking(move || {
-                    let read = crate::typography::desktop_font::read(font.as_deref());
+                    let read = crate::typography::read_font(font.as_deref());
                     outbox.emit(Event::Font {
                         asked,
                         read: Box::new(read),
                     });
                 });
             }
+            Command::ListFonts => {
+                let outbox = self.outbox.clone();
+                tokio::task::spawn_blocking(move || {
+                    outbox.emit(Event::Fonts(crate::typography::installed_font::families()));
+                });
+            }
             Command::Flush { done } => {
                 let saves = self.saves.clone();
                 tokio::spawn(async move {
@@ -2071,6 +2085,7 @@ fn session_of(command: &Command) -> SessionId {
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
         | Command::ReadFont { .. }
+        | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_) => SessionId(0),
     }
@@ -2102,6 +2117,7 @@ fn request_of(command: &Command) -> Option<RequestId> {
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
         | Command::ReadFont { .. }
+        | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_) => None,
     }
@@ -2191,6 +2207,7 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
         | Command::ReadFont { .. }
+        | Command::ListFonts
         | Command::Flush { .. }
         | Command::Agents(_) => return,
     };
@@ -2476,6 +2493,7 @@ async fn run_session(
             | Command::RevealSettingsFile { .. }
             | Command::ReadTheme { .. }
             | Command::ReadFont { .. }
+            | Command::ListFonts
             | Command::Flush { .. }
             | Command::Agents(_) => None,
         };
diff --git a/src/testing.rs b/src/testing.rs
index 3888c0f..fe05fdd 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -1586,9 +1586,10 @@ impl Harness {
 
     /// Draws with `look` from the next frame on.
     pub fn set_look(&mut self, look: crate::theme::Look) {
-        let look = look.with_text(&self.app.settings, false);
+        let look = look.with_text(&self.app.settings, self.app.installed_drawn());
         self.app.look = look;
-        self.app.fonts = crate::theme::install_with(&self.ctx, false, &look, None);
+        let read = self.app.desktop_font.faces.as_ref();
+        self.app.fonts = crate::theme::install_with(&self.ctx, false, &look, read);
         crate::theme::apply(&self.ctx, &self.app.palette, &look);
     }
 }
diff --git a/src/theme.rs b/src/theme.rs
index 7b7806f..e31036b 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -772,7 +772,7 @@ pub fn install(ctx: &egui::Context, system_fallbacks: bool, look: &Look) {
 
 /// [`install`], with `desktop`, the terminal look's font as fontconfig gave
 /// it (never in tests or screenshots): the bundled one is drawn without.
-/// Returns the fonts it set, for [`set_terminal_faces`].
+/// Returns the fonts it set, for [`set_read_faces`].
 pub fn install_with(
     ctx: &egui::Context,
     system_fallbacks: bool,
@@ -790,10 +790,11 @@ pub fn install_with(
     fonts
 }
 
-/// Draws the terminal look in `desktop`, the font fontconfig gave for the
-/// settings' name, or in the bundled one for none: `fonts`, the ones set,
-/// with the terminal's faces put in. Nothing is read from the disk here.
-pub fn set_terminal_faces(
+/// Draws `desktop`, the font read for the settings' name: the terminal
+/// look in the one fontconfig gave, or in the bundled one for none; macOS's
+/// data in an installed one, or in Plex Mono for none. `fonts`, the ones
+/// set, with the faces put in. Nothing is read from the disk here.
+pub fn set_read_faces(
     ctx: &egui::Context,
     fonts: &mut egui::FontDefinitions,
     look: &Look,
@@ -801,9 +802,12 @@ pub fn set_terminal_faces(
 ) {
     crate::typography::configure(fonts, look, desktop);
     // The others have the rendering's hinting already.
-    for weight in [400, 500, 700] {
-        let name = crate::typography::family_name(Typeface::Terminal, Kind::Mono, weight);
-        if let Some(data) = fonts.font_data.get_mut(&name) {
+    let terminal = [400, 500, 700]
+        .map(|weight| crate::typography::family_name(Typeface::Terminal, Kind::Mono, weight));
+    let installed =
+        [400, 500].map(|weight| crate::typography::data_family_name(DataFace::Installed, weight));
+    for name in terminal.iter().chain(&installed) {
+        if let Some(data) = fonts.font_data.get_mut(name) {
             text_rendering().tweak(&mut std::sync::Arc::make_mut(data).tweak);
         }
     }
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index fad160c..2d6f7bf 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -301,10 +301,11 @@ pub(crate) fn marks_id(look: &Look) -> Id {
     Id::new(("cell-marks", look.faces))
 }
 
-/// Forgets the marks found for `look`'s faces: the terminal look's face
-/// changed under them (a new font read through fontconfig).
-pub fn forget_marks(ctx: &egui::Context, look: &Look) {
-    ctx.data_mut(|data| data.remove::<Marks>(marks_id(look)));
+/// Forgets the marks found for every look's faces: a face read from the
+/// system changed under them (the terminal look's through fontconfig, or
+/// macOS's installed data font), at every size they were found at.
+pub fn forget_marks(ctx: &egui::Context) {
+    ctx.data_mut(|data| data.remove_by_type::<Marks>());
 }
 
 /// Paints `text` in `role` at `x` (its left edge, or its right with
```

- [ ] **Step 4: Check.** Every check above.

- [ ] **Step 5: Commit.** `git add src && git commit -m "Read the installed data font at the start and after a change, and list the families when Settings opens"`

### Task 4: The menu lists the installed fonts, and the row says one that is not installed

**Files:**
- Modify: `src/ui/settings/sheet.rs`, `src/ui/settings/mod.rs`, `src/app.rs` (`data_font_missing`)
- Test: `src/ui/text_settings_tests.rs`
- Docs: `docs/_guide/macos.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the tests.** The menu with a list of the test's own: the bundled faces first, the heading, the families; the default marked; a family picked is written; read, the field reads it and the menu marks it; a bundled face again is drawn at once. Without a list, no heading. The test of task 3 grows the field and the small print: the name on its way, "“Gone Mono” is not installed: IBM Plex Mono is used" once its read says so. Step 3's message changes.

```diff
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index d74c399..f68d41e 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -230,7 +230,7 @@ fn a_data_font_that_is_not_bundled_is_drawn_as_the_default_and_said() {
     on_the_pane(&mut harness);
     assert!(shows(
         &mut harness,
-        "“Fira Code” is not bundled: IBM Plex Mono is used"
+        "“Fira Code” is not installed: IBM Plex Mono is used"
     ));
     assert!(!shows(&mut harness, "Grid cells, SQL editor, JSON"));
     // The menu shows the face drawn.
@@ -359,6 +359,23 @@ fn the_screen_says_the_font_fontconfig_drew_and_one_it_has_not() {
     assert!(!file(&harness).contains("font ="));
 }
 
+/// Whether the entry called `label` is told to a screen reader as the
+/// one in use.
+fn current(harness: &mut Harness, label: &str) -> Option<bool> {
+    let tree = harness.settle();
+    let node = crate::testing::node(&tree, label, egui::accesskit::Role::Button)?;
+    let (_, node) = tree.nodes.iter().find(|(id, _)| *id == node)?;
+    Some(node.toggled() == Some(egui::accesskit::Toggled::True))
+}
+
+/// What the data font's field reads.
+fn data_font_shown(harness: &mut Harness) -> Option<String> {
+    let tree = harness.settle();
+    let id = crate::testing::node(&tree, "Data font", egui::accesskit::Role::ComboBox)?;
+    let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id)?;
+    node.value().map(str::to_owned)
+}
+
 /// What CoreText would give for `name`: Hack's faces (egui's own), or
 /// none where it is not installed.
 fn installed_read(name: &str, found: bool) -> Read {
@@ -384,15 +401,74 @@ fn answer(harness: &mut Harness, asked: u64, read: Read) {
 }
 
 #[test]
-fn an_installed_font_is_drawn_once_read_and_the_default_where_it_is_not() {
+fn the_mac_menu_lists_the_installed_fonts_after_the_bundled_ones() {
+    let mut harness = sheet(Look::macos());
+    // CoreText's list, as the backend gives it: a list of the test's.
+    harness.app.installed_fonts = vec!["Fira Code".into(), "Menlo".into()];
+    on_the_pane(&mut harness);
+    harness.click("Data font");
+    let tree = harness.settle();
+    let place = |name| crate::testing::bounds(&tree, name, egui::accesskit::Role::Button);
+    let names = [
+        "Data font IBM Plex Mono",
+        "Data font JetBrains Mono",
+        "Data font Fira Code",
+        "Data font Menlo",
+    ];
+    let tops: Vec<f32> = names
+        .iter()
+        .map(|name| place(name).unwrap_or_else(|| panic!("{name}")).top())
+        .collect();
+    assert!(tops.is_sorted() && tops[0] < tops[3], "{tops:?}");
+    assert!(shows(&mut harness, "Installed"));
+    assert_eq!(current(&mut harness, "Data font IBM Plex Mono"), Some(true));
+    assert_eq!(current(&mut harness, "Data font Menlo"), Some(false));
+    let saves = settings_saves(&harness);
+    harness.click("Data font Menlo");
+    harness.settle();
+    assert_eq!(harness.app.settings.font.as_deref(), Some("Menlo"));
+    assert!(file(&harness).contains("font = \"Menlo\""));
+    assert_eq!(settings_saves(&harness), saves + 1);
+    // Read, it is drawn: the field reads it, and the menu has it in use.
+    answer(&mut harness, 1, installed_read("Menlo", true));
+    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
+    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Menlo"));
+    assert!(shows(&mut harness, "Grid cells, SQL editor, JSON"));
+    harness.click("Data font");
+    assert_eq!(current(&mut harness, "Data font Menlo"), Some(true));
+    assert_eq!(
+        current(&mut harness, "Data font IBM Plex Mono"),
+        Some(false)
+    );
+    // A bundled face again: drawn as it always was.
+    harness.click("Data font JetBrains Mono");
+    harness.settle();
+    assert_eq!(harness.app.settings.font.as_deref(), Some("JetBrains Mono"));
+    assert_eq!(harness.app.look.faces.data, DataFace::JetBrainsMono);
+}
+
+#[test]
+fn without_installed_fonts_the_menu_has_the_bundled_ones_alone() {
+    let mut harness = sheet(Look::macos());
+    on_the_pane(&mut harness);
+    harness.click("Data font");
+    assert!(harness.has("Data font JetBrains Mono"));
+    assert!(!shows(&mut harness, "Installed"));
+}
+
+#[test]
+fn an_installed_font_is_drawn_once_read_and_said_where_it_is_not_installed() {
     let mut harness = sheet(Look::macos());
     // Where the desktop's fonts are read, as the app does outside tests.
     harness.app.follow_desktop = true;
     harness.app.change_settings(|settings| {
         settings.font = Some("Fira Code".into());
     });
-    // On its way: the default until it ends.
+    on_the_pane(&mut harness);
+    // On its way: the field reads the name, and nothing is said.
     assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Fira Code"));
+    assert!(shows(&mut harness, "Grid cells, SQL editor, JSON"));
     answer(&mut harness, 1, installed_read("Fira Code", true));
     assert_eq!(harness.app.look.faces.data, DataFace::Installed);
     // Its faces lead the installed families.
@@ -409,9 +485,18 @@ fn an_installed_font_is_drawn_once_read_and_the_default_where_it_is_not() {
         settings.font = Some("Gone Mono".into());
     });
     assert_eq!(harness.app.look.faces.data, DataFace::Installed);
-    // Not installed: the default.
+    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Gone Mono"));
+    // Not installed: the default, said, and the field reads the default.
     answer(&mut harness, 2, installed_read("Gone Mono", false));
     assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    assert!(shows(
+        &mut harness,
+        "“Gone Mono” is not installed: IBM Plex Mono is used"
+    ));
+    assert_eq!(
+        data_font_shown(&mut harness).as_deref(),
+        Some("IBM Plex Mono")
+    );
     let fonts = &harness.app.fonts;
     let family = egui::FontFamily::Name("installed-mono-400".into());
     assert_eq!(fonts.families[&family][0], "plex-mono-400");
```

- [ ] **Step 2: Run them.** `$C test --locked -p tabletist --lib -- text_settings`. Expected: three fail: `the_mac_menu_lists_the_installed_fonts_after_the_bundled_ones` (no node "Data font Fira Code"), `an_installed_font_is_drawn_once_read_and_said_where_it_is_not_installed` (`left: Some("IBM Plex Mono"), right: Some("Fira Code")`), and step 3's not-bundled test (the old words).

- [ ] **Step 3: Write the code.** `Skin` carries `DataFonts` (the list and `App::data_font_missing()`); the field reads an installed name drawn or on its way, the default for a missing one; the menu adds a heading, "Installed", and the families; the small print's words.

```diff
diff --git a/src/app.rs b/src/app.rs
index 7e181f2..6568f71 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5962,6 +5962,16 @@ impl App {
         look
     }
 
+    /// The data font the settings name where nothing draws it on macOS: no
+    /// bundled face, and no installed one read, nor on its way. The default
+    /// is drawn in its place.
+    pub(crate) fn data_font_missing(&self) -> Option<&str> {
+        let name = crate::theme::DataFace::of(self.settings.font.as_deref()).err()?;
+        let reading = self.font_asked > self.font_taken;
+        let drawn = self.look.faces.data == crate::theme::DataFace::Installed;
+        (!drawn && !reading).then_some(name)
+    }
+
     /// Asks the backend for the installed monospace families the data
     /// font's menu lists, where macOS draws data in a face of its own and
     /// the desktop's fonts are read (not in tests or the demo).
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index c370d25..85975bd 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -105,9 +105,10 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     }
 }
 
-/// What is said of a data font the settings name that is not bundled,
-/// `{name}` for its name: the default is drawn instead. English.
-pub const FONT_NOT_BUNDLED: &str = "“{name}” is not bundled: IBM Plex Mono is used";
+/// What is said of a data font the settings name that is neither bundled
+/// nor installed, `{name}` for its name: the default is drawn instead.
+/// English.
+pub const FONT_NOT_INSTALLED: &str = "“{name}” is not installed: IBM Plex Mono is used";
 
 /// What is said of a pinned theme that is nowhere to be found, `{name}`
 /// for its name: what is drawn instead. English.
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index bb39fac..558a336 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -88,6 +88,21 @@ pub(super) struct Skin<'a> {
     /// The zone database's names, which the zone menu lists: none until
     /// the backend has read it.
     pub(super) zones: &'a [String],
+    /// The data fonts beyond the bundled faces, on macOS.
+    pub(super) fonts: DataFonts<'a>,
+}
+
+/// The installed fonts macOS's data font menu lists after the bundled
+/// faces, and the one the settings name where nothing draws it.
+#[derive(Clone, Copy)]
+pub(super) struct DataFonts<'a> {
+    /// The installed monospace families: none until the backend has
+    /// listed them.
+    pub(super) installed: &'a [String],
+    /// The font the settings name, where it is no bundled face and no
+    /// installed one is read or on its way ([`App::data_font_missing`]):
+    /// the default is drawn in its place.
+    pub(super) missing: Option<&'a str>,
 }
 
 impl Skin<'_> {
@@ -110,6 +125,10 @@ pub(super) fn show(
         locale: app.locale,
         system: app.system_zone.iana_name(),
         zones: app.zones.as_deref().map_or(&[], crate::zones::Zones::names),
+        fonts: DataFonts {
+            installed: &app.installed_fonts,
+            missing: app.data_font_missing(),
+        },
     };
     let screen = ctx.content_rect();
     let width = WIDTH.min(screen.width() - 2.0 * MARGIN).max(NAV + 320.0);
@@ -584,11 +603,13 @@ pub(super) fn row(
     )
     .wrap(column)
     .layout(ui.ctx());
-    // A data font that is not bundled is said in place of the small
+    // A data font that is not installed is said in place of the small
     // print, where the row has the room for it.
-    let print = match not_bundled(option, settings, look) {
+    let print = match missing_font(option, skin) {
         Some(missing) => {
-            let said = skin.say(super::FONT_NOT_BUNDLED).replace("{name}", missing);
+            let said = skin
+                .say(super::FONT_NOT_INSTALLED)
+                .replace("{name}", missing);
             Some((said, palette.warning))
         }
         None => small_print(option).map(|print| (skin.say(print), palette.dim)),
@@ -650,11 +671,11 @@ pub(super) fn row(
 
 /// The data font the settings name, on its row, where it is no face the
 /// look has: the default is drawn in its place.
-fn not_bundled<'a>(option: OptionId, settings: &'a Settings, look: &Look) -> Option<&'a str> {
-    let plex = look.faces.typeface == Typeface::Plex;
-    (option == OptionId::Font && plex)
-        .then(|| DataFace::of(settings.font.as_deref()).err())
-        .flatten()
+fn missing_font<'a>(option: OptionId, skin: &Skin<'a>) -> Option<&'a str> {
+    let plex = skin.look.faces.typeface == Typeface::Plex;
+    skin.fonts
+        .missing
+        .filter(|_| option == OptionId::Font && plex)
 }
 
 /// Whether the keyboard came in this frame to a control in `place`, on the
@@ -894,8 +915,10 @@ fn control(
     }
 }
 
-/// The data font's menu: the bundled faces. A name that is none of them
-/// is drawn as the default, which the row's hint says.
+/// The data font's menu: the bundled faces, then the installed monospace
+/// families. An installed font reads as the settings name it, drawn or on
+/// its way; a name that nothing draws reads as the default drawn in its
+/// place, which the row's small print says.
 fn font_menu(
     ui: &mut Ui,
     (name, width): (&str, f32),
@@ -904,22 +927,34 @@ fn font_menu(
     actions: &mut Vec<Action>,
 ) {
     let Skin { look, palette, .. } = *skin;
-    let drawn = DataFace::of(settings.font.as_deref()).unwrap_or_default();
-    let shown = drawn.name().unwrap_or_default();
+    let named = DataFace::of(settings.font.as_deref());
+    let installed = named.err().filter(|_| skin.fonts.missing.is_none());
+    let bundled = named.unwrap_or_default();
+    let shown = installed.or(bundled.name()).unwrap_or_default();
     let dropdown = menu::Dropdown::new("data-font", shown, width);
     let menu = dropdown.show(ui, look, palette, |ui| {
-        for face in DataFace::BUNDLED {
-            let text = face.name().unwrap_or_default();
+        let mut pick = |ui: &mut Ui, text: &str, current: bool, font: Option<String>| {
             let entry_name = format!("{name} {text}");
-            let entry = menu::Item::choice(text, face == drawn)
+            let entry = menu::Item::choice(text, current)
                 .name(&entry_name)
                 .show(ui, look, palette);
             if entry.clicked() {
-                // The default is no value: the file leaves it out.
-                let font = (face != DataFace::default()).then(|| text.to_owned());
                 actions.push(Action::SetOption(OptionValue::Font(font)));
                 ui.close();
             }
+        };
+        for face in DataFace::BUNDLED {
+            let text = face.name().unwrap_or_default();
+            // The default is no value: the file leaves it out.
+            let font = (face != DataFace::default()).then(|| text.to_owned());
+            pick(ui, text, installed.is_none() && face == bundled, font);
+        }
+        if !skin.fonts.installed.is_empty() {
+            menu::heading(ui, &skin.say("Installed"), look, palette);
+        }
+        for family in skin.fonts.installed {
+            let current = installed.is_some_and(|name| name.eq_ignore_ascii_case(family));
+            pick(ui, family, current, Some(family.clone()));
         }
     });
     menu.response.widget_info(|| {
```

- [ ] **Step 4: The website.**

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index f599b90..7acf33b 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -31,9 +31,11 @@ launch, are on the
   **Tabletist → Settings…** opens Settings.
 - **Text has two sizes.** **Settings › Appearance** sizes the interface
   (Small, Default or Large) and the data apart: grid cells, the SQL
-  editor and JSON, in IBM Plex Mono or JetBrains Mono, from 11 to 16
-  points. Cmd+= and Cmd+- change the data's size wherever you are, and
-  Cmd+0 puts back 12.5.
+  editor and JSON, from 11 to 16 points, in IBM Plex Mono or JetBrains
+  Mono, which Tabletist bundles, or in a monospace font installed on the
+  Mac (Menlo, or one of your own), which the menu lists after them.
+  Cmd+= and Cmd+- change the data's size wherever you are, and Cmd+0
+  puts back 12.5.
 
 {% include shot.html file="macos-table-dark" alt="The same window in the dark appearance" %}
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 4e0c483..a511733 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -32,7 +32,7 @@ are made in the app, so it needs no connection and reads no database.
 | **Theme** (macOS and Windows) | System, Light or Dark. System follows the system's light or dark as it changes, Auto on macOS too | System |
 | **Theme** (Omarchy) | Follow Omarchy, or pin the theme it shows now, which then stays when Omarchy switches. The row says the theme drawn, and under it the palette it gave | Follow Omarchy |
 | **Interface size** (macOS and Windows) | Small, Default or Large: the sidebar, tabs, buttons and the inspector's labels, from a body text of 12, 13 or 14 points | Default |
-| **Data font** (macOS and Windows) | The face of grid cells, the SQL editor and JSON on macOS, IBM Plex Mono or JetBrains Mono (Windows draws its data in Inter, and offers none), and its size, 11 to 16 points by halves | IBM Plex Mono, 12.5 pt |
+| **Data font** (macOS and Windows) | The face of grid cells, the SQL editor and JSON on macOS, IBM Plex Mono or JetBrains Mono, or an installed monospace font, listed after them (Windows draws its data in Inter, and offers none), and its size, 11 to 16 points by halves | IBM Plex Mono, 12.5 pt |
 | **Font** (Omarchy) | The system's monospace, as fontconfig resolves it, or a font you name in the file by its fontconfig name. The row says the font drawn; one fontconfig does not have is drawn as the system's monospace, and the row says so | System monospace |
 | **Size** (Omarchy) | 9 to 24 points: everything in the window, which is drawn larger or smaller as a terminal's text is | 13 |
 | **Row density** (Omarchy: row height) | Compact, Regular or Roomy. On macOS and Windows a step of 8 points either side of Regular, and taller where a large data size needs it; in the Omarchy look one, one and a half or two lines. The grid keeps the row at its top | Regular |
@@ -117,7 +117,7 @@ bar = "tint"
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
 | `appearance.interface_size` | `"small"`, `"default"` or `"large"`, on macOS and Windows | `"default"` |
-| `appearance.font` | On macOS the data's face, `"IBM Plex Mono"` or `"JetBrains Mono"`; `"monospace"` is IBM Plex Mono too, and any other name is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
+| `appearance.font` | On macOS the data's face, `"IBM Plex Mono"`, `"JetBrains Mono"` or the family name of an installed font (`"Menlo"`); `"monospace"` is IBM Plex Mono too, and a name no installed font has is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
 | `appearance.font_size` | Points, whole or by halves (`13`, `12.5`). On macOS and Windows the data's, 11 to 16; in the Omarchy look all of it, 9 to 24. A size past a look's range is drawn at its nearest end. Written once you set it | Not set: 12.5 on macOS and Windows, 13 in the Omarchy look |
 | `appearance.theme` | On macOS and Windows `"system"`, `"light"` or `"dark"`. In the Omarchy look `"omarchy"`, or a theme to pin: an Omarchy theme's name (`"Tokyo Night"`), a palette file in the `themes` folder (`"Nord.json"`; its name alone where no Omarchy theme has it), or a path that starts with `/` or `~/` to a palette file or an Omarchy theme's folder. A value meant for the other looks is drawn as not set and stays in the file | Not set: the system's light or dark, or the Omarchy theme |
 | `appearance.grid.row_height` | `"compact"`, `"regular"` or `"roomy"` | `"regular"` |
```

- [ ] **Step 5: Check.** Every check above, and `cd docs && bin/check` for the pages.

- [ ] **Step 6: Commit.** `git add src docs && git commit -m "List the installed monospace fonts in the data font menu and say one that is not installed"`

---

## Left open

A review of the draft (which found the dropped-read defect above) also raised these, unproven, not done here:

- A listed family that cannot be read (bitmap only, or a variable font inside a collection, whose instances' PostScript names are no face's of the file) is said not installed while the menu lists it.
- `set_read_faces` runs `plex()` again, which puts Plex, JetBrains Mono and the key symbols back without the hinting tweak `install_with` gave them. On macOS the tweak's values are the globals, so nothing is expected to show.
- `Arc::make_mut` in `set_read_faces` copies a font file shared by two weights (as the terminal's already did), and a collection is read once per face picked.
- The read at the start runs on the UI thread with no time bound (fontconfig's has half a second a call). CoreText answers from its own cache; a Mac with very many fonts could start later.
- An installed font with a tall line box may want a taller grid row than `Look::row`'s 1.3 lines.
- Fonts not marked fixed-pitch (some Iosevka builds) are not listed, but are drawn when the file names them.

## By hand, on a Mac

Not possible in these sessions (Linux, no display). On a Mac, with a monospace font of your own installed:

1. Settings › Appearance › Data font: the bundled two, "Installed", then Menlo and yours, sorted; no `.SF NS Mono`.
2. Pick yours: the grid redraws in it at once; quit and start again: it is drawn from the first frame.
3. Write `font = "menlo"` in the file: Menlo is drawn, the field reads "menlo".
4. Write `font = "No Such Mono"`: Plex Mono, and the row says it is not installed.
5. Uninstall the font picked, start again: Plex Mono, said.
