//! The connection tab bar across the top of the window.

use egui::{
    Align, Frame, Layout, Margin, Rect, Sense, UiBuilder, WidgetInfo, WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTab, ConnTabContent};
use crate::theme::Icon;
use crate::typography::Text;
use crate::ui::widgets::icon_button;

pub const HEIGHT: f32 = 36.0;
const TAB_WIDTH: f32 = 180.0;

/// The text a tab shows and is announced as.
pub fn tab_title(app: &App, tab: &ConnTab) -> String {
    match &tab.content {
        ConnTabContent::Picker(_) => gettext(app.locale, "New tab").into_owned(),
        ConnTabContent::Workspace(workspace) => workspace.name.clone(),
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let look = app.look;
    // macOS: the tabs share the unified title bar with the window buttons,
    // centred on the same line and starting after them. AppKit measures in
    // window points, which egui's zoom scales away from its own.
    let zoom = ui.ctx().zoom_factor();
    let inset = app.titlebar.inset / zoom;
    let height = HEIGHT.max(app.titlebar.height / zoom);
    let vertical = ((height - (HEIGHT - 8.0)) / 2.0).round() as i8;
    // The underline style sits flush with the bar's bottom edge so its line
    // meets the content below.
    let margin = if look.tabs == crate::theme::TabStyle::Underline {
        Margin {
            left: 6,
            right: 6,
            top: 2 * vertical,
            bottom: 0,
        }
    } else {
        Margin::symmetric(6, vertical)
    };
    egui::Panel::top("conn-tabs")
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(margin))
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                // The window buttons sit over this corner, outside the
                // scrolling tabs so they never slide under the buttons; the
                // drag area runs right up to the first tab (no spacing).
                let spacing = ui.spacing().item_spacing.x;
                if inset > 0.0 {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    window_drag_area(ui, inset - 6.0, HEIGHT - 8.0);
                }
                egui::ScrollArea::horizontal()
                    .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                    .show(ui, |ui| {
                        ui.spacing_mut().item_spacing.x = spacing;
                        ui.horizontal(|ui| {
                            let mut track = crate::ui::widgets::TabTrack::begin(ui, &look);
                            if track.is_shown() {
                                ui.spacing_mut().item_spacing.x = 0.0;
                            }
                            for index in 0..app.tabs.len() {
                                track.add(tab(app, ui, index));
                            }
                            track.end(ui, look.tab_radius, &palette);
                            ui.spacing_mut().item_spacing.x = spacing;
                            let new_label = gettext(app.locale, "New connection tab");
                            if icon_button(ui, Icon::Plus, &new_label, &look, &palette).clicked() {
                                app.actions.push(Action::NewConnTab);
                            }
                            window_drag_area(ui, ui.available_width(), HEIGHT - 8.0);
                        });
                    });
            });
        });
}

/// Empty tab bar that moves the window when dragged and zooms it when
/// double-clicked, as a title bar does (the bar is the title bar on macOS).
fn window_drag_area(ui: &mut egui::Ui, width: f32, height: f32) {
    if width <= 0.0 {
        return;
    }
    let (_, response) = ui.allocate_exact_size(vec2(width, height), Sense::click_and_drag());
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
}

/// One tab. The tab's own click area is allocated first and its close button
/// after it, so the button sits on top and receives its own clicks. Returns
/// the tab's rect.
fn tab(app: &mut App, ui: &mut egui::Ui, index: usize) -> Rect {
    let palette = app.palette;
    let look = app.look;
    let id = app.tabs[index].id;
    let title = tab_title(app, &app.tabs[index]);
    let active = index == app.active;
    let status_color = match &app.tabs[index].content {
        ConnTabContent::Picker(_) => None,
        ConnTabContent::Workspace(workspace) => Some(match workspace.status {
            crate::model::SessionStatus::Connected => {
                workspace.color.color().unwrap_or(palette.accent)
            }
            crate::model::SessionStatus::Connecting { .. } => palette.warning,
            crate::model::SessionStatus::Disconnected(_) => palette.danger,
            crate::model::SessionStatus::Cancelled => palette.dim,
        }),
    };
    let (rect, response) = ui.allocate_exact_size(vec2(TAB_WIDTH, HEIGHT - 8.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, active, &title));
    if response.clicked() {
        app.actions.push(Action::ActivateConnTab(id));
    }
    if response.middle_clicked() {
        app.actions.push(Action::CloseConnTab(id));
    }
    crate::ui::widgets::tab(
        ui,
        rect,
        active,
        response.hovered(),
        look.tab_radius,
        &look,
        &palette,
    );

    let inner = rect.shrink2(vec2(8.0, 0.0));
    let close_rect = Rect::from_center_size(
        pos2(inner.right() - 12.0, inner.center().y),
        vec2(24.0, 24.0),
    );
    let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close_rect));
    let close = format!("{} {title}", gettext(app.locale, "Close"));
    if icon_button(&mut close_ui, Icon::X, &close, &look, &palette).clicked() {
        app.actions.push(Action::CloseConnTab(id));
    }

    let label_rect = Rect::from_min_max(inner.min, pos2(close_rect.left() - 4.0, inner.max.y));
    let mut label_ui = ui.new_child(
        UiBuilder::new()
            .max_rect(label_rect)
            .layout(Layout::left_to_right(Align::Center)),
    );
    // Tabs in a track touch; the dot and the title keep their spacing.
    label_ui.spacing_mut().item_spacing = ui.ctx().global_style().spacing.item_spacing;
    if let Some(color) = status_color {
        let (dot, _) = label_ui.allocate_exact_size(vec2(8.0, 8.0), Sense::hover());
        label_ui.painter().circle_filled(dot.center(), 3.5, color);
    }
    let color = if active {
        palette.text
    } else {
        palette.secondary
    };
    let role = crate::ui::widgets::body(&look);
    let room = label_ui.available_width();
    let shown = crate::ui::grid::ellipsize(&title, room, false, |text| {
        role.width(ui.ctx(), look.faces, text)
    });
    Text::one(&look, role, &shown, color)
        .layout(ui.ctx())
        .label(&mut label_ui);
    rect
}
