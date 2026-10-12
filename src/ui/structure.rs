//! The Structure view: columns, indexes and foreign keys as plain tables.
//! Over them, for a table, what labels its rows in the pickers of the
//! foreign keys that point at it, and the ways to change that here: the
//! chooser, the column under the view's cursor, a column's menu.

use egui::accesskit::Role;
use egui::{CornerRadius, Frame, Rect, Sense, Stroke, WidgetInfo, WidgetType, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Scope};
use crate::labels::{self, Refused, Resolved, Source};
use crate::model::{Action, ConnTabId, Pane, TabId};
use crate::theme::{self, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::{display_safe, for_display};
use crate::ui::{focus, label_chooser, menu, states, widgets};

/// One cell of a structure table: body text, or code.
fn cell(
    ui: &mut egui::Ui,
    text: &str,
    code: bool,
    look: &Look,
    palette: &Palette,
) -> egui::Response {
    let role = if code {
        widgets::code(look)
    } else {
        widgets::body(look)
    };
    widgets::label(ui, role, text, palette.text, look)
}

/// What labels the table's rows, as the view's line words it, and where
/// the column's name stands in the words: "name · Automatic". Of a table
/// with nothing to choose it says so.
pub fn label_words(
    label: &Resolved,
    table: &str,
    look: &Look,
    locale: Locale,
) -> (String, Option<std::ops::Range<usize>>) {
    let words = match (&label.column, label.source) {
        (_, Source::KeysOnly) => {
            return (super::key_picker::nothing_text(table, look, locale), None);
        }
        (Some(_), Source::Automatic) => "{column} · Automatic",
        (Some(_), Source::User) => "{column} · Set by you",
        (None, Source::Automatic) => "Keys only · Automatic",
        (None, Source::User) => "Keys only · Set by you",
    };
    let words = look.label(&gettext(locale, words));
    let Some((column, at)) = label.column.as_deref().zip(words.find("{column}")) else {
        return (words, None);
    };
    // A placeholder: a translation puts the name where it reads.
    let column = display_safe(column);
    let said = words.replacen("{column}", &column, 1);
    (said, Some(at..at + column.len()))
}

/// Why a column that was named for the table's label was not taken: "id
/// cannot be the label (the key)".
pub fn refused_text(name: &str, why: Refused, table: &str, look: &Look, locale: Locale) -> String {
    let name = display_safe(name);
    match why {
        Refused::NoColumn => look
            .label(&gettext(locale, "No column {column} in {table}"))
            .replace("{column}", &name)
            .replace("{table}", &display_safe(table)),
        Refused::Excluded(excluded) => {
            let reason = label_chooser::reason(excluded, look, locale);
            look.label(&gettext(locale, "{column} cannot be the label ({reason})"))
                .replace("{column}", &name)
                .replace("{reason}", &reason)
        }
    }
}

/// How tall the buttons of the label's line are.
const BUTTON: f32 = 26.0;

/// The widest the sentence under the label's line runs before it wraps,
/// in the terminal look. In a card it has the card's room, and no less
/// than `NARROW` in a window too narrow for the buttons beside it.
const HELP: f32 = 640.0;
const NARROW: f32 = 240.0;

/// The value tags' slot the label's tag is drawn in: their blue.
const TAG: usize = 1;

/// What the label's part of the view was asked this frame.
#[derive(Default)]
struct Asked {
    /// Change… was clicked.
    change: bool,
    /// Use Automatic was clicked.
    automatic: bool,
    /// What the chooser hangs under.
    anchor: Option<Rect>,
    /// The button that opens it, where the look has one.
    button: Option<Rect>,
}

/// The label's part of the view, over the columns: what labels the
/// table's rows and who chose it, the ways to change it (buttons in the
/// looks with a pointer, the keys in the terminal look), and a sentence
/// that says what it is for. The looks with a pointer draw it as a card
/// across the view, with the buttons at its end. `refused` is why a column
/// that was named was not taken, while that is to be said here.
fn label_part(
    ui: &mut egui::Ui,
    label: &Resolved,
    (table, refused): (&str, Option<&str>),
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Asked {
    let mut asked = Asked::default();
    let (words, column) = label_words(label, table, look, locale);
    let at = column.unwrap_or(words.len()..words.len());
    let (body, small) = (widgets::body(look), widgets::secondary(look));
    let nothing = label.source == Source::KeysOnly;
    let key = |command| crate::ui::keys::written_in(ui.ctx(), look, command, Scope::Grid);
    let (choose_key, use_key) = (key(Command::OpenLabelChooser), key(Command::UseAsLabel));
    let automatic_key = key(Command::LabelAutomatic);
    let title = if look.terminal {
        gettext(locale, "fk label")
    } else {
        gettext(locale, "Label for foreign keys")
    };
    let automatic = gettext(locale, "Use Automatic");
    let (change, name) = (
        gettext(locale, "Change…"),
        gettext(locale, "Change label column"),
    );
    let why = super::key_picker::nothing_text(table, look, locale);
    // The way back, where the user chose, and what changes the label.
    let back = || widgets::ButtonSpec::new(&automatic).padding(10.0);
    let changes = || {
        let button = widgets::ButtonSpec::new(&change)
            .padding(10.0)
            .shortcut(&choose_key)
            .label(&name);
        if nothing {
            button.disabled(&why)
        } else {
            button
        }
    };
    let chosen = label.source == Source::User;
    let mut part = |ui: &mut egui::Ui| {
        ui.horizontal(|ui| {
            // The buttons end the card: the words have the room before
            // them.
            let buttons = if look.terminal {
                0.0
            } else if chosen {
                back().width(ui, look) + ui.spacing().item_spacing.x + changes().width(ui, look)
            } else {
                changes().width(ui, look)
            };
            let room = ui.available_width() - buttons - 16.0;
            ui.vertical(|ui| {
                let line = ui.horizontal(|ui| {
                    let role = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OBody);
                    let title = widgets::label(ui, role, &title, palette.text, look);
                    asked.anchor = Some(title.rect);
                    ui.add_space(8.0);
                    Text::new(look)
                        .add(body, &words[..at.start], palette.secondary)
                        .add(widgets::code(look), &words[at.clone()], palette.text)
                        .add(body, &words[at.end..], palette.secondary)
                        .layout(ui.ctx())
                        .label(ui);
                    if !look.terminal {
                        return;
                    }
                    ui.add_space(16.0);
                    // No buttons in this look: the keys. Where the user
                    // chose, the one that hands the choice back; else the
                    // two that choose, where there is anything to choose.
                    let keys = match label.source {
                        Source::KeysOnly => None,
                        Source::User => {
                            let back = gettext(locale, "back to automatic");
                            let keys = Text::new(look)
                                .add(small, &automatic_key, palette.text)
                                .space(small, " ")
                                .add(small, &back, palette.dim);
                            Some(keys)
                        }
                        Source::Automatic => {
                            let chooses = gettext(locale, "choose");
                            let uses = gettext(locale, "use the column under the cursor");
                            let keys = Text::new(look)
                                .add(small, &choose_key, palette.text)
                                .space(small, " ")
                                .add(small, &chooses, palette.dim)
                                .add(small, " · ", palette.dim)
                                .add(small, &use_key, palette.text)
                                .space(small, " ")
                                .add(small, &uses, palette.dim);
                            Some(keys)
                        }
                    };
                    if let Some(keys) = keys {
                        keys.layout(ui.ctx()).label(ui);
                    }
                });
                // The line as one thing, for a screen reader: "Label for
                // foreign keys: name, Automatic".
                let group = gettext(locale, "Label for foreign keys: {label}")
                    .replace("{label}", &words.replace(" · ", ", "));
                let whole = ui.interact(
                    line.response.rect,
                    ui.id().with("label-for-foreign-keys"),
                    Sense::hover(),
                );
                whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &group));
                ui.ctx().accesskit_node_builder(whole.id, |node| {
                    node.set_role(Role::Group);
                });
                let (help, wrap) = if look.terminal {
                    (
                        "Fields that point at {table} show and search the label column · on this computer only, nothing is written to the database",
                        HELP,
                    )
                } else {
                    (
                        "Fields that point at {table} show and search this column. On this computer only; nothing is written to the database.",
                        room.max(NARROW),
                    )
                };
                if look.terminal {
                    ui.add_space(4.0);
                } else {
                    // The card's lines stand close, as the design's do.
                    ui.spacing_mut().item_spacing.y = 2.0;
                }
                let help = look
                    .label(&gettext(locale, help))
                    .replace("{table}", &display_safe(table));
                Text::one(look, small, &help, palette.secondary)
                    .wrap(wrap)
                    .layout(ui.ctx())
                    .label(ui);
                let seen = ui.id().with("label-refused");
                if let Some(refused) = refused {
                    let danger = states::Tone::Danger.color(palette);
                    let said = widgets::label(ui, small, refused, danger, look);
                    // It is said at the view's top, and the cursor it was
                    // asked on may be far down: what was just said is
                    // brought into view.
                    let was: Option<String> = ui.data(|data| data.get_temp(seen));
                    if was.as_deref() != Some(refused) {
                        ui.data_mut(|data| data.insert_temp(seen, refused.to_owned()));
                        said.scroll_to_me_animation(None, egui::style::ScrollAnimation::none());
                    }
                } else {
                    ui.data_mut(|data| data.remove_temp::<String>(seen));
                }
            });
            if look.terminal {
                return;
            }
            ui.add_space((ui.available_width() - buttons).max(16.0));
            if chosen {
                asked.automatic = back().show(ui, BUTTON, look, palette).clicked();
            }
            let response = changes().show(ui, BUTTON, look, palette);
            asked.change = response.clicked();
            asked.anchor = Some(response.rect);
            asked.button = Some(response.rect);
        });
    };
    if look.terminal {
        part(ui);
    } else {
        Frame::new()
            .fill(palette.panel)
            // The design's 1 pt border outside 12 and 10 of padding.
            .stroke(Stroke::new(1.0, palette.surface_hover))
            .corner_radius(CornerRadius::same(look.radius))
            .inner_margin(egui::Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                part(ui);
            });
    }
    ui.add_space(16.0);
    asked
}

/// The mark on the name of the column that labels the table's rows: a
/// tag in the value tags' blue, and in the terminal look an arrow at the
/// name in the accent.
fn label_tag(ui: &mut egui::Ui, column: &str, (look, palette, locale): (&Look, &Palette, Locale)) {
    let word = gettext(locale, "label");
    let rect = if look.terminal {
        let mark = super::workspace::drawable(ui, TextRole::OBody, look, &["◂", "<"]);
        let text = format!("{} {word}", mark.unwrap_or("<"));
        widgets::label(ui, widgets::body(look), &text, palette.accent, look).rect
    } else {
        let (color, fill) = super::value_tags::slot_colors(TAG, look, palette);
        let laid = Text::one(look, TextRole::Shortcut, &word, color).layout(ui.ctx());
        let (rect, _) = ui.allocate_exact_size(vec2(laid.width() + 12.0, 18.0), Sense::hover());
        let fill = fill.unwrap_or(palette.surface_hover);
        ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
        laid.paint_left(ui.painter(), rect.left() + 6.0, rect.center().y);
        rect
    };
    let name = gettext(locale, "{column}, label column").replace("{column}", &display_safe(column));
    widgets::announce(ui, rect, &name);
}

/// Column names as a list, each as `display_safe` shows it.
fn names(columns: &[String]) -> String {
    columns
        .iter()
        .map(|column| display_safe(column))
        .collect::<Vec<_>>()
        .join(", ")
}

/// A table's column title, or a quiet note.
fn quiet(ui: &mut egui::Ui, text: &str, look: &Look, palette: &Palette) {
    widgets::label(ui, widgets::body(look), text, palette.secondary, look);
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let mut actions = Vec::new();
    let area = ui.max_rect();
    let id = object_tab;
    let skin = (&look, &palette, locale);
    // What labels the table's rows for the foreign keys that point at
    // it: none for a view, and none before the structure is known.
    let label = app.structure_label(tab, id);
    // Why a column named for it was not taken. The terminal look says it
    // in its status line.
    let refused = workspace
        .label_refused
        .as_ref()
        .filter(|(of, _, _)| *of == id && !look.terminal)
        .map(|(_, name, why)| refused_text(name, *why, &object.object.name, &look, locale));
    // The view's cursor shows while the view has the keys.
    let cursor = (workspace.pane == Pane::Grid).then_some(object.structure_cursor);
    let dialect = workspace.driver.dialect();
    let mut asked = Asked::default();
    if let Some(error) = super::data_view::shown_error(&object.structure) {
        egui::Frame::new()
            .inner_margin(egui::Margin::same(theme::PAD))
            .show(ui, |ui| {
                super::data_view::error_box(ui, error, &look, &palette, locale, || {
                    actions.push(Action::RetryStructure { tab, object_tab })
                });
            });
    } else if let Some(structure) = &object.structure.value {
        // Both ways: a long default (a sequence's nextval) must not push
        // the Key column out of reach.
        // A place of its own for each tab: the cursor's line, once brought
        // into view, is in view when the tab is come back to.
        egui::ScrollArea::both()
            .id_salt(("structure", object_tab.0))
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::new()
                    .inner_margin(egui::Margin::same(theme::PAD))
                    .show(ui, |ui| {
                        if let Some(label) = &label {
                            let table = (object.object.name.as_str(), refused.as_deref());
                            asked = label_part(ui, label, table, skin);
                        }
                        heading(ui, &gettext(locale, "Columns"), &look, &palette);
                        // Where each column's line stands, and the fill
                        // under the cursor's, painted once its place is
                        // known: under the line's words.
                        let mut lines = Vec::with_capacity(structure.columns.len());
                        let mut fill = None;
                        let labelled = label.as_ref().and_then(|label| label.column.as_deref());
                        let columns = egui::Grid::new(("structure-columns", object_tab.0))
                            .striped(true)
                            .spacing([16.0, 6.0])
                            .show(ui, |ui| {
                                for title in ["Name", "Type", "Nullable", "Default", "Key"] {
                                    quiet(ui, &gettext(locale, title), &look, &palette);
                                }
                                ui.end_row();
                                for (line, column) in structure.columns.iter().enumerate() {
                                    if cursor == Some(line) {
                                        fill = Some(ui.painter().add(egui::Shape::Noop));
                                    }
                                    let name = ui
                                        .horizontal(|ui| {
                                            let name = display_safe(&column.name);
                                            cell(ui, &name, false, &look, &palette);
                                            if labelled == Some(column.name.as_str()) {
                                                label_tag(ui, &column.name, skin);
                                            }
                                        })
                                        .response;
                                    lines.push(name.rect.y_range());
                                    let type_name = display_safe(&column.type_name);
                                    cell(ui, &type_name, true, &look, &palette);
                                    let nullable = if column.nullable {
                                        gettext(locale, "yes")
                                    } else {
                                        gettext(locale, "no")
                                    };
                                    cell(ui, &nullable, false, &look, &palette);
                                    let default =
                                        for_display(column.default.as_deref().unwrap_or(""));
                                    cell(ui, &default, true, &look, &palette);
                                    let key = if structure.primary_key.contains(&column.name) {
                                        "PK"
                                    } else {
                                        ""
                                    };
                                    cell(ui, key, false, &look, &palette);
                                    ui.end_row();
                                }
                            })
                            .response
                            .rect;
                        // The columns are one Tab stop, not one for each
                        // line: with the keyboard on it the view has the
                        // keys, and its cursor shows where they are. The
                        // pane keys and F6 come to it as they come to the
                        // rows, whose place this view stands in.
                        // Over what shows of the view, as the rows' grid
                        // is over its own: the columns may be scrolled
                        // out of it, and a stop must stand somewhere.
                        let seen = ui.clip_rect();
                        let stop = ui.interact(
                            seen,
                            ui.id().with(("structure-keys", object_tab.0)),
                            Sense::focusable_noninteractive(),
                        );
                        let named = gettext(locale, "Columns");
                        stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &named));
                        ui.ctx().accesskit_node_builder(stop.id, |node| {
                            node.set_role(Role::Group);
                        });
                        focus::pane(ui, &stop);
                        focus::hint(ui, &stop, seen, focus::Ring::Own);
                        focus::region(ui, focus::Region::Grid, seen);
                        focus::claim(ui, focus::Region::Grid, &stop);
                        if stop.gained_focus() {
                            actions.push(Action::GridKeys(tab));
                        }
                        // The lines themselves, for the pointer: a click
                        // puts the cursor on one and gives the view the
                        // keys, and a line has a menu. Clicked and never
                        // focused: Tab passes them by.
                        let keys: Vec<&str> =
                            structure.primary_key.iter().map(String::as_str).collect();
                        // The line the cursor was last brought into view
                        // on. Forgotten while the view has no keys: the
                        // view may be scrolled meanwhile, and the cursor
                        // is brought into view when the keys come back.
                        let seen = ui.id().with(("structure-cursor", object_tab.0));
                        if cursor.is_none() {
                            ui.data_mut(|data| data.remove_temp::<usize>(seen));
                        }
                        for (line, (column, rows)) in
                            structure.columns.iter().zip(&lines).enumerate()
                        {
                            let rect = Rect::from_x_y_ranges(columns.x_range(), rows.expand(3.0));
                            let row = ("structure-column", object_tab.0, line);
                            let response = ui.interact(rect, ui.id().with(row), Sense::CLICK);
                            if response.clicked() || response.secondary_clicked() {
                                actions.push(Action::SetStructureCursor { tab, id, line });
                                actions.push(Action::GridKeys(tab));
                            }
                            if cursor == Some(line) {
                                // A cursor that moved is brought into
                                // view, as the tree brings its own: the
                                // view scrolls, and its keys act on the
                                // line the cursor is on. Jump, not
                                // animate: the next key may come at once.
                                let was: Option<usize> = ui.data(|data| data.get_temp(seen));
                                if was != Some(line) {
                                    ui.data_mut(|data| data.insert_temp(seen, line));
                                    let jump = egui::style::ScrollAnimation::none();
                                    // Up and down only: the line as far
                                    // across as the window shows, so a
                                    // view scrolled sideways stays there.
                                    let across = ui.clip_rect().x_range();
                                    let shown = Rect::from_x_y_ranges(across, rect.y_range());
                                    ui.scroll_to_rect_animation(shown, None, jump);
                                }
                                if let Some(fill) = fill {
                                    let shape = egui::Shape::rect_filled(
                                        rect,
                                        CornerRadius::ZERO,
                                        palette.selection,
                                    );
                                    ui.painter().set(fill, shape);
                                }
                                if look.terminal {
                                    let bar =
                                        Text::one(&look, TextRole::OBody, "▌", palette.accent);
                                    let wide = TextRole::OBody.width(ui.ctx(), look.faces, "▌");
                                    bar.layout(ui.ctx()).paint_left(
                                        ui.painter(),
                                        rect.left() - wide - 2.0,
                                        rect.center().y,
                                    );
                                }
                            }
                            // Only a table's columns label anything.
                            if label.is_none() {
                                continue;
                            }
                            menu::context(&response, &look, &palette, |ui| {
                                let uses = gettext(locale, "Use as label");
                                let key = crate::ui::keys::written_in(
                                    ui.ctx(),
                                    &look,
                                    Command::UseAsLabel,
                                    Scope::Grid,
                                );
                                let why = labels::excluded(dialect, column, &keys)
                                    .map(|why| label_chooser::reason(why, &look, locale));
                                let mut item = menu::Item::action(&uses).detail(&key);
                                if let Some(why) = &why {
                                    item = item.disabled(why);
                                }
                                if item.show(ui, &look, &palette).clicked() {
                                    let line = Some(line);
                                    actions.push(Action::UseAsLabel { tab, id, line });
                                    ui.close();
                                }
                                let copy = gettext(locale, "Copy name");
                                if menu::Item::action(&copy)
                                    .show(ui, &look, &palette)
                                    .clicked()
                                {
                                    ui.ctx().copy_text(column.name.clone());
                                    ui.close();
                                }
                            });
                        }
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Indexes"), &look, &palette);
                        if structure.indexes.is_empty() {
                            let text = gettext(locale, "No indexes");
                            quiet(ui, &text, &look, &palette);
                        } else {
                            egui::Grid::new(("structure-indexes", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in ["Name", "Columns", "Unique", "Method"] {
                                        quiet(ui, &gettext(locale, title), &look, &palette);
                                    }
                                    ui.end_row();
                                    for index in &structure.indexes {
                                        cell(
                                            ui,
                                            &display_safe(&index.name),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        let columns = names(&index.columns);
                                        cell(ui, &columns, false, &look, &palette);
                                        let unique = if index.primary {
                                            "PK"
                                        } else if index.unique {
                                            "yes"
                                        } else {
                                            ""
                                        };
                                        cell(ui, unique, false, &look, &palette);
                                        let method =
                                            display_safe(index.method.as_deref().unwrap_or(""));
                                        cell(ui, &method, false, &look, &palette);
                                        ui.end_row();
                                    }
                                });
                        }
                        ui.add_space(16.0);
                        heading(ui, &gettext(locale, "Foreign keys"), &look, &palette);
                        if structure.foreign_keys.is_empty() {
                            let text = gettext(locale, "No foreign keys");
                            quiet(ui, &text, &look, &palette);
                        } else {
                            egui::Grid::new(("structure-fks", object_tab.0))
                                .striped(true)
                                .spacing([16.0, 6.0])
                                .show(ui, |ui| {
                                    for title in
                                        ["Name", "Columns", "References", "On update", "On delete"]
                                    {
                                        quiet(ui, &gettext(locale, title), &look, &palette);
                                    }
                                    ui.end_row();
                                    for key in &structure.foreign_keys {
                                        let name = display_safe(key.name.as_deref().unwrap_or("·"));
                                        cell(ui, &name, false, &look, &palette);
                                        let columns = names(&key.columns);
                                        cell(ui, &columns, false, &look, &palette);
                                        let table = format!(
                                            "{}.{}",
                                            display_safe(&key.ref_schema),
                                            display_safe(&key.ref_table)
                                        );
                                        let target = if key.ref_columns.is_empty() {
                                            table
                                        } else {
                                            format!("{table} ({})", names(&key.ref_columns))
                                        };
                                        cell(ui, &target, false, &look, &palette);
                                        cell(
                                            ui,
                                            &display_safe(&key.on_update),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        cell(
                                            ui,
                                            &display_safe(&key.on_delete),
                                            false,
                                            &look,
                                            &palette,
                                        );
                                        ui.end_row();
                                    }
                                });
                        }
                    });
            });
    }
    if asked.change {
        actions.push(Action::ToggleLabelChooser { tab, id });
    }
    if asked.automatic {
        let choice = labels::LabelChoice::Automatic;
        actions.push(Action::SetLabel { tab, id, choice });
    }
    // The chooser, under what opened it. It never takes the keyboard:
    // `ui::keys` routes its keys, as under a picker.
    if let (Some(chooser), Some(label), Some(anchor), Some(structure)) = (
        &object.label_chooser,
        &label,
        asked.anchor,
        &object.structure.value,
    ) {
        let keys: Vec<&str> = structure.primary_key.iter().map(String::as_str).collect();
        let open = label_chooser::Open {
            at: chooser.at,
            lines: labels::lines(dialect, Some(&structure.columns), &keys),
            choice: chooser.choice.clone(),
            automatic: label.automatic.clone(),
            loading: false,
            arrived: false,
            table: object.object.name.clone(),
            connection: (workspace.name.clone(), workspace.environment),
        };
        let place = label_chooser::Place::Under(anchor);
        let shown = label_chooser::show(ui, &open, place, None, skin);
        match shown.asked {
            Some(label_chooser::Asked::Choose(line)) => {
                let line = Some(line);
                actions.push(Action::ChooseLabel { tab, id, line });
            }
            Some(label_chooser::Asked::Point(line)) => {
                actions.push(Action::PointLabelChooser { tab, id, line });
            }
            None => {
                // A press beside it closes it, as it closes a menu. Not
                // one on the button that opened it, whose click closes it.
                let pressed = ui.input(|input| {
                    let at = input.pointer.interact_pos();
                    at.filter(|_| input.pointer.any_pressed())
                });
                let beside = pressed.is_some_and(|at| {
                    !shown.rect.contains(at)
                        && !asked.button.is_some_and(|button| button.contains(at))
                });
                if beside {
                    actions.push(Action::CloseLabelChooser { tab, id });
                }
            }
        }
    }
    // A fetch that has lasted, over whatever is up: a refresh keeps the
    // structure it replaces on screen.
    if let Some(waited) = object.structure.running_for() {
        if super::states::lasted(waited) {
            let (text, name, keys) = (
                look.label(&gettext(locale, "Loading structure…")),
                look.label(&gettext(locale, "Cancel")),
                super::data_view::cancel_keys(&app.keymap, &look),
            );
            let cancel = super::states::key_button(&name, &keys, &look).label("Cancel query");
            if super::states::running(ui, area, &text, waited, Some(cancel), &look, &palette) {
                actions.push(Action::CancelQuery(tab));
            }
        } else {
            // Come back when the wait is long enough to show.
            ui.ctx()
                .request_repaint_after(super::states::DELAY - waited);
        }
    }
    app.actions.extend(actions);
}

fn heading(ui: &mut egui::Ui, text: &str, look: &Look, palette: &Palette) {
    let role = TextRole::pick(look, TextRole::TableTitle, TextRole::OTableTitle);
    widgets::label(ui, role, text, palette.text, look);
    ui.add_space(4.0);
}
