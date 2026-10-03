//! A connection tab: the connection bar, what connecting shows, the strip
//! of a lost connection, the sidebar, the object tabs and the open object.

use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use tabletist_db::TlsMode;

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, ObjectView, SessionStatus};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Region};
use crate::ui::format::display_safe;
use crate::ui::states;
use crate::ui::widgets::{self, ButtonSpec};

/// The connection bar's height.
pub fn bar_height(look: &Look) -> f32 {
    // macOS: 56 and a 3 pt stripe above, a 1 pt rule below. Omarchy: 48
    // and the rule.
    if look.terminal { 49.0 } else { 60.0 }
}

/// The macOS bar's stripe in the environment colour.
const STRIPE: f32 = 3.0;

/// The bar's height in the window. The bar is the macOS title bar, and at
/// least as tall as the window has it (window points, which egui's `zoom`
/// scales away from its own).
fn top_bar_height(app: &App, zoom: f32) -> f32 {
    bar_height(&app.look).max(app.titlebar.height / zoom)
}

/// The line the bar's contents centre on, below its top: under the macOS
/// stripe and over the rule, or the middle of the terminal bar.
pub fn bar_line(app: &App, zoom: f32) -> f32 {
    let height = top_bar_height(app, zoom);
    if app.look.terminal {
        height / 2.0
    } else {
        (STRIPE + height - 1.0) / 2.0
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let look = app.look;
    top_bar(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let opened = workspace.opened();
    if look.terminal && opened {
        super::object_tabs::show(app, ui, tab);
        status_line(app, ui, tab);
    }
    if !opened {
        // Nothing of the database to show yet: how connecting goes, or
        // why it failed.
        opening(app, ui, tab);
        return;
    }
    banner(app, ui, tab);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let active = workspace.active_tab;
    // What a lost connection left on screen reads as old.
    let stale = !matches!(workspace.status, SessionStatus::Connected);
    let view = workspace.active_object_tab().map(|object| object.view);
    // The tab the row panel shows a row of: a table's Data view, or a SQL
    // editor with a row of its result selected.
    let row_panel = workspace.row_panel_tab();
    super::sidebar::show(app, ui, tab);
    if look.terminal
        && let Some(shown) = row_panel
    {
        super::row_panel::show(app, ui, tab, shown);
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| {
            if stale {
                ui.multiply_opacity(states::STALE);
            }
            if !look.terminal {
                super::object_tabs::show(app, ui, tab);
                if let Some(shown) = row_panel {
                    super::row_panel::show(app, ui, tab, shown);
                }
            }
            let sql = app
                .workspace(tab)
                .is_some_and(|workspace| workspace.active_sql_tab().is_some());
            match active {
                Some(sql_tab) if sql => super::sql_editor::show(app, ui, tab, sql_tab),
                Some(object_tab) => {
                    super::data_view::header(app, ui, tab, object_tab);
                    if view == Some(ObjectView::Data) {
                        super::data_view::toolbar(app, ui, tab, object_tab);
                    }
                    if !look.terminal {
                        super::data_view::footer(app, ui, tab, object_tab);
                    }
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(app.palette.window))
                        .show(ui, |ui| match view {
                            Some(ObjectView::Structure) => {
                                super::structure::show(app, ui, tab, object_tab)
                            }
                            _ => {
                                if super::filter_bar::is_open(app, tab, object_tab) {
                                    let palette = app.palette;
                                    egui::Panel::top(egui::Id::new((
                                        "filter-bar",
                                        tab.0,
                                        object_tab.0,
                                    )))
                                    .resizable(false)
                                    .show_separator_line(false)
                                    .frame(
                                        Frame::new()
                                            .fill(palette.panel)
                                            .inner_margin(Margin::symmetric(12, 8)),
                                    )
                                    .show(ui, |ui| {
                                        super::filter_bar::show(app, ui, tab, object_tab);
                                        let rect = ui.max_rect().expand2(vec2(12.0, 8.0));
                                        widgets::hline(
                                            ui,
                                            rect.x_range(),
                                            rect.bottom(),
                                            palette.outline,
                                        );
                                    });
                                }
                                super::data_view::show(app, ui, tab, object_tab)
                            }
                        });
                }
                None => {
                    ui.centered_and_justified(|ui| {
                        Text::one(
                            &look,
                            widgets::body(&look),
                            &gettext(app.locale, "Select a table or view in the sidebar"),
                            app.palette.secondary,
                        )
                        .layout(ui.ctx())
                        .label(ui);
                    });
                }
            }
        });
}

/// A tab that has not shown its content yet: how connecting goes, or why
/// it failed.
fn opening(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let fill = app.palette.window;
    let failed = app.workspace(tab).is_some_and(|workspace| {
        matches!(
            workspace.status,
            SessionStatus::Disconnected(_) | SessionStatus::Cancelled
        )
    });
    egui::CentralPanel::default()
        .frame(Frame::new().fill(fill))
        .show(ui, |ui| {
            if failed {
                failure(app, ui, tab);
            } else {
                connecting(app, ui, tab);
            }
        });
}

/// The steps of a connect, the one under way with its time, and, for a
/// connect with nothing to lose, the button that gives up: in the middle
/// of the tab, or in the terminal look from its top left.
fn connecting(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let spec = &workspace.spec;
    let sqlite = spec.driver == tabletist_db::Driver::Sqlite;
    let target = if sqlite {
        spec.summary()
    } else {
        format!("{}:{}", spec.host, spec.port)
    };
    let target = match spec.ssh.as_ref().filter(|_| !sqlite) {
        Some(ssh) => format!("{target} {} {}", say("via"), ssh.host),
        None => target,
    };
    let connected = matches!(workspace.status, SessionStatus::Connected);
    let (reach, load) = (
        say(if sqlite { "Open" } else { "Connect to" }),
        say("Load schema"),
    );
    let list = [
        states::Step {
            state: if connected {
                states::StepState::Done
            } else {
                states::StepState::Running(workspace.connecting_for())
            },
            text: &reach,
            detail: &target,
        },
        states::Step {
            state: if connected {
                states::StepState::Running(workspace.tree.schemas.running_for())
            } else {
                states::StepState::Waiting
            },
            text: &load,
            detail: "",
        },
    ];
    let body = ui.max_rect();
    let height = states::steps_height(list.len(), &look);
    let button_height = states::button_height(&look);
    let name = say("Cancel");
    // A tab with SQL editors open keeps them through a switch of database:
    // it has no button that would close them, and the bar's Disconnect is
    // the way out. Named apart from a password prompt's Cancel, which can
    // be open over it.
    let cancel = workspace
        .can_give_up()
        .then(|| states::key_button(&name, "esc", &look).label("Cancel connecting"));
    let width = cancel
        .as_ref()
        .map_or(0.0, |cancel| cancel.width(ui, &look));
    let room = (body.width() - 2.0 * states::INSET).max(0.0);
    let (steps, button) = if look.terminal {
        let top = body.left_top() + vec2(states::INSET, states::INSET);
        (
            Rect::from_min_size(top, vec2(room, height)),
            Rect::from_min_size(
                pos2(top.x, top.y + height + 8.0),
                vec2(width, button_height),
            ),
        )
    } else {
        // The design's 300 wide list with the button 14 under it. A step
        // that needs more (a long host, a tunnel) widens it, up to the
        // room there is.
        let block = states::steps_width(ui, &list, &look).max(300.0).min(room);
        let total = height + 14.0 + button_height;
        let top = (body.center().y - total / 2.0).max(body.top() + states::INSET);
        let center = body.center().x;
        (
            Rect::from_min_size(pos2(center - block / 2.0, top), vec2(block, height)),
            Rect::from_min_size(
                pos2(center - width / 2.0, top + height + 14.0),
                vec2(width, button_height),
            ),
        )
    };
    states::steps(ui, steps, &list, &look, &palette);
    if let Some(cancel) = cancel
        && cancel.show_at(ui, button, &look, &palette).clicked()
    {
        app.actions.push(Action::Disconnect(tab));
    }
}

/// What a connect that failed says first and the sign beside it; `None`
/// is a prompt the user closed. `say` writes our words as the look does.
fn failure_title(
    error: Option<&tabletist_db::Error>,
    spec: &tabletist_db::ConnectSpec,
    say: impl Fn(&'static str) -> String,
) -> (Icon, String) {
    use tabletist_db::{Driver, Error, SshStage};
    let Some(error) = error else {
        return (Icon::CircleAlert, say("Connection cancelled"));
    };
    match error {
        Error::Auth(_) if spec.user.is_empty() => (Icon::Lock, say("Login refused")),
        Error::Auth(_) => (
            Icon::Lock,
            format!("{} {}", say("Password rejected for"), spec.user),
        ),
        Error::Connect(_) | Error::Timeout if spec.driver == Driver::Sqlite => (
            Icon::CircleAlert,
            format!("{} {}", say("Can't open"), spec.summary()),
        ),
        Error::Connect(_) | Error::Timeout => (
            Icon::WifiOff,
            format!("{} {}:{}", say("Can't reach"), spec.host, spec.port),
        ),
        Error::Tls(_) => (Icon::ShieldAlert, say("TLS or certificate problem")),
        Error::Ssh {
            stage: SshStage::HostKeyUnknown { .. } | SshStage::HostKeyMismatch { .. },
            ..
        } => (Icon::ShieldAlert, say("SSH host key not trusted")),
        Error::Ssh { .. } => {
            let host = spec.ssh.as_ref().map_or("", |ssh| ssh.host.as_str());
            (
                Icon::WifiOff,
                format!("{} {host} {}", say("SSH tunnel to"), say("failed")),
            )
        }
        Error::InvalidSpec(_) => (
            Icon::CircleAlert,
            say("The connection's settings are not valid"),
        ),
        _ => (Icon::CircleAlert, say("Could not connect")),
    }
}

/// A connect that failed before the tab showed anything: what failed in
/// plain words, the exact error under it, and the buttons, the one that
/// fixes the cause first.
fn failure(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    use tabletist_db::Error;
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let error = match &workspace.status {
        SessionStatus::Disconnected(error) => Some(error),
        _ => None,
    };
    let (icon, title) = failure_title(error, &workspace.spec, say);
    let sentence = error
        .map(|error| crate::ui::format::describe_error(locale, error))
        .unwrap_or_default();
    let raw = error.map(ToString::to_string).unwrap_or_default();
    // The settings are what is wrong: changing them comes first.
    let edit_first = matches!(
        error,
        Some(Error::Auth(_) | Error::Tls(_) | Error::InvalidSpec(_))
    );
    let tls = matches!(error, Some(Error::Tls(_)));
    let conn = workspace.conn_id.clone();
    let body = ui.max_rect();
    // The design's 28 at the sides; the card is no wider than 520.
    let side = if look.terminal { states::INSET } else { 28.0 };
    let width = (body.width() - 2.0 * side).clamp(0.0, 520.0);
    let (left, top) = if look.terminal {
        (body.left() + side, body.top() + states::INSET)
    } else {
        (
            body.center().x - width / 2.0,
            body.top() + (body.height() * 0.2).max(24.0),
        )
    };
    let column = Rect::from_min_max(pos2(left, top), pos2(left + width, body.bottom()));
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("failure")
            .max_rect(column)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
    let card = states::Card {
        tone: states::Tone::Danger,
        icon,
        title: &title,
        text: &sentence,
    };
    states::card(&mut ui, &card, &look, &palette);
    // The exact error when it says more than the sentence: always on
    // screen, so keyboards and screen readers get it.
    if !raw.is_empty() && raw != sentence {
        Text::one(
            &look,
            widgets::secondary(&look),
            &say("Details"),
            palette.dim,
        )
        .layout(ui.ctx())
        .label(&mut ui);
        Text::one(&look, widgets::code(&look), &raw, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(&mut ui);
    }
    let (mut retry, mut edit) = (false, false);
    let details = format!("{title}\n{raw}");
    ui.horizontal(|ui| {
        let height = states::button_height(&look);
        let (again, change, copy) = (say("Retry"), say("Edit connection"), say("Copy details"));
        // The button that fixes the cause is the primary one.
        fn lead(button: ButtonSpec<'_>, first: bool) -> ButtonSpec<'_> {
            if first { button.primary() } else { button }
        }
        let retry_button = lead(states::button(&again, &look).label("Retry"), !edit_first);
        let edit_button = lead(
            states::button(&change, &look).label("Edit connection"),
            edit_first,
        );
        if edit_first {
            edit = edit_button.show(ui, height, &look, &palette).clicked();
            retry = retry_button.show(ui, height, &look, &palette).clicked();
        } else {
            retry = retry_button.show(ui, height, &look, &palette).clicked();
            edit = edit_button.show(ui, height, &look, &palette).clicked();
        }
        // A prompt the user closed has no error, so nothing to copy.
        if error.is_none() {
            return;
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let copy = states::button(&copy, &look).label("Copy details").quiet();
            if copy.show(ui, height, &look, &palette).clicked() {
                ui.ctx().copy_text(details.clone());
            }
        });
    });
    if tls {
        let note = say(
            "There is no \"connect anyway\". Change the TLS mode or the host in the connection.",
        );
        Text::one(&look, widgets::secondary(&look), &note, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(&mut ui);
    }
    if retry {
        app.actions.push(Action::Reconnect(tab));
    }
    if edit {
        app.actions.push(Action::EditConnection(conn));
    }
}

/// What the connection bar says: the open connections, and more about the
/// one the bar belongs to.
struct BarInfo {
    env: crate::env::Environment,
    /// Whether the bar's own connection opens read-only.
    read_only: bool,
    database: String,
    databases: Vec<String>,
    tls: Option<(&'static str, Tone)>,
    ssh_host: Option<String>,
    chips: Vec<Chip>,
}

/// How a status reads: fine, or a warning.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Good,
    Warn,
}

/// How a connection's session stands, as its chip shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Link {
    Connected,
    Connecting,
    Disconnected,
    Cancelled,
}

/// One open connection in the bar.
struct Chip {
    tab: ConnTabId,
    /// Whether the bar is this connection's own.
    own: bool,
    name: String,
    env: crate::env::Environment,
    /// Under the name: the database (a file's name for SQLite), or how the
    /// session stands while it is not connected.
    line: String,
    link: Link,
    /// Its place among the open connections, from 1: Cmd/Ctrl+1 is the
    /// first.
    number: usize,
    /// What its card says: a name and a value per line.
    card: Vec<(String, String)>,
}

fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
    let workspace = app.workspace(tab)?;
    let spec = &workspace.spec;
    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
    // Local traffic never crosses a network, so its TLS says nothing; a
    // remote connection always says how far it can be trusted.
    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
    let tls = remote.then(|| {
        let encrypted =
            matches!(workspace.status, SessionStatus::Connected).then_some(workspace.encrypted);
        let (text, warn) = tls_status(spec.effective_tls(), encrypted);
        (text, if warn { Tone::Warn } else { Tone::Good })
    });
    Some(BarInfo {
        env: workspace.environment,
        read_only: workspace.access == tabletist_db::Access::ReadOnly,
        database: if sqlite {
            String::new()
        } else {
            spec.database.clone()
        },
        databases: workspace.databases.value.clone().unwrap_or_default(),
        tls,
        ssh_host: spec.ssh.as_ref().map(|ssh| ssh.host.clone()),
        chips: chips(app, tab),
    })
}

/// A chip for each open connection, in the header's order. `own` is the
/// connection whose bar this is.
fn chips(app: &App, own: ConnTabId) -> Vec<Chip> {
    let now = crate::util::now_secs();
    app.open_connections()
        .enumerate()
        .map(|(index, (tab, workspace))| {
            let (link, state) = match &workspace.status {
                SessionStatus::Connected => (Link::Connected, None),
                SessionStatus::Connecting { .. } => (Link::Connecting, Some("Connecting…")),
                SessionStatus::Disconnected(_) => (Link::Disconnected, Some("Disconnected")),
                SessionStatus::Cancelled => (Link::Cancelled, Some("Cancelled")),
            };
            let line = match state {
                Some(state) => app.look.label(&gettext(app.locale, state)),
                None => target(workspace),
            };
            Chip {
                tab,
                own: tab == own,
                name: workspace.name.clone(),
                env: workspace.environment,
                line,
                link,
                number: index + 1,
                card: card_rows(workspace, &app.look, app.locale, now),
            }
        })
        .collect()
}

/// What a chip's card says about its connection: where it points, what
/// serves it, how far it can be trusted, and how it stands.
fn card_rows(
    workspace: &crate::model::Workspace,
    look: &Look,
    locale: crate::i18n::Locale,
    now: u64,
) -> Vec<(String, String)> {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let spec = &workspace.spec;
    let sqlite = workspace.driver == tabletist_db::Driver::Sqlite;
    let mut rows = Vec::new();
    if sqlite {
        let path = spec
            .sqlite_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_default();
        rows.push((say("File"), path));
    } else {
        rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
        if !spec.database.is_empty() {
            rows.push((say("Database"), display_safe(&spec.database).into_owned()));
        }
        if !spec.user.is_empty() {
            rows.push((say("User"), spec.user.clone()));
        }
    }
    let server = workspace
        .server_version
        .value
        .clone()
        .unwrap_or_else(|| workspace.driver.label().to_owned());
    rows.push((say("Server"), server));
    let connected = matches!(workspace.status, SessionStatus::Connected);
    let remote = !sqlite && !crate::model::is_local_host(&spec.host);
    let tls = if remote {
        let encrypted = connected.then_some(workspace.encrypted);
        say(tls_status(spec.effective_tls(), encrypted).0)
    } else {
        say("Local")
    };
    let ssh = match &spec.ssh {
        Some(ssh) => format!("{} {}", say("SSH via"), ssh.host),
        None => say("no SSH"),
    };
    rows.push((say("Security"), format!("{tls} · {ssh}")));
    let state = match &workspace.status {
        SessionStatus::Connected => {
            let tabs = match workspace.tabs.len() {
                0 => say("no tabs open"),
                1 => say("1 tab open"),
                count => format!("{count} {}", say("tabs open")),
            };
            let since = crate::connections::when(workspace.connected_at, now);
            (say("Connected"), format!("{since} · {tabs}"))
        }
        SessionStatus::Connecting { .. } => (say("Status"), say("Connecting…")),
        SessionStatus::Disconnected(_) => (say("Status"), say("Disconnected")),
        SessionStatus::Cancelled => (say("Status"), say("Cancelled")),
    };
    rows.push(state);
    rows
}

/// What a click on `chip` does, as its card says it. The bar's own chip
/// without other databases to switch to does nothing.
fn card_hint(
    chip: &Chip,
    switchable: bool,
    look: &Look,
    locale: crate::i18n::Locale,
) -> Option<String> {
    let text = if !chip.own {
        "Click to switch to this connection"
    } else if switchable {
        "Click to switch database"
    } else {
        return None;
    };
    Some(look.label(&gettext(locale, text)))
}

/// A chip's card, shown while the pointer rests on it: the connection and
/// its key, what [`card_rows`] says, and what a click does.
fn chip_card(ui: &mut egui::Ui, chip: &Chip, hint: Option<&str>, look: &Look, palette: &Palette) {
    let small = widgets::secondary(look);
    let strong = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
    let colors = crate::env::env_colors(chip.env, crate::env::Platform::of(look), palette);
    let badge = if look.terminal {
        Badge::Tracked
    } else {
        Badge::Chip
    };
    ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
    ui.horizontal(|ui| {
        let name = Text::one(look, strong, &chip.name, palette.text)
            .layout(ui.ctx())
            .label(ui);
        let width = env_badge_width(ui, chip.env, badge, look);
        let (place, _) = ui.allocate_exact_size(vec2(width, name.rect.height()), Sense::hover());
        env_badge(
            ui,
            place.left(),
            place.center().y,
            chip.env,
            &colors,
            badge,
            look,
        );
        if chip.number <= 9 {
            let key = look.label(&format!("{}{}", look.command_key(), chip.number));
            Text::one(look, small, &key, palette.dim)
                .layout(ui.ctx())
                .label(ui);
        }
    });
    egui::Grid::new(("chip-card", chip.tab.0))
        .num_columns(2)
        .spacing(vec2(10.0, 6.0))
        .show(ui, |ui| {
            for (label, value) in &chip.card {
                Text::one(look, small, label, palette.dim)
                    .layout(ui.ctx())
                    .label(ui);
                Text::one(look, small, value, palette.text)
                    .layout(ui.ctx())
                    .label(ui);
                ui.end_row();
            }
        });
    if let Some(hint) = hint {
        Text::one(look, small, hint, palette.dim)
            .layout(ui.ctx())
            .label(ui);
    }
}

/// Where a connection points, in a word: its database, or the file's name
/// for SQLite.
fn target(workspace: &crate::model::Workspace) -> String {
    match &workspace.spec.sqlite_path {
        Some(path) => crate::model::file_name(&path.display().to_string()),
        None => workspace.spec.database.clone(),
    }
}

/// The narrowest a chip's text gets. Chips that do not fit even so are
/// clipped at the row's end.
const FLOOR: f32 = 48.0;

/// The widest a chip's text may be so that `columns` take `room` at most:
/// the wider ones are cut to it, the others keep their width. Never under
/// `FLOOR`.
fn text_cap(columns: &[f32], room: f32) -> f32 {
    let widest = columns.iter().copied().fold(0.0, f32::max);
    let used = |cap: f32| columns.iter().map(|width| width.min(cap)).sum::<f32>();
    if used(widest) <= room {
        return widest;
    }
    let (mut fits, mut too_wide) = (FLOOR.min(widest), widest);
    for _ in 0..24 {
        let middle = (fits + too_wide) / 2.0;
        if used(middle) <= room {
            fits = middle;
        } else {
            too_wide = middle;
        }
    }
    fits
}

/// How the bar's row is shared.
#[derive(Debug, PartialEq)]
struct Fit {
    /// How many of the pills show.
    pills: usize,
    /// The widest a chip's text may be.
    cap: f32,
}

/// Shares `room` between the chips and the pills after them, each `gap`
/// from the last. A chip is `(fixed, column)`: what it draws around its
/// text, and its text's own width. The pills give way first, from the
/// last; then the widest texts are cut.
fn fit(chips: &[(f32, f32)], pills: &[f32], gap: f32, room: f32) -> Fit {
    let fixed = chips.iter().map(|(fixed, _)| fixed).sum::<f32>()
        + gap * chips.len().saturating_sub(1) as f32;
    let columns: Vec<f32> = chips.iter().map(|(_, column)| *column).collect();
    let natural = fixed + columns.iter().sum::<f32>();
    let after = |shown: usize| pills[..shown].iter().map(|pill| gap + pill).sum::<f32>();
    let mut shown = pills.len();
    while shown > 0 && natural + after(shown) > room {
        shown -= 1;
    }
    Fit {
        pills: shown,
        cap: text_cap(&columns, room - fixed - after(shown)),
    }
}

/// Where each chip goes in `row`: `widths` laid from its left, `gap`
/// apart, slid left by as much as brings the bar's own chip (`own`) wholly
/// into the row when they overflow it.
fn place(
    widths: &[f32],
    own: Option<usize>,
    gap: f32,
    row: Rect,
    y: f32,
    height: f32,
) -> Vec<Rect> {
    let mut rects = Vec::with_capacity(widths.len());
    let mut x = row.left();
    for width in widths {
        rects.push(Rect::from_min_size(
            pos2(x, y - height / 2.0),
            vec2(*width, height),
        ));
        x += width + gap;
    }
    let over = own
        .and_then(|own| rects.get(own))
        .map_or(0.0, |own| (own.right() - row.right()).max(0.0));
    for rect in &mut rects {
        *rect = rect.translate(vec2(-over, 0.0));
    }
    rects
}

/// The colour that says how a chip's session stands: its environment's
/// while connected.
fn link_color(link: Link, env: &crate::env::EnvColors, palette: &Palette) -> egui::Color32 {
    match link {
        Link::Connected => env.base(),
        Link::Connecting => palette.warning,
        Link::Disconnected => palette.danger,
        Link::Cancelled => palette.dim,
    }
}

/// What a click on another connection's chip is announced as.
fn switch_label(chip: &Chip, look: &Look, locale: crate::i18n::Locale) -> String {
    format!(
        "{} {} · {}",
        gettext(locale, "Switch to"),
        chip.name,
        chip.env.label(crate::env::Platform::of(look))
    )
}

/// Announces the bar's own chip: the database switcher.
fn announce_switcher(
    response: &egui::Response,
    info: &BarInfo,
    switchable: bool,
    locale: crate::i18n::Locale,
) {
    response.widget_info(|| {
        let mut announced = egui::WidgetInfo::labeled(
            egui::WidgetType::ComboBox,
            switchable,
            gettext(locale, "Database"),
        );
        announced.current_text_value = Some(display_safe(&info.database).into_owned());
        announced
    });
}

/// The pop-up of the server's other databases, under the bar's own chip.
fn database_menu(
    response: &egui::Response,
    tab: ConnTabId,
    info: &BarInfo,
    look: &Look,
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
    egui::Popup::menu(response).show(|ui| {
        ui.set_min_width(response.rect.width());
        for database in &info.databases {
            // Database names come from the server: nothing hidden.
            let text = Text::one(
                look,
                role,
                &display_safe(database),
                egui::Color32::PLACEHOLDER,
            )
            .layout(ui.ctx());
            if ui
                .add(egui::Button::selectable(
                    *database == info.database,
                    text.galley,
                ))
                .clicked()
                && *database != info.database
            {
                actions.push(Action::SwitchDatabase {
                    tab,
                    database: database.clone(),
                });
            }
        }
    });
}

/// The connection bar: the connection's environment colour behind the open
/// connections, and the way out.
fn top_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(info) = bar_info(app, tab) else {
        return;
    };
    let env = crate::env::env_colors(info.env, crate::env::Platform::of(&look), &palette);
    let (tint, border) = (env.bar_bg(), env.bar_border());
    // macOS: the bar is the title bar, beside the window buttons.
    let zoom = ui.ctx().zoom_factor();
    let inset = app.titlebar.inset / zoom;
    let height = top_bar_height(app, zoom);
    let mut actions = Vec::new();
    egui::Panel::top(egui::Id::new(("workspace-top", tab.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(tint))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            // The empty bar moves the window, as a title bar does. Only
            // the pointer can: it is no stop for the Tab key.
            let drag = ui.interact(rect, ui.id().with("drag"), Sense::CLICK | Sense::DRAG);
            focus::region(ui, Region::Header, rect);
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            if drag.double_clicked() {
                let maximized = ui.input(|input| input.viewport().maximized.unwrap_or(false));
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, border);
            if look.terminal {
                terminal_bar(
                    ui,
                    tab,
                    rect,
                    &info,
                    &env,
                    &look,
                    &palette,
                    &mut actions,
                    locale,
                );
            } else {
                let stripe = Rect::from_min_size(rect.min, vec2(rect.width(), STRIPE));
                ui.painter()
                    .rect_filled(stripe, CornerRadius::ZERO, env.base());
                let body = Rect::from_min_max(
                    pos2(rect.left() + inset, rect.top() + STRIPE),
                    pos2(rect.right(), rect.bottom() - 1.0),
                );
                mac_bar(
                    ui,
                    tab,
                    body,
                    &info,
                    &env,
                    &look,
                    &palette,
                    &mut actions,
                    locale,
                );
            }
        });
    app.actions.extend(actions);
}

/// macOS: the Connections button, a chip for each open connection (the
/// bar's own is a pop-up of the server's other databases), the read-only
/// pill of a connection that is, and Disconnect.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn mac_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &crate::env::EnvColors,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    // The bar's own rule, and faces over its tint: nearly white on a light
    // bar, a thin wash of white on a dark one.
    let rim = env.bar_border();
    let face = |alpha: f32| {
        let alpha = if palette.dark { alpha * 0.09 } else { alpha };
        egui::Color32::WHITE.gamma_multiply(alpha)
    };
    let hair = Stroke::new(widgets::hairline(ui), rim);
    let corner = CornerRadius::same(look.radius);
    // Connections, 16 in: the way to the saved connections, a face of the
    // bar's own under the server icon.
    let connections =
        Rect::from_min_size(pos2(rect.left() + 16.0, center - 16.0), vec2(34.0, 32.0));
    {
        let label = gettext(locale, "Connections");
        let response = ui.interact(connections, ui.id().with("connections"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
        focus::claim(ui, Region::Header, &response);
        let (fill, tint) = if response.hovered() {
            (face(1.0), palette.text)
        } else {
            (face(0.7), palette.secondary)
        };
        ui.painter().rect_filled(connections, corner, fill);
        ui.painter()
            .rect_stroke(connections, corner, hair, StrokeKind::Inside);
        Icon::Server.image(tint, 16.0).paint_at(
            ui,
            Rect::from_center_size(connections.center(), vec2(16.0, 16.0)),
        );
        if response
            .on_hover_text(connections_hint(look, locale))
            .clicked()
        {
            actions.push(Action::ShowConnections);
        }
    }
    // Disconnect, 12 in from the right: an icon and its label, 6 apart.
    let label = gettext(locale, "Disconnect");
    let text = Text::one(look, TextRole::UiBody, &label, palette.secondary).layout(ui.ctx());
    let width = 10.0 + 14.0 + 6.0 + text.width() + 10.0;
    let button = Rect::from_min_size(
        pos2(rect.right() - 12.0 - width, center - 16.0),
        vec2(width, 32.0),
    );
    let response = ui.interact(button, ui.id().with("disconnect"), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    if response.hovered() {
        ui.painter()
            .rect_filled(button, CornerRadius::same(6), face(0.7));
    }
    Icon::LogIn.image(palette.secondary, 14.0).paint_at(
        ui,
        Rect::from_center_size(pos2(button.left() + 17.0, center), vec2(14.0, 14.0)),
    );
    text.paint_left(ui.painter(), button.left() + 30.0, center);
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }

    // The row between them, 12 from each: the chips, then the pills. What
    // does not fit is cut at the row's end.
    let left = connections.right() + 12.0;
    let row = Rect::from_min_max(
        pos2(left, rect.top()),
        pos2((button.left() - 12.0).max(left), rect.bottom()),
    );
    let mut strip = ui.new_child(egui::UiBuilder::new().id_salt("chips").max_rect(row));
    strip.shrink_clip_rect(row);
    let ui = &mut strip;
    let switchable = info.databases.len() > 1;
    let platform = crate::env::Platform::of(look);
    // A chip: 12, an 8 pt dot, 10, the name and its environment over the
    // database, and 12 (the bar's own: 10, the pop-up's chevrons, 10).
    let name_role = |chip: &Chip| {
        if chip.own {
            TextRole::UiBodySemibold
        } else {
            TextRole::UiBodyStrong
        }
    };
    let line_role = |chip: &Chip| {
        if chip.link == Link::Connected {
            TextRole::MonoSecondary
        } else {
            TextRole::Secondary
        }
    };
    let measured: Vec<(f32, f32)> = info
        .chips
        .iter()
        .map(|chip| {
            let name = name_role(chip).width(ui.ctx(), look.faces, &chip.name);
            let badge = env_badge_width(ui, chip.env, Badge::Chip, look);
            let line = line_role(chip).width(ui.ctx(), look.faces, &display_safe(&chip.line));
            let tail = if chip.own && switchable {
                10.0 + 12.0 + 10.0
            } else {
                12.0
            };
            (30.0 + tail, (name + 6.0 + badge).max(line))
        })
        .collect();
    // Pills after the chips: read-only when it is, then TLS and SSH when
    // remote.
    let mut pills = Vec::new();
    if info.read_only {
        pills.push((
            Some(Icon::Lock),
            gettext(locale, "Read-only").into_owned(),
            palette.secondary,
        ));
    }
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        pills.push((Some(Icon::Lock), gettext(locale, text).into_owned(), color));
    }
    if let Some(host) = &info.ssh_host {
        pills.push((
            None,
            format!("{} {host}", gettext(locale, "via SSH")),
            palette.secondary,
        ));
    }
    let pill_widths: Vec<f32> = pills
        .iter()
        .map(|(icon, text, _)| {
            let icon = if icon.is_some() { 12.0 + 6.0 } else { 0.0 };
            20.0 + icon + TextRole::Secondary.width(ui.ctx(), look.faces, text)
        })
        .collect();
    let shared = fit(&measured, &pill_widths, 12.0, row.width());
    let widths: Vec<f32> = measured
        .iter()
        .map(|(fixed, column)| fixed + column.min(shared.cap))
        .collect();
    let own = info.chips.iter().position(|chip| chip.own);
    let rects = place(&widths, own, 12.0, row, center, 40.0);
    for ((chip, rect), (fixed, _)) in info.chips.iter().zip(&rects).zip(&measured) {
        let hit = rect.intersect(row);
        if !hit.is_positive() {
            continue;
        }
        let colors = crate::env::env_colors(chip.env, platform, palette);
        let hint = card_hint(chip, switchable, look, locale);
        let response = ui
            .interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click())
            .on_hover_ui(|ui| chip_card(ui, chip, hint.as_deref(), look, palette));
        let (name_color, fill) = if chip.own {
            announce_switcher(&response, info, switchable, locale);
            (palette.text, Some(face(0.85)))
        } else {
            let label = switch_label(chip, look, locale);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            (palette.secondary, response.hovered().then(|| face(0.5)))
        };
        if let Some(fill) = fill {
            ui.painter().rect_filled(*rect, corner, fill);
        }
        ui.painter()
            .rect_stroke(*rect, corner, hair, StrokeKind::Inside);
        ui.painter().circle_filled(
            pos2(rect.left() + 16.0, center),
            4.0,
            link_color(chip.link, &colors, palette),
        );
        // Two lines 2 apart, centred on the bar's line; what the chip has
        // no room for is cut with an ellipsis.
        let (top, bottom) = (center - 8.0, center + 9.0);
        let x = rect.left() + 30.0;
        let room = rect.width() - fixed;
        let badge = env_badge_width(ui, chip.env, Badge::Chip, look);
        let role = name_role(chip);
        let name = crate::ui::grid::ellipsize(&chip.name, room - 6.0 - badge, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let name = Text::one(look, role, &name, name_color).layout(ui.ctx());
        name.paint_left(ui.painter(), x, top);
        env_badge(
            ui,
            x + name.width() + 6.0,
            top,
            chip.env,
            &colors,
            Badge::Chip,
            look,
        );
        let role = line_role(chip);
        let line_color = if chip.link == Link::Connected {
            palette.dim
        } else {
            link_color(chip.link, &colors, palette)
        };
        let safe = display_safe(&chip.line);
        let line = crate::ui::grid::ellipsize(&safe, room, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let line = Text::one(look, role, &line, line_color).layout(ui.ctx());
        line.paint_left(ui.painter(), x, bottom);
        if chip.own {
            // The bar's own connection is read as text too.
            for (laid, y) in [(&name, top), (&line, bottom)] {
                widgets::announce(
                    ui,
                    Rect::from_min_size(pos2(x, y - 8.0), vec2(laid.width().max(1.0), 16.0)),
                    laid.galley.text(),
                );
            }
            if switchable {
                Icon::ChevronsUpDown.image(palette.dim, 12.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(12.0, 12.0)),
                );
                database_menu(&response, tab, info, look, actions);
            }
        } else if response.clicked() {
            actions.push(Action::ActivateConnTab(chip.tab));
        }
    }
    let mut left = rects.last().map_or(row.left(), |last| last.right() + 12.0);
    for ((icon, text, color), width) in pills.iter().zip(&pill_widths).take(shared.pills) {
        let rect = Rect::from_min_size(pos2(left, center - 13.0), vec2(*width, 26.0));
        let corner = CornerRadius::same(13);
        ui.painter().rect_filled(rect, corner, face(0.8));
        ui.painter()
            .rect_stroke(rect, corner, hair, StrokeKind::Inside);
        let mut x = rect.left() + 10.0;
        if let Some(icon) = icon {
            icon.image(*color, 12.0).paint_at(
                ui,
                Rect::from_center_size(pos2(x + 6.0, center), vec2(12.0, 12.0)),
            );
            x += 12.0 + 6.0;
        }
        let label = Text::one(look, TextRole::Secondary, text, *color).layout(ui.ctx());
        label.paint_left(ui.painter(), x, center);
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(x, center - 8.0), vec2(label.width(), 16.0)),
            text,
        );
        left = rect.right() + 12.0;
    }
}

/// Omarchy: the Connections button, a chip for each open connection (the
/// bar's own is a pop-up of the server's other databases), read-only when
/// it is, and the key that closes the connection.
#[allow(clippy::too_many_arguments)] // one call site; the pieces are unrelated
fn terminal_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    rect: Rect,
    info: &BarInfo,
    env: &crate::env::EnvColors,
    look: &Look,
    palette: &Palette,
    actions: &mut Vec<Action>,
    locale: crate::i18n::Locale,
) {
    let center = rect.center().y;
    let connection_line = env.bar_border();
    // Twelve in and twelve apart, as the design's row.
    let mut x = rect.left() + 12.0;
    // Connections first: a boxed word, eight in from its line, that opens
    // the saved connections.
    {
        let label = gettext(locale, "Connections");
        let text = Text::one(
            look,
            TextRole::OBody,
            &format!("≡ {}", gettext(locale, "conn")),
            palette.text,
        )
        .layout(ui.ctx());
        let button = Rect::from_min_size(
            pos2(x, center - 12.0),
            vec2(text.width() + 16.0 + 2.0, 24.0),
        );
        let response = ui.interact(button, ui.id().with("connections"), Sense::click());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
        focus::claim(ui, Region::Header, &response);
        let line = if response.hovered() {
            env.base()
        } else {
            connection_line
        };
        ui.painter().rect_stroke(
            button,
            CornerRadius::same(3),
            Stroke::new(1.0, line),
            StrokeKind::Inside,
        );
        text.paint_left(ui.painter(), button.left() + 9.0, center);
        if response
            .on_hover_text(connections_hint(look, locale))
            .clicked()
        {
            actions.push(Action::ShowConnections);
        }
        x = button.right() + 12.0;
    }
    // The way out, a muted note that also answers a click.
    let note = "ctrl+shift+w disconnect";
    let width = widgets::measure(ui, label_copy(look, TextRole::OSecondary, note));
    let left = rect.right() - 12.0 - width;
    let hit = Rect::from_min_size(
        pos2(left - 4.0, rect.top()),
        vec2(width + 8.0, rect.height()),
    );
    let response = ui.interact(hit, ui.id().with("disconnect"), Sense::click());
    let label = gettext(locale, "Disconnect");
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
    let color = if response.hovered() {
        palette.text
    } else {
        palette.dim
    };
    widgets::paint_text(
        ui,
        left,
        center,
        Text::one(look, TextRole::OSecondary, note, color),
    );
    if response.clicked() {
        actions.push(Action::Disconnect(tab));
    }

    // The row between them: the chips, then the tags. What does not fit is
    // cut at the row's end, 12 before the note.
    let row = Rect::from_min_max(
        pos2(x, rect.top()),
        pos2((hit.left() - 12.0).max(x), rect.bottom()),
    );
    let mut strip = ui.new_child(egui::UiBuilder::new().id_salt("chips").max_rect(row));
    strip.shrink_clip_rect(row);
    let ui = &mut strip;
    let switchable = info.databases.len() > 1;
    let platform = crate::env::Platform::of(look);
    let arrow = widgets::measure(ui, label_copy(look, TextRole::OBody, "▾"));
    // A chip: 5, the environment's tag, 8, the name over the database, and
    // 8 (the bar's own: 8, the pop-up's arrow, 8).
    let name_role = |chip: &Chip| {
        if chip.own {
            TextRole::OGroup
        } else {
            TextRole::OBody
        }
    };
    let badge_style = |chip: &Chip| {
        if chip.own {
            Badge::Tracked
        } else {
            Badge::Outlined
        }
    };
    let small = TextRole::OSecondary;
    let measured: Vec<(f32, f32)> = info
        .chips
        .iter()
        .map(|chip| {
            let badge = env_badge_width(ui, chip.env, badge_style(chip), look);
            let name = name_role(chip).width(ui.ctx(), look.faces, &chip.name);
            let line = small.width(ui.ctx(), look.faces, &display_safe(&chip.line));
            let tail = if chip.own && switchable {
                8.0 + arrow + 8.0
            } else {
                8.0
            };
            (5.0 + badge + 8.0 + tail, name.max(line))
        })
        .collect();
    // Tags after the chips: read-only when it is, then TLS and SSH when
    // remote.
    let mut tags = Vec::new();
    if info.read_only {
        tags.push((gettext(locale, "read-only").into_owned(), palette.text));
    }
    if let Some((text, tone)) = info.tls {
        let color = match tone {
            Tone::Good => palette.success,
            Tone::Warn => palette.warning,
        };
        tags.push((gettext(locale, text).into_owned(), color));
    }
    if let Some(host) = &info.ssh_host {
        tags.push((
            format!("{} {host}", gettext(locale, "via SSH")),
            palette.dim,
        ));
    }
    let tag_role = TextRole::OCaption;
    let tag_widths: Vec<f32> = tags
        .iter()
        .map(|(text, _)| widgets::measure(ui, label_copy(look, tag_role, text)) + 14.0 + 2.0)
        .collect();
    let shared = fit(&measured, &tag_widths, 12.0, row.width());
    let widths: Vec<f32> = measured
        .iter()
        .map(|(fixed, column)| fixed + column.min(shared.cap))
        .collect();
    let own = info.chips.iter().position(|chip| chip.own);
    let rects = place(&widths, own, 12.0, row, center, 38.0);
    let corner = CornerRadius::same(3);
    for ((chip, rect), (fixed, _)) in info.chips.iter().zip(&rects).zip(&measured) {
        let hit = rect.intersect(row);
        if !hit.is_positive() {
            continue;
        }
        let colors = crate::env::env_colors(chip.env, platform, palette);
        let hint = card_hint(chip, switchable, look, locale);
        let response = ui
            .interact(hit, ui.id().with(("chip", chip.tab.0)), Sense::click())
            .on_hover_ui(|ui| chip_card(ui, chip, hint.as_deref(), look, palette));
        // The bar's own chip: the panel's colour inside its environment's
        // line. The others: the window's line, the text's when pointed at.
        let line = if chip.own {
            announce_switcher(&response, info, switchable, locale);
            ui.painter().rect_filled(*rect, corner, palette.panel);
            colors.base()
        } else {
            let label = switch_label(chip, look, locale);
            response
                .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, &label));
            if response.hovered() {
                palette.dim
            } else {
                palette.outline
            }
        };
        ui.painter()
            .rect_stroke(*rect, corner, Stroke::new(1.0, line), StrokeKind::Inside);
        let style = badge_style(chip);
        let badge = env_badge(
            ui,
            rect.left() + 5.0,
            center,
            chip.env,
            &colors,
            style,
            look,
        );
        // A 13 pt name over a 12 pt line, centred on the bar's line; what
        // the chip has no room for is cut with an ellipsis.
        let (top, bottom) = (center - 7.5, center + 8.0);
        let x = rect.left() + 5.0 + badge + 8.0;
        let room = rect.width() - fixed;
        let role = name_role(chip);
        let name = crate::ui::grid::ellipsize(&chip.name, room, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let name = Text::one(look, role, &name, palette.text).layout(ui.ctx());
        name.paint_left(ui.painter(), x, top);
        let line_color = if chip.link == Link::Connected {
            palette.dim
        } else {
            link_color(chip.link, &colors, palette)
        };
        let safe = display_safe(&chip.line);
        let line = crate::ui::grid::ellipsize(&safe, room, false, |text| {
            small.width(ui.ctx(), look.faces, text)
        });
        let line = Text::one(look, small, &line, line_color).layout(ui.ctx());
        line.paint_left(ui.painter(), x, bottom);
        if chip.own {
            // The bar's own connection is read as text too.
            for (laid, y) in [(&name, top), (&line, bottom)] {
                widgets::announce(
                    ui,
                    Rect::from_min_size(pos2(x, y - 8.0), vec2(laid.width().max(1.0), 16.0)),
                    laid.galley.text(),
                );
            }
            if switchable {
                widgets::paint_text(
                    ui,
                    rect.right() - 8.0 - arrow,
                    center,
                    Text::one(look, TextRole::OBody, "▾", palette.dim),
                );
                database_menu(&response, tab, info, look, actions);
            }
        } else if response.clicked() {
            actions.push(Action::ActivateConnTab(chip.tab));
        }
    }
    let mut x = rects.last().map_or(row.left(), |last| last.right() + 12.0);
    let line = tag_role.row_height(ui.ctx(), look.faces);
    for ((text, color), width) in tags.iter().zip(&tag_widths).take(shared.pills) {
        let rect = Rect::from_min_size(
            pos2(x, center - (line + 2.0) / 2.0),
            vec2(*width, line + 2.0),
        );
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, connection_line),
            StrokeKind::Inside,
        );
        widgets::paint_label(ui, x + 8.0, center, Text::one(look, tag_role, text, *color));
        x = rect.right() + 12.0;
    }
}

/// Text for measuring only: never painted, so never recorded.
fn label_copy(look: &Look, role: TextRole, text: &str) -> Text {
    Text::one(look, role, text, egui::Color32::PLACEHOLDER)
}

/// The Connections button's tooltip: its name and the key that does the
/// same, as the look spells both.
fn connections_hint(look: &Look, locale: crate::i18n::Locale) -> String {
    look.label(&format!(
        "{} · {}O",
        gettext(locale, "Connections"),
        look.command_key()
    ))
}

/// How an environment badge draws.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// macOS: a rounded tint, the label as it is.
    Mac,
    /// macOS, in the connection bar's chips: smaller, six at its sides.
    Chip,
    /// The terminal's solid upper-case tag, letter-spaced (the connection
    /// bar's own chip).
    Tracked,
    /// The same tag as an outline in the environment's colour (the bar's
    /// other chips).
    Outlined,
    /// The terminal's tag without letter spacing (the connections list).
    Plain,
}

impl Badge {
    /// Whose label it shows, the label's role, the room at its sides, and
    /// its corner.
    fn shape(self) -> (crate::env::Platform, TextRole, f32, u8) {
        use crate::env::Platform::{Native, Omarchy};
        match self {
            Self::Mac => (Native, TextRole::EnvTag, 7.0, 10),
            Self::Chip => (Native, TextRole::ChipEnv, 6.0, 8),
            Self::Tracked | Self::Outlined => (Omarchy, TextRole::OEnvLabel, 6.0, 2),
            Self::Plain => (Omarchy, TextRole::OBadge, 7.0, 3),
        }
    }
}

/// The width [`env_badge`] takes.
pub fn env_badge_width(
    ui: &egui::Ui,
    env: crate::env::Environment,
    style: Badge,
    look: &Look,
) -> f32 {
    let (platform, role, pad, _) = style.shape();
    role.width(ui.ctx(), look.faces, env.label(platform)) + 2.0 * pad
}

/// An environment badge at `x`, centred on `y`. Returns its width.
#[allow(clippy::too_many_arguments)] // position, what, and how
pub fn env_badge(
    ui: &egui::Ui,
    x: f32,
    y: f32,
    env: crate::env::Environment,
    colors: &crate::env::EnvColors,
    style: Badge,
    look: &Look,
) -> f32 {
    // One point above and below the text.
    let (platform, role, pad, corner) = style.shape();
    let outlined = style == Badge::Outlined;
    let ink = if outlined {
        colors.base()
    } else {
        colors.badge_fg()
    };
    let laid = Text::one(look, role, env.label(platform), ink).layout(ui.ctx());
    let height = laid.height() + 2.0;
    let rect = Rect::from_min_size(
        pos2(x, y - height / 2.0),
        vec2(laid.width() + 2.0 * pad, height),
    );
    let corner = CornerRadius::same(corner);
    if outlined {
        ui.painter().rect_stroke(
            rect,
            corner,
            Stroke::new(1.0, colors.base()),
            StrokeKind::Inside,
        );
    } else {
        ui.painter().rect_filled(rect, corner, colors.badge_bg());
    }
    laid.paint_left(ui.painter(), x + pad, y);
    rect.width()
}

/// Omarchy's bottom line: the keys that work here, and the page's range
/// or, on a SQL editor, the cursor and what the last run did.
fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let palette = app.palette;
    let look = app.look;
    let locale = app.locale;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let read_only = workspace.access == tabletist_db::Access::ReadOnly;
    let editor = super::sql_editor::status_summary(app, tab);
    let on_editor = editor.is_some();
    // The completion list's own key, while it is open with a row to put
    // in. One whose row is about to be inserted is not open any more: the
    // mode line is drawn before the editor inserts it.
    let completing = workspace
        .active_sql_tab()
        .and_then(|sql| sql.completion.as_ref())
        .is_some_and(|list| !list.accept && !list.candidates.is_empty());
    let summary = editor
        .or_else(|| {
            let object = workspace.active_object_tab()?;
            let page = object.page()?;
            let first = object.query.offset + u64::from(!page.rows.is_empty());
            let last = object.query.offset + page.rows.len() as u64;
            let total = object
                .count
                .value
                .map(|count| count.to_string())
                .or_else(|| (!page.has_more).then(|| last.to_string()))
                .unwrap_or_else(|| "?".into());
            let rows = format!(
                "{first}–{last}/{total} · {}",
                crate::ui::format::elapsed(page.elapsed)
            );
            // The columns in view first, while some are out of it.
            let columns = (object.view == ObjectView::Data)
                .then(|| {
                    super::data_view::columns_note(ui.ctx(), workspace, tab, object, &look, locale)
                })
                .flatten();
            Some(match columns {
                Some(columns) => format!("{columns} · {rows}"),
                None => rows,
            })
        })
        .unwrap_or_default();
    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
        .exact_size(31.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let y = rect.top() + 1.0 + 15.0;
            let table_hints = [
                ("j/k", "row", true),
                ("h/l", "col", true),
                ("enter", "inspect", true),
                ("i", "inspector", true),
                ("/", "filter", true),
                ("ctrl+b", "tables", true),
                ("y", "copy", true),
                ("s", "structure", true),
            ];
            // An editor's keys: a table's do nothing on it.
            let words = [
                "run",
                "run all",
                "cancel",
                "leave editor",
                "tables",
                "complete",
            ]
            .map(|word| gettext(locale, word));
            let mut editor_hints: Vec<widgets::Hint<'_>> = vec![
                ("ctrl+enter", &*words[0], true),
                ("ctrl+shift+enter", &*words[1], true),
                ("ctrl+.", &*words[2], true),
                ("esc", &*words[3], true),
                ("ctrl+b", &*words[4], true),
            ];
            // An open completion list: its key leads.
            if completing {
                editor_hints.insert(0, ("tab", &*words[5], true));
            }
            let hints: &[widgets::Hint<'_>] = if on_editor {
                &editor_hints
            } else {
                &table_hints
            };
            let role = TextRole::OSecondary;
            let measure = |text: &str| widgets::measure(ui, label_copy(&look, role, text));
            let summary_width = measure(&summary);
            // Everything 18 apart, 12 in from the ends. What does not fit
            // before the range gives way from the end.
            let gap = 18.0;
            let fit = |limit: f32| {
                (0..=hints.len())
                    .rev()
                    .find(|&count| {
                        rect.left()
                            + 12.0
                            + widgets::key_hints_width(ui, &hints[..count], gap, &look, &palette)
                            <= limit
                    })
                    .unwrap_or(0)
            };
            if on_editor {
                // The toolbar says the transaction is read-only; the line
                // has no editing keys to strike out.
                let shown = fit(rect.right() - 12.0 - summary_width - gap);
                let at = (rect.left() + 12.0, y);
                widgets::key_hints(ui, at, &hints[..shown], gap, &look, &palette);
                widgets::paint_text_right(
                    ui,
                    rect.right() - 12.0,
                    y,
                    Text::one(&look, role, &summary, palette.dim),
                );
                return;
            }
            // A read-only connection's tag ends the keys. Without one,
            // neither it nor the gap before it takes room from them.
            let tag = read_only.then(|| gettext(locale, "read-only"));
            let tag_width = tag.as_ref().map_or(0.0, |tag| measure(tag) + 12.0 + 2.0);
            let tag_room = if tag.is_some() { tag_width + gap } else { 0.0 };
            let limit = rect.right() - 12.0 - summary_width - gap - tag_room;
            let disabled = ["e edit", "o new row", "dd delete", ":w write"];
            let disabled_width = disabled.iter().map(|text| measure(text)).sum::<f32>()
                + 14.0 * (disabled.len() - 1) as f32;
            let mut x = rect.left() + 12.0;
            let shown = fit(limit);
            x += widgets::key_hints(ui, (x, y), &hints[..shown], gap, &look, &palette) + gap;
            let separator = measure("│");
            if shown == hints.len() && x + separator + gap + disabled_width <= limit {
                x += widgets::paint_text(ui, x, y, Text::one(&look, role, "│", palette.outline))
                    + gap;
                // Editing's keys, struck through at half strength.
                let faded = palette.panel.lerp_to_gamma(palette.dim, 0.5);
                for (index, text) in disabled.iter().enumerate() {
                    if index > 0 {
                        x += 14.0;
                    }
                    let width = widgets::paint_text(ui, x, y, Text::one(&look, role, text, faded));
                    ui.painter().hline(
                        egui::Rangef::new(x, x + width),
                        y + 0.5,
                        Stroke::new(1.0, faded),
                    );
                    x += width;
                }
                x += gap;
            }
            if let Some(tag) = &tag {
                let pill = Rect::from_min_size(pos2(x, y - 9.0), vec2(tag_width, 18.0));
                ui.painter().rect_stroke(
                    pill,
                    CornerRadius::same(3),
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
                widgets::paint_text(ui, x + 7.0, y, Text::one(&look, role, tag, palette.dim));
            }
            widgets::paint_text_right(
                ui,
                rect.right() - 12.0,
                y,
                Text::one(&look, role, &summary, palette.dim),
            );
        });
}

/// What the top bar says about TLS, and whether to say it as a warning.
/// `encrypted` is what the session negotiated, once it has connected.
/// `prefer` and `require` do not check the certificate (as in libpq), so
/// anyone on the network path could be the server.
fn tls_status(mode: TlsMode, encrypted: Option<bool>) -> (&'static str, bool) {
    match (mode, encrypted) {
        (TlsMode::Disable, _) | (_, Some(false)) => ("Not encrypted", true),
        (TlsMode::Prefer | TlsMode::Require, _) => ("TLS, not verified", true),
        (TlsMode::VerifyCa, _) => ("TLS, host name not checked", false),
        (TlsMode::VerifyFull, _) => ("TLS verified", false),
    }
}

/// The strip over a tab whose connection was lost: what happened, the way
/// back (Reconnect) and the way out (Disconnect). What was on screen stays
/// under it. While a reconnect is on its way the strip says so.
fn banner(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let name = display_safe(&workspace.name).into_owned();
    // What happened, the plain sentence when it says more, the exact error.
    let (lead, sentence, raw, reconnecting) = match &workspace.status {
        SessionStatus::Connected => return,
        SessionStatus::Connecting { .. } => (
            format!("{} {name}…", say("Reconnecting to")),
            String::new(),
            String::new(),
            true,
        ),
        SessionStatus::Disconnected(error) => {
            // The environment, when the connection has one (matching it is
            // env.rs's alone).
            let env = if workspace.environment == crate::env::Environment::None {
                String::new()
            } else {
                format!(
                    " · {}",
                    workspace.environment.label(crate::env::Platform::Native)
                )
            };
            // A plain loss is what the lead says already.
            let sentence = if matches!(error, tabletist_db::Error::ConnectionLost(_)) {
                String::new()
            } else {
                crate::ui::format::describe_error(locale, error)
            };
            (
                format!("{} {name}{env} {}", say("Connection to"), say("lost.")),
                sentence,
                error.to_string(),
                false,
            )
        }
        SessionStatus::Cancelled => (
            say("Connection cancelled."),
            String::new(),
            String::new(),
            false,
        ),
    };
    let tone = states::Tone::Warning;
    let mut reconnect = false;
    let mut disconnect = false;
    let shown = Frame::new()
        .fill(tone.fill(&look, &palette))
        .inner_margin(Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 12.0;
                let height = if look.terminal { 24.0 } else { 28.0 };
                let (out, back) = (say("Disconnect"), say("Reconnect"));
                let leave = states::button(&out, &look)
                    .label("Disconnect")
                    .salt("lost")
                    .quiet();
                let again =
                    (!reconnecting).then(|| states::button(&back, &look).label("Reconnect"));
                // The buttons keep their room at the right, 8 apart and 12
                // after the text.
                let buttons = again
                    .as_ref()
                    .map_or(0.0, |again| again.width(ui, &look) + 8.0)
                    + leave.width(ui, &look);
                if reconnecting {
                    // The spinner asks for the frames that keep it turning.
                    let (at, _) = ui.allocate_exact_size(vec2(14.0, 14.0), Sense::hover());
                    states::spinner(ui, at, palette.warning, &palette);
                }
                // The text wraps in what is left: a long error must not
                // push the buttons out of the window.
                let room = (ui.available_width() - 12.0 - buttons).max(0.0);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    let strong = TextRole::pick(&look, TextRole::UiBodySemibold, TextRole::OGroup);
                    Text::one(&look, strong, &lead, palette.text)
                        .wrap(room)
                        .layout(ui.ctx())
                        .label(ui);
                    if !sentence.is_empty() {
                        Text::one(&look, widgets::body(&look), &sentence, palette.secondary)
                            .wrap(room)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                    // The exact error when it says more (visible, so
                    // keyboards and screen readers get it).
                    if !raw.is_empty() && raw != sentence {
                        Text::one(&look, widgets::secondary(&look), &raw, palette.secondary)
                            .wrap(room)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    disconnect = leave.show(ui, height, &look, &palette).clicked();
                    if let Some(again) = again {
                        reconnect = again.show(ui, height, &look, &palette).clicked();
                    }
                });
            });
        });
    let strip = shown.response.rect;
    widgets::hline(
        ui,
        strip.x_range(),
        strip.bottom() - 0.5,
        tone.line(&look, &palette),
    );
    if reconnect {
        app.actions.push(Action::Reconnect(tab));
    }
    if disconnect {
        app.actions.push(Action::Disconnect(tab));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_rows_room_goes_to_the_chips_before_the_pills() {
        // Two chips, 40 around texts of 100 and 60, and two pills, all 12
        // apart: 252 for the chips, 406 with both pills.
        let chips = [(40.0, 100.0), (40.0, 60.0)];
        let pills = [80.0, 50.0];
        assert_eq!(
            fit(&chips, &pills, 12.0, 406.0),
            Fit {
                pills: 2,
                cap: 100.0
            }
        );
        // The last pill gives way first, then the other.
        assert_eq!(fit(&chips, &pills, 12.0, 405.0).pills, 1);
        assert_eq!(fit(&chips, &pills, 12.0, 300.0).pills, 0);
        // Then the widest text is cut: 20 short leaves it 80.
        let tight = fit(&chips, &pills, 12.0, 232.0);
        assert_eq!(tight.pills, 0);
        assert!((tight.cap - 80.0).abs() < 0.01, "{tight:?}");
        // Under the narrower text, both are cut alike.
        let tighter = fit(&chips, &pills, 12.0, 192.0);
        assert!((tighter.cap - 50.0).abs() < 0.01, "{tighter:?}");
        // Never under the floor.
        assert_eq!(fit(&chips, &pills, 12.0, 10.0).cap, FLOOR);
    }

    #[test]
    fn chips_that_overflow_slide_to_keep_the_bars_own_whole() {
        // Three chips of 90, 12 apart, in a row of 200: the third ends at 394.
        let row = Rect::from_min_max(pos2(100.0, 0.0), pos2(300.0, 40.0));
        let widths = [90.0, 90.0, 90.0];
        let first = place(&widths, Some(0), 12.0, row, 20.0, 40.0);
        assert_eq!(first[0].left(), 100.0, "the first is whole where it is");
        let last = place(&widths, Some(2), 12.0, row, 20.0, 40.0);
        assert_eq!(last[2].right(), 300.0);
        assert_eq!(last[0].left(), 6.0, "the others slide with it");
        assert_eq!(last[2].center().y, 20.0);
        // A row that holds them all slides nothing.
        let wide = Rect::from_min_max(pos2(100.0, 0.0), pos2(500.0, 40.0));
        assert_eq!(
            place(&widths, Some(2), 12.0, wide, 20.0, 40.0)[0].left(),
            100.0
        );
    }

    #[test]
    fn a_chips_card_says_where_the_connection_points_and_how_it_stands() {
        let locale = crate::i18n::Locale::default();
        let look = Look::macos();
        let row = |name: &str, value: &str| (name.to_owned(), value.to_owned());
        // The fixture: a SQLite file, still connecting.
        let mut workspace = crate::testing::workspace();
        assert_eq!(
            card_rows(&workspace, &look, locale, 0),
            [
                row("File", "/tmp/fixture.db"),
                row("Server", "SQLite"),
                row("Security", "Local · no SSH"),
                row("Status", "Connecting…"),
            ]
        );
        // A server behind a bastion, connected two hours ago.
        let now = 1_790_683_200;
        let (mut spec, _) = tabletist_db::ConnectSpec::from_url(
            "postgres://app@db.example.com:5432/bookshop_production?sslmode=verify-full",
        )
        .unwrap();
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: Some(22),
            user: "deploy".into(),
            auth: tabletist_db::SshAuth::Agent,
        });
        workspace.spec = spec;
        workspace.driver = tabletist_db::Driver::Postgres;
        workspace.status = SessionStatus::Connected;
        workspace.encrypted = true;
        workspace.connected_at = Some(now - 7_200);
        assert_eq!(
            card_rows(&workspace, &look, locale, now),
            [
                row("Host", "db.example.com:5432"),
                row("Database", "bookshop_production"),
                row("User", "app"),
                row("Server", "PostgreSQL"),
                row("Security", "TLS verified · SSH via bastion"),
                row("Connected", "2 h ago · no tabs open"),
            ]
        );
        // The terminal look says its own words in lower case, never a name.
        let terminal = card_rows(&workspace, &Look::omarchy(), locale, now);
        assert_eq!(terminal[0], row("host", "db.example.com:5432"));
        assert_eq!(
            terminal[4],
            row("security", "tls verified · ssh via bastion")
        );
    }

    #[test]
    fn a_chips_card_says_what_a_click_does() {
        let locale = crate::i18n::Locale::default();
        let chip = |own: bool| Chip {
            tab: ConnTabId(1),
            own,
            name: "Fixture".into(),
            env: crate::env::Environment::Dev,
            line: "fixture.db".into(),
            link: Link::Connected,
            number: 1,
            card: Vec::new(),
        };
        let hint = |own: bool, switchable: bool, look: &Look| {
            card_hint(&chip(own), switchable, look, locale)
        };
        let mac = Look::macos();
        assert_eq!(
            hint(false, false, &mac).as_deref(),
            Some("Click to switch to this connection")
        );
        assert_eq!(
            hint(true, true, &mac).as_deref(),
            Some("Click to switch database")
        );
        // The bar's own chip with one database does nothing: no hint.
        assert_eq!(hint(true, false, &mac), None);
        assert_eq!(
            hint(false, true, &Look::omarchy()).as_deref(),
            Some("click to switch to this connection")
        );
    }

    #[test]
    fn the_connections_hint_names_the_key_as_the_look_spells_it() {
        let locale = crate::i18n::Locale::default();
        assert_eq!(connections_hint(&Look::macos(), locale), "Connections · ⌘O");
        assert_eq!(
            connections_hint(&Look::standard(), locale),
            "Connections · Ctrl+O"
        );
        assert_eq!(
            connections_hint(&Look::omarchy(), locale),
            "connections · ctrl+o"
        );
    }

    #[test]
    fn a_failed_connects_title_names_what_was_tried() {
        use tabletist_db::{ConnectSpec, Error, SshStage};
        let (mut spec, _) = ConnectSpec::from_url("postgres://reader@db.example.com/app").unwrap();
        let say = |text: &str| text.to_owned();
        let title = |error: Option<&Error>, spec: &ConnectSpec| failure_title(error, spec, say).1;
        assert_eq!(title(None, &spec), "Connection cancelled");
        assert_eq!(
            title(Some(&Error::Auth("no".into())), &spec),
            "Password rejected for reader"
        );
        assert_eq!(
            title(Some(&Error::Connect("refused".into())), &spec),
            "Can't reach db.example.com:5432"
        );
        assert_eq!(
            title(Some(&Error::Timeout), &spec),
            "Can't reach db.example.com:5432"
        );
        assert_eq!(
            title(Some(&Error::Tls("bad certificate".into())), &spec),
            "TLS or certificate problem"
        );
        spec.user.clear();
        assert_eq!(
            title(Some(&Error::Auth("no".into())), &spec),
            "Login refused"
        );
        spec.ssh = Some(tabletist_db::SshSpec {
            host: "bastion".into(),
            port: None,
            user: String::new(),
            auth: tabletist_db::SshAuth::KeyFile {
                path: "~/.ssh/id_ed25519".into(),
            },
        });
        let tunnel = Error::Ssh {
            stage: SshStage::Connect,
            message: "timed out".into(),
        };
        assert_eq!(title(Some(&tunnel), &spec), "SSH tunnel to bastion failed");
        let file = ConnectSpec::sqlite("/tmp/shop.db");
        assert_eq!(
            title(Some(&Error::Connect("unable to open".into())), &file),
            "Can't open shop.db"
        );
    }
}
