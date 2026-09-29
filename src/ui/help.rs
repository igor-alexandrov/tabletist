//! The keyboard shortcuts dialog (`?`).

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::theme;
use crate::ui::keys::{SHORTCUTS, keys_label};

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !matches!(app.dialog, Some(Dialog::Help)) {
        return;
    }
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let mut actions = Vec::new();
    let modal = crate::ui::widgets::modal(egui::Id::new("help"), &look, &palette).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.label(
            RichText::new(gettext(locale, "Keyboard shortcuts"))
                .font(theme::semibold(theme::TEXT_TITLE))
                .color(palette.text),
        );
        ui.add_space(8.0);
        egui::Grid::new("shortcuts")
            .num_columns(2)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (keys, what) in SHORTCUTS {
                    ui.label(
                        RichText::new(keys_label(keys))
                            .font(theme::mono(theme::TEXT_MONO))
                            .color(palette.secondary),
                    );
                    ui.label(RichText::new(gettext(locale, what)).color(palette.text));
                    ui.end_row();
                }
            });
        ui.add_space(8.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.button(gettext(locale, "Close")).clicked() {
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
