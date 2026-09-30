//! Asks for a connection's password, SSH password or key passphrase.

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog, SecretKind};
use crate::ui::widgets;

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::Password(prompt)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let modal = crate::ui::widgets::modal(egui::Id::new("password-prompt"), &look, &palette).show(
        ctx,
        |ui| {
            ui.set_width(360.0);
            let title = format!(
                "{} {}",
                match prompt.kind {
                    SecretKind::Database => gettext(locale, "Password for"),
                    SecretKind::SshPassword => gettext(locale, "SSH password for"),
                    SecretKind::SshPassphrase => gettext(locale, "Key passphrase for"),
                },
                prompt.name
            );
            widgets::label(
                ui,
                widgets::dialog_title(&look),
                &title,
                palette.text,
                &look,
            );
            if let Some(message) = &prompt.message {
                widgets::label(ui, widgets::body(&look), message, palette.secondary, &look);
            }
            ui.add_space(6.0);
            let field = ui.add(
                crate::ui::widgets::single(ui, &mut prompt.password, &look)
                    .password(true)
                    .desired_width(f32::INFINITY),
            );
            if !field.has_focus() && prompt.password.is_empty() {
                field.request_focus();
            }
            let entered = field.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            crate::ui::widgets::checkbox(
                ui,
                &mut prompt.save,
                &gettext(locale, "Save in keyring"),
                &look,
                &palette,
            );
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::widgets::primary_button(
                        ui,
                        &gettext(locale, "Connect"),
                        &look,
                        &palette,
                    )
                    .clicked()
                        || entered
                    {
                        actions.push(Action::SubmitPassword);
                    }
                    if widgets::button(ui, &gettext(locale, "Cancel"), &look).clicked() {
                        actions.push(Action::CancelPassword);
                    }
                });
            });
        },
    );
    if modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CancelPassword);
    }
    app.actions.extend(actions);
}
