//! The connection dialog: new or edit, any driver, with an optional SSH tunnel.

use egui::RichText;
use tabletist_db::{Driver, TlsMode};

use crate::app::App;
use crate::connections::{ColorTag, PasswordMode};
use crate::i18n::gettext;
use crate::model::{Action, Dialog, SshAuthKind, TestState};
use crate::theme;

const TLS_MODES: [(TlsMode, &str); 5] = [
    (TlsMode::Disable, "Off"),
    (TlsMode::Prefer, "Prefer (not verified)"),
    (TlsMode::Require, "Require (not verified)"),
    (TlsMode::VerifyCa, "Verify certificate"),
    (TlsMode::VerifyFull, "Verify certificate and host"),
];

fn tls_label(mode: TlsMode) -> &'static str {
    TLS_MODES
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("", |(_, label)| label)
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::Connection(form)) = &mut app.dialog else {
        return;
    };
    let mut actions = Vec::new();
    let title = if form.editing.is_some() {
        gettext(locale, "Edit connection")
    } else {
        gettext(locale, "New connection")
    };
    // A modal: nothing behind it can be clicked while it is open.
    let modal = crate::ui::widgets::modal(egui::Id::new("connection-dialog"), &look, &palette)
        .show(ctx, |ui| {
            ui.set_width(480.0);
            ui.label(
                RichText::new(title.as_ref())
                    .font(theme::semibold(theme::TEXT_TITLE))
                    .color(palette.text),
            );
            ui.add_space(8.0);
            // The fields scroll when the window is short, so the buttons stay on screen.
            let room = (ui.ctx().content_rect().height() - 180.0).max(120.0);
            egui::ScrollArea::vertical()
                .max_height(room)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for (driver, enabled) in [
                            (Driver::Sqlite, true),
                            (Driver::Postgres, true),
                            (Driver::MySql, true),
                        ] {
                            let selected = form.driver == driver;
                            let response = ui
                                .add_enabled_ui(enabled, |ui| {
                                    crate::ui::widgets::toggle(
                                        ui,
                                        selected,
                                        driver.label(),
                                        &look,
                                        &palette,
                                    )
                                })
                                .inner;
                            let response = if enabled {
                                response
                            } else {
                                response.on_disabled_hover_text(gettext(
                                    locale,
                                    "Coming in a later version",
                                ))
                            };
                            if response.clicked() && form.driver != driver {
                                actions.push(Action::SetDriver(driver));
                            }
                        }
                    });
                    ui.add_space(6.0);
                    egui::Grid::new("connection-form")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            let label_name = ui.label(gettext(locale, "Name")).id;
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.name, &look)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label_name);
                            ui.end_row();

                            let label_color = ui.label(gettext(locale, "Color")).id;
                            let _ = crate::ui::widgets::popup_button(
                                ui,
                                egui::ComboBox::from_id_salt("color-tag")
                                    .selected_text(form.color.label()),
                                &look,
                                &palette,
                                |ui| {
                                    for tag in ColorTag::ALL {
                                        ui.selectable_value(&mut form.color, tag, tag.label());
                                    }
                                },
                            )
                            .response
                            .labelled_by(label_color);
                            ui.end_row();

                            if form.driver == Driver::Sqlite {
                                let label_file = ui.label(gettext(locale, "File")).id;
                                ui.horizontal(|ui| {
                                    ui.add(
                                        crate::ui::widgets::single(
                                            ui,
                                            &mut form.sqlite_path,
                                            &look,
                                        )
                                        .desired_width(300.0),
                                    )
                                    .labelled_by(label_file);
                                    if ui.button(gettext(locale, "Choose…")).clicked() {
                                        actions.push(Action::PickSqliteFile);
                                    }
                                });
                                ui.end_row();
                            } else {
                                let label_host = ui.label(gettext(locale, "Host")).id;
                                ui.horizontal(|ui| {
                                    ui.add(
                                        crate::ui::widgets::single(ui, &mut form.host, &look)
                                            .desired_width(250.0),
                                    )
                                    .labelled_by(label_host);
                                    let label_port = ui.label(gettext(locale, "Port")).id;
                                    ui.add(
                                        crate::ui::widgets::single(ui, &mut form.port, &look)
                                            .desired_width(60.0),
                                    )
                                    .labelled_by(label_port);
                                });
                                ui.end_row();

                                let label_user = ui.label(gettext(locale, "User")).id;
                                ui.add(
                                    crate::ui::widgets::single(ui, &mut form.user, &look)
                                        .desired_width(f32::INFINITY),
                                )
                                .labelled_by(label_user);
                                ui.end_row();

                                let label_password = ui.label(gettext(locale, "Password")).id;
                                let hint = if form.has_saved_password {
                                    gettext(locale, "Saved in keyring")
                                } else {
                                    gettext(locale, "")
                                };
                                ui.add(
                                    crate::ui::widgets::single(ui, &mut form.password, &look)
                                        .password(true)
                                        .hint_text(hint)
                                        .desired_width(f32::INFINITY),
                                )
                                .labelled_by(label_password);
                                ui.end_row();

                                ui.label("");
                                let _ = crate::ui::widgets::popup_button(
                                    ui,
                                    egui::ComboBox::from_id_salt("password-mode").selected_text(
                                        match form.password_mode {
                                            PasswordMode::Keyring => {
                                                gettext(locale, "Save in keyring")
                                            }
                                            PasswordMode::Ask => gettext(locale, "Ask every time"),
                                            PasswordMode::None => gettext(locale, "No password"),
                                        },
                                    ),
                                    &look,
                                    &palette,
                                    |ui| {
                                        ui.selectable_value(
                                            &mut form.password_mode,
                                            PasswordMode::Keyring,
                                            gettext(locale, "Save in keyring"),
                                        );
                                        ui.selectable_value(
                                            &mut form.password_mode,
                                            PasswordMode::Ask,
                                            gettext(locale, "Ask every time"),
                                        );
                                        ui.selectable_value(
                                            &mut form.password_mode,
                                            PasswordMode::None,
                                            gettext(locale, "No password"),
                                        );
                                    },
                                )
                                .response
                                .labelled_by(label_password);
                                ui.end_row();

                                let label_database = ui.label(gettext(locale, "Database")).id;
                                ui.add(
                                    crate::ui::widgets::single(ui, &mut form.database, &look)
                                        .hint_text(gettext(locale, "Same as the user"))
                                        .desired_width(f32::INFINITY),
                                )
                                .labelled_by(label_database);
                                ui.end_row();

                                let label_tls = ui.label(gettext(locale, "TLS")).id;
                                let _ = crate::ui::widgets::popup_button(
                                    ui,
                                    egui::ComboBox::from_id_salt("tls-mode")
                                        .selected_text(tls_label(form.tls)),
                                    &look,
                                    &palette,
                                    |ui| {
                                        for (mode, label) in TLS_MODES {
                                            ui.selectable_value(&mut form.tls, mode, label);
                                        }
                                    },
                                )
                                .response
                                .labelled_by(label_tls);
                                ui.end_row();

                                if matches!(form.tls, TlsMode::VerifyCa | TlsMode::VerifyFull) {
                                    let label_ca_file = ui.label(gettext(locale, "CA file")).id;
                                    ui.add(
                                        crate::ui::widgets::single(ui, &mut form.ca_file, &look)
                                            .hint_text(gettext(locale, "System certificates"))
                                            .desired_width(f32::INFINITY),
                                    )
                                    .labelled_by(label_ca_file);
                                    ui.end_row();
                                }
                            }

                            let label_paste_url = ui.label(gettext(locale, "Paste URL")).id;
                            ui.horizontal(|ui| {
                                let response = ui
                                    .add(
                                        crate::ui::widgets::single(ui, &mut form.url, &look)
                                            .hint_text("postgres://user@host/db")
                                            .desired_width(300.0),
                                    )
                                    .labelled_by(label_paste_url);
                                let entered = response.lost_focus()
                                    && ui.input(|i| i.key_pressed(egui::Key::Enter));
                                if ui.button(gettext(locale, "Fill")).clicked() || entered {
                                    actions.push(Action::ApplyUrl);
                                }
                            });
                            ui.end_row();
                        });

                    if form.driver != Driver::Sqlite {
                        ssh_section(ui, form, locale, &look, &palette, &mut actions);
                    }

                    if let Some(message) = &form.message {
                        ui.add_space(4.0);
                        ui.label(RichText::new(message).color(palette.danger));
                    }
                    match &form.test {
                        TestState::Idle => {}
                        TestState::Running(_) => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(gettext(locale, "Testing…"));
                            });
                        }
                        TestState::Passed => {
                            ui.label(
                                RichText::new(gettext(locale, "Connection works."))
                                    .color(palette.accent),
                            );
                        }
                        TestState::Failed(message) => {
                            ui.label(RichText::new(message).color(palette.danger));
                        }
                        TestState::Untrusted {
                            host, fingerprint, ..
                        } => {
                            ui.label(
                                RichText::new(format!(
                                    "{} {host}: {fingerprint}",
                                    gettext(locale, "Unknown SSH host key for")
                                ))
                                .color(palette.warning),
                            );
                            if ui.button(gettext(locale, "Trust and test")).clicked() {
                                actions.push(Action::TrustTestHostKey);
                            }
                        }
                    }
                });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button(gettext(locale, "Test")).clicked() {
                    actions.push(Action::TestConnection);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if crate::ui::widgets::primary_button(
                        ui,
                        &gettext(locale, "Save & Connect"),
                        &look,
                        &palette,
                    )
                    .clicked()
                    {
                        actions.push(Action::SaveConnection { connect: true });
                    }
                    if ui.button(gettext(locale, "Save")).clicked() {
                        actions.push(Action::SaveConnection { connect: false });
                    }
                    if ui.button(gettext(locale, "Cancel")).clicked() {
                        actions.push(Action::CloseDialog);
                    }
                });
            });
        });
    // Escape closes the dialog, unless it is closing an open list first.
    if modal.is_top_modal
        && !modal.any_popup_open
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}

/// The collapsible SSH tunnel fields.
fn ssh_section(
    ui: &mut egui::Ui,
    form: &mut crate::model::ConnectionForm,
    locale: crate::i18n::Locale,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    actions: &mut Vec<Action>,
) {
    egui::CollapsingHeader::new(gettext(locale, "SSH tunnel"))
        .default_open(form.ssh)
        .show(ui, |ui| {
            crate::ui::widgets::checkbox(
                ui,
                &mut form.ssh,
                &gettext(locale, "Connect through SSH"),
                look,
                palette,
            );
            ui.add_enabled_ui(form.ssh, |ui| {
                egui::Grid::new("ssh-form")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        let label_ssh_host = ui.label(gettext(locale, "SSH host")).id;
                        ui.horizontal(|ui| {
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.ssh_host, look)
                                    .desired_width(250.0),
                            )
                            .labelled_by(label_ssh_host);
                            let label_ssh_port = ui.label(gettext(locale, "SSH port")).id;
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.ssh_port, look)
                                    .desired_width(60.0),
                            )
                            .labelled_by(label_ssh_port);
                        });
                        ui.end_row();

                        let label_ssh_user = ui.label(gettext(locale, "SSH user")).id;
                        ui.add(
                            crate::ui::widgets::single(ui, &mut form.ssh_user, look)
                                .desired_width(f32::INFINITY),
                        )
                        .labelled_by(label_ssh_user);
                        ui.end_row();

                        let label_auth = ui.label(gettext(locale, "Authentication")).id;
                        let _ = crate::ui::widgets::popup_button(
                            ui,
                            egui::ComboBox::from_id_salt("ssh-auth")
                                .selected_text(gettext(locale, form.ssh_auth.label())),
                            look,
                            palette,
                            |ui| {
                                for kind in SshAuthKind::ALL {
                                    ui.selectable_value(
                                        &mut form.ssh_auth,
                                        kind,
                                        gettext(locale, kind.label()),
                                    );
                                }
                            },
                        )
                        .response
                        .labelled_by(label_auth);
                        ui.end_row();

                        if form.ssh_auth == SshAuthKind::KeyFile {
                            let label_key_file = ui.label(gettext(locale, "Key file")).id;
                            ui.horizontal(|ui| {
                                ui.add(
                                    crate::ui::widgets::single(ui, &mut form.ssh_key_file, look)
                                        .desired_width(300.0),
                                )
                                .labelled_by(label_key_file);
                                if ui.button(gettext(locale, "Choose…")).clicked() {
                                    actions.push(Action::PickKeyFile);
                                }
                            });
                            ui.end_row();
                        }

                        if form.ssh_auth != SshAuthKind::Agent {
                            let label = if form.ssh_auth == SshAuthKind::Password {
                                gettext(locale, "SSH password")
                            } else {
                                gettext(locale, "Passphrase")
                            };
                            let label_secret = ui.label(label).id;
                            let hint = if form.ssh_secret_is_saved() {
                                gettext(locale, "Saved in keyring")
                            } else {
                                gettext(locale, "")
                            };
                            ui.add(
                                crate::ui::widgets::single(ui, &mut form.ssh_secret, look)
                                    .password(true)
                                    .hint_text(hint)
                                    .desired_width(f32::INFINITY),
                            )
                            .labelled_by(label_secret);
                            ui.end_row();

                            ui.label("");
                            let _ = crate::ui::widgets::popup_button(
                                ui,
                                egui::ComboBox::from_id_salt("ssh-secret-mode").selected_text(
                                    match form.ssh_secret_mode {
                                        PasswordMode::Ask => gettext(locale, "Ask every time"),
                                        _ => gettext(locale, "Save in keyring"),
                                    },
                                ),
                                look,
                                palette,
                                |ui| {
                                    ui.selectable_value(
                                        &mut form.ssh_secret_mode,
                                        PasswordMode::Keyring,
                                        gettext(locale, "Save in keyring"),
                                    );
                                    ui.selectable_value(
                                        &mut form.ssh_secret_mode,
                                        PasswordMode::Ask,
                                        gettext(locale, "Ask every time"),
                                    );
                                },
                            )
                            .response
                            .labelled_by(label_secret);
                            ui.end_row();
                        }
                    });
            });
        });
}
