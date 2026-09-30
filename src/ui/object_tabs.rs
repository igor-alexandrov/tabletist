//! The object tab bar inside a connection tab. Preview tabs read fainter.

use egui::{CornerRadius, Frame, Id, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId};
use crate::theme::{self, Icon, Look, Palette};
use crate::ui::widgets::{self, icon_button};

/// The strip's height, per look.
pub fn height(look: &Look) -> f32 {
    if look.terminal { 23.0 } else { 34.0 }
}

/// A macOS tab's width for a title `text` points wide.
fn mac_width(text: f32) -> f32 {
    (text + 97.0).clamp(138.0, 240.0)
}

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
            let bar = ui.max_rect();
            widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.outline);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            ui.horizontal(|ui| {
                if look.terminal {
                    let (cell, _) =
                        ui.allocate_exact_size(vec2(27.0, bar.height()), Sense::hover());
                    let mut child = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(Rect::from_center_size(cell.center(), vec2(24.0, 22.0))),
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
                            for (index, (id, name, pinned)) in tabs.iter().enumerate() {
                                let is_active = Some(*id) == active;
                                let one = Tab {
                                    index,
                                    name,
                                    pinned: *pinned,
                                    active: is_active,
                                };
                                let response = if look.terminal {
                                    terminal_tab(ui, &one, bar, &palette)
                                } else {
                                    mac_tab(ui, &one, bar, &look, &palette)
                                };
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
                                    actions.push(Action::ActivateObjectTab {
                                        tab,
                                        object_tab: *id,
                                    });
                                }
                                if response.middle_clicked() {
                                    actions.push(Action::CloseObjectTab {
                                        tab,
                                        object_tab: *id,
                                    });
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
                                            response.rect.right() - 15.0,
                                            response.rect.center().y,
                                        ),
                                        vec2(20.0, 20.0),
                                    );
                                    let mut close_ui =
                                        ui.new_child(egui::UiBuilder::new().max_rect(close_rect));
                                    let close = format!("{} {name}", gettext(locale, "Close"));
                                    if small_close(&mut close_ui, &close, &look, &palette).clicked()
                                    {
                                        actions.push(Action::CloseObjectTab {
                                            tab,
                                            object_tab: *id,
                                        });
                                    }
                                }
                            }
                        });
                    });
            });
            if look.terminal {
                let cell = Rect::from_min_max(pos2(bar.right() - 27.0, bar.top()), bar.max);
                ui.painter()
                    .rect_filled(cell, CornerRadius::ZERO, palette.panel);
                widgets::vline(ui, cell.left() + 0.5, bar.y_range(), palette.outline);
                widgets::hline(ui, cell.x_range(), bar.bottom() - 0.5, palette.outline);
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(Rect::from_center_size(cell.center(), vec2(24.0, 22.0))),
                );
                let tint = if row_panel {
                    palette.accent
                } else {
                    palette.dim
                };
                let response = icon_button(
                    &mut child,
                    Icon::PanelRight,
                    &gettext(locale, "Show or hide the row panel"),
                    &look,
                    &palette,
                );
                if row_panel {
                    Icon::PanelRight.image(tint, 16.0).paint_at(
                        &child,
                        Rect::from_center_size(response.rect.center(), vec2(16.0, 16.0)),
                    );
                }
                if response.clicked() {
                    actions.push(Action::ToggleRowPanel(tab));
                }
            }
        });
    app.actions.extend(actions);
}

/// One tab to draw.
struct Tab<'a> {
    index: usize,
    name: &'a str,
    pinned: bool,
    active: bool,
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
    let (font, color) = match (tab.active, tab.pinned) {
        (true, true) => (theme::medium(theme::TEXT), palette.text),
        (true, false) => (theme::medium(theme::TEXT), palette.secondary),
        (false, true) => (theme::regular(theme::TEXT), palette.secondary),
        (false, false) => (theme::regular(theme::TEXT), palette.dim),
    };
    let job = widgets::styled(tab.name, font, color, 0.0);
    let galley = ui.painter().layout_job(job);
    let width = mac_width(galley.size().x);
    let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
    let painter = ui.painter();
    if tab.active {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
        let line = Rect::from_min_size(rect.min, vec2(rect.width(), 2.0));
        painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.surface);
    }
    if !tab.active {
        widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.border);
    }
    let center = rect.center().y + if tab.active { 1.0 } else { 0.0 };
    let icon_color = if tab.active {
        palette.secondary
    } else {
        palette.dim
    };
    Icon::Table.image(icon_color, 15.0).paint_at(
        ui,
        Rect::from_center_size(pos2(rect.left() + 21.5, center), vec2(15.0, 15.0)),
    );
    let text_rect = Rect::from_min_max(
        pos2(rect.left() + 33.0, rect.top()),
        pos2(rect.right() - 30.0, rect.bottom()),
    );
    ui.painter().with_clip_rect(text_rect).galley(
        pos2(text_rect.left(), center - galley.size().y / 2.0),
        galley,
        egui::Color32::PLACEHOLDER,
    );
    let _ = look;
    response
}

/// The terminal look: a number, then the name; the active tab bold with an
/// accent line on top.
fn terminal_tab(ui: &mut egui::Ui, tab: &Tab<'_>, bar: Rect, palette: &Palette) -> egui::Response {
    let number = format!("{} ", tab.index + 1);
    let font = if tab.active {
        theme::semibold(theme::TEXT)
    } else {
        theme::regular(theme::TEXT)
    };
    let name_color = if tab.active {
        palette.text
    } else if tab.pinned {
        palette.dim
    } else {
        palette.faint
    };
    let mut job = widgets::styled(&number, theme::regular(theme::TEXT), palette.dim, 0.0);
    job.append(
        tab.name,
        0.0,
        egui::TextFormat {
            font_id: font,
            color: name_color,
            ..Default::default()
        },
    );
    let galley = ui.painter().layout_job(job);
    let width = galley.size().x + 22.0;
    let (rect, response) = ui.allocate_exact_size(vec2(width, bar.height()), Sense::click());
    let painter = ui.painter();
    if tab.active {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.window);
        let line = Rect::from_min_size(rect.min, vec2(rect.width(), 2.0));
        painter.rect_filled(line, CornerRadius::ZERO, palette.accent);
    } else if response.hovered() {
        painter.rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.06));
    }
    widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.outline);
    ui.painter().galley(
        pos2(
            rect.left() + 11.0,
            rect.center().y + 1.0 - galley.size().y / 2.0,
        ),
        galley,
        egui::Color32::PLACEHOLDER,
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
