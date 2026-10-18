//! Settings › Appearance, its Text section: the interface's size and the
//! data font of macOS and Windows, the terminal look's one font and size,
//! and the keys that change the size wherever the keyboard is.

use egui::{Key, Modifiers};

use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
use crate::model::Action;
use crate::settings::{FontSize, InterfaceSize, OptionValue};
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
    // The other looks size their text by role, at 1.
    let mut harness = sheet(Look::macos());
    harness
        .app
        .change_settings(|settings| settings.font_size = FontSize::of(16.0));
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
}

#[test]
fn egui_text_styles_follow_the_interface_size() {
    let mut harness = sheet(Look::macos());
    harness
        .app
        .change_settings(|settings| settings.interface_size = InterfaceSize::Large);
    logic(&mut harness);
    let body = harness
        .ctx
        .global_style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map(|font| font.size);
    assert_eq!(body, Some(14.0));
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
                let look = look.with_text(&settings).with_grid(&settings);
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

#[test]
fn the_mac_keys_step_and_reset_the_data_size_and_write_it() {
    let mut harness = sheet(Look::macos());
    let saves = settings_saves(&harness);
    harness.press(Key::Equals, Modifiers::COMMAND);
    assert_eq!(harness.app.settings.font_size, FontSize::of(13.0));
    assert!(file(&harness).contains("font_size = 13\n"));
    harness.press(Key::Minus, Modifiers::COMMAND);
    harness.press(Key::Minus, Modifiers::COMMAND);
    assert_eq!(harness.app.settings.font_size, FontSize::of(12.0));
    harness.press(Key::Num0, Modifiers::COMMAND);
    assert_eq!(harness.app.settings.font_size, None);
    assert!(!file(&harness).contains("font_size"));
    assert_eq!(settings_saves(&harness), saves + 4);
    // egui zooms nothing: the keys are the keymap's.
    logic(&mut harness);
    assert_eq!(harness.ctx.zoom_factor(), 1.0);
    assert!(!harness.ctx.options(|options| options.zoom_with_keyboard));
    // Under the Settings window too, which shows the size.
    on_the_pane(&mut harness);
    harness.press(Key::Equals, Modifiers::COMMAND);
    assert_eq!(harness.app.settings.font_size, FontSize::of(13.0));
    assert!(harness.has("Data font size"));
    let tree = harness.settle();
    let menu = crate::testing::node(&tree, "Data font size", egui::accesskit::Role::ComboBox)
        .and_then(|id| tree.nodes.iter().find(|(node, _)| *node == id))
        .map(|(_, node)| node.value().map(str::to_owned));
    assert_eq!(menu, Some(Some("13 pt".to_owned())));
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
    // No key resets it here: `R` on its row does.
    harness.press(Key::Num0, Modifiers::CTRL);
    assert_eq!(harness.app.settings.font_size, FontSize::of(9.0));
}

#[test]
fn the_sheet_sets_the_interface_size_and_the_data_font() {
    let mut harness = sheet(Look::macos());
    on_the_pane(&mut harness);
    // The board's words.
    for words in [
        "TEXT",
        "Sidebar, tabs, buttons, inspector labels",
        "Grid cells, SQL editor, JSON",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    for size in ["Small · 12", "Default · 13", "Large · 14"] {
        assert!(harness.has(&format!("Interface size: {size}")), "{size}");
    }
    let saves = settings_saves(&harness);
    harness.click("Interface size: Large · 14");
    harness.click("Data font");
    harness.click("Data font JetBrains Mono");
    harness.click("Data font size");
    harness.click("Data font size 14 pt");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.interface_size, InterfaceSize::Large);
    assert_eq!(settings.font.as_deref(), Some("JetBrains Mono"));
    assert_eq!(settings.font_size, FontSize::of(14.0));
    assert_eq!(settings_saves(&harness), saves + 3);
    for line in [
        "interface_size = \"large\"",
        "font = \"JetBrains Mono\"",
        "font_size = 14\n",
    ] {
        assert!(file(&harness).contains(line), "{line}");
    }
    // Every role drawn from here on has them.
    let faces = harness.app.look.faces;
    assert_eq!(faces.interface, InterfaceSize::Large);
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
fn windows_sizes_its_data_and_has_no_face_to_pick() {
    let mut harness = sheet(Look::standard());
    on_the_pane(&mut harness);
    assert!(harness.has("Interface size: Default · 13"));
    let tree = harness.settle();
    let menu = |name| crate::testing::node(&tree, name, egui::accesskit::Role::ComboBox);
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
        "“Fira Code” is not bundled: IBM Plex Mono is used"
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
        OptionValue::InterfaceSize(InterfaceSize::Small),
        OptionValue::Font(Some("JetBrains Mono".into())),
        OptionValue::FontSize(FontSize::of(15.0)),
    ] {
        harness.app.apply(Action::SetOption(value));
    }
    on_the_pane(&mut harness);
    harness.app.apply(Action::ResetSettings);
    harness.app.apply(Action::ConfirmResetSettings(true));
    let settings = &harness.app.settings;
    assert_eq!(settings.interface_size, InterfaceSize::Default);
    assert_eq!(settings.font, None);
    assert_eq!(settings.font_size, None);
}
