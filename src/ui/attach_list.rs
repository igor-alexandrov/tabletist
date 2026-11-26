//! The list of saved connections a Files connection can attach, a row
//! each: a box to tick, the connection's environment, its name and
//! database, and, once ticked, the name SQL calls it by, which can be
//! typed over. A row that cannot be ticked says why. macOS draws each row
//! as the boards' attach row (the environment's stripe and badge, the
//! engine and database under the name); the terminal look as one line,
//! `[x] DEV Bookshop as bookshop_dev`.

use egui::{CornerRadius, Rect, Sense, Ui, pos2, vec2};

use crate::env::{Environment, Platform, env_colors};
use crate::i18n::{Locale, gettext};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::workspace::{Badge, env_badge};
use crate::ui::{focus, widgets};

/// One saved connection, as its row says it.
pub struct Row<'a> {
    pub name: &'a str,
    pub environment: Environment,
    /// Its engine's name: "PostgreSQL".
    pub engine: &'a str,
    pub database: &'a str,
    /// It opens read-only, so it is attached so.
    pub read_only: bool,
    /// Why it cannot be ticked, in the look's words.
    pub blocked: Option<String>,
    /// The name SQL calls it by, where it is ticked.
    pub attached: Option<&'a mut String>,
}

/// The id of the box of the row at `at`: the keyboard's stop on it.
pub fn toggle_id(at: usize) -> egui::Id {
    egui::Id::new(("attach-toggle", at))
}

/// The id of the name field of the row at `at`.
pub fn name_id(at: usize) -> egui::Id {
    egui::Id::new(("attach-name", at))
}

/// The row whose box has the keyboard, among `rows` rows.
pub fn focused(ctx: &egui::Context, rows: usize) -> Option<usize> {
    let focused = ctx.memory(|memory| memory.focused())?;
    (0..rows).find(|at| toggle_id(*at) == focused)
}

/// The rows, under one another; the row whose box was ticked or unticked
/// this frame, if one was. Announced as a list named `name`.
pub fn show(
    ui: &mut Ui,
    name: &str,
    rows: &mut [Row<'_>],
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Option<usize> {
    let mut toggled = None;
    // Under one another, whatever the layout of the cell it is put in.
    let builder = egui::UiBuilder::new()
        .id_salt(name)
        .layout(egui::Layout::top_down(egui::Align::Min));
    ui.scope_builder(builder, |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::List);
            node.set_label(name);
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        let framed = !look.terminal;
        let frame = egui::Frame::new()
            .stroke(match framed {
                true => egui::Stroke::new(1.0, palette.border),
                false => egui::Stroke::NONE,
            })
            .corner_radius(CornerRadius::same(if framed { 8 } else { 0 }));
        frame.show(ui, |ui| {
            for (at, row) in rows.iter_mut().enumerate() {
                if at > 0 && framed {
                    let y = ui.cursor().top();
                    widgets::hline(ui, ui.max_rect().x_range(), y, palette.surface_hover);
                }
                let pressed = match look.terminal {
                    true => line(ui, at, row, look, palette, locale),
                    false => sheet_row(ui, at, row, look, palette, locale),
                };
                if pressed {
                    toggled = Some(at);
                }
            }
        });
    });
    toggled
}

/// The words "Attach Bookshop dev": what the box is named for a screen
/// reader, the environment after the name, as the boards write it.
fn named(row: &Row<'_>, locale: Locale) -> String {
    // One pass: a name that reads "{environment}" stays as it is.
    let words = gettext(locale, "Attach {name} {environment}");
    let environment = row.environment.label(Platform::Native);
    let (before, after) = words.split_once("{name}").unwrap_or((&words, ""));
    let put = |part: &str| part.replace("{environment}", environment);
    format!("{}{}{}", put(before), row.name, put(after))
}

/// The box, as a stop of its own: whether a press of it (the pointer,
/// Space or Enter) ticked or unticked it this frame.
fn toggle(
    ui: &mut Ui,
    at: usize,
    rect: Rect,
    row: &Row<'_>,
    locale: Locale,
) -> (egui::Response, bool) {
    // One that cannot be ticked still takes the keyboard, so the arrows
    // pass through it and a screen reader says why. One ticked before it
    // could not be still unticks.
    let checked = row.attached.is_some();
    let enabled = row.blocked.is_none() || checked;
    let response = ui.interact(
        rect,
        toggle_id(at),
        if enabled {
            Sense::click()
        } else {
            Sense::focusable_noninteractive()
        },
    );
    // The arrows are the list's while a box has the keyboard (the form
    // moves it to the next row's), not egui's to move it to the next stop.
    if response.has_focus() {
        let filter = egui::EventFilter {
            vertical_arrows: true,
            ..Default::default()
        };
        ui.ctx()
            .memory_mut(|memory| memory.set_focus_lock_filter(response.id, filter));
    }
    let label = named(row, locale);
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Checkbox, enabled, checked, &label)
    });
    let pressed = enabled && crate::ui::disclosure::pressed(ui, &response);
    (response, pressed)
}

/// macOS: the board's attach row, 40 tall.
fn sheet_row(
    ui: &mut Ui,
    at: usize,
    row: &mut Row<'_>,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) -> bool {
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, 40.0), Sense::hover());
    let center = rect.center().y;
    let colors = env_colors(row.environment, Platform::Native, palette);
    let enabled = row.blocked.is_none();
    let faded = |color: egui::Color32| match enabled {
        true => color,
        false => color.gamma_multiply(0.5),
    };
    // The box.
    let box_rect = Rect::from_center_size(pos2(rect.left() + 17.0, center), vec2(14.0, 14.0));
    let (response, pressed) = toggle(ui, at, box_rect.expand(4.0), row, locale);
    let checked = row.attached.is_some();
    let painter = ui.painter().clone();
    painter.rect(
        box_rect,
        CornerRadius::same(look.radius.min(4)),
        match checked {
            true => faded(palette.accent),
            false => palette.panel,
        },
        egui::Stroke::new(1.0, faded(palette.border)),
        egui::StrokeKind::Inside,
    );
    if checked {
        let stroke = egui::Stroke::new(1.6, widgets::ink_on(palette.accent, palette));
        let (l, c) = (box_rect.left(), box_rect.center());
        painter.line_segment([pos2(l + 3.0, c.y), pos2(c.x - 1.0, c.y + 3.5)], stroke);
        painter.line_segment(
            [
                pos2(c.x - 1.0, c.y + 3.5),
                pos2(box_rect.right() - 3.0, c.y - 3.5),
            ],
            stroke,
        );
    }
    focus::hint(ui, &response, box_rect, focus::Ring::Outer { radius: 4 });
    // The environment's stripe.
    let stripe = Rect::from_center_size(pos2(rect.left() + 37.0, center), vec2(6.0, 24.0));
    painter.rect_filled(stripe, CornerRadius::same(3), faded(colors.base()));
    // The name, the badge and read-only; the engine and database under.
    let left = stripe.right() + 10.0;
    let top = center - 8.0;
    let mut x = left;
    x += widgets::paint_label(
        ui,
        x,
        top,
        Text::one(
            look,
            TextRole::UiBodySemibold,
            row.name,
            faded(palette.text),
        ),
    );
    x += 8.0;
    x += env_badge(ui, x, top, row.environment, &colors, Badge::Mac, look) + 8.0;
    if row.read_only {
        widgets::paint_label(
            ui,
            x,
            top,
            Text::one(
                look,
                TextRole::Secondary,
                &gettext(locale, "read-only"),
                palette.dim,
            ),
        );
    }
    let under = format!("{} · {}", row.engine, row.database);
    widgets::paint_label(
        ui,
        left,
        center + 9.0,
        Text::one(look, TextRole::Secondary, &under, faded(palette.dim)),
    );
    // On the right: the name SQL calls it by, or why it cannot be ticked.
    let right = Rect::from_min_max(pos2(rect.right() - 210.0, rect.top()), rect.max)
        .shrink2(vec2(10.0, 6.0));
    match (&mut row.attached, &row.blocked) {
        (Some(sql), _) => {
            name_field(ui, at, right, sql, look, locale, TextRole::Code);
        }
        (None, Some(why)) => right_label(
            ui,
            right.right(),
            center,
            why,
            TextRole::Secondary,
            look,
            palette,
        ),
        (None, None) => {
            let words = gettext(locale, "not attached");
            right_label(
                ui,
                right.right(),
                center,
                &words,
                TextRole::MonoSecondary,
                look,
                palette,
            );
        }
    }
    pressed
}

/// `text` ending at `right`, centred on `y`, in the faint colour: said to
/// screen readers too.
fn right_label(
    ui: &Ui,
    right: f32,
    y: f32,
    text: &str,
    role: TextRole,
    look: &Look,
    palette: &Palette,
) {
    let width = role.width(ui.ctx(), look.faces, text);
    widgets::paint_label(
        ui,
        right - width,
        y,
        Text::one(look, role, text, palette.faint),
    );
}

/// The terminal look: `[x] DEV Bookshop as bookshop_dev · read-only`.
fn line(
    ui: &mut Ui,
    at: usize,
    row: &mut Row<'_>,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) -> bool {
    let role = TextRole::OBody;
    let height = 28.0;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    let center = rect.center().y;
    let colors = env_colors(row.environment, Platform::Omarchy, palette);
    let checked = row.attached.is_some();
    let enabled = row.blocked.is_none();
    let mark = if checked { "[x]" } else { "[ ]" };
    let mark_width = role.width(ui.ctx(), look.faces, mark);
    let mark_rect = Rect::from_min_size(
        pos2(rect.left(), center - height / 2.0),
        vec2(mark_width + 4.0, height),
    );
    let (response, pressed) = toggle(ui, at, mark_rect, row, locale);
    let painter = ui.painter().clone();
    if focus::shown(&response) {
        painter.rect_filled(mark_rect, CornerRadius::ZERO, palette.selection);
    }
    let mut x = rect.left() + 2.0;
    x += widgets::paint_label(
        ui,
        x,
        center,
        Text::one(
            look,
            role,
            mark,
            match (checked, enabled) {
                (true, _) => palette.success,
                (false, true) => palette.dim,
                (false, false) => palette.faint,
            },
        ),
    ) + 10.0;
    x += env_badge(ui, x, center, row.environment, &colors, Badge::Plain, look) + 10.0;
    x += widgets::paint_label(
        ui,
        x,
        center,
        Text::one(
            look,
            role,
            row.name,
            if enabled { palette.text } else { palette.faint },
        ),
    ) + 8.0;
    // Its database, dim: `· bookshop_development`.
    if !row.database.is_empty() {
        let database = format!("· {}", row.database);
        x += widgets::paint_label(ui, x, center, Text::one(look, role, &database, palette.dim))
            + 10.0;
    }
    focus::hint(ui, &response, mark_rect, focus::Ring::Bar);
    match (&mut row.attached, &row.blocked) {
        (Some(sql), _) => {
            let words = look.label(&gettext(locale, "as"));
            x += widgets::paint_label(ui, x, center, Text::one(look, role, &words, palette.dim));
            x += 6.0;
            let field = Rect::from_min_max(
                pos2(x, rect.top() + 2.0),
                pos2((x + 220.0).min(rect.right()), rect.bottom() - 2.0),
            );
            name_field(ui, at, field, sql, look, locale, role);
            if row.read_only {
                widgets::paint_label(
                    ui,
                    field.right() + 8.0,
                    center,
                    Text::one(
                        look,
                        role,
                        &look.label(&format!("· {}", gettext(locale, "read-only"))),
                        palette.dim,
                    ),
                );
            }
        }
        (None, Some(why)) => {
            widgets::paint_label(ui, x, center, Text::one(look, role, why, palette.faint));
        }
        (None, None) => {}
    }
    pressed
}

/// The field the name SQL calls a ticked connection by is typed in, in
/// `rect`.
fn name_field(
    ui: &mut Ui,
    at: usize,
    rect: Rect,
    sql: &mut String,
    look: &Look,
    locale: Locale,
    role: TextRole,
) {
    let named = gettext(locale, "SQL name");
    ui.put(
        rect,
        widgets::field(ui, sql, look, role, rect.height(), 6)
            .id(name_id(at))
            .desired_width(rect.width()),
    )
    .widget_info(|| egui::WidgetInfo::text_edit(true, "", sql.as_str(), named.as_ref()));
}
