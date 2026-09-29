//! Quick open (Cmd/Ctrl+P): find a loaded table or view by name.

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let (tab, query) = match &app.dialog {
        Some(Dialog::QuickOpen(open)) => (open.tab, open.query.clone()),
        _ => return,
    };
    let matches = app.quick_open_matches(tab, &query);
    let Some(Dialog::QuickOpen(open)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal =
        crate::ui::widgets::modal(egui::Id::new("quick-open"), &look, &palette).show(ctx, |ui| {
            ui.set_width(520.0);
            ui.label(
                RichText::new(gettext(locale, "Open table or view"))
                    .font(theme::semibold(theme::TEXT_TITLE))
                    .color(palette.text),
            );
            ui.add_space(6.0);
            let field = crate::ui::widgets::add_search(
                ui,
                crate::ui::widgets::search_field(
                    ui,
                    &mut open.query,
                    &gettext(locale, "Name"),
                    &look,
                )
                .desired_width(f32::INFINITY),
                &look,
            );
            field.request_focus();
            if field.changed() {
                open.selected = 0;
            }
            ui.label(
                RichText::new(gettext(locale, "Searches schemas loaded in the sidebar"))
                    .color(palette.secondary),
            );
            ui.add_space(4.0);
            if matches.is_empty() {
                ui.label(RichText::new(gettext(locale, "No matches")).color(palette.secondary));
            }
            // Scroll to the selection only when it moved, so a wheel scroll
            // is not undone on the next frame.
            let reveal = open.scrolled_to != Some(open.selected);
            open.scrolled_to = Some(open.selected);
            let output = egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for (index, (object, _)) in matches.iter().enumerate() {
                        let label = format!("{}.{}", object.schema, object.name);
                        let selected = index == open.selected;
                        let (rect, response) = ui.allocate_exact_size(
                            egui::vec2(ui.available_width(), look.tree_row),
                            egui::Sense::click(),
                        );
                        response.widget_info(|| {
                            egui::WidgetInfo::selected(
                                egui::WidgetType::SelectableLabel,
                                true,
                                selected,
                                &label,
                            )
                        });
                        crate::ui::widgets::selection(
                            ui,
                            rect,
                            selected,
                            response.hovered(),
                            &look,
                            &palette,
                        );
                        // Long names end in "…" and show in full on hover.
                        let font = theme::regular(theme::TEXT);
                        let color = crate::ui::widgets::selection_text(selected, &look, &palette);
                        let room = rect.width() - 20.0;
                        let shown = crate::ui::grid::ellipsize(&label, room, false, |text| {
                            ui.painter()
                                .layout_no_wrap(text.to_owned(), font.clone(), color)
                                .size()
                                .x
                        });
                        if shown != label {
                            response.clone().on_hover_text(&label);
                        }
                        ui.painter().text(
                            egui::pos2(rect.left() + 10.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            shown,
                            font,
                            color,
                        );
                        if reveal && index == open.selected {
                            response.scroll_to_me(None);
                        }
                        if response.clicked() {
                            open.selected = index;
                            actions.push(Action::QuickOpenPick);
                        }
                    }
                });
            open.scroll_offset = output.state.offset.y;
        });
    if modal.is_top_modal {
        ctx.input_mut(|input| {
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown) {
                actions.push(Action::QuickOpenMove(1));
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp) {
                actions.push(Action::QuickOpenMove(-1));
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Enter) {
                actions.push(Action::QuickOpenPick);
            }
            if input.consume_key(egui::Modifiers::NONE, egui::Key::Escape) {
                actions.push(Action::CloseDialog);
            }
        });
    }
    app.actions.extend(actions);
}
