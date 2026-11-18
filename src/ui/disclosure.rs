//! A section that opens and closes, as the Components board's disclosure
//! row draws it: closed, a row of its title and a one-line summary; open,
//! its body under a heading that closes it again.
//!
//! macOS: closed, a row 40 high in a hairline box with corners of 8: `›`,
//! the title in bold in a column 190 wide, then the summary, cut to fit.
//! Open, the box the dialog's groups are drawn in, its legend `⌄` and the
//! title, the hint the first line inside it.
//!
//! The terminal look: a heading over a rule, `▸ title  summary · enter
//! opens` closed (the key as the keymap has it) and `▾ title  hint` open, with the bar at its left while
//! it has the keyboard.
//!
//! Either is a button whose expanded state is the section's, named with
//! its title and, closed, the summary: "Reading CSV, Header detect, …".

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};

use crate::style::Icon;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets;

/// The closed row's height.
const ROW: f32 = 40.0;
/// The room at the row's sides.
const INSET: f32 = 14.0;
/// The closed row's title column.
const TITLE: f32 = 190.0;
/// The chevron's size, and the room after it.
const CHEVRON: f32 = 10.0;
const GAP: f32 = 12.0;

/// What a section says of itself.
pub struct Section<'a> {
    /// Tells the section from the others in its `Ui`.
    pub salt: &'a str,
    pub title: &'a str,
    /// What the closed row says after the title.
    pub summary: &'a str,
    /// What the open heading says after the title; may be empty.
    pub hint: &'a str,
    /// The terminal look's closed heading's last words: the key that opens
    /// it, from the keymap, and the word for what it does, as `enter
    /// opens`. The macOS row does not show it.
    pub opens: &'a str,
}

impl Section<'_> {
    /// The name a screen reader hears: the title, and closed, the summary
    /// as words after a comma.
    pub fn name(&self, open: bool) -> String {
        match open || self.summary.is_empty() {
            true => self.title.to_owned(),
            false => format!("{}, {}", self.title, self.summary.replace(" · ", ", ")),
        }
    }
}

/// The section: its closed row, or its heading and what `add` draws under
/// it. A press of the row or the heading, by the pointer or by Enter or
/// Space while it has the keyboard, opens or closes it. `fill` is what the
/// open box sits on, which its legend breaks the edge with, and `rule` the
/// colour of its edges. Returns the row's or the heading's response.
#[allow(clippy::too_many_arguments)] // the section, its state, `add`, and the look it is drawn in
pub fn show(
    ui: &mut Ui,
    section: &Section<'_>,
    open: &mut bool,
    look: &Look,
    palette: &Palette,
    fill: Color32,
    rule: Color32,
    add: impl FnOnce(&mut Ui),
) -> Response {
    if look.terminal {
        let response = terminal_heading(ui, section, *open, look, palette);
        if response.clicked() {
            *open = !*open;
        }
        if *open {
            add(ui);
        }
        return response;
    }
    if !*open {
        let response = row(ui, section, look, palette, rule);
        if response.clicked() {
            *open = true;
        }
        return response;
    }
    open_box(ui, section, open, look, palette, fill, rule, add)
}

/// Says the toggle is a button named `name`, open or closed.
fn announce(response: &Response, name: &str, open: bool) {
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, name));
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_expanded(open));
}

/// macOS: the closed row.
fn row(
    ui: &mut Ui,
    section: &Section<'_>,
    look: &Look,
    palette: &Palette,
    rule: Color32,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let id = ui.id().with(("disclosure", section.salt));
    let response = ui.interact(rect, id, Sense::click());
    announce(&response, &section.name(false), false);
    if ui.is_rect_visible(rect) {
        let painter = ui.painter();
        let ground = match response.hovered() {
            true => palette.surface_hover,
            false => Color32::TRANSPARENT,
        };
        painter.rect(
            rect,
            CornerRadius::same(8),
            ground,
            Stroke::new(widgets::hairline(ui), rule),
            StrokeKind::Inside,
        );
        let middle = rect.center().y;
        let chevron = Rect::from_center_size(
            pos2(rect.left() + INSET + CHEVRON / 2.0, middle),
            vec2(CHEVRON, CHEVRON),
        );
        Icon::ChevronRight
            .image(palette.dim, CHEVRON)
            .paint_at(ui, chevron);
        let title_x = chevron.right() + GAP;
        Text::one(look, TextRole::UiBodySemibold, section.title, palette.text)
            .layout(ui.ctx())
            .paint_left(painter, title_x, middle);
        let summary_x = title_x + TITLE + GAP;
        let room = rect.right() - INSET - summary_x;
        if room > 0.0 && !section.summary.is_empty() {
            let mut text = Text::one(look, TextRole::UiBody, section.summary, palette.secondary);
            text.job_mut().wrap = egui::text::TextWrapping::truncate_at_width(room);
            text.layout(ui.ctx()).paint_left(painter, summary_x, middle);
        }
    }
    focus::hint(ui, &response, rect, Ring::Outer { radius: 8 });
    response
}

/// macOS: the open box, its legend the toggle that closes it.
#[allow(clippy::too_many_arguments)] // `show`'s own, passed on
fn open_box(
    ui: &mut Ui,
    section: &Section<'_>,
    open: &mut bool,
    look: &Look,
    palette: &Palette,
    fill: Color32,
    rule: Color32,
    add: impl FnOnce(&mut Ui),
) -> Response {
    let legend =
        Text::one(look, TextRole::Legend, section.title, palette.secondary).layout(ui.ctx());
    let half = legend.height() / 2.0;
    ui.add_space(half);
    // The legend breaks the edge, `⌄` before its words, 6 of `fill` at
    // each side. It takes the keyboard before what the box holds, so Tab
    // goes from it into the section.
    let start = ui.cursor().min;
    let width = CHEVRON + 6.0 + legend.width();
    let gap = Rect::from_min_size(
        pos2(start.x + 15.0, start.y - half),
        vec2(width + 12.0, legend.height()),
    );
    let id = ui.id().with(("disclosure", section.salt));
    let response = ui.interact(gap, id, Sense::click());
    announce(&response, &section.name(true), true);
    let border = ui.painter().add(egui::Shape::Noop);
    let rect = egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 15,
            right: 15,
            top: (half + 15.0).round() as i8,
            bottom: 13,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if !section.hint.is_empty() {
                // The hint, under the legend, 4 above what follows.
                ui.add_space(-4.0);
                widgets::label(ui, TextRole::Secondary, section.hint, palette.dim, look);
                ui.add_space(4.0);
            }
            add(ui);
        })
        .response
        .rect;
    ui.painter().set(
        border,
        egui::epaint::RectShape::stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(widgets::hairline(ui), rule),
            StrokeKind::Inside,
        ),
    );
    let painter = ui.painter();
    painter.rect_filled(gap, CornerRadius::ZERO, fill);
    let chevron = Rect::from_center_size(
        pos2(gap.left() + 6.0 + CHEVRON / 2.0, gap.center().y),
        vec2(CHEVRON, CHEVRON),
    );
    let tint = match response.hovered() {
        true => palette.text,
        false => palette.dim,
    };
    Icon::ChevronDown.image(tint, CHEVRON).paint_at(ui, chevron);
    legend.paint(painter, pos2(chevron.right() + 6.0, gap.top()));
    focus::hint(ui, &response, gap, Ring::Outer { radius: 3 });
    if response.clicked() {
        *open = false;
    }
    response
}

/// The terminal look: the heading, closed or open.
fn terminal_heading(
    ui: &mut Ui,
    section: &Section<'_>,
    open: bool,
    look: &Look,
    palette: &Palette,
) -> Response {
    let (mark, after) = match open {
        true => ("▾ ", section.hint.to_owned()),
        false => (
            "▸ ",
            [section.summary, section.opens]
                .into_iter()
                .filter(|words| !words.is_empty())
                .collect::<Vec<_>>()
                .join(" · "),
        ),
    };
    let role = TextRole::OBody;
    let mut text = Text::new(look)
        .add(TextRole::OGroup, mark, palette.dim)
        .add(TextRole::OGroup, section.title, palette.text);
    if !after.is_empty() {
        text = text.space(role, "  ").add(role, &after, palette.dim);
    }
    let width = ui.available_width();
    text.job_mut().wrap = egui::text::TextWrapping::truncate_at_width(width);
    let laid = text.layout(ui.ctx());
    ui.add_space(8.0);
    let height = laid.height() + 4.0;
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    // The hint, which the heading's name leaves out, for a screen reader:
    // under the toggle, which takes the press.
    if open && !section.hint.is_empty() {
        widgets::announce(ui, rect, section.hint);
    }
    let id = ui.id().with(("disclosure", section.salt));
    let response = ui.interact(rect, id, Sense::click());
    announce(&response, &section.name(open), open);
    if focus::shown(&response) {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.selection);
    }
    laid.paint_left(ui.painter(), rect.left(), rect.top() + laid.height() / 2.0);
    let (line, _) = ui.allocate_exact_size(vec2(width, 1.0), Sense::hover());
    widgets::hline(ui, line.x_range(), line.center().y, palette.outline);
    focus::hint(ui, &response, rect, Ring::Bar);
    ui.add_space(8.0);
    response
}
