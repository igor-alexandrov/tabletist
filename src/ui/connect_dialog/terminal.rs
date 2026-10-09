//! The connection dialog as the terminal look draws it: a two-column form
//! of labels and fields under headings, with its keys in the footer.

use egui::{Color32, Rect, Response, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::Driver;

use crate::connections::PasswordMode;
use crate::i18n::gettext;
use crate::keymap::Command;
use crate::model::{Action, ConnectionForm, SshAuthKind};
use crate::typography::{Text, TextRole};
use crate::ui::terminal_dialog::{self, Key};
use crate::ui::widgets::{self, ButtonSpec};

use super::choice::{Choice, Group, choose, driver_choice, environment_choice};
use super::{
    Skin, StatusAt, TLS_MODES, ca_field, connection_line, field_with_button, file_field, hint,
    input, intercept_warning, keeps, named, note, paint_status, set_keeps, show_url,
    ssh_host_field, ssh_host_note, ssh_key_hint, ssh_port_hint, ssh_secret_name, ssh_user_hint,
    subtitle, title, url_field,
};

/// The terminal look: one tinted line with the title, the connection, and
/// the key that shows the URL field.
pub(super) fn terminal_header(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let rect = terminal_dialog::head(ui, skin.inner_radius(), skin.env.bar_bg(), palette);
    let y = terminal_dialog::head_line(rect);
    let label = skin.say("Paste URL");
    let key = crate::ui::keys::written(ui.ctx(), look, Command::FormPasteUrl);
    let hint = [(&key, label.as_str(), true)];
    let width = widgets::key_hints_width(ui, &hint, 0.0, look, palette);
    let place = Rect::from_min_size(
        pos2(rect.right() - 14.0 - width, rect.top()),
        vec2(width, 40.0),
    );
    // The title, then the connection, which ends before the hint.
    let mut x = rect.left() + 14.0;
    x += widgets::paint_label(
        ui,
        x,
        y,
        Text::one(
            look,
            TextRole::OScreenTitle,
            &skin.say(title(form)),
            palette.text,
        ),
    ) + 12.0;
    let room = place.left() - 12.0 - x;
    connection_line(
        ui,
        (x, y),
        &subtitle(form, skin),
        TextRole::OBody,
        room,
        skin,
    );
    widgets::key_hints(ui, (place.left(), y), &hint, 0.0, look, palette);
    let name = gettext(skin.locale, "Paste URL");
    if ButtonSpec::new(&name).hidden_at(ui, place).clicked() {
        show_url(ui.ctx(), form, !form.url_mode);
    }
}

/// The terminal look: the Test's status at the left, the keys at the
/// right. Each key's hint is its button too. The status comes first: the
/// hints that do not fit beside it are left out, from the left, and their
/// buttons stay for screen readers.
pub(super) fn terminal_footer(
    ui: &mut Ui,
    form: &ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let rect = terminal_dialog::foot(ui, skin.inner_radius(), palette);
    let y = terminal_dialog::foot_line(rect);
    let status = paint_status(ui, form, StatusAt::Left(rect.left() + 14.0), y, skin);
    // The command whose key it is, what it does, the button it stands
    // for, and what that does.
    let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
    let keys = [
        (Command::FormNextField, "next", None),
        (
            Command::FormTest,
            "test",
            Some(("Test", Action::TestConnection)),
        ),
        (
            Command::FormSave,
            "save",
            Some(("Save", Action::SaveConnection { connect: false })),
        ),
        (
            Command::FormSaveAndConnect,
            "connect",
            Some(("Save & Connect", Action::SaveConnection { connect: true })),
        ),
        (
            Command::FormCancel,
            "cancel",
            Some(("Cancel", Action::CloseDialog)),
        ),
    ];
    let written: Vec<_> = keys.iter().map(|(command, ..)| key(*command)).collect();
    let names: Vec<_> = keys
        .iter()
        .map(|(_, _, button)| button.as_ref().map(|(name, _)| gettext(skin.locale, name)))
        .collect();
    let hints: Vec<Key<'_>> = keys
        .iter()
        .zip(&names)
        .zip(&written)
        .map(|(((command, label, _), name), key)| Key {
            key,
            label,
            button: name.as_deref(),
            // Saving is what the dialog is for: its key takes the accent.
            lead: *command == Command::FormSave,
            disabled: None,
        })
        .collect();
    // What the status leaves, 16 clear of it.
    let taken = if status > 0.0 { status + 16.0 } else { 0.0 };
    let pressed = terminal_dialog::keys(ui, rect, taken, &hints, look, palette);
    if let Some(index) = pressed
        && let Some((_, _, Some((_, action)))) = keys.into_iter().nth(index)
    {
        actions.push(action);
    }
}

/// The terminal look: a label in the 150 pt column, then what `add` draws
/// on the same line, 8 below the row before it. The row is as tall as a
/// field.
pub(super) fn terminal_row(
    ui: &mut Ui,
    label: &str,
    skin: &Skin,
    add: impl FnOnce(&mut Ui, egui::Id),
) {
    terminal_row_of(ui, label, skin.field_height(), skin, add);
}

/// [`terminal_row`] `height` tall: a row of words or of buttons is as tall
/// as they are, as the design's rows are.
fn terminal_row_of(
    ui: &mut Ui,
    label: &str,
    height: f32,
    skin: &Skin,
    add: impl FnOnce(&mut Ui, egui::Id),
) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let center = egui::Layout::left_to_right(egui::Align::Center);
    // Not `Ui::horizontal`: that row is never shorter than a control.
    ui.allocate_ui_with_layout(vec2(width, height), center, |ui| {
        ui.spacing_mut().item_spacing.x = 14.0;
        let label = ui
            .allocate_ui_with_layout(vec2(150.0, height), center, |ui| {
                ui.set_min_size(vec2(150.0, height));
                widgets::label(ui, TextRole::OBody, label, palette.dim, look)
            })
            .inner;
        ui.allocate_ui_with_layout(vec2(width - 164.0, height), center, |ui| {
            ui.set_min_size(vec2(width - 164.0, height));
            ui.spacing_mut().item_spacing.x = 8.0;
            add(ui, label.id);
        });
    });
    ui.add_space(8.0);
}

/// A cell `width` wide on a terminal row, for a field that would take the
/// whole line otherwise. It is as tall as the field in it: the row centres
/// what it holds, and a cell of no height would hang from the row's middle.
fn cell(ui: &mut Ui, width: f32, skin: &Skin, add: impl FnOnce(&mut Ui) -> Response) -> Response {
    ui.allocate_ui_with_layout(
        vec2(width, skin.field_height()),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.set_width(width);
            add(ui)
        },
    )
    .inner
}

/// The terminal look: a heading over a rule, 8 below the rows before it.
fn terminal_section(ui: &mut Ui, text: &str, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    ui.add_space(8.0);
    widgets::label(ui, TextRole::OGroup, text, palette.text, look);
    ui.add_space(4.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.center().y, palette.outline);
    ui.add_space(8.0);
}

/// The terminal look's check box: `[x]` and what it turns on. `mark` is
/// the colour of a set mark.
fn terminal_check(
    ui: &mut Ui,
    checked: &mut bool,
    text: &str,
    name: &str,
    mark: Color32,
    skin: &Skin,
) -> Response {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let on = *checked;
    let (glyph, color) = if on {
        ("[x]", mark)
    } else {
        ("[ ]", palette.dim)
    };
    let mut laid = Text::new(look).add(role, glyph, color);
    if !text.is_empty() {
        laid = laid.space(role, " ").add(role, text, palette.dim);
    }
    let laid = laid.layout(ui.ctx());
    let (rect, mut response) = ui.allocate_exact_size(laid.size(), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
    laid.paint(ui.painter(), rect.min);
    if response.clicked() {
        *checked = !*checked;
        response.mark_changed();
    }
    response
}

/// The terminal look's "keyring" check after a secret; returns its width.
fn terminal_keyring_width(ui: &Ui, skin: &Skin) -> f32 {
    let text = format!("[x] {}", skin.say(skin.keyring()));
    TextRole::OBody.width(ui.ctx(), skin.look.faces, &text)
}

/// The terminal look's "keyring" check; screen readers hear `name`, which
/// says whose secret it keeps.
fn terminal_keyring(ui: &mut Ui, mode: &mut PasswordMode, name: &'static str, skin: &Skin) {
    let mut keep = keeps(*mode);
    let text = skin.say(skin.keyring());
    let name = gettext(skin.locale, name);
    if terminal_check(ui, &mut keep, &text, &name, skin.palette.success, skin).changed() {
        set_keeps(mode, keep);
    }
}

/// The terminal look's body: label and field, a line each, under headings.
pub(super) fn terminal_body(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let height = skin.field_height();
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 18,
            right: 18,
            top: 16,
            // The last row's 8, and 8 more.
            bottom: 8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if form.url_mode {
                terminal_row(ui, &skin.say("URL"), skin, |ui, label| {
                    let field = url_field(ui, form, skin).labelled_by(label);
                    // The fields are in view under the URL, so they must
                    // show what will be saved: a URL fills them when the
                    // keyboard leaves it, by Enter or to edit a field.
                    if field.lost_focus() && !form.url.trim().is_empty() {
                        actions.push(Action::ApplyUrl);
                    }
                });
            }
            terminal_row(ui, &skin.say("Name"), skin, |ui, label| {
                let name = ui
                    .add(input(ui, &mut form.name, role, height, skin))
                    .labelled_by(label);
                if focus_name {
                    name.request_focus();
                }
            });
            let heading = skin.say("Type");
            terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
                driver_choice(ui, form, &heading, skin, actions);
            });
            let heading = skin.say("Environment");
            terminal_row_of(ui, &heading, Group::Buttons.height(), skin, |ui, _| {
                environment_choice(ui, form, &heading, skin);
            });
            match form.driver {
                Driver::Sqlite => {
                    terminal_section(ui, &skin.say("Database"), skin);
                    file_field(ui, form, skin, actions);
                }
                Driver::Postgres | Driver::MySql => {
                    terminal_section(ui, &skin.say("Server"), skin);
                    terminal_server(ui, form, skin);
                    terminal_section(ui, &skin.say("Security"), skin);
                    terminal_security(ui, form, skin, actions);
                }
            }
            terminal_section(ui, &skin.say("Safety"), skin);
            // A line of text, as tall as the text.
            let line = role.row_height(ui.ctx(), look.faces);
            terminal_row_of(ui, &skin.say("Read-only"), line, skin, |ui, _| {
                let name = gettext(skin.locale, "Open read-only");
                let promise = skin.say("Block every write from this app");
                let note = format!("· {}", skin.say("Default for production"));
                // The note goes under the promise where one line has no
                // room for both: no row may widen the dialog.
                let width = |text: &str| role.width(ui.ctx(), look.faces, text);
                let one_line =
                    width(&format!("[x] {promise}")) + 8.0 + width(&note) <= ui.available_width();
                let mut draw = |ui: &mut Ui| {
                    // The environment's colour, as text can take it.
                    let mark = skin.env_ink(&skin.env);
                    let mut on = form.read_only();
                    if terminal_check(ui, &mut on, &promise, &name, mark, skin).changed() {
                        form.read_only = Some(on);
                    }
                    widgets::label(ui, role, &note, palette.dim, look);
                };
                if one_line {
                    draw(ui);
                } else {
                    ui.vertical(draw);
                }
            });
        });
}

/// The terminal look: the host and port on one line, then the database,
/// the user and the password.
fn terminal_server(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let role = TextRole::OBody;
    let height = skin.field_height();
    terminal_row(ui, &skin.say("Host : Port"), skin, |ui, _| {
        let host = ui.available_width() - 88.0;
        let field = cell(ui, host, skin, |ui| {
            ui.add(input(ui, &mut form.host, role, height, skin))
        });
        named(&field, "Host", skin);
        let field = cell(ui, 80.0, skin, |ui| {
            ui.add(input(ui, &mut form.port, role, height, skin))
        });
        named(&field, "Port", skin);
    });
    terminal_row(ui, &skin.say("Database"), skin, |ui, label| {
        let hint = hint(ui, &skin.say("Same as the user"), role, skin);
        ui.add(input(ui, &mut form.database, role, height, skin).hint_text(hint))
            .labelled_by(label);
    });
    terminal_row(ui, &skin.say("User"), skin, |ui, label| {
        ui.add(input(ui, &mut form.user, role, height, skin))
            .labelled_by(label);
    });
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    terminal_row(ui, &skin.say("Password"), skin, |ui, label| {
        let field = ui.available_width() - terminal_keyring_width(ui, skin) - 10.0;
        ui.spacing_mut().item_spacing.x = 10.0;
        cell(ui, field, skin, |ui| {
            let hint = hint(ui, &saved, role, skin);
            ui.add(
                input(ui, &mut form.password, role, height, skin)
                    .password(true)
                    .hint_text(hint),
            )
        })
        .labelled_by(label);
        terminal_keyring(ui, &mut form.password_mode, skin.keyring(), skin);
    });
}

/// The terminal look: the TLS mode as words, the CA file, and the tunnel.
fn terminal_security(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::OBody;
    let height = skin.field_height();
    let heading = skin.say("SSL mode");
    terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
        let options = TLS_MODES.map(|(mode, label)| (mode, Choice::plain(label)));
        let clicked = choose(
            ui,
            "tls-mode",
            Some(&heading),
            &options,
            form.tls,
            Group::Words,
            skin,
        );
        if let Some(mode) = clicked {
            form.tls = mode;
        }
    });
    terminal_row(ui, &skin.say("CA cert"), skin, |ui, label| {
        ca_field(ui, form, label, skin, actions);
    });
    if form.password_can_be_intercepted() {
        intercept_warning(ui, skin);
        ui.add_space(8.0);
    }
    terminal_row(ui, &skin.say("SSH tunnel"), skin, |ui, _| {
        let name = gettext(skin.locale, "Connect through SSH tunnel");
        terminal_check(
            ui,
            &mut form.ssh,
            &skin.say("Connect through SSH"),
            &name,
            skin.palette.success,
            skin,
        );
    });
    if !form.ssh {
        return;
    }
    // What an empty port, user and key file mean: the typed host's values
    // from ~/.ssh/config, else 22 and the login name.
    let hints = form.ssh_hints();
    terminal_row(ui, &skin.say("SSH host : Port"), skin, |ui, _| {
        let host = ui.available_width() - 88.0;
        // The button that lists the config's hosts follows the field.
        let field = cell(ui, host, skin, |ui| {
            ssh_host_field(ui, form, role, "", skin, actions)
        });
        named(&field, "SSH host", skin);
        let field = cell(ui, 80.0, skin, |ui| {
            let hint = hint(ui, &ssh_port_hint(&hints), role, skin);
            ui.add(input(ui, &mut form.ssh_port, role, height, skin).hint_text(hint))
        });
        named(&field, "SSH port", skin);
    });
    // What the config resolves the host to, in the fields' column.
    if let Some((text, color)) = ssh_host_note(&hints, skin) {
        let line = widgets::secondary(skin.look).row_height(ui.ctx(), skin.look.faces);
        terminal_row_of(ui, "", line, skin, |ui, _| {
            note(ui, &text, color, skin);
        });
    }
    terminal_row(ui, &skin.say("SSH user"), skin, |ui, label| {
        let hint = hint(ui, &ssh_user_hint(&hints, skin), role, skin);
        ui.add(input(ui, &mut form.ssh_user, role, height, skin).hint_text(hint))
            .labelled_by(label);
    });
    let heading = skin.say("Authentication");
    terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
        let labels = SshAuthKind::ALL.map(|kind| skin.say(kind.label()));
        let options: Vec<(SshAuthKind, Choice<'_>)> = SshAuthKind::ALL
            .iter()
            .zip(&labels)
            .map(|(kind, label)| (*kind, Choice::plain(label)))
            .collect();
        let clicked = choose(
            ui,
            "ssh-auth",
            Some(&heading),
            &options,
            form.ssh_auth,
            Group::Words,
            skin,
        );
        if let Some(kind) = clicked {
            form.ssh_auth = kind;
        }
    });
    if form.ssh_auth == SshAuthKind::KeyFile {
        terminal_row(ui, &skin.say("Key file"), skin, |ui, label| {
            let key = ssh_key_hint(&hints, skin).unwrap_or_default();
            let (field, clicked) =
                field_with_button(ui, &skin.say("Choose…"), "Choose a key file", skin, |ui| {
                    let hint = hint(ui, &key, role, skin);
                    ui.add(input(ui, &mut form.ssh_key_file, role, height, skin).hint_text(hint))
                });
            field.labelled_by(label);
            if clicked {
                actions.push(Action::PickKeyFile);
            }
        });
    }
    if form.ssh_auth != SshAuthKind::Agent {
        let name = ssh_secret_name(form.ssh_auth);
        let saved = if form.ssh_secret_is_saved() {
            skin.say(skin.saved_hint())
        } else {
            String::new()
        };
        terminal_row(ui, &skin.say(name), skin, |ui, label| {
            let field = ui.available_width() - terminal_keyring_width(ui, skin) - 10.0;
            ui.spacing_mut().item_spacing.x = 10.0;
            cell(ui, field, skin, |ui| {
                let hint = hint(ui, &saved, role, skin);
                ui.add(
                    input(ui, &mut form.ssh_secret, role, height, skin)
                        .password(true)
                        .hint_text(hint),
                )
            })
            .labelled_by(label);
            terminal_keyring(ui, &mut form.ssh_secret_mode, skin.ssh_keyring(), skin);
        });
    }
}
