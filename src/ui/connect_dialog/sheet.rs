//! The connection dialog as macOS and the standard look draw it: a sheet
//! of labelled fields in groups, between a header and a footer of buttons.

use egui::{
    CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::{Driver, TlsMode};

use crate::connections::PasswordMode;
use crate::env::Environment;
use crate::i18n::gettext;
use crate::model::{Action, ConnectionForm, SshAuthKind};
use crate::theme::{self, Icon};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

use super::choice::{Choice, Group, choose, driver_choice, environment_choice};
use super::{
    Column, Skin, StatusAt, TLS_MODES, ca_field, connection_line, field_with_button, file_field,
    hint, input, intercept_warning, keeps, named, note, paint_check, paint_status, row, set_keeps,
    show_url, ssh_host_field, ssh_host_note, ssh_key_hint, ssh_port_hint, ssh_secret_name,
    ssh_user_hint, subtitle, title, url_field,
};

/// The stripe in the environment's colour along the macOS dialog's top.
/// A connection with no environment has none: its grey would merge with
/// the dimmed window behind the sheet and square off the corners. The
/// room stays, so nothing moves when an environment is chosen.
const STRIPE: f32 = 5.0;

fn tls_label(mode: TlsMode) -> &'static str {
    TLS_MODES
        .iter()
        .find(|(m, _)| *m == mode)
        .map_or("", |(_, label)| label)
}

/// macOS: the environment's stripe (when there is one), the title over the
/// connection's name, the Parameters and URL tabs, and Close.
pub(super) fn header(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let heading = Text::one(
        look,
        TextRole::DialogTitle,
        &skin.say(title(form)),
        palette.text,
    )
    .layout(ui.ctx());
    let role = TextRole::UiBody;
    let connection = subtitle(form, skin);
    // The second line is as tall as its text, however much of it shows.
    let line = Text::one(look, role, &connection, palette.dim)
        .layout(ui.ctx())
        .height();
    // 16 above, the two lines 2 apart, 12 below.
    let block = heading.height() + 2.0 + line;
    let height = STRIPE + 16.0 + block + 12.0;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if skin.environment != Environment::None {
        ui.painter().add(egui::Shape::convex_polygon(
            widgets::top_cap(rect, f32::from(skin.inner_radius()), STRIPE),
            skin.env.base(),
            Stroke::NONE,
        ));
    }
    let top = rect.top() + STRIPE + 16.0;
    let left = rect.left() + 20.0;
    // Close at the right, the tabs 12 before it, on the block's centre.
    // The tabs are made first, so the keyboard reaches them before Close.
    let controls = Rect::from_min_max(pos2(left, top), pos2(rect.right() - 20.0, top + block));
    let close_place = Rect::from_center_size(
        pos2(controls.right() - 15.0, controls.center().y),
        vec2(30.0, 30.0),
    );
    let right_to_left = egui::Layout::right_to_left(egui::Align::Center);
    let mut tabs_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(controls.with_max_x(close_place.left() - 12.0))
            .layout(right_to_left),
    );
    let parameters = gettext(skin.locale, "Parameters");
    let url = gettext(skin.locale, "URL");
    let tabs = [
        (false, Choice::plain(&parameters)),
        (true, Choice::plain(&url)),
    ];
    let clicked = choose(
        &mut tabs_ui,
        "input",
        None,
        &tabs,
        form.url_mode,
        Group::Track(26.0),
        skin,
    );
    if let Some(url) = clicked {
        show_url(ui.ctx(), form, url);
    }
    let tabs_left = tabs_ui.min_rect().left();
    let mut close_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(close_place)
            .layout(right_to_left),
    );
    let close = gettext(skin.locale, "Close");
    if widgets::icon_button_sized(
        &mut close_ui,
        Icon::X,
        &close,
        vec2(30.0, 30.0),
        14.0,
        look,
        palette,
    )
    .clicked()
    {
        actions.push(Action::CloseDialog);
    }
    // The title, and under it the connection, which ends before the tabs.
    heading.paint(ui.painter(), pos2(left, top));
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(left, top), heading.size()),
        heading.galley.text(),
    );
    let center = top + heading.height() + 2.0 + line / 2.0;
    let room = tabs_left - 12.0 - left;
    connection_line(ui, (left, center), &connection, role, room, skin);
}

/// macOS: a label over what `add` draws, 5 apart, naming it.
fn labelled(
    ui: &mut Ui,
    text: &str,
    role: TextRole,
    skin: &Skin,
    add: impl FnOnce(&mut Ui) -> Response,
) -> Response {
    let label = widgets::label(ui, role, text, skin.palette.secondary, skin.look);
    ui.add_space(5.0);
    add(ui).labelled_by(label.id)
}

/// macOS: the "Keychain" check box beside a secret, on the field's line.
/// Screen readers hear `name`, which says whose secret it keeps.
/// `below_label` puts it beside a field that has its label above it.
fn keyring_box(
    ui: &mut Ui,
    mode: &mut PasswordMode,
    name: &'static str,
    below_label: bool,
    skin: &Skin,
) {
    let Skin { look, palette, .. } = *skin;
    if below_label {
        let line = TextRole::Secondary.row_height(ui.ctx(), look.faces);
        ui.add_space(line + 5.0);
    }
    ui.allocate_ui_with_layout(
        vec2(ui.available_width(), skin.field_height()),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            let mut keep = keeps(*mode);
            let text = gettext(skin.locale, skin.keyring());
            let hover = gettext(
                skin.locale,
                "Saved for the next connect. Off, it is asked for every time.",
            );
            let response =
                widgets::checkbox(ui, &mut keep, &text, look, palette).on_hover_text(hover);
            named(&response, name, skin);
            if response.changed() {
                set_keeps(mode, keep);
            }
        },
    );
}

/// macOS: a list of choices drawn as the designs draw one, a field with a
/// mark at its right: the look's own up and down chevrons, or one pointing
/// down. It relies on [`super::style_controls`] having set the fields'
/// fill, border and height on `ui`: it takes them as they are.
fn select<R>(
    ui: &mut Ui,
    combo: egui::ComboBox,
    skin: &Skin,
    contents: impl FnOnce(&mut Ui) -> R,
) -> egui::InnerResponse<Option<R>> {
    let icon = if skin.look.raised_popups {
        Icon::ChevronsUpDown
    } else {
        Icon::ChevronDown
    };
    ui.scope(|ui| {
        // The fields' fill while it rests. Under the pointer, and with
        // the keyboard on it, it keeps the look's own, which its menu's
        // rows share.
        let fill = ui.visuals().extreme_bg_color;
        let states = &mut ui.visuals_mut().widgets;
        states.inactive.weak_bg_fill = fill;
        states.open.weak_bg_fill = fill;
        // Its text starts where a field's does.
        ui.spacing_mut().button_padding.x = f32::from(skin.text_inset());
        combo
            .icon(move |ui, rect, visuals, _open| {
                let place = Rect::from_center_size(rect.center(), vec2(14.0, 14.0));
                icon.image(visuals.fg_stroke.color, 14.0)
                    .paint_at(ui, place);
            })
            .show_ui(ui, contents)
    })
    .inner
}

/// macOS: a group of fields in a hairline box, its heading set into the
/// top edge.
fn fieldset(ui: &mut Ui, legend: &str, skin: &Skin, add: impl FnOnce(&mut Ui)) {
    let Skin { look, palette, .. } = *skin;
    let legend = Text::one(look, TextRole::Legend, legend, palette.secondary).layout(ui.ctx());
    let half = legend.height() / 2.0;
    ui.add_space(half);
    let border = ui.painter().add(egui::Shape::Noop);
    let rect = egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 15,
            right: 15,
            // The heading's lower half, then 15 as at the sides: the
            // group's 14 inside its edge.
            top: (half + 15.0).round() as i8,
            bottom: 13,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        })
        .response
        .rect;
    ui.painter().set(
        border,
        egui::epaint::RectShape::stroke(
            rect,
            CornerRadius::same(10),
            Stroke::new(widgets::hairline(ui), skin.rule()),
            StrokeKind::Inside,
        ),
    );
    // The heading breaks the edge: 6 of the dialog's colour at each side.
    let gap = Rect::from_min_size(
        pos2(rect.left() + 15.0, rect.top() - half),
        vec2(legend.width() + 12.0, legend.height()),
    );
    ui.painter().rect_filled(gap, CornerRadius::ZERO, skin.fill);
    legend.paint(ui.painter(), pos2(gap.left() + 6.0, gap.top()));
    widgets::announce(ui, gap, legend.galley.text());
}

/// macOS: the dialog's body, between the header and the footer.
pub(super) fn body(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 20,
            right: 20,
            top: 4,
            bottom: 18,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            if form.url_mode {
                url_pane(ui, form, skin, actions);
            } else {
                identity(ui, form, skin, focus_name, actions);
                ui.add_space(18.0);
                match form.driver {
                    Driver::Sqlite => {
                        fieldset(ui, &skin.say("Database"), skin, |ui| {
                            file_field(ui, form, skin, actions);
                        });
                    }
                    Driver::Postgres | Driver::MySql => {
                        fieldset(ui, &skin.say("Server"), skin, |ui| {
                            server(ui, form, skin);
                        });
                        ui.add_space(18.0);
                        fieldset(ui, &skin.say("Security"), skin, |ui| {
                            security(ui, form, skin, actions);
                        });
                    }
                }
                ui.add_space(18.0);
                safety(ui, form, skin);
            }
        });
}

/// The name beside the kind of database, then the environment.
fn identity(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    skin: &Skin,
    focus_name: bool,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    row(
        ui,
        12.0,
        &[Column::Fr(1.4), Column::Fr(1.0)],
        |ui, column| {
            if column == 0 {
                let name = labelled(ui, &skin.say("Name"), TextRole::FormLabel, skin, |ui| {
                    ui.add(input(ui, &mut form.name, TextRole::UiBody, 34.0, skin))
                });
                if focus_name {
                    name.request_focus();
                }
            } else {
                let heading = skin.say("Type");
                widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
                ui.add_space(5.0);
                driver_choice(ui, form, &heading, skin, actions);
            }
        },
    );
    ui.add_space(18.0);
    let heading = skin.say("Environment");
    widgets::label(ui, TextRole::FormLabel, &heading, palette.secondary, look);
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 12.0;
        environment_choice(ui, form, &heading, skin);
        widgets::label(
            ui,
            TextRole::Secondary,
            &skin.say("Sets the connection color"),
            palette.dim,
            look,
        );
    });
}

/// macOS: where the server is and who logs in.
fn server(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
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
    let same = skin.say("Same as the user");
    labelled(ui, &skin.say("Database"), small, skin, |ui| {
        let hint = hint(ui, &same, role, skin);
        ui.add(input(ui, &mut form.database, role, height, skin).hint_text(hint))
    });
    ui.add_space(10.0);
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("User"), small, skin, |ui| {
                ui.add(input(ui, &mut form.user, role, height, skin))
            });
        }
    });
    ui.add_space(10.0);
    let saved = if form.password_is_saved() {
        skin.say(skin.saved_hint())
    } else {
        String::new()
    };
    row(ui, 12.0, &columns, |ui, column| {
        if column == 0 {
            labelled(ui, &skin.say("Password"), small, skin, |ui| {
                let hint = hint(ui, &saved, TextRole::UiBody, skin);
                ui.add(
                    input(ui, &mut form.password, TextRole::UiBody, height, skin)
                        .password(true)
                        .hint_text(hint),
                )
            });
        } else {
            keyring_box(ui, &mut form.password_mode, skin.keyring(), true, skin);
        }
    });
}

/// macOS: TLS, then the SSH tunnel.
fn security(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let small = TextRole::Secondary;
    row(
        ui,
        12.0,
        &[Column::Fr(1.0), Column::Fr(1.0)],
        |ui, column| {
            if column == 0 {
                let width = ui.available_width();
                labelled(ui, &skin.say("SSL mode"), small, skin, |ui| {
                    select(
                        ui,
                        egui::ComboBox::from_id_salt("tls-mode")
                            .width(width)
                            .selected_text(tls_label(form.tls)),
                        skin,
                        |ui| {
                            for (mode, label) in TLS_MODES {
                                ui.selectable_value(&mut form.tls, mode, label);
                            }
                        },
                    )
                    .response
                });
            } else {
                let label = widgets::label(
                    ui,
                    small,
                    &skin.say("CA certificate"),
                    palette.secondary,
                    look,
                );
                ui.add_space(5.0);
                ca_field(ui, form, label.id, skin, actions);
            }
        },
    );
    if form.password_can_be_intercepted() {
        ui.add_space(10.0);
        intercept_warning(ui, skin);
    }
    ui.add_space(12.0);
    // A line of text, not a row as tall as a field: the tunnel's fields
    // follow 10 below it.
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = widgets::body(look).row_height(ui.ctx(), look.faces);
        widgets::checkbox(
            ui,
            &mut form.ssh,
            &skin.say("Connect through SSH tunnel"),
            look,
            palette,
        );
    });
    if !form.ssh {
        return;
    }
    ui.add_space(10.0);
    // The tunnel's fields sit under the box's label, 22 in.
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 22,
            ..egui::Margin::ZERO
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ssh_fields(ui, form, skin, actions);
        });
}

/// macOS: the tunnel's host, port, user and login in one line, then what
/// the login needs. The host is named by its placeholder. The port, the
/// user and the key file show there what an empty field means: the typed
/// host's values from ~/.ssh/config, else 22 and the login name. Under the
/// line, what the config resolves the host to.
fn ssh_fields(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let role = TextRole::MonoSecondary;
    let height = skin.field_height();
    let hints = form.ssh_hints();
    let port = ssh_port_hint(&hints);
    // The field's name, where no user is known to show in its place.
    let user = Some(ssh_user_hint(&hints, skin))
        .filter(|user| !user.is_empty())
        .unwrap_or_else(|| skin.say("User"));
    let columns = [
        Column::Fr(1.2),
        Column::Px(64.0),
        Column::Fr(0.8),
        Column::Fr(1.0),
    ];
    row(ui, 10.0, &columns, |ui, column| match column {
        0 => {
            let field = ssh_host_field(ui, form, role, &skin.say("SSH host"), skin, actions);
            named(&field, "SSH host", skin);
        }
        1 => {
            let hint = hint(ui, &port, role, skin);
            let field = ui.add(input(ui, &mut form.ssh_port, role, height, skin).hint_text(hint));
            named(&field, "SSH port", skin);
        }
        2 => {
            let hint = hint(ui, &user, role, skin);
            let field = ui.add(input(ui, &mut form.ssh_user, role, height, skin).hint_text(hint));
            named(&field, "SSH user", skin);
        }
        _ => {
            let width = ui.available_width();
            let name = gettext(skin.locale, "Authentication");
            let value = gettext(skin.locale, form.ssh_auth.label());
            let response = select(
                ui,
                egui::ComboBox::from_id_salt("ssh-auth")
                    .width(width)
                    .selected_text(&*value),
                skin,
                |ui| {
                    for kind in SshAuthKind::ALL {
                        ui.selectable_value(
                            &mut form.ssh_auth,
                            kind,
                            gettext(skin.locale, kind.label()),
                        );
                    }
                },
            )
            .response;
            response.widget_info(|| {
                let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &*name);
                info.current_text_value = Some(value.to_string());
                info
            });
        }
    });
    if let Some((text, color)) = ssh_host_note(&hints, skin) {
        ui.add_space(6.0);
        note(ui, &text, color, skin);
    }
    if form.ssh_auth == SshAuthKind::KeyFile {
        ui.add_space(10.0);
        let key = ssh_key_hint(&hints, skin).unwrap_or_else(|| skin.say("Key file"));
        let (field, clicked) =
            field_with_button(ui, &skin.say("Choose…"), "Choose a key file", skin, |ui| {
                let hint = hint(ui, &key, role, skin);
                ui.add(input(ui, &mut form.ssh_key_file, role, height, skin).hint_text(hint))
            });
        named(&field, "Key file", skin);
        if clicked {
            actions.push(Action::PickKeyFile);
        }
    }
    if form.ssh_auth != SshAuthKind::Agent {
        ui.add_space(10.0);
        let name = ssh_secret_name(form.ssh_auth);
        let placeholder = if form.ssh_secret_is_saved() {
            format!("{} · {}", skin.say(name), skin.say(skin.saved_hint()))
        } else {
            skin.say(name)
        };
        row(
            ui,
            12.0,
            &[Column::Fr(1.0), Column::Px(96.0)],
            |ui, column| {
                if column == 0 {
                    let hint = hint(ui, &placeholder, TextRole::UiBody, skin);
                    let field = ui.add(
                        input(ui, &mut form.ssh_secret, TextRole::UiBody, height, skin)
                            .password(true)
                            .hint_text(hint),
                    );
                    named(&field, name, skin);
                } else {
                    keyring_box(
                        ui,
                        &mut form.ssh_secret_mode,
                        skin.ssh_keyring(),
                        false,
                        skin,
                    );
                }
            },
        );
    }
}

/// A 14 pt check box in the environment's colour.
fn env_check(ui: &mut Ui, on: bool, name: &str, skin: &Skin) -> Response {
    // Three below the line's top, as the design sets the box.
    let (rect, response) = ui.allocate_exact_size(vec2(14.0, 17.0), Sense::click());
    response.widget_info(|| WidgetInfo::selected(WidgetType::Checkbox, true, on, name));
    let mark = Rect::from_min_size(rect.min + vec2(0.0, 3.0), vec2(14.0, 14.0));
    let radius = skin.look.radius.min(4);
    // The ring goes round the box, not round the line it sits in.
    crate::ui::focus::hint(
        ui,
        &response,
        mark,
        crate::ui::focus::Ring::Outer { radius },
    );
    let radius = CornerRadius::same(radius);
    let painter = ui.painter();
    if on {
        let fill = skin.env.base();
        // The tick is cut out of the box in the dialog's own colour, unless
        // its text colour reads better there: the environments' colours
        // are the same on a dark sheet as on a light one.
        let tick = if theme::contrast(skin.fill, fill) >= theme::contrast(skin.palette.text, fill) {
            skin.fill
        } else {
            skin.palette.text
        };
        painter.rect_filled(mark, radius, fill);
        paint_check(painter, mark, tick);
    } else {
        painter.rect_stroke(
            mark,
            radius,
            Stroke::new(1.0, skin.palette.secondary),
            StrokeKind::Inside,
        );
    }
    response
}

/// What the read-only promise says under its title.
const READ_ONLY_NOTE: &str =
    "Blocks every write from this app. On by default for production; turn off to edit.";

/// macOS: the read-only promise, on the environment's tint. The box shows
/// the environment's default until it is clicked, and is the user's
/// choice from then on.
fn safety(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let title = skin.say("Open read-only");
    egui::Frame::new()
        .fill(skin.env.bar_bg())
        .corner_radius(CornerRadius::same(10))
        .inner_margin(egui::Margin::symmetric(14, 12))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 10.0;
                let on = form.read_only();
                let mut toggled = env_check(ui, on, &title, skin).clicked();
                ui.vertical(|ui| {
                    // The title toggles the box under the pointer, as a
                    // check box's own label does. It takes clicks alone:
                    // the box stays the one check box and the one Tab stop.
                    let label = Text::one(look, TextRole::UiBodyStrong, &title, palette.text)
                        .layout(ui.ctx())
                        .label_sense(ui, Sense::CLICK);
                    toggled |= label.clicked();
                    ui.add_space(2.0);
                    // The secondary colour, warmed by the environment's
                    // text colour: 0.4 is the share at which production's
                    // red on the light palette comes out in the design's
                    // tone.
                    let tone = theme::mix(palette.secondary, skin.env_ink(&skin.env), 0.4);
                    widgets::label(
                        ui,
                        TextRole::Secondary,
                        &skin.say(READ_ONLY_NOTE),
                        tone,
                        look,
                    );
                });
                if toggled {
                    form.read_only = Some(!on);
                }
            });
        });
}

/// macOS: the URL tab, one field that fills the parameters.
fn url_pane(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let label = widgets::label(
        ui,
        TextRole::FormLabel,
        &skin.say("Connection URL"),
        palette.secondary,
        look,
    );
    ui.add_space(5.0);
    let (field, clicked) = field_with_button(ui, &skin.say("Fill"), "Fill", skin, |ui| {
        url_field(ui, form, skin)
    });
    let field = field.labelled_by(label.id);
    let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
    if clicked || entered {
        actions.push(Action::ApplyUrl);
    }
    ui.add_space(6.0);
    widgets::label(
        ui,
        TextRole::Secondary,
        &skin.say("Fills the parameters. A password in the URL moves to the password field."),
        palette.dim,
        look,
    );
}

/// macOS: Delete at the left; the Test's status, Test, Cancel and the two
/// ways to save at the right. Saving is the main thing an edit does, and
/// connecting the main thing a new connection does.
pub(super) fn footer(ui: &mut Ui, form: &ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 61.0), Sense::hover());
    let radius = skin.inner_radius();
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius,
            se: radius,
        },
        skin.footer(),
    );
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, skin.rule());
    let y = rect.top() + 1.0 + 30.0;
    let editing = form.editing.is_some();
    let save = gettext(skin.locale, "Save");
    let connect = gettext(skin.locale, "Save & Connect");
    let test = gettext(skin.locale, "Test");
    let cancel = gettext(skin.locale, "Cancel");
    let delete = gettext(skin.locale, "Delete");
    let button = |text, primary: bool| {
        let spec = ButtonSpec::new(text);
        if primary {
            spec.primary().padding(16.0)
        } else {
            spec
        }
    };
    // Right to left, 10 apart: the primary, the other way to save, Cancel
    // and Test.
    let (first, second) = if editing {
        (&save, &connect)
    } else {
        (&connect, &save)
    };
    let specs = [
        (&test, false, Action::TestConnection),
        (&cancel, false, Action::CloseDialog),
        (second, false, Action::SaveConnection { connect: editing }),
        (first, true, Action::SaveConnection { connect: !editing }),
    ];
    let widths: Vec<f32> = specs
        .iter()
        .map(|(text, primary, _)| button(text, *primary).width(ui, look))
        .collect();
    let total = widths.iter().sum::<f32>() + 10.0 * (specs.len() - 1) as f32;
    let mut left = rect.right() - 20.0 - total;
    let buttons = left;
    // Delete first: the keyboard reaches the buttons left to right.
    if let Some(id) = &form.editing {
        let laid = Text::one(look, widgets::body(look), &delete, palette.danger).layout(ui.ctx());
        let place =
            Rect::from_min_size(pos2(rect.left() + 20.0, y - 16.0), vec2(laid.width(), 32.0));
        let name = format!("{} {}", delete, form.name.trim());
        let response = ui.interact(place, ui.id().with("delete"), Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &name));
        laid.paint_left(ui.painter(), place.left(), y);
        if response.hovered() || response.has_focus() {
            let under = y + laid.height() / 2.0;
            ui.painter()
                .hline(place.x_range(), under, Stroke::new(1.0, palette.danger));
        }
        if response.clicked() {
            actions.push(Action::DeleteConnection(id.clone()));
            actions.push(Action::CloseDialog);
        }
    }
    paint_status(ui, form, StatusAt::Right(buttons - 10.0), y, skin);
    for ((text, primary, action), width) in specs.into_iter().zip(widths) {
        let place = Rect::from_min_size(pos2(left, y - 16.0), vec2(width, 32.0));
        left += width + 10.0;
        if button(text, primary)
            .show_at(ui, place, look, palette)
            .clicked()
        {
            actions.push(action);
        }
    }
}
