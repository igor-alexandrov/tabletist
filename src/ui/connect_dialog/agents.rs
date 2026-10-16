//! The connection editor's Agents part: what agents may do on the
//! connection (Off or Read), the tables they may not read, how many rows
//! an answer holds, and how long a statement may run. The boards' rows but
//! masked columns, which come with 3a, and the third level, with writes.

use egui::{
    Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
};

use super::choice::{Choice, Group, choose};
use super::{Skin, input};
use crate::connections::AgentAccess;
use crate::model::{ConnectionForm, TableFocus, TablesMode};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

/// The levels, as the switch says them.
const LEVELS: [(AgentAccess, &str); 2] = [(AgentAccess::Off, "Off"), (AgentAccess::Read, "Read")];

/// The Tables control's modes, as it says them.
const MODES: [(TablesMode, &str); 3] = [
    (TablesMode::All, "All"),
    (TablesMode::AllExcept, "All except"),
    (TablesMode::Only, "Only"),
];

/// What the Tables control says under it, of a list that hides: what
/// agents are and are not told, and what a check in the app cannot stop
/// (the MCP plan's B.3). Not the board's "Hidden tables don't exist for the
/// agent": agents are told the names (question 8). Agreed on 2026-10-16.
const HIDDEN_HINT: &str = "Agents are told these names, never what is in them: the rows, the \
                           columns, and the views over them. A function from an extension can \
                           still read them. For strict isolation, use a database user without \
                           access.";

/// The same of an "Only" list, whose names are the ones agents may read.
const ONLY_HINT: &str = "Agents may read these tables and no others, the ones made later \
                         included. A function from an extension can still read the others. For \
                         strict isolation, use a database user without access.";

/// The hint for the mode chosen: none for "All", which hides nothing.
fn tables_hint(form: &ConnectionForm) -> Option<&'static str> {
    match form.agents_tables.mode {
        TablesMode::All => None,
        TablesMode::AllExcept => Some(HIDDEN_HINT),
        TablesMode::Only => Some(ONLY_HINT),
    }
}

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

/// The Tables control's switch: All, All except or Only.
fn tables_mode(ui: &mut Ui, form: &mut ConnectionForm, heading: &str, group: Group, skin: &Skin) {
    let labels = MODES.map(|(_, word)| skin.say(word));
    let options: Vec<(TablesMode, Choice<'_>)> = MODES
        .iter()
        .zip(&labels)
        .map(|((mode, _), label)| (*mode, Choice::plain(label)))
        .collect();
    let current = form.agents_tables.mode;
    if let Some(mode) = choose(
        ui,
        "agents-tables",
        Some(heading),
        &options,
        current,
        group,
        skin,
    ) {
        form.agents_tables.mode = mode;
        form.agents_table_typed = None;
        form.agents_table_error = None;
    }
}

/// The names the chosen mode lists, each with its × to take it out, and
/// Add table… after them, which opens a field for a name to be typed.
/// Enter adds the name and keeps the field open, with the keyboard, for
/// the next; a name the field still holds when it loses the keyboard is
/// added then; Escape closes the field and nothing else. A name that does
/// not read is said under the names, and the field keeps it, and the
/// keyboard, to be put right. Nothing for "All".
fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Some(names) = form.agents_tables.names() else {
        return;
    };
    let written: Vec<String> = names.iter().map(ToString::to_string).collect();
    let height = if skin.look.terminal { 21.0 } else { 24.0 };
    let mut removed = None;
    let focus = form.agents_table_focus.take();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (at, name) in written.iter().enumerate() {
            if chip(ui, name, at, height, skin) {
                removed = Some(at);
            }
        }
        match form.agents_table_typed.as_mut() {
            Some(typed) => {
                let role = widgets::code(skin.look);
                let id = ui.id().with("agents-table-typed");
                let placeholder = super::hint(ui, &skin.say("table, or schema.table"), role, skin);
                let field = ui.add_sized(
                    vec2(200.0, height),
                    input(ui, typed, role, height, skin)
                        .id(id)
                        .hint_text(placeholder),
                );
                let name = skin.say("Table to add");
                ui.ctx()
                    .accesskit_node_builder(field.id, |node| node.set_label(name.as_str()));
                if focus == Some(TableFocus::Field) {
                    field.request_focus();
                }
                let pressed = |key| ui.input(|input| input.key_pressed(key));
                if field.lost_focus() && pressed(egui::Key::Escape) {
                    // The field's own: the dialog is not closed by it.
                    ui.input_mut(|input| {
                        input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)
                    });
                    form.agents_table_typed = None;
                    form.agents_table_error = None;
                    form.agents_table_focus = Some(TableFocus::Add);
                } else if field.lost_focus() {
                    let entered = pressed(egui::Key::Enter);
                    let empty = typed.trim().is_empty();
                    match form.take_typed_table() {
                        // Enter on a name: the field stays, for the next.
                        Ok(()) if entered && !empty => {
                            form.agents_table_typed = Some(String::new());
                            form.agents_table_focus = Some(TableFocus::Field);
                        }
                        Ok(()) => form.agents_table_typed = None,
                        Err(_) if entered => form.agents_table_focus = Some(TableFocus::Field),
                        Err(_) => {}
                    }
                }
            }
            None => {
                let add = skin.say("Add table…");
                let button = ButtonSpec::new(&add).padding(8.0).role(TextRole::pick(
                    skin.look,
                    TextRole::Secondary,
                    TextRole::OBody,
                ));
                let response = button.show(ui, height, skin.look, skin.palette);
                if focus == Some(TableFocus::Add) {
                    response.request_focus();
                }
                if response.clicked() {
                    form.agents_table_typed = Some(String::new());
                    form.agents_table_focus = Some(TableFocus::Field);
                }
            }
        }
    });
    if let Some(at) = removed {
        form.agents_tables.remove(at);
    }
}

/// Why the name typed does not read, under the names: a line of its own,
/// after the row that holds them.
fn table_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
    if let Some(why) = &form.agents_table_error {
        let said = skin.say("The table's name {why}.").replace("{why}", why);
        ui.add_space(4.0);
        let room = ui.available_width();
        Text::one(
            skin.look,
            TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
            &said,
            skin.palette.danger,
        )
        .wrap(room)
        .layout(ui.ctx())
        .label(ui);
    }
}

/// A listed name, `customers ×`, bordered; the × takes it out, and screen
/// readers hear it as "Remove customers". Whether it was pressed.
fn chip(ui: &mut Ui, name: &str, at: usize, height: f32, skin: &Skin) -> bool {
    let Skin { look, palette, .. } = *skin;
    let role = widgets::code(look);
    let cross = if look.terminal { "x" } else { "×" };
    let name_width = role.width(ui.ctx(), look.faces, name);
    let cross_width = role.width(ui.ctx(), look.faces, cross);
    let (left, gap, right) = (8.0, 6.0, 6.0);
    let size = vec2(
        (left + name_width + gap + cross_width + right).ceil(),
        height,
    );
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let corner = CornerRadius::same(if look.terminal {
        0
    } else {
        look.radius.saturating_sub(2)
    });
    let painter = ui.painter().clone();
    painter.rect_filled(rect, corner, palette.panel);
    painter.rect_stroke(
        rect,
        corner,
        Stroke::new(1.0, palette.border),
        StrokeKind::Inside,
    );
    let text = Text::one(look, role, name, palette.text).layout(ui.ctx());
    let y = rect.center().y - text.galley.size().y / 2.0;
    painter.galley(egui::pos2(rect.left() + left, y), text.galley, palette.text);
    let cell = Rect::from_min_max(
        egui::pos2(rect.right() - right - cross_width - gap / 2.0, rect.top()),
        rect.right_bottom(),
    );
    let response = ui.interact(
        cell,
        ui.id().with(("agents-table-remove", at, name)),
        Sense::click(),
    );
    let label = skin.say("Remove {name}").replace("{name}", name);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), &label));
    let ink = if response.hovered() || response.has_focus() {
        palette.text
    } else {
        palette.faint
    };
    let mark = Text::one(look, role, cross, ink).layout(ui.ctx());
    let y = rect.center().y - mark.galley.size().y / 2.0;
    painter.galley(
        egui::pos2(rect.right() - right - cross_width, y),
        mark.galley,
        ink,
    );
    response.clicked()
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
        ui.add_space(12.0);
        let tables = skin.say("Tables");
        widgets::label(ui, small, &tables, palette.secondary, look);
        ui.add_space(5.0);
        tables_mode(ui, form, &tables, Group::Track(28.0), skin);
        if form.agents_tables.names().is_some() {
            ui.add_space(6.0);
            table_names(ui, form, skin);
            table_error(ui, form, skin);
        }
        if let Some(hint) = tables_hint(form) {
            ui.add_space(6.0);
            sentence(ui, small, &skin.say(hint), skin);
        }
        ui.add_space(12.0);
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
    let tables = skin.say("Tables");
    super::terminal::terminal_row_of(ui, &tables, line, skin, |ui, _| {
        tables_mode(ui, form, &tables, Group::Words, skin);
    });
    if form.agents_tables.names().is_some() {
        super::terminal::terminal_row_of(ui, "", line + 4.0, skin, |ui, _| {
            table_names(ui, form, skin);
        });
        table_error(ui, form, skin);
    }
    if let Some(hint) = tables_hint(form) {
        sentence(ui, TextRole::OSecondary, &skin.say(hint), skin);
        ui.add_space(4.0);
    }
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
