# Settings › Appearance, step 2: Theme Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Theme section to Settings › Appearance in both looks: System, Light and Dark tiles on macOS and Windows, and on Omarchy a theme that follows the desktop or is pinned, with the palette it gives. Each look reads its own values of `appearance.theme` and keeps the other looks' values in the file.

**Architecture:** `appearance.theme` becomes a `Theme` value in `Settings`, kept as the file says it. Each look reads it in its own way: `Theme::mode` gives System, Light or Dark for macOS and Windows, and `Theme::pin` gives the terminal look's pin. `theme::resolve` takes the look. The terminal look's pin takes one of three forms. A file name (`"Nord.json"`) is read from the themes folder (fastframe's `Catalog`, as today). A name (`"Tokyo Night"`) is an Omarchy theme, else a palette of the folder by that name. A path is a palette file or an Omarchy theme's folder. Names and paths are read on the backend by the new `desktop_theme` module (`Command::ReadTheme`), which also reads the name of Omarchy's current theme. The pane plugs into the window's existing machinery (`OptionId`, `OptionValue`, `SettingsPane`): tiles in `sheet.rs`, and a theme row and a palette row in `terminal.rs`.

**Tech Stack:** Rust, egui (crmne fork, `ViewportCommand::SetTheme`), fastframe-theme 0.4.1 (`Catalog`, `render_seed`, `parse_palette`), Omarchy's `omarchy-theme-color`, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-17-settings-appearance-design.md`, section "Step 2: Theme". This plan is step 2. Step 1 (Grid and Environment) is merged; steps 3 (Text and the font size keys) and 4 (Preview) get their own plans.

---

## Decided before code: how each look reads `appearance.theme`

The spec left one question open. Today `appearance.theme` names a palette file in the themes folder in every look. The step decides how that value, and the new ones, read on macOS and on Omarchy. The rule is "a value meant for the other look is ignored, and the file left as it is". This plan builds the reading below, **for your yes**:

| `appearance.theme` | macOS and Windows | The terminal look (every Linux desktop) |
|---|---|---|
| not set | System | follow the desktop |
| `"system"` | System | ignored: follows the desktop |
| `"light"`, `"dark"` | that mode, whatever the system says | ignored: follows the desktop |
| `"omarchy"` | ignored: System | follow the desktop |
| a file name: `"Nord.json"` | ignored: System | pinned: that palette file of the themes folder |
| a name: `"Tokyo Night"`, `"Nord"` | ignored: System | pinned: an Omarchy theme, else a palette of the themes folder whose name it is, ignoring case |
| a path: `"/…"`, `"~/…"` | ignored: System | pinned: a palette file, or an Omarchy theme's folder |

"Follow the desktop" is Omarchy's theme on Omarchy, and the system's light or dark on any other Linux desktop, as today. "Ignored" means the look draws its default and shows that default as chosen (the System tile, or "follow omarchy"). The value stays in `settings.toml` through every other write until the theme is changed in that look's Settings, or reset.

**What this withdraws:** today a palette file named in the file (`theme = "Nord.json"`) recolours macOS and Windows too, and `docs/_guide/macos.md` says "Your own colors work on a Mac too". With this reading macOS and Windows draw only their own light and dark: the board gives them three tiles, and the spec says "Only `Palette::light()` and `Palette::dark()`". Someone with such a line today gets the system's light or dark after the upgrade, without a word: no tile says it and nothing is told. The other way (B) keeps a palette file working there from the file. No tile would be lit, and a line under the tiles would name the file. That costs a fourth state in the sheet and a pin read on macOS and Windows too. Say which.

**Another Linux desktop:** the terminal look is every Linux desktop's. On GNOME or KDE it follows the system's light or dark, as today, and the row still says "follow omarchy · pin"; `pin` does nothing there (no Omarchy theme to name), and the hint says "now: dark, from the system". `"light"` and `"dark"` are macOS's and Windows's values, so they do not force a mode there. Two small questions: should `"light"` and `"dark"` force a mode in the terminal look where Omarchy is not running, and should the row say "follow the system" there? This plan builds neither.

## Where this departs from the boards and the spec

For a yes or a no before code:

| The board or the spec asks | Step 2 builds | Why |
|---|---|---|
| The Omarchy board's file pane writes `theme = "omarchy"` | `theme` is written once it is set, and `R` takes it out | Its default depends on the look and the writer does not know the look, as with `zebra` in step 1. |
| The Omarchy board draws the theme row following, "now: tokyo night · switches with omarchy-theme", and nothing else | Pinned: "now: Tokyo Night · pinned". A pin found nowhere: "“Gone” not found · following omarchy", in the warning colour. Another desktop: "now: dark, from the system". The palette row says "read from the omarchy theme", "read from {pin}", or "tabletist's own light or dark" | The board draws one state; these are the others. |
| "Pin a theme by name or path" | `pin` in the window pins the Omarchy theme of the moment, by its name. Any other name or path is set in the file (`ctrl+e`) | The board's row is two choices, follow omarchy and pin, with no field to type a name in. |
| (not drawn) | With no Omarchy theme to name (another desktop, or the name not read yet), `pin` does nothing | There is nothing to pin. The file still pins by name. |
| The palette row | Shown under the theme row; the cursor passes it | It has nothing to change. |
| "A pin that cannot be loaded falls back to following, said in the status line" | Said once in the status line, and on the theme row | The Settings screen covers the status line, and a pin is most often typed with the screen open. |
| The macOS board's hint "System follows macOS, including Auto." | The same on macOS; "System follows Windows." on Windows | Windows draws the sheet too, with the macOS board's words elsewhere (step 1). |
| The tiles' colours (#f4f3f0, #1e1e1c on the board) | Each tile is a window in its own palette: the sidebar's colour (`panel`) round the content's (`window`) | The spec: colours come from the palettes there are. |
| The board's note beside the key, `# omarchy \| "Tokyo Night" \| path` | That note on Linux, `# system \| light \| dark` on macOS and Windows | The note names the values of the look the platform draws. |
| The macOS subtitle | Still the title alone | Step 4, with the preview (as in step 1). |
| "Light and Dark force a palette" | They force the window's own parts too (its title bar, the system's menus and panels), through `ViewportCommand::SetTheme`; System gives them back | A dark window under a light title bar and light menus reads as broken. Seen only on a Mac by hand: the tests check the command sent. |
| (the theme's name, on the row) | Followed, Omarchy's name is lower-cased as the look writes words ("now: tokyo night"); pinned, the value is shown as the file says it ("now: Tokyo Night · pinned") | A pin can be a path, whose case matters. |

Found while drafting, fixed here: the Omarchy guide's Settings keys said Ctrl+H/L move between "General, Agents"; Appearance is between them since step 1.

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found. A pin found nowhere was said again whenever Omarchy switched, because the read of it landed while the themes folder was still being read. A late read of an earlier pin took the place of the newer pin's read. The status line's message was said at launch, when no status line is shown, and never again. `h`, or a click on "follow omarchy", wrote `"omarchy"` over a macOS value that the row showed as followed. A pin by the name of a theme that both Omarchy and the shared presets have (Tokyo Night) drew the preset's colours until the read came. The docs said a bare name pins a palette, but it is an Omarchy theme's first. Two smaller fixes: a name is lowered as Omarchy lowers it (ASCII letters only), and a path to something that is not a file (a pipe, a device) is not read.

The plan's second reviewer, reading it against its diffs, found one more: an Omarchy theme folder of the user's that overrides only some files of a shipped theme was "not found"; the colours now come from the first folder that has them, as `omarchy-theme-set` lays the two over each other.

Left as they are, for the record:
- A name pin is not handed to the themes folder's scan as the file to read first (only `"X.json"` is). In a folder past fastframe's 128 palettes, a name can miss where its file's name would not.
- A palette file that exists but does not parse is told as "not found".
- The themes folder is still scanned on macOS and Windows, where nothing reads it now, and rescanned when the theme setting changes there, a tile clicked too. A `"X.json"` value still has the scan read that file first.
- A pinned path, or an Omarchy theme's folder, is not watched: an edit of it shows at the next Omarchy switch, change of the themes folder or of the settings.
- In the demo, or with no home folder, a name or path pin is never read: it draws as the desktop followed, and the row says so.
- The hint "switches with omarchy-theme" is the board's own wording.

## Files

| File | What it does after this plan |
|---|---|
| `src/settings.rs` | `Theme` (the key as the file says it), `Mode`, `ThemeChoice`; `Settings::theme` in place of `custom_theme` (the old JSON's `custom_theme` still reads); the key's note; `OptionId::Theme` and `OptionValue::Theme`. |
| `src/theme.rs` | `PinFrom` (where a pin is read from), `catalog_pin`, and `resolve` by look. |
| `src/desktop_theme.rs` | New: Omarchy's current theme's name, and a pinned theme by name or path, read from files and `omarchy-theme-color`. |
| `src/lib.rs` | Declares it. |
| `src/backend.rs` | `Command::ReadTheme`, answered with `Event::Theme`. |
| `src/app.rs` | `PinState`; the pin drawn from the backend's read or the themes folder; the reads asked for; a pin found nowhere told once; a pin set from the window named; the window told the mode (`SetTheme`). |
| `src/model.rs` | The theme row first on the Appearance pane. |
| `src/ui/settings/{mod,sheet,terminal}.rs` | The section in both looks: tiles, and the theme and palette rows. |
| `src/testing.rs` | `Harness::system_theme`: the system's light or dark, as frames are told it. |
| `src/ui/theme_settings_tests.rs` | New: the section's headless tests. |
| `src/ui/appearance_settings_tests.rs`, `src/ui/mod.rs` | Step 1's tests move one row down; three helpers shared; the new file declared. |
| `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md` | The setting, the key's values, pinning. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The key, and what each look reads of it

**Files:**
- Modify: `src/settings.rs`, `src/theme.rs`, `src/app.rs`

- [ ] **Step 1: Write the failing tests**

The settings' tests take `theme` for `custom_theme`, read and write every value, and check what each look reads. The note beside the key depends on the platform, so the test asks the key for it. The theme's tests resolve by look. The app's test of a pin that changes runs in the terminal look: the harness draws the Windows look on every platform.

```diff
diff --git a/src/app.rs b/src/app.rs
index 37f535c..de6d9ee 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5794,6 +5794,8 @@ mod tests {
     #[test]
     fn a_new_theme_name_is_resolved_at_the_next_logic_pass() {
         let mut harness = Harness::new();
+        // A pin is the terminal look's, on every platform the tests run on.
+        harness.app.look = crate::theme::Look::omarchy();
         // A catalog that holds the theme already: no directory is read.
         let mut nord = Palette::dark();
         nord.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
@@ -5805,7 +5807,7 @@ mod tests {
             false,
         );
         let settings = Settings {
-            custom_theme: Some("Nord.json".into()),
+            theme: crate::settings::Theme::Pinned("Nord.json".into()),
             ..harness.app.settings.clone()
         };
         harness.app.apply_settings(settings);
diff --git a/src/settings.rs b/src/settings.rs
index 04bba94..e1c8a4e 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1041,7 +1041,7 @@ mod tests {
         assert!(!settings.group_digits);
         assert!(settings.value_tags);
         assert!(!settings.show_system_schemas);
-        assert_eq!(settings.custom_theme, None);
+        assert_eq!(settings.theme, Theme::Unset);
         assert_eq!(settings.sql_limit, 1_000);
         assert_eq!(settings.sql_timeout_secs, Some(30));
         assert!(!settings.mcp_enabled);
@@ -1098,7 +1098,7 @@ bar = \"tint\"  # tint | label
     #[test]
     fn a_theme_gets_its_table_and_no_timeout_is_zero() {
         let settings = Settings {
-            custom_theme: Some("My \"Nord\".json".into()),
+            theme: Theme::Pinned("My \"Nord\".json".into()),
             sql_timeout_secs: None,
             timestamps: Timestamps::Full,
             ..Settings::default()
@@ -1112,10 +1112,12 @@ bar = \"tint\"  # tint | label
             text.contains("sql_timeout_secs = 0  # 0 waits forever\n"),
             "{text}"
         );
+        // The note names the values of the look this platform draws.
+        let note = Key::Theme.note().unwrap();
         assert!(
-            text.contains(
-                "\n\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"\n\n[appearance.grid]\n"
-            ),
+            text.contains(&format!(
+                "\n\n[appearance]\ntheme = \"My \\\"Nord\\\".json\"  # {note}\n\n[appearance.grid]\n"
+            )),
             "{text}"
         );
     }
@@ -1136,7 +1138,7 @@ bar = \"tint\"  # tint | label
             group_digits: true,
             value_tags: false,
             show_system_schemas: true,
-            custom_theme: Some("My \"Nord\".json".into()),
+            theme: Theme::Pinned("My \"Nord\".json".into()),
             sql_limit: 100,
             sql_timeout_secs: None,
             mcp_enabled: true,
@@ -1453,7 +1455,7 @@ bar = \"tint\"  # tint | label
                     [editor]\nsql_limit = 100\n";
         let loaded = Settings::from_toml(text);
         assert_eq!(loaded.invalid, vec![4, 5, 6]);
-        assert_eq!(loaded.settings.custom_theme, None);
+        assert_eq!(loaded.settings.theme, Theme::Unset);
         assert_eq!(loaded.settings.page_size, 100);
         assert_eq!(loaded.settings.sql_limit, Settings::default().sql_limit);
         assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
@@ -1467,7 +1469,7 @@ bar = \"tint\"  # tint | label
         // The same on a last line with no break after it.
         let loaded = Settings::from_toml("[appearance]\ntheme = \"\"\"Nord");
         assert_eq!(loaded.invalid, vec![2]);
-        assert_eq!(loaded.settings.custom_theme, None);
+        assert_eq!(loaded.settings.theme, Theme::Unset);
     }
 
     #[test]
@@ -1478,7 +1480,7 @@ bar = \"tint\"  # tint | label
              [editor]\r\nsql_limit = 100\r\n\r\n\r\n",
         );
         assert_eq!(loaded.invalid, vec![4, 5, 6]);
-        assert_eq!(loaded.settings.custom_theme, None);
+        assert_eq!(loaded.settings.theme, Theme::Unset);
         assert_eq!(loaded.settings.page_size, 100);
         assert_eq!(loaded.lines, vec![(Key::PageSize, 2)]);
     }
@@ -1548,7 +1550,7 @@ bar = \"tint\"  # tint | label
         let expected = Settings {
             page_size: 100,
             show_system_schemas: true,
-            custom_theme: Some("Nord.json".into()),
+            theme: Theme::Pinned("Nord.json".into()),
             sql_limit: 100,
             sql_timeout_secs: None,
             ..Settings::default()
@@ -1924,4 +1926,71 @@ bar = \"tint\"  # tint | label
         // A reset leaves them to the look.
         assert_eq!(OptionId::Zebra.default_value(), OptionValue::Zebra(None));
     }
+
+    #[test]
+    fn each_theme_value_is_read_and_written_as_it_was() {
+        for (text, theme) in [
+            ("system", Theme::System),
+            ("light", Theme::Light),
+            ("dark", Theme::Dark),
+            ("omarchy", Theme::Omarchy),
+            ("Tokyo Night", Theme::Pinned("Tokyo Night".into())),
+            ("Nord.json", Theme::Pinned("Nord.json".into())),
+            ("~/themes/sunset", Theme::Pinned("~/themes/sunset".into())),
+            // A word in another case is a name, not the word.
+            ("Dark", Theme::Pinned("Dark".into())),
+        ] {
+            let loaded = Settings::from_toml(&format!("[appearance]\ntheme = {text:?}\n"));
+            assert_eq!(loaded.invalid, Vec::<usize>::new(), "{text}");
+            assert_eq!(loaded.settings.theme, theme, "{text}");
+            assert_eq!(theme.text(), Some(text));
+            let written = loaded.settings.to_toml();
+            assert!(
+                written.contains(&format!("theme = {text:?}  #")),
+                "{written}"
+            );
+            assert_eq!(Settings::from_toml(&written).settings.theme, theme);
+        }
+        // Unset, it is not written; an empty name or no text is ignored.
+        assert!(!Settings::default().to_toml().contains("theme"));
+        for line in ["theme = \"\"", "theme = 1"] {
+            let loaded = Settings::from_toml(&format!("[appearance]\n{line}\n"));
+            assert_eq!(loaded.invalid, vec![2], "{line}");
+            assert_eq!(loaded.settings.theme, Theme::Unset);
+        }
+    }
+
+    #[test]
+    fn each_look_reads_its_own_theme_values_and_follows_its_desktop_on_the_others() {
+        let pinned = Theme::Pinned("Tokyo Night".into());
+        // macOS and Windows: the three tiles; anything else is the system's.
+        for (theme, mode) in [
+            (Theme::Unset, Mode::System),
+            (Theme::System, Mode::System),
+            (Theme::Light, Mode::Light),
+            (Theme::Dark, Mode::Dark),
+            (Theme::Omarchy, Mode::System),
+            (pinned.clone(), Mode::System),
+        ] {
+            assert_eq!(theme.mode(), mode, "{theme:?}");
+        }
+        // The terminal look: a pin, or the desktop for anything else.
+        for (theme, pin) in [
+            (Theme::Unset, None),
+            (Theme::System, None),
+            (Theme::Light, None),
+            (Theme::Dark, None),
+            (Theme::Omarchy, None),
+            (pinned.clone(), Some("Tokyo Night")),
+        ] {
+            assert_eq!(theme.pin(), pin, "{theme:?}");
+        }
+        // A value the look does not read is kept through another change.
+        let mut settings = Settings::from_toml("[appearance]\ntheme = \"omarchy\"\n").settings;
+        settings.row_height = RowHeight::Roomy;
+        assert_eq!(
+            Settings::from_toml(&settings.to_toml()).settings.theme,
+            Theme::Omarchy
+        );
+    }
 }
diff --git a/src/theme.rs b/src/theme.rs
index b630341..f27241c 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -798,47 +798,99 @@ mod tests {
         assert!(!palette.set("no_such_colour", egui::Color32::RED));
     }
 
+    /// What the terminal look draws with `theme`, the pin read from the
+    /// catalog as the app reads one of the themes folder.
+    fn terminal(catalog: &Catalog, theme: &Theme, system: Option<egui::Theme>) -> Palette {
+        let pinned = theme.pin().and_then(|name| catalog_pin(catalog, name));
+        resolve(&Look::omarchy(), theme, pinned, catalog, system)
+    }
+
     #[test]
     fn without_a_choice_the_os_theme_decides() {
         let catalog = Catalog::preview(vec![nord()], false);
-        assert_eq!(
-            resolve(&catalog, None, Some(egui::Theme::Light)),
-            Palette::light()
-        );
-        assert_eq!(
-            resolve(&catalog, None, Some(egui::Theme::Dark)),
-            Palette::dark()
-        );
-        assert_eq!(resolve(&catalog, None, None), Palette::dark());
+        for look in Look::ALL {
+            let at = |system| resolve(&look, &Theme::Unset, None, &catalog, system);
+            assert_eq!(at(Some(egui::Theme::Light)), Palette::light());
+            assert_eq!(at(Some(egui::Theme::Dark)), Palette::dark());
+            assert_eq!(at(None), Palette::dark());
+        }
+    }
+
+    #[test]
+    fn macos_and_windows_draw_the_mode_the_settings_name() {
+        let catalog = Catalog::preview(vec![nord(), omarchy()], true);
+        for look in [Look::macos(), Look::standard()] {
+            let at = |theme: Theme, system| resolve(&look, &theme, None, &catalog, system);
+            let light = Some(egui::Theme::Light);
+            assert_eq!(at(Theme::Dark, light), Palette::dark());
+            assert_eq!(at(Theme::Light, Some(egui::Theme::Dark)), Palette::light());
+            assert_eq!(at(Theme::System, light), Palette::light());
+            // The terminal look's values are the system's here, Omarchy
+            // followed or not, a palette there or not.
+            assert_eq!(at(Theme::Omarchy, light), Palette::light());
+            assert_eq!(
+                at(Theme::Pinned("Nord.json".into()), light),
+                Palette::light()
+            );
+        }
     }
 
     #[test]
     fn omarchy_wins_over_the_os_theme_when_followed() {
         let catalog = Catalog::preview(vec![omarchy()], true);
-        assert_eq!(
-            resolve(&catalog, None, Some(egui::Theme::Dark)),
-            omarchy().palette
-        );
+        let dark = Some(egui::Theme::Dark);
+        for theme in [Theme::Unset, Theme::Omarchy, Theme::Light, Theme::Dark] {
+            assert_eq!(
+                terminal(&catalog, &theme, dark),
+                omarchy().palette,
+                "{theme:?}"
+            );
+        }
     }
 
     #[test]
-    fn a_chosen_theme_wins_over_everything() {
+    fn a_pinned_theme_wins_over_everything_in_the_terminal_look() {
         let catalog = Catalog::preview(vec![nord(), omarchy()], true);
-        assert_eq!(
-            resolve(&catalog, Some("Nord.json"), Some(egui::Theme::Light)),
-            nord().palette
-        );
+        let light = Some(egui::Theme::Light);
+        for name in ["Nord.json", "Nord", "nord"] {
+            let theme = Theme::Pinned(name.into());
+            assert_eq!(terminal(&catalog, &theme, light), nord().palette, "{name}");
+        }
+        // Omarchy's live palette is no theme to pin by its file's name.
+        let theme = Theme::Pinned("omarchy".into());
+        assert_eq!(catalog_pin(&catalog, "Omarchy"), None);
+        assert_eq!(terminal(&catalog, &theme, light), omarchy().palette);
     }
 
     #[test]
-    fn an_unknown_custom_theme_falls_back_to_the_system_theme() {
+    fn an_unknown_pin_falls_back_to_the_desktop() {
         let catalog = Catalog::preview(vec![nord()], false);
+        let theme = Theme::Pinned("Missing.json".into());
         assert_eq!(
-            resolve(&catalog, Some("Missing.json"), Some(egui::Theme::Light)),
+            terminal(&catalog, &theme, Some(egui::Theme::Light)),
             Palette::light()
         );
     }
 
+    #[test]
+    fn a_pin_is_read_from_where_its_name_says() {
+        assert_eq!(PinFrom::of("Nord.json"), PinFrom::File("Nord.json"));
+        assert_eq!(PinFrom::of("my.JSON"), PinFrom::File("my.JSON"));
+        assert_eq!(PinFrom::of("Tokyo Night"), PinFrom::Name("Tokyo Night"));
+        assert_eq!(PinFrom::of("/x/Nord.json"), PinFrom::Path("/x/Nord.json"));
+        assert_eq!(
+            PinFrom::of("~/themes/sunset"),
+            PinFrom::Path("~/themes/sunset")
+        );
+        // Neither a file of the folder nor a path the app can find.
+        assert_eq!(
+            PinFrom::of("themes/Nord.json"),
+            PinFrom::Name("themes/Nord.json")
+        );
+        assert_eq!(PinFrom::of("Nord.json").file(), Some("Nord.json"));
+        assert_eq!(PinFrom::of("Nord").file(), None);
+    }
+
     #[test]
     fn applying_a_palette_sets_egui_visuals() {
         let ctx = egui::Context::default();
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings::tests theme::tests`
Expected: the crate's tests do not compile:

```
error[E0425]: cannot find type `Theme` in this scope
   --> src/theme.rs:803:44
    |
803 |     fn terminal(catalog: &Catalog, theme: &Theme, system: Option<egui::Theme>) -> Palette {
...
error: could not compile `tabletist` (lib test) due to 81 previous errors
```

- [ ] **Step 3: Read the key as each look does**

`Theme` holds what the file says: the four words, or any other text as a pin. An empty name is an ignored line. `mode` is what macOS and Windows draw and `pin` is the terminal look's pin; each look takes the others' values as its own default.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index e1c8a4e..111f460 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -129,6 +129,88 @@ impl EnvBar {
     }
 }
 
+/// What `appearance.theme` says. macOS and Windows read `"system"`,
+/// `"light"` and `"dark"`; the terminal look reads `"omarchy"`, a theme's
+/// name and a path. A value meant for the other looks is drawn as the
+/// look's own default, and kept: the file says it as it did until the
+/// theme is changed in this look.
+#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
+#[serde(from = "Option<String>")]
+pub enum Theme {
+    /// No value: each look follows its desktop.
+    #[default]
+    Unset,
+    /// The system's light or dark, as it changes.
+    System,
+    Light,
+    Dark,
+    /// The Omarchy theme of the moment, as `omarchy-theme` switches it.
+    Omarchy,
+    /// A theme by name or path that stays when Omarchy's changes: what the
+    /// file said, as it said it.
+    Pinned(String),
+}
+
+/// What macOS and Windows draw: the system's light or dark, or one of the
+/// two whatever the system says.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum Mode {
+    System,
+    Light,
+    Dark,
+}
+
+impl Theme {
+    /// What the file's text says. An empty text says nothing.
+    pub fn of(text: &str) -> Self {
+        match text {
+            "" => Self::Unset,
+            "system" => Self::System,
+            "light" => Self::Light,
+            "dark" => Self::Dark,
+            "omarchy" => Self::Omarchy,
+            name => Self::Pinned(name.to_owned()),
+        }
+    }
+
+    /// The file's text: `None` for no value.
+    pub fn text(&self) -> Option<&str> {
+        Some(match self {
+            Self::Unset => return None,
+            Self::System => "system",
+            Self::Light => "light",
+            Self::Dark => "dark",
+            Self::Omarchy => "omarchy",
+            Self::Pinned(name) => name,
+        })
+    }
+
+    /// What macOS and Windows draw. A value of the terminal look's is the
+    /// system's.
+    pub fn mode(&self) -> Mode {
+        match self {
+            Self::Light => Mode::Light,
+            Self::Dark => Mode::Dark,
+            Self::Unset | Self::System | Self::Omarchy | Self::Pinned(_) => Mode::System,
+        }
+    }
+
+    /// The terminal look's pin: `None` follows the desktop. A value of
+    /// macOS and Windows follows it too.
+    pub fn pin(&self) -> Option<&str> {
+        match self {
+            Self::Pinned(name) => Some(name),
+            Self::Unset | Self::System | Self::Light | Self::Dark | Self::Omarchy => None,
+        }
+    }
+}
+
+impl From<Option<String>> for Theme {
+    fn from(text: Option<String>) -> Self {
+        text.map_or(Self::Unset, |text| Self::of(&text))
+    }
+}
+
 /// Everything the user can set. A key the file lacks takes its default and
 /// one this version does not know is ignored, so older and newer files both
 /// load. The serde derive reads the `settings.json` of versions before the
@@ -147,9 +229,10 @@ pub struct Settings {
     pub value_tags: bool,
     /// Show `pg_catalog`, `information_schema`, `mysql`, `sys` and friends.
     pub show_system_schemas: bool,
-    /// A palette file name in the themes directory. `None` follows the
-    /// desktop: Omarchy when present, else the OS light/dark setting.
-    pub custom_theme: Option<String>,
+    /// The theme each look draws. Older versions wrote a palette file's
+    /// name here, which the terminal look still reads.
+    #[serde(rename = "custom_theme")]
+    pub theme: Theme,
     /// Rows a SQL editor statement keeps (the Limit menu).
     pub sql_limit: u32,
     /// Seconds before a SQL editor run is cancelled; `None` waits forever.
@@ -250,6 +333,13 @@ impl Key {
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
             Self::EnvBar => Some("tint | label"),
+            // The values of the look the platform draws: Linux has the
+            // terminal look, the others the system's light and dark.
+            Self::Theme => Some(if cfg!(target_os = "linux") {
+                "omarchy | \"Tokyo Night\" | path"
+            } else {
+                "system | light | dark"
+            }),
             _ => None,
         }
     }
@@ -762,7 +852,7 @@ impl Settings {
             Key::McpEnabled => self.mcp_enabled.to_string(),
             Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::McpNotify => self.mcp_notify.to_string(),
-            Key::Theme => toml_string(self.custom_theme.as_deref()?),
+            Key::Theme => toml_string(self.theme.text()?),
             Key::RowHeight => toml_string(self.row_height.name()),
             Key::Zebra => self.zebra?.to_string(),
             Key::ColumnTypes => self.column_types.to_string(),
@@ -843,7 +933,10 @@ impl Settings {
             Key::McpEnabled => flag.map(|flag| self.mcp_enabled = flag),
             Key::McpLogDays => number.map(|number| self.mcp_log_days = number),
             Key::McpNotify => flag.map(|flag| self.mcp_notify = flag),
-            Key::Theme => text.map(|name| self.custom_theme = Some(name.to_owned())),
+            // An empty name names nothing: the line is ignored.
+            Key::Theme => text
+                .filter(|text| !text.is_empty())
+                .map(|text| self.theme = Theme::of(text)),
             Key::RowHeight => text
                 .and_then(RowHeight::from_name)
                 .map(|height| self.row_height = height),
@@ -1014,7 +1107,7 @@ impl Default for Settings {
             group_digits: false,
             value_tags: true,
             show_system_schemas: false,
-            custom_theme: None,
+            theme: Theme::Unset,
             sql_limit: 1_000,
             sql_timeout_secs: Some(30),
             mcp_enabled: false,
```

- [ ] **Step 4: Resolve the palette by look**

`PinFrom` tells a pin's form: a palette file of the folder, a path, or a name. `catalog_pin` finds a palette of the folder by its file or its name, never Omarchy's live palette. macOS and Windows draw the mode; the terminal look draws the pin, else Omarchy, else the system's.

```diff
diff --git a/src/theme.rs b/src/theme.rs
index f27241c..6d98ec9 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -3,7 +3,7 @@
 
 use egui::{Color32, CornerRadius, Stroke, Vec2};
 
-use crate::settings::RowHeight;
+use crate::settings::{Mode, RowHeight, Theme};
 
 /// A palette file from the themes directory.
 pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
@@ -536,21 +536,92 @@ pub fn mix(base: Color32, color: Color32, amount: f32) -> Color32 {
     base.lerp_to_gamma(color, amount)
 }
 
-/// Picks the palette to draw with: the user's chosen file, else Omarchy's
-/// live palette when the desktop is Omarchy, else the OS light/dark setting.
-pub fn resolve(catalog: &Catalog, custom: Option<&str>, system: Option<egui::Theme>) -> Palette {
-    if let Some(theme) = custom.and_then(|name| catalog.find(name)) {
-        return theme.palette.with_readable_labels();
+/// Where a pinned theme is read from, as its name is written.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum PinFrom<'a> {
+    /// `Nord.json`: a palette file in the themes folder, as older versions
+    /// named one.
+    File(&'a str),
+    /// `/…` or `~/…`: a palette file, or an Omarchy theme's folder.
+    Path(&'a str),
+    /// `Tokyo Night`: an Omarchy theme, else a palette of the themes folder
+    /// by its name.
+    Name(&'a str),
+}
+
+impl<'a> PinFrom<'a> {
+    pub fn of(name: &'a str) -> Self {
+        if name.starts_with('/') || name.starts_with("~/") {
+            Self::Path(name)
+        } else if !name.contains(['/', '\\'])
+            && name
+                .rsplit_once('.')
+                .is_some_and(|(_, extension)| extension.eq_ignore_ascii_case("json"))
+        {
+            Self::File(name)
+        } else {
+            Self::Name(name)
+        }
+    }
+
+    /// The palette file the themes folder is to read first, for a pin
+    /// that names one.
+    pub fn file(self) -> Option<&'a str> {
+        match self {
+            Self::File(file) => Some(file),
+            Self::Path(_) | Self::Name(_) => None,
+        }
+    }
+}
+
+/// The palette of the themes folder a pin names: its file, or a palette
+/// called `name` whatever its case (`tokyo night` is `Tokyo Night.json`).
+/// Never Omarchy's live palette, which is no theme of its own.
+pub fn catalog_pin(catalog: &Catalog, name: &str) -> Option<Palette> {
+    let named = |theme: &&CustomTheme| {
+        theme.filename != fastframe_theme::omarchy::FILENAME
+            && (theme.filename == name
+                || fastframe_theme::display_name(&theme.filename).to_lowercase()
+                    == name.to_lowercase())
+    };
+    catalog
+        .themes()
+        .iter()
+        .find(named)
+        .map(|theme| theme.palette)
+}
+
+/// Picks the palette to draw with. macOS and Windows draw light or dark:
+/// the system's, or the one `theme` names. The terminal look draws the
+/// pinned theme where it was read (`pinned`), else Omarchy's live palette
+/// when the desktop is Omarchy, else the system's light or dark.
+pub fn resolve(
+    look: &Look,
+    theme: &Theme,
+    pinned: Option<Palette>,
+    catalog: &Catalog,
+    system: Option<egui::Theme>,
+) -> Palette {
+    let system = match system {
+        Some(egui::Theme::Light) => Palette::light(),
+        _ => Palette::dark(),
+    };
+    if !look.terminal {
+        return match theme.mode() {
+            Mode::System => system,
+            Mode::Light => Palette::light(),
+            Mode::Dark => Palette::dark(),
+        };
+    }
+    if let Some(palette) = pinned {
+        return palette.with_readable_labels();
     }
     if catalog.follows_omarchy()
         && let Some(theme) = catalog.system_theme()
     {
         return theme.palette.with_readable_labels();
     }
-    match system {
-        Some(egui::Theme::Light) => Palette::light(),
-        _ => Palette::dark(),
-    }
+    system
 }
 
 /// Adds the shared presets and, on Omarchy, the desktop's live palette.
```

- [ ] **Step 5: Draw it in the app**

The themes folder reads a pinned palette file first, as it read the old name, in every look. Only the terminal look draws a pin.

```diff
diff --git a/src/app.rs b/src/app.rs
index de6d9ee..0c79639 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -3749,7 +3749,7 @@ impl App {
                 }
             }
         }
-        if old.custom_theme != self.settings.custom_theme {
+        if old.theme != self.settings.theme {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
         }
@@ -5181,7 +5181,7 @@ impl App {
             let repaint = ctx.clone();
             self.themes.start(
                 self.dirs.themes_dir(),
-                self.settings.custom_theme.clone(),
+                self.palette_file(),
                 &fastframe_theme::Waker::new(move || repaint.request_repaint()),
             );
             // Not in tests or the demo, which must not watch the user's
@@ -5193,10 +5193,28 @@ impl App {
         theme::apply(ctx, &self.palette, &self.look);
     }
 
+    /// The palette file a pin names, which the themes folder reads first.
+    fn palette_file(&self) -> Option<String> {
+        let pin = self.settings.theme.pin()?;
+        theme::PinFrom::of(pin).file().map(str::to_owned)
+    }
+
+    /// The pinned theme's palette, where the terminal look has one: read
+    /// from the themes folder.
+    fn pinned_palette(&self) -> Option<Palette> {
+        if !self.look.terminal {
+            return None;
+        }
+        let pin = self.settings.theme.pin()?;
+        theme::catalog_pin(&self.themes, pin)
+    }
+
     fn resolve_palette(&self) -> Palette {
         theme::resolve(
+            &self.look,
+            &self.settings.theme,
+            self.pinned_palette(),
             &self.themes,
-            self.settings.custom_theme.as_deref(),
             self.system_theme,
         )
     }
@@ -5211,7 +5229,7 @@ impl App {
             let repaint = ctx.clone();
             self.themes.start(
                 self.dirs.themes_dir(),
-                self.settings.custom_theme.clone(),
+                self.palette_file(),
                 &fastframe_theme::Waker::new(move || repaint.request_repaint()),
             );
         }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings::tests theme::tests app::tests::a_new_theme`
Expected: PASS. Then the four checks above.

- [ ] **Step 7: Commit**

```bash
git add src/settings.rs src/theme.rs src/app.rs
git commit -m "Read appearance.theme as each look does, and keep a value meant for another"
```

---

### Task 2: Omarchy's theme of the moment, and a pin by name or path

**Files:**
- Create: `src/desktop_theme.rs`
- Modify: `src/lib.rs`, `src/backend.rs`, `src/app.rs`, `src/ui/settings/mod.rs`

- [ ] **Step 1: Write the failing tests**

What the app does with the backend's read. A pin is drawn once read, and an earlier pin's read is ignored, even one that comes late. A name no Omarchy theme has is taken from the themes folder, but only once read. A pin found nowhere is told once, in a status line that is shown, and not again when Omarchy switches. Reads are asked for in the terminal look only, and never in tests or the demo.

```diff
diff --git a/src/app.rs b/src/app.rs
index 0c79639..ff9b03e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -5838,6 +5838,204 @@ mod tests {
         assert!(!harness.app.themes.loading());
     }
 
+    /// A harness in the terminal look, following an Omarchy palette, with
+    /// Nord among the themes folder's.
+    fn following_omarchy() -> (Harness, Palette, Palette) {
+        let mut harness = Harness::new();
+        harness.app.look = crate::theme::Look::omarchy();
+        let mut nord = Palette::dark();
+        nord.accent = egui::Color32::from_rgb(0x88, 0xc0, 0xd0);
+        let mut omarchy = Palette::dark();
+        omarchy.accent = egui::Color32::from_rgb(0xf3, 0x8d, 0x70);
+        let theme = |filename: &str, palette| crate::theme::CustomTheme {
+            filename: filename.into(),
+            palette,
+        };
+        harness.app.themes = Catalog::preview(
+            vec![
+                theme("Nord.json", nord),
+                theme(fastframe_theme::omarchy::FILENAME, omarchy),
+            ],
+            true,
+        );
+        (harness, nord, omarchy)
+    }
+
+    fn pin(harness: &mut Harness, name: &str) {
+        let settings = Settings {
+            theme: crate::settings::Theme::Pinned(name.into()),
+            ..harness.app.settings.clone()
+        };
+        harness.app.apply_settings(settings);
+        harness.app.logic(&harness.ctx.clone());
+    }
+
+    /// What the backend read of the pin `name`.
+    fn read(harness: &mut Harness, name: &str, palette: Option<Palette>) {
+        let read = crate::desktop_theme::Read {
+            current: Some("Ristretto".into()),
+            pin: Some((
+                name.into(),
+                palette.map(|palette| crate::desktop_theme::Pinned {
+                    palette,
+                    omarchy: true,
+                }),
+            )),
+        };
+        harness.app.apply(Action::Backend(Event::Theme(read)));
+        harness.app.logic(&harness.ctx.clone());
+    }
+
+    #[test]
+    fn a_pin_the_backend_read_is_drawn_and_the_desktop_until_then() {
+        let (mut harness, _, omarchy) = following_omarchy();
+        pin(&mut harness, "Tokyo Night");
+        assert_eq!(harness.app.pin_state(), Some(PinState::Reading));
+        assert_eq!(harness.app.palette, omarchy);
+        let mut tokyo = Palette::dark();
+        tokyo.accent = egui::Color32::from_rgb(0x7a, 0xa2, 0xf7);
+        read(&mut harness, "Tokyo Night", Some(tokyo));
+        assert_eq!(harness.app.palette, tokyo.with_readable_labels());
+        assert!(matches!(harness.app.pin_state(), Some(PinState::Drawn(_))));
+        assert_eq!(
+            harness.app.desktop_theme.current.as_deref(),
+            Some("Ristretto")
+        );
+        // A read of an earlier pin is not this one's.
+        pin(&mut harness, "Sunset");
+        assert_eq!(harness.app.palette, omarchy);
+        assert_eq!(harness.app.pin_state(), Some(PinState::Reading));
+        // macOS and Windows have no pin.
+        harness.app.look = crate::theme::Look::macos();
+        assert_eq!(harness.app.pin_state(), None);
+    }
+
+    #[test]
+    fn a_name_no_omarchy_theme_has_is_a_palette_of_the_themes_folder() {
+        let (mut harness, nord, omarchy) = following_omarchy();
+        pin(&mut harness, "nord");
+        // Omarchy may have a theme of that name: the desktop until it is
+        // read.
+        assert_eq!(harness.app.palette, omarchy);
+        read(&mut harness, "nord", None);
+        assert_eq!(harness.app.palette, nord);
+        assert!(
+            matches!(harness.app.pin_state(), Some(PinState::Drawn(pinned)) if !pinned.omarchy)
+        );
+    }
+
+    #[test]
+    fn a_pin_found_nowhere_follows_the_desktop_and_is_said_once() {
+        let (mut harness, _, omarchy) = following_omarchy();
+        pin(&mut harness, "Gone");
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx),
+            None,
+            "not while it is read"
+        );
+        read(&mut harness, "Gone", None);
+        assert_eq!(harness.app.palette, omarchy);
+        assert_eq!(harness.app.pin_state(), Some(PinState::Missing));
+        // Connections have no status line: it waits for a workspace's.
+        assert_eq!(crate::ui::toast::said(&harness.ctx), None);
+        harness.connect_fake();
+        harness.app.logic(&harness.ctx.clone());
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx).as_deref(),
+            Some("theme “Gone” not found: following omarchy")
+        );
+        // Read again, it is not said again.
+        crate::ui::toast::say(&harness.ctx, "something else");
+        read(&mut harness, "Gone", None);
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx).as_deref(),
+            Some("something else")
+        );
+        // A palette file the folder does not have is found nowhere either.
+        pin(&mut harness, "Missing.json");
+        assert_eq!(harness.app.pin_state(), Some(PinState::Missing));
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx).as_deref(),
+            Some("theme “Missing.json” not found: following omarchy")
+        );
+    }
+
+    #[test]
+    fn a_late_read_of_an_earlier_pin_is_dropped() {
+        let (mut harness, _, _) = following_omarchy();
+        pin(&mut harness, "Tokyo Night");
+        pin(&mut harness, "Gone");
+        read(&mut harness, "Gone", None);
+        assert_eq!(harness.app.pin_state(), Some(PinState::Missing));
+        read(&mut harness, "Tokyo Night", Some(Palette::light()));
+        assert_eq!(harness.app.pin_state(), Some(PinState::Missing));
+        assert_eq!(harness.app.desktop_theme.pin, Some(("Gone".into(), None)));
+    }
+
+    #[test]
+    fn a_missing_pin_read_again_while_the_folder_is_read_is_not_said_again() {
+        let (mut harness, _, _) = following_omarchy();
+        harness.connect_fake();
+        pin(&mut harness, "Gone");
+        read(&mut harness, "Gone", None);
+        assert!(crate::ui::toast::said(&harness.ctx).is_some());
+        crate::ui::toast::say(&harness.ctx, "something else");
+        // Omarchy switched: the folder and the pin are read again, the
+        // pin first.
+        let folder = tempfile::tempdir().unwrap();
+        let waker = fastframe_theme::Waker::default();
+        harness
+            .app
+            .themes
+            .start(folder.path().to_owned(), None, &waker);
+        read(&mut harness, "Gone", None);
+        assert_eq!(harness.app.pin_state(), Some(PinState::Reading));
+        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
+        while harness.app.themes.loading() && std::time::Instant::now() < deadline {
+            harness.app.logic(&harness.ctx.clone());
+            std::thread::sleep(std::time::Duration::from_millis(5));
+        }
+        harness.app.logic(&harness.ctx.clone());
+        assert_eq!(harness.app.pin_state(), Some(PinState::Missing));
+        assert_eq!(
+            crate::ui::toast::said(&harness.ctx).as_deref(),
+            Some("something else")
+        );
+    }
+
+    #[test]
+    fn the_terminal_look_asks_for_a_read_of_the_name_and_the_pin() {
+        let home = tempfile::tempdir().unwrap();
+        let (mut harness, _, _) = following_omarchy();
+        harness.app.follow_desktop = true;
+        harness.app.dirs.home = Some(home.path().to_owned());
+        let asked = |harness: &mut Harness| -> Vec<Option<String>> {
+            std::mem::take(&mut harness.app.backend.sent)
+                .into_iter()
+                .filter_map(|command| match command {
+                    Command::ReadTheme { places, pin } => {
+                        assert_eq!(places.home, home.path());
+                        Some(pin)
+                    }
+                    _ => None,
+                })
+                .collect()
+        };
+        pin(&mut harness, "Tokyo Night");
+        assert_eq!(asked(&mut harness), vec![Some("Tokyo Night".to_owned())]);
+        // The themes folder reads a palette file: the backend only the name.
+        pin(&mut harness, "Nord.json");
+        assert_eq!(asked(&mut harness), vec![None]);
+        // Not where nothing is pinned or followed, nor in the other looks.
+        harness.app.look = crate::theme::Look::macos();
+        pin(&mut harness, "Sunset");
+        assert_eq!(asked(&mut harness), Vec::<Option<String>>::new());
+        harness.app.look = crate::theme::Look::omarchy();
+        harness.app.follow_desktop = false;
+        pin(&mut harness, "Dusk");
+        assert_eq!(asked(&mut harness), Vec::<Option<String>>::new());
+    }
+
     fn ids(app: &App) -> Vec<u64> {
         app.tabs.iter().map(|tab| tab.id.0).collect()
     }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- app::tests`
Expected: the crate's tests do not compile:

```
error[E0599]: no variant named `ReadTheme` found for enum `backend::Command`
    --> src/app.rs:6016:30
     |
6016 |                     Command::ReadTheme { places, pin } => {
...
error: could not compile `tabletist` (lib test) due to 27 previous errors
```

- [ ] **Step 3: Read Omarchy's themes**

The module and its own tests. A name is looked for as `omarchy-theme-set` names folders (`Tokyo Night` is `tokyo-night`): the colours are the user's folder's where it has a `colors.toml`, else the shipped one's, as `omarchy-theme-set` lays the user's folder over the shipped one. A name that would lead out of them is none. A theme's colours come from `omarchy-theme-color --all`, through the user's Tabletist template where there is one, as fastframe renders the theme of the moment. `~/` in a path is the home. Files are read to 64 KB at most.

```diff
diff --git a/src/desktop_theme.rs b/src/desktop_theme.rs
new file mode 100644
index 0000000..739709c
--- /dev/null
+++ b/src/desktop_theme.rs
@@ -0,0 +1,394 @@
+//! What the terminal look reads of Omarchy's themes beside the live palette
+//! fastframe follows: the name of the theme of the moment, and a theme
+//! pinned by name or by path. Files and Omarchy's own tool: the backend
+//! reads them, never a frame.
+
+use std::io;
+use std::path::{Path, PathBuf};
+use std::process::{Command, Stdio};
+
+use crate::theme::{Palette, PinFrom};
+
+/// The most a palette file, a template, a theme's name or a theme's
+/// colours are read to: far more than any of them is.
+const LIMIT: u64 = 64 * 1024;
+
+/// Tabletist's own Omarchy template: what maps a theme's colours onto the
+/// palette when the user has no template of their own.
+const TEMPLATE: &str = include_str!("../contrib/omarchy/tabletist.json.tpl");
+
+/// Where Omarchy keeps what is read, for one user.
+#[derive(Clone, Debug, PartialEq, Eq)]
+pub struct Places {
+    /// The user's home: what `~/` in a pin's path stands for.
+    pub home: PathBuf,
+    /// The themes Omarchy ships, `$OMARCHY_PATH/themes`.
+    pub shipped: PathBuf,
+}
+
+impl Places {
+    /// The places of the user whose home is `home`, with Omarchy where
+    /// `$OMARCHY_PATH` says, else where it installs itself.
+    pub fn of(home: &Path) -> Self {
+        let omarchy = std::env::var_os("OMARCHY_PATH")
+            .filter(|path| !path.is_empty())
+            .map_or_else(|| home.join(".local/share/omarchy"), PathBuf::from);
+        Self {
+            home: home.to_owned(),
+            shipped: omarchy.join("themes"),
+        }
+    }
+
+    /// The file Omarchy writes the theme of the moment's name in.
+    fn current_name(&self) -> PathBuf {
+        self.home.join(".local/state/omarchy/current/theme.name")
+    }
+
+    /// The user's own themes, which come before the shipped ones.
+    fn own(&self) -> PathBuf {
+        self.home.join(".config/omarchy/themes")
+    }
+
+    /// The user's own Tabletist template, which Omarchy renders instead
+    /// of the app's.
+    fn template(&self) -> PathBuf {
+        self.home.join(".config/omarchy/themed/tabletist.json.tpl")
+    }
+}
+
+/// A pinned theme that was read.
+#[derive(Clone, Copy, Debug, PartialEq)]
+pub struct Pinned {
+    pub palette: Palette,
+    /// From an Omarchy theme's colours, rather than a palette file.
+    pub omarchy: bool,
+}
+
+/// What one read found.
+#[derive(Clone, Debug, Default, PartialEq)]
+pub struct Read {
+    /// The name of Omarchy's theme of the moment as Omarchy shows it
+    /// (`Tokyo Night`), where it says one.
+    pub current: Option<String>,
+    /// The pin that was read, as the settings write it, and its theme:
+    /// `None` when it is no Omarchy theme or path that could be read.
+    pub pin: Option<(String, Option<Pinned>)>,
+}
+
+/// Reads the theme of the moment's name and, where the settings pin one
+/// by name or path, that theme. `colors` gives a theme's colours as
+/// `omarchy-theme-color --all` prints them, from its `colors.toml`.
+pub fn read(
+    places: &Places,
+    pin: Option<&str>,
+    colors: impl Fn(&Path) -> io::Result<String>,
+) -> Read {
+    let current = read_small(&places.current_name())
+        .ok()
+        .and_then(|name| shown_name(name.trim()));
+    let pin = pin.map(|pin| {
+        let theme = pinned(places, pin, &colors).unwrap_or_else(|error| {
+            log::warn!("unable to read the pinned theme: {error}");
+            None
+        });
+        (pin.to_owned(), theme)
+    });
+    Read { current, pin }
+}
+
+/// The theme `pin` names: an Omarchy theme by name (the user's, then the
+/// shipped one), a palette file or a theme's folder by path. `None` for a
+/// name no Omarchy theme has, or a pin of the themes folder.
+fn pinned(
+    places: &Places,
+    pin: &str,
+    colors: &impl Fn(&Path) -> io::Result<String>,
+) -> io::Result<Option<Pinned>> {
+    match PinFrom::of(pin) {
+        PinFrom::File(_) => Ok(None),
+        PinFrom::Name(name) => {
+            let Some(folder) = folder_name(name) else {
+                return Ok(None);
+            };
+            // `omarchy-theme-set` lays the user's folder over the shipped
+            // one, file by file: the colours are the first folder's that
+            // has them.
+            for themes in [places.own(), places.shipped.clone()] {
+                let theme = themes.join(&folder);
+                if theme.join("colors.toml").is_file() {
+                    return omarchy_theme(places, &theme, colors).map(Some);
+                }
+            }
+            Ok(None)
+        }
+        PinFrom::Path(path) => {
+            let path = match path.strip_prefix("~/") {
+                Some(rest) => places.home.join(rest),
+                None => PathBuf::from(path),
+            };
+            if path.is_dir() {
+                return omarchy_theme(places, &path, colors).map(Some);
+            }
+            let palette =
+                fastframe_theme::parse_palette(&read_small(&path)?).map_err(io::Error::other)?;
+            Ok(Some(Pinned {
+                palette,
+                omarchy: false,
+            }))
+        }
+    }
+}
+
+/// The palette of the Omarchy theme in `folder`: its colours through the
+/// user's template, else the app's.
+fn omarchy_theme(
+    places: &Places,
+    folder: &Path,
+    colors: &impl Fn(&Path) -> io::Result<String>,
+) -> io::Result<Pinned> {
+    let template = places.template();
+    let template = if template.is_file() {
+        read_small(&template)?
+    } else {
+        TEMPLATE.to_owned()
+    };
+    let colors = colors(&folder.join("colors.toml"))?;
+    let rendered = fastframe_theme::omarchy::render_seed::<Palette>(&template, &colors)
+        .map_err(io::Error::other)?;
+    let palette = fastframe_theme::parse_palette(&rendered).map_err(io::Error::other)?;
+    Ok(Pinned {
+        palette,
+        omarchy: true,
+    })
+}
+
+/// The folder a theme called `name` is in, as `omarchy-theme-set` finds
+/// it (`Tokyo Night` is `tokyo-night`). `None` for a name that would lead
+/// out of the themes' folder.
+fn folder_name(name: &str) -> Option<String> {
+    // `tr '[:upper:]' '[:lower:]'`, as `omarchy-theme-set` lowers it:
+    // ASCII letters only.
+    let folder = name.to_ascii_lowercase().replace(' ', "-");
+    let plain = !folder.is_empty() && !folder.starts_with('.') && !folder.contains(['/', '\\']);
+    plain.then_some(folder)
+}
+
+/// A theme's folder name as Omarchy shows it: `tokyo-night` is
+/// `Tokyo Night`.
+fn shown_name(folder: &str) -> Option<String> {
+    let words: Vec<String> = folder
+        .split('-')
+        .filter(|word| !word.is_empty())
+        .map(|word| {
+            let mut letters = word.chars();
+            letters.next().map_or_else(String::new, |first| {
+                first.to_uppercase().chain(letters).collect()
+            })
+        })
+        .collect();
+    (!words.is_empty()).then(|| words.join(" "))
+}
+
+/// A theme's colours from Omarchy's own tool, as fastframe reads the
+/// current theme's.
+pub fn omarchy_colors(file: &Path) -> io::Result<String> {
+    let output = Command::new("omarchy-theme-color")
+        .arg("--file")
+        .arg(file)
+        .arg("--all")
+        .stdin(Stdio::null())
+        .output()?;
+    if !output.status.success() || output.stdout.len() as u64 > LIMIT {
+        return Err(io::Error::other("the theme's colours could not be read"));
+    }
+    String::from_utf8(output.stdout).map_err(io::Error::other)
+}
+
+/// A small file's text: an error for one larger than [`LIMIT`], and for
+/// what is no file (a pipe or a device would hold the read).
+fn read_small(path: &Path) -> io::Result<String> {
+    use std::io::Read as _;
+    if !std::fs::metadata(path)?.is_file() {
+        return Err(io::Error::other("not a file"));
+    }
+    let mut text = String::new();
+    std::fs::File::open(path)?
+        .take(LIMIT + 1)
+        .read_to_string(&mut text)?;
+    if text.len() as u64 > LIMIT {
+        return Err(io::Error::other("the file is too large"));
+    }
+    Ok(text)
+}
+
+#[cfg(test)]
+mod tests {
+    use super::*;
+
+    /// A home with Omarchy's folders, and the places in it.
+    fn home() -> (tempfile::TempDir, Places) {
+        let root = tempfile::tempdir().unwrap();
+        let places = Places {
+            home: root.path().to_owned(),
+            shipped: root.path().join("omarchy/themes"),
+        };
+        (root, places)
+    }
+
+    /// Writes `path` under the home, with its folders.
+    fn put(places: &Places, path: &str, text: &str) {
+        let path = places.home.join(path);
+        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
+        std::fs::write(path, text).unwrap();
+    }
+
+    /// Colours as `omarchy-theme-color --all` prints them, with `accent`
+    /// for the accent.
+    fn colors_with(accent: &str) -> String {
+        format!(
+            "mode\tdark\nbackground\t#1a1b26\ndark_background\t#16161e\n\
+             lighter_background\t#292e42\nforeground\t#c0caf5\nmuted\t#565f89\n\
+             accent\t{accent}\nselection\t#283457\nred\t#f7768e\n\
+             green\t#9ece6a\nyellow\t#e0af68\ncyan\t#7dcfff\n\
+             orange\t#ff9e64\nmagenta\t#bb9af7\nblue\t#7aa2f7\n\
+             bright_red\t#f7768e\nbright_green\t#9ece6a\n"
+        )
+    }
+
+    /// Reads a theme's colours from the accent its `colors.toml` holds:
+    /// what the tool would print, without the tool.
+    fn colors(file: &Path) -> io::Result<String> {
+        let accent = std::fs::read_to_string(file)?;
+        Ok(colors_with(accent.trim()))
+    }
+
+    fn accent(read: &Read) -> Option<egui::Color32> {
+        read.pin.as_ref()?.1.map(|pinned| pinned.palette.accent)
+    }
+
+    #[test]
+    fn the_theme_of_the_moment_is_named_as_omarchy_shows_it() {
+        let (_root, places) = home();
+        assert_eq!(read(&places, None, colors).current, None);
+        put(
+            &places,
+            ".local/state/omarchy/current/theme.name",
+            "tokyo-night\n",
+        );
+        assert_eq!(
+            read(&places, None, colors).current.as_deref(),
+            Some("Tokyo Night")
+        );
+        put(&places, ".local/state/omarchy/current/theme.name", "\n");
+        assert_eq!(read(&places, None, colors).current, None);
+    }
+
+    #[test]
+    fn a_name_is_the_users_omarchy_theme_before_the_shipped_one() {
+        let (_root, places) = home();
+        put(&places, "omarchy/themes/tokyo-night/colors.toml", "#111111");
+        let read_pin = |pin| accent(&read(&places, Some(pin), colors));
+        let shipped = egui::Color32::from_rgb(0x11, 0x11, 0x11);
+        assert_eq!(read_pin("Tokyo Night"), Some(shipped));
+        assert_eq!(read_pin("tokyo-night"), Some(shipped));
+        put(
+            &places,
+            ".config/omarchy/themes/tokyo-night/colors.toml",
+            "#222222",
+        );
+        assert_eq!(
+            read_pin("Tokyo Night"),
+            Some(egui::Color32::from_rgb(0x22, 0x22, 0x22))
+        );
+        // A folder of the user's without colours takes the shipped ones.
+        put(&places, ".config/omarchy/themes/sunset/neovim.lua", "");
+        put(&places, "omarchy/themes/sunset/colors.toml", "#333333");
+        assert_eq!(
+            read_pin("Sunset"),
+            Some(egui::Color32::from_rgb(0x33, 0x33, 0x33))
+        );
+        let pinned = read(&places, Some("Tokyo Night"), colors).pin.unwrap();
+        assert_eq!(pinned.0, "Tokyo Night");
+        assert!(pinned.1.unwrap().omarchy);
+        // No such theme: nothing, for the app to look in the themes folder.
+        assert_eq!(
+            read(&places, Some("Nord"), colors).pin,
+            Some(("Nord".into(), None))
+        );
+    }
+
+    #[test]
+    fn the_users_template_maps_a_pinned_theme_as_omarchy_would() {
+        let (_root, places) = home();
+        put(&places, "omarchy/themes/sunset/colors.toml", "#111111");
+        let own = TEMPLATE.replace("\"accent\": \"{{ accent }}\"", "\"accent\": \"{{ red }}\"");
+        assert_ne!(own, TEMPLATE);
+        put(&places, ".config/omarchy/themed/tabletist.json.tpl", &own);
+        assert_eq!(
+            accent(&read(&places, Some("Sunset"), colors)),
+            Some(egui::Color32::from_rgb(0xf7, 0x76, 0x8e))
+        );
+    }
+
+    #[test]
+    fn a_path_is_a_themes_folder_or_a_palette_file() {
+        let (_root, places) = home();
+        put(&places, "mine/sunset/colors.toml", "#333333");
+        let folder = places.home.join("mine/sunset");
+        let pin = folder.to_str().unwrap();
+        assert_eq!(
+            accent(&read(&places, Some(pin), colors)),
+            Some(egui::Color32::from_rgb(0x33, 0x33, 0x33))
+        );
+        put(
+            &places,
+            "mine/Plain.json",
+            "{\"base\": \"light\", \"colors\": {\"accent\": \"#444444\"}}",
+        );
+        let pinned = read(&places, Some("~/mine/Plain.json"), colors)
+            .pin
+            .unwrap();
+        assert_eq!(
+            pinned.1.unwrap().palette.accent,
+            egui::Color32::from_rgb(0x44, 0x44, 0x44)
+        );
+        assert!(!pinned.1.unwrap().omarchy);
+        // What cannot be read pins nothing.
+        for pin in ["~/mine/Missing.json", "~/mine/sunset/colors.toml"] {
+            assert_eq!(accent(&read(&places, Some(pin), colors)), None, "{pin}");
+        }
+        // What is no file is not read: a device would never end.
+        #[cfg(unix)]
+        assert_eq!(accent(&read(&places, Some("/dev/zero"), colors)), None);
+        let broken = |_: &Path| -> io::Result<String> { Err(io::Error::other("no tool")) };
+        assert_eq!(accent(&read(&places, Some(pin), broken)), None);
+    }
+
+    #[test]
+    fn a_name_never_leads_out_of_the_themes_folders() {
+        let (_root, places) = home();
+        put(&places, "omarchy/escape/colors.toml", "#555555");
+        for pin in ["../escape", "..", ".hidden", "a\\b"] {
+            assert_eq!(accent(&read(&places, Some(pin), colors)), None, "{pin}");
+        }
+        // A palette file of the themes folder is the catalog's, not read here.
+        assert_eq!(
+            read(&places, Some("Nord.json"), colors).pin,
+            Some(("Nord.json".into(), None))
+        );
+    }
+
+    #[test]
+    fn a_folder_name_is_shown_in_words() {
+        assert_eq!(shown_name("tokyo-night").as_deref(), Some("Tokyo Night"));
+        assert_eq!(shown_name("ristretto").as_deref(), Some("Ristretto"));
+        assert_eq!(
+            shown_name("catppuccin-latte").as_deref(),
+            Some("Catppuccin Latte")
+        );
+        assert_eq!(shown_name("").as_deref(), None);
+        assert_eq!(folder_name("Tokyo Night").as_deref(), Some("tokyo-night"));
+        // As Omarchy lowers a name: its ASCII letters.
+        assert_eq!(folder_name("Ünique Dark").as_deref(), Some("Ünique-dark"));
+    }
+}
diff --git a/src/lib.rs b/src/lib.rs
index ba2ba18..d35a44e 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -6,6 +6,7 @@ pub mod backend;
 pub mod clock;
 pub mod completion;
 pub mod connections;
+pub mod desktop_theme;
 pub mod edit;
 pub mod endpoint;
 pub mod entrypoint;
```

- [ ] **Step 4: Read on the backend**

```diff
diff --git a/src/backend.rs b/src/backend.rs
index 4e38917..a221f30 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -186,6 +186,12 @@ pub enum Command {
         path: PathBuf,
         text: String,
     },
+    /// Reads the name of Omarchy's theme of the moment and the theme
+    /// `pin` names, for the terminal look. Answered with [`Event::Theme`].
+    ReadTheme {
+        places: crate::desktop_theme::Places,
+        pin: Option<String>,
+    },
     /// Signals `done` once every save sent before it is on disk.
     Flush {
         done: mpsc::Sender<()>,
@@ -342,6 +348,8 @@ pub enum Event {
     /// a directory that cannot be watched, or such a directory that can be
     /// watched after all.
     SettingsWatch { live: bool },
+    /// What [`Command::ReadTheme`] read.
+    Theme(crate::desktop_theme::Read),
     /// The settings file changed on disk: its text. `own` when it is, byte
     /// for byte, what the backend itself wrote there last: the app's own
     /// save coming back, which it must not take for someone else's change.
@@ -1871,6 +1879,17 @@ impl Worker {
                     );
                 });
             }
+            Command::ReadTheme { places, pin } => {
+                let outbox = self.outbox.clone();
+                tokio::task::spawn_blocking(move || {
+                    let read = crate::desktop_theme::read(
+                        &places,
+                        pin.as_deref(),
+                        crate::desktop_theme::omarchy_colors,
+                    );
+                    outbox.emit(Event::Theme(read));
+                });
+            }
             Command::Flush { done } => {
                 let saves = self.saves.clone();
                 tokio::spawn(async move {
@@ -1920,6 +1939,7 @@ fn session_of(command: &Command) -> SessionId {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
+        | Command::ReadTheme { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => SessionId(0),
     }
@@ -1948,6 +1968,7 @@ fn request_of(command: &Command) -> Option<RequestId> {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
+        | Command::ReadTheme { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => None,
     }
@@ -2034,6 +2055,7 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
         | Command::WatchSettings { .. }
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
+        | Command::ReadTheme { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => return,
     };
@@ -2316,6 +2338,7 @@ async fn run_session(
             | Command::WatchSettings { .. }
             | Command::EditSettingsFile { .. }
             | Command::RevealSettingsFile { .. }
+            | Command::ReadTheme { .. }
             | Command::Flush { .. }
             | Command::Agents(_) => None,
         };
```

- [ ] **Step 5: Ask for the read, draw it, and tell a pin found nowhere**

A read is asked for at the start, when the themes folder reloads (Omarchy switched), and when the settings name another theme; never in tests or the demo. A read of another pin than the one asked for now leaves the pin's read as it was. Until the read comes, a name or a path draws the desktop: the folder's palette of an Omarchy theme's name would flash. `PinState` says how the pin stands, for the toast here and the pane in Task 3. The toast waits for a status line, which the terminal look has in a workspace under no dialog, and a pin read again after Omarchy switches is not said again.

```diff
diff --git a/src/app.rs b/src/app.rs
index ff9b03e..baf151b 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -14,7 +14,7 @@ use crate::backend::{
 use crate::completion::Need;
 use crate::connections::{PasswordMode, SavedConnection};
 use crate::edit::EditorPlace;
-use crate::i18n::Locale;
+use crate::i18n::{Locale, gettext};
 use crate::model::{Action, ConnTab, ConnTabContent, ConnTabId, PickerState};
 use crate::model::{
     Advance, CellPos, Completion, ConnectionForm, Dialog, Fetch, FilterBar, FilterRow, Held,
@@ -65,6 +65,17 @@ pub struct TitleBar {
     pub inset: f32,
 }
 
+/// How the terminal look's pinned theme stands.
+#[derive(Debug, Clone, Copy, PartialEq)]
+pub enum PinState {
+    /// Drawn: what the pin named was found.
+    Drawn(crate::desktop_theme::Pinned),
+    /// Not read yet: the desktop is followed until it is.
+    Reading,
+    /// Nowhere to be found: the desktop is followed instead.
+    Missing,
+}
+
 /// What quick open lists.
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub enum Found {
@@ -124,6 +135,14 @@ pub struct App {
     follow_desktop: bool,
     /// The settings named another theme since the last `logic`.
     theme_changed: bool,
+    /// What the backend last read of Omarchy's themes: the name of the
+    /// theme of the moment, and the pinned theme.
+    pub desktop_theme: crate::desktop_theme::Read,
+    /// A read came back since the last `logic`.
+    desktop_read: bool,
+    /// The pin the status line last said could not be read: it is said
+    /// once.
+    pin_told: Option<String>,
     /// The window title last sent, so it is sent only when it changes.
     window_title: String,
     /// What the next frame asks of the window: to come forward, or the
@@ -242,6 +261,9 @@ impl App {
             system_theme: None,
             follow_desktop: false,
             theme_changed: false,
+            desktop_theme: crate::desktop_theme::Read::default(),
+            desktop_read: false,
+            pin_told: None,
             window_title: "Tabletist".into(),
             window_asks: Vec::new(),
             closed_editors: Vec::new(),
@@ -3133,6 +3155,16 @@ impl App {
                 }
             }
             Event::SettingsWatch { live } => self.settings_file.live = live,
+            Event::Theme(read) => {
+                // Reads end in any order: one that read another pin
+                // than the one asked for now leaves the last one's.
+                let asked = read.pin.as_ref().map(|(pin, _)| pin.as_str()) == self.pin_to_read();
+                self.desktop_theme.current = read.current;
+                if asked {
+                    self.desktop_theme.pin = read.pin;
+                }
+                self.desktop_read = true;
+            }
             Event::SettingsFile { text, own } => {
                 // What the app holds already: its own write coming back,
                 // or a save that changed nothing.
@@ -5184,6 +5216,7 @@ impl App {
                 self.palette_file(),
                 &fastframe_theme::Waker::new(move || repaint.request_repaint()),
             );
+            self.read_desktop_theme();
             // Not in tests or the demo, which must not watch the user's
             // directories any more than they scan them.
             self.watch_settings();
@@ -5199,21 +5232,87 @@ impl App {
         theme::PinFrom::of(pin).file().map(str::to_owned)
     }
 
-    /// The pinned theme's palette, where the terminal look has one: read
-    /// from the themes folder.
-    fn pinned_palette(&self) -> Option<Palette> {
+    /// Asks the backend for the name of Omarchy's theme of the moment and
+    /// the theme a pin names by name or path: the terminal look's, where
+    /// the desktop is followed.
+    fn read_desktop_theme(&mut self) {
+        let Some(home) = self.dirs.home.as_deref() else {
+            return;
+        };
+        if !self.look.terminal || !self.follow_desktop {
+            return;
+        }
+        let pin = self.pin_to_read().map(str::to_owned);
+        self.backend.send(Command::ReadTheme {
+            places: crate::desktop_theme::Places::of(home),
+            pin,
+        });
+    }
+
+    /// The pin the backend reads: one by name or path. A palette file is
+    /// the themes folder's to read.
+    fn pin_to_read(&self) -> Option<&str> {
+        let pin = self.settings.theme.pin()?;
+        theme::PinFrom::of(pin).file().is_none().then_some(pin)
+    }
+
+    /// The pinned theme, where the terminal look has one drawn: a palette
+    /// file from the themes folder; a name or a path as the backend read
+    /// it, and a name no Omarchy theme has from the folder too.
+    fn pinned(&self) -> Option<crate::desktop_theme::Pinned> {
         if !self.look.terminal {
             return None;
         }
         let pin = self.settings.theme.pin()?;
-        theme::catalog_pin(&self.themes, pin)
+        let from_folder = || {
+            let palette = theme::catalog_pin(&self.themes, pin)?;
+            Some(crate::desktop_theme::Pinned {
+                palette,
+                omarchy: false,
+            })
+        };
+        let read = self.desktop_theme.pin.as_ref();
+        let read = read
+            .filter(|(read, _)| read == pin)
+            .map(|(_, theme)| *theme);
+        match (theme::PinFrom::of(pin), read) {
+            (theme::PinFrom::File(_), _) => from_folder(),
+            (_, Some(Some(theme))) => Some(theme),
+            // Only once read: the folder's palette of an Omarchy theme's
+            // name (Tokyo Night has both) would be drawn until then.
+            (theme::PinFrom::Name(_), Some(None)) => from_folder(),
+            (theme::PinFrom::Name(_) | theme::PinFrom::Path(_), _) => None,
+        }
+    }
+
+    /// How the terminal look's pin stands: drawn, still being read, or
+    /// not to be found, when there is one.
+    pub fn pin_state(&self) -> Option<PinState> {
+        let pin = self.settings.theme.pin().filter(|_| self.look.terminal)?;
+        if let Some(pinned) = self.pinned() {
+            return Some(PinState::Drawn(pinned));
+        }
+        // A file waits for the themes folder, a name or a path for the
+        // backend: in tests and the demo they are never read.
+        let read = match theme::PinFrom::of(pin) {
+            theme::PinFrom::File(_) => !self.themes.loading(),
+            theme::PinFrom::Name(_) | theme::PinFrom::Path(_) => {
+                let read = self.desktop_theme.pin.as_ref();
+                read.is_some_and(|(read, _)| read == pin) && !self.themes.loading()
+            }
+        };
+        Some(if read {
+            PinState::Missing
+        } else {
+            PinState::Reading
+        })
     }
 
     fn resolve_palette(&self) -> Palette {
         theme::resolve(
             &self.look,
             &self.settings.theme,
-            self.pinned_palette(),
+            self.pinned().map(|pinned| pinned.palette),
             &self.themes,
             self.system_theme,
         )
@@ -5225,6 +5324,8 @@ impl App {
         // The settings named another theme: where the desktop is followed
         // the catalog reads that file first, as it does at the start.
         let renamed = std::mem::take(&mut self.theme_changed);
+        // Omarchy switched themes, or the settings pin another: the name
+        // of the moment and the pin are read again with the folder.
         if self.themes.needs_reload() || (renamed && self.follow_desktop) {
             let repaint = ctx.clone();
             self.themes.start(
@@ -5232,10 +5333,12 @@ impl App {
                 self.palette_file(),
                 &fastframe_theme::Waker::new(move || repaint.request_repaint()),
             );
+            self.read_desktop_theme();
         }
         let scanned = self.themes.poll();
+        let read = std::mem::take(&mut self.desktop_read);
         let system = ctx.system_theme();
-        if scanned || renamed || system != self.system_theme {
+        if scanned || renamed || read || system != self.system_theme {
             self.system_theme = system;
             let palette = self.resolve_palette();
             if palette != self.palette {
@@ -5243,11 +5346,41 @@ impl App {
                 theme::apply(ctx, &palette, &self.look);
             }
         }
+        self.tell_missing_pin(ctx);
         // A hidden window draws no frame: its close request is answered
         // here, or nothing would hold it back.
         self.hold_close(ctx);
     }
 
+    /// Says in the status line, once, that the pinned theme could not be
+    /// found and the desktop is followed instead. It waits for a status
+    /// line to say it in: the terminal look's is a workspace's, under no
+    /// dialog.
+    fn tell_missing_pin(&mut self, ctx: &egui::Context) {
+        let state = self.pin_state();
+        let pin = self.settings.theme.pin();
+        // A pin read again (Omarchy switched) is not found yet: what was
+        // told of it stays told.
+        if state == Some(PinState::Reading) {
+            if self.pin_told.as_deref() != pin {
+                self.pin_told = None;
+            }
+            return;
+        }
+        let Some(pin) = pin.filter(|_| state == Some(PinState::Missing)) else {
+            self.pin_told = None;
+            return;
+        };
+        let seen = self.dialog.is_none() && self.workspace(self.active_tab_id()).is_some();
+        if self.pin_told.as_deref() == Some(pin) || !seen {
+            return;
+        }
+        let said = gettext(self.locale, crate::ui::settings::pin_missing(&self.themes));
+        let said = self.look.label(&said).replace("{name}", pin);
+        crate::ui::toast::say(ctx, &said);
+        self.pin_told = Some(pin.to_owned());
+    }
+
     /// The foreign key picker's searches wait for a pause in the typing:
     /// the ones that are due are sent, and the frame comes back when the
     /// next one is.
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 173c653..a9731d4 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -74,6 +74,16 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
     }
 }
 
+/// What is said of a pinned theme that is nowhere to be found, `{name}`
+/// for its name: what is drawn instead. English.
+pub fn pin_missing(catalog: &crate::theme::Catalog) -> &'static str {
+    if catalog.follows_omarchy() {
+        "Theme “{name}” not found: following Omarchy"
+    } else {
+        "Theme “{name}” not found: following the system"
+    }
+}
+
 /// The days the log keeps a call, as the window writes them.
 pub(super) fn days(locale: Locale, days: u32) -> String {
     ngettext(locale, "{n} day", "{n} days", days).replace("{n}", &days.to_string())
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- app::tests desktop_theme`
Expected: PASS. Then the four checks above.

- [ ] **Step 7: Try it on the desktop**

On Omarchy, put `theme = "Tokyo Night"` in `~/.config/tabletist/settings.toml` while Tabletist runs on another theme: it recolours, and `omarchy-theme-set` no longer changes it. With a connection open and Settings closed, `theme = "Nowhere"` says so in the status line and follows Omarchy (with Settings open, the theme row says it). Then take the line out.

- [ ] **Step 8: Commit**

```bash
git add src/desktop_theme.rs src/lib.rs src/backend.rs src/app.rs src/ui/settings/mod.rs
git commit -m "Read the Omarchy theme's name and a pinned theme on the backend"
```

---

### Task 3: The terminal look's theme and palette rows

**Files:**
- Modify: `src/settings.rs`, `src/model.rs`, `src/app.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/appearance_settings_tests.rs`, `src/ui/mod.rs`
- Create: `src/ui/theme_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

A new test file, declared in `src/ui/mod.rs`, in a window as wide as the board's, where the hints have room beside the file. It borrows three helpers of step 1's file, which become `pub(super)`. Step 1's key tests move down one row, past the theme's, and the options' test takes the new key and section.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 111f460..0bc1072 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1778,6 +1778,7 @@ bar = \"tint\"  # tint | label
                 Key::Timestamps,
                 Key::GroupDigits,
                 Key::ValueTags,
+                Key::Theme,
                 Key::RowHeight,
                 Key::Zebra,
                 Key::ColumnTypes,
@@ -1796,7 +1797,12 @@ bar = \"tint\"  # tint | label
         ]
         .concat();
         // The Appearance pane's sections hold its options, in order.
-        let sections = [&OptionId::GRID[..], &OptionId::ENVIRONMENT[..]].concat();
+        let sections = [
+            &OptionId::THEME[..],
+            &OptionId::GRID[..],
+            &OptionId::ENVIRONMENT[..],
+        ]
+        .concat();
         assert_eq!(sections, OptionId::APPEARANCE);
         assert_eq!(panes, OptionId::ALL);
         let defaults = Settings::default();
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
index 2420027..bd70a42 100644
--- a/src/ui/appearance_settings_tests.rs
+++ b/src/ui/appearance_settings_tests.rs
@@ -43,7 +43,7 @@ fn covers(look: Look, rows: usize) -> (Harness, ConnTabId, TabId) {
 }
 
 /// Whether the last frame painted a text that has `words` in it.
-fn shows(harness: &mut Harness, words: &str) -> bool {
+pub(super) fn shows(harness: &mut Harness, words: &str) -> bool {
     harness.settle();
     harness.painted.iter().any(|(text, _)| text.contains(words))
 }
@@ -356,7 +356,7 @@ fn pane(harness: &Harness) -> Option<SettingsPane> {
 }
 
 /// How many times the settings were saved.
-fn settings_saves(harness: &Harness) -> usize {
+pub(super) fn settings_saves(harness: &Harness) -> usize {
     harness
         .app
         .backend
@@ -375,7 +375,7 @@ fn settings_saves(harness: &Harness) -> usize {
 }
 
 /// Settings on its Appearance pane.
-fn on_the_pane(harness: &mut Harness) {
+pub(super) fn on_the_pane(harness: &mut Harness) {
     harness.app.apply(Action::ShowSettings);
     harness
         .app
@@ -462,7 +462,8 @@ fn the_screen_sets_each_appearance_option_with_its_keys() {
     let key = |harness: &mut Harness, key: egui::Key, text: &str| {
         super::tests::type_key(harness, key, text);
     };
-    // Row height: one to the left, compact.
+    // Row height, under the theme: one to the left, compact.
+    key(&mut harness, egui::Key::J, "j");
     key(&mut harness, egui::Key::H, "h");
     // Alternate rows, on in the terminal until set: space turns it off.
     key(&mut harness, egui::Key::J, "j");
@@ -544,7 +545,9 @@ fn a_reset_and_a_toggle_in_one_frame_turn_the_look_s_stripes_off() {
     let mut harness = Harness::new();
     harness.set_look(Look::omarchy());
     on_the_pane(&mut harness);
-    super::tests::type_key(&mut harness, egui::Key::J, "j");
+    for _ in 0..2 {
+        super::tests::type_key(&mut harness, egui::Key::J, "j");
+    }
     change(&mut harness, |settings| settings.zebra = Some(true));
     // R puts them back to the look's, which shows them; Space then turns
     // them off.
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 523f526..6dc3725 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -64,6 +64,8 @@ pub mod sql_text;
 pub mod states;
 pub mod structure;
 pub mod terminal_dialog;
+#[cfg(test)]
+mod theme_settings_tests;
 pub mod toast;
 pub mod value_tags;
 pub mod widgets;
diff --git a/src/ui/theme_settings_tests.rs b/src/ui/theme_settings_tests.rs
new file mode 100644
index 0000000..143c277
--- /dev/null
+++ b/src/ui/theme_settings_tests.rs
@@ -0,0 +1,238 @@
+//! Settings › Appearance, its Theme section: the terminal look's follow
+//! and pin with the palette they give, and the tiles of macOS and Windows.
+
+use egui::accesskit::{Role, Toggled};
+
+use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
+use crate::backend::Event;
+use crate::desktop_theme::{Pinned, Read};
+use crate::model::Action;
+use crate::settings::{Settings, Theme};
+use crate::testing::Harness;
+use crate::theme::{Catalog, CustomTheme, Look, Palette};
+
+/// Whether the control called `label` is told to a screen reader as set.
+fn set(harness: &mut Harness, label: &str) -> Option<bool> {
+    let tree = harness.settle();
+    let node = crate::testing::node(&tree, label, Role::RadioButton)?;
+    let (_, node) = tree.nodes.iter().find(|(id, _)| *id == node)?;
+    Some(node.toggled() == Some(Toggled::True))
+}
+
+/// Omarchy's live palette, with an accent of its own.
+fn omarchy_palette() -> Palette {
+    let mut palette = Palette::dark();
+    palette.accent = egui::Color32::from_rgb(0xf3, 0x8d, 0x70);
+    palette
+}
+
+/// The terminal look in a window as wide as the board's, where the rows'
+/// hints have room beside the file.
+fn terminal() -> Harness {
+    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
+    harness.set_look(Look::omarchy());
+    harness
+}
+
+/// The terminal look following Omarchy, whose theme of the moment is
+/// Tokyo Night.
+fn following() -> Harness {
+    let mut harness = terminal();
+    harness.app.themes = Catalog::preview(
+        vec![CustomTheme {
+            filename: fastframe_theme::omarchy::FILENAME.into(),
+            palette: omarchy_palette(),
+        }],
+        true,
+    );
+    harness.app.desktop_theme.current = Some("Tokyo Night".into());
+    harness.app.logic(&harness.ctx.clone());
+    harness
+}
+
+/// The settings with `theme`, as an edit of the file sets it.
+fn with_theme(harness: &mut Harness, theme: Theme) {
+    let settings = Settings {
+        theme,
+        ..harness.app.settings.clone()
+    };
+    harness.app.apply_settings(settings);
+    harness.app.logic(&harness.ctx.clone());
+}
+
+/// What the backend read of the pin `name`: an Omarchy theme's palette.
+fn read(harness: &mut Harness, name: &str, palette: Option<Palette>) {
+    let read = Read {
+        current: Some("Tokyo Night".into()),
+        pin: Some((
+            name.into(),
+            palette.map(|palette| Pinned {
+                palette,
+                omarchy: true,
+            }),
+        )),
+    };
+    harness.app.apply(Action::Backend(Event::Theme(read)));
+    harness.app.logic(&harness.ctx.clone());
+}
+
+fn key(harness: &mut Harness, key: egui::Key, text: &str) {
+    super::tests::type_key(harness, key, text);
+}
+
+#[test]
+fn the_screen_shows_omarchys_theme_and_the_palette_it_gives() {
+    let mut harness = following();
+    on_the_pane(&mut harness);
+    for words in [
+        "theme",
+        "follow omarchy",
+        "now: tokyo night · switches with omarchy-theme",
+        "palette",
+        "read from the omarchy theme",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    assert_eq!(set(&mut harness, "Theme: follow omarchy"), Some(true));
+    assert_eq!(set(&mut harness, "Theme: pin"), Some(false));
+    // The eight swatches, in the palette's colours: the accent among them.
+    let palette = harness.app.palette;
+    let swatch = |color| {
+        harness
+            .fills
+            .iter()
+            .any(|(rect, fill)| *fill == color && rect.size() == egui::vec2(18.0, 14.0))
+    };
+    for color in [
+        palette.window,
+        palette.accent,
+        palette.danger,
+        palette.orange,
+    ] {
+        assert!(swatch(color), "{color:?}");
+    }
+}
+
+#[test]
+fn l_pins_the_theme_omarchy_shows_and_h_follows_it_again() {
+    let mut harness = following();
+    on_the_pane(&mut harness);
+    let saves = settings_saves(&harness);
+    key(&mut harness, egui::Key::L, "l");
+    assert_eq!(
+        harness.app.settings.theme,
+        Theme::Pinned("Tokyo Night".into())
+    );
+    assert_eq!(settings_saves(&harness), saves + 1);
+    assert!(
+        harness
+            .app
+            .settings_file
+            .text
+            .contains("theme = \"Tokyo Night\"")
+    );
+    // Drawn once the backend read it.
+    let mut tokyo = Palette::dark();
+    tokyo.accent = egui::Color32::from_rgb(0x7a, 0xa2, 0xf7);
+    read(&mut harness, "Tokyo Night", Some(tokyo));
+    assert!(shows(&mut harness, "now: Tokyo Night · pinned"));
+    assert!(shows(&mut harness, "read from the omarchy theme"));
+    assert_eq!(set(&mut harness, "Theme: pin"), Some(true));
+    // l again keeps the pin; h follows Omarchy again.
+    key(&mut harness, egui::Key::L, "l");
+    assert_eq!(
+        harness.app.settings.theme,
+        Theme::Pinned("Tokyo Night".into())
+    );
+    key(&mut harness, egui::Key::H, "h");
+    assert_eq!(harness.app.settings.theme, Theme::Omarchy);
+    // Space flips it, and R takes the value out of the file.
+    harness.press(egui::Key::Space, egui::Modifiers::NONE);
+    harness.settle();
+    assert_eq!(
+        harness.app.settings.theme,
+        Theme::Pinned("Tokyo Night".into())
+    );
+    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
+    harness.settle();
+    assert_eq!(harness.app.settings.theme, Theme::Unset);
+    assert!(!harness.app.settings_file.text.contains("theme ="));
+}
+
+#[test]
+fn a_pin_and_a_flip_in_one_frame_follow_omarchy_again() {
+    let mut harness = following();
+    on_the_pane(&mut harness);
+    harness.frame(vec![
+        crate::testing::key(egui::Key::L, egui::Modifiers::NONE),
+        egui::Event::Text("l".into()),
+        crate::testing::key(egui::Key::Space, egui::Modifiers::NONE),
+    ]);
+    harness.settle();
+    assert_eq!(harness.app.settings.theme, Theme::Omarchy);
+}
+
+#[test]
+fn with_no_omarchy_theme_to_name_pin_does_nothing() {
+    let mut harness = terminal();
+    on_the_pane(&mut harness);
+    assert!(shows(&mut harness, "now: dark, from the system"));
+    assert!(shows(&mut harness, "tabletist's own light or dark"));
+    let saves = settings_saves(&harness);
+    key(&mut harness, egui::Key::L, "l");
+    assert_eq!(harness.app.settings.theme, Theme::Unset);
+    assert_eq!(settings_saves(&harness), saves);
+}
+
+#[test]
+fn a_value_of_macos_is_the_desktop_here_and_stays_in_the_file() {
+    let mut harness = following();
+    with_theme(&mut harness, Theme::Dark);
+    on_the_pane(&mut harness);
+    assert_eq!(set(&mut harness, "Theme: follow omarchy"), Some(true));
+    assert_eq!(harness.app.palette, omarchy_palette());
+    // h, and a click on follow, change nothing the row shows: nothing.
+    let saves = settings_saves(&harness);
+    key(&mut harness, egui::Key::H, "h");
+    harness.click("Theme: follow omarchy");
+    harness.settle();
+    assert_eq!(harness.app.settings.theme, Theme::Dark);
+    assert_eq!(settings_saves(&harness), saves);
+    // Another option's change writes the file again, with the value in it.
+    key(&mut harness, egui::Key::J, "j");
+    key(&mut harness, egui::Key::H, "h");
+    assert!(harness.app.settings_file.text.contains("theme = \"dark\""));
+    assert_eq!(harness.app.settings.theme, Theme::Dark);
+}
+
+#[test]
+fn a_pin_found_nowhere_is_said_beside_it() {
+    let mut harness = following();
+    with_theme(&mut harness, Theme::Pinned("Gone".into()));
+    read(&mut harness, "Gone", None);
+    on_the_pane(&mut harness);
+    let said = "“Gone” not found · following omarchy";
+    assert!(shows(&mut harness, said));
+    assert_eq!(
+        harness.painted_color(said),
+        Some(harness.app.palette.warning)
+    );
+    // A palette file is said as written: the pin is drawn from it.
+    with_theme(&mut harness, Theme::Pinned("~/mine/Sunset.json".into()));
+    let mut sunset = Palette::light();
+    sunset.accent = egui::Color32::from_rgb(0x44, 0x44, 0x44);
+    let file = Read {
+        current: Some("Tokyo Night".into()),
+        pin: Some((
+            "~/mine/Sunset.json".into(),
+            Some(Pinned {
+                palette: sunset,
+                omarchy: false,
+            }),
+        )),
+    };
+    harness.app.apply(Action::Backend(Event::Theme(file)));
+    harness.app.logic(&harness.ctx.clone());
+    assert!(shows(&mut harness, "now: ~/mine/Sunset.json · pinned"));
+    assert!(shows(&mut harness, "read from ~/mine/Sunset.json"));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- theme_settings appearance_settings settings::tests`
Expected: the crate's tests do not compile:

```
error[E0599]: no variant, associated function, or constant named `THEME` found for enum `settings::OptionId` in the current scope
    --> src/settings.rs:1801:24
     |
 350 | pub enum OptionId {
...
error: could not compile `tabletist` (lib test) due to 1 previous error
```

- [ ] **Step 3: The option**

`OptionId::Theme` is stored under `appearance.theme`. `ThemeChoice` is the theme as the window sets it, with no name: a pin set from the window is the Omarchy theme of the moment, which the app knows and the settings do not. So `OptionValue::set` keeps a pin as it is and leaves a new one to `App::apply`. `stepped` and `flipped` are the terminal look's: follow at the left, pin at the right.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 0bc1072..d0bc8dd 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -160,6 +160,20 @@ pub enum Mode {
     Dark,
 }
 
+/// A theme as the window sets one: a tile of macOS and Windows, or the
+/// terminal look's follow and pin. Unlike [`Theme`] it holds no name: a
+/// pin set from the window is the theme Omarchy shows, whose name the app
+/// knows and the settings do not.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub enum ThemeChoice {
+    Unset,
+    System,
+    Light,
+    Dark,
+    Omarchy,
+    Pinned,
+}
+
 impl Theme {
     /// What the file's text says. An empty text says nothing.
     pub fn of(text: &str) -> Self {
@@ -195,6 +209,18 @@ impl Theme {
         }
     }
 
+    /// What the window shows as set: a pin without its name.
+    pub fn choice(&self) -> ThemeChoice {
+        match self {
+            Self::Unset => ThemeChoice::Unset,
+            Self::System => ThemeChoice::System,
+            Self::Light => ThemeChoice::Light,
+            Self::Dark => ThemeChoice::Dark,
+            Self::Omarchy => ThemeChoice::Omarchy,
+            Self::Pinned(_) => ThemeChoice::Pinned,
+        }
+    }
+
     /// The terminal look's pin: `None` follows the desktop. A value of
     /// macOS and Windows follows it too.
     pub fn pin(&self) -> Option<&str> {
@@ -358,6 +384,7 @@ pub enum OptionId {
     LogDays,
     /// Whether a new agent is told of: `mcp.notify`.
     AgentsNotify,
+    Theme,
     RowHeight,
     Zebra,
     ColumnTypes,
@@ -368,11 +395,12 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's and then
     /// the Agents pane's.
-    pub const ALL: [OptionId; 12] = [
+    pub const ALL: [OptionId; 13] = [
         Self::PageSize,
         Self::Timestamps,
         Self::GroupDigits,
         Self::ValueTags,
+        Self::Theme,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
@@ -389,15 +417,17 @@ impl OptionId {
         Self::GroupDigits,
         Self::ValueTags,
     ];
-    /// The Appearance pane's, in the order it has them: its Grid
-    /// section's, then its Environment section's.
-    pub const APPEARANCE: [OptionId; 5] = [
+    /// The Appearance pane's, in the order it has them: its Theme
+    /// section's, its Grid section's, then its Environment section's.
+    pub const APPEARANCE: [OptionId; 6] = [
+        Self::Theme,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
         Self::TableList,
         Self::EnvBar,
     ];
+    pub const THEME: [OptionId; 1] = [Self::Theme];
     pub const GRID: [OptionId; 4] = [
         Self::RowHeight,
         Self::Zebra,
@@ -418,6 +448,7 @@ impl OptionId {
             Self::AgentsOn => Key::McpEnabled,
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
+            Self::Theme => Key::Theme,
             Self::RowHeight => Key::RowHeight,
             Self::Zebra => Key::Zebra,
             Self::ColumnTypes => Key::ColumnTypes,
@@ -442,6 +473,7 @@ pub enum OptionValue {
     AgentsOn(bool),
     LogDays(u32),
     AgentsNotify(bool),
+    Theme(ThemeChoice),
     RowHeight(RowHeight),
     /// `None` is the look's default.
     Zebra(Option<bool>),
@@ -460,6 +492,7 @@ impl OptionValue {
             Self::AgentsOn(_) => OptionId::AgentsOn,
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
+            Self::Theme(_) => OptionId::Theme,
             Self::RowHeight(_) => OptionId::RowHeight,
             Self::Zebra(_) => OptionId::Zebra,
             Self::ColumnTypes(_) => OptionId::ColumnTypes,
@@ -478,6 +511,18 @@ impl OptionValue {
             Self::AgentsOn(on) => settings.mcp_enabled = on,
             Self::LogDays(days) => settings.mcp_log_days = days,
             Self::AgentsNotify(on) => settings.mcp_notify = on,
+            Self::Theme(choice) => {
+                settings.theme = match choice {
+                    ThemeChoice::Unset => Theme::Unset,
+                    ThemeChoice::System => Theme::System,
+                    ThemeChoice::Light => Theme::Light,
+                    ThemeChoice::Dark => Theme::Dark,
+                    ThemeChoice::Omarchy => Theme::Omarchy,
+                    // A pin keeps its name. A new one is named by
+                    // `App::apply`, which knows Omarchy's theme.
+                    ThemeChoice::Pinned => return,
+                }
+            }
             Self::RowHeight(height) => settings.row_height = height,
             Self::Zebra(on) => settings.zebra = on,
             Self::ColumnTypes(on) => settings.column_types = on,
@@ -733,6 +778,7 @@ impl Settings {
             OptionId::AgentsOn => OptionValue::AgentsOn(self.mcp_enabled),
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
+            OptionId::Theme => OptionValue::Theme(self.theme.choice()),
             OptionId::RowHeight => OptionValue::RowHeight(self.row_height),
             OptionId::Zebra => OptionValue::Zebra(self.zebra),
             OptionId::ColumnTypes => OptionValue::ColumnTypes(self.column_types),
@@ -787,6 +833,12 @@ impl Settings {
                 OptionValue::LogDays(next.unwrap_or(days))
             }
             OptionId::AgentsNotify => OptionValue::AgentsNotify(forward),
+            // The terminal look's two: follow at the left, pin at the right.
+            OptionId::Theme => OptionValue::Theme(if forward {
+                ThemeChoice::Pinned
+            } else {
+                ThemeChoice::Omarchy
+            }),
             OptionId::RowHeight => {
                 let at = RowHeight::ALL
                     .iter()
@@ -822,6 +874,11 @@ impl Settings {
             OptionId::AgentsOn => Some(OptionValue::AgentsOn(!self.mcp_enabled)),
             OptionId::LogDays => None,
             OptionId::AgentsNotify => Some(OptionValue::AgentsNotify(!self.mcp_notify)),
+            // As the terminal look draws it: a pin, or the desktop followed.
+            OptionId::Theme => Some(OptionValue::Theme(match self.theme.pin() {
+                Some(_) => ThemeChoice::Omarchy,
+                None => ThemeChoice::Pinned,
+            })),
             OptionId::RowHeight => None,
             // Unset, it is flipped from off: the window flips the
             // settings as it shows them, with the look's default in.
```

- [ ] **Step 4: Pin the theme of the moment**

```diff
diff --git a/src/app.rs b/src/app.rs
index baf151b..7e4646e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1808,6 +1808,10 @@ impl App {
                     dialog.row = row;
                 }
             }
+            // A pin set from the window: the settings know no name for it.
+            Action::SetOption(crate::settings::OptionValue::Theme(
+                crate::settings::ThemeChoice::Pinned,
+            )) => self.pin_theme(),
             // The server's switch, turned on: the person's own doing, as
             // Turn on is, which keeps a change made outside that is held.
             Action::SetOption(crate::settings::OptionValue::AgentsOn(true)) => {
@@ -3743,6 +3747,18 @@ impl App {
         self.save_settings();
     }
 
+    /// Pins the theme Omarchy shows now, by its name. A pin stays as it
+    /// is. With no theme of Omarchy's to name (another desktop, or none
+    /// read yet), nothing is pinned: the file pins one by name.
+    fn pin_theme(&mut self) {
+        if self.settings.theme.pin().is_some() {
+            return;
+        }
+        if let Some(name) = self.desktop_theme.current.clone() {
+            self.change_settings(|settings| settings.theme = crate::settings::Theme::Pinned(name));
+        }
+    }
+
     /// Replaces the settings and does what the ones that changed ask for.
     /// Every change comes through here, from the app or from the file, and
     /// nothing is written here: saving is [`App::change_settings`]'s.
diff --git a/src/model.rs b/src/model.rs
index cb3fdca..3587b79 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2170,6 +2170,7 @@ impl SettingsPane {
                 SettingsRow::Option(OptionId::ValueTags),
             ],
             Self::Appearance => &[
+                SettingsRow::Option(OptionId::Theme),
                 SettingsRow::Option(OptionId::RowHeight),
                 SettingsRow::Option(OptionId::Zebra),
                 SettingsRow::Option(OptionId::ColumnTypes),
```

- [ ] **Step 5: The rows**

`shown` gives the theme as the look reads it, and the keys' copy of the settings pins the theme of the moment, as the app will. A key or a click that sets the theme the row shows already does nothing, so a macOS value that the row shows as followed stays in the file. The theme row is two segments with its hint; the palette row is eight swatches of the drawn palette (the board's background, accent, green, yellow, red, magenta and its two value colours) and where they came from. The sheet's matches name the option: its tiles come in Task 4, drawn on their own.

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index a9731d4..e97d977 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -16,7 +16,10 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
-use crate::settings::{EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps};
+use crate::settings::{
+    EnvBar, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, Theme, ThemeChoice,
+    Timestamps,
+};
 
 /// The label of an option's row. English: the caller translates, and the
 /// terminal look lower-cases.
@@ -29,6 +32,7 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::AgentsOn => "Let agents use Tabletist",
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
+        OptionId::Theme => "Theme",
         OptionId::RowHeight => "Row density",
         OptionId::Zebra => "Alternate row shading",
         OptionId::ColumnTypes => "Show column types",
@@ -52,6 +56,7 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
+        | OptionId::Theme
         | OptionId::TableList => label(option),
     }
 }
@@ -69,6 +74,7 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         | OptionId::GroupDigits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify
+        | OptionId::Theme
         | OptionId::RowHeight
         | OptionId::Zebra => None,
     }
@@ -103,14 +109,35 @@ pub(super) fn is_on(value: OptionValue) -> bool {
 
 /// The settings as the window shows them: the server's switch as it is in
 /// force, off where a change made outside the app that turned it on is
-/// held, and the stripes as the look draws them while they are not set.
+/// held, the stripes as the look draws them while they are not set, and
+/// the theme as the look reads it: a value of the other looks' is the
+/// desktop followed.
 pub(super) fn shown(app: &App) -> Settings {
     let mut settings = app.settings.clone();
     settings.mcp_enabled = app.agents_on();
     settings.zebra = Some(settings.zebra_in(app.look.terminal));
+    settings.theme = match (app.look.terminal, settings.theme.mode()) {
+        (true, _) if settings.theme.pin().is_some() => settings.theme,
+        (true, _) => Theme::Omarchy,
+        (false, Mode::System) => Theme::System,
+        (false, Mode::Light) => Theme::Light,
+        (false, Mode::Dark) => Theme::Dark,
+    };
     settings
 }
 
+/// Puts `value` in the window's own copy of the settings, as `App::apply`
+/// will put it in the app's: a new pin is Omarchy's theme of the moment.
+fn set_shown(app: &App, value: OptionValue, settings: &mut Settings) {
+    value.set(settings);
+    if value == OptionValue::Theme(ThemeChoice::Pinned)
+        && settings.theme.pin().is_none()
+        && let Some(name) = &app.desktop_theme.current
+    {
+        settings.theme = Theme::Pinned(name.clone());
+    }
+}
+
 /// A timestamp as the grid shows one at each precision.
 pub(super) fn sample_timestamp(timestamps: Timestamps) -> &'static str {
     match timestamps {
@@ -182,6 +209,14 @@ pub(super) fn choices(option: OptionId, settings: &Settings) -> Vec<(Said, Optio
             (Word("off"), OptionValue::AgentsNotify(false)),
             (Word("on"), OptionValue::AgentsNotify(true)),
         ],
+        // The terminal look's: macOS and Windows draw their tiles.
+        OptionId::Theme => vec![
+            (
+                Word("follow omarchy"),
+                OptionValue::Theme(ThemeChoice::Omarchy),
+            ),
+            (Word("pin"), OptionValue::Theme(ThemeChoice::Pinned)),
+        ],
         OptionId::RowHeight => vec![
             (Word("compact"), OptionValue::RowHeight(RowHeight::Compact)),
             (Word("regular"), OptionValue::RowHeight(RowHeight::Regular)),
@@ -413,8 +448,16 @@ fn keys(app: &App, ctx: &egui::Context, dialog: &SettingsDialog, actions: &mut V
                         Asked::Step { forward } => Some(settings.stepped(option, forward)),
                         _ => settings.flipped(option),
                     };
-                    if let Some(value) = value {
-                        value.set(&mut settings);
+                    // A step to the theme the row shows already changes
+                    // nothing: a value of the other looks', shown as the
+                    // desktop followed, stays in the file.
+                    let same = |value: &OptionValue| {
+                        option == OptionId::Theme
+                            && asked != Asked::Reset
+                            && *value == settings.value(option)
+                    };
+                    if let Some(value) = value.filter(|value| !same(value)) {
+                        set_shown(app, value, &mut settings);
                         // Stripes put back to the look's are drawn as the
                         // look draws them: the next key flips those.
                         settings.zebra = Some(settings.zebra_in(app.look.terminal));
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 630fa44..2be89e0 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -475,6 +475,8 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         | OptionId::Zebra
         | OptionId::ColumnTypes => SWITCH,
         OptionId::LogDays => look.control_height,
+        // The Theme section is tiles, drawn on their own: no row's.
+        OptionId::Theme => 0.0,
     }
 }
 
@@ -614,6 +616,8 @@ fn control(
                 info
             });
         }
+        // The Theme section is tiles, drawn on their own: no row's.
+        OptionId::Theme => {}
     }
 }
 
@@ -692,8 +696,10 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
             palette.dim,
         ),
         OptionId::ValueTags => return tags(ui, at, right, settings.value_tags, skin),
-        // What the Appearance pane's rows say is their small print.
-        OptionId::AgentsOn
+        // What the Appearance pane's rows say is their small print, and its
+        // tiles have their own line.
+        OptionId::Theme
+        | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
         | OptionId::RowHeight
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index cd2ea72..af78721 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -7,7 +7,7 @@
 use egui::emath::GuiRounding as _;
 use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};
 
-use crate::app::App;
+use crate::app::{App, PinState};
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::Command;
 use crate::model::{Action, SettingsDialog, SettingsPane, SettingsRow};
@@ -38,6 +38,8 @@ const LABEL: f32 = 280.0;
 const VALUE: f32 = 320.0;
 /// The space at each side of a segment's word.
 const SEGMENT_PAD: f32 = 6.0;
+/// One of the palette's swatches.
+const SWATCH: egui::Vec2 = vec2(18.0, 14.0);
 /// The file's pane: this wide, or this share of the window where that is
 /// less, and not there in a window narrower than `FILE_FROM`.
 const FILE: f32 = 620.0;
@@ -53,6 +55,32 @@ pub(super) struct Skin<'a> {
     /// Whether a connection window is open: what Enter on the Agents
     /// pane's status and log rows shows the tab of.
     pub(super) window: bool,
+    /// What the theme's rows say of the desktop.
+    pub(super) desktop: Desktop<'a>,
+}
+
+/// What the Appearance pane's theme rows say beside their values: Omarchy's
+/// theme of the moment, and how a pin stands.
+#[derive(Clone, Copy)]
+pub(super) struct Desktop<'a> {
+    /// The name of Omarchy's theme of the moment, where it is known.
+    pub(super) current: Option<&'a str>,
+    /// The pin as the settings write it, and how it stands.
+    pub(super) pin: Option<(&'a str, PinState)>,
+    /// Whether Omarchy's palette is followed: else the system's light or
+    /// dark is.
+    pub(super) follows_omarchy: bool,
+}
+
+impl<'a> Desktop<'a> {
+    pub(super) fn of(app: &'a App) -> Self {
+        let pin = app.settings.theme.pin().zip(app.pin_state());
+        Self {
+            current: app.desktop_theme.current.as_deref(),
+            pin,
+            follows_omarchy: app.themes.follows_omarchy(),
+        }
+    }
 }
 
 impl Skin<'_> {
@@ -94,6 +122,7 @@ pub(super) fn show(
         palette: &app.palette,
         locale: app.locale,
         window: app.agents_window().is_some(),
+        desktop: Desktop::of(app),
     };
     let screen = ctx.content_rect();
     let id = egui::Id::new("settings");
@@ -341,7 +370,8 @@ fn appearance_rows(
     );
     let mut index = 0;
     for (heading_text, top, options) in [
-        ("Grid", 8.0, &OptionId::GRID[..]),
+        ("Theme", 8.0, &OptionId::THEME[..]),
+        ("Grid", 12.0, &OptionId::GRID[..]),
         ("Environment", 12.0, &OptionId::ENVIRONMENT[..]),
     ] {
         heading(ui, heading_text, top, skin);
@@ -357,6 +387,116 @@ fn appearance_rows(
             );
             index += 1;
         }
+        // Under the theme, the colours it gave: no row of the cursor's.
+        if options == OptionId::THEME {
+            palette_row(ui, columns, skin, actions);
+        }
+    }
+}
+
+/// The colours the theme gave, eight of them as the board has them, and
+/// where they were read from. Nothing to change: the cursor passes it.
+fn palette_row(
+    ui: &mut Ui,
+    (label_width, value_width): (f32, f32),
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let palette = skin.palette;
+    let (row, left, right, _) = row_frame(ui, (usize::MAX, "Palette"), false, skin, actions);
+    let label_width = label_width.min(right - left);
+    let value_width = value_width.min(right - left - label_width);
+    let swatches = [
+        palette.window,
+        palette.accent,
+        palette.success,
+        palette.warning,
+        palette.danger,
+        palette.magenta,
+        palette.info,
+        palette.orange,
+    ];
+    let mut x = left + label_width;
+    for color in swatches {
+        let swatch = Rect::from_center_size(pos2(x + SWATCH.x / 2.0, row.center().y), SWATCH);
+        ui.painter().rect(
+            swatch,
+            CornerRadius::same(2),
+            color,
+            Stroke::new(1.0, palette.outline),
+            StrokeKind::Inside,
+        );
+        x += SWATCH.x + 4.0;
+    }
+    let hint = palette_source(skin).layout(ui.ctx());
+    let hint_left = left + label_width + value_width;
+    if hint_left + hint.width() <= right {
+        hint.paint_left(ui.painter(), hint_left, row.center().y);
+        widgets::announce(
+            ui,
+            Rect::from_min_size(pos2(hint_left, row.top()), vec2(hint.width(), ROW)),
+            hint.galley.text(),
+        );
+    }
+}
+
+/// Where the drawn colours came from: an Omarchy theme, a palette file the
+/// pin names, or the app's own light or dark.
+fn palette_source(skin: &Skin) -> Text {
+    let Skin {
+        look,
+        palette,
+        desktop,
+        ..
+    } = *skin;
+    let said = match desktop.pin {
+        Some((_, PinState::Drawn(pinned))) if pinned.omarchy => {
+            skin.say("read from the omarchy theme")
+        }
+        Some((name, PinState::Drawn(_))) => skin.say("read from {name}").replace("{name}", name),
+        _ if desktop.follows_omarchy => skin.say("read from the omarchy theme"),
+        _ => skin.say("tabletist's own light or dark"),
+    };
+    Text::one(look, TextRole::OBody, &said, palette.dim)
+}
+
+/// What the theme's row says beside its value: the theme drawn now, and
+/// what changes it, or why a pin is not drawn.
+fn theme_hint(skin: &Skin) -> Text {
+    let Skin {
+        look,
+        palette,
+        desktop,
+        ..
+    } = *skin;
+    let role = TextRole::OBody;
+    match desktop.pin {
+        Some((name, PinState::Drawn(_))) => {
+            let said = skin.say("now: {name} · pinned").replace("{name}", name);
+            Text::one(look, role, &said, palette.dim)
+        }
+        Some((name, PinState::Missing)) => {
+            let said = skin.say(if desktop.follows_omarchy {
+                "“{name}” not found · following omarchy"
+            } else {
+                "“{name}” not found · following the system"
+            });
+            Text::one(look, role, &said.replace("{name}", name), palette.warning)
+        }
+        // A pin still being read is drawn as the desktop until it is.
+        Some((_, PinState::Reading)) | None if desktop.follows_omarchy => {
+            let name = look.label(desktop.current.unwrap_or("Omarchy"));
+            let said = skin.say("now: {name} · switches with omarchy-theme");
+            Text::one(look, role, &said.replace("{name}", &name), palette.dim)
+        }
+        Some((_, PinState::Reading)) | None => {
+            let said = skin.say(if palette.dark {
+                "now: dark, from the system"
+            } else {
+                "now: light, from the system"
+            });
+            Text::one(look, role, &said, palette.dim)
+        }
     }
 }
 
@@ -705,6 +845,7 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
         }
         OptionId::Timestamps
         | OptionId::GroupDigits
+        | OptionId::Theme
         | OptionId::RowHeight
         | OptionId::TableList
         | OptionId::EnvBar => {
@@ -864,6 +1005,7 @@ fn option_row(
         }
         OptionId::Timestamps
         | OptionId::GroupDigits
+        | OptionId::Theme
         | OptionId::RowHeight
         | OptionId::TableList
         | OptionId::EnvBar => {
@@ -1027,6 +1169,7 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
             )
             .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
         OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
+        OptionId::Theme => theme_hint(skin),
         OptionId::RowHeight => dim(&skin.say("1 · 1.5 · 2 lines")),
         OptionId::Zebra => Text::new(look),
         OptionId::ColumnTypes => dim(&skin.say("second header line")),
@@ -1216,7 +1359,12 @@ fn choice(
     }
     if button.clicked() {
         actions.push(Action::SelectSettingsRow(index));
-        actions.push(Action::SetOption(value));
+        // The theme it shows already: a value of the other looks', shown
+        // as the desktop followed, stays in the file.
+        let shown = matches!(press, Press::Value { set: true });
+        if !(shown && value.option() == OptionId::Theme) {
+            actions.push(Action::SetOption(value));
+        }
     }
 }
 
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- theme_settings appearance_settings settings::tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 7: Look at it**

Compare the pane with the "Omarchy – Settings, Appearance" board by eye, through a throwaway scene in `src/shots.rs`: the Omarchy look at 1920 × 1080, the catalog following a Tokyo Night palette, `desktop_theme.current` set, Settings on the Appearance pane. Render it with `cargo test --locked --features shots --lib shots::<scene> -- --ignored`, keep it local, and remove it before the commit.

- [ ] **Step 8: Commit**

```bash
git add src/settings.rs src/model.rs src/app.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/appearance_settings_tests.rs src/ui/mod.rs src/ui/theme_settings_tests.rs
git commit -m "Follow or pin the Omarchy theme from Settings › Appearance"
```

---

### Task 4: The tiles of macOS and Windows

**Files:**
- Modify: `src/ui/settings/sheet.rs`, `src/app.rs`, `src/testing.rs`, `src/ui/theme_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The harness tells its frames the system's light or dark, as the window does (`Harness::system_theme`), and the tests run the app's logic as the window runs it between frames, to read the window commands it sends.

```diff
diff --git a/src/testing.rs b/src/testing.rs
index 12d3d01..b98f013 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -19,6 +19,9 @@ pub struct Harness {
     pub viewport_commands: Vec<egui::ViewportCommand>,
     /// Whether the window reports itself fullscreen.
     pub fullscreen: bool,
+    /// The system's light or dark, as every frame is told it: unknown
+    /// until a test says.
+    pub system_theme: Option<egui::Theme>,
     /// Asks the window to close in the next frame, as its close button or
     /// the desktop does. Taken by that frame.
     pub close_requested: bool,
@@ -71,6 +74,7 @@ impl Harness {
             screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
             events,
             focused: self.window_focused,
+            system_theme: self.system_theme,
             ..Default::default()
         };
         input
@@ -141,6 +145,7 @@ impl Harness {
             screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
             events,
             focused: self.window_focused,
+            system_theme: self.system_theme,
             ..Default::default()
         };
         input
@@ -1473,6 +1478,7 @@ impl Harness {
             scale: 1.0,
             viewport_commands: Vec::new(),
             fullscreen: false,
+            system_theme: None,
             close_requested: false,
             painted: Vec::new(),
             text_rects: Vec::new(),
diff --git a/src/ui/theme_settings_tests.rs b/src/ui/theme_settings_tests.rs
index 143c277..cbfcc7f 100644
--- a/src/ui/theme_settings_tests.rs
+++ b/src/ui/theme_settings_tests.rs
@@ -1,5 +1,6 @@
 //! Settings › Appearance, its Theme section: the terminal look's follow
-//! and pin with the palette they give, and the tiles of macOS and Windows.
+//! and pin with the palette they give, and the tiles of macOS and Windows
+//! with the window's own light or dark.
 
 use egui::accesskit::{Role, Toggled};
 
@@ -46,10 +47,25 @@ fn following() -> Harness {
         true,
     );
     harness.app.desktop_theme.current = Some("Tokyo Night".into());
-    harness.app.logic(&harness.ctx.clone());
+    logic(&mut harness);
     harness
 }
 
+/// One pass of the app's logic, as the window runs it between frames: the
+/// window commands it sent. The system's light or dark is the last
+/// frame's.
+fn logic(harness: &mut Harness) -> Vec<egui::ViewportCommand> {
+    let mut input = egui::RawInput::default();
+    input.viewports.entry(egui::ViewportId::ROOT).or_default();
+    let app = &mut harness.app;
+    let output = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
+    output
+        .viewport_commands
+        .get(&egui::ViewportId::ROOT)
+        .cloned()
+        .unwrap_or_default()
+}
+
 /// The settings with `theme`, as an edit of the file sets it.
 fn with_theme(harness: &mut Harness, theme: Theme) {
     let settings = Settings {
@@ -57,7 +73,7 @@ fn with_theme(harness: &mut Harness, theme: Theme) {
         ..harness.app.settings.clone()
     };
     harness.app.apply_settings(settings);
-    harness.app.logic(&harness.ctx.clone());
+    logic(harness);
 }
 
 /// What the backend read of the pin `name`: an Omarchy theme's palette.
@@ -73,7 +89,7 @@ fn read(harness: &mut Harness, name: &str, palette: Option<Palette>) {
         )),
     };
     harness.app.apply(Action::Backend(Event::Theme(read)));
-    harness.app.logic(&harness.ctx.clone());
+    logic(harness);
 }
 
 fn key(harness: &mut Harness, key: egui::Key, text: &str) {
@@ -232,7 +248,133 @@ fn a_pin_found_nowhere_is_said_beside_it() {
         )),
     };
     harness.app.apply(Action::Backend(Event::Theme(file)));
-    harness.app.logic(&harness.ctx.clone());
+    logic(&mut harness);
     assert!(shows(&mut harness, "now: ~/mine/Sunset.json · pinned"));
     assert!(shows(&mut harness, "read from ~/mine/Sunset.json"));
 }
+
+/// The sheet's look on its Appearance pane, the system light.
+fn sheet(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness.system_theme = Some(egui::Theme::Light);
+    on_the_pane(&mut harness);
+    logic(&mut harness);
+    harness
+}
+
+#[test]
+fn the_sheet_sets_the_theme_with_its_tiles() {
+    let hint = if cfg!(target_os = "windows") {
+        "System follows Windows."
+    } else {
+        "System follows macOS, including Auto."
+    };
+    for look in [Look::macos(), Look::standard()] {
+        let mut harness = sheet(look);
+        assert!(shows(&mut harness, "THEME"), "{}", look.name);
+        assert!(shows(&mut harness, hint), "{}", look.name);
+        // A radio group named Theme, its tiles in the board's order.
+        let tree = harness.settle();
+        let group = crate::testing::node(&tree, "Theme", Role::RadioGroup).expect("the group");
+        let (_, group) = tree.nodes.iter().find(|(id, _)| *id == group).unwrap();
+        let tiles: Vec<_> = group
+            .children()
+            .iter()
+            .filter_map(|child| tree.nodes.iter().find(|(id, _)| id == child))
+            .filter(|(_, node)| node.role() == Role::RadioButton)
+            .filter_map(|(_, node)| node.label())
+            .collect();
+        assert_eq!(tiles, ["System", "Light", "Dark"], "{}", look.name);
+        assert_eq!(set(&mut harness, "System"), Some(true));
+        let saves = settings_saves(&harness);
+        harness.click("Dark");
+        harness.settle();
+        assert_eq!(harness.app.settings.theme, Theme::Dark);
+        assert_eq!(set(&mut harness, "Dark"), Some(true));
+        logic(&mut harness);
+        assert_eq!(
+            harness.app.palette,
+            Palette::dark(),
+            "whatever the system says"
+        );
+        harness.click("Light");
+        harness.click("System");
+        harness.settle();
+        assert_eq!(harness.app.settings.theme, Theme::System);
+        assert!(
+            harness
+                .app
+                .settings_file
+                .text
+                .contains("theme = \"system\"")
+        );
+        assert_eq!(settings_saves(&harness), saves + 3);
+        logic(&mut harness);
+        assert_eq!(harness.app.palette, Palette::light());
+    }
+}
+
+#[test]
+fn a_mode_the_settings_name_is_the_windows_own_and_system_gives_it_back() {
+    let mut harness = sheet(Look::macos());
+    assert_eq!(
+        logic(&mut harness),
+        Vec::new(),
+        "the system's, as it starts"
+    );
+    harness.click("Dark");
+    assert_eq!(
+        logic(&mut harness),
+        vec![egui::ViewportCommand::SetTheme(egui::SystemTheme::Dark)]
+    );
+    // Told once.
+    assert_eq!(logic(&mut harness), Vec::new());
+    harness.click("System");
+    assert_eq!(
+        logic(&mut harness),
+        vec![egui::ViewportCommand::SetTheme(
+            egui::SystemTheme::SystemDefault
+        )]
+    );
+    // The terminal look never tells the window: its palette is the theme's.
+    let mut harness = following();
+    let settings = Settings {
+        theme: Theme::Dark,
+        ..harness.app.settings.clone()
+    };
+    harness.app.apply_settings(settings);
+    assert_eq!(logic(&mut harness), Vec::new());
+}
+
+#[test]
+fn a_value_of_the_terminal_look_is_the_system_here_and_stays_in_the_file() {
+    let mut harness = sheet(Look::macos());
+    with_theme(&mut harness, Theme::Pinned("Tokyo Night".into()));
+    assert_eq!(set(&mut harness, "System"), Some(true));
+    assert_eq!(harness.app.palette, Palette::light());
+    harness.click("Row density: Compact · 28");
+    harness.settle();
+    assert!(
+        harness
+            .app
+            .settings_file
+            .text
+            .contains("theme = \"Tokyo Night\"")
+    );
+    // A tile clicked is this look's choice: the other's value goes.
+    harness.click("Dark");
+    harness.settle();
+    assert_eq!(harness.app.settings.theme, Theme::Dark);
+}
+
+#[test]
+fn reset_to_defaults_puts_the_theme_back_to_the_desktops() {
+    let mut harness = sheet(Look::macos());
+    with_theme(&mut harness, Theme::Dark);
+    harness.app.apply(Action::ResetSettings);
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    harness.settle();
+    assert_eq!(harness.app.settings.theme, Theme::Unset);
+    assert!(!harness.app.settings_file.text.contains("theme ="));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- theme_settings`
Expected: three fail. The reset test passes already: Task 3 put the theme among the pane's options.

```
test ui::theme_settings_tests::the_sheet_sets_the_theme_with_its_tiles ... FAILED
test ui::theme_settings_tests::a_value_of_the_terminal_look_is_the_system_here_and_stays_in_the_file ... FAILED
test ui::theme_settings_tests::a_mode_the_settings_name_is_the_windows_own_and_system_gives_it_back ... FAILED
test result: FAILED. 7 passed; 3 failed; 0 ignored; 0 measured; 2599 filtered out; finished in 0.02s
```

- [ ] **Step 3: The tiles**

A radio group named Theme, its three tiles radio buttons: each a window in its palette, the one set ringed in the accent and its word in the strong weight, and the hint after them. Only a click on a tile that is not set changes the theme, so a terminal-look value shown as System stays in the file until another tile is clicked.

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index 2be89e0..eddb1af 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -9,7 +9,9 @@ use egui::{CornerRadius, Rect, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2,
 use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
-use crate::settings::{EnvBar, OptionId, OptionValue, RowHeight, Settings, TableList, Timestamps};
+use crate::settings::{
+    EnvBar, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, ThemeChoice, Timestamps,
+};
 use crate::theme::{Look, Palette};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
@@ -47,6 +49,9 @@ const MENU: f32 = 120.0;
 /// A segment's height, and the switch's.
 const SEGMENT: f32 = 22.0;
 const SWITCH: f32 = 18.0;
+/// A theme tile's window, and the space round it inside the tile's ring.
+const TILE: egui::Vec2 = vec2(120.0, 72.0);
+const TILE_PAD: f32 = 6.0;
 /// What a short window leaves of the rows at the least: about one of them.
 const ROWS_MIN: f32 = 56.0;
 /// The footer: the space over and under what it says, and between its
@@ -153,6 +158,7 @@ pub(super) fn show(
                     &mut pane,
                     ("Appearance", None),
                     &[
+                        ("Theme", &OptionId::THEME),
                         ("Grid", &OptionId::GRID),
                         ("Environment color", &OptionId::ENVIRONMENT),
                     ],
@@ -332,22 +338,156 @@ fn content(
                     ui.add_space(2.0);
                 }
                 for option in options.iter().copied() {
-                    row(
-                        ui,
-                        option,
-                        index < last,
-                        column,
-                        settings,
-                        skin,
-                        actions,
-                        None,
-                    );
+                    if option == OptionId::Theme {
+                        tiles(ui, settings, skin, actions);
+                    } else {
+                        row(
+                            ui,
+                            option,
+                            index < last,
+                            column,
+                            settings,
+                            skin,
+                            actions,
+                            None,
+                        );
+                    }
                     index += 1;
                 }
             }
         });
 }
 
+/// The Theme section: a tile for each mode, a radio group named by the
+/// option, the one set ringed in the accent, and what System does after
+/// them. Each tile draws a window in its palette: the sidebar's colour
+/// round the content's, System's half one and half the other.
+fn tiles(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let name = skin.say(label(OptionId::Theme));
+    let label_role = TextRole::UiBody;
+    let line = label_role.row_height(ui.ctx(), look.faces);
+    let tile = vec2(
+        TILE.x + 2.0 * TILE_PAD,
+        TILE_PAD + TILE.y + 6.0 + line + TILE_PAD,
+    );
+    let height = 6.0 + tile.y + 12.0 + 1.0;
+    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
+    // The tiles in a Ui of their own, whose node is their parent in the
+    // accessibility tree: a radio group with the option's name.
+    let place = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() - 1.0));
+    let group = ui.new_child(egui::UiBuilder::new().id_salt("theme").max_rect(place));
+    group
+        .ctx()
+        .accesskit_node_builder(group.unique_id(), |node| {
+            node.set_role(egui::accesskit::Role::RadioGroup);
+            node.set_label(name.as_str());
+        });
+    let (light, dark) = (Palette::light(), Palette::dark());
+    let mut x = rect.left();
+    let top = rect.top() + 6.0;
+    for (mode, word) in [
+        (Mode::System, "System"),
+        (Mode::Light, "Light"),
+        (Mode::Dark, "Dark"),
+    ] {
+        let area = Rect::from_min_size(pos2(x, top), tile);
+        x += tile.x + 12.0;
+        let word = skin.say(word);
+        let set = settings.theme.mode() == mode;
+        let button = group.interact(area, group.id().with(word.as_str()), Sense::click());
+        button.widget_info(|| WidgetInfo::selected(WidgetType::RadioButton, true, set, &word));
+        focus::hint(&group, &button, area, Ring::Outer { radius: 10 });
+        if button.clicked() && !set {
+            let choice = match mode {
+                Mode::System => ThemeChoice::System,
+                Mode::Light => ThemeChoice::Light,
+                Mode::Dark => ThemeChoice::Dark,
+            };
+            actions.push(Action::SetOption(OptionValue::Theme(choice)));
+        }
+        let painter = group.painter();
+        if set {
+            let ring = Stroke::new(2.0, palette.accent);
+            painter.rect_stroke(
+                area,
+                CornerRadius::same(10),
+                ring,
+                egui::StrokeKind::Outside,
+            );
+        }
+        let window = Rect::from_min_size(area.min + vec2(TILE_PAD, TILE_PAD), TILE);
+        let content = Rect::from_min_max(
+            window.min + vec2(22.0, 12.0),
+            pos2(window.right() - 10.0, window.bottom()),
+        );
+        let halves = |rect: Rect| rect.split_left_right_at_x(window.center().x);
+        let parts = match mode {
+            Mode::System => {
+                let ((left, right), (inner_left, inner_right)) = (halves(window), halves(content));
+                vec![
+                    (left, light.panel),
+                    (right, dark.panel),
+                    (inner_left, light.window),
+                    (inner_right, dark.window),
+                ]
+            }
+            Mode::Light => vec![(window, light.panel), (content, light.window)],
+            Mode::Dark => vec![(window, dark.panel), (content, dark.window)],
+        };
+        let window_radius = CornerRadius::same(8);
+        let content_radius = CornerRadius {
+            nw: 6,
+            ..CornerRadius::ZERO
+        };
+        for (index, (part, color)) in parts.into_iter().enumerate() {
+            // The window's corners are its own; the content's only at its
+            // top left, as the board draws it. System's halves are cut
+            // down the middle.
+            let mut radius = if part.top() == window.top() {
+                window_radius
+            } else {
+                content_radius
+            };
+            if mode == Mode::System {
+                let left = index % 2 == 0;
+                if left {
+                    radius.ne = 0;
+                    radius.se = 0;
+                } else {
+                    radius.nw = 0;
+                    radius.sw = 0;
+                }
+            }
+            painter.rect_filled(part, radius, color);
+        }
+        let border = Stroke::new(1.0, palette.border);
+        painter.rect_stroke(window, window_radius, border, egui::StrokeKind::Inside);
+        let role = if set {
+            TextRole::UiBodyStrong
+        } else {
+            label_role
+        };
+        let text = Text::one(look, role, &word, palette.text).layout(group.ctx());
+        text.paint_left(
+            painter,
+            area.center().x - text.width() / 2.0,
+            window.bottom() + 6.0 + line / 2.0,
+        );
+    }
+    let hint = if cfg!(target_os = "windows") {
+        "System follows Windows."
+    } else {
+        "System follows macOS, including Auto."
+    };
+    let hint = Text::one(look, TextRole::Secondary, &skin.say(hint), palette.dim).layout(ui.ctx());
+    let at = x - 12.0 + 8.0;
+    if at + hint.width() <= rect.right() {
+        write(ui, at, top + tile.y / 2.0, &hint);
+    }
+}
+
 /// How wide the rows' labels have it: `LABEL`, or what the widest small
 /// print of `options` takes in a face that writes it wider, so that it
 /// stays on one line. A sheet too narrow for that and the controls takes
```

- [ ] **Step 4: Tell the window**

The window's own parts (the title bar, the system's menus and panels) take the mode the settings name. System gives them back to the system; the terminal look never sends it.

```diff
diff --git a/src/app.rs b/src/app.rs
index 7e4646e..55922d4 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -135,6 +135,9 @@ pub struct App {
     follow_desktop: bool,
     /// The settings named another theme since the last `logic`.
     theme_changed: bool,
+    /// The light or dark the window's own parts were last told to take:
+    /// the system's until the settings name one.
+    window_theme: egui::SystemTheme,
     /// What the backend last read of Omarchy's themes: the name of the
     /// theme of the moment, and the pinned theme.
     pub desktop_theme: crate::desktop_theme::Read,
@@ -261,6 +264,7 @@ impl App {
             system_theme: None,
             follow_desktop: false,
             theme_changed: false,
+            window_theme: egui::SystemTheme::SystemDefault,
             desktop_theme: crate::desktop_theme::Read::default(),
             desktop_read: false,
             pin_told: None,
@@ -5363,6 +5367,17 @@ impl App {
             }
         }
         self.tell_missing_pin(ctx);
+        // macOS and Windows: the window's own parts (its title bar, the
+        // menus and panels of the system) in the mode the settings name.
+        let mode = match (self.look.terminal, self.settings.theme.mode()) {
+            (true, _) | (false, crate::settings::Mode::System) => egui::SystemTheme::SystemDefault,
+            (false, crate::settings::Mode::Light) => egui::SystemTheme::Light,
+            (false, crate::settings::Mode::Dark) => egui::SystemTheme::Dark,
+        };
+        if mode != self.window_theme {
+            self.window_theme = mode;
+            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(mode));
+        }
         // A hidden window draws no frame: its close request is answered
         // here, or nothing would hold it back.
         self.hold_close(ctx);
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --locked --lib -- theme_settings`
Expected: PASS. Then the four checks above.

- [ ] **Step 6: Look at it**

Compare the sheet with the "macOS – Settings, Appearance" board by eye, in light and dark, through a throwaway scene as in Task 3 (the macOS look at 1440 × 1000). Remove it before the commit. On a Mac, pick Dark with the system light: the title bar and a context menu turn dark too, and System gives them back.

- [ ] **Step 7: Commit**

```bash
git add src/ui/settings/sheet.rs src/app.rs src/testing.rs src/ui/theme_settings_tests.rs
git commit -m "Pick System, Light or Dark from Settings › Appearance on macOS and Windows"
```

---

### Task 5: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`

- [ ] **Step 1: Describe the setting, the key and pinning**

````diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 82c30eb..1c858ea 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -24,9 +24,8 @@ launch, are on the
   tabs do in Safari. In full screen the window's buttons are hidden and
   the bar takes the whole width.
 - **Light and dark follow the system.** Tabletist changes with the
-  Appearance setting while it runs.
-  [Your own colors]({% link _guide/omarchy.md %}#your-own-colors) work on a
-  Mac too.
+  Appearance setting while it runs. **Settings › Appearance** can keep it
+  light or dark whatever the system says.
 - **The app menu has the usual two.** **Tabletist → About Tabletist**
   shows the version, and **Tabletist → Settings…** opens Settings.
 
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index ed6c228..82f10fa 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -251,7 +251,7 @@ offers.
 | `j` `k` | Move |
 | `h` `l` | Change the option |
 | Space | Toggle the option |
-| Ctrl+H, Ctrl+L | The pane before or after: General, Agents |
+| Ctrl+H, Ctrl+L | The pane before or after: General, Appearance, Agents |
 | Enter | On the Agents pane: show the Agents tab, Connections or the log, from the status, access or log row |
 | `y` | On the Agents pane: copy the command that connects Claude Code |
 | Ctrl+E | Open `settings.toml` in your editor |
@@ -271,20 +271,45 @@ system's light or dark setting.
 Tabletist maps an Omarchy theme onto its own palette with a template. The
 one it ships is
 [`contrib/omarchy/tabletist.json.tpl`](https://github.com/igor-alexandrov/tabletist/blob/main/contrib/omarchy/tabletist.json.tpl)
-in the source.
+in the source; a `tabletist.json.tpl` of your own in
+`~/.config/omarchy/themed` is used instead.
+
+### Pinning a theme
+
+**Settings › Appearance** shows the theme drawn now and the palette it
+gave. `l` on the theme row pins the theme Omarchy shows: Tabletist keeps
+it when you switch Omarchy's theme, and `h` follows Omarchy again.
+
+A theme can also be pinned in `settings.toml`, by the name of an Omarchy
+theme, yours or one Omarchy ships:
+
+```toml
+[appearance]
+theme = "Tokyo Night"
+```
 
 ### Your own colors
 
 A palette file in the `themes` folder of Tabletist's settings folder can
-replace the desktop's colors. Name the file in `settings.toml`:
+replace the desktop's colors. Pin it by its file's name:
 
 ```toml
 [appearance]
 theme = "my-theme.json"
 ```
 
-A named file wins over the Omarchy theme and over the system's light or
-dark setting. The folder and the file are described in
+A name alone (`"my-theme"`) is an Omarchy theme's first, and the
+palette's only where no Omarchy theme has it. A file or an Omarchy
+theme's folder anywhere is pinned by its path, starting with `/` or `~/`:
+
+```toml
+[appearance]
+theme = "~/themes/my-theme.json"
+```
+
+A pin that cannot be found or read is told in the status line once a
+connection is open, and on the theme row of Settings › Appearance, and
+Tabletist follows the desktop instead. The folder and the file are described in
 [Settings and Files]({% link _reference/settings-and-files.md %}).
 
 ## Settings
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index ac392ee..768c8b8 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -17,11 +17,13 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **Numbers** | `1240.50` or `1,240.50`. Grouping is for display only: a copy gives the raw value | `1240.50` |
 | **Value tags** | Colors for enum, CHECK and boolean columns | On |
 
-A second pane, **Appearance**, sets how the grids and the connection bar
-are drawn. Each change shows in every open window at once.
+A second pane, **Appearance**, sets the theme and how the grids and the
+connection bar are drawn. Each change shows in every open window at once.
 
 | Option | Choices | Default |
 | --- | --- | --- |
+| **Theme** (macOS and Windows) | System, Light or Dark. System follows the system's light or dark as it changes, Auto on macOS too | System |
+| **Theme** (Omarchy) | Follow Omarchy, or pin the theme it shows now, which then stays when Omarchy switches. The row says the theme drawn, and under it the palette it gave | Follow Omarchy |
 | **Row density** (Omarchy: row height) | Compact, Regular or Roomy. On macOS and Windows a step of 8 points either side of Regular; in the Omarchy look one, one and a half or two lines. The grid keeps the row at its top | Regular |
 | **Alternate row shading** (Omarchy: alternate rows) | On or off, in the table grid and the SQL editor's results | Off on macOS and Windows, on in the Omarchy look |
 | **Show column types** (Omarchy: column types) | On or off: the second line of the grid's header | On |
@@ -87,7 +89,7 @@ bar = "tint"
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
-| `appearance.theme` | The name of a palette file in the `themes` folder | Not set |
+| `appearance.theme` | On macOS and Windows `"system"`, `"light"` or `"dark"`. In the Omarchy look `"omarchy"`, or a theme to pin: an Omarchy theme's name (`"Tokyo Night"`), a palette file in the `themes` folder (`"Nord.json"`; its name alone where no Omarchy theme has it), or a path that starts with `/` or `~/` to a palette file or an Omarchy theme's folder. A value meant for the other looks is drawn as not set and stays in the file | Not set: the system's light or dark, or the Omarchy theme |
 | `appearance.grid.row_height` | `"compact"`, `"regular"` or `"roomy"` | `"regular"` |
 | `appearance.grid.zebra` | `true` or `false`. Written once you set it; without it, the look's default | Not set: off on macOS and Windows, on in the Omarchy look |
 | `appearance.grid.column_types` | `true` or `false` | `true` |
@@ -97,7 +99,8 @@ bar = "tint"
 | `mcp.log_days` | 1 to 3650 days. How long the log of agents' calls keeps a call | `30` |
 | `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |
 
-`show_system_schemas` and `theme` are set only in the file. The SQL
+`show_system_schemas` is set only in the file, and so is a pin by a
+name or a path other than the Omarchy theme of the moment. The SQL
 editor's **Limit** and **Timeout** menus set the two `editor` keys.
 
 A line that cannot be read is ignored, keeps its default, and is reported
````

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/_guide/omarchy.md docs/_guide/macos.md
git commit -m "Describe the theme setting and pinning on the website"
```

---

## After the plan

Open one pull request for the step. Its description: a summary; the reading of `appearance.theme` in each look (the table at the top, with the answer given); the departures table; and how it was tried by hand (Task 2 Step 7 on Omarchy, Task 4 Step 6 on a Mac, or that it was not). No screenshot goes to GitHub unblurred.
