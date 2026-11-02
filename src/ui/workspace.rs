//! A connection tab: the connection bar, what connecting shows, the strip
//! of a lost connection, the sidebar, the object tabs and the open object.

use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, pos2, vec2};
use tabletist_db::{Access, TlsMode};

use crate::app::App;
use crate::format::display_safe;
use crate::i18n::gettext;
use crate::keymap::Command;
use crate::model::{Action, ConnTabId, ObjectView, SessionStatus, TabId};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Region};
use crate::ui::menu;
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
/// least as tall as the window has it: `titlebar` is its height in window
/// points, which egui's `zoom` scales away from its own.
fn top_bar_height(look: &Look, titlebar: f32, zoom: f32) -> f32 {
    bar_height(look).max(titlebar / zoom)
}

/// The line the bar's contents centre on, below its top: under the macOS
/// stripe and over the rule, or the middle of the terminal bar.
pub fn bar_line(look: &Look, titlebar: f32, zoom: f32) -> f32 {
    let height = top_bar_height(look, titlebar, zoom);
    if look.terminal {
        height / 2.0
    } else {
        (STRIPE + height - 1.0) / 2.0
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let look = app.look;
    if let Some(info) = bar_info(app, tab) {
        let paint = super::Paint::of(app);
        let titlebar = (app.titlebar.inset, app.titlebar.height);
        let env_bar = app.settings.env_bar;
        if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
            let asked = top_bar(ui, paint, tab, workspace, &info, env_bar, titlebar);
            app.actions.extend(asked);
        }
    }
    let (_, asked) = super::notices(
        ui,
        super::Paint::of(app),
        app.notice.as_deref(),
        &app.agents_held,
        &app.connections.connections,
    );
    app.actions.extend(asked);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if !workspace.opened() {
        // Nothing of the database to show yet: how connecting goes, or
        // why it failed.
        let asked = opening(ui, super::Paint::of(app), tab, workspace);
        app.actions.extend(asked);
        return;
    }
    if look.terminal {
        object_tabs(app, ui, tab);
        status_line(app, ui, tab);
    }
    if let Some(workspace) = app.workspace(tab) {
        let asked = banner(ui, super::Paint::of(app), tab, workspace);
        app.actions.extend(asked);
    }
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let active = workspace.active_tab;
    // What a lost connection left on screen reads as old. A session closed
    // while idle, or opening again for a use, left the page as it was.
    let stale = !workspace.takes_commands();
    let view = workspace.active_object_tab().map(|object| object.view);
    // The tab the row panel shows a row of: a table's Data view, or a SQL
    // editor with a row of its result selected.
    let row_panel = workspace.row_panel_tab();
    sidebar(app, ui, tab);
    if look.terminal
        && let Some(shown) = row_panel
    {
        inspector(app, ui, tab, shown);
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(app.palette.window))
        .show(ui, |ui| {
            if stale {
                ui.multiply_opacity(states::STALE);
            }
            if !look.terminal {
                object_tabs(app, ui, tab);
                if let Some(shown) = row_panel {
                    inspector(app, ui, tab, shown);
                }
            }
            let sql = app
                .workspace(tab)
                .is_some_and(|workspace| workspace.active_sql_tab().is_some());
            let agents = app
                .workspace(tab)
                .is_some_and(crate::model::Workspace::agents_in_front);
            let history = app
                .workspace(tab)
                .is_some_and(crate::model::Workspace::history_in_front);
            match active {
                Some(sql_tab) if sql => sql_editor(app, ui, tab, sql_tab),
                Some(_) if agents => agents_tab(app, ui, tab),
                Some(_) if history => history_tab(app, ui, tab),
                Some(object_tab) => {
                    header(app, ui, tab, object_tab);
                    if view == Some(ObjectView::Data) {
                        toolbar(app, ui, tab, object_tab);
                    }
                    if !look.terminal {
                        match view {
                            Some(ObjectView::Ddl) => ddl_footer(app, ui, tab, object_tab),
                            _ => footer(app, ui, tab, object_tab),
                        }
                        // Bottom panels stack upwards: the bar of pending
                        // changes stands on the footer.
                        pending_bar(app, ui, tab, object_tab);
                    }
                    // And what a save would run stands on the bar: in the
                    // terminal look, on the status line.
                    review(app, ui, tab, object_tab);
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(app.palette.window))
                        .show(ui, |ui| match view {
                            Some(ObjectView::Structure) => structure(app, ui, tab, object_tab),
                            Some(ObjectView::Ddl) => ddl(app, ui, tab, object_tab),
                            _ => {
                                let open = app
                                    .workspace(tab)
                                    .and_then(|workspace| workspace.object_tab(object_tab))
                                    .is_some_and(super::filter_bar::is_open);
                                if open {
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
                                        let paint = super::Paint::of(app);
                                        if let Some(object) =
                                            app.workspace_mut(tab).and_then(|workspace| {
                                                workspace.object_tab_mut(object_tab)
                                            })
                                        {
                                            let asked =
                                                super::filter_bar::show(ui, paint, tab, object);
                                            app.actions.extend(asked);
                                        }
                                        let rect = ui.max_rect().expand2(vec2(12.0, 8.0));
                                        widgets::hline(
                                            ui,
                                            rect.x_range(),
                                            rect.bottom(),
                                            palette.outline,
                                        );
                                    });
                                }
                                data_grid(app, ui, tab, object_tab)
                            }
                        });
                }
                None => start(app, ui, tab),
            }
        });
}

/// The sidebar, handed its workspace, the keymap, whether the system's
/// schemas are listed and what its Agents entry says: its terminal filter
/// line edits the tree's filter in place.
fn sidebar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let paint = super::Paint::of(app);
    let show_system = app.settings.show_system_schemas;
    let agents = app.agents_here(tab);
    if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
        let asked =
            super::sidebar::show(ui, paint, tab, workspace, &app.keymap, show_system, agents);
        app.actions.extend(asked);
    }
}

/// The start screen, handed its workspace, what it asks of the app and the
/// keymap: its search field edits the workspace's search in place.
fn start(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let paint = super::Paint::of(app);
    let Some(asked) = start_asked(app, tab) else {
        return;
    };
    if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
        let asked = super::start::show(ui, paint, tab, workspace, asked, &app.keymap);
        app.actions.extend(asked);
    }
}

/// What the start screen of `tab` asks of the app: none where the
/// workspace is gone.
fn start_asked(app: &App, tab: ConnTabId) -> Option<super::start::Asked> {
    let workspace = app.workspace(tab)?;
    let recents = app.start_recents(tab);
    let largest = if recents.is_empty() {
        app.start_largest(tab)
    } else {
        Vec::new()
    };
    let ready = match &app.updates.install {
        crate::update::Install::Ready { version, .. } => Some(version.clone()),
        _ => None,
    };
    Some(super::start::Asked {
        info: bar_info(app, tab)?,
        schema: app.start_schema(tab),
        recents,
        largest,
        agents: app.agents_here(tab),
        saved: app.connections.get(&workspace.conn_id).is_some(),
        matches: app.start_matches(tab),
        ready,
        grouped: app.settings.grouped(app.layout()),
    })
}

/// The Agents tab, handed its workspace, the saved connections, the keymap
/// and what it asks of the app.
fn agents_tab(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let paint = super::Paint::of(app);
    let Some(asked) = agents_asked(app, tab) else {
        return;
    };
    if let Some(workspace) = app.workspace(tab) {
        let asked = super::agents::show(
            ui,
            paint,
            tab,
            workspace,
            &app.connections,
            &app.keymap,
            asked,
        );
        app.actions.extend(asked);
    }
}

/// What the Agents tab of `tab` asks of the app: none where the workspace
/// is gone.
fn agents_asked(app: &App, tab: ConnTabId) -> Option<super::agents::Asked> {
    let here = app.agents_here(tab)?;
    let conn = &app.workspace(tab)?.conn_id;
    let saved = &app.connections.connections;
    Some(super::agents::Asked {
        here,
        connection: app.agents_name(tab).unwrap_or_default(),
        allowed: app
            .connections
            .get(conn)
            .map(|saved| app.agents_access(saved)),
        sessions: app.agents.here(conn).into_iter().cloned().collect(),
        elsewhere: app
            .agents
            .elsewhere(conn)
            .into_iter()
            .map(|session| super::agents::Elsewhere::of(saved, session))
            .collect(),
        calls: app.agents.calls_on(conn).cloned().collect(),
        selected: app.agents_selected(tab).map(|session| session.id.clone()),
        log_days: app.settings.mcp_log_days,
    })
}

/// The strip of tabs, handed its workspace and the keymap.
fn object_tabs(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let paint = super::Paint::of(app);
    if let Some(workspace) = app.workspace(tab) {
        let asked = super::object_tabs::show(ui, paint, tab, workspace, &app.keymap);
        app.actions.extend(asked);
    }
}

/// A SQL editor tab, handed its workspace, the settings and the keymap:
/// the editor edits its tab's text in place.
fn sql_editor(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, sql_tab: TabId) {
    let paint = super::Paint::of(app);
    if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
        let asked = super::sql_editor::show(
            ui,
            paint,
            tab,
            sql_tab,
            workspace,
            &app.settings,
            &app.keymap,
        );
        app.actions.extend(asked);
    }
}

/// The History tab, handed its workspace and the name it shows.
fn history_tab(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let paint = super::Paint::of(app);
    let name = app.history_name(tab);
    let keep = app.settings.history_keep;
    if let Some(workspace) = app.workspace(tab) {
        let asked = super::history::show(ui, paint, tab, workspace, &app.keymap, &name, keep);
        app.actions.extend(asked);
    }
}

/// The open object's header, handed its tab and workspace.
fn header(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if let Some(object) = workspace.object_tab(object_tab) {
        let asked = super::data_view::header(ui, paint, tab, workspace, object);
        app.actions.extend(asked);
    }
}

/// The Data view's toolbar, handed its workspace: the terminal's WHERE
/// line edits the tab's clause in place.
fn toolbar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    if let Some(workspace) = app.workspace_mut(tab) {
        let asked = super::data_view::toolbar(ui, paint, tab, object_tab, workspace);
        app.actions.extend(asked);
    }
}

/// The open object's footer, handed its tab, the settings and the keymap.
fn footer(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if let Some(object) = workspace.object_tab(object_tab) {
        let asked = super::data_view::footer(
            ui,
            paint,
            tab,
            workspace,
            object,
            &app.settings,
            &app.keymap,
        );
        app.actions.extend(asked);
    }
}

/// The table's grid, handed its workspace and what its editor reads of
/// the rest: the field on a cell edits its editor's text in place.
fn data_grid(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let hold = app.dialog.is_none();
    let calendar = app.calendar();
    if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
        let around = super::data_view::Around {
            settings: &app.settings,
            keymap: &app.keymap,
            hold,
            calendar,
        };
        let asked = super::data_view::show(ui, paint, tab, object_tab, workspace, around);
        app.actions.extend(asked);
    }
}

/// The row panel, of the tab `shown`, handed its workspace and what its
/// editor reads of the rest: its field edits the editor's text in place.
fn inspector(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, shown: TabId) {
    let paint = super::Paint::of(app);
    let hold = app.dialog.is_none();
    let calendar = app.calendar();
    if let Some(workspace) = crate::app::workspace_in_mut(&mut app.tabs, tab) {
        let around = super::data_view::Around {
            settings: &app.settings,
            keymap: &app.keymap,
            hold,
            calendar,
        };
        let asked = super::row_panel::show(ui, paint, tab, shown, workspace, around);
        app.actions.extend(asked);
    }
}

/// The DDL view's footer, handed its tab.
fn ddl_footer(app: &App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if let Some(object) = workspace.object_tab(object_tab) {
        super::ddl::footer(ui, paint, tab, object, workspace.driver);
    }
}

/// The bar of pending changes, handed its tab and what it asks of the rest.
fn pending_bar(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let shown = app.workspace(tab).and_then(|workspace| {
        let object = workspace
            .object_tab(object_tab)
            .filter(|object| super::pending_bar::shows(object))?;
        Some((workspace, object))
    });
    let asked = match shown {
        Some((workspace, object)) => {
            let asked = super::pending_bar::Asked {
                doubts: super::key_picker::unmatched(workspace, object).len(),
                blocked: app.save_blocked(tab, object_tab),
                lacking: app.lacking(tab, object_tab),
            };
            super::pending_bar::show(ui, paint, tab, object, asked)
        }
        None => {
            super::pending_bar::pass(ui);
            Vec::new()
        }
    };
    app.actions.extend(asked);
}

/// The Review SQL panel, handed its tab.
fn review(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let shown = app
        .workspace(tab)
        .and_then(|workspace| Some((workspace, workspace.object_tab(object_tab)?)));
    let asked = match shown {
        Some((workspace, object)) => {
            let read_only = workspace.access == Access::ReadOnly;
            let whole = || app.review_whole(tab, object_tab);
            super::review::show(ui, paint, tab, object, read_only, whole)
        }
        None => {
            super::review::pass(ui);
            Vec::new()
        }
    };
    app.actions.extend(asked);
}

/// The Structure view, handed its tab and the label the app resolves.
fn structure(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let shown = app
        .workspace(tab)
        .and_then(|workspace| Some((workspace, workspace.object_tab(object_tab)?)));
    let Some((workspace, object)) = shown else {
        return;
    };
    let label = app.structure_label(tab, object_tab);
    let asked = super::structure::show(ui, paint, tab, workspace, object, label, &app.keymap);
    app.actions.extend(asked);
}

/// The DDL view, handed its tab.
fn ddl(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let paint = super::Paint::of(app);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let asked = super::ddl::show(ui, paint, tab, object, workspace.driver);
    app.actions.extend(asked);
}

/// A tab that has not shown its content yet: how connecting goes, or why
/// it failed.
fn opening(
    ui: &mut egui::Ui,
    paint: super::Paint,
    tab: ConnTabId,
    workspace: &crate::model::Workspace,
) -> Vec<Action> {
    let failed = matches!(
        workspace.status,
        SessionStatus::Disconnected(_) | SessionStatus::Cancelled
    );
    egui::CentralPanel::default()
        .frame(Frame::new().fill(paint.palette.window))
        .show(ui, |ui| {
            if failed {
                failure(ui, paint, tab, workspace)
            } else {
                connecting(ui, paint, tab, workspace)
            }
        })
        .inner
}

/// The steps of a connect, the one under way with its time, and, for a
/// connect with nothing to lose, the button that gives up: in the middle
/// of the tab, or in the terminal look from its top left.
fn connecting(
    ui: &mut egui::Ui,
    paint: super::Paint,
    tab: ConnTabId,
    workspace: &crate::model::Workspace,
) -> Vec<Action> {
    use tabletist_db::Driver;
    let super::Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let spec = &workspace.spec;
    let (reach, target) = match spec.driver {
        Driver::Sqlite => ("Open", spec.summary()),
        Driver::Postgres | Driver::MySql => {
            let target = format!("{}:{}", spec.host, spec.port);
            let target = match &spec.ssh {
                Some(ssh) => format!("{target} {} {}", say("via"), ssh.host),
                None => target,
            };
            ("Connect to", target)
        }
    };
    let connected = matches!(workspace.status, SessionStatus::Connected);
    let (reach, load) = (say(reach), say("Load schema"));
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
    let cancel_key = crate::ui::keys::written(ui.ctx(), &look, Command::CancelConnecting);
    let cancel = workspace
        .can_give_up()
        .then(|| states::key_button(&name, &cancel_key, &look).label("Cancel connecting"));
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
    let mut actions = Vec::new();
    if let Some(cancel) = cancel
        && cancel.show_at(ui, button, &look, &palette).clicked()
    {
        actions.push(Action::Disconnect(tab));
    }
    actions
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
        Error::Connect(_) | Error::Timeout => match spec.driver {
            Driver::Sqlite => (
                Icon::CircleAlert,
                format!("{} {}", say("Can't open"), spec.summary()),
            ),
            Driver::Postgres | Driver::MySql => (
                Icon::WifiOff,
                format!("{} {}:{}", say("Can't reach"), spec.host, spec.port),
            ),
        },
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
fn failure(
    ui: &mut egui::Ui,
    paint: super::Paint,
    tab: ConnTabId,
    workspace: &crate::model::Workspace,
) -> Vec<Action> {
    use tabletist_db::Error;
    let super::Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let error = match &workspace.status {
        SessionStatus::Disconnected(error) => Some(error),
        _ => None,
    };
    let (icon, title) = failure_title(error, &workspace.spec, say);
    let sentence = error
        .map(|error| crate::format::describe_error(locale, error))
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
    let mut actions = Vec::new();
    if retry {
        actions.push(Action::Reconnect(tab));
    }
    if edit {
        actions.push(Action::EditConnection(conn));
    }
    actions
}

/// What the connection bar says: the open connections, and more about the
/// one the bar belongs to.
pub(super) struct BarInfo {
    pub(super) env: crate::env::Environment,
    /// Whether the bar's own connection opens read-only.
    pub(super) read_only: bool,
    pub(super) database: String,
    pub(super) databases: Vec<String>,
    tls: Option<(&'static str, Tone)>,
    ssh_host: Option<String>,
    chips: Vec<Chip>,
    /// The key that shows the connections, and the one that closes this
    /// one, as the look writes them.
    connections_key: crate::keymap::Written,
    close_key: crate::keymap::Written,
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
    /// Closed while idle: it connects again when it is used.
    Idle,
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
    /// The key that goes to it, by its place among the open connections,
    /// where that place has one: the first nine do.
    key: crate::keymap::Written,
    /// What its card says: a name and a value per line.
    card: Vec<(String, String)>,
}

/// Whether a connection's traffic leaves this machine: a file's never does.
fn is_remote(workspace: &crate::model::Workspace) -> bool {
    use tabletist_db::Driver;
    match workspace.driver {
        Driver::Sqlite => false,
        Driver::Postgres | Driver::MySql => !crate::model::is_local_host(&workspace.spec.host),
    }
}

pub(super) fn bar_info(app: &App, tab: ConnTabId) -> Option<BarInfo> {
    use tabletist_db::Driver;
    let workspace = app.workspace(tab)?;
    let spec = &workspace.spec;
    // Local traffic never crosses a network, so its TLS says nothing; a
    // remote connection always says how far it can be trusted.
    let tls = is_remote(workspace).then(|| {
        let encrypted =
            matches!(workspace.status, SessionStatus::Connected).then_some(workspace.encrypted);
        let (text, warn) = tls_status(spec.effective_tls(), encrypted);
        (text, if warn { Tone::Warn } else { Tone::Good })
    });
    Some(BarInfo {
        env: workspace.environment,
        read_only: workspace.access == tabletist_db::Access::ReadOnly,
        database: match workspace.driver {
            Driver::Sqlite => String::new(),
            Driver::Postgres | Driver::MySql => spec.database.clone(),
        },
        databases: workspace.databases.value.clone().unwrap_or_default(),
        tls,
        ssh_host: spec.ssh.as_ref().map(|ssh| ssh.host.clone()),
        chips: chips(app, tab),
        connections_key: app.keymap.key(app.layout(), Command::Connections),
        close_key: app.keymap.key(app.layout(), Command::CloseConnection),
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
                SessionStatus::Idle => (Link::Idle, Some("Disconnected while idle")),
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
                key: {
                    let window = Command::GoToConnectionWindow;
                    app.keymap.digit_label(app.layout(), window, index + 1)
                },
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
    use tabletist_db::Driver;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let spec = &workspace.spec;
    let mut rows = Vec::new();
    match workspace.driver {
        Driver::Sqlite => {
            let path = spec
                .sqlite_path
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            rows.push((say("File"), path));
        }
        Driver::Postgres | Driver::MySql => {
            rows.push((say("Host"), format!("{}:{}", spec.host, spec.port)));
            if !spec.database.is_empty() {
                rows.push((say("Database"), display_safe(&spec.database).into_owned()));
            }
            if !spec.user.is_empty() {
                rows.push((say("User"), spec.user.clone()));
            }
        }
    }
    let server = workspace
        .server_version
        .value
        .clone()
        .unwrap_or_else(|| workspace.driver.label().to_owned());
    rows.push((say("Server"), server));
    let connected = matches!(workspace.status, SessionStatus::Connected);
    let tls = if is_remote(workspace) {
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
        SessionStatus::Idle => (say("Status"), say("Disconnected while idle")),
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
        if !chip.key.is_empty() {
            Text::one(look, small, &chip.key, palette.dim)
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
pub fn target(workspace: &crate::model::Workspace) -> String {
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
        Link::Cancelled | Link::Idle => palette.dim,
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

/// The id of the pop-up of the server's other databases: the bar's own
/// chip's, or the start screen's button's. A key opens one with
/// [`egui::Popup::open_id`].
pub fn database_menu_id(tab: ConnTabId, start: bool) -> egui::Id {
    egui::Id::new(("database-menu", tab.0, start))
}

/// The pop-up of the server's other databases, under the bar's own chip
/// or the start screen's button (`response`), with `id` its own.
pub(super) fn database_menu(
    response: &egui::Response,
    id: egui::Id,
    tab: ConnTabId,
    info: &BarInfo,
    (look, palette): (&Look, &Palette),
    actions: &mut Vec<Action>,
) {
    let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
    menu::under_id(response, id, response.rect.width(), look, palette, |ui| {
        for database in &info.databases {
            // Database names come from the server: nothing hidden.
            let name = display_safe(database);
            let item = menu::Item::choice(&name, *database == info.database).role(role);
            if item.show(ui, look, palette).clicked() && *database != info.database {
                actions.push(Action::SwitchDatabase {
                    tab,
                    database: database.clone(),
                });
            }
        }
    });
}

/// The connection bar: the connection's environment colour behind the open
/// connections, and the way out. `info` is what it shows of the workspace,
/// `env_bar` how the settings tint it, and `titlebar` the title bar's inset
/// and height in window points. It takes the workspace's one-time ask for
/// its own chip's menu, and returns its actions.
fn top_bar(
    ui: &mut egui::Ui,
    paint: super::Paint,
    tab: ConnTabId,
    workspace: &mut crate::model::Workspace,
    info: &BarInfo,
    env_bar: crate::settings::EnvBar,
    (inset, titlebar): (f32, f32),
) -> Vec<Action> {
    // Quick open's Switch database asks for the chip's menu, as its key
    // does: the menu opens as the chip is drawn.
    if std::mem::take(&mut workspace.database_menu_asked) {
        egui::Popup::open_id(ui.ctx(), database_menu_id(tab, false));
    }
    let super::Paint {
        locale,
        palette,
        look,
    } = paint;
    let platform = crate::env::Platform::of(&look);
    let env = crate::env::env_colors(info.env, platform, &palette);
    // The bar's fill and rule as the settings have them, and the lines
    // over it with them.
    let env = env.on_bar(info.env, env_bar, platform, &palette);
    let (tint, border) = (env.bar_bg(), env.bar_border());
    // macOS: the bar is the title bar, beside the window buttons.
    let zoom = ui.ctx().zoom_factor();
    let inset = inset / zoom;
    let height = top_bar_height(&look, titlebar, zoom);
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
                    info,
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
                    info,
                    &env,
                    &look,
                    &palette,
                    &mut actions,
                    locale,
                );
            }
        });
    actions
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
            .on_hover_text(connections_hint(look, locale, &info.connections_key))
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
                let id = database_menu_id(tab, false);
                database_menu(&response, id, tab, info, (look, palette), actions);
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
            .on_hover_text(connections_hint(look, locale, &info.connections_key))
            .clicked()
        {
            actions.push(Action::ShowConnections);
        }
        x = button.right() + 12.0;
    }
    // The way out, a muted note that also answers a click.
    let note = format!("{} {}", info.close_key, gettext(locale, "disconnect"));
    let note = note.as_str();
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
                let id = database_menu_id(tab, false);
                database_menu(&response, id, tab, info, (look, palette), actions);
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
fn connections_hint(
    look: &Look,
    locale: crate::i18n::Locale,
    key: &crate::keymap::Written,
) -> String {
    // The key is written by the keymap, in the look's own case: only the
    // name is the look's to lower.
    let name = look.label(&gettext(locale, "Connections"));
    format!("{name} · {key}")
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

/// Why a cell cannot be edited, as the terminal's mode line says it: in the
/// look's lower case, but for the table's name, which is its own.
pub fn lock_line(
    lock: crate::edit::Lock,
    table: &str,
    look: &Look,
    locale: crate::i18n::Locale,
) -> String {
    let text = super::cell_editor::lock_text(lock, table, locale);
    match text.strip_prefix(table) {
        Some(rest) if !table.is_empty() => format!("{table}{}", look.label(rest)),
        _ => look.label(&text),
    }
}

/// What editing adds to a table's line in the terminal look.
#[derive(Default)]
struct Editing {
    /// `i` would open an editor on the selected cell.
    can_edit: bool,
    /// Insert mode: the column being edited and its type.
    insert: Option<String>,
    /// A visual selection of rows is on, with this many rows.
    visual: Option<usize>,
    /// Where Tab moves on to: the next cell from an editor on a cell, the
    /// next field from one in the row panel. Nowhere from the large
    /// editor, in either place: Tab is its text's own.
    walks: Option<&'static str>,
    /// Insert mode is in a foreign key cell's picker: the line names the
    /// list's keys in place of the editor's.
    picking: bool,
    /// What the picker's options are labelled by, and where in the words
    /// the column's name stands: "label name · automatic".
    label: Option<(String, Option<std::ops::Range<usize>>)>,
    /// The chooser of that label is open and has the keys: what it is
    /// called, for the line to say in place of the column and its keys.
    choosing: Option<String>,
    /// The picker's table has something to choose a label from: the
    /// chooser's key stands among the list's.
    labels: bool,
    /// What the picker says of its label for a while, in place of the
    /// editor's keys, and in which tone: that it changed (a success),
    /// that the column chosen is gone (a warning), or that there is
    /// nothing to choose (plain words).
    told: Option<(Option<states::Tone>, String)>,
    /// How much is pending, while anything is: "3 pending · 2 rows".
    pending: Option<String>,
    /// The rows to add, while there are any: "+1 new".
    added: Option<String>,
    /// `o` would add a row.
    can_add: bool,
    /// The rows to delete, while there are any: "-1 to delete".
    deleted: Option<String>,
    /// `dd` would delete the row under the cursor, drop it or restore it.
    can_delete: bool,
    /// The cells to fix and the ones a save failed on: "1 error".
    errors: Option<String>,
    /// What the tab's new rows still need, for insert mode, which has no
    /// room for why a save waits: "publisher_id required".
    needs: Option<String>,
    /// Why the cell asked for is locked, what the last save came to, or
    /// why what is pending cannot be saved.
    said: Option<Said>,
}

/// One thing the line says besides its keys and its counts.
struct Said {
    /// What leads it and in which tone: the first of these marks that the
    /// look's font can draw.
    mark: Option<(&'static [&'static str], states::Tone)>,
    text: String,
    /// What follows a failure, dimmed: where there is room for it.
    tail: Option<String>,
    /// The colour of the text itself.
    color: egui::Color32,
}

/// The mark of a save that wrote. The second for a face without the first:
/// the look's font is the desktop's own, and the bundled one lacks it.
const WROTE: &[&str] = &["✓", "√"];
/// The mark of a save that wrote nothing, with the same care.
const FAILED: &[&str] = &["✗", "✕", "x"];

/// The first of `marks` that `role`'s font has a glyph for.
pub(super) fn drawable(
    ui: &egui::Ui,
    role: TextRole,
    look: &Look,
    marks: &[&'static str],
) -> Option<&'static str> {
    let font = role.font_id(look.faces);
    let has = |mark: &str| ui.ctx().fonts_mut(|fonts| fonts.has_glyphs(&font, mark));
    marks.iter().copied().find(|mark| has(mark))
}

/// What editing adds to the line of the table the workspace of `tab`
/// shows.
fn editing_status(app: &App, tab: ConnTabId) -> Editing {
    use super::pending_bar::{block_text, counted, note_line, written_text};
    use crate::edit::Note;
    let (palette, look, locale) = (&app.palette, &app.look, app.locale);
    let Some(workspace) = app.workspace(tab) else {
        return Editing::default();
    };
    let Some(object) = workspace.active_object_tab() else {
        return Editing::default();
    };
    let say = |text: &'static str| gettext(locale, text);
    let edits = &object.edits;
    // The keys that edit are the grid's: the Structure view has none.
    let data = object.view == ObjectView::Data;
    let can_edit = data
        && crate::edit::Table::of(workspace, object)
            .zip(object.selection)
            .is_some_and(|(table, cell)| table.lock(cell).is_none());
    let insert = edits.editor.as_ref().filter(|_| data).and_then(|editor| {
        let column = object.page()?.columns.get(editor.cell.col)?;
        // The type as the grid's header names it.
        let kind = crate::format::type_label(&column.type_name, column.kind);
        Some(format!(
            "{} · {}",
            display_safe(&column.name),
            display_safe(&kind)
        ))
    });
    let counts = edits.counts();
    let pending = (counts.changes > 0).then(|| {
        format!(
            "{} {} · {}",
            counts.changes,
            say("pending"),
            counted(locale, counts.rows, "row", "rows")
        )
    });
    let added = (counts.added > 0).then(|| format!("+{} {}", counts.added, say("new")));
    let deleted = (counts.deleted > 0).then(|| format!("-{} {}", counts.deleted, say("to delete")));
    let can_delete = data
        && object.selection.is_some_and(|cell| {
            crate::edit::Table::of(workspace, object)
                .is_some_and(|table| table.no_delete(cell.row).is_none())
        });
    let can_add = data
        && crate::edit::Table::of(workspace, object).is_some_and(|table| table.no_rows().is_none());
    // The rows of a visual selection, while one is on and the rows show.
    let visual = (data && object.visual()).then(|| object.selected_rows().len());
    let errors = counts.to_fix + counts.failed;
    let errors = (errors > 0).then(|| counted(locale, errors, "error", "errors"));
    // What a new row still needs, by its column: said while a value is
    // typed too.
    let needs = {
        let lacking = app.lacking(tab, object.id);
        match lacking.as_slice() {
            [] => None,
            [one] => Some(format!("{one} {}", look.label(&say("required")))),
            more => Some(look.label(&format!("{} {}", more.len(), say("values required")))),
        }
    };
    // What a save came to stands a step back from the keys and the counts.
    let back = palette.secondary;
    // Why what is pending cannot be saved, as the other looks say it on
    // their disabled Save. A save that is running is said by its tab.
    let blocked = edits
        .pending()
        .then(|| app.save_blocked(tab, object.id))
        .flatten()
        .filter(|block| *block != crate::model::SaveBlock::Saving)
        .map(|block| match block {
            // What a new row still needs: in the danger tone, as the
            // row's own star is.
            crate::model::SaveBlock::Required => Said {
                mark: None,
                text: needs.clone().unwrap_or_default(),
                tail: None,
                color: states::Tone::Danger.color(palette),
            },
            block => Said {
                mark: None,
                text: look.label(&block_text(block, counts.to_fix, locale)),
                tail: None,
                color: back,
            },
        });
    let said = if let Some(line) = &workspace.command_error {
        // What the `:` prompt was given, until the next key.
        let line = crate::format::capped(line);
        Some(Said {
            mark: None,
            text: format!("{} {}", say("not a command:"), display_safe(&line)),
            tail: None,
            color: states::Tone::Danger.color(palette),
        })
    } else if let Some((_, name, why)) = workspace
        .label_refused
        .as_ref()
        .filter(|(of, _, _)| *of == object.id)
    {
        // A column that was named for this table's label and not taken,
        // with why, until the next key.
        let table = &object.object.name;
        Some(Said {
            mark: None,
            text: super::structure::refused_text(name, *why, table, look, locale),
            tail: None,
            color: states::Tone::Danger.color(palette),
        })
    } else if workspace.review_refused && !edits.pending() {
        // `:diff` found nothing to show. No mistake of the typing: it
        // reads as the keys do, until the next key. Never over a table
        // that has something pending: another tab's, shown by a click.
        Some(Said {
            mark: None,
            text: look.label(&say("nothing pending")),
            tail: None,
            color: palette.text,
        })
    } else if workspace.save_refused && blocked.is_some() {
        // A save was just asked for: why it was not made comes first,
        // until the next key.
        blocked
    } else if let Some((_, lock)) = edits.why.filter(|_| data) {
        // In place of the note the other looks hang on the cell. It was
        // asked for a moment ago: it reads as the keys do.
        let table = display_safe(&object.object.name);
        let text = lock_line(lock, &table, look, locale);
        (!text.is_empty()).then_some(Said {
            mark: None,
            text,
            tail: None,
            color: palette.text,
        })
    } else if let Some(note) = &edits.note {
        // The row's key and the database's words as they are: the look's
        // lower case is for the app's own.
        let text = note_line(note, object, look, locale);
        Some(match note {
            Note::Conflict { .. } => Said {
                mark: Some((&["≠"], states::Tone::Warning)),
                text: format!("{} {text}", say("conflict")),
                tail: None,
                color: back,
            },
            Note::Failed { .. } => Said {
                mark: Some((FAILED, states::Tone::Danger)),
                text: format!("{} {text}", say("failed")),
                tail: Some(format!(
                    "{} · {}",
                    say("rolled back"),
                    say("cells stay pending in red")
                )),
                color: back,
            },
            // The banner says that everything rolled back: the tail says
            // what is left on screen.
            Note::FailedDelete { .. } => Said {
                mark: Some((FAILED, states::Tone::Danger)),
                text,
                tail: Some(say("the row stays marked").into_owned()),
                color: back,
            },
            // Of a save that wrote.
            Note::AlreadyDeleted(_) => Said {
                mark: Some((WROTE, states::Tone::Success)),
                text,
                tail: None,
                color: back,
            },
            _ => Said {
                mark: Some((FAILED, states::Tone::Danger)),
                text,
                tail: None,
                color: back,
            },
        })
    } else {
        // Until the next edit or page, as the other looks' footer has it.
        // With something pending, why it cannot be saved, while it cannot.
        let saved = edits.saved.as_ref().filter(|_| !edits.holds());
        let saved = saved.map(|saved| Said {
            mark: Some((WROTE, states::Tone::Success)),
            text: look.label(&written_text(saved, locale)),
            tail: None,
            color: back,
        });
        saved.or(blocked)
    };
    let walks = edits.editor.as_ref().and_then(|editor| {
        let panel = editor.place == crate::edit::EditorPlace::Panel;
        (!editor.large).then_some(if panel { "next field" } else { "next cell" })
    });
    let picking = edits.editor.as_ref().is_some_and(|editor| {
        // A search that failed leaves the plain text editor, keys and all.
        !editor.large
            && editor
                .picker
                .as_ref()
                .is_some_and(|picker| picker.failed.is_none())
    });
    // The list has no foot in this look: the line says what labels it.
    let picker = edits
        .editor
        .as_ref()
        .filter(|_| picking)
        .and_then(|editor| editor.picker.as_ref());
    let label =
        picker.and_then(|picker| super::key_picker::label_line(&picker.label, look, locale));
    // Not where the table is its key alone: the key opens nothing there.
    let labels = picker.is_some_and(|picker| {
        use crate::key_picker::Columns;
        use crate::labels::Source;
        match &picker.label {
            Columns::Waiting => true,
            Columns::Known(label) => match label.source {
                Source::Automatic | Source::User => true,
                Source::KeysOnly => false,
            },
        }
    });
    let choosing = picker
        .filter(|picker| picker.chooser.is_some())
        .map(|picker| super::label_chooser::title(&picker.target.name, look, locale));
    // One thing at a time: what was just asked for, what is gone, what
    // changed.
    let told = picker.and_then(|picker| {
        use super::key_picker::{changed_line, gone_text, nothing_text};
        use crate::key_picker::SAID;
        if picker.nothing.is_some_and(|at| at.elapsed() < SAID) {
            let said = nothing_text(&picker.target.name, look, locale);
            return Some((None, said));
        }
        if picker
            .gone
            .as_ref()
            .is_some_and(|(_, at)| at.elapsed() < SAID)
        {
            let gone = gone_text(picker, look, locale)?;
            return Some((Some(states::Tone::Warning), gone.line));
        }
        let said = changed_line(picker, look, locale)?;
        Some((Some(states::Tone::Success), said))
    });
    Editing {
        can_edit,
        insert,
        walks,
        picking,
        label,
        choosing,
        labels,
        told,
        pending,
        added,
        can_add,
        deleted,
        can_delete,
        visual,
        errors,
        needs,
        said,
    }
}

/// The height of the terminal look's status line, and of what takes its
/// place: the `:` prompt, the question about a run that changes every row.
pub(crate) const STATUS_LINE: f32 = 31.0;

/// The field of the terminal's `:` prompt in the tab `tab`.
fn command_id(tab: ConnTabId) -> egui::Id {
    egui::Id::new(("command-line", tab.0))
}

/// The status line while the `:` prompt is open: the colon, the line being
/// typed, and at the right what is pending. Enter runs the line; Esc, and
/// the keyboard going anywhere else, close the prompt. It edits the
/// prompt's text and takes its focus flag, and nothing else.
fn command_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (palette, look, locale) = (app.palette, app.look, app.locale);
    let editing = editing_status(app, tab);
    let Some(workspace) = app.workspace_mut(tab) else {
        return;
    };
    let focus = std::mem::take(&mut workspace.focus_command);
    let Some(line) = workspace.command.as_mut() else {
        return;
    };
    let mut actions = Vec::new();
    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
        .exact_size(STATUS_LINE)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let y = rect.top() + 1.0 + 15.0;
            let role = TextRole::OSecondary;
            let measure = |text: &str| widgets::measure(ui, label_copy(&look, role, text));
            // What is pending keeps its place on the line: at its end.
            let mut right = rect.right() - 12.0;
            for (text, tone) in [
                (&editing.errors, states::Tone::Danger),
                (&editing.deleted, states::Tone::Danger),
                (&editing.pending, states::Tone::Warning),
                (&editing.added, states::Tone::Success),
            ] {
                if let Some(text) = text {
                    let x = right - measure(text);
                    let text = Text::one(&look, role, text, tone.color(&palette));
                    right = x - 18.0;
                    widgets::paint_label(ui, x, y, text);
                }
            }
            let mut x = rect.left() + 12.0;
            x += widgets::paint_text(ui, x, y, Text::one(&look, role, ":", palette.text));
            let height = role.row_height(ui.ctx(), look.faces);
            let field = Rect::from_min_max(
                pos2(x, y - height / 2.0),
                pos2(right.max(x), y + height / 2.0),
            );
            let id = command_id(tab);
            if focus {
                // Before the field is added: what this frame types is its
                // text already.
                ui.memory_mut(|memory| memory.request_focus(id));
            }
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field));
            let mut layouter = crate::typography::layouter(&look, role, palette.text);
            let response = child.add(
                egui::TextEdit::singleline(line)
                    .id(id)
                    .font(role.font_id(look.faces))
                    .frame(Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(field.width())
                    .layouter(&mut layouter),
            );
            let name = gettext(locale, "Command");
            response.widget_info(|| {
                egui::WidgetInfo::labeled(egui::WidgetType::TextEdit, true, &*name)
            });
            // A line of the bar, not a box: its caret says where the
            // keyboard is.
            focus::hint(ui, &response, field, focus::Ring::Own);
            // Enter gives the keyboard up, and runs the line. Without the
            // keyboard for any other reason the prompt is closed: it is
            // open only while it is typed into.
            let enter = ui.input(|input| input.key_pressed(egui::Key::Enter));
            let escape = ui.input(|input| input.key_pressed(egui::Key::Escape));
            if response.lost_focus() && enter {
                actions.push(Action::RunCommand(tab));
            } else if response.lost_focus() && escape {
                actions.push(Action::EscapeCommand(tab));
            } else if !response.has_focus() {
                actions.push(Action::CloseCommand(tab));
            }
        });
    app.actions.extend(actions);
}

/// Omarchy's bottom line: the keys that work here, and the page's range
/// or, on a SQL editor, the cursor and what the last run did. On a table it
/// says what is pending too, and while a cell is edited it is the mode
/// line: `-- INSERT --`, the column, the counts and the keys that leave.
/// The `:` prompt takes the whole of it while it is open.
fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let palette = app.palette;
    let look = app.look;
    let locale = app.locale;
    // The line's keys, as the keymap writes them for the look.
    let key = |command: Command| app.keymap.key(app.layout(), command);
    let cancel_key = key(Command::CancelQuery);
    let (rows_key, cols_key) = (key(Command::MoveRow), key(Command::MoveColumn));
    let (inspect_key, edit_key) = (key(Command::OpenInspector), key(Command::EditCell));
    let (add_key, delete_key) = (key(Command::AddRow), key(Command::DeleteRow));
    let (filter_key, sidebar_key) = (key(Command::WhereFilter), key(Command::ToggleSidebar));
    let (copy_key, save_key) = (key(Command::CopyCells), key(Command::SaveChanges));
    let review_key = key(Command::ReviewSql);
    let (agents_copy_key, agents_on_key) =
        (key(Command::CopyAgentCommand), key(Command::TurnAgentsOn));
    let (agents_allow_key, agents_settings_key) =
        (key(Command::AllowAgents), key(Command::ShowAgentSettings));
    let (agents_move_key, agents_stop_key, agents_here_key) = (
        key(Command::MoveInAgentSessions),
        key(Command::StopAgentSession),
        key(Command::StopAgentsHere),
    );
    // The Agents tab in front: its keys, by the state it shows.
    let agents = app
        .workspace(tab)
        .filter(|workspace| workspace.agents_in_front())
        .and_then(|_| app.agents_here(tab));
    // The tab's own `l`, not the prompt's `:agents log`.
    let (agents_log_key, agents_scope_key, agents_filter_key) = (
        app.keymap.key_in(
            app.layout(),
            Command::ShowAgentsLog,
            crate::keymap::Scope::Agents,
        ),
        key(Command::ToggleAgentsLogScope),
        key(Command::FilterAgentsLog),
    );
    let (agents_details_key, agents_open_key, agents_export_key) = (
        key(Command::ShowAgentsLogDetails),
        key(Command::OpenAgentsLogCall),
        key(Command::ExportAgentsLog),
    );
    // Its Log view, while it shows: whether every connection's, whether a
    // call is selected, and whether that call's statement opens (it has
    // one, on a connection still saved).
    let agents_log = app.workspace(tab).and_then(|workspace| {
        workspace.tabs.iter().find_map(|open| match open {
            crate::model::Tab::Agents(agents) if agents.view == crate::model::AgentsView::Log => {
                let log = &agents.log;
                let call = log
                    .selected
                    .and_then(|id| log.calls.iter().find(|logged| logged.id == id));
                let opens = call.is_some_and(|logged| {
                    logged.entry.sql.is_some()
                        && logged.entry.connection_id.as_ref().is_some_and(|id| {
                            app.connections
                                .get(&crate::connections::ConnectionId(id.clone()))
                                .is_some()
                        })
                });
                Some((log.all, call.is_some(), opens, log.total > 0))
            }
            _ => None,
        })
    });
    // Whether its connection is saved: one saved no more has nothing to
    // allow.
    let agents_saved = app
        .workspace(tab)
        .is_some_and(|workspace| app.connections.get(&workspace.conn_id).is_some());
    // Whether its list has sessions, elsewhere ones too.
    let agents_listed = app
        .workspace(tab)
        .is_some_and(|workspace| !app.agents.listed(&workspace.conn_id).is_empty());
    let (run_key, run_all_key) = (key(Command::RunStatement), key(Command::RunAll));
    let (leave_key, complete_key) = (key(Command::LeaveEditor), key(Command::InsertCompletion));
    let (keep_key, next_key) = (key(Command::KeepEdit), key(Command::CommitNext));
    let (visual_delete_key, leave_visual_key) =
        (key(Command::DeleteSelection), key(Command::LeaveVisual));
    // The keys of a foreign key's list, while one is open under the field.
    let pick_keys = app
        .keymap
        .together(app.layout(), &[Command::NextPick, Command::PreviousPick]);
    let (set_key, close_key) = (key(Command::CommitDown), key(Command::ClosePicker));
    // The keys of the chooser of that list's label, while it is open: the
    // pairs that move in it, each as one hint (`j/k`, `ctrl+n/p`). The
    // arrows are not said.
    let chooser_moves: Vec<crate::keymap::Written> = (0..2)
        .filter_map(|nth| {
            let moves = (Command::NextLabel, Command::PreviousLabel);
            let scope = crate::keymap::Scope::Prompt;
            app.keymap.pair(app.layout(), moves, scope, nth)
        })
        .collect();
    // The key that opens it from the list.
    let label_key = key(Command::ChangeLabelColumn);
    let (choose_key, back_key) = (key(Command::ChooseLabel), key(Command::CloseLabelChooser));
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    if workspace.command.is_some() {
        command_line(app, ui, tab);
        return;
    }
    // No tab: the start screen's keys, and that no table is open.
    if workspace.active_tab.is_none() {
        super::start::status_line(ui, super::Paint::of(app), tab, &app.keymap);
        return;
    }
    let read_only = workspace.access == tabletist_db::Access::ReadOnly;
    let editor = super::sql_editor::status_summary(workspace, app.locale);
    let on_editor = editor.is_some();
    // Vim's mode, where the editor has the keyboard: the line starts with
    // it, as vim's does, and out of normal mode Esc goes to normal mode.
    let vim_mode = super::sql_editor::vim_mode(ui.ctx(), &app.look, tab, workspace, &app.settings);
    let to_normal = vim_mode.is_some_and(|mode| mode != crate::vim::Mode::Normal);
    // The Structure view of the table in front shows: the line names
    // that view's keys in the rows' place. The keys that choose what
    // labels the table's rows are named where there is anything to
    // choose: not for a view, nor for a table of its key alone.
    let structure = workspace
        .active_object_tab()
        .filter(|object| object.view == ObjectView::Structure);
    let labels = structure
        .and_then(|object| app.structure_label(tab, object.id))
        .is_some_and(|label| match label.source {
            crate::labels::Source::Automatic | crate::labels::Source::User => true,
            crate::labels::Source::KeysOnly => false,
        });
    let (column_key, use_key) = (key(Command::MoveInStructure), key(Command::UseAsLabel));
    let (chooser_key, automatic_key) =
        (key(Command::OpenLabelChooser), key(Command::LabelAutomatic));
    // The DDL view of the table in front: its line names that view's keys,
    // and says how long the text is in the place of the rows' range.
    let ddl = workspace
        .active_object_tab()
        .filter(|object| object.view == ObjectView::Ddl);
    let ddl_keys = [
        key(Command::MoveInDdl),
        key(Command::SelectDdlLines),
        key(Command::CopyDdl),
        key(Command::OpenDdlInEditor),
        app.keymap.together(
            app.layout(),
            &[Command::NextDdlSection, Command::PreviousDdlSection],
        ),
        key(Command::ShowRows),
        key(Command::ShowStructure),
        key(Command::LeaveDdlVisual),
    ];
    // The DDL view's text shows (not its read, nor its error), and the
    // terminal look's selection of its lines is on.
    let ddl_text = ddl.is_some_and(|object| object.ddl_shown().is_some());
    let ddl_visual = ddl_text && ddl.is_some_and(|object| object.ddl_visual.is_some());
    // The completion list's own key, while it is open with a row to put
    // in. One whose row is about to be inserted is not open any more: the
    // mode line is drawn before the editor inserts it.
    let completing = workspace
        .active_sql_tab()
        .and_then(|sql| sql.completion.as_ref())
        .is_some_and(|list| !list.accept && !list.candidates.is_empty());
    // "2 lines selected · 10/27": the selection, then the cursor's line of
    // all of them, in the place of the rows' range.
    let lines = ddl.and_then(|object| {
        object.ddl_shown()?;
        let at = format!("{}/{}", object.ddl_cursor + 1, object.ddl_lines());
        Some(match object.ddl_selected() {
            Some(selected) => {
                let count = u32::try_from(selected.len()).unwrap_or(u32::MAX);
                let said =
                    crate::i18n::ngettext(locale, "{n} line selected", "{n} lines selected", count)
                        .replace("{n}", &selected.len().to_string());
                format!("{said} · {at}")
            }
            None => at,
        })
    });
    let summary = editor
        .or(lines)
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
                crate::format::elapsed(page.elapsed)
            );
            // The columns in view first, while some are out of it.
            let columns = (object.view == ObjectView::Data)
                .then(|| {
                    let fit = super::data_view::Fit::of(workspace, &app.settings, app.layout());
                    super::data_view::columns_note(ui.ctx(), tab, object, fit, &look, locale)
                })
                .flatten();
            Some(match columns {
                Some(columns) => format!("{columns} · {rows}"),
                None => rows,
            })
        })
        .unwrap_or_default();
    // The History tab in front: what its selected entry offers, and what
    // its line says of a key that did nothing.
    let history_line = app
        .workspace(tab)
        .filter(|workspace| workspace.history_in_front())
        .and_then(crate::model::Workspace::history_tab)
        .map(|history| {
            let chosen = history.chosen().map(|row| {
                (
                    row.entry.runs_again(),
                    row.entry.source,
                    row.entry.sql.is_some(),
                )
            });
            (chosen, history.said.clone(), history.shown().count())
        });
    let history_key = app.keymap.key(app.layout(), Command::History);
    let history_keys = [
        Command::MoveInHistory,
        Command::RunHistoryAgain,
        Command::OpenHistoryEntry,
        Command::ShowHistoryCall,
        Command::ReapplyHistoryFilter,
        Command::CopyHistorySql,
        Command::FilterHistory,
        Command::ClearHistory,
        Command::CloseHistoryLine,
    ]
    .map(|command| app.keymap.key(app.layout(), command));
    let history_words = [
        "move",
        "run again",
        "open in sql",
        "agents log",
        "reapply filter",
        "copy sql",
        "filter",
        "clear",
        "back",
    ]
    .map(|word| gettext(locale, word));
    let mut editing = editing_status(app, tab);
    // What was just done (a copy of the review's SQL), where the line has
    // nothing of a lock or of a save to say: this look floats no toast.
    if editing.said.is_none() {
        editing.said = super::toast::said(ui.ctx()).map(|text| Said {
            mark: None,
            text: look.label(&text),
            tail: None,
            color: palette.text,
        });
    }
    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
        .exact_size(STATUS_LINE)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            // Where a question that takes the line's place stands.
            let line = super::every_row_prompt::status_line_id(tab);
            ui.ctx().data_mut(|data| data.insert_temp(line, rect));
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let y = rect.top() + 1.0 + 15.0;
            // The row is shown in the inspector; the cell is edited, where
            // it can be.
            let edit = gettext(locale, "edit");
            let mut table_hints: Vec<widgets::Hint<'_>> = vec![
                (&rows_key, "row", true),
                (&cols_key, "col", true),
                (&inspect_key, "inspect", true),
            ];
            if editing.can_edit {
                table_hints.push((&edit_key, &*edit, true));
            }
            // A new row is opened, where the table takes one.
            let new_row = gettext(locale, "new row");
            if editing.can_add {
                table_hints.push((&add_key, &*new_row, true));
            }
            // The row under the cursor is deleted, where it can be.
            let delete = gettext(locale, "delete");
            if editing.can_delete {
                table_hints.push((&delete_key, &*delete, true));
            }
            table_hints.extend([
                (&filter_key, "filter", true),
                (&sidebar_key, "tables", true),
                (&copy_key, "copy", true),
            ]);
            // The prompt's line that saves, where a save can be made.
            let write = gettext(locale, "write");
            if !read_only {
                table_hints.push((&save_key, &*write, true));
            }
            // And the one that shows what a save would run, while there is
            // anything to show.
            let review = gettext(locale, "review");
            let reviewed =
                editing.pending.is_some() || editing.added.is_some() || editing.deleted.is_some();
            if reviewed {
                table_hints.push((&review_key, &*review, true));
            }
            // An editor's keys: a table's do nothing on it.
            let words = [
                "run",
                "run all",
                "cancel",
                if to_normal { "normal" } else { "leave editor" },
                "tables",
                "complete",
            ]
            .map(|word| gettext(locale, word));
            let mut editor_hints: Vec<widgets::Hint<'_>> = vec![
                (&run_key, &*words[0], true),
                (&run_all_key, &*words[1], true),
                (&cancel_key, &*words[2], true),
                (&leave_key, &*words[3], true),
                (&sidebar_key, &*words[4], true),
            ];
            // An open completion list: its key leads.
            if completing {
                editor_hints.insert(0, (&complete_key, &*words[5], true));
            }
            // The Structure view's keys: a row's do nothing on it.
            let structure_words =
                ["column", "use as label", "choose", "tables"].map(|word| gettext(locale, word));
            let mut structure_hints: Vec<widgets::Hint<'_>> =
                vec![(&column_key, &*structure_words[0], true)];
            if labels {
                // The line that hands the choice back says what it does
                // by itself.
                structure_hints.extend([
                    (&use_key, &*structure_words[1], true),
                    (&chooser_key, &*structure_words[2], true),
                    (&automatic_key, "", true),
                ]);
            }
            structure_hints.push((&sidebar_key, &*structure_words[3], true));
            // What is pending in the table's rows is saved and reviewed
            // from here too, as from the rows.
            if !read_only {
                structure_hints.push((&save_key, &*write, true));
            }
            if editing.pending.is_some() || editing.added.is_some() {
                structure_hints.push((&review_key, &*review, true));
            }
            // The DDL view's, as the board has them: Copy and Open only
            // where the text shows, Esc only while lines are selected. What
            // is pending in the table's rows is saved and reviewed from
            // here too.
            let ddl_words = [
                "line",
                "select lines",
                "copy",
                "open in sql",
                "section",
                "rows",
                "structure",
                "normal",
            ]
            .map(|word| gettext(locale, word));
            let mut ddl_hints: Vec<widgets::Hint<'_>> = ddl_keys
                .iter()
                .zip(&ddl_words)
                .enumerate()
                .filter(|(index, _)| match index {
                    2 | 3 => ddl_text,
                    7 => ddl_visual,
                    _ => true,
                })
                .map(|(_, (key, word))| (key, &**word, true))
                .collect();
            if !read_only {
                ddl_hints.push((&save_key, &*write, true));
            }
            if editing.pending.is_some() || editing.added.is_some() {
                ddl_hints.push((&review_key, &*review, true));
            }
            let (copy_word, on_word) = (gettext(locale, "copy"), gettext(locale, "turn on"));
            let allow_word = gettext(locale, "allow agents");
            let settings_word = gettext(locale, "settings");
            let stop_words = [
                gettext(locale, "sessions"),
                gettext(locale, "stop session"),
                gettext(locale, "stop agents here"),
            ];
            let mut agents_hints: Vec<widgets::Hint<'_>> = match &agents {
                Some(crate::model::AgentsHere::ServerOff) => vec![
                    (&agents_on_key, &*on_word, true),
                    (&agents_settings_key, &*settings_word, true),
                ],
                Some(crate::model::AgentsHere::NoneConnected) if agents_listed => vec![
                    (&agents_move_key, &*stop_words[0], true),
                    (&agents_stop_key, &*stop_words[1], true),
                    (&agents_copy_key, &*copy_word, true),
                    (&agents_settings_key, &*settings_word, true),
                ],
                Some(crate::model::AgentsHere::NoneConnected) => vec![
                    (&agents_copy_key, &*copy_word, true),
                    (&agents_settings_key, &*settings_word, true),
                ],
                Some(crate::model::AgentsHere::Connected { .. }) => vec![
                    (&agents_move_key, &*stop_words[0], true),
                    (&agents_stop_key, &*stop_words[1], true),
                    (&agents_here_key, &*stop_words[2], true),
                    (&agents_copy_key, &*copy_word, true),
                ],
                Some(crate::model::AgentsHere::OffHere) if agents_saved => {
                    vec![(&agents_allow_key, &*allow_word, true)]
                }
                Some(crate::model::AgentsHere::OffHere) => Vec::new(),
                None => Vec::new(),
            };
            let log_words = [
                gettext(locale, "log"),
                gettext(locale, "activity"),
                gettext(locale, "calls"),
                gettext(locale, "all connections"),
                gettext(locale, "this connection"),
                gettext(locale, "filter"),
                gettext(locale, "details"),
                gettext(locale, "open"),
                gettext(locale, "export"),
            ];
            match agents_log {
                // The Log view's keys in the place of the activity's.
                Some((all, selected, opens, any)) => {
                    let scope = if all { &*log_words[4] } else { &*log_words[3] };
                    agents_hints = vec![
                        (&agents_log_key, &*log_words[1], true),
                        (&agents_scope_key, scope, true),
                        (&agents_filter_key, &*log_words[5], true),
                        (&agents_move_key, &*log_words[2], true),
                    ];
                    if selected {
                        agents_hints.push((&agents_details_key, &*log_words[6], true));
                    }
                    if opens {
                        agents_hints.push((&agents_open_key, &*log_words[7], true));
                    }
                    if any {
                        agents_hints.push((&agents_export_key, &*log_words[8], true));
                    }
                }
                None => agents_hints.push((&agents_log_key, &*log_words[0], true)),
            }
            agents_hints.push((&sidebar_key, "tables", true));
            if let Some((chosen, said, shown)) = &history_line {
                // j/k move, r run again, enter open in sql, a agents log,
                // f reapply filter, y copy sql, / filter, :history clear,
                // esc back: each where the selected entry offers it.
                let offered = |index: usize| match (index, chosen) {
                    (0 | 6 | 7 | 8, _) => true,
                    (_, None) => false,
                    (1, Some((again, _, _))) => *again,
                    (2 | 5, Some((_, _, sql))) => *sql,
                    (3, Some((_, source, _))) => *source == crate::history::Source::Agent,
                    (4, Some((_, source, _))) => *source == crate::history::Source::Filter,
                    _ => false,
                };
                let hints: Vec<widgets::Hint<'_>> = history_keys
                    .iter()
                    .zip(&history_words)
                    .enumerate()
                    .filter(|(index, _)| offered(*index))
                    .map(|(_, (key, word))| (key, &**word, true))
                    .collect();
                let left = rect.left() + 12.0;
                widgets::key_hints(ui, (left, y), &hints, 18.0, &look, &palette);
                // Why a key did nothing, or what was just done, at the
                // line's end: this look floats no toast.
                // Else how many it shows: ":history · 184 statements".
                let count = u32::try_from(*shown).unwrap_or(u32::MAX);
                let count = crate::i18n::ngettext(locale, "{n} statement", "{n} statements", count)
                    .replace("{n}", &shown.to_string());
                let said = said
                    .as_ref()
                    .map(|said| (look.label(said), palette.warning))
                    .or_else(|| {
                        editing
                            .said
                            .as_ref()
                            .map(|said| (said.text.clone(), said.color))
                    })
                    .or_else(|| Some((format!("{} · {count}", history_key.as_str()), palette.dim)));
                if let Some((said, color)) = said {
                    let role = TextRole::OSecondary;
                    let text = Text::one(&look, role, &said, color);
                    let width = widgets::paint_text_right(ui, rect.right() - 12.0, y, text);
                    let place = Rect::from_min_size(
                        pos2(rect.right() - 12.0 - width, y - 8.0),
                        vec2(width.max(1.0), 16.0),
                    );
                    widgets::announce(ui, place, &said);
                }
                return;
            }
            if agents.is_some() {
                let left = rect.left() + 12.0;
                widgets::key_hints(ui, (left, y), &agents_hints, 18.0, &look, &palette);
                // What was just done (the command copied), at the line's
                // end: this look floats no toast.
                if let Some(said) = &editing.said {
                    let role = TextRole::OSecondary;
                    let text = Text::one(&look, role, &said.text, said.color);
                    let width = widgets::paint_text_right(ui, rect.right() - 12.0, y, text);
                    let place = Rect::from_min_size(
                        pos2(rect.right() - 12.0 - width, y - 8.0),
                        vec2(width.max(1.0), 16.0),
                    );
                    widgets::announce(ui, place, &said.text);
                }
                return;
            }
            let hints: &[widgets::Hint<'_>] = if on_editor {
                &editor_hints
            } else if structure.is_some() {
                &structure_hints
            } else if ddl.is_some() {
                &ddl_hints
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
                let mut left = rect.left() + 12.0;
                if let Some(mode) =
                    vim_mode.and_then(|mode| mode.label().map(|label| (mode, label)))
                {
                    let (mode, label) = mode;
                    let color = if mode == crate::vim::Mode::Insert {
                        palette.success
                    } else {
                        palette.magenta
                    };
                    let label = gettext(locale, label);
                    let text = Text::one(&look, TextRole::OModeLine, &label, color);
                    left += widgets::paint_label(ui, left, y, text) + gap;
                }
                let shown =
                    fit(rect.right() - 12.0 - summary_width - gap - (left - rect.left() - 12.0));
                let at = (left, y);
                widgets::key_hints(ui, at, &hints[..shown], gap, &look, &palette);
                widgets::paint_text_right(
                    ui,
                    rect.right() - 12.0,
                    y,
                    Text::one(&look, role, &summary, palette.dim),
                );
                return;
            }
            let left = rect.left() + 12.0;
            let room =
                |text: &Option<String>| text.as_ref().map_or(0.0, |text| measure(text) + gap);
            // What is pending is never dropped for width: everything else
            // on the line gives way to it.
            // The new rows stand with the pending changes, before them.
            let pending_room =
                room(&editing.added) + room(&editing.pending) + room(&editing.deleted);
            let errors_room = room(&editing.errors);
            let counts = |ui: &egui::Ui, mut x: f32| {
                for (text, tone) in [
                    (&editing.added, states::Tone::Success),
                    (&editing.pending, states::Tone::Warning),
                    (&editing.deleted, states::Tone::Danger),
                    (&editing.errors, states::Tone::Danger),
                ] {
                    if let Some(text) = text {
                        let text = Text::one(&look, role, text, tone.color(&palette));
                        x += widgets::paint_label(ui, x, y, text) + gap;
                    }
                }
                x
            };
            if let Some(rows) = editing.visual {
                // A visual selection of rows: the mode, how many rows it
                // holds and the counts. Its keys, at the right, give way.
                let mode = gettext(locale, "-- VISUAL --");
                let mode = || Text::one(&look, TextRole::OModeLine, &mode, palette.magenta);
                let held = crate::ui::pending_bar::counted(locale, rows, "row", "rows");
                let deletes = gettext(locale, "delete");
                let normal = gettext(locale, "normal");
                let keys = || {
                    Text::new(&look)
                        .add(role, &visual_delete_key, palette.text)
                        .space(role, " ")
                        .add(role, &deletes, palette.dim)
                        .add(role, " · ", palette.dim)
                        .add(role, &leave_visual_key, palette.text)
                        .space(role, " ")
                        .add(role, &normal, palette.dim)
                };
                let right = rect.right() - 12.0;
                let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
                let text = Text::one(&look, role, &held, palette.secondary);
                x += widgets::paint_label(ui, x, y, text) + gap;
                x = counts(ui, x);
                if let Some(said) = &editing.said {
                    // What the line has to say, in place of the keys: why
                    // the selection's delete was refused, most of all. The
                    // selection stays on then, and this look has no toast
                    // but the line.
                    let room = (right - x).max(0.0);
                    let shown = crate::ui::grid::ellipsize(&said.text, room, false, measure);
                    let text = Text::one(&look, role, &shown, said.color);
                    let width = widgets::paint_text_right(ui, right, y, text);
                    // The whole of it for a screen reader, cut or not.
                    let place = Rect::from_min_size(
                        pos2(right - width, y - 8.0),
                        vec2(width.max(1.0), 16.0),
                    );
                    widgets::announce(ui, place, &said.text);
                } else if x + widgets::measure(ui, keys()) <= right {
                    widgets::paint_text_right(ui, right, y, keys());
                }
                return;
            }
            if let Some(column) = &editing.insert {
                // Insert mode. The keys at the right give way first, then
                // the column: the mode and the counts stay.
                let mode = gettext(locale, "-- INSERT --");
                let mode = || Text::one(&look, TextRole::OModeLine, &mode, palette.success);
                let normal = gettext(locale, "normal");
                let next = editing.walks.map(|next| gettext(locale, next));
                let moves = gettext(locale, "move");
                let sets = gettext(locale, "set");
                let closes = gettext(locale, "close");
                let labels = gettext(locale, "label");
                let keys = || {
                    if editing.picking {
                        // The list's keys, in place of the editor's. Only
                        // Esc differs: it closes the picker and drops the
                        // edit, where it keeps a text; Enter takes the
                        // highlighted option, and Tab goes on as ever.
                        let mut keys = Text::new(&look)
                            .add(role, &pick_keys, palette.text)
                            .space(role, " ")
                            .add(role, &moves, palette.dim)
                            .add(role, " · ", palette.dim)
                            .add(role, &set_key, palette.text)
                            .space(role, " ")
                            .add(role, &sets, palette.dim)
                            .add(role, " · ", palette.dim);
                        // The key that chooses what labels the list, where
                        // there is anything to choose.
                        if editing.labels {
                            keys = keys
                                .add(role, &label_key, palette.text)
                                .space(role, " ")
                                .add(role, &labels, palette.dim)
                                .add(role, " · ", palette.dim);
                        }
                        return keys
                            .add(role, &close_key, palette.text)
                            .space(role, " ")
                            .add(role, &closes, palette.dim);
                    }
                    let leave = Text::new(&look)
                        .add(role, &keep_key, palette.text)
                        .space(role, " ")
                        .add(role, &normal, palette.dim);
                    let Some(next) = &next else {
                        return leave;
                    };
                    leave
                        .add(role, " · ", palette.dim)
                        .add(role, &next_key, palette.text)
                        .space(role, " ")
                        .add(role, next, palette.dim)
                };
                let right = rect.right() - 12.0;
                if let Some(title) = &editing.choosing {
                    // The chooser has the keys: the line names it, and
                    // them where there is room.
                    let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
                    let text = Text::one(&look, role, title, palette.secondary);
                    x += widgets::paint_label(ui, x, y, text) + gap;
                    let (or, chooses, back) = (
                        gettext(locale, "or"),
                        gettext(locale, "choose"),
                        gettext(locale, "back"),
                    );
                    let keys = || {
                        let mut keys = Text::new(&look);
                        if let Some(first) = chooser_moves.first() {
                            keys = keys.add(role, first, palette.text);
                        }
                        for more in chooser_moves.iter().skip(1) {
                            keys = keys
                                .space(role, " ")
                                .add(role, &or, palette.dim)
                                .space(role, " ")
                                .add(role, more, palette.text);
                        }
                        keys.space(role, " ")
                            .add(role, &moves, palette.dim)
                            .add(role, " · ", palette.dim)
                            .add(role, &choose_key, palette.text)
                            .space(role, " ")
                            .add(role, &chooses, palette.dim)
                            .add(role, " · ", palette.dim)
                            .add(role, &back_key, palette.text)
                            .space(role, " ")
                            .add(role, &back, palette.dim)
                    };
                    if x + widgets::measure(ui, keys()) <= right {
                        widgets::paint_text_right(ui, right, y, keys());
                    }
                    return;
                }
                if let Some((tone, said)) = &editing.told {
                    // What became of the label, for a while: in place of
                    // the column and the keys, which come back after it.
                    let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
                    let color = tone.map_or(palette.text, |tone| tone.color(&palette));
                    let wrote = match tone {
                        Some(states::Tone::Success) => drawable(ui, role, &look, WROTE),
                        Some(states::Tone::Warning | states::Tone::Danger) | None => None,
                    };
                    if let Some(mark) = wrote {
                        let mark = Text::one(&look, role, mark, color);
                        x += widgets::paint_label(ui, x, y, mark) + 6.0;
                    }
                    widgets::paint_label(ui, x, y, Text::one(&look, role, said, color));
                    ui.ctx().request_repaint_after(crate::key_picker::SAID);
                    return;
                }
                let fixed = left + widgets::measure(ui, mode()) + pending_room + errors_room;
                let column_room = measure(column) + gap;
                let named = fixed + column_room <= right;
                let keys_width = widgets::measure(ui, keys());
                let mut x = left + widgets::paint_label(ui, left, y, mode()) + gap;
                if named {
                    let text = Text::one(&look, role, column, palette.secondary);
                    x += widgets::paint_label(ui, x, y, text) + gap;
                }
                x = counts(ui, x);
                if let Some(needs) = &editing.needs {
                    // In the danger tone, as the header's star is, where
                    // there is room before the keys.
                    let danger = states::Tone::Danger.color(&palette);
                    let said = || Text::one(&look, role, needs, danger);
                    if x + widgets::measure(ui, said()) <= right {
                        x += widgets::paint_label(ui, x, y, said()) + gap;
                    }
                }
                if let Some((said, column)) = &editing.label {
                    // The column's name stands out of the words round it.
                    // It gives way before the keys do.
                    let at = column.clone().unwrap_or(said.len()..said.len());
                    let label = || {
                        Text::new(&look)
                            .add(role, &said[..at.start], palette.dim)
                            .add(role, &said[at.clone()], palette.text)
                            .add(role, &said[at.end..], palette.dim)
                    };
                    if named && x + widgets::measure(ui, label()) + gap + keys_width <= right {
                        x += widgets::paint_label(ui, x, y, label()) + gap;
                    }
                }
                if named && x + keys_width <= right {
                    widgets::paint_text_right(ui, right, y, keys());
                } else if named && editing.picking {
                    // The one key that behaves differently here, before
                    // the whole line is dropped.
                    let short = || {
                        Text::new(&look)
                            .add(role, "esc", palette.text)
                            .space(role, " ")
                            .add(role, &closes, palette.dim)
                    };
                    if x + widgets::measure(ui, short()) <= right {
                        widgets::paint_text_right(ui, right, y, short());
                    }
                }
                return;
            }
            // The DDL view's selection of lines: its mode stands first, and
            // the rest of the line is drawn after it.
            let left = if ddl_visual {
                let mode = gettext(locale, "-- VISUAL LINE --");
                let mode = Text::one(&look, TextRole::OModeLine, &mode, palette.accent);
                left + widgets::paint_label(ui, left, y, mode) + gap
            } else {
                left
            };
            // A read-only connection's tag ends the keys. Without one,
            // neither it nor the gap before it takes room from them.
            let tag = read_only.then(|| {
                let tag = gettext(locale, "read-only");
                let width = measure(&tag) + 12.0 + 2.0;
                (tag, width)
            });
            let tag_room = tag.as_ref().map_or(0.0, |(_, width)| width + gap);
            let fixed = tag_room + pending_room + errors_room;
            // Where the page's range begins, a gap before it.
            let end = rect.right() - 12.0 - summary_width - gap;
            // What the line says of a lock or of a save: whole where the
            // line without its keys has the room, cut where it has not.
            // Its mark is the first the look's font can draw.
            let said = editing.said.as_ref().map(|said| {
                let mark = said.mark.and_then(|(marks, tone)| {
                    drawable(ui, role, &look, marks).map(|mark| (mark, tone.color(&palette)))
                });
                let lead = mark.map_or(0.0, |(mark, _)| measure(mark) + measure(" "));
                let room = (end - left - fixed - lead).max(0.0);
                let whole = measure(&said.text);
                // What follows a failure only beside the whole of it.
                let tail = said
                    .tail
                    .as_ref()
                    .filter(|tail| whole + measure(" ") + measure(tail) <= room);
                let shown = crate::ui::grid::ellipsize(&said.text, room, false, measure);
                let tail_room = tail.map_or(0.0, |tail| measure(" ") + measure(tail));
                let width = lead + measure(&shown) + tail_room;
                (mark, shown, tail, width)
            });
            let said_room = said.as_ref().map_or(0.0, |said| said.3 + gap);
            let limit = end - fixed - said_room - (left - rect.left() - 12.0);
            // The key that adds a row is here only where the table takes
            // none, and the line that saves only on a connection that
            // cannot write.
            let mut disabled: Vec<String> = Vec::new();
            // The DDL view's line names no key of the rows, struck or not.
            if !editing.can_add && ddl.is_none() {
                disabled.push(format!("{add_key} new row"));
            }
            if !editing.can_delete && ddl.is_none() {
                disabled.push(format!("{delete_key} delete"));
            }
            if read_only {
                disabled.push(format!("{save_key} write"));
            }
            // None where the table takes every key: nothing is struck.
            let disabled_width = disabled.iter().map(|text| measure(text)).sum::<f32>()
                + 14.0 * disabled.len().saturating_sub(1) as f32;
            let mut x = left;
            let shown = fit(limit);
            if shown > 0 {
                x += widgets::key_hints(ui, (x, y), &hints[..shown], gap, &look, &palette) + gap;
            }
            let separator = measure("│");
            let struck = !disabled.is_empty();
            if struck && shown == hints.len() && x + separator + gap + disabled_width <= limit {
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
            if let Some((tag, width)) = &tag {
                let pill = Rect::from_min_size(pos2(x, y - 9.0), vec2(*width, 18.0));
                ui.painter().rect_stroke(
                    pill,
                    CornerRadius::same(3),
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
                widgets::paint_text(ui, x + 7.0, y, Text::one(&look, role, tag, palette.dim));
                x += width + gap;
            }
            x = counts(ui, x);
            if let (Some((mark, shown, tail, _)), Some(whole)) = (said, &editing.said) {
                if let Some((mark, color)) = mark {
                    x += widgets::paint_text(ui, x, y, Text::one(&look, role, mark, color))
                        + measure(" ");
                }
                let text = Text::one(&look, role, &shown, whole.color);
                let width = widgets::paint_text(ui, x, y, text);
                // The whole of it for a screen reader, cut or not.
                let place = Rect::from_min_size(pos2(x, y - 8.0), vec2(width.max(1.0), 16.0));
                widgets::announce(ui, place, &whole.text);
                x += width;
                if let Some(tail) = tail {
                    x += measure(" ");
                    x += widgets::paint_label(ui, x, y, Text::one(&look, role, tail, palette.dim));
                }
                x += gap;
            }
            // The range, unless what stands before it reaches that far.
            if x <= rect.right() - 12.0 - summary_width {
                widgets::paint_text_right(
                    ui,
                    rect.right() - 12.0,
                    y,
                    Text::one(&look, role, &summary, palette.dim),
                );
            }
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
fn banner(
    ui: &mut egui::Ui,
    paint: super::Paint,
    tab: ConnTabId,
    workspace: &crate::model::Workspace,
) -> Vec<Action> {
    let super::Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let name = display_safe(&workspace.name).into_owned();
    // What happened, the plain sentence when it says more, the exact error.
    let (lead, sentence, raw, reconnecting) = match &workspace.status {
        // Closed on purpose: nothing went wrong, and the next use connects,
        // with no strip of its own.
        SessionStatus::Connected | SessionStatus::Idle => return Vec::new(),
        SessionStatus::Connecting { .. } if workspace.woken => return Vec::new(),
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
                crate::format::describe_error(locale, error)
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
    let mut actions = Vec::new();
    if reconnect {
        actions.push(Action::Reconnect(tab));
    }
    if disconnect {
        actions.push(Action::Disconnect(tab));
    }
    actions
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
            key: crate::keymap::Written::default(),
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
        let keymap = crate::keymap::Keymap::default();
        let hint = |look: Look| {
            let layout = crate::keymap::Layout::of(&look);
            let key = keymap.key(layout, Command::Connections);
            connections_hint(&look, locale, &key)
        };
        assert_eq!(hint(Look::macos()), "Connections · ⌘O");
        assert_eq!(hint(Look::standard()), "Connections · Ctrl+O");
        assert_eq!(hint(Look::omarchy()), "connections · ctrl+shift+c");
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
