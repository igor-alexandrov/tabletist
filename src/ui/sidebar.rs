//! The sidebar: filter, recent objects, the schema picker, the shown
//! schema's objects in prefix groups or one flat list, and the button that
//! opens a SQL editor.

use egui::{CornerRadius, Id, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::{ObjectKind, ObjectRef};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, TreeNode, TreeRow};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus;
use crate::ui::format::display_safe;
use crate::ui::widgets::{self, ButtonSpec, icon_button};

/// The sidebar's width when it opens, per look: the design's 264 and 248
/// and the 1 pt rule it draws outside them.
fn default_width(look: &Look) -> f32 {
    if look.terminal { 265.0 } else { 249.0 }
}

/// Space around the filter field: macOS 12 (8 under), terminal 10 (6).
fn margin(look: &Look) -> f32 {
    if look.terminal { 10.0 } else { 12.0 }
}

/// The filter field's height: the design's 30 and 34 and the 1 pt border it
/// draws outside them (its boxes are content-box).
fn field_height(look: &Look) -> f32 {
    if look.terminal { 32.0 } else { 36.0 }
}

/// Recent rows (macOS only).
const RECENT_ROW: f32 = 28.0;

/// The Reload button's spinner, and a loading row's.
const RELOAD_SPINNER: f32 = 14.0;
const ROW_SPINNER: f32 = 12.0;

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
    // Where the cursor starts when the Tab key comes to the tree: where it
    // was, else on the open object's row.
    let cursor_start = workspace.tree.cursor.clone().or_else(|| {
        let open = active.as_ref()?;
        rows.iter()
            .map(|row| &row.node)
            .find(|node| matches!(node, TreeNode::Object(object, _) if object == open))
            .cloned()
    });
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
    // Whether the keys come to the tree and the keyboard is in use: then
    // its cursor shows, and the terminal look marks the pane.
    let keys = workspace.pane == crate::model::Pane::Tree;
    let lit = keys && focus::visible(ui.ctx()) && !focus::on_control(ui.ctx());

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
        // The rule sits outside the content, as the design's border does.
        .frame(egui::Frame::new().fill(fill).inner_margin(egui::Margin {
            right: 1,
            ..Default::default()
        }))
        .show_separator_line(false)
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::vline(ui, full.right() + 0.5, full.y_range(), palette.outline);
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
                        role: TextRole::pick(&look, TextRole::UiBody, TextRole::OField),
                    };
                    let field = widgets::filter_field(
                        ui,
                        &mut workspace.tree.filter,
                        &hint,
                        size,
                        style,
                        &look,
                        &palette,
                    );
                    field.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, "Filter"));
                    focus::region(ui, focus::Region::Search, field.rect.expand(8.0));
                    focus::claim(ui, focus::Region::Search, &field);
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
                    Text::one(
                        &look,
                        widgets::body(&look),
                        &gettext(locale, "Loading…"),
                        palette.text,
                    )
                    .layout(ui.ctx())
                    .label(ui);
                });
            }
            if nothing {
                ui.add_space(12.0);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    let note = |ui: &mut egui::Ui, role, text: &str| {
                        Text::one(&look, role, text, palette.secondary)
                            .layout(ui.ctx())
                            .label(ui);
                    };
                    note(
                        ui,
                        widgets::body(&look),
                        &gettext(locale, "This database has no schemas you can see."),
                    );
                    note(
                        ui,
                        widgets::secondary(&look),
                        &gettext(
                            locale,
                            "The connected user may lack the privileges to list them.",
                        ),
                    );
                    if other_databases {
                        note(
                            ui,
                            widgets::secondary(&look),
                            &gettext(locale, "Or pick another database in the top bar."),
                        );
                    }
                    ui.add_space(6.0);
                    if widgets::button(ui, &gettext(locale, "Refresh"), &look).clicked() {
                        actions.push(Action::RefreshTree(tab));
                    }
                });
            }
            for error in [schemas_error, shown_error].into_iter().flatten() {
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    let width = ui.available_width() - pad;
                    Text::one(&look, widgets::body(&look), &error, palette.danger)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                });
                ui.horizontal(|ui| {
                    ui.add_space(pad);
                    if widgets::button(ui, &gettext(locale, "Retry"), &look).clicked() {
                        actions.push(Action::RefreshTree(tab));
                    }
                });
            }
            let footer = if look.terminal { FOOTER } else { SQL_FOOTER };
            let body = ui.available_height() - footer;
            ui.allocate_ui(vec2(full.width(), body.max(0.0)), |ui| {
                // The tree is one Tab stop, not one for each row: with the
                // keyboard on it the arrows move its cursor.
                let area = Rect::from_min_size(ui.cursor().min, vec2(full.width(), body.max(0.0)));
                let stop = ui.interact(
                    area,
                    Id::new(("tree-keys", tab.0)),
                    Sense::focusable_noninteractive(),
                );
                stop.widget_info(|| {
                    WidgetInfo::labeled(WidgetType::Other, true, gettext(locale, "Objects"))
                });
                ui.ctx().accesskit_node_builder(stop.id, |node| {
                    node.set_role(egui::accesskit::Role::Group);
                });
                focus::pane(ui, &stop);
                focus::hint(ui, &stop, area, focus::Ring::Own);
                focus::region(ui, focus::Region::Tree, area);
                focus::claim(ui, focus::Region::Tree, &stop);
                // The cursor it had, the open object's row, or its first.
                let start = cursor_start
                    .clone()
                    .or_else(|| rows.first().map(|row| row.node.clone()));
                if stop.gained_focus()
                    && let Some(node) = start
                {
                    actions.push(Action::SetTreeCursor { tab, node });
                }
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
                                    keys,
                                    lit,
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
                terminal_footer(ui, full, &look, &palette);
            } else {
                sql_button(ui, full, tab, locale, &look, &palette, &mut actions);
            }
            if lit {
                focus::pane_border(ui, full, &look, &palette);
            }
        });
    if reveal_pending && let Some(workspace) = app.workspace_mut(tab) {
        workspace.tree.reveal_cursor = false;
    }
    app.actions.extend(actions);
}

/// The terminal look's key hints under the tree.
const HINTS: [(&str, &str); 3] = [("ctrl+b", "hide"), ("t", "tree"), ("enter", "open")];

/// The terminal footer's padding (8 above and below, 12 at the sides), and
/// its height for two lines.
const FOOTER: f32 = 8.0 + 2.0 * 15.0 + 8.0 + 1.0;

/// The sidebar's key hints, with a rule above: keys in the text colour,
/// the rest muted, wrapping as words do.
fn terminal_footer(ui: &mut egui::Ui, full: Rect, look: &Look, palette: &Palette) {
    let top = full.bottom() - FOOTER;
    widgets::hline(ui, full.x_range(), top + 0.5, palette.outline);
    let role = TextRole::OCaption;
    let mut text = Text::new(look);
    for (index, (key, label)) in HINTS.iter().enumerate() {
        if index > 0 {
            text = text.space(role, " ");
        }
        // Each action carries the dot that separates it from the next.
        let label = if index + 1 < HINTS.len() {
            format!("{label} ·")
        } else {
            (*label).to_owned()
        };
        text = text
            .add(role, key, palette.text)
            .space(role, " ")
            .add(role, &label, palette.dim);
    }
    text.wrap(full.width() - 24.0)
        .layout(ui.ctx())
        .paint(ui.painter(), pos2(full.left() + 12.0, top + 1.0 + 8.0));
}

/// The room under the tree for the SQL Editor button (macOS and standard):
/// a rule, then 10 above and 12 below a 34 pt button.
const SQL_FOOTER: f32 = 1.0 + 10.0 + 34.0 + 12.0;

/// The button that opens a SQL editor, with its keys: as wide as the
/// sidebar, 12 in from its sides.
fn sql_button(
    ui: &mut egui::Ui,
    full: Rect,
    tab: ConnTabId,
    locale: Locale,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
) {
    let top = full.bottom() - SQL_FOOTER;
    widgets::hline(ui, full.x_range(), top + 0.5, palette.surface_hover);
    let rect = Rect::from_min_size(
        pos2(full.left() + 12.0, top + 1.0 + 10.0),
        vec2(full.width() - 24.0, 34.0),
    );
    let label = gettext(locale, "SQL Editor");
    let keys = format!("{}T", look.command_key());
    // 10 at its sides, a 14 pt plus, 8, the words; the keys at the right.
    let button = || {
        ButtonSpec::new(&label)
            .icon(Icon::Plus)
            .primary()
            .padding(10.0)
            .gap(8.0)
            .justified()
    };
    // In a sidebar too narrow for them the keys give way.
    let with_keys = button().shortcut(&keys);
    let button = if with_keys.width(ui, look) <= rect.width() {
        with_keys
    } else {
        button()
    };
    let response = button
        .show_at(ui, rect, look, palette)
        .on_hover_text(gettext(locale, "Open a new SQL editor"));
    if response.clicked() {
        actions.push(Action::NewSqlTab(tab));
    }
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
    let label_text = widgets::section_label("Recent", look, palette).layout(ui.ctx());
    let label_height = label_text.height() + 8.0;
    let (label, _) = ui.allocate_exact_size(vec2(width, label_height), Sense::hover());
    label_text.paint_left(ui.painter(), label.left() + 16.0, label.center().y);
    for (object, kind) in recent {
        let (row, response) = ui.allocate_exact_size(vec2(width, RECENT_ROW), Sense::click());
        let name = display_safe(&object.name);
        response.widget_info(|| {
            WidgetInfo::labeled(WidgetType::Button, true, format!("Recent {name}"))
        });
        let selected = Some(object) == active;
        widgets::selection(ui, row, selected, response.hovered(), look, palette);
        row_ring(ui, &response, widgets::selection_rect(row, look), look);
        let (role, color) = if selected {
            (TextRole::UiBodyStrong, palette.accent_hover)
        } else {
            (TextRole::UiBody, palette.text)
        };
        let room = row.width() - 32.0;
        let shown = crate::ui::grid::ellipsize(&name, room, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        widgets::paint_text(
            ui,
            row.left() + 16.0,
            row.center().y,
            Text::one(look, role, &shown, color),
        );
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
        let label = widgets::section_label(&display_safe(schema), look, palette).layout(ui.ctx());
        let text_width = label.width();
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
            info.current_text_value = Some(display_safe(schema).into_owned());
            info
        });
        if response.hovered() && !look.terminal {
            ui.painter()
                .rect_filled(hit, CornerRadius::same(6), palette.surface);
        }
        label.paint_left(ui.painter(), left, center);
        let glyph = pos2(left + text_width + gap + glyph_width / 2.0, center);
        if look.terminal {
            let mark = Text::one(look, TextRole::OCaption, "▾", palette.dim).layout(ui.ctx());
            mark.paint_center(ui.painter(), glyph);
        } else {
            Icon::ChevronDown
                .image(palette.secondary, 10.0)
                .paint_at(ui, Rect::from_center_size(glyph, vec2(10.0, 10.0)));
        }
        let picked = widgets::popup_menu(&response, 160.0, look, || {
            schemas
                .iter()
                .map(|other| widgets::MenuChoice {
                    text: display_safe(other).into_owned(),
                    name: None,
                    selected: Some(other.as_str()) == shown,
                })
                .collect()
        });
        if let Some(schema) = picked.and_then(|index| schemas.get(index)) {
            actions.push(Action::ShowSchema {
                tab,
                schema: schema.clone(),
            });
        }
    }
    // Right to left, 8 in: Reload (macOS, 28 square), 2 apart, then the
    // tree/flat switch.
    let mut right = rect.right() - 8.0;
    if !look.terminal {
        let refresh = Rect::from_min_size(pos2(right - 28.0, center - 14.0), vec2(28.0, 28.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(refresh));
        if loading {
            egui::Spinner::new().size(RELOAD_SPINNER).paint_at(
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
    let cells: f32 = if look.terminal {
        ["tree", "flat"]
            .iter()
            .map(|text| TextRole::OCaption.width(ui.ctx(), look.faces, text) + 12.0)
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
    /// The arrows are the tree's.
    keys: bool,
    /// And the keyboard is in use: the cursor shows.
    lit: bool,
}

/// A row with the keyboard is ringed inside its highlight: a ring outside
/// it would be cut by the list it scrolls in.
fn row_ring(ui: &egui::Ui, response: &egui::Response, highlight: Rect, look: &Look) {
    let radius = look.radius.saturating_sub(2);
    focus::hint(ui, response, highlight, focus::Ring::Inset { radius });
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
        TextRole::OBody.width(ui.ctx(), look.faces, "▾")
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
    let small = widgets::secondary(look);
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
            let refresh = gettext(locale, "Refresh");
            let link_width = small.width(ui.ctx(), look.faces, &refresh);
            let room = ui.available_width() - link_width - ui.spacing().item_spacing.x;
            let note = gettext(locale, "No tables or views");
            let shown = crate::ui::grid::ellipsize(&note, room.max(0.0), false, |text| {
                small.width(ui.ctx(), look.faces, text)
            });
            Text::one(look, small, &shown, palette.secondary)
                .layout(ui.ctx())
                .label(ui);
            let link = Text::one(look, small, &refresh, palette.accent)
                .layout(ui.ctx())
                .label_sense(ui, Sense::click());
            link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, refresh.as_ref()));
            if link.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            if link.clicked() {
                actions.push(Action::RefreshTree(tab));
            }
        });
        return;
    }
    // A row takes a click, not the Tab key: the tree is the stop.
    let (rect, response) = ui.allocate_exact_size(
        vec2(ui.available_width(), row_height(row, look)),
        Sense::CLICK,
    );
    // Names come from the server: nothing hidden in them.
    let full_name = match &row.node {
        TreeNode::Object(object, _) => display_safe(&object.name),
        _ => display_safe(&row.label),
    };
    let label = display_safe(&row.label);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &*full_name));
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
    // The terminal's bar on the open object's row is muted while the keys
    // are elsewhere: selected, not focused.
    if selected && !marks.keys && look.selection == crate::theme::Selection::Bar {
        let edge = Rect::from_min_size(band.min, vec2(2.0, band.height()));
        ui.painter()
            .rect_filled(edge, CornerRadius::ZERO, palette.dim);
    }
    // The row the arrows are on, while the keyboard is in use and its keys
    // come to the tree: the ring a focused row takes. Apart from the open
    // object's fill: one says where the keys are, the other what is open.
    if marks.cursor == Some(&row.node) && marks.lit {
        ui.painter().rect_stroke(
            highlight,
            CornerRadius::same(look.radius.saturating_sub(2)),
            egui::Stroke::new(2.0, palette.accent),
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
            widgets::paint_text(
                ui,
                fold_x,
                center,
                Text::one(look, TextRole::OBody, mark, palette.dim),
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
    let trailing_role = match (&row.node, look.terminal) {
        (TreeNode::Group(..), true) => TextRole::OCaption,
        (TreeNode::Group(..), false) => TextRole::ColumnType,
        (_, true) => TextRole::OColumnType,
        (_, false) => TextRole::TagSmall,
    };
    let trailing_width = trailing.as_ref().map_or(0.0, |text| {
        widgets::paint_text_right(
            ui,
            right,
            center,
            Text::one(look, trailing_role, text, trailing_color),
        )
    });
    // Groups: macOS Plex Mono 500 at 12, the terminal bold. Objects: 13,
    // the selected one medium in the strong accent (macOS) or in the text
    // colour among muted ones (terminal).
    let (role, color) = match row.node {
        TreeNode::Group(..) if look.terminal => (TextRole::OGroup, palette.text),
        TreeNode::Group(..) => (TextRole::MonoGroup, palette.text),
        _ if selected && look.terminal => (TextRole::OBody, palette.text),
        _ if selected => (TextRole::UiBodyStrong, palette.accent_hover),
        _ if look.terminal => (TextRole::OBody, palette.secondary),
        _ => (
            TextRole::UiBody,
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
    let shown = crate::ui::grid::ellipsize(&label, room, false, |text| {
        role.width(ui.ctx(), look.faces, text)
    });
    if shown != label || label != full_name {
        response.clone().on_hover_text(&*full_name);
    }
    Text::one(look, role, &shown, color)
        .layout(ui.ctx())
        .paint_left(&ui.painter().with_clip_rect(rect), x, center);
    if row.loading {
        let spinner =
            Rect::from_center_size(pos2(highlight.right() - 10.0, center), vec2(12.0, 12.0));
        egui::Spinner::new().size(ROW_SPINNER).paint_at(ui, spinner);
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
