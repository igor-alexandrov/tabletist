//! Settings › Appearance, its Text section: the interface's size and the
//! data font of macOS and Windows, the terminal look's one font and size,
//! and the keys that change the size wherever the keyboard is.

use crate::settings::{FontSize, InterfaceSize};
use crate::testing::Harness;
use crate::theme::Look;

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
