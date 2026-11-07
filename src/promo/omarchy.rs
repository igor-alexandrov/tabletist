//! Omarchy switching its theme while a video is filmed, as
//! `omarchy-theme-set` does on the desktop: the app is told what it is
//! told there, and recolours as it does there. Nothing of the desktop's is
//! changed: its themes are only read.
//!
//! No GPU here, so the tests run in the normal suite.

use crate::app::App;
use crate::backend::Event;
use crate::desktop_theme::{self, Read};
use crate::model::Action;
use crate::theme::{Catalog, Palette};

/// The palette of the Omarchy theme called `name`, as Omarchy shows it
/// (`Retro 82`): its colours through Tabletist's template, read as the app
/// reads a theme pinned by name, the user's own theme before the shipped
/// one.
#[cfg(feature = "shots")]
pub fn palette(name: &str) -> Result<Palette, String> {
    let home = directories::BaseDirs::new()
        .map(|dirs| dirs.home_dir().to_path_buf())
        .ok_or("there is no home folder to find Omarchy's themes in")?;
    let read = desktop_theme::read(
        &desktop_theme::Places::of(&home),
        Some(name),
        desktop_theme::omarchy_colors,
    );
    match read.pin {
        Some((_, Some(theme))) if theme.omarchy => Ok(theme.palette),
        _ => Err(format!("Omarchy here has no theme called {name:?}")),
    }
}

/// Omarchy switched to `name`, whose palette is `palette`: the catalogue's
/// scan finds it as the live palette, and the backend's read the theme's
/// name (`asked`, counted from 1, as the app counts its reads). Then the
/// frame's logic takes them, as it does on the desktop.
pub fn switch(app: &mut App, ctx: &egui::Context, asked: u64, name: &str, palette: Palette) {
    app.themes = Catalog::preview(
        vec![fastframe_theme::CustomTheme {
            filename: fastframe_theme::omarchy::FILENAME.into(),
            palette,
        }],
        true,
    );
    let read = Read {
        current: Some(name.into()),
        pin: None,
    };
    app.apply(Action::Backend(Event::Theme { asked, read }));
    app.logic(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Harness;

    fn accented(rgb: [u8; 3]) -> Palette {
        let mut palette = Palette::dark();
        palette.accent = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        palette
    }

    #[test]
    fn a_switch_recolours_the_window_as_omarchy_s_does() {
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let ctx = harness.ctx.clone();
        let ristretto = accented([0xf3, 0x8d, 0x70]);
        switch(&mut harness.app, &ctx, 1, "Ristretto", ristretto);
        assert_eq!(harness.app.palette, ristretto.with_readable_labels());
        assert_eq!(
            harness.app.desktop_theme.current.as_deref(),
            Some("Ristretto")
        );
        assert_eq!(
            ctx.global_style().visuals.hyperlink_color,
            harness.app.palette.accent,
            "egui's style is the new palette's"
        );
        // And again: each switch is a newer read.
        let mut latte = Palette::light();
        latte.accent = egui::Color32::from_rgb(0x1e, 0x66, 0xf5);
        switch(&mut harness.app, &ctx, 2, "Catppuccin Latte", latte);
        assert_eq!(harness.app.palette, latte.with_readable_labels());
        assert_eq!(
            harness.app.desktop_theme.current.as_deref(),
            Some("Catppuccin Latte")
        );
    }

    #[test]
    fn a_pinned_theme_stays_when_omarchy_switches() {
        // The app's own rule, not the video's: a pin is not the desktop's.
        let mut harness = Harness::new();
        harness.set_look(crate::theme::Look::omarchy());
        let pinned = accented([0x7a, 0xa2, 0xf7]);
        harness.app.desktop_theme.pin = Some((
            "Tokyo Night".into(),
            Some(desktop_theme::Pinned {
                palette: pinned,
                omarchy: true,
            }),
        ));
        harness.app.settings.theme = crate::settings::Theme::Pinned("Tokyo Night".into());
        let ctx = harness.ctx.clone();
        switch(
            &mut harness.app,
            &ctx,
            1,
            "Nord",
            accented([0x81, 0xa1, 0xc1]),
        );
        assert_eq!(harness.app.palette, pinned.with_readable_labels());
    }
}
