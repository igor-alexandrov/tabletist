//! The connection dialog: new or edit, any driver, with an optional SSH
//! tunnel. macOS draws a sheet of labelled fields in groups under the
//! environment's colour; the terminal look a two-column form with its keys
//! in the footer.

mod choice;
mod sheet;
mod terminal;

use egui::{
    Color32, CornerRadius, Rect, Response, Sense, Stroke, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::ssh_config::Proxy;
use tabletist_db::{Driver, TlsMode};

use crate::app::App;
use crate::connections::PasswordMode;
use crate::env::{EnvColors, Environment, Platform, env_colors};
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnectionForm, Dialog, SshAuthKind, SshHints, TestState};
use crate::theme::{self, Faces, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

use sheet::{body, footer, header};
use terminal::{terminal_body, terminal_footer, terminal_header, terminal_row};

/// The TLS modes by their libpq names, as the picker says them.
const TLS_MODES: [(TlsMode, &str); 5] = [
    (TlsMode::Disable, "disable"),
    (TlsMode::Prefer, "prefer"),
    (TlsMode::Require, "require"),
    (TlsMode::VerifyCa, "verify-ca"),
    (TlsMode::VerifyFull, "verify-full"),
];

const INTERCEPT_WARNING: &str = "The password can be intercepted on the network. \
                                 To prevent it, verify the certificate and host, \
                                 or use an SSH tunnel.";

/// Where the dialog's top edge settled once it opened (see [`Placement`]).
fn placement_id() -> egui::Id {
    egui::Id::new("connection-dialog-placement")
}

/// Set for one frame after the URL field appears: it takes the keyboard.
fn focus_url_id() -> egui::Id {
    egui::Id::new("connection-dialog-focus-url")
}

/// The dialog's place. egui centres a modal on its size, so a dialog that
/// grows (PostgreSQL's fields, the SSH section, the TLS warning) would move
/// under the pointer. It opens where a server's form would be centred, so
/// the short SQLite form and that one share a top edge; then the edge stays
/// put and the dialog grows downwards.
#[derive(Clone, Copy, Default)]
struct Placement {
    /// The top edge, once a frame has measured the form. Until then the
    /// dialog is laid out but not painted, so it is first seen where it
    /// stays.
    top: Option<f32>,
}

/// The height the dialog keeps room for when it opens: a server's form
/// without a tunnel or a message, rounded up. The sheet's is the macOS
/// look's, the taller of its two.
fn reserve(look: &Look) -> f32 {
    if look.terminal { 615.0 } else { 815.0 }
}

/// Where the top edge of a dialog `height` tall goes on a screen
/// `screen` tall.
fn top_edge(screen: f32, height: f32) -> f32 {
    ((screen - height) / 2.0).max(24.0)
}

/// How the dialog draws: the look, the palette, and the environment that
/// colours it.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
    environment: Environment,
    env: EnvColors,
    /// The dialog's own background.
    fill: Color32,
}

impl<'a> Skin<'a> {
    fn new(look: &'a Look, palette: &'a Palette, locale: Locale, environment: Environment) -> Self {
        Self {
            look,
            palette,
            locale,
            environment,
            env: env_colors(environment, Platform::of(look), palette),
            fill: if look.terminal {
                palette.window
            } else {
                palette.overlay
            },
        }
    }

    /// `text` translated, in the look's case.
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }

    /// `environment`'s colours in this look and palette.
    fn colors(&self, environment: Environment) -> EnvColors {
        env_colors(environment, Platform::of(self.look), self.palette)
    }

    /// The dialog's border: the terminal look's 2 pt edge in the
    /// environment's colour, a hairline where a dark dialog would merge
    /// with a dark window, none on light.
    fn stroke(&self) -> Stroke {
        if self.look.terminal {
            Stroke::new(2.0, self.env.base())
        } else if self.palette.dark {
            Stroke::new(1.0, self.palette.outline)
        } else {
            Stroke::NONE
        }
    }

    /// The dialog's corners, and those of what fills it edge to edge.
    fn radius(&self) -> u8 {
        if self.look.terminal {
            6
        } else {
            self.look.dialog_radius + 2
        }
    }

    fn inner_radius(&self) -> u8 {
        self.radius().saturating_sub(self.stroke().width as u8)
    }

    /// Rules between the dialog's parts and round its groups.
    fn rule(&self) -> Color32 {
        if self.look.terminal {
            self.palette.outline
        } else {
            self.palette.surface_hover
        }
    }

    /// The footer's fill: the panel tone, or the window's where the dialog
    /// itself has the panel's.
    fn footer(&self) -> Color32 {
        if self.fill == self.palette.panel {
            self.palette.window
        } else {
            self.palette.panel
        }
    }

    /// An environment's colour as text on the dialog. The sheet takes the
    /// badge's text colour, which is made to read (4.5 to 1) on the badge's
    /// own fill and on no other. The raised segment and the read-only note
    /// are other fills, close to the badge's in tone (the sheet's colour,
    /// raised or tinted a little): it reads there with somewhat less
    /// contrast on a dark sheet, and is kept so that the name has the
    /// colour its badge has everywhere else. The terminal look takes the
    /// environment's own, the theme's, moved towards the text colour until
    /// it reads on the dialog: a light theme's yellow would not.
    fn env_ink(&self, colors: &EnvColors) -> Color32 {
        if !self.look.terminal {
            return colors.badge_fg();
        }
        let base = colors.base();
        let mut ink = base;
        let mut amount = 0.0;
        while theme::contrast(ink, self.fill) < 4.5 && amount < 1.0 {
            amount += 0.1;
            ink = theme::mix(base, self.palette.text, amount);
        }
        ink
    }

    /// Where saved passwords go, as this platform calls it.
    fn keyring(&self) -> &'static str {
        if self.look.faces == Faces::Plex {
            "Keychain"
        } else {
            "Keyring"
        }
    }

    /// The name of the box that keeps the SSH secret: the database
    /// password's has the same text beside it.
    fn ssh_keyring(&self) -> &'static str {
        if self.look.faces == Faces::Plex {
            "Keychain for the SSH secret"
        } else {
            "Keyring for the SSH secret"
        }
    }

    fn saved_hint(&self) -> &'static str {
        if self.look.faces == Faces::Plex {
            "Saved in the keychain"
        } else {
            "Saved in the keyring"
        }
    }

    /// The height of a field: the terminal's 30, macOS's 32 (34 for the
    /// name).
    fn field_height(&self) -> f32 {
        if self.look.terminal { 30.0 } else { 32.0 }
    }

    /// Where a field's text starts from its edge: past the border and the
    /// designs' padding (8 in the terminal look, 10 on macOS). A list of
    /// choices starts its text there too.
    fn text_inset(&self) -> i8 {
        if self.look.terminal { 9 } else { 11 }
    }

    /// Monospace for what goes to the server: hosts, names, paths.
    fn value_role(&self) -> TextRole {
        TextRole::pick(self.look, TextRole::GridCell, TextRole::OBody)
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(Dialog::Connection(form)) = &mut app.dialog else {
        ctx.data_mut(|data| data.remove::<Placement>(placement_id()));
        return;
    };
    let placement: Placement = ctx
        .data(|data| data.get_temp(placement_id()))
        .unwrap_or_default();
    // Until the form's height is known the dialog is measured, not shown.
    let measuring = placement.top.is_none();
    // A frame that is not shown cannot keep the keyboard, so Name is given
    // it again on each one, and a last time on the first frame shown.
    let focus_name = form.focus_name;
    form.focus_name &= measuring;
    let mut actions = Vec::new();
    // The dialog's own keys are taken before anything draws: a focused
    // button would read Mod+Enter as a press of itself. An open list keeps
    // them, as it keeps Escape.
    if !egui::Popup::is_any_open(ctx) {
        // A Test that is running is not started again by its key.
        let testing = matches!(form.test, TestState::Running(_));
        ctx.input_mut(|input| {
            use egui::Key;
            if take_mod_key(input, Key::Enter) {
                actions.push(Action::SaveConnection { connect: true });
            }
            if take_mod_key(input, Key::S) {
                actions.push(Action::SaveConnection { connect: false });
            }
            if take_mod_key(input, Key::T) && !testing {
                actions.push(Action::TestConnection);
            }
        });
    }
    let skin = Skin::new(&look, &palette, locale, form.environment());
    let screen = ctx.content_rect();
    let width: f32 = if look.terminal { 780.0 } else { 700.0 };
    let width = width.min(screen.width() - 48.0).max(320.0);
    // A modal: nothing behind it can be clicked while it is open.
    let id = egui::Id::new("connection-dialog");
    let top = placement
        .top
        .unwrap_or_else(|| top_edge(screen.height(), reserve(&look)));
    let frame = if measuring {
        unseen(frame(&skin))
    } else {
        frame(&skin)
    };
    // The dialog is whole on the first frame it paints. egui fades an area
    // in by drawing everything in it see-through for a moment, and through
    // a sheet this large the window shows in every field.
    let area = egui::Modal::default_area(id)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, top))
        .fade_in(false);
    let mut modal = widgets::modal(id, &look, &palette).frame(frame).area(area);
    if measuring {
        // Nor is the window veiled before there is a dialog over it.
        modal = modal.backdrop_color(Color32::TRANSPARENT);
    }
    let modal = modal.show(ctx, |ui| {
        if measuring {
            ui.set_invisible();
        }
        ui.set_width(width);
        // An area offers its content the height it had on the last frame,
        // so a form that grew would reach its new height a step per frame.
        ui.set_max_height(screen.height());
        style_controls(ui, &skin);
        if look.terminal {
            terminal_header(ui, form, &skin);
        } else {
            header(ui, form, &skin, &mut actions);
        }
        // The fields scroll when the window is short, so the messages and
        // the footer stay on screen.
        let chrome = if look.terminal { 86.0 } else { 140.0 };
        let space = screen.bottom() - top - chrome - 24.0;
        let messages = Messages::new(ui.ctx(), form, width, (space / 3.0).max(60.0), &skin);
        let room = (space - messages.height(&skin)).max(120.0);
        let fields = egui::ScrollArea::vertical()
            .max_height(room)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                if look.terminal {
                    terminal_body(ui, form, &skin, focus_name, &mut actions);
                } else {
                    body(ui, form, &skin, focus_name, &mut actions);
                }
            });
        messages.show(ui, &skin, &mut actions);
        if look.terminal {
            terminal_footer(ui, form, &skin, &mut actions);
        } else {
            footer(ui, form, &skin, &mut actions);
        }
        // What the form needs beyond what it got, unless this frame only
        // sized the area: egui squeezes that one, so it measures nothing.
        let hidden = (fields.content_size.y - fields.inner_rect.height()).max(0.0);
        (!ui.is_sizing_pass()).then_some(hidden)
    });
    // A form taller than the room kept for it (a tunnel's fields) is
    // centred on its own height; the edge stays put from then on.
    if measuring {
        if let Some(hidden) = modal.inner {
            let height = modal.response.rect.height() + hidden;
            let top = top_edge(screen.height(), height.max(reserve(&look)));
            ctx.data_mut(|data| data.insert_temp(placement_id(), Placement { top: Some(top) }));
        }
        // Shown from the next frame on.
        ctx.request_repaint();
    }
    if modal.is_top_modal && !modal.any_popup_open {
        let typing = ctx.text_edit_focused();
        let toggle_url = ctx.input_mut(|input| {
            use egui::{Key, Modifiers};
            // Escape closes the dialog, unless it is closing an open list
            // first.
            if input.consume_key(Modifiers::NONE, Key::Escape) {
                actions.push(Action::CloseDialog);
            }
            // The terminal look's `u`, when it is not a letter being typed.
            look.terminal && !typing && input.consume_key(Modifiers::NONE, Key::U)
        });
        // After the input is released: showing the field writes to egui's
        // memory, which sits behind the same lock.
        if toggle_url {
            show_url(ctx, form, !form.url_mode);
        }
    }
    app.actions.extend(actions);
}

/// Takes every press of Mod+`key` out of the input. True when one of them
/// was the key going down, not the keyboard repeating it while it is held.
fn take_mod_key(input: &mut egui::InputState, key: egui::Key) -> bool {
    let mut pressed = false;
    input.events.retain(|event| match event {
        egui::Event::Key {
            key: found,
            pressed: true,
            repeat,
            modifiers,
            ..
        } if *found == key && modifiers.matches_logically(egui::Modifiers::COMMAND) => {
            pressed |= !*repeat;
            false
        }
        _ => true,
    });
    pressed
}

/// Shows or hides the URL field; shown, it takes the keyboard.
fn show_url(ctx: &egui::Context, form: &mut ConnectionForm, shown: bool) {
    form.url_mode = shown;
    if shown {
        ctx.data_mut(|data| data.insert_temp(focus_url_id(), true));
    }
}

/// `frame` taking the same room and painting nothing, for the frames that
/// only measure the dialog.
fn unseen(frame: egui::Frame) -> egui::Frame {
    let stroke = Stroke::new(frame.stroke.width, Color32::TRANSPARENT);
    frame
        .fill(Color32::TRANSPARENT)
        .stroke(stroke)
        .shadow(egui::epaint::Shadow::NONE)
}

/// The dialog's frame: its parts fill it edge to edge, so it has no margin
/// of its own.
fn frame(skin: &Skin) -> egui::Frame {
    let frame = egui::Frame::new()
        .fill(skin.fill)
        .corner_radius(CornerRadius::same(skin.radius()))
        .stroke(skin.stroke());
    if skin.look.terminal {
        frame
    } else {
        frame.shadow(egui::epaint::Shadow {
            offset: [0, 24],
            blur: 64,
            spread: 0,
            color: skin.palette.shadow,
        })
    }
}

/// Fields as the designs draw them: boxes with a border on the field
/// colour, and no spacing but what the layout adds. A dropdown takes the
/// fill from here.
fn style_controls(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let (fill, border, radius, width) = if look.terminal {
        (palette.panel, palette.outline, 3, 1.0)
    } else {
        let fill = if palette.dark {
            palette.surface
        } else {
            palette.window
        };
        (
            fill,
            palette.border,
            look.radius.saturating_sub(1),
            widgets::hairline(ui),
        )
    };
    let visuals = ui.visuals_mut();
    visuals.extreme_bg_color = fill;
    for state in [
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        state.corner_radius = CornerRadius::same(radius);
        state.bg_stroke = Stroke::new(width, border);
    }
    ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
    ui.spacing_mut().interact_size.y = skin.field_height();
}

fn title(form: &ConnectionForm) -> &'static str {
    if form.editing.is_some() {
        "Edit connection"
    } else {
        "New connection"
    }
}

/// What the header says under (or after) the title: the connection's name
/// and environment, or its kind until it has either. The name is as typed;
/// the rest is in the look's case.
fn subtitle(form: &ConnectionForm, skin: &Skin) -> String {
    let name = form.name.trim();
    let mut parts = Vec::new();
    if !name.is_empty() {
        parts.push(name.to_owned());
    }
    if skin.environment != Environment::None {
        // As its badge says it on the sheet, in the look's case.
        parts.push(skin.say(skin.environment.label(Platform::Native)));
    }
    if parts.is_empty() {
        parts.push(skin.look.label(form.driver.label()));
    }
    parts.join(" · ")
}

/// The header's line about the connection, from `left` on the line centred
/// at `center`. A long name (a URL names a connection `user@host:port/db`)
/// is cut short with "…" to `room`, and shown whole under the pointer.
fn connection_line(
    ui: &mut Ui,
    (left, center): (f32, f32),
    text: &str,
    role: TextRole,
    room: f32,
    skin: &Skin,
) {
    let Skin { look, palette, .. } = *skin;
    let shown = crate::ui::grid::ellipsize(text, room.max(0.0), false, |text| {
        role.width(ui.ctx(), look.faces, text)
    });
    let laid = Text::one(look, role, &shown, palette.dim).layout(ui.ctx());
    laid.paint_left(ui.painter(), left, center);
    let place = Rect::from_min_size(pos2(left, center - laid.height() / 2.0), laid.size());
    let response = ui.interact(place, ui.id().with("connection-line"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &*shown));
    if shown != text {
        response.on_hover_text(text);
    }
}

/// A column of a [`row`]: a share of the free width, or a fixed width.
#[derive(Clone, Copy)]
enum Column {
    Fr(f32),
    Px(f32),
}

/// The widths `columns` take in `total`, `gap` apart.
fn column_widths(total: f32, gap: f32, columns: &[Column]) -> Vec<f32> {
    let fixed: f32 = columns
        .iter()
        .map(|column| match column {
            Column::Px(width) => *width,
            Column::Fr(_) => 0.0,
        })
        .sum();
    let shares: f32 = columns
        .iter()
        .map(|column| match column {
            Column::Fr(share) => *share,
            Column::Px(_) => 0.0,
        })
        .sum();
    let free = (total - fixed - gap * columns.len().saturating_sub(1) as f32).max(0.0);
    columns
        .iter()
        .map(|column| match column {
            Column::Px(width) => *width,
            Column::Fr(share) => (free * share / shares.max(f32::EPSILON)).floor(),
        })
        .collect()
}

/// Cells side by side across the available width, their tops in line.
fn row(ui: &mut Ui, gap: f32, columns: &[Column], mut cell: impl FnMut(&mut Ui, usize)) {
    let widths = column_widths(ui.available_width(), gap, columns);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for (index, width) in widths.into_iter().enumerate() {
            ui.allocate_ui_with_layout(
                vec2(width, 0.0),
                egui::Layout::top_down(egui::Align::Min),
                |ui| {
                    ui.set_width(width);
                    cell(ui, index);
                },
            );
        }
    });
}

/// A field filling its cell, `height` tall.
fn input<'t>(
    ui: &Ui,
    text: &'t mut String,
    role: TextRole,
    height: f32,
    skin: &Skin,
) -> egui::TextEdit<'t> {
    let pad = skin.text_inset();
    widgets::field(ui, text, skin.look, role, height, pad).desired_width(f32::INFINITY)
}

/// The placeholder of the field that fills `ui`, in the field's role. A
/// field grows to hold a placeholder laid out for it, so one longer than
/// the field (a value from ~/.ssh/config in a narrow column) is cut to
/// the field's room with "…". Screen readers still hear all of it.
fn hint(ui: &Ui, text: &str, role: TextRole, skin: &Skin) -> std::sync::Arc<egui::Galley> {
    let room = ui.available_width() - 2.0 * f32::from(skin.text_inset());
    let mut text = Text::one(skin.look, role, text, skin.palette.faint);
    text.job_mut().wrap = egui::text::TextWrapping::truncate_at_width(room.max(0.0));
    text.layout(ui.ctx()).galley
}

/// The button beside a field, with the field's corners: it says `text`
/// and screen readers hear `name`.
fn side_button<'a>(text: &'a str, name: &'a str, skin: &Skin) -> ButtonSpec<'a> {
    ButtonSpec::new(text)
        .label(name)
        .padding(10.0)
        .radius(if skin.look.terminal {
            3
        } else {
            skin.look.radius.saturating_sub(1)
        })
}

/// The space between a field and the button beside it.
fn side_gap(skin: &Skin) -> f32 {
    if skin.look.terminal { 8.0 } else { 6.0 }
}

/// A field and the button that picks a file for it, 6 apart (8 in the
/// terminal look). Returns the field, and whether the button was clicked.
fn field_with_button(
    ui: &mut Ui,
    button: &str,
    name: &'static str,
    skin: &Skin,
    field: impl FnOnce(&mut Ui) -> Response,
) -> (Response, bool) {
    let Skin { look, palette, .. } = *skin;
    let height = skin.field_height();
    let name = gettext(skin.locale, name);
    let spec = || side_button(button, &name, skin);
    let gap = side_gap(skin);
    let width = spec().width(ui, look);
    let mut field = Some(field);
    let mut output = None;
    let mut clicked = false;
    row(
        ui,
        gap,
        &[Column::Fr(1.0), Column::Px(width)],
        |ui, column| {
            if column == 0 {
                if let Some(field) = field.take() {
                    output = Some(field(ui));
                }
            } else {
                clicked = spec().show(ui, height, look, palette).clicked();
            }
        },
    );
    let response = output.unwrap_or_else(|| ui.response());
    (response, clicked)
}

/// Names `response` for screen readers, where no label beside it does.
/// Only the name is set: the node keeps the role and the state egui gave
/// it, so a password field stays one and a check box keeps its mark.
fn named(response: &Response, name: &'static str, skin: &Skin) {
    let name = gettext(skin.locale, name);
    response
        .ctx
        .accesskit_node_builder(response.id, |node| node.set_label(&*name));
}

/// The password's mode as a check box: saved in the keyring, or asked for
/// on every connect.
fn keeps(mode: PasswordMode) -> bool {
    mode != PasswordMode::Ask
}

fn set_keeps(mode: &mut PasswordMode, keep: bool) {
    *mode = if keep {
        PasswordMode::Keyring
    } else {
        PasswordMode::Ask
    };
}

/// SQLite: the file, and the button that picks one.
fn file_field(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin, actions: &mut Vec<Action>) {
    let role = skin.value_role();
    let height = skin.field_height();
    let choose = skin.say("Choose…");
    let mut pick = |ui: &mut Ui, label: egui::Id| {
        let (field, clicked) =
            field_with_button(ui, &choose, "Choose a database file", skin, |ui| {
                ui.add(input(ui, &mut form.sqlite_path, role, height, skin))
            });
        field.labelled_by(label);
        if clicked {
            actions.push(Action::PickSqliteFile);
        }
    };
    if skin.look.terminal {
        terminal_row(ui, &skin.say("File"), skin, |ui, label| {
            pick(ui, label);
        });
    } else {
        let label = widgets::label(
            ui,
            TextRole::Secondary,
            &skin.say("File"),
            skin.palette.secondary,
            skin.look,
        );
        ui.add_space(5.0);
        pick(ui, label.id);
    }
}

/// Whether the TLS mode checks the server against a CA file. `require`
/// uses one too (as verify-ca, like libpq).
fn uses_ca_file(mode: TlsMode) -> bool {
    matches!(
        mode,
        TlsMode::Require | TlsMode::VerifyCa | TlsMode::VerifyFull
    )
}

/// What an empty CA file means in each mode. verify-ca refuses the system
/// roots: they vouch for any public certificate.
fn ca_hint(mode: TlsMode) -> &'static str {
    match mode {
        TlsMode::Require => "None (not checked)",
        TlsMode::VerifyCa => "Required",
        TlsMode::VerifyFull => "System certificates",
        TlsMode::Disable | TlsMode::Prefer => "Not used",
    }
}

/// The CA certificate's field and its button, greyed out in the modes that
/// check nothing.
fn ca_field(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    label: egui::Id,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(skin.look, TextRole::MonoSecondary, TextRole::OBody);
    let height = skin.field_height();
    let placeholder = skin.say(ca_hint(form.tls));
    ui.add_enabled_ui(uses_ca_file(form.tls), |ui| {
        let (field, clicked) = field_with_button(
            ui,
            &skin.say("Choose…"),
            "Choose a CA certificate",
            skin,
            |ui| {
                let hint = hint(ui, &placeholder, role, skin);
                ui.add(input(ui, &mut form.ca_file, role, height, skin).hint_text(hint))
            },
        );
        field.labelled_by(label);
        if clicked {
            actions.push(Action::PickCaFile);
        }
    });
}

/// The warning under the TLS mode when it leaves the password readable.
fn intercept_warning(ui: &mut Ui, skin: &Skin) {
    let text = gettext(skin.locale, INTERCEPT_WARNING);
    note(ui, &text, skin.palette.warning, skin);
}

/// What the SSH secret is, for the chosen login.
fn ssh_secret_name(auth: SshAuthKind) -> &'static str {
    if auth == SshAuthKind::Password {
        "SSH password"
    } else {
        "Passphrase"
    }
}

/// The SSH host's field with `placeholder`. When ~/.ssh/config names hosts,
/// a button after it lists their aliases, each with its HostName; choosing
/// one fills the field.
fn ssh_host_field(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    role: TextRole,
    placeholder: &str,
    skin: &Skin,
    actions: &mut Vec<Action>,
) -> Response {
    let Skin { look, palette, .. } = *skin;
    let height = skin.field_height();
    let hosts = &form.ssh_hosts;
    let host = &mut form.ssh_host;
    let field = |ui: &mut Ui, host: &mut String| {
        let hint = hint(ui, placeholder, role, skin);
        ui.add(input(ui, host, role, height, skin).hint_text(hint))
    };
    if hosts.is_empty() {
        return field(ui, host);
    }
    let name = gettext(skin.locale, "Hosts from ~/.ssh/config");
    let spec = || {
        side_button("", &name, skin)
            .icon(Icon::ChevronDown)
            // A button's width counts the gap after its icon even when no
            // text follows.
            .gap(0.0)
    };
    let width = spec().width(ui, look);
    let mut output = None;
    row(
        ui,
        side_gap(skin),
        &[Column::Fr(1.0), Column::Px(width)],
        |ui, column| {
            if column == 0 {
                output = Some(field(ui, host));
                return;
            }
            let button = spec().show(ui, height, look, palette).on_hover_text(&*name);
            egui::Popup::menu(&button).show(|ui| {
                for host in hosts {
                    ui.horizontal(|ui| {
                        if widgets::button(ui, &host.alias, look).clicked() {
                            actions.push(Action::PickSshHost(host.alias.clone()));
                            // A menu closes by itself on a pointer's click
                            // only: the keyboard and a screen reader pick
                            // too.
                            ui.close();
                        }
                        if let Some(name) = &host.config.host_name {
                            widgets::label(ui, widgets::secondary(look), name, palette.dim, look);
                        }
                    });
                }
            });
        },
    );
    output.unwrap_or_else(|| ui.response())
}

/// What an empty SSH port means: the port ~/.ssh/config gives the typed
/// host, else 22.
fn ssh_port_hint(hints: &SshHints) -> String {
    hints.port.unwrap_or(22).to_string()
}

/// A value ~/.ssh/config gives the typed host, saying where it is from.
fn from_config(value: &str, skin: &Skin) -> String {
    format!("{value} ({})", gettext(skin.locale, "from ~/.ssh/config"))
}

/// What an empty SSH user means: the config's user for the typed host,
/// else the login name. Empty when that is not known.
fn ssh_user_hint(hints: &SshHints, skin: &Skin) -> String {
    match &hints.user {
        Some(user) => from_config(user, skin),
        None => tabletist_db::ssh_config::login().unwrap_or_default(),
    }
}

/// What an empty key file means: the config's key for the typed host, when
/// it names one.
fn ssh_key_hint(hints: &SshHints, skin: &Skin) -> Option<String> {
    hints.key_file.as_ref().map(|path| from_config(path, skin))
}

/// What goes under the SSH host, and its colour: the host name
/// ~/.ssh/config resolves the typed host to, or why the tunnel cannot
/// follow the config there. `None` when there is nothing to say.
fn ssh_host_note(hints: &SshHints, skin: &Skin) -> Option<(String, Color32)> {
    let warning = match &hints.proxy {
        Some(Proxy::Jump(_)) => Some("Uses ProxyJump, which Tabletist does not support yet."),
        Some(Proxy::Command(_)) => Some("Uses ProxyCommand, which Tabletist does not support yet."),
        Some(Proxy::Off) | None => None,
    };
    if let Some(warning) = warning {
        let text = gettext(skin.locale, warning).to_string();
        return Some((text, skin.palette.warning));
    }
    let name = hints.host_name.as_ref()?;
    let text = format!("{name} {}", gettext(skin.locale, "from ~/.ssh/config"));
    Some((text, skin.palette.dim))
}

/// A line of small print about a field, wrapped to the room there is.
fn note(ui: &mut Ui, text: &str, color: Color32, skin: &Skin) {
    Text::one(skin.look, widgets::secondary(skin.look), text, color)
        .wrap(ui.available_width())
        .layout(ui.ctx())
        .label(ui);
}

/// The URL field, with the chosen driver's URL as its placeholder. It
/// takes the keyboard on the frame it appears.
fn url_field(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) -> Response {
    let role = skin.value_role();
    let example = match form.driver {
        Driver::Sqlite => "sqlite:///path/to/file.db",
        Driver::MySql => "mysql://user@host/db",
        Driver::Postgres => "postgres://user@host/db",
    };
    let hint = hint(ui, example, role, skin);
    let field = ui.add(input(ui, &mut form.url, role, skin.field_height(), skin).hint_text(hint));
    let focus = ui
        .ctx()
        .data_mut(|data| data.remove_temp::<bool>(focus_url_id()))
        .unwrap_or(false);
    if focus {
        field.request_focus();
    }
    field
}

/// What went wrong: a field that does not describe a connection, a failed
/// Test, or an SSH host key to trust first. It sits between the fields and
/// the footer, outside what scrolls, so it is on screen wherever the
/// fields are scrolled to. It is laid out before anything is drawn, so the
/// fields get the room it leaves on the same frame.
struct Messages {
    /// Each text in its colour, wrapped to the dialog's width.
    notices: Vec<(crate::typography::Laid, Color32)>,
    /// The texts scroll by themselves past this height, so a long message
    /// cannot push the footer off the screen.
    cap: f32,
    /// An unknown SSH host key: the button that trusts it follows.
    untrusted: bool,
}

impl Messages {
    /// The space between two notices, and before the button.
    const GAP: f32 = 8.0;

    fn new(ctx: &egui::Context, form: &ConnectionForm, width: f32, cap: f32, skin: &Skin) -> Self {
        let Skin { look, palette, .. } = *skin;
        let mut texts: Vec<(std::borrow::Cow<'_, str>, Color32)> = Vec::new();
        if let Some(message) = &form.message {
            texts.push((message.into(), palette.danger));
        }
        let mut untrusted = false;
        match &form.test {
            TestState::Idle | TestState::Running(_) | TestState::Passed => {}
            TestState::Failed(message) => texts.push((message.into(), palette.danger)),
            TestState::Untrusted {
                host, fingerprint, ..
            } => {
                let text = format!(
                    "{} {host}: {fingerprint}",
                    gettext(skin.locale, "Unknown SSH host key for")
                );
                texts.push((text.into(), palette.warning));
                untrusted = true;
            }
        }
        // As wide as the dialog less its margins, and a box's own.
        let wrap = width - 2.0 * Self::side(skin) - 2.0 * Self::pad(skin).x;
        let lay = |wrap: f32| -> Vec<(crate::typography::Laid, Color32)> {
            texts
                .iter()
                .map(|(text, color)| {
                    let laid = Text::one(look, widgets::body(look), text, *color)
                        .wrap(wrap)
                        .layout(ctx);
                    (laid, *color)
                })
                .collect()
        };
        let mut messages = Self {
            notices: lay(wrap),
            cap,
            untrusted,
        };
        // Texts that will scroll leave room for the scroll bar.
        if messages.texts_height(skin) > cap {
            let bar = ctx.global_style().spacing.scroll.allocated_width();
            messages.notices = lay(wrap - bar);
        }
        messages
    }

    /// The margin at each side of the messages.
    fn side(skin: &Skin) -> f32 {
        if skin.look.terminal { 18.0 } else { 20.0 }
    }

    /// The margin over the messages. The terminal look's are plain lines,
    /// which would touch a field the scrolling cuts short.
    fn above(skin: &Skin) -> f32 {
        if skin.look.terminal { 8.0 } else { 0.0 }
    }

    /// The margin under the messages, above the footer.
    fn below(skin: &Skin) -> f32 {
        if skin.look.terminal { 12.0 } else { 18.0 }
    }

    /// What a notice keeps clear around its text: macOS draws a tinted
    /// box, the terminal look a plain coloured line.
    fn pad(skin: &Skin) -> egui::Vec2 {
        if skin.look.terminal {
            egui::Vec2::ZERO
        } else {
            vec2(14.0, 10.0)
        }
    }

    /// How tall the notices are, one under the other, with nothing cut.
    fn texts_height(&self, skin: &Skin) -> f32 {
        let pad = 2.0 * Self::pad(skin).y;
        let texts: f32 = self
            .notices
            .iter()
            .map(|(laid, _)| laid.height() + pad)
            .sum();
        texts + Self::GAP * self.notices.len().saturating_sub(1) as f32
    }

    /// The height the messages take in the dialog: nothing when nothing
    /// went wrong.
    fn height(&self, skin: &Skin) -> f32 {
        if self.notices.is_empty() {
            return 0.0;
        }
        let button = if self.untrusted {
            Self::GAP + skin.field_height()
        } else {
            0.0
        };
        Self::above(skin) + self.texts_height(skin).min(self.cap) + button + Self::below(skin)
    }

    fn show(self, ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
        if self.notices.is_empty() {
            return;
        }
        let Skin { look, palette, .. } = *skin;
        let side = Self::side(skin) as i8;
        let pad = Self::pad(skin);
        egui::Frame::new()
            .inner_margin(egui::Margin {
                left: side,
                right: side,
                top: Self::above(skin) as i8,
                bottom: Self::below(skin) as i8,
            })
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                egui::ScrollArea::vertical()
                    .id_salt("messages")
                    .max_height(self.cap)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for (index, (laid, color)) in self.notices.into_iter().enumerate() {
                            if index > 0 {
                                ui.add_space(Self::GAP);
                            }
                            if look.terminal {
                                laid.label(ui);
                                continue;
                            }
                            egui::Frame::new()
                                .fill(theme::mix(skin.fill, color, 0.08))
                                .corner_radius(CornerRadius::same(10))
                                .inner_margin(egui::Margin::symmetric(pad.x as i8, pad.y as i8))
                                .show(ui, |ui| {
                                    ui.set_width(ui.available_width());
                                    laid.label(ui);
                                });
                        }
                    });
                // The way out of an unknown host key stays in reach however
                // long the text above it is.
                if self.untrusted {
                    ui.add_space(Self::GAP);
                    let trust = skin.say("Trust and test");
                    if ButtonSpec::new(&trust)
                        .label(&gettext(skin.locale, "Trust and test"))
                        .show(ui, skin.field_height(), look, palette)
                        .clicked()
                    {
                        actions.push(Action::TrustTestHostKey);
                    }
                }
            });
    }
}

/// What the footer says about the last Test: that it is running, or what
/// it reached and how long that took.
enum Status {
    Running(String),
    Passed(String),
}

fn status(form: &ConnectionForm, skin: &Skin) -> Option<Status> {
    match form.test {
        TestState::Running(_) => Some(Status::Running(skin.say("Testing…"))),
        TestState::Passed => {
            let driver = form
                .test_spec
                .as_ref()
                .map_or(form.driver, |spec| spec.driver);
            let mut text = format!("{} · {}", gettext(skin.locale, "Connected"), driver.label());
            if let Some(took) = form.test_took {
                text = format!("{text} · {}", crate::ui::format::elapsed(took));
            }
            Some(Status::Passed(skin.look.label(&text)))
        }
        TestState::Idle | TestState::Failed(_) | TestState::Untrusted { .. } => None,
    }
}

/// Where the Test's status goes: from its left edge on, or up to its right
/// edge.
#[derive(Clone, Copy)]
enum StatusAt {
    Left(f32),
    Right(f32),
}

/// Paints the Test's status at `x`, centred on `y`. Returns its width:
/// nothing when there is no status to show.
fn paint_status(ui: &mut Ui, form: &ConnectionForm, x: StatusAt, y: f32, skin: &Skin) -> f32 {
    let Skin { look, palette, .. } = *skin;
    let Some(status) = status(form, skin) else {
        return 0.0;
    };
    let role = widgets::secondary(look);
    let (text, color) = match &status {
        Status::Running(text) => (text, palette.dim),
        Status::Passed(text) => (text, palette.success),
    };
    let laid = Text::one(look, role, text, color).layout(ui.ctx());
    // The mark and the gap after it. The terminal look's are smaller:
    // together about what a letter and a space of its text take, where
    // its design has a tick typed before the words.
    let (mark, gap) = if look.terminal {
        (10.0, 5.0)
    } else {
        (12.0, 6.0)
    };
    let left = match x {
        StatusAt::Left(left) => left,
        StatusAt::Right(right) => right - laid.width() - mark - gap,
    };
    let place = Rect::from_center_size(pos2(left + mark / 2.0, y), vec2(mark, mark));
    match status {
        Status::Running(_) => {
            ui.put(place, egui::Spinner::new().size(mark).color(color));
        }
        Status::Passed(_) => paint_check(ui.painter(), place, color),
    }
    let x = left + mark + gap;
    laid.paint_left(ui.painter(), x, y);
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(x, y - laid.height() / 2.0), laid.size()),
        laid.galley.text(),
    );
    mark + gap + laid.width()
}

/// A check mark filling `place`: the plain tick the designs put before a
/// Test that passed, and in the box of what always holds.
fn paint_check(painter: &egui::Painter, place: Rect, color: Color32) {
    let unit = place.width() / 24.0;
    let at = |x: f32, y: f32| place.min + vec2(x, y) * unit;
    painter.line(
        vec![at(5.0, 12.0), at(10.0, 17.0), at(19.0, 7.0)],
        Stroke::new(2.6 * unit, color),
    );
}
