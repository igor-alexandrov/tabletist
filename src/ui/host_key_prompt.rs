//! Asks whether to trust an SSH host seen for the first time.

use egui::RichText;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::theme;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::HostKey(prompt)) = &app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal = crate::ui::widgets::modal(egui::Id::new("host-key-prompt"), &look, &palette);
    let modal = modal.show(ctx, |ui| {
        ui.set_width(420.0);
        ui.label(
            RichText::new(gettext(locale, "Trust this SSH host?"))
                .font(theme::semibold(theme::TEXT_TITLE))
                .color(palette.text),
        );
        ui.add_space(4.0);
        ui.label(
            RichText::new(format!(
                "{}:{} {}",
                prompt.host,
                prompt.port,
                gettext(
                    locale,
                    "has a key this computer has not seen before. \
                     Compare its fingerprint with the server's before trusting it."
                )
            ))
            .color(palette.secondary),
        );
        ui.add_space(6.0);
        ui.add(
            egui::Label::new(
                RichText::new(&prompt.fingerprint)
                    .font(theme::mono(theme::TEXT_MONO))
                    .color(palette.text),
            )
            .selectable(true),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if crate::ui::widgets::primary_button(
                    ui,
                    &gettext(locale, "Trust and connect"),
                    &look,
                    &palette,
                )
                .clicked()
                {
                    actions.push(Action::TrustHostKey);
                }
                if ui.button(gettext(locale, "Cancel")).clicked() {
                    actions.push(Action::CancelHostKey);
                }
            });
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CancelHostKey);
    }
    app.actions.extend(actions);
}
