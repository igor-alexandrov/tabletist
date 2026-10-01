//! The keyboard shortcuts dialog (`?`).

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::typography::Text;
use crate::ui::keys::{SHORTCUTS, keys_label};
use crate::ui::widgets;

/// Space between the keys and what they do.
const GAP: f32 = 16.0;

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !matches!(app.dialog, Some(Dialog::Help)) {
        return;
    }
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let mut actions = Vec::new();
    let modal = crate::ui::widgets::modal(egui::Id::new("help"), &look, &palette).show(ctx, |ui| {
        // As wide as its widest keys and description side by side, and no
        // wider or taller than the window leaves room for: the list scrolls
        // when the window is small, so the buttons stay on screen.
        let (mut keys_width, mut what_width) = (0.0_f32, 0.0_f32);
        for (keys, what) in SHORTCUTS {
            let keys = Text::one(&look, widgets::code(&look), &keys_label(keys), palette.text);
            let what = Text::one(
                &look,
                widgets::body(&look),
                &gettext(locale, what),
                palette.text,
            );
            keys_width = keys_width.max(widgets::measure(ui, keys));
            what_width = what_width.max(widgets::measure(ui, what));
        }
        let screen = ui.ctx().content_rect();
        let width = (keys_width + GAP + what_width).ceil();
        ui.set_width(width.min(screen.width() - 80.0).max(120.0));
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &gettext(locale, "Keyboard shortcuts"),
            palette.text,
            &look,
        );
        ui.add_space(8.0);
        egui::ScrollArea::both()
            .max_height((screen.height() - 180.0).max(120.0))
            .auto_shrink([false, true])
            .show(ui, |ui| {
                egui::Grid::new("shortcuts")
                    .num_columns(2)
                    .spacing([GAP, 6.0])
                    .show(ui, |ui| {
                        for (keys, what) in SHORTCUTS {
                            widgets::label(
                                ui,
                                widgets::code(&look),
                                &keys_label(keys),
                                palette.secondary,
                                &look,
                            );
                            widgets::label(
                                ui,
                                widgets::body(&look),
                                &gettext(locale, what),
                                palette.text,
                                &look,
                            );
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::button(ui, &gettext(locale, "Close"), &look).clicked() {
                actions.push(Action::CloseDialog);
            }
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}
