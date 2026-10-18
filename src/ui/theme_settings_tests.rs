//! Settings › Appearance, its Theme section: the terminal look's follow
//! and pin with the palette they give, and the tiles of macOS and Windows.

use egui::accesskit::{Role, Toggled};

use super::appearance_settings_tests::{on_the_pane, settings_saves, shows};
use crate::backend::Event;
use crate::desktop_theme::{Pinned, Read};
use crate::model::Action;
use crate::settings::{Settings, Theme};
use crate::testing::Harness;
use crate::theme::{Catalog, CustomTheme, Look, Palette};

/// Whether the control called `label` is told to a screen reader as set.
fn set(harness: &mut Harness, label: &str) -> Option<bool> {
    let tree = harness.settle();
    let node = crate::testing::node(&tree, label, Role::RadioButton)?;
    let (_, node) = tree.nodes.iter().find(|(id, _)| *id == node)?;
    Some(node.toggled() == Some(Toggled::True))
}

/// Omarchy's live palette, with an accent of its own.
fn omarchy_palette() -> Palette {
    let mut palette = Palette::dark();
    palette.accent = egui::Color32::from_rgb(0xf3, 0x8d, 0x70);
    palette
}

/// The terminal look in a window as wide as the board's, where the rows'
/// hints have room beside the file.
fn terminal() -> Harness {
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(Look::omarchy());
    harness
}

/// The terminal look following Omarchy, whose theme of the moment is
/// Tokyo Night.
fn following() -> Harness {
    let mut harness = terminal();
    harness.app.themes = Catalog::preview(
        vec![CustomTheme {
            filename: fastframe_theme::omarchy::FILENAME.into(),
            palette: omarchy_palette(),
        }],
        true,
    );
    harness.app.desktop_theme.current = Some("Tokyo Night".into());
    harness.app.logic(&harness.ctx.clone());
    harness
}

/// The settings with `theme`, as an edit of the file sets it: the file's
/// text comes back from the disk.
fn with_theme(harness: &mut Harness, theme: Theme) {
    let settings = Settings {
        theme,
        ..harness.app.settings.clone()
    };
    let text = settings.to_toml();
    harness
        .app
        .apply(Action::Backend(Event::SettingsFile { text, own: false }));
    harness.app.logic(&harness.ctx.clone());
}

/// What the backend read of the pin `name`: an Omarchy theme's palette.
fn read(harness: &mut Harness, name: &str, palette: Option<Palette>) {
    let read = Read {
        current: Some("Tokyo Night".into()),
        pin: Some((
            name.into(),
            palette.map(|palette| Pinned {
                palette,
                omarchy: true,
            }),
        )),
    };
    harness.app.apply(Action::Backend(Event::Theme(read)));
    harness.app.logic(&harness.ctx.clone());
}

fn key(harness: &mut Harness, key: egui::Key, text: &str) {
    super::tests::type_key(harness, key, text);
}

#[test]
fn the_screen_shows_omarchys_theme_and_the_palette_it_gives() {
    let mut harness = following();
    on_the_pane(&mut harness);
    for words in [
        "theme",
        "follow omarchy",
        "now: tokyo night · switches with omarchy-theme",
        "palette",
        "read from the omarchy theme",
    ] {
        assert!(shows(&mut harness, words), "{words}");
    }
    assert_eq!(set(&mut harness, "Theme: follow omarchy"), Some(true));
    assert_eq!(set(&mut harness, "Theme: pin"), Some(false));
    // The eight swatches, in the palette's colours: the accent among them.
    let palette = harness.app.palette;
    let swatch = |color| {
        harness
            .fills
            .iter()
            .any(|(rect, fill)| *fill == color && rect.size() == egui::vec2(18.0, 14.0))
    };
    for color in [
        palette.window,
        palette.accent,
        palette.danger,
        palette.orange,
    ] {
        assert!(swatch(color), "{color:?}");
    }
}

#[test]
fn l_pins_the_theme_omarchy_shows_and_h_follows_it_again() {
    let mut harness = following();
    on_the_pane(&mut harness);
    let saves = settings_saves(&harness);
    key(&mut harness, egui::Key::L, "l");
    assert_eq!(
        harness.app.settings.theme,
        Theme::Pinned("Tokyo Night".into())
    );
    assert_eq!(settings_saves(&harness), saves + 1);
    assert!(
        harness
            .app
            .settings_file
            .text
            .contains("theme = \"Tokyo Night\"")
    );
    // Drawn once the backend read it.
    let mut tokyo = Palette::dark();
    tokyo.accent = egui::Color32::from_rgb(0x7a, 0xa2, 0xf7);
    read(&mut harness, "Tokyo Night", Some(tokyo));
    assert!(shows(&mut harness, "now: Tokyo Night · pinned"));
    assert!(shows(&mut harness, "read from the omarchy theme"));
    assert_eq!(set(&mut harness, "Theme: pin"), Some(true));
    // l again keeps the pin; h follows Omarchy again.
    key(&mut harness, egui::Key::L, "l");
    assert_eq!(
        harness.app.settings.theme,
        Theme::Pinned("Tokyo Night".into())
    );
    key(&mut harness, egui::Key::H, "h");
    assert_eq!(harness.app.settings.theme, Theme::Omarchy);
    // Space flips it, and R takes the value out of the file.
    harness.press(egui::Key::Space, egui::Modifiers::NONE);
    harness.settle();
    assert_eq!(
        harness.app.settings.theme,
        Theme::Pinned("Tokyo Night".into())
    );
    harness.press(egui::Key::R, egui::Modifiers::SHIFT);
    harness.settle();
    assert_eq!(harness.app.settings.theme, Theme::Unset);
    assert!(!harness.app.settings_file.text.contains("theme ="));
}

#[test]
fn a_pin_and_a_flip_in_one_frame_follow_omarchy_again() {
    let mut harness = following();
    on_the_pane(&mut harness);
    harness.frame(vec![
        crate::testing::key(egui::Key::L, egui::Modifiers::NONE),
        egui::Event::Text("l".into()),
        crate::testing::key(egui::Key::Space, egui::Modifiers::NONE),
    ]);
    harness.settle();
    assert_eq!(harness.app.settings.theme, Theme::Omarchy);
}

#[test]
fn with_no_omarchy_theme_to_name_pin_does_nothing() {
    let mut harness = terminal();
    on_the_pane(&mut harness);
    assert!(shows(&mut harness, "now: dark, from the system"));
    assert!(shows(&mut harness, "tabletist's own light or dark"));
    let saves = settings_saves(&harness);
    key(&mut harness, egui::Key::L, "l");
    assert_eq!(harness.app.settings.theme, Theme::Unset);
    assert_eq!(settings_saves(&harness), saves);
}

#[test]
fn a_value_of_macos_is_the_desktop_here_and_stays_in_the_file() {
    let mut harness = following();
    with_theme(&mut harness, Theme::Dark);
    on_the_pane(&mut harness);
    assert_eq!(set(&mut harness, "Theme: follow omarchy"), Some(true));
    assert_eq!(harness.app.palette, omarchy_palette());
    // h, and a click on follow, change nothing the row shows: nothing.
    let saves = settings_saves(&harness);
    key(&mut harness, egui::Key::H, "h");
    harness.click("Theme: follow omarchy");
    harness.settle();
    assert_eq!(harness.app.settings.theme, Theme::Dark);
    assert_eq!(settings_saves(&harness), saves);
    // Another option's change writes the file again, with the value in it.
    key(&mut harness, egui::Key::J, "j");
    key(&mut harness, egui::Key::H, "h");
    assert!(harness.app.settings_file.text.contains("theme = \"dark\""));
    assert_eq!(harness.app.settings.theme, Theme::Dark);
}

#[test]
fn a_pin_found_nowhere_is_said_beside_it() {
    let mut harness = following();
    with_theme(&mut harness, Theme::Pinned("Gone".into()));
    read(&mut harness, "Gone", None);
    on_the_pane(&mut harness);
    let said = "“Gone” not found · following omarchy";
    assert!(shows(&mut harness, said));
    assert_eq!(
        harness.painted_color(said),
        Some(harness.app.palette.warning)
    );
    // A palette file is said as written: the pin is drawn from it.
    with_theme(&mut harness, Theme::Pinned("~/mine/Sunset.json".into()));
    let mut sunset = Palette::light();
    sunset.accent = egui::Color32::from_rgb(0x44, 0x44, 0x44);
    let file = Read {
        current: Some("Tokyo Night".into()),
        pin: Some((
            "~/mine/Sunset.json".into(),
            Some(Pinned {
                palette: sunset,
                omarchy: false,
            }),
        )),
    };
    harness.app.apply(Action::Backend(Event::Theme(file)));
    harness.app.logic(&harness.ctx.clone());
    assert!(shows(&mut harness, "now: ~/mine/Sunset.json · pinned"));
    assert!(shows(&mut harness, "read from ~/mine/Sunset.json"));
}
