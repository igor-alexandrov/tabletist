//! The tab bar inside a connection tab: the open tables and views and the
//! SQL editors. Preview tabs read fainter.

use egui::{CornerRadius, Frame, Id, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{self, Action, ConnTabId, TabId};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets::{self, ButtonSpec, icon_button};

/// The strip's height, per look.
pub fn height(look: &Look) -> f32 {
    // 36 (macOS) or 32 (terminal), and the rule under the strip.
    if look.terminal { 33.0 } else { 37.0 }
}

/// A macOS tab's width for a title `text` points wide: 14 in, a 14 pt
/// icon, 8, the title, 8, the 22 pt close button and 6 (at least 150). A
/// tab keeps the room for its close button while it shows none, so the
/// strip stays put when another tab becomes the active one.
fn mac_width(text: f32) -> f32 {
    (14.0 + 14.0 + 8.0 + text + 8.0 + 22.0 + 6.0).max(150.0)
}

/// A toggle's cell at an end of the strip: the row panel's in every look,
/// and the sidebar's in the terminal's.
const TOGGLE_CELL: f32 = 40.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    // Each tab's id, name, whether it is pinned and whether it is a SQL
    // editor (never a preview, so it draws like a pinned tab).
    let tabs: Vec<(TabId, String, bool, bool)> = workspace
        .tabs
        .iter()
        .map(|open| match open {
            model::Tab::Object(object) => {
                let shared = workspace.name_is_shared(&object.object);
                let name = crate::ui::format::object_title(&object.object, shared);
                (object.id, name, object.pinned, false)
            }
            model::Tab::Sql(sql) => (
                sql.id,
                format!("{} {}", gettext(locale, "Query"), sql.number),
                true,
                true,
            ),
        })
        .collect();
    let active = workspace.active_tab;
    let row_panel = workspace.row_panel;
    if tabs.is_empty() && !look.terminal {
        return;
    }
    let mut actions = Vec::new();
    let fill = if look.terminal {
        palette.panel
    } else {
        palette.surface_hover
    };
    egui::Panel::top(Id::new(("object-tabs", tab.0)))
        .exact_size(height(&look))
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            let full = ui.max_rect();
            focus::region(ui, focus::Region::Tabs, full);
            let rule = if look.terminal {
                palette.outline
            } else {
                palette.border
            };
            widgets::hline(ui, full.x_range(), full.bottom() - 0.5, rule);
            // The tabs stand on the rule, above it.
            let bar = Rect::from_min_max(full.min, pos2(full.right(), full.bottom() - 1.0));
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.horizontal(|ui| {
                if look.terminal {
                    let (cell, _) =
                        ui.allocate_exact_size(vec2(TOGGLE_CELL, bar.height()), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(Rect::from_center_size(cell.center(), vec2(24.0, 24.0))),
                    );
                    if icon_button(
                        &mut child,
                        Icon::PanelLeft,
                        &gettext(locale, "Show or hide the sidebar"),
                        &look,
                        &palette,
                    )
                    .clicked()
                    {
                        actions.push(Action::ToggleSidebar(tab));
                    }
                    widgets::vline(ui, cell.right() - 0.5, bar.y_range(), palette.outline);
                }
                egui::ScrollArea::horizontal()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                            for (index, (id, name, pinned, sql)) in tabs.iter().enumerate() {
                                let is_active = Some(*id) == active;
                                let one = Tab {
                                    index,
                                    name,
                                    pinned: *pinned,
                                    active: is_active,
                                    sql: *sql,
                                };
                                let response = if look.terminal {
                                    terminal_tab(ui, &one, bar, &look, &palette)
                                } else {
                                    mac_tab(ui, &one, bar, &look, &palette)
                                };
                                if is_active {
                                    focus::claim(ui, focus::Region::Tabs, &response);
                                }
                                let label = format!("{name} {}", gettext(locale, "tab"));
                                response.widget_info(|| {
                                    WidgetInfo::selected(
                                        WidgetType::Button,
                                        true,
                                        is_active,
                                        &label,
                                    )
                                });
                                if response.double_clicked() {
                                    actions.push(Action::PinObjectTab {
                                        tab,
                                        object_tab: *id,
                                    });
                                } else if response.clicked() {
                                    actions.push(Action::ActivateTab { tab, id: *id });
                                }
                                if response.middle_clicked() {
                                    actions.push(Action::CloseTab { tab, id: *id });
                                }
                                // The close button: on the active tab, or
                                // where the pointer is (macOS).
                                if !look.terminal
                                    && (is_active
                                        || response.hovered()
                                        || ui.rect_contains_pointer(response.rect))
                                {
                                    let close_rect = Rect::from_center_size(
                                        pos2(
                                            response.rect.right() - 17.0,
                                            response.rect.center().y,
                                        ),
                                        vec2(22.0, 22.0),
                                    );
                                    let mut close_ui =
                                        ui.new_child(egui::UiBuilder::new().max_rect(close_rect));
                                    let close = format!("{} {name}", gettext(locale, "Close"));
                                    if small_close(&mut close_ui, &close, &look, &palette).clicked()
                                    {
                                        actions.push(Action::CloseTab { tab, id: *id });
                                    }
                                }
                            }
                            if look.terminal {
                                new_sql(ui, bar, tab, locale, &look, &palette, &mut actions);
                            } else {
                                // Room to scroll the last tab clear of the
                                // row panel's toggle.
                                ui.add_space(TOGGLE_CELL);
                            }
                        });
                    });
            });
            // The row panel's toggle, in every look and on every tab: a SQL
            // editor's result has a row panel too. A closed panel has no
            // button of its own left to open it with.
            let cell = Rect::from_min_max(pos2(bar.right() - TOGGLE_CELL, bar.top()), bar.max);
            ui.painter().rect_filled(cell, CornerRadius::ZERO, fill);
            widgets::vline(ui, cell.left() + 0.5, bar.y_range(), rule);
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(Rect::from_center_size(cell.center(), vec2(24.0, 24.0))),
            );
            let label = gettext(locale, "Show or hide the row panel");
            let response = icon_button(&mut child, Icon::PanelRight, &label, &look, &palette);
            // Whether the panel is open is more than the icon's colour.
            response
                .widget_info(|| WidgetInfo::selected(WidgetType::Button, true, row_panel, &label));
            // The other looks' strip has the tone a button takes under the
            // pointer: there the button takes the tone a tab does.
            let lit = !look.terminal && response.hovered();
            if lit {
                child.painter().rect_filled(
                    response.rect,
                    CornerRadius::same(look.radius),
                    palette.surface,
                );
            }
            if row_panel || lit {
                let tint = if row_panel {
                    palette.accent
                } else {
                    palette.text
                };
                Icon::PanelRight.image(tint, 16.0).paint_at(
                    &child,
                    Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)),
                );
            }
            if response.clicked() {
                actions.push(Action::ToggleRowPanel(tab));
            }
        });
    app.actions.extend(actions);
}

/// The terminal's `+ sql` after the last tab: 8 from it and 4 from the
/// strip's edges, 10 at its sides inside a 1 pt border, then room to
/// scroll it clear of the row panel's toggle.
fn new_sql(
    ui: &mut egui::Ui,
    bar: Rect,
    tab: ConnTabId,
    locale: crate::i18n::Locale,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
) {
    let text = format!("+ {}", gettext(locale, "sql"));
    let label = gettext(locale, "New SQL editor");
    let button = ButtonSpec::new(&text)
        .label(&label)
        .primary()
        .role(TextRole::OGroup)
        .shortcut("ctrl+t")
        .shortcut_role(TextRole::OBody)
        .padding(11.0)
        .gap(8.0);
    let width = button.width(ui, look);
    let (cell, _) = ui.allocate_exact_size(
        vec2(8.0 + width + 8.0 + TOGGLE_CELL, bar.height()),
        Sense::hover(),
    );
    let place = Rect::from_min_size(
        pos2(cell.left() + 8.0, cell.top() + 4.0),
        vec2(width, cell.height() - 8.0),
    );
    if button.show_at(ui, place, look, palette).clicked() {
        actions.push(Action::NewSqlTab(tab));
    }
}

/// One tab to draw.
struct Tab<'a> {
    index: usize,
    name: &'a str,
    pinned: bool,
    active: bool,
    /// A SQL editor, not a table or view.
    sql: bool,
}

/// macOS: full-height tabs; the active one white with an accent line on
/// top, joining the content below.
fn mac_tab(
    ui: &mut egui::Ui,
    tab: &Tab<'_>,
    bar: Rect,
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    // A preview tab (replaced by the next click) reads a step fainter; the
    // design has no italics.
    let (role, color) = match (tab.active, tab.pinned) {
        (true, true) => (TextRole::UiBodyStrong, palette.text),
        (true, false) => (TextRole::UiBodyStrong, palette.secondary),
        (false, true) => (TextRole::UiBody, palette.secondary),
        (false, false) => (TextRole::UiBody, palette.dim),
    };
    let label = Text::one(look, role, tab.name, color).layout(ui.ctx());
    // Room for the title at its widest, the active tab's weight.
    let width = mac_width(TextRole::UiBodyStrong.width(ui.ctx(), look.faces, tab.name));
    let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
    // Inside the tab: the strip scrolls, and a ring outside would be cut.
    focus::hint(ui, &response, rect, Ring::Inset { radius: 4 });
    let painter = ui.painter();
    if tab.active {
        // White down to the content, over the strip's rule.
        let face = Rect::from_min_max(rect.min, pos2(rect.right(), rect.bottom() + 1.0));
        painter.rect_filled(face, CornerRadius::ZERO, palette.window);
        let line = Rect::from_min_size(rect.min, vec2(rect.width(), 2.0));
        painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.surface);
    }
    widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.border);
    let center = rect.center().y;
    let icon = if tab.sql { Icon::Code } else { Icon::Table };
    icon.image(palette.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(rect.left() + 21.0, center), vec2(14.0, 14.0)),
    );
    let text_rect = Rect::from_min_max(
        pos2(rect.left() + 36.0, rect.top()),
        pos2(rect.right() - 6.0 - 22.0 - 8.0, rect.bottom()),
    );
    label.paint_left(
        &ui.painter().with_clip_rect(text_rect),
        text_rect.left(),
        center,
    );
    response
}

/// The terminal look: a number, then the name; the active tab bold with an
/// accent line on top.
fn terminal_tab(
    ui: &mut egui::Ui,
    tab: &Tab<'_>,
    bar: Rect,
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    // 12 in, the number, 8, the name, 12 out. Only the active tab's name
    // takes the text colour; a preview tab's reads fainter still.
    let role = TextRole::OBody;
    let number = (tab.index + 1).to_string();
    // The look's lower case for an editor's name; a table keeps its own.
    let name = if tab.sql {
        look.label(tab.name)
    } else {
        tab.name.to_owned()
    };
    let name_color = match (tab.active, tab.pinned) {
        (true, _) => palette.text,
        (false, true) => palette.dim,
        (false, false) => palette.faint,
    };
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let (number_width, name_width) = (measure(&number), measure(&name));
    let width = 12.0 + number_width + 8.0 + name_width + 12.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
    focus::hint(ui, &response, rect, Ring::Inset { radius: 0 });
    let painter = ui.painter();
    if tab.active {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
        let line = Rect::from_min_size(rect.min, vec2(rect.width(), 2.0));
        painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.06));
    }
    widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.outline);
    let y = rect.center().y;
    widgets::paint_text(
        ui,
        rect.left() + 12.0,
        y,
        Text::one(look, role, &number, palette.dim),
    );
    widgets::paint_text(
        ui,
        rect.left() + 12.0 + number_width + 8.0,
        y,
        Text::one(look, role, &name, name_color),
    );
    response
}

/// The small × that closes a macOS tab.
fn small_close(ui: &mut egui::Ui, label: &str, look: &Look, palette: &Palette) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(20.0, 20.0), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            CornerRadius::same(look.radius.saturating_sub(3)),
            palette.surface_hover,
        );
    }
    Icon::X
        .image(palette.dim, 13.0)
        .paint_at(ui, Rect::from_center_size(rect.center(), vec2(13.0, 13.0)));
    response.on_hover_text(label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Harness;

    #[test]
    fn a_mac_tab_is_as_wide_in_every_state() {
        let mut harness = Harness::new();
        let (look, palette) = (harness.app.look, harness.app.palette);
        assert!(!look.terminal);
        // One under the least width, one near it, one well over.
        for name in ["accounts", "assignees_filters", "a_table_with_a_long_name"] {
            let mut widths = Vec::new();
            harness.frame_with(|ui| {
                let bar = Rect::from_min_size(ui.max_rect().min, vec2(1_200.0, 36.0));
                ui.horizontal(|ui| {
                    for (active, pinned) in
                        [(true, true), (true, false), (false, true), (false, false)]
                    {
                        let tab = Tab {
                            index: 0,
                            name,
                            pinned,
                            active,
                            sql: false,
                        };
                        widths.push(mac_tab(ui, &tab, bar, &look, &palette).rect.width());
                    }
                });
            });
            assert!(
                widths.iter().all(|width| *width == widths[0]),
                "{name}: {widths:?}"
            );
        }
    }

    #[test]
    fn activating_another_tab_leaves_the_strip_in_place() {
        let mut harness = Harness::new();
        let tab = harness.connect_fake();
        harness.click("users");
        harness.add_sql_tab(tab);
        let place = |harness: &mut Harness| {
            let tree = harness.settle();
            ["users tab", "Query 1 tab"].map(|label| {
                crate::testing::bounds(&tree, label, egui::accesskit::Role::Button)
                    .unwrap_or_else(|| panic!("{label} missing"))
            })
        };
        let before = place(&mut harness);
        harness.click("Query 1 tab");
        assert_eq!(place(&mut harness), before);
    }
}
