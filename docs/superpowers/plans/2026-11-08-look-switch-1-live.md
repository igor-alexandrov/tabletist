# Look switch, run 1: Classic or Omarchy, live — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use rubypowers:subagent-driven-development (recommended) or rubypowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every OS chooses between its classic look and the Omarchy look in Settings › Appearance or `settings.toml` (`appearance.look`), and the window switches at once.

**Architecture:** `Look::chosen(choice, os)` replaces the compile-time `Look::for_platform()` as the source of the look, and a look carries its key layout (`Look::keys`), so Linux's classic look is macOS's drawn with Windows' Ctrl keys. A new option, `OptionId::Look`, is stored as `appearance.look` and set from both settings screens. `App::apply_settings` notices a new choice and `App::switch_look` draws it over two frames: the fonts of the look drawn and of the new one are installed in one, which egui puts in place at the next frame's start, and the new look is drawn from that next frame. Linux builds bundle IBM Plex, the macOS look's faces, for its classic look.

**Tech Stack:** Rust 2024, egui/eframe, the workspace's headless `Harness` (`src/testing.rs`).

**Spec:** `docs/superpowers/specs/2026-11-08-look-switch-design.md`. Run 2 (the Windows menu strip, the macOS menu with Omarchy keys, the Omarchy top bar beside macOS's window buttons, fonts and the Updates pane off Linux) is a later plan.

**Base:** 6bf52932 (the spec's commit on claude/macos-omarchi-ui-switch-2e984c). Commands run from the worktree root. Cargo is `~/.cargo/bin/cargo` here (the mise shim fails).

---

## Departures from the spec, for a yes

1. **The sheet has no Interface section.** On the macOS/Windows sheet (920 pt tall, as the canvas draws it) a section above Theme pushed the Appearance preview about 50 pt below the fold. Igor chose (2026-11-08) to put the control at the right of the Theme section's tiles instead; the tiles' hint ("System follows macOS, including Auto.") moves under them. A sheet too narrow for both draws the look as an ordinary row under the tiles. The Omarchy board keeps its `interface` section, first in `appearance`.
2. **Reset to defaults leaves the look as it is.** Appearance's Reset to defaults puts back every option of the pane but the look: a question about defaults should not change the whole window. The spec says nothing of it.
3. **The theme's note in the file follows the chosen look.** `theme = "dark"  # system | light | dark` on a Linux file that chooses the classic look, where the OS's note would name Omarchy's values. A file that chooses nothing keeps the OS's note, as today.
4. **The switch takes two frames.** egui puts fonts set in a frame in place at the next frame's start; a look drawn in the frame its fonts were set in panics on an unbound family (`plex-sans-600`). So the fonts of both looks are installed in one frame (`style::install_both`: a change back before the switch lands draws either), the new look is drawn from the next, and its fonts alone are set then. A switch to the terminal look reads the desktop's monospace first, as a start does, so no line moves after it lands. Settings on Appearance then has the keyboard on the look's control: the board's cursor on its row, the sheet's `asked` ground on the control's block.
5. **Linux bundles IBM Plex and Noto's key symbols** (`src/typography/fonts.rs`, about 800 KB more in the Linux binary) and About lists their notices there. The classic look on Linux is macOS's and draws in Plex; a Linux build had only JetBrains Mono, and the look's first label would panic. The app's own tests build every face, so `tests/looks.rs` draws each look a system can choose in the app as it ships.
6. **Words about macOS follow the look's keys, not its face.** "Passwords are stored in the macOS Keychain", the connection form's "Keychain", and the System tile's "System follows macOS, including Auto." went by the Plex face; they now go by the Mac's keys (`look.keys == Layout::Mac`), so Linux's classic look says "system keyring" and "System follows the desktop's light or dark.", and macOS screenshots rendered on Linux keep the Mac's words.
7. **Smaller points.** `Look::for_platform` is no longer `const`. The OS is the existing `update::release::Os`, whose fourth value, `Other`, draws the standard look as before. Picking the look already drawn while nothing is set writes nothing, as the other per-look options do (`OptionValue::for_look`).

## Files

| File | Task | What changes |
| --- | --- | --- |
| `crates/tabletist-core/src/theme.rs` | 1 | `Look::keys`, `Look::with_keys`, `Look::chosen`; `for_platform` calls `chosen(None, Os::current())` |
| `crates/tabletist-core/src/keymap.rs` | 1 | `Layout::of` returns `look.keys` (was inferred from the Plex typeface) |
| `crates/tabletist-core/src/settings.rs` | 1, 2 | `LookChoice` (1); `Settings::look`, `Key::Look`, `OptionId::Look`, `OptionValue::Look`, the theme's note (2) |
| `crates/tabletist-core/src/model.rs` | 2 | The look in the Appearance pane's sections and board rows; out of its reset |
| `src/ui/settings/mod.rs` | 2 | The option's label, small print, choices; `shown` fills the look drawn |
| `src/ui/settings/sheet.rs` | 2 | The control beside the theme's tiles (`look_beside`); its hint under them |
| `src/ui/settings/terminal.rs` | 2 | The board's `interface` section and row |
| `src/app.rs` | 2, 3 | The start takes the chosen look (2); `LookSwitch`, `switch_look`, the change in `apply_settings` (3) |
| `src/typography/fonts.rs`, `src/ui/about.rs` | 2 | Plex and the key symbols bundled on Linux, their notices in About |
| `src/ui/picker.rs`, `src/ui/connect_dialog/mod.rs` | 2 | The Keychain's words by the Mac's keys |
| `tests/looks.rs` | 2 | Each look this system can choose draws in the app as it ships |
| `src/ui/*_tests.rs` (five files) | 2 | The board's rows start one lower; a pixel of zoom rounding in the preview's; the System tile's words by the look's keys |
| `src/style.rs` | 3 | `install_both`: the fonts of two looks in one set |
| `src/ui/look_switch_tests.rs`, `src/ui/mod.rs` | 3 | The live switch, from both screens and the file |
| `docs/index.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md` | 4 | The Interface setting and `appearance.look` |

Commit after each task (signed: if 1Password refuses, stage and stop for Igor to unlock it).

---

### Task 1: `Look::chosen` and the look's key layout

A look says which keys it binds and writes (`Look::keys`), and `Look::chosen` gives the look a choice makes on an OS. Nothing chooses yet: `Look::for_platform()` is `chosen(None, Os::current())`, the look of today.

**Files:**
- Modify: `crates/tabletist-core/src/theme.rs`, `crates/tabletist-core/src/keymap.rs`, `crates/tabletist-core/src/settings.rs` (the `LookChoice` type)

- [ ] **Step 1: Write the failing test**

```diff
diff --git a/crates/tabletist-core/src/theme.rs b/crates/tabletist-core/src/theme.rs
--- a/crates/tabletist-core/src/theme.rs
+++ b/crates/tabletist-core/src/theme.rs
@@ -957,6 +957,33 @@
     }
 
     #[test]
+    fn each_os_chooses_its_classic_look_or_the_omarchy_one() {
+        use crate::settings::LookChoice::{Classic, Omarchy};
+        let linux_classic = Look::macos().with_keys(Layout::Windows);
+        for (choice, os, look) in [
+            (None, Os::MacOs, Look::macos()),
+            (None, Os::Windows, Look::standard()),
+            (None, Os::Linux, Look::omarchy()),
+            (None, Os::Other, Look::standard()),
+            (Some(Classic), Os::MacOs, Look::macos()),
+            (Some(Classic), Os::Windows, Look::standard()),
+            (Some(Classic), Os::Linux, linux_classic),
+            (Some(Classic), Os::Other, Look::standard()),
+            (Some(Omarchy), Os::MacOs, Look::omarchy()),
+            (Some(Omarchy), Os::Windows, Look::omarchy()),
+            (Some(Omarchy), Os::Linux, Look::omarchy()),
+        ] {
+            assert_eq!(Look::chosen(choice, os), look, "{choice:?} on {os:?}");
+        }
+        // Linux's classic look is macOS's with Windows' keys.
+        assert_eq!(linux_classic.name, "macos");
+        assert_eq!(Layout::of(&linux_classic), Layout::Windows);
+        for look in Look::ALL {
+            assert_eq!(Layout::of(&look), look.keys, "{}", look.name);
+        }
+    }
+
+    #[test]
     fn a_themes_faint_labels_are_lightened_until_they_read() {
         let palette = crate::fixtures::tokyo_night();
         assert!(
```

- [ ] **Step 2: Run it to see it fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist-core --lib theme::tests::each_os_chooses`
Expected: it does not compile:

```
error[E0432]: unresolved import `crate::settings::LookChoice`
error[E0433]: cannot find type `Layout` in this scope
error[E0433]: cannot find type `Os` in this scope
error[E0599]: no method named `with_keys` found for struct `theme::Look` in the current scope
error[E0599]: no associated function or constant named `chosen` found for struct `theme::Look` in the current scope
error[E0609]: no field `keys` on type `theme::Look`
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/keymap.rs b/crates/tabletist-core/src/keymap.rs
--- a/crates/tabletist-core/src/keymap.rs
+++ b/crates/tabletist-core/src/keymap.rs
@@ -15,7 +15,7 @@
 
 use egui::Key;
 
-use crate::theme::{Look, Typeface};
+use crate::theme::Look;
 
 mod overrides;
 pub use overrides::{Bind, Line, Never, Overrides, Refused, Unbind, Why, intern, refuse_key};
@@ -25,7 +25,8 @@
 pub enum Layout {
     /// The macOS look: its own keys, written with glyphs.
     Mac,
-    /// The Windows look: the Mac's keys with Ctrl for Cmd, written out.
+    /// The Windows look, and Linux's classic look (macOS's drawn with
+    /// these): the Mac's keys with Ctrl for Cmd, written out.
     Windows,
     /// The Omarchy look: vim keys and the `:` prompt, in lower case.
     Omarchy,
@@ -34,14 +35,9 @@
 impl Layout {
     pub const ALL: [Layout; 3] = [Layout::Mac, Layout::Windows, Layout::Omarchy];
 
+    /// The layout `look` binds and writes its keys in.
     pub fn of(look: &Look) -> Self {
-        if look.terminal {
-            Self::Omarchy
-        } else if look.faces.typeface == Typeface::Plex {
-            Self::Mac
-        } else {
-            Self::Windows
-        }
+        look.keys
     }
 }
 
diff --git a/crates/tabletist-core/src/settings.rs b/crates/tabletist-core/src/settings.rs
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -204,6 +204,47 @@
         [Self::Tint, Self::Label]
             .into_iter()
             .find(|choice| choice.name() == name)
+    }
+}
+
+/// What `appearance.look` says: the OS's classic look, or the Omarchy
+/// experience. No value is the look the OS draws without one
+/// ([`crate::theme::Look::chosen`]).
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum LookChoice {
+    /// macOS's look on macOS and Linux, the standard look on Windows.
+    Classic,
+    /// The terminal look, keyboard first.
+    Omarchy,
+}
+
+impl LookChoice {
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Classic => "classic",
+            Self::Omarchy => "omarchy",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        [Self::Classic, Self::Omarchy]
+            .into_iter()
+            .find(|choice| choice.name() == name)
+    }
+
+    /// The choice a look drawn is: the terminal's, or a classic one.
+    pub fn of(terminal: bool) -> Self {
+        if terminal {
+            Self::Omarchy
+        } else {
+            Self::Classic
+        }
+    }
+
+    /// Whether it is the terminal look.
+    pub fn terminal(self) -> bool {
+        self == Self::Omarchy
     }
 }
 
diff --git a/crates/tabletist-core/src/theme.rs b/crates/tabletist-core/src/theme.rs
--- a/crates/tabletist-core/src/theme.rs
+++ b/crates/tabletist-core/src/theme.rs
@@ -4,7 +4,11 @@
 
 use egui::Color32;
 
-use crate::settings::{FontSize, FontSizes, InterfaceSize, Mode, RowHeight, Settings, Theme};
+use crate::keymap::Layout;
+use crate::settings::{
+    FontSize, FontSizes, InterfaceSize, LookChoice, Mode, RowHeight, Settings, Theme,
+};
+use crate::update::release::Os;
 
 /// A palette file from the themes directory.
 pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
@@ -496,6 +500,9 @@
     /// A keyboard-first terminal interface (Omarchy): lower-case labels
     /// that lead with their keys, and key hints in bars.
     pub terminal: bool,
+    /// How the look binds and writes its keys: the Mac's, Windows', or
+    /// Omarchy's.
+    pub keys: Layout,
 }
 
 impl Look {
@@ -525,6 +532,7 @@
             faces: Faces::of(Typeface::Inter),
             primary: PrimaryStyle::Accent,
             terminal: false,
+            keys: Layout::Windows,
         }
     }
 
@@ -553,6 +561,7 @@
             faces: Faces::of(Typeface::Plex),
             primary: PrimaryStyle::Ink,
             terminal: false,
+            keys: Layout::Mac,
         }
     }
 
@@ -581,6 +590,25 @@
             faces: Faces::of(Typeface::Terminal),
             primary: PrimaryStyle::Outline,
             terminal: true,
+            keys: Layout::Omarchy,
+        }
+    }
+
+    /// The look with `keys` for its keys.
+    pub const fn with_keys(self, keys: Layout) -> Self {
+        Self { keys, ..self }
+    }
+
+    /// The look `choice` gives on `os`: with none, the look the OS draws
+    /// (the terminal look on Linux); its classic look; or the Omarchy
+    /// experience. Linux's classic look is macOS's with Windows' keys:
+    /// Ctrl written out, never ⌘.
+    pub const fn chosen(choice: Option<LookChoice>, os: Os) -> Self {
+        match (choice, os) {
+            (Some(LookChoice::Omarchy), _) | (None, Os::Linux) => Self::omarchy(),
+            (Some(LookChoice::Classic), Os::Linux) => Self::macos().with_keys(Layout::Windows),
+            (_, Os::MacOs) => Self::macos(),
+            (_, Os::Windows | Os::Other) => Self::standard(),
         }
     }
 
@@ -630,16 +658,10 @@
         row.max(line)
     }
 
-    /// The look of the OS this build runs on. Every Linux desktop gets the
-    /// Omarchy look.
-    pub const fn for_platform() -> Self {
-        if cfg!(target_os = "macos") {
-            Self::macos()
-        } else if cfg!(target_os = "linux") {
-            Self::omarchy()
-        } else {
-            Self::standard()
-        }
+    /// The look of the OS this build runs on, with nothing chosen. Every
+    /// Linux desktop gets the Omarchy look.
+    pub fn for_platform() -> Self {
+        Self::chosen(None, Os::current())
     }
 
     /// `text` as this look labels things: lower case in the terminal look.
```

- [ ] **Step 4: Run the checks**

Run:
```bash
~/.cargo/bin/cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```
Expected: all pass:

```
test result: ok. 2860 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in 49.16s
test result: ok. 1042 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.52s
```

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/theme.rs crates/tabletist-core/src/keymap.rs crates/tabletist-core/src/settings.rs
git commit -m "Give a look its key layout, and choose a look for an OS"
```

---

### Task 2: The `appearance.look` setting and its controls

`appearance.look` (`"classic"` or `"omarchy"`) is read and written, with its note, and a value it cannot have is marked and ignored. Both screens set it: the sheet's control beside the theme's tiles, the board's `interface` row first in `appearance`. The look drawn, picked while nothing is set, writes nothing. A launch draws the look the file chooses; a change while the app runs takes effect at the next launch until Task 3. Appearance's Reset to defaults leaves the look as it is. Linux builds bundle Plex for the classic look, and words about macOS (the Keychain, what the System tile follows) go by the look's keys.

**Files:**
- Modify: `crates/tabletist-core/src/settings.rs`, `crates/tabletist-core/src/model.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/app.rs` (the start), `src/typography/fonts.rs`, `src/ui/about.rs`, `src/ui/picker.rs`, `src/ui/connect_dialog/mod.rs`
- Create: `tests/looks.rs`
- Test: `crates/tabletist-core/src/settings.rs` (tests), `src/ui/about.rs` (tests), and the five settings test files whose board rows move down one: `src/ui/appearance_settings_tests.rs`, `src/ui/preview_settings_tests.rs`, `src/ui/settings_access_tests.rs`, `src/ui/text_settings_tests.rs`, `src/ui/theme_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The new tests of the key, its note and its steps; the fixture that sets every key gets the look; the key lists get `Key::Look` before `Key::Theme`. In the app's tests, the board's cursor starts on the `interface` row now: each walk to a row takes one `j` more, and the theme's tests start on the theme's row (`on_the_theme`). The preview's rows are found within the pixel the terminal look's zoom rounds them to, and the System tile's words follow the look's keys. About lists Plex's and the key symbols' notices on Linux too, and `tests/looks.rs` starts the app as it ships in each look the system can choose.

```diff
diff --git a/crates/tabletist-core/src/settings.rs b/crates/tabletist-core/src/settings.rs
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -3420,6 +3420,7 @@
             confirm_close_edits: false,
             idle_disconnect: Idle::Hours4,
             show_system_schemas: true,
+            look: Some(LookChoice::Omarchy),
             theme: Theme::Pinned("My \"Nord\".json".into()),
             interface_size: InterfaceSize::Large,
             font: Some("JetBrains Mono".into()),
@@ -3455,9 +3456,10 @@
 
     #[test]
     fn only_what_is_not_the_default_is_written_each_table_in_its_place() {
-        // The theme's note and the font's name the values of the look this
-        // platform draws; Windows has none for the font.
-        let note = Key::Theme.note().unwrap();
+        // The theme's note names the values of the look the file chooses,
+        // the terminal's; the font's those of the look this platform draws
+        // with none chosen, and Windows has none for the font.
+        let note = theme_note(true);
         let fonts = Key::Font
             .note()
             .map_or(String::new(), |note| format!("  # {note}"));
@@ -3515,6 +3517,7 @@
 notify = false
 
 [appearance]
+look = \"omarchy\"  # classic | omarchy
 theme = \"My \\\"Nord\\\".json\"  # {note}
 interface_size = \"large\"  # small | default | large
 font = \"JetBrains Mono\"{fonts}
@@ -3623,17 +3626,18 @@
                 (Key::McpEnabled, 46),
                 (Key::McpLogDays, 47),
                 (Key::McpNotify, 48),
-                (Key::Theme, 51),
-                (Key::InterfaceSize, 52),
-                (Key::Font, 53),
-                (Key::FontSize, 54),
-                (Key::RowHeight, 57),
-                (Key::Zebra, 58),
-                (Key::ColumnTypes, 59),
-                (Key::TableList, 60),
-                (Key::EnvBar, 63),
-                (Key::UpdatesCheck, 66),
-                (Key::UpdatesSkip, 67),
+                (Key::Look, 51),
+                (Key::Theme, 52),
+                (Key::InterfaceSize, 53),
+                (Key::Font, 54),
+                (Key::FontSize, 55),
+                (Key::RowHeight, 58),
+                (Key::Zebra, 59),
+                (Key::ColumnTypes, 60),
+                (Key::TableList, 61),
+                (Key::EnvBar, 64),
+                (Key::UpdatesCheck, 67),
+                (Key::UpdatesSkip, 68),
             ]
         );
     }
@@ -4732,6 +4736,7 @@
                 Key::ProductionReadOnly,
                 Key::ConfirmCloseEdits,
                 Key::IdleDisconnect,
+                Key::Look,
                 Key::Theme,
                 Key::InterfaceSize,
                 Key::Font,
@@ -4784,6 +4789,7 @@
         // row; the terminal look's has a row for each, and no interface
         // size.
         let sections = [
+            &OptionId::INTERFACE[..],
             &OptionId::THEME[..],
             &OptionId::TEXT[..],
             &[OptionId::FontSize],
@@ -5021,6 +5027,78 @@
         let loaded = Settings::from_toml("[appearance]\nrow_height = \"compact\"\n");
         assert_eq!(loaded.invalid, Vec::<usize>::new());
         assert_eq!(loaded.settings.row_height, RowHeight::Regular);
+    }
+
+    #[test]
+    fn the_look_is_read_and_written_with_its_note_and_a_value_it_cannot_have_ignored() {
+        let loaded = Settings::from_toml("[appearance]\nlook = \"omarchy\"\n");
+        assert_eq!(loaded.settings.look, Some(LookChoice::Omarchy));
+        assert_eq!(loaded.invalid, Vec::<usize>::new());
+        // First in its table, over the theme.
+        let settings = Settings {
+            look: Some(LookChoice::Classic),
+            theme: Theme::Dark,
+            ..Settings::default()
+        };
+        assert!(
+            settings.to_toml().ends_with(
+                "\n[appearance]\nlook = \"classic\"  # classic | omarchy\n\
+                 theme = \"dark\"  # system | light | dark\n"
+            ),
+            "{}",
+            settings.to_toml()
+        );
+        let loaded = Settings::from_toml("[appearance]\nlook = \"gnome\"\n");
+        assert_eq!(loaded.settings.look, None);
+        assert_eq!(loaded.invalid, vec![2]);
+    }
+
+    #[test]
+    fn the_theme_s_note_names_the_values_of_the_look_the_file_chooses() {
+        for (look, note) in [
+            (LookChoice::Omarchy, "omarchy | \"Tokyo Night\" | path"),
+            (LookChoice::Classic, "system | light | dark"),
+        ] {
+            let settings = Settings {
+                look: Some(look),
+                theme: Theme::Pinned("Nord".into()),
+                ..Settings::default()
+            };
+            let line = format!("theme = \"Nord\"  # {note}\n");
+            assert!(settings.to_toml().contains(&line), "{look:?}");
+        }
+    }
+
+    #[test]
+    fn the_look_steps_to_its_side_and_the_one_drawn_unset_stays_unset() {
+        let settings = Settings::default();
+        assert_eq!(
+            settings.stepped(OptionId::Look, false),
+            OptionValue::Look(Some(LookChoice::Classic))
+        );
+        assert_eq!(
+            settings.stepped(OptionId::Look, true),
+            OptionValue::Look(Some(LookChoice::Omarchy))
+        );
+        // The window flips the look as it shows it: set to the one drawn.
+        let shown = Settings {
+            look: Some(LookChoice::Omarchy),
+            ..Settings::default()
+        };
+        assert_eq!(
+            shown.flipped(OptionId::Look),
+            Some(OptionValue::Look(Some(LookChoice::Classic)))
+        );
+        // The look drawn, picked while nothing is set, is not set.
+        let omarchy = OptionValue::Look(Some(LookChoice::Omarchy));
+        assert_eq!(
+            omarchy.clone().for_look(&settings, true),
+            OptionValue::Look(None)
+        );
+        assert_eq!(omarchy.clone().for_look(&settings, false), omarchy);
+        // Set, it is the person's.
+        let classic = OptionValue::Look(Some(LookChoice::Classic));
+        assert_eq!(classic.clone().for_look(&shown, false), classic);
     }
 
     #[test]
diff --git a/src/ui/about.rs b/src/ui/about.rs
--- a/src/ui/about.rs
+++ b/src/ui/about.rs
@@ -273,8 +273,11 @@
         let shown = |harness: &Harness, words: &str| {
             harness.painted.iter().any(|(text, _)| text.contains(words))
         };
-        // This build's: JetBrains Mono on Linux, with its notice whole.
+        // This build's: JetBrains Mono, with its notice whole, and the
+        // faces of macOS's look, which Linux draws as its classic look.
         assert!(shown(&harness, "The JetBrains Mono Project Authors"));
+        assert!(shown(&harness, "IBM Corp."));
+        assert!(shown(&harness, "The Noto Project Authors"));
         assert!(shown(&harness, "SIL OPEN FONT LICENSE Version 1.1"));
         // A second press folds them, and so does a close.
         harness.click("Font licenses");
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
--- a/src/ui/appearance_settings_tests.rs
+++ b/src/ui/appearance_settings_tests.rs
@@ -462,9 +462,9 @@
     let key = |harness: &mut Harness, key: egui::Key, text: &str| {
         super::tests::type_key(harness, key, text);
     };
-    // Row height, under the theme and the text's two: one to the left,
-    // compact.
-    for _ in 0..3 {
+    // Row height, under the interface, the theme and the text's two: one
+    // to the left, compact.
+    for _ in 0..4 {
         key(&mut harness, egui::Key::J, "j");
     }
     key(&mut harness, egui::Key::H, "h");
@@ -548,7 +548,7 @@
     let mut harness = Harness::new();
     harness.set_look(Look::omarchy());
     on_the_pane(&mut harness);
-    for _ in 0..4 {
+    for _ in 0..5 {
         super::tests::type_key(&mut harness, egui::Key::J, "j");
     }
     change(&mut harness, |settings| settings.zebra = Some(true));
diff --git a/src/ui/preview_settings_tests.rs b/src/ui/preview_settings_tests.rs
--- a/src/ui/preview_settings_tests.rs
+++ b/src/ui/preview_settings_tests.rs
@@ -119,7 +119,10 @@
                     egui::pos2(rect.x1 as f32, rect.y1 as f32),
                 )
             })
-            .find(|rect| preview.contains_rect(*rect))
+            // Within the pixel the window's zoom rounds the two to: the
+            // terminal look's text size zooms its window, and the board's
+            // Interface section puts the preview on a fractional point.
+            .find(|rect| preview.expand(1.0).contains_rect(*rect))
             .expect("a row in the preview")
     };
     [row(1), row(2)]
diff --git a/src/ui/settings_access_tests.rs b/src/ui/settings_access_tests.rs
--- a/src/ui/settings_access_tests.rs
+++ b/src/ui/settings_access_tests.rs
@@ -324,6 +324,7 @@
         &mut harness,
         SettingsPane::Appearance,
         &[
+            (OptionId::Look, Some("classic is the system's own look")),
             (OptionId::Theme, Some("now: dark, from the system")),
             // The row has no control but its own: nothing is read from
             // fontconfig in a test.
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -299,7 +299,9 @@
         assert!(shows(&mut harness, words), "{words}");
     }
     let saves = settings_saves(&harness);
-    // The size, under the theme and the font: a step up, then two down.
+    // The size, under the interface, the theme and the font: a step up,
+    // then two down.
+    key(&mut harness, Key::J, "j");
     key(&mut harness, Key::J, "j");
     key(&mut harness, Key::J, "j");
     assert!(shows(&mut harness, "13"));
@@ -352,7 +354,9 @@
     let said = "now: fira code, from fontconfig";
     assert!(shows(&mut harness, said));
     assert_eq!(harness.painted_color(said), Some(harness.app.palette.dim));
-    // R puts the system's monospace back.
+    // R puts the system's monospace back, on the font's row under the
+    // interface's and the theme's.
+    key(&mut harness, Key::J, "j");
     key(&mut harness, Key::J, "j");
     harness.press(Key::R, Modifiers::SHIFT);
     assert_eq!(harness.app.settings.font, None);
diff --git a/src/ui/theme_settings_tests.rs b/src/ui/theme_settings_tests.rs
--- a/src/ui/theme_settings_tests.rs
+++ b/src/ui/theme_settings_tests.rs
@@ -7,10 +7,18 @@
 use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
 use crate::backend::Event;
 use crate::desktop_theme::{Pinned, Read};
+use crate::keymap::Layout;
 use crate::model::Action;
 use crate::settings::{Settings, Theme};
 use crate::testing::Harness;
 use crate::theme::{Catalog, CustomTheme, Look, Palette};
+
+/// Settings › Appearance with the board's cursor on the theme's row,
+/// under the interface's.
+fn on_the_theme(harness: &mut Harness) {
+    on_the_pane(harness);
+    super::tests::type_key(harness, egui::Key::J, "j");
+}
 
 /// Whether the control called `label` is told to a screen reader as set.
 fn set(harness: &mut Harness, label: &str) -> Option<bool> {
@@ -138,7 +146,7 @@
 #[test]
 fn l_pins_the_theme_omarchy_shows_and_h_follows_it_again() {
     let mut harness = following();
-    on_the_pane(&mut harness);
+    on_the_theme(&mut harness);
     let saves = settings_saves(&harness);
     key(&mut harness, egui::Key::L, "l");
     assert_eq!(
@@ -184,7 +192,7 @@
 #[test]
 fn a_pin_and_a_flip_in_one_frame_follow_omarchy_again() {
     let mut harness = following();
-    on_the_pane(&mut harness);
+    on_the_theme(&mut harness);
     harness.frame(vec![
         crate::testing::key(egui::Key::L, egui::Modifiers::NONE),
         egui::Event::Text("l".into()),
@@ -197,7 +205,7 @@
 #[test]
 fn with_no_omarchy_theme_to_name_pin_does_nothing() {
     let mut harness = terminal();
-    on_the_pane(&mut harness);
+    on_the_theme(&mut harness);
     assert!(shows(&mut harness, "now: dark, from the system"));
     assert!(shows(&mut harness, "tabletist's own light or dark"));
     let saves = settings_saves(&harness);
@@ -210,7 +218,7 @@
 fn a_value_of_macos_is_the_desktop_here_and_stays_in_the_file() {
     let mut harness = following();
     with_theme(&mut harness, Theme::Dark);
-    on_the_pane(&mut harness);
+    on_the_theme(&mut harness);
     assert_eq!(set(&mut harness, "Theme: follow omarchy"), Some(true));
     assert_eq!(harness.app.palette, omarchy_palette());
     // h, and a click on follow, change nothing the row shows: nothing.
@@ -307,12 +315,16 @@
 
 #[test]
 fn the_sheet_sets_the_theme_with_its_tiles() {
-    let hint = if cfg!(target_os = "windows") {
-        "System follows Windows."
-    } else {
-        "System follows macOS, including Auto."
-    };
     for look in [Look::macos(), Look::standard()] {
+        // What the System tile follows: the OS of the look's keys, and off
+        // Windows the desktop where they are Windows' (Linux's classic).
+        let hint = if cfg!(target_os = "windows") {
+            "System follows Windows."
+        } else if look.keys == Layout::Mac {
+            "System follows macOS, including Auto."
+        } else {
+            "System follows the desktop's light or dark."
+        };
         let mut harness = sheet(look);
         assert!(shows(&mut harness, "THEME"), "{}", look.name);
         assert!(shows(&mut harness, hint), "{}", look.name);
diff --git a/tests/looks.rs b/tests/looks.rs
new file mode 100644
--- /dev/null
+++ b/tests/looks.rs
@@ -0,0 +1,32 @@
+//! Each look this system can choose draws in the app as it ships. The
+//! app's own tests build every bundled face (`cfg(test)`), so a face a
+//! look names but this system's build lacks shows only here, where the
+//! app is built as it ships: the first label of that look would panic.
+
+use tabletist::app::App;
+use tabletist::backend::Backend;
+use tabletist::paths::AppDirs;
+use tabletist::settings::{LookChoice, Settings};
+
+#[test]
+fn each_look_this_system_can_choose_draws() {
+    let root = std::env::temp_dir().join(format!("tabletist-looks-{}", std::process::id()));
+    for choice in [None, Some(LookChoice::Classic), Some(LookChoice::Omarchy)] {
+        let dir = root.join(choice.map_or("none", LookChoice::name));
+        std::fs::create_dir_all(&dir).expect("a settings folder");
+        let settings = Settings {
+            look: choice,
+            ..Settings::default()
+        };
+        let mut app = App::new(AppDirs::at(&dir), settings.into(), Backend::recording());
+        let ctx = egui::Context::default();
+        app.attach(&ctx, false);
+        // The start draws the look the settings choose.
+        for _ in 0..3 {
+            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.frame_ui(ui));
+            // No renderer takes the textures.
+            output.textures_delta.clear();
+        }
+    }
+    let _ = std::fs::remove_dir_all(&root);
+}
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked --workspace --lib`
Expected: the core tests do not compile, which stops the run there; the app's test changes are checked in Step 4:

```
error[E0560]: struct `settings::Settings` has no field named `look`
error[E0425]: cannot find function `theme_note` in this scope
error[E0599]: no variant, associated function, or constant named `Look` found for enum `settings::Key` in the current scope
error[E0599]: no variant, associated function, or constant named `INTERFACE` found for enum `settings::OptionId` in the current scope
error[E0609]: no field `look` on type `settings::Settings`
error[E0599]: no variant, associated function, or constant named `Look` found for enum `settings::OptionId` in the current scope
error[E0599]: no variant, associated function, or constant named `Look` found for enum `settings::OptionValue` in the current scope
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/crates/tabletist-core/src/model.rs b/crates/tabletist-core/src/model.rs
--- a/crates/tabletist-core/src/model.rs
+++ b/crates/tabletist-core/src/model.rs
@@ -3525,8 +3525,10 @@
                 ("Data", &OptionId::DATA),
                 ("Safety", &OptionId::SAFETY),
             ],
+            // The look's control is beside the theme's tiles, in their
+            // section: the sheet has no room for another.
             Self::Appearance => &[
-                ("Theme", &OptionId::THEME),
+                ("Theme", &[OptionId::Look, OptionId::Theme]),
                 ("Text", &OptionId::TEXT),
                 ("Grid", &OptionId::GRID),
                 ("Environment color", &OptionId::ENVIRONMENT),
@@ -3573,13 +3575,18 @@
 
     /// What Reset to defaults puts back: the pane's options but the
     /// server's switch, which turned off would end every agent's session
-    /// for a question about defaults. The Editor pane's are all of its
+    /// for a question about defaults, and but the look (Appearance's first),
+    /// which would change the whole window under the person. The Editor pane's are all of its
     /// keys, the ones it has no row for yet among them.
     pub fn reset_options(self) -> &'static [crate::settings::OptionId] {
         use crate::settings::OptionId;
         match self {
             Self::General => &OptionId::GENERAL,
-            Self::Appearance => &OptionId::APPEARANCE,
+            // But the look: a question about defaults would change the
+            // whole window under the person.
+            Self::Appearance => OptionId::APPEARANCE
+                .split_first()
+                .map_or(&[], |(_, rest)| rest),
             Self::Editor => &OptionId::EDITOR,
             // Reset all… of its own, which asks first.
             Self::Keyboard => &[],
@@ -3606,6 +3613,7 @@
                 SettingsRow::Option(OptionId::IdleDisconnect),
             ],
             Self::Appearance => &[
+                SettingsRow::Option(OptionId::Look),
                 SettingsRow::Option(OptionId::Theme),
                 SettingsRow::Option(OptionId::Font),
                 SettingsRow::Option(OptionId::FontSize),
diff --git a/crates/tabletist-core/src/settings.rs b/crates/tabletist-core/src/settings.rs
--- a/crates/tabletist-core/src/settings.rs
+++ b/crates/tabletist-core/src/settings.rs
@@ -689,6 +689,10 @@
     pub idle_disconnect: Idle,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
     pub show_system_schemas: bool,
+    /// The look: the OS's classic one or the Omarchy experience. `None`
+    /// until the user sets it: the look the OS draws without one.
+    #[serde(skip)]
+    pub look: Option<LookChoice>,
     /// The theme each look draws. Older versions wrote a palette file's
     /// name here, which the terminal look still reads.
     #[serde(rename = "custom_theme")]
@@ -828,6 +832,7 @@
     McpEnabled,
     McpLogDays,
     McpNotify,
+    Look,
     Theme,
     InterfaceSize,
     Font,
@@ -845,7 +850,7 @@
     /// In the order the file has them: a table's keys together.
     /// An old key comes before the one that took its place: the new one
     /// is read over it.
-    pub const ALL: [Key; 43] = [
+    pub const ALL: [Key; 44] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -878,6 +883,7 @@
         Self::McpEnabled,
         Self::McpLogDays,
         Self::McpNotify,
+        Self::Look,
         Self::Theme,
         Self::InterfaceSize,
         Self::Font,
@@ -926,6 +932,7 @@
             Self::McpEnabled => ("mcp", "enabled"),
             Self::McpLogDays => ("mcp", "log_days"),
             Self::McpNotify => ("mcp", "notify"),
+            Self::Look => ("appearance", "look"),
             Self::Theme => ("appearance", "theme"),
             Self::InterfaceSize => ("appearance", "interface_size"),
             Self::Font => ("appearance", "font"),
@@ -969,6 +976,7 @@
             Self::Keywords => Some("upper | lower | keep"),
             Self::Commas => Some("trailing | leading"),
             Self::HistoryKeep => Some("0 keeps none"),
+            Self::Look => Some("classic | omarchy"),
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
             Self::EnvBar => Some("tint | label"),
@@ -981,15 +989,22 @@
             Self::Font if cfg!(target_os = "macos") => {
                 Some("\"IBM Plex Mono\" | \"JetBrains Mono\"")
             }
-            // The values of the look the platform draws: Linux has the
-            // terminal look, the others the system's light and dark.
-            Self::Theme => Some(if cfg!(target_os = "linux") {
-                "omarchy | \"Tokyo Night\" | path"
-            } else {
-                "system | light | dark"
-            }),
+            // The values of the look the platform draws with none chosen:
+            // Linux has the terminal look, the others the system's light
+            // and dark. A file that chooses one has its look's.
+            Self::Theme => Some(theme_note(cfg!(target_os = "linux"))),
             _ => None,
         }
+    }
+}
+
+/// The values the theme takes in the terminal look (`terminal`) or the
+/// others, as the file notes them.
+fn theme_note(terminal: bool) -> &'static str {
+    if terminal {
+        "omarchy | \"Tokyo Night\" | path"
+    } else {
+        "system | light | dark"
     }
 }
 
@@ -1022,6 +1037,8 @@
     LogDays,
     /// Whether a new agent is told of: `mcp.notify`.
     AgentsNotify,
+    /// The classic look or the Omarchy experience: `appearance.look`.
+    Look,
     Theme,
     /// macOS's and Windows's Interface size: `appearance.interface_size`.
     InterfaceSize,
@@ -1065,7 +1082,7 @@
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's, the Editor
     /// pane's, the Agents pane's and then the Updates pane's.
-    pub const ALL: [OptionId; 39] = [
+    pub const ALL: [OptionId; 40] = [
         Self::OnLaunch,
         Self::RestoreTabs,
         Self::PageSize,
@@ -1077,6 +1094,7 @@
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
+        Self::Look,
         Self::Theme,
         Self::InterfaceSize,
         Self::Font,
@@ -1135,11 +1153,14 @@
         Self::ConfirmCloseEdits,
         Self::IdleDisconnect,
     ];
-    /// The Appearance pane's, in the order it has them: its Theme
-    /// section's, its Text section's, its Grid section's, then its
-    /// Environment section's. Each look shows the Text section's its own
-    /// way ([`OptionId::TEXT`], [`OptionId::TERMINAL_TEXT`]).
-    pub const APPEARANCE: [OptionId; 9] = [
+    /// The Appearance pane's, in the order it has them: the look's (the
+    /// terminal look's Interface section; beside the theme's tiles on
+    /// macOS and Windows), its Theme section's, its Text section's, its
+    /// Grid section's, then its Environment section's. Each look shows the
+    /// Text section's its own way ([`OptionId::TEXT`],
+    /// [`OptionId::TERMINAL_TEXT`]).
+    pub const APPEARANCE: [OptionId; 10] = [
+        Self::Look,
         Self::Theme,
         Self::InterfaceSize,
         Self::Font,
@@ -1150,6 +1171,7 @@
         Self::TableList,
         Self::EnvBar,
     ];
+    pub const INTERFACE: [OptionId; 1] = [Self::Look];
     pub const THEME: [OptionId; 1] = [Self::Theme];
     /// macOS's and Windows's Text section: the interface's size, and the
     /// data font's row, which has the size's menu beside the font's.
@@ -1237,6 +1259,7 @@
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
+            Self::Look => Key::Look,
             Self::Theme => Key::Theme,
             Self::InterfaceSize => Key::InterfaceSize,
             Self::Font => Key::Font,
@@ -1289,6 +1312,8 @@
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
+    /// `None` is the OS's own look.
+    Look(Option<LookChoice>),
     Theme(ThemeChoice),
     InterfaceSize(InterfaceSize),
     /// `None` is the look's own font.
@@ -1347,6 +1372,12 @@
             {
                 Self::Keywords(None)
             }
+            // The look drawn while nothing is set is the OS's own.
+            Self::Look(Some(choice))
+                if settings.look.is_none() && choice == LookChoice::of(terminal) =>
+            {
+                Self::Look(None)
+            }
             other => other,
         }
     }
@@ -1367,6 +1398,7 @@
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
+            Self::Look(_) => OptionId::Look,
             Self::Theme(_) => OptionId::Theme,
             Self::InterfaceSize(_) => OptionId::InterfaceSize,
             Self::Font(_) => OptionId::Font,
@@ -1412,6 +1444,7 @@
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
+            Self::Look(choice) => settings.look = choice,
             Self::Theme(choice) => {
                 settings.theme = match choice {
                     ThemeChoice::Unset => Theme::Unset,
@@ -2081,6 +2114,7 @@
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
+            OptionId::Look => OptionValue::Look(self.look),
             OptionId::Theme => OptionValue::Theme(self.theme.choice()),
             OptionId::InterfaceSize => OptionValue::InterfaceSize(self.interface_size),
             OptionId::Font => OptionValue::Font(self.font.clone()),
@@ -2195,6 +2229,8 @@
                 OptionValue::LogDays(next.unwrap_or(days))
             }
             OptionId::AgentsNotify => OptionValue::AgentsNotify(forward),
+            // Classic at the left, Omarchy at the right.
+            OptionId::Look => OptionValue::Look(Some(LookChoice::of(forward))),
             // The terminal look's two: follow at the left, pin at the right.
             OptionId::Theme => OptionValue::Theme(if forward {
                 ThemeChoice::Pinned
@@ -2377,6 +2413,9 @@
                 let name = name.trim();
                 (!name.is_empty()).then(|| name.to_owned().into())
             }
+            (Key::Theme, _) if self.look.is_some() => {
+                self.look.map(|look| theme_note(look.terminal()).into())
+            }
             _ => key.note().map(Into::into),
         }
     }
@@ -2430,6 +2469,11 @@
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
+            // Unset, it is flipped from classic: the window flips the
+            // settings as it shows them, with the look drawn in.
+            OptionId::Look => Some(OptionValue::Look(Some(LookChoice::of(
+                !self.look.is_some_and(LookChoice::terminal),
+            )))),
             // As the terminal look draws it: a pin, or the desktop followed.
             OptionId::Theme => Some(OptionValue::Theme(match self.theme.pin() {
                 Some(_) => ThemeChoice::Omarchy,
@@ -2521,6 +2565,7 @@
             Key::McpEnabled => self.mcp_enabled.to_string(),
             Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::McpNotify => self.mcp_notify.to_string(),
+            Key::Look => toml_string(self.look?.name()),
             Key::Theme => toml_string(self.theme.text()?),
             Key::InterfaceSize => toml_string(self.interface_size.name()),
             Key::Font => toml_string(self.font.as_deref()?),
@@ -2871,6 +2916,9 @@
             Key::FontSize => points
                 .and_then(FontSize::of)
                 .map(|size| self.font_size = Some(size)),
+            Key::Look => text
+                .and_then(LookChoice::from_name)
+                .map(|choice| self.look = Some(choice)),
             Key::RowHeight => text
                 .and_then(RowHeight::from_name)
                 .map(|height| self.row_height = height),
@@ -3053,6 +3101,7 @@
             Key::McpEnabled => self.mcp_enabled = other.mcp_enabled,
             Key::McpLogDays => self.mcp_log_days = other.mcp_log_days,
             Key::McpNotify => self.mcp_notify = other.mcp_notify,
+            Key::Look => self.look = other.look,
             Key::Theme => self.theme = other.theme.clone(),
             Key::InterfaceSize => self.interface_size = other.interface_size,
             Key::Font => self.font = other.font.clone(),
@@ -3143,6 +3192,7 @@
             mcp_enabled: false,
             mcp_log_days: 30,
             mcp_notify: true,
+            look: None,
             row_height: RowHeight::Regular,
             zebra: None,
             column_types: true,
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -424,7 +424,8 @@
         };
         let writes_session = loaded_connections.problem.is_none();
         // An installed data font is read once the window exists (`attach`).
-        let look = crate::theme::Look::for_platform().with_text(&settings, false);
+        let look = crate::theme::Look::chosen(settings.look, crate::update::release::Os::current())
+            .with_text(&settings, false);
         let mut app = Self {
             dirs,
             settings,
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -1,8 +1,8 @@
 //! The font files behind each look, one egui family per weight.
 //!
 //! - Standard: Inter (fastframe's set-up), with egui's monospace for data.
-//! - macOS: IBM Plex Sans (400, 500, 600) and IBM Plex Mono (400, 500),
-//!   bundled: `plex-sans-400` ... `plex-mono-500`; JetBrains Mono (400,
+//! - macOS, and Linux's classic look: IBM Plex Sans (400, 500, 600) and
+//!   IBM Plex Mono (400, 500), bundled on both: `plex-sans-400` ... `plex-mono-500`; JetBrains Mono (400,
 //!   500), the other bundled face data can be drawn in: `jetbrains-mono-400`
 //!   and `jetbrains-mono-500`; and the installed font the settings name for
 //!   the data, where one was read ([`crate::font_files::installed_font`]):
@@ -27,15 +27,15 @@
 use super::{Kind, family_name};
 use crate::font_files::TerminalFaces;
 use crate::theme::{Look, Typeface};
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 use {super::data_family_name, crate::theme::DataFace};
 
 /// IBM Plex Sans, variable in weight and width (SIL OFL 1.1).
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 pub(crate) const PLEX_SANS: &[u8] = include_bytes!("../../assets/fonts/IBMPlexSans-Variable.ttf");
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 pub(crate) const PLEX_MONO: &[u8] = include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf");
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 pub(crate) const PLEX_MONO_MEDIUM: &[u8] =
     include_bytes!("../../assets/fonts/IBMPlexMono-Medium.ttf");
 /// Noto Sans Symbols 2 cut down to the keyboard symbols Plex lacks (⌘ ⌥ ⇧
@@ -43,7 +43,7 @@
 /// shortcuts (both SIL OFL 1.1, of the same authors). It keeps the line of
 /// the first cut, which the symbols' place on the baseline is tuned to.
 /// Neither has ⇥: the keymap writes that key as a word.
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 pub(crate) const KEYS: &[u8] = include_bytes!("../../assets/fonts/NotoSansSymbols2-Keys.ttf");
 /// JetBrains Mono, variable in weight (SIL OFL 1.1): Omarchy's default, for
 /// desktops without a monospace font of their own, and the other data face
@@ -53,14 +53,14 @@
     include_bytes!("../../assets/fonts/JetBrainsMono-Variable.ttf");
 
 /// A face's line box in ems: what epaint places its glyphs by.
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 #[derive(Clone, Copy)]
 struct Metrics {
     ascent: f32,
     descent: f32,
 }
 
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 impl Metrics {
     /// The baseline, below the middle of the line box.
     fn baseline(self) -> f32 {
@@ -69,13 +69,13 @@
 }
 
 /// IBM Plex's line box, Sans and Mono alike.
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 const PLEX_METRICS: Metrics = Metrics {
     ascent: 1.025,
     descent: 0.275,
 };
 /// The key symbols' line box: it reaches far deeper below the baseline.
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 const KEYS_METRICS: Metrics = Metrics {
     ascent: 1.069,
     descent: 0.63,
@@ -87,8 +87,9 @@
 /// centre on them: Plex's centres a little above its capitals. Inter's and
 /// JetBrains Mono's centre on theirs, and a desktop font's are not known.
 pub fn capitals(typeface: Typeface) -> Option<f32> {
-    // Off macOS the Plex look draws in Inter.
-    (typeface == Typeface::Plex && cfg!(any(target_os = "macos", test))).then_some(PLEX_CAPITALS)
+    // Windows, which bundles no Plex, never draws the Plex look.
+    (typeface == Typeface::Plex && cfg!(any(target_os = "macos", target_os = "linux", test)))
+        .then_some(PLEX_CAPITALS)
 }
 
 /// fastframe's families for Inter's named weights.
@@ -168,7 +169,7 @@
     );
 }
 
-#[cfg(any(target_os = "macos", test))]
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 fn plex(fonts: &mut FontDefinitions, installed: Option<&TerminalFaces>) {
     for weight in [400, 500, 600] {
         let name = family_name(Typeface::Plex, Kind::Sans, weight);
@@ -236,7 +237,7 @@
     }
 }
 
-#[cfg(not(any(target_os = "macos", test)))]
+#[cfg(not(any(target_os = "macos", target_os = "linux", test)))]
 fn plex(fonts: &mut FontDefinitions, _installed: Option<&TerminalFaces>) {
     inter(fonts);
 }
diff --git a/src/ui/about.rs b/src/ui/about.rs
--- a/src/ui/about.rs
+++ b/src/ui/about.rs
@@ -18,13 +18,14 @@
 
 /// The fonts this build bundles, each with its licence as the font's
 /// authors give it: the SIL Open Font License has its notice go with the
-/// font. Windows draws in fastframe's fonts, and bundles none of its own.
+/// font. Linux bundles macOS's faces for its classic look. Windows draws in
+/// fastframe's fonts, and bundles none of its own.
 const FONT_NOTICES: &[&str] = &[
-    #[cfg(target_os = "macos")]
+    #[cfg(any(target_os = "macos", target_os = "linux"))]
     include_str!("../../assets/fonts/IBMPlex-OFL.txt"),
     #[cfg(any(target_os = "macos", target_os = "linux"))]
     include_str!("../../assets/fonts/JetBrainsMono-OFL.txt"),
-    #[cfg(target_os = "macos")]
+    #[cfg(any(target_os = "macos", target_os = "linux"))]
     include_str!("../../assets/fonts/NotoSansSymbols2-OFL.txt"),
 ];
 
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -20,7 +20,7 @@
 use crate::keymap::{Command, Keymap, Scope};
 use crate::model::{Action, ConnectionForm, LoadedTables, SshAuthKind, SshHints, TestState};
 use crate::style::Icon;
-use crate::theme::{self, Look, Palette, Typeface};
+use crate::theme::{self, Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::widgets::{self, ButtonSpec};
 use crate::ui::{Paint, menu};
@@ -184,9 +184,11 @@
         ink
     }
 
-    /// Where saved passwords go, as this platform calls it.
+    /// Where saved passwords go, as this platform calls it: macOS's look
+    /// with the Mac's keys is on macOS (Linux's classic look has Windows'
+    /// keys, and the system keyring).
     fn keyring(&self) -> &'static str {
-        if self.look.faces.typeface == Typeface::Plex {
+        if self.look.keys == crate::keymap::Layout::Mac {
             "Keychain"
         } else {
             "Keyring"
@@ -196,7 +198,7 @@
     /// The name of the box that keeps the SSH secret: the database
     /// password's has the same text beside it.
     fn ssh_keyring(&self) -> &'static str {
-        if self.look.faces.typeface == Typeface::Plex {
+        if self.look.keys == crate::keymap::Layout::Mac {
             "Keychain for the SSH secret"
         } else {
             "Keyring for the SSH secret"
@@ -204,7 +206,7 @@
     }
 
     fn saved_hint(&self) -> &'static str {
-        if self.look.faces.typeface == Typeface::Plex {
+        if self.look.keys == crate::keymap::Layout::Mac {
             "Saved in the keychain"
         } else {
             "Saved in the keyring"
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -8,10 +8,10 @@
 use crate::app::App;
 use crate::connections::{ConnectionId, SavedConnection, SavedConnections};
 use crate::i18n::gettext;
-use crate::keymap::Command;
+use crate::keymap::{Command, Layout};
 use crate::model::{Action, ConnTabContent, PickerState};
 use crate::style::Icon;
-use crate::theme::{self, Look, Palette};
+use crate::theme::{Look, Palette};
 use crate::typography::{Text, TextRole};
 use crate::ui::menu;
 use crate::ui::states;
@@ -557,7 +557,7 @@
         }
         let secrets = if look.terminal {
             TERMINAL_SECRETS.to_owned()
-        } else if look.faces.typeface == theme::Typeface::Plex {
+        } else if look.keys == Layout::Mac {
             gettext(locale, "Passwords are stored in the macOS Keychain").into_owned()
         } else {
             gettext(locale, "Passwords are stored in the system keyring").into_owned()
@@ -586,7 +586,9 @@
     let found = saved.search(search);
     if saved.connections.is_empty() {
         // First launch: why the list is empty, and the one next step.
-        let keyring = if look.faces.typeface == theme::Typeface::Plex {
+        // macOS's look with the Mac's keys: on macOS (Linux's classic look
+        // has Windows' keys, and the system keyring).
+        let keyring = if look.keys == Layout::Mac {
             gettext(locale, "Tabletist stores passwords in the macOS Keychain.")
         } else {
             gettext(locale, "Tabletist stores passwords in the system keyring.")
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -28,8 +28,8 @@
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
     Autocomplete, Commas, EditorKeys, EnvBar, Idle, InterfaceSize, KeywordCase, Launch,
-    LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings, TableList,
-    Theme, ThemeChoice, Timestamps, UpdateCheck, Zone,
+    LineNumbers, LookChoice, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings,
+    TableList, Theme, ThemeChoice, Timestamps, UpdateCheck, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
@@ -50,6 +50,7 @@
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
+        OptionId::Look => "Interface",
         OptionId::Theme => "Theme",
         OptionId::InterfaceSize => "Interface size",
         OptionId::Font => "Data font",
@@ -131,6 +132,7 @@
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
+        | OptionId::Look
         | OptionId::Theme
         | OptionId::InterfaceSize
         | OptionId::TableList
@@ -168,6 +170,7 @@
         OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
         OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
         OptionId::EnvBar => Some("Production always uses the full tint"),
+        OptionId::Look => Some("Omarchy: keyboard-first, square, monospace"),
         OptionId::InterfaceSize => Some("Sidebar, tabs, buttons, inspector labels"),
         OptionId::Font => Some("Grid cells, SQL editor, JSON"),
         OptionId::NewTab => Some("The badge in each tab can still switch it"),
@@ -335,10 +338,11 @@
 /// force, off where a change made outside the app that turned it on is
 /// held, the grouping of numbers as the look has it where it is not set,
 /// the stripes and the font's size as the look draws them while they are
-/// not set, and the theme as the look reads it: a value of the other
-/// looks' is the desktop followed.
+/// not set, the look as it is drawn, and the theme as the look reads it:
+/// a value of the other looks' is the desktop followed.
 pub(super) fn shown(app: &App) -> Settings {
     let mut settings = app.settings.clone();
+    settings.look = Some(LookChoice::of(app.look.terminal));
     settings.mcp_enabled = app.agents_on();
     settings.group_digits = Some(app.settings.grouped(app.layout()));
     settings.zebra = Some(settings.zebra_in(app.look.terminal));
@@ -603,6 +607,16 @@
         OptionId::AgentsNotify => vec![
             (Word("off"), OptionValue::AgentsNotify(false)),
             (Word("on"), OptionValue::AgentsNotify(true)),
+        ],
+        OptionId::Look => vec![
+            (
+                Word("classic"),
+                OptionValue::Look(Some(LookChoice::Classic)),
+            ),
+            (
+                Word("omarchy"),
+                OptionValue::Look(Some(LookChoice::Omarchy)),
+            ),
         ],
         // The terminal look's: macOS and Windows draw their tiles.
         OptionId::Theme => vec![
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -13,8 +13,8 @@
 use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
     Autocomplete, Commas, EditorKeys, EnvBar, FontSizes, Idle, Indent, InterfaceSize, KeywordCase,
-    Launch, LineNumbers, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope, Settings,
-    TableList, ThemeChoice, Timestamps, UpdateCheck, Zone,
+    Launch, LineNumbers, LookChoice, Mode, NewTab, OptionId, OptionValue, RowHeight, RunScope,
+    Settings, TableList, ThemeChoice, Timestamps, UpdateCheck, Zone,
 };
 use crate::theme::{DataFace, Look, Palette, Typeface};
 use crate::typography::{Laid, Text, TextRole};
@@ -493,8 +493,23 @@
                     ui.add_space(2.0);
                 }
                 for option in options.iter().copied() {
-                    if option == OptionId::Theme {
-                        tiles(ui, settings, skin, actions);
+                    if option == OptionId::Look {
+                        // Beside the theme's tiles: see `tiles`.
+                    } else if option == OptionId::Theme {
+                        // A sheet too narrow to have the look's control
+                        // beside the tiles has its row under them.
+                        if !tiles(ui, settings, skin, actions) {
+                            row(
+                                ui,
+                                OptionId::Look,
+                                true,
+                                column,
+                                settings,
+                                skin,
+                                actions,
+                                None,
+                            );
+                        }
                     } else {
                         // The button after the one row that has one.
                         let beside = after
@@ -534,10 +549,12 @@
 }
 
 /// The Theme section: a tile for each mode, a radio group named by the
-/// option, the one set ringed in the accent, and what System does after
+/// option, the one set ringed in the accent, and what System does under
 /// them. Each tile draws a window in its palette: the sidebar's colour
-/// round the content's, System's half one and half the other.
-fn tiles(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
+/// round the content's, System's half one and half the other. The look's
+/// control is at the section's right, where the sheet is wide enough for
+/// it: false where it is not, and the control is not drawn.
+fn tiles(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) -> bool {
     let Skin { look, palette, .. } = *skin;
     let name = skin.say(label(OptionId::Theme));
     let label_role = TextRole::UiBody;
@@ -546,8 +563,10 @@
         TILE.x + 2.0 * TILE_PAD,
         TILE_PAD + TILE.y + 6.0 + line + TILE_PAD,
     );
-    let height = 6.0 + tile.y + 12.0 + 1.0;
+    let hint_line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
+    let height = 6.0 + tile.y + 4.0 + hint_line + 12.0 + 1.0;
     let ground = ui.painter().add(egui::Shape::Noop);
+    let look_ground = ui.painter().add(egui::Shape::Noop);
     let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
     widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
     // The tiles in a Ui of their own, whose node is their parent in the
@@ -652,18 +671,95 @@
             window.bottom() + 6.0 + line / 2.0,
         );
     }
-    let hint = if cfg!(target_os = "windows") {
+    let hint = theme_hint(look);
+    let hint = Text::one(look, TextRole::Secondary, &skin.say(hint), palette.dim).layout(ui.ctx());
+    let at = rect.left() + TILE_PAD;
+    if at + hint.width() <= rect.right() {
+        write(ui, at, top + tile.y + 4.0 + hint_line / 2.0, &hint);
+    }
+    let tiles_end = x - 12.0;
+    let beside = Rect::from_min_max(pos2(tiles_end + GAP, top), pos2(rect.right(), top + tile.y));
+    let drawn = look_beside(ui, beside, settings, skin, actions);
+    let row = rect.with_max_y(rect.bottom() - 1.0);
+    let theme_place = place.with_max_x(tiles_end);
+    asked(
+        ui,
+        ground,
+        (row, theme_place),
+        OptionId::Theme,
+        skin,
+        actions,
+    );
+    // The look's own ground: the block, not the tiles beside it.
+    if let Some(block) = drawn {
+        let ground = block.expand2(vec2(10.0, 8.0));
+        asked(
+            ui,
+            look_ground,
+            (ground, block),
+            OptionId::Look,
+            skin,
+            actions,
+        );
+    }
+    drawn.is_some()
+}
+
+/// What the System tile follows, under the tiles: the OS of the look's
+/// keys. Linux's classic look, macOS's with Windows' keys off Windows,
+/// follows the desktop.
+fn theme_hint(look: &Look) -> &'static str {
+    if cfg!(target_os = "windows") {
         "System follows Windows."
+    } else if look.keys == Layout::Mac {
+        "System follows macOS, including Auto."
     } else {
-        "System follows macOS, including Auto."
-    };
-    let hint = Text::one(look, TextRole::Secondary, &skin.say(hint), palette.dim).layout(ui.ctx());
-    let at = x - 12.0 + 8.0;
-    if at + hint.width() <= rect.right() {
-        write(ui, at, top + tile.y / 2.0, &hint);
-    }
-    let row = rect.with_max_y(rect.bottom() - 1.0);
-    asked(ui, ground, (row, place), OptionId::Theme, skin, actions);
+        "System follows the desktop's light or dark."
+    }
+}
+
+/// The look's control in `place`, beside the theme's tiles: its label over
+/// its small print, and its segments under them, at the place's right and
+/// in its middle. Where it is drawn; `None` where `place` is too narrow.
+fn look_beside(
+    ui: &mut Ui,
+    place: Rect,
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) -> Option<Rect> {
+    let Skin { look, palette, .. } = *skin;
+    let name = said(ui.ctx(), look, skin.locale, label(OptionId::Look));
+    let name = Text::one(look, TextRole::UiBody, &name, palette.text).layout(ui.ctx());
+    let description = small_print(OptionId::Look).map(|print| skin.say(print));
+    let print = description.as_deref().unwrap_or_default();
+    let print = Text::one(look, TextRole::Secondary, print, palette.dim).layout(ui.ctx());
+    let control = vec2(CONTROLS, SEGMENT + 4.0);
+    let width = name.width().max(print.width()).max(control.x).ceil();
+    if width > place.width() {
+        return None;
+    }
+    let height = name.height() + 2.0 + print.height() + 8.0 + control.y;
+    let min = pos2(place.right() - width, place.center().y - height / 2.0);
+    let block = Rect::from_min_size(min, vec2(width, height));
+    write(ui, min.x, min.y + name.height() / 2.0, &name);
+    let print_y = min.y + name.height() + 2.0;
+    write(ui, min.x, print_y + print.height() / 2.0, &print);
+    let at = Rect::from_min_size(pos2(min.x, print_y + print.height() + 8.0), control);
+    let builder = egui::UiBuilder::new()
+        .id_salt("look")
+        .max_rect(at)
+        .layout(egui::Layout::left_to_right(egui::Align::Center));
+    let mut child = ui.new_child(builder);
+    let helper = super::helper(OptionId::Look).map(|text| skin.say(text));
+    self::control(
+        &mut child,
+        (OptionId::Look, helper.as_deref()),
+        settings,
+        skin,
+        actions,
+    );
+    Some(block)
 }
 
 /// How wide the rows' labels have it: `LABEL`, or what the widest small
@@ -935,7 +1031,9 @@
         OptionId::PageSize | OptionId::Tz | OptionId::OnLaunch => look.control_height,
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
-        OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
+        OptionId::Look | OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => {
+            SEGMENT + 4.0
+        }
         OptionId::InterfaceSize => SEGMENT + 4.0,
         OptionId::Font | OptionId::FontSize => look.control_height,
         OptionId::NewTab
@@ -1261,6 +1359,16 @@
                 actions.push(Action::SetOption(OptionValue::TableList(TableList::of(
                     picked == 1,
                 ))));
+            }
+        }
+        // Classic is the look of the OS: macOS's on macOS and Linux, the
+        // standard look on Windows.
+        OptionId::Look => {
+            let words = [skin.say("Classic"), skin.say("Omarchy")];
+            let selected = usize::from(settings.look.is_some_and(LookChoice::terminal));
+            if let Some(picked) = segments(ui, (&name, description), &words, selected, skin) {
+                let choice = [LookChoice::Classic, LookChoice::Omarchy][picked];
+                actions.push(Action::SetOption(OptionValue::Look(Some(choice))));
             }
         }
         OptionId::EnvBar => {
@@ -1640,7 +1748,8 @@
         }
         // What the Appearance pane's rows say is their small print, and its
         // tiles have their own line.
-        OptionId::Theme
+        OptionId::Look
+        | OptionId::Theme
         | OptionId::OnLaunch
         | OptionId::RestoreTabs
         | OptionId::ProductionReadOnly
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -600,7 +600,8 @@
     );
     let mut index = 0;
     for (heading_text, top, options) in [
-        ("Theme", 8.0, &OptionId::THEME[..]),
+        ("Interface", 8.0, &OptionId::INTERFACE[..]),
+        ("Theme", 12.0, &OptionId::THEME[..]),
         ("Text", 12.0, &OptionId::TERMINAL_TEXT[..]),
         ("Grid", 12.0, &OptionId::GRID[..]),
         ("Environment", 12.0, &OptionId::ENVIRONMENT[..]),
@@ -1203,6 +1204,7 @@
         OptionId::Font => width(&font_value(settings, skin)),
         OptionId::Timestamps
         | OptionId::GroupDigits
+        | OptionId::Look
         | OptionId::Theme
         | OptionId::InterfaceSize
         | OptionId::RowHeight
@@ -1493,6 +1495,7 @@
         }
         OptionId::Timestamps
         | OptionId::GroupDigits
+        | OptionId::Look
         | OptionId::Theme
         | OptionId::InterfaceSize
         | OptionId::RowHeight
@@ -1709,6 +1712,7 @@
             )
             .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
         OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
+        OptionId::Look => dim(&skin.say("classic is the system's own look")),
         OptionId::Theme => theme_hint(skin),
         // macOS's and Windows's: no row of this look's.
         OptionId::InterfaceSize => Text::new(look),
```

- [ ] **Step 4: Run the checks**

Run the four commands of Task 1's Step 4.
Expected: all pass:

```
test result: ok. 2860 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in 18.95s
test result: ok. 1045 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 106.04s
```

- [ ] **Step 5: Commit**

```bash
git add crates/tabletist-core/src/settings.rs crates/tabletist-core/src/model.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/app.rs src/typography/fonts.rs src/ui/about.rs src/ui/picker.rs src/ui/connect_dialog/mod.rs tests/looks.rs src/ui/appearance_settings_tests.rs src/ui/preview_settings_tests.rs src/ui/settings_access_tests.rs src/ui/text_settings_tests.rs src/ui/theme_settings_tests.rs
git commit -m "Add the Interface setting: the classic look or the Omarchy one"
```

---

### Task 3: Switch the look live

A new choice, from either screen or an edit of the file, is drawn at once: the fonts of both looks are installed at the next frame's start (the terminal look's desktop font read first), and the frame after draws the new look in its palette, its style and its keymap. A change back before it lands draws every frame. Settings stays on Appearance with the keyboard on the look's control. Each look keeps its own defaults for what is not set, and each editor starts over where the switch changes its keybindings.

**Files:**
- Modify: `src/app.rs`, `src/style.rs`
- Create: `src/ui/look_switch_tests.rs`; register it in `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

```diff
diff --git a/src/ui/look_switch_tests.rs b/src/ui/look_switch_tests.rs
new file mode 100644
--- /dev/null
+++ b/src/ui/look_switch_tests.rs
@@ -0,0 +1,242 @@
+//! Settings › Appearance, its Interface: the OS's classic look or the
+//! Omarchy experience, chosen on either screen or in the file, and drawn
+//! from the next frames on.
+
+use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
+use crate::keymap::{Command, Layout};
+use crate::model::{Dialog, SettingsPane};
+use crate::settings::{LookChoice, OptionId, Settings};
+use crate::testing::Harness;
+use crate::theme::Look;
+use crate::update::release::Os;
+
+/// The settings changed by `change`, as an edit of the file changes them.
+fn change(harness: &mut Harness, change: impl FnOnce(&mut Settings)) {
+    let mut settings = harness.app.settings.clone();
+    change(&mut settings);
+    harness.app.apply_settings(settings);
+    harness.settle();
+}
+
+/// The settings file as the app last wrote it.
+fn file(harness: &Harness) -> &str {
+    &harness.app.settings_file.text
+}
+
+/// The pane Settings shows and its row the board's keys act on.
+fn shown_row(harness: &Harness) -> Option<(SettingsPane, usize)> {
+    match &harness.app.dialog {
+        Some(Dialog::Settings(dialog)) => Some((dialog.pane, dialog.row)),
+        _ => None,
+    }
+}
+
+/// Whether `harness` draws `look` as the settings choose it: its shape and
+/// its keys (the text sizes are the settings').
+fn draws(harness: &Harness, look: Look) -> bool {
+    let drawn = harness.app.look;
+    (drawn.name, drawn.terminal, drawn.keys) == (look.name, look.terminal, look.keys)
+}
+
+#[test]
+fn the_sheet_switches_to_the_omarchy_look_at_once() {
+    let mut harness = Harness::with_size(egui::vec2(1440.0, 1000.0));
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    // Beside the theme's tiles, with its small print.
+    assert!(shows(
+        &mut harness,
+        "Omarchy: keyboard-first, square, monospace"
+    ));
+    let saves = settings_saves(&harness);
+    harness.click("Interface: Omarchy");
+    harness.settle();
+    assert_eq!(harness.app.settings.look, Some(LookChoice::Omarchy));
+    assert!(file(&harness).contains("look = \"omarchy\"  # classic | omarchy\n"));
+    assert_eq!(settings_saves(&harness), saves + 1);
+    assert!(draws(&harness, Look::omarchy()));
+    assert_eq!(harness.app.layout(), Layout::Omarchy);
+    // The board, still on Appearance, its cursor on the interface's row.
+    assert_eq!(shown_row(&harness), Some((SettingsPane::Appearance, 0)));
+    assert!(shows(&mut harness, "classic is the system's own look"));
+}
+
+#[test]
+fn the_board_switches_to_the_classic_look_at_once() {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    // The cursor starts on the interface's row: h is classic.
+    assert_eq!(shown_row(&harness), Some((SettingsPane::Appearance, 0)));
+    super::tests::type_key(&mut harness, egui::Key::H, "h");
+    harness.settle();
+    assert_eq!(harness.app.settings.look, Some(LookChoice::Classic));
+    assert!(file(&harness).contains("look = \"classic\"  # classic | omarchy\n"));
+    let classic = Look::chosen(Some(LookChoice::Classic), Os::current());
+    assert!(draws(&harness, classic));
+    assert!(!harness.app.look.terminal);
+    // The sheet, on Appearance, with the keyboard on the look's control.
+    match &harness.app.dialog {
+        Some(Dialog::Settings(dialog)) => {
+            assert_eq!(dialog.pane, SettingsPane::Appearance);
+            assert_eq!(dialog.asked, Some(OptionId::Look));
+        }
+        other => panic!("{other:?}"),
+    }
+    assert!(shows(
+        &mut harness,
+        "Omarchy: keyboard-first, square, monospace"
+    ));
+    let focused = harness
+        .ctx
+        .memory(|memory| memory.focused())
+        .and_then(|id| harness.ctx.read_response(id))
+        .map(|response| response.rect);
+    let control = harness
+        .painted_rect("Classic")
+        .expect("the classic segment");
+    assert!(
+        focused.is_some_and(|rect| rect.intersects(control)),
+        "{focused:?} {control:?}"
+    );
+}
+
+#[test]
+fn an_edit_of_the_file_switches_the_look_and_back() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    change(&mut harness, |settings| {
+        settings.look = Some(LookChoice::Omarchy);
+    });
+    assert!(draws(&harness, Look::omarchy()));
+    // Taken out: the OS's own look.
+    change(&mut harness, |settings| settings.look = None);
+    assert!(draws(&harness, Look::chosen(None, Os::current())));
+}
+
+#[test]
+fn the_look_drawn_is_the_look_set_without_a_value() {
+    // Choosing what is drawn while nothing is set writes nothing.
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    on_the_pane(&mut harness);
+    let saves = settings_saves(&harness);
+    super::tests::type_key(&mut harness, egui::Key::L, "l");
+    assert_eq!(harness.app.settings.look, None);
+    assert!(!file(&harness).contains("look ="));
+    assert_eq!(settings_saves(&harness), saves);
+    assert!(draws(&harness, Look::omarchy()));
+}
+
+#[test]
+fn reset_to_defaults_keeps_the_look() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    // As the window sets them: written to the file.
+    harness.app.change_settings(|settings| {
+        settings.look = Some(LookChoice::Omarchy);
+        settings.column_types = false;
+    });
+    on_the_pane(&mut harness);
+    harness.app.apply(crate::model::Action::ResetSettings);
+    harness
+        .app
+        .apply(crate::model::Action::ConfirmResetSettings(true));
+    harness.settle();
+    assert!(harness.app.settings.column_types);
+    assert_eq!(harness.app.settings.look, Some(LookChoice::Omarchy));
+    assert!(draws(&harness, Look::omarchy()));
+}
+
+#[test]
+fn the_new_look_has_its_own_defaults_and_keeps_what_is_set() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    // Alternate rows unset: off on macOS, on in the terminal.
+    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
+    change(&mut harness, |settings| {
+        settings.look = Some(LookChoice::Omarchy);
+    });
+    assert!(harness.app.look.with_grid(&harness.app.settings).zebra);
+    // Set, it stays as set in either look.
+    change(&mut harness, |settings| {
+        settings.zebra = Some(false);
+        settings.look = Some(LookChoice::Classic);
+    });
+    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
+    change(&mut harness, |settings| {
+        settings.look = Some(LookChoice::Omarchy);
+    });
+    assert!(!harness.app.look.with_grid(&harness.app.settings).zebra);
+}
+
+#[test]
+fn the_classic_look_writes_its_keys_as_its_os_has_them() {
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    change(&mut harness, |settings| {
+        settings.look = Some(LookChoice::Classic);
+    });
+    let written = crate::ui::keys::written(&harness.ctx, &harness.app.look, Command::FindObject);
+    let written = written.as_str();
+    // ⌘ on macOS alone: Linux's classic look is macOS's with Ctrl.
+    if Os::current() == Os::MacOs {
+        assert!(written.contains('⌘'), "{written}");
+    } else {
+        assert!(!written.contains('⌘'), "{written}");
+        assert!(written.contains("Ctrl"), "{written}");
+    }
+}
+
+#[test]
+fn choosing_again_before_a_switch_lands_draws_every_frame() {
+    // The fonts of both looks are in place for the frame between: a change
+    // back, from either screen or the file, a frame after the first.
+    for (from, to) in [
+        (Look::macos(), LookChoice::Omarchy),
+        (Look::omarchy(), LookChoice::Classic),
+    ] {
+        let mut harness = Harness::new();
+        harness.set_look(from);
+        harness.settle();
+        let back = LookChoice::of(from.terminal);
+        harness.app.apply(crate::model::Action::SetOption(
+            crate::settings::OptionValue::Look(Some(to)),
+        ));
+        harness.frame(Vec::new());
+        let mut settings = harness.app.settings.clone();
+        settings.look = Some(back);
+        harness.app.apply_settings(settings);
+        harness.frame(Vec::new());
+        harness.frame(Vec::new());
+        harness.settle();
+        assert_eq!(harness.app.look.terminal, from.terminal, "{}", from.name);
+        // And on to the other once more, the frames in between drawn.
+        harness.app.apply(crate::model::Action::SetOption(
+            crate::settings::OptionValue::Look(Some(to)),
+        ));
+        harness.settle();
+        assert_eq!(harness.app.look.terminal, to.terminal(), "{}", from.name);
+    }
+}
+
+#[test]
+fn a_switch_that_changes_the_keybindings_starts_each_editor_over() {
+    // The Omarchy look's keys are vim's while none are set: a new tab
+    // types, in insert mode.
+    let mut harness = Harness::new();
+    harness.set_look(Look::omarchy());
+    let tab = harness.connect_fake();
+    harness.press(egui::Key::T, egui::Modifiers::COMMAND);
+    harness.settle();
+    let mode = |harness: &Harness| {
+        let workspace = harness.app.workspace(tab).expect("a workspace");
+        workspace.active_sql_tab().expect("an editor").vim.mode
+    };
+    assert_eq!(mode(&harness), crate::vim::Mode::Insert);
+    // The classic look's are the standard ones: vim starts over.
+    change(&mut harness, |settings| {
+        settings.look = Some(LookChoice::Classic);
+    });
+    assert_eq!(mode(&harness), crate::vim::Mode::Normal);
+}
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -98,6 +98,8 @@
 pub mod label_chooser;
 #[cfg(test)]
 mod label_tests;
+#[cfg(test)]
+mod look_switch_tests;
 pub mod menu;
 pub mod object_tabs;
 #[cfg(test)]
```

- [ ] **Step 2: Run them to see them fail**

Run: `~/.cargo/bin/cargo test --locked -p tabletist --lib ui::look_switch_tests`
Expected: the switches do not happen:

```
test ui::look_switch_tests::the_classic_look_writes_its_keys_as_its_os_has_them ... FAILED
test ui::look_switch_tests::the_new_look_has_its_own_defaults_and_keeps_what_is_set ... FAILED
test ui::look_switch_tests::an_edit_of_the_file_switches_the_look_and_back ... FAILED
test ui::look_switch_tests::choosing_again_before_a_switch_lands_draws_every_frame ... FAILED
test ui::look_switch_tests::reset_to_defaults_keeps_the_look ... FAILED
test ui::look_switch_tests::the_look_drawn_is_the_look_set_without_a_value ... ok
test ui::look_switch_tests::the_board_switches_to_the_classic_look_at_once ... FAILED
test ui::look_switch_tests::the_sheet_switches_to_the_omarchy_look_at_once ... FAILED
test ui::look_switch_tests::a_switch_that_changes_the_keybindings_starts_each_editor_over ... FAILED
test result: FAILED. 1 passed; 8 failed; 0 ignored; 0 measured; 2876 filtered out; finished in 0.08s
```

- [ ] **Step 3: Write the code**

```diff
diff --git a/src/app.rs b/src/app.rs
--- a/src/app.rs
+++ b/src/app.rs
@@ -95,6 +95,17 @@
     Reading,
     /// Nowhere to be found: the desktop is followed instead.
     Missing,
+}
+
+/// A look the settings chose, on its way to being drawn: egui puts fonts
+/// set in one frame in place at the next frame's start, so the look's are
+/// installed a frame before it is drawn.
+#[derive(Clone, Copy, Debug)]
+enum LookSwitch {
+    /// Its fonts are to be installed.
+    Asked(crate::theme::Look),
+    /// Its fonts are in place from this frame: it is drawn.
+    Installed(crate::theme::Look),
 }
 
 pub struct App {
@@ -255,6 +266,8 @@
     /// The look's text changed since the last `logic`: egui's own text
     /// styles follow it.
     restyle: bool,
+    /// A look the settings chose, on its way to being drawn.
+    look_switch: Option<LookSwitch>,
     /// The zoom the window was last given: the terminal look's text size,
     /// which sizes the whole window there.
     zoom: Option<f32>,
@@ -494,6 +507,7 @@
             font_read: false,
             fonts: egui::FontDefinitions::empty(),
             restyle: false,
+            look_switch: None,
             zoom: None,
             window_title: "Tabletist".into(),
             window_asks: Vec::new(),
@@ -5767,17 +5781,7 @@
         self.refresh_keymap();
         let terminal = self.look.terminal;
         if old.editor_keys_in(terminal) != self.settings.editor_keys_in(terminal) {
-            // Other keybindings: every editor in normal mode, as vim
-            // starts, its history starting at its script.
-            for tab in &mut self.tabs {
-                if let ConnTabContent::Workspace(workspace) = &mut tab.content {
-                    for sql in workspace.sql_tabs_mut() {
-                        sql.vim.to_normal();
-                        let text = sql.text.clone();
-                        sql.vim.start_over(&text);
-                    }
-                }
-            }
+            self.restart_editors();
         }
         if old.timestamps != self.settings.timestamps {
             let full = self.settings.timestamps == crate::settings::Timestamps::Full;
@@ -5844,6 +5848,20 @@
                     workspace.tree.flat = flat;
                 }
             }
+        }
+        // Another look than the one drawn or on its way: its fonts are
+        // installed at the next frame's start, and it is drawn from the
+        // frame after, with the keymap of its layout (`App::switch_look`).
+        let chosen =
+            crate::theme::Look::chosen(self.settings.look, crate::update::release::Os::current());
+        let coming = match self.look_switch {
+            Some(LookSwitch::Asked(look) | LookSwitch::Installed(look)) => look,
+            None => self.look,
+        };
+        if old.look != self.settings.look
+            && (chosen.name, chosen.keys) != (coming.name, coming.keys)
+        {
+            self.look_switch = Some(LookSwitch::Asked(chosen));
         }
         if old.theme != self.settings.theme {
             // Reading a theme needs the window: `logic` has it.
@@ -8223,6 +8241,76 @@
         })
     }
 
+    /// Takes the look the settings chose ([`App::apply_settings`]) a step
+    /// on, at the start of a frame: the fonts of the look drawn and of the
+    /// new one installed, which egui puts in place at the next frame's
+    /// start, and in that frame the new look drawn, in its palette and
+    /// style, its fonts alone set for the frames after, and the desktop's
+    /// theme read in `logic`. Each editor starts over where the look's
+    /// keybindings are others. Settings, open on Appearance, has the
+    /// keyboard on the look's row in the screen the new look draws.
+    fn switch_look(&mut self, ctx: &egui::Context) {
+        match self.look_switch.take() {
+            None => {}
+            // The fonts of both: the next frame draws the look now, or
+            // the new one where nothing chooses again before it lands.
+            Some(LookSwitch::Asked(next)) => {
+                // The terminal look's font read before its first frame, as
+                // a start reads it (`attach`): one that came later would
+                // move every line.
+                if next.terminal && !self.look.terminal && self.follow_desktop {
+                    self.desktop_font = crate::font_files::read_font(self.settings.font.as_deref());
+                }
+                let next = next.with_text(&self.settings, self.installed_drawn());
+                let desktop = self.desktop_font.faces.as_ref();
+                let looks = (&self.look, &next);
+                self.fonts = style::install_both(ctx, self.follow_desktop, looks, desktop);
+                self.look_switch = Some(LookSwitch::Installed(next));
+                ctx.request_repaint();
+            }
+            Some(LookSwitch::Installed(next)) => {
+                let was = self.look.terminal;
+                self.look = next;
+                self.look = self.text_look();
+                // The terminal's was read as its fonts were set.
+                if !self.look.terminal {
+                    self.read_desktop_font();
+                }
+                // Its fonts alone from the next frame on.
+                let desktop = self.desktop_font.faces.as_ref();
+                self.fonts = style::install_with(ctx, self.follow_desktop, &self.look, desktop);
+                if self.settings.editor_keys_in(was)
+                    != self.settings.editor_keys_in(self.look.terminal)
+                {
+                    self.restart_editors();
+                }
+                crate::ui::grid::forget_marks(ctx);
+                self.palette = self.resolve_palette();
+                self.apply_style(ctx);
+                self.theme_changed = true;
+                self.list_fonts();
+                if matches!(&self.dialog, Some(Dialog::Settings(dialog)) if dialog.pane == crate::model::SettingsPane::Appearance)
+                {
+                    self.apply(Action::ShowSetting(crate::settings::OptionId::Look));
+                }
+            }
+        }
+    }
+
+    /// Other keybindings: every editor in normal mode, as vim starts, its
+    /// history starting at its script.
+    fn restart_editors(&mut self) {
+        for tab in &mut self.tabs {
+            if let ConnTabContent::Workspace(workspace) = &mut tab.content {
+                for sql in workspace.sql_tabs_mut() {
+                    sql.vim.to_normal();
+                    let text = sql.text.clone();
+                    sql.vim.start_over(&text);
+                }
+            }
+        }
+    }
+
     fn resolve_palette(&self) -> Palette {
         theme::resolve(
             &self.look,
@@ -8495,6 +8583,8 @@
         // Before anything is drawn: a close request that pending changes
         // hold back is asked about in this frame.
         self.hold_close(ui.ctx());
+        // Before anything is drawn: a look the settings chose.
+        self.switch_look(ui.ctx());
         // Before anything names a key: the keymap its labels are read from,
         // made for the look's layout.
         if self.keymap_layout != Some(self.layout()) {
diff --git a/src/style.rs b/src/style.rs
--- a/src/style.rs
+++ b/src/style.rs
@@ -46,6 +46,25 @@
     ctx.set_fonts(fonts.clone());
     egui_extras::install_image_loaders(ctx);
     fastframe_icons::install::<Icon>(ctx);
+    fonts
+}
+
+/// The fonts of `look` and of `next` in one set, made and set as
+/// [`install_with`] sets one look's: what a look switch sets, which egui
+/// puts in place at the next frame's start, a frame that may draw either.
+pub fn install_both(
+    ctx: &egui::Context,
+    system_fallbacks: bool,
+    (look, next): (&Look, &Look),
+    desktop: Option<&TerminalFaces>,
+) -> egui::FontDefinitions {
+    let mut fonts = fastframe_fonts::FontSetup::default()
+        .system_fallbacks(system_fallbacks)
+        .definitions();
+    crate::typography::configure(&mut fonts, look, desktop);
+    crate::typography::configure(&mut fonts, next, desktop);
+    text_rendering().apply_to(&mut fonts);
+    ctx.set_fonts(fonts.clone());
     fonts
 }
 
```

- [ ] **Step 4: Run the checks**

Run the four commands of Task 1's Step 4.
Expected: all pass:

```
test result: ok. 2869 passed; 0 failed; 16 ignored; 0 measured; 0 filtered out; finished in 16.59s
test result: ok. 1045 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 3.61s
```

- [ ] **Step 5: Look at it**

Render the sheet and the board in a throwaway scene (`src/shots.rs`, `#[ignore]`, `--features shots`), on Appearance, before and after a switch each way, and compare by eye. Remove the scene; commit no picture.

- [ ] **Step 6: Commit**

```bash
git add src/app.rs src/style.rs src/ui/look_switch_tests.rs src/ui/mod.rs
git commit -m "Switch the look live when the Interface setting changes"
```

---

### Task 4: The docs

**Files:**
- Modify: `docs/index.md`, `docs/_guide/macos.md`, `docs/_guide/omarchy.md`, `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Write the pages**

```diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -5,8 +5,9 @@
 ---
 
 On a Mac, Tabletist has the macOS look: rounded controls, IBM Plex Sans
-for the interface and IBM Plex Mono for data. There is no setting that
-switches to another look.
+for the interface and IBM Plex Mono for data. **Settings › Appearance ›
+Interface**, beside the theme, switches to
+[the Omarchy look]({% link _guide/omarchy.md %}) and back, at once.
 
 {% include shot.html file="macos-table" alt="Tabletist in its macOS look, with two connections in the bar at the top" %}
 
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -6,8 +6,11 @@
 
 On Linux, Tabletist takes the Omarchy look: square corners, the keyboard
 first, vim keys, and the desktop's monospace font throughout. Every Linux
-desktop gets it. On macOS and Windows the app has a desktop look instead,
-and there is no setting that switches between them.
+desktop gets it until you choose otherwise. On macOS and Windows the app
+starts in its classic look. **Settings › Appearance › Interface** switches
+between the two at once, on every system: on Linux the classic look is the
+macOS one with its keys written with Ctrl, as Windows writes them. In this
+look the setting is the `interface` row, first in `appearance`.
 
 {% include shot.html file="omarchy-table" alt="Tabletist in its Omarchy look: a table filtered with a WHERE line, a row open beside it, and key hints along the bottom" width="2880" height="1800" %}
 
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -24,14 +24,19 @@
 | **Ask before closing with unsaved edits** | On or off. Off, closing a table, its connection or the window, or loading another page of it, discards its pending changes without asking | On |
 | **Disconnect when idle** | Never, or after 5, 15 or 30 minutes, 1 hour or 4 hours unused. The server session is closed; the tabs stay, and the connection connects again when something next needs it: a refresh, a page, a table opened, a run or a save | After 30 min |
 
-A second pane, **Appearance**, sets the theme, the text, and how the grids
-and the connection bar are drawn. Each change shows in every open window
+A second pane, **Appearance**, sets the look, the theme, the text, and how
+the grids and the connection bar are drawn. Each change shows in every open window
 at once, and in the preview under the options: two rows of a table under
 the bar of a dev connection, drawn as the grid and the bar are. Its rows
 are made in the app, so it needs no connection and reads no database.
 
+Where the pane says macOS and Windows, it means their look, which Linux has
+too when **Interface** is Classic; Omarchy means the Omarchy look on any
+system.
+
 | Option | Choices | Default |
 | --- | --- | --- |
+| **Interface** | Classic or Omarchy. Classic is the system's own look: macOS's on a Mac, Windows' on Windows, and on Linux macOS's with its keys written with Ctrl. The window changes at once, and Settings stays open on Appearance. In the classic look the control is beside the theme's tiles; in the Omarchy look it is the first row. Reset to defaults leaves it as it is | Omarchy on Linux, Classic on macOS and Windows |
 | **Theme** (macOS and Windows) | System, Light or Dark. System follows the system's light or dark as it changes, Auto on macOS too | System |
 | **Theme** (Omarchy) | Follow Omarchy, or pin the theme it shows now, which then stays when Omarchy switches. The row says the theme drawn, and under it the palette it gave | Follow Omarchy |
 | **Interface size** (macOS and Windows) | Small, Default or Large: the sidebar, tabs, buttons and the inspector's labels, from a body text of 12, 13 or 14 points | Default |
@@ -293,8 +298,9 @@
 | `editor.format.keywords` | `"upper"`, `"lower"` or `"keep"` | Not set: upper on macOS and Windows, lower in the Omarchy look |
 | `editor.format.commas` | `"trailing"` or `"leading"` | `"trailing"` |
 | `editor.history.keep` | 0 to 100000 statements kept for each connection, from every source; `0` keeps none. A smaller number deletes the oldest past it | `1000` |
+| `appearance.look` | `"classic"` or `"omarchy"`. Written once you set it; an edit of the file switches the window at once | Not set: the system's own look, the Omarchy look on Linux |
 | `appearance.interface_size` | `"small"`, `"default"` or `"large"`, on macOS and Windows | `"default"` |
-| `appearance.font` | On macOS the data's face, `"IBM Plex Mono"`, `"JetBrains Mono"` or the family name of an installed font (`"Menlo"`); `"monospace"` is IBM Plex Mono too, and a name no installed font has is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
+| `appearance.font` | On macOS the data's face, `"IBM Plex Mono"`, `"JetBrains Mono"` or the family name of an installed font (`"Menlo"`); `"monospace"` is IBM Plex Mono too, and a name no installed font has is drawn as IBM Plex Mono, which the window says. Linux's classic look draws `"IBM Plex Mono"` or `"JetBrains Mono"`. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
 | `appearance.font_size` | Points, whole or by halves (`13`, `12.5`). On macOS and Windows the data's, 11 to 16; in the Omarchy look all of it, 9 to 24. A size past a look's range is drawn at its nearest end. Written once you set it | Not set: 12.5 on macOS and Windows, 13 in the Omarchy look |
 | `appearance.theme` | On macOS and Windows `"system"`, `"light"` or `"dark"`. In the Omarchy look `"omarchy"`, or a theme to pin: an Omarchy theme's name (`"Tokyo Night"`), a palette file in the `themes` folder (`"Nord.json"`; its name alone where no Omarchy theme has it), or a path that starts with `/` or `~/` to a palette file or an Omarchy theme's folder. A value meant for the other looks is drawn as not set and stays in the file | Not set: the system's light or dark, or the Omarchy theme |
 | `appearance.grid.row_height` | `"compact"`, `"regular"` or `"roomy"` | `"regular"` |
diff --git a/docs/index.md b/docs/index.md
--- a/docs/index.md
+++ b/docs/index.md
@@ -71,7 +71,9 @@
 The picture above is the macOS look, and Windows gets a close relative of
 it. On Linux, Tabletist takes the Omarchy look: square, keyboard first,
 with vim keys and the desktop's monospace font throughout. On Omarchy it
-follows the desktop's theme and recolors when you switch themes.
+follows the desktop's theme and recolors when you switch themes. Each
+system can have the other look too: **Settings › Appearance › Interface**
+switches between the classic look and the Omarchy one at once.
 Read about [the Omarchy look]({% link _guide/omarchy.md %}) and
 [Tabletist on macOS]({% link _guide/macos.md %}).
 
```

- [ ] **Step 2: Commit**

```bash
git add docs/index.md docs/_guide/macos.md docs/_guide/omarchy.md docs/_reference/settings-and-files.md
git commit -m "Say how to switch between the classic look and the Omarchy one"
```

---

## Not in this run

- The Windows menu strip in the Omarchy look, the macOS menu bar with Omarchy keys, the Omarchy top bar beside macOS's window buttons, and the fonts and the Updates pane off Linux: run 2.
- The data font's note in the file still names the OS's values (`fontconfig name` on Linux), whichever look the file chooses.
- Linux's classic look lists no installed fonts for the data (the list is CoreText's, macOS's): its menu has the two bundled faces.
- Off Linux, the Omarchy look's 500 and 700 weights draw in Inter where the desktop's monospace is not read (no JetBrains Mono is bundled there): run 2's fonts.
