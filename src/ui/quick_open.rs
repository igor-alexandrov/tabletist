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
    let (keymap, layout) = (app.keymap.clone(), app.layout());
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
                        let label = crate::ui::format::object_title(object, true);
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
                        crate::ui::focus::hint(
                            ui,
                            &response,
                            crate::ui::widgets::selection_rect(rect, &look),
                            crate::ui::focus::Ring::Inset {
                                radius: look.radius.saturating_sub(2),
                            },
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
        use crate::keymap::{Command, Scope};
        ctx.input_mut(|input| {
            let moves = keymap.chords_now(layout, Command::QuickOpenMove, Scope::Prompt, true);
            for keys in moves {
                let step = if keys == "up" { -1 } else { 1 };
                if crate::ui::keys::presses(input, layout, keys).count > 0 {
                    actions.push(Action::QuickOpenMove(step));
                }
            }
            let on = |input: &mut egui::InputState, command| {
                let scope = Scope::Prompt;
                crate::ui::keys::asked(input, &keymap, layout, command, scope, true).count > 0
            };
            if on(input, Command::QuickOpenPick) {
                actions.push(Action::QuickOpenPick);
            }
            if on(input, Command::QuickOpenClose) {
                actions.push(Action::CloseDialog);
            }
        });
    }
    app.actions.extend(actions);
}
