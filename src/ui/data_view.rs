//! The open object: its header (name, counts, the Data/Structure switch),
//! the toolbar (filters and sort), the grid or its error or empty state,
//! and the status footer.

use egui::{
    CornerRadius, Frame, Id, Margin, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType, pos2,
    vec2,
};
use tabletist_db::{SortDir, ValueKind};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectTab, ObjectTabId, ObjectView};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::grid::{self, Cell, Column, Style};
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
        parts.push(object.object.schema.clone());
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
fn paint_named(ui: &egui::Ui, x: f32, y: f32, text: Text, name: &str) -> f32 {
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
pub fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let name = object.object.name.clone();
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
            for ((target, label, _), width) in views.iter().zip(widths) {
                let cell = Rect::from_min_size(pos2(x, track.top() + 3.0), vec2(width, 28.0));
                x += width;
                let response =
                    ui.interact(cell, ui.id().with(("view", label.as_ref())), Sense::click());
                let selected = view == *target;
                response.widget_info(|| {
                    WidgetInfo::selected(WidgetType::Button, true, selected, label.as_ref())
                });
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
                text.paint(ui.painter(), cell.center() - text.size() / 2.0);
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

/// Above the grid: Add filter and the filters in use (macOS), or the
/// terminal's WHERE line; the sort; and how timestamps are shown.
pub fn toolbar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
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
    let filters: Vec<String> = object
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
        .collect();
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
            }
            if let Some((column, dir)) = &sort {
                let (_, cleared) =
                    sort_chip(ui, None, x, center, column, *dir, &look, &palette, locale);
                if cleared {
                    actions.push(Action::ClearSort { tab, object_tab });
                }
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
                let link_width = small.width(ui.ctx(), look.faces, &link);
                let hit = Rect::from_min_size(
                    pos2(right - link_width, center - 9.0),
                    vec2(link_width, 18.0),
                );
                let response = ui.interact(hit, ui.id().with("precision"), Sense::click());
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, link.as_ref()));
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
                // "… second · ", the space before the link kept out of the text.
                let space = small.width(ui.ctx(), look.faces, " ");
                widgets::paint_text_right(
                    ui,
                    hit.left() - space,
                    center,
                    Text::one(&look, small, &format!("{said} ·"), palette.dim),
                );
                if response.clicked() {
                    actions.push(Action::ToggleFullPrecision(tab));
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
    object_tab: ObjectTabId,
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

/// The status footer: the page's range and paging, then what is selected
/// and how long the query took.
pub fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(object) = app.workspace(tab).and_then(|w| w.object_tab(object_tab)) else {
        return;
    };
    let view = object.view;
    let loading =
        object.rows.is_loading() || object.structure.is_loading() || object.count.is_loading();
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
                            if loading {
                                gettext(locale, "Loading…")
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
                    if loading {
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
                    let state = if selected {
                        format!(
                            "{} · {}",
                            gettext(locale, "1 row selected"),
                            gettext(locale, "read-only")
                        )
                    } else {
                        gettext(locale, "read-only").into_owned()
                    };
                    note(ui, &state, status, &look);
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

/// What the grid's header says under a column's name.
fn type_line(
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
        _ => type_name.to_owned(),
    };
    let line = match (look.terminal, key, target) {
        (true, true, _) => "pk".to_owned(),
        (true, _, Some(target)) => format!("→ {target}"),
        (false, _, Some(target)) => format!("{base} → {target}"),
        _ => base,
    };
    (line, key)
}

/// Text columns holding a handful of short values (a status, a kind) show
/// them as tags: each distinct value's colour index, by name.
pub(crate) fn tag_hues(page: &tabletist_db::RowPage, col: usize) -> Option<Vec<Box<str>>> {
    if page.columns[col].kind != ValueKind::Text || page.rows.len() < 4 {
        return None;
    }
    let mut distinct: Vec<Box<str>> = Vec::new();
    for row in &page.rows {
        match &row[col] {
            tabletist_db::Value::Null => {}
            tabletist_db::Value::Text(text) => {
                if text.chars().count() > 16
                    || text.contains(char::is_whitespace)
                    || text.is_empty()
                {
                    return None;
                }
                if !distinct.contains(text) {
                    distinct.push(text.clone());
                    if distinct.len() > 6 {
                        return None;
                    }
                }
            }
            _ => return None,
        }
    }
    (distinct.len() >= 2 && distinct.len() < page.rows.len()).then(|| {
        distinct.sort();
        distinct
    })
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: ObjectTabId) {
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
    let mut actions = Vec::new();
    if let Some(error) = &object.rows.error {
        Frame::new().inner_margin(Margin::same(12)).show(ui, |ui| {
            error_box(ui, error, &look, &palette, locale, || {
                actions.push(Action::RetryRows { tab, object_tab })
            });
        });
    } else if let Some(page) = object.page() {
        if page.rows.is_empty() {
            let filtered = !object.query.filters.is_empty() || object.query.raw_where.is_some();
            let table = object.kind == tabletist_db::ObjectKind::Table;
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() / 3.0);
                let text = if filtered {
                    gettext(locale, "No rows match the filter")
                } else if table {
                    gettext(locale, "This table is empty")
                } else {
                    gettext(locale, "No rows")
                };
                Text::one(&look, widgets::body(&look), &text, palette.secondary)
                    .layout(ui.ctx())
                    .label(ui);
                if filtered {
                    ui.add_space(8.0);
                    let label = gettext(locale, "Clear filter");
                    if widgets::button(ui, &label, &look).clicked() {
                        actions.push(Action::ClearFilters { tab, object_tab });
                    }
                }
            });
        } else {
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
                    }
                })
                .collect();
            let ctx = ui.ctx().clone();
            let tags: Vec<Option<Vec<Box<str>>>> = (0..page.columns.len())
                .map(|col| tag_hues(page, col))
                .collect();
            let output = grid::show(
                ui,
                // Full precision widens timestamps: the columns fit again.
                Id::new(("grid", tab.0, object_tab.0, full_precision)),
                &columns,
                page.rows.len(),
                object.query.offset,
                object.selection,
                &palette,
                &look,
                |row, col| {
                    let value = &page.rows[row][col];
                    let kind = page.columns[col].kind;
                    if value.is_null() {
                        return Cell {
                            text: "NULL".into(),
                            null: true,
                            style: Style::Plain,
                        };
                    }
                    if let (Some(values), tabletist_db::Value::Text(text)) = (&tags[col], value)
                        && let Some(hue) = values.iter().position(|known| known == text)
                    {
                        return Cell {
                            text: text.as_ref().into(),
                            null: false,
                            style: Style::Tag(hue),
                        };
                    }
                    if let (ValueKind::Json, tabletist_db::Value::Text(text)) = (kind, value)
                        && text.len() <= 16 * 1024
                        && let Some(doc) = crate::ui::json_view::parsed_in(&ctx, text)
                    {
                        let (count, strings) = crate::ui::json_view::summary(&doc);
                        let shown = if look.terminal {
                            strings.join(" · ")
                        } else {
                            strings.into_iter().next().unwrap_or_default()
                        };
                        return Cell {
                            text: shown.into(),
                            null: false,
                            style: Style::Json(count),
                        };
                    }
                    let text = format::cell_text(value);
                    let text = if kind == ValueKind::Temporal && !full_precision {
                        match format::to_the_second(&text) {
                            std::borrow::Cow::Borrowed(_) => text,
                            std::borrow::Cow::Owned(short) => short.into(),
                        }
                    } else {
                        text
                    };
                    Cell {
                        text,
                        null: false,
                        style: Style::Plain,
                    }
                },
            );
            if let Some(cell) = output.clicked {
                actions.push(Action::SelectCell {
                    tab,
                    object_tab,
                    cell,
                });
            }
            if let Some(col) = output.sort_clicked {
                actions.push(Action::SortBy {
                    tab,
                    object_tab,
                    column: page.columns[col].name.clone(),
                });
            }
        }
    } else {
        ui.centered_and_justified(|ui| {
            ui.spinner();
        });
    }
    app.actions.extend(actions);
}

/// An error with its code, detail and hint, and a Retry button.
pub fn error_box(
    ui: &mut egui::Ui,
    error: &tabletist_db::Error,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
    mut retry: impl FnMut(),
) {
    Frame::new()
        .fill(palette.danger.gamma_multiply(0.12))
        .inner_margin(Margin::same(12))
        .corner_radius(egui::CornerRadius::same(look.tab_radius))
        .show(ui, |ui| {
            let width = ui.available_width();
            let line = |ui: &mut egui::Ui, text: &str, color| {
                Text::one(look, widgets::body(look), text, color)
                    .wrap(width)
                    .layout(ui.ctx())
                    .label(ui);
            };
            line(ui, &error.to_string(), palette.text);
            if let tabletist_db::Error::Query {
                code, detail, hint, ..
            } = error
            {
                for (label, text) in [("Code", code), ("Detail", detail), ("Hint", hint)] {
                    if let Some(text) = text {
                        line(
                            ui,
                            &format!("{}: {text}", gettext(locale, label)),
                            palette.secondary,
                        );
                    }
                }
            }
            if widgets::button(ui, &gettext(locale, "Retry"), look).clicked() {
                retry();
            }
        });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tabletist_db::{ColumnMeta, RowPage, Value};

    fn page(values: &[&str]) -> RowPage {
        RowPage {
            columns: vec![ColumnMeta {
                name: "kind".into(),
                type_name: "varchar".into(),
                kind: ValueKind::Text,
            }],
            rows: values
                .iter()
                .map(|value| vec![Value::Text((*value).into())])
                .collect(),
            has_more: false,
            ordered_by_key: true,
            elapsed: std::time::Duration::ZERO,
        }
    }

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
            let object_tab = harness.app.workspace(tab).unwrap().active_object.unwrap();
            harness.app.apply(Action::SelectCell {
                tab,
                object_tab,
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

    #[test]
    fn a_few_short_repeated_values_become_tags() {
        let tags = tag_hues(&page(&["gold", "silver", "gold", "silver"]), 0);
        assert_eq!(tags, Some(vec!["gold".into(), "silver".into()]));
        assert_eq!(
            tag_hues(&page(&["a", "b", "c", "d"]), 0),
            None,
            "all different"
        );
        assert_eq!(tag_hues(&page(&["a b", "a b", "c", "c"]), 0), None, "words");
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
