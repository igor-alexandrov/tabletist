//! The Agents tab: what agents do on the window's connection. One state
//! of four, as the sidebar's Agents entry names it: the server off, access
//! Off here, nobody here with the command that connects one, or the
//! sessions that are here with what they called. The sessions that work
//! elsewhere are "Also:", and each can be stopped.

use egui::{Frame, Id, Margin, Rect, pos2, vec2};

use crate::app::App;
use crate::i18n::{gettext, ngettext};
use crate::model::{Action, AgentsHere, ConnTabId};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::{states, widgets};

/// What the app says once the command is on the clipboard.
pub const COPIED_COMMAND: &str = "Copied the command";

/// Puts the command that connects an agent on the clipboard, and says so.
/// Nothing while it is still being worked out, which takes a moment once,
/// as the app starts.
pub fn copy_command(ctx: &egui::Context, locale: crate::i18n::Locale) {
    if let Some(command) = crate::endpoint::agent_command_ready() {
        ctx.copy_text(command.to_owned());
        crate::ui::toast::say(ctx, &gettext(locale, COPIED_COMMAND));
    }
}

/// The height of the tab's line at its top, and of its status bar.
const HEADER: f32 = 40.0;
const STATUS: f32 = 33.0;

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(here) = app.agents_here(tab) else {
        return;
    };
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    // As an agent names it: "Bookshop · dev".
    let connection = app.agents_name(tab).unwrap_or_default();
    let conn = workspace.conn_id.clone();
    // As the saved connection is, which the agent's session is opened as:
    // the window keeps what it was opened with.
    let environment = app
        .connections
        .get(&conn)
        .map_or(workspace.environment, |saved| saved.environment);
    let runs_as = environment.named(crate::env::Platform::Native);
    // What the connection allows, whether the server runs or not: what is
    // in force, where a change made outside the app is held.
    let allowed = app
        .connections
        .get(&conn)
        .map(|saved| app.agents_access(saved));
    let access = look.label(&match allowed {
        Some(crate::connections::AgentAccess::Read) => gettext(locale, "Read"),
        Some(crate::connections::AgentAccess::Off) | None => gettext(locale, "Off"),
    });
    let sessions: Vec<tabletist_mcp::SessionInfo> =
        app.agents.here(&conn).into_iter().cloned().collect();
    let elsewhere: Vec<Elsewhere> = app
        .agents
        .elsewhere(&conn)
        .into_iter()
        .map(|session| Elsewhere::of(app, session))
        .collect();
    let calls: Vec<tabletist_mcp::Call> = app.agents.calls_on(&conn).cloned().collect();
    let selected = app.agents_selected(tab).map(|session| session.id.clone());
    let layout = app.layout();
    let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
    let copy_key = app
        .keymap
        .key(layout, crate::keymap::Command::CopyAgentCommand);
    let on_key = app.keymap.key(layout, crate::keymap::Command::TurnAgentsOn);
    let stop_key = app
        .keymap
        .key(layout, crate::keymap::Command::StopAgentSession);
    let show_elsewhere_key = app
        .keymap
        .key(layout, crate::keymap::Command::ShowAgentsElsewhere);
    let mut actions = Vec::new();
    egui::Panel::top(Id::new(("agents-header", tab.0)))
        .exact_size(HEADER)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(16, 0)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect().expand2(vec2(16.0, 0.0));
            let rule = if look.terminal {
                palette.outline
            } else {
                palette.border
            };
            widgets::hline(ui, full.x_range(), full.bottom() - 0.5, rule);
            // At the right, and in the macOS look alone: Stop agents here
            // while anyone is, and before it who works elsewhere. The
            // terminal look lists those under its sessions, with its keys.
            let mut right = full.right() - 16.0;
            if !look.terminal && matches!(here, AgentsHere::Connected { .. }) {
                let label = gettext(locale, "Stop agents here");
                let button = states::button(&label, &look).drops();
                let width = button.width(ui, &look);
                let at = Rect::from_center_size(
                    pos2(right - width / 2.0, full.center().y),
                    vec2(width, 28.0),
                );
                if button.show_at(ui, at, &look, &palette).clicked() {
                    stop_here(
                        ui.ctx(),
                        tab,
                        sessions.len(),
                        &connection,
                        locale,
                        &mut actions,
                    );
                }
                right -= width + 12.0;
            }
            if !look.terminal
                && let Some(first) = elsewhere.first()
            {
                right = also(
                    ui,
                    (full, right),
                    first,
                    elsewhere.len() - 1,
                    (&look, &palette, locale),
                    &mut actions,
                ) - 16.0;
            }
            let on = look.label(&gettext(locale, "Agents on"));
            // A connection's name gives way at its end to a narrow tab.
            let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
            let body = widgets::body(&look);
            let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
            let room = (right - full.left() - 16.0 - around).max(0.0);
            let name = crate::ui::grid::ellipsize(&connection, room, false, |text| {
                strong.width(ui.ctx(), look.faces, text)
            });
            let line = Text::new(&look)
                .add(widgets::body(&look), &on, palette.secondary)
                .space(widgets::body(&look), " ")
                .add(strong, &name, palette.text)
                .add(
                    widgets::body(&look),
                    &format!(" · {access}"),
                    palette.secondary,
                );
            widgets::paint_text(ui, full.left() + 16.0, full.center().y, line);
            widgets::announce(ui, full, &format!("{on} {connection} · {access}"));
        });
    if !look.terminal {
        status_bar(ui, tab, (&here, &show_key), (&look, &palette, locale));
    }
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            match &here {
                AgentsHere::ServerOff => {
                    let notice = states::Notice {
                        icon: Icon::SquareTerminal,
                        title: &gettext(locale, "The agent server is off"),
                        text: &gettext(
                            locale,
                            "Turn it on to let agents like Claude Code use the connections you allow. It runs only while Tabletist is open, on this computer.",
                        ),
                    };
                    let label = gettext(locale, "Turn on");
                    let button = states::key_button(&label, &on_key, &look).primary();
                    if states::empty(ui, rect, &notice, vec![button], &look, &palette).is_some() {
                        actions.push(Action::TurnAgentsOn);
                    }
                }
                AgentsHere::OffHere => {
                    let title = format!("{} {connection}", gettext(locale, "Agents can't use"));
                    let notice = states::Notice {
                        icon: Icon::SquareTerminal,
                        title: &title,
                        text: &gettext(
                            locale,
                            "Access is Off for this connection, so agents don't see it at all.",
                        ),
                    };
                    states::empty(ui, rect, &notice, Vec::new(), &look, &palette);
                }
                AgentsHere::NoneConnected if look.terminal && !elsewhere.is_empty() => {
                    // Who works elsewhere is listed at the left, as with
                    // agents here, and the way to connect one beside it.
                    let listed = (&sessions[..], &elsewhere[..], selected.as_deref());
                    terminal_nav(ui, tab, listed, &show_elsewhere_key, (&look, &palette, locale));
                    let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
                    egui::CentralPanel::default()
                        .frame(Frame::new().fill(palette.window))
                        .show(ui, |ui| {
                            let rect = ui.max_rect();
                            none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
                        });
                }
                AgentsHere::NoneConnected => {
                    let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
                    none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
                }
                AgentsHere::Connected { .. } if look.terminal => {
                    let keys = (&stop_key, &show_elsewhere_key);
                    let listed = (&sessions[..], &elsewhere[..], selected.as_deref());
                    terminal_activity(ui, tab, listed, &calls, keys, runs_as.as_deref(), (&look, &palette, locale));
                }
                AgentsHere::Connected { .. } => {
                    let stopped = activity(
                        ui,
                        rect,
                        (&sessions, &calls),
                        runs_as.as_deref(),
                        (&look, &palette, locale),
                    );
                    actions.extend(stopped.into_iter().map(Action::StopAgentSession));
                }
            }
        });
    app.actions.extend(actions);
}

/// The macOS look's status bar: who is here, the server, and the key that
/// shows the tab.
fn status_bar(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    (here, key): (&AgentsHere, &crate::keymap::Written),
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    // Who is here, in the state that has anyone: an agent stays among
    // those that named this connection once its access is Off.
    let count = match here {
        AgentsHere::Connected { count, .. } => *count,
        AgentsHere::ServerOff | AgentsHere::OffHere | AgentsHere::NoneConnected => 0,
    };
    let people = match count {
        0 => gettext(locale, "No agents here").into_owned(),
        count => ngettext(
            locale,
            "{n} agent here",
            "{n} agents here",
            u32::try_from(count).unwrap_or(u32::MAX),
        )
        .replace("{n}", &count.to_string()),
    };
    let server = match here {
        AgentsHere::ServerOff => gettext(locale, "Server: off"),
        _ => gettext(locale, "Server: stdio"),
    };
    let shows = format!("{} {}", key.as_str(), gettext(locale, "shows this tab"));
    egui::Panel::bottom(Id::new(("agents-status", tab.0)))
        .exact_size(STATUS)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
            let role = widgets::secondary(look);
            let left = format!("{people} · {server}");
            widgets::paint_text(
                ui,
                full.left() + 16.0,
                full.center().y,
                Text::one(look, role, &left, status),
            );
            widgets::announce(ui, full, &left);
            let laid = Text::one(look, role, &shows, status).layout(ui.ctx());
            laid.paint_left(
                ui.painter(),
                full.right() - 16.0 - laid.width(),
                full.center().y,
            );
        });
}

/// Nobody here: how to connect an agent, the command with Copy beside it,
/// and what agents may do on this connection. Centred under an icon tile,
/// or in the terminal look from the top left without one, as an empty
/// state is (`states::empty`).
fn none_connected(
    ui: &mut egui::Ui,
    rect: Rect,
    (connection, name, access): (&str, &str, &str),
    copy_key: &crate::keymap::Written,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    let ctx = ui.ctx().clone();
    let terminal = look.terminal;
    let room = (rect.width() - 2.0 * states::INSET).max(0.0);
    let measure = if terminal { room } else { room.min(420.0) };
    let align = if terminal {
        egui::Align::Min
    } else {
        egui::Align::Center
    };
    let lay = |role: TextRole, text: &str, color| {
        let mut text = Text::one(look, role, text, color).wrap(measure);
        text.job_mut().halign = align;
        text.layout(&ctx)
    };
    let title = format!("{} {connection}", gettext(locale, "No agents connected to"));
    let text = gettext(
        locale,
        "Agents like Claude Code can read this database. Connect one with this command, then ask it about {name}.",
    )
    .replace("{name}", name);
    let title_laid = lay(states::title_role(look), &title, palette.text);
    let text_laid = lay(widgets::body(look), &text, palette.secondary);
    // The command, and Copy beside it: the command gives way in its
    // middle to a window too narrow for it, and is copied whole.
    let label = gettext(locale, "Copy");
    let copy = states::key_button(&label, copy_key, look).icon(Icon::Copy);
    let copy_width = copy.width(ui, look);
    let height = states::button_height(look);
    let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
    // Worked out off the UI thread as the app starts: a moment's "…" until
    // then, and a frame asked for to show it.
    let command = crate::endpoint::agent_command_ready().unwrap_or_else(|| {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
        "…"
    });
    let fits = (room - 8.0 - copy_width - 24.0).clamp(0.0, 520.0);
    let shown = crate::ui::grid::ellipsize(command, fits, true, |text| {
        mono.width(&ctx, look.faces, text)
    });
    let command_laid = Text::one(look, mono, &shown, palette.text).layout(&ctx);
    let row_width = command_laid.width() + 24.0 + 8.0 + copy_width;
    // What agents may do here.
    let on_this = look.label(&gettext(locale, "On this connection"));
    let access_word = look.label(&gettext(locale, "Access"));
    let small = widgets::secondary(look);
    let facts = Text::new(look)
        .add(small, &on_this, palette.dim)
        .space(small, "   ")
        .add(small, &access_word, palette.secondary)
        .space(small, " ")
        .add(small, access, palette.text)
        .layout(&ctx);
    let (tile, under_tile) = if terminal { (0.0, 0.0) } else { (44.0, 14.0) };
    let total = tile
        + under_tile
        + title_laid.height()
        + 8.0
        + text_laid.height()
        + 14.0
        + height
        + 18.0
        + facts.height();
    let (x, mut y) = if terminal {
        (rect.left() + states::INSET, rect.top() + states::INSET)
    } else {
        let top = rect.center().y - total / 2.0;
        (rect.center().x, top.max(rect.top() + states::INSET))
    };
    // A piece's left edge, for one `width` wide.
    let left = |width: f32| if terminal { x } else { x - width / 2.0 };
    if !terminal {
        let tile_rect = Rect::from_center_size(pos2(x, y + tile / 2.0), vec2(tile, tile));
        ui.painter().rect_filled(
            tile_rect,
            egui::CornerRadius::same(look.radius + 4),
            palette.surface,
        );
        Icon::SquareTerminal.image(palette.dim, 20.0).paint_at(
            ui,
            Rect::from_center_size(tile_rect.center(), vec2(20.0, 20.0)),
        );
        y += tile + under_tile;
    }
    let paint = |ui: &egui::Ui, laid: &crate::typography::Laid, said: &str, y: f32| {
        laid.paint(ui.painter(), pos2(x, y));
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(left(laid.width()), y), laid.size()),
            said,
        );
    };
    paint(ui, &title_laid, &title, y);
    y += title_laid.height() + 8.0;
    paint(ui, &text_laid, &text, y);
    y += text_laid.height() + 14.0;
    let row_left = left(row_width);
    let boxed = Rect::from_min_size(pos2(row_left, y), vec2(command_laid.width() + 24.0, height));
    widgets::card(ui, boxed, false, look, palette);
    command_laid.paint_left(ui.painter(), boxed.left() + 12.0, boxed.center().y);
    widgets::announce(ui, boxed, command);
    let at = Rect::from_min_size(pos2(boxed.right() + 8.0, y), vec2(copy_width, height));
    if copy.show_at(ui, at, look, palette).clicked() {
        copy_command(ui.ctx(), locale);
    }
    y += height + 18.0;
    let facts_left = left(facts.width());
    facts.paint_left(ui.painter(), facts_left, y + facts.height() / 2.0);
    widgets::announce(
        ui,
        Rect::from_min_size(pos2(facts_left, y), facts.size()),
        &format!("{on_this}: {access_word} {access}"),
    );
}

/// A session that works elsewhere, as "Also:" names it: its agent, and
/// the connection it named last, by the name an agent sees, where that is
/// a connection that is saved.
pub struct Elsewhere {
    pub id: String,
    pub agent: String,
    pub on: Option<(crate::connections::ConnectionId, String)>,
}

impl Elsewhere {
    pub fn of(app: &App, session: &tabletist_mcp::SessionInfo) -> Self {
        let saved = &app.connections.connections;
        let names = crate::backend::agent_names(saved);
        let on = session.connections.iter().rev().find_map(|id| {
            saved
                .iter()
                .zip(&names)
                .find(|(saved, _)| saved.id.0 == *id)
                .map(|(saved, name)| (saved.id.clone(), name.clone()))
        });
        Self {
            id: session.id.clone(),
            agent: crate::model::agent_name(session),
            on,
        }
    }

    /// "Cursor on Bookshop · staging", or "Cursor · no connection yet".
    fn said(&self, locale: crate::i18n::Locale) -> String {
        match &self.on {
            Some((_, name)) => format!("{} {} {name}", self.agent, gettext(locale, "on")),
            None => format!("{} · {}", self.agent, gettext(locale, "no connection yet")),
        }
    }
}

/// Stops the agents here, and says so: the toast names how many and the
/// connection whose access is Off now.
pub fn stop_here(
    ctx: &egui::Context,
    tab: ConnTabId,
    count: usize,
    connection: &str,
    locale: crate::i18n::Locale,
    actions: &mut Vec<Action>,
) {
    actions.push(Action::StopAgentsHere(tab));
    let said = ngettext(
        locale,
        "Stopped {n} agent. Access is now Off for {name}.",
        "Stopped {n} agents. Access is now Off for {name}.",
        u32::try_from(count).unwrap_or(u32::MAX),
    )
    .replace("{n}", &count.to_string())
    .replace("{name}", connection);
    crate::ui::toast::say(ctx, &said);
}

/// The macOS header's "Also:": the session elsewhere that called last,
/// with Show where it works on a saved connection, and how many more there
/// are. Drawn from `right` leftwards on `full`'s line; returns where it
/// starts.
fn also(
    ui: &mut egui::Ui,
    (full, right): (Rect, f32),
    first: &Elsewhere,
    more: usize,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
    actions: &mut Vec<Action>,
) -> f32 {
    let mut right = right;
    if let Some((conn, _)) = &first.on {
        let label = gettext(locale, "Show");
        let show = widgets::ButtonSpec::new(&label).link().padding(6.0);
        let width = show.width(ui, look);
        let at = Rect::from_center_size(
            pos2(right - width / 2.0, full.center().y),
            vec2(width, 24.0),
        );
        if show.show_at(ui, at, look, palette).clicked() {
            actions.push(Action::ShowAgentsOn(conn.clone()));
        }
        right -= width + 4.0;
    }
    let mut said = format!("{} {}", gettext(locale, "Also:"), first.said(locale));
    if more > 0 {
        said.push_str(
            &ngettext(
                locale,
                " and {n} more",
                " and {n} more",
                u32::try_from(more).unwrap_or(u32::MAX),
            )
            .replace("{n}", &more.to_string()),
        );
    }
    let laid = Text::one(look, widgets::secondary(look), &said, palette.secondary).layout(ui.ctx());
    let left = right - laid.width();
    laid.paint_left(ui.painter(), left, full.center().y);
    widgets::announce(
        ui,
        Rect::from_min_max(pos2(left, full.top()), pos2(right, full.bottom())),
        &said,
    );
    left
}

/// The macOS look's activity: the sessions that are here, each with Stop
/// session, and under them the calls on this connection. Returns the
/// sessions whose Stop session was clicked.
fn activity(
    ui: &mut egui::Ui,
    rect: Rect,
    (sessions, calls): (&[tabletist_mcp::SessionInfo], &[tabletist_mcp::Call]),
    runs_as: Option<&str>,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) -> Vec<String> {
    let mut stopped = Vec::new();
    // More than the tab has the height for scrolls, under its header and
    // above its status bar.
    let mut area = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("agents-activity")
        .auto_shrink([false, false])
        .show(&mut area, |ui| {
            let rows = SESSION_ROW * sessions.len() as f32;
            let height = rows + 16.0 + feed_height(calls.len(), look);
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
            stopped = session_rows(ui, rect, sessions, runs_as, (look, palette, locale));
            let feed_rect =
                Rect::from_min_max(pos2(rect.left(), rect.top() + rows + 16.0), rect.max);
            feed(ui, feed_rect, calls, (look, palette, locale));
        });
    stopped
}

/// A session's row in the macOS look.
const SESSION_ROW: f32 = 44.0;

/// The sessions' rows in `rect`, from its top: who, where it was started,
/// since when and how many calls; at the right as what it runs, and Stop
/// session. Returns the sessions whose Stop session was clicked.
fn session_rows(
    ui: &mut egui::Ui,
    rect: Rect,
    sessions: &[tabletist_mcp::SessionInfo],
    runs_as: Option<&str>,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) -> Vec<String> {
    let mut stopped = Vec::new();
    let mut top = rect.top();
    let label = gettext(locale, "Stop session");
    for session in sessions {
        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), SESSION_ROW));
        let agent = crate::model::agent_name(session);
        let facts = session_facts(session, locale);
        // Stop session at the right, and as what it runs before it.
        let button = states::button(&label, look).salt(&session.id);
        let width = button.width(ui, look);
        let at = Rect::from_center_size(
            pos2(line.right() - 16.0 - width / 2.0, line.center().y),
            vec2(width, 26.0),
        );
        if button.show_at(ui, at, look, palette).clicked() {
            stopped.push(session.id.clone());
        }
        let mut right = at.left() - 12.0;
        let small = widgets::secondary(look);
        let mut said = format!("{agent} · {facts}");
        if let Some(runs_as) = runs_as {
            let runs = format!("{} {runs_as}", gettext(locale, "Runs as"));
            let laid = Text::one(look, small, &runs, palette.secondary).layout(ui.ctx());
            laid.paint_left(ui.painter(), right - laid.width(), line.center().y);
            right -= laid.width() + 16.0;
            said = format!("{said} · {runs}");
        }
        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
        // What does not fit the line gives way at its end, the name last.
        let room = (right - line.left() - 30.0).max(0.0);
        let name_width = |text: &str| strong.width(ui.ctx(), look.faces, text);
        let shown_agent = crate::ui::grid::ellipsize(&agent, room, false, name_width);
        let after = (room - name_width(&shown_agent) - name_width("  ")).max(0.0);
        let shown_facts = crate::ui::grid::ellipsize(&facts, after, false, |text| {
            small.width(ui.ctx(), look.faces, text)
        });
        let text = Text::new(look)
            .add(strong, &shown_agent, palette.text)
            .space(strong, "  ")
            .add(small, &shown_facts, palette.secondary);
        // The dot is painted: not every face has the glyph.
        let dot = states::Tone::Success.color(palette);
        ui.painter()
            .circle_filled(pos2(line.left() + 19.5, line.center().y), 3.5, dot);
        widgets::paint_text(ui, line.left() + 30.0, line.center().y, text);
        widgets::announce(ui, line, &said);
        widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
        top += SESSION_ROW;
    }
    stopped
}

/// Where a session was started, since when, and how many calls it made.
fn session_facts(session: &tabletist_mcp::SessionInfo, locale: crate::i18n::Locale) -> String {
    let since = std::time::SystemTime::now()
        .duration_since(session.connected_at)
        .unwrap_or_default();
    let mut facts = Vec::new();
    if let Some(project) = &session.project {
        facts.push(project.clone());
    }
    facts.push(format!(
        "{} {}",
        gettext(locale, "connected"),
        ago(since, locale)
    ));
    facts.push(
        ngettext(
            locale,
            "{n} call",
            "{n} calls",
            u32::try_from(session.calls).unwrap_or(u32::MAX),
        )
        .replace("{n}", &session.calls.to_string()),
    );
    facts.join(" · ")
}

/// The terminal look's activity, as its board draws it: at the left the
/// sessions here, then those elsewhere, one of them selected; at the right
/// the selected one with its key, over the calls on this connection.
fn terminal_activity(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    (here, elsewhere, selected): (&[tabletist_mcp::SessionInfo], &[Elsewhere], Option<&str>),
    calls: &[tabletist_mcp::Call],
    (stop_key, show_key): (&crate::keymap::Written, &crate::keymap::Written),
    runs_as: Option<&str>,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    terminal_nav(
        ui,
        tab,
        (here, elsewhere, selected),
        show_key,
        (look, palette, locale),
    );
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The selected session's line, where it is one of those here.
            let chosen = here
                .iter()
                .find(|session| selected == Some(session.id.as_str()));
            let line = Rect::from_min_size(full.min, vec2(full.width(), 34.0));
            if let Some(session) = chosen {
                ui.painter().rect_filled(line, 0.0, palette.panel);
                widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.outline);
                let mut facts = session_facts(session, locale);
                if let Some(runs_as) = runs_as {
                    facts = format!("{facts} · {} {runs_as}", gettext(locale, "runs as"));
                }
                let agent = look.label(&crate::model::agent_name(session));
                let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
                let body = widgets::body(look);
                let stop = Text::new(look)
                    .add(body, stop_key.as_str(), palette.accent)
                    .space(body, " ")
                    .add(body, &gettext(locale, "stop session"), palette.secondary)
                    .layout(ui.ctx());
                let right = line.right() - 12.0 - stop.width();
                stop.paint_left(ui.painter(), right, line.center().y);
                let room = (right - line.left() - 24.0).max(0.0);
                let text = format!("{agent}  {facts}");
                let shown = crate::ui::grid::ellipsize(&text, room, false, |text| {
                    body.width(ui.ctx(), look.faces, text)
                });
                // The name in the text colour, as far as it was not cut.
                let (name, rest) = match shown.strip_prefix(agent.as_str()) {
                    Some(rest) => (agent.as_str(), rest),
                    None => (shown.as_ref(), ""),
                };
                let laid = Text::new(look).add(strong, name, palette.text).add(
                    body,
                    rest,
                    palette.secondary,
                );
                widgets::paint_text(ui, line.left() + 12.0, line.center().y, laid);
                widgets::announce(ui, line, &text);
            }
            let top = if chosen.is_some() {
                line.bottom() + 14.0
            } else {
                full.top() + 14.0
            };
            let area_rect = Rect::from_min_max(
                pos2(full.left() + 14.0, top),
                pos2(full.right() - 14.0, full.bottom()),
            );
            let mut area = ui.new_child(egui::UiBuilder::new().max_rect(area_rect));
            egui::ScrollArea::vertical()
                .id_salt("agents-feed")
                .auto_shrink([false, false])
                .show(&mut area, |ui| {
                    let height = feed_height(calls.len(), look);
                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), height),
                        egui::Sense::hover(),
                    );
                    feed(ui, rect, calls, (look, palette, locale));
                });
        });
}

/// The terminal look's list of sessions, at the left: those here, then
/// those elsewhere, the selected one filled.
fn terminal_nav(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    (here, elsewhere, selected): (&[tabletist_mcp::SessionInfo], &[Elsewhere], Option<&str>),
    show_key: &crate::keymap::Written,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    let shows = format!("{} {}", show_key.as_str(), gettext(locale, "shows"));
    // A narrow window gives the calls the room the list would take.
    let nav = NAV.min(ui.available_width() * 0.35);
    egui::Panel::left(Id::new(("agents-sessions", tab.0)))
        .exact_size(nav)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::vline(ui, full.right() - 0.5, full.y_range(), palette.outline);
            let mut y = full.top() + 10.0;
            nav_group(
                ui,
                (full, &mut y),
                &gettext(locale, "On this connection"),
                (look, palette),
            );
            for session in here {
                let first = look.label(&crate::model::agent_name(session));
                let second = session_facts(session, locale);
                let chosen = selected == Some(session.id.as_str());
                nav_row(
                    ui,
                    (full, &mut y),
                    (&first, &second),
                    chosen,
                    (look, palette),
                );
            }
            if !elsewhere.is_empty() {
                y += 8.0;
                nav_group(
                    ui,
                    (full, &mut y),
                    &gettext(locale, "Elsewhere"),
                    (look, palette),
                );
            }
            for other in elsewhere {
                let first = look.label(&other.agent);
                let second = match &other.on {
                    Some((_, name)) => format!("{name} · {shows}"),
                    None => gettext(locale, "no connection yet").into_owned(),
                };
                let chosen = selected == Some(other.id.as_str());
                nav_row(
                    ui,
                    (full, &mut y),
                    (&first, &second),
                    chosen,
                    (look, palette),
                );
            }
        });
}

/// The terminal look's list of sessions: its width.
const NAV: f32 = 300.0;

/// A group's title in the terminal look's list of sessions.
fn nav_group(
    ui: &egui::Ui,
    (full, y): (Rect, &mut f32),
    title: &str,
    (look, palette): (&Look, &Palette),
) {
    let laid = Text::one(
        look,
        widgets::body(look),
        &look.label(title),
        palette.secondary,
    )
    .layout(ui.ctx());
    laid.paint_left(ui.painter(), full.left() + 14.0, *y + 11.0);
    *y += 22.0;
}

/// A session in the terminal look's list: its agent, and a line under it.
/// The selected one is filled, with the accent at its left edge.
fn nav_row(
    ui: &egui::Ui,
    (full, y): (Rect, &mut f32),
    (first, second): (&str, &str),
    chosen: bool,
    (look, palette): (&Look, &Palette),
) {
    let rect = Rect::from_min_size(pos2(full.left(), *y), vec2(full.width() - 1.0, 44.0));
    if chosen {
        ui.painter().rect_filled(rect, 0.0, palette.selection);
        let edge = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
        ui.painter().rect_filled(edge, 0.0, palette.accent);
    }
    let body = widgets::body(look);
    let room = (rect.width() - 28.0 - 14.0).max(0.0);
    let fit = |text: &str| -> String {
        crate::ui::grid::ellipsize(text, room, false, |text| {
            body.width(ui.ctx(), look.faces, text)
        })
        .into_owned()
    };
    let dot = states::Tone::Success.color(palette);
    ui.painter()
        .circle_filled(pos2(rect.left() + 18.0, rect.top() + 12.0), 3.5, dot);
    let laid = Text::one(look, body, &fit(first), palette.text).layout(ui.ctx());
    laid.paint_left(ui.painter(), rect.left() + 28.0, rect.top() + 12.0);
    let laid = Text::one(look, body, &fit(second), palette.secondary).layout(ui.ctx());
    laid.paint_left(ui.painter(), rect.left() + 28.0, rect.top() + 32.0);
    widgets::announce(ui, rect, &format!("{first} · {second}"));
    *y += 48.0;
}

/// A line of the feed, and its head.
fn feed_row(look: &Look) -> f32 {
    if look.terminal { 26.0 } else { 30.0 }
}

/// The feed's height: its head, and a line for each call or for saying
/// there is none yet.
fn feed_height(calls: usize, look: &Look) -> f32 {
    feed_row(look) * (1 + calls.max(1)) as f32
}

/// The calls on this connection, the newest first: when, the tool, what
/// it asked and how it ended, under a head that names the four.
fn feed(
    ui: &egui::Ui,
    rect: Rect,
    calls: &[tabletist_mcp::Call],
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    let row = feed_row(look);
    let body = widgets::body(look);
    let small = widgets::secondary(look);
    let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
    // Time, tool, request, result: the request takes what is left, and
    // in a narrow tab the tool and the result give way first.
    let room = (rect.width() - 24.0).max(0.0);
    let time = 90.0_f32.min(room * 0.2);
    let (tool, result) = (150.0_f32.min(room * 0.2), 230.0_f32.min(room * 0.3));
    let request = (room - time - tool - result).max(0.0);
    let columns = [
        (rect.left() + 12.0, time),
        (rect.left() + 12.0 + time, tool),
        (rect.left() + 12.0 + time + tool, request),
        (rect.right() - 12.0 - result, result),
    ];
    let cell =
        |ui: &egui::Ui, (x, width): (f32, f32), y: f32, role: TextRole, text: &str, color| {
            let shown = crate::ui::grid::ellipsize(text, width - 10.0, false, |text| {
                role.width(ui.ctx(), look.faces, text)
            });
            Text::one(look, role, &shown, color)
                .layout(ui.ctx())
                .paint_left(ui.painter(), x, y);
        };
    let head = Rect::from_min_size(rect.min, vec2(rect.width(), row));
    ui.painter().rect_filled(head, 0.0, palette.panel);
    widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.border);
    let names =
        ["Time", "Tool", "Request", "Result"].map(|name| look.label(&gettext(locale, name)));
    for (column, name) in columns.iter().zip(&names) {
        cell(ui, *column, head.center().y, small, name, palette.secondary);
    }
    widgets::announce(ui, head, &names.join(" · "));
    let now = std::time::SystemTime::now();
    let mut y = head.bottom();
    if calls.is_empty() {
        let said = look.label(&gettext(locale, "No calls on this connection yet."));
        cell(
            ui,
            (columns[0].0, rect.width() - 24.0),
            y + row / 2.0,
            small,
            &said,
            palette.dim,
        );
        widgets::announce(
            ui,
            Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row)),
            &said,
        );
        return;
    }
    for call in calls {
        let line = Rect::from_min_size(pos2(rect.left(), y), vec2(rect.width(), row));
        let when = ago(now.duration_since(call.at).unwrap_or_default(), locale);
        let (ended, color) = ended(call, palette, locale);
        let asked = call.request.as_deref().unwrap_or_default();
        let center = line.center().y;
        cell(
            ui,
            columns[0],
            center,
            small,
            &look.label(&when),
            palette.secondary,
        );
        cell(ui, columns[1], center, body, &call.tool, palette.text);
        cell(ui, columns[2], center, mono, asked, palette.text);
        cell(ui, columns[3], center, small, &ended, color);
        widgets::announce(
            ui,
            line,
            &format!("{when} · {} · {asked} · {ended}", call.tool),
        );
        if !look.terminal {
            widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
        }
        y += row;
    }
}

/// How a call ended, as the feed says it, and its colour: what it
/// returned, or why it did not.
fn ended(
    call: &tabletist_mcp::Call,
    palette: &Palette,
    locale: crate::i18n::Locale,
) -> (String, egui::Color32) {
    let count = |one: &'static str, many: &'static str, n: u64| {
        ngettext(locale, one, many, u32::try_from(n).unwrap_or(u32::MAX))
            .replace("{n}", &n.to_string())
    };
    if call.result == "ok" {
        let said = match (call.tool.as_str(), call.rows) {
            ("describe_schema", Some(tables)) => {
                let mut said = count("{n} table", "{n} tables", tables);
                if let Some(columns) = call.columns {
                    said = format!("{said} · {}", count("{n} column", "{n} columns", columns));
                }
                said
            }
            (_, Some(rows)) => format!(
                "{} · {} ms",
                count("{n} row", "{n} rows", rows),
                call.took.as_millis()
            ),
            (_, None) => gettext(locale, "ok").into_owned(),
        };
        return (said, states::Tone::Success.color(palette));
    }
    let (word, color) = match call.result.as_str() {
        "timeout" => (
            gettext(locale, "timed out"),
            states::Tone::Warning.color(palette),
        ),
        "cancelled" => (gettext(locale, "cancelled"), palette.secondary),
        "stopped" => (gettext(locale, "stopped"), palette.secondary),
        "error" => (
            gettext(locale, "failed"),
            states::Tone::Danger.color(palette),
        ),
        _ => (
            gettext(locale, "refused"),
            states::Tone::Danger.color(palette),
        ),
    };
    let said = match &call.message {
        // On one line: a database's own words may run over several.
        Some(message) => format!(
            "{word} · {}",
            message.split_whitespace().collect::<Vec<_>>().join(" ")
        ),
        None => word.into_owned(),
    };
    (said, color)
}

/// How long ago, as the person reads it: "just now", "4 min", "2 h".
pub fn ago(since: std::time::Duration, locale: crate::i18n::Locale) -> String {
    let minutes = since.as_secs() / 60;
    match minutes {
        0 => gettext(locale, "just now").into_owned(),
        1..60 => format!("{minutes} {}", gettext(locale, "min ago")),
        _ => format!("{} {}", minutes / 60, gettext(locale, "h ago")),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_while_ago_is_said_in_minutes_then_hours() {
        let locale = crate::i18n::Locale::default();
        let ago = |secs| super::ago(std::time::Duration::from_secs(secs), locale);
        assert_eq!(ago(30), "just now");
        assert_eq!(ago(240), "4 min ago");
        assert_eq!(ago(2 * 3600 + 60), "2 h ago");
    }

    /// Where agents cannot reach the app, nothing of them shows: no entry,
    /// and no tab.
    #[cfg(not(unix))]
    #[test]
    fn nothing_of_agents_shows_where_they_cannot_reach_the_app() {
        let mut harness = crate::testing::Harness::new();
        let tab = harness.connect_fake();
        let conn = harness.app.workspace(tab).unwrap().conn_id.clone();
        let mut saved = harness.app.connections.get(&conn).unwrap().clone();
        saved.agents.access = crate::connections::AgentAccess::Read;
        harness.app.connections.upsert(saved);
        for enabled in [false, true] {
            harness.app.settings.mcp_enabled = enabled;
            harness.settle();
            for state in ["Server off", "Off here", "None connected"] {
                assert!(!harness.has(&format!("Agents, {state}")), "{state}");
            }
        }
        harness.app.apply(crate::model::Action::ShowAgents(tab));
        assert_eq!(harness.app.workspace(tab).unwrap().agents_tab(), None);
    }
}
