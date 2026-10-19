# Settings › Appearance, step 4: Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** End Settings › Appearance with the boards' preview, in both looks: a dev connection's bar named Bookshop over two rows of the Bookshop's `book_covers`, drawn as the options have them at each change (theme, text, row density, alternate rows, column types, the bar's tint) and asking the backend for nothing. The macOS and Windows sheet gets the board's line under the pane's title, "Theme, type and how dense the grid is. The preview updates as you change things.", which step 1 left out until there was a preview.

**Architecture:** One new file, `src/ui/settings/preview.rs`, draws the preview into the place its caller gives: the sheet's rows (`sheet.rs`) after Environment color, and the Omarchy screen's rows (`terminal.rs`) after environment. The bar is painted from the colours the connection bar takes (`env::env_colors` and `EnvColors::on_bar`, so `bar_colors` decides the tint) and its environment badge (`workspace::env_badge`). The rows are the tables' own grid (`grid::show`) given a look made from the settings (`Look::with_grid`), its cells made by the table view's `data_view::cell` from values held in the file, and drawn in a disabled `Ui` whose faded drawing is turned off: a picture, which takes no click, no Tab and no wheel. Nothing is read from a connection, so nothing is sent.

**Tech Stack:** Rust, egui (crmne fork: `Ui::disable`, `Visuals::disabled_alpha`, AccessKit node builders), the grid of `src/ui/grid.rs`, headless tests through `src/testing.rs`.

**Spec:** `docs/superpowers/specs/2026-10-17-settings-appearance-design.md`, section "Step 4: Preview". This plan is step 4, the last. Steps 1 (Grid and Environment), 2 (Theme) and 3 (Text) are merged.

---

## Decided before code: the grid as a picture

The preview draws the tables' grid itself, so that each option shows as a table will show it. That grid is made to be used: a Tab stop with the arrows, rows that take a click and light under the pointer, a scroll area for the wheel. This plan builds the following, **for your yes**:

- **The grid in a disabled `Ui`, drawn at full strength.** egui gives a disabled `Ui` no focus, no click, no hover and no wheel, and fades what it draws by `Visuals::disabled_alpha`, which the preview sets to 1 for its own `Ui`. `grid::show` is not changed. A screen reader is told of a group named "Preview" holding the grid's nodes, each disabled. The cost: with a table open under the sheet, the tree has two grids called "Rows" with rows called "Row 1" and "Row 2", the preview's disabled. The preview's tests find its rows inside its group; `testing::bounds` returns whichever comes first. The grid also marks its place for F6 (`focus::region`), which does nothing here: the app reads no workspace keys while a dialog is open.
- The other way, a flag on `grid::show` that leaves out its Tab stop and makes its rows hover-only, changes the signature every table and SQL result calls, for one caller. Say if you want it instead.
- The grid's widths are fitted again on every frame (its memory forgotten before each draw), so a change of the data's face or size, or of the header's lines, refits the columns at once. Two rows and four columns: nothing to measure that a frame would feel.

## Where this departs from the boards and the spec

For a yes or a no before code:

| The board or the spec asks | Step 4 builds | Why |
|---|---|---|
| "It never sends the backend anything (test: no Command is sent while the pane is shown and its options change)" | The preview sends nothing. Each change of an option still writes `settings.toml` (`Command::Save`), as every option does. The test: while the pane is shown and every option changes, the only commands sent are those saves, one a change and one for Reset to defaults, in each look, with a table open under the sheet | A change that wrote nothing would be lost at the next start. |
| macOS bar: a 3 pt stripe, then 30 pt with a 7 pt dot, "Bookshop" and the `dev` pill | The same, in the bar's own colours, and a 1 pt rule under it in the bar's line colour, as the real bar has | With Stripe only the bar is the panel's colour, as the header under it is: the rule parts them. |
| Omarchy bar: `DEV`, "Bookshop", "bookshop_development", a rule under | The same: the solid tag of the bar's own chip, the name bold, the database dim | |
| macOS grid: a 38 pt header, 36 pt rows (the Regular segment's "36") | The table grid's own heights: a 45 pt header (31 without types), 37 pt Regular rows with their rule (29 Compact, 45 Roomy; Windows its own) | The preview is the grid as a table draws it; step 1 kept Regular at today's 37. |
| `id` drawn as a plain column | The same: no key icon on macOS, no key colour or `pk` in the terminal, and `id` not held at the left | The boards draw no key; a key would also give `id` the 64 pt column and the terminal's `pk` in place of its type. Say if you want the key shown as a table shows it. |
| Omarchy grid: a 26 pt header and 26 pt rows | The terminal grid's: a 41 pt header (27 without types), 30 pt Regular rows (20 Compact, 40 Roomy) | As the macOS row above. |
| Column widths: 56, 200, 90 and the rest on macOS; 50, 220, 90 and the rest on Omarchy | Each column fitted to its name, its type and its values, as a table fits them; `created_at` takes the rest | The widths follow the data's face and size, as a table's do. |
| The bar, drawn as a 30 pt line | A drawing of its own (`paint_bar`) in the real bar's colours (`on_bar`, so `bar_colors` decides the tint) and with its badge (`env_badge`), not the connection bar itself (`top_bar`: 60 or 49 pt, with Connections, the chips and Disconnect). The macOS name in the bar's chip role, 13 pt semibold (the board's 12) | The boards draw a line, not the bar. A later change to the bar's layout does not reach the preview; a change to its colours or its badge does. |
| The `created_at` header: "timestamp" | "timestamp · no tz", as every grid writes a `timestamp` column's type | The header's line is the table view's (`data_view::type_line`). |
| Omarchy grid: the names alone, dim, on the dark fill; `id` to the right and the rest to the left; no gutter | The terminal grid's header: each name in the text colour over its type while Column types is on (the board's file has it on), the numbers to the right, the 22 pt gutter where the grid's cursor stands | The preview follows Column types, and is the grid a table draws. |
| (the ids under a grouping of digits) | `9100000000000000001` as it is, in every look | The ids name rows, and a table never groups its keys' digits. |
| (General's options) | Value tags followed: off, `print` and `ebook` are plain text. Timestamps and the zone change nothing: the values have no fraction and no offset | The kinds are drawn as a table draws them. |
| Omarchy tags: `print` tan, `ebook` teal | The terminal's slots 0 and 1 (the theme's orange and its cyan), as a table draws a closed set's first two values | The same reason. |
| (where the preview sits) | Under the last section, in the pane's rows: on macOS a window shorter than about 1000 pt scrolls the rows to it, as it scrolls to the last option today. On Omarchy it is no row of the cursor's: in a window too short for the pane, `j` stops at the connection bar and the wheel shows the preview | The board draws the sheet without its footer, at 1000 pt. At 1440 × 1000 all of it shows, and all of the Omarchy screen at 1920 × 1080. |
| (a narrow sheet) | The line under a pane's title wraps to the pane: General's too, which in a 720 pt window ran to 722 pt, past the window's edge, and pushed its controls past it | The Appearance line is longer than General's and broke the 720 pt test of step 3. |
| macOS: the line under the title | Shown, the board's words | Step 1 drew the title alone until there was a preview (the spec's table). |

A reviewer given the draft broke it with tests of its own, and the draft now fixes what it found.

It found no way to make the preview send anything, fall behind an option, take the keyboard or the pointer, or move the table under the sheet, in any look, with or without a connection, through Reset to defaults and a reload of the file. It found two things now fixed: the macOS and Windows grid's rule under its last row lay on the card's bottom border, a line two pixels thick (the grid now ends on the border, which is drawn over it); and on Windows the card stood on a fraction of a point, so the grid's rows, which stand on whole pixels, ran past it (the card is put on whole pixels; `the_preview_draws_two_bookshop_rows_under_a_dev_bar` holds it, its rows inside the card in the Windows look). The first is how it looks, compared by eye, not tested.

A second reviewer read this plan against its diffs, the spec and the boards. Its points are in: the table names every place the build is not the boards' drawing, the tests now open a table under the sheet, put every option back with Reset to defaults, click a row, check that the columns are fitted again and that the line under a title wraps, and the prose says which tests run in which look.

Left as they are, for the record:
- With a table open under the sheet, the accessibility tree has the preview's "Rows" and "Row 1", "Row 2" beside the table's (the decision above).
- In a sheet short enough to scroll, its scroll bar lies over the card's right border, as it lies over the ends of the rows' rules.
- In a narrow sheet at a large data size, the grid's "+2 ›" pill for the columns out of view shows over the preview, and does nothing there.

## Files

| File | What it does after this plan |
|---|---|
| `src/ui/settings/preview.rs` | New: the preview, both looks: the card, the bar, the grid as a picture. |
| `src/ui/settings/mod.rs` | Declares `preview`. |
| `src/ui/settings/sheet.rs` | The Appearance pane's line under its title, wrapped; the preview after Environment color; no rule under it. |
| `src/ui/settings/terminal.rs` | The preview after the environment rows. |
| `src/env.rs` | The preview among the views that may ask for an environment's colours (`only_the_environment_surfaces_ask_for_its_colours`). |
| `src/ui/preview_settings_tests.rs` | New: the preview's headless tests. |
| `src/ui/mod.rs` | Declares the tests. |
| `docs/_reference/settings-and-files.md` | The preview, in a sentence. |

Every task ends with the repository's checks passing:

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

(`cargo` is `~/.cargo/bin/cargo` in these sessions.) Commit after each task, signed as usual. If signing says "agent refused operation", 1Password is locked: leave the change staged and say so.

---

### Task 1: The preview on the macOS and Windows sheet

**Files:**
- Create: `src/ui/settings/preview.rs`, `src/ui/preview_settings_tests.rs`
- Modify: `src/ui/settings/mod.rs`, `src/ui/settings/sheet.rs`, `src/env.rs`, `src/ui/mod.rs`

- [ ] **Step 1: Write the failing tests**

The sheet says what the preview does under its title, and the line under each pane's title wraps in the smallest sheet (720 × 480), General's too. The preview shows the two rows under a dev bar, inside the group a screen reader is told of; it follows each grid option and the bar's option; it follows the theme, the data's size (its columns fitted again) and the interface's size, and value tags; it sends nothing but the file's saves while every option changes and Reset to defaults puts them back, with `book_covers` open in a table under the sheet; and it takes no Tab, no hover, no click and no wheel. Windows draws the same sheet with its own grid, so the four tests that loop over `looks()` run in its look and the macOS one; the others run in the macOS look. Task 2 adds the terminal look to `looks()`.

Not tested, for the record: the data's face (IBM Plex Mono and JetBrains Mono have the same widths, and the harness records no face; step 3's tests cover the look the preview is drawn with), the theme on Windows and Omarchy, and the terminal's font size (the window's zoom, step 3's). That the Appearance pane draws no rule under the preview, and that the card's border lies over the last row's rule, are what it looks like: compared by eye in Step 5, not tested.

```diff
diff --git a/src/ui/mod.rs b/src/ui/mod.rs
index b5f51a0..f9e12bb 100644
--- a/src/ui/mod.rs
+++ b/src/ui/mod.rs
@@ -51,6 +51,8 @@ pub mod object_tabs;
 pub mod password_prompt;
 pub mod pending_bar;
 pub mod picker;
+#[cfg(test)]
+mod preview_settings_tests;
 pub mod quick_open;
 pub mod review;
 pub mod row_form;
diff --git a/src/ui/preview_settings_tests.rs b/src/ui/preview_settings_tests.rs
new file mode 100644
index 0000000..9246ebe
--- /dev/null
+++ b/src/ui/preview_settings_tests.rs
@@ -0,0 +1,415 @@
+//! Settings › Appearance, its preview: two Bookshop rows under a dev
+//! connection's bar, drawn as the options have them as they change, and
+//! nothing asked of the backend for them.
+
+use egui::accesskit::Role;
+
+use super::appearance_settings_tests::{on_the_pane, shows};
+use crate::backend::{Command, StateFile};
+use crate::env::{Environment, Platform};
+use crate::model::{Action, SettingsPane};
+use crate::settings::{
+    EnvBar, FontSize, InterfaceSize, OptionValue, RowHeight, TableList, ThemeChoice,
+};
+use crate::testing::{Harness, bounds};
+use crate::theme::Look;
+
+/// The looks that draw the sheet: Windows draws the macOS one, with its
+/// own grid.
+fn looks() -> [Look; 2] {
+    [Look::standard(), Look::macos()]
+}
+
+/// Settings on its Appearance pane, in a window as large as the look's
+/// board, with the sheet's rows scrolled to their end: the preview.
+fn shown(look: Look) -> Harness {
+    shown_over(look, false)
+}
+
+/// [`shown`], over the Bookshop's `book_covers` open in a table where
+/// `table` says.
+fn shown_over(look: Look, table: bool) -> Harness {
+    let size = if look.terminal {
+        egui::vec2(1920.0, 1080.0)
+    } else {
+        egui::vec2(1440.0, 1000.0)
+    };
+    let mut harness = Harness::with_size(size);
+    harness.set_look(look);
+    if table {
+        harness.book_covers();
+    }
+    on_the_pane(&mut harness);
+    // The board's window shows all of the pane; the rows scroll where a
+    // larger text or taller rows make them taller than the sheet leaves.
+    if !look.terminal {
+        wheel(&mut harness, -2000.0);
+    }
+    harness
+}
+
+/// The wheel turned by `delta` over the middle of the window.
+fn wheel(harness: &mut Harness, delta: f32) {
+    let middle = (harness.size / 2.0).to_pos2();
+    wheel_at(harness, middle, delta);
+}
+
+/// The wheel turned by `delta` with the pointer at `at`.
+fn wheel_at(harness: &mut Harness, at: egui::Pos2, delta: f32) {
+    harness.frame(vec![
+        egui::Event::PointerMoved(at),
+        egui::Event::MouseWheel {
+            unit: egui::MouseWheelUnit::Point,
+            delta: egui::vec2(0.0, delta),
+            modifiers: egui::Modifiers::NONE,
+            phase: egui::TouchPhase::Move,
+        },
+    ]);
+    // The wheel scrolls smoothly: until it has stopped.
+    for _ in 0..5 {
+        harness.finish_animations();
+    }
+    // A scroll asked for lands two frames later.
+    harness.settle();
+    harness.settle();
+}
+
+/// An option set, as the pane's controls set it, and the app's logic run
+/// after it, as the window runs it between frames: a theme is put in
+/// place there.
+fn set(harness: &mut Harness, value: OptionValue) {
+    harness.app.apply(Action::SetOption(value));
+    let mut input = egui::RawInput::default();
+    input.viewports.entry(egui::ViewportId::ROOT).or_default();
+    let app = &mut harness.app;
+    let _ = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
+    harness.settle();
+    harness.settle();
+}
+
+/// Whether the preview shows `text`, the whole of a piece of text.
+fn in_preview(harness: &mut Harness, text: &str) -> bool {
+    let preview = preview(harness);
+    harness
+        .text_rects
+        .iter()
+        .any(|(painted, at)| painted == text && preview.contains_rect(*at))
+}
+
+/// Where the preview is: the group a screen reader is told of.
+fn preview(harness: &mut Harness) -> egui::Rect {
+    let tree = harness.settle();
+    let name = harness.app.look.label("Preview");
+    bounds(&tree, &name, Role::Group).expect("the preview")
+}
+
+/// The preview's two rows, as drawn.
+fn rows(harness: &mut Harness) -> [egui::Rect; 2] {
+    let preview = preview(harness);
+    let tree = harness.settle();
+    // In the preview: a table open under the sheet has rows of these names.
+    let row = |number: u64| {
+        let name = format!("Row {number}");
+        tree.nodes
+            .iter()
+            .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(&name))
+            .filter_map(|(_, node)| node.bounds())
+            .map(|rect| {
+                egui::Rect::from_min_max(
+                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
+                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
+                )
+            })
+            .find(|rect| preview.contains_rect(*rect))
+            .expect("a row in the preview")
+    };
+    [row(1), row(2)]
+}
+
+/// Whether the preview has a fill of `color` over all of `rect`.
+fn filled(harness: &mut Harness, rect: egui::Rect, color: egui::Color32) -> bool {
+    let preview = preview(harness);
+    harness.fills.iter().any(|(fill, painted)| {
+        *painted == color && preview.contains_rect(*fill) && fill.contains_rect(rect)
+    })
+}
+
+#[test]
+fn the_sheet_says_what_the_preview_does_under_its_title() {
+    let mut harness = shown(Look::macos());
+    let words = "Theme, type and how dense the grid is. The preview updates as you change things.";
+    assert!(shows(&mut harness, words));
+}
+
+#[test]
+fn the_preview_draws_two_bookshop_rows_under_a_dev_bar() {
+    for look in looks() {
+        let mut harness = shown(look);
+        let preview = preview(&mut harness);
+        for words in [
+            "Bookshop",
+            Environment::Dev.label(Platform::of(&look)),
+            "publisher_id",
+            "created_at",
+            "9100000000000000001",
+            "9100000000000000002",
+            "print",
+            "ebook",
+            "2026-01-12 09:14:03",
+            "2026-01-12 17:02:41",
+        ] {
+            assert!(shows(&mut harness, words), "{} {words}", look.name);
+            let at = harness.painted_rect(words).expect("painted");
+            assert!(preview.contains_rect(at), "{} {words}", look.name);
+        }
+        // The terminal's bar names the database too, as its chip does.
+        assert_eq!(
+            shows(&mut harness, "bookshop_development"),
+            look.terminal,
+            "{}",
+            look.name
+        );
+        for row in rows(&mut harness) {
+            assert!(preview.contains_rect(row), "{}", look.name);
+        }
+        // The kinds are value tags, each in its slot's colour.
+        for (kind, slot) in [("print", 0), ("ebook", 1)] {
+            let (ink, _) =
+                crate::ui::value_tags::slot_colors(slot, &harness.app.look, &harness.app.palette);
+            assert_eq!(
+                harness.painted_color(kind),
+                Some(ink),
+                "{} {kind}",
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn the_preview_s_rows_follow_the_grid_options() {
+    for look in looks() {
+        let mut harness = shown(look);
+        // Each density's rows, as the table grid has them.
+        let mut heights = Vec::new();
+        for height in [RowHeight::Compact, RowHeight::Regular, RowHeight::Roomy] {
+            set(&mut harness, OptionValue::RowHeight(height));
+            if !look.terminal {
+                wheel(&mut harness, -2000.0);
+            }
+            let expected = harness.app.look.with_grid(&harness.app.settings).row();
+            for row in rows(&mut harness) {
+                assert_eq!(row.height(), expected, "{} {height:?}", look.name);
+            }
+            heights.push(expected);
+        }
+        assert!(
+            heights.is_sorted() && heights[0] < heights[2],
+            "{heights:?}"
+        );
+        // The second row is striped where the rows are.
+        let panel = harness.app.palette.panel;
+        for zebra in [true, false] {
+            set(&mut harness, OptionValue::Zebra(Some(zebra)));
+            let [first, second] = rows(&mut harness);
+            assert_eq!(filled(&mut harness, second, panel), zebra, "{}", look.name);
+            assert!(!filled(&mut harness, first, panel), "{}", look.name);
+        }
+        // Without the types the header is one line, and says no type.
+        let header = |harness: &mut Harness| rows(harness)[0].top() - preview(harness).top();
+        let two = header(&mut harness);
+        assert!(in_preview(&mut harness, "varchar"), "{}", look.name);
+        set(&mut harness, OptionValue::ColumnTypes(false));
+        let one = header(&mut harness);
+        assert!(one < two, "{} {one} {two}", look.name);
+        assert!(!in_preview(&mut harness, "varchar"), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_preview_s_bar_follows_the_environment_option() {
+    for look in looks() {
+        let mut harness = shown(look);
+        let platform = Platform::of(&look);
+        let palette = harness.app.palette;
+        let (tint, _) = crate::env::bar_colors(Environment::Dev, EnvBar::Tint, platform, &palette);
+        let bar = |harness: &mut Harness| {
+            let preview = preview(harness);
+            let name = harness.painted_rect("Bookshop").expect("the name");
+            egui::Rect::from_x_y_ranges(preview.shrink(1.0).x_range(), name.y_range())
+        };
+        let at = bar(&mut harness);
+        assert!(filled(&mut harness, at, tint), "{}", look.name);
+        set(&mut harness, OptionValue::EnvBar(EnvBar::Label));
+        let at = bar(&mut harness);
+        assert!(!filled(&mut harness, at, tint), "{}", look.name);
+        let panel = harness.app.palette.panel;
+        assert!(filled(&mut harness, at, panel), "{}", look.name);
+        // The environment is still said.
+        let label = Environment::Dev.label(platform);
+        assert!(in_preview(&mut harness, label), "{}", look.name);
+    }
+}
+
+#[test]
+fn the_preview_follows_the_theme_and_the_text() {
+    let mut harness = shown(Look::macos());
+    set(&mut harness, OptionValue::Theme(ThemeChoice::Light));
+    let light = harness.painted_color("Bookshop");
+    assert_eq!(light, Some(harness.app.palette.text));
+    set(&mut harness, OptionValue::Theme(ThemeChoice::Dark));
+    assert!(harness.app.palette.dark);
+    let dark = harness.painted_color("Bookshop");
+    assert_eq!(dark, Some(harness.app.palette.text));
+    assert_ne!(light, dark);
+    // The data's size widens a cell's text, the interface's the header.
+    let wide = |harness: &mut Harness, text: &str| {
+        harness.settle();
+        harness.painted_rect(text).expect("painted").width()
+    };
+    let (cell, name) = (
+        wide(&mut harness, "9100000000000000001"),
+        wide(&mut harness, "publisher_id"),
+    );
+    let left = |harness: &mut Harness| {
+        harness.settle();
+        let text = harness.painted_rect("2026-01-12 09:14:03");
+        text.expect("painted").left()
+    };
+    let after = left(&mut harness);
+    set(&mut harness, OptionValue::FontSize(FontSize::of(16.0)));
+    assert!(wide(&mut harness, "9100000000000000001") > cell);
+    assert_eq!(wide(&mut harness, "publisher_id"), name);
+    // The columns are fitted again: the ones after the ids move right.
+    assert!(left(&mut harness) > after);
+    set(
+        &mut harness,
+        OptionValue::InterfaceSize(InterfaceSize::Large),
+    );
+    assert!(wide(&mut harness, "publisher_id") > name);
+    // Value tags off: the kinds are plain text.
+    set(&mut harness, OptionValue::ValueTags(false));
+    assert_eq!(
+        harness.painted_color("print"),
+        Some(harness.app.palette.text)
+    );
+}
+
+#[test]
+fn the_preview_sends_the_backend_nothing() {
+    for look in looks() {
+        // A table open under the sheet, which a frame could ask rows for.
+        let mut harness = shown_over(look, true);
+        let before = harness.app.backend.sent.len();
+        let mut values = vec![
+            OptionValue::RowHeight(RowHeight::Roomy),
+            OptionValue::Zebra(Some(!look.terminal)),
+            OptionValue::ColumnTypes(false),
+            OptionValue::TableList(TableList::Flat),
+            OptionValue::EnvBar(EnvBar::Label),
+            OptionValue::ValueTags(false),
+            OptionValue::FontSize(FontSize::of(15.0)),
+        ];
+        if !look.terminal {
+            values.push(OptionValue::Theme(ThemeChoice::Dark));
+            values.push(OptionValue::InterfaceSize(InterfaceSize::Small));
+            values.push(OptionValue::Font(Some("JetBrains Mono".into())));
+        }
+        let changes = values.len();
+        for value in values {
+            set(&mut harness, value);
+            wheel(&mut harness, -2000.0);
+        }
+        // And every option put back at once.
+        harness.app.apply(Action::ResetSettings);
+        harness.app.apply(Action::ConfirmResetSettings(true));
+        wheel(&mut harness, -2000.0);
+        assert!(harness.painted_rect("Bookshop").is_some(), "{}", look.name);
+        rows(&mut harness);
+        // Each change is written to the file, and that is all.
+        let sent = &harness.app.backend.sent[before..];
+        assert_eq!(sent.len(), changes + 1, "{} {sent:?}", look.name);
+        for command in sent {
+            assert!(
+                matches!(
+                    command,
+                    Command::Save {
+                        file: StateFile::Settings(_),
+                        ..
+                    }
+                ),
+                "{} {command:?}",
+                look.name
+            );
+        }
+    }
+}
+
+#[test]
+fn the_preview_takes_no_tab_pointer_or_wheel() {
+    let mut harness = shown(Look::macos());
+    // Tab goes round the sheet's controls and never into the preview.
+    for _ in 0..40 {
+        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
+        let preview = preview(&mut harness);
+        let focused = harness
+            .ctx
+            .memory(|memory| memory.focused())
+            .and_then(|id| harness.ctx.read_response(id));
+        if let Some(focused) = focused {
+            assert!(!preview.intersects(focused.rect), "{:?}", focused.rect);
+        }
+    }
+    // A row under the pointer is not lit, as a table's is, and a click on
+    // it takes the keyboard nowhere and asks for nothing.
+    wheel(&mut harness, -2000.0);
+    let [row, _] = rows(&mut harness);
+    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
+    let palette = harness.app.palette;
+    let hover = palette.panel.lerp_to_gamma(palette.surface, 0.5);
+    assert!(!filled(&mut harness, row, hover));
+    let sent = harness.app.backend.sent.len();
+    let press = |pressed| egui::Event::PointerButton {
+        pos: row.center(),
+        button: egui::PointerButton::Primary,
+        pressed,
+        modifiers: egui::Modifiers::NONE,
+    };
+    harness.frame(vec![press(true)]);
+    harness.frame(vec![press(false)]);
+    harness.settle();
+    assert_eq!(harness.ctx.memory(|memory| memory.focused()), None);
+    assert_eq!(harness.app.backend.sent.len(), sent);
+    assert!(harness.app.dialog.is_some(), "the sheet stays");
+    // In a window too short for all of the pane, the wheel over the
+    // preview scrolls the sheet: the preview moves down.
+    let mut harness = Harness::new();
+    harness.set_look(Look::macos());
+    on_the_pane(&mut harness);
+    wheel(&mut harness, -2000.0);
+    let before = preview(&mut harness);
+    wheel_at(&mut harness, before.center(), 300.0);
+    assert!(preview(&mut harness).top() > before.top());
+}
+
+#[test]
+fn the_line_under_a_pane_s_title_wraps_in_the_smallest_sheet() {
+    // The sheet ends 40 in from the window's edge.
+    for (pane, words) in [
+        (SettingsPane::General, "How Tabletist shows data"),
+        (SettingsPane::Appearance, "Theme, type and how dense"),
+    ] {
+        let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
+        harness.set_look(Look::macos());
+        harness.app.apply(Action::ShowSettings);
+        harness.app.apply(Action::ShowSettingsPane(pane));
+        harness.settle();
+        let line = harness
+            .text_rects
+            .iter()
+            .find(|(text, _)| text.starts_with(words))
+            .map(|(_, at)| *at)
+            .expect("the line");
+        assert!(line.right() <= 680.0, "{pane:?} {line:?}");
+    }
+}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- ui::preview_settings_tests`
Expected: FAIL. All eight fail: Settings has no preview yet ("the preview" is not found, and nothing paints "Bookshop"), the sheet says nothing under the Appearance pane's title, and General's line runs to 722 pt in the 720 pt window.

- [ ] **Step 3: Draw the preview, and say what it does**

`preview::show` allocates the card across the rows, `side` in from each edge and on whole pixels: the bar, then the grid's header and two rows at the heights the settings give, the last ending on the card's bottom border, which is drawn over its rule. The bar is painted in `on_bar`'s colours, with the macOS stripe and pill or the terminal's tag and database. The grid is the tables' `grid::show` in a disabled `Ui` drawn at full strength, its widths forgotten before each frame; its cells come from `data_view::cell`, `kind` tagged when value tags are on. The card's corners are rounded over what it holds by a stroke of the window's colour outside them, clipped to the card so that it paints nothing round it, then its border.

Beside the sections, the sheet tells `content` whether the pane ends with the preview: the Appearance pane does, under its own section label, 14 under its last row, and draws no rule of its own under the rows. The line under each pane's title wraps to the pane. `src/env.rs`'s test of who may ask for an environment's colours lists the preview, which shows one.

```diff
diff --git a/src/env.rs b/src/env.rs
index b608ac9..e03041a 100644
--- a/src/env.rs
+++ b/src/env.rs
@@ -890,6 +890,7 @@ mod tests {
             "src/ui/picker.rs",
             "src/ui/connect_dialog/mod.rs",
             "src/ui/env_tests.rs",
+            "src/ui/settings/preview.rs",
         ];
         let mut found = Vec::new();
         for (path, text) in sources() {
diff --git a/src/ui/settings/mod.rs b/src/ui/settings/mod.rs
index 334c0e6..c370d25 100644
--- a/src/ui/settings/mod.rs
+++ b/src/ui/settings/mod.rs
@@ -7,6 +7,7 @@
 
 mod agents;
 mod file_pane;
+mod preview;
 mod sheet;
 mod terminal;
 
diff --git a/src/ui/settings/preview.rs b/src/ui/settings/preview.rs
new file mode 100644
index 0000000..10e75d0
--- /dev/null
+++ b/src/ui/settings/preview.rs
@@ -0,0 +1,243 @@
+//! The Appearance pane's preview: two rows of the Bookshop's `book_covers`
+//! under the bar of a dev connection named Bookshop, drawn by the grid the
+//! tables are drawn with and in the bar's colours, as the settings have
+//! them now. The rows are made here: nothing is asked of a database, and
+//! nothing is sent to the backend for them.
+
+use egui::emath::GuiRounding as _;
+use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
+use tabletist_db::{ColumnMeta, Value, ValueKind};
+
+use crate::env::{Environment, Platform};
+use crate::settings::Settings;
+use crate::theme::{Look, Palette};
+use crate::typography::{Text, TextRole};
+use crate::ui::data_view::{self, Shown};
+use crate::ui::grid::{self, Column};
+use crate::ui::value_tags::Tags;
+use crate::ui::workspace::{Badge, env_badge};
+
+/// The connection the bar names, its environment and its database.
+const NAME: &str = "Bookshop";
+const ENV: Environment = Environment::Dev;
+const DATABASE: &str = "bookshop_development";
+/// The kinds `book_covers` allows, in the database's order: each value
+/// takes its tag's slot by its place.
+const KINDS: [&str; 3] = ["print", "ebook", "audio"];
+/// The bar's line, and the macOS stripe over it.
+const BAR: f32 = 30.0;
+const STRIPE: f32 = 3.0;
+
+/// The preview's columns, as a server would describe them.
+fn columns() -> [ColumnMeta; 4] {
+    let meta = |name: &str, type_name: &str, kind| ColumnMeta {
+        name: name.into(),
+        type_name: type_name.into(),
+        kind,
+    };
+    [
+        meta("id", "int8", ValueKind::Numeric),
+        meta("publisher_id", "int8", ValueKind::Numeric),
+        meta("kind", "varchar", ValueKind::Text),
+        meta("created_at", "timestamp", ValueKind::Temporal),
+    ]
+}
+
+/// The preview's two rows.
+fn rows() -> [[Value; 4]; 2] {
+    [
+        [
+            Value::Int(2),
+            Value::Int(9_100_000_000_000_000_001),
+            Value::Text("print".into()),
+            Value::Text("2026-01-12 09:14:03".into()),
+        ],
+        [
+            Value::Int(3),
+            Value::Int(9_100_000_000_000_000_002),
+            Value::Text("ebook".into()),
+            Value::Text("2026-01-12 17:02:41".into()),
+        ],
+    ]
+}
+
+/// The preview across `ui`'s width, `side` in from each of its edges, in a
+/// group named `name`: the bar, then the grid's header and its two rows.
+pub(super) fn show(
+    ui: &mut Ui,
+    (name, side): (&str, f32),
+    settings: &Settings,
+    look: &Look,
+    palette: &Palette,
+) {
+    // The grid's rows as dense and as striped, and its header as tall, as
+    // the settings have them.
+    let grid_look = look.with_grid(settings);
+    let bar = if look.terminal {
+        BAR + 1.0
+    } else {
+        STRIPE + BAR + 1.0
+    };
+    let rows_height = grid::header_height(&grid_look) + 2.0 * grid_look.row();
+    // The last row's rule, which the macOS grid draws under each row, is
+    // the card's bottom border: the border is drawn over it, not under.
+    let height = 1.0 + bar + rows_height;
+    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
+    // On whole pixels, as the grid puts its rows: its last row ends on the
+    // card's bottom.
+    let card = place
+        .shrink2(vec2(side, 0.0))
+        .round_to_pixels(ui.pixels_per_point());
+    if !card.is_positive() {
+        return;
+    }
+    let inside = card.shrink(1.0);
+    let mut group = ui.new_child(egui::UiBuilder::new().id_salt("preview").max_rect(card));
+    // A group with the section's name, in the card's place: what is in it
+    // is told to a screen reader as a picture's parts, disabled.
+    group
+        .ctx()
+        .accesskit_node_builder(group.unique_id(), |node| {
+            node.set_role(egui::accesskit::Role::Group);
+            node.set_label(name);
+            node.set_bounds(egui::accesskit::Rect {
+                x0: card.min.x.into(),
+                y0: card.min.y.into(),
+                x1: card.max.x.into(),
+                y1: card.max.y.into(),
+            });
+        });
+    let top = Rect::from_min_size(inside.min, vec2(inside.width(), bar));
+    paint_bar(&group, top, settings, look, palette);
+    let below = Rect::from_min_max(
+        pos2(inside.left(), top.bottom()),
+        pos2(inside.right(), card.bottom()),
+    );
+    paint_grid(&mut group, below, settings, &grid_look, palette);
+    // The card's rounded corners over the square ones of what it holds:
+    // the window's colour outside them, held to the card so that it paints
+    // only its corners, then its border.
+    let radius = if look.terminal { 3 } else { 10 };
+    let mask = (f32::from(radius) * (std::f32::consts::SQRT_2 - 1.0)).ceil() + 1.0;
+    group
+        .painter()
+        .with_clip_rect(card.intersect(group.clip_rect()))
+        .rect_stroke(
+            card,
+            CornerRadius::same(radius),
+            Stroke::new(mask, palette.window),
+            StrokeKind::Outside,
+        );
+    let painter = group.painter();
+    let border = if look.terminal {
+        palette.outline
+    } else {
+        palette.border
+    };
+    painter.rect_stroke(
+        card,
+        CornerRadius::same(radius),
+        Stroke::new(1.0, border),
+        StrokeKind::Inside,
+    );
+}
+
+/// The connection bar in `rect`, its rule at the bottom: its fill as the
+/// settings have it, the macOS stripe over it, and the connection named
+/// with its environment, as the bar's own chip names it.
+fn paint_bar(ui: &Ui, rect: Rect, settings: &Settings, look: &Look, palette: &Palette) {
+    let platform = Platform::of(look);
+    let env = crate::env::env_colors(ENV, platform, palette).on_bar(
+        ENV,
+        settings.env_bar,
+        platform,
+        palette,
+    );
+    let painter = ui.painter();
+    painter.rect_filled(rect, CornerRadius::ZERO, env.bar_bg());
+    crate::ui::widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, env.bar_border());
+    if look.terminal {
+        // The environment's tag, the name, and the database dim after it.
+        let center = rect.top() + BAR / 2.0;
+        let mut x = rect.left() + 10.0;
+        x += env_badge(ui, x, center, ENV, &env, Badge::Tracked, look) + 8.0;
+        let name = Text::one(look, TextRole::OGroup, NAME, palette.text).layout(ui.ctx());
+        name.paint_left(painter, x, center);
+        x += name.width() + 8.0;
+        Text::one(look, TextRole::OBody, DATABASE, palette.dim)
+            .layout(ui.ctx())
+            .paint_left(painter, x, center);
+    } else {
+        // The stripe, then the bar's dot, the name and the environment's
+        // pill.
+        let stripe = Rect::from_min_size(rect.min, vec2(rect.width(), STRIPE));
+        painter.rect_filled(stripe, CornerRadius::ZERO, env.base());
+        let center = rect.top() + STRIPE + BAR / 2.0;
+        let mut x = rect.left() + 12.0;
+        painter.circle_filled(pos2(x + 3.5, center), 3.5, env.base());
+        x += 7.0 + 8.0;
+        let role = TextRole::UiBodySemibold;
+        let name = Text::one(look, role, NAME, palette.text).layout(ui.ctx());
+        name.paint_left(painter, x, center);
+        x += name.width() + 8.0;
+        env_badge(ui, x, center, ENV, &env, Badge::Chip, look);
+    }
+}
+
+/// The grid in `rect`: the header and the two rows, drawn by the tables'
+/// grid as a picture of it. It takes no click, no Tab and no wheel, and
+/// fits its columns again each frame, to the faces and the header the
+/// settings have now.
+fn paint_grid(ui: &mut Ui, rect: Rect, settings: &Settings, look: &Look, palette: &Palette) {
+    let mut still = ui.new_child(egui::UiBuilder::new().id_salt("grid").max_rect(rect));
+    // Disabled at full strength: egui draws a disabled Ui faded.
+    still.visuals_mut().disabled_alpha = 1.0;
+    still.disable();
+    let id = still.id().with("rows");
+    grid::forget(still.ctx(), id);
+    let metas = columns();
+    let rows = rows();
+    let kinds = KINDS.map(String::from);
+    let tags = |col: usize| match col {
+        2 => Tags::Values(&kinds).when(settings.value_tags),
+        _ => Tags::None,
+    };
+    let columns: Vec<Column> = metas
+        .iter()
+        .map(|meta| {
+            let (type_line, _) =
+                data_view::type_line(&meta.name, &meta.type_name, meta.kind, None, look);
+            Column {
+                name: &meta.name,
+                type_line,
+                numeric: meta.kind == ValueKind::Numeric,
+                sort: None,
+                key: false,
+                // The last takes what is left, as the boards' grids have it.
+                flexible: meta.name == "created_at",
+                sortable: false,
+                required: false,
+            }
+        })
+        .collect();
+    let ctx = still.ctx().clone();
+    // The ids name rows: not in threes, as a table's keys are not.
+    let shown = Shown::default();
+    grid::show(
+        &mut still,
+        id,
+        &columns,
+        rows.len(),
+        None,
+        false,
+        palette,
+        look,
+        &|row| grid::Row {
+            number: Some(row as u64 + 1),
+            ..grid::Row::default()
+        },
+        None,
+        None,
+        |row, col| data_view::cell(&ctx, &rows[row][col], &metas[col], &tags(col), look, shown),
+    );
+}
diff --git a/src/ui/settings/sheet.rs b/src/ui/settings/sheet.rs
index e5d4bee..e03ee32 100644
--- a/src/ui/settings/sheet.rs
+++ b/src/ui/settings/sheet.rs
@@ -165,23 +165,27 @@ pub(super) fn show(
                 SettingsPane::General => content(
                     &mut pane,
                     ("General", Some(GENERAL)),
-                    &[("Data", &OptionId::DATA), ("Safety", &OptionId::SAFETY)],
+                    (
+                        &[("Data", &OptionId::DATA), ("Safety", &OptionId::SAFETY)],
+                        false,
+                    ),
                     &app.settings,
                     limit,
                     &skin,
                     actions,
                 ),
-                // The board's line under the title names the preview, which
-                // the pane does not have yet: the title stands alone.
                 SettingsPane::Appearance => content(
                     &mut pane,
-                    ("Appearance", None),
-                    &[
-                        ("Theme", &OptionId::THEME),
-                        ("Text", &OptionId::TEXT),
-                        ("Grid", &OptionId::GRID),
-                        ("Environment color", &OptionId::ENVIRONMENT),
-                    ],
+                    ("Appearance", Some(APPEARANCE)),
+                    (
+                        &[
+                            ("Theme", &OptionId::THEME),
+                            ("Text", &OptionId::TEXT),
+                            ("Grid", &OptionId::GRID),
+                            ("Environment color", &OptionId::ENVIRONMENT),
+                        ],
+                        true,
+                    ),
                     &super::shown(app),
                     limit,
                     &skin,
@@ -196,8 +200,9 @@ pub(super) fn show(
             // `ROWS_MIN` of them leaves the sheet shorter than that.
             let bottom = limit.max(rows) + foot;
             // The last row leaves its rule to the footer's: where there is
-            // room between them, it has one of its own.
-            if rows < limit - 0.5 {
+            // room between them, it has one of its own. The preview, which
+            // ends the Appearance pane, has its border.
+            if rows < limit - 0.5 && dialog.pane != SettingsPane::Appearance {
                 let across = egui::Rangef::new(room.left() + NAV + SIDE, room.right() - SIDE);
                 widgets::hline(ui, across, rows - 0.5, skin.palette.surface);
             }
@@ -299,15 +304,19 @@ fn nav(
 
 /// What the General pane says under its title.
 const GENERAL: &str = "How Tabletist shows data and protects your work. Changes apply right away.";
+/// What the Appearance pane says under its title.
+const APPEARANCE: &str =
+    "Theme, type and how dense the grid is. The preview updates as you change things.";
 
 /// The tab: its title, what it is for where it says, and its options under
-/// the heading of each group. The rows end at `limit` at the latest, and
-/// scroll when they are taller than that leaves them.
+/// the heading of each group, then the preview where it has one. The rows
+/// end at `limit` at the latest, and scroll when they are taller than that
+/// leaves them.
 #[allow(clippy::too_many_arguments)] // one pane: what it says, what it shows, what it pushes
 fn content(
     ui: &mut Ui,
     (title, about): (&'static str, Option<&'static str>),
-    sections: &[(&'static str, &[OptionId])],
+    (sections, preview): (&[(&'static str, &[OptionId])], bool),
     settings: &Settings,
     limit: f32,
     skin: &Skin,
@@ -323,7 +332,12 @@ fn content(
     );
     if let Some(about) = about {
         ui.add_space(2.0);
-        widgets::label(ui, TextRole::UiBody, &skin.say(about), palette.dim, look);
+        // Wrapped to the pane: a narrow sheet would otherwise grow as wide
+        // as the line, and its rows with it.
+        Text::one(look, TextRole::UiBody, &skin.say(about), palette.dim)
+            .wrap(ui.available_width())
+            .layout(ui.ctx())
+            .label(ui);
     }
     ui.add_space(6.0 + 14.0);
     let Some((first, _)) = sections.first() else {
@@ -376,6 +390,18 @@ fn content(
                     index += 1;
                 }
             }
+            // Under the options, 14 under the last of them, the preview,
+            // and 14 under it.
+            if preview {
+                let name = skin.say("Preview");
+                ui.add_space(14.0);
+                widgets::section_label(&name, look, palette)
+                    .layout(ui.ctx())
+                    .label(ui);
+                ui.add_space(2.0);
+                super::preview::show(ui, (&name, 0.0), settings, look, palette);
+                ui.add_space(14.0);
+            }
         });
 }
 
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::preview_settings_tests ui::appearance_settings_tests ui::text_settings_tests env::tests`
Expected: PASS (`the_data_font_row_fits_the_smallest_sheet` with them: the wrapped line keeps the 720 pt sheet inside the window). Then the four checks above.

- [ ] **Step 5: Compare with the board by eye**

Not a test, and nothing of it is committed. Append a throwaway `#[test] #[ignore]` scene to `src/shots.rs` that opens Settings › Appearance in `Harness::for_shots(egui::vec2(1440.0, 1000.0), 1.0, light, Look::macos())`, light and dark, scrolls the rows to their end with the wheel, and shoots it; then sets Stripe only, alternate rows, Compact and no column types and shoots again. Run it with `cargo test --locked --features shots --lib shots::<name> -- --ignored`, read the PNGs in `target/shots/`, and set them beside the "macOS – Settings, Appearance" board's Preview. Remove the scene before committing. Keep the PNGs local.

- [ ] **Step 6: Commit**

```bash
git add src/ui/settings/preview.rs src/ui/settings/mod.rs src/ui/settings/sheet.rs src/env.rs src/ui/preview_settings_tests.rs src/ui/mod.rs
git commit -m "Preview the grid and the bar at the end of Settings › Appearance on macOS and Windows"
```

---

### Task 2: The preview on the Omarchy screen

**Files:**
- Modify: `src/ui/settings/terminal.rs`, `src/ui/preview_settings_tests.rs`

- [ ] **Step 1: Write the failing tests**

The four tests that loop over the looks run in the terminal look too.

```diff
diff --git a/src/ui/preview_settings_tests.rs b/src/ui/preview_settings_tests.rs
index 9246ebe..c71e21d 100644
--- a/src/ui/preview_settings_tests.rs
+++ b/src/ui/preview_settings_tests.rs
@@ -14,10 +14,9 @@ use crate::settings::{
 use crate::testing::{Harness, bounds};
 use crate::theme::Look;
 
-/// The looks that draw the sheet: Windows draws the macOS one, with its
-/// own grid.
-fn looks() -> [Look; 2] {
-    [Look::standard(), Look::macos()]
+/// Every look: Windows draws the macOS sheet, with its own grid.
+fn looks() -> [Look; 3] {
+    Look::ALL
 }
 
 /// Settings on its Appearance pane, in a window as large as the look's
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `cargo test --locked --lib -- ui::preview_settings_tests`
Expected: FAIL. Four fail, in the terminal look: `the_preview_draws_two_bookshop_rows_under_a_dev_bar`, `the_preview_s_rows_follow_the_grid_options` and `the_preview_s_bar_follows_the_environment_option` find no preview ("the preview"), and `the_preview_sends_the_backend_nothing` finds no "Bookshop" painted. The four that are the sheet's alone pass.

- [ ] **Step 3: Draw the preview under the environment rows**

Under the last group, a "preview" heading as the groups have, then the card, 4 under the heading and 4 over what follows, as far in as the rows' words. The cursor's rows are the options': the preview is none of them.

```diff
diff --git a/src/ui/settings/terminal.rs b/src/ui/settings/terminal.rs
index 10580db..a99e020 100644
--- a/src/ui/settings/terminal.rs
+++ b/src/ui/settings/terminal.rs
@@ -391,7 +391,8 @@ fn rows(ui: &mut Ui, settings: &Settings, cursor: usize, skin: &Skin, actions: &
 }
 
 /// The Appearance pane's options, a row each, under the headings of their
-/// groups. Their columns line up across the groups.
+/// groups, and the preview under them. Their columns line up across the
+/// groups.
 fn appearance_rows(
     ui: &mut Ui,
     settings: &Settings,
@@ -431,6 +432,13 @@ fn appearance_rows(
             palette_row(ui, columns, skin, actions);
         }
     }
+    // Under the options, the preview, no row of the cursor's either: 4
+    // over and under it, as far in as the rows' words.
+    heading(ui, "Preview", 12.0, skin);
+    ui.add_space(4.0);
+    let name = skin.say("Preview");
+    super::preview::show(ui, (&name, ROW_SIDE), settings, skin.look, skin.palette);
+    ui.add_space(4.0);
 }
 
 /// The colours the theme gave, eight of them as the board has them, and
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `cargo test --locked --lib -- ui::preview_settings_tests ui::appearance_settings_tests`
Expected: PASS. Then the four checks above.

- [ ] **Step 5: Compare with the board by eye**

As in Task 1, with `Look::omarchy()` at 1920 × 1080 (no scroll needed), beside the "Omarchy – Settings, Appearance" board's preview, in its Gruvbox, Tokyo Night and Catppuccin if the scene's palette is set to them. Remove the scene before committing.

- [ ] **Step 6: Commit**

```bash
git add src/ui/settings/terminal.rs src/ui/preview_settings_tests.rs
git commit -m "Preview the grid and the bar on the Omarchy Settings screen"
```

---

### Task 3: The website

**Files:**
- Modify: `docs/_reference/settings-and-files.md`

- [ ] **Step 1: Say that the pane has a preview**

```diff
diff --git a/docs/_reference/settings-and-files.md b/docs/_reference/settings-and-files.md
index 10d08e1..4e0c483 100644
--- a/docs/_reference/settings-and-files.md
+++ b/docs/_reference/settings-and-files.md
@@ -23,7 +23,9 @@ Cmd/Ctrl+, opens Settings. Changes apply right away.
 
 A second pane, **Appearance**, sets the theme, the text, and how the grids
 and the connection bar are drawn. Each change shows in every open window
-at once.
+at once, and in the preview under the options: two rows of a table under
+the bar of a dev connection, drawn as the grid and the bar are. Its rows
+are made in the app, so it needs no connection and reads no database.
 
 | Option | Choices | Default |
 | --- | --- | --- |
```

- [ ] **Step 2: Check the site**

Run: `cd docs && bin/check`
Expected: the build and its checks pass.

- [ ] **Step 3: Commit**

```bash
git add docs/_reference/settings-and-files.md
git commit -m "Say on the website that Settings › Appearance has a preview"
```

---

## After the plan

Open one pull request for the step. Its description: a summary; the decision at the top (the grid as a picture), with the answer given; the departures table; and how it was compared with the boards (Task 1 and 2's Step 5, by eye, kept local). No screenshot goes to GitHub unblurred. The website's screenshots show Settings › General, so none is rendered again.
