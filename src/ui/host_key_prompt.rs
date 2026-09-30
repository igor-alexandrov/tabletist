//! Asks whether to trust an SSH host seen for the first time.

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::typography::Text;
use crate::ui::widgets;

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
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &gettext(locale, "Trust this SSH host?"),
            palette.text,
            &look,
        );
        ui.add_space(4.0);
        let message = format!(
            "{}:{} {}",
            prompt.host,
            prompt.port,
            gettext(
                locale,
                "has a key this computer has not seen before. \
                 Compare its fingerprint with the server's before trusting it."
            )
        );
        let width = ui.available_width();
        Text::one(&look, widgets::body(&look), &message, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(6.0);
        let fingerprint = Text::one(
            &look,
            widgets::code(&look),
            &prompt.fingerprint,
            palette.text,
        )
        .layout(ui.ctx());
        ui.add(egui::Label::new(fingerprint.galley).selectable(true));
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
                if widgets::button(ui, &gettext(locale, "Cancel"), &look).clicked() {
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
