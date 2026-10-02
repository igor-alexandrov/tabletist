//! The connection dialog's choices of one among several: the driver, the
//! environment, and the segments and words either layout draws them as.

use egui::{
    Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::Driver;

use crate::env::{EnvColors, Environment};
use crate::model::{Action, ConnectionForm};
use crate::theme;
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

use super::Skin;

const DRIVERS: [Driver; 3] = [Driver::Sqlite, Driver::Postgres, Driver::MySql];

/// One choice of a [`radio_group`], as it is drawn.
#[derive(Clone, Copy)]
pub(super) struct Choice<'a> {
    label: &'a str,
    /// An environment's colours, when the choice is one.
    tint: Option<EnvColors>,
}

impl<'a> Choice<'a> {
    pub(super) fn plain(label: &'a str) -> Self {
        Self { label, tint: None }
    }
}

/// How a [`radio_group`] draws.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Group {
    /// macOS: a sunken track, the chosen segment raised off it. Segments
    /// are this tall.
    Track(f32),
    /// The terminal look's environments: bordered buttons in each one's
    /// colour, the chosen one filled.
    Buttons,
    /// The terminal look's other choices: words in a line, the chosen one
    /// boxed in the accent.
    Words,
}

impl Group {
    /// How tall the choices are.
    pub(super) fn height(self) -> f32 {
        match self {
            Self::Track(height) => height,
            Self::Buttons => 26.0,
            // A line of the look's text (17), 2 above and below it, and
            // the 1 pt box round the chosen word.
            Self::Words => 23.0,
        }
    }
}

/// The dot before a chosen environment, and the gap after it.
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 6.0;

/// One of several values chosen by a [`radio_group`]: the value clicked.
/// `name` is the heading the choices stand under; with it, screen readers
/// hear them as one group of that name.
pub(super) fn choose<T: Copy + PartialEq>(
    ui: &mut Ui,
    salt: &str,
    name: Option<&str>,
    options: &[(T, Choice<'_>)],
    current: T,
    group: Group,
    skin: &Skin,
) -> Option<T> {
    let choices: Vec<Choice<'_>> = options.iter().map(|(_, choice)| *choice).collect();
    let chosen = options
        .iter()
        .position(|(value, _)| *value == current)
        .unwrap_or(0);
    let clicked = match name {
        None => radio_group(ui, salt, &choices, chosen, group, skin),
        // The choices are drawn in a Ui of their own, whose node is their
        // parent in the accessibility tree: a radio group with the name.
        Some(name) => {
            ui.scope_builder(egui::UiBuilder::new().id_salt(salt), |ui| {
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::RadioGroup);
                    node.set_label(name);
                });
                radio_group(ui, salt, &choices, chosen, group, skin)
            })
            .inner
        }
    };
    clicked.map(|index| options[index].0)
}

/// Choices of which one is chosen; returns the one clicked. Each is
/// announced as a radio button by its own text.
fn radio_group(
    ui: &mut Ui,
    salt: &str,
    choices: &[Choice<'_>],
    chosen: usize,
    group: Group,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    let idle_role = widgets::body(look);
    let chosen_role = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
    // A button's padding is its border and 10 more.
    let (pad, gap, inset) = match group {
        Group::Track(_) => (12.0, 0.0, 3.0),
        Group::Buttons => (11.0, 6.0, 0.0),
        Group::Words => (8.0, 6.0, 0.0),
    };
    let height = group.height();
    let role = |index: usize| {
        if index == chosen && group != Group::Words {
            chosen_role
        } else {
            idle_role
        }
    };
    let dotted =
        |index: usize| index == chosen && group != Group::Words && choices[index].tint.is_some();
    let widths: Vec<f32> = choices
        .iter()
        .enumerate()
        .map(|(index, choice)| {
            let text = role(index).width(ui.ctx(), look.faces, choice.label);
            let dot = if dotted(index) { DOT + DOT_GAP } else { 0.0 };
            (text + dot + 2.0 * pad).ceil()
        })
        .collect();
    let size = vec2(
        widths.iter().sum::<f32>() + gap * choices.len().saturating_sub(1) as f32 + 2.0 * inset,
        height + 2.0 * inset,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let enabled = ui.is_enabled();
    let painter = ui.painter().clone();
    if let Group::Track(_) = group {
        painter.rect_filled(rect, CornerRadius::same(look.radius), palette.surface);
    }
    let corner = CornerRadius::same(match group {
        Group::Track(_) => look.radius.saturating_sub(2),
        Group::Buttons | Group::Words => 3,
    });
    let mut clicked = None;
    let mut left = rect.left() + inset;
    for (index, choice) in choices.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(left, rect.top() + inset), vec2(widths[index], height));
        left += widths[index] + gap;
        let active = index == chosen;
        let response = ui.interact(cell, ui.id().with((salt, index)), Sense::click());
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, enabled, active, choice.label)
        });
        if response.clicked() {
            clicked = Some(index);
        }
        let hovered = response.hovered() && enabled;
        let tint = choice.tint.as_ref();
        let color = match group {
            Group::Track(_) => {
                if active {
                    painter.add(
                        egui::epaint::Shadow {
                            offset: [0, 1],
                            blur: 2,
                            spread: 0,
                            color: Color32::from_black_alpha(20),
                        }
                        .as_shape(cell, corner),
                    );
                    painter.rect_filled(cell, corner, widgets::raised_fill(palette));
                    tint.map_or(palette.text, |colors| skin.env_ink(colors))
                } else if hovered {
                    palette.text
                } else {
                    palette.secondary
                }
            }
            Group::Buttons => {
                if active {
                    let fill = tint.map_or(palette.text, EnvColors::badge_bg);
                    painter.rect_filled(cell, corner, fill);
                    // The badge's text, unless the dialog's own reads
                    // better on the fill: a light theme's pale environment
                    // takes the text colour.
                    let ink = tint.map_or(palette.panel, EnvColors::badge_fg);
                    if theme::contrast(ink, fill) >= theme::contrast(palette.text, fill) {
                        ink
                    } else {
                        palette.text
                    }
                } else {
                    let ink = tint.map_or(palette.text, |colors| skin.env_ink(colors));
                    if hovered {
                        painter.rect_filled(cell, corner, palette.text.gamma_multiply(0.08));
                    }
                    painter.rect_stroke(
                        cell,
                        corner,
                        Stroke::new(1.0, palette.outline),
                        StrokeKind::Inside,
                    );
                    ink
                }
            }
            Group::Words => {
                if active {
                    painter.rect_stroke(
                        cell,
                        corner,
                        Stroke::new(1.0, palette.accent),
                        StrokeKind::Inside,
                    );
                    palette.accent
                } else if hovered {
                    palette.text
                } else {
                    palette.dim
                }
            }
        };
        crate::ui::focus::hint(
            ui,
            &response,
            cell,
            crate::ui::focus::Ring::Outer { radius: corner.nw },
        );
        let mut x = cell.left() + pad;
        if dotted(index) {
            let dot = match (group, tint) {
                (Group::Track(_), Some(colors)) => colors.base(),
                _ => color,
            };
            painter.circle_filled(pos2(x + DOT / 2.0, cell.center().y), DOT / 2.0, dot);
            x += DOT + DOT_GAP;
        }
        Text::one(look, role(index), choice.label, color)
            .layout(ui.ctx())
            .paint_left(&painter, x, cell.center().y);
    }
    clicked
}

/// The driver choice: SQLite, PostgreSQL or MySQL, under `heading`.
pub(super) fn driver_choice(
    ui: &mut Ui,
    form: &ConnectionForm,
    heading: &str,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let labels = DRIVERS.map(|driver| skin.look.label(driver.label()));
    let options: Vec<(Driver, Choice<'_>)> = DRIVERS
        .iter()
        .zip(&labels)
        .map(|(driver, label)| (*driver, Choice::plain(label)))
        .collect();
    let group = if skin.look.terminal {
        Group::Words
    } else {
        Group::Track(28.0)
    };
    let clicked = choose(
        ui,
        "driver",
        Some(heading),
        &options,
        form.driver,
        group,
        skin,
    );
    if let Some(driver) = clicked
        && driver != form.driver
    {
        actions.push(Action::SetDriver(driver));
    }
}

/// The environments as the dialog names them. A badge says them its own
/// way (`Environment::label`). The names are tied to [`Environment::ALL`]
/// by position, not by a `match`: src/env.rs keeps every match on an
/// environment to itself. The array's length follows `ALL`'s, and a
/// reorder is caught by the tests that name each environment: the env
/// test of the dialog's colours and the dialog's own test of its choices.
const ENVIRONMENT_NAMES: [&str; Environment::ALL.len()] =
    ["Local", "Dev", "Staging", "Production", "None"];

/// The environment choice, under `heading`. It is what the badges say
/// (PROD), and it gives the connection its colour. The one shown as chosen
/// follows where the connection points until one is clicked.
pub(super) fn environment_choice(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    heading: &str,
    skin: &Skin,
) {
    let labels = ENVIRONMENT_NAMES.map(|name| skin.say(name));
    let options: Vec<(Environment, Choice<'_>)> = Environment::ALL
        .iter()
        .zip(&labels)
        .map(|(environment, label)| {
            let choice = Choice {
                label,
                tint: Some(skin.colors(*environment)),
            };
            (*environment, choice)
        })
        .collect();
    let group = if skin.look.terminal {
        Group::Buttons
    } else {
        Group::Track(28.0)
    };
    let clicked = choose(
        ui,
        "environment",
        Some(heading),
        &options,
        skin.environment,
        group,
        skin,
    );
    if let Some(environment) = clicked {
        form.environment = Some(environment);
    }
}
