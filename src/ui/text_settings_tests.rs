//! Settings › Appearance, its Text section: the interface's scale and the
//! data font of macOS and Windows, the terminal look's one font and size,
//! and the keys that change the scale or the size wherever the keyboard is.

use egui::{Key, Modifiers};

use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
use crate::font_files::desktop_font::Read;
use crate::model::Action;
use crate::settings::{FontSize, OptionValue, UiScale};
use crate::testing::Harness;
use crate::theme::{DataFace, Look};

/// One pass of the app's logic, as the window runs it between frames.
fn logic(harness: &mut Harness) {
    let mut input = egui::RawInput::default();
    input.viewports.entry(egui::ViewportId::ROOT).or_default();
    let app = &mut harness.app;
    let _ = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
    // A zoom set there is the next frame's.
    harness.settle();
}

fn sheet(look: Look) -> Harness {
    let mut harness = Harness::new();
    harness.set_look(look);
    harness
}

/// The terminal look in a window as wide as the board's, where the rows'
/// hints have room beside the file.
fn terminal() -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    harness
}

fn key(harness: &mut Harness, key: Key, text: &str) {
    super::tests::type_key(harness, key, text);
}

/// The settings file as the app last wrote it.
fn file(harness: &Harness) -> &str {
    &harness.app.settings_file.text
}

#[test]
fn the_terminal_look_draws_the_whole_window_at_its_size() {
    let mut harness = terminal();
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(19.5));
    logic(&mut harness);
    // To a whole point, as the look steps.
    assert_eq!(harness.ctx.zoom_factor(), 20.0 / 13.0);
    // Up to 32.
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(32.0));
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 32.0 / 13.0);
    // The other looks' data size does not zoom the window: their scale
    // does.
    let mut harness = sheet(Look::macos());
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(16.0));
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
}

#[test]
fn each_interface_scale_sets_the_zoom_factor_to_its_own() {
    for look in [Look::macos(), Look::standard()] {
        let mut harness = sheet(look);
        for scale in UiScale::ALL {
            harness
                .app
                .apply(Action::SetOption(OptionValue::InterfaceScale(scale)));
            logic(&mut harness);
            assert_eq!(harness.ctx.zoom_factor(), scale.factor(), "{}", look.name);
            // egui multiplies the display's own scale by it.
            let native = harness.ctx.native_pixels_per_point().unwrap_or(1.0);
            assert_eq!(harness.ctx.pixels_per_point(), native * scale.factor());
        }
    }
    // On a display of 2 pixels a point too.
    let mut harness = sheet(Look::macos());
    harness.scale = 2.0;
    harness
        .app
        .apply(Action::SetOption(OptionValue::InterfaceScale(
            UiScale::of(1.25).unwrap(),
        )));
    logic(&mut harness);
    assert_eq!(harness.ctx.pixels_per_point(), 2.5);
}

#[test]
fn the_interface_keeps_the_designs_sizes_and_the_scale_zooms_them() {
    let mut harness = sheet(Look::macos());
    harness.app.change_settings(|settings| {
        settings.ui_scale = UiScale::of(1.5).unwrap();
    });
    logic(&mut harness);
    let body = harness
        .ctx
        .global_style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map(|font| font.size);
    // 13 points, at 1.5 points a pixel's worth: 19.5 pixels.
    assert_eq!(body, Some(13.0));
    assert_eq!(harness.ctx.zoom_factor(), 1.5);
}

#[test]
fn the_data_font_size_multiplies_with_the_scale() {
    for look in [Look::macos(), Look::standard()] {
        let mut harness = sheet(look);
        harness.app.change_settings(|settings| {
            settings.ui_scale = UiScale::of(1.5).unwrap();
            settings.font_size = FontSize::of(12.5);
        });
        logic(&mut harness);
        let faces = harness.app.look.faces;
        let cell = crate::typography::TextRole::GridCell.size(faces);
        // 12.5 points at 1.5: 18.75 pixels on a display of one a point.
        assert_eq!(cell * harness.ctx.zoom_factor(), 18.75, "{}", look.name);
        harness.app.change_settings(|settings| {
            settings.font_size = FontSize::of(16.0);
        });
        logic(&mut harness);
        let faces = harness.app.look.faces;
        let cell = crate::typography::TextRole::GridCell.size(faces);
        assert_eq!(cell * harness.ctx.zoom_factor(), 24.0, "{}", look.name);
    }
}

#[test]
fn a_grid_row_holds_a_line_of_its_data_at_every_size() {
    for look in [Look::macos(), Look::standard()] {
        let mut harness = sheet(look);
        harness.settle();
        for halves in 22..=32 {
            for density in crate::settings::RowHeight::ALL {
                let settings = crate::settings::Settings {
                    font_size: Some(FontSize::halves(halves)),
                    row_height: density,
                    ..Default::default()
                };
                let look = look.with_text(&settings, false).with_grid(&settings);
                let line = crate::ui::grid::data_role(&look).row_height(&harness.ctx, look.faces);
                assert!(line <= look.row(), "{} {halves} {density:?}", look.name);
            }
        }
    }
    // The design's rows at its size: nobody's grid changes on upgrade.
    let compact = crate::settings::Settings {
        row_height: crate::settings::RowHeight::Compact,
        ..Default::default()
    };
    assert_eq!(Look::macos().with_grid(&compact).row(), 29.0);
    assert_eq!(Look::standard().with_grid(&compact).row(), 18.0);
}

/// The interface scale the app has, in percent.
fn percent(harness: &Harness) -> u16 {
    harness.app.settings.ui_scale.percent()
}

#[test]
fn the_mac_keys_step_through_the_scales_stop_at_the_ends_and_write_it() {
    // macOS's Cmd, and Windows's and Linux's Ctrl, which egui-winit sends
    // there with its command flag.
    let ctrl = Modifiers {
        ctrl: true,
        command: true,
        ..Modifiers::NONE
    };
    for (look, chord) in [
        (Look::macos(), Modifiers::COMMAND),
        (Look::standard(), ctrl),
    ] {
        let mut harness = sheet(look);
        let saves = settings_saves(&harness);
        harness.press(Key::Equals, chord);
        assert_eq!(percent(&harness), 110, "{}", look.name);
        assert!(file(&harness).contains("ui_scale = 1.1"));
        harness.press(Key::Equals, chord);
        assert_eq!(percent(&harness), 125);
        for _ in 0..8 {
            harness.press(Key::Equals, chord);
        }
        assert_eq!(percent(&harness), 250, "stops at the top");
        harness.press(Key::Num0, chord);
        assert_eq!(percent(&harness), 100);
        assert!(!file(&harness).contains("ui_scale"));
        harness.press(Key::Minus, chord);
        harness.press(Key::Minus, chord);
        assert_eq!(percent(&harness), 90, "stops at the bottom");
        assert!(file(&harness).contains("ui_scale = 0.9"));
        // A save each time it changed: two up, the four left to the top,
        // 100%, and 90%. None at an end.
        assert_eq!(settings_saves(&harness), saves + 8);
        // The data's size stays as it is: it has no key.
        assert_eq!(harness.app.settings.font_size, None);
        // egui zooms nothing by itself: the keys are the keymap's.
        assert!(!harness.ctx.options(|options| options.zoom_with_keyboard));
        logic(&mut harness);
        assert_eq!(harness.ctx.zoom_factor(), 0.9);
    }
}

#[test]
fn a_key_that_changes_the_scale_says_so_in_the_toast() {
    let mut harness = sheet(Look::macos());
    harness.press(Key::Equals, Modifiers::COMMAND);
    harness.press(Key::Equals, Modifiers::COMMAND);
    harness.press(Key::Equals, Modifiers::COMMAND);
    let reset = harness.app.keymap.key(
        harness.app.layout(),
        crate::keymap::Command::DefaultTextSize,
    );
    let said = format!("Interface 150% · 20 px text · {} resets", reset.as_str());
    assert!(shows(&mut harness, &said), "{said}: {:?}", harness.painted);
    // For the toast's 4 seconds.
    assert_eq!(crate::ui::toast::SECONDS, 4.0);
}

#[test]
fn the_scale_keys_work_under_the_settings_window_which_shows_it_unsaid() {
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    harness.press(Key::Equals, Modifiers::COMMAND);
    assert_eq!(percent(&harness), 110);
    let tree = harness.settle();
    let menu = crate::testing::node(&tree, "Interface scale", egui::accesskit::Role::ComboBox)
        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
        .map(|(_, node)| node.value().map(str::to_owned));
    assert_eq!(menu, Some(Some("110 percent, 14 pixel text".to_owned())));
    // Picked in the window, it is said nowhere: the window shows it.
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    harness.click("Interface scale");
    harness.click("150 percent, 20 pixel text");
    assert_eq!(percent(&harness), 150);
    assert!(!shows(&mut harness, "Interface 150%"));
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.5);
    assert!(harness.app.dialog.is_some(), "the window stays open");
}

#[test]
fn the_terminal_keys_step_the_size_anywhere_and_stop_at_its_ends() {
    let mut harness = terminal();
    harness.press(Key::Equals, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, FontSize::of(14.0));
    // Under the Settings screen, whose row follows.
    on_the_pane(&mut harness);
    harness.press(Key::Minus, Modifiers::CTRL);
    harness.press(Key::Minus, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
    assert!(harness.app.dialog.is_some(), "the screen stays open");
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(9.0));
    harness.press(Key::Minus, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, FontSize::of(9.0));
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(31.0));
    harness.press(Key::Equals, Modifiers::CTRL);
    harness.press(Key::Equals, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, FontSize::of(32.0));
    // Ctrl+0 takes it back to 13, out of the file.
    harness.press(Key::Num0, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, None);
    assert_eq!(
        harness.app.settings.font_size_in(true),
        FontSize::of(13.0).unwrap()
    );
    assert!(!file(&harness).contains("font_size"));
    // The scale is the other looks': the terminal's keys leave it.
    assert_eq!(percent(&harness), 100);
}

#[test]
fn a_terminal_key_that_changes_the_size_says_so_on_the_status_line() {
    let mut harness = terminal();
    let reset = harness.app.keymap.key(
        harness.app.layout(),
        crate::keymap::Command::DefaultTextSize,
    );
    let said = |size: u8| format!("font size {size} · {} resets", reset.as_str());
    for _ in 0..5 {
        harness.press(Key::Equals, Modifiers::CTRL);
    }
    // On the connections' screen,
    assert!(shows(&mut harness, &said(18)), "{:?}", harness.painted);
    // on a connection's start screen,
    let tab = harness.connect_fake();
    harness.press(Key::Minus, Modifiers::CTRL);
    assert!(shows(&mut harness, &said(17)));
    // and with a SQL tab open.
    harness.add_sql_tab(tab);
    harness.press(Key::Minus, Modifiers::CTRL);
    assert!(shows(&mut harness, &said(16)), "{:?}", harness.painted);
}

#[test]
fn the_sheet_sets_the_interface_scale_and_the_data_font() {
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    // The board's words.
    let keys = |command| {
        crate::ui::keys::written(&harness.ctx, &harness.app.look, command)
            .as_str()
            .to_owned()
    };
    let hint = format!(
        "{} larger · {} smaller · {} back to 100%",
        keys(crate::keymap::Command::LargerText),
        keys(crate::keymap::Command::SmallerText),
        keys(crate::keymap::Command::DefaultTextSize),
    );
    for words in [
        "TEXT",
        "Everything in the window: text, controls, rows and spacing",
        "Grid cells, SQL editor, JSON · scales with the interface",
        "100% · 13 px text",
        &hint,
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // Every scale in the menu, read out with the size of its text.
    harness.click("Interface scale");
    for (percent, pixels) in [
        (90, 12),
        (100, 13),
        (110, 14),
        (125, 16),
        (150, 20),
        (175, 23),
        (200, 26),
        (250, 33),
    ] {
        let name = format!("{percent} percent, {pixels} pixel text");
        assert!(harness.has(&name), "{name}");
        assert!(shows(&mut harness, &format!("{percent}%")), "{percent}");
    }
    assert_eq!(
        current(&mut harness, "100 percent, 13 pixel text"),
        Some(true)
    );
    let saves = settings_saves(&harness);
    harness.click("125 percent, 16 pixel text");
    harness.click("Data font");
    harness.click("Data font JetBrains Mono");
    harness.click("Data font size");
    harness.click("Data font size 14 pt");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.ui_scale, UiScale::of(1.25).unwrap());
    assert_eq!(settings.font.as_deref(), Some("JetBrains Mono"));
    assert_eq!(settings.font_size, FontSize::of(14.0));
    assert_eq!(settings_saves(&harness), saves + 3);
    for line in [
        "ui_scale = 1.25",
        "font = \"JetBrains Mono\"",
        "font_size = 14",
    ] {
        assert!(file(&harness).contains(line), "{line}");
    }
    assert!(shows(&mut harness, "125% · 16 px text"));
    // Every role drawn from here on has them.
    let faces = harness.app.look.faces;
    assert_eq!(faces.data, DataFace::JetBrainsMono);
    assert_eq!(faces.data_size, FontSize::halves(28));
    // The default face is no value: the file leaves it out.
    harness.click("Data font");
    harness.click("Data font IBM Plex Mono");
    harness.settle();
    assert_eq!(harness.app.settings.font, None);
    assert!(!file(&harness).contains("font = "));
}

#[test]
fn changing_the_scale_keeps_the_keyboard_where_it_was() {
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    harness.click("Data font size");
    harness.press(Key::Escape, Modifiers::NONE);
    let focused = |harness: &mut Harness| {
        let tree = harness.settle();
        tree.focus
    };
    let before = focused(&mut harness);
    for _ in 0..5 {
        harness.press(Key::Equals, Modifiers::COMMAND);
    }
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 2.0);
    assert_eq!(focused(&mut harness), before);
}

#[test]
fn windows_sizes_its_data_and_has_no_face_to_pick() {
    let mut harness = sheet(Look::standard());
    on_the_pane(&mut harness);
    assert!(shows(&mut harness, "100% · 13 px text"));
    let tree = harness.settle();
    let menu = |name| crate::testing::node(&tree, name, egui::accesskit::Role::ComboBox);
    assert!(menu("Interface scale").is_some());
    // The keys alone beside it: with Ctrl's, the words have no room.
    let keys = [
        crate::keymap::Command::LargerText,
        crate::keymap::Command::SmallerText,
        crate::keymap::Command::DefaultTextSize,
    ]
    .map(|command| {
        crate::ui::keys::written(&harness.ctx, &harness.app.look, command)
            .as_str()
            .to_owned()
    });
    assert!(
        shows(&mut harness, &keys.join(" / ")),
        "{:?}",
        harness.painted
    );
    assert!(!shows(&mut harness, "back to 100%"));
    assert!(menu("Data font size").is_some());
    assert!(menu("Data font").is_none(), "its data is drawn in Inter");
    harness.click("Data font size");
    harness.click("Data font size 11 pt");
    harness.settle();
    assert_eq!(harness.app.settings.font_size, FontSize::of(11.0));
}

#[test]
fn a_data_font_that_is_not_bundled_is_drawn_as_the_default_and_said() {
    let mut harness = sheet(Look::macos());
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
    on_the_pane(&mut harness);
    assert!(shows(
        &mut harness,
        "“Fira Code” is not installed: IBM Plex Mono is used"
    ));
    assert!(!shows(&mut harness, "Grid cells, SQL editor, JSON"));
    // The menu shows the face drawn.
    let tree = harness.settle();
    let menu = crate::testing::node(&tree, "Data font", egui::accesskit::Role::ComboBox)
        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
        .map(|(_, node)| node.value().map(str::to_owned));
    assert_eq!(menu, Some(Some("IBM Plex Mono".to_owned())));
}

#[test]
fn the_data_font_row_fits_the_smallest_sheet() {
    let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    let tree = harness.settle();
    let menu = |name| crate::testing::bounds(&tree, name, egui::accesskit::Role::ComboBox);
    let (font, size) = (menu("Data font").unwrap(), menu("Data font size").unwrap());
    // The sheet ends 40 in from the window's edge, the font's menu gives
    // way.
    assert!(size.right() <= 680.0, "{size:?}");
    assert!(
        font.width() < 220.0 && font.right() < size.left(),
        "{font:?}"
    );
}

#[test]
fn reset_to_defaults_puts_the_text_back() {
    let mut harness = sheet(Look::macos());
    for value in [
        OptionValue::InterfaceScale(UiScale::of(2.0).unwrap()),
        OptionValue::Font(Some("JetBrains Mono".into())),
        OptionValue::FontSize(FontSize::of(15.0)),
    ] {
        harness.app.apply(Action::SetOption(value));
    }
    on_the_pane(&mut harness);
    harness.app.apply(Action::ResetSettings);
    harness.app.apply(Action::ConfirmResetSettings(true));
    let settings = &harness.app.settings;
    assert_eq!(settings.ui_scale, UiScale::DEFAULT);
    assert_eq!(settings.font, None);
    assert_eq!(settings.font_size, None);
}

#[test]
fn the_screen_shows_the_font_and_steps_the_size_with_its_keys() {
    let mut harness = terminal();
    on_the_pane(&mut harness);
    // The board's rows, the font as the bundled one in tests: nothing is
    // read from fontconfig. The size's keys as the keymap writes them.
    let keymap = crate::keymap::Keymap::default();
    let written = |command| keymap.label(crate::keymap::Layout::Omarchy, command);
    let keys = format!(
        "{} / {} / {} anywhere · rows and panes grow with it",
        written(crate::keymap::Command::LargerText),
        written(crate::keymap::Command::SmallerText),
        written(crate::keymap::Command::DefaultTextSize)
    );
    for words in [
        "text",
        "system monospace",
        "9 to 32",
        "now: jetbrains mono, bundled",
        &keys,
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    let saves = settings_saves(&harness);
    // The size, under the interface, the theme and the font: a step up,
    // then two down.
    key(&mut harness, Key::J, "j");
    key(&mut harness, Key::J, "j");
    key(&mut harness, Key::J, "j");
    assert!(shows(&mut harness, "13"));
    key(&mut harness, Key::L, "l");
    assert_eq!(harness.app.settings.font_size, FontSize::of(14.0));
    assert!(file(&harness).contains("font_size = 14"));
    key(&mut harness, Key::H, "h");
    key(&mut harness, Key::H, "h");
    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
    assert_eq!(settings_saves(&harness), saves + 3);
    // R takes it out of the file, the look's own again.
    harness.press(Key::R, Modifiers::SHIFT);
    assert_eq!(harness.app.settings.font_size, None);
    assert!(!file(&harness).contains("font_size"));
    // The font has nothing to step to: a name is set in the file.
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    let saves = settings_saves(&harness);
    key(&mut harness, Key::K, "k");
    key(&mut harness, Key::L, "l");
    harness.press(Key::Space, Modifiers::NONE);
    assert_eq!(harness.app.settings.font.as_deref(), Some("Fira Code"));
    assert_eq!(settings_saves(&harness), saves);
}

#[test]
fn the_screen_says_the_font_fontconfig_drew_and_one_it_has_not() {
    let mut harness = terminal();
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    harness.app.desktop_font = Read {
        asked: Some("Fira Code".into()),
        faces: None,
        family: Some("JetBrainsMono Nerd Font".into()),
        missing: true,
    };
    on_the_pane(&mut harness);
    assert!(shows(&mut harness, "Fira Code"));
    let said = "“Fira Code” not found · now: jetbrainsmono nerd font";
    assert!(shows(&mut harness, said));
    assert_eq!(
        harness.painted_color(said),
        Some(harness.app.palette.warning)
    );
    // Found: what fontconfig drew, dim.
    harness.app.desktop_font.missing = false;
    harness.app.desktop_font.family = Some("Fira Code".into());
    let said = "now: fira code, from fontconfig";
    assert!(shows(&mut harness, said));
    assert_eq!(harness.painted_color(said), Some(harness.app.palette.dim));
    // R puts the system's monospace back, on the font's row under the
    // interface's and the theme's.
    key(&mut harness, Key::J, "j");
    key(&mut harness, Key::J, "j");
    harness.press(Key::R, Modifiers::SHIFT);
    assert_eq!(harness.app.settings.font, None);
    assert!(!file(&harness).contains("font ="));
}

#[test]
fn a_font_not_found_is_read_out_and_describes_its_row() {
    let mut harness = terminal();
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    harness.app.desktop_font = Read {
        asked: Some("Fira Code".into()),
        faces: None,
        family: Some("JetBrainsMono Nerd Font".into()),
        missing: true,
    };
    on_the_pane(&mut harness);
    let said = "“Fira Code” not found · now: jetbrainsmono nerd font";
    let tree = harness.settle();
    assert!(super::settings_access_tests::heard(&tree, said));
    let row = crate::testing::node(&tree, "Font", egui::accesskit::Role::Button).expect("the row");
    let (_, row) = tree
        .nodes
        .iter()
        .find(|(id, _)| *id == row)
        .expect("the row");
    assert_eq!(row.description(), Some(said));
    // Found, it is no warning: the row says what fontconfig drew, and
    // nothing is read out.
    harness.app.desktop_font.missing = false;
    harness.app.desktop_font.family = Some("Fira Code".into());
    let tree = harness.settle();
    assert!(!super::settings_access_tests::heard(&tree, said));
    let found = "now: fira code, from fontconfig";
    assert!(!super::settings_access_tests::heard(&tree, found));
    let row = crate::testing::node(&tree, "Font", egui::accesskit::Role::Button).expect("the row");
    let (_, row) = tree
        .nodes
        .iter()
        .find(|(id, _)| *id == row)
        .expect("the row");
    assert_eq!(row.description(), Some(found));
}

/// Whether the entry called `label` is told to a screen reader as the
/// one in use.
fn current(harness: &mut Harness, label: &str) -> Option<bool> {
    let tree = harness.settle();
    let node = crate::testing::node(&tree, label, egui::accesskit::Role::Button)?;
    let (_, node) = tree.nodes.iter().find(|(id, _)| *id == node)?;
    Some(node.toggled() == Some(egui::accesskit::Toggled::True))
}

/// What the data font's field reads.
fn data_font_shown(harness: &mut Harness) -> Option<String> {
    let tree = harness.settle();
    let id = crate::testing::node(&tree, "Data font", egui::accesskit::Role::ComboBox)?;
    let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id)?;
    node.value().map(str::to_owned)
}

/// What CoreText would give for `name`: Hack's faces (egui's own), or
/// none where it is not installed.
fn installed_read(name: &str, found: bool) -> Read {
    let hack = egui::FontDefinitions::default().font_data["Hack"].clone();
    Read {
        asked: Some(name.into()),
        faces: found.then(|| [hack.clone(), hack.clone(), hack]),
        family: found.then(|| name.into()),
        missing: !found,
    }
}

/// The backend's answer to the font read numbered `asked`, then the
/// logic's pass that draws it.
fn answer(harness: &mut Harness, asked: u64, read: Read) {
    harness
        .app
        .apply(Action::Backend(crate::backend::Event::Font {
            asked,
            read: Box::new(read),
        }));
    logic(harness);
}

#[test]
fn the_mac_menu_lists_the_installed_fonts_after_the_bundled_ones() {
    let mut harness = sheet(Look::macos());
    // CoreText's list, as the backend gives it: a list of the test's.
    harness.app.installed_fonts = vec!["Fira Code".into(), "Menlo".into()];
    on_the_pane(&mut harness);
    harness.click("Data font");
    let tree = harness.settle();
    let place = |name| crate::testing::bounds(&tree, name, egui::accesskit::Role::Button);
    let names = [
        "Data font IBM Plex Mono",
        "Data font JetBrains Mono",
        "Data font Fira Code",
        "Data font Menlo",
    ];
    let tops: Vec<f32> = names
        .iter()
        .map(|name| place(name).unwrap_or_else(|| panic!("{name}")).top())
        .collect();
    assert!(tops.is_sorted() && tops[0] < tops[3], "{tops:?}");
    assert!(shows(&mut harness, "Installed"));
    assert_eq!(current(&mut harness, "Data font IBM Plex Mono"), Some(true));
    assert_eq!(current(&mut harness, "Data font Menlo"), Some(false));
    let saves = settings_saves(&harness);
    harness.click("Data font Menlo");
    harness.settle();
    assert_eq!(harness.app.settings.font.as_deref(), Some("Menlo"));
    assert!(file(&harness).contains("font = \"Menlo\""));
    assert_eq!(settings_saves(&harness), saves + 1);
    // Read, it is drawn: the field reads it, and the menu has it in use.
    answer(&mut harness, 1, installed_read("Menlo", true));
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Menlo"));
    assert!(shows(&mut harness, "Grid cells, SQL editor, JSON"));
    harness.click("Data font");
    assert_eq!(current(&mut harness, "Data font Menlo"), Some(true));
    assert_eq!(
        current(&mut harness, "Data font IBM Plex Mono"),
        Some(false)
    );
    // A bundled face again: drawn as it always was.
    harness.click("Data font JetBrains Mono");
    harness.settle();
    assert_eq!(harness.app.settings.font.as_deref(), Some("JetBrains Mono"));
    assert_eq!(harness.app.look.faces.data, DataFace::JetBrainsMono);
}

#[test]
fn without_installed_fonts_the_menu_has_the_bundled_ones_alone() {
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    harness.click("Data font");
    assert!(harness.has("Data font JetBrains Mono"));
    assert!(!shows(&mut harness, "Installed"));
}

#[test]
fn an_installed_font_is_drawn_once_read_and_said_where_it_is_not_installed() {
    let mut harness = sheet(Look::macos());
    // Where the desktop's fonts are read, as the app does outside tests.
    harness.app.follow_desktop = true;
    harness.app.change_settings(|settings| {
        settings.font = Some("Fira Code".into());
    });
    on_the_pane(&mut harness);
    // On its way: the field reads the name, and nothing is said.
    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Fira Code"));
    assert!(shows(&mut harness, "Grid cells, SQL editor, JSON"));
    answer(&mut harness, 1, installed_read("Fira Code", true));
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
    // Its faces lead the installed families.
    let fonts = &harness.app.fonts;
    for weight in [400, 500] {
        let family = egui::FontFamily::Name(format!("installed-mono-{weight}").into());
        assert_eq!(
            fonts.families[&family][0],
            format!("installed-mono-{weight}")
        );
    }
    // Another on its way: the one drawn stays until it ends.
    harness.app.change_settings(|settings| {
        settings.font = Some("Gone Mono".into());
    });
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
    assert_eq!(data_font_shown(&mut harness).as_deref(), Some("Gone Mono"));
    // Not installed: the default, said, and the field reads the default.
    answer(&mut harness, 2, installed_read("Gone Mono", false));
    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
    assert!(shows(
        &mut harness,
        "“Gone Mono” is not installed: IBM Plex Mono is used"
    ));
    assert_eq!(
        data_font_shown(&mut harness).as_deref(),
        Some("IBM Plex Mono")
    );
    let fonts = &harness.app.fonts;
    let family = egui::FontFamily::Name("installed-mono-400".into());
    assert_eq!(fonts.families[&family][0], "plex-mono-400");
}

#[test]
fn a_read_for_a_name_since_changed_does_not_replace_the_face_drawn() {
    let mut harness = sheet(Look::macos());
    harness.app.follow_desktop = true;
    let font = |harness: &mut Harness, name: &str| {
        harness.app.change_settings(|settings| {
            settings.font = Some(name.into());
        });
    };
    let leads = |harness: &Harness| {
        let family = egui::FontFamily::Name("installed-mono-400".into());
        harness.app.fonts.families[&family][0].clone()
    };
    font(&mut harness, "Menlo");
    answer(&mut harness, 1, installed_read("Menlo", true));
    // Iosevka, then Menlo again: Iosevka's read ends first, and finds
    // nothing. Menlo's faces stay drawn until its own read ends.
    font(&mut harness, "Iosevka");
    font(&mut harness, "Menlo");
    answer(&mut harness, 2, installed_read("Iosevka", false));
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
    assert_eq!(leads(&harness), "installed-mono-400");
    answer(&mut harness, 3, installed_read("Menlo", true));
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
    // The newest read is taken whatever it read.
    font(&mut harness, "Gone Mono");
    answer(&mut harness, 4, installed_read("Gone Mono", false));
    assert_eq!(harness.app.look.faces.data, DataFace::PlexMono);
    assert_eq!(leads(&harness), "plex-mono-400");
    // From a bundled face too: it stays until the read ends.
    font(&mut harness, "JetBrains Mono");
    font(&mut harness, "Fira Code");
    assert_eq!(harness.app.look.faces.data, DataFace::JetBrainsMono);
    answer(&mut harness, 5, installed_read("Fira Code", true));
    assert_eq!(harness.app.look.faces.data, DataFace::Installed);
}
