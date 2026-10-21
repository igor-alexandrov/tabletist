//! The Keyboard pane of the terminal look's Settings screen: the `/` line
//! that filters the commands by name (and `?` then keys, which finds them
//! by a key), the rows by group with the scope, the keys and the action,
//! a cursor the screen's keys move, and the line under a row that is
//! recorded. The file pane beside it shows the `[keys.*]` part of
//! `settings.toml` (`file_pane::show_keys`).

use egui::{CornerRadius, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::keymap::{Command, Group, Keymap, Layout, Row, Scope, Spelled, Stroke, Why, fixed};
use crate::model::{Action, KeyboardPage, Recorded, Recording, SettingsDialog, SettingsPane};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets;

use super::keyboard::{self, Listed};
use super::terminal::Skin;

/// A row of the list, and a group's heading.
const ROW: f32 = 26.0;
const HEADING: f32 = 28.0;
/// Space at the list's sides.
const SIDE: f32 = 16.0;
/// The scope column and the keys column.
const SCOPE: f32 = 120.0;
const KEYS: f32 = 260.0;
/// The filter line.
const FILTER: f32 = 28.0;

/// The rows the pane lists now, in order, whatever their group.
fn flat(listed: &[Listed]) -> Vec<Row> {
    listed
        .iter()
        .flat_map(|listed| listed.rows.iter().copied())
        .collect()
}

/// The row the cursor is on: the one the page says, where it is listed,
/// else the first listed.
pub(super) fn cursor(app: &App, page: &KeyboardPage) -> Option<Row> {
    let rows = flat(&keyboard::listed(app, page));
    page.cursor
        .filter(|cursor| rows.contains(cursor))
        .or_else(|| rows.first().copied())
}

/// What a key of the screen asks of the pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Asked {
    Move(isize),
    Group { forward: bool },
    Record,
    Unbind,
    Reset,
    Filter,
    Find,
    Edit,
    Pane { forward: bool },
    Close,
}

/// The keys `command` has on the screen.
fn chords(keymap: &Keymap, command: Command) -> impl Iterator<Item = &'static str> + '_ {
    keymap.chords_now(Layout::Omarchy, command, Scope::Prompt, true)
}

/// The waiting first key of the screen's keys of several (`d` of `dd`).
fn waiting_id() -> egui::Id {
    egui::Id::new("keyboard-pane-typed")
}

/// The pane's keys while nothing is recorded and the `/` line does not
/// have the keyboard: they move the cursor among the rows listed, record,
/// unbind and reset its row, filter and find, open the file, show
/// another pane and close the screen.
pub(super) fn keys(
    app: &App,
    ctx: &egui::Context,
    dialog: &SettingsDialog,
    actions: &mut Vec<Action>,
) {
    let keymap = app.keymap.clone();
    let page = &dialog.keyboard;
    let listed = keyboard::listed(app, page);
    let rows = flat(&listed);
    let mut at = cursor(app, page)
        .and_then(|cursor| rows.iter().position(|row| *row == cursor))
        .unwrap_or(0);
    let mut waiting: String = ctx
        .data(|data| data.get_temp(waiting_id()))
        .unwrap_or_default();
    let mut asked: Vec<Asked> = Vec::new();
    ctx.input_mut(|input| {
        let held = input.modifiers;
        // What is typed is the frame's text where it brings some (a real
        // keyboard sends both), else what its bare keys type.
        let texts = input
            .events
            .iter()
            .any(|event| matches!(event, egui::Event::Text(text) if !text.trim().is_empty()));
        input.events.retain(|event| {
            let typed = match event {
                egui::Event::Text(text) => {
                    let mut chars = text.chars();
                    match (chars.next(), chars.next()) {
                        (Some(char), None) if !char.is_whitespace() => Some(char),
                        _ => None,
                    }
                }
                egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } if !texts
                    && !(modifiers.command
                        || modifiers.ctrl
                        || modifiers.mac_cmd
                        || modifiers.alt) =>
                {
                    crate::keymap::char_of(*key, modifiers.shift)
                }
                _ => None,
            };
            if let Some(char) = typed {
                waiting.push(char);
                let unbind = chords(&keymap, Command::SettingsUnbind).any(|keys| {
                    keys.chars().filter(|char| *char != ' ').collect::<String>() == waiting
                });
                if unbind {
                    waiting.clear();
                    asked.push(Asked::Unbind);
                    return false;
                }
                let begins =
                    chords(&keymap, Command::SettingsUnbind).any(|keys| keys.starts_with(&waiting));
                if begins {
                    return false;
                }
                waiting.clear();
            } else if let egui::Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } = event
            {
                // A key that types nothing ends a wait; the key of a letter
                // whose text comes beside it does not.
                let plain =
                    !(modifiers.command || modifiers.ctrl || modifiers.mac_cmd || modifiers.alt);
                let letter = crate::keymap::char_of(*key, false).is_some();
                if !(plain && letter) {
                    waiting.clear();
                }
            }
            for (command, way) in [
                (Command::SettingsMove, None),
                (Command::SettingsChange, None),
                (Command::SettingsOpenRow, Some(Asked::Record)),
                (Command::SettingsReset, Some(Asked::Reset)),
                (Command::SettingsFilter, Some(Asked::Filter)),
                (Command::SettingsFindKey, Some(Asked::Find)),
                (Command::SettingsOpenFile, Some(Asked::Edit)),
                (Command::SettingsPane, None),
                (Command::SettingsClose, Some(Asked::Close)),
            ] {
                for (nth, keys) in chords(&keymap, command).enumerate() {
                    let pressed = match crate::keymap::spell(keys) {
                        Ok(Spelled::Strokes(strokes)) => match strokes.as_slice() {
                            [Stroke::Text(letter)] => typed == Some(*letter),
                            [Stroke::Key { .. }] => {
                                crate::ui::keys::press_of(Layout::Omarchy, held, event, keys)
                                    .is_some()
                            }
                            _ => false,
                        },
                        _ => false,
                    };
                    if !pressed {
                        continue;
                    }
                    asked.push(match (command, way) {
                        (_, Some(way)) => way,
                        (Command::SettingsMove, _) => {
                            Asked::Move(if matches!(keys, "k" | "up") { -1 } else { 1 })
                        }
                        (Command::SettingsChange, _) => Asked::Group {
                            forward: !matches!(keys, "h" | "left"),
                        },
                        _ => Asked::Pane { forward: nth == 1 },
                    });
                    return false;
                }
            }
            true
        });
    });
    ctx.data_mut(|data| data.insert_temp(waiting_id(), waiting));
    for asked in asked {
        let row = rows.get(at).copied();
        match asked {
            Asked::Move(by) => {
                if !rows.is_empty() {
                    at = at.saturating_add_signed(by).min(rows.len() - 1);
                    actions.push(Action::KeyboardCursor(rows[at]));
                }
            }
            Asked::Group { forward } => {
                let Some(row) = row else { continue };
                let group = row.command.info().group;
                let index = listed
                    .iter()
                    .position(|listed| listed.group == group)
                    .unwrap_or(0);
                let next = if forward {
                    (index + 1).min(listed.len() - 1)
                } else if listed[index].rows.first() == Some(&row) {
                    index.saturating_sub(1)
                } else {
                    index
                };
                if let Some(first) = listed[next].rows.first() {
                    at = rows.iter().position(|row| row == first).unwrap_or(at);
                    actions.push(Action::KeyboardCursor(*first));
                }
            }
            Asked::Record => {
                if let Some(row) = row
                    && fixed(Layout::Omarchy, row).is_none()
                {
                    actions.push(Action::RecordKeys(Recorded::Row(row)));
                }
            }
            Asked::Unbind => {
                if let Some(row) = row {
                    actions.push(Action::UnbindRow(row));
                }
            }
            Asked::Reset => {
                if let Some(row) = row {
                    actions.push(Action::ResetRow(row));
                }
            }
            Asked::Filter => actions.push(Action::KeyboardFiltering(true)),
            Asked::Find => actions.push(Action::RecordKeys(Recorded::Find)),
            Asked::Edit => actions.push(Action::EditSettingsFile),
            Asked::Pane { forward } => {
                let listed = SettingsPane::listed();
                let at = listed
                    .iter()
                    .position(|shown| *shown == SettingsPane::Keyboard)
                    .unwrap_or(0);
                let next = match forward {
                    true => (at + 1).min(listed.len() - 1),
                    false => at.saturating_sub(1),
                };
                if listed[next] != SettingsPane::Keyboard {
                    actions.push(Action::ShowSettingsPane(listed[next]));
                }
            }
            // A key looked for is let go first: the list is whole again.
            Asked::Close if page.found.is_some() => actions.push(Action::KeyboardFind(None)),
            Asked::Close => actions.push(Action::CloseDialog),
        }
    }
}

/// The pane in `ui`: the filter line, the rows by group, the line of a
/// refused key, and the note of what is never bound.
pub(super) fn rows(
    ui: &mut Ui,
    app: &App,
    page: &KeyboardPage,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    ui.add_space(8.0);
    filter_line(ui, page, skin, actions);
    // The column heads.
    let (heads, _) = ui.allocate_exact_size(vec2(ui.available_width(), 24.0), Sense::hover());
    let role = TextRole::OSecondary;
    let middle = heads.center().y;
    let (scope_x, keys_x, action_x) = columns(heads);
    for (x, word) in [(scope_x, "scope"), (keys_x, "keys"), (action_x, "action")] {
        widgets::paint_text(
            ui,
            x,
            middle,
            Text::one(look, role, &skin.say(word), palette.dim),
        );
    }
    let listed = keyboard::listed(app, page);
    let on = cursor(app, page);
    if listed.is_empty() {
        let said = if page.changed_only {
            skin.say("no shortcuts changed")
        } else {
            skin.say("no commands match")
        };
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
        widgets::paint_label(
            ui,
            rect.left() + SIDE,
            rect.center().y,
            Text::one(look, role, &said, palette.dim),
        );
    } else {
        let builder = egui::UiBuilder::new().id_salt("keyboard-table");
        ui.scope_builder(builder, |ui| {
            let label = skin.say("keyboard shortcuts");
            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                node.set_role(egui::accesskit::Role::Table);
                node.set_label(label.as_str());
            });
            for Listed { group, rows } in &listed {
                group_rows(ui, app, page, (*group, rows), on, skin, actions);
            }
        });
    }
    ui.add_space(12.0);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let note = skin.say("super is never bound: hyprland owns it · insert-mode keys follow vim");
    widgets::paint_label(
        ui,
        rect.left() + SIDE,
        rect.center().y,
        Text::one(look, role, &note, palette.dim),
    );
}

/// Where a row's three columns start in `rect`: the scope, the keys, the
/// action.
fn columns(rect: Rect) -> (f32, f32, f32) {
    let scope = rect.left() + SIDE;
    (scope, scope + SCOPE, scope + SCOPE + KEYS)
}

/// The `/` line: the filter's field while it has the keyboard, else what
/// it filters by, or a key looked for, or what the two keys do.
fn filter_line(ui: &mut Ui, page: &KeyboardPage, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width() - 2.0 * SIDE;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), FILTER + 8.0), Sense::hover());
    let place = Rect::from_min_size(pos2(rect.left() + SIDE, rect.top()), vec2(width, FILTER));
    let finding = page
        .recording
        .as_ref()
        .filter(|recording| recording.of == Recorded::Find);
    if let Some(found) = page.found.filter(|_| finding.is_none()) {
        let key = crate::keymap::key_written(Layout::Omarchy, found);
        let close = crate::ui::keys::published(ui.ctx()).key_in(
            Layout::Omarchy,
            Command::SettingsClose,
            Scope::Prompt,
        );
        let said = skin
            .say("keys {keys} · {close} shows every command")
            .replace("{keys}", key.as_str())
            .replace("{close}", close.as_str());
        ui.painter().rect_stroke(
            place,
            CornerRadius::same(3),
            egui::Stroke::new(1.0, palette.outline),
            egui::StrokeKind::Inside,
        );
        widgets::paint_label(
            ui,
            place.left() + 8.0,
            place.center().y,
            Text::one(look, TextRole::OBody, &said, palette.text),
        );
        return;
    }
    if let Some(finding) = finding {
        let said = if finding.typed.is_empty() {
            skin.say("? press the keys to find")
        } else {
            format!("? {}", finding.typed)
        };
        ui.painter().rect_stroke(
            place,
            CornerRadius::same(3),
            egui::Stroke::new(1.0, palette.accent),
            egui::StrokeKind::Inside,
        );
        widgets::paint_label(
            ui,
            place.left() + 8.0,
            place.center().y,
            Text::one(look, TextRole::OBody, &said, palette.accent),
        );
        return;
    }
    let mut text = page.search.clone();
    let hint = skin.say("filter commands · ? find by key");
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
    let style = widgets::FieldStyle {
        fill: None,
        boxed: true,
        role: TextRole::OBody,
    };
    let response = widgets::filter_field(
        &mut child,
        &mut text,
        &hint,
        place.size(),
        style,
        look,
        palette,
    );
    if page.filtering && !response.has_focus() && !response.lost_focus() {
        response.request_focus();
    }
    if response.changed() {
        actions.push(Action::KeyboardSearch(text));
    }
    // Enter and Esc give the keys back to the rows, the filter kept.
    if response.lost_focus() && page.filtering {
        actions.push(Action::KeyboardFiltering(false));
    }
    if response.gained_focus() && !page.filtering {
        actions.push(Action::KeyboardFiltering(true));
    }
}

/// A group's heading, then its rows.
fn group_rows(
    ui: &mut Ui,
    app: &App,
    page: &KeyboardPage,
    (group, rows): (Group, &[Row]),
    on: Option<Row>,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let builder = egui::UiBuilder::new().id_salt(("keyboard-group", group.name(Layout::Omarchy)));
    ui.scope_builder(builder, |ui| {
        let heading = skin.say(group.name(Layout::Omarchy));
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::RowGroup);
            node.set_label(heading.as_str());
        });
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADING), Sense::hover());
        widgets::paint_label(
            ui,
            rect.left() + SIDE,
            rect.bottom() - 8.0,
            Text::one(look, TextRole::OSecondary, &heading, palette.dim),
        );
        for row in rows {
            let recording = page
                .recording
                .as_ref()
                .filter(|recording| recording.of == Recorded::Row(*row));
            key_row(ui, app, *row, (on == Some(*row), recording), skin, actions);
            if let Some((keys, why)) = recording.and_then(|recording| recording.refused.as_ref()) {
                let swaps = recording.is_some_and(|recording| recording.swaps);
                refusal_line(ui, app, (*row, keys, why, swaps), skin);
            }
        }
    });
}

/// A row: its scope, its keys (or "press keys…" and what is typed so far
/// while it records), and its action.
fn key_row(
    ui: &mut Ui,
    app: &App,
    row: Row,
    (on_cursor, recording): (bool, Option<&Recording>),
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let layout = Layout::Omarchy;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let middle = rect.center().y;
    let (scope_x, keys_x, action_x) = columns(rect);
    let name = keyboard::name(app, row.command);
    let scope = skin.say(row.scope.name(layout));
    let fixed = fixed(layout, row);
    let changed = app.keymap.changed(layout, row);
    let id = ui.id().with(("keyboard-row", row));
    let built = row.command.info().built;
    let line = ui.interact(rect, id, Sense::click());
    let spoken = keyboard::spoken_keys(app, row);
    let said = keyboard::said(app, row, [&name, &scope, &spoken]);
    line.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &said));
    ui.ctx().accesskit_node_builder(line.id, |node| {
        node.set_role(egui::accesskit::Role::Row);
    });
    if line.clicked() {
        actions.push(Action::KeyboardCursor(row));
    }
    // The cursor's row stays in view as the keys move it, and a row that
    // records keeps its line under it in view too. Jump, not animate: the
    // next key may come at once.
    if on_cursor || recording.is_some() {
        let shown = rect.with_max_y(rect.bottom() + ROW);
        if !ui.clip_rect().contains_rect(shown) {
            ui.scroll_to_rect_animation(shown, None, egui::style::ScrollAnimation::none());
        }
    }
    if on_cursor || recording.is_some() {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.selection);
        ui.painter().rect_filled(
            Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
            CornerRadius::ZERO,
            palette.accent,
        );
    }
    let role = TextRole::OBody;
    let ink = if fixed.is_some() {
        palette.dim
    } else {
        palette.text
    };
    widgets::paint_text(
        ui,
        scope_x,
        middle,
        Text::one(look, role, &scope, palette.dim),
    );
    let keys_place = Rect::from_min_max(
        pos2(keys_x, rect.top()),
        pos2(action_x - 8.0, rect.bottom()),
    );
    match recording {
        Some(recording) => {
            let press = skin.say("press keys…");
            let width = widgets::measure(ui, Text::one(look, role, &press, palette.window)) + 12.0;
            let chip = Rect::from_min_size(pos2(keys_x, middle - 10.0), vec2(width, 20.0));
            ui.painter()
                .rect_filled(chip, CornerRadius::ZERO, palette.accent);
            widgets::paint_text(
                ui,
                chip.left() + 6.0,
                middle,
                Text::one(look, role, &press, palette.window),
            );
            // What is typed so far, or the keys last refused.
            let shown = match &recording.refused {
                Some((keys, _)) if recording.typed.is_empty() && !keys.is_empty() => {
                    crate::keymap::key_written(layout, keys).as_str().to_owned()
                }
                _ => recording.typed.clone(),
            };
            widgets::paint_text(
                ui,
                chip.right() + 8.0,
                middle,
                Text::one(look, role, &shown, palette.text),
            );
            widgets::announce_polite(
                ui,
                keys_place,
                &skin.say("Recording. Press the new shortcut, Escape to cancel."),
            );
        }
        None => {
            let keys = app.keymap.row_keys(layout, row);
            let written: Vec<&str> = keys.iter().map(|key| key.as_str()).collect();
            let (text, color) = if written.is_empty() {
                (skin.say("not set"), palette.faint)
            } else {
                (
                    written.join("  "),
                    if fixed.is_some() {
                        palette.dim
                    } else {
                        palette.accent
                    },
                )
            };
            let width =
                widgets::paint_text(ui, keys_x, middle, Text::one(look, role, &text, color));
            if changed {
                let dot = pos2(keys_x + width + 8.0, middle);
                ui.painter().circle_filled(dot, 3.0, palette.accent);
            }
        }
    }
    let width = widgets::paint_text(ui, action_x, middle, Text::one(look, role, &name, ink));
    // A command nothing answers yet keeps its keys, and says so.
    if !built {
        let note = skin.say("not built yet");
        let x = action_x + width + 8.0;
        widgets::paint_text(ui, x, middle, Text::one(look, role, &note, palette.faint));
    }
    // The keys are a button that records, as on the other looks: Enter on
    // the row does it from the keys.
    let sense = if fixed.is_some() {
        Sense::focusable_noninteractive()
    } else {
        Sense::click()
    };
    let caps = ui.interact(keys_place, ui.id().with(("keyboard-caps", row)), sense);
    let named = match fixed {
        Some(_) => skin
            .say("shortcut for {name}: {keys}. it cannot be changed.")
            .replace("{name}", &name)
            .replace("{keys}", &spoken),
        None => {
            // The screen's key that records, as the keymap writes it.
            let record = crate::ui::keys::published(ui.ctx()).key_in(
                layout,
                Command::SettingsOpenRow,
                Scope::Prompt,
            );
            skin.say("shortcut for {name}: {keys}. press {record} to record.")
                .replace("{name}", &name)
                .replace("{keys}", &spoken)
                .replace("{record}", record.as_str())
        }
    };
    caps.widget_info(|| WidgetInfo::labeled(WidgetType::Button, fixed.is_none(), &named));
    focus::hint(ui, &caps, keys_place, Ring::Inset { radius: 0 });
    if caps.clicked() && fixed.is_none() && recording.is_none() {
        actions.push(Action::KeyboardCursor(row));
        actions.push(Action::RecordKeys(Recorded::Row(row)));
    }
}

/// The line under a row whose last key was refused: why, then `[s] swap`
/// where another row has the key, and `[esc] cancel`. A screen reader is
/// told why as an alert, and the keys as hints of their own.
fn refusal_line(
    ui: &mut Ui,
    app: &App,
    (row, keys, why, swaps): (Row, &'static str, &Why, bool),
    skin: &Skin,
) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let middle = rect.center().y;
    let left = rect.left() + SIDE;
    let bar = Rect::from_min_size(pos2(left, rect.top() + 2.0), vec2(2.0, rect.height() - 4.0));
    ui.painter()
        .rect_filled(bar, CornerRadius::ZERO, palette.warning);
    let said = keyboard::refusal(app, row, keys, why);
    let role = TextRole::OBody;
    let width = widgets::paint_text(
        ui,
        left + 10.0,
        middle,
        Text::one(look, role, &said, palette.warning),
    );
    let place = Rect::from_min_max(
        pos2(left + 10.0, rect.top()),
        pos2(left + 10.0 + width.max(1.0), rect.bottom()),
    );
    let alert = ui.interact(
        place,
        ui.id().with(("keyboard-refusal", row)),
        Sense::hover(),
    );
    alert.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
    ui.ctx().accesskit_node_builder(alert.id, |node| {
        node.set_role(egui::accesskit::Role::Alert);
    });
    let keymap = crate::ui::keys::published(ui.ctx());
    let key = |command| {
        keymap
            .key_in(Layout::Omarchy, command, Scope::Prompt)
            .bracketed()
    };
    let (swap, cancel) = (key(Command::SettingsSwap), key(Command::SettingsClose));
    let (swap_word, cancel_word) = (skin.say("swap"), skin.say("cancel"));
    let mut hints: Vec<widgets::Hint<'_>> = Vec::new();
    if swaps {
        hints.push((&swap, &swap_word, true));
    }
    hints.push((&cancel, &cancel_word, true));
    let x = place.right() + 12.0;
    let width = widgets::key_hints(ui, (x, middle), &hints, 12.0, look, palette);
    let hinted = Rect::from_min_max(pos2(x, rect.top()), pos2(x + width.max(1.0), rect.bottom()));
    let named: Vec<String> = hints
        .iter()
        .map(|(key, word, _)| format!("{} {word}", key.as_str()))
        .collect();
    widgets::announce(ui, hinted, &named.join("  "));
}
