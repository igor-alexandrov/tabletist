# Settings › Appearance, step 3: Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the Text section to Settings › Appearance in both looks. macOS and Windows get an Interface size (Small · 12, Default · 13, Large · 14) for the interface's own text, and a data font and size (IBM Plex Mono or JetBrains Mono, 11 to 16 points) for grid cells, the SQL editor and JSON. The Omarchy look gets one font (the system's monospace through fontconfig, or a fontconfig name) and one size (9 to 24) for everything. egui's own zoom keys go, and keymap commands change and reset `appearance.font_size` and write it. About shows the bundled fonts' licenses.

**Architecture:** Three keys join `[appearance]`: `interface_size`, `font` and `font_size`. A look's `Faces` stops being a bare typeface and carries the sizes and the data face the settings give (`Look::with_text`), so every call that already passes `look.faces` to a `TextRole` gets them. `TextRole::spec` keeps the design's sizes; `TextRole::spec_in(faces)` scales them, the data roles by the data's size and the others by the interface's. The terminal look keeps its roles as they are and draws the whole window larger or smaller, through egui's zoom factor, set from the settings in `App::logic`. On macOS JetBrains Mono is bundled beside Plex Mono, so a change of face lays the text out again and loads nothing. On Omarchy the font is read through fontconfig at the start, as the desktop's monospace always was, and on the backend after a change (`Command::ReadFont`). The keys are three new keymap commands, read under a dialog too.

**Tech Stack:** Rust, egui (crmne fork: `Options::zoom_with_keyboard`, `Context::set_zoom_factor`, `FontDefinitions`), fontconfig's `fc-match`, the bundled IBM Plex and JetBrains Mono, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-17-settings-appearance-design.md`, section "Step 3: Text". This plan is step 3. Steps 1 (Grid and Environment) and 2 (Theme) are merged; step 4 (Preview) gets its own plan.

---

## Decided before code: how a role gets its size

`TextRole::spec` is a `const fn` of fixed sizes, and every view draws through `role.format(look.faces, …)`, `role.font_id(look.faces)` and their kin: 256 calls. This plan builds the following, **for your yes**:

- **macOS and Windows: the roles are scaled.** `Faces` becomes a struct: the typeface (the old enum, now `Typeface`), the interface's size, the data's face and the data's size. `Look::with_text(&settings)` fills it in; the app's look is set again whenever the settings change. `TextRole::spec_in(faces)` multiplies the design's size: a data role (grid cells, value tags, JSON and its chip, inspector values, the SQL editor's code, the completion list) by the data's size over 12.5, every other role by the interface's body size over 13. Line heights, letter spacing and the capitals' centring follow from the size. The calls stay as they are; the eleven that compared `look.faces == Faces::Plex` compare the typeface.
- **The terminal look: the window is zoomed.** One size for everything, 9 to 24, is drawn by egui's zoom factor (`size / 13`), and the roles keep their sizes. The terminal look's heights are hand-sized in points across the views (about seventy of them: bars, rows, controls, gutters), as a terminal's cells are; text scaled alone would be cut in its bars past about 16 points. The cost: away from 13 a 1 px rule is drawn at a fraction of a pixel, softer than now.
- The other way, roles scaled on Omarchy too, needs the seventy heights to follow the size first. Say if you want that instead.

## Where this departs from the boards and the spec

For a yes or a no before code:

| The board or the spec asks | Step 3 builds | Why |
|---|---|---|
| macOS data size "11 to 16" (spec); the board's menu shows "12.5 pt" | 11 to 16 by half points, 12.5 to start with. Omarchy: 9 to 24 by whole points, 13 to start with | 12.5 is today's grid cell: nobody's grid changes on upgrade, as in step 1. |
| The Omarchy board's file pane writes `font = "monospace"` and `font_size = 13` | `font` and `font_size` are written once they are set; reset takes them out | Their defaults depend on the look and the writer does not know the look, as with `zebra` and `theme`. |
| Omarchy's font row: "system monospace", "now: jetbrains mono, from fontconfig" | The same words. The family is fontconfig's own name, lower-cased ("now: jetbrainsmono nerd font, from fontconfig" on Omarchy). A name fontconfig does not have: "“Fira Code” not found · now: …" in the warning colour. Nothing read (tests, the demo, no `fc-match`): "now: jetbrains mono, bundled" | The board draws one state. |
| "or a fontconfig name" | Set in the file (`ctrl+e`). The row has no choices: `h`, `l` and Space do nothing on it, `R` puts back the system's monospace | The board's row is a value, not choices, and has no field to type a name in. |
| "A missing font falls back to the default and says so" | macOS: the Data font row's small print becomes "“Fira Code” is not bundled: IBM Plex Mono is used", in the warning colour, and the menu shows IBM Plex Mono. Omarchy: on the font row (above), and a line in the log. Not in the status line | The rows are where the font is set; step 2 also told a missing pin in the status line, because its spec asked for it. |
| `monospace` in the file on macOS | The default face, with nothing said | It is the terminal look's default, the same key in the same file. |
| The keys ⌘= ⌘- ⌘0 (macOS), `ctrl+=` `ctrl+-` (Omarchy) | Those, and Ctrl+= Ctrl+- Ctrl+0 on Windows (the Mac's keys with Ctrl, as every Windows key is). They hold under a dialog too, the Settings window among them. On macOS and Windows they change the data's size, not the interface's | The Omarchy board says "ctrl+= / ctrl+- anywhere". Omarchy's reset is `R` on the size row, as the spec gives it no key. |
| (egui's zoom took ⌘+ as well: ⌘⇧= and the keypad's +) | ⌘= and ⌘- alone. The keypad's − shrinks, its + does nothing | The spec names ⌘=; the keymap spells no `+` key, and adding one is a change of its own. Say if you want it. |
| (an egui zoom saved by an earlier version) | Put back to 1 on macOS and Windows, to the size on Omarchy, at the first frame | The settings own the size now. |
| Windows draws the macOS sheet (steps 1 and 2) | The Text section with Interface size, and the Data font row with the size's menu alone | Windows draws its data in Inter, the interface's face, and bundles no monospace: offering one would cost 450 KB and change the look. Its grid cells get a role of their own, `DataBody` (Inter at 13 points, as `UiBody` drew them), so that the data's size reaches them. |
| "The OFL notice of the bundled fonts is shown with the app's licence" | A "Font licenses" link under About's license line unfolds the notices of the fonts Tabletist bundles itself (`assets/fonts`), whole and selectable: IBM Plex, JetBrains Mono and Noto Sans Symbols 2 on macOS, JetBrains Mono on Linux. Windows has none of these: no link. Inter, which fastframe compiles in on every platform, and egui's own fonts are not listed | No board draws About. The OFL asks that its notice goes with the font. The crates' fonts come with those crates' licenses; listing them is a change of its own. Say if you want it in this step. |
| (the keys in a text field) | The size keys act while a field has the keyboard too, its text left as it is | No field takes those chords. |
| Interface size "Sidebar, tabs, buttons, inspector labels" | Every role that is not data, the grid's header among them. Control and row heights stay | Large's 14 points fit the 28 point controls; the board names examples, not a list. |
| Data font "Grid cells, SQL editor, JSON" | Those, and what macOS draws in the grid cell's role elsewhere: the Agents tab's command and log cells, the connection dialog's field values, the picker's host. They follow the data's face and size too | They are values sent to or read from a server, drawn as data; giving them a role of their own is a change of those views. |
| (rows at a large data size) | A grid row on macOS and Windows is at least as tall as a line of its data and a point: on Windows, Compact rows grow from 18 to 19, 20 and 21 points between 14 and 16. At the default size every row is as it was | Text taller than its row would run into the next. Omarchy's rows grow with the window's zoom. |
| The macOS subtitle | Still the title alone | Step 4, with the preview (as in steps 1 and 2). |

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found. On a 720 point window the Data font row ran past the macOS sheet: the font's menu now gives way, down to 100 points. On Windows, Compact rows were shorter than a line of data at 15 and 16 points. A `font_size` past a hundred was a line ignored rather than a size clamped, as the docs and the other number keys have it. `" monospace "` with spaces was "not bundled" on macOS. The About notices stayed unfolded when About closed another way than its own button. And the grid's marks for empty values, found once for each set of faces, were kept when a fontconfig read changed the terminal's face: they are found again.

Left as they are, for the record:
- The Interface size segments, like Row density's already, reach a little past the macOS sheet's edge in a 720 point window.
- A grid's column widths, fitted to its text when its first page came, are kept when the data's size changes: a value that no longer fits is cut short until the table is opened again. A width the user dragged is theirs either way.
- A fontconfig name is taken as found when the family fontconfig gives begins with it, ignoring case and spaces (`JetBrains Mono` is `JetBrainsMono Nerd Font`, and `Iosevka` is `Iosevka Term`).
- The start still waits for `fc-match`, up to half a second a run, as it did: three runs for a font found, four for a name fontconfig does not have (two seconds at worst, where it was a second and a half). A change of font is read on the backend.
- The terminal look's file pane writes `font_size = 13` only once it is set, as above, and the size row's hint is the keymap's two keys.

## Files

| File | What it does after this plan |
|---|---|
| `src/settings.rs` | `InterfaceSize`, `FontSize` (half points), `FontSizes` (a look's range and step); `Settings::interface_size`, `font`, `font_size`; their keys, read and written; `font_size_in` and `stepped_font_size`; `OptionId` and `OptionValue` for the three. |
| `src/theme.rs` | `Typeface` (the old `Faces`), `DataFace`, and `Faces` with the sizes; `Look::with_text`; `install_with` and `set_terminal_faces`. |
| `src/typography.rs` | `TextRole::spec_in`, `is_data`, `DataBody`; every size, spacing and line height in the faces given. |
| `src/typography/fonts.rs` | JetBrains Mono on macOS; the terminal's faces handed in (`TerminalFaces`). |
| `src/typography/desktop_font.rs` | `read`: the font a fontconfig name gives, its family, and whether it was found. |
| `src/backend.rs` | `Command::ReadFont`, answered with `Event::Font`. |
| `src/app.rs` | The look's text from the settings; egui's styles set again; the zoom; the font read at the start and after a change; `Action::TextSize`. |
| `src/keymap.rs`, `src/ui/keys.rs`, `src/model.rs` | `LargerText`, `SmallerText`, `DefaultTextSize`, read wherever the keyboard is; the font and size rows on the Omarchy pane. |
| `src/ui/settings/{mod,sheet,terminal}.rs` | The Text section in both looks. |
| `src/ui/about.rs` | The fonts' licenses. |
| `src/ui/text_settings_tests.rs` | New: the section's and the keys' headless tests. |
| `src/ui/grid.rs` | Windows' data role; the marks for empty values found again after a font read. |
| `src/ui/{mod,picker,sql_text}.rs`, `src/ui/connect_dialog/mod.rs` | The typeface compared where `Faces` was. |
| `src/testing.rs`, `src/ui/data_view.rs`, `src/ui/appearance_settings_tests.rs` | The test look's text and fonts; `configure` given no desktop faces; step 1's tests two rows further down. |
| `docs/_reference/settings-and-files.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md` | The settings, the keys, the font. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The text keys in the file

**Files:**
- Modify: `src/settings.rs`

- [ ] **Step 1: Write the failing tests**

The new keys read as the boards write them, and a value a key cannot have is ignored by its line. A size is read to the half point and brought into the range of every look; each look then draws it in its own range and steps it by its own step. The file's every-key test and the line each key is found on take the three keys.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index c65fbf4..b9b1e54 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -1798,6 +1798,9 @@ mod tests {
             confirm_close_edits: false,
             show_system_schemas: true,
             theme: Theme::Pinned("My \"Nord\".json".into()),
+            interface_size: InterfaceSize::Large,
+            font: Some("JetBrains Mono".into()),
+            font_size: FontSize::of(14.5),
             sql_limit: 100,
             sql_timeout_secs: None,
             mcp_enabled: true,
@@ -1813,8 +1816,12 @@ mod tests {
 
     #[test]
     fn only_what_is_not_the_default_is_written_each_table_in_its_place() {
-        // The theme's note names the values of the look this platform draws.
+        // The theme's note and the font's name the values of the look this
+        // platform draws; Windows has none for the font.
         let note = Key::Theme.note().unwrap();
+        let fonts = Key::Font
+            .note()
+            .map_or(String::new(), |note| format!("  # {note}"));
         assert_eq!(
             all_set().to_toml(),
             format!(
@@ -1845,6 +1852,9 @@ notify = false
 
 [appearance]
 theme = \"My \\\"Nord\\\".json\"  # {note}
+interface_size = \"large\"  # small | default | large
+font = \"JetBrains Mono\"{fonts}
+font_size = 14.5
 
 [appearance.grid]
 row_height = \"roomy\"  # compact | regular | roomy
@@ -1929,11 +1939,14 @@ bar = \"label\"  # tint | label
                 (Key::McpLogDays, 22),
                 (Key::McpNotify, 23),
                 (Key::Theme, 26),
-                (Key::RowHeight, 29),
-                (Key::Zebra, 30),
-                (Key::ColumnTypes, 31),
-                (Key::TableList, 32),
-                (Key::EnvBar, 35),
+                (Key::InterfaceSize, 27),
+                (Key::Font, 28),
+                (Key::FontSize, 29),
+                (Key::RowHeight, 32),
+                (Key::Zebra, 33),
+                (Key::ColumnTypes, 34),
+                (Key::TableList, 35),
+                (Key::EnvBar, 38),
             ]
         );
     }
@@ -3268,4 +3281,115 @@ bar = \"label\"  # tint | label
             Theme::Omarchy
         );
     }
+
+    #[test]
+    fn the_text_keys_read_as_the_boards_write_them() {
+        // The Omarchy board's file.
+        let text = "[appearance]\ntheme = \"omarchy\"\nfont = \"monospace\"  # fontconfig name\n\
+                    font_size = 13\n";
+        let loaded = Settings::from_toml(text);
+        assert_eq!(loaded.invalid, Vec::<usize>::new());
+        assert_eq!(loaded.settings.font.as_deref(), Some("monospace"));
+        assert_eq!(loaded.settings.font_size, FontSize::of(13.0));
+        assert_eq!(loaded.lines[1..], [(Key::Font, 3), (Key::FontSize, 4)]);
+        // macOS's: a size between two points, and the interface's size.
+        let text = "[appearance]\ninterface_size = \"small\"\nfont = \"JetBrains Mono\"\n\
+                    font_size = 12.5\n";
+        let settings = Settings::from_toml(text).settings;
+        assert_eq!(settings.interface_size, InterfaceSize::Small);
+        assert_eq!(settings.font.as_deref(), Some("JetBrains Mono"));
+        assert_eq!(settings.font_size, FontSize::of(12.5));
+        // Neither look's font and size is written until it is set.
+        assert_eq!(Settings::default().to_toml(), HEADER);
+        let set = Settings {
+            font_size: FontSize::of(16.0),
+            ..Settings::default()
+        };
+        assert!(set.to_toml().ends_with("[appearance]\nfont_size = 16\n"));
+    }
+
+    #[test]
+    fn a_text_key_with_no_value_it_can_have_is_ignored_by_its_line() {
+        let text = "[appearance]\ninterface_size = \"huge\"\nfont = \"\"\nfont_size = \"big\"\n\
+                    [editor]\nsql_limit = 100\n";
+        let loaded = Settings::from_toml(text);
+        assert_eq!(loaded.invalid, vec![2, 3, 4]);
+        assert_eq!(loaded.settings.interface_size, InterfaceSize::Default);
+        assert_eq!(loaded.settings.font, None);
+        assert_eq!(loaded.settings.font_size, None);
+        assert_eq!(loaded.settings.sql_limit, 100);
+        // No size a font has: none, or nothing at all.
+        for size in ["0", "-3", "0.2", "nan", "inf"] {
+            let loaded = Settings::from_toml(&format!("[appearance]\nfont_size = {size}\n"));
+            assert_eq!(loaded.invalid, vec![2], "{size}");
+        }
+    }
+
+    #[test]
+    fn a_font_size_is_read_to_the_half_point_and_into_every_looks_range() {
+        let read = |size: &str| {
+            Settings::from_toml(&format!("[appearance]\nfont_size = {size}\n"))
+                .settings
+                .font_size
+                .map(FontSize::text)
+        };
+        assert_eq!(read("12.3").as_deref(), Some("12.5"));
+        assert_eq!(read("1_3.0").as_deref(), Some("13"));
+        assert_eq!(read("14").as_deref(), Some("14"));
+        // Past both looks' ranges: brought to the nearest end of them, as
+        // a number of the other keys is.
+        assert_eq!(read("30").as_deref(), Some("24"));
+        assert_eq!(read("100.5").as_deref(), Some("24"));
+        assert_eq!(read("99999999999").as_deref(), Some("24"));
+        assert_eq!(read("4").as_deref(), Some("9"));
+    }
+
+    #[test]
+    fn each_look_draws_the_font_size_in_its_own_range() {
+        let at = |size: Option<f64>| Settings {
+            font_size: size.and_then(FontSize::of),
+            ..Settings::default()
+        };
+        let drawn = |settings: &Settings, terminal: bool| settings.font_size_in(terminal).text();
+        // Each look's own, before any is set.
+        assert_eq!(drawn(&at(None), false), "12.5");
+        assert_eq!(drawn(&at(None), true), "13");
+        // macOS and Windows: 11 to 16, by halves. The terminal: 9 to 24,
+        // by whole points.
+        assert_eq!(drawn(&at(Some(20.0)), false), "16");
+        assert_eq!(drawn(&at(Some(9.0)), false), "11");
+        assert_eq!(drawn(&at(Some(20.0)), true), "20");
+        assert_eq!(drawn(&at(Some(12.5)), true), "13");
+        assert_eq!(drawn(&at(Some(13.5)), false), "13.5");
+        // Every size macOS's menu lists.
+        let sizes: Vec<String> = FontSizes::of(false)
+            .all()
+            .into_iter()
+            .map(FontSize::text)
+            .collect();
+        assert_eq!(
+            sizes,
+            [
+                "11", "11.5", "12", "12.5", "13", "13.5", "14", "14.5", "15", "15.5", "16"
+            ]
+        );
+    }
+
+    #[test]
+    fn the_font_size_steps_by_the_looks_step_and_stops_at_its_ends() {
+        let step = |size: f64, terminal: bool, larger: bool| {
+            let settings = Settings {
+                font_size: FontSize::of(size),
+                ..Settings::default()
+            };
+            settings.stepped_font_size(terminal, larger).text()
+        };
+        assert_eq!(step(12.5, false, true), "13");
+        assert_eq!(step(12.5, false, false), "12");
+        assert_eq!(step(16.0, false, true), "16");
+        assert_eq!(step(11.0, false, false), "11");
+        assert_eq!(step(13.0, true, true), "14");
+        assert_eq!(step(24.0, true, true), "24");
+        assert_eq!(step(9.0, true, false), "9");
+    }
 }
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings::tests`
Expected: the crate's tests do not compile:

```
error[E0560]: struct `settings::Settings` has no field named `interface_size`
error[E0433]: cannot find type `InterfaceSize` in this scope
error[E0560]: struct `settings::Settings` has no field named `font`
...
error: could not compile `tabletist` (lib test) due to 35 previous errors
```

- [ ] **Step 3: Read and write the three keys**

```diff
diff --git a/src/settings.rs b/src/settings.rs
index b9b1e54..7b11bb0 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -201,6 +201,135 @@ impl EnvBar {
     }
 }
 
+/// How large macOS's and Windows's own text is: the sidebar, the tabs,
+/// the buttons and the inspector's labels. The data has a size of its own.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
+pub enum InterfaceSize {
+    Small,
+    #[default]
+    Default,
+    Large,
+}
+
+impl InterfaceSize {
+    /// In the order the window draws them.
+    pub const ALL: [InterfaceSize; 3] = [Self::Small, Self::Default, Self::Large];
+
+    /// As the file says it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::Small => "small",
+            Self::Default => "default",
+            Self::Large => "large",
+        }
+    }
+
+    pub fn from_name(name: &str) -> Option<Self> {
+        Self::ALL.into_iter().find(|choice| choice.name() == name)
+    }
+
+    /// The size of the body text in points: 12, 13 or 14. Every other
+    /// style of the interface keeps its size to it.
+    pub fn points(self) -> f32 {
+        match self {
+            Self::Small => 12.0,
+            Self::Default => 13.0,
+            Self::Large => 14.0,
+        }
+    }
+}
+
+/// A font size in half points. The file writes it in points: `13`,
+/// `12.5`.
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
+pub struct FontSize(u8);
+
+impl FontSize {
+    pub const fn halves(halves: u8) -> Self {
+        Self(halves)
+    }
+
+    /// The nearest half point to `points`, a hundred at most: `None` for a
+    /// size no font has (none, less than a point, no end to it).
+    pub fn of(points: f64) -> Option<Self> {
+        let halves = (points * 2.0).round();
+        (points.is_finite() && halves >= 2.0).then_some(Self(halves.min(200.0) as u8))
+    }
+
+    pub fn points(self) -> f32 {
+        f32::from(self.0) / 2.0
+    }
+
+    /// As the file and the window write it: `13`, `12.5`.
+    pub fn text(self) -> String {
+        if self.0.is_multiple_of(2) {
+            (self.0 / 2).to_string()
+        } else {
+            format!("{}.5", self.0 / 2)
+        }
+    }
+}
+
+/// The sizes a look's text can take, and the step between them. macOS and
+/// Windows size the data from 11 to 16 points by halves, 12.5 to start
+/// with; the terminal look sizes everything from 9 to 24 by whole points,
+/// 13 to start with.
+#[derive(Clone, Copy, Debug, PartialEq, Eq)]
+pub struct FontSizes {
+    pub min: FontSize,
+    pub max: FontSize,
+    pub default: FontSize,
+    /// In half points.
+    step: u8,
+}
+
+impl FontSizes {
+    pub const fn of(terminal: bool) -> Self {
+        if terminal {
+            Self {
+                min: FontSize(18),
+                max: FontSize(48),
+                default: FontSize(26),
+                step: 2,
+            }
+        } else {
+            Self {
+                min: FontSize(22),
+                max: FontSize(32),
+                default: FontSize(25),
+                step: 1,
+            }
+        }
+    }
+
+    /// Every size, smallest first: what the macOS menu lists.
+    pub fn all(self) -> Vec<FontSize> {
+        (self.min.0..=self.max.0)
+            .step_by(usize::from(self.step))
+            .map(FontSize)
+            .collect()
+    }
+
+    /// `size` brought into the range, on a step: what the look draws a
+    /// size of the other look's at.
+    pub fn fit(self, size: FontSize) -> FontSize {
+        let size = size.0.clamp(self.min.0, self.max.0);
+        let steps = (size - self.min.0 + self.step / 2) / self.step;
+        FontSize((self.min.0 + steps * self.step).min(self.max.0))
+    }
+
+    /// The size a step up (`larger`) or down from `size`, no further than
+    /// the range's ends.
+    pub fn stepped(self, size: FontSize, larger: bool) -> FontSize {
+        let size = self.fit(size).0;
+        FontSize(if larger {
+            (size + self.step).min(self.max.0)
+        } else {
+            size.saturating_sub(self.step).max(self.min.0)
+        })
+    }
+}
+
 /// What `appearance.theme` says. macOS and Windows read `"system"`,
 /// `"light"` and `"dark"`; the terminal look reads `"omarchy"`, a theme's
 /// name and a path. A value meant for the other looks is drawn as the
@@ -345,6 +474,20 @@ pub struct Settings {
     /// name here, which the terminal look still reads.
     #[serde(rename = "custom_theme")]
     pub theme: Theme,
+    /// How large the interface's own text is, on macOS and Windows.
+    #[serde(skip)]
+    pub interface_size: InterfaceSize,
+    /// The font the data is drawn in on macOS (a bundled one, by name), or
+    /// the terminal look's one font (a fontconfig name). `None` until the
+    /// user sets it: the look's own, IBM Plex Mono or the system's
+    /// monospace.
+    #[serde(skip)]
+    pub font: Option<String>,
+    /// The data's size on macOS and Windows, or all of the terminal
+    /// look's text. `None` until the user sets it: the look's own
+    /// ([`FontSizes::default`]).
+    #[serde(skip)]
+    pub font_size: Option<FontSize>,
     /// Rows a SQL editor statement keeps (the Limit menu).
     pub sql_limit: u32,
     /// Seconds before a SQL editor run is cancelled; `None` waits forever.
@@ -389,6 +532,9 @@ pub enum Key {
     McpLogDays,
     McpNotify,
     Theme,
+    InterfaceSize,
+    Font,
+    FontSize,
     RowHeight,
     Zebra,
     ColumnTypes,
@@ -398,7 +544,7 @@ pub enum Key {
 
 impl Key {
     /// In the order the file has them: a table's keys together.
-    pub const ALL: [Key; 19] = [
+    pub const ALL: [Key; 22] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -413,6 +559,9 @@ impl Key {
         Self::McpLogDays,
         Self::McpNotify,
         Self::Theme,
+        Self::InterfaceSize,
+        Self::Font,
+        Self::FontSize,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
@@ -437,6 +586,9 @@ impl Key {
             Self::McpLogDays => ("mcp", "log_days"),
             Self::McpNotify => ("mcp", "notify"),
             Self::Theme => ("appearance", "theme"),
+            Self::InterfaceSize => ("appearance", "interface_size"),
+            Self::Font => ("appearance", "font"),
+            Self::FontSize => ("appearance", "font_size"),
             Self::RowHeight => ("appearance.grid", "row_height"),
             Self::Zebra => ("appearance.grid", "zebra"),
             Self::ColumnTypes => ("appearance.grid", "column_types"),
@@ -455,6 +607,13 @@ impl Key {
             Self::RowHeight => Some("compact | regular | roomy"),
             Self::TableList => Some("tree | flat"),
             Self::EnvBar => Some("tint | label"),
+            Self::InterfaceSize => Some("small | default | large"),
+            // The fonts of the look the platform draws, as the theme's.
+            // Windows draws its data in Inter, whatever the key says.
+            Self::Font if cfg!(target_os = "linux") => Some("fontconfig name"),
+            Self::Font if cfg!(target_os = "macos") => {
+                Some("\"IBM Plex Mono\" | \"JetBrains Mono\"")
+            }
             // The values of the look the platform draws: Linux has the
             // terminal look, the others the system's light and dark.
             Self::Theme => Some(if cfg!(target_os = "linux") {
@@ -1161,6 +1320,19 @@ impl Settings {
         self.zebra.unwrap_or(terminal)
     }
 
+    /// The size the terminal look (`terminal`) draws its text at, or the
+    /// others their data: as set, in the look's range, else the look's own.
+    pub fn font_size_in(&self, terminal: bool) -> FontSize {
+        let sizes = FontSizes::of(terminal);
+        sizes.fit(self.font_size.unwrap_or(sizes.default))
+    }
+
+    /// The font size a step up (`larger`) or down from the one the look
+    /// draws: what the font size keys set.
+    pub fn stepped_font_size(&self, terminal: bool, larger: bool) -> FontSize {
+        FontSizes::of(terminal).stepped(self.font_size_in(terminal), larger)
+    }
+
     /// The value one step from `option`'s, towards the right (`forward`)
     /// or the left, as the window draws its choices: the next page size
     /// (none past the ends), the segment on that side, a check on to the
@@ -1313,6 +1485,9 @@ impl Settings {
             Key::McpLogDays => self.mcp_log_days.to_string(),
             Key::McpNotify => self.mcp_notify.to_string(),
             Key::Theme => toml_string(self.theme.text()?),
+            Key::InterfaceSize => toml_string(self.interface_size.name()),
+            Key::Font => toml_string(self.font.as_deref()?),
+            Key::FontSize => self.font_size?.text(),
             Key::RowHeight => toml_string(self.row_height.name()),
             Key::Zebra => self.zebra?.to_string(),
             Key::ColumnTypes => self.column_types.to_string(),
@@ -1514,6 +1689,10 @@ impl Settings {
         self.mcp_log_days = self
             .mcp_log_days
             .clamp(Self::MIN_MCP_LOG_DAYS, Self::MAX_MCP_LOG_DAYS);
+        // Into the range of every look: each draws it in its own
+        // ([`Settings::font_size_in`]).
+        let sizes = FontSizes::of(true);
+        self.font_size = self.font_size.map(|size| size.clamp(sizes.min, sizes.max));
         self
     }
 
@@ -1529,6 +1708,11 @@ impl Settings {
             .and_then(|number| u32::try_from(number.clamp(0, i64::from(u32::MAX))).ok());
         let flag = value.as_bool();
         let text = value.as_str();
+        // A size in points, whole or not: `13`, `12.5`.
+        let points = number.map(f64::from).or_else(|| {
+            let float = value.as_float()?.as_str().replace('_', "");
+            float.parse::<f64>().ok()
+        });
         match key {
             Key::PageSize => number.map(|number| self.page_size = number),
             Key::Timestamps => text
@@ -1549,6 +1733,15 @@ impl Settings {
             Key::Theme => text
                 .filter(|text| !text.is_empty())
                 .map(|text| self.theme = Theme::of(text)),
+            Key::InterfaceSize => text
+                .and_then(InterfaceSize::from_name)
+                .map(|size| self.interface_size = size),
+            Key::Font => text
+                .filter(|text| !text.trim().is_empty())
+                .map(|text| self.font = Some(text.to_owned())),
+            Key::FontSize => points
+                .and_then(FontSize::of)
+                .map(|size| self.font_size = Some(size)),
             Key::RowHeight => text
                 .and_then(RowHeight::from_name)
                 .map(|height| self.row_height = height),
@@ -1677,6 +1870,9 @@ impl Settings {
             Key::McpLogDays => self.mcp_log_days = other.mcp_log_days,
             Key::McpNotify => self.mcp_notify = other.mcp_notify,
             Key::Theme => self.theme = other.theme.clone(),
+            Key::InterfaceSize => self.interface_size = other.interface_size,
+            Key::Font => self.font = other.font.clone(),
+            Key::FontSize => self.font_size = other.font_size,
             Key::RowHeight => self.row_height = other.row_height,
             Key::Zebra => self.zebra = other.zebra,
             Key::ColumnTypes => self.column_types = other.column_types,
@@ -1736,6 +1932,9 @@ impl Default for Settings {
             confirm_close_edits: true,
             show_system_schemas: false,
             theme: Theme::Unset,
+            interface_size: InterfaceSize::Default,
+            font: None,
+            font_size: None,
             sql_limit: 1_000,
             sql_timeout_secs: Some(30),
             mcp_enabled: false,
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings::tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/settings.rs
git commit -m "Read and write the text keys of settings.toml"
```

---

### Task 2: Every role at the settings' sizes, and the terminal zoomed

**Files:**
- Create: `src/ui/text_settings_tests.rs`
- Modify: `src/theme.rs`, `src/typography.rs`, `src/typography/fonts.rs`, `src/app.rs`, `src/testing.rs`, `src/keymap.rs`, `src/ui/grid.rs`, `src/ui/mod.rs`, `src/ui/picker.rs`, `src/ui/sql_text.rs`, `src/ui/connect_dialog/mod.rs`

- [ ] **Step 1: Write the failing tests**

On macOS and Windows the interface's roles keep to the body's size and the data's to the data's; before anything is set, and in the terminal look whatever is set, every role has the design's size. A data font is one of the bundled by its name, and `monospace` is the default. macOS draws its monospace data roles in the face named, and its interface's monospace in Plex, with both faces loaded; JetBrains Mono needs no shift for its capitals. Windows' grid cells are data too. In the app, egui's text styles follow the interface's size, the terminal look draws the whole window at its size, and a grid row on macOS and Windows holds a line of its data at every size, the design's rows kept at the default. The SQL editor's test asks for the line height in the faces drawn. The new test file is declared in `src/ui/mod.rs`.

```diff
diff --git a/src/theme.rs b/src/theme.rs
index d1f5a46..81fa945 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -839,6 +839,78 @@ mod tests {
         assert_eq!(data_role(&Look::macos()), TextRole::GridCell);
         assert_eq!(data_role(&Look::macos()).spec().kind, Kind::Mono);
         assert_eq!(data_role(&Look::standard()).spec().kind, Kind::Sans);
+        // Data in every look, which the data's size sizes.
+        for look in Look::ALL {
+            assert!(data_role(&look).is_data() || look.terminal, "{}", look.name);
+        }
+    }
+
+    /// The settings with the interface's size, the data font and its size
+    /// as given.
+    fn text(interface: InterfaceSize, font: Option<&str>, size: f64) -> Settings {
+        Settings {
+            interface_size: interface,
+            font: font.map(str::to_owned),
+            font_size: FontSize::of(size),
+            ..Settings::default()
+        }
+    }
+
+    #[test]
+    fn the_settings_size_the_interface_and_the_data_apart() {
+        use crate::typography::TextRole;
+        for look in [Look::macos(), Look::standard()] {
+            let faces = look
+                .with_text(&text(InterfaceSize::Large, None, 16.0))
+                .faces;
+            // The interface's roles keep to the body's 14 as the design's
+            // to 13, the data's to 16 as the design's to 12.5.
+            assert_eq!(TextRole::UiBody.size(faces), 14.0, "{}", look.name);
+            assert_eq!(TextRole::Secondary.size(faces), 12.0 * 14.0 / 13.0);
+            assert_eq!(TextRole::MonoSecondary.size(faces), 12.0 * 14.0 / 13.0);
+            assert_eq!(TextRole::GridCell.size(faces), 16.0);
+            assert_eq!(TextRole::DataBody.size(faces), 13.0 * 16.0 / 12.5);
+            assert_eq!(TextRole::Code.size(faces), 13.0 * 16.0 / 12.5);
+            // Lines keep to their size: the editor's 22 at 13.
+            let code = TextRole::Code.line_height(faces).unwrap();
+            assert!((code - 22.0 * 16.0 / 12.5).abs() < 1e-4, "{code}");
+        }
+        // Before any is set, and in the terminal look whatever is set, the
+        // design's.
+        for faces in [
+            Look::macos().faces,
+            Look::omarchy()
+                .with_text(&text(InterfaceSize::Large, Some("Fira Code"), 20.0))
+                .faces,
+        ] {
+            for role in TextRole::ALL {
+                assert_eq!(role.size(faces), role.spec().size, "{}", role.id());
+            }
+        }
+    }
+
+    #[test]
+    fn a_data_font_is_one_of_the_bundled_by_its_name() {
+        assert_eq!(DataFace::of(None), Ok(DataFace::PlexMono));
+        assert_eq!(
+            DataFace::of(Some("jetbrains mono")),
+            Ok(DataFace::JetBrainsMono)
+        );
+        assert_eq!(DataFace::of(Some("IBM Plex Mono")), Ok(DataFace::PlexMono));
+        // The terminal look's default names the desktop's own: the default
+        // here too, and nothing to say.
+        assert_eq!(DataFace::of(Some("monospace")), Ok(DataFace::PlexMono));
+        assert_eq!(DataFace::of(Some(" monospace ")), Ok(DataFace::PlexMono));
+        assert_eq!(
+            DataFace::of(Some(" JetBrains Mono ")),
+            Ok(DataFace::JetBrainsMono)
+        );
+        assert_eq!(DataFace::of(Some("Fira Code")), Err("Fira Code"));
+        let look = Look::macos().with_text(&text(InterfaceSize::Default, Some("Fira Code"), 12.5));
+        assert_eq!(look.faces.data, DataFace::PlexMono);
+        let look =
+            Look::macos().with_text(&text(InterfaceSize::Default, Some("JetBrains Mono"), 12.5));
+        assert_eq!(look.faces.data, DataFace::JetBrainsMono);
     }
 
     fn nord() -> CustomTheme {
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
index 692c9e2..815b130 100644
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -260,8 +260,11 @@ fn terminal(fonts: &mut FontDefinitions, desktop: bool) {
 #[cfg(test)]
 mod tests {
     use super::*;
+    use crate::theme::Faces;
     use crate::typography::{Text, TextRole};
 
+    const PLEX: Faces = Faces::of(Typeface::Plex);
+
     #[test]
     fn macos_draws_the_keys_its_shortcuts_name() {
         let mut harness = crate::testing::Harness::new();
@@ -271,7 +274,7 @@ mod tests {
             crate::typography::TextRole::Secondary,
             crate::typography::TextRole::Shortcut,
         ] {
-            let font = role.font_id(Faces::Plex);
+            let font = role.font_id(PLEX);
             assert!(
                 harness
                     .ctx
@@ -294,7 +297,7 @@ mod tests {
             crate::typography::TextRole::Secondary,
             crate::typography::TextRole::Shortcut,
         ] {
-            let font = role.font_id(Faces::Plex);
+            let font = role.font_id(PLEX);
             for command in crate::keymap::Command::ALL {
                 let label = keymap.label(crate::keymap::Layout::Mac, *command);
                 assert!(
@@ -346,7 +349,7 @@ mod tests {
                 TextRole::MonoSecondary,
             ] {
                 // Each of these symbols stands on the baseline, like the N.
-                let laid = Text::in_faces(Faces::Plex)
+                let laid = Text::in_faces(PLEX)
                     .add(role, "N⌘⌫⌦", egui::Color32::WHITE)
                     .layout(&ctx);
                 let letter = painted_bottom(&laid.galley, 'N');
@@ -374,7 +377,7 @@ mod tests {
                 .filter(|role| !role.id().starts_with("o-"));
             for role in plex {
                 for text in ["H", "⌘H"] {
-                    let laid = Text::in_faces(Faces::Plex)
+                    let laid = Text::in_faces(PLEX)
                         .add(role, text, egui::Color32::WHITE)
                         .layout(&ctx);
                     for line in [32.0, 100.3] {
@@ -382,7 +385,7 @@ mod tests {
                         let top =
                             ((line - laid.middle()) * pixels_per_point).round() / pixels_per_point;
                         let baseline = top + painted_bottom(&laid.galley, 'H');
-                        let middle = baseline - PLEX_CAPITALS * role.size() / 2.0;
+                        let middle = baseline - PLEX_CAPITALS * role.size(PLEX) / 2.0;
                         let off = (middle - line).abs() * pixels_per_point;
                         assert!(
                             off <= 0.51,
@@ -397,9 +400,9 @@ mod tests {
 
     #[test]
     fn only_plex_centres_text_by_its_capitals() {
-        assert_eq!(capitals(Faces::Plex), Some(PLEX_CAPITALS));
-        assert_eq!(capitals(Faces::Inter), None);
-        assert_eq!(capitals(Faces::Terminal), None);
+        assert_eq!(capitals(Typeface::Plex), Some(PLEX_CAPITALS));
+        assert_eq!(capitals(Typeface::Inter), None);
+        assert_eq!(capitals(Typeface::Terminal), None);
     }
 
     /// The metrics above are the bundled files' own.
@@ -442,6 +445,38 @@ mod tests {
         fonts.families[&family][0].clone()
     }
 
+    #[test]
+    fn macos_draws_data_in_the_face_the_settings_name() {
+        use crate::settings::Settings;
+        let fonts = configured(&Look::macos());
+        // Both faces are there, whichever is drawn.
+        for weight in [400, 500] {
+            let name = format!("jetbrains-mono-{weight}");
+            assert_eq!(first(&fonts, FontFamily::Name(name.as_str().into())), name);
+        }
+        let family = |faces: Faces, role: TextRole| match role.font_id(faces).family {
+            FontFamily::Name(name) => name.to_string(),
+            other => format!("{other:?}"),
+        };
+        let settings = Settings {
+            font: Some("JetBrains Mono".into()),
+            ..Settings::default()
+        };
+        let jetbrains = Look::macos().with_text(&settings).faces;
+        assert_eq!(family(jetbrains, TextRole::GridCell), "jetbrains-mono-400");
+        assert_eq!(
+            family(jetbrains, TextRole::CompletionMatch),
+            "jetbrains-mono-500"
+        );
+        assert_eq!(family(jetbrains, TextRole::Code), "jetbrains-mono-400");
+        // The interface's monospace stays Plex.
+        assert_eq!(family(jetbrains, TextRole::MonoSecondary), "plex-mono-400");
+        assert_eq!(family(PLEX, TextRole::GridCell), "plex-mono-400");
+        // JetBrains Mono's line centres on its capitals: no shift for them.
+        let laid = Text::in_faces(jetbrains).add(TextRole::GridCell, "H", egui::Color32::WHITE);
+        assert_eq!(laid.capitals, None);
+    }
+
     #[test]
     fn macos_draws_plex_and_omarchy_one_monospace_face() {
         let mac = configured(&Look::macos());
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 46dade3..2137d0d 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -65,6 +65,8 @@ pub mod states;
 pub mod structure;
 pub mod terminal_dialog;
 #[cfg(test)]
+mod text_settings_tests;
+#[cfg(test)]
 mod theme_settings_tests;
 pub mod toast;
 pub mod value_tags;
diff --git a/src/ui/sql_text.rs b/src/ui/sql_text.rs
index 2606415..abe561c 100644
--- a/src/ui/sql_text.rs
+++ b/src/ui/sql_text.rs
@@ -863,7 +863,7 @@ mod tests {
         assert_eq!(sections[0].format, format);
         for section in sections {
             assert_eq!(section.format.font_id, format.font_id);
-            assert_eq!(section.format.line_height, role.line_height());
+            assert_eq!(section.format.line_height, role.line_height(look.faces));
         }
         // An empty script is one line, as tall as any other.
         let empty = &galleys[1];
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
new file mode 100644
index 0000000..bd60796
--- /dev/null
+++ b/src/ui/text_settings_tests.rs
@@ -0,0 +1,94 @@
+//! Settings › Appearance, its Text section: the interface's size and the
+//! data font of macOS and Windows, the terminal look's one font and size,
+//! and the keys that change the size wherever the keyboard is.
+
+use crate::settings::{FontSize, InterfaceSize};
+use crate::testing::Harness;
+use crate::theme::Look;
+
+/// One pass of the app's logic, as the window runs it between frames.
+fn logic(harness: &mut Harness) {
+    let mut input = egui::RawInput::default();
+    input.viewports.entry(egui::ViewportId::ROOT).or_default();
+    let app = &mut harness.app;
+    let _ = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
+    // A zoom set there is the next frame's.
+    harness.settle();
+}
+
+fn sheet(look: Look) -> Harness {
+    let mut harness = Harness::new();
+    harness.set_look(look);
+    harness
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
+#[test]
+fn the_terminal_look_draws_the_whole_window_at_its_size() {
+    let mut harness = terminal();
+    logic(&mut harness);
+    assert_eq!(harness.ctx.zoom_factor(), 1.0);
+    harness
+        .app
+        .change_settings(|settings| settings.font_size = FontSize::of(19.5));
+    logic(&mut harness);
+    // To a whole point, as the look steps.
+    assert_eq!(harness.ctx.zoom_factor(), 20.0 / 13.0);
+    // The other looks size their text by role, at 1.
+    let mut harness = sheet(Look::macos());
+    harness
+        .app
+        .change_settings(|settings| settings.font_size = FontSize::of(16.0));
+    logic(&mut harness);
+    assert_eq!(harness.ctx.zoom_factor(), 1.0);
+}
+
+#[test]
+fn egui_text_styles_follow_the_interface_size() {
+    let mut harness = sheet(Look::macos());
+    harness
+        .app
+        .change_settings(|settings| settings.interface_size = InterfaceSize::Large);
+    logic(&mut harness);
+    let body = harness
+        .ctx
+        .global_style()
+        .text_styles
+        .get(&egui::TextStyle::Body)
+        .map(|font| font.size);
+    assert_eq!(body, Some(14.0));
+}
+
+#[test]
+fn a_grid_row_holds_a_line_of_its_data_at_every_size() {
+    for look in [Look::macos(), Look::standard()] {
+        let mut harness = sheet(look);
+        harness.settle();
+        for halves in 22..=32 {
+            for density in crate::settings::RowHeight::ALL {
+                let settings = crate::settings::Settings {
+                    font_size: Some(FontSize::halves(halves)),
+                    row_height: density,
+                    ..Default::default()
+                };
+                let look = look.with_text(&settings).with_grid(&settings);
+                let line = crate::ui::grid::data_role(&look).row_height(&harness.ctx, look.faces);
+                assert!(line <= look.row(), "{} {halves} {density:?}", look.name);
+            }
+        }
+    }
+    // The design's rows at its size: nobody's grid changes on upgrade.
+    let compact = crate::settings::Settings {
+        row_height: crate::settings::RowHeight::Compact,
+        ..Default::default()
+    };
+    assert_eq!(Look::macos().with_grid(&compact).row(), 29.0);
+    assert_eq!(Look::standard().with_grid(&compact).row(), 18.0);
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- theme::tests typography::fonts text_settings`
Expected: the crate's tests do not compile:

```
error[E0425]: cannot find type `InterfaceSize` in this scope
error[E0425]: cannot find type `Settings` in this scope
error[E0422]: cannot find struct, variant or union type `Settings` in this scope
...
error: could not compile `tabletist` (lib test) due to 46 previous errors
```

- [ ] **Step 3: The faces with their sizes**

`Faces` becomes the typeface with the sizes and the data face; the look takes them from the settings. Where a view compared the faces with `Faces::Plex`, it compares the typeface.

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index b084eb0..96e3b40 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -12,7 +12,7 @@
 
 use egui::Key;
 
-use crate::theme::{Faces, Look};
+use crate::theme::{Look, Typeface};
 
 /// How a look binds its keys and writes them.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
@@ -31,7 +31,7 @@ impl Layout {
     pub fn of(look: &Look) -> Self {
         if look.terminal {
             Self::Omarchy
-        } else if look.faces == Faces::Plex {
+        } else if look.faces.typeface == Typeface::Plex {
             Self::Mac
         } else {
             Self::Windows
diff --git a/src/theme.rs b/src/theme.rs
index 81fa945..528bd05 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -3,7 +3,7 @@
 
 use egui::{Color32, CornerRadius, Stroke, Vec2};
 
-use crate::settings::{Mode, RowHeight, Theme};
+use crate::settings::{FontSize, FontSizes, InterfaceSize, Mode, RowHeight, Settings, Theme};
 
 /// A palette file from the themes directory.
 pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
@@ -329,7 +329,7 @@ pub enum DataFont {
 
 /// The typefaces a look draws with (see [`crate::typography`]).
 #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
-pub enum Faces {
+pub enum Typeface {
     /// Inter, with egui's monospace.
     Inter,
     /// IBM Plex Sans and IBM Plex Mono.
@@ -338,6 +338,83 @@ pub enum Faces {
     Terminal,
 }
 
+/// The monospace face macOS draws data in: one of the two it bundles.
+#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
+pub enum DataFace {
+    #[default]
+    PlexMono,
+    JetBrainsMono,
+}
+
+impl DataFace {
+    /// In the order the menu lists them.
+    pub const ALL: [DataFace; 2] = [Self::PlexMono, Self::JetBrainsMono];
+
+    /// The face's name, as the menu and the file write it.
+    pub fn name(self) -> &'static str {
+        match self {
+            Self::PlexMono => "IBM Plex Mono",
+            Self::JetBrainsMono => "JetBrains Mono",
+        }
+    }
+
+    /// The face `font` names, whatever its case: the default for none, and
+    /// for `monospace`, which names the desktop's own (the terminal
+    /// look's default). A name of no bundled face is given back: it falls
+    /// back to the default, which says so.
+    pub fn of(font: Option<&str>) -> Result<Self, &str> {
+        let font = font.map(str::trim);
+        let Some(font) = font.filter(|font| !font.eq_ignore_ascii_case("monospace")) else {
+            return Ok(Self::default());
+        };
+        Self::ALL
+            .into_iter()
+            .find(|face| face.name().eq_ignore_ascii_case(font))
+            .ok_or(font)
+    }
+}
+
+/// A look's typefaces at the sizes the settings give them: what every
+/// text role is drawn in. The terminal look keeps its roles' sizes and
+/// draws the whole window larger or smaller
+/// ([`Settings::font_size_in`]).
+#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
+pub struct Faces {
+    pub typeface: Typeface,
+    /// The interface's own text, on macOS and Windows.
+    pub interface: InterfaceSize,
+    /// The data's face, on macOS.
+    pub data: DataFace,
+    /// The data's size, on macOS and Windows.
+    pub data_size: FontSize,
+}
+
+impl Faces {
+    /// `typeface` at the sizes it has before the settings say any.
+    pub const fn of(typeface: Typeface) -> Self {
+        Self {
+            typeface,
+            interface: InterfaceSize::Default,
+            data: DataFace::PlexMono,
+            data_size: FontSizes::of(false).default,
+        }
+    }
+
+    /// The faces at the sizes `settings` give, and the data in the face
+    /// they name. The terminal look's stay as they are.
+    pub fn with_text(self, settings: &Settings) -> Self {
+        if self.typeface == Typeface::Terminal {
+            return Self::of(self.typeface);
+        }
+        Self {
+            typeface: self.typeface,
+            interface: settings.interface_size,
+            data: DataFace::of(settings.font.as_deref()).unwrap_or_default(),
+            data_size: settings.font_size_in(false),
+        }
+    }
+}
+
 /// How the one primary button in a view draws.
 #[derive(Clone, Copy, Debug, PartialEq, Eq)]
 pub enum PrimaryStyle {
@@ -421,7 +498,7 @@ impl Look {
             sidebar_tinted: true,
             panel_separators: true,
             data_font: DataFont::Proportional,
-            faces: Faces::Inter,
+            faces: Faces::of(Typeface::Inter),
             primary: PrimaryStyle::Accent,
             terminal: false,
         }
@@ -449,7 +526,7 @@ impl Look {
             sidebar_tinted: true,
             panel_separators: true,
             data_font: DataFont::Monospace,
-            faces: Faces::Plex,
+            faces: Faces::of(Typeface::Plex),
             primary: PrimaryStyle::Ink,
             terminal: false,
         }
@@ -477,12 +554,21 @@ impl Look {
             sidebar_tinted: false,
             panel_separators: true,
             data_font: DataFont::Monospace,
-            faces: Faces::Terminal,
+            faces: Faces::of(Typeface::Terminal),
             primary: PrimaryStyle::Outline,
             terminal: true,
         }
     }
 
+    /// The look with its text as `settings` have it: the interface's size,
+    /// and the data's face and size.
+    pub fn with_text(self, settings: &Settings) -> Self {
+        Self {
+            faces: self.faces.with_text(settings),
+            ..self
+        }
+    }
+
     /// The look with the grids as `settings` have them: how dense their
     /// rows are, whether odd ones are shaded, and whether the header has
     /// the types' line.
@@ -497,16 +583,26 @@ impl Look {
 
     /// A grid row's height at the look's density. The terminal counts in
     /// lines, a regular row being one and a half: one or two. The others
-    /// step 8 either side, the regular row's rule kept in each.
+    /// step 8 either side, the regular row's rule kept in each, and are at
+    /// least as tall as a line of their data: a large data size makes the
+    /// short rows taller.
     pub fn row(&self) -> f32 {
         let regular = self.grid_row;
-        match (self.density, self.terminal) {
+        let row = match (self.density, self.terminal) {
             (RowHeight::Regular, _) => regular,
             (RowHeight::Compact, true) => regular * 2.0 / 3.0,
             (RowHeight::Roomy, true) => regular * 4.0 / 3.0,
             (RowHeight::Compact, false) => regular - 8.0,
             (RowHeight::Roomy, false) => regular + 8.0,
+        };
+        if self.terminal {
+            return row;
         }
+        // A line of Inter and of Plex Mono is at most 1.3 times the data's
+        // size. JetBrains Mono's is 1.32, where the rows of macOS are
+        // taller than that needs anyway.
+        let line = (self.faces.data_size.points() * 1.3).ceil();
+        row.max(line)
     }
 
     /// The look of the OS this build runs on. Every Linux desktop gets the
diff --git a/src/ui/connect_dialog/mod.rs b/src/ui/connect_dialog/mod.rs
index 250377d..ea65d15 100644
--- a/src/ui/connect_dialog/mod.rs
+++ b/src/ui/connect_dialog/mod.rs
@@ -20,7 +20,7 @@ use crate::env::{EnvColors, Environment, Platform, env_colors};
 use crate::i18n::{Locale, gettext};
 use crate::keymap::{Command, Scope};
 use crate::model::{Action, ConnectionForm, Dialog, SshAuthKind, SshHints, TestState};
-use crate::theme::{self, Faces, Icon, Look, Palette};
+use crate::theme::{self, Icon, Look, Palette, Typeface};
 use crate::typography::{Text, TextRole};
 use crate::ui::menu;
 use crate::ui::widgets::{self, ButtonSpec};
@@ -186,7 +186,7 @@ impl<'a> Skin<'a> {
 
     /// Where saved passwords go, as this platform calls it.
     fn keyring(&self) -> &'static str {
-        if self.look.faces == Faces::Plex {
+        if self.look.faces.typeface == Typeface::Plex {
             "Keychain"
         } else {
             "Keyring"
@@ -196,7 +196,7 @@ impl<'a> Skin<'a> {
     /// The name of the box that keeps the SSH secret: the database
     /// password's has the same text beside it.
     fn ssh_keyring(&self) -> &'static str {
-        if self.look.faces == Faces::Plex {
+        if self.look.faces.typeface == Typeface::Plex {
             "Keychain for the SSH secret"
         } else {
             "Keyring for the SSH secret"
@@ -204,7 +204,7 @@ impl<'a> Skin<'a> {
     }
 
     fn saved_hint(&self) -> &'static str {
-        if self.look.faces == Faces::Plex {
+        if self.look.faces.typeface == Typeface::Plex {
             "Saved in the keychain"
         } else {
             "Saved in the keyring"
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index 7d2bb03..db34417 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -36,9 +36,9 @@ const MAX_INITIAL_WIDTH: f32 = 331.0;
 /// A numeric key column's width, as the design's grids fix it: 64 on
 /// macOS, 56 in the terminal (0 elsewhere: sized to its content).
 fn key_width(look: &crate::theme::Look) -> f32 {
-    match (look.terminal, look.faces) {
+    match (look.terminal, look.faces.typeface) {
         (true, _) => 56.0,
-        (false, crate::theme::Faces::Plex) => 64.0,
+        (false, crate::theme::Typeface::Plex) => 64.0,
         _ => 0.0,
     }
 }
@@ -276,7 +276,8 @@ fn text_width(ui: &Ui, text: &str, role: TextRole, look: &Look) -> f32 {
 /// elsewhere. Row-panel values use it too.
 pub fn data_role(look: &Look) -> TextRole {
     match look.data_font {
-        DataFont::Proportional => TextRole::UiBody,
+        // The interface's sans, at the data's size.
+        DataFont::Proportional => TextRole::DataBody,
         DataFont::Monospace => TextRole::pick(look, TextRole::GridCell, TextRole::OBody),
     }
 }
diff --git a/src/ui/picker.rs b/src/ui/picker.rs
index 161a730..e21894f 100644
--- a/src/ui/picker.rs
+++ b/src/ui/picker.rs
@@ -386,7 +386,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     }
     let secrets = if look.terminal {
         TERMINAL_SECRETS.to_owned()
-    } else if look.faces == theme::Faces::Plex {
+    } else if look.faces.typeface == theme::Typeface::Plex {
         gettext(locale, "Passwords are stored in the macOS Keychain").into_owned()
     } else {
         gettext(locale, "Passwords are stored in the system keyring").into_owned()
@@ -414,7 +414,7 @@ pub fn show(app: &mut App, ui: &mut egui::Ui) {
     let found = app.connections.search(&search);
     if app.connections.connections.is_empty() {
         // First launch: why the list is empty, and the one next step.
-        let keyring = if look.faces == theme::Faces::Plex {
+        let keyring = if look.faces.typeface == theme::Typeface::Plex {
             gettext(locale, "Tabletist stores passwords in the macOS Keychain.")
         } else {
             gettext(locale, "Tabletist stores passwords in the system keyring.")
```

- [ ] **Step 4: Size each role in the faces given**

`spec_in` scales the design's size; every size, spacing and line height is taken in the faces given. JetBrains Mono is loaded on macOS beside Plex Mono, at the same two weights. Windows' grid cells get `DataBody`.

```diff
diff --git a/src/typography.rs b/src/typography.rs
index 6f1d49a..93275f7 100644
--- a/src/typography.rs
+++ b/src/typography.rs
@@ -8,6 +8,12 @@
 //! look's faces (Plex on macOS, the terminal's monospace on Omarchy, Inter
 //! elsewhere).
 //!
+//! A role's size in [`TextRole::spec`] is the design's. The settings size
+//! the text on macOS and Windows ([`TextRole::spec_in`]): the data roles
+//! keep to the data's size, 12.5 points in the design, and the others to
+//! the interface's, 13. The terminal look draws the whole window larger or
+//! smaller instead, its roles as they are.
+//!
 //! Text is built with [`Text`] and drawn from the [`Laid`] it lays out.
 
 pub mod desktop_font;
@@ -22,7 +28,12 @@ use egui::{
     Color32, FontFamily, FontId, Galley, Painter, Pos2, Response, Sense, TextFormat, Ui, Vec2,
 };
 
-use crate::theme::{Faces, Look};
+use crate::theme::{DataFace, Faces, Look, Typeface};
+
+/// The data's size and the interface's in the design: what a role's size
+/// in [`TextRole::spec`] is drawn at before the settings change them.
+const DESIGN_DATA: f32 = 12.5;
+const DESIGN_INTERFACE: f32 = 13.0;
 
 /// Sans or monospace: what a role asks for. The Omarchy look has one
 /// monospace face for both.
@@ -99,6 +110,9 @@ pub enum TextRole {
     Legend,
     /// What an empty area or a failed connection says first.
     StateTitle,
+    /// Grid cells and row-panel values in Inter, where the look draws data
+    /// in its sans (Windows).
+    DataBody,
     // Omarchy, from the spec.
     OBody,
     OTableTitle,
@@ -120,7 +134,7 @@ pub enum TextRole {
 }
 
 impl TextRole {
-    pub const ALL: [TextRole; 42] = [
+    pub const ALL: [TextRole; 43] = [
         Self::UiBody,
         Self::UiBodyStrong,
         Self::TableTitle,
@@ -150,6 +164,7 @@ impl TextRole {
         Self::FormLabel,
         Self::Legend,
         Self::StateTitle,
+        Self::DataBody,
         Self::OBody,
         Self::OTableTitle,
         Self::OScreenTitle,
@@ -197,6 +212,7 @@ impl TextRole {
             Self::FormLabel => "form-label",
             Self::Legend => "legend",
             Self::StateTitle => "state-title",
+            Self::DataBody => "data-body",
             Self::OBody => "o-body",
             Self::OTableTitle => "o-table-title",
             Self::OScreenTitle => "o-screen-title",
@@ -277,6 +293,7 @@ impl TextRole {
             Self::FormLabel => style(Sans, 500, 12.0),
             Self::Legend => style(Sans, 600, 12.0),
             Self::StateTitle => style(Sans, 600, 15.0),
+            Self::DataBody => style(Sans, 400, 13.0),
             Self::OBody => style(Mono, 400, 13.0),
             Self::OTableTitle => style(Mono, 700, 15.0),
             Self::OScreenTitle => style(Mono, 700, 14.0),
@@ -293,30 +310,78 @@ impl TextRole {
         }
     }
 
+    /// Whether the role draws data, which the data's size sizes (and on
+    /// macOS its face draws, where the role is monospace): grid cells,
+    /// the SQL editor and JSON. Every other role is the interface's.
+    pub const fn is_data(self) -> bool {
+        matches!(
+            self,
+            Self::GridCell
+                | Self::Json
+                | Self::ValueTag
+                | Self::JsonChip
+                | Self::InspectorValue
+                | Self::Code
+                | Self::CompletionName
+                | Self::CompletionMatch
+                | Self::DataBody
+        )
+    }
+
+    /// The role as `faces` draw it: [`Self::spec`] at the settings' sizes.
+    /// A data role keeps to the data's size as the design's keeps to 12.5
+    /// points, and the others to the interface's as the design's to 13.
+    /// The terminal look's are the design's.
+    pub fn spec_in(self, faces: Faces) -> Spec {
+        let mut spec = self.spec();
+        let (size, design) = match faces.typeface {
+            Typeface::Terminal => return spec,
+            Typeface::Inter | Typeface::Plex if self.is_data() => {
+                (faces.data_size.points(), DESIGN_DATA)
+            }
+            Typeface::Inter | Typeface::Plex => (faces.interface.points(), DESIGN_INTERFACE),
+        };
+        spec.size = spec.size * size / design;
+        spec
+    }
+
     /// The Omarchy role when the look is a terminal's, else the macOS one.
     pub fn pick(look: &Look, mac: Self, omarchy: Self) -> Self {
         if look.terminal { omarchy } else { mac }
     }
 
-    pub fn size(self) -> f32 {
-        self.spec().size
+    /// The role's size in points in `faces`.
+    pub fn size(self, faces: Faces) -> f32 {
+        self.spec_in(faces).size
+    }
+
+    /// The data face `faces` draw the role in, where they draw it in one:
+    /// a monospace data role on macOS.
+    fn data_face(self, faces: Faces) -> Option<DataFace> {
+        let spec = self.spec();
+        (faces.typeface == Typeface::Plex && self.is_data() && spec.kind == Kind::Mono)
+            .then_some(faces.data)
     }
 
     /// The role's font in `faces`.
     pub fn font_id(self, faces: Faces) -> FontId {
-        let spec = self.spec();
-        FontId::new(spec.size, family(faces, spec.kind, spec.weight))
+        let spec = self.spec_in(faces);
+        let family = match self.data_face(faces) {
+            Some(face) => data_family(face, spec.weight),
+            None => family(faces, spec.kind, spec.weight),
+        };
+        FontId::new(spec.size, family)
     }
 
-    /// Letter spacing in points.
-    pub fn tracking(self) -> f32 {
-        let spec = self.spec();
+    /// Letter spacing in points, in `faces`.
+    pub fn tracking(self, faces: Faces) -> f32 {
+        let spec = self.spec_in(faces);
         spec.tracking * spec.size
     }
 
-    /// Line height in points, when the role sets one.
-    pub fn line_height(self) -> Option<f32> {
-        let spec = self.spec();
+    /// Line height in points in `faces`, when the role sets one.
+    pub fn line_height(self, faces: Faces) -> Option<f32> {
+        let spec = self.spec_in(faces);
         spec.line_height.map(|factor| factor * spec.size)
     }
 
@@ -324,8 +389,8 @@ impl TextRole {
     pub fn format(self, faces: Faces, color: Color32) -> TextFormat {
         TextFormat {
             font_id: self.font_id(faces),
-            extra_letter_spacing: self.tracking(),
-            line_height: self.line_height(),
+            extra_letter_spacing: self.tracking(faces),
+            line_height: self.line_height(faces),
             color,
             ..Default::default()
         }
@@ -352,9 +417,12 @@ impl TextRole {
     }
 
     /// The height of the role's capitals in points, in faces whose line box
-    /// does not centre on them (Plex).
+    /// does not centre on them (Plex, and not JetBrains Mono).
     fn capitals(self, faces: Faces) -> Option<f32> {
-        fonts::capitals(faces).map(|ems| ems * self.size())
+        if self.data_face(faces) == Some(DataFace::JetBrainsMono) {
+            return None;
+        }
+        fonts::capitals(faces.typeface).map(|ems| ems * self.size(faces))
     }
 
     /// Where a line of the role centres, below its top (see
@@ -369,43 +437,63 @@ impl TextRole {
 
     /// The height of one line in the role, in `faces`.
     pub fn row_height(self, ctx: &egui::Context, faces: Faces) -> f32 {
-        self.line_height()
+        self.line_height(faces)
             .unwrap_or_else(|| ctx.fonts_mut(|fonts| fonts.row_height(&self.font_id(faces))))
     }
 }
 
-/// The weights `faces` has for `kind`.
-fn weights(faces: Faces, kind: Kind) -> &'static [u16] {
-    match (faces, kind) {
-        (Faces::Plex, Kind::Sans) => &[400, 500, 600],
-        (Faces::Plex, Kind::Mono) => &[400, 500],
-        (Faces::Terminal, _) => &[400, 500, 700],
-        (Faces::Inter, Kind::Sans) => &[400, 500, 600, 700],
-        (Faces::Inter, Kind::Mono) => &[400],
+/// The weights `typeface` has for `kind`.
+fn weights(typeface: Typeface, kind: Kind) -> &'static [u16] {
+    match (typeface, kind) {
+        (Typeface::Plex, Kind::Sans) => &[400, 500, 600],
+        (Typeface::Plex, Kind::Mono) => &[400, 500],
+        (Typeface::Terminal, _) => &[400, 500, 700],
+        (Typeface::Inter, Kind::Sans) => &[400, 500, 600, 700],
+        (Typeface::Inter, Kind::Mono) => &[400],
     }
 }
 
-/// The name of the family holding `faces`' `kind` at `weight`.
-pub fn family_name(faces: Faces, kind: Kind, weight: u16) -> String {
-    let face = match (faces, kind) {
-        (Faces::Plex, Kind::Sans) => "plex-sans",
-        (Faces::Plex, Kind::Mono) => "plex-mono",
-        (Faces::Terminal, _) => "omarchy-mono",
-        (Faces::Inter, Kind::Sans) => "inter",
-        (Faces::Inter, Kind::Mono) => "standard-mono",
+/// The name of the family holding `typeface`'s `kind` at `weight`.
+pub fn family_name(typeface: Typeface, kind: Kind, weight: u16) -> String {
+    let face = match (typeface, kind) {
+        (Typeface::Plex, Kind::Sans) => "plex-sans",
+        (Typeface::Plex, Kind::Mono) => "plex-mono",
+        (Typeface::Terminal, _) => "omarchy-mono",
+        (Typeface::Inter, Kind::Sans) => "inter",
+        (Typeface::Inter, Kind::Mono) => "standard-mono",
     };
     format!("{face}-{weight}")
 }
 
-/// The family for `kind` at `weight` in `faces`, at the nearest weight the
-/// faces have (ties go heavier).
-pub fn family(faces: Faces, kind: Kind, weight: u16) -> FontFamily {
-    let nearest = weights(faces, kind)
+/// The name of the family holding the data face `face` at `weight`: each
+/// is bundled at 400 and 500, as Plex Mono is.
+pub fn data_family_name(face: DataFace, weight: u16) -> String {
+    match face {
+        DataFace::PlexMono => family_name(Typeface::Plex, Kind::Mono, weight),
+        DataFace::JetBrainsMono => format!("jetbrains-mono-{weight}"),
+    }
+}
+
+/// The nearest of `weights` to `weight` (ties go heavier).
+fn nearest(weights: &[u16], weight: u16) -> u16 {
+    weights
         .iter()
         .copied()
         .min_by_key(|have| (have.abs_diff(weight), u16::MAX - have))
-        .unwrap_or(400);
-    FontFamily::Name(family_name(faces, kind, nearest).into())
+        .unwrap_or(400)
+}
+
+/// The family for `kind` at `weight` in `faces`, at the nearest weight the
+/// faces have.
+pub fn family(faces: Faces, kind: Kind, weight: u16) -> FontFamily {
+    let weight = nearest(weights(faces.typeface, kind), weight);
+    FontFamily::Name(family_name(faces.typeface, kind, weight).into())
+}
+
+/// The family of the data face `face` at the nearest weight it has.
+fn data_family(face: DataFace, weight: u16) -> FontFamily {
+    let weight = nearest(weights(Typeface::Plex, Kind::Mono), weight);
+    FontFamily::Name(data_family_name(face, weight).into())
 }
 
 /// Text in one or more roles, before layout.
@@ -646,7 +734,7 @@ pub fn layouter(
 /// roles.
 pub fn text_styles(faces: Faces) -> std::collections::BTreeMap<egui::TextStyle, FontId> {
     use egui::TextStyle;
-    let (small, body, heading, mono) = if faces == Faces::Terminal {
+    let (small, body, heading, mono) = if faces.typeface == Typeface::Terminal {
         (
             TextRole::OSecondary,
             TextRole::OBody,
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
index 815b130..89b9d29 100644
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -2,10 +2,12 @@
 //!
 //! - Standard: Inter (fastframe's set-up), with egui's monospace for data.
 //! - macOS: IBM Plex Sans (400, 500, 600) and IBM Plex Mono (400, 500),
-//!   bundled: `plex-sans-400` ... `plex-mono-500`.
-//! - Omarchy: the desktop's monospace font at 400, 500 and 700, or the
-//!   bundled JetBrains Mono when fontconfig finds none: `omarchy-mono-400`
-//!   ... `omarchy-mono-700`.
+//!   bundled: `plex-sans-400` ... `plex-mono-500`; and JetBrains Mono
+//!   (400, 500), the other face data can be drawn in: `jetbrains-mono-400`
+//!   and `jetbrains-mono-500`.
+//! - Omarchy: the desktop's monospace font at 400, 500 and 700 (fontconfig
+//!   reads it, [`super::desktop_font`]), or the bundled JetBrains Mono when
+//!   it found none: `omarchy-mono-400` ... `omarchy-mono-700`.
 //!
 //! egui's own Proportional and Monospace families (and fastframe's Inter
 //! weight families, which the edit connection dialog still names) lead with
@@ -18,7 +20,9 @@ use egui::epaint::text::VariationCoords;
 use egui::{FontData, FontDefinitions, FontFamily};
 
 use super::{Kind, family_name};
-use crate::theme::{Faces, Look};
+use crate::theme::{Look, Typeface};
+#[cfg(any(target_os = "macos", test))]
+use {super::data_family_name, crate::theme::DataFace};
 
 /// IBM Plex Sans, variable in weight and width (SIL OFL 1.1).
 #[cfg(any(target_os = "macos", test))]
@@ -36,8 +40,9 @@ pub(crate) const PLEX_MONO_MEDIUM: &[u8] =
 #[cfg(any(target_os = "macos", test))]
 pub(crate) const KEYS: &[u8] = include_bytes!("../../assets/fonts/NotoSansSymbols2-Keys.ttf");
 /// JetBrains Mono, variable in weight (SIL OFL 1.1): Omarchy's default, for
-/// desktops without a monospace font of their own.
-#[cfg(any(target_os = "linux", test))]
+/// desktops without a monospace font of their own, and the other data face
+/// of macOS.
+#[cfg(any(target_os = "macos", target_os = "linux", test))]
 pub(crate) const JETBRAINS_MONO: &[u8] =
     include_bytes!("../../assets/fonts/JetBrainsMono-Variable.ttf");
 
@@ -72,12 +77,12 @@ const KEYS_METRICS: Metrics = Metrics {
 /// The height of Plex's capitals in ems, Sans and Mono alike.
 const PLEX_CAPITALS: f32 = 0.698;
 
-/// The height of `faces`' capitals in ems, where the line box does not
+/// The height of `typeface`'s capitals in ems, where the line box does not
 /// centre on them: Plex's centres a little above its capitals. Inter's and
 /// JetBrains Mono's centre on theirs, and a desktop font's are not known.
-pub fn capitals(faces: Faces) -> Option<f32> {
+pub fn capitals(typeface: Typeface) -> Option<f32> {
     // Off macOS the Plex look draws in Inter.
-    (faces == Faces::Plex && cfg!(any(target_os = "macos", test))).then_some(PLEX_CAPITALS)
+    (typeface == Typeface::Plex && cfg!(any(target_os = "macos", test))).then_some(PLEX_CAPITALS)
 }
 
 /// fastframe's families for Inter's named weights.
@@ -126,10 +131,10 @@ fn alias(fonts: &mut FontDefinitions, name: &str, from: &FontFamily) {
 /// Replaces the faces fastframe set up with the look's. `desktop` allows
 /// reading the desktop's own monospace font (never in tests or screenshots).
 pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: bool) {
-    match look.faces {
-        Faces::Inter => inter(fonts),
-        Faces::Plex => plex(fonts),
-        Faces::Terminal => terminal(fonts, desktop),
+    match look.faces.typeface {
+        Typeface::Inter => inter(fonts),
+        Typeface::Plex => plex(fonts),
+        Typeface::Terminal => terminal(fonts, desktop),
     }
 }
 
@@ -141,11 +146,15 @@ fn inter(fonts: &mut FontDefinitions) {
         (600, FontFamily::Name(SEMIBOLD.into())),
         (700, FontFamily::Name(BOLD.into())),
     ] {
-        alias(fonts, &family_name(Faces::Inter, Kind::Sans, weight), &from);
+        alias(
+            fonts,
+            &family_name(Typeface::Inter, Kind::Sans, weight),
+            &from,
+        );
     }
     alias(
         fonts,
-        &family_name(Faces::Inter, Kind::Mono, 400),
+        &family_name(Typeface::Inter, Kind::Mono, 400),
         &FontFamily::Monospace,
     );
 }
@@ -153,20 +162,30 @@ fn inter(fonts: &mut FontDefinitions) {
 #[cfg(any(target_os = "macos", test))]
 fn plex(fonts: &mut FontDefinitions) {
     for weight in [400, 500, 600] {
-        let name = family_name(Faces::Plex, Kind::Sans, weight);
+        let name = family_name(Typeface::Plex, Kind::Sans, weight);
         fonts
             .font_data
             .insert(name.clone(), weighted(PLEX_SANS, f32::from(weight)));
         own(fonts, &name, &name, &FontFamily::Proportional);
     }
     for (weight, bytes) in [(400, PLEX_MONO), (500, PLEX_MONO_MEDIUM)] {
-        let name = family_name(Faces::Plex, Kind::Mono, weight);
+        let name = family_name(Typeface::Plex, Kind::Mono, weight);
         fonts
             .font_data
             .insert(name.clone(), Arc::new(FontData::from_static(bytes)));
         own(fonts, &name, &name, &FontFamily::Monospace);
     }
-    let sans = |weight| family_name(Faces::Plex, Kind::Sans, weight);
+    // The other face data can be drawn in, at the same weights, whichever
+    // the settings name: a change of it lays the text out again, and
+    // loads nothing.
+    for weight in [400, 500] {
+        let name = data_family_name(DataFace::JetBrainsMono, weight);
+        fonts
+            .font_data
+            .insert(name.clone(), weighted(JETBRAINS_MONO, f32::from(weight)));
+        own(fonts, &name, &name, &FontFamily::Monospace);
+    }
+    let sans = |weight| family_name(Typeface::Plex, Kind::Sans, weight);
     lead(fonts, FontFamily::Proportional, &sans(400));
     lead(fonts, FontFamily::Name(MEDIUM.into()), &sans(500));
     lead(fonts, FontFamily::Name(SEMIBOLD.into()), &sans(600));
@@ -175,7 +194,7 @@ fn plex(fonts: &mut FontDefinitions) {
     lead(
         fonts,
         FontFamily::Monospace,
-        &family_name(Faces::Plex, Kind::Mono, 400),
+        &family_name(Typeface::Plex, Kind::Mono, 400),
     );
     // The keyboard symbols right behind each face, ahead of the fallbacks.
     // epaint centres a fallback face's line box on the leading face's
@@ -238,18 +257,18 @@ fn terminal(fonts: &mut FontDefinitions, desktop: bool) {
             };
             alias(
                 fonts,
-                &family_name(Faces::Terminal, Kind::Mono, weight),
+                &family_name(Typeface::Terminal, Kind::Mono, weight),
                 &from,
             );
         }
         return;
     };
     for (weight, data) in [400, 500, 700].into_iter().zip(faces) {
-        let name = family_name(Faces::Terminal, Kind::Mono, weight);
+        let name = family_name(Typeface::Terminal, Kind::Mono, weight);
         fonts.font_data.insert(name.clone(), data);
         own(fonts, &name, &name, &FontFamily::Monospace);
     }
-    let face = |weight| family_name(Faces::Terminal, Kind::Mono, weight);
+    let face = |weight| family_name(Typeface::Terminal, Kind::Mono, weight);
     lead(fonts, FontFamily::Monospace, &face(400));
     lead(fonts, FontFamily::Proportional, &face(400));
     lead(fonts, FontFamily::Name(MEDIUM.into()), &face(500));
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index 2137d0d..dea0879 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -8527,7 +8527,7 @@ value_tags   = true
                 saved.password = PasswordMode::Keyring;
                 harness.app.connections.upsert(saved);
                 harness.app.apply(crate::model::Action::EditConnection(id));
-                let hint = look.label(if look.faces == crate::theme::Faces::Plex {
+                let hint = look.label(if look.faces.typeface == crate::theme::Typeface::Plex {
                     "Saved in the keychain"
                 } else {
                     "Saved in the keyring"
@@ -8778,7 +8778,7 @@ value_tags   = true
     fn each_keyring_box_says_which_secret_it_keeps() {
         use crate::connections::PasswordMode;
         for look in crate::theme::Look::ALL {
-            let keyring = if look.faces == crate::theme::Faces::Plex {
+            let keyring = if look.faces.typeface == crate::theme::Typeface::Plex {
                 "Keychain"
             } else {
                 "Keyring"
```

- [ ] **Step 5: Draw it in the app**

The app's look is given the settings' text at the start and at every change of them; `logic` sets egui's own styles again, and the window's zoom from the terminal look's size. A grid row is at least as tall as a line of its data (in `Look::row`, Step 3).

```diff
diff --git a/src/app.rs b/src/app.rs
index 19e6483..6b18564 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -159,6 +159,12 @@ pub struct App {
     /// The pin the status line last said could not be read: it is said
     /// once.
     pin_told: Option<String>,
+    /// The look's text changed since the last `logic`: egui's own text
+    /// styles follow it.
+    restyle: bool,
+    /// The zoom the window was last given: the terminal look's text size,
+    /// which sizes the whole window there.
+    zoom: Option<f32>,
     /// The window title last sent, so it is sent only when it changes.
     window_title: String,
     /// What the next frame asks of the window: to come forward, or the
@@ -252,6 +258,7 @@ impl App {
             crate::settings::Zone::System | crate::settings::Zone::Utc => None,
         };
         let (table_lists, table_lists_unread) = load_table_lists(&dirs.table_lists_file());
+        let look = crate::theme::Look::for_platform().with_text(&settings);
         let mut app = Self {
             dirs,
             settings,
@@ -261,7 +268,7 @@ impl App {
             zones: None,
             locale: Locale::default(),
             palette: Palette::dark(),
-            look: crate::theme::Look::for_platform(),
+            look,
             keymap: std::sync::Arc::default(),
             themes: Catalog::default(),
             tabs: Vec::new(),
@@ -290,6 +297,8 @@ impl App {
             theme_asked: 0,
             theme_taken: 0,
             pin_told: None,
+            restyle: false,
+            zoom: None,
             window_title: "Tabletist".into(),
             window_asks: Vec::new(),
             closed_editors: Vec::new(),
@@ -4091,6 +4100,13 @@ impl App {
             // Reading a theme needs the window: `logic` has it.
             self.theme_changed = true;
         }
+        // The text's sizes and the data's face: every role drawn from here
+        // on has them, and egui's own styles are set again in `logic`.
+        let look = self.look.with_text(&self.settings);
+        if look != self.look {
+            self.look = look;
+            self.restyle = true;
+        }
         // An edit of the file that turns the server on is held.
         self.check_agents();
         if self.serves_agents {
@@ -5600,6 +5616,17 @@ impl App {
         });
     }
 
+    /// How large the window draws everything: the terminal look's text
+    /// size against its 13 points. The other looks size their text by
+    /// role, and draw at 1.
+    pub fn zoom_factor(&self) -> f32 {
+        if self.look.terminal {
+            self.settings.font_size_in(true).points() / 13.0
+        } else {
+            1.0
+        }
+    }
+
     /// The pin the backend reads: one by name or path. A palette file is
     /// the themes folder's to read.
     fn pin_to_read(&self) -> Option<&str> {
@@ -5698,6 +5725,14 @@ impl App {
             }
         }
         self.tell_missing_pin(ctx);
+        if std::mem::take(&mut self.restyle) {
+            theme::apply(ctx, &self.palette, &self.look);
+        }
+        let zoom = self.zoom_factor();
+        if self.zoom != Some(zoom) {
+            self.zoom = Some(zoom);
+            ctx.set_zoom_factor(zoom);
+        }
         // macOS and Windows: the window's own parts (its title bar, the
         // menus and panels of the system) in the mode the settings name.
         let mode = match (self.look.terminal, self.settings.theme.mode()) {
diff --git a/src/testing.rs b/src/testing.rs
index 0fa9397..e4f70b6 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -1586,6 +1586,7 @@ impl Harness {
 
     /// Draws with `look` from the next frame on.
     pub fn set_look(&mut self, look: crate::theme::Look) {
+        let look = look.with_text(&self.app.settings);
         self.app.look = look;
         crate::theme::install(&self.ctx, false, &look);
         crate::theme::apply(&self.ctx, &self.app.palette, &look);
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- theme::tests typography::fonts text_settings`
Expected: PASS. Then the four checks above.

- [ ] **Step 7: Look at it**

Optional, by eye, never as a test: a throwaway scene in `src/shots.rs` with `interface_size = "large"` and `font = "JetBrains Mono"`, rendered with `cargo test --locked --features shots --lib shots::<scene> -- --ignored`, shows larger chrome, the grid in JetBrains Mono and the rows as tall as before. Remove the scene and its pictures.

- [ ] **Step 8: Commit**

```bash
git add src/theme.rs src/typography.rs src/typography/fonts.rs src/app.rs src/testing.rs src/keymap.rs src/ui/grid.rs src/ui/mod.rs src/ui/picker.rs src/ui/sql_text.rs src/ui/connect_dialog/mod.rs src/ui/text_settings_tests.rs
git commit -m "Draw each role at the text sizes the settings give, and the terminal zoomed"
```

---

### Task 3: The font size keys

**Files:**
- Modify: `src/keymap.rs`, `src/model.rs`, `src/ui/keys.rs`, `src/app.rs`, `src/ui/text_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

On macOS ⌘= and ⌘- step the data's size and ⌘0 takes it out of the file, each written; egui zooms nothing. The keys hold under the Settings window. In the terminal look `ctrl+=` and `ctrl+-` step the size under its Settings screen too, stop at the ends, and no key resets it. The keymap's generated tests check the new keys against every other.

```diff
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index bd60796..aeeb3b2 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -2,6 +2,9 @@
 //! data font of macOS and Windows, the terminal look's one font and size,
 //! and the keys that change the size wherever the keyboard is.
 
+use egui::{Key, Modifiers};
+
+use super::appearance_settings_tests::{on_the_pane, settings_saves};
 use crate::settings::{FontSize, InterfaceSize};
 use crate::testing::Harness;
 use crate::theme::Look;
@@ -30,6 +33,11 @@ fn terminal() -> Harness {
     harness
 }
 
+/// The settings file as the app last wrote it.
+fn file(harness: &Harness) -> &str {
+    &harness.app.settings_file.text
+}
+
 #[test]
 fn the_terminal_look_draws_the_whole_window_at_its_size() {
     let mut harness = terminal();
@@ -92,3 +100,48 @@ fn a_grid_row_holds_a_line_of_its_data_at_every_size() {
     assert_eq!(Look::macos().with_grid(&compact).row(), 29.0);
     assert_eq!(Look::standard().with_grid(&compact).row(), 18.0);
 }
+
+#[test]
+fn the_mac_keys_step_and_reset_the_data_size_and_write_it() {
+    let mut harness = sheet(Look::macos());
+    let saves = settings_saves(&harness);
+    harness.press(Key::Equals, Modifiers::COMMAND);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(13.0));
+    assert!(file(&harness).contains("font_size = 13\n"));
+    harness.press(Key::Minus, Modifiers::COMMAND);
+    harness.press(Key::Minus, Modifiers::COMMAND);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
+    harness.press(Key::Num0, Modifiers::COMMAND);
+    assert_eq!(harness.app.settings.font_size, None);
+    assert!(!file(&harness).contains("font_size"));
+    assert_eq!(settings_saves(&harness), saves + 4);
+    // egui zooms nothing: the keys are the keymap's.
+    logic(&mut harness);
+    assert_eq!(harness.ctx.zoom_factor(), 1.0);
+    assert!(!harness.ctx.options(|options| options.zoom_with_keyboard));
+    // Under the Settings window too, which shows the size.
+    on_the_pane(&mut harness);
+    harness.press(Key::Equals, Modifiers::COMMAND);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(13.0));
+}
+
+#[test]
+fn the_terminal_keys_step_the_size_anywhere_and_stop_at_its_ends() {
+    let mut harness = terminal();
+    harness.press(Key::Equals, Modifiers::CTRL);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(14.0));
+    // Under the Settings screen, whose row follows.
+    on_the_pane(&mut harness);
+    harness.press(Key::Minus, Modifiers::CTRL);
+    harness.press(Key::Minus, Modifiers::CTRL);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
+    assert!(harness.app.dialog.is_some(), "the screen stays open");
+    harness
+        .app
+        .change_settings(|settings| settings.font_size = FontSize::of(9.0));
+    harness.press(Key::Minus, Modifiers::CTRL);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(9.0));
+    // No key resets it here: `R` on its row does.
+    harness.press(Key::Num0, Modifiers::CTRL);
+    assert_eq!(harness.app.settings.font_size, FontSize::of(9.0));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- text_settings`
Expected:

```
test ui::text_settings_tests::the_terminal_keys_step_the_size_anywhere_and_stop_at_its_ends ... FAILED
test ui::text_settings_tests::the_mac_keys_step_and_reset_the_data_size_and_write_it ... FAILED
test result: FAILED. 3 passed; 2 failed; 0 ignored; 0 measured; 2710 filtered out
```

- [ ] **Step 3: The commands and their keys**

```diff
diff --git a/src/keymap.rs b/src/keymap.rs
index 96e3b40..6a433d7 100644
--- a/src/keymap.rs
+++ b/src/keymap.rs
@@ -318,6 +318,9 @@ commands! {
     GoToConnectionWindow,
     SwitchDatabase,
     Settings,
+    LargerText,
+    SmallerText,
+    DefaultTextSize,
     CloseConnection,
     NextConnection,
     PreviousConnection,
@@ -571,6 +574,9 @@ impl Command {
             }
             C::SwitchDatabase => info("Switch database", "Switch database", Window).reserved(),
             C::Settings => info("Settings", "Settings", Window),
+            C::LargerText => info("Larger text", "", Window),
+            C::SmallerText => info("Smaller text", "", Window),
+            C::DefaultTextSize => info("Default text size", "", Window),
             C::CloseConnection => info("Close connection", "", Window),
             C::NextConnection => info("Next connection", "", Window),
             C::PreviousConnection => info("Previous connection", "", Window),
@@ -896,6 +902,8 @@ impl Binding {
 }
 
 const GLOBAL: &[Scope] = &[Scope::Global];
+/// The global scope and a prompt's: keys that hold under a dialog too.
+const ANYWHERE: &[Scope] = &[Scope::Global, Scope::Prompt];
 const CONNECTIONS: &[Scope] = &[Scope::Connections];
 const SIDEBAR: &[Scope] = &[Scope::Sidebar];
 const GRID: &[Scope] = &[Scope::Grid];
@@ -936,6 +944,21 @@ pub const BINDINGS: &[Binding] = {
             keys(&[k("cmd+,")]),
             keys(&[k("ctrl+,")]),
         ),
+        // The text's size, under a dialog too: the Settings window shows
+        // what it changes. The terminal look resets it from that window.
+        bind(
+            C::LargerText,
+            ANYWHERE,
+            keys(&[k("cmd+=")]),
+            keys(&[k("ctrl+=")]),
+        ),
+        bind(
+            C::SmallerText,
+            ANYWHERE,
+            keys(&[k("cmd+-")]),
+            keys(&[k("ctrl+-")]),
+        ),
+        bind(C::DefaultTextSize, ANYWHERE, keys(&[k("cmd+0")]), NONE),
         bind(
             C::CloseConnection,
             GLOBAL,
```

- [ ] **Step 4: Read them wherever the keyboard is, and write the size**

`keys::text_size` runs every frame, before the check that keeps a dialog's keys from the workspace. egui's own zoom keys are turned off at the start.

```diff
diff --git a/src/app.rs b/src/app.rs
index 6b18564..d71a8db 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -1961,6 +1961,13 @@ impl App {
                     self.dialog = Some(Dialog::About);
                 }
             }
+            Action::TextSize(larger) => {
+                let terminal = self.look.terminal;
+                self.change_settings(|settings| {
+                    settings.font_size =
+                        larger.map(|larger| settings.stepped_font_size(terminal, larger));
+                });
+            }
             Action::ShowSettings => {
                 // The shortcuts dialog offers it, and gives way to it. Asked
                 // for while it is open, it stays as it is.
@@ -5569,6 +5576,9 @@ impl App {
     /// and tests, which must not scan the user's themes or Omarchy.
     pub fn attach(&mut self, ctx: &egui::Context, follow_desktop: bool) {
         self.follow_desktop = follow_desktop;
+        // The font size keys are the keymap's: they change the setting,
+        // where egui's would zoom the whole window and forget it.
+        ctx.options_mut(|options| options.zoom_with_keyboard = false);
         theme::install(ctx, follow_desktop, &self.look);
         if follow_desktop {
             theme::enable_desktop_themes(&mut self.themes);
@@ -5803,6 +5813,7 @@ impl App {
         // the last frame's actions left, after anything the backend
         // delivered just now.
         self.refresh_completion(ui.ctx());
+        crate::ui::keys::text_size(self, ui.ctx());
         // A dialog takes the keyboard: no shortcut acts behind it, and a
         // first key that waited for its second (`cc`) waits no longer.
         if self.dialog.is_none() {
diff --git a/src/model.rs b/src/model.rs
index deb6d4d..e8f91e1 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -800,6 +800,9 @@ pub enum Action {
     ShowAbout,
     /// Show the Settings window.
     ShowSettings,
+    /// The font size a step larger (`Some(true)`), smaller, or back to the
+    /// look's own (`None`), written to the settings.
+    TextSize(Option<bool>),
     /// Show the Settings window on its Agents pane: from the Agents tab.
     ShowAgentSettings,
     /// Show this pane of the Settings window, its cursor on its first row.
diff --git a/src/ui/keys.rs b/src/ui/keys.rs
index 4fcca3d..7e488cf 100644
--- a/src/ui/keys.rs
+++ b/src/ui/keys.rs
@@ -991,6 +991,31 @@ pub fn handle(app: &mut App, ctx: &egui::Context) {
     app.actions.extend(actions);
 }
 
+/// The text size keys, wherever the keyboard is: under a dialog too, where
+/// [`handle`] reads no key. The Settings window shows what they change.
+pub fn text_size(app: &mut App, ctx: &egui::Context) {
+    let layout = app.layout();
+    // In a field as out of one: no field takes these chords.
+    let normal = !ctx.text_edit_focused();
+    let scope = if app.dialog.is_some() {
+        Scope::Prompt
+    } else {
+        Scope::Global
+    };
+    for (command, larger) in [
+        (Command::LargerText, Some(true)),
+        (Command::SmallerText, Some(false)),
+        (Command::DefaultTextSize, None),
+    ] {
+        let pressed =
+            ctx.input_mut(|input| asked(input, &app.keymap, layout, command, scope, normal));
+        // Each press a step, its repeats while held too.
+        for _ in 0..pressed.count {
+            app.actions.push(Action::TextSize(larger));
+        }
+    }
+}
+
 /// Whether a copy that comes this frame is Ctrl+C where Ctrl is the command
 /// key, by what is held: Ctrl and nothing else. Not Cmd+C, the copy where
 /// Cmd is the command key; nor Ctrl+Shift+C, a Copy key or a menu's Copy,
```

- [ ] **Step 5: Run the tests to see them pass**

Run: `cargo test --locked --lib -- text_settings keymap`
Expected: PASS, the keymap's `no_two_commands_share_a_key_where_both_could_be_read` with them. Then the four checks above (`tests/keys.rs` finds no key written by hand).

- [ ] **Step 6: Commit**

```bash
git add src/keymap.rs src/model.rs src/ui/keys.rs src/app.rs src/ui/text_settings_tests.rs
git commit -m "Change the font size with the keymap's keys, and turn egui's zoom off"
```

---

### Task 4: The terminal look's font through fontconfig

**Files:**
- Modify: `src/typography/desktop_font.rs`, `src/typography/fonts.rs`, `src/typography.rs`, `src/theme.rs`, `src/backend.rs`, `src/app.rs`, `src/testing.rs`, `src/ui/grid.rs`, `src/ui/data_view.rs`

- [ ] **Step 1: Write the failing tests**

`fc-match` is asked for the family too; its first name is the one taken. A family is the font named when it begins with the name. A name goes into a pattern with what fontconfig reads otherwise escaped. In the app, a change of the font asks the backend for a read in the terminal look where the desktop's fonts are read, and nowhere else; a read is drawn at the next pass of the logic, with the grid's marks found again for the new face, and an older one that ends later is dropped. The fonts' own tests give `configure` no desktop faces.

```diff
diff --git a/src/app.rs b/src/app.rs
index d71a8db..68f06aa 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -6619,6 +6619,93 @@ mod tests {
         assert_eq!(asked(&mut harness), Vec::<Option<String>>::new());
     }
 
+    /// The fonts the backend was asked to read since the last call.
+    fn fonts_asked(harness: &mut Harness) -> Vec<(Option<String>, u64)> {
+        std::mem::take(&mut harness.app.backend.sent)
+            .into_iter()
+            .filter_map(|command| match command {
+                Command::ReadFont { font, asked } => Some((font, asked)),
+                _ => None,
+            })
+            .collect()
+    }
+
+    #[test]
+    fn a_new_font_is_read_where_the_desktops_fonts_are() {
+        let mut harness = Harness::new();
+        harness.app.look = crate::theme::Look::omarchy();
+        harness.app.follow_desktop = true;
+        let font = |harness: &mut Harness, font: Option<&str>| {
+            let font = font.map(str::to_owned);
+            harness.app.change_settings(|settings| settings.font = font);
+        };
+        font(&mut harness, Some("Fira Code"));
+        assert_eq!(
+            fonts_asked(&mut harness),
+            vec![(Some("Fira Code".into()), 1)]
+        );
+        font(&mut harness, None);
+        assert_eq!(fonts_asked(&mut harness), vec![(None, 2)]);
+        // A change of something else reads nothing.
+        harness
+            .app
+            .change_settings(|settings| settings.page_size = 500);
+        assert_eq!(fonts_asked(&mut harness), vec![]);
+        // Not in tests and the demo, nor in the other looks, whose fonts
+        // are bundled.
+        harness.app.follow_desktop = false;
+        font(&mut harness, Some("Iosevka"));
+        assert_eq!(fonts_asked(&mut harness), vec![]);
+        harness.app.follow_desktop = true;
+        harness.app.look = crate::theme::Look::macos();
+        font(&mut harness, Some("JetBrains Mono"));
+        assert_eq!(fonts_asked(&mut harness), vec![]);
+    }
+
+    #[test]
+    fn a_font_read_is_drawn_and_an_older_one_that_ends_later_is_dropped() {
+        let mut harness = Harness::new();
+        harness.set_look(crate::theme::Look::omarchy());
+        let read = |family: &str| crate::typography::desktop_font::Read {
+            asked: Some(family.into()),
+            // Faces of their own, to tell them apart.
+            faces: Some(std::array::from_fn(|_| {
+                let hack = egui::FontDefinitions::default().font_data["Hack"].clone();
+                std::sync::Arc::new((*hack).clone())
+            })),
+            family: Some(family.into()),
+            missing: false,
+        };
+        let answer = |harness: &mut Harness, asked: u64, family: &str| {
+            harness.app.apply(Action::Backend(Event::Font {
+                asked,
+                read: Box::new(read(family)),
+            }));
+        };
+        // The marks the old face had are found again for the new one.
+        harness.settle();
+        let marks = crate::ui::grid::marks_id(&harness.app.look);
+        crate::ui::grid::marks(&harness.ctx, &harness.app.look);
+        answer(&mut harness, 2, "Iosevka");
+        answer(&mut harness, 1, "Fira Code");
+        assert_eq!(harness.app.desktop_font.family.as_deref(), Some("Iosevka"));
+        // Drawn at the next pass of the logic: the terminal's families lead
+        // with the face read, Hack's here.
+        harness.app.logic(&harness.ctx.clone());
+        harness.settle();
+        let hack = egui::FontDefinitions::default().font_data["Hack"]
+            .font
+            .len();
+        let drawn = harness
+            .ctx
+            .fonts(|fonts| fonts.definitions().font_data["omarchy-mono-400"].font.len());
+        assert_eq!(drawn, hack);
+        let kept = harness
+            .ctx
+            .data(|data| data.get_temp::<crate::ui::format::Marks>(marks));
+        assert!(kept.is_none());
+    }
+
     fn ids(app: &App) -> Vec<u64> {
         app.tabs.iter().map(|tab| tab.id.0).collect()
     }
diff --git a/src/typography/desktop_font.rs b/src/typography/desktop_font.rs
index 162eef8..5683bb2 100644
--- a/src/typography/desktop_font.rs
+++ b/src/typography/desktop_font.rs
@@ -153,6 +153,42 @@ mod parse_tests {
         assert_eq!(parse_fc_match("relative.ttf\n0"), None);
         assert_eq!(parse_fc_match("/x.ttf\nnot-a-number"), None);
     }
+
+    #[test]
+    fn the_family_is_the_first_name_on_the_third_line() {
+        let output = "/usr/share/fonts/TTF/JetBrainsMonoNerdFont-Regular.ttf\n0\n\
+                      JetBrainsMono Nerd Font,JetBrainsMono NF\n";
+        assert_eq!(
+            parse_family(output).as_deref(),
+            Some("JetBrainsMono Nerd Font")
+        );
+        assert_eq!(parse_family("/x.ttf\n0\n"), None);
+        assert_eq!(parse_family("/x.ttf\n0"), None);
+    }
+}
+
+#[cfg(test)]
+mod name_tests {
+    use super::*;
+
+    #[test]
+    fn a_family_is_the_font_named_when_it_begins_with_its_name() {
+        assert!(is_named("Fira Code", "fira code"));
+        assert!(is_named("JetBrainsMono Nerd Font", "JetBrains Mono"));
+        assert!(is_named("Iosevka Term", "Iosevka"));
+        // fontconfig's stand-in for a name it does not know.
+        assert!(!is_named("DejaVu Sans Mono", "Fira Code"));
+        assert!(!is_named("Fira Code", ""));
+    }
+
+    #[test]
+    fn a_name_is_written_into_a_pattern_as_a_family() {
+        assert_eq!(pattern("Fira Code"), "Fira Code");
+        assert_eq!(pattern(" Source Code Pro "), "Source Code Pro");
+        // A dash would start a size, a colon a property, a comma another
+        // family.
+        assert_eq!(pattern("Fira-Code:x,y\\z"), "Fira\\-Code\\:x\\,y\\\\z");
+    }
 }
 
 #[cfg(all(test, target_os = "linux"))]
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
index 89b9d29..4f04a4d 100644
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -456,7 +456,7 @@ mod tests {
         let mut fonts = fastframe_fonts::FontSetup::default()
             .system_fallbacks(false)
             .definitions();
-        configure(&mut fonts, look, false);
+        configure(&mut fonts, look, None);
         fonts
     }
 
diff --git a/src/ui/data_view.rs b/src/ui/data_view.rs
index 945c5ad..eb482b3 100644
--- a/src/ui/data_view.rs
+++ b/src/ui/data_view.rs
@@ -2877,7 +2877,7 @@ mod tests {
             .system_fallbacks(false)
             .definitions();
         for look in Look::ALL {
-            crate::typography::configure(&mut fonts, &look, false);
+            crate::typography::configure(&mut fonts, &look, None);
         }
         ctx.set_fonts(fonts);
         let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- desktop_font app::tests::a_ typography::fonts`
Expected: the crate's tests do not compile:

```
error[E0422]: cannot find struct, variant or union type `Read` in module `crate::typography::desktop_font`
error[E0425]: cannot find function `marks_id` in module `crate::ui::grid`
error[E0599]: no variant named `ReadFont` found for enum `backend::Command`
...
error: could not compile `tabletist` (lib test) due to 18 previous errors; 1 warning emitted
```

- [ ] **Step 3: Read a font by its fontconfig name**

```diff
diff --git a/src/typography/desktop_font.rs b/src/typography/desktop_font.rs
index 5683bb2..25b7674 100644
--- a/src/typography/desktop_font.rs
+++ b/src/typography/desktop_font.rs
@@ -1,5 +1,8 @@
 //! The desktop's monospace font on Linux, as fontconfig resolves it
-//! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it).
+//! (JetBrainsMono Nerd Font on Omarchy; `omarchy font set` changes it), or
+//! the font the settings name by its fontconfig name.
+
+use super::fonts::TerminalFaces;
 
 #[cfg(target_os = "linux")]
 use std::{
@@ -12,7 +15,13 @@ use std::{
 #[cfg(target_os = "linux")]
 const FC_MATCH_LIMIT: Duration = Duration::from_millis(500);
 
-/// Reads `fc-match -f '%{file}\n%{index}'`: an absolute path and a face index.
+/// What `fc-match` is asked to write: a file, a face index and the
+/// families' names.
+#[cfg(target_os = "linux")]
+const FORMAT: &str = "%{file}\n%{index}\n%{family}";
+
+/// Reads the first two lines of what `fc-match` is asked to write: an
+/// absolute path and a face index.
 /// Linux only: the path is POSIX, which Windows would not call absolute.
 #[cfg(target_os = "linux")]
 pub fn parse_fc_match(output: &str) -> Option<(PathBuf, u32)> {
@@ -23,6 +32,108 @@ pub fn parse_fc_match(output: &str) -> Option<(PathBuf, u32)> {
     path.is_absolute().then_some((path, index))
 }
 
+/// The family `fc-match`'s third line names, as it is asked to write it:
+/// the first of its names
+/// (`JetBrainsMono Nerd Font,JetBrainsMono NF` is the first).
+pub fn parse_family(output: &str) -> Option<String> {
+    let family = output.lines().nth(2)?.split(',').next()?.trim();
+    (!family.is_empty()).then(|| family.to_owned())
+}
+
+/// Whether `family`, which fontconfig matched, is the font `name` asked
+/// for: the same name, or one that begins with it, whatever the case and
+/// the spaces (`JetBrains Mono` is `JetBrainsMono Nerd Font`). For a name it
+/// does not know, fontconfig gives another font.
+pub fn is_named(family: &str, name: &str) -> bool {
+    let squeeze = |text: &str| {
+        text.chars()
+            .filter(|char| !char.is_whitespace())
+            .flat_map(char::to_lowercase)
+            .collect::<String>()
+    };
+    let name = squeeze(name);
+    !name.is_empty() && squeeze(family).starts_with(&name)
+}
+
+/// `name` as a fontconfig pattern writes a family: with a backslash
+/// before what the pattern reads otherwise (`-` before a size, `:` before
+/// a property, `,` between families).
+pub fn pattern(name: &str) -> String {
+    let mut pattern = String::with_capacity(name.len());
+    for char in name.trim().chars() {
+        if matches!(char, '\\' | '-' | ':' | ',') {
+            pattern.push('\\');
+        }
+        pattern.push(char);
+    }
+    pattern
+}
+
+/// What fontconfig gave the terminal look for the font the settings name.
+#[derive(Clone, Debug, Default)]
+pub struct Read {
+    /// The font asked for, as the settings name it: `None` is the
+    /// system's monospace.
+    pub asked: Option<String>,
+    /// The faces at 400, 500 and 700: `None` where fontconfig gave none
+    /// that can be drawn (the bundled one is drawn then).
+    pub faces: Option<TerminalFaces>,
+    /// The family drawn, as fontconfig names it.
+    pub family: Option<String>,
+    /// Whether the font asked for is not there: the system's monospace is
+    /// drawn in its place.
+    pub missing: bool,
+}
+
+/// The font named `asked` (the system's monospace for `None`), at 400, 500
+/// and 700, or the system's monospace in its place where fontconfig does
+/// not have it. Each `fc-match` is bounded to half a second; a font found
+/// takes three, a name not found four: two seconds at worst. It runs at the
+/// start (as the fonts always did) and on the backend after.
+pub fn read(asked: Option<&str>) -> Read {
+    let named = asked
+        .map(str::trim)
+        .filter(|name| !name.is_empty() && !name.eq_ignore_ascii_case("monospace"));
+    // The medium and bold of the font `base` gave as `regular`.
+    let weights = |base: &str, regular: egui::FontData| {
+        let medium = matched(&format!("{base}:medium")).map(|(face, _)| face);
+        let bold = matched(&format!("{base}:bold")).map(|(face, _)| face);
+        let regular = std::sync::Arc::new(regular);
+        let medium = medium.map_or_else(|| regular.clone(), std::sync::Arc::new);
+        let bold = bold.map_or_else(|| medium.clone(), std::sync::Arc::new);
+        [regular, medium, bold]
+    };
+    let faces_of = |base: &str| {
+        let (regular, family) = matched(base)?;
+        Some((weights(base, regular), family))
+    };
+    let mut missing = false;
+    let found = match named {
+        Some(name) => {
+            let base = pattern(name);
+            // The weights are asked for only once the name is found.
+            match matched(&base) {
+                Some((regular, Some(family))) if is_named(&family, name) => {
+                    Some((weights(&base, regular), Some(family)))
+                }
+                _ => {
+                    missing = true;
+                    log::warn!("the font {name:?} is not found: the system's monospace is drawn");
+                    faces_of("monospace")
+                }
+            }
+        }
+        None => faces_of("monospace"),
+    };
+    let (faces, family) = found.map_or((None, None), |(faces, family)| (Some(faces), family));
+    Read {
+        asked: asked.map(str::to_owned),
+        faces,
+        family,
+        missing,
+    }
+}
+
 /// Runs `command` and returns its output, or `None` if it cannot start, fails,
 /// or is still running after `limit` (then it is killed and reaped). Each
 /// `None` logs exactly one warning. Stderr is discarded.
@@ -95,20 +206,20 @@ pub fn load(path: &Path, index: u32) -> Option<egui::FontData> {
     Some(data)
 }
 
-/// The desktop's monospace font for a fontconfig `pattern` (`monospace`,
-/// `monospace:bold`), or `None` (with one logged warning) when fontconfig is
-/// missing or names a file egui could not read.
-pub fn monospace(pattern: &str) -> Option<egui::FontData> {
+/// The font fontconfig gives for `pattern` (`monospace`, `Fira
+/// Code:bold`), with the family it names, or `None` (with one logged
+/// warning) when fontconfig is missing or names a file egui could not read.
+fn matched(pattern: &str) -> Option<(egui::FontData, Option<String>)> {
     #[cfg(target_os = "linux")]
     {
         let output = run_bounded(
-            Command::new("fc-match").args(["-f", "%{file}\n%{index}", pattern]),
+            Command::new("fc-match").args(["-f", FORMAT, pattern]),
             FC_MATCH_LIMIT,
         );
         let output = output?;
         let text = String::from_utf8_lossy(&output.stdout);
         let Some((path, index)) = parse_fc_match(&text) else {
-            log::warn!("fc-match gave no monospace font");
+            log::warn!("fc-match gave no font for {pattern:?}");
             return None;
         };
         let font = load(&path, index);
@@ -118,7 +229,7 @@ pub fn monospace(pattern: &str) -> Option<egui::FontData> {
                 path.display()
             );
         }
-        font
+        font.map(|font| (font, parse_family(&text)))
     }
     #[cfg(not(target_os = "linux"))]
     {
```

- [ ] **Step 4: Hand the faces in**

`configure` takes the terminal's faces, read by the caller, and no longer reads them. `install_with` keeps the fonts it set, and `set_terminal_faces` puts a new read's faces into them.

```diff
diff --git a/src/testing.rs b/src/testing.rs
index e4f70b6..89e5b84 100644
--- a/src/testing.rs
+++ b/src/testing.rs
@@ -1588,7 +1588,7 @@ impl Harness {
     pub fn set_look(&mut self, look: crate::theme::Look) {
         let look = look.with_text(&self.app.settings);
         self.app.look = look;
-        crate::theme::install(&self.ctx, false, &look);
+        self.app.fonts = crate::theme::install_with(&self.ctx, false, &look, None);
         crate::theme::apply(&self.ctx, &self.app.palette, &look);
     }
 }
diff --git a/src/theme.rs b/src/theme.rs
index 528bd05..1723b05 100644
--- a/src/theme.rs
+++ b/src/theme.rs
@@ -4,6 +4,7 @@
 use egui::{Color32, CornerRadius, Stroke, Vec2};
 
 use crate::settings::{FontSize, FontSizes, InterfaceSize, Mode, RowHeight, Settings, Theme};
+use crate::typography::{Kind, TerminalFaces};
 
 /// A palette file from the themes directory.
 pub type CustomTheme = fastframe_theme::CustomTheme<Palette>;
@@ -750,15 +751,47 @@ fn text_rendering() -> fastframe_text::TextRendering {
 /// Installs fonts, image loaders and icons once per egui context.
 /// `system_fallbacks` is off in tests and screenshots for reproducible output.
 pub fn install(ctx: &egui::Context, system_fallbacks: bool, look: &Look) {
+    let _ = install_with(ctx, system_fallbacks, look, None);
+}
+
+/// [`install`], with `desktop`, the terminal look's font as fontconfig gave
+/// it (never in tests or screenshots): the bundled one is drawn without.
+/// Returns the fonts it set, for [`set_terminal_faces`].
+pub fn install_with(
+    ctx: &egui::Context,
+    system_fallbacks: bool,
+    look: &Look,
+    desktop: Option<&TerminalFaces>,
+) -> egui::FontDefinitions {
     let mut fonts = fastframe_fonts::FontSetup::default()
         .system_fallbacks(system_fallbacks)
         .definitions();
-    // Tests and screenshots (no system fallbacks) stay reproducible.
-    crate::typography::configure(&mut fonts, look, system_fallbacks);
+    crate::typography::configure(&mut fonts, look, desktop);
     text_rendering().apply_to(&mut fonts);
-    ctx.set_fonts(fonts);
+    ctx.set_fonts(fonts.clone());
     egui_extras::install_image_loaders(ctx);
     fastframe_icons::install::<Icon>(ctx);
+    fonts
+}
+
+/// Draws the terminal look in `desktop`, the font fontconfig gave for the
+/// settings' name, or in the bundled one for none: `fonts`, the ones set,
+/// with the terminal's faces put in. Nothing is read from the disk here.
+pub fn set_terminal_faces(
+    ctx: &egui::Context,
+    fonts: &mut egui::FontDefinitions,
+    look: &Look,
+    desktop: Option<&TerminalFaces>,
+) {
+    crate::typography::configure(fonts, look, desktop);
+    // The others have the rendering's hinting already.
+    for weight in [400, 500, 700] {
+        let name = crate::typography::family_name(Typeface::Terminal, Kind::Mono, weight);
+        if let Some(data) = fonts.font_data.get_mut(&name) {
+            text_rendering().tweak(&mut std::sync::Arc::make_mut(data).tweak);
+        }
+    }
+    ctx.set_fonts(fonts.clone());
 }
 
 /// Applies the palette and look to egui's own widgets and text styles.
diff --git a/src/typography.rs b/src/typography.rs
index 93275f7..e166193 100644
--- a/src/typography.rs
+++ b/src/typography.rs
@@ -19,7 +19,7 @@
 pub mod desktop_font;
 mod fonts;
 
-pub use fonts::configure;
+pub use fonts::{TerminalFaces, configure};
 
 use std::sync::Arc;
 
diff --git a/src/typography/fonts.rs b/src/typography/fonts.rs
index 4f04a4d..3d92bea 100644
--- a/src/typography/fonts.rs
+++ b/src/typography/fonts.rs
@@ -128,9 +128,13 @@ fn alias(fonts: &mut FontDefinitions, name: &str, from: &FontFamily) {
     fonts.families.insert(FontFamily::Name(name.into()), list);
 }
 
-/// Replaces the faces fastframe set up with the look's. `desktop` allows
-/// reading the desktop's own monospace font (never in tests or screenshots).
-pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: bool) {
+/// The terminal look's monospace at 400, 500 and 700.
+pub type TerminalFaces = [Arc<FontData>; 3];
+
+/// Replaces the faces fastframe set up with the look's. `desktop` is the
+/// desktop's own monospace that fontconfig gave the terminal look (never in
+/// tests or screenshots), which has the bundled one drawn without it.
+pub fn configure(fonts: &mut FontDefinitions, look: &Look, desktop: Option<&TerminalFaces>) {
     match look.faces.typeface {
         Typeface::Inter => inter(fonts),
         Typeface::Plex => plex(fonts),
@@ -216,19 +220,8 @@ fn plex(fonts: &mut FontDefinitions) {
     inter(fonts);
 }
 
-/// The desktop's monospace at 400, 500 and 700, when fontconfig has it.
-fn desktop_faces() -> Option<[Arc<FontData>; 3]> {
-    let regular = super::desktop_font::monospace("monospace")?;
-    let medium = super::desktop_font::monospace("monospace:medium");
-    let bold = super::desktop_font::monospace("monospace:bold");
-    let regular = Arc::new(regular);
-    let medium = medium.map_or_else(|| regular.clone(), Arc::new);
-    let bold = bold.map_or_else(|| medium.clone(), Arc::new);
-    Some([regular, medium, bold])
-}
-
 #[cfg(any(target_os = "linux", test))]
-fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
+fn bundled_terminal_faces() -> Option<TerminalFaces> {
     Some([
         weighted(JETBRAINS_MONO, 400.0),
         weighted(JETBRAINS_MONO, 500.0),
@@ -237,16 +230,13 @@ fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
 }
 
 #[cfg(not(any(target_os = "linux", test)))]
-fn bundled_terminal_faces() -> Option<[Arc<FontData>; 3]> {
+fn bundled_terminal_faces() -> Option<TerminalFaces> {
     None
 }
 
 /// Omarchy: one monospace face for the interface and the data alike.
-fn terminal(fonts: &mut FontDefinitions, desktop: bool) {
-    let faces = desktop
-        .then(desktop_faces)
-        .flatten()
-        .or_else(bundled_terminal_faces);
+fn terminal(fonts: &mut FontDefinitions, desktop: Option<&TerminalFaces>) {
+    let faces = desktop.cloned().or_else(bundled_terminal_faces);
     let Some(faces) = faces else {
         // No monospace of its own: the families still exist, on Inter.
         for weight in [400, 500, 700] {
```

- [ ] **Step 5: Read on the backend, and draw what it read**

The grid's marks for empty values were found once for each set of faces; the terminal's face can now change under the same `Faces`, so a read forgets them.

```diff
diff --git a/src/app.rs b/src/app.rs
index 68f06aa..55d042e 100644
--- a/src/app.rs
+++ b/src/app.rs
@@ -159,6 +159,18 @@ pub struct App {
     /// The pin the status line last said could not be read: it is said
     /// once.
     pin_told: Option<String>,
+    /// What fontconfig gave the terminal look for the font the settings
+    /// name: nothing in tests, the demo and the other looks.
+    pub desktop_font: crate::typography::desktop_font::Read,
+    /// The number of the last font read asked for, and of the newest one
+    /// taken, as for the theme's.
+    font_asked: u64,
+    font_taken: u64,
+    /// A font read came back since the last `logic`: its faces are to be
+    /// drawn.
+    font_read: bool,
+    /// The fonts set at the start, which a font read puts its faces in.
+    pub(crate) fonts: egui::FontDefinitions,
     /// The look's text changed since the last `logic`: egui's own text
     /// styles follow it.
     restyle: bool,
@@ -297,6 +309,11 @@ impl App {
             theme_asked: 0,
             theme_taken: 0,
             pin_told: None,
+            desktop_font: crate::typography::desktop_font::Read::default(),
+            font_asked: 0,
+            font_taken: 0,
+            font_read: false,
+            fonts: egui::FontDefinitions::empty(),
             restyle: false,
             zoom: None,
             window_title: "Tabletist".into(),
@@ -3387,6 +3404,15 @@ impl App {
                 }
             }
             Event::SettingsWatch { live } => self.settings_file.live = live,
+            Event::Font { asked, read } => {
+                // An older read that ends later read a font since renamed.
+                if asked < self.font_taken {
+                    return;
+                }
+                self.font_taken = asked;
+                self.desktop_font = *read;
+                self.font_read = true;
+            }
             Event::Theme { asked, read } => {
                 // Reads end in any order: one older than the newest taken
                 // saw a theme of Omarchy's that is gone, and one that read
@@ -4114,6 +4140,9 @@ impl App {
             self.look = look;
             self.restyle = true;
         }
+        if old.font != self.settings.font {
+            self.read_desktop_font();
+        }
         // An edit of the file that turns the server on is held.
         self.check_agents();
         if self.serves_agents {
@@ -5579,7 +5608,14 @@ impl App {
         // The font size keys are the keymap's: they change the setting,
         // where egui's would zoom the whole window and forget it.
         ctx.options_mut(|options| options.zoom_with_keyboard = false);
-        theme::install(ctx, follow_desktop, &self.look);
+        // Read here, before the first frame, as the desktop's monospace
+        // always was: a font that arrived later would move every line.
+        if follow_desktop && self.look.terminal {
+            self.desktop_font =
+                crate::typography::desktop_font::read(self.settings.font.as_deref());
+        }
+        let desktop = self.desktop_font.faces.as_ref();
+        self.fonts = theme::install_with(ctx, follow_desktop, &self.look, desktop);
         if follow_desktop {
             theme::enable_desktop_themes(&mut self.themes);
             let repaint = ctx.clone();
@@ -5626,6 +5662,19 @@ impl App {
         });
     }
 
+    /// Asks the backend for the terminal look's font as the settings name
+    /// it, where the desktop's fonts are read (not in tests or the demo).
+    fn read_desktop_font(&mut self) {
+        if !self.look.terminal || !self.follow_desktop {
+            return;
+        }
+        self.font_asked += 1;
+        self.backend.send(Command::ReadFont {
+            font: self.settings.font.clone(),
+            asked: self.font_asked,
+        });
+    }
+
     /// How large the window draws everything: the terminal look's text
     /// size against its 13 points. The other looks size their text by
     /// role, and draw at 1.
@@ -5735,6 +5784,11 @@ impl App {
             }
         }
         self.tell_missing_pin(ctx);
+        if std::mem::take(&mut self.font_read) {
+            let desktop = self.desktop_font.faces.as_ref();
+            theme::set_terminal_faces(ctx, &mut self.fonts, &self.look, desktop);
+            crate::ui::grid::forget_marks(ctx, &self.look);
+        }
         if std::mem::take(&mut self.restyle) {
             theme::apply(ctx, &self.palette, &self.look);
         }
diff --git a/src/backend.rs b/src/backend.rs
index 861637a..039e4fe 100644
--- a/src/backend.rs
+++ b/src/backend.rs
@@ -196,6 +196,13 @@ pub enum Command {
         pin: Option<String>,
         asked: u64,
     },
+    /// Reads the terminal look's font through fontconfig: the one `font`
+    /// names, or the system's monospace. Answered with [`Event::Font`],
+    /// which carries `asked`: reads end in any order.
+    ReadFont {
+        font: Option<String>,
+        asked: u64,
+    },
     /// Signals `done` once every save sent before it is on disk.
     Flush {
         done: mpsc::Sender<()>,
@@ -358,6 +365,11 @@ pub enum Event {
         asked: u64,
         read: crate::desktop_theme::Read,
     },
+    /// What the [`Command::ReadFont`] numbered `asked` read.
+    Font {
+        asked: u64,
+        read: Box<crate::typography::desktop_font::Read>,
+    },
     /// The time zone database, read whole ([`Command::LoadZones`]).
     Zones(Arc<crate::zones::Zones>),
     /// The settings file changed on disk: its text. `own` when it is, byte
@@ -1906,6 +1918,16 @@ impl Worker {
                     outbox.emit(Event::Theme { asked, read });
                 });
             }
+            Command::ReadFont { font, asked } => {
+                let outbox = self.outbox.clone();
+                tokio::task::spawn_blocking(move || {
+                    let read = crate::typography::desktop_font::read(font.as_deref());
+                    outbox.emit(Event::Font {
+                        asked,
+                        read: Box::new(read),
+                    });
+                });
+            }
             Command::Flush { done } => {
                 let saves = self.saves.clone();
                 tokio::spawn(async move {
@@ -1957,6 +1979,7 @@ fn session_of(command: &Command) -> SessionId {
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
+        | Command::ReadFont { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => SessionId(0),
     }
@@ -1987,6 +2010,7 @@ fn request_of(command: &Command) -> Option<RequestId> {
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
+        | Command::ReadFont { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => None,
     }
@@ -2075,6 +2099,7 @@ fn fail(outbox: &Outbox, command: Command, error: Error) {
         | Command::EditSettingsFile { .. }
         | Command::RevealSettingsFile { .. }
         | Command::ReadTheme { .. }
+        | Command::ReadFont { .. }
         | Command::Flush { .. }
         | Command::Agents(_) => return,
     };
@@ -2359,6 +2384,7 @@ async fn run_session(
             | Command::EditSettingsFile { .. }
             | Command::RevealSettingsFile { .. }
             | Command::ReadTheme { .. }
+            | Command::ReadFont { .. }
             | Command::Flush { .. }
             | Command::Agents(_) => None,
         };
diff --git a/src/ui/grid.rs b/src/ui/grid.rs
index db34417..fad160c 100644
--- a/src/ui/grid.rs
+++ b/src/ui/grid.rs
@@ -286,7 +286,7 @@ pub fn data_role(look: &Look) -> TextRole {
 /// design's when the fonts at hand have them (see [`Marks::pick`]). Found
 /// once for each set of faces, not for every cell.
 pub fn marks(ctx: &egui::Context, look: &Look) -> Marks {
-    let id = Id::new(("cell-marks", look.faces));
+    let id = marks_id(look);
     if let Some(marks) = ctx.data(|data| data.get_temp::<Marks>(id)) {
         return marks;
     }
@@ -296,6 +296,17 @@ pub fn marks(ctx: &egui::Context, look: &Look) -> Marks {
     marks
 }
 
+/// Where the marks found for `look`'s faces are kept.
+pub(crate) fn marks_id(look: &Look) -> Id {
+    Id::new(("cell-marks", look.faces))
+}
+
+/// Forgets the marks found for `look`'s faces: the terminal look's face
+/// changed under them (a new font read through fontconfig).
+pub fn forget_marks(ctx: &egui::Context, look: &Look) {
+    ctx.data_mut(|data| data.remove::<Marks>(marks_id(look)));
+}
+
 /// Paints `text` in `role` at `x` (its left edge, or its right with
 /// `right`), centred on `y`.
 #[allow(clippy::too_many_arguments)] // the text, its place, and its look
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `cargo test --locked --lib -- desktop_font app::tests::a_ typography::fonts`
Expected: PASS. Then the four checks above.

- [ ] **Step 7: Try it on the desktop**

On Omarchy, by hand: `font = "Fira Code"` (or another installed monospace) in `settings.toml` while Tabletist runs redraws the window in it; a name that is not installed draws the system's monospace. Say in the pull request whether this was done.

- [ ] **Step 8: Commit**

```bash
git add src/typography/desktop_font.rs src/typography/fonts.rs src/typography.rs src/theme.rs src/backend.rs src/app.rs src/testing.rs src/ui/grid.rs src/ui/data_view.rs
git commit -m "Read the terminal look's font through fontconfig, at the start and on the backend"
```

---

### Task 5: The options, and the sheet's Text section

**Files:**
- Modify: `src/settings.rs`, `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/ui/settings/terminal.rs`, `src/ui/text_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The three options are stored under their keys, on the Appearance pane in the sections each look draws; the window's size steps are the terminal look's, and the interface's size steps through its three. On macOS the sheet's Text section has the board's words, sets the interface's size, the data font and its size with clicks, and writes each; the default face is no value. Windows has the size's menu and no face's. A font that is not bundled is drawn as the default, which the row says in place of its small print. In the smallest window the row stays inside the sheet, its font menu narrower. Reset to defaults puts the text back. Under the Settings window ⌘= shows in the size's menu.

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 7b11bb0..033e982 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -3164,6 +3164,9 @@ bar = \"label\"  # tint | label
                 Key::ProductionReadOnly,
                 Key::ConfirmCloseEdits,
                 Key::Theme,
+                Key::InterfaceSize,
+                Key::Font,
+                Key::FontSize,
                 Key::RowHeight,
                 Key::Zebra,
                 Key::ColumnTypes,
@@ -3185,13 +3188,22 @@ bar = \"label\"  # tint | label
         // options, in order.
         let sections = [&OptionId::DATA[..], &OptionId::SAFETY[..]].concat();
         assert_eq!(sections, OptionId::GENERAL);
+        // The sheet's Text section has the size beside the font, on its
+        // row; the terminal look's has a row for each, and no interface
+        // size.
         let sections = [
             &OptionId::THEME[..],
+            &OptionId::TEXT[..],
+            &[OptionId::FontSize],
             &OptionId::GRID[..],
             &OptionId::ENVIRONMENT[..],
         ]
         .concat();
         assert_eq!(sections, OptionId::APPEARANCE);
+        assert_eq!(
+            OptionId::TERMINAL_TEXT,
+            [OptionId::Font, OptionId::FontSize]
+        );
         assert_eq!(panes, OptionId::ALL);
         let defaults = Settings::default();
         for option in OptionId::ALL {
@@ -3590,5 +3602,36 @@ bar = \"label\"  # tint | label
         assert_eq!(step(13.0, true, true), "14");
         assert_eq!(step(24.0, true, true), "24");
         assert_eq!(step(9.0, true, false), "9");
+        // The window's steps are the terminal look's.
+        let settings = Settings::default();
+        assert_eq!(
+            settings.stepped(OptionId::FontSize, true),
+            OptionValue::FontSize(FontSize::of(14.0))
+        );
+    }
+
+    #[test]
+    fn the_interface_size_steps_through_its_three() {
+        let step = |size: InterfaceSize, forward: bool| {
+            let settings = Settings {
+                interface_size: size,
+                ..Settings::default()
+            };
+            settings.stepped(OptionId::InterfaceSize, forward)
+        };
+        assert_eq!(
+            step(InterfaceSize::Default, true),
+            OptionValue::InterfaceSize(InterfaceSize::Large)
+        );
+        assert_eq!(
+            step(InterfaceSize::Large, true),
+            OptionValue::InterfaceSize(InterfaceSize::Large)
+        );
+        assert_eq!(
+            step(InterfaceSize::Small, false),
+            OptionValue::InterfaceSize(InterfaceSize::Small)
+        );
+        let points = InterfaceSize::ALL.map(InterfaceSize::points);
+        assert_eq!(points, [12.0, 13.0, 14.0]);
     }
 }
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index aeeb3b2..f06d691 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -4,10 +4,11 @@
 
 use egui::{Key, Modifiers};
 
-use super::appearance_settings_tests::{on_the_pane, settings_saves};
-use crate::settings::{FontSize, InterfaceSize};
+use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
+use crate::model::Action;
+use crate::settings::{FontSize, InterfaceSize, OptionValue};
 use crate::testing::Harness;
-use crate::theme::Look;
+use crate::theme::{DataFace, Look};
 
 /// One pass of the app's logic, as the window runs it between frames.
 fn logic(harness: &mut Harness) {
@@ -123,6 +124,12 @@ fn the_mac_keys_step_and_reset_the_data_size_and_write_it() {
     on_the_pane(&mut harness);
     harness.press(Key::Equals, Modifiers::COMMAND);
     assert_eq!(harness.app.settings.font_size, FontSize::of(13.0));
+    assert!(harness.has("Data font size"));
+    let tree = harness.settle();
+    let menu = crate::testing::node(&tree, "Data font size", egui::accesskit::Role::ComboBox)
+        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
+        .map(|(_, node)| node.value().map(str::to_owned));
+    assert_eq!(menu, Some(Some("13 pt".to_owned())));
 }
 
 #[test]
@@ -145,3 +152,122 @@ fn the_terminal_keys_step_the_size_anywhere_and_stop_at_its_ends() {
     harness.press(Key::Num0, Modifiers::CTRL);
     assert_eq!(harness.app.settings.font_size, FontSize::of(9.0));
 }
+
+#[test]
+fn the_sheet_sets_the_interface_size_and_the_data_font() {
+    let mut harness = sheet(Look::macos());
+    on_the_pane(&mut harness);
+    // The board's words.
+    for words in [
+        "TEXT",
+        "Sidebar, tabs, buttons, inspector labels",
+        "Grid cells, SQL editor, JSON",
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    for size in ["Small · 12", "Default · 13", "Large · 14"] {
+        assert!(harness.has(&format!("Interface size: {size}")), "{size}");
+    }
+    let saves = settings_saves(&harness);
+    harness.click("Interface size: Large · 14");
+    harness.click("Data font");
+    harness.click("Data font JetBrains Mono");
+    harness.click("Data font size");
+    harness.click("Data font size 14 pt");
+    harness.settle();
+    let settings = &harness.app.settings;
+    assert_eq!(settings.interface_size, InterfaceSize::Large);
+    assert_eq!(settings.font.as_deref(), Some("JetBrains Mono"));
+    assert_eq!(settings.font_size, FontSize::of(14.0));
+    assert_eq!(settings_saves(&harness), saves + 3);
+    for line in [
+        "interface_size = \"large\"",
+        "font = \"JetBrains Mono\"",
+        "font_size = 14\n",
+    ] {
+        assert!(file(&harness).contains(line), "{line}");
+    }
+    // Every role drawn from here on has them.
+    let faces = harness.app.look.faces;
+    assert_eq!(faces.interface, InterfaceSize::Large);
+    assert_eq!(faces.data, DataFace::JetBrainsMono);
+    assert_eq!(faces.data_size, FontSize::halves(28));
+    // The default face is no value: the file leaves it out.
+    harness.click("Data font");
+    harness.click("Data font IBM Plex Mono");
+    harness.settle();
+    assert_eq!(harness.app.settings.font, None);
+    assert!(!file(&harness).contains("font = "));
+}
+
+#[test]
+fn windows_sizes_its_data_and_has_no_face_to_pick() {
+    let mut harness = sheet(Look::standard());
+    on_the_pane(&mut harness);
+    assert!(harness.has("Interface size: Default · 13"));
+    let tree = harness.settle();
+    let menu = |name| crate::testing::node(&tree, name, egui::accesskit::Role::ComboBox);
+    assert!(menu("Data font size").is_some());
+    assert!(menu("Data font").is_none(), "its data is drawn in Inter");
+    harness.click("Data font size");
+    harness.click("Data font size 11 pt");
+    harness.settle();
+    assert_eq!(harness.app.settings.font_size, FontSize::of(11.0));
+}
+
+#[test]
+fn a_data_font_that_is_not_bundled_is_drawn_as_the_default_and_said() {
+    let mut harness = sheet(Look::macos());
+    harness.app.change_settings(|settings| {
+        settings.font = Some("Fira Code".into());
+    });
+    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
+    on_the_pane(&mut harness);
+    assert!(shows(
+        &mut harness,
+        "“Fira Code” is not bundled: IBM Plex Mono is used"
+    ));
+    assert!(!shows(&mut harness, "Grid cells, SQL editor, JSON"));
+    // The menu shows the face drawn.
+    let tree = harness.settle();
+    let menu = crate::testing::node(&tree, "Data font", egui::accesskit::Role::ComboBox)
+        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
+        .map(|(_, node)| node.value().map(str::to_owned));
+    assert_eq!(menu, Some(Some("IBM Plex Mono".to_owned())));
+}
+
+#[test]
+fn the_data_font_row_fits_the_smallest_sheet() {
+    let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    let tree = harness.settle();
+    let menu = |name| crate::testing::bounds(&tree, name, egui::accesskit::Role::ComboBox);
+    let (font, size) = (menu("Data font").unwrap(), menu("Data font size").unwrap());
+    // The sheet ends 40 in from the window's edge, the font's menu gives
+    // way.
+    assert!(size.right() <= 680.0, "{size:?}");
+    assert!(
+        font.width() < 220.0 && font.right() < size.left(),
+        "{font:?}"
+    );
+}
+
+#[test]
+fn reset_to_defaults_puts_the_text_back() {
+    let mut harness = sheet(Look::macos());
+    for value in [
+        OptionValue::InterfaceSize(InterfaceSize::Small),
+        OptionValue::Font(Some("JetBrains Mono".into())),
+        OptionValue::FontSize(FontSize::of(15.0)),
+    ] {
+        harness.app.apply(Action::SetOption(value));
+    }
+    on_the_pane(&mut harness);
+    harness.app.apply(Action::ResetSettings);
+    harness.app.apply(Action::ConfirmResetSettings(true));
+    let settings = &harness.app.settings;
+    assert_eq!(settings.interface_size, InterfaceSize::Default);
+    assert_eq!(settings.font, None);
+    assert_eq!(settings.font_size, None);
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- settings::tests text_settings`
Expected: the crate's tests do not compile:

```
error[E0599]: no variant, associated function, or constant named `TEXT` found for enum `settings::OptionId` in the current scope
error[E0599]: no variant, associated function, or constant named `FontSize` found for enum `settings::OptionId` in the current scope
error[E0599]: no variant, associated function, or constant named `TERMINAL_TEXT` found for enum `settings::OptionId` in the current scope
...
error: could not compile `tabletist` (lib test) due to 14 previous errors
```

- [ ] **Step 3: The options**

```diff
diff --git a/src/settings.rs b/src/settings.rs
index 033e982..f842b40 100644
--- a/src/settings.rs
+++ b/src/settings.rs
@@ -647,6 +647,12 @@ pub enum OptionId {
     /// Whether a new agent is told of: `mcp.notify`.
     AgentsNotify,
     Theme,
+    /// macOS's and Windows's Interface size: `appearance.interface_size`.
+    InterfaceSize,
+    /// The data font, or the terminal look's font: `appearance.font`.
+    Font,
+    /// The size that goes with it: `appearance.font_size`.
+    FontSize,
     RowHeight,
     Zebra,
     ColumnTypes,
@@ -657,7 +663,7 @@ pub enum OptionId {
 impl OptionId {
     /// Every option, the General pane's, the Appearance pane's and then
     /// the Agents pane's.
-    pub const ALL: [OptionId; 16] = [
+    pub const ALL: [OptionId; 19] = [
         Self::PageSize,
         Self::Timestamps,
         Self::Tz,
@@ -666,6 +672,9 @@ impl OptionId {
         Self::ProductionReadOnly,
         Self::ConfirmCloseEdits,
         Self::Theme,
+        Self::InterfaceSize,
+        Self::Font,
+        Self::FontSize,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
@@ -695,9 +704,14 @@ impl OptionId {
     ];
     pub const SAFETY: [OptionId; 2] = [Self::ProductionReadOnly, Self::ConfirmCloseEdits];
     /// The Appearance pane's, in the order it has them: its Theme
-    /// section's, its Grid section's, then its Environment section's.
-    pub const APPEARANCE: [OptionId; 6] = [
+    /// section's, its Text section's, its Grid section's, then its
+    /// Environment section's. Each look shows the Text section's its own
+    /// way ([`OptionId::TEXT`], [`OptionId::TERMINAL_TEXT`]).
+    pub const APPEARANCE: [OptionId; 9] = [
         Self::Theme,
+        Self::InterfaceSize,
+        Self::Font,
+        Self::FontSize,
         Self::RowHeight,
         Self::Zebra,
         Self::ColumnTypes,
@@ -705,6 +719,11 @@ impl OptionId {
         Self::EnvBar,
     ];
     pub const THEME: [OptionId; 1] = [Self::Theme];
+    /// macOS's and Windows's Text section: the interface's size, and the
+    /// data font's row, which has the size's menu beside the font's.
+    pub const TEXT: [OptionId; 2] = [Self::InterfaceSize, Self::Font];
+    /// The terminal look's: one font and one size for everything.
+    pub const TERMINAL_TEXT: [OptionId; 2] = [Self::Font, Self::FontSize];
     pub const GRID: [OptionId; 4] = [
         Self::RowHeight,
         Self::Zebra,
@@ -729,6 +748,9 @@ impl OptionId {
             Self::LogDays => Key::McpLogDays,
             Self::AgentsNotify => Key::McpNotify,
             Self::Theme => Key::Theme,
+            Self::InterfaceSize => Key::InterfaceSize,
+            Self::Font => Key::Font,
+            Self::FontSize => Key::FontSize,
             Self::RowHeight => Key::RowHeight,
             Self::Zebra => Key::Zebra,
             Self::ColumnTypes => Key::ColumnTypes,
@@ -758,6 +780,11 @@ pub enum OptionValue {
     LogDays(u32),
     AgentsNotify(bool),
     Theme(ThemeChoice),
+    InterfaceSize(InterfaceSize),
+    /// `None` is the look's own font.
+    Font(Option<String>),
+    /// `None` is the look's own size.
+    FontSize(Option<FontSize>),
     RowHeight(RowHeight),
     /// `None` is the look's default.
     Zebra(Option<bool>),
@@ -780,6 +807,9 @@ impl OptionValue {
             Self::LogDays(_) => OptionId::LogDays,
             Self::AgentsNotify(_) => OptionId::AgentsNotify,
             Self::Theme(_) => OptionId::Theme,
+            Self::InterfaceSize(_) => OptionId::InterfaceSize,
+            Self::Font(_) => OptionId::Font,
+            Self::FontSize(_) => OptionId::FontSize,
             Self::RowHeight(_) => OptionId::RowHeight,
             Self::Zebra(_) => OptionId::Zebra,
             Self::ColumnTypes(_) => OptionId::ColumnTypes,
@@ -813,6 +843,9 @@ impl OptionValue {
                     ThemeChoice::Pinned => return,
                 }
             }
+            Self::InterfaceSize(size) => settings.interface_size = size,
+            Self::Font(font) => settings.font = font,
+            Self::FontSize(size) => settings.font_size = size,
             Self::RowHeight(height) => settings.row_height = height,
             Self::Zebra(on) => settings.zebra = on,
             Self::ColumnTypes(on) => settings.column_types = on,
@@ -1306,6 +1339,9 @@ impl Settings {
             OptionId::LogDays => OptionValue::LogDays(self.mcp_log_days),
             OptionId::AgentsNotify => OptionValue::AgentsNotify(self.mcp_notify),
             OptionId::Theme => OptionValue::Theme(self.theme.choice()),
+            OptionId::InterfaceSize => OptionValue::InterfaceSize(self.interface_size),
+            OptionId::Font => OptionValue::Font(self.font.clone()),
+            OptionId::FontSize => OptionValue::FontSize(self.font_size),
             OptionId::RowHeight => OptionValue::RowHeight(self.row_height),
             OptionId::Zebra => OptionValue::Zebra(self.zebra),
             OptionId::ColumnTypes => OptionValue::ColumnTypes(self.column_types),
@@ -1384,6 +1420,25 @@ impl Settings {
             } else {
                 ThemeChoice::Omarchy
             }),
+            OptionId::InterfaceSize => {
+                let at = InterfaceSize::ALL
+                    .iter()
+                    .position(|choice| *choice == self.interface_size)
+                    .unwrap_or(1);
+                let next = if forward {
+                    (at + 1).min(InterfaceSize::ALL.len() - 1)
+                } else {
+                    at.saturating_sub(1)
+                };
+                OptionValue::InterfaceSize(InterfaceSize::ALL[next])
+            }
+            // A name is set in the file: the window has none to step to.
+            OptionId::Font => OptionValue::Font(self.font.clone()),
+            // The terminal look's steps: the window steps the size there.
+            // macOS and Windows have a menu of theirs.
+            OptionId::FontSize => {
+                OptionValue::FontSize(Some(self.stepped_font_size(true, forward)))
+            }
             OptionId::RowHeight => {
                 let at = RowHeight::ALL
                     .iter()
@@ -1451,6 +1506,7 @@ impl Settings {
                 Some(_) => ThemeChoice::Omarchy,
                 None => ThemeChoice::Pinned,
             })),
+            OptionId::InterfaceSize | OptionId::Font | OptionId::FontSize => None,
             OptionId::RowHeight => None,
             // Unset, it is flipped from off: the window flips the
             // settings as it shows them, with the look's default in.
```

- [ ] **Step 4: What the rows say, in both looks**

The labels and small print of the board, the choices the terminal look's keys step through, and the settings as the window shows them (the size as the look draws it while it is not set).

```diff
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 255941b..df56e33 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -17,8 +17,8 @@ use crate::i18n::{Locale, gettext, ngettext};
 use crate::keymap::{Command, Keymap, Layout, Scope, Spelled, Stroke};
 use crate::model::{Action, AgentsView, Dialog, SettingsDialog, SettingsPane, SettingsRow};
 use crate::settings::{
-    EnvBar, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, Theme, ThemeChoice,
-    Timestamps, Zone,
+    EnvBar, InterfaceSize, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, Theme,
+    ThemeChoice, Timestamps, Zone,
 };
 
 /// The label of an option's row. English: the caller translates, and the
@@ -36,6 +36,9 @@ pub(super) fn label(option: OptionId) -> &'static str {
         OptionId::LogDays => "Keep the activity log",
         OptionId::AgentsNotify => "Notify when an agent connects",
         OptionId::Theme => "Theme",
+        OptionId::InterfaceSize => "Interface size",
+        OptionId::Font => "Data font",
+        OptionId::FontSize => "Data font size",
         OptionId::RowHeight => "Row density",
         OptionId::Zebra => "Alternate row shading",
         OptionId::ColumnTypes => "Show column types",
@@ -57,6 +60,9 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         OptionId::Zebra => "Alternate rows",
         OptionId::ColumnTypes => "Column types",
         OptionId::EnvBar => "Connection bar",
+        // One font and one size for everything.
+        OptionId::Font => "Font",
+        OptionId::FontSize => "Size",
         OptionId::PageSize
         | OptionId::Timestamps
         | OptionId::GroupDigits
@@ -65,6 +71,7 @@ pub(super) fn terminal_label(option: OptionId) -> &'static str {
         | OptionId::LogDays
         | OptionId::AgentsNotify
         | OptionId::Theme
+        | OptionId::InterfaceSize
         | OptionId::TableList => label(option),
     }
 }
@@ -80,17 +87,24 @@ pub(super) fn small_print(option: OptionId) -> Option<&'static str> {
         OptionId::ColumnTypes => Some("Second line in headers: int8, varchar, jsonb"),
         OptionId::TableList => Some("Default for new connections; the sidebar toggle overrides it"),
         OptionId::EnvBar => Some("Production always uses the full tint"),
+        OptionId::InterfaceSize => Some("Sidebar, tabs, buttons, inspector labels"),
+        OptionId::Font => Some("Grid cells, SQL editor, JSON"),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::ConfirmCloseEdits
         | OptionId::AgentsOn
         | OptionId::AgentsNotify
         | OptionId::Theme
+        | OptionId::FontSize
         | OptionId::RowHeight
         | OptionId::Zebra => None,
     }
 }
 
+/// What is said of a data font the settings name that is not bundled,
+/// `{name}` for its name: the default is drawn instead. English.
+pub const FONT_NOT_BUNDLED: &str = "“{name}” is not bundled: IBM Plex Mono is used";
+
 /// What is said of a pinned theme that is nowhere to be found, `{name}`
 /// for its name: what is drawn instead. English.
 pub fn pin_missing(catalog: &crate::theme::Catalog) -> &'static str {
@@ -138,14 +152,15 @@ pub(super) fn is_on(value: &OptionValue) -> bool {
 /// The settings as the window shows them: the server's switch as it is in
 /// force, off where a change made outside the app that turned it on is
 /// held, the grouping of numbers as the look has it where it is not set,
-/// the stripes as the look draws them while they are not set, and the
-/// theme as the look reads it: a value of the other looks' is the desktop
-/// followed.
+/// the stripes and the font's size as the look draws them while they are
+/// not set, and the theme as the look reads it: a value of the other
+/// looks' is the desktop followed.
 pub(super) fn shown(app: &App) -> Settings {
     let mut settings = app.settings.clone();
     settings.mcp_enabled = app.agents_on();
     settings.group_digits = Some(app.settings.grouped(app.layout()));
     settings.zebra = Some(settings.zebra_in(app.look.terminal));
+    settings.font_size = Some(settings.font_size_in(app.look.terminal));
     settings.theme = match (app.look.terminal, settings.theme.mode()) {
         (true, _) if settings.theme.pin().is_some() => settings.theme,
         (true, _) => Theme::Omarchy,
@@ -265,6 +280,26 @@ pub(super) fn choices(
             ),
             (Word("pin"), OptionValue::Theme(ThemeChoice::Pinned)),
         ],
+        OptionId::InterfaceSize => vec![
+            (
+                Word("small"),
+                OptionValue::InterfaceSize(InterfaceSize::Small),
+            ),
+            (
+                Word("default"),
+                OptionValue::InterfaceSize(InterfaceSize::Default),
+            ),
+            (
+                Word("large"),
+                OptionValue::InterfaceSize(InterfaceSize::Large),
+            ),
+        ],
+        // A name is set in the file: the window offers none.
+        OptionId::Font => Vec::new(),
+        OptionId::FontSize => vec![
+            (Word("smaller"), settings.stepped(option, false)),
+            (Word("larger"), settings.stepped(option, true)),
+        ],
         OptionId::RowHeight => vec![
             (Word("compact"), OptionValue::RowHeight(RowHeight::Compact)),
             (Word("regular"), OptionValue::RowHeight(RowHeight::Regular)),
```

- [ ] **Step 5: The sheet's rows**

```diff
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index b95644e..2011570 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -10,10 +10,10 @@ use crate::app::App;
 use crate::i18n::{Locale, gettext, ngettext};
 use crate::model::{Action, SettingsDialog, SettingsPane};
 use crate::settings::{
-    EnvBar, Mode, OptionId, OptionValue, RowHeight, Settings, TableList, ThemeChoice, Timestamps,
-    Zone,
+    EnvBar, FontSizes, InterfaceSize, Mode, OptionId, OptionValue, RowHeight, Settings, TableList,
+    ThemeChoice, Timestamps, Zone,
 };
-use crate::theme::{Look, Palette};
+use crate::theme::{DataFace, Look, Palette, Typeface};
 use crate::typography::{Laid, Text, TextRole};
 use crate::ui::focus::{self, Ring};
 use crate::ui::format::group_digits;
@@ -49,6 +49,13 @@ const HINT_GAP: f32 = 12.0;
 const MENU: f32 = 120.0;
 /// The zone's menu: as wide as the artboard's, for `System · America/…`.
 const ZONE_MENU: f32 = 220.0;
+/// The data font's menu and its size's, as wide as the artboard's. A
+/// narrow sheet takes from the font's, down to the least it has.
+const FONT_MENU: f32 = 220.0;
+const FONT_MENU_MIN: f32 = 100.0;
+const SIZE_MENU: f32 = 90.0;
+/// Between the data font's menu and its size's.
+const MENU_GAP: f32 = 12.0;
 /// How tall the zone menu's list of zones is at most: it scrolls.
 const ZONE_LIST: f32 = 240.0;
 /// A segment's height, and the switch's.
@@ -171,6 +178,7 @@ pub(super) fn show(
                     ("Appearance", None),
                     &[
                         ("Theme", &OptionId::THEME),
+                        ("Text", &OptionId::TEXT),
                         ("Grid", &OptionId::GRID),
                         ("Environment color", &OptionId::ENVIRONMENT),
                     ],
@@ -550,8 +558,17 @@ pub(super) fn row(
     )
     .wrap(column)
     .layout(ui.ctx());
-    let print = small_print(option).map(|print| {
-        Text::one(look, TextRole::Secondary, &skin.say(print), palette.dim)
+    // A data font that is not bundled is said in place of the small
+    // print, where the row has the room for it.
+    let print = match not_bundled(option, settings, look) {
+        Some(missing) => {
+            let said = skin.say(super::FONT_NOT_BUNDLED).replace("{name}", missing);
+            Some((said, palette.warning))
+        }
+        None => small_print(option).map(|print| (skin.say(print), palette.dim)),
+    };
+    let print = print.map(|(print, color)| {
+        Text::one(look, TextRole::Secondary, &print, color)
             .wrap(column)
             .layout(ui.ctx())
     });
@@ -605,6 +622,15 @@ pub(super) fn row(
     hint(ui, pos2(left, middle), rect.right(), option, settings, skin);
 }
 
+/// The data font the settings name, on its row, where it is no face the
+/// look has: the default is drawn in its place.
+fn not_bundled<'a>(option: OptionId, settings: &'a Settings, look: &Look) -> Option<&'a str> {
+    let plex = look.faces.typeface == Typeface::Plex;
+    (option == OptionId::Font && plex)
+        .then(|| DataFace::of(settings.font.as_deref()).err())
+        .flatten()
+}
+
 /// Whether the keyboard came in this frame to a control in `place`, on the
 /// layer `ui` draws on: the open menu's entries hang over the rows under
 /// it, on a layer of their own.
@@ -625,6 +651,8 @@ fn control_height(option: OptionId, look: &Look) -> f32 {
         // The track has 2 over its segments and under them.
         OptionId::Timestamps | OptionId::GroupDigits => SEGMENT + 4.0,
         OptionId::RowHeight | OptionId::TableList | OptionId::EnvBar => SEGMENT + 4.0,
+        OptionId::InterfaceSize => SEGMENT + 4.0,
+        OptionId::Font | OptionId::FontSize => look.control_height,
         OptionId::ValueTags
         | OptionId::ProductionReadOnly
         | OptionId::ConfirmCloseEdits
@@ -726,6 +754,40 @@ fn control(
                 )));
             }
         }
+        OptionId::InterfaceSize => {
+            // Each with the size of its body text, as the board writes
+            // them.
+            let words = InterfaceSize::ALL.map(|size| {
+                let word = match size {
+                    InterfaceSize::Small => "Small",
+                    InterfaceSize::Default => "Default",
+                    InterfaceSize::Large => "Large",
+                };
+                format!("{} · {}", skin.say(word), size.points())
+            });
+            let selected = InterfaceSize::ALL
+                .iter()
+                .position(|size| *size == settings.interface_size)
+                .unwrap_or(1);
+            if let Some(picked) = segments(ui, &name, &words, selected, skin) {
+                actions.push(Action::SetOption(OptionValue::InterfaceSize(
+                    InterfaceSize::ALL[picked],
+                )));
+            }
+        }
+        // The face's menu where the look draws data in a face of its own
+        // (macOS), then the size's. Windows draws its data in Inter, the
+        // interface's face: the size's alone.
+        OptionId::Font => {
+            if look.faces.typeface == Typeface::Plex {
+                let width =
+                    (ui.available_width() - MENU_GAP - SIZE_MENU).clamp(FONT_MENU_MIN, FONT_MENU);
+                font_menu(ui, (&name, width), settings, skin, actions);
+                ui.add_space(MENU_GAP);
+            }
+            size_menu(ui, settings, skin, actions);
+        }
+        OptionId::FontSize => size_menu(ui, settings, skin, actions),
         OptionId::TableList => {
             let words = [skin.say("Grouped by prefix"), skin.say("Flat")];
             let selected = usize::from(settings.table_list.flat());
@@ -784,6 +846,68 @@ fn control(
     }
 }
 
+/// The data font's menu: the bundled faces. A name that is none of them
+/// is drawn as the default, which the row's hint says.
+fn font_menu(
+    ui: &mut Ui,
+    (name, width): (&str, f32),
+    settings: &Settings,
+    skin: &Skin,
+    actions: &mut Vec<Action>,
+) {
+    let Skin { look, palette, .. } = *skin;
+    let drawn = DataFace::of(settings.font.as_deref()).unwrap_or_default();
+    let dropdown = menu::Dropdown::new("data-font", drawn.name(), width);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        for face in DataFace::ALL {
+            let entry_name = format!("{name} {}", face.name());
+            let entry = menu::Item::choice(face.name(), face == drawn)
+                .name(&entry_name)
+                .show(ui, look, palette);
+            if entry.clicked() {
+                // The default is no value: the file leaves it out.
+                let font = (face != DataFace::default()).then(|| face.name().to_owned());
+                actions.push(Action::SetOption(OptionValue::Font(font)));
+                ui.close();
+            }
+        }
+    });
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
+        info.current_text_value = Some(drawn.name().to_owned());
+        info
+    });
+}
+
+/// The data's size: a menu of every size macOS and Windows draw, by half
+/// points.
+fn size_menu(ui: &mut Ui, settings: &Settings, skin: &Skin, actions: &mut Vec<Action>) {
+    let Skin { look, palette, .. } = *skin;
+    let name = skin.say(label(OptionId::FontSize));
+    let points = |size: crate::settings::FontSize| format!("{} pt", size.text());
+    let drawn = settings.font_size_in(false);
+    let shown = points(drawn);
+    let dropdown = menu::Dropdown::new("data-font-size", &shown, SIZE_MENU);
+    let menu = dropdown.show(ui, look, palette, |ui| {
+        for size in FontSizes::of(false).all() {
+            let text = points(size);
+            let entry_name = format!("{name} {text}");
+            let entry = menu::Item::choice(&text, size == drawn)
+                .name(&entry_name)
+                .show(ui, look, palette);
+            if entry.clicked() {
+                actions.push(Action::SetOption(OptionValue::FontSize(Some(size))));
+                ui.close();
+            }
+        }
+    });
+    menu.response.widget_info(|| {
+        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &name);
+        info.current_text_value = Some(shown.clone());
+        info
+    });
+}
+
 /// The days the log's menu offers: the list, with the days in use in their
 /// place when the file set others.
 fn log_days(current: u32) -> Vec<u32> {
@@ -921,6 +1045,9 @@ fn hint(ui: &Ui, at: egui::Pos2, right: f32, option: OptionId, settings: &Settin
         | OptionId::AgentsOn
         | OptionId::LogDays
         | OptionId::AgentsNotify
+        | OptionId::InterfaceSize
+        | OptionId::Font
+        | OptionId::FontSize
         | OptionId::RowHeight
         | OptionId::Zebra
         | OptionId::ColumnTypes
```

- [ ] **Step 6: The terminal look's values and hints**

What the screen draws for the two options, used from Task 6 on, when its pane lists them.

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index eec655c..941b055 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -61,6 +61,30 @@ pub(super) struct Skin<'a> {
     pub(super) named: Option<&'a str>,
     /// What the theme's rows say of the desktop.
     pub(super) desktop: Desktop<'a>,
+    /// What the font's row says of the font drawn.
+    pub(super) font: FontShown<'a>,
+}
+
+/// What fontconfig gave for the font the settings name, which the font's
+/// row says beside its value.
+#[derive(Clone, Copy)]
+pub(super) struct FontShown<'a> {
+    /// The family drawn, as fontconfig names it: `None` where nothing was
+    /// read from it (the bundled face is drawn then).
+    pub(super) family: Option<&'a str>,
+    /// The font the settings name, where fontconfig does not have it: the
+    /// system's monospace is drawn in its place.
+    pub(super) missing: Option<&'a str>,
+}
+
+impl<'a> FontShown<'a> {
+    pub(super) fn of(app: &'a App) -> Self {
+        let read = &app.desktop_font;
+        Self {
+            family: read.family.as_deref(),
+            missing: read.asked.as_deref().filter(|_| read.missing),
+        }
+    }
 }
 
 /// What the Appearance pane's theme rows say beside their values: Omarchy's
@@ -129,6 +153,7 @@ pub(super) fn show(
         system: app.system_zone.iana_name(),
         named: app.named_zone.as_deref(),
         desktop: Desktop::of(app),
+        font: FontShown::of(app),
     };
     let screen = ctx.content_rect();
     let id = egui::Id::new("settings");
@@ -513,6 +538,49 @@ fn theme_hint(skin: &Skin) -> Text {
     }
 }
 
+/// What the font's row says beside its value: the family fontconfig drew,
+/// or that the font named is not there.
+fn font_hint(skin: &Skin) -> Text {
+    let Skin {
+        look,
+        palette,
+        font,
+        ..
+    } = *skin;
+    let role = TextRole::OBody;
+    let family = font.family.map(|family| look.label(family));
+    match (font.missing, family) {
+        (None, Some(family)) => {
+            let said = skin.say("now: {family}, from fontconfig");
+            Text::one(look, role, &said.replace("{family}", &family), palette.dim)
+        }
+        (None, None) => Text::one(
+            look,
+            role,
+            &skin.say("now: jetbrains mono, bundled"),
+            palette.dim,
+        ),
+        // Shorter: the row has the room for what was not found first.
+        (Some(name), family) => {
+            let drawn = family.unwrap_or_else(|| skin.say("jetbrains mono, bundled"));
+            let said = skin.say("“{name}” not found · now: {family}");
+            let said = said.replace("{name}", name).replace("{family}", &drawn);
+            Text::one(look, role, &said, palette.warning)
+        }
+    }
+}
+
+/// The font's value: its name as the settings write it, or the system's
+/// monospace.
+fn font_value(settings: &Settings, skin: &Skin) -> String {
+    match settings.font.as_deref().map(str::trim) {
+        Some(name) if !name.is_empty() && !name.eq_ignore_ascii_case("monospace") => {
+            name.to_owned()
+        }
+        _ => skin.say("system monospace"),
+    }
+}
+
 /// The Agents pane: the server's switch and its status, how to connect an
 /// agent, where access is set, and the log. Its rows are
 /// [`SettingsPane::rows`]'s, in order; the command, the line for other
@@ -853,12 +921,14 @@ fn value_width(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> f
     let width = |text: &str| role.width(ui.ctx(), skin.look.faces, text);
     match option {
         // Between its chevrons, as the cursor's row has it.
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::FontSize => {
             width(&format!("‹ {} ›", stepped_text(option, settings, skin)))
         }
+        OptionId::Font => width(&font_value(settings, skin)),
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::Theme
+        | OptionId::InterfaceSize
         | OptionId::RowHeight
         | OptionId::TableList
         | OptionId::EnvBar => {
@@ -891,6 +961,7 @@ fn stepped_text(option: OptionId, settings: &Settings, skin: &Skin) -> String {
             }
             (zone, system) => super::zone_text(skin.locale, zone, system),
         }),
+        OptionId::FontSize => settings.font_size_in(true).text(),
         _ => settings.page_size.to_string(),
     }
 }
@@ -1003,7 +1074,7 @@ fn option_row(
     if let Some(button) = button
         && matches!(
             option,
-            OptionId::PageSize | OptionId::LogDays | OptionId::Tz
+            OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::FontSize
         )
     {
         let name = gettext(locale, terminal_label(option));
@@ -1025,12 +1096,23 @@ fn option_row(
         on_cursor,
     };
     match option {
-        OptionId::PageSize | OptionId::LogDays | OptionId::Tz => {
+        OptionId::PageSize | OptionId::LogDays | OptionId::Tz | OptionId::FontSize => {
             stepper(ui, cell, option, settings, skin, actions)
         }
+        // Nothing to step to: a name is set in the file.
+        OptionId::Font => {
+            let value = Text::one(
+                skin.look,
+                TextRole::OBody,
+                &font_value(settings, skin),
+                skin.palette.text,
+            );
+            widgets::paint_text(ui, cell.left, y, value);
+        }
         OptionId::Timestamps
         | OptionId::GroupDigits
         | OptionId::Theme
+        | OptionId::InterfaceSize
         | OptionId::RowHeight
         | OptionId::TableList
         | OptionId::EnvBar => {
@@ -1201,6 +1283,16 @@ fn hint(ui: &Ui, option: OptionId, settings: &Settings, skin: &Skin) -> Text {
             .add(role, &format!(" {}", skin.say("shows it")), palette.dim),
         OptionId::AgentsNotify => dim(&skin.say("a desktop notification")),
         OptionId::Theme => theme_hint(skin),
+        // macOS's and Windows's: no row of this look's.
+        OptionId::InterfaceSize => Text::new(look),
+        OptionId::Font => font_hint(skin),
+        // The keys that change it, from the keymap, wherever the keyboard
+        // is.
+        OptionId::FontSize => Text::new(look)
+            .add(role, &key(ui, look, Command::LargerText), palette.accent)
+            .add(role, " / ", palette.dim)
+            .add(role, &key(ui, look, Command::SmallerText), palette.accent)
+            .add(role, &format!(" {}", skin.say("anywhere")), palette.dim),
         OptionId::RowHeight => dim(&skin.say("1 · 1.5 · 2 lines")),
         OptionId::Zebra => Text::new(look),
         OptionId::ColumnTypes => dim(&skin.say("second header line")),
```

- [ ] **Step 7: Run the tests to see them pass**

Run: `cargo test --locked --lib -- settings::tests text_settings`
Expected: PASS. Then the four checks above.

- [ ] **Step 8: Commit**

```bash
git add src/settings.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/ui/settings/terminal.rs src/ui/text_settings_tests.rs
git commit -m "Set the interface size and the data font from Settings › Appearance on macOS and Windows"
```

---

### Task 6: The font and its size on the Omarchy screen

**Files:**
- Modify: `src/model.rs`, `src/ui/settings/terminal.rs`, `src/ui/appearance_settings_tests.rs`, `src/ui/text_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The screen shows the board's rows, the font as the bundled one in tests and the size's keys as the keymap writes them; `l` and `h` on the size row step it and write it, `R` takes it out, and the font row has nothing to step to: a name set stays, and nothing is written. A font fontconfig drew is said dim, one it does not have in the warning colour. Step 1's key tests move two rows further down, past the font and the size.

```diff
diff --git a/src/ui/appearance_settings_tests.rs b/src/ui/appearance_settings_tests.rs
index bd70a42..1cdc9b7 100644
--- a/src/ui/appearance_settings_tests.rs
+++ b/src/ui/appearance_settings_tests.rs
@@ -462,8 +462,11 @@ fn the_screen_sets_each_appearance_option_with_its_keys() {
     let key = |harness: &mut Harness, key: egui::Key, text: &str| {
         super::tests::type_key(harness, key, text);
     };
-    // Row height, under the theme: one to the left, compact.
-    key(&mut harness, egui::Key::J, "j");
+    // Row height, under the theme and the text's two: one to the left,
+    // compact.
+    for _ in 0..3 {
+        key(&mut harness, egui::Key::J, "j");
+    }
     key(&mut harness, egui::Key::H, "h");
     // Alternate rows, on in the terminal until set: space turns it off.
     key(&mut harness, egui::Key::J, "j");
@@ -545,7 +548,7 @@ fn a_reset_and_a_toggle_in_one_frame_turn_the_look_s_stripes_off() {
     let mut harness = Harness::new();
     harness.set_look(Look::omarchy());
     on_the_pane(&mut harness);
-    for _ in 0..2 {
+    for _ in 0..4 {
         super::tests::type_key(&mut harness, egui::Key::J, "j");
     }
     change(&mut harness, |settings| settings.zebra = Some(true));
diff --git a/src/ui/text_settings_tests.rs b/src/ui/text_settings_tests.rs
index f06d691..25b0ccf 100644
--- a/src/ui/text_settings_tests.rs
+++ b/src/ui/text_settings_tests.rs
@@ -9,6 +9,7 @@ use crate::model::Action;
 use crate::settings::{FontSize, InterfaceSize, OptionValue};
 use crate::testing::Harness;
 use crate::theme::{DataFace, Look};
+use crate::typography::desktop_font::Read;
 
 /// One pass of the app's logic, as the window runs it between frames.
 fn logic(harness: &mut Harness) {
@@ -34,6 +35,10 @@ fn terminal() -> Harness {
     harness
 }
 
+fn key(harness: &mut Harness, key: Key, text: &str) {
+    super::tests::type_key(harness, key, text);
+}
+
 /// The settings file as the app last wrote it.
 fn file(harness: &Harness) -> &str {
     &harness.app.settings_file.text
@@ -271,3 +276,85 @@ fn reset_to_defaults_puts_the_text_back() {
     assert_eq!(settings.font, None);
     assert_eq!(settings.font_size, None);
 }
+
+#[test]
+fn the_screen_shows_the_font_and_steps_the_size_with_its_keys() {
+    let mut harness = terminal();
+    on_the_pane(&mut harness);
+    // The board's rows, the font as the bundled one in tests: nothing is
+    // read from fontconfig. The size's keys as the keymap writes them.
+    let keymap = crate::keymap::Keymap::default();
+    let written = |command| keymap.label(crate::keymap::Layout::Omarchy, command);
+    let keys = format!(
+        "{} / {} anywhere",
+        written(crate::keymap::Command::LargerText),
+        written(crate::keymap::Command::SmallerText)
+    );
+    for words in [
+        "text",
+        "system monospace",
+        "now: jetbrains mono, bundled",
+        &keys,
+    ] {
+        assert!(shows(&mut harness, words), "{words}");
+    }
+    let saves = settings_saves(&harness);
+    // The size, under the theme and the font: a step up, then two down.
+    key(&mut harness, Key::J, "j");
+    key(&mut harness, Key::J, "j");
+    assert!(shows(&mut harness, "13"));
+    key(&mut harness, Key::L, "l");
+    assert_eq!(harness.app.settings.font_size, FontSize::of(14.0));
+    assert!(file(&harness).contains("font_size = 14\n"));
+    key(&mut harness, Key::H, "h");
+    key(&mut harness, Key::H, "h");
+    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
+    assert_eq!(settings_saves(&harness), saves + 3);
+    // R takes it out of the file, the look's own again.
+    harness.press(Key::R, Modifiers::SHIFT);
+    assert_eq!(harness.app.settings.font_size, None);
+    assert!(!file(&harness).contains("font_size"));
+    // The font has nothing to step to: a name is set in the file.
+    harness.app.change_settings(|settings| {
+        settings.font = Some("Fira Code".into());
+    });
+    let saves = settings_saves(&harness);
+    key(&mut harness, Key::K, "k");
+    key(&mut harness, Key::L, "l");
+    harness.press(Key::Space, Modifiers::NONE);
+    assert_eq!(harness.app.settings.font.as_deref(), Some("Fira Code"));
+    assert_eq!(settings_saves(&harness), saves);
+}
+
+#[test]
+fn the_screen_says_the_font_fontconfig_drew_and_one_it_has_not() {
+    let mut harness = terminal();
+    harness.app.change_settings(|settings| {
+        settings.font = Some("Fira Code".into());
+    });
+    harness.app.desktop_font = Read {
+        asked: Some("Fira Code".into()),
+        faces: None,
+        family: Some("JetBrainsMono Nerd Font".into()),
+        missing: true,
+    };
+    on_the_pane(&mut harness);
+    assert!(shows(&mut harness, "Fira Code"));
+    let said = "“Fira Code” not found · now: jetbrainsmono nerd font";
+    assert!(shows(&mut harness, said));
+    assert_eq!(
+        harness.painted_color(said),
+        Some(harness.app.palette.warning)
+    );
+    // Found: what fontconfig drew, dim.
+    harness.app.desktop_font.missing = false;
+    harness.app.desktop_font.family = Some("Fira Code".into());
+    let said = "now: fira code, from fontconfig";
+    assert!(shows(&mut harness, said));
+    assert_eq!(harness.painted_color(said), Some(harness.app.palette.dim));
+    // R puts the system's monospace back.
+    key(&mut harness, Key::J, "j");
+    harness.press(Key::R, Modifiers::SHIFT);
+    assert_eq!(harness.app.settings.font, None);
+    assert!(!file(&harness).contains("font ="));
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- text_settings appearance_settings`
Expected:

```
test ui::text_settings_tests::the_screen_shows_the_font_and_steps_the_size_with_its_keys ... FAILED
test ui::text_settings_tests::the_screen_says_the_font_fontconfig_drew_and_one_it_has_not ... FAILED
test ui::appearance_settings_tests::a_reset_and_a_toggle_in_one_frame_turn_the_look_s_stripes_off ... FAILED
test ui::appearance_settings_tests::the_screen_sets_each_appearance_option_with_its_keys ... FAILED
test result: FAILED. 25 passed; 4 failed; 0 ignored; 0 measured; 2699 filtered out
```

- [ ] **Step 3: The rows**

```diff
diff --git a/src/model.rs b/src/model.rs
index e8f91e1..7fd381b 100644
--- a/src/model.rs
+++ b/src/model.rs
@@ -2500,6 +2500,8 @@ impl SettingsPane {
             ],
             Self::Appearance => &[
                 SettingsRow::Option(OptionId::Theme),
+                SettingsRow::Option(OptionId::Font),
+                SettingsRow::Option(OptionId::FontSize),
                 SettingsRow::Option(OptionId::RowHeight),
                 SettingsRow::Option(OptionId::Zebra),
                 SettingsRow::Option(OptionId::ColumnTypes),
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 941b055..0595e89 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -409,6 +409,7 @@ fn appearance_rows(
     let mut index = 0;
     for (heading_text, top, options) in [
         ("Theme", 8.0, &OptionId::THEME[..]),
+        ("Text", 12.0, &OptionId::TERMINAL_TEXT[..]),
         ("Grid", 12.0, &OptionId::GRID[..]),
         ("Environment", 12.0, &OptionId::ENVIRONMENT[..]),
     ] {
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- text_settings appearance_settings`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/model.rs src/ui/settings/terminal.rs src/ui/appearance_settings_tests.rs src/ui/text_settings_tests.rs
git commit -m "Show the font and its size on the Omarchy Settings screen"
```

---

### Task 7: The fonts' licenses in About

**Files:**
- Modify: `src/ui/about.rs`

- [ ] **Step 1: Write the failing test**

The link unfolds the notices, a second press folds them, and so does any close. Not on Windows, which bundles none of these fonts and draws no link.

```diff
diff --git a/src/ui/about.rs b/src/ui/about.rs
index 08657c9..c654594 100644
--- a/src/ui/about.rs
+++ b/src/ui/about.rs
@@ -190,6 +190,38 @@ mod tests {
         }
     }
 
+    /// Windows bundles no font of its own: it has no link.
+    #[cfg(any(target_os = "macos", target_os = "linux"))]
+    #[test]
+    fn about_shows_the_licenses_of_the_fonts_it_bundles() {
+        let mut harness = Harness::new();
+        open(&mut harness);
+        harness.click("Font licenses");
+        harness.settle();
+        let shown = |harness: &Harness, words: &str| {
+            harness.painted.iter().any(|(text, _)| text.contains(words))
+        };
+        // This build's: JetBrains Mono on Linux, with its notice whole.
+        assert!(shown(&harness, "The JetBrains Mono Project Authors"));
+        assert!(shown(&harness, "SIL OPEN FONT LICENSE Version 1.1"));
+        // A second press folds them, and so does a close.
+        harness.click("Font licenses");
+        harness.settle();
+        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
+        harness.click("Font licenses");
+        harness.click("Close");
+        open(&mut harness);
+        harness.settle();
+        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
+        // And when it closes another way.
+        harness.click("Font licenses");
+        harness.app.apply(Action::CloseDialog);
+        harness.settle();
+        harness.app.apply(Action::ShowAbout);
+        harness.settle();
+        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
+    }
+
     #[test]
     fn close_and_escape_close_about() {
         let mut harness = Harness::new();
```

- [ ] **Step 2: Run the test to see it fail**

Run: `cargo test --locked --lib -- ui::about`
Expected:

```
test ui::about::tests::about_shows_the_licenses_of_the_fonts_it_bundles ... FAILED
test result: FAILED. 5 passed; 1 failed; 0 ignored; 0 measured; 2723 filtered out
```

- [ ] **Step 3: The link and the notices**

```diff
diff --git a/src/ui/about.rs b/src/ui/about.rs
index c654594..ee4da00 100644
--- a/src/ui/about.rs
+++ b/src/ui/about.rs
@@ -16,6 +16,26 @@ const ICON_SIZE: f32 = 72.0;
 /// As LICENSE has it.
 const COPYRIGHT: &str = "© 2026 Igor Alexandrov";
 
+/// The fonts this build bundles, each with its licence as the font's
+/// authors give it: the SIL Open Font License has its notice go with the
+/// font. Windows draws in fastframe's fonts, and bundles none of its own.
+const FONT_NOTICES: &[&str] = &[
+    #[cfg(target_os = "macos")]
+    include_str!("../../assets/fonts/IBMPlex-OFL.txt"),
+    #[cfg(any(target_os = "macos", target_os = "linux"))]
+    include_str!("../../assets/fonts/JetBrainsMono-OFL.txt"),
+    #[cfg(target_os = "macos")]
+    include_str!("../../assets/fonts/NotoSansSymbols2-OFL.txt"),
+];
+
+/// Whether the fonts' notices are unfolded, in egui's memory.
+fn notices_id() -> egui::Id {
+    egui::Id::new("about-font-licenses")
+}
+
+/// How tall the fonts' notices are at most: they scroll.
+const NOTICES_HEIGHT: f32 = 140.0;
+
 /// The version and what it was built for: what a bug report needs.
 pub fn version() -> String {
     format!(
@@ -34,6 +54,9 @@ pub fn repository() -> &'static str {
 
 pub fn show(app: &mut App, ctx: &egui::Context) {
     if !matches!(app.dialog, Some(Dialog::About)) {
+        // The next About starts with the notices folded, whatever took
+        // its place.
+        ctx.data_mut(|data| data.remove::<bool>(notices_id()));
         return;
     }
     let locale = app.locale;
@@ -84,6 +107,7 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
                 palette.secondary,
                 &look,
             );
+            font_notices(ui, &look, &palette, locale);
         });
         ui.add_space(12.0);
         ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
@@ -100,6 +124,44 @@ pub fn show(app: &mut App, ctx: &egui::Context) {
     app.actions.extend(actions);
 }
 
+/// The link that shows the bundled fonts' licences, and under it, once it
+/// is pressed, their notices: selectable, to be copied whole. Whether they
+/// show is the dialog's own, kept in egui's memory while it is open.
+fn font_notices(
+    ui: &mut egui::Ui,
+    look: &crate::theme::Look,
+    palette: &crate::theme::Palette,
+    locale: crate::i18n::Locale,
+) {
+    if FONT_NOTICES.is_empty() {
+        return;
+    }
+    let id = notices_id();
+    let mut open = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
+    ui.add_space(2.0);
+    let label = gettext(locale, "Font licenses");
+    let link = widgets::ButtonSpec::new(&label).link().padding(6.0);
+    let size = vec2(link.width(ui, look), 24.0);
+    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
+    if link.show_at(ui, rect, look, palette).clicked() {
+        open = !open;
+        ui.data_mut(|data| data.insert_temp(id, open));
+    }
+    if !open {
+        return;
+    }
+    ui.add_space(4.0);
+    let notices = FONT_NOTICES.join("\n\n");
+    egui::ScrollArea::vertical()
+        .max_height(NOTICES_HEIGHT)
+        .show(ui, |ui| {
+            let text = Text::one(look, widgets::secondary(look), &notices, palette.secondary)
+                .wrap(ui.available_width())
+                .layout(ui.ctx());
+            ui.add(egui::Label::new(text.galley).selectable(true));
+        });
+}
+
 /// `text` across the dialog, wrapped, with every line centred.
 fn centered(ui: &mut egui::Ui, mut text: Text) {
     let width = ui.available_width();
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::about`
Expected: PASS (`about_and_the_way_to_it_fit_the_smallest_window` with them). Then the four checks above.

- [ ] **Step 5: Commit**

```bash
git add src/ui/about.rs
git commit -m "Show the licenses of the bundled fonts in About"
```

---

### Task 8: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`, `docs/_reference/keyboard-shortcuts.md`, `docs/_guide/omarchy.md`, `docs/_guide/macos.md`

- [ ] **Step 1: Describe the settings, the keys and the font**

````diff
diff --git a/docs/_guide/macos.md b/docs/_guide/macos.md
index 6a50193..f599b90 100644
--- a/docs/_guide/macos.md
+++ b/docs/_guide/macos.md
@@ -27,7 +27,13 @@ launch, are on the
   Appearance setting while it runs. **Settings › Appearance** can keep it
   light or dark whatever the system says.
 - **The app menu has the usual two.** **Tabletist → About Tabletist**
-  shows the version, and **Tabletist → Settings…** opens Settings.
+  shows the version and the licenses of the fonts it bundles, and
+  **Tabletist → Settings…** opens Settings.
+- **Text has two sizes.** **Settings › Appearance** sizes the interface
+  (Small, Default or Large) and the data apart: grid cells, the SQL
+  editor and JSON, in IBM Plex Mono or JetBrains Mono, from 11 to 16
+  points. Cmd+= and Cmd+- change the data's size wherever you are, and
+  Cmd+0 puts back 12.5.
 
 {% include shot.html file="macos-table-dark" alt="The same window in the dark appearance" %}
 
@@ -55,6 +61,7 @@ a Mac keyboard labels Delete.
 | Cmd+Option+A | Show the Agents tab |
 | F6, Shift+F6 | Next and previous part of the window |
 | Cmd+, | Settings |
+| Cmd+=, Cmd+-, Cmd+0 | Larger, smaller and default data text |
 | `?` | The list of keys |
 
 ### Quick open
diff --git a/docs/_guide/omarchy.md b/docs/_guide/omarchy.md
index ce1444d..ba66bcd 100644
--- a/docs/_guide/omarchy.md
+++ b/docs/_guide/omarchy.md
@@ -37,6 +37,7 @@ back.
 | Ctrl+H, Ctrl+L | Move between the parts of the window |
 | F6, Shift+F6 | Next and previous part of the window |
 | Ctrl+, | Settings |
+| Ctrl+=, Ctrl+- | Larger and smaller text: the whole window, under a dialog too |
 | `:agents` | Show the Agents tab |
 | `:agents log` | Show the Agents tab's log |
 | `?` | The list of keys |
@@ -320,4 +321,22 @@ Cmd/Ctrl+, opens Settings. In the Omarchy look it is a full screen with
 `settings.toml` shown beside the options. Edits made to the file in your
 editor are applied while Tabletist runs.
 
+### The font and its size
+
+Tabletist draws in the desktop's monospace font, as fontconfig resolves
+it (`omarchy font set` changes it), and **Settings › Appearance** names the
+font drawn. Another font is named in `settings.toml` by its fontconfig
+name; one fontconfig does not have is drawn as the system's monospace,
+and the font row says so:
+
+```toml
+[appearance]
+font = "Fira Code"
+font_size = 15
+```
+
+The size, 9 to 24, sizes the whole window, as a terminal's font size
+does. `h` and `l` on the size row change it, and so do Ctrl+= and Ctrl+-
+anywhere in the window; `R` on the row puts back 13.
+
 {% include shot.html file="omarchy-settings" alt="The Settings screen in the Omarchy look: the options on the left and settings.toml on the right" width="2880" height="1800" %}
diff --git a/docs/_reference/keyboard-shortcuts.md b/docs/_reference/keyboard-shortcuts.md
index 8f2d603..a210eeb 100644
--- a/docs/_reference/keyboard-shortcuts.md
+++ b/docs/_reference/keyboard-shortcuts.md
@@ -57,6 +57,7 @@ In the connection dialog:
 | Mod+Shift+[ and ] | Previous and next tab |
 | F6, Shift+F6 | Next and previous part of the window |
 | Mod+, | Settings |
+| Mod+=, Mod+-, Mod+0 | Larger, smaller and default data text, under a dialog too |
 | ? | This list |
 
 ## Tables
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index d1a19db..4e9b090 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -20,14 +20,19 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 | **New production connections open read-only** | On or off: where the **Open read-only** box of a new production connection starts. A connection you saved keeps its own box | On |
 | **Ask before closing with unsaved edits** | On or off. Off, closing a table, its connection or the window, or loading another page of it, discards its pending changes without asking | On |
 
-A second pane, **Appearance**, sets the theme and how the grids and the
-connection bar are drawn. Each change shows in every open window at once.
+A second pane, **Appearance**, sets the theme, the text, and how the grids
+and the connection bar are drawn. Each change shows in every open window
+at once.
 
 | Option | Choices | Default |
 | --- | --- | --- |
 | **Theme** (macOS and Windows) | System, Light or Dark. System follows the system's light or dark as it changes, Auto on macOS too | System |
 | **Theme** (Omarchy) | Follow Omarchy, or pin the theme it shows now, which then stays when Omarchy switches. The row says the theme drawn, and under it the palette it gave | Follow Omarchy |
-| **Row density** (Omarchy: row height) | Compact, Regular or Roomy. On macOS and Windows a step of 8 points either side of Regular; in the Omarchy look one, one and a half or two lines. The grid keeps the row at its top | Regular |
+| **Interface size** (macOS and Windows) | Small, Default or Large: the sidebar, tabs, buttons and the inspector's labels, from a body text of 12, 13 or 14 points | Default |
+| **Data font** (macOS and Windows) | The face of grid cells, the SQL editor and JSON on macOS, IBM Plex Mono or JetBrains Mono (Windows draws its data in Inter, and offers none), and its size, 11 to 16 points by halves | IBM Plex Mono, 12.5 pt |
+| **Font** (Omarchy) | The system's monospace, as fontconfig resolves it, or a font you name in the file by its fontconfig name. The row says the font drawn; one fontconfig does not have is drawn as the system's monospace, and the row says so | System monospace |
+| **Size** (Omarchy) | 9 to 24 points: everything in the window, which is drawn larger or smaller as a terminal's text is | 13 |
+| **Row density** (Omarchy: row height) | Compact, Regular or Roomy. On macOS and Windows a step of 8 points either side of Regular, and taller where a large data size needs it; in the Omarchy look one, one and a half or two lines. The grid keeps the row at its top | Regular |
 | **Alternate row shading** (Omarchy: alternate rows) | On or off, in the table grid and the SQL editor's results | Off on macOS and Windows, on in the Omarchy look |
 | **Show column types** (Omarchy: column types) | On or off: the second line of the grid's header | On |
 | **Table list** | Grouped by prefix (Omarchy: tree) or Flat. The default for a connection whose list you have not switched; the sidebar's switch (Omarchy: `t`) sets the connection's own | Grouped by prefix |
@@ -56,8 +61,8 @@ Settings are kept in `settings.toml`, which is safe to edit by hand.
 Tabletist watches the file and applies your edits while it runs. The file
 holds only what differs from the defaults. Until you change an option
 there is no file, and one the app writes starts with a header line. Here
-is every key at its default (`group_digits` has none of its own: it
-follows the look):
+is every key at its default (`group_digits`, `theme`, `font`, `font_size`
+and `zebra` have none of their own: they follow the look):
 
 ```toml
 [data]
@@ -82,6 +87,9 @@ enabled = false
 log_days = 30
 notify = true
 
+[appearance]
+interface_size = "default"
+
 [appearance.grid]
 row_height = "regular"
 column_types = true
@@ -103,6 +111,9 @@ bar = "tint"
 | `sidebar.show_system_schemas` | `true` shows schemas such as `pg_catalog` and `information_schema` | `false` |
 | `editor.sql_limit` | 1 to 10000 rows for each statement | `1000` |
 | `editor.sql_timeout_secs` | Seconds. `0` waits forever | `30` |
+| `appearance.interface_size` | `"small"`, `"default"` or `"large"`, on macOS and Windows | `"default"` |
+| `appearance.font` | On macOS the data's face, `"IBM Plex Mono"` or `"JetBrains Mono"`; `"monospace"` is IBM Plex Mono too, and any other name is drawn as IBM Plex Mono, which the window says. Windows draws its data in Inter, whatever it says. In the Omarchy look a fontconfig name (`"Fira Code"`); `"monospace"` is the system's. Written once you set it | Not set: IBM Plex Mono, or the system's monospace |
+| `appearance.font_size` | Points, whole or by halves (`13`, `12.5`). On macOS and Windows the data's, 11 to 16; in the Omarchy look all of it, 9 to 24. A size past a look's range is drawn at its nearest end. Written once you set it | Not set: 12.5 on macOS and Windows, 13 in the Omarchy look |
 | `appearance.theme` | On macOS and Windows `"system"`, `"light"` or `"dark"`. In the Omarchy look `"omarchy"`, or a theme to pin: an Omarchy theme's name (`"Tokyo Night"`), a palette file in the `themes` folder (`"Nord.json"`; its name alone where no Omarchy theme has it), or a path that starts with `/` or `~/` to a palette file or an Omarchy theme's folder. A value meant for the other looks is drawn as not set and stays in the file | Not set: the system's light or dark, or the Omarchy theme |
 | `appearance.grid.row_height` | `"compact"`, `"regular"` or `"roomy"` | `"regular"` |
 | `appearance.grid.zebra` | `true` or `false`. Written once you set it; without it, the look's default | Not set: off on macOS and Windows, on in the Omarchy look |
@@ -114,8 +125,11 @@ bar = "tint"
 | `mcp.notify` | `true` tells you when an agent connects: a notification on Linux, the Dock icon bouncing on macOS, the taskbar button flashing on Windows | `true` |
 
 `show_system_schemas` is set only in the file, and so is a pin by a
-name or a path other than the Omarchy theme of the moment. The SQL
-editor's **Limit** and **Timeout** menus set the two `editor` keys.
+name or a path other than the Omarchy theme of the moment, and a font by
+its fontconfig name. The SQL editor's **Limit** and **Timeout** menus set
+the two `editor` keys, and the text size keys (Cmd+= Cmd+- Cmd+0 on macOS, Ctrl+=
+Ctrl+- Ctrl+0 on Windows, `ctrl+=` `ctrl+-` in the Omarchy look) set
+`font_size`, wherever the keyboard is.
 
 A line that cannot be read is ignored and reported in the Settings window,
 and its key keeps its last good value: the one it had before the edit, or
````

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md docs/_reference/keyboard-shortcuts.md docs/_guide/omarchy.md docs/_guide/macos.md
git commit -m "Describe the text settings and the font size keys on the website"
```

---

## After the plan

Open one pull request for the step. Its description: a summary; how a role gets its size (the decision at the top, with the answer given); the departures table; and how it was tried by hand (Task 4 Step 7 on Omarchy, a Mac if one was at hand, or that it was not). No screenshot goes to GitHub unblurred.
