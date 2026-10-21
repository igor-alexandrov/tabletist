//! Settings › Agents as the sheet draws it: the server's switch beside the
//! pane's title, whether it runs and who is connected, how to connect an
//! agent, where access is set, what keeps agents within bounds, and the
//! log's options.

use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::app::App;
use crate::endpoint::Agent;
use crate::i18n::{gettext, ngettext};
use crate::keymap::Command;
use crate::model::{Action, AgentsView};
use crate::settings::OptionId;
use crate::theme::Icon;
use crate::typography::{Laid, Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec, Segment};

use super::sheet::{After, Skin, asked, label_column, row};

/// What a short window leaves of the pane under its title at the least.
const ROWS_MIN: f32 = 56.0;
/// The space inside a box, at its sides and over and under what it holds.
const PAD: egui::Vec2 = vec2(14.0, 12.0);
/// A segment's height, and the gap between a group's heading and what
/// follows.
const SEGMENT: f32 = 22.0;
const UNDER_HEADING: f32 = 8.0;
/// Between the groups.
const GROUP_GAP: f32 = 16.0;
/// The switch beside the title, and the gap before it.
const SWITCH: f32 = 30.0;
const SWITCH_GAP: f32 = 16.0;

/// What the toast says once an agent's setup is on the clipboard.
const COPIED_ENTRY: &str = "Copied the entry for mcp.json";

/// The pane, from the top of `ui` down. What is under its title scrolls
/// when it is taller than `limit` leaves it.
pub(super) fn content(
    ui: &mut Ui,
    app: &App,
    agent: Agent,
    limit: f32,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let settings = super::shown(app);
    title(ui, settings.mcp_enabled, skin, actions);
    ui.add_space(14.0);
    let room = (limit - ui.cursor().top()).max(ROWS_MIN);
    egui::ScrollArea::vertical()
        .id_salt("agents")
        .max_height(room)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            status(ui, app, skin, actions);
            heading(ui, "Connect an agent", skin);
            connect(ui, agent, skin, actions);
            heading(ui, "Access", skin);
            access(ui, skin, actions);
            ui.add_space(GROUP_GAP - 4.0);
            safety(ui, skin);
            heading(ui, "Log", skin);
            let column = label_column(ui, &OptionId::AGENTS, skin);
            let show_log = gettext(skin.locale, "Show log");
            let reason = gettext(skin.locale, super::NO_WINDOW);
            let after = After {
                text: &show_log,
                disabled: app.agents_window().is_none().then_some(&*reason),
                action: Action::ShowAgentsFromSettings(AgentsView::Log),
            };
            let log = OptionId::LogDays;
            row(ui, log, true, column, &settings, skin, actions, Some(after));
            let notify = OptionId::AgentsNotify;
            row(ui, notify, false, column, &settings, skin, actions, None);
        });
}

/// The pane's title and what it is for, with the server's switch at the
/// right: off is what Stop all agents was (question 11), until it is
/// turned on again.
fn title(ui: &mut Ui, on: bool, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let words = (width - SWITCH - SWITCH_GAP).max(0.0);
    let name = Text::one(
        look,
        TextRole::DialogTitle,
        &skin.say("Agents"),
        palette.text,
    )
    .layout(ui.ctx());
    let said = if cfg!(target_os = "macos") {
        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the Keychain."
    } else if cfg!(windows) {
        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in Credential Manager."
    } else {
        "Let coding agents query your databases through Tabletist (MCP). They use your saved connections; passwords stay in the keyring."
    };
    let said = skin.say(said);
    let what = Text::one(look, TextRole::UiBody, &said, palette.dim)
        .wrap(words)
        .layout(ui.ctx());
    let height = name.height() + 2.0 + what.height();
    let ground = ui.painter().add(egui::Shape::Noop);
    let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
    paint(ui, &name, pos2(rect.left(), rect.top()));
    paint(
        ui,
        &what,
        pos2(rect.left(), rect.top() + name.height() + 2.0),
    );
    let place = Rect::from_min_max(pos2(rect.right() - SWITCH, rect.top()), rect.max);
    let centred = egui::Layout::right_to_left(egui::Align::Center);
    let builder = egui::UiBuilder::new().id_salt("switch").max_rect(place);
    let mut switch = ui.new_child(builder.layout(centred));
    let named = gettext(skin.locale, super::label(OptionId::AgentsOn));
    // Described by the words under the title, as a row's switch is by
    // its small print.
    if widgets::toggle(&mut switch, on, &named, Some(&said), palette).clicked() {
        actions.push(Action::SetOption(crate::settings::OptionValue::AgentsOn(
            !on,
        )));
    }
    // Quick open's row of the switch is the title's.
    asked(ui, ground, (rect, place), OptionId::AgentsOn, skin, actions);
}

/// Paints `text` with its top left at `at`, and names it for screen
/// readers.
fn paint(ui: &Ui, text: &Laid, at: egui::Pos2) {
    text.paint(ui.painter(), at);
    widgets::announce(ui, Rect::from_min_size(at, text.size()), text.galley.text());
}

/// A group's heading, with the gap over it.
fn heading(ui: &mut Ui, text: &'static str, skin: &Skin) {
    ui.add_space(GROUP_GAP);
    widgets::section_label(&skin.say(text), skin.look, skin.palette)
        .layout(ui.ctx())
        .label(ui);
    ui.add_space(UNDER_HEADING);
}

/// A box `height` tall across the pane, its fill the panel's where
/// `filled`, with a hairline round it. Returns where it is.
fn boxed(ui: &mut Ui, height: f32, filled: bool, skin: &Skin) -> Rect {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let corner = CornerRadius::same(look.radius);
    if filled {
        ui.painter().rect_filled(rect, corner, palette.panel);
    }
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, palette.surface),
        StrokeKind::Inside,
    );
    rect
}

/// Whether the server runs, how agents reach it and who is connected, with
/// Show Agents tab: the window's in front, or with Connections in front the
/// last window's, disabled with no window open.
fn status(ui: &mut Ui, app: &App, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let role = TextRole::UiBody;
    let on = app.agents_on();
    let height = look.control_height + 2.0 * 10.0;
    let rect = boxed(ui, height, true, skin);
    let y = rect.center().y;
    let mut x = rect.left() + PAD.x;
    let dot = if on { palette.success } else { palette.dim };
    ui.painter().circle_filled(pos2(x + 4.0, y), 4.0, dot);
    x += 8.0 + 10.0;
    let (state, how) = if on {
        ("Running", "stdio · this computer only")
    } else {
        ("Off", "Agents can't connect")
    };
    x += widgets::paint_label(
        ui,
        x,
        y,
        Text::one(look, TextRole::UiBodyStrong, &skin.say(state), palette.text),
    ) + 10.0;
    let how = Text::one(look, role, &skin.say(how), palette.dim).layout(ui.ctx());
    // At the right: the button, and before it who is connected.
    let label = gettext(locale, "Show Agents tab");
    let reason = gettext(locale, super::NO_WINDOW);
    let mut button = ButtonSpec::new(&label);
    if app.agents_window().is_none() {
        button = button.disabled(&reason);
    }
    let width = button.width(ui, look);
    let place = Rect::from_center_size(
        pos2(rect.right() - PAD.x - width / 2.0, y),
        vec2(width, look.control_height),
    );
    if button.show_at(ui, place, look, palette).clicked() {
        actions.push(Action::ShowAgentsFromSettings(AgentsView::Activity));
    }
    let mut right = place.left() - 12.0;
    if on {
        let count = app.agents.sessions.len();
        let (said, color) = match count {
            0 => (
                gettext(locale, "No agents connected").into_owned(),
                palette.dim,
            ),
            count => (
                ngettext(
                    locale,
                    "{n} agent connected",
                    "{n} agents connected",
                    u32::try_from(count).unwrap_or(u32::MAX),
                )
                .replace("{n}", &count.to_string()),
                palette.success,
            ),
        };
        let text = Text::one(look, TextRole::UiBodyStrong, &said, color).layout(ui.ctx());
        // Whole or left out, as is how agents reach it.
        if right - text.width() >= x + how.width() + 12.0 {
            text.paint_right(ui.painter(), right, y);
            widgets::announce(
                ui,
                Rect::from_center_size(pos2(right - text.width() / 2.0, y), text.size()),
                &said,
            );
            right -= text.width() + 12.0;
        }
    }
    if x + how.width() <= right {
        how.paint_left(ui.painter(), x, y);
        widgets::announce(
            ui,
            Rect::from_center_size(pos2(x + how.width() / 2.0, y), how.size()),
            how.galley.text(),
        );
    }
}

/// What the switch's segment for `agent` says.
fn agent_name(agent: Agent) -> &'static str {
    match agent {
        Agent::ClaudeCode => "Claude Code",
        Agent::Cursor => "Cursor",
        Agent::Other => "Other",
    }
}

/// How `agent` takes what is under the switch.
fn agent_hint(agent: Agent) -> &'static str {
    match agent {
        Agent::ClaudeCode => "Run this once in your project:",
        Agent::Cursor => {
            "Add its tabletist entry under mcpServers in ~/.cursor/mcp.json, or in .cursor/mcp.json in a project:"
        }
        Agent::Other => "An agent that starts MCP servers over stdio runs this command:",
    }
}

/// Puts what connects `agent` on the clipboard, and says so. Nothing while
/// it is still being worked out, as the app starts.
fn copy_setup(ctx: &egui::Context, agent: Agent, locale: crate::i18n::Locale) {
    if let Some(setup) = crate::endpoint::agent_setup_ready(agent) {
        ctx.copy_text(setup.to_owned());
        let said = match agent {
            Agent::Cursor => COPIED_ENTRY,
            Agent::ClaudeCode | Agent::Other => crate::ui::agents::COPIED_COMMAND,
        };
        crate::ui::toast::say(ctx, &gettext(locale, said));
    }
}

/// The switch between the agents, what the chosen one takes, and that with
/// Copy beside it.
fn connect(ui: &mut Ui, agent: Agent, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    // The switch, and what the agent takes beside it, wrapped in what the
    // switch leaves.
    let words = Agent::ALL.map(|agent| gettext(locale, agent_name(agent)).into_owned());
    let role = TextRole::FieldLabel;
    let widest = words
        .iter()
        .map(|word| role.width(ui.ctx(), look.faces, word))
        .fold(0.0, f32::max);
    let size = vec2((widest + 20.0).ceil(), SEGMENT);
    let heading = gettext(locale, "Connect an agent");
    let names = words.each_ref().map(|word| format!("{heading}: {word}"));
    let parts: Vec<Segment<'_>> = words
        .iter()
        .zip(&names)
        .map(|(text, name)| Segment::Named { text, name })
        .collect();
    let selected = Agent::ALL
        .iter()
        .position(|shown| *shown == agent)
        .unwrap_or(0);
    let switch = 3.0 * size.x + 4.0;
    let said = skin.say(agent_hint(agent));
    let hint = Text::one(look, TextRole::Secondary, &said, palette.dim)
        .wrap((ui.available_width() - switch - 10.0).max(0.0))
        .layout(ui.ctx());
    let height = (SEGMENT + 4.0).max(hint.height());
    let (line, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let place = Rect::from_min_size(line.min, vec2(switch, height));
    let centred = egui::Layout::left_to_right(egui::Align::Center);
    let builder = egui::UiBuilder::new().id_salt("agent").max_rect(place);
    let mut row = ui.new_child(builder.layout(centred));
    // A radio group named by the heading over it, described by what the
    // chosen agent takes, which is written beside it.
    let group = widgets::Group {
        name: &heading,
        description: Some(&said),
    };
    if let Some(picked) =
        widgets::segmented(&mut row, &parts, selected, size, Some(group), look, palette)
    {
        actions.push(Action::ChooseAgentSetup(Agent::ALL[picked]));
    }
    let at = row.min_rect().right() + 10.0;
    paint(ui, &hint, pos2(at, line.center().y - hint.height() / 2.0));
    ui.add_space(8.0);

    // What the agent takes, in the code's face, a line of it at a time:
    // a long path is cut by the box, and copied whole.
    let setup = crate::endpoint::agent_setup_ready(agent).unwrap_or_else(|| {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
        "…"
    });
    let mono = TextRole::MonoSecondary;
    let line = (mono.row_height(ui.ctx(), look.faces) * 1.25).round();
    let lines = setup.lines().count().max(1);
    let label = gettext(locale, "Copy");
    let copy = ButtonSpec::new(&label).icon(Icon::Copy);
    let copy_width = copy.width(ui, look);
    let height = (lines as f32 * line + 20.0).max(look.control_height);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let code = rect.with_max_x((rect.right() - copy_width - 8.0).max(rect.left()));
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_filled(code, corner, palette.panel);
    ui.painter().rect_stroke(
        code,
        corner,
        Stroke::new(1.0, palette.surface),
        StrokeKind::Inside,
    );
    // A line gives way in its middle to a box too narrow for it.
    let fits = (code.width() - 24.0).max(0.0);
    let ctx = ui.ctx().clone();
    for (index, text) in setup.lines().enumerate() {
        let y = code.top() + 10.0 + (index as f32 + 0.5) * line;
        let shown =
            crate::ui::grid::ellipsize(text, fits, true, |text| mono.width(&ctx, look.faces, text));
        Text::one(look, mono, &shown, palette.text)
            .layout(ui.ctx())
            .paint_left(ui.painter(), code.left() + 12.0, y);
    }
    widgets::announce(ui, code, setup);
    let at = Rect::from_min_size(
        pos2(rect.right() - copy_width, rect.top()),
        vec2(copy_width, look.control_height),
    );
    if copy.show_at(ui, at, look, palette).clicked() {
        copy_setup(ui.ctx(), agent, locale);
    }
    ui.add_space(6.0);
    widgets::label(
        ui,
        TextRole::Secondary,
        &skin.say("Each agent gets its own name in the Agents tab."),
        palette.dim,
        look,
    );
}

/// Where access is set: in each connection, with the button that opens
/// Connections.
fn access(ui: &mut Ui, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin {
        look,
        palette,
        locale,
        ..
    } = *skin;
    let label = gettext(locale, "Open Connections");
    let key = crate::ui::keys::written(ui.ctx(), look, Command::Connections);
    let button = ButtonSpec::new(&label).shortcut(&key);
    let width = button.width(ui, look);
    let words = (ui.available_width() - 2.0 * PAD.x - width - 12.0).max(0.0);
    let strong = Text::one(
        look,
        TextRole::UiBodyStrong,
        &skin.say("Set for each connection, in Connections"),
        palette.text,
    )
    .wrap(words)
    .layout(ui.ctx());
    let said = Text::one(
        look,
        TextRole::Secondary,
        &skin.say("Every connection has an Agents section: Off or Read, with limits on rows and time. New connections start Off."),
        palette.dim,
    )
    .wrap(words)
    .layout(ui.ctx());
    let height = (strong.height() + 2.0 + said.height()).max(look.control_height) + 2.0 * PAD.y;
    let rect = boxed(ui, height, true, skin);
    let top = rect.center().y - (strong.height() + 2.0 + said.height()) / 2.0;
    paint(ui, &strong, pos2(rect.left() + PAD.x, top));
    paint(
        ui,
        &said,
        pos2(rect.left() + PAD.x, top + strong.height() + 2.0),
    );
    let place = Rect::from_center_size(
        pos2(rect.right() - PAD.x - width / 2.0, rect.center().y),
        vec2(width, look.control_height),
    );
    if button.show_at(ui, place, look, palette).clicked() {
        actions.push(Action::CloseDialog);
        actions.push(Action::ShowConnections);
    }
}

/// What keeps agents within bounds, in a box of its own.
fn safety(ui: &mut Ui, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let width = (ui.available_width() - 2.0 * PAD.x).max(0.0);
    let strong = Text::one(
        look,
        TextRole::UiBodyStrong,
        &skin.say("How it stays safe"),
        palette.text,
    )
    .layout(ui.ctx());
    let said = Text::one(
        look,
        TextRole::Secondary,
        &skin.say("Reads run in a read-only transaction with each connection's limits. No DDL, no multiple statements, no COPY. Agents see what the connection's database user can see, and never more."),
        palette.secondary,
    )
    .wrap(width)
    .layout(ui.ctx());
    let height = strong.height() + 6.0 + said.height() + 2.0 * PAD.y;
    let rect = boxed(ui, height, false, skin);
    let left = rect.left() + PAD.x;
    paint(ui, &strong, pos2(left, rect.top() + PAD.y));
    paint(
        ui,
        &said,
        pos2(left, rect.top() + PAD.y + strong.height() + 6.0),
    );
}
