//! A bordered list of files, a row each: the file's icon, its path with
//! the file's name in the text colour and its folder dimmer, its type, its
//! size, and × that takes it off the list. A row whose file is not there
//! says so where its size would be. The Files connection's Source draws it
//! in both looks; the terminal look marks the keyboard's row as its other
//! lists do.

use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::i18n::{Locale, gettext};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// One file, as its row says it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The path as the app shows it (`~` for the home folder).
    pub path: String,
    /// Its format's name (`CSV`), or empty where its name says none.
    pub kind: String,
    /// Its size (`1.6 GB`), or empty until it is known.
    pub size: String,
    /// The file is not there any more.
    pub missing: bool,
}

/// What a frame of the list was asked: a row's × (the file to take off),
/// or a row clicked (the keyboard's row from then on).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Asked {
    pub removed: Option<usize>,
    pub chosen: Option<usize>,
}

/// The row's height, and the widths of the type, size and × columns.
fn metrics(look: &Look) -> (f32, f32, f32, f32) {
    match look.terminal {
        true => (26.0, 80.0, 80.0, 22.0),
        false => (34.0, 86.0, 80.0, 30.0),
    }
}

/// The list of `rows`, `cursor` the keyboard's row. Announced as a list
/// named `name`, each row an item by its path, each × a button
/// "Remove orders.csv".
pub fn show(
    ui: &mut Ui,
    name: &str,
    rows: &[Row],
    cursor: Option<usize>,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Asked {
    let (height, kind_width, size_width, remove_width) = metrics(look);
    let role = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
    let radius = if look.terminal { 0 } else { 8 };
    let width = ui.available_width();
    let mut asked = Asked::default();
    ui.scope_builder(egui::UiBuilder::new().id_salt(name), |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::List);
            node.set_label(name);
        });
        ui.spacing_mut().item_spacing.y = 0.0;
        let top = ui.cursor().top();
        for (index, row) in rows.iter().enumerate() {
            let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::ListItem);
                node.set_label(row.path.as_str());
                if cursor == Some(index) {
                    node.set_selected(true);
                }
            });
            if response.clicked() {
                asked.chosen = Some(index);
            }
            let painter = ui.painter().clone();
            if cursor == Some(index) && look.terminal {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.selection);
                painter.rect_filled(
                    Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
                    CornerRadius::ZERO,
                    palette.accent,
                );
            } else if response.hovered() {
                painter.rect_filled(rect, CornerRadius::ZERO, palette.surface_hover);
            }
            if index > 0 {
                widgets::hline(ui, rect.x_range(), rect.top(), palette.surface_hover);
            }
            let center = rect.center().y;
            let faded = |color: egui::Color32| match row.missing {
                true => color.gamma_multiply(0.6),
                false => color,
            };
            // The icon, then the path, ending before the type.
            let mut left = rect.left() + if look.terminal { 8.0 } else { 10.0 };
            if !look.terminal {
                Icon::File.image(faded(palette.dim), 14.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(left + 7.0, center), vec2(14.0, 14.0)),
                );
                left += 20.0;
            }
            let right = rect.right() - remove_width - size_width - kind_width - 4.0;
            let (folder, file) = match row.path.rfind(['/', '\\']) {
                Some(at) => row.path.split_at(at + 1),
                None => ("", row.path.as_str()),
            };
            let room = (right - left).max(0.0);
            let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
            // The file's name whole where it fits; its folder cut first.
            let file_width = measure(file).min(room);
            let shown_folder = crate::ui::grid::ellipsize(folder, room - file_width, true, measure);
            let mut x = left;
            if !shown_folder.is_empty() {
                x += widgets::paint_label(
                    ui,
                    x,
                    center,
                    Text::one(look, role, &shown_folder, faded(palette.dim)),
                );
            }
            let shown_file = crate::ui::grid::ellipsize(file, room - (x - left), false, measure);
            widgets::paint_label(
                ui,
                x,
                center,
                Text::one(look, role, &shown_file, faded(palette.text)),
            );
            // The type and the size, right-aligned in their columns.
            let kind_right = right + kind_width;
            Text::one(look, small, &row.kind, faded(palette.dim))
                .layout(ui.ctx())
                .paint_right(&painter, kind_right - 10.0, center);
            let (size, size_color) = match row.missing {
                true => (gettext(locale, "missing").into_owned(), palette.danger),
                false => (row.size.clone(), palette.dim),
            };
            Text::one(look, small, &size, size_color)
                .layout(ui.ctx())
                .paint_right(&painter, kind_right + size_width - 10.0, center);
            // ×, which takes the file off the list and never off the disk.
            let file_name = file.trim();
            let label = format!("{} {file_name}", gettext(locale, "Remove"));
            let button = Rect::from_center_size(
                pos2(rect.right() - remove_width / 2.0 - 2.0, center),
                vec2(remove_width - 4.0, height - 8.0),
            );
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(button));
            if widgets::icon_button_sized(
                &mut child,
                Icon::X,
                &label,
                button.size(),
                12.0,
                look,
                palette,
            )
            .clicked()
            {
                asked.removed = Some(index);
            }
        }
        let bottom = ui.cursor().top();
        let outline = Rect::from_min_max(
            pos2(ui.max_rect().left(), top),
            pos2(ui.max_rect().left() + width, bottom.max(top + 1.0)),
        );
        ui.painter().rect_stroke(
            outline,
            CornerRadius::same(radius),
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
    });
    asked
}

/// The table names as small tags in a line that wraps, as the found line
/// of a Files connection shows them on macOS.
pub fn tags(ui: &mut Ui, names: &[&str], look: &Look, palette: &Palette) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for name in names {
            let laid =
                Text::one(look, TextRole::MonoSecondary, name, palette.secondary).layout(ui.ctx());
            let size = vec2(laid.width() + 14.0, 20.0);
            let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
            ui.painter()
                .rect_filled(rect, CornerRadius::same(6), palette.surface_hover);
            laid.paint_left(ui.painter(), rect.left() + 7.0, rect.center().y);
            widgets::announce(ui, rect, name);
        }
    });
}
