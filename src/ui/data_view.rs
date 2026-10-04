//! The open object: its header (name, counts, the Data/Structure switch),
//! the toolbar (filters and sort), the grid or its error or empty state,
//! and the status footer.

use egui::{
    CornerRadius, Frame, Id, Margin, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType, pos2,
    vec2,
};
use std::collections::BTreeMap;

use tabletist_db::{NewValue, SortDir, Value, ValueKind};

use crate::app::App;
use crate::edit::{Pending, State, Table};
use crate::i18n::gettext;
use crate::model::{Action, CellPos, ConnTabId, EditStart, ObjectTab, ObjectView, TabId};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::cell_editor;
use crate::ui::focus;
use crate::ui::format;
use crate::ui::grid::{self, Cell, Column, Mark, Style};
use crate::ui::states;
use crate::ui::widgets;

/// Left and right padding of the header, toolbar and footer.
fn side(look: &Look) -> f32 {
    if look.terminal { 16.0 } else { 20.0 }
}

/// The height of one line in `role`, as CSS's `line-height: normal`.
fn line(ui: &egui::Ui, role: TextRole, look: &Look) -> f32 {
    role.row_height(ui.ctx(), look.faces)
}

/// A secondary-text label in the layout.
fn note(ui: &mut egui::Ui, text: &str, color: egui::Color32, look: &Look) -> egui::Response {
    Text::one(look, widgets::secondary(look), text, color)
        .layout(ui.ctx())
        .label(ui)
}

/// What the footer says of the selection and of a read-only connection;
/// empty when there is nothing to say.
fn state_note(selected: bool, read_only: bool, locale: crate::i18n::Locale) -> String {
    let mut parts = Vec::new();
    if selected {
        parts.push(gettext(locale, "1 row selected"));
    }
    if read_only {
        parts.push(gettext(locale, "read-only"));
    }
    parts.join(" · ")
}

/// The parts of "13 rows · 6 columns · public", as far as it is known.
fn subtitle(object: &ObjectTab, look: &Look, locale: crate::i18n::Locale) -> Vec<String> {
    let mut parts = Vec::new();
    let page = object.page();
    let rows = object.count.value.or_else(|| {
        let filtered = !object.query.filters.is_empty() || object.query.raw_where.is_some();
        page.filter(|page| !page.has_more && object.query.offset == 0)
            .map(|page| page.rows.len() as u64)
            .or(object.estimated_rows.filter(|_| !filtered))
    });
    if let Some(rows) = rows {
        let noun = if rows == 1 { "row" } else { "rows" };
        parts.push(format!(
            "{} {}",
            format::group_digits(rows),
            gettext(locale, noun)
        ));
    }
    if let Some(page) = page {
        let count = page.columns.len();
        let noun = match (count == 1, look.terminal) {
            (true, true) => "col",
            (false, true) => "cols",
            (true, false) => "column",
            (false, false) => "columns",
        };
        parts.push(format!("{count} {}", gettext(locale, noun)));
    }
    if !look.terminal {
        parts.push(format::display_safe(&object.object.schema).into_owned());
    }
    parts
}

/// As many of `parts` as fit `room` (measured by `width`), joined with
/// " · ": the summary gives way from its end, then goes altogether.
fn fit_parts(parts: &[String], room: f32, width: impl Fn(&str) -> f32) -> Option<String> {
    (1..=parts.len())
        .rev()
        .map(|kept| parts[..kept].join(" · "))
        .find(|text| width(text) <= room)
}

/// Paints `text` from `x`, centred on `y`, and names it `name` for screen
/// readers (the whole text, when the painted one is cut). Returns its width.
pub fn paint_named(ui: &egui::Ui, x: f32, y: f32, text: Text, name: &str) -> f32 {
    let laid = text.layout(ui.ctx());
    let width = laid.paint_left(ui.painter(), x, y);
    let size = laid.size();
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(x, y - size.y / 2.0), vec2(width, size.y)),
        name,
    );
    width
}

/// The object's name and counts, the Data/Structure switch, and Add row
/// (disabled until editing arrives).
pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let name = format::display_safe(&object.object.name).into_owned();
    let view = object.view;
    let parts = subtitle(object, &look, locale);
    let summary = parts.join(" · ");
    let mut actions = Vec::new();
    // macOS: 14 above and 12 below the title and its line, 2 apart.
    // Terminal: 12 above and 10 below a line holding a 2 pt underline.
    let title_role = TextRole::pick(&look, TextRole::TableTitle, TextRole::OTableTitle);
    let sub_role = TextRole::pick(&look, TextRole::Secondary, TextRole::OBody);
    let (title_line, sub_line) = (line(ui, title_role, &look), line(ui, sub_role, &look));
    let title = |text: &str, color| Text::one(&look, title_role, text, color);
    let sub = |text: &str, color| Text::one(&look, sub_role, text, color);
    let height = if look.terminal {
        12.0 + title_line.max(line(ui, TextRole::OBody, &look) + 6.0) + 10.0 + 1.0
    } else {
        14.0 + title_line + 2.0 + sub_line + 12.0 + 1.0
    };
    egui::Panel::top(Id::new(("object-header", tab.0, object_tab.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            focus::region(ui, focus::Region::Toolbar, rect);
            let divider = if look.terminal {
                palette.outline
            } else {
                palette.surface_hover
            };
            widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, divider);
            let left = rect.left() + side(&look);
            let right = rect.right() - side(&look);
            let views = [
                (ObjectView::Data, gettext(locale, "Data"), "d"),
                (ObjectView::Structure, gettext(locale, "Structure"), "s"),
            ];
            let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
            if look.terminal {
                let center = rect.top() + (rect.height() - 1.0 + 2.0) / 2.0;
                let role = TextRole::OBody;
                let widths: Vec<f32> = views
                    .iter()
                    .map(|(_, label, key)| width(role, &format!("{key} {}", label.to_lowercase())))
                    .collect();
                // The views keep their place at the right, 16 clear of
                // their 4 pt hit margin; the summary gives way, then the
                // title is cut.
                let room = right
                    - (widths.iter().sum::<f32>() + 14.0 * (widths.len() - 1) as f32)
                    - 4.0
                    - 16.0
                    - left;
                let shown =
                    grid::ellipsize(&name, room.max(0.0), false, |text| width(title_role, text));
                let x =
                    left + paint_named(ui, left, center, title(&shown, palette.text), &name) + 16.0;
                if let Some(shown) =
                    fit_parts(&parts, left + room - x, |text| width(sub_role, text))
                {
                    paint_named(ui, x, center, sub(&shown, palette.dim), &summary);
                }
                // `d data  s structure`, the active one underlined.
                let mut x = right;
                for ((target, label, key), width) in views.iter().zip(widths).rev() {
                    let label = label.to_lowercase();
                    let hit = Rect::from_min_size(
                        pos2(x - width - 4.0, rect.top()),
                        vec2(width + 8.0, rect.height()),
                    );
                    let response = ui.interact(hit, ui.id().with(("view", *key)), Sense::click());
                    let selected = view == *target;
                    if selected {
                        focus::claim(ui, focus::Region::Toolbar, &response);
                    }
                    let name = views
                        .iter()
                        .find(|(v, ..)| v == target)
                        .map(|(_, l, _)| l.to_string())
                        .unwrap_or_default();
                    response.widget_info(|| {
                        WidgetInfo::selected(WidgetType::Button, true, selected, &name)
                    });
                    let color = if selected {
                        palette.accent
                    } else {
                        palette.dim
                    };
                    // The key stays muted; the word takes the state's colour.
                    widgets::paint_text(
                        ui,
                        x - width,
                        center,
                        Text::new(&look)
                            .add(role, key, palette.dim)
                            .space(role, " ")
                            .add(role, &label, color),
                    );
                    if selected {
                        let y = center + line(ui, role, &look) / 2.0 + 2.0;
                        ui.painter().rect_filled(
                            Rect::from_min_max(pos2(x - width, y), pos2(x, y + 2.0)),
                            CornerRadius::ZERO,
                            palette.accent,
                        );
                    }
                    if response.clicked() && !selected {
                        actions.push(Action::SetView {
                            tab,
                            object_tab,
                            view: *target,
                        });
                    }
                    x -= width + 14.0;
                }
                return;
            }
            let title_y = rect.top() + 14.0 + title_line / 2.0;
            let sub_y = rect.top() + 14.0 + title_line + 2.0 + sub_line / 2.0;
            let center = rect.top() + 14.0 + (title_line + 2.0 + sub_line) / 2.0;
            // The Data/Structure switch 16 + 12 after the title: a 3 pt
            // track round 28 pt segments, 14 at their sides.
            let widths: Vec<f32> = views
                .iter()
                .map(|(_, label, _)| width(TextRole::UiBodyStrong, label) + 28.0)
                .collect();
            let switch = widths.iter().sum::<f32>() + 6.0;
            // Add row, disabled until editing arrives, keeps 12 clear of the
            // switch. When the room runs out the summary gives way first,
            // then Add row drops its text, then it goes, and only then is
            // the title cut.
            let label = gettext(locale, "Add row");
            let reason = gettext(locale, "Editing arrives in a later version");
            let add_row = |short: bool| {
                let button = widgets::ButtonSpec::new(if short { "" } else { &label })
                    .label(&label)
                    .icon(Icon::Plus)
                    .role(TextRole::UiBodyStrong)
                    .disabled(&reason);
                if short { button.gap(0.0) } else { button }
            };
            let stack = |button: Option<bool>| {
                let button = button.map_or(0.0, |short| 12.0 + add_row(short).width(ui, &look));
                right - left - 16.0 - 12.0 - switch - button
            };
            let title_width = width(title_role, &name);
            let button = [Some(false), Some(true), None]
                .into_iter()
                .find(|button| stack(*button) >= title_width)
                .flatten();
            let room = stack(button).max(0.0);
            let shown = grid::ellipsize(&name, room, false, |text| width(title_role, text));
            let title_width = paint_named(ui, left, title_y, title(&shown, palette.text), &name);
            let sub_width = fit_parts(&parts, room, |text| width(sub_role, text))
                .map_or(0.0, |shown| {
                    paint_named(ui, left, sub_y, sub(&shown, palette.dim), &summary)
                });
            let mut x = left + title_width.max(sub_width) + 16.0 + 12.0;
            let track = Rect::from_min_size(pos2(x, center - 17.0), vec2(switch, 34.0));
            ui.painter()
                .rect_filled(track, CornerRadius::same(look.radius), palette.surface);
            x += 3.0;
            for (index, ((target, label, _), width)) in views.iter().zip(widths).enumerate() {
                let cell = Rect::from_min_size(pos2(x, track.top() + 3.0), vec2(width, 28.0));
                x += width;
                let selected = view == *target;
                // One Tab stop for the switch; the arrows choose inside it.
                let (response, arrow) = focus::segment(
                    ui,
                    cell,
                    ui.id().with(("view", label.as_ref())),
                    ui.id().with("views"),
                    (index, views.len()),
                    selected,
                );
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::Button, true, selected, label.as_ref())
                });
                let radius = look.radius.saturating_sub(2);
                focus::hint(ui, &response, cell, focus::Ring::Edge { radius });
                if selected {
                    focus::claim(ui, focus::Region::Toolbar, &response);
                }
                if let Some((chosen, _, _)) = arrow.and_then(|index| views.get(index)) {
                    actions.push(Action::SetView {
                        tab,
                        object_tab,
                        view: *chosen,
                    });
                }
                if selected {
                    let corner = CornerRadius::same(look.radius.saturating_sub(2));
                    ui.painter().add(
                        egui::epaint::Shadow {
                            offset: [0, 1],
                            blur: 2,
                            spread: 0,
                            color: egui::Color32::from_black_alpha(20),
                        }
                        .as_shape(cell, corner),
                    );
                    ui.painter().rect_filled(cell, corner, palette.window);
                }
                let (role, color) = if selected {
                    (TextRole::UiBodyStrong, palette.text)
                } else {
                    (TextRole::UiBody, palette.secondary)
                };
                let text = Text::one(&look, role, label, color).layout(ui.ctx());
                text.paint_center(ui.painter(), cell.center());
                if response.clicked() && !selected {
                    actions.push(Action::SetView {
                        tab,
                        object_tab,
                        view: *target,
                    });
                }
            }
            if let Some(short) = button {
                let button = add_row(short);
                let width = button.width(ui, &look);
                let place =
                    Rect::from_min_size(pos2(right - width, center - 16.0), vec2(width, 32.0));
                button.show_at(ui, place, &look, &palette);
            }
        });
    app.actions.extend(actions);
}

/// The sort order as a chip: "Sort created_at ↑ ×", or the terminal's
/// "sort created_at ↑".
#[allow(clippy::too_many_arguments)] // two call sites; the pieces are unrelated
fn sort_chip(
    ui: &mut egui::Ui,
    right_align: Option<f32>,
    left: f32,
    center: f32,
    column: &str,
    dir: SortDir,
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) -> (Rect, bool) {
    let arrow = if dir == SortDir::Asc { "↑" } else { "↓" };
    let column = format!("{column} {arrow}");
    let measure = |text: &str, role: TextRole| role.width(ui.ctx(), look.faces, text);
    if look.terminal {
        // One bordered line: "sort created_at ↑", 8 at its sides.
        let text = format!("{} {column}", gettext(locale, "sort"));
        let role = TextRole::OBody;
        let height = line(ui, role, look) + 2.0 + 2.0;
        let width = measure(&text, role) + 16.0 + 2.0;
        let x = right_align.map_or(left, |right| right - width);
        let chip = Rect::from_min_size(pos2(x, center - height / 2.0), vec2(width, height));
        ui.painter().rect_stroke(
            chip,
            CornerRadius::same(3),
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
        widgets::paint_text(
            ui,
            chip.left() + 9.0,
            center,
            Text::one(look, role, &text, palette.text),
        );
        return (chip, false);
    }
    // macOS: 10 in, "Sort", 6, the column in Plex Mono 12, 6, a 20 pt ×, 6.
    let word = gettext(locale, "Sort").into_owned();
    let (word_role, column_role) = (TextRole::UiBody, TextRole::MonoSecondary);
    let width =
        10.0 + measure(&word, word_role) + 6.0 + measure(&column, column_role) + 6.0 + 20.0 + 6.0;
    let x = right_align.map_or(left, |right| right - width);
    let chip = Rect::from_min_size(pos2(x, center - 14.0), vec2(width, 28.0));
    ui.painter()
        .rect_filled(chip, CornerRadius::same(6), palette.surface);
    let mut x = chip.left() + 10.0;
    x += widgets::paint_text(
        ui,
        x,
        center,
        Text::one(look, word_role, &word, palette.dim),
    ) + 6.0;
    widgets::paint_text(
        ui,
        x,
        center,
        Text::one(look, column_role, &column, palette.text),
    );
    let hit = Rect::from_min_size(pos2(chip.right() - 26.0, center - 10.0), vec2(20.0, 20.0));
    let response = ui.interact(hit, ui.id().with("clear-sort"), Sense::click());
    let label = gettext(locale, "Clear sort");
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    if response.hovered() {
        ui.painter()
            .rect_filled(hit, CornerRadius::same(4), palette.surface_hover);
    }
    Icon::X
        .image(palette.secondary, 10.0)
        .paint_at(ui, Rect::from_center_size(hit.center(), vec2(10.0, 10.0)));
    (chip, response.clicked())
}

/// The applied filters as the toolbar's chips write them, the raw WHERE
/// last.
fn filter_texts(object: &ObjectTab) -> Vec<String> {
    object
        .query
        .filters
        .iter()
        .map(|filter| {
            let op = crate::ui::filter_bar::op_label(filter.op);
            if matches!(
                filter.op,
                tabletist_db::FilterOp::IsNull | tabletist_db::FilterOp::IsNotNull
            ) {
                format!("{} {op}", filter.column)
            } else {
                format!("{} {op} {}", filter.column, filter.value)
            }
        })
        .chain(object.query.raw_where.clone())
        .collect()
}

/// Above the grid: Add filter and the filters in use (macOS), or the
/// terminal's WHERE line; the sort; and how timestamps are shown.
pub fn toolbar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let full_precision = workspace.full_precision;
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let filters = filter_texts(object);
    let sort = object
        .query
        .sort
        .first()
        .map(|sort| (sort.column.clone(), sort.dir));
    let temporal = object.page().is_some_and(|page| {
        page.columns
            .iter()
            .any(|column| column.kind == ValueKind::Temporal)
    });
    let mut actions = Vec::new();
    // 44 (macOS) or 34 (terminal, on the dark tone), and a rule.
    let height = if look.terminal { 35.0 } else { 45.0 };
    let fill = if look.terminal {
        palette.panel
    } else {
        palette.window
    };
    egui::Panel::top(Id::new(("object-toolbar", tab.0, object_tab.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
            let center = rect.top() + (rect.height() - 1.0) / 2.0;
            let left = rect.left() + side(&look);
            let right = rect.right() - side(&look);
            if look.terminal {
                // The sort, then the WHERE line in the room left of it.
                let mut limit = right;
                if let Some((column, dir)) = &sort {
                    let (chip, _) = sort_chip(
                        ui,
                        Some(right),
                        left,
                        center,
                        column,
                        *dir,
                        &look,
                        &palette,
                        locale,
                    );
                    limit = chip.left() - 12.0;
                }
                where_line(
                    app,
                    ui,
                    tab,
                    object_tab,
                    Rect::from_min_max(pos2(left, rect.top()), pos2(limit, rect.bottom())),
                    &mut actions,
                );
                return;
            }
            // Add filter: a dashed button that opens the filter editor.
            let label = gettext(locale, "Add filter");
            let text_width = TextRole::UiBody.width(ui.ctx(), look.faces, &label);
            // 28 tall; a 1 pt dashed border, 10 of padding, a 13 pt funnel,
            // 6, the words (a browser's buttons are border-box).
            let button = Rect::from_min_size(
                pos2(left, center - 14.0),
                vec2(1.0 + 10.0 + 13.0 + 6.0 + text_width + 10.0 + 1.0, 28.0),
            );
            let response = ui.interact(button, ui.id().with("add-filter"), Sense::click());
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
            if response.hovered() {
                ui.painter()
                    .rect_filled(button, CornerRadius::same(6), palette.panel);
            }
            dashed_rect(
                ui,
                button,
                6.0,
                palette.faint.lerp_to_gamma(palette.window, 0.45),
            );
            Icon::Funnel.image(palette.secondary, 13.0).paint_at(
                ui,
                Rect::from_center_size(pos2(button.left() + 17.5, center), vec2(13.0, 13.0)),
            );
            widgets::paint_text(
                ui,
                button.left() + 30.0,
                center,
                Text::one(&look, TextRole::UiBody, &label, palette.secondary),
            );
            if response.clicked() {
                actions.push(Action::ToggleFilterBar(tab));
            }
            let mut x = button.right() + 8.0;
            // Where the button, the chips and the sort end.
            let mut taken = button.right();
            for (index, text) in filters.iter().enumerate() {
                let mono = TextRole::MonoSecondary;
                let width = mono.width(ui.ctx(), look.faces, text);
                let chip = Rect::from_min_size(pos2(x, center - 14.0), vec2(width + 42.0, 28.0));
                ui.painter()
                    .rect_filled(chip, CornerRadius::same(6), palette.surface);
                Icon::Funnel.image(palette.dim, 11.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(chip.left() + 13.0, center), vec2(11.0, 11.0)),
                );
                widgets::paint_text(
                    ui,
                    chip.left() + 23.0,
                    center,
                    Text::one(&look, mono, text, palette.text),
                );
                let hit =
                    Rect::from_center_size(pos2(chip.right() - 12.0, center), vec2(18.0, 18.0));
                let remove = ui.interact(hit, ui.id().with(("drop-filter", index)), Sense::click());
                let name = format!("{} {text}", gettext(locale, "Remove filter"));
                remove.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
                Icon::X
                    .image(palette.dim, 11.0)
                    .paint_at(ui, Rect::from_center_size(hit.center(), vec2(11.0, 11.0)));
                if remove.clicked() {
                    actions.push(Action::DropFilter {
                        tab,
                        object_tab,
                        index,
                    });
                }
                x = chip.right() + 8.0;
                taken = chip.right();
            }
            if let Some((column, dir)) = &sort {
                let (chip, cleared) =
                    sort_chip(ui, None, x, center, column, *dir, &look, &palette, locale);
                if cleared {
                    actions.push(Action::ClearSort { tab, object_tab });
                }
                taken = chip.right();
            }
            if temporal {
                let small = TextRole::Secondary;
                let (said, link) = if full_precision {
                    (
                        gettext(locale, "Timestamps shown in full"),
                        gettext(locale, "To the second"),
                    )
                } else {
                    (
                        gettext(locale, "Timestamps shown to the second"),
                        gettext(locale, "Full precision"),
                    )
                };
                // The hint keeps to the room the chips left, 12 clear of them:
                // the sentence goes first, then the link.
                let room = right - (taken + 12.0);
                let link_width = small.width(ui.ctx(), look.faces, &link);
                if link_width <= room {
                    let hit = Rect::from_min_size(
                        pos2(right - link_width, center - 9.0),
                        vec2(link_width, 18.0),
                    );
                    let response = ui.interact(hit, ui.id().with("precision"), Sense::click());
                    response
                        .widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, link.as_ref()));
                    let color = if response.hovered() {
                        palette.accent_hover
                    } else {
                        palette.accent
                    };
                    widgets::paint_text(
                        ui,
                        hit.left(),
                        center,
                        Text::one(&look, small, &link, color),
                    );
                    if response.clicked() {
                        actions.push(Action::ToggleFullPrecision(tab));
                    }
                    // "… second · ", the space before the link kept out of the text.
                    let space = small.width(ui.ctx(), look.faces, " ");
                    let said = format!("{said} ·");
                    if small.width(ui.ctx(), look.faces, &said) + space + link_width <= room {
                        widgets::paint_text_right(
                            ui,
                            hit.left() - space,
                            center,
                            Text::one(&look, small, &said, palette.dim),
                        );
                    }
                }
            }
        });
    app.actions.extend(actions);
}

/// The terminal look's `/ where …` line: a raw WHERE clause, run on Enter.
fn where_line(
    app: &mut App,
    ui: &mut egui::Ui,
    tab: ConnTabId,
    object_tab: TabId,
    rect: Rect,
    actions: &mut Vec<Action>,
) {
    let palette = app.palette;
    let locale = app.locale;
    let look = app.look;
    let focus = app
        .workspace_mut(tab)
        .is_some_and(|workspace| std::mem::take(&mut workspace.focus_where));
    let Some(object) = app
        .workspace_mut(tab)
        .and_then(|w| w.object_tab_mut(object_tab))
    else {
        return;
    };
    let center = rect.center().y;
    let role = TextRole::OBody;
    let mut x = rect.left();
    x += widgets::paint_text(
        ui,
        x,
        center,
        Text::one(
            &look,
            TextRole::OGroup,
            "/",
            palette.accent,
        ),
    ) + 10.0
        // The design's input keeps its 2 pt padding.
        + 2.0;
    // "where", then the clause a space on, as one line of text reads.
    x += widgets::paint_text(ui, x, center, Text::one(&look, role, "where", palette.dim))
        + role.width(ui.ctx(), look.faces, " ");
    let line = role.row_height(ui.ctx(), look.faces);
    let field = Rect::from_min_max(
        pos2(x, center - line / 2.0),
        pos2(rect.right(), center + line / 2.0),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
    let mut layouter = crate::typography::layouter(&look, role, palette.secondary);
    let placeholder = Text::one(
        &look,
        role,
        &gettext(locale, "a condition, then Enter"),
        palette.faint,
    )
    .layout(ui.ctx());
    let response = child.add(
        egui::TextEdit::singleline(&mut object.filter.raw_text)
            .font(role.font_id(look.faces))
            .frame(egui::Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(field.width())
            .hint_text(placeholder.galley)
            .layouter(&mut layouter),
    );
    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "WHERE"));
    // A line of the bar, not a box: its caret says where the keyboard is.
    crate::ui::focus::hint(ui, &response, field, crate::ui::focus::Ring::Own);
    if focus {
        response.request_focus();
    }
    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        object.filter.raw = !object.filter.raw_text.trim().is_empty();
        actions.push(Action::ApplyFilters { tab, object_tab });
    }
}

/// A dashed outline round `rect`, its corners cut at `radius`.
fn dashed_rect(ui: &egui::Ui, rect: Rect, radius: f32, color: egui::Color32) {
    let stroke = Stroke::new(widgets::hairline(ui).max(1.0), color);
    let rect = rect.shrink(0.5);
    let r = radius;
    let sides = [
        [
            pos2(rect.left() + r, rect.top()),
            pos2(rect.right() - r, rect.top()),
        ],
        [
            pos2(rect.right(), rect.top() + r),
            pos2(rect.right(), rect.bottom() - r),
        ],
        [
            pos2(rect.right() - r, rect.bottom()),
            pos2(rect.left() + r, rect.bottom()),
        ],
        [
            pos2(rect.left(), rect.bottom() - r),
            pos2(rect.left(), rect.top() + r),
        ],
    ];
    for side in sides {
        ui.painter()
            .extend(egui::Shape::dashed_line(&side, stroke, 3.0, 2.5));
    }
    // The corners, as short arcs.
    let corners = [
        (pos2(rect.left() + r, rect.top() + r), std::f32::consts::PI),
        (
            pos2(rect.right() - r, rect.top() + r),
            1.5 * std::f32::consts::PI,
        ),
        (pos2(rect.right() - r, rect.bottom() - r), 0.0),
        (
            pos2(rect.left() + r, rect.bottom() - r),
            0.5 * std::f32::consts::PI,
        ),
    ];
    for (center, start) in corners {
        let points: Vec<egui::Pos2> = (0..=4)
            .map(|step| {
                let angle = start + step as f32 / 4.0 * std::f32::consts::FRAC_PI_2;
                center + r * vec2(angle.cos(), angle.sin())
            })
            .collect();
        ui.painter().add(egui::Shape::line(points, stroke));
    }
}

/// What decides how wide a grid's cells are, beside its rows. A grid keeps
/// the widths it fitted under its id, so these are part of the id: when one
/// changes the columns are fitted again, as a grid of its own.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Fit {
    /// Timestamps with their fraction.
    pub full_precision: bool,
    /// Numbers with their digits in threes.
    pub grouped: bool,
    /// Values of a closed set as tags, which pad their text.
    pub value_tags: bool,
}

impl Fit {
    pub fn of(workspace: &crate::model::Workspace, settings: &crate::settings::Settings) -> Self {
        Self {
            full_precision: workspace.full_precision,
            grouped: settings.group_digits,
            value_tags: settings.value_tags,
        }
    }
}

/// The id of a table's grid, one per [`Fit`].
fn grid_id(tab: ConnTabId, object_tab: TabId, fit: Fit) -> Id {
    Id::new(("grid", tab.0, object_tab.0, fit))
}

/// Where egui's memory keeps which grid the table `object_tab` was last
/// drawn with.
fn table_id(tab: ConnTabId, object_tab: TabId) -> Id {
    Id::new(("table-grid", tab.0, object_tab.0))
}

/// What a status line says of the columns while some are out of view:
/// `Columns 1–10 of 40 · id pinned`, or the terminal's `cols 1–7 of 40`.
/// Nothing while every column shows. It reads what the grid drew last, so
/// it is a frame behind a scroll.
pub fn columns_note(
    ctx: &egui::Context,
    tab: ConnTabId,
    object: &ObjectTab,
    fit: Fit,
    look: &Look,
    locale: crate::i18n::Locale,
) -> Option<String> {
    let id = grid_id(tab, object.id, fit);
    let shown = grid::columns_shown(ctx, id).filter(grid::ColumnsShown::partial)?;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let noun = if look.terminal { "cols" } else { "Columns" };
    let mut note = format!(
        "{} {}–{} {} {}",
        say(noun),
        shown.first + 1,
        shown.last + 1,
        say("of"),
        shown.total
    );
    let first = object.page().and_then(|page| page.columns.first());
    if let (true, Some(column)) = (shown.pinned, first) {
        note.push_str(&format!(
            " · {} {}",
            format::display_safe(&column.name),
            say("pinned")
        ));
    }
    Some(note)
}

/// The status footer: the page's range and paging, then what is selected
/// and how long the query took.
pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let view = object.view;
    let fetching = object.rows.is_loading();
    let counting = object.count.is_loading();
    let count_error = object.count.error.as_ref().map(ToString::to_string);
    // Counting helps only when the page is not the whole result.
    let partial = object
        .page()
        .is_some_and(|page| page.has_more || object.query.offset > 0);
    let can_count = object.count.value.is_none() && !counting && partial;
    let page = object.page();
    let filtered = !object.query.filters.is_empty() || object.query.raw_where.is_some();
    let range = page.and_then(|page| {
        format::range_label(
            object.query.offset,
            page.rows.len(),
            page.has_more,
            // The table's estimate is not the filtered total.
            object.estimated_rows.filter(|_| !filtered),
            object.count.value,
        )
    });
    let can_next = page.is_some_and(|page| page.has_more) && !object.rows.is_loading();
    let can_prev = object.query.offset > 0 && !object.rows.is_loading();
    let timing = page.map(|page| format::elapsed(page.elapsed));
    let unordered = page.is_some_and(|page| !page.ordered_by_key) && object.query.sort.is_empty();
    let selected = object.selection.is_some();
    let read_only = app
        .workspace(tab)
        .is_some_and(|workspace| workspace.access == tabletist_db::Access::ReadOnly);
    let state = state_note(selected, read_only, locale);
    let columns = app
        .workspace(tab)
        .filter(|_| view == ObjectView::Data)
        .and_then(|workspace| {
            let fit = Fit::of(workspace, &app.settings);
            columns_note(ui.ctx(), tab, object, fit, &look, locale)
        });
    let mut actions = Vec::new();
    egui::Panel::bottom(Id::new(("object-footer", tab.0, object_tab.0)))
        .exact_size(33.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(side(&look) as i8, 0)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect().expand2(vec2(side(&look), 0.0));
            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
            // The status bar's grey sits between the secondary and muted
            // tones; notes that explain something stay secondary.
            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
            ui.add_space(1.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                if view == ObjectView::Data {
                    let range = range
                        .clone()
                        .map(|range| format!("{} {range}", gettext(locale, "Rows")))
                        .unwrap_or_else(|| {
                            if fetching {
                                gettext(locale, "Waiting for server")
                            } else {
                                gettext(locale, "No rows")
                            }
                            .into_owned()
                        });
                    note(ui, &range, status, &look);
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (enabled, icon, label, action) in [
                        (
                            can_prev,
                            Icon::ChevronLeft,
                            "Previous page",
                            Action::PrevPage { tab, object_tab },
                        ),
                        (
                            can_next,
                            Icon::ChevronRight,
                            "Next page",
                            Action::NextPage { tab, object_tab },
                        ),
                    ] {
                        ui.add_enabled_ui(enabled, |ui| {
                            if pager(ui, icon, &gettext(locale, label), enabled, &palette).clicked()
                            {
                                actions.push(action);
                            }
                        });
                    }
                    ui.spacing_mut().item_spacing.x = 16.0;
                    ui.add_space(14.0);
                    // The columns in view, where the footer has the room: what
                    // its right end says comes first.
                    if let Some(columns) = &columns {
                        let width = |text: &str| {
                            widgets::secondary(&look).width(ui.ctx(), look.faces, text)
                        };
                        let query = timing
                            .as_ref()
                            .map(|timing| format!("{} {timing}", gettext(locale, "Query")));
                        // A state with nothing to say takes no room.
                        let state_width = if state.is_empty() {
                            0.0
                        } else {
                            width(&state) + 16.0
                        };
                        let taken =
                            state_width + query.as_deref().map_or(0.0, |query| 16.0 + width(query));
                        if ui.available_width() >= width(columns) + 16.0 + taken {
                            note(ui, columns, status, &look);
                        }
                    }
                    if filtered {
                        note(ui, &gettext(locale, "Filtered"), palette.accent, &look)
                            .on_hover_text(gettext(locale, "Cmd/Ctrl+F edits the filter"));
                    }
                    if counting {
                        note(ui, &gettext(locale, "Counting…"), palette.secondary, &look);
                    } else if can_count {
                        let link = Text::one(
                            &look,
                            widgets::secondary(&look),
                            &gettext(locale, "Count"),
                            palette.accent,
                        )
                        .layout(ui.ctx())
                        .label_sense(ui, Sense::click());
                        link.widget_info(|| {
                            WidgetInfo::labeled(WidgetType::Link, true, gettext(locale, "Count"))
                        });
                        let link = match &count_error {
                            Some(error) => link.on_hover_text(error),
                            None => link.on_hover_text(gettext(locale, "Count the rows exactly")),
                        };
                        if link.clicked() {
                            actions.push(Action::CountRows { tab, object_tab });
                        }
                    }
                    if unordered {
                        note(ui, &gettext(locale, "Unordered"), palette.secondary, &look)
                            .on_hover_text(gettext(
                                locale,
                                "This object has no primary key, so rows may move between pages.",
                            ));
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 16.0;
                    if counting {
                        if widgets::icon_button(
                            ui,
                            Icon::CircleX,
                            &gettext(locale, "Cancel query"),
                            &look,
                            &palette,
                        )
                        .clicked()
                        {
                            actions.push(Action::CancelQuery(tab));
                        }
                        ui.spinner();
                    } else if let Some(timing) = &timing {
                        note(
                            ui,
                            &format!("{} {timing}", gettext(locale, "Query")),
                            status,
                            &look,
                        );
                    }
                    if !state.is_empty() {
                        note(ui, &state, status, &look);
                    }
                });
            });
        });
    app.actions.extend(actions);
}

/// A footer's page arrow: a bare chevron, faint when there is nowhere to go.
fn pager(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    enabled: bool,
    palette: &Palette,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(24.0, 24.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, enabled, label));
    if response.hovered() && enabled {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(5), palette.surface);
    }
    // Disabled: the status grey at 35%.
    let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
    let tint = if enabled {
        status
    } else {
        palette.panel.lerp_to_gamma(status, 0.35)
    };
    icon.image(tint, 12.0)
        .paint_at(ui, Rect::from_center_size(rect.center(), vec2(12.0, 12.0)));
    response.on_hover_text(label)
}

/// What the grid's header says under a column's name, and whether the
/// column is part of the primary key. A SQL editor's result has no
/// `structure`: its columns say their types alone.
pub fn type_line(
    name: &str,
    type_name: &str,
    kind: ValueKind,
    structure: Option<&tabletist_db::Structure>,
    look: &Look,
) -> (String, bool) {
    let key = structure
        .is_some_and(|structure| structure.primary_key.iter().any(|column| column == name));
    let target = structure.and_then(|structure| {
        structure
            .foreign_keys
            .iter()
            .find(|foreign| foreign.columns.len() == 1 && foreign.columns[0] == name)
            .map(|foreign| foreign.ref_table.clone())
    });
    let base = match (kind, type_name) {
        (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
        (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
        _ => format::type_label(type_name, kind).into_owned(),
    };
    let line = match (look.terminal, key, target) {
        (true, true, _) => "pk".to_owned(),
        (true, _, Some(target)) => format!("→ {target}"),
        (false, _, Some(target)) => format!("{base} → {target}"),
        _ => base,
    };
    (line, key)
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    // An editor that is open has the keyboard, unless a dialog has it.
    let hold = app.dialog.is_none();
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let fit = Fit::of(workspace, &app.settings);
    // The arrows are the grid's once the user worked in it.
    let keys = workspace.pane == crate::model::Pane::Grid;
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    // What editing asks of the workspace and the tab together, read before
    // the tab is taken for its editor's text.
    let computed = computed_columns(workspace, object);
    let target = editor_target(workspace, object, tab, hold);
    // The tab itself from here on: the field on a cell edits the text its
    // editor holds, beside the page the grid reads. Nothing else of it is
    // changed.
    let Some(object) = app
        .workspace_mut(tab)
        .and_then(|workspace| workspace.object_tab_mut(object_tab))
    else {
        return;
    };
    let mut actions = Vec::new();
    let area = ui.max_rect();
    // How long the fetch in flight has been going, read once: every piece
    // below sees the same wait.
    let waited = object.rows.running_for();
    let lasted = waited.is_some_and(states::lasted);
    if let Some(error) = shown_error(&object.rows) {
        Frame::new().inner_margin(Margin::same(12)).show(ui, |ui| {
            error_box(ui, error, &look, &palette, locale, || {
                actions.push(Action::RetryRows { tab, object_tab })
            });
        });
    } else if let Some(page) = object.rows.value.as_ref() {
        let structure = object.structure.value.as_ref();
        let columns: Vec<Column<'_>> = page
            .columns
            .iter()
            .map(|column| {
                let (type_line, key) = type_line(
                    &column.name,
                    &column.type_name,
                    column.kind,
                    structure,
                    &look,
                );
                Column {
                    name: &column.name,
                    type_line,
                    numeric: column.kind == ValueKind::Numeric,
                    sort: object.sort_of(&column.name),
                    key,
                    flexible: column.kind == ValueKind::Json,
                    sortable: true,
                }
            })
            .collect();
        let ctx = ui.ctx().clone();
        let tags: Vec<_> = crate::ui::value_tags::Tags::of_page(page, structure)
            .into_iter()
            .map(|tags| tags.when(fit.value_tags))
            .collect();
        // Grouping is for amounts: a key reads as the name it is.
        let shown: Vec<Shown> = page
            .columns
            .iter()
            .map(|column| Shown {
                full_precision: fit.full_precision,
                grouped: fit.grouped && !is_key(&column.name, structure),
            })
            .collect();
        // What is pending, and what else of editing the cells show: read
        // field by field, beside the editor whose text the field edits.
        let mut changes = Changes::of(
            &object.edits.cells,
            object.edits.saving.is_some(),
            object.edits.saved.as_ref(),
            computed,
            &ctx,
        );
        // Why the cell last asked for cannot be edited, at that cell. The
        // terminal says it in its mode line.
        changes.why = object
            .edits
            .why
            .filter(|_| !look.terminal)
            .map(|(cell, lock)| {
                let table = format::display_safe(&object.object.name);
                (cell, cell_editor::lock_text(lock, &table, locale))
            });
        let mut editor = object.edits.editor.as_mut();
        let editing = editor.as_ref().map(|editor| editor.cell);
        // What the field says of this frame, once the grid has drawn it on
        // its cell.
        let mut outcome = cell_editor::Outcome::default();
        let mut field = |ui: &mut egui::Ui, rect: Rect| {
            if let (Some(editor), Some(target)) = (editor.as_deref_mut(), &target) {
                let skin = (&look, &palette, locale);
                outcome = cell_editor::field(ui, rect, editor, target, skin);
            }
        };
        // A fit come back to is fitted to the rows now on screen: what the
        // grid of the fit before kept goes when this one is drawn.
        let id = grid_id(tab, object_tab, fit);
        grid::keep(ui.ctx(), table_id(tab, object_tab), id);
        let output = grid::show(
            ui,
            id,
            &columns,
            page.rows.len(),
            object.query.offset,
            object.selection,
            keys,
            &palette,
            &look,
            &|row| crate::edit::row_mark(changes.cells, row),
            editing,
            Some(&mut field),
            |row, col| {
                let loaded = &page.rows[row][col];
                let column = &page.columns[col];
                // A pending cell shows its new value, drawn as any value.
                let value = changes.values.get(&(row, col)).unwrap_or(loaded);
                let mut cell = cell(&ctx, value, column, &tags[col], &look, shown[col]);
                changes.mark(&mut cell, (row, col), loaded, column, &look, locale);
                cell
            },
        );
        let id = object_tab;
        // The text is noted as typed before anything ends the edit: an
        // editor that was not typed into closes without a change, and a
        // first keystroke may share its frame with Enter or a click away.
        if outcome.changed {
            actions.push(Action::EditorTyped { tab, id });
        }
        if outcome.large {
            actions.push(Action::EditorBreak { tab, id });
        } else if let Some(then) = outcome.commit {
            actions.push(Action::CommitEdit { tab, id, then });
        } else if outcome.cancel {
            actions.push(Action::CancelEdit { tab, id });
        } else if outcome.left {
            actions.push(Action::LeaveEdit { tab, id });
        }
        if let Some(cell) = output.clicked {
            actions.push(Action::SelectCell { tab, id, cell });
        }
        // A second click edits the cell, where the look edits in place.
        if let Some(cell) = output.double_clicked.filter(|_| !look.terminal) {
            let start = EditStart::Value;
            actions.push(Action::EditCell {
                tab,
                id,
                cell,
                start,
            });
        }
        // The Tab key came to the grid: the arrows are its own now.
        if output.focused {
            actions.push(Action::GridKeys(tab));
        }
        if let Some(col) = output.sort_clicked {
            actions.push(Action::SortBy {
                tab,
                object_tab,
                column: page.columns[col].name.clone(),
            });
        }
        // Not while a fetch has lasted: its box would sit on the state's
        // title or its button, and says what is happening by itself.
        if page.rows.is_empty() && !lasted {
            // The headers stay: the columns are still worth reading.
            let under = Rect::from_min_max(
                pos2(area.left(), area.top() + grid::header_height(&look)),
                area.max,
            );
            empty_rows(
                ui,
                under,
                object,
                tab,
                (&look, &palette, locale),
                &mut actions,
            );
        }
    } else if lasted {
        // Rows on their way and none to show yet: the shape of a grid.
        if !look.terminal {
            states::progress(ui, area.x_range(), area.top(), &palette);
        }
        let rows = Rect::from_min_max(pos2(area.left(), area.top() + 2.0), area.max);
        states::skeleton(ui, rows, &look, &palette);
    }
    // A fetch that has lasted, over whatever is up: a refresh and the next
    // page keep the page they replace on screen.
    if let Some(waited) = waited {
        if lasted {
            let (text, name, keys) = (
                look.label(&gettext(locale, "Running query…")),
                look.label(&gettext(locale, "Cancel")),
                cancel_keys(&look),
            );
            let cancel = states::key_button(&name, &keys, &look).label("Cancel query");
            if states::running(ui, area, &text, waited, Some(cancel), &look, &palette) {
                actions.push(Action::CancelQuery(tab));
            }
        } else {
            // Come back when the wait is long enough to show.
            ui.ctx().request_repaint_after(states::DELAY - waited);
        }
    }
    app.actions.extend(actions);
}

/// What editing shows in a table's grid: the pending cells, the ones a save
/// just wrote, and the columns no edit reaches.
struct Changes<'a> {
    cells: &'a BTreeMap<(usize, usize), Pending>,
    /// Each pending cell's new value, as a value a cell draws.
    values: BTreeMap<(usize, usize), Value>,
    /// The columns the database computes, in a table that can be edited.
    computed: Vec<bool>,
    /// A save is running.
    saving: bool,
    /// The cells the last save wrote, while they show it.
    saved: &'a [CellPos],
    /// The cell that was asked for and cannot be edited, and why.
    why: Option<(CellPos, String)>,
}

/// The page's columns that the database computes, in a table that can be
/// edited: none in any other. Decided once for each column, and not by
/// asking why each cell is locked: a save and a fetch lock every cell for a
/// while, and a computed column is drawn as one through both.
fn computed_columns(workspace: &crate::model::Workspace, object: &ObjectTab) -> Vec<bool> {
    let computes = |structure: &tabletist_db::Structure| {
        structure.columns.iter().any(|column| column.generated)
    };
    let table = Table::of(workspace, object)
        .filter(|table| table.structure.is_some_and(computes))
        .filter(|table| table.never().is_none());
    let Some(table) = table else {
        return Vec::new();
    };
    (0..table.page.columns.len())
        .map(|col| table.column(col).is_some_and(|column| column.generated))
        .collect()
}

/// The cell the tab's open editor is on, as its field needs it.
fn editor_target(
    workspace: &crate::model::Workspace,
    object: &ObjectTab,
    tab: ConnTabId,
    hold: bool,
) -> Option<cell_editor::Target> {
    let editor = object.edits.editor.as_ref()?;
    let table = Table::of(workspace, object)?;
    let column = table.page.columns.get(editor.cell.col)?;
    let max_chars = match table.class(editor.cell.col) {
        Some(tabletist_db::ColumnClass::Text { max_chars }) => max_chars,
        _ => None,
    };
    // The type as the header names it, not as the structure does.
    let type_name = format::type_label(&column.type_name, column.kind);
    Some(cell_editor::Target {
        id: cell_editor::field_id(tab, object.id),
        name: column.name.clone(),
        type_name: format::display_safe(&type_name).into_owned(),
        max_chars,
        hold,
    })
}

impl<'a> Changes<'a> {
    fn of(
        cells: &'a BTreeMap<(usize, usize), Pending>,
        saving: bool,
        saved: Option<&'a crate::edit::Saved>,
        computed: Vec<bool>,
        ctx: &egui::Context,
    ) -> Self {
        let values = cells
            .iter()
            .map(|(at, pending)| {
                let value = match &pending.new {
                    NewValue::Text(text) => Value::Text(text.as_str().into()),
                    NewValue::Null => Value::Null,
                };
                (*at, value)
            })
            .collect();
        let saved = saved.and_then(|saved| {
            let left = crate::edit::SAVED_FOR.checked_sub(saved.at.elapsed())?;
            // Come back when the moment is over, to draw them as they are.
            ctx.request_repaint_after(left);
            Some(saved.cells.as_slice())
        });
        Self {
            cells,
            values,
            computed,
            saving,
            saved: saved.unwrap_or_default(),
            why: None,
        }
    }

    /// Marks `cell`, the page's cell `at` (row, column) that loaded as
    /// `loaded`, and says what it tells the pointer.
    fn mark(
        &self,
        cell: &mut Cell<'_>,
        at: (usize, usize),
        loaded: &Value,
        column: &tabletist_db::ColumnMeta,
        look: &Look,
        locale: crate::i18n::Locale,
    ) {
        let (row, col) = at;
        // A cell that is not there has no reason to give.
        if let Some((asked, why)) = &self.why
            && *asked == (CellPos { row, col })
            && !why.is_empty()
        {
            cell.note = Some(why.clone());
        }
        let Some(pending) = self.cells.get(&at) else {
            cell.mark = if self.saved.contains(&CellPos { row, col }) {
                Mark::Saved
            } else if self.computed.get(col).copied().unwrap_or(false) {
                Mark::Locked
            } else {
                Mark::None
            };
            return;
        };
        let saving = self.saving;
        match &pending.state {
            State::Ready => {
                cell.mark = if saving { Mark::Saving } else { Mark::Pending };
                let was = format::cell_text(loaded);
                cell.hint = Some(format!("{} {was}", gettext(locale, "was")));
            }
            State::ToFix(problem) => {
                cell.mark = Mark::Trouble;
                let typed = match &pending.new {
                    NewValue::Text(text) => Some(text.as_str()),
                    NewValue::Null => None,
                };
                let type_name = format::type_label(&column.type_name, column.kind);
                let type_name = format::display_safe(&type_name);
                let message = cell_editor::problem_text(problem, &type_name, typed, locale);
                // The terminal has its own key for it, said in its own
                // place.
                cell.hint = Some(if look.terminal {
                    message
                } else {
                    format!(
                        "{message}\n{} · {}Z {}",
                        gettext(locale, "Checked before saving"),
                        look.command_key(),
                        gettext(locale, "reverts")
                    )
                });
            }
            State::Failed(error) => {
                // It is sent again by the save that is running.
                cell.mark = if saving { Mark::Saving } else { Mark::Trouble };
                cell.hint = Some(failure_text(error));
            }
        }
    }
}

/// What the database said of a statement that failed, with its code: a
/// failed cell's words.
fn failure_text(error: &tabletist_db::Error) -> String {
    match error {
        tabletist_db::Error::Query {
            code: Some(code),
            message,
            ..
        } => format!("{code} {}", format::capped(message)),
        other => format::capped(&other.to_string()).into_owned(),
    }
}

/// The keys that cancel a query, as the look writes them: `⌘.`, `Ctrl+.`
/// or the terminal's `ctrl+.`.
pub fn cancel_keys(look: &Look) -> String {
    format!("{}.", look.label(look.command_key()))
}

/// What a page with no rows says under its column headers: that the table
/// is empty, that the page is past its last row, or which filters leave
/// nothing, and the way out of each.
fn empty_rows(
    ui: &mut egui::Ui,
    rect: Rect,
    object: &ObjectTab,
    tab: ConnTabId,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
    actions: &mut Vec<Action>,
) {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let name = format::display_safe(&object.object.name);
    let object_tab = object.id;
    let filters = filter_texts(object);
    // Past the first page there are rows, whatever the filters: they end
    // before this page, and the way out is the page before.
    let past = object.query.offset > 0;
    if past || filters.is_empty() {
        let title = format!(
            "{} {name}",
            say(if past {
                "No more rows in"
            } else {
                "No rows in"
            })
        );
        let text = if past {
            say("This page is past the last row.")
        } else if object.kind == tabletist_db::ObjectKind::Table {
            say("The table exists and is empty.")
        } else {
            say("It returned no rows.")
        };
        let notice = states::Notice {
            icon: Icon::Table,
            title: &title,
            text: &text,
        };
        let (reload, previous) = (say("Reload"), say("Previous page"));
        let button = if past {
            states::button(&previous, look).label("Previous page")
        } else {
            // No key beside it: Cmd/Ctrl+R refreshes the tree while the
            // tree has the keys.
            let button = states::button(&reload, look).label("Reload");
            if look.terminal {
                button
            } else {
                button.icon(Icon::RefreshCw)
            }
        };
        if states::empty(ui, rect, &notice, vec![button], look, palette).is_some() {
            actions.push(if past {
                Action::PrevPage { tab, object_tab }
            } else {
                Action::Refresh(tab)
            });
        }
        return;
    }
    let title = if filters.len() == 1 {
        say("No rows match the filter")
    } else {
        format!(
            "{} {} {}",
            say("No rows match"),
            filters.len(),
            say("filters")
        )
    };
    let none = format!(
        "{} {}.",
        say("None matches"),
        filters.join(&format!(" {} ", say("and")))
    );
    // The catalog's estimate of the whole table, when it has one. None is
    // an estimate of 0: older PostgreSQL says so of a table never analysed.
    let text = match object.estimated_rows.filter(|rows| *rows > 0) {
        Some(rows) => format!(
            "{name} {} {} {}. {none}",
            say("has about"),
            format::group_digits(rows),
            say(if rows == 1 { "row" } else { "rows" })
        ),
        None => none,
    };
    let notice = states::Notice {
        icon: Icon::Funnel,
        title: &title,
        text: &text,
    };
    let (clear, last) = (say("Clear filters"), say("Remove last filter"));
    let mut buttons = vec![states::button(&clear, look).label("Clear filters")];
    if filters.len() > 1 {
        buttons.push(
            states::button(&last, look)
                .label("Remove last filter")
                .quiet(),
        );
    }
    match states::empty(ui, rect, &notice, buttons, look, palette) {
        Some(0) => actions.push(Action::ClearFilters { tab, object_tab }),
        // `DropFilter` counts the raw WHERE last, as `filter_texts` does.
        Some(_) => actions.push(Action::DropFilter {
            tab,
            object_tab,
            index: filters.len() - 1,
        }),
        None => {}
    }
}

/// How a cell writes its value out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Shown {
    /// Timestamps with the fraction the server sent.
    pub full_precision: bool,
    /// A number's integer digits in threes.
    pub grouped: bool,
}

/// Whether `name` is a column of the table's primary key or of one of its
/// foreign keys: a number that names a row, which grouping would only make
/// harder to read. Nothing is, until the table is described.
fn is_key(name: &str, structure: Option<&tabletist_db::Structure>) -> bool {
    structure.is_some_and(|structure| {
        structure.primary_key.iter().any(|column| column == name)
            || structure
                .foreign_keys
                .iter()
                .any(|foreign| foreign.columns.iter().any(|column| column == name))
    })
}

/// A cell of a result grid, a table's or a SQL editor's: a value from its
/// column's closed set (`tags`) as its tag, and anything else as
/// [`plain_cell`] reads it. NULL is never a tag, and a colour is a colour
/// before it is one: the order is NULL, colour, tag, then the rest.
pub fn cell<'a>(
    ctx: &egui::Context,
    value: &'a tabletist_db::Value,
    column: &tabletist_db::ColumnMeta,
    tags: &crate::ui::value_tags::Tags<'_>,
    look: &Look,
    shown: Shown,
) -> Cell<'a> {
    if let Some(style) = tags.style(value)
        && format::color(value).is_none()
    {
        return Cell {
            text: format::cell_text(value),
            style,
            ..Default::default()
        };
    }
    plain_cell(ctx, value, column, look, shown)
}

/// A cell's text and style when it is no tag: NULL, a binary value's type
/// and size, a colour's swatch, a document at a glance, what stands for
/// text with nothing to see, an array's elements, a timestamp to the second
/// and a number in threes as `shown` asks, and anything else as it reads.
pub fn plain_cell<'a>(
    ctx: &egui::Context,
    value: &'a tabletist_db::Value,
    column: &tabletist_db::ColumnMeta,
    look: &Look,
    shown: Shown,
) -> Cell<'a> {
    let kind = column.kind;
    let styled = |text: std::borrow::Cow<'a, str>, style| Cell {
        text,
        style,
        ..Default::default()
    };
    if value.is_null() {
        return Cell {
            text: "NULL".into(),
            null: true,
            ..Default::default()
        };
    }
    if let tabletist_db::Value::Bytes(bytes) = value {
        // Sixteen bytes read as the UUID they hold, as a value does.
        if format::uuid(bytes).is_some() {
            return styled(format::cell_text(value), Style::Plain);
        }
        // Its type and size, never its bytes: a chip, or the terminal's
        // muted words.
        let label = format::binary_label(&column.type_name, bytes.len(), look.terminal);
        let style = if look.terminal {
            Style::Quiet
        } else {
            Style::Chip
        };
        return styled(label.into(), style);
    }
    if let Some(color) = format::color(value) {
        return styled(format::cell_text(value), Style::Color(color));
    }
    if let Some(doc) =
        crate::ui::json_view::document(ctx, kind, value, crate::ui::json_view::CELL_MAX)
    {
        let (count, strings) = crate::ui::json_view::summary(&doc);
        let glance = if look.terminal {
            strings.join(" · ")
        } else {
            strings.into_iter().next().unwrap_or_default()
        };
        return styled(
            format::one_line(&glance).into_owned().into(),
            Style::Json(count),
        );
    }
    let grouped = kind == ValueKind::Numeric && shown.grouped;
    let text = match value {
        tabletist_db::Value::Text(text) => {
            let marks = grid::marks(ctx, look);
            // An empty string and a blank one say what they are.
            if let Some(blank) = format::blank_text(text, marks) {
                return styled(blank.into(), Style::Quiet);
            }
            // A number is grouped before it is cut to a cell's length: one
            // cut first ends in an ellipsis, which is no number to group.
            let line = match grouped.then(|| format::group_number(text)) {
                Some(std::borrow::Cow::Owned(number)) => {
                    format::cell_line(&number, marks).into_owned().into()
                }
                _ => format::cell_line(text, marks),
            };
            if format::is_array(&column.type_name, kind) {
                if line == "{}" {
                    return styled(line, Style::Quiet);
                }
                if format::array_items(&line).is_some() {
                    return styled(line, Style::Array);
                }
            }
            line
        }
        other => {
            let text = format::cell_text(other);
            match grouped.then(|| format::group_number(&text)) {
                Some(std::borrow::Cow::Owned(number)) => number.into(),
                _ => text,
            }
        }
    };
    let text = if kind == ValueKind::Temporal && !shown.full_precision {
        match format::to_the_second(&text) {
            std::borrow::Cow::Borrowed(_) => text,
            std::borrow::Cow::Owned(short) => short.into(),
        }
    } else {
        text
    };
    styled(text, Style::Plain)
}

/// The error a view shows in place of what `fetch` holds: see
/// [`crate::model::Fetch::shown_error`].
pub fn shown_error<T>(fetch: &crate::model::Fetch<T>) -> Option<&tabletist_db::Error> {
    fetch.shown_error()
}

/// An error as a card with its code, detail and hint, and Retry and
/// Copy details buttons. The card shows what a database said as
/// [`format::capped`] cuts it, and scrolls in the room over the buttons,
/// which stay in reach under an error of any length. Copy details copies
/// all of it.
pub fn error_box(
    ui: &mut egui::Ui,
    error: &tabletist_db::Error,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
    mut retry: impl FnMut(),
) {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    // What the card shows, cut without a copy of all of it first: a
    // database's message can hold megabytes.
    let shown = match error {
        tabletist_db::Error::Query { message, .. } => format::capped(message),
        other => format::capped(&other.to_string()).into_owned().into(),
    };
    // What else the database said, each under its name.
    let mut more = Vec::new();
    if let tabletist_db::Error::Query {
        code, detail, hint, ..
    } = error
    {
        for (label, said) in [("Code", code), ("Detail", detail), ("Hint", hint)] {
            if let Some(said) = said {
                more.push((gettext(locale, label), said.as_str()));
            }
        }
    }
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
        let height = states::button_height(look);
        // The room the buttons leave, and no more: a scroll area keeps
        // 64 pt by itself, which would push them out of a short view.
        let room = (ui.available_height() - height - ui.spacing().item_spacing.y).max(0.0);
        egui::ScrollArea::vertical()
            .id_salt("error")
            .max_height(room)
            .min_scrolled_height(0.0)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let card = states::Card {
                    tone: states::Tone::Danger,
                    icon: Icon::CircleAlert,
                    title: &shown,
                    text: "",
                };
                states::card(ui, &card, look, palette);
                let width = ui.available_width();
                for (label, said) in &more {
                    let line = format!("{label}: {}", format::capped(said));
                    Text::one(look, widgets::body(look), &line, palette.secondary)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                }
            });
        ui.horizontal(|ui| {
            let (again, copy) = (say("Retry"), say("Copy details"));
            let again = states::button(&again, look).label("Retry");
            if again.show(ui, height, look, palette).clicked() {
                retry();
            }
            let copy = states::button(&copy, look).label("Copy details").quiet();
            if copy.show(ui, height, look, palette).clicked() {
                // All of it, put together only now: only what the card
                // shows is cut.
                let mut details = error.to_string();
                for (label, said) in &more {
                    details.push_str(&format!("\n{label}: {said}"));
                }
                ui.ctx().copy_text(details);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::{ColumnMeta, RowPage, Value};

    /// Bookshop's `book_images`: 13 rows of 6 columns.
    fn book_images() -> RowPage {
        let column = |name: &str, type_name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind,
        };
        RowPage {
            columns: vec![
                column("id", "int8", ValueKind::Numeric),
                column("book_id", "int8", ValueKind::Numeric),
                column("kind", "varchar", ValueKind::Text),
                column("image_data", "jsonb", ValueKind::Json),
                column("created_at", "timestamp", ValueKind::Temporal),
                column("deleted_at", "timestamp", ValueKind::Temporal),
            ],
            rows: (0..13i64)
                .map(|i| {
                    vec![
                        Value::Int(i + 2),
                        Value::Int(1_048_576 + i * 7_919),
                        Value::Text("cover".into()),
                        Value::Text(r#"{"storage": "store"}"#.into()),
                        Value::Text("2026-06-03 15:47:52.977704".into()),
                        Value::Null,
                    ]
                })
                .collect(),
            has_more: false,
            ordered_by_key: true,
            elapsed: std::time::Duration::ZERO,
        }
    }

    #[test]
    fn the_header_gives_way_instead_of_overlapping_in_a_narrow_view() {
        use crate::testing::{Harness, bounds};
        use egui::accesskit::Role;
        for look in Look::ALL {
            // A 1000 pt window with the row panel open leaves the grid
            // about 400 pt.
            let mut harness = Harness::with_size(vec2(1000.0, 650.0));
            harness.set_look(look);
            let tab = harness.connect_fake();
            harness.app.apply(Action::OpenObject {
                tab,
                object: tabletist_db::ObjectRef::new("public", "book_images"),
                kind: tabletist_db::ObjectKind::Table,
                pin: true,
            });
            harness.answer_rows(book_images());
            let object_tab = harness.app.workspace(tab).unwrap().active_tab.unwrap();
            harness.app.apply(Action::SelectCell {
                tab,
                id: object_tab,
                cell: crate::model::CellPos { row: 4, col: 0 },
            });
            let tree = harness.settle();
            let summary = if look.terminal {
                "13 rows · 6 cols"
            } else {
                "13 rows · 6 columns · public"
            };
            let mut parts = vec![("Data", bounds(&tree, "Data", Role::Button))];
            parts.push(("Structure", bounds(&tree, "Structure", Role::Button)));
            assert!(
                parts.iter().all(|(_, rect)| rect.is_some()),
                "the switch is missing in {}",
                look.name
            );
            // The rest may give way, but what is drawn must not overlap.
            parts.push(("the title", bounds(&tree, "book_images", Role::Label)));
            parts.push(("the summary", bounds(&tree, summary, Role::Label)));
            parts.push(("Add row", bounds(&tree, "Add row", Role::Button)));
            let parts: Vec<(&str, Rect)> = parts
                .into_iter()
                .filter_map(|(name, rect)| Some((name, rect?)))
                .collect();
            for (i, (a, first)) in parts.iter().enumerate() {
                for (b, second) in &parts[i + 1..] {
                    assert!(
                        !first.shrink(0.5).intersects(second.shrink(0.5)),
                        "{a} at {first:?} overlaps {b} at {second:?} in {}",
                        look.name
                    );
                }
            }
        }
    }

    /// A context with every look's faces that has drawn a frame: its fonts
    /// are there to ask for the marks. No system fonts, as in every test.
    fn context() -> egui::Context {
        let ctx = egui::Context::default();
        let mut fonts = fastframe_fonts::FontSetup::default()
            .system_fallbacks(false)
            .definitions();
        for look in Look::ALL {
            crate::typography::configure(&mut fonts, &look, false);
        }
        ctx.set_fonts(fonts);
        let mut output = ctx.run_ui(egui::RawInput::default(), |_| {});
        output.textures_delta.clear();
        ctx
    }

    /// A result column of `kind`, its type named `type_name`.
    fn meta(type_name: &str, kind: ValueKind) -> tabletist_db::ColumnMeta {
        tabletist_db::ColumnMeta {
            name: "column".into(),
            type_name: type_name.into(),
            kind,
        }
    }

    #[test]
    fn a_cell_says_what_a_value_with_nothing_to_show_is() {
        // Without the system's fonts the marks are the plain ones.
        let ctx = context();
        let cell = |value: &Value, column: &tabletist_db::ColumnMeta, look: &Look| {
            let cell = plain_cell(&ctx, value, column, look, Shown::default());
            (cell.text.into_owned(), cell.style)
        };
        let text = |text: &str| Value::Text(text.into());
        let words = meta("text", ValueKind::Text);
        for look in Look::ALL {
            let marks = grid::marks(&ctx, &look);
            let (space, line) = (marks.space, marks.line);
            assert_eq!(cell(&text(""), &words, &look), ("''".into(), Style::Quiet));
            assert_eq!(
                cell(&text("  "), &words, &look),
                (format!("{space}{space}"), Style::Quiet)
            );
            // Text with a line break stays text, the break in sight.
            assert_eq!(
                cell(&text("Night Train\nPart One"), &words, &look),
                (format!("Night Train{line}Part One"), Style::Plain)
            );
            assert_eq!(
                cell(&text("   Leading spaces kept"), &words, &look),
                ("   Leading spaces kept".into(), Style::Plain)
            );
        }
    }

    #[test]
    fn an_array_and_a_binary_value_are_told_by_their_column() {
        let ctx = context();
        let cell = |value: &Value, column: &tabletist_db::ColumnMeta, look: &Look| {
            let cell = plain_cell(&ctx, value, column, look, Shown::default());
            (cell.text.into_owned(), cell.style)
        };
        let text = |text: &str| Value::Text(text.into());
        let (mac, terminal) = (Look::macos(), Look::omarchy());
        let languages = meta("_text", ValueKind::Other);
        for look in [mac, terminal] {
            assert_eq!(
                cell(&text("{en,fr}"), &languages, &look),
                ("{en,fr}".into(), Style::Array)
            );
            assert_eq!(
                cell(&text("{}"), &languages, &look),
                ("{}".into(), Style::Quiet)
            );
            // What is no array in an array's column is the text it is.
            assert_eq!(
                cell(&text("[0:1]={a,b}"), &languages, &look),
                ("[0:1]={a,b}".into(), Style::Plain)
            );
        }
        // The same text in a column of another type is only text.
        assert_eq!(
            cell(&text("{en,fr}"), &meta("int4range", ValueKind::Other), &mac),
            ("{en,fr}".into(), Style::Plain)
        );
        let cover = Value::Bytes(vec![0; 49_358].into());
        let bytea = meta("bytea", ValueKind::Binary);
        assert_eq!(
            cell(&cover, &bytea, &mac),
            ("bytea · 48.2 KB".into(), Style::Chip)
        );
        assert_eq!(
            cell(&cover, &bytea, &terminal),
            ("bytea 48.2K".into(), Style::Quiet)
        );
    }

    #[test]
    fn a_plain_cell_reads_as_its_value_does_in_any_grid() {
        let ctx = context();
        let mac = Look::macos();
        let cell = |value: &Value, kind, look: &Look, full| {
            let shown = Shown {
                full_precision: full,
                ..Shown::default()
            };
            let cell = plain_cell(&ctx, value, &meta("", kind), look, shown);
            (cell.text.into_owned(), cell.null, cell.style)
        };
        assert_eq!(
            cell(&Value::Null, ValueKind::Text, &mac, false),
            ("NULL".into(), true, Style::Plain)
        );
        // A colour wins over what else its column could be.
        let blue = Style::Color(egui::Color32::from_rgb(0x3a, 0x7b, 0xd5));
        assert_eq!(
            cell(&Value::Text("#3a7bd5".into()), ValueKind::Json, &mac, false),
            ("#3a7bd5".into(), false, blue)
        );
        // A document at a glance: its keys counted, its strings shown (the
        // first on macOS, all of them in the terminal).
        let document = Value::Text(r#"{"storage": "store", "kind": "cover"}"#.into());
        assert_eq!(
            cell(&document, ValueKind::Json, &mac, false),
            ("store".into(), false, Style::Json(2))
        );
        assert_eq!(
            cell(&document, ValueKind::Json, &Look::omarchy(), false),
            ("store · cover".into(), false, Style::Json(2))
        );
        // A text column holds a document too when its value is one.
        assert_eq!(
            cell(&document, ValueKind::Text, &mac, false),
            ("store".into(), false, Style::Json(2))
        );
        // The same text in a column of another kind is plain text.
        let (text, _, style) = cell(&document, ValueKind::Other, &mac, false);
        assert_eq!(
            (text.as_str(), style),
            (r#"{"storage": "store", "kind": "cover"}"#, Style::Plain)
        );
        // A timestamp to the second, until asked for all of it.
        let at = Value::Text("2026-06-03 15:47:52.977704".into());
        assert_eq!(
            cell(&at, ValueKind::Temporal, &mac, false).0,
            "2026-06-03 15:47:52"
        );
        assert_eq!(
            cell(&at, ValueKind::Temporal, &mac, true).0,
            "2026-06-03 15:47:52.977704"
        );
        assert_eq!(
            cell(&Value::Int(42), ValueKind::Numeric, &mac, false),
            ("42".into(), false, Style::Plain)
        );
    }

    #[test]
    fn a_number_is_grouped_only_when_asked() {
        let ctx = context();
        let look = Look::macos();
        let text = |value: &Value, kind, shown| {
            plain_cell(&ctx, value, &meta("", kind), &look, shown)
                .text
                .into_owned()
        };
        let grouped = Shown {
            grouped: true,
            ..Shown::default()
        };
        let amount = Value::Text("1240.50".into());
        assert_eq!(
            text(&Value::Int(1_234_567), ValueKind::Numeric, grouped),
            "1,234,567"
        );
        assert_eq!(text(&amount, ValueKind::Numeric, grouped), "1,240.50");
        assert_eq!(
            text(&Value::Int(1_234_567), ValueKind::Numeric, Shown::default()),
            "1234567"
        );
        // Digits that are not a number's: a text column's stay as they are.
        assert_eq!(
            text(&Value::Text("1234567".into()), ValueKind::Text, grouped),
            "1234567"
        );
    }

    #[test]
    fn a_number_too_long_for_a_cell_is_grouped_before_it_is_cut() {
        let ctx = context();
        let look = Look::macos();
        let grouped = Shown {
            grouped: true,
            ..Shown::default()
        };
        // An exact numeric of more digits than a cell shows.
        let long = Value::Text("1".repeat(format::CELL_MAX_CHARS + 44).into());
        let cell = plain_cell(&ctx, &long, &meta("", ValueKind::Numeric), &look, grouped);
        assert!(cell.text.starts_with("111,111,111,"), "{}", cell.text);
        assert!(cell.text.ends_with('…'), "{}", cell.text);
        assert_eq!(cell.text.chars().count(), format::CELL_MAX_CHARS + 1);
    }

    #[test]
    fn a_key_is_a_column_of_the_primary_key_or_of_a_foreign_one() {
        let structure = tabletist_db::Structure {
            primary_key: vec!["id".into()],
            foreign_keys: vec![tabletist_db::ForeignKeyInfo {
                columns: vec!["publisher_id".into()],
                ..Default::default()
            }],
            ..Default::default()
        };
        assert!(is_key("id", Some(&structure)));
        assert!(is_key("publisher_id", Some(&structure)));
        assert!(!is_key("price", Some(&structure)));
        // Not described yet: nothing is known to be a key.
        assert!(!is_key("id", None));
    }

    #[test]
    fn a_tag_comes_after_null_and_a_colour_and_before_the_rest() {
        use crate::ui::value_tags::Tags;
        let ctx = context();
        let look = Look::macos();
        let allowed = ["#fff".to_owned(), "cover".to_owned(), "{}".to_owned()];
        let tags = Tags::Values(&allowed);
        let style = |value: &Value, kind, tags: &Tags<'_>| {
            let cell = cell(&ctx, value, &meta("", kind), tags, &look, Shown::default());
            (cell.null, cell.style)
        };
        let text = |text: &str| Value::Text(text.into());
        assert_eq!(
            style(&Value::Null, ValueKind::Text, &tags),
            (true, Style::Plain)
        );
        // An allowed value that is a colour draws as the colour.
        let white = Style::Color(egui::Color32::WHITE);
        assert_eq!(style(&text("#fff"), ValueKind::Text, &tags), (false, white));
        assert_eq!(
            style(&text("cover"), ValueKind::Text, &tags),
            (false, Style::Tag(1))
        );
        // One that is a document draws as the tag, not the document.
        assert_eq!(
            style(&text("{}"), ValueKind::Json, &tags),
            (false, Style::Tag(2))
        );
        // A value the list does not name, and a column with no list, are
        // plain; a boolean is a tag with no list at all.
        assert_eq!(
            style(&text("preview"), ValueKind::Text, &tags),
            (false, Style::Plain)
        );
        assert_eq!(
            style(&text("cover"), ValueKind::Text, &Tags::None),
            (false, Style::Plain)
        );
        assert_eq!(
            style(&Value::Bool(true), ValueKind::Bool, &Tags::Bool),
            (false, Style::Tag(0))
        );
    }

    #[test]
    fn the_header_names_keys_and_where_foreign_keys_point() {
        let structure = tabletist_db::Structure {
            primary_key: vec!["id".into()],
            foreign_keys: vec![tabletist_db::ForeignKeyInfo {
                name: None,
                columns: vec!["book_id".into()],
                ref_schema: "public".into(),
                ref_table: "books".into(),
                ref_columns: vec!["id".into()],
                on_update: String::new(),
                on_delete: String::new(),
            }],
            ..Default::default()
        };
        let mac = Look::macos();
        let terminal = Look::omarchy();
        let line =
            |name, type_name, kind, look| type_line(name, type_name, kind, Some(&structure), look);
        assert_eq!(
            line("id", "int8", ValueKind::Numeric, &mac),
            ("int8".into(), true)
        );
        assert_eq!(
            line("id", "int8", ValueKind::Numeric, &terminal),
            ("pk".into(), true)
        );
        assert_eq!(
            line("book_id", "int8", ValueKind::Numeric, &mac),
            ("int8 → books".into(), false)
        );
        assert_eq!(
            line("book_id", "int8", ValueKind::Numeric, &terminal),
            ("→ books".into(), false)
        );
        assert_eq!(
            line("at", "timestamp", ValueKind::Temporal, &mac),
            ("timestamp · no tz".into(), false)
        );
    }
}
