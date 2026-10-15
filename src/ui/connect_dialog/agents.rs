//! The connection editor's Agents part: what agents may do on the
//! connection (Off or Read), how many rows an answer holds, and how long a
//! statement may run. Phase 1b's rows of the boards: hidden tables come
//! with 1c, masked columns with 3a, the third level with writes.

use egui::{Align, Ui};

use super::choice::{Choice, Group, choose};
use super::{Skin, input};
use crate::connections::AgentAccess;
use crate::model::ConnectionForm;
use crate::typography::TextRole;
use crate::ui::widgets;

/// The levels, as the switch says them.
const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "Off"), (AgentAccess::Read, "Read")];

/// What the part says under its rows, from the board.
const NOTE: &str =
    "When Open read-only is on, agents get Read at most. Every connection starts with agents Off.";

/// The access switch, as a group of choices headed `heading`.
fn access(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, skin: &Skin) {
    let labels = LEVELS.map(|(_, word)| skin.say(word));
    let options: Vec<(AgentAccess, Choice<'_>)> = LEVELS
        .iter()
        .zip(&labels)
        .map(|((level, _), label)| (*level, Choice::plain(label)))
        .collect();
    let current = form.agents.access;
    if let Some(level) = choose(
        ui,
        "agents-access",
        Some(heading),
        &options,
        current,
        group,
        skin,
    ) {
        form.agents.access = level;
    }
}

/// "Agents use this connection as dev.": the environment an agent's
/// session opens as. Nothing for a connection with none.
fn runs_as(skin: &Skin) -> Option<String> {
    let named = skin.environment.named(crate::env::Platform::Native)?;
    Some(
        skin.say("Agents use this connection as {env}.")
            .replace("{env}", &named),
    )
}

/// A sentence of the part, wrapped to the room it has: no line may widen
/// the dialog.
fn sentence(ui: &mut Ui, role: TextRole, text: &str, skin: &Skin) {
    let room = ui.available_width();
    crate::typography::Text::one(skin.look, role, text, skin.palette.dim)
        .wrap(room)
        .layout(ui.ctx())
        .label(ui);
}

/// Brings the whole part into sight, where the dialog was opened at it
/// or a value of it is wrong: asked for on each frame that is seen, from
/// `top` to where the part ends, until it is in sight, as the dialog's
/// first frame only measures it.
fn shown(ui: &Ui, form: &mut ConnectionForm, top: f32) {
    if !form.show_agents || !ui.is_visible() || ui.is_sizing_pass() {
        return;
    }
    let part = egui::Rect::from_min_max(
        egui::pos2(ui.min_rect().left(), top),
        egui::pos2(ui.min_rect().right(), ui.cursor().top()),
    );
    let clip = ui.clip_rect();
    // A part taller than the room is in sight from its top.
    let seen = clip.contains_rect(part)
        || (part.height() > clip.height() && (part.top() - clip.top()).abs() < 1.0);
    if seen {
        form.show_agents = false;
    } else {
        ui.scroll_to_rect(part, Some(Align::TOP));
    }
}

/// macOS: the "Agents" fieldset after Safety.
pub(super) fn sheet(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Skin { look, palette, .. } = *skin;
    let top = ui.cursor().top();
    super::sheet::fieldset(ui, &skin.say("Agents"), skin, |ui| {
        let small = TextRole::Secondary;
        let heading = skin.say("Access through Tabletist (MCP)");
        widgets::label(ui, small, &heading, palette.secondary, look);
        ui.add_space(5.0);
        access(ui, form, &heading, Group::Track(28.0), skin);
        if let Some(said) = runs_as(skin) {
            ui.add_space(6.0);
            sentence(ui, small, &said, skin);
        }
        ui.add_space(10.0);
        let (role, height) = (skin.value_role(), skin.field_height());
        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
        super::row(ui, 12.0, &columns, |ui, column| {
            let (label, text) = match column {
                0 => (skin.say("Rows per answer"), &mut form.agents_rows),
                _ => (
                    skin.say("Statement timeout, seconds"),
                    &mut form.agents_timeout,
                ),
            };
            let label = widgets::label(ui, small, &label, palette.secondary, look);
            ui.add_space(5.0);
            ui.add(input(ui, text, role, height, skin))
                .labelled_by(label.id);
        });
        ui.add_space(8.0);
        sentence(ui, small, &skin.say(NOTE), skin);
    });
    shown(ui, form, top);
}

/// The terminal look: an "agents" section after Safety.
pub(super) fn terminal(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let look = skin.look;
    let top = ui.cursor().top();
    super::terminal::terminal_section(ui, &skin.say("Agents"), skin);
    let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
    let heading = skin.say("Access");
    super::terminal::terminal_row_of(ui, &heading, line, skin, |ui, _| {
        access(ui, form, &heading, Group::Words, skin);
    });
    let (role, height) = (TextRole::OBody, skin.field_height());
    super::terminal::terminal_row(ui, &skin.say("Rows per answer"), skin, |ui, label| {
        ui.add(input(ui, &mut form.agents_rows, role, height, skin))
            .labelled_by(label);
    });
    super::terminal::terminal_row(ui, &skin.say("Timeout, seconds"), skin, |ui, label| {
        ui.add(input(ui, &mut form.agents_timeout, role, height, skin))
            .labelled_by(label);
    });
    let mut said = skin.say(NOTE);
    if let Some(runs_as) = runs_as(skin) {
        said = format!("{runs_as} {said}");
    }
    ui.add_space(4.0);
    sentence(ui, TextRole::OSecondary, &said, skin);
    shown(ui, form, top);
}
