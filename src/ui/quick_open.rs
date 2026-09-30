//! Quick open (Cmd/Ctrl+P): find a loaded table or view by name.

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::typography::Text;
use crate::ui::widgets;

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
            widgets::label(
                ui,
                widgets::dialog_title(&look),
                &gettext(locale, "Open table or view"),
                palette.text,
                &look,
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
            widgets::label(
                ui,
                widgets::body(&look),
                &gettext(locale, "Searches schemas loaded in the sidebar"),
                palette.secondary,
                &look,
            );
            ui.add_space(4.0);
            if matches.is_empty() {
                widgets::label(
                    ui,
                    widgets::body(&look),
                    &gettext(locale, "No matches"),
                    palette.secondary,
                    &look,
                );
            }
            // Scroll to the selection only when it moved, so a wheel scroll
            // is not undone on the next frame.
            let reveal = open.scrolled_to != Some(open.selected);
            open.scrolled_to = Some(open.selected);
            let output = egui::ScrollArea::vertical()
                .max_height(360.0)
                .show(ui, |ui| {
                    for (index, (object, _)) in matches.iter().enumerate() {
                        let label = format!(
                            "{}.{}",
                            crate::ui::format::display_safe(&object.schema),
                            crate::ui::format::display_safe(&object.name)
                        );
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
                        let role = widgets::body(&look);
                        let color = crate::ui::widgets::selection_text(selected, &look, &palette);
                        let room = rect.width() - 20.0;
                        let shown = crate::ui::grid::ellipsize(&label, room, false, |text| {
                            role.width(ui.ctx(), look.faces, text)
                        });
                        if shown != label {
                            response.clone().on_hover_text(&label);
                        }
                        widgets::paint_text(
                            ui,
                            rect.left() + 10.0,
                            rect.center().y,
                            Text::one(&look, role, &shown, color),
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
