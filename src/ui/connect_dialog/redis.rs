//! A Redis connection's groups of the dialog, in both looks: the server
//! and its database, signing in, TLS, and how the keys are browsed. The
//! SSH tunnel and the safety box are the other servers'.

use egui::{Response, Ui};

use crate::i18n::gettext;
use crate::model::{Action, ConnectionForm};
use crate::typography::{Text, TextRole};
use crate::ui::menu;
use crate::ui::widgets;

use super::choice::{Choice, Group, choose};
use super::sheet::{keyring_box, labelled, select};
use super::terminal::{
    cell, terminal_check, terminal_keyring, terminal_keyring_width, terminal_row, terminal_row_of,
};
use super::{
    Column, Skin, ca_field, field_with_button, forget_saved_password, hint, input, named, note, row,
};

/// What Sign in says under its fields.
const SIGN_IN_HINT: &str = "Leave Username empty to sign in as the default user with a \
                            password only. Leave both empty for a server without a password.";

/// What Browsing says under its fields.
const BROWSING_HINT: &str =
    "Keys are listed with SCAN, never KEYS, so a large database doesn't block the server.";

/// What the safety box says of the server's side.
const ACL_NOTE: &str = "Tabletist sends only read commands to Redis. For the server to refuse \
                        writes too, sign in as an ACL user without write permission.";

/// The databases the dropdown lists: the last Test's count, else Redis's
/// 16.
fn databases(form: &ConnectionForm) -> u32 {
    form.redis_probe
        .as_ref()
        .map_or(16, |probe| probe.databases.max(1))
}

/// A database as the dropdown names it: its number, and its keys after a
/// Test ("0 · 1,204 keys", "2 · empty").
fn database_label(form: &ConnectionForm, number: u32) -> String {
    match &form.redis_probe {
        Some(probe) => crate::keyspace::database_label(number, probe.keys_in(number)),
        None => number.to_string(),
    }
}

/// "Group keys by `:` · 1,000 keys at a time · all keys": Browsing's
/// summary while it is closed.
fn browsing_summary(form: &ConnectionForm, skin: &Skin) -> String {
    let by = format!(
        "{} {}",
        skin.say("Group keys by"),
        match form.redis_separator.as_str() {
            "" => skin.say("nothing"),
            separator => format!("`{separator}`"),
        }
    );
    let page = format!(
        "{} {}",
        crate::format::group_digits(u64::from(form.redis_page)),
        skin.say("keys at a time")
    );
    let which = match form.redis_pattern.trim() {
        "" | "*" => skin.say("all keys"),
        pattern => format!("{} {pattern}", skin.say("keys matching")),
    };
    format!("{by} · {page} · {which}")
}

/// macOS: host and port, then the database.
pub(super) fn server(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let role = skin.value_role();
    let small = TextRole::Secondary;
    let height = skin.field_height();
    let columns = [Column::Fr(1.0), Column::Px(96.0)];
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("Host"), small, skin, |ui| {
                ui.add(input(ui, &mut form.host, role, height, skin))
            });
        } else {
            labelled(ui, &skin.say("Port"), small, skin, |ui| {
                ui.add(input(ui, &mut form.port, role, height, skin))
            });
        }
    });
    ui.add_space(10.0);
    row(
        ui,
        12.0,
        &[Column::Fr(1.0), Column::Fr(1.0)],
        |ui, column| {
            if column == 0 {
                let width = ui.available_width();
                labelled(ui, &skin.say("Database"), small, skin, |ui| {
                    database_dropdown(ui, form, width, skin)
                });
            }
        },
    );
}

/// The database dropdown: a list of the databases, with their keys after
/// a Test.
fn database_dropdown(ui: &mut Ui, form: &mut ConnectionForm, width: f32, skin: &Skin) -> Response {
    let Skin { look, palette, .. } = *skin;
    let current = database_label(form, form.redis_database);
    let count = databases(form);
    let labels: Vec<String> = (0..count).map(|n| database_label(form, n)).collect();
    let response = select(ui, "redis-database", &current, width, skin)
        .show(ui, look, palette, |ui| {
            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                node.set_role(egui::accesskit::Role::ListBox);
            });
            for (number, label) in labels.iter().enumerate() {
                let number = u32::try_from(number).unwrap_or(0);
                menu::value(ui, &mut form.redis_database, number, label, look, palette);
            }
        })
        .response;
    let name = gettext(skin.locale, "Database");
    response.widget_info(|| {
        let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, &*name);
        info.current_text_value = Some(current.clone());
        info
    });
    response
}

/// macOS: the username (empty for the default user), the password with
/// its keyring box, and what leaving them empty means.
pub(super) fn sign_in(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = skin.value_role();
    let small = TextRole::Secondary;
    let height = skin.field_height();
    let columns = [Column::Fr(1.0), Column::Px(96.0)];
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("Username"), small, skin, |ui| {
                let hint = hint(ui, "default", role, skin);
                ui.add(input(ui, &mut form.user, role, height, skin).hint_text(hint))
            });
        }
    });
    ui.add_space(10.0);
    let forgettable = form.password.is_empty() && form.password_is_saved();
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    let mut field = None;
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            field = Some(labelled(ui, &skin.say("Password"), small, skin, |ui| {
                let hint = hint(ui, &saved, TextRole::UiBody, skin);
                ui.add(
                    input(ui, &mut form.password, TextRole::UiBody, height, skin)
                        .password(true)
                        .hint_text(hint),
                )
            }));
        } else {
            keyring_box(ui, &mut form.password_mode, skin.keyring(), true, skin);
        }
    });
    if let Some(field) = field {
        forget_saved_password(ui, &field, forgettable, skin, actions);
    }
    ui.add_space(8.0);
    note(ui, &skin.say(SIGN_IN_HINT), skin.palette.dim, skin);
}

/// macOS: Use TLS, and under it the CA, the client's certificate and key,
/// and the check of the server's name. The tunnel follows (`ssh_part`).
pub(super) fn security(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = widgets::body(look).row_height(ui.ctx(), look.faces);
        widgets::checkbox(
            ui,
            &mut form.redis_tls,
            &skin.say("Use TLS (rediss://)"),
            look,
            palette,
        );
    });
    if form.redis_tls {
        ui.add_space(10.0);
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: 22,
                ..egui::Margin::ZERO
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                tls_fields(ui, form, skin, actions);
            });
    }
    if form.password_can_be_intercepted() {
        ui.add_space(10.0);
        super::intercept_warning(ui, skin);
    }
}

fn tls_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let small = TextRole::Secondary;
    let label = widgets::label(
        ui,
        small,
        &skin.say("CA certificate"),
        palette.secondary,
        look,
    );
    ui.add_space(5.0);
    ca_field(ui, form, label.id, skin, actions);
    ui.add_space(10.0);
    row(
        ui,
        12.0,
        &[Column::Fr(1.0), Column::Fr(1.0)],
        |ui, column| {
            let (text, which) = if column == 0 {
                ("Client certificate", false)
            } else {
                ("Client key", true)
            };
            let label = widgets::label(ui, small, &skin.say(text), palette.secondary, look);
            ui.add_space(5.0);
            client_file(ui, form, which, label.id, skin, actions);
        },
    );
    ui.add_space(10.0);
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = widgets::body(look).row_height(ui.ctx(), look.faces);
        widgets::checkbox(
            ui,
            &mut form.redis_verify_host,
            &skin.say("Verify the server's hostname"),
            look,
            palette,
        );
    });
}

/// The client's certificate (`key` false) or its key, with Choose….
fn client_file(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    key: bool,
    label: egui::Id,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(skin.look, TextRole::MonoSecondary, TextRole::OBody);
    let height = skin.field_height();
    let (name, placeholder): (&'static str, &'static str) = if key {
        ("Choose the client key", "Optional")
    } else {
        ("Choose a client certificate", "Optional")
    };
    let placeholder = skin.say(placeholder);
    let (field, clicked) = field_with_button(ui, &skin.say("Choose…"), name, skin, |ui| {
        let hint = hint(ui, &placeholder, role, skin);
        let text = if key {
            &mut form.redis_key
        } else {
            &mut form.redis_cert
        };
        ui.add(input(ui, text, role, height, skin).hint_text(hint))
    });
    field.labelled_by(label);
    if clicked {
        actions.push(Action::PickRedisFile { key });
    }
}

/// macOS: Browsing, a line that opens on its fields: the separator, keys
/// at a time, the pattern and the key limit.
pub(super) fn browsing(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let summary = browsing_summary(form, skin);
    let title = skin.say("Browsing");
    let open = form.redis_browsing_open;
    let frame = egui::Frame::new()
        .stroke(egui::Stroke::new(widgets::hairline(ui), skin.rule()))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 10));
    frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        let line = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            let mark = if open { "▾" } else { "▸" };
            widgets::label(
                ui,
                TextRole::UiBodyStrong,
                &format!("{mark} {title}"),
                palette.text,
                look,
            );
            if !open {
                let width = ui.available_width();
                let shown = crate::ui::grid::ellipsize(&summary, width, false, |text| {
                    TextRole::Secondary.width(ui.ctx(), look.faces, text)
                });
                widgets::label(ui, TextRole::Secondary, &shown, palette.secondary, look);
            }
        });
        let toggle = ui.interact(
            line.response.rect,
            ui.id().with("redis-browsing"),
            egui::Sense::click(),
        );
        toggle.widget_info(|| {
            egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, &title)
        });
        if toggle.clicked() {
            form.redis_browsing_open = !open;
        }
        if open {
            ui.add_space(12.0);
            browsing_fields(ui, form, skin);
        }
    });
}

fn browsing_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = skin.value_role();
    let small = TextRole::Secondary;
    let height = skin.field_height();
    row(
        ui,
        12.0,
        &[Column::Px(96.0), Column::Fr(1.0)],
        |ui, column| {
            if column == 0 {
                labelled(ui, &skin.say("Key separator"), small, skin, |ui| {
                    ui.add(input(ui, &mut form.redis_separator, role, height, skin))
                });
            } else {
                let heading = skin.say("Keys at a time");
                widgets::label(ui, small, &heading, palette.secondary, look);
                ui.add_space(5.0);
                page_choice(ui, form, &heading, skin);
            }
        },
    );
    ui.add_space(10.0);
    row(
        ui,
        12.0,
        &[Column::Fr(1.0), Column::Px(140.0)],
        |ui, column| {
            if column == 0 {
                labelled(ui, &skin.say("Only keys matching"), small, skin, |ui| {
                    ui.add(
                        input(ui, &mut form.redis_pattern, role, height, skin)
                            .hint_text(hint(ui, "*", role, skin)),
                    )
                });
            } else {
                labelled(ui, &skin.say("Key limit"), small, skin, |ui| {
                    ui.add(input(ui, &mut form.redis_key_limit, role, height, skin))
                });
            }
        },
    );
    ui.add_space(8.0);
    note(ui, &skin.say(BROWSING_HINT), palette.dim, skin);
}

/// Keys at a time: 500, 1,000 or 5,000.
fn page_choice(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, skin: &Skin) {
    let labels = tabletist_db::KEY_PAGES.map(|page| crate::format::group_digits(u64::from(page)));
    let options: Vec<(u32, Choice<'_>)> = tabletist_db::KEY_PAGES
        .iter()
        .zip(&labels)
        .map(|(page, label)| (*page, Choice::plain(label)))
        .collect();
    let group = if skin.look.terminal {
        Group::Words
    } else {
        Group::Track(skin.field_height() - 2.0)
    };
    if let Some(page) = choose(
        ui,
        "redis-page",
        Some(heading),
        &options,
        form.redis_page,
        group,
        skin,
    ) {
        form.redis_page = page;
    }
}

/// The note under the safety box: what the server itself guarantees.
pub(super) fn acl_note(ui: &mut Ui, skin: &Skin) {
    note(ui, &skin.say(ACL_NOTE), skin.palette.dim, skin);
}

/// What Agents says on a Redis connection: nothing to give them yet. A
/// row that cannot be opened, in the agents' place.
pub(super) fn agents(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let title = skin.say("Agents");
    let later = skin.say("Agents for Redis come later");
    if look.terminal {
        super::terminal::terminal_section(ui, &title, skin);
        let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
        terminal_row_of(ui, &skin.say("Access"), line, skin, |ui, _| {
            widgets::label(ui, TextRole::OBody, &later, palette.dim, look);
        });
        return;
    }
    egui::Frame::new()
        .stroke(egui::Stroke::new(widgets::hairline(ui), skin.rule()))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(14, 10))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(false, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    widgets::label(
                        ui,
                        TextRole::UiBodyStrong,
                        &format!("▸ {title}"),
                        palette.dim,
                        look,
                    );
                    widgets::label(ui, TextRole::Secondary, &skin.say("Off"), palette.dim, look);
                    widgets::label(ui, TextRole::Secondary, &later, palette.dim, look);
                });
            });
        });
}

/// The terminal look: host and port on one line, the database, the
/// username, the password, and the hint.
pub(super) fn terminal_server(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
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
        let width = ui.available_width();
        database_dropdown(ui, form, width, skin).labelled_by(label);
    });
    super::terminal::terminal_section(ui, &skin.say("Sign in"), skin);
    terminal_row(ui, &skin.say("Username"), skin, |ui, label| {
        let hint = hint(ui, "default", role, skin);
        ui.add(input(ui, &mut form.user, role, height, skin).hint_text(hint))
            .labelled_by(label);
    });
    let forgettable = form.password.is_empty() && form.password_is_saved();
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    terminal_row(ui, &skin.say("Password"), skin, |ui, label| {
        let field = ui.available_width() - terminal_keyring_width(ui, skin) - 10.0;
        ui.spacing_mut().item_spacing.x = 10.0;
        let field = cell(ui, field, skin, |ui| {
            let hint = hint(ui, &saved, role, skin);
            ui.add(
                input(ui, &mut form.password, role, height, skin)
                    .password(true)
                    .hint_text(hint),
            )
        })
        .labelled_by(label);
        terminal_keyring(ui, &mut form.password_mode, skin.keyring(), skin);
        forget_saved_password(ui, &field, forgettable, skin, actions);
    });
    terminal_note(ui, &skin.say(SIGN_IN_HINT), skin);
}

/// A dim note in the fields' column of the terminal look.
fn terminal_note(ui: &mut Ui, text: &str, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width() - 164.0;
    let laid = Text::one(look, widgets::secondary(look), text, palette.dim)
        .wrap(width.max(80.0))
        .layout(ui.ctx());
    terminal_row_of(ui, "", laid.height(), skin, |ui, _| {
        laid.label(ui);
    });
}

/// The terminal look: TLS, with what it needs under it. The tunnel's rows
/// follow (`terminal_ssh`).
pub(super) fn terminal_security(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let line = TextRole::OBody.row_height(ui.ctx(), skin.look.faces);
    terminal_row_of(ui, &skin.say("TLS"), line, skin, |ui, _| {
        let name = gettext(skin.locale, "Use TLS (rediss://)");
        terminal_check(
            ui,
            &mut form.redis_tls,
            "rediss://",
            &name,
            skin.palette.success,
            skin,
        );
    });
    if form.redis_tls {
        terminal_row(ui, &skin.say("CA cert"), skin, |ui, label| {
            ca_field(ui, form, label, skin, actions);
        });
        for (text, key) in [("Client cert", false), ("Client key", true)] {
            terminal_row(ui, &skin.say(text), skin, |ui, label| {
                client_file(ui, form, key, label, skin, actions);
            });
        }
        terminal_row_of(ui, &skin.say("Verify host"), line, skin, |ui, _| {
            let name = gettext(skin.locale, "Verify the server's hostname");
            terminal_check(
                ui,
                &mut form.redis_verify_host,
                &skin.say("Check the name in the certificate"),
                &name,
                skin.palette.success,
                skin,
            );
        });
    }
    if form.password_can_be_intercepted() {
        super::intercept_warning(ui, skin);
        ui.add_space(8.0);
    }
}

/// The terminal look's Browsing: a heading that opens on its rows, its
/// summary beside it while it is closed.
pub(super) fn terminal_browsing(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::OBody;
    let height = skin.field_height();
    let open = form.redis_browsing_open;
    let title = skin.say("Browsing");
    ui.add_space(8.0);
    let line = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let mark = if open { "▾" } else { "▸" };
        widgets::label(
            ui,
            TextRole::OGroup,
            &format!("{mark} {title}"),
            palette.text,
            look,
        );
        if !open {
            let summary = browsing_summary(form, skin);
            let width = ui.available_width();
            let shown = crate::ui::grid::ellipsize(&summary, width, false, |text| {
                role.width(ui.ctx(), look.faces, text)
            });
            widgets::label(ui, role, &shown, palette.dim, look);
        }
    });
    let toggle = ui.interact(
        line.response.rect,
        ui.id().with("redis-browsing"),
        egui::Sense::click(),
    );
    toggle.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::CollapsingHeader, true, &title)
    });
    if toggle.clicked() {
        form.redis_browsing_open = !open;
    }
    ui.add_space(4.0);
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.center().y, palette.outline);
    ui.add_space(8.0);
    if !open {
        return;
    }
    terminal_row(ui, &skin.say("Key separator"), skin, |ui, label| {
        cell(ui, 60.0, skin, |ui| {
            ui.add(input(ui, &mut form.redis_separator, role, height, skin))
        })
        .labelled_by(label);
    });
    let heading = skin.say("Keys at a time");
    terminal_row_of(ui, &heading, Group::Words.height(), skin, |ui, _| {
        page_choice(ui, form, &heading, skin);
    });
    terminal_row(ui, &skin.say("Only matching"), skin, |ui, label| {
        cell(ui, 220.0, skin, |ui| {
            let hint = hint(ui, "*", role, skin);
            ui.add(input(ui, &mut form.redis_pattern, role, height, skin).hint_text(hint))
        })
        .labelled_by(label);
    });
    terminal_row(ui, &skin.say("Key limit"), skin, |ui, label| {
        cell(ui, 120.0, skin, |ui| {
            ui.add(input(ui, &mut form.redis_key_limit, role, height, skin))
        })
        .labelled_by(label);
    });
    terminal_note(ui, &skin.say(BROWSING_HINT), skin);
}
