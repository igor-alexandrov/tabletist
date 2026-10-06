//! The row panel: every field of the selected row, in full.

use egui::{
    CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, TextEdit, WidgetInfo, WidgetType,
    pos2, vec2,
};
use std::sync::Arc;

use tabletist_db::{Value, ValueKind};

use crate::app::App;
use crate::edit::{Editor, Lock, State};
use crate::i18n::gettext;
use crate::model::{Action, CellPos, ConnTabId, RowFields, Tab, TabId, Workspace};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Region};
use crate::ui::format;
use crate::ui::json_view;
use crate::ui::row_form::{self, Form, Part};
use crate::ui::states::Tone;
use crate::ui::widgets;

/// The panel's width when it opens: macOS 344 and its 1 pt rule, terminal
/// 460 and its 2 pt accent edge (the design draws both outside the width).
fn width(look: &Look) -> f32 {
    if look.terminal { 462.0 } else { 345.0 }
}

/// The least width the panel's edge can be dragged to, with room to spare.
const NARROWEST: f32 = 260.0;

/// The id of the control a field of the row panel is: its value, or its
/// pencil where the value has clicks of its own (a tree, a list), or its
/// label where it has neither. The keyboard is on the field while it is
/// on this. One per column, whatever row the panel shows: the keyboard
/// stays on a column's field when the selection steps to another row.
pub fn field_stop(tab: ConnTabId, id: TabId, col: usize) -> Id {
    Id::new(("row-panel-field", tab, id, col))
}

/// The column of the panel's field that has the keyboard, of the tab's
/// `columns`: what `ui/keys.rs` asks before it gives a key to the grid.
pub fn focused_field(
    ctx: &egui::Context,
    tab: ConnTabId,
    id: TabId,
    columns: usize,
) -> Option<usize> {
    let focused = ctx.memory(|memory| memory.focused())?;
    (0..columns).find(|col| field_stop(tab, id, *col) == focused)
}
/// The most it can be dragged to.
const WIDEST: f32 = 560.0;

/// The widths the panel may have when it shares `room` points with the
/// grid or the editor beside it: never more than half, so what it sits
/// beside keeps at least as much. In a narrow window that goes under the
/// panel's least width, and the panel gives way too.
fn width_range(room: f32) -> egui::Rangef {
    let most = (room / 2.0).clamp(0.0, WIDEST);
    egui::Rangef::new(NARROWEST.min(most), most)
}

/// The width egui last drew the panel `id` at, its edge included. egui
/// does not update it while the edge is being dragged: it is the width
/// from before the drag until the pointer lets go.
fn drawn_width(ui: &egui::Ui, id: Id) -> Option<f32> {
    egui::PanelState::load(ui.ctx(), id).map(|state| state.outer_rect.width())
}

/// The panel's edge: a 1 pt rule, the terminal's 2 pt accent.
fn edge(look: &Look) -> f32 {
    if look.terminal { 2.0 } else { 1.0 }
}

/// Space at the panel's sides: macOS 16, terminal 14.
fn side(look: &Look) -> f32 {
    if look.terminal { 14.0 } else { 16.0 }
}

/// The height of one line in `role`.
fn line_of(ui: &egui::Ui, role: TextRole, look: &Look) -> f32 {
    role.row_height(ui.ctx(), look.faces)
}

/// A colour value's swatch.
const SWATCH: f32 = 14.0;

/// Between two key hints of the terminal's header.
const HINT_GAP: f32 = 12.0;

/// Field labels and notes: 11.5 in both looks.
fn caption(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::FieldLabel, TextRole::OCaption)
}

/// What the row panel knows about a column besides its name and type.
struct FieldInfo {
    key: bool,
    /// The table a single-column foreign key points at.
    target: Option<tabletist_db::ObjectRef>,
    /// The column it points at there.
    target_column: String,
}

/// A column's label: `id · int8 · primary key`, then what [`facts`] says of
/// its value (`cover · bytea · 48.2 KB · looks like JPEG`). The terminal's
/// half-width cells name a timestamp alone, as the design does.
/// Names and types come from the server, so nothing hidden in them shows.
fn label(
    name: &str,
    type_name: &str,
    kind: ValueKind,
    info: &FieldInfo,
    facts: &[String],
    look: &Look,
) -> String {
    let name = format::display_safe(name).into_owned();
    if look.terminal && kind == ValueKind::Temporal {
        return name;
    }
    let mut parts = vec![name];
    let type_name = match (kind, type_name) {
        (ValueKind::Temporal, "timestamp") => "timestamp · no tz".to_owned(),
        (ValueKind::Temporal, "timestamptz") => "timestamp · tz".to_owned(),
        _ => format::display_safe(&format::type_label(type_name, kind)).into_owned(),
    };
    parts.push(type_name);
    if info.key {
        parts.push(if look.terminal {
            "pk".into()
        } else {
            "primary key".into()
        });
    }
    parts.extend(facts.iter().cloned());
    let mut text = parts.join(" · ");
    if let (Some(target), true) = (&info.target, look.terminal) {
        text.push_str(&format!(" → {}", format::display_safe(&target.name)));
    }
    text
}

/// A text value longer than this says how long it is in its label.
const COUNTED_CHARS: usize = 120;

/// The longest array the panel lists: its text is read into elements every
/// frame it shows. A longer one is text, cut and unfolded as any long text.
const ARRAY_MAX: usize = 64 * 1024;

/// Whether `text` is an array the panel lists element by element.
fn listed(text: &str, column: &tabletist_db::ColumnMeta) -> bool {
    text.len() <= ARRAY_MAX && format::is_array(&column.type_name, column.kind)
}

/// What a field's label says of its value, after its column's name and
/// type: how many elements an array has, how big a binary value is and what
/// it starts as, how long a long text is.
fn facts(
    value: &Value,
    column: &tabletist_db::ColumnMeta,
    formatted: Option<&format::FieldText>,
    look: &Look,
    locale: crate::i18n::Locale,
) -> Vec<String> {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    match value {
        // Sixteen bytes read as a UUID: there is nothing more to say.
        Value::Bytes(bytes) if format::uuid(bytes).is_some() => Vec::new(),
        Value::Bytes(bytes) if look.terminal => std::iter::once(format::terse_size(bytes.len()))
            .chain(format::sniff(bytes).map(str::to_lowercase))
            .collect(),
        Value::Bytes(bytes) => std::iter::once(format::human_size(bytes.len()))
            .chain(format::sniff(bytes).map(|kind| format!("{} {kind}", say("looks like"))))
            .collect(),
        Value::Text(text) if listed(text, column) => format::array_items(text)
            .map(|array| {
                let count = array.items.len();
                let noun = if count == 1 { "element" } else { "elements" };
                format!("{count} {}", say(noun))
            })
            .into_iter()
            .collect(),
        Value::Text(_) => formatted
            .and_then(|field| field.characters)
            .filter(|count| *count > COUNTED_CHARS)
            .map(|count| {
                let noun = if look.terminal { "chars" } else { "characters" };
                format!("{} {}", format::group_digits(count as u64), say(noun))
            })
            .into_iter()
            .collect(),
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => Vec::new(),
    }
}

/// A row by its key: each of the `key` columns' names and the row's value
/// in it, both as the grid shows them (short, and nothing hidden). The
/// panel's title names a row so (`id 2`), and so does what a save says of
/// one. `None` when the page does not hold every column of the key.
pub fn key_parts(
    columns: &[tabletist_db::ColumnMeta],
    row: &[Value],
    key: &[String],
) -> Option<Vec<(String, String)>> {
    key.iter()
        .map(|name| {
            let col = columns.iter().position(|column| column.name == *name)?;
            Some((
                format::display_safe(name).into_owned(),
                format::cell_text(row.get(col)?).into_owned(),
            ))
        })
        .collect()
}

/// What the panel shows a row of: a table's page, or a SQL editor's result.
struct Source<'a> {
    /// The table's name, or "Query 3": under the title in the macOS header.
    name: String,
    columns: &'a [tabletist_db::ColumnMeta],
    rows: &'a [Vec<Value>],
    selection: Option<CellPos>,
    /// The table's structure, once described. A result has none: no key
    /// names its rows, no foreign key links them, and only its booleans
    /// are tags.
    structure: Option<&'a tabletist_db::Structure>,
    /// The rows before the first one here: a table's page offset.
    offset: u64,
    /// The selected row's text, formatted by the app.
    texts: Option<&'a RowFields>,
    /// Whether the row is a table's. A result's row has no editing
    /// controls under it, and no `y` key to copy from it.
    table: bool,
}

/// The source the tab `id` gives the panel: none for a closed tab, or a
/// SQL editor that shows no rows.
fn source(workspace: &Workspace, id: TabId, locale: crate::i18n::Locale) -> Option<Source<'_>> {
    match workspace.tab(id)? {
        Tab::Object(object) => {
            let (columns, rows) = match object.page() {
                Some(page) => (page.columns.as_slice(), page.rows.as_slice()),
                None => (&[][..], &[][..]),
            };
            Some(Source {
                name: format::object_title(
                    &object.object,
                    workspace.name_is_shared(&object.object),
                ),
                columns,
                rows,
                selection: object.selection,
                structure: object.structure.value.as_ref(),
                offset: object.query.offset,
                texts: object.selected_fields(),
                table: true,
            })
        }
        Tab::Sql(sql) => {
            let (columns, rows, _) = sql.shown_rows()?;
            Some(Source {
                name: format!("{} {}", gettext(locale, "Query"), sql.number),
                columns,
                rows,
                // The cell only while its row shows: under the Messages
                // pane there is no row for the panel to be about.
                selection: sql.selected_row().and(sql.selection),
                structure: None,
                offset: 0,
                texts: sql.selected_fields(),
                table: false,
            })
        }
    }
}

/// Draws the panel for the tab `id`: an object tab, or a SQL editor.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
    // The editor the panel draws is out of its tab while the panel is
    // drawn: the field edits its text, and everything else is read.
    let mut editor = row_form::take_editor(app, tab, id);
    // The field an edit ended in is owed the keyboard. The request is
    // the reducer's: the panel reads it, and says when it is met.
    let focus = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
        .and_then(|object| object.focus_field);
    draw(app, ui, (tab, id), editor.as_mut(), focus);
    row_form::put_editor(app, tab, id, editor);
}

/// The panel itself. `editor` is the tab's editor where the panel draws
/// it, and `focus` the column whose field takes the keyboard.
fn draw(
    app: &mut App,
    ui: &mut egui::Ui,
    (tab, id): (ConnTabId, TabId),
    editor: Option<&mut Editor>,
    focus: Option<usize>,
) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let value_tags = app.settings.value_tags;
    // An editor that is open has the keyboard, unless a dialog has it.
    let hold = app.dialog.is_none();
    // `za` asked to fold the documents.
    let fold = app.workspace_mut(tab).is_some_and(|workspace| {
        let asked = workspace.fold_documents == Some(id);
        if asked {
            workspace.fold_documents = None;
        }
        asked
    });
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(source) = source(workspace, id, locale) else {
        return;
    };
    let mut actions = Vec::new();
    // What ends the panel's open field, queued ahead of the rest.
    let mut ending = Vec::new();
    let panel = Id::new(("row-panel", tab.0));
    let range = width_range(ui.available_width());
    // egui remembers the width it last drew, which a small window cuts
    // down. The width the user gave the panel (or its default) is kept
    // beside it, to come back to when there is room again.
    let drawn = drawn_width(ui, panel);
    let wanted: f32 = ui.data_mut(|data| {
        *data.get_persisted_mut_or_insert_with(panel.with("wanted"), || {
            drawn.unwrap_or(width(&look))
        })
    });
    let target = wanted.clamp(range.min, range.max);
    // Only a drag makes egui's panel wider: with room again, its least
    // width is the target until it is drawn that wide.
    let least = if drawn.is_some_and(|drawn| drawn + 1.0 < target) {
        target
    } else {
        range.min
    };
    egui::Panel::right(panel)
        // With no width to choose from, the edge is no handle.
        .resizable(range.min < range.max)
        .default_size(target)
        .size_range(least..=range.max)
        .show_separator_line(false)
        // The edge sits outside the content, as the design's border does.
        .frame(
            Frame::new()
                .fill(palette.window)
                .inner_margin(egui::Margin {
                    left: edge(&look) as i8,
                    ..Default::default()
                }),
        )
        .show(ui, |ui| {
            let full = ui.max_rect();
            // The terminal marks the inspector's edge in the accent, as
            // Omarchy marks a focused window.
            if look.terminal {
                ui.painter().rect_filled(
                    Rect::from_min_size(
                        full.min - vec2(edge(&look), 0.0),
                        vec2(edge(&look), full.height()),
                    ),
                    CornerRadius::ZERO,
                    palette.accent,
                );
            } else {
                widgets::vline(ui, full.left() - 0.5, full.y_range(), palette.outline);
            }
            let selected = source
                .selection
                .and_then(|cell| source.rows.get(cell.row).map(|row| (cell, row)));
            // Formatted by the app when the selection changed, never here.
            let texts = source.texts;
            let Some((cell, row)) = selected else {
                ui.centered_and_justified(|ui| {
                    Text::one(
                        &look,
                        widgets::body(&look),
                        &gettext(locale, "Select a row to see its fields"),
                        palette.secondary,
                    )
                    .layout(ui.ctx())
                    .label(ui);
                });
                return;
            };
            // A part of the window to step to once it has a row to show.
            focus::region(ui, Region::Panel, full);
            // What editing adds, for a table's row that can be edited.
            let object = match workspace.tab(id) {
                Some(Tab::Object(object)) => Some(object),
                _ => None,
            };
            let mut form = match object {
                Some(object) => {
                    let how = (hold, look.terminal);
                    Form::of(workspace, object, tab, cell.row, editor, how)
                }
                None => Form::none(),
            };
            form.focus = focus;
            // Why no field of the row can be edited, as the panel's first
            // line says it: in the grid's own words, the terminal's in its
            // lower case.
            let locked = object.zip(form.locked).map(|(object, lock)| {
                // The design's words for the commonest of them.
                if lock == Lock::ReadOnly {
                    return look.label(&gettext(locale, "Read-only connection"));
                }
                let table = format::display_safe(&object.object.name);
                if look.terminal {
                    crate::ui::workspace::lock_line(lock, &table, &look, locale)
                } else {
                    crate::ui::cell_editor::lock_text(lock, &table, locale)
                }
            });
            let structure = source.structure;
            let info = |name: &str| {
                let key =
                    structure.is_some_and(|s| s.primary_key.iter().any(|column| column == name));
                let foreign = structure.and_then(|s| {
                    s.foreign_keys
                        .iter()
                        .find(|foreign| foreign.columns.len() == 1 && foreign.columns[0] == name)
                });
                FieldInfo {
                    key,
                    target: foreign.map(|f| {
                        tabletist_db::ObjectRef::new(f.ref_schema.clone(), f.ref_table.clone())
                    }),
                    target_column: foreign
                        .and_then(|f| f.ref_columns.first().cloned())
                        .unwrap_or_default(),
                }
            };
            // Values the grid shows as tags keep their colour here.
            let tags: Vec<_> = source
                .columns
                .iter()
                .map(|column| crate::ui::value_tags::Tags::of(column, structure).when(value_tags))
                .collect();
            let tag_of = |col: usize, value: &Value| tags[col].style(value);
            // The row's name: its key, else its number. The key a save
            // finds the row by (`row_key`), so the bar and a save's line
            // name the row as the title does: a primary key, or else a
            // unique index. Of one column only: more do not fit the title.
            let (key_column, key_value) = structure
                .and_then(tabletist_db::Structure::row_key)
                .filter(|key| key.len() == 1)
                .and_then(|key| key_parts(source.columns, row, &key))
                .and_then(|mut parts| parts.pop())
                .unzip();
            let number = source.offset + cell.row as u64 + 1;
            let side = side(&look);
            // Header: 52 (macOS) or 40 (terminal), and its rule.
            let header_height = if look.terminal { 41.0 } else { 53.0 };
            let (header, _) =
                ui.allocate_exact_size(vec2(full.width(), header_height), Sense::hover());
            let divider = if look.terminal {
                palette.outline
            } else {
                palette.surface_hover
            };
            widgets::hline(ui, header.x_range(), header.bottom() - 0.5, divider);
            let rows = source.rows.len();
            let can_prev = cell.row > 0;
            let can_next = cell.row + 1 < rows;
            if look.terminal {
                let y = header.top() + 20.0;
                let role = TextRole::OBody;
                let mut x = header.left() + side;
                x += widgets::paint_text(
                    ui,
                    x,
                    y,
                    Text::one(
                        &look,
                        TextRole::OGroup,
                        &gettext(locale, "row"),
                        palette.text,
                    ),
                ) + 10.0;
                // esc ×: 8 in from the right, 24 tall, 8 at its sides, 6
                // before the ×; the prev/next hint 10 before it.
                let small = TextRole::OSecondary;
                let esc_width =
                    8.0 + small.width(ui.ctx(), look.faces, "esc") + 6.0 + 10.0 + 8.0 + 2.0;
                let esc = Rect::from_min_size(
                    pos2(header.right() - 8.0 - esc_width, y - 12.0),
                    vec2(esc_width, 24.0),
                );
                // The row's name: its key and value, or its number.
                let keyed = key_column.as_ref().zip(key_value.as_ref());
                let whole = match keyed {
                    Some((key, value)) => format!("{key} {value}"),
                    None => number.to_string(),
                };
                let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
                // The hint gives way before the name does: first the keys
                // that reach and edit a field, which a row that can be
                // edited offers, then the words of its own keys, when the
                // whole name does not fit before them. The name is cut
                // where the hint begins, 10 before it.
                let steps: widgets::Hint<'_> = ("[ ]", "prev/next", true);
                let focus: widgets::Hint<'_> = ("ctrl+l", "focus", true);
                let editable = source.table && locked.is_none();
                let hints: [&[widgets::Hint<'_>]; 4] = [
                    &[steps, focus, ("i", "edit field", true)],
                    &[steps, focus],
                    &[steps],
                    &[("[ ]", "", true)],
                ];
                let fit = |hint: &[widgets::Hint<'_>]| {
                    let width = widgets::key_hints_width(ui, hint, HINT_GAP, &look, &palette);
                    (width, esc.left() - 10.0 - width - 10.0 - x)
                };
                let offered = &hints[if editable { 0 } else { 2 }..];
                let fitting = offered.iter().find(|hint| measure(&whole) <= fit(hint).1);
                let hint = *fitting.unwrap_or(&hints[3]);
                let (width, room) = fit(hint);
                // A number keeps its last digits.
                let shown = crate::ui::grid::ellipsize(&whole, room, keyed.is_none(), measure);
                let name = match keyed {
                    Some((key, _)) => {
                        let value = shown.strip_prefix(key.as_str());
                        match value.and_then(|value| value.strip_prefix(' ')) {
                            Some(value) => Text::one(&look, role, key, palette.dim)
                                .space(role, " ")
                                .add(role, value, palette.warning),
                            // Cut within the key.
                            None => Text::one(&look, role, &shown, palette.dim),
                        }
                    }
                    None => Text::one(&look, role, &shown, palette.warning),
                };
                widgets::paint_text(ui, x, y, name);
                let response = ui.interact(esc, ui.id().with("close"), Sense::click());
                let close = gettext(locale, "Close the row panel");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &close));
                focus::claim(ui, Region::Panel, &response);
                ui.painter().rect_stroke(
                    esc,
                    CornerRadius::same(3),
                    Stroke::new(1.0, palette.outline),
                    StrokeKind::Inside,
                );
                widgets::paint_text(
                    ui,
                    esc.left() + 9.0,
                    y,
                    Text::one(&look, small, "esc", palette.dim),
                );
                Icon::X.image(palette.dim, 10.0).paint_at(
                    ui,
                    Rect::from_center_size(pos2(esc.right() - 9.0 - 5.0, y), vec2(10.0, 10.0)),
                );
                if response.clicked() {
                    actions.push(Action::ToggleRowPanel(tab));
                }
                widgets::key_hints(
                    ui,
                    (esc.left() - 10.0 - width, y),
                    hint,
                    HINT_GAP,
                    &look,
                    &palette,
                );
                for (enabled, step, label) in
                    [(can_prev, -1, "Previous row"), (can_next, 1, "Next row")]
                {
                    let hit = Rect::from_min_size(
                        pos2(
                            esc.left() - 10.0 - width + if step < 0 { 0.0 } else { 12.0 },
                            y - 9.0,
                        ),
                        vec2(12.0, 18.0),
                    );
                    let response = ui.interact(hit, ui.id().with(("step", step)), Sense::click());
                    response.widget_info(|| {
                        WidgetInfo::labeled(WidgetType::Button, enabled, gettext(locale, label))
                    });
                    if response.clicked() && enabled {
                        actions.push(Action::MoveSelection {
                            tab,
                            id,
                            rows: step,
                            cols: 0,
                        });
                    }
                }
            } else {
                let title = match (&key_column, &key_value) {
                    (Some(key), Some(value)) => {
                        format!("{} · {key} {value}", gettext(locale, "Row"))
                    }
                    _ => format!("{} {number}", gettext(locale, "Row")),
                };
                // The title over the table's name, centred in the 52.
                let (title_role, sub_role) = (TextRole::UiBodySemibold, TextRole::FieldLabel);
                let (title_line, sub_line) =
                    (line_of(ui, title_role, &look), line_of(ui, sub_role, &look));
                let top = header.top() + (52.0 - title_line - sub_line) / 2.0;
                widgets::paint_label(
                    ui,
                    header.left() + side,
                    top + title_line / 2.0,
                    Text::one(&look, title_role, &title, palette.text),
                );
                // Under it the table's name, or how much of the row is
                // not saved yet, while anything is.
                let changed = texts.map_or(0, |texts| texts.pending.iter().flatten().count());
                let under = match changed {
                    0 => Text::one(&look, sub_role, &source.name, palette.dim),
                    _ => {
                        let what = if changed == 1 {
                            "unsaved change"
                        } else {
                            "unsaved changes"
                        };
                        let said = format!("{changed} {}", gettext(locale, what));
                        Text::one(&look, sub_role, &said, palette.warning)
                    }
                };
                widgets::paint_text(
                    ui,
                    header.left() + side,
                    top + title_line + sub_line / 2.0,
                    under,
                );
                // Three 30 pt buttons, 4 apart, 8 in from the right.
                let y = header.top() + 26.0;
                let mut right = header.right() - 8.0;
                for (icon, label, enabled, action) in [
                    (
                        Icon::X,
                        "Close the row panel",
                        true,
                        Action::ToggleRowPanel(tab),
                    ),
                    (
                        Icon::ChevronDown,
                        "Next row",
                        can_next,
                        Action::MoveSelection {
                            tab,
                            id,
                            rows: 1,
                            cols: 0,
                        },
                    ),
                    (
                        Icon::ChevronUp,
                        "Previous row",
                        can_prev,
                        Action::MoveSelection {
                            tab,
                            id,
                            rows: -1,
                            cols: 0,
                        },
                    ),
                ] {
                    let place = Rect::from_min_size(pos2(right - 30.0, y - 15.0), vec2(30.0, 30.0));
                    right -= 34.0;
                    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
                    child.add_enabled_ui(enabled, |ui| {
                        let button = widgets::icon_button_sized(
                            ui,
                            icon,
                            &gettext(locale, label),
                            vec2(30.0, 30.0),
                            14.0,
                            &look,
                            &palette,
                        );
                        // The panel starts with its first button.
                        if icon == Icon::X {
                            focus::claim(ui, Region::Panel, &button);
                        }
                        if button.clicked() {
                            actions.push(action);
                        }
                    });
                }
            }
            // A row no field of which can be edited says why, once, in a
            // line under the header: a lock, and the reason.
            let mut top = header.bottom();
            if let Some(reason) = locked.as_deref().filter(|_| source.table) {
                let role = caption(&look);
                let height = 9.0 + line_of(ui, role, &look) + 9.0 + 1.0;
                let strip = Rect::from_min_size(pos2(full.left(), top), vec2(full.width(), height));
                top = strip.bottom();
                if !look.terminal {
                    ui.painter()
                        .rect_filled(strip, CornerRadius::ZERO, palette.panel);
                }
                widgets::hline(ui, strip.x_range(), strip.bottom() - 0.5, divider);
                let y = strip.center().y - 0.5;
                let mut x = strip.left() + side;
                if !look.terminal {
                    Icon::Lock.image(palette.dim, 11.0).paint_at(
                        ui,
                        Rect::from_center_size(pos2(x + 5.5, y), vec2(11.0, 11.0)),
                    );
                    x += 17.0;
                }
                let room = strip.right() - side - x;
                // Cut at the panel's side, as a field's label is.
                let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
                let shown = crate::ui::grid::ellipsize(reason, room, false, measure);
                widgets::paint_text(ui, x, y, Text::one(&look, role, &shown, palette.dim));
                // The whole of it for a screen reader, cut or not.
                let place = Rect::from_min_size(pos2(x, y - 8.0), vec2(room.max(1.0), 16.0));
                widgets::announce(ui, place, reason);
            }
            // Footer: the row's controls. Its rule, the buttons (28 or 32)
            // and, where the row can be edited, the line under them that
            // says how (the terminal's head names its keys). A SQL result
            // has none: its rows are no table's to edit.
            let note = line_of(ui, caption(&look), &look);
            let footer_height = if !source.table {
                0.0
            } else if look.terminal {
                1.0 + 10.0 + 28.0 + 10.0
            } else if locked.is_some() {
                1.0 + 12.0 + 32.0 + 12.0
            } else {
                1.0 + 12.0 + 32.0 + 8.0 + note + 12.0
            };
            let body = Rect::from_min_max(
                pos2(full.left(), top),
                pos2(full.right(), full.bottom() - footer_height),
            );
            let foot = Rect::from_min_max(pos2(full.left(), body.bottom()), full.max);
            if source.table {
                editing_footer(ui, foot, locked.as_deref(), &look, &palette, locale);
            }
            let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body));
            let skin = FieldSkin {
                look: &look,
                palette: &palette,
                locale,
                fold,
                texts,
                copy_key: source.table,
            };
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(&mut body_ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    // A pending cell is drawn by its new value, as the grid
                    // draws it: a document, a tag, a NULL.
                    let fields: Vec<(usize, &tabletist_db::ColumnMeta, &Value)> = source
                        .columns
                        .iter()
                        .zip(row.iter())
                        .enumerate()
                        .map(|(col, (column, value))| {
                            let pending = pending_field(texts, col);
                            (col, column, pending.map_or(value, |cell| &cell.new))
                        })
                        .collect();
                    let ctx = ui.ctx().clone();
                    let is_doc = |column: &tabletist_db::ColumnMeta, value: &Value| {
                        column.kind == ValueKind::Json && !value.is_null()
                            || json_doc(&ctx, value, column.kind).is_some()
                    };
                    if look.terminal {
                        // Short fields two to a line, documents after them.
                        // Two to a line; a foreign key takes a line of its
                        // own, for its value and its link.
                        let short: Vec<_> =
                            fields.iter().filter(|(_, c, v)| !is_doc(c, v)).collect();
                        let mut lines: Vec<Vec<&(usize, &tabletist_db::ColumnMeta, &Value)>> =
                            Vec::new();
                        // Short fields fill a line two at a time; a foreign
                        // key waits for the line in progress, then takes one
                        // of its own.
                        let mut pair = Vec::new();
                        let mut waiting = Vec::new();
                        for entry in short {
                            if info(&entry.1.name).target.is_some() {
                                if pair.is_empty() {
                                    lines.push(vec![entry]);
                                } else {
                                    waiting.push(entry);
                                }
                                continue;
                            }
                            pair.push(entry);
                            if pair.len() == 2 {
                                lines.push(std::mem::take(&mut pair));
                                lines.extend(waiting.drain(..).map(|wide| vec![wide]));
                            }
                        }
                        if !pair.is_empty() {
                            lines.push(pair);
                        }
                        lines.extend(waiting.into_iter().map(|wide| vec![wide]));
                        lines.retain(|line| !line.is_empty());
                        // Two columns, each cell 8 above and below, 14 at its
                        // sides; 6 above the first line.
                        ui.add_space(6.0);
                        let column_width = ui.available_width() / 2.0;
                        for pair in &lines {
                            ui.add_space(8.0);
                            ui.horizontal_top(|ui| {
                                let wide =
                                    pair.len() == 1 && info(&pair[0].1.name).target.is_some();
                                // A sliver of a panel has no room for them.
                                let half = if wide {
                                    2.0 * column_width - 2.0 * side
                                } else {
                                    column_width - 2.0 * side
                                }
                                .max(0.0);
                                ui.add_space(side);
                                for (index, (col, column, value)) in
                                    pair.iter().copied().enumerate()
                                {
                                    // 14 after one cell and 14 before the next.
                                    if index > 0 {
                                        ui.add_space(2.0 * side);
                                    }
                                    ui.allocate_ui_with_layout(
                                        vec2(half, 0.0),
                                        egui::Layout::top_down(egui::Align::Min),
                                        |ui| {
                                            ui.set_width(half);
                                            let info = info(&column.name);
                                            field(
                                                ui,
                                                tab,
                                                id,
                                                cell,
                                                *col,
                                                column,
                                                value,
                                                &info,
                                                tag_of(*col, value),
                                                skin,
                                                &mut form,
                                                &mut actions,
                                            );
                                            was(ui, *col, &column.name, skin);
                                        },
                                    );
                                }
                            });
                            ui.add_space(8.0);
                        }
                        ui.add_space(6.0);
                        for (col, column, value) in fields.iter().filter(|(_, c, v)| is_doc(c, v)) {
                            // Documents on the window tone, under a rule.
                            let top = ui.cursor().top();
                            let area = Rect::from_min_max(
                                pos2(ui.max_rect().left(), top),
                                ui.max_rect().max,
                            );
                            ui.painter()
                                .rect_filled(area, CornerRadius::ZERO, palette.window);
                            widgets::hline(ui, ui.max_rect().x_range(), top + 0.5, palette.outline);
                            ui.add_space(12.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(side);
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - side).max(0.0));
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        id,
                                        cell,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        skin,
                                        &mut form,
                                        &mut actions,
                                    );
                                    was(ui, *col, &column.name, skin);
                                });
                            });
                            ui.add_space(12.0);
                        }
                    } else {
                        for (col, column, value) in &fields {
                            // Each field: 12 above and below, its rule under
                            // it; documents on the panel tone.
                            let top = ui.cursor().top();
                            let document = is_doc(column, value);
                            let backdrop = ui.painter().add(egui::Shape::Noop);
                            ui.add_space(12.0);
                            ui.horizontal_top(|ui| {
                                ui.add_space(side);
                                ui.vertical(|ui| {
                                    ui.set_width((ui.available_width() - side).max(0.0));
                                    let info = info(&column.name);
                                    field(
                                        ui,
                                        tab,
                                        id,
                                        cell,
                                        *col,
                                        column,
                                        value,
                                        &info,
                                        tag_of(*col, value),
                                        skin,
                                        &mut form,
                                        &mut actions,
                                    );
                                    // A document's label line is its
                                    // controls': what it was is said
                                    // under it.
                                    if document && was(ui, *col, &column.name, skin) {
                                        let cell = Some(CellPos {
                                            row: cell.row,
                                            col: *col,
                                        });
                                        actions.push(Action::RevertCell { tab, id, cell });
                                    }
                                });
                            });
                            ui.add_space(12.0);
                            let y = ui.cursor().top();
                            if document {
                                ui.painter().set(
                                    backdrop,
                                    egui::epaint::RectShape::filled(
                                        Rect::from_min_max(
                                            pos2(ui.max_rect().left(), top),
                                            pos2(ui.max_rect().right(), y),
                                        ),
                                        CornerRadius::ZERO,
                                        palette.panel,
                                    ),
                                );
                            }
                            ui.add_space(1.0);
                            widgets::hline(ui, ui.max_rect().x_range(), y + 0.5, palette.surface);
                        }
                    }
                });
            ending.append(&mut form.ending);
            // The field that was owed the keyboard was drawn and has it:
            // the reducer forgets the request. A field that was not drawn
            // (its row gone from the panel) is still owed it.
            if let (Some(col), None) = (focus, form.focus) {
                actions.push(Action::FieldFocused { tab, id, col });
            }
        });
    // The edge was dragged: that is the width wanted from now on. egui
    // stores a width only when the drag is over, so a width that did not
    // change this frame is not a drag's (a window resized mid-drag).
    if let Some(after) = drawn_width(ui, panel)
        && drawn.is_some_and(|before| before != after)
        && (after - target).abs() > 1.0
    {
        ui.data_mut(|data| data.insert_persisted(panel.with("wanted"), after));
    }
    app.actions.extend(ending);
    app.actions.extend(actions);
}

/// How fields draw.
#[derive(Clone, Copy)]
struct FieldSkin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: crate::i18n::Locale,
    /// Fold or unfold documents this frame (`za`).
    fold: bool,
    /// The selected row's text, formatted by the app.
    texts: Option<&'a crate::model::RowFields>,
    /// Whether `y` copies the selected cell: in a table, not in a result.
    copy_key: bool,
}

/// What is pending in the column `col` of the row `texts` is of.
fn pending_field(texts: Option<&RowFields>, col: usize) -> Option<&crate::model::PendingField> {
    texts?.pending.get(col)?.as_ref()
}

/// Under a pending field's value: what it loaded as, "was {loaded}". Where
/// the label's line has no room for it: a document's, and every field's in
/// the terminal look. The other looks end it in `revert`, a link: says
/// whether that was pressed.
fn was(ui: &mut egui::Ui, col: usize, column: &str, skin: FieldSkin<'_>) -> bool {
    let FieldSkin {
        look,
        palette,
        locale,
        texts,
        ..
    } = skin;
    let Some(pending) = pending_field(texts, col) else {
        return false;
    };
    ui.add_space(if look.terminal { 2.0 } else { 3.0 });
    let role = caption(look);
    let text = format!("{} {}", look.label(&gettext(locale, "was")), pending.was);
    let width = ui.available_width();
    if look.terminal {
        // Its key reverts: `u`.
        Text::one(look, role, &text, palette.dim)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        return false;
    }
    let revert = gettext(locale, "revert");
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let after = measure(" · ") + measure(&revert);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        Text::one(look, role, &text, palette.warning)
            .wrap((width - after).max(24.0))
            .layout(ui.ctx())
            .label(ui);
        Text::one(look, role, " · ", palette.warning)
            .layout(ui.ctx())
            .label(ui);
        let link = Text::one(look, role, &revert, palette.warning)
            .layout(ui.ctx())
            .label_sense(ui, Sense::click());
        let name = format!(
            "{} {}",
            gettext(locale, "Revert"),
            format::display_safe(column)
        );
        link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &name));
        widgets::hline(
            ui,
            link.rect.x_range(),
            link.rect.bottom() - 0.5,
            palette.warning,
        );
        if link.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        link.clicked()
    })
    .inner
}

/// The pending mark after a field's label: the dot a pending cell's row
/// and the pending bar wear, or the terminal's `~`.
const MARK: f32 = 6.0;

/// One field of the row of the `selected` cell: its label (with a copy
/// button, or a document's controls, and the pending mark when its cell is
/// pending), then its value. Where the row's form lets the value be edited
/// the value is one control: a click on it opens the editor in its place.
/// A value with clicks of its own (a tree, a list) has a pencil in its
/// label line instead.
#[allow(clippy::too_many_arguments)] // one call site per layout
fn field(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    tab_id: TabId,
    selected: CellPos,
    col: usize,
    column: &tabletist_db::ColumnMeta,
    value: &Value,
    info: &FieldInfo,
    tag: Option<crate::ui::grid::Style>,
    skin: FieldSkin<'_>,
    form: &mut Form<'_>,
    actions: &mut Vec<Action>,
) {
    let FieldSkin {
        look,
        palette,
        locale,
        texts,
        copy_key,
        ..
    } = skin;
    let row = selected.row;
    // The request whose answer holds the row: the same row of another
    // page or another result keeps no folds and nothing expanded.
    let request = texts.and_then(|texts| texts.request);
    let label_role = caption(look);
    let formatted = texts.and_then(|texts| texts.fields.get(col));
    let doc = json_doc(ui.ctx(), value, column.kind);
    // A document's label shares its line with the document's controls.
    let facts = if doc.is_some() {
        Vec::new()
    } else {
        facts(value, column, formatted, look, locale)
    };
    let text = label(
        &column.name,
        &column.type_name,
        column.kind,
        info,
        &facts,
        look,
    );
    let width = ui.available_width();
    // A document's label line holds its 26 pt copy button (macOS); a
    // plain label is one line of its text.
    let label_height = if doc.is_some() && !look.terminal {
        26.0
    } else {
        line_of(ui, label_role, look)
    };
    let (line, response) = ui.allocate_exact_size(vec2(width, label_height), Sense::hover());
    let name_id = response.id;
    let pending_cell = pending_field(texts, col);
    let pending = pending_cell.is_some();
    // What stands against a pending cell, in the words its cell of the
    // grid says it in: what its column refuses of the text, or what the
    // database said of its row's statement.
    let trouble = pending_cell.and_then(|cell| match &cell.state {
        State::Ready => None,
        State::ToFix(problem) => {
            let typed = match &cell.new {
                Value::Text(text) => Some(&**text),
                _ => None,
            };
            let type_name = format::type_label(&column.type_name, column.kind);
            let type_name = format::display_safe(&type_name);
            let said = crate::ui::cell_editor::problem_text(problem, &type_name, typed, locale);
            Some(said)
        }
        State::Failed(error) => Some(crate::ui::data_view::failure_text(error)),
    });
    let tone = match (&trouble, pending) {
        (Some(_), _) => Some(Tone::Danger),
        (None, true) => Some(Tone::Warning),
        (None, false) => None,
    };
    // A screen reader hears the mark as a word.
    let name = if pending {
        format!("{text}, {}", gettext(locale, "pending"))
    } else {
        text.clone()
    };
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &name));
    let part = form.part(col);
    let editable = part == Part::Editable;
    // Why this field alone cannot be edited, where it has a reason to give.
    let locked = match part {
        Part::Locked(lock) => {
            let reason = crate::ui::cell_editor::lock_text(lock, "", locale);
            (!reason.is_empty()).then_some(reason)
        }
        Part::Read | Part::Editable | Part::InGrid | Part::Editing => None,
    };
    // What a screen reader hears of the field's control: its column, its
    // type and its value, then what a pending one was and why a locked
    // one cannot be edited.
    let mut said = format!(
        "{}, {}, {}",
        format::display_safe(&column.name),
        column.type_name,
        match formatted {
            Some(text) if !value.is_null() => text.short.as_str(),
            _ => "NULL",
        },
    );
    if let Some(pending) = pending_field(texts, col) {
        let from = gettext(locale, "changed from");
        said = format!("{said}, {from} {}", pending.was);
    }
    if let Some(reason) = &locked {
        said = format!("{said}, {}: {reason}", gettext(locale, "read-only"));
    }
    // A value read as a tree or as a list has clicks of its own: its
    // pencil, in the label line, is what edits it.
    let lists = matches!(value, Value::Text(text) if listed(text, column));
    let column_name = format::display_safe(&column.name);
    let measure = |text: &str| label_role.width(ui.ctx(), look.faces, text);
    // What a pending cell was, and the way back to it, end the label's
    // line: "was print · revert". A document's line is its controls', and
    // the terminal look's has its hints: there it is said under the value.
    let was = pending_cell
        .filter(|_| !look.terminal && doc.is_none())
        .map(|cell| format!("{} {}", gettext(locale, "was"), cell.was));
    let revert = gettext(locale, "revert");
    let revert_width = measure(" · ") + measure(&revert);
    // The label gives way to it, down to a few words of what the cell was.
    let was_room = was
        .as_ref()
        .map_or(0.0, |was| measure(was).min(96.0) + revert_width + 12.0);
    // Cut at the column's edge, as the field's own width allows, before
    // the mark of a pending document, the lock of a field that cannot be
    // edited and the pencil of one that has one.
    let mark = if pending && doc.is_some() && !look.terminal {
        6.0 + MARK
    } else {
        0.0
    };
    let lock_room = if locked.is_some() { 6.0 + LOCK } else { 0.0 };
    let pencil_room = if editable && (doc.is_some() || lists) {
        PENCIL + 2.0
    } else {
        0.0
    };
    let room = line.width() - 30.0 - was_room - mark - lock_room - pencil_room;
    let shown = crate::ui::grid::ellipsize(&text, room, false, measure);
    let label_width = widgets::paint_text(
        ui,
        line.left(),
        line.center().y,
        Text::one(look, label_role, &shown, palette.dim),
    );
    if let Some(tone) = tone {
        if look.terminal {
            // The terminal look's mark stands in the gutter, as a changed
            // row's does in the grid: `~`, or `!` for a cell in trouble.
            let sign = if tone == Tone::Danger { "!" } else { "~" };
            let sign = Text::one(look, label_role, sign, tone.color(palette));
            let (_, side) = field_box(look);
            widgets::paint_text_right(ui, line.left() - side + 2.0, line.center().y, sign);
        } else if doc.is_some() {
            let x = line.left() + label_width + 6.0;
            ui.painter().circle_filled(
                pos2(x + MARK / 2.0, line.center().y),
                MARK / 2.0,
                tone.color(palette),
            );
        }
    }
    // Where the label line's Copy ends: at the line's right, or before
    // what a pending cell was.
    let mut copy_right = line.right();
    if let Some(was) = &was {
        let left = line.left() + label_width + lock_room + 12.0 + 26.0;
        let room = (line.right() - left - revert_width).max(0.0);
        let shown = crate::ui::grid::ellipsize(was, room, false, measure);
        let words = Text::one(look, label_role, &shown, palette.warning)
            .add(label_role, " · ", palette.warning)
            .add(label_role, &revert, palette.warning);
        let width = widgets::paint_text_right(ui, line.right(), line.center().y, words);
        copy_right = line.right() - width - 6.0;
        // `revert` is a link: it puts back what the cell loaded as,
        // whatever cell is selected.
        let word = Rect::from_min_max(pos2(line.right() - measure(&revert), line.top()), line.max);
        let link = ui.interact(word.expand(2.0), name_id.with("revert"), Sense::click());
        let name = format!("{} {column_name}", gettext(locale, "Revert"));
        link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &name));
        widgets::hline(ui, word.x_range(), word.bottom() - 0.5, palette.warning);
        if link.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if link.clicked() {
            actions.push(Action::RevertCell {
                tab,
                id: tab_id,
                cell: Some(CellPos { row, col }),
            });
        }
    }
    if let Some(reason) = &locked {
        // A small lock after the label, always there: under the pointer
        // it says why, and a screen reader hears it after the label.
        let place = Rect::from_center_size(
            pos2(
                line.left() + label_width + mark + 6.0 + LOCK / 2.0,
                line.center().y,
            ),
            vec2(LOCK, LOCK),
        );
        let lock = ui.interact(place.expand(3.0), name_id.with("lock"), Sense::hover());
        let said = format!("{}: {reason}", gettext(locale, "Locked"));
        lock.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
        Icon::Lock.image(palette.dim, LOCK).paint_at(ui, place);
        lock.on_hover_text(reason.as_str());
    }
    let copy_label = format!("{} {column_name}", gettext(locale, "Copy"));
    let edit_label = format!("{} {column_name}", gettext(locale, "Edit"));
    let hovered = ui.rect_contains_pointer(line.expand2(vec2(16.0, 30.0)));
    let stop = field_stop(tab, tab_id, col);
    // The pencil of a value that has clicks of its own, at `right`: beside
    // Copy, and shown as Copy is. It is the field's stop.
    let pencil_at = |ui: &mut egui::Ui, right: f32, size: f32, shown: bool| {
        let place = Rect::from_min_size(
            pos2(right - size, line.center().y - size / 2.0),
            vec2(size, size),
        );
        let response = ui.interact(place, stop, Sense::click());
        let face = (Icon::Pencil, edit_label.as_str(), shown);
        caption_face(ui, place, response, face, look, palette)
    };
    let mut pencil = None;
    if let Some(doc) = &doc {
        // A document's own controls: fold everything, and copy.
        let copy = Rect::from_min_size(
            pos2(line.right() - 26.0, line.center().y - 13.0),
            vec2(26.0, 26.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
        if look.terminal {
            // "za fold · y copy", keys in the text colour, at the right. A
            // SQL result has no `y`: the hint offers only the fold.
            let role = TextRole::OCaption;
            let hints = Text::new(look)
                .add(role, "za", palette.text)
                .space(role, " ");
            let hints = if copy_key {
                hints
                    .add(role, "fold ·", palette.dim)
                    .space(role, " ")
                    .add(role, "y", palette.text)
                    .space(role, " ")
                    .add(role, "copy", palette.dim)
            } else {
                hints.add(role, "fold", palette.dim)
            };
            let hints = widgets::paint_text_right(ui, line.right(), line.center().y, hints);
            if caption_button(&mut child, Icon::Copy, &copy_label, false, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
            // The pencil stands before the hints.
            if editable {
                let right = line.right() - hints - 6.0;
                pencil = Some(pencil_at(ui, right, PENCIL, hovered));
            }
        } else {
            if caption_button(&mut child, Icon::Copy, &copy_label, true, look, palette).clicked() {
                ui.ctx().copy_text(format::plain_text(value));
            }
            // The pencil beside Copy, bordered as it is: 4 between them.
            let mut left = copy.left();
            if editable {
                pencil = Some(pencil_at(ui, left - 4.0, 26.0, true));
                left -= 4.0 + 26.0;
            }
            let id = Id::new(("row-panel-json", tab, tab_id, request, row, col));
            // "Collapse all": a 24 pt button 6 at its sides, 4 before the
            // buttons.
            let link_right = left - 4.0 - 6.0;
            json_view::fold_all_link(
                ui,
                id,
                doc,
                link_right,
                line.center().y,
                locale,
                look,
                palette,
            );
        }
    } else {
        let copy = Rect::from_min_size(
            pos2(copy_right - 22.0, line.center().y - 11.0),
            vec2(22.0, 22.0),
        );
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(copy));
        if caption_button(&mut child, Icon::Copy, &copy_label, hovered, look, palette).clicked() {
            ui.ctx().copy_text(format::plain_text(value));
        }
    }
    ui.add_space(if doc.is_some() {
        if look.terminal { 8.0 } else { 6.0 }
    } else if look.terminal {
        2.0
    } else {
        3.0
    });
    // Values in the data face at 13; the terminal's timestamps at 12.
    let role = if matches!(column.kind, ValueKind::Json | ValueKind::Binary) {
        TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary)
    } else if look.terminal && column.kind == ValueKind::Temporal {
        TextRole::OSecondary
    } else {
        TextRole::pick(look, TextRole::InspectorValue, TextRole::OBody)
    };
    // The row's form makes one control of a value that can be edited, and
    // of one locked for a reason of its own: its text takes no caret.
    let whole = editable || locked.is_some();
    let pad = if whole { box_pad(ui, role, look) } else { 0.0 };
    // The control's states are painted behind the value.
    let behind = ui.painter().add(egui::Shape::Noop);
    let shown = match part {
        // The editor, in the value's place.
        Part::Editing if form.editing(ui, (tab, tab_id), role, (look, palette, locale)) => return,
        Part::InGrid => {
            row_form::in_grid(ui, look, palette, locale);
            Shown::default()
        }
        Part::Editing | Part::Read | Part::Editable | Part::Locked(_) => {
            let read = Reading {
                doc,
                formatted,
                name_id,
                role,
                whole,
                pad,
                // A locked value reads as one that is not the user's to
                // change. The terminal look writes a pending value in its
                // tone, as its grid does, in place of a tint behind it.
                color: match tone {
                    Some(tone) if look.terminal => Some(tone.color(palette)),
                    _ => locked.as_ref().map(|_| palette.secondary),
                },
            };
            value_of(
                ui, tab, tab_id, row, col, column, value, info, tag, skin, read, actions,
            )
        }
    };
    // An edit of a locked field was asked for here: it says why, under
    // its value, until the selection moves. Brought into view once.
    if let (Some(reason), true) = (&locked, form.refused(col)) {
        ui.add_space(if look.terminal { 2.0 } else { 3.0 });
        let width = ui.available_width();
        let note = Text::one(look, caption(look), reason, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        let asked = Id::new(("row-panel-refused", tab, tab_id));
        let at = (request, row, col);
        let seen = ui.data(|data| data.get_temp(asked)) == Some(at);
        if !seen {
            note.scroll_to_me(None);
            ui.data_mut(|data| data.insert_temp(asked, at));
        }
    }
    // What stands against the field's cell, under its value: the panel
    // has the room the grid gives a tooltip.
    if let Some(message) = &trouble {
        ui.add_space(if look.terminal { 2.0 } else { 3.0 });
        let width = ui.available_width();
        Text::one(look, caption(look), message, palette.danger)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
    }
    // A row the form has no part in: its values are read, as ever.
    if part == Part::Read {
        return;
    }
    // A list is edited by a pencil beside Copy, as a document is.
    if editable && shown.place.is_none() && pencil.is_none() {
        pencil = Some(pencil_at(ui, copy_right - 22.0 - 2.0, PENCIL, hovered));
    }
    // The value as one control: a click edits it, or says why it cannot
    // be edited.
    let control = shown.place.filter(|_| whole).map(|place| {
        // The box is the room the value keeps round its lines: it reaches
        // neither the label's line above nor what stands under the value.
        let frame = outline(line.left(), place, pad, look);
        let response = ui.interact(frame, stop, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
        let corner = CornerRadius::same(look.radius);
        let hovering = editable && response.hovered();
        if look.terminal {
            if hovering {
                let stroke = Stroke::new(1.0, palette.accent);
                ui.painter()
                    .rect_stroke(frame, corner, stroke, StrokeKind::Inside);
            }
        } else {
            // What the cell holds, as its cell of the grid says it: a
            // pending one on the warning's tint with a 2 pt bar at its
            // left, one in trouble on the failure's with a line round it.
            // Under the pointer a value that can be edited shows the
            // design's hover: the fill of a grid's row there and a border
            // inside it, a pencil at its right, and the text cursor.
            let hover = crate::ui::grid::row_fill(false, true, false, look, palette)
                .unwrap_or(palette.surface);
            let fill = match tone {
                Some(tone) => Some(tone.fill(look, palette)),
                None => hovering.then_some(hover),
            };
            let filled = |rect, corner, color| {
                egui::Shape::from(egui::epaint::RectShape::filled(rect, corner, color))
            };
            if let Some(fill) = fill {
                let shapes = if tone == Some(Tone::Warning) {
                    // The bar is the box in the warning's colour, and the
                    // tint over all of it but 2 at its left: a bar of its
                    // own would stand out of the box's round corners.
                    let rest = Rect::from_min_max(pos2(frame.left() + 2.0, frame.top()), frame.max);
                    let right = CornerRadius {
                        nw: 0,
                        sw: 0,
                        ..corner
                    };
                    vec![
                        filled(frame, corner, palette.warning),
                        filled(rest, right, fill),
                    ]
                } else {
                    vec![filled(frame, corner, fill)]
                };
                ui.painter().set(behind, egui::Shape::Vec(shapes));
            }
            let line = match tone {
                Some(Tone::Danger) => Some(palette.danger),
                _ => hovering.then_some(palette.border),
            };
            if let Some(color) = line {
                let stroke = Stroke::new(1.0, color);
                ui.painter()
                    .rect_stroke(frame, corner, stroke, StrokeKind::Inside);
            }
            if hovering {
                ui.ctx().set_cursor_icon(egui::CursorIcon::Text);
                let pencil = Rect::from_center_size(
                    pos2(frame.right() - 8.0 - 6.0, frame.center().y),
                    vec2(12.0, 12.0),
                );
                // A value that reaches the box's end runs under the
                // pencil, not through it.
                ui.painter().rect_filled(
                    pencil.expand2(vec2(4.0, 2.0)),
                    CornerRadius::ZERO,
                    fill.unwrap_or(hover),
                );
                Icon::Pencil
                    .image(palette.secondary, 12.0)
                    .paint_at(ui, pencil);
            }
        }
        let ring = if look.terminal {
            // The terminal look's cursor on a field: the selection's tone
            // behind the value, and a 2 pt bar in the accent at its left.
            // It is there whenever the field has the keyboard, as the
            // grid's is, and is all that marks it.
            if response.has_focus() {
                let bar = Rect::from_min_size(frame.min, vec2(2.0, frame.height()));
                let filled = |rect, color| {
                    egui::Shape::from(egui::epaint::RectShape::filled(
                        rect,
                        CornerRadius::ZERO,
                        color,
                    ))
                };
                let cursor = vec![
                    filled(frame, palette.selection),
                    filled(bar, palette.accent),
                ];
                ui.painter().set(behind, egui::Shape::Vec(cursor));
            }
            focus::Ring::Own
        } else {
            focus::Ring::Inset {
                radius: look.radius,
            }
        };
        focus::hint(ui, &response, frame, ring);
        response
    });
    // Every field of a row that can be edited has a stop, so the keyboard
    // can stand on each: one with neither a control nor a pencil (a
    // binary value, a locked document, a field the grid is editing) has
    // its label.
    let bare = (control.is_none() && pencil.is_none()).then(|| {
        let place = Rect::from_min_size(line.min, vec2(label_width.max(1.0), line.height()));
        let response = ui.interact(place, stop, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
        response
    });
    let Some(stop) = control.as_ref().or(pencil.as_ref()).or(bare.as_ref()) else {
        return;
    };
    // What starts an edit of the value here: a click on it. (Its keys are
    // read in `ui/keys.rs`.) Asked of a locked field, a click says why it
    // is locked.
    let edit = stop.clicked();
    if stop.has_focus() {
        // The field that takes the keyboard becomes the grid's selected
        // cell. Said once, as it takes it: a selection moved since (a
        // click in the grid, a key of the grid's) is not pulled back.
        if stop.gained_focus() && selected.col != col {
            actions.push(Action::FieldFocused {
                tab,
                id: tab_id,
                col,
            });
        }
        // Up and Down are the panel's, to step its fields with, and Esc
        // its way back to the grid: egui neither moves the keyboard off
        // the stop with the arrows nor drops it. The field's keys are
        // read with the grid's (`ui/keys.rs`), before the panel is drawn:
        // Enter and F2 never reach the stop, which Space still presses.
        let keys = egui::EventFilter {
            vertical_arrows: true,
            horizontal_arrows: true,
            escape: true,
            ..Default::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(stop.id, keys));
    }
    if edit && part != Part::InGrid {
        // A click gives a control no keyboard of itself. An editor that
        // opens takes it; a locked field, which opens none, is given it,
        // so the keys go on from the field that was asked.
        if locked.is_some() {
            stop.request_focus();
        }
        actions.push(Action::EditField {
            tab,
            id: tab_id,
            cell: CellPos { row, col },
            start: crate::model::EditStart::Value,
        });
    }
    // An edit ended and this field is owed the keyboard: on its stop.
    if form.focus == Some(col) {
        form.focus = None;
        stop.request_focus();
        stop.scroll_to_me(None);
        // What shows that it has the keyboard was drawn before it had
        // it: the next frame draws it.
        ui.ctx().request_repaint();
    }
}

/// The pencil of a list that can be edited: 22 pt, as Copy is.
const PENCIL: f32 = 22.0;

/// The lock after the label of a field that cannot be edited.
const LOCK: f32 = 11.0;

/// The box of a value that is one control: what the pointer shows round
/// it, and the field the editor opens in its place. How tall it is for a
/// value of one line, and how far it stands out at each side of the value,
/// which is flush with its label.
pub(super) fn field_box(look: &Look) -> (f32, f32) {
    if look.terminal {
        (26.0, 6.0)
    } else {
        (30.0, 8.0)
    }
}

/// The room a control's value keeps above and below its lines, in `role`:
/// what makes its box as tall as [`field_box`] says. The value keeps it at
/// rest too, so nothing moves when the pointer comes over it or its editor
/// opens.
fn box_pad(ui: &egui::Ui, role: TextRole, look: &Look) -> f32 {
    let (height, _) = field_box(look);
    ((height - line_of(ui, role, look)) / 2.0).max(2.0)
}

/// Where a field's box is, round the place of its value, in a field whose
/// label starts at `left`: `pad` above and below the value's lines, which
/// is the room they keep.
fn outline(left: f32, place: Rect, pad: f32, look: &Look) -> Rect {
    let (_, side) = field_box(look);
    Rect::from_min_max(
        pos2(left - side, place.top() - pad),
        pos2(place.right() + side, place.bottom() + pad),
    )
}

/// What a field's value is read from, besides the value itself.
struct Reading<'a> {
    /// The value as a tree, when it holds a document.
    doc: Option<Arc<json_view::Doc>>,
    /// The value's text, formatted by the app.
    formatted: Option<&'a format::FieldText>,
    /// The label the value is named by.
    name_id: Id,
    /// The role its text is drawn in.
    role: TextRole,
    /// Whether the row's form makes one control of the whole value: its
    /// text is drawn, and takes neither the pointer nor a caret.
    whole: bool,
    /// The room such a value keeps above and below its lines, for its box.
    pad: f32,
    /// The colour its text is written in, where it is not the text's own:
    /// a value that cannot be changed, the terminal look's pending one.
    color: Option<egui::Color32>,
}

/// What a field's value came to on screen.
#[derive(Default)]
struct Shown {
    /// Where a click edits it: its text, or the line of the NULL mark or
    /// of the stand-in of an empty or a blank text. None for what has its
    /// own clicks (a tree, a list).
    place: Option<Rect>,
}

/// A field's value, under its label: read, selected and copied from.
#[allow(clippy::too_many_arguments)] // the field's own, passed on
fn value_of(
    ui: &mut egui::Ui,
    tab: ConnTabId,
    tab_id: TabId,
    row: usize,
    col: usize,
    column: &tabletist_db::ColumnMeta,
    value: &Value,
    info: &FieldInfo,
    tag: Option<crate::ui::grid::Style>,
    skin: FieldSkin<'_>,
    read: Reading<'_>,
    actions: &mut Vec<Action>,
) -> Shown {
    let FieldSkin {
        look,
        palette,
        locale,
        fold,
        texts,
        ..
    } = skin;
    let Reading {
        doc,
        formatted,
        name_id,
        role,
        whole,
        pad,
        color,
    } = read;
    let request = texts.and_then(|texts| texts.request);
    let column_name = format::display_safe(&column.name);
    let mut shown = Shown::default();
    // A value with nothing to select is its whole line, to the pointer.
    let line_of_it = |ui: &egui::Ui, rect: Rect| {
        Rect::from_min_max(rect.min, pos2(ui.max_rect().right(), rect.bottom()))
    };
    if value.is_null() {
        ui.add_space(pad);
        let mark = crate::ui::grid::null_label(ui, look, palette).rect;
        ui.add_space(pad);
        shown.place = Some(line_of_it(ui, mark));
        return shown;
    }
    if let Some(doc) = doc {
        if fold {
            json_view::toggle_fold_all(
                ui,
                Id::new(("row-panel-json", tab, tab_id, request, row, col)),
            );
        }
        if let Some(file) = json_view::attachment(&doc) {
            attachment_card(ui, &file, look, palette);
            ui.add_space(if look.terminal { 10.0 } else { 6.0 });
        }
        let id = Id::new(("row-panel-json", tab, tab_id, request, row, col));
        if look.terminal {
            json_view::show(ui, id, &doc, &column_name, locale, palette, look);
        } else {
            Frame::new()
                .fill(palette.window)
                // The design's 1 pt border outside 12 and 10 of padding
                // (egui counts the stroke into the margin).
                .stroke(Stroke::new(1.0, palette.surface_hover))
                .corner_radius(CornerRadius::same(look.radius))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    json_view::show(ui, id, &doc, &column_name, locale, palette, look);
                });
        }
        return shown;
    }
    let Some(formatted) = formatted else {
        return shown;
    };
    let say = |text: &'static str| look.label(&gettext(locale, text));
    match value {
        // Sixteen bytes are the UUID they hold: text, laid out below.
        Value::Bytes(bytes) if format::uuid(bytes).is_some() => {}
        Value::Bytes(bytes) => {
            if binary(ui, bytes, &formatted.short, look, palette, locale) {
                actions.push(Action::SaveValue {
                    tab,
                    id: tab_id,
                    row,
                    col,
                });
            }
            return shown;
        }
        Value::Text(text) => {
            let marks = crate::ui::grid::marks(ui.ctx(), look);
            if let Some(blank) = format::blank_text(text, marks) {
                let note = if text.is_empty() {
                    say("empty string")
                } else {
                    say("whitespace only")
                };
                ui.add_space(pad);
                let stood = stand_in(ui, &blank, &note, look, palette);
                ui.add_space(pad);
                shown.place = Some(line_of_it(ui, stood));
                return shown;
            }
            if listed(text, column)
                && let Some(array) = format::array_items(text)
            {
                if array.items.is_empty() {
                    stand_in(ui, "{}", &say("empty array"), look, palette);
                } else {
                    elements(ui, &array, look, palette, locale);
                }
                return shown;
            }
        }
        Value::Null | Value::Bool(_) | Value::Int(_) | Value::Float(_) => {}
    }
    let expanded_id = Id::new(("row-panel-expanded", tab, tab_id, request, row, col));
    let expanded: bool = ui.data(|data| data.get_temp(expanded_id)).unwrap_or(false);
    // Whether the value takes more lines than the panel shows at first:
    // known once it is laid out at the width it gets.
    let mut tall = false;
    let long = formatted.full.is_some();
    let read = match &formatted.full {
        Some(full) if expanded => full,
        _ => &formatted.short,
    };
    // The follow link sits at the value's right.
    let follow = info.target.as_ref().map(|target| {
        if look.terminal {
            ("gd open".to_owned(), target.clone())
        } else {
            (
                format!(
                    "{} {} →",
                    gettext(locale, "Open"),
                    singular(&format::display_safe(&target.name))
                ),
                target.clone(),
            )
        }
    });
    // A tag's value in its text colour: the grid's chips stay in the grid.
    let color = color.unwrap_or_else(|| {
        tag.map_or(palette.text, |style| {
            crate::ui::value_tags::style_colors(style, look, palette).0
        })
    });
    let small = widgets::secondary(look);
    ui.add_space(pad);
    ui.horizontal_top(|ui| {
        // A colour (`#3a7bd5`) leads with a swatch of it, on the first line.
        if let Some(swatch) = format::color(value) {
            let (place, _) =
                ui.allocate_exact_size(vec2(SWATCH, line_of(ui, role, look)), Sense::hover());
            crate::ui::grid::paint_swatch(
                ui.painter(),
                ui,
                place.center(),
                SWATCH,
                swatch,
                look,
                palette,
            );
            ui.add_space(8.0);
        }
        let link = follow.as_ref().map_or(0.0, |(text, _)| {
            small.width(ui.ctx(), look.faces, text) + 12.0
        });
        let room = (ui.available_width() - link).max(24.0);
        ui.allocate_ui(vec2(room, 0.0), |ui| {
            let mut layouter = crate::typography::layouter(look, role, color);
            tall = layouter(ui, &read.as_str(), room).rows.len() > CLAMP_ROWS;
            let mut edit = |ui: &mut egui::Ui| {
                let value = ui
                    .add(
                        TextEdit::multiline(&mut read.as_str())
                            .font(role.font_id(look.faces))
                            // Flush with the field name above.
                            .frame(egui::Frame::NONE)
                            .margin(egui::Margin::ZERO)
                            .desired_width(room)
                            .desired_rows(1)
                            // The whole value of a row's form is one
                            // control, drawn over this: the text is not
                            // the pointer's, and takes no caret.
                            .interactive(!whole)
                            .layouter(&mut layouter),
                    )
                    .labelled_by(name_id);
                // A value to read and select, not a field: its caret says
                // where the keyboard is.
                if !whole {
                    focus::hint(ui, &value, value.rect, focus::Ring::Own);
                }
                shown.place = Some(value.rect);
            };
            if tall && !expanded {
                // The first lines, the last of them fading out (macOS):
                // "Show all" below gives the rest.
                let line = line_of(ui, role, look);
                let size = vec2(room, CLAMP_ROWS as f32 * line);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect));
                child.set_clip_rect(rect.intersect(ui.clip_rect()));
                edit(&mut child);
                // The lines that show are the value's place.
                shown.place = Some(rect);
                if !look.terminal {
                    let last =
                        Rect::from_min_max(pos2(rect.left(), rect.bottom() - line), rect.max);
                    fade(ui, last, palette.window);
                }
            } else {
                edit(ui);
            }
        });
        if let Some((text, target)) = follow {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                let link = Text::one(look, small, &text, palette.accent)
                    .layout(ui.ctx())
                    .label_sense(ui, Sense::click());
                let name = format!(
                    "{} {}",
                    gettext(locale, "Open"),
                    format::display_safe(&target.name)
                );
                link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &name));
                if link.clicked() {
                    actions.push(Action::FollowForeignKey {
                        tab,
                        object: target.clone(),
                        column: info.target_column.clone(),
                        value: format::plain_text(value),
                    });
                }
            });
        }
    });
    ui.add_space(pad);
    if long || tall {
        let label = if expanded {
            gettext(locale, "Show less").into_owned()
        } else {
            format!("{} ({})", gettext(locale, "Show all"), formatted.size)
        };
        let link = Text::one(look, small, &label, palette.accent)
            .layout(ui.ctx())
            .label_sense(ui, Sense::click());
        link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &label));
        if link.clicked() {
            ui.data_mut(|data| data.insert_temp(expanded_id, !expanded));
        }
    }
    shown
}

/// The lines of a long value the panel shows before "Show all".
const CLAMP_ROWS: usize = 3;

/// Fades `rect` from clear at its top to `color` at its bottom: the last
/// line of a value cut short runs out into the panel.
fn fade(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
    let clear = color.gamma_multiply(0.0);
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), clear);
    mesh.colored_vertex(rect.right_top(), clear);
    mesh.colored_vertex(rect.right_bottom(), color);
    mesh.colored_vertex(rect.left_bottom(), color);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    ui.painter().add(mesh);
}

/// A value with nothing to see: what the grid writes for it (`''`, a mark
/// for each space, `{}`), and in words what that is.
fn stand_in(ui: &mut egui::Ui, shown: &str, note: &str, look: &Look, palette: &Palette) -> Rect {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        let role = crate::ui::grid::data_role(look);
        Text::one(look, role, shown, palette.faint)
            .layout(ui.ctx())
            .label(ui);
        Text::one(look, widgets::secondary(look), note, palette.secondary)
            .layout(ui.ctx())
            .label(ui);
    })
    .response
    .rect
}

/// The most elements of an array the panel lists.
const ELEMENTS_SHOWN: usize = 100;

/// An array's elements, one to a line after their positions (which count
/// from 1, as PostgreSQL's do).
fn elements(
    ui: &mut egui::Ui,
    array: &format::ArrayItems<'_>,
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) {
    let role = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
    let shown = array.items.len().min(ELEMENTS_SHOWN);
    // The positions share a column as wide as the last one's.
    let column = role.width(ui.ctx(), look.faces, &format!("[{shown}]"));
    let line = line_of(ui, role, look);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = vec2(8.0, 2.0);
        for (index, item) in array.items.iter().take(ELEMENTS_SHOWN).enumerate() {
            ui.horizontal_top(|ui| {
                let (place, _) = ui.allocate_exact_size(vec2(column, line), Sense::hover());
                let position = format!("[{}]", index + 1);
                widgets::paint_text(
                    ui,
                    place.left(),
                    place.center().y,
                    Text::one(look, role, &position, palette.faint),
                );
                Text::one(look, role, &format::display_safe(item), palette.text)
                    .wrap(ui.available_width())
                    .layout(ui.ctx())
                    .label(ui);
            });
        }
        let more = array.items.len() - shown;
        if more > 0 {
            let rest = format!(
                "… {} {}",
                format::group_digits(more as u64),
                look.label(&gettext(locale, "more elements"))
            );
            Text::one(look, role, &rest, palette.faint)
                .layout(ui.ctx())
                .label(ui);
        }
    });
}

/// A binary value: its first bytes, how many more there are, and a link
/// that saves the whole of it to a file (Copy gives it as hex). Returns
/// whether the link was clicked.
fn binary(
    ui: &mut egui::Ui,
    bytes: &[u8],
    preview: &str,
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) -> bool {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    if bytes.is_empty() {
        Text::one(
            look,
            widgets::secondary(look),
            &say("no bytes"),
            palette.secondary,
        )
        .layout(ui.ctx())
        .label(ui);
        return false;
    }
    let role = TextRole::pick(look, TextRole::Json, TextRole::OSecondary);
    let more = bytes.len().saturating_sub(format::HEX_PREVIEW);
    let rest = (more > 0).then(|| {
        let count = format::group_digits(more as u64);
        if look.terminal {
            format!("… {count} {}", say("more"))
        } else {
            format!(
                "… {count} {} · {} {} {}",
                say("more bytes"),
                say("first"),
                format::HEX_PREVIEW,
                say("shown")
            )
        }
    });
    let lines = |ui: &mut egui::Ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        Text::one(look, role, preview, palette.secondary)
            .layout(ui.ctx())
            .label(ui);
        if let Some(rest) = &rest {
            Text::one(look, role, rest, palette.faint)
                .layout(ui.ctx())
                .label(ui);
        }
    };
    if look.terminal {
        ui.scope(lines);
    } else {
        Frame::new()
            .fill(palette.panel)
            // The design's 1 pt border outside 8 and 10 of padding.
            .stroke(Stroke::new(1.0, palette.surface_hover))
            .corner_radius(CornerRadius::same(look.radius))
            .inner_margin(egui::Margin::symmetric(10, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                lines(ui);
            });
    }
    let save = say("Save to file…");
    let link = Text::one(look, widgets::secondary(look), &save, palette.accent)
        .layout(ui.ctx())
        .label_sense(ui, Sense::click());
    link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &save));
    link.clicked()
}

/// `books` becomes `book`, for "Open book →".
fn singular(table: &str) -> &str {
    table
        .strip_suffix("ies")
        .map(|_| table)
        .or_else(|| table.strip_suffix('s'))
        .unwrap_or(table)
}

/// A button of a field's label line (Copy, the pencil): bordered next to a
/// document, otherwise a bare icon shown when the pointer is near. It is
/// always there for keyboards and screen readers.
fn caption_button(
    ui: &mut egui::Ui,
    icon: Icon,
    label: &str,
    shown: bool,
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    let size = ui.max_rect().size();
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    caption_face(ui, rect, response, (icon, label, shown), look, palette)
}

/// Draws a button of a field's label line at `rect`, for the `response`
/// its place gave: its icon, its name, and whether it shows with the
/// pointer away.
fn caption_face(
    ui: &egui::Ui,
    rect: Rect,
    response: egui::Response,
    (icon, label, shown): (Icon, &str, bool),
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    let size = rect.size();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label));
    if shown || response.hovered() || response.has_focus() {
        let corner = CornerRadius::same(look.radius.saturating_sub(2));
        if response.hovered() {
            ui.painter().rect_filled(rect, corner, palette.surface);
        }
        if size.x >= 24.0 && !look.terminal {
            ui.painter().rect_stroke(
                rect,
                corner,
                Stroke::new(widgets::hairline(ui), palette.border),
                StrokeKind::Inside,
            );
        }
        icon.image(palette.secondary, 13.0)
            .paint_at(ui, Rect::from_center_size(rect.center(), vec2(13.0, 13.0)));
    }
    response.on_hover_text(label)
}

/// A stored file's card: an image tile, its name, and what is known of it.
fn attachment_card(
    ui: &mut egui::Ui,
    file: &json_view::Attachment,
    look: &Look,
    palette: &Palette,
) {
    let width = ui.available_width();
    if look.terminal {
        // A bordered line: a 14 pt icon, the name, its size; 6 and 8 in.
        let name_role = TextRole::OBody;
        let height = line_of(ui, name_role, look) + 12.0 + 2.0;
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(3),
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
        let y = rect.center().y;
        Icon::Image.image(palette.dim, 14.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.left() + 9.0 + 7.0, y), vec2(14.0, 14.0)),
        );
        let x = rect.left() + 9.0 + 14.0 + 8.0;
        let x =
            x + widgets::paint_text(
                ui,
                x,
                y,
                Text::one(look, name_role, &file.filename, palette.text),
            ) + 8.0;
        let details: Vec<&str> = file
            .details
            .split(" · ")
            .filter(|part| part.contains('×') || part.ends_with('B'))
            .collect();
        widgets::paint_text(
            ui,
            x,
            y,
            Text::one(
                look,
                TextRole::OSecondary,
                &details.join(" · ").replace(" × ", "×"),
                palette.dim,
            ),
        );
        return;
    }
    // macOS: 8 in, a 44 pt tile, 10, the name over its details (2 apart).
    let (rect, _) = ui.allocate_exact_size(vec2(width, 62.0), Sense::hover());
    let corner = CornerRadius::same(look.radius);
    ui.painter().rect_filled(rect, corner, palette.window);
    ui.painter().rect_stroke(
        rect,
        corner,
        Stroke::new(widgets::hairline(ui), palette.surface_hover),
        StrokeKind::Inside,
    );
    let tile = Rect::from_min_size(pos2(rect.left() + 9.0, rect.top() + 9.0), vec2(44.0, 44.0));
    ui.painter().rect_filled(
        tile,
        CornerRadius::same(6),
        palette.surface.lerp_to_gamma(palette.surface_hover, 0.3),
    );
    let icon = if file.image { Icon::Image } else { Icon::Copy };
    icon.image(palette.faint, 18.0)
        .paint_at(ui, Rect::from_center_size(tile.center(), vec2(18.0, 18.0)));
    let x = tile.right() + 10.0;
    let (name_role, detail_role) = (TextRole::UiBodyStrong, TextRole::FieldLabel);
    let (name_line, detail_line) = (line_of(ui, name_role, look), line_of(ui, detail_role, look));
    let top = rect.center().y - (name_line + 2.0 + detail_line) / 2.0;
    widgets::paint_text(
        ui,
        x,
        top + name_line / 2.0,
        Text::one(look, name_role, &file.filename, palette.text),
    );
    widgets::paint_text(
        ui,
        x,
        top + name_line + 2.0 + detail_line / 2.0,
        Text::one(look, detail_role, &file.details, palette.dim),
    );
}

/// The footer of a table's row: Duplicate and Delete, which wait for a
/// later version, and under them how a value of the row is edited. `locked`
/// is why no field of the row can be edited, in the look's words: the
/// buttons say that under the pointer, and nothing is said of editing.
fn editing_footer(
    ui: &mut egui::Ui,
    rect: Rect,
    locked: Option<&str>,
    look: &Look,
    palette: &Palette,
    locale: crate::i18n::Locale,
) {
    let side = side(look);
    if !look.terminal {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
    }
    widgets::hline(
        ui,
        rect.x_range(),
        rect.top() + 0.5,
        if look.terminal {
            palette.outline
        } else {
            palette.surface_hover
        },
    );
    let later = gettext(
        locale,
        "Duplicating and deleting rows arrive in a later version",
    );
    // Why neither can be pressed: the row's reason, where it has one.
    let why = locked.unwrap_or(later.as_ref());
    let inner = rect.shrink2(vec2(side, 0.0));
    let gap = 6.0;
    let top = rect.top() + 1.0 + if look.terminal { 10.0 } else { 12.0 };
    let role = widgets::body(look);
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let height = if look.terminal {
        // Two equal cells, the keys in the text colour: what waits is
        // dashed, at 55%.
        let keys = [("yy p", "duplicate"), ("dd", "delete")];
        let width = (inner.width() - gap) / 2.0;
        let faded = |color: egui::Color32| palette.panel.lerp_to_gamma(color, 0.55);
        for (index, (key, label)) in keys.iter().enumerate() {
            let place = Rect::from_min_size(
                pos2(inner.left() + index as f32 * (width + gap), top),
                vec2(width, 28.0),
            );
            let response = ui.interact(place, ui.id().with(("edit", index)), Sense::hover());
            let name = format!("{key} {label}");
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, false, &name));
            let _ = response.on_hover_text(why);
            dashed(ui, place, faded(palette.outline));
            // A cell too narrow for both shows its key alone.
            let text = Text::one(look, role, key, faded(palette.text));
            let total = measure(&name);
            let (total, text) = if total > width {
                (measure(key), text)
            } else {
                let text = text.space(role, " ");
                (total, text.add(role, label, faded(palette.dim)))
            };
            let x = place.center().x - total / 2.0;
            widgets::paint_text(ui, x, place.center().y, text);
        }
        28.0
    } else {
        // Content-wide buttons sharing what is left equally: a 13 pt icon,
        // 6, the word.
        let labels = [
            (Icon::Copy, gettext(locale, "Duplicate")),
            (Icon::Trash2, gettext(locale, "Delete")),
        ];
        let widths: Vec<f32> = labels
            .iter()
            .map(|(_, text)| 13.0 + 6.0 + measure(text))
            .collect();
        let extra = (inner.width() - gap - widths.iter().sum::<f32>()) / 2.0;
        let mut x = inner.left();
        for ((icon, text), width) in labels.iter().zip(widths) {
            let place = Rect::from_min_size(pos2(x, top), vec2(width + extra, 32.0));
            x += width + extra + gap;
            widgets::ButtonSpec::new(text)
                .icon(*icon)
                .icon_size(13.0)
                .padding(0.0)
                .disabled(why)
                .show_at(ui, place, look, palette);
        }
        32.0
    };
    // How a value is edited, under the buttons of a row that can be: the
    // terminal look's head names its keys, and a row that cannot be edited
    // says why at the top of the panel.
    if look.terminal || locked.is_some() {
        return;
    }
    let note_role = caption(look);
    let y = top + height + 8.0 + line_of(ui, note_role, look) / 2.0;
    let measure = |text: &str| note_role.width(ui.ctx(), look.faces, text);
    let key = format!("{}I", look.command_key());
    let click = gettext(locale, "Click a value or press");
    let edit = gettext(locale, "to edit");
    let said = format!("{click} {key} {edit}");
    // Cut at the panel's side, as a field's label is.
    let hint = if measure(&said) <= inner.width() {
        Text::one(look, note_role, &click, palette.dim)
            .space(note_role, " ")
            .add(note_role, &key, palette.secondary)
            .space(note_role, " ")
            .add(note_role, &edit, palette.dim)
    } else {
        let shown = crate::ui::grid::ellipsize(&said, inner.width(), false, measure);
        Text::one(look, note_role, &shown, palette.dim)
    };
    widgets::paint_text(ui, inner.left(), y, hint);
    // The whole of it for a screen reader, cut or not.
    let place = Rect::from_min_size(
        pos2(inner.left(), y - 8.0),
        vec2(inner.width().max(1.0), 16.0),
    );
    widgets::announce(ui, place, &said);
}

/// A dashed 1 pt outline round `rect`.
pub(super) fn dashed(ui: &egui::Ui, rect: Rect, color: egui::Color32) {
    let stroke = Stroke::new(1.0, color);
    let rect = rect.shrink(0.5);
    for side in [
        [rect.left_top(), rect.right_top()],
        [rect.right_top(), rect.right_bottom()],
        [rect.right_bottom(), rect.left_bottom()],
        [rect.left_bottom(), rect.left_top()],
    ] {
        ui.painter()
            .extend(egui::Shape::dashed_line(&side, stroke, 3.0, 3.0));
    }
}

/// A value as a tree, when it holds a document that is small enough.
fn json_doc(ctx: &egui::Context, value: &Value, kind: ValueKind) -> Option<Arc<json_view::Doc>> {
    json_view::document(ctx, kind, value, json_view::TREE_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_takes_at_most_half_the_room() {
        let range = |room: f32| {
            let range = width_range(room);
            (range.min, range.max)
        };
        // Room to spare: the widths the edge can be dragged to.
        assert_eq!(range(1400.0), (NARROWEST, WIDEST));
        // Half the room is the most the panel takes.
        assert_eq!(range(700.0), (NARROWEST, 350.0));
        // Under its least width the panel gives way too.
        assert_eq!(range(400.0), (200.0, 200.0));
        assert_eq!(range(0.0), (0.0, 0.0));
    }

    /// A table open on one row of awkward values, that row in the panel.
    fn awkward(look: Look) -> crate::testing::Harness {
        awkward_with(look, "{en,fr}")
    }

    /// [`awkward`], its array column holding `languages`.
    fn awkward_with(look: Look, languages: &str) -> crate::testing::Harness {
        use tabletist_db::{ColumnMeta, ObjectKind, ObjectRef, RowPage};
        let mut harness = crate::testing::Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.app.apply(Action::OpenObject {
            tab,
            object: ObjectRef::new("main", "editions"),
            kind: ObjectKind::Table,
            pin: true,
        });
        let column = |name: &str, type_name: &str, kind| ColumnMeta {
            name: name.into(),
            type_name: type_name.into(),
            kind,
        };
        let text = |value: &str| Value::Text(value.into());
        let jpeg: Vec<u8> = [0xff, 0xd8, 0xff, 0xe0]
            .into_iter()
            .chain(std::iter::repeat_n(0, 96))
            .collect();
        harness.answer_rows(RowPage {
            columns: vec![
                column("title", "text", ValueKind::Text),
                column("subtitle", "text", ValueKind::Text),
                column("languages", "_text", ValueKind::Other),
                column("cover", "bytea", ValueKind::Binary),
                column("notes", "text", ValueKind::Text),
            ],
            rows: vec![vec![
                text(&"A long title. ".repeat(40)),
                text(""),
                text(languages),
                Value::Bytes(jpeg.into()),
                Value::Null,
            ]],
            has_more: false,
            ordered_by_key: true,
            elapsed: std::time::Duration::ZERO,
        });
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        harness.app.apply(Action::SelectCell {
            tab,
            id,
            cell: CellPos { row: 0, col: 0 },
        });
        harness.settle();
        harness
    }

    #[test]
    fn a_fields_label_says_what_its_value_does_not_show() {
        for look in Look::ALL {
            let mut harness = awkward(look);
            let labels = crate::testing::labels(&harness.settle());
            let expected = if look.terminal {
                [
                    "title · text · 560 chars",
                    "languages · text[] · 2 elements",
                    "cover · bytea · 100B · jpeg",
                ]
            } else {
                [
                    "title · text · 560 characters",
                    "languages · text[] · 2 elements",
                    "cover · bytea · 100 B · looks like JPEG",
                ]
            };
            for label in expected {
                assert!(
                    labels.iter().any(|found| found == label),
                    "{label} in {}: {labels:?}",
                    look.name
                );
            }
            // A value that shows whole says nothing more.
            assert!(labels.iter().any(|found| found == "subtitle · text"));
        }
    }

    #[test]
    fn values_with_nothing_to_see_say_what_they_are() {
        for look in Look::ALL {
            let mut harness = awkward(look);
            let labels = crate::testing::labels(&harness.settle());
            for text in ["''", "empty string", "NULL", "en", "fr"] {
                assert!(
                    labels.iter().any(|found| found == text),
                    "{text} in {}: {labels:?}",
                    look.name
                );
            }
            // A binary value is its first bytes and a count of the rest.
            let painted = |text: &str| {
                harness
                    .painted
                    .iter()
                    .any(|(piece, _)| piece.contains(text))
            };
            assert!(painted("ff d8 ff e0 00 00"), "{:?}", harness.painted);
            assert!(painted("76"), "the bytes past the first 24");
            // An array's elements stand after their positions.
            assert!(painted("[1]") && painted("[2]"), "{}", look.name);
        }
    }

    #[test]
    fn a_binary_value_is_saved_whole_under_a_name_of_its_own() {
        for look in Look::ALL {
            let mut harness = awkward(look);
            let link = look.label("Save to file…");
            harness.click(&link);
            // The dialog is the system's; the backend was asked with its
            // title, the whole value and a name that says what it is.
            assert_eq!(
                harness.app.backend.saves,
                [(
                    "Save value".to_owned(),
                    "editions-cover.jpg".to_owned(),
                    100
                )],
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_value_that_is_gone_is_not_saved() {
        let mut harness = awkward(Look::macos());
        let tab = harness.app.active_tab_id();
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        // A row the page no longer has, and a column that holds text.
        for (row, col) in [(7, 3), (0, 0)] {
            harness.app.apply(Action::SaveValue { tab, id, row, col });
        }
        assert!(harness.app.backend.saves.is_empty());
    }

    #[test]
    fn an_array_too_long_to_read_every_frame_is_text() {
        let long = format!("{{{}}}", vec!["element"; 20_000].join(","));
        let mut harness = awkward_with(Look::macos(), &long);
        let labels = crate::testing::labels(&harness.settle());
        // No list and no count of elements: its length, as any long text.
        assert!(
            labels.iter().any(|label| {
                label.starts_with("languages · text[] · ") && label.ends_with(" characters")
            }),
            "{labels:?}"
        );
        assert!(!labels.iter().any(|label| label.contains("elements")));
        assert!(labels.iter().any(|label| label.starts_with("Show all")));
    }

    #[test]
    fn a_long_value_shows_its_start_until_asked_for_all() {
        for look in Look::ALL {
            let mut harness = awkward(look);
            assert!(harness.has("Show all (560 B)"), "{}", look.name);
            assert!(!harness.has("Show less"));
            harness.click("Show all (560 B)");
            assert!(harness.has("Show less"), "{}", look.name);
            harness.click("Show less");
            assert!(harness.has("Show all (560 B)"), "{}", look.name);
        }
    }
}
