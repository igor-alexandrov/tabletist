//! The connection editor's Agents part: what agents may do on the
//! connection (Off or Read), the tables they may not read, the columns
//! whose values they get masked, how many rows an answer holds, and how
//! long a statement may run. The boards' rows but the third level, which
//! comes with writes. A Files connection keeps no hidden tables and no
//! masked columns: its part has the access, the rows and the timeout.

use egui::{
    Align, CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, vec2,
};

use super::choice::{Choice, Group, choose};
use super::{Skin, input};
use tabletist_db::Driver;

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

/// What the Masked columns control says under it: what agents get of a
/// masked column, and what they may not do with one (the MCP plan's B.4).
/// Not the board's "The agent sees the name and type; values come back as
/// •••." alone: a statement that uses one otherwise is refused, wherever a
/// column of its name is. Agreed on 2026-10-18.
const MASKED_HINT: &str = "Agents see the name and type; values come back as •••. A query that \
                           filters, joins, sorts or computes with one is refused, in any table \
                           with a column of that name.";

/// The hint for the mode chosen: none for "All", which hides nothing.
fn tables_hint(form: &ConnectionForm) -> Option<&'static str> {
    match form.agents_tables.mode {
        TablesMode::All => None,
        TablesMode::AllExcept => Some(HIDDEN_HINT),
        TablesMode::Only => Some(ONLY_HINT),
    }
}

/// What the part says under its rows, from the board.
const NOTE: &str = "When Open read-only is on, agents get Read at most. A new connection starts with agents Off, \
     a Files one with Read.";

/// What a Files connection's part says under its rows: agents read, and
/// the files are never theirs to change.
const FILES_NOTE: &str = "Agents can query the files, never change them.";

/// Whether the connection keeps the Tables and Masked columns controls:
/// a Files connection has no catalog that would hold them to a statement.
fn guards_tables(form: &ConnectionForm) -> bool {
    match form.driver {
        Driver::Postgres | Driver::MySql | Driver::Sqlite => true,
        Driver::Files => false,
    }
}

/// The note under the part's rows.
fn note(form: &ConnectionForm) -> &'static str {
    match guards_tables(form) {
        true => NOTE,
        false => FILES_NOTE,
    }
}

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
        // The person's choice: a type's default no longer moves it.
        form.agents_chosen = true;
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
        // A name typed for the list shown goes to it first, as when the
        // field loses the keyboard; one that does not read keeps the field,
        // and the mode, until it is put right.
        if form.take_typed_table().is_err() {
            form.agents_table_focus = Some(TableFocus::Field);
            return;
        }
        form.agents_tables.mode = mode;
        form.agents_table_typed = None;
        form.agents_table_error = None;
    }
}

/// The names the chosen mode lists, each with its × to take it out, and
/// Add table… after them, which opens a field for a name to be typed.
/// Enter adds the name and keeps the field open, with the keyboard, for
/// the next; a name the field still holds when it loses the keyboard is
/// added then; the form's Cancel closes the field and nothing else. A
/// name that does not read is said under the names, and the field keeps
/// it to be put right; after Enter it keeps the keyboard too, and a Tab or
/// a click elsewhere goes where it was going. Nothing for "All".
fn table_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let Some(names) = form.agents_tables.names() else {
        return;
    };
    let written: Vec<String> = names.iter().map(ToString::to_string).collect();
    // Which are in no table of the schemas an open window loaded.
    let missing: Vec<bool> = names
        .iter()
        .map(|name| {
            form.agents_loaded
                .as_ref()
                .and_then(|loaded| loaded.has(name))
                == Some(false)
        })
        .collect();
    let height = if skin.look.terminal { 21.0 } else { 24.0 };
    let mut removed = None;
    let focus = form.agents_table_focus.take();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (at, (name, missing)) in written.iter().zip(&missing).enumerate() {
            if chip(ui, "agents-table", name, at, *missing, height, skin) {
                removed = Some(at);
            }
        }
        let only = form.agents_tables.mode == TablesMode::Only;
        let listed = form.agents_tables.names().cloned().unwrap_or_default();
        let offered_for = |form: &ConnectionForm, typed: &str| {
            form.agents_loaded
                .as_ref()
                .map(|loaded| loaded.offered(typed, &listed, only))
                .unwrap_or_default()
        };
        match form.agents_table_typed.clone() {
            Some(before) => {
                let role = widgets::code(skin.look);
                let id = ui.id().with("agents-table-typed");
                let list = id.with("list");
                // Up and Down move in the list, taken before the field
                // reads them, so that they do not move its caret too.
                let offered = offered_for(form, &before);
                let rows: Vec<String> = offered.iter().map(ToString::to_string).collect();
                follow_pick(ui, list, &rows, &mut form.agents_table_pick);
                if !offered.is_empty() && ui.memory(|memory| memory.has_focus(id)) {
                    let step = moved(ui, skin);
                    if step != 0 {
                        let last = offered.len() as isize - 1;
                        form.agents_table_pick = Some(match form.agents_table_pick {
                            None if step > 0 => 0,
                            None => last as usize,
                            Some(at) => (at as isize + step).rem_euclid(last + 1) as usize,
                        });
                    }
                }
                let placeholder = super::hint(ui, &skin.say("table, or schema.table"), role, skin);
                let Some(typed) = form.agents_table_typed.as_mut() else {
                    return;
                };
                let field = ui.add_sized(
                    vec2(200.0, height),
                    input(ui, typed, role, height, skin)
                        .id(id)
                        .hint_text(placeholder),
                );
                let typed = typed.clone();
                if field.changed() {
                    form.agents_table_pick = None;
                }
                // Matched again only where the field changed what it holds.
                let offered = match typed == before {
                    true => offered,
                    false => offered_for(form, &typed),
                };
                if form.agents_table_pick.is_some_and(|at| at >= offered.len()) {
                    form.agents_table_pick = None;
                }
                if focus == Some(TableFocus::Field) {
                    field.request_focus();
                }
                let on_list = released_on(ui, list);
                // Only while the field has the keyboard, or lets it go this
                // frame (to a press on the list): a field left open with a
                // name that does not read lists nothing over the fields
                // below it.
                let listing = field.has_focus() || field.lost_focus();
                let clicked = match offered.is_empty() || !listing {
                    true => None,
                    false => {
                        let rows: Vec<String> = offered.iter().map(ToString::to_string).collect();
                        let note = skin.say("Tables loaded in the sidebar");
                        let pick = form.agents_table_pick;
                        offered_list(ui, list, field.rect, &rows, pick, &note, skin)
                    }
                };
                // Screen readers hear the table the keyboard is on.
                let name = skin.say("Table to add");
                let chosen = form
                    .agents_table_pick
                    .map(|at| list.with(at).accesskit_id());
                ui.ctx().accesskit_node_builder(field.id, |node| {
                    node.set_label(name.as_str());
                    if let Some(chosen) = chosen {
                        node.set_active_descendant(chosen);
                    }
                });
                let pressed = |key| ui.input(|input| input.key_pressed(key));
                let cancelled = cancel_asked(ui, &field, skin);
                if let Some(at) = clicked {
                    form.agents_table_typed = Some(offered[at].to_string());
                    form.agents_table_pick = None;
                    if form.take_typed_table().is_ok() {
                        form.agents_table_typed = Some(String::new());
                    }
                    form.agents_table_focus = Some(TableFocus::Field);
                } else if cancelled {
                    field.surrender_focus();
                    form.agents_table_pick = None;
                    form.agents_table_typed = None;
                    form.agents_table_error = None;
                    form.agents_table_focus = Some(TableFocus::Add);
                } else if field.lost_focus() && on_list && !offered.is_empty() {
                    // Pressed on the list, not on a table: the keyboard
                    // goes back to the field.
                    form.agents_table_focus = Some(TableFocus::Field);
                } else if field.lost_focus() {
                    let entered = pressed(egui::Key::Enter);
                    let empty = typed.trim().is_empty() && form.agents_table_pick.is_none();
                    // The table chosen in the list, or what is typed.
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

/// The masked columns, each with its × to take it out, and Add column…
/// after them, which opens a field for a name, as Add table… does. Its
/// list offers the tables the sidebar loaded; one chosen goes into the
/// field, `orders.`, and the list offers its columns, fetched from an open
/// window, to be masked.
fn column_names(ui: &mut Ui, form: &mut ConnectionForm, skin: &Skin) {
    let written: Vec<String> = form.agents_masked.iter().map(ToString::to_string).collect();
    // Which name no column of the database, as the windows know it.
    let missing: Vec<bool> = form
        .agents_masked
        .iter()
        .map(|entry| {
            form.agents_loaded
                .as_ref()
                .and_then(|loaded| loaded.has_column(entry))
                == Some(false)
        })
        .collect();
    let height = if skin.look.terminal { 21.0 } else { 24.0 };
    let mut removed = None;
    let focus = form.agents_column_focus.take();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        for (at, (name, missing)) in written.iter().zip(&missing).enumerate() {
            if chip(ui, "agents-column", name, at, *missing, height, skin) {
                removed = Some(at);
            }
        }
        let Some(before) = form.agents_column_typed.clone() else {
            let add = skin.say("Add column…");
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
                form.agents_column_typed = Some(String::new());
                form.agents_column_focus = Some(TableFocus::Field);
            }
            return;
        };
        let role = widgets::code(skin.look);
        let id = ui.id().with("agents-column-typed");
        let list = id.with("list");
        let offered = form.columns_offered(&before);
        let rows: Vec<String> = offered.rows.iter().map(ToString::to_string).collect();
        follow_pick(ui, list, &rows, &mut form.agents_column_pick);
        if !offered.rows.is_empty() && ui.memory(|memory| memory.has_focus(id)) {
            let step = moved(ui, skin);
            if step != 0 {
                let last = offered.rows.len() as isize - 1;
                form.agents_column_pick = Some(match form.agents_column_pick {
                    None if step > 0 => 0,
                    None => last as usize,
                    Some(at) => (at as isize + step).rem_euclid(last + 1) as usize,
                });
            }
        }
        let placeholder = super::hint(ui, &skin.say("table.column"), role, skin);
        let Some(typed) = form.agents_column_typed.as_mut() else {
            return;
        };
        let field = ui.add_sized(
            vec2(200.0, height),
            input(ui, typed, role, height, skin)
                .id(id)
                .hint_text(placeholder),
        );
        let typed = typed.clone();
        if field.changed() {
            form.agents_column_pick = None;
        }
        let offered = match typed == before {
            true => offered,
            false => form.columns_offered(&typed),
        };
        if form
            .agents_column_pick
            .is_some_and(|at| at >= offered.rows.len())
        {
            form.agents_column_pick = None;
        }
        if focus == Some(TableFocus::Field) {
            field.request_focus();
        }
        let on_list = released_on(ui, list);
        // What the list says it offers: the tables the sidebar loaded, or
        // the columns of the one typed, which may be on their way or may
        // not have come.
        let among = |objects: &[tabletist_db::ObjectRef]| {
            offered.tables.iter().any(|object| objects.contains(object))
        };
        let (loading, failed) = form
            .agents_loaded
            .as_ref()
            .map(|loaded| {
                (
                    among(&loaded.columns_loading),
                    among(&loaded.columns_failed),
                )
            })
            .unwrap_or_default();
        let note = match &offered.of {
            None => skin.say("Tables loaded in the sidebar"),
            Some(table) => {
                let said = match (loading, failed) {
                    (true, _) => skin.say("Loading the columns of {table}…"),
                    (false, true) if offered.rows.is_empty() => {
                        skin.say("The columns of {table} could not be read")
                    }
                    _ => skin.say("Columns of {table}"),
                };
                said.replace("{table}", &table.to_string())
            }
        };
        let shows = !offered.rows.is_empty() || loading || failed;
        let listing = field.has_focus() || field.lost_focus();
        let clicked = match shows && listing {
            false => None,
            true => {
                let rows: Vec<String> = offered.rows.iter().map(ToString::to_string).collect();
                let pick = form.agents_column_pick;
                offered_list(ui, list, field.rect, &rows, pick, &note, skin)
            }
        };
        // Screen readers hear the row the keyboard is on.
        let name = skin.say("Column to mask");
        let chosen = form
            .agents_column_pick
            .map(|at| list.with(at).accesskit_id());
        ui.ctx().accesskit_node_builder(field.id, |node| {
            node.set_label(name.as_str());
            if let Some(chosen) = chosen {
                node.set_active_descendant(chosen);
            }
        });
        let pressed = |key| ui.input(|input| input.key_pressed(key));
        let cancelled = cancel_asked(ui, &field, skin);
        // Enter on a table, or a click on one, puts it in the field for its
        // columns, the caret after it.
        let into = |form: &mut ConnectionForm, at: usize| match &offered.rows[at] {
            crate::model::OfferedColumn::Table(table) => {
                let typed = format!("{table}.");
                let mut state =
                    egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
                let end = egui::text::CCursor::new(typed.chars().count());
                state
                    .cursor
                    .set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
                form.agents_column_typed = Some(typed);
                form.agents_column_pick = None;
                form.agents_column_focus = Some(TableFocus::Field);
                true
            }
            crate::model::OfferedColumn::Column(_) => false,
        };
        if let Some(at) = clicked {
            if !into(form, at) {
                form.agents_column_typed = Some(offered.rows[at].to_string());
                form.agents_column_pick = None;
                if form.take_typed_column().is_ok() {
                    form.agents_column_typed = Some(String::new());
                }
                form.agents_column_focus = Some(TableFocus::Field);
            }
        } else if cancelled {
            field.surrender_focus();
            form.agents_column_pick = None;
            form.agents_column_typed = None;
            form.agents_column_error = None;
            form.agents_column_focus = Some(TableFocus::Add);
        } else if field.lost_focus() && on_list && shows {
            form.agents_column_focus = Some(TableFocus::Field);
        } else if field.lost_focus() {
            let entered = pressed(egui::Key::Enter);
            let pick = form.agents_column_pick;
            if entered && pick.is_some_and(|at| into(form, at)) {
                return;
            }
            let empty = typed.trim().is_empty() && pick.is_none();
            match form.take_typed_column() {
                Ok(()) if entered && !empty => {
                    form.agents_column_typed = Some(String::new());
                    form.agents_column_focus = Some(TableFocus::Field);
                }
                Ok(()) => form.agents_column_typed = None,
                Err(_) if entered => form.agents_column_focus = Some(TableFocus::Field),
                Err(_) => {}
            }
        }
    });
    if let Some(at) = removed
        && at < form.agents_masked.len()
    {
        form.agents_masked.remove(at);
    }
}

/// Under masked columns one of which names no column of the database:
/// what that does to agents (the MCP plan's C.3), and how to put it right.
fn column_missing(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
    let Some(loaded) = &form.agents_loaded else {
        return;
    };
    if !form
        .agents_masked
        .iter()
        .any(|entry| loaded.has_column(entry) == Some(false))
    {
        return;
    }
    let said = skin.say(
        "Agents may not use this connection while a masked column is not in its database. Take \
         the name out, or add the column's new name.",
    );
    ui.add_space(4.0);
    let room = ui.available_width();
    Text::one(
        skin.look,
        TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
        &said,
        skin.palette.warning,
    )
    .wrap(room)
    .layout(ui.ctx())
    .label(ui);
}

/// Why the column's name typed does not read, under the names.
fn column_error(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
    if let Some(why) = &form.agents_column_error {
        let said = skin.say("The column's name {why}.").replace("{why}", why);
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

/// Keeps the row the keyboard is on in a list whose rows changed under
/// it (a table's columns came, a sidebar loaded more): the same row where
/// it is still offered, none where it is not, so that Enter adds what was
/// chosen. `rows` are the ones offered now; the last ones are kept with
/// `list`.
fn follow_pick(ui: &Ui, list: egui::Id, rows: &[String], pick: &mut Option<usize>) {
    let kept = list.with("rows");
    let before: Option<Vec<String>> = ui.data(|data| data.get_temp(kept));
    if let (Some(before), Some(at)) = (before, *pick)
        && before != rows
    {
        *pick = before
            .get(at)
            .and_then(|row| rows.iter().position(|now| now == row));
    }
    ui.data_mut(|data| data.insert_temp(kept, rows.to_vec()));
}

/// Whether the form's Cancel, as the keymap has it, was pressed while
/// `field` has the keyboard or has just let it go: the field's own, taken
/// out of the frame so that the dialog is not closed by it.
fn cancel_asked(ui: &Ui, field: &egui::Response, skin: &Skin) -> bool {
    (field.has_focus() || field.lost_focus()) && {
        let keymap = crate::ui::keys::published(ui.ctx());
        let layout = crate::keymap::Layout::of(skin.look);
        ui.input_mut(|input| {
            let cancel = crate::keymap::Command::FormCancel;
            let scope = crate::keymap::Scope::Prompt;
            crate::ui::keys::asked(input, &keymap, layout, cancel, scope, true).count > 0
        })
    }
}

/// Whether the pointer was released over the list `list` this frame. A
/// release there, where the field gives up the keyboard, is no leaving it:
/// the list was last seen there. Only a release this frame: a pointer that
/// rests there says nothing of a Tab.
fn released_on(ui: &Ui, list: egui::Id) -> bool {
    let released = ui.input(|input| input.pointer.any_released());
    released
        && ui
            .ctx()
            .memory(|memory| memory.area_rect(list))
            .zip(ui.input(|input| input.pointer.latest_pos()))
            .is_some_and(|(rect, at)| rect.contains(at))
}

/// How far Add table…'s list was asked to move this frame: up by one,
/// down by one, or not, by the keys the keymap gives the form for it, as
/// quick open's list reads its own.
fn moved(ui: &mut Ui, skin: &Skin) -> isize {
    use crate::keymap::{Command, Layout, Scope};
    let keymap = crate::ui::keys::published(ui.ctx());
    let layout = Layout::of(skin.look);
    ui.input_mut(|input| {
        let mut step = 0;
        for keys in keymap.chords_now(layout, Command::FormTableMove, Scope::Prompt, true) {
            let way = if keys == "up" { -1 } else { 1 };
            if crate::ui::keys::presses(input, layout, keys).count > 0 {
                step += way;
            }
        }
        step
    })
}

/// The rows offered for what is typed, floating under the field at
/// `under`, the one the keyboard is on (`pick`) marked, with a line, `note`,
/// that says where they come from. The place of the one clicked.
fn offered_list(
    ui: &mut Ui,
    id: egui::Id,
    under: Rect,
    offered: &[String],
    pick: Option<usize>,
    note: &str,
    skin: &Skin,
) -> Option<usize> {
    let Skin { look, palette, .. } = *skin;
    let role = widgets::code(look);
    let row = if look.terminal { 21.0 } else { 24.0 };
    let width = under.width().max(240.0);
    let mut clicked = None;
    // Under the field where it has the room, else over it: never on it.
    let tall = offered.len() as f32 * row + 40.0;
    let room = ui.ctx().content_rect().bottom() - under.bottom();
    let (pivot, at) = match room >= tall + 8.0 {
        true => (egui::Align2::LEFT_TOP, under.left_bottom() + vec2(0.0, 4.0)),
        false => (egui::Align2::LEFT_BOTTOM, under.left_top() - vec2(0.0, 4.0)),
    };
    egui::Area::new(id)
        .order(egui::Order::Tooltip)
        .pivot(pivot)
        .fixed_pos(at)
        .show(ui.ctx(), |ui| {
            let corner = CornerRadius::same(look.radius);
            let border = if look.terminal {
                palette.accent
            } else {
                palette.border
            };
            // As the board's popovers: a soft shadow on macOS, none in the
            // terminal look, whose list stands out by its accent border.
            let shadow = match look.terminal {
                true => egui::epaint::Shadow::NONE,
                false => egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(36),
                },
            };
            egui::Frame::new()
                .fill(palette.panel)
                .stroke(Stroke::new(1.0, border))
                .corner_radius(corner)
                .shadow(shadow)
                .inner_margin(egui::Margin::same(4))
                .show(ui, |ui| {
                    ui.set_width(width);
                    for (at, written) in offered.iter().enumerate() {
                        let (rect, _) = ui.allocate_exact_size(vec2(width, row), Sense::hover());
                        let response = ui.interact(rect, id.with(at), Sense::click());
                        let chosen = pick == Some(at);
                        response.widget_info(|| {
                            WidgetInfo::selected(
                                WidgetType::SelectableLabel,
                                ui.is_enabled(),
                                chosen,
                                written,
                            )
                        });
                        widgets::selection(ui, rect, chosen, response.hovered(), look, palette);
                        let ink = widgets::selection_text(chosen, look, palette);
                        let text = Text::one(look, role, written, ink).layout(ui.ctx());
                        let y = rect.center().y - text.galley.size().y / 2.0;
                        ui.painter()
                            .galley(egui::pos2(rect.left() + 8.0, y), text.galley, ink);
                        if response.clicked() {
                            clicked = Some(at);
                        }
                    }
                    let small = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
                    ui.add_space(2.0);
                    widgets::label(ui, small, note, palette.faint, look);
                });
        });
    clicked
}

/// Under an "All except" list that names a table not in the database: what
/// that does to agents (the MCP plan's C.3), and how to put it right.
fn table_missing(ui: &mut Ui, form: &ConnectionForm, skin: &Skin) {
    let Some(loaded) = &form.agents_loaded else {
        return;
    };
    let gone = match form.agents_tables.mode {
        TablesMode::AllExcept => form
            .agents_tables
            .hidden
            .iter()
            .any(|name| loaded.has(name) == Some(false)),
        TablesMode::All | TablesMode::Only => false,
    };
    if !gone {
        return;
    }
    let said = skin.say(
        "Agents may not use this connection while a hidden table is not in its database. Take \
         the name out, or add the table's new name.",
    );
    ui.add_space(4.0);
    let room = ui.available_width();
    Text::one(
        skin.look,
        TextRole::pick(skin.look, TextRole::Secondary, TextRole::OSecondary),
        &said,
        skin.palette.warning,
    )
    .wrap(room)
    .layout(ui.ctx())
    .label(ui);
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
fn chip(
    ui: &mut Ui,
    kind: &str,
    name: &str,
    at: usize,
    missing: bool,
    height: f32,
    skin: &Skin,
) -> bool {
    let Skin { look, palette, .. } = *skin;
    let role = widgets::code(look);
    let cross = if look.terminal { "x" } else { "×" };
    let name_width = role.width(ui.ctx(), look.faces, name);
    let cross_width = role.width(ui.ctx(), look.faces, cross);
    // A name in no table of the database says so after itself.
    let mark = skin.say("not in this database");
    let mark_role = TextRole::pick(look, TextRole::Secondary, TextRole::OSecondary);
    let mark_width = match missing {
        true => 8.0 + mark_role.width(ui.ctx(), look.faces, &mark),
        false => 0.0,
    };
    let (left, gap, right) = (8.0, 6.0, 6.0);
    let size = vec2(
        (left + name_width + mark_width + gap + cross_width + right).ceil(),
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
    if missing {
        let marked = Text::one(look, mark_role, &mark, palette.warning).layout(ui.ctx());
        let x = rect.left() + left + name_width + 8.0;
        let y = rect.center().y - marked.galley.size().y / 2.0;
        painter.galley(egui::pos2(x, y), marked.galley, palette.warning);
        // Heard as the name and the mark.
        let place = Rect::from_min_max(rect.left_top(), egui::pos2(x + mark_width, rect.bottom()));
        let said = format!("{name}, {mark}");
        ui.interact(
            place,
            ui.id().with((kind, "missing", at, name)),
            Sense::hover(),
        )
        .widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
    }
    let cell = Rect::from_min_max(
        egui::pos2(rect.right() - right - cross_width - gap / 2.0, rect.top()),
        rect.right_bottom(),
    );
    let response = ui.interact(
        cell,
        ui.id().with((kind, "remove", at, name)),
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
        // Tables and Masked columns side by side, as the board has them.
        let columns = [super::Column::Fr(1.0), super::Column::Fr(1.0)];
        let guards = guards_tables(form);
        super::row(ui, 12.0, &columns, |ui, column| match column {
            _ if !guards => {}
            0 => {
                let tables = skin.say("Tables");
                widgets::label(ui, small, &tables, palette.secondary, look);
                ui.add_space(5.0);
                tables_mode(ui, form, &tables, Group::Track(28.0), skin);
                if form.agents_tables.names().is_some() {
                    ui.add_space(6.0);
                    table_names(ui, form, skin);
                    table_error(ui, form, skin);
                    table_missing(ui, form, skin);
                }
                if let Some(hint) = tables_hint(form) {
                    ui.add_space(6.0);
                    sentence(ui, small, &skin.say(hint), skin);
                }
            }
            _ => {
                let masked = skin.say("Masked columns");
                widgets::label(ui, small, &masked, palette.secondary, look);
                ui.add_space(5.0);
                column_names(ui, form, skin);
                column_error(ui, form, skin);
                column_missing(ui, form, skin);
                ui.add_space(6.0);
                sentence(ui, small, &skin.say(MASKED_HINT), skin);
            }
        });
        if guards {
            ui.add_space(12.0);
        }
        let (role, height) = (skin.value_role(), skin.field_height());
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
        sentence(ui, small, &skin.say(note(form)), skin);
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
    if guards_tables(form) {
        terminal_guards(ui, form, line, skin);
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
    let mut said = skin.say(note(form));
    if let Some(runs_as) = runs_as(skin) {
        said = format!("{runs_as} {said}");
    }
    ui.add_space(4.0);
    sentence(ui, TextRole::OSecondary, &said, skin);
    shown(ui, form, top);
}

/// The terminal look's Tables and Masked rows.
fn terminal_guards(ui: &mut Ui, form: &mut ConnectionForm, line: f32, skin: &Skin) {
    let tables = skin.say("Tables");
    super::terminal::terminal_row_of(ui, &tables, line, skin, |ui, _| {
        tables_mode(ui, form, &tables, Group::Words, skin);
    });
    if form.agents_tables.names().is_some() {
        super::terminal::terminal_row_of(ui, "", line + 4.0, skin, |ui, _| {
            table_names(ui, form, skin);
        });
        table_error(ui, form, skin);
        table_missing(ui, form, skin);
    }
    if let Some(hint) = tables_hint(form) {
        sentence(ui, TextRole::OSecondary, &skin.say(hint), skin);
        ui.add_space(4.0);
    }
    let masked = skin.say("Masked");
    super::terminal::terminal_row_of(ui, &masked, line + 4.0, skin, |ui, _| {
        column_names(ui, form, skin);
    });
    column_error(ui, form, skin);
    column_missing(ui, form, skin);
    sentence(ui, TextRole::OSecondary, &skin.say(MASKED_HINT), skin);
    ui.add_space(4.0);
}
