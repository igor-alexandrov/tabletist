//! The object tab bar inside a connection tab. Preview tabs are italic.

use egui::{
    Align, Frame, Id, Layout, Margin, Rect, RichText, Sense, UiBuilder, WidgetInfo, WidgetType,
    pos2, vec2,
};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId};
use crate::theme::{self, Icon};
use crate::ui::widgets::icon_button;

const HEIGHT: f32 = 32.0;
const TAB_WIDTH: f32 = 170.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let tabs: Vec<_> = workspace
        .objects
        .iter()
        .map(|object| (object.id, object.object.name.clone(), object.pinned))
        .collect();
    let active = workspace.active_object;
    if tabs.is_empty() {
        return;
    }
    // The underline style sits flush with the bar's bottom edge.
    let margin = if look.tabs == crate::theme::TabStyle::Underline {
        Margin {
            left: 6,
            right: 6,
            top: 6,
            bottom: 0,
        }
    } else {
        Margin::symmetric(6, 3)
    };
    let mut actions = Vec::new();
    egui::Panel::top(Id::new(("object-tabs", tab.0)))
        .exact_size(HEIGHT)
        .resizable(false)
        .show_separator_line(look.panel_separators)
        .frame(Frame::new().fill(palette.panel).inner_margin(margin))
        .show(ui, |ui| {
            egui::ScrollArea::horizontal()
                .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden)
                .show(ui, |ui| {
                    // The row is as tall as the tabs, so a taller control
                    // height does not centre them below the bar's top.
                    let interact_height = ui.spacing().interact_size.y;
                    ui.spacing_mut().interact_size.y = interact_height.min(HEIGHT - 6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().interact_size.y = interact_height;
                        let mut track = crate::ui::widgets::TabTrack::begin(ui, &look);
                        if track.is_shown() {
                            ui.spacing_mut().item_spacing.x = 0.0;
                        }
                        for (id, name, pinned) in tabs {
                            let is_active = Some(id) == active;
                            let (rect, response) = ui
                                .allocate_exact_size(vec2(TAB_WIDTH, HEIGHT - 6.0), Sense::click());
                            track.add(rect);
                            let label = format!("{name} {}", gettext(locale, "tab"));
                            response.widget_info(|| {
                                WidgetInfo::selected(WidgetType::Button, true, is_active, &label)
                            });
                            if response.double_clicked() {
                                actions.push(Action::PinObjectTab {
                                    tab,
                                    object_tab: id,
                                });
                            } else if response.clicked() {
                                actions.push(Action::ActivateObjectTab {
                                    tab,
                                    object_tab: id,
                                });
                            }
                            if response.middle_clicked() {
                                actions.push(Action::CloseObjectTab {
                                    tab,
                                    object_tab: id,
                                });
                            }
                            crate::ui::widgets::tab(
                                ui,
                                rect,
                                is_active,
                                response.hovered(),
                                look.radius,
                                &look,
                                &palette,
                            );
                            let inner = rect.shrink2(vec2(8.0, 0.0));
                            let close_rect = Rect::from_center_size(
                                pos2(inner.right() - 12.0, inner.center().y),
                                vec2(24.0, 24.0),
                            );
                            let mut close_ui = ui.new_child(UiBuilder::new().max_rect(close_rect));
                            let close = format!("{} {name}", gettext(locale, "Close"));
                            if icon_button(&mut close_ui, Icon::X, &close, &look, &palette)
                                .clicked()
                            {
                                actions.push(Action::CloseObjectTab {
                                    tab,
                                    object_tab: id,
                                });
                            }
                            let mut label_ui = ui.new_child(
                                UiBuilder::new()
                                    .max_rect(Rect::from_min_max(
                                        inner.min,
                                        pos2(close_rect.left() - 4.0, inner.max.y),
                                    ))
                                    .layout(Layout::left_to_right(Align::Center)),
                            );
                            let mut text = RichText::new(&name)
                                .font(theme::regular(theme::TEXT))
                                .color(if is_active {
                                    palette.text
                                } else {
                                    palette.secondary
                                });
                            if !pinned {
                                text = text.italics();
                            }
                            label_ui.add(egui::Label::new(text).truncate().selectable(false));
                        }
                        track.end(ui, look.radius, &palette);
                    });
                });
        });
    app.actions.extend(actions);
}
