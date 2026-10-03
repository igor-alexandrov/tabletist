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
fn a_connections_chip_takes_its_environments_colour() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            let tab = harness.connect_fake();
            harness.app.workspace_mut(tab).unwrap().environment = env;
            // A second connection in another environment shows, so the
            // colours found are the first chip's.
            let other = if env == Environment::None {
                Environment::Dev
            } else {
                Environment::None
            };
            harness.press(Key::O, Modifiers::COMMAND);
            let second = harness.connect_fake();
            harness.app.workspace_mut(second).unwrap().environment = other;
            harness.settle();
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            // The dot: small and square, where the stripe is neither.
            let dot = |rect: egui::Rect| rect.width() == rect.height() && rect.width() < 16.0;
            // Behind the other: a dot on macOS, an outlined tag in the
            // terminal.
            if look.terminal {
                assert!(harness.strokes.contains(&colors.base()), "outline, {case}");
                assert_eq!(
                    harness.painted_color(label(&harness, env)),
                    Some(colors.base()),
                    "outlined tag, {case}"
                );
            } else {
                assert!(filled(&harness, colors.base(), dot), "behind, {case}");
            }
            // Showing: the dot again, or the solid tag.
            harness.app.apply(Action::ActivateConnTab(tab));
            harness.settle();
            if look.terminal {
                assert!(filled(&harness, colors.badge_bg(), |_| true), "tag, {case}");
                assert_eq!(
                    harness.painted_color(label(&harness, env)),
                    Some(colors.badge_fg()),
                    "tag text, {case}"
                );
            } else {
                assert!(filled(&harness, colors.base(), dot), "showing, {case}");
            }
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

/// The environment as the connection dialog's choices name it, in the
/// look's case. (A badge says it its own way: `label`.)
fn dialog_name(harness: &Harness, env: Environment) -> String {
    let names = ["Local", "Dev", "Staging", "Production", "None"];
    let at = Environment::ALL
        .iter()
        .position(|listed| *listed == env)
        .expect("a listed environment");
    harness.app.look.label(names[at])
}

#[test]
fn the_connection_dialog_takes_the_chosen_environments_colours() {
    for (look, palette) in setups() {
        for env in Environment::ALL {
            let mut harness = harness(look, palette);
            harness.app.apply(Action::NewConnection);
            if let Some(Dialog::Connection(form)) = &mut harness.app.dialog {
                // Named, so the header's line about the connection is not
                // the environment's name alone.
                form.name = "Bookshop".into();
                form.environment = Some(env);
                // The box follows the environment: set, it is filled in
                // each one's colour.
                form.read_only = Some(true);
            }
            let tree = harness.finish_animations();
            let colors = colors(&harness, env);
            let case = format!("{} dark={} {env:?}", look.name, palette.dark);
            // The dialog's title: a label, where the window behind has a
            // button of the same name.
            let title = crate::testing::bounds(
                &tree,
                &look.label("New connection"),
                egui::accesskit::Role::Label,
            )
            .expect("the dialog's title");
            assert_eq!(
                harness.painted_color(&dialog_name(&harness, env)),
                Some(colors.badge_fg()),
                "the chosen environment's name, {case}"
            );
            if look.terminal {
                // The border, the chosen button, and the title's line.
                assert!(harness.strokes.contains(&colors.base()), "border, {case}");
                assert!(
                    filled(&harness, colors.badge_bg(), |_| true),
                    "chosen button, {case}"
                );
                let title_line = |rect: egui::Rect| rect.contains_rect(title);
                assert!(
                    filled(&harness, colors.bar_bg(), title_line),
                    "title line, {case}"
                );
            } else {
                // The dot inside the chosen segment: the stripe and the
                // read-only note's box take the same colour elsewhere.
                let segment = crate::testing::bounds(
                    &tree,
                    &dialog_name(&harness, env),
                    egui::accesskit::Role::RadioButton,
                )
                .expect("the chosen segment");
                let in_segment = |rect: egui::Rect| segment.contains_rect(rect);
                assert!(filled(&harness, colors.base(), in_segment), "dot, {case}");
                // The read-only note's tint, and its box inside it.
                let boxed = |note: egui::Rect| {
                    filled(&harness, colors.base(), |rect| note.contains_rect(rect))
                };
                assert!(
                    filled(&harness, colors.bar_bg(), boxed),
                    "read-only note, {case}"
                );
                // The stripe runs over the header, from before the title to
                // past Close. No environment, no stripe.
                let close = crate::testing::bounds(&tree, "Close", egui::accesskit::Role::Button)
                    .expect("Close");
                let stripe = |rect: egui::Rect| {
                    rect.left() <= title.left()
                        && rect.right() >= close.right()
                        && rect.bottom() <= title.top()
                };
                assert_eq!(
                    filled(&harness, colors.base(), stripe),
                    env != Environment::None,
                    "stripe, {case}"
                );
            }
        }
    }
}
