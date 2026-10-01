//! Every surface that shows a connection's environment paints the colours
//! `env_colors()` gives it, for every environment, on macOS (light and dark)
//! and Omarchy. These check the colours a frame paints, never pixels.

use egui::{Key, Modifiers};

use crate::env::{EnvColors, Environment, Platform, env_colors};
use crate::model::{Action, Dialog};
use crate::testing::Harness;
use crate::theme::{Look, Palette};

/// macOS light, macOS dark, and Omarchy (whose theme is the palette).
fn setups() -> [(Look, Palette); 3] {
    [
        (Look::macos(), Palette::light()),
        (Look::macos(), Palette::dark()),
        (Look::omarchy(), Palette::dark()),
    ]
}

fn harness(look: Look, palette: Palette) -> Harness {
    let mut harness = Harness::new();
    harness.app.palette = palette;
    harness.set_look(look);
    harness
}

fn colors(harness: &Harness, env: Environment) -> EnvColors {
    env_colors(env, Platform::of(&harness.app.look), &harness.app.palette)
}

fn label(harness: &Harness, env: Environment) -> &'static str {
    env.label(Platform::of(&harness.app.look))
}

fn filled(harness: &Harness, color: egui::Color32, fits: impl Fn(egui::Rect) -> bool) -> bool {
    harness
        .fills
        .iter()
        .any(|(rect, fill)| *fill == color && fits(*rect))
}

#[test]
fn the_connection_bar_takes_its_environments_colours() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().environment = env;
            harness.settle();
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            assert!(filled(&harness, colors.bar_bg(), |_| true), "tint, {case}");
            assert!(
                harness.strokes.contains(&colors.bar_border()),
                "rule, {case}"
            );
            if look.terminal {
                assert!(filled(&harness, colors.badge_bg(), |_| true), "{case}");
                assert_eq!(
                    harness.painted_color(label(&harness, env)),
                    Some(colors.badge_fg()),
                    "badge, {case}"
                );
            } else {
                let stripe = |rect: egui::Rect| rect.height() == 3.0 && rect.top() == 0.0;
                assert!(filled(&harness, colors.base(), stripe), "stripe, {case}");
            }
        }
    }
}

#[test]
fn a_connected_tabs_dot_is_its_environments_colour() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().environment = env;
            let dot = |rect: egui::Rect| rect.width() == 7.0;
            let base = colors(&harness, env).base();
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            // Inactive, behind a new tab; then active.
            harness.press(Key::O, Modifiers::COMMAND);
            assert!(filled(&harness, base, dot), "inactive, {case}");
            harness.app.apply(Action::ActivateConnTab(tab));
            harness.settle();
            assert!(filled(&harness, base, dot), "active, {case}");
        }
    }
}

#[test]
fn the_connections_list_shows_each_environment_in_its_colours() {
    for (look, palette) in setups() {
        let mut harness = harness(look, palette);
        for env in Environment::ALL {
            harness
                .app
                .connections
                .upsert(crate::connections::SavedConnection {
                    id: crate::connections::ConnectionId::new(),
                    name: format!("Bookshop {env:?}"),
                    environment: env,
                    read_only: None,
                    password: crate::connections::PasswordMode::None,
                    ssh_secret: crate::connections::PasswordMode::None,
                    spec: tabletist_db::ConnectSpec::sqlite(format!("/tmp/bookshop-{env:?}.db")),
                });
        }
        harness.settle();
        for env in Environment::ALL {
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            assert_eq!(
                harness.painted_color(label(&harness, env)),
                Some(colors.badge_fg()),
                "badge text, {case}"
            );
            assert!(
                filled(&harness, colors.badge_bg(), |_| true),
                "badge, {case}"
            );
            if !look.terminal {
                let stripe = |rect: egui::Rect| rect.width() == 6.0;
                assert!(filled(&harness, colors.base(), stripe), "stripe, {case}");
            }
        }
    }
}

#[test]
fn the_connection_dialog_takes_the_chosen_environments_colours() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            harness.app.apply(Action::NewConnection);
            if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
                form.environment = Some(env);
            }
            harness.finish_animations();
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            assert_eq!(
                harness.painted_color(label(&harness, env)),
                Some(colors.badge_fg()),
                "chosen segment, {case}"
            );
            assert!(filled(&harness, colors.badge_bg(), |_| true), "{case}");
            if look.terminal {
                assert!(harness.strokes.contains(&colors.base()), "border, {case}");
            } else {
                assert!(filled(&harness, colors.base(), |_| true), "stripe, {case}");
            }
        }
    }
}
