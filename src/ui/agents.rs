//! The Agents tab: what agents do on the window's connection. One state
//! of four, as the sidebar's Agents entry names it: the server off, access
//! Off here, nobody here with the command that connects one, or the
//! sessions that are here.

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
    let layout = app.layout();
    let show_key = app.keymap.key(layout, crate::keymap::Command::ShowAgents);
    let copy_key = app
        .keymap
        .key(layout, crate::keymap::Command::CopyAgentCommand);
    let on_key = app.keymap.key(layout, crate::keymap::Command::TurnAgentsOn);
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
            let on = look.label(&gettext(locale, "Agents on"));
            // A connection's name gives way at its end to a narrow tab.
            let strong = TextRole::pick(&look, TextRole::UiBodyStrong, TextRole::OBody);
            let body = widgets::body(&look);
            let around = body.width(ui.ctx(), look.faces, &format!("{on}  · {access}"));
            let room = (full.width() - 32.0 - around).max(0.0);
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
                AgentsHere::NoneConnected => {
                    let name = app.workspace(tab).map(|w| w.name.clone()).unwrap_or_default();
                    none_connected(ui, rect, (&connection, &name, &access), &copy_key, (&look, &palette, locale));
                }
                AgentsHere::Connected { .. } => {
                    sessions_here(
                        ui,
                        rect,
                        &sessions,
                        runs_as.as_deref(),
                        (&look, &palette, locale),
                    );
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

/// The sessions that are here, the one that called last first: who, where
/// it was started, since when, how many calls, and as what it runs.
fn sessions_here(
    ui: &mut egui::Ui,
    rect: Rect,
    sessions: &[tabletist_mcp::SessionInfo],
    runs_as: Option<&str>,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    let row = if look.terminal { 22.0 } else { 44.0 };
    // More sessions than the tab has the height for scroll, under its
    // header and above its status bar.
    let mut area = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    egui::ScrollArea::vertical()
        .id_salt("agents-sessions")
        .auto_shrink([false, false])
        .show(&mut area, |ui| {
            let height = 12.0 + row * sessions.len() as f32 + 12.0;
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
            session_rows(ui, rect, sessions, runs_as, (look, palette, locale));
        });
}

/// The sessions' rows in `rect`, from its top: one for each, `row` high.
fn session_rows(
    ui: &mut egui::Ui,
    rect: Rect,
    sessions: &[tabletist_mcp::SessionInfo],
    runs_as: Option<&str>,
    (look, palette, locale): (&Look, &Palette, crate::i18n::Locale),
) {
    let row = if look.terminal { 22.0 } else { 44.0 };
    let mut top = rect.top() + 12.0;
    let now = std::time::SystemTime::now();
    for session in sessions {
        let line = Rect::from_min_size(pos2(rect.left(), top), vec2(rect.width(), row));
        let agent = crate::model::agent_name(session);
        let agent = if look.terminal {
            look.label(&agent)
        } else {
            agent
        };
        let since = now.duration_since(session.connected_at).unwrap_or_default();
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
        // A connection with no environment runs as nothing to name.
        if let Some(runs_as) = runs_as {
            facts.push(format!("{} {runs_as}", gettext(locale, "Runs as")));
        }
        let facts = facts.join(" · ");
        let dot = crate::ui::states::Tone::Success.color(palette);
        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
        // What does not fit the line gives way at its end, the name last.
        let room = (line.width() - 30.0 - 16.0).max(0.0);
        let name_width = |text: &str| strong.width(ui.ctx(), look.faces, text);
        let shown_agent = crate::ui::grid::ellipsize(&agent, room, false, name_width);
        let after = (room - name_width(&shown_agent) - name_width("  ")).max(0.0);
        let small = widgets::secondary(look);
        let shown_facts = crate::ui::grid::ellipsize(&facts, after, false, |text| {
            small.width(ui.ctx(), look.faces, text)
        });
        let text = Text::new(look)
            .add(strong, &shown_agent, palette.text)
            .space(strong, "  ")
            .add(small, &shown_facts, palette.secondary);
        // The dot is painted: not every face has the glyph.
        ui.painter()
            .circle_filled(pos2(line.left() + 19.5, line.center().y), 3.5, dot);
        let x = line.left() + 30.0;
        widgets::paint_text(ui, x, line.center().y, text);
        widgets::announce(ui, line, &format!("{agent} · {facts}"));
        if !look.terminal {
            widgets::hline(ui, line.x_range(), line.bottom() - 0.5, palette.border);
        }
        top += row;
    }
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
