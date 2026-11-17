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

const DRIVERS: [Driver; 5] = [
    Driver::Sqlite,
    Driver::Postgres,
    Driver::MySql,
    Driver::BigQuery,
    Driver::Files,
];

/// One choice of a [`radio_group`], as it is drawn.
#[derive(Clone, Copy)]
pub(super) struct Choice<'a> {
    label: &'a str,
    /// An environment's colours, when the choice is one.
    tint: Option<EnvColors>,
    /// Why the choice cannot be made, where it cannot: it is still a Tab
    /// stop, which says why to the pointer, the keyboard and a screen
    /// reader.
    off: Option<&'a str>,
}

impl<'a> Choice<'a> {
    pub(super) fn plain(label: &'a str) -> Self {
        Self {
            label,
            tint: None,
            off: None,
        }
    }

    /// A choice that cannot be made, for `why`.
    pub(super) fn off(label: &'a str, why: &'a str) -> Self {
        Self {
            off: Some(why),
            ..Self::plain(label)
        }
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
    /// Radio buttons in rows across the width given (see [`radio_grid`]):
    /// the type, which grows past what a line holds.
    Grid,
}

impl Group {
    /// How tall the choices are. A grid's height depends on how many
    /// choices there are and the width: [`GridShape::height`].
    pub(super) fn height(self) -> f32 {
        match self {
            Self::Track(height) => height,
            Self::Buttons => 26.0,
            // A line of the look's text (17), 2 above and below it, and
            // the 1 pt box round the chosen word.
            Self::Words => 23.0,
            Self::Grid => 28.0,
        }
    }
}

/// The dot before a chosen environment, and the gap after it.
const DOT: f32 = 8.0;
const DOT_GAP: f32 = 6.0;

/// How a [`radio_grid`] lays out in a look: at most `columns` to a row,
/// none narrower than `min`, `gap` apart; rows `row` tall, `row_gap` apart.
struct GridShape {
    columns: usize,
    min: f32,
    gap: f32,
    row: f32,
    row_gap: f32,
}

/// macOS: the Components Radio, four to a row in the dialog's 660.
const MAC_GRID: GridShape = GridShape {
    columns: 4,
    min: 150.0,
    gap: 12.0,
    row: 28.0,
    row_gap: 2.0,
};

/// The terminal look: `(•) name`, three to a row in the value column's 576.
const TERMINAL_GRID: GridShape = GridShape {
    columns: 3,
    min: 170.0,
    gap: 14.0,
    row: 23.0,
    row_gap: 0.0,
};

impl GridShape {
    fn of(skin: &Skin) -> &'static GridShape {
        if skin.look.terminal {
            &TERMINAL_GRID
        } else {
            &MAC_GRID
        }
    }

    /// The columns `width` holds: fewer in a dialog a narrow window
    /// squeezes.
    fn columns(&self, width: f32) -> usize {
        (((width + self.gap) / (self.min + self.gap)).floor() as usize).clamp(1, self.columns)
    }

    /// How tall `count` choices stand across `width`.
    fn height(&self, count: usize, width: f32) -> f32 {
        let rows = count.div_ceil(self.columns(width)).max(1);
        rows as f32 * self.row + (rows - 1) as f32 * self.row_gap
    }
}

/// How tall the type's grid stands across `width`: the terminal row it
/// sits on is as tall.
pub(super) fn driver_height(width: f32, skin: &Skin) -> f32 {
    GridShape::of(skin).height(DRIVERS.len(), width)
}

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
    let draw = |ui: &mut Ui| match group {
        Group::Grid => radio_grid(ui, salt, &choices, chosen, skin),
        Group::Track(_) | Group::Buttons | Group::Words => {
            radio_group(ui, salt, &choices, chosen, group, skin)
        }
    };
    let clicked = match name {
        None => draw(ui),
        // The choices are drawn in a Ui of their own, whose node is their
        // parent in the accessibility tree: a radio group with the name.
        Some(name) => {
            ui.scope_builder(egui::UiBuilder::new().id_salt(salt), |ui| {
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::RadioGroup);
                    node.set_label(name);
                });
                draw(ui)
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
        Group::Grid => unreachable!("a grid is drawn by radio_grid"),
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
        Group::Grid => unreachable!("a grid is drawn by radio_grid"),
    });
    let mut clicked = None;
    let mut left = rect.left() + inset;
    for (index, choice) in choices.iter().enumerate() {
        let cell = Rect::from_min_size(pos2(left, rect.top() + inset), vec2(widths[index], height));
        left += widths[index] + gap;
        let active = index == chosen;
        let sense = match choice.off {
            Some(_) => Sense::focusable_noninteractive() | Sense::hover(),
            None => Sense::click(),
        };
        let mut response = ui.interact(cell, ui.id().with((salt, index)), sense);
        let can = enabled && choice.off.is_none();
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, can, active, choice.label)
        });
        if let Some(why) = choice.off {
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_description(why);
            });
            if crate::ui::focus::shown(&response) {
                response.show_tooltip_text(why);
            }
            response = response.on_hover_text(why);
        }
        if response.clicked() && choice.off.is_none() {
            clicked = Some(index);
        }
        let hovered = response.hovered() && can;
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
                } else if choice.off.is_some() {
                    palette.dim
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
            Group::Grid => unreachable!("a grid is drawn by radio_grid"),
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

/// Choices as radio buttons in rows across the width the Ui has; returns
/// the one clicked or, on macOS, the one the arrows chose. Each is
/// announced as a radio button by its own text. On macOS the chosen one is
/// the grid's only Tab stop and the arrows move in the grid; in the
/// terminal look each is a Tab stop that Enter or Space picks.
fn radio_grid(
    ui: &mut Ui,
    salt: &str,
    choices: &[Choice<'_>],
    chosen: usize,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    let shape = GridShape::of(skin);
    let width = ui.available_width();
    let columns = shape.columns(width);
    let cell_width = (width - shape.gap * (columns - 1) as f32) / columns as f32;
    let size = vec2(width, shape.height(choices.len(), width));
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter().clone();
    let enabled = ui.is_enabled();
    let group = ui.id().with((salt, "grid"));
    let mut clicked = None;
    for (index, choice) in choices.iter().enumerate() {
        let (row, column) = (index / columns, index % columns);
        let cell = Rect::from_min_size(
            pos2(
                rect.left() + column as f32 * (cell_width + shape.gap),
                rect.top() + row as f32 * (shape.row + shape.row_gap),
            ),
            vec2(cell_width, shape.row),
        );
        let active = index == chosen;
        let id = ui.id().with((salt, index));
        let (response, arrow) = if look.terminal {
            (ui.interact(cell, id, Sense::click()), None)
        } else {
            crate::ui::focus::grid_segment(
                ui,
                cell,
                id,
                group,
                (index, choices.len(), columns),
                active,
            )
        };
        let can = enabled && choice.off.is_none();
        response.widget_info(|| {
            WidgetInfo::selected(WidgetType::RadioButton, can, active, choice.label)
        });
        if can && response.clicked() {
            clicked = Some(index);
        }
        if can {
            clicked = arrow.or(clicked);
        }
        let center = cell.center().y;
        if look.terminal {
            // The Components Radio: (•) in the accent, ( ) muted; the
            // keyboard's cursor is the selection's tone and the ▌ bar.
            let cursor = crate::ui::focus::shown(&response);
            if cursor {
                painter.rect_filled(cell, CornerRadius::ZERO, palette.selection);
            }
            let (mark, mark_ink) = if active {
                ("(•) ", palette.accent)
            } else {
                ("( ) ", palette.dim)
            };
            let name_ink = if active || (can && response.hovered()) {
                palette.text
            } else {
                palette.dim
            };
            let role = TextRole::OBody;
            let x = cell.left() + 8.0;
            let x = x + Text::one(look, role, mark, mark_ink)
                .layout(ui.ctx())
                .paint_left(&painter, x, center);
            Text::one(look, role, choice.label, name_ink)
                .layout(ui.ctx())
                .paint_left(&painter, x, center);
            crate::ui::focus::hint(ui, &response, cell, crate::ui::focus::Ring::Bar);
        } else {
            // The Components Radio: a 16 circle, the chosen one filled in
            // the accent round a dot.
            let mark = Rect::from_center_size(pos2(cell.left() + 8.0, center), vec2(16.0, 16.0));
            if active {
                painter.circle_filled(mark.center(), 8.0, palette.accent);
                painter.circle_filled(mark.center(), 3.0, palette.on_accent);
            } else {
                let edge = if can && response.hovered() {
                    palette.border.lerp_to_gamma(palette.text, 0.2)
                } else {
                    palette.border
                };
                painter.circle_filled(mark.center(), 8.0, palette.window);
                painter.circle_stroke(mark.center(), 7.5, Stroke::new(1.0, edge));
            }
            crate::ui::focus::hint(
                ui,
                &response,
                mark,
                crate::ui::focus::Ring::Outer { radius: 8 },
            );
            let role = if active {
                TextRole::UiBodyStrong
            } else {
                TextRole::UiBody
            };
            let ink = if can { palette.text } else { palette.dim };
            Text::one(look, role, choice.label, ink)
                .layout(ui.ctx())
                .paint_left(&painter, cell.left() + 24.0, center);
        }
    }
    clicked
}

/// The driver choice: every type as a radio button in a grid, under
/// `heading`.
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
    let clicked = choose(
        ui,
        "driver",
        Some(heading),
        &options,
        form.driver,
        Group::Grid,
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
/// by position, not by a `match`: `crate::env` keeps every match on an
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
    heading: &str,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let labels = ENVIRONMENT_NAMES.map(|name| skin.say(name));
    let options: Vec<(Environment, Choice<'_>)> = Environment::ALL
        .iter()
        .zip(&labels)
        .map(|(environment, label)| {
            let choice = Choice {
                label,
                tint: Some(skin.colors(*environment)),
                off: None,
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
        actions.push(Action::SetEnvironment(environment));
    }
}
