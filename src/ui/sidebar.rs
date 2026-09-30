//! The sidebar: filter, recent objects, the schema picker, and the shown
//! schema's objects in prefix groups or one flat list.

use egui::{CornerRadius, Id, Rect, RichText, Sense, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::{ObjectKind, ObjectRef};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, TreeNode, TreeRow};
use crate::theme::{self, Icon, Look, Palette};
use crate::ui::widgets::{self, icon_button};

/// The sidebar's width when it opens, per look.
fn default_width(look: &Look) -> f32 {
    if look.terminal { 264.0 } else { 248.0 }
}

/// Space around the filter field: macOS 12 (8 under), terminal 10 (6).
fn margin(look: &Look) -> f32 {
    if look.terminal { 10.0 } else { 12.0 }
}

/// The filter field's height.
fn field_height(look: &Look) -> f32 {
    if look.terminal { 30.0 } else { 34.0 }
}

/// Recent rows (macOS only).
const RECENT_ROW: f32 = 28.0;

/// The tree's inset from the sidebar's sides.
fn tree_inset(look: &Look) -> f32 {
    if look.terminal { 4.0 } else { 8.0 }
}

/// A tree row's height: groups stand taller than their objects.
fn row_height(row: &TreeRow, look: &Look) -> f32 {
    match (&row.node, look.terminal) {
        (TreeNode::Group(..), true) => 26.0,
        (TreeNode::Group(..), false) => 28.0,
        _ => look.tree_row,
    }
}

/// The name kind of an object as the tree tags it, if not a table.
fn kind_tag(locale: Locale, kind: ObjectKind) -> Option<String> {
    match kind {
        ObjectKind::Table => None,
        ObjectKind::View => Some(gettext(locale, "view").into_owned()),
        ObjectKind::MaterializedView => Some(gettext(locale, "mview").into_owned()),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let show_system = app.settings.show_system_schemas;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if workspace.sidebar_hidden {
        return;
    }
    let rows = workspace.tree.visible_rows(workspace.driver, show_system);
    let schemas = workspace
        .tree
        .visible_schemas(workspace.driver, show_system);
    let shown = workspace.tree.shown_schema(workspace.driver, show_system);
    let flat = workspace.tree.flat;
    let shown_loading = shown
        .as_ref()
        .and_then(|schema| workspace.tree.nodes.get(schema))
        .is_some_and(|node| node.objects.is_loading());
    let shown_error = shown
        .as_ref()
        .and_then(|schema| workspace.tree.nodes.get(schema))
        .and_then(|node| node.objects.error.as_ref().map(ToString::to_string));
    let schemas_loading =
        workspace.tree.schemas.is_loading() && workspace.tree.schemas.value.is_none();
    // Loaded, and nothing to show (none, or only system schemas hidden).
    let nothing = workspace.tree.schemas.value.is_some() && schemas.is_empty();
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
    let recent: Vec<(ObjectRef, ObjectKind)> = if look.terminal {
        Vec::new()
    } else {
        workspace.recent.clone()
    };
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

    // The terminal's sidebar takes the theme's dark background too.
    let fill = if look.sidebar_tinted || look.terminal {
        palette.panel
    } else {
        palette.window
    };
    egui::Panel::left(Id::new(("sidebar", tab.0)))
        .resizable(true)
        .default_size(default_width(&look))
        .size_range(160.0..=400.0)
        .frame(egui::Frame::new().fill(fill))
        .show_separator_line(false)
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::vline(ui, full.right() - 0.5, full.y_range(), palette.outline);
            let pad = margin(&look);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.add_space(pad);
            ui.horizontal(|ui| {
                ui.add_space(pad);
                if let Some(workspace) = app.workspace_mut(tab) {
                    let hint = if look.terminal {
                        gettext(locale, "filter objects")
                    } else {
                        gettext(locale, "Find any object…")
                    };
                    let size = vec2(full.width() - 2.0 * pad, field_height(&look));
                    let style = widgets::FieldStyle {
                        fill: Some(palette.window),
                        boxed: true,
                        text_size: if look.terminal { 12.5 } else { theme::TEXT },
                    };
                    widgets::filter_field(
                        ui,
                        &mut workspace.tree.filter,
                        &hint,
                        size,
                        style,
                        &look,
                        &palette,
                    )
                    .widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Filter"));
                }
            });
            ui.add_space(if look.terminal { 6.0 } else { 8.0 });
            if !recent.is_empty() {
                recent_section(
                    ui,
                    &recent,
                    active.as_ref(),
                    tab,
                    &look,
                    &palette,
                    &mut actions,
                );
            }
            schema_header(
                ui,
                tab,
                &schemas,
                shown.as_deref(),
                flat,
                shown_loading,
                SchemaHeader {
                    locale,
                    look: &look,
                    palette: &palette,
                },
                &mut actions,
            );
            if schemas_loading {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    ui.spinner();
                    ui.label(gettext(locale, "Loading…"));
                });
            }
            if nothing {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
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
            for error in [schemas_error, shown_error].into_iter().flatten() {
                ui.horizontal_wrapped(|ui| {
                    ui.add_space(pad);
                    ui.label(RichText::new(error).color(palette.danger));
                });
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    if ui.button(gettext(locale, "Retry")).clicked() {
                        actions.push(Action::RefreshTree(tab));
                    }
                });
            }
            let footer = if look.terminal { FOOTER } else { 0.0 };
            let body = ui.available_height() - footer;
            ui.allocate_ui(vec2(full.width(), body.max(0.0)), |ui| {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                        let top = ui.cursor().top();
                        let heights: Vec<f32> =
                            rows.iter().map(|row| row_height(row, &look)).collect();
                        if let Some(index) = reveal {
                            let y = top + heights[..index].iter().sum::<f32>();
                            let row = Rect::from_min_size(
                                pos2(ui.cursor().left(), y),
                                vec2(1.0, heights[index]),
                            );
                            // Jump, not animate: the next key may come at once.
                            ui.scroll_to_rect_animation(
                                row,
                                None,
                                egui::style::ScrollAnimation::none(),
                            );
                        }
                        widgets::virtual_rows_varying(ui, &heights, |ui, index| {
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
                        ui.add_space(8.0);
                    });
            });
            if look.terminal {
                terminal_footer(ui, full, &palette);
            }
        });
    if reveal_pending && let Some(workspace) = app.workspace_mut(tab) {
        workspace.tree.reveal_cursor = false;
    }
    app.actions.extend(actions);
}

/// The terminal look's key hints under the tree.
const HINTS: [(&str, &str); 3] = [("ctrl+b", "hide"), ("t", "tree"), ("enter", "open")];

/// The terminal footer's text size and padding (8 above and below, 12 at
/// the sides), and its height for two lines.
const FOOTER_TEXT: f32 = theme::TEXT_LABEL;
const FOOTER: f32 = 8.0 + 2.0 * 15.0 + 8.0 + 1.0;

/// The sidebar's key hints, with a rule above: keys in the text colour,
/// the rest muted, wrapping as words do.
fn terminal_footer(ui: &mut egui::Ui, full: Rect, palette: &Palette) {
    let top = full.bottom() - FOOTER;
    widgets::hline(ui, full.x_range(), top + 0.5, palette.outline);
    let font = theme::regular(FOOTER_TEXT);
    let mut job = egui::text::LayoutJob::default();
    for (index, (key, label)) in HINTS.iter().enumerate() {
        let muted = egui::TextFormat::simple(font.clone(), palette.dim);
        if index > 0 {
            job.append(" · ", 0.0, muted.clone());
        }
        job.append(
            key,
            0.0,
            egui::TextFormat::simple(font.clone(), palette.text),
        );
        job.append(&format!(" {label}"), 0.0, muted);
    }
    job.wrap.max_width = full.width() - 24.0;
    let galley = ui.painter().layout_job(job);
    ui.painter().galley(
        pos2(full.left() + 12.0, top + 1.0 + 8.0),
        galley,
        egui::Color32::PLACEHOLDER,
    );
}

/// The Recent section: the objects opened lately, newest first.
fn recent_section(
    ui: &mut egui::Ui,
    recent: &[(ObjectRef, ObjectKind)],
    active: Option<&ObjectRef>,
    tab: ConnTabId,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
) {
    let width = ui.available_width();
    // The label: 4 above and below, 16 in.
    let label_job = widgets::section_label("Recent", look, palette);
    let label_height = ui.painter().layout_job(label_job.clone()).size().y + 8.0;
    let (label, _) = ui.allocate_exact_size(vec2(width, label_height), Sense::hover());
    widgets::paint_job(ui, label.left() + 16.0, label.center().y, label_job);
    for (object, kind) in recent {
        let (row, response) = ui.allocate_exact_size(vec2(width, RECENT_ROW), Sense::click());
        let name = object.name.clone();
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, format!("Recent {name}"))
        });
        let selected = Some(object) == active;
        widgets::selection(ui, row, selected, response.hovered(), look, palette);
        let (font, color) = if selected {
            (theme::medium(theme::TEXT), palette.accent_hover)
        } else {
            (theme::regular(theme::TEXT), palette.text)
        };
        let room = row.width() - 32.0;
        let shown = crate::ui::grid::ellipsize(&object.name, room, false, |text| {
            ui.painter()
                .layout_no_wrap(text.to_owned(), font.clone(), color)
                .size()
                .x
        });
        widgets::paint_text(ui, row.left() + 16.0, row.center().y, &shown, font, color);
        if response.clicked() {
            actions.push(Action::OpenObject {
                tab,
                object: object.clone(),
                kind: *kind,
                pin: true,
            });
        }
    }
    ui.add_space(8.0);
}

/// What the schema header draws with.
#[derive(Clone, Copy)]
struct SchemaHeader<'a> {
    locale: Locale,
    look: &'a Look,
    palette: &'a Palette,
}

/// The schema picker, the tree/flat switch and Refresh.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn schema_header(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    schemas: &[String],
    shown: Option<&str>,
    flat: bool,
    loading: bool,
    skin: SchemaHeader<'_>,
    actions: &mut Vec<Action>,
) {
    let SchemaHeader {
        locale,
        look,
        palette,
    } = skin;
    let width = ui.available_width();
    // macOS: a rule, then 8 above and 4 below a 28 pt row, 16 in and 8 from
    // the right. Terminal: 4 above and below a 22 pt row, 12 in and 8.
    let (above, row, below, left_pad) = if look.terminal {
        (4.0, 22.0, 4.0, 12.0)
    } else {
        (8.0, 28.0, 4.0, 16.0)
    };
    let (band, _) = ui.allocate_exact_size(vec2(width, above + row + below), Sense::hover());
    if !look.terminal {
        widgets::hline(ui, band.x_range(), band.top() + 0.5, palette.surface_hover);
    }
    let rect = Rect::from_min_size(pos2(band.left(), band.top() + above), vec2(width, row));
    let left = rect.left() + left_pad;
    let center = rect.center().y;
    // The schema: a menu of the others.
    if let Some(schema) = shown {
        let galley = ui
            .painter()
            .layout_job(widgets::section_label(schema, look, palette));
        let text_width = galley.size().x;
        let (glyph_width, gap) = if look.terminal {
            (8.0, 4.0)
        } else {
            (10.0, 4.0)
        };
        let hit = Rect::from_min_size(
            pos2(left - 6.0, center - 13.0),
            vec2(6.0 + text_width + gap + glyph_width + 6.0, 26.0),
        );
        let response = ui.interact(hit, ui.id().with("schema-menu"), Sense::click());
        response.widget_info(|| {
            let mut info =
                WidgetInfo::labeled(WidgetType::ComboBox, true, gettext(locale, "Schema"));
            info.current_text_value = Some(schema.to_owned());
            info
        });
        if response.hovered() && !look.terminal {
            ui.painter()
                .rect_filled(hit, CornerRadius::same(6), palette.surface);
        }
        ui.painter().galley(
            pos2(left, center - galley.size().y / 2.0),
            galley,
            egui::Color32::PLACEHOLDER,
        );
        let glyph = pos2(left + text_width + gap + glyph_width / 2.0, center);
        if look.terminal {
            ui.painter().text(
                glyph,
                egui::Align2::CENTER_CENTER,
                "▾",
                theme::regular(theme::TEXT_LABEL),
                palette.dim,
            );
        } else {
            Icon::ChevronDown
                .image(palette.secondary, 10.0)
                .paint_at(ui, Rect::from_center_size(glyph, vec2(10.0, 10.0)));
        }
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(160.0);
            for other in schemas {
                if ui
                    .selectable_label(Some(other.as_str()) == shown, other)
                    .clicked()
                {
                    actions.push(Action::ShowSchema {
                        tab,
                        schema: other.clone(),
                    });
                }
            }
        });
    }
    // Right to left, 8 in: Reload (macOS, 28 square), 2 apart, then the
    // tree/flat switch.
    let mut right = rect.right() - 8.0;
    if !look.terminal {
        let refresh = Rect::from_min_size(pos2(right - 28.0, center - 14.0), vec2(28.0, 28.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(refresh));
        if loading {
            egui::Spinner::new().size(14.0).paint_at(
                &child,
                Rect::from_center_size(refresh.center(), vec2(14.0, 14.0)),
            );
        } else if icon_button(
            &mut child,
            Icon::RefreshCw,
            &gettext(locale, "Refresh objects"),
            look,
            palette,
        )
        .clicked()
        {
            actions.push(Action::RefreshTree(tab));
        }
        right = refresh.left() - 2.0;
    }
    let (segments, segment) = if look.terminal {
        (
            [
                widgets::Segment::Text("tree"),
                widgets::Segment::Text("flat"),
            ],
            vec2(0.0, 20.0),
        )
    } else {
        (
            [
                widgets::Segment::Icon(Icon::ListTree, "Tree"),
                widgets::Segment::Icon(Icon::List, "Flat"),
            ],
            vec2(26.0, 22.0),
        )
    };
    // The switch's size, measured as `segmented` lays it out.
    let font = theme::regular(theme::TEXT_LABEL);
    let cells: f32 = if look.terminal {
        ["tree", "flat"]
            .iter()
            .map(|text| {
                ui.painter()
                    .layout_no_wrap((*text).to_owned(), font.clone(), egui::Color32::WHITE)
                    .size()
                    .x
                    + 12.0
            })
            .sum()
    } else {
        2.0 * segment.x
    };
    let pad = if look.terminal { 1.0 } else { 2.0 };
    let total = vec2(cells + 2.0 * pad, segment.y + 2.0 * pad);
    let place = Rect::from_min_size(pos2(right - total.x, center - total.y / 2.0), total);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
    if let Some(index) = widgets::segmented(
        &mut child,
        &segments,
        usize::from(flat),
        segment,
        look,
        palette,
    ) && (index == 1) != flat
    {
        actions.push(Action::ToggleFlatTree(tab));
    }
}

/// Which rows are highlighted: the open object and the keyboard cursor.
#[derive(Clone, Copy)]
struct Marks<'a> {
    active: Option<&'a ObjectRef>,
    cursor: Option<&'a TreeNode>,
}

/// How rows are drawn: the platform look and the colour palette.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
}

/// Where a tree row draws its label. macOS: the chevron 8 into the row
/// (itself 8 in), then 6 to the name; grouped objects 26 into theirs.
/// Terminal: 4 and 8 in, the glyph, 6; objects 24 into theirs.
fn label_x(rect: Rect, depth: u8, look: &Look, glyph: f32) -> f32 {
    let inset = tree_inset(look);
    let (pad, nested) = if look.terminal {
        (8.0, 24.0)
    } else {
        (8.0, 26.0)
    };
    rect.left() + inset + if depth > 0 { nested } else { pad + glyph + 6.0 }
}

/// The width of the fold mark: macOS's 12 pt chevron, the terminal's ▾.
fn glyph_width(ui: &egui::Ui, look: &Look) -> f32 {
    if look.terminal {
        ui.painter()
            .layout_no_wrap(
                "▾".into(),
                theme::regular(theme::TEXT),
                egui::Color32::WHITE,
            )
            .size()
            .x
    } else {
        12.0
    }
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
    let small = theme::regular(theme::TEXT_SMALL);
    // A quiet note, not a row: the text is never clickable, and a small
    // link after it reloads the tree in case objects appeared since. In a
    // narrow sidebar the note gives way ("…") so the link stays in view.
    if matches!(row.node, TreeNode::Empty(_)) {
        let size = vec2(ui.available_width(), look.tree_row);
        let layout = egui::Layout::left_to_right(egui::Align::Center);
        ui.allocate_ui_with_layout(size, layout, |ui| {
            ui.set_min_size(size);
            ui.spacing_mut().item_spacing.x = 6.0;
            let left = label_x(ui.max_rect(), 0, look, glyph_width(ui, look));
            ui.add_space(left - ui.max_rect().left());
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
                            .font(small.clone())
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
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), row_height(row, look)),
        Sense::click(),
    );
    let full_name = match &row.node {
        TreeNode::Object(object, _) => object.name.clone(),
        _ => row.label.clone(),
    };
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &full_name));
    let selected = matches!(&row.node, TreeNode::Object(object, _) if Some(object) == marks.active);
    // Rows sit in the tree's inset: the macOS pill fills it, the terminal
    // bar spans it.
    let band = if look.terminal {
        rect.shrink2(vec2(tree_inset(look), 0.0))
    } else {
        rect
    };
    widgets::selection(ui, band, selected, response.hovered(), look, palette);
    let highlight = widgets::selection_rect(band, look);
    if marks.cursor == Some(&row.node) {
        ui.painter().rect_stroke(
            highlight.shrink(0.5),
            CornerRadius::same(look.radius.saturating_sub(2)),
            egui::Stroke::new(1.0, palette.accent),
            egui::StrokeKind::Inside,
        );
    }
    let center = rect.center().y;
    let glyph = glyph_width(ui, look);
    let x = label_x(rect, row.depth, look, glyph);
    let fold_x = rect.left() + tree_inset(look) + 8.0;
    if let Some(expanded) = row.expanded {
        if look.terminal {
            let mark = if expanded { "▾" } else { "▸" };
            ui.painter().text(
                pos2(fold_x, center),
                egui::Align2::LEFT_CENTER,
                mark,
                theme::regular(theme::TEXT),
                palette.dim,
            );
        } else {
            let icon = if expanded {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            icon.image(palette.dim, 12.0).paint_at(
                ui,
                Rect::from_center_size(pos2(fold_x + 6.0, center), vec2(12.0, 12.0)),
            );
        }
    }
    let right = rect.right() - tree_inset(look) - 8.0;
    // What sits at the right: a group's count or an object's kind.
    let (trailing, trailing_color) = match &row.node {
        TreeNode::Group(..) => (row.count.map(|count| count.to_string()), palette.dim),
        TreeNode::Object(_, kind) => (
            kind_tag(locale, *kind),
            if look.terminal {
                palette.dim
            } else {
                palette.faint
            },
        ),
        TreeNode::Empty(_) => (None, palette.dim),
    };
    let trailing_font = match (&row.node, look.terminal) {
        (TreeNode::Group(..), true) => theme::regular(theme::TEXT_LABEL),
        (TreeNode::Group(..), false) => theme::regular(theme::TEXT_CAPTION),
        (_, true) => theme::regular(theme::TEXT_CAPTION),
        (_, false) => theme::regular(10.5),
    };
    let trailing_width = trailing.as_ref().map_or(0.0, |text| {
        let width = ui
            .painter()
            .layout_no_wrap(text.clone(), trailing_font.clone(), trailing_color)
            .size()
            .x;
        widgets::paint_text_right(
            ui,
            right,
            center,
            text,
            trailing_font.clone(),
            trailing_color,
        );
        width
    });
    // Groups: macOS Plex Mono 500 at 12, the terminal bold. Objects: 13,
    // the selected one medium in the strong accent (macOS) or in the text
    // colour among muted ones (terminal).
    let (font, color) = match row.node {
        TreeNode::Group(..) if look.terminal => (theme::semibold(theme::TEXT), palette.text),
        TreeNode::Group(..) => (theme::mono_medium(theme::TEXT_SMALL), palette.text),
        _ if selected && look.terminal => (theme::regular(theme::TEXT), palette.text),
        _ if selected => (theme::medium(theme::TEXT), palette.accent_hover),
        _ if look.terminal => (theme::regular(theme::TEXT), palette.secondary),
        _ => (
            theme::regular(theme::TEXT),
            palette.text.lerp_to_gamma(palette.secondary, 0.37),
        ),
    };
    // Long names end in "…" and show in full on hover.
    // Room for the spinner or error dot only when there is one.
    let marker = if row.loading || row.error.is_some() {
        22.0
    } else {
        0.0
    };
    let room = right - trailing_width - 8.0 - x - marker;
    let shown = crate::ui::grid::ellipsize(&row.label, room, false, |text| {
        ui.painter()
            .layout_no_wrap(text.to_owned(), font.clone(), color)
            .size()
            .x
    });
    if shown != row.label || row.label != full_name {
        response.clone().on_hover_text(&full_name);
    }
    ui.painter().with_clip_rect(rect).text(
        pos2(x, center),
        egui::Align2::LEFT_CENTER,
        shown,
        font,
        color,
    );
    if row.loading {
        let spinner =
            Rect::from_center_size(pos2(highlight.right() - 10.0, center), vec2(12.0, 12.0));
        egui::Spinner::new().size(12.0).paint_at(ui, spinner);
    }
    if let Some(error) = &row.error {
        response.clone().on_hover_text(error);
        ui.painter()
            .circle_filled(pos2(highlight.right() - 10.0, center), 4.0, palette.danger);
    }
    match &row.node {
        TreeNode::Group(schema, prefix) if response.clicked() => {
            actions.push(Action::ToggleGroup {
                tab,
                schema: schema.clone(),
                prefix: prefix.clone(),
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
