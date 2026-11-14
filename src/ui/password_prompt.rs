//! Asks for a connection's password, SSH password or key passphrase.

use crate::i18n::gettext;
use crate::model::{Action, PasswordPrompt, SecretKind};
use crate::ui::{Paint, widgets};

/// Draws the prompt over the window, and returns what it asks for.
pub fn show(ctx: &egui::Context, paint: Paint, prompt: &mut PasswordPrompt) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
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
            let title = widgets::label(
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
            let field = ui
                .add(
                    crate::ui::widgets::single(ui, &mut prompt.password, &look)
                        .password(true)
                        .desired_width(f32::INFINITY),
                )
                .labelled_by(title.id);
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
    actions
}

#[cfg(test)]
mod tests {
    use crate::model::{Dialog, PasswordPrompt, SecretKind};
    use crate::testing::Harness;

    #[test]
    fn the_field_is_named_by_the_prompts_title() {
        let mut harness = Harness::new();
        harness.app.dialog = Some(Dialog::Password(Box::new(PasswordPrompt {
            tab: harness.app.active_tab_id(),
            kind: SecretKind::Database,
            name: "Prod".into(),
            password: String::new(),
            save: true,
            message: None,
        })));
        let tree = harness.settle();
        let (_, field) = tree
            .nodes
            .iter()
            .find(|(_, node)| node.role() == egui::accesskit::Role::PasswordInput)
            .expect("the password field");
        let names: Vec<&str> = field
            .labelled_by()
            .iter()
            .filter_map(|id| tree.nodes.iter().find(|(node, _)| node == id))
            .filter_map(|(_, label)| label.label().or_else(|| label.value()))
            .collect();
        assert_eq!(names, ["Password for Prod"]);
    }
}
