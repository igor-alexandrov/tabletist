//! The sidebar: filter, refresh, and the tree of schemas and objects.

use egui::{
    Align2, CornerRadius, Frame, Id, Margin, RichText, Sense, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::ObjectKind;

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, TreeNode, TreeRow};
use crate::theme::{self, Icon};
use crate::ui::widgets::{icon_button, virtual_rows};

const INDENT: f32 = 14.0;

fn group_label(locale: Locale, kind: ObjectKind, count: usize) -> String {
    let name = match kind {
        ObjectKind::Table => gettext(locale, "Tables"),
        ObjectKind::View => gettext(locale, "Views"),
        ObjectKind::MaterializedView => gettext(locale, "Materialized views"),
    };
    format!("{name} ({count})")
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let show_system = app.settings.show_system_schemas;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let rows = workspace.tree.visible_rows(workspace.driver, show_system);
    let schemas_loading =
        workspace.tree.schemas.is_loading() && workspace.tree.schemas.value.is_none();
    // Loaded, and nothing to show (none, or only system schemas hidden).
    let nothing = workspace.tree.schemas.value.is_some()
        && rows.is_empty()
        && workspace.tree.filter.trim().is_empty();
    // The top bar offers other databases (PostgreSQL with more than one).
    let other_databases = workspace
        .databases
        .value
        .as_ref()
        .is_some_and(|databases| databases.len() > 1);
    let schemas_error = workspace
        .tree
        .schemas
        .error
        .as_ref()
        .map(ToString::to_string);
    let active = workspace
        .active_object_tab()
        .map(|object| object.object.clone());
    // The keyboard cursor, drawn only while the tree has the arrows.
    let cursor = workspace
        .tree
        .cursor
        .clone()
        .filter(|_| workspace.pane == crate::model::Pane::Tree);
    // After a key moved the cursor, scroll its row into view.
    let reveal = workspace
        .tree
        .reveal_cursor
        .then(|| {
            let cursor = workspace.tree.cursor.as_ref()?;
            rows.iter().position(|row| &row.node == cursor)
        })
        .flatten();
    let reveal_pending = workspace.tree.reveal_cursor;
    let mut actions = Vec::new();

    egui::Panel::left(Id::new(("sidebar", tab.0)))
        .resizable(true)
        .default_size(240.0)
        .size_range(180.0..=400.0)
        .frame(
            Frame::new()
                .fill(if look.sidebar_tinted {
                    palette.panel
                } else {
                    palette.window
                })
                .inner_margin(Margin::same(8)),
        )
        .show_separator_line(true)
        .show(ui, |ui| {
            // Right to left: the button takes its space first and the field
            // exactly the rest (margins included), so nothing asks the
            // panel for more width than it has; a resizable panel would
            // grow to fit, a little more every frame.
            let row = egui::vec2(ui.available_width(), look.control_height);
            let layout = egui::Layout::right_to_left(egui::Align::Center);
            ui.allocate_ui_with_layout(row, layout, |ui| {
                if icon_button(
                    ui,
                    Icon::RefreshCw,
                    &gettext(locale, "Refresh objects"),
                    &look,
                    &palette,
                )
                .clicked()
                {
                    actions.push(Action::RefreshTree(tab));
                }
                if let Some(workspace) = app.workspace_mut(tab) {
                    let size = egui::vec2(ui.available_width(), look.control_height);
                    let field = crate::ui::widgets::search_field(
                        ui,
                        &mut workspace.tree.filter,
                        &gettext(locale, "Filter"),
                        &look,
                    );
                    ui.allocate_ui(size, |ui| {
                        ui.set_min_size(size);
                        crate::ui::widgets::add_search(ui, field.desired_width(size.x), &look)
                    });
                }
            });
            ui.add_space(6.0);
            if schemas_loading {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(gettext(locale, "Loading…"));
                });
            }
            if nothing {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.label(
                        RichText::new(gettext(locale, "This database has no schemas you can see."))
                            .color(palette.secondary),
                    );
                    ui.label(
                        RichText::new(gettext(
                            locale,
                            "The connected user may lack the privileges to list them.",
                        ))
                        .small()
                        .color(palette.secondary),
                    );
                    if other_databases {
                        ui.label(
                            RichText::new(gettext(
                                locale,
                                "Or pick another database in the top bar.",
                            ))
                            .small()
                            .color(palette.secondary),
                        );
                    }
                    ui.add_space(6.0);
                    if ui.button(gettext(locale, "Refresh")).clicked() {
                        actions.push(Action::RefreshTree(tab));
                    }
                });
            }
            if let Some(error) = schemas_error {
                ui.label(RichText::new(error).color(palette.danger));
                if ui.button(gettext(locale, "Retry")).clicked() {
                    actions.push(Action::RefreshTree(tab));
                }
            }
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let top = ui.cursor().top();
                    if let Some(index) = reveal {
                        let y = top + index as f32 * look.tree_row;
                        let row = egui::Rect::from_min_size(
                            pos2(ui.cursor().left(), y),
                            vec2(1.0, look.tree_row),
                        );
                        // Jump, not animate: the next key may come at once.
                        ui.scroll_to_rect_animation(
                            row,
                            None,
                            egui::style::ScrollAnimation::none(),
                        );
                    }
                    virtual_rows(ui, rows.len(), look.tree_row, |ui, index| {
                        tree_row(
                            ui,
                            &rows[index],
                            tab,
                            Marks {
                                active: active.as_ref(),
                                cursor: cursor.as_ref(),
                            },
                            locale,
                            Skin {
                                look: &look,
                                palette: &palette,
                            },
                            &mut actions,
                        );
                    });
                });
        });
    if reveal_pending && let Some(workspace) = app.workspace_mut(tab) {
        workspace.tree.reveal_cursor = false;
    }
    app.actions.extend(actions);
}

/// Which rows are highlighted: the open object and the keyboard cursor.
#[derive(Clone, Copy)]
struct Marks<'a> {
    active: Option<&'a tabletist_db::ObjectRef>,
    cursor: Option<&'a TreeNode>,
}

/// How rows are drawn: the platform look and the colour palette.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a crate::theme::Look,
    palette: &'a crate::theme::Palette,
}

/// Where a tree row at `depth` draws in its `rect`: the highlight and
/// keyboard cursor, and the left edge of its chevron. Content starts inside
/// the highlight, so the macOS pill holds a top-level chevron.
fn row_frame(rect: egui::Rect, depth: u8, look: &crate::theme::Look) -> (egui::Rect, f32) {
    let highlight = crate::ui::widgets::selection_rect(rect, look);
    (
        highlight,
        highlight.left() + 4.0 + f32::from(depth) * INDENT,
    )
}

fn tree_row(
    ui: &mut egui::Ui,
    row: &TreeRow,
    tab: ConnTabId,
    marks: Marks<'_>,
    locale: Locale,
    skin: Skin<'_>,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette } = skin;
    // A quiet note, not a row: the text is never clickable, and a small
    // link after it reloads the tree in case objects appeared since. In a
    // narrow sidebar the note gives way ("…") so the link stays in view.
    if matches!(row.node, TreeNode::Empty(_)) {
        let size = vec2(ui.available_width(), look.tree_row);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(size, layout, |ui| {
            ui.set_min_size(size);
            let (_, left) = row_frame(ui.max_rect(), row.depth, look);
            ui.add_space(left + 18.0 - ui.max_rect().left());
            let small = theme::regular(theme::TEXT_SMALL);
            let refresh = RichText::new(gettext(locale, "Refresh"))
                .font(small.clone())
                .color(palette.accent);
            let link_width = ui
                .painter()
                .layout_no_wrap(refresh.text().to_owned(), small.clone(), palette.accent)
                .size()
                .x;
            let room = ui.available_width() - link_width - ui.spacing().item_spacing.x;
            ui.scope(|ui| {
                ui.set_max_width(room.max(0.0));
                ui.add(
                    egui::Label::new(
                        RichText::new(gettext(locale, "No tables or views"))
                            .font(small)
                            .color(palette.secondary),
                    )
                    .truncate(),
                );
            });
            if ui.link(refresh).clicked() {
                actions.push(Action::RefreshTree(tab));
            }
        });
        return;
    }
    let (rect, response) =
        ui.allocate_exact_size(vec2(ui.available_width(), look.tree_row), Sense::click());
    let label = match &row.node {
        TreeNode::Group(_, kind) => group_label(locale, *kind, row.count.unwrap_or(0)),
        _ => row.label.clone(),
    };
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &label));
    let selected = matches!(&row.node, TreeNode::Object(object, _) if Some(object) == marks.active);
    crate::ui::widgets::selection(ui, rect, selected, response.hovered(), look, palette);
    let (highlight, left) = row_frame(rect, row.depth, look);
    if marks.cursor == Some(&row.node) {
        ui.painter().rect_stroke(
            highlight.shrink(0.5),
            CornerRadius::same(look.radius),
            egui::Stroke::new(1.0, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
    if let Some(expanded) = row.expanded {
        let icon = if expanded {
            Icon::ChevronDown
        } else {
            Icon::ChevronRight
        };
        icon.image(palette.secondary, 14.0).paint_at(
            ui,
            egui::Rect::from_center_size(pos2(left + 7.0, rect.center().y), vec2(14.0, 14.0)),
        );
    }
    let (font, color) = match row.node {
        TreeNode::Schema(_) => (theme::medium(theme::TEXT), palette.text),
        TreeNode::Group(..) => (theme::regular(theme::TEXT_SMALL), palette.secondary),
        TreeNode::Object(..) | TreeNode::Empty(_) => (
            theme::regular(theme::TEXT),
            crate::ui::widgets::selection_text(selected, look, palette),
        ),
    };
    // Long names end in "…" and show in full on hover.
    // Room for the spinner or error dot only when there is one.
    let marker = if row.loading || row.error.is_some() {
        22.0
    } else {
        6.0
    };
    let room = highlight.right() - (left + 18.0) - marker;
    let shown = crate::ui::grid::ellipsize(&label, room, false, |text| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), color)
            .size()
            .x
    });
    if shown != label {
        response.clone().on_hover_text(&label);
    }
    ui.painter().with_clip_rect(rect).text(
        pos2(left + 18.0, rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        font,
        color,
    );
    if row.loading {
        let spinner = egui::Rect::from_center_size(
            pos2(highlight.right() - 10.0, rect.center().y),
            vec2(12.0, 12.0),
        );
        egui::Spinner::new().size(12.0).paint_at(ui, spinner);
    }
    if let Some(error) = &row.error {
        response.clone().on_hover_text(error);
        ui.painter().circle_filled(
            pos2(highlight.right() - 10.0, rect.center().y),
            4.0,
            palette.danger,
        );
    }
    match &row.node {
        TreeNode::Schema(schema) if response.clicked() => {
            actions.push(Action::ToggleSchema {
                tab,
                schema: schema.clone(),
            });
        }
        TreeNode::Group(schema, kind) if response.clicked() => {
            actions.push(Action::ToggleGroup {
                tab,
                schema: schema.clone(),
                kind: *kind,
            });
        }
        TreeNode::Object(object, kind) => {
            if response.double_clicked() {
                actions.push(Action::OpenObject {
                    tab,
                    object: object.clone(),
                    kind: *kind,
                    pin: true,
                });
            } else if response.clicked() {
                actions.push(Action::OpenObject {
                    tab,
                    object: object.clone(),
                    kind: *kind,
                    pin: false,
                });
            }
        }
        _ => {}
    }
    // A click also puts the keyboard cursor here and gives the tree the
    // arrows (after the row's own action, which may give them to the grid).
    if response.clicked() {
        actions.push(Action::SetTreeCursor {
            tab,
            node: row.node.clone(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chevrons_and_the_cursor_sit_inside_the_highlight() {
        let rect = egui::Rect::from_min_size(pos2(0.0, 0.0), vec2(240.0, 26.0));
        for look in crate::theme::Look::ALL {
            let (highlight, left) = row_frame(rect, 0, &look);
            assert_eq!(
                highlight,
                crate::ui::widgets::selection_rect(rect, &look),
                "{}",
                look.name
            );
            assert!(
                left >= highlight.left() + 2.0,
                "{}: the chevron at {left} starts outside the highlight at {}",
                look.name,
                highlight.left()
            );
        }
    }
}
