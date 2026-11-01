//! The Keyboard pane of the Settings sheet (macOS, Windows): every row of
//! the keymap the look has, by group, with its scope and its keys as key
//! caps. A click on the caps (or Return on them) records a new key; a
//! changed row has a dot before its caps and, under the pointer or the
//! keyboard, a button that puts its default back. The search field finds
//! a command by its name, or by its keys where a chord with ⌘, ⌃ or ⌥ is
//! pressed in it.

use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2};

use crate::keymap::{Group, Keymap, Layout, Row, Why, Written, fixed};
use crate::model::{Action, KeyboardPage, Recorded, Recording};
use crate::theme::{Icon, mix};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::widgets::{self, ButtonSpec, Segment};

use super::keyboard::{self, Listed};
use super::sheet::Skin;

/// A row of the list.
const ROW: f32 = 30.0;
/// A group's heading, with the room above it.
const HEADING: f32 = 30.0;
/// The Scope column and the Shortcut column.
const SCOPE: f32 = 120.0;
const SHORTCUT: f32 = 240.0;
/// Space inside a row at each side.
const PAD: f32 = 8.0;
/// A key cap: its height, the space at each side of its key, and between
/// two.
const CAP: f32 = 19.0;
const CAP_PAD: f32 = 5.0;
const CAP_GAP: f32 = 3.0;
/// The changed dot.
const DOT: f32 = 6.0;
/// The reset button of a changed row.
const RESET: f32 = 22.0;
/// The line a refused key gets under its row.
const REFUSAL: f32 = 32.0;
/// The All / Changed control's segments.
const SEGMENT: egui::Vec2 = vec2(104.0, 22.0);
/// The least of the list a short window leaves.
const ROWS_MIN: f32 = 56.0;

/// The search field's id: the chord pressed in it is read before it is.
pub(crate) fn search_id() -> egui::Id {
    egui::Id::new("keyboard-search")
}

/// The refusal line's buttons: the keys that press them are theirs, not
/// the recorder's.
fn swap_id() -> egui::Id {
    egui::Id::new("keyboard-swap")
}

fn cancel_id() -> egui::Id {
    egui::Id::new("keyboard-cancel")
}

/// Whether the keyboard is on a button of the refusal line.
pub(super) fn on_refusal(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.has_focus(swap_id()) || memory.has_focus(cancel_id()))
}

/// The pane: its title and what it is for, Reset all…, the search and the
/// switch, and the list under its column heads, which scrolls and ends at
/// `limit` at the latest.
pub(super) fn content(
    ui: &mut Ui,
    keymap: &Keymap,
    page: &KeyboardPage,
    limit: f32,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let corner = ui.cursor().min;
    let layout = Layout::of(look);
    let changed = keyboard::changed_count(keymap, layout);

    // Reset all…, at the right of the title.
    let reset = skin.say("Reset all…");
    let none = skin.say("No shortcuts changed");
    let mut button = ButtonSpec::new(&reset).salt("keyboard-reset-all");
    if changed == 0 {
        button = button.disabled(&none);
    }
    let button_width = button.width(ui, look);
    let place = Rect::from_min_size(
        pos2(corner.x + width - button_width, corner.y),
        vec2(button_width, look.control_height),
    );
    if button.show_at(ui, place, look, palette).clicked() {
        actions.push(Action::AskResetAllKeys(true));
    }
    let head = Rect::from_min_size(corner, vec2(width - button_width - 12.0, f32::INFINITY));
    let mut words = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(head)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    widgets::label(
        &mut words,
        TextRole::DialogTitle,
        &skin.say("Keyboard"),
        palette.text,
        look,
    );
    words.add_space(2.0);
    Text::one(
        look,
        TextRole::UiBody,
        &skin.say(
            "Click a shortcut to record a new one. The same keys can do different things in different scopes.",
        ),
        palette.dim,
    )
    .wrap(head.width())
    .layout(ui.ctx())
    .label(&mut words);
    let bottom = words.min_rect().bottom().max(place.bottom());
    ui.advance_cursor_after_rect(Rect::from_min_max(corner, pos2(corner.x + width, bottom)));
    ui.add_space(12.0);

    search_row(ui, page, changed, skin, actions);
    if let Some(found) = page.found {
        ui.add_space(6.0);
        found_line(ui, found, skin, actions);
    }
    ui.add_space(8.0);

    // The column heads.
    let top = ui.cursor().top();
    let heads = Rect::from_min_size(pos2(corner.x, top), vec2(width, 26.0));
    let role = TextRole::Secondary;
    let middle = heads.center().y;
    let (name_x, scope_x, keys_right) = columns(heads);
    widgets::paint_text(
        ui,
        name_x,
        middle,
        Text::one(look, role, &skin.say("Command"), palette.dim),
    );
    widgets::paint_text(
        ui,
        scope_x,
        middle,
        Text::one(look, role, &skin.say("Scope"), palette.dim),
    );
    widgets::paint_text_right(
        ui,
        keys_right,
        middle,
        Text::one(look, role, &skin.say("Shortcut"), palette.dim),
    );
    widgets::hline(ui, heads.x_range(), heads.bottom() - 0.5, palette.border);
    ui.advance_cursor_after_rect(heads);

    let listed = keyboard::listed(keymap, layout, skin.locale, page);
    let room = (limit - ui.cursor().top()).max(ROWS_MIN);
    egui::ScrollArea::vertical()
        .id_salt("keyboard-rows")
        .max_height(room)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            if listed.is_empty() {
                let said = if page.changed_only && page.search.is_empty() && page.found.is_none() {
                    skin.say("No shortcuts changed")
                } else {
                    skin.say("No commands match")
                };
                ui.add_space(24.0);
                widgets::label(ui, TextRole::UiBody, &said, palette.dim, look);
                return;
            }
            // The list is a table of its groups, each a group of rows.
            let builder = egui::UiBuilder::new().id_salt("keyboard-table");
            ui.scope_builder(builder, |ui| {
                let label = skin.say("Keyboard shortcuts");
                ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                    node.set_role(egui::accesskit::Role::Table);
                    node.set_label(label.as_str());
                });
                for Listed { group, rows } in &listed {
                    group_rows(ui, keymap, page, *group, rows, skin, actions);
                }
            });
        });
}

/// Where a row's three columns are, in `rect`: the name's left, the
/// scope's left, and the keys' right.
fn columns(rect: Rect) -> (f32, f32, f32) {
    let keys_right = rect.right() - PAD;
    let scope_x = keys_right - SHORTCUT - SCOPE;
    (rect.left() + PAD, scope_x, keys_right)
}

/// The search field and the All / Changed switch.
fn search_row(
    ui: &mut Ui,
    page: &KeyboardPage,
    changed: usize,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let width = ui.available_width();
    let top = ui.cursor().min;
    let switch = 2.0 * SEGMENT.x + 4.0;
    let field_rect = Rect::from_min_size(top, vec2(width - switch - 8.0, look.control_height));
    // A chord pressed in the field finds the rows it is a key of, in place
    // of being typed: read before the field reads the frame's keys.
    if ui.memory(|memory| memory.has_focus(search_id()))
        && let Some(keys) = keyboard::find_chord(ui.ctx(), Layout::of(look))
    {
        actions.push(Action::KeyboardFind(Some(keys)));
    }
    let mut text = page.search.clone();
    let hint = skin.say("Search commands or press a shortcut");
    let mut field_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(field_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let field = widgets::search_field(&field_ui, &mut text, &hint, look)
        .id(search_id())
        .desired_width(field_rect.width());
    let response = widgets::add_search(&mut field_ui, field, look);
    if response.changed() {
        actions.push(Action::KeyboardSearch(text));
    }
    let switch_rect = Rect::from_min_size(
        pos2(top.x + width - switch, top.y),
        vec2(switch, look.control_height),
    );
    let mut segment_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(switch_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let all = skin.say("All");
    let (changed_text, changed_name) = if changed == 0 {
        (skin.say("Changed"), skin.say("Changed, none"))
    } else {
        (
            format!("{} · {changed}", skin.say("Changed")),
            format!("{}, {changed}", skin.say("Changed")),
        )
    };
    let shown = skin.say("Shortcuts shown");
    let picked = widgets::segmented(
        &mut segment_ui,
        &[
            Segment::Text(&all),
            Segment::Named {
                text: &changed_text,
                name: &changed_name,
            },
        ],
        usize::from(page.changed_only),
        SEGMENT,
        Some(widgets::Group {
            name: &shown,
            description: None,
        }),
        look,
        palette,
    );
    if let Some(picked) = picked {
        let only = picked == 1;
        if only != page.changed_only {
            actions.push(Action::KeyboardChanged(only));
        }
    }
    ui.advance_cursor_after_rect(field_rect.union(switch_rect));
}

/// What the list is cut to while a key is looked for, and the link that
/// lists every row again.
fn found_line(ui: &mut Ui, found: &'static str, skin: &Skin, actions: &mut Vec<Action>) {
    let Skin { look, palette, .. } = *skin;
    let top = ui.cursor().min;
    let role = TextRole::Secondary;
    let height = 20.0;
    let middle = top.y + height / 2.0;
    let key = crate::keymap::key_written(Layout::of(look), found);
    let said = skin
        .say("Commands bound to {keys}")
        .replace("{keys}", key.as_str());
    let width = widgets::paint_label(
        ui,
        top.x,
        middle,
        Text::one(look, role, &said, palette.secondary),
    );
    let all = skin.say("Show all");
    let all_width = role.width(ui.ctx(), look.faces, &all).ceil();
    let rect = Rect::from_min_size(pos2(top.x + width + 12.0, top.y), vec2(all_width, height));
    let link = ui.interact(rect, ui.id().with("keyboard-show-all"), Sense::click());
    link.widget_info(|| WidgetInfo::labeled(WidgetType::Link, true, &all));
    let color = if link.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    widgets::paint_text(ui, rect.left(), middle, Text::one(look, role, &all, color));
    focus::hint(ui, &link, rect, Ring::Outer { radius: 3 });
    if link.clicked() {
        actions.push(Action::KeyboardFind(None));
    }
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(ui.available_width(), height)));
}

/// A group: its heading, then its rows, a group of rows to a screen
/// reader, named by the heading.
fn group_rows(
    ui: &mut Ui,
    keymap: &Keymap,
    page: &KeyboardPage,
    group: Group,
    rows: &[Row],
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let layout = Layout::of(look);
    let builder = egui::UiBuilder::new().id_salt(("keyboard-group", group.name(layout)));
    ui.scope_builder(builder, |ui| {
        let heading = skin.say(group.name(layout));
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::RowGroup);
            node.set_label(heading.as_str());
        });
        let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), HEADING), Sense::hover());
        let text = widgets::section_label(&heading, look, palette);
        widgets::paint_label(ui, rect.left() + PAD, rect.bottom() - 8.0, text);
        for row in rows {
            let recording = page
                .recording
                .as_ref()
                .filter(|recording| recording.of == Recorded::Row(*row));
            key_row(ui, keymap, *row, recording, skin, actions);
            if let Some((keys, why)) = recording.and_then(|recording| recording.refused.as_ref()) {
                let swaps = recording.is_some_and(|recording| recording.swaps);
                refusal_line(ui, (*row, keys, why, swaps), skin, actions);
            }
        }
    });
}

/// The keys of a row as caps, drawn right to left from `right` on
/// `middle`: the left edge they reach.
fn caps(ui: &Ui, right: f32, middle: f32, keys: &[Written], skin: &Skin) -> f32 {
    let Skin { look, palette, .. } = *skin;
    let role = TextRole::Shortcut;
    let mut x = right;
    for key in keys.iter().rev() {
        let width = role.width(ui.ctx(), look.faces, key.as_str()).ceil() + 2.0 * CAP_PAD;
        let rect = Rect::from_min_size(pos2(x - width, middle - CAP / 2.0), vec2(width, CAP));
        let painter = ui.painter();
        painter.rect_filled(rect, CornerRadius::same(4), palette.window);
        painter.rect_stroke(
            rect,
            CornerRadius::same(4),
            Stroke::new(1.0, palette.border),
            StrokeKind::Inside,
        );
        // The cap's lower edge is thicker: a key, not a tag.
        widgets::hline(
            ui,
            egui::Rangef::new(rect.left() + 2.0, rect.right() - 2.0),
            rect.bottom() - 1.0,
            palette.border,
        );
        widgets::paint_text(
            ui,
            rect.left() + CAP_PAD,
            middle,
            Text::one(look, role, key.as_str(), palette.secondary),
        );
        x = rect.left() - CAP_GAP;
    }
    x + CAP_GAP
}

/// A row: the command's name, its scope, and its keys as caps (a button
/// that records), with the changed dot and the reset button of a changed
/// row; "Press keys…" while it records.
fn key_row(
    ui: &mut Ui,
    keymap: &Keymap,
    row: Row,
    recording: Option<&Recording>,
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let layout = Layout::of(look);
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    let middle = rect.center().y;
    let (name_x, scope_x, keys_right) = columns(rect);
    let name = keyboard::name(look, skin.locale, row.command);
    let scope = skin.say(row.scope.name(layout));
    let fixed = fixed(layout, row);
    let changed = keymap.changed(layout, row);
    let built = row.command.info().built;
    let spoken = keyboard::spoken_keys(keymap, layout, skin.locale, row);
    let id = ui.id().with(("keyboard-row", row));
    // The row, for the pointer's hover and its context menu, and what a
    // screen reader says of it.
    let line = ui.interact(rect, id, Sense::click());
    let said = keyboard::said(keymap, look, skin.locale, row, [&name, &scope, &spoken]);
    line.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &said));
    ui.ctx().accesskit_node_builder(line.id, |node| {
        node.set_role(egui::accesskit::Role::Row);
    });
    // The caps' button ends where the reset button's slot begins: the
    // pointer reaches each.
    let caps_rect = Rect::from_min_max(
        pos2(keys_right - SHORTCUT, rect.top()),
        pos2(keys_right - RESET - 2.0, rect.bottom()),
    );
    let caps_id = ui.id().with(("keyboard-caps", row));
    let reset_id = ui.id().with(("keyboard-reset", row));
    let keyboard_here = ui.memory(|memory| memory.has_focus(caps_id) || memory.has_focus(reset_id));
    let hovered = line.hovered() || ui.rect_contains_pointer(rect);
    let painter = ui.painter();
    if recording.is_some() {
        let ground = mix(palette.window, palette.accent, 0.08);
        painter.rect_filled(rect, CornerRadius::same(4), ground);
        painter.rect_filled(
            Rect::from_min_size(rect.min, vec2(2.0, rect.height())),
            CornerRadius::ZERO,
            palette.accent,
        );
    } else if hovered {
        painter.rect_filled(rect, CornerRadius::same(4), palette.surface_hover);
    }
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.surface);
    let ink = if fixed.is_some() {
        palette.dim
    } else {
        palette.text
    };
    let width = widgets::paint_text(
        ui,
        name_x,
        middle,
        Text::one(look, TextRole::UiBody, &name, ink),
    );
    // A command nothing answers yet keeps its keys, and says so where the
    // Command column has room: a screen reader is told it always.
    if !built {
        let said = skin.say("Not built yet");
        let note = || Text::one(look, TextRole::Secondary, &said, palette.faint);
        let x = name_x + width + 8.0;
        if x + widgets::measure(ui, note()) <= scope_x - 8.0 {
            widgets::paint_text(ui, x, middle, note());
        }
    }
    widgets::paint_text(
        ui,
        scope_x,
        middle,
        Text::one(look, TextRole::Secondary, &scope, palette.dim),
    );

    // The reset button of a changed row, under the pointer or the
    // keyboard, after the caps.
    // The reset button's place, kept in every row so the caps line up:
    // a changed row's button is there for the keyboard and a screen reader
    // always, and drawn under the pointer or the keyboard.
    let right = keys_right - RESET - 6.0;
    if changed && fixed.is_none() {
        let defaults: Vec<String> = crate::keymap::defaults(layout, row)
            .into_iter()
            .map(|keys| crate::keymap::key_written(layout, keys).as_str().to_owned())
            .collect();
        let shown = if defaults.is_empty() {
            skin.say("Reset: not set")
        } else {
            skin.say("Reset to {keys}")
                .replace("{keys}", &defaults.join(", "))
        };
        let named = skin
            .say("Reset {name} to {keys}")
            .replace("{name}", &name)
            .replace(
                "{keys}",
                &keyboard::spoken_defaults(layout, skin.locale, row),
            );
        let place =
            Rect::from_center_size(pos2(keys_right - RESET / 2.0, middle), vec2(RESET, RESET));
        let reset = ui.interact(place, reset_id, Sense::click());
        reset.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &named));
        if hovered || keyboard_here {
            if reset.hovered() {
                let corner = CornerRadius::same(look.radius.saturating_sub(2));
                ui.painter()
                    .rect_filled(place, corner, palette.surface_active);
            }
            let tint = if reset.hovered() {
                palette.text
            } else {
                palette.secondary
            };
            Icon::RefreshCw
                .image(tint, 14.0)
                .paint_at(ui, Rect::from_center_size(place.center(), vec2(14.0, 14.0)));
        }
        focus::hint(ui, &reset, place, Ring::Outer { radius: 4 });
        let reset = reset.on_hover_text(&shown);
        if reset.clicked() {
            actions.push(Action::ResetRow(row));
        }
    }
    // The caps, or what stands for them: while a key pressed is refused,
    // that key.
    let refused = recording
        .and_then(|recording| recording.refused.as_ref())
        .map(|(keys, _)| *keys)
        .filter(|keys| !keys.is_empty());
    let keys = match refused {
        Some(keys) => vec![crate::keymap::key_written(layout, keys)],
        None => keymap.row_keys(layout, row),
    };
    let left = if keys.is_empty() {
        let role = TextRole::Secondary;
        let width = widgets::paint_text_right(
            ui,
            right,
            middle,
            Text::one(look, role, &skin.say("Not set"), palette.faint),
        );
        right - width
    } else {
        caps(ui, right, middle, &keys, skin)
    };
    let mut before = left;
    if changed {
        let dot = Rect::from_center_size(pos2(left - 6.0 - DOT / 2.0, middle), vec2(DOT, DOT));
        ui.painter()
            .circle_filled(dot.center(), DOT / 2.0, palette.accent);
        before = dot.left();
    }
    if let Some(recording) = recording {
        let role = TextRole::Secondary;
        let press = if recording.typed.is_empty() {
            skin.say("Press keys…")
        } else {
            recording.typed.clone()
        };
        widgets::paint_text_right(
            ui,
            before - 8.0,
            middle,
            Text::one(look, role, &press, palette.accent),
        );
        widgets::announce_polite(
            ui,
            caps_rect,
            &skin.say("Recording. Press the new shortcut, Escape to cancel."),
        );
    }
    // The caps are a button that records: a fixed row's keys can be read
    // and not changed.
    let sense = if fixed.is_some() {
        Sense::focusable_noninteractive()
    } else {
        Sense::click()
    };
    let button = ui.interact(caps_rect, caps_id, sense);
    let named = match fixed {
        Some(_) => skin
            .say("Shortcut for {name}: {keys}. It cannot be changed.")
            .replace("{name}", &name)
            .replace("{keys}", &spoken),
        None => skin
            .say("Shortcut for {name}: {keys}. Press Return to record.")
            .replace("{name}", &name)
            .replace("{keys}", &spoken),
    };
    button.widget_info(|| WidgetInfo::labeled(WidgetType::Button, fixed.is_none(), &named));
    focus::hint(
        ui,
        &button,
        caps_rect.shrink(2.0),
        Ring::Inset { radius: 4 },
    );
    if button.clicked() && fixed.is_none() && recording.is_none() {
        actions.push(Action::RecordKeys(Recorded::Row(row)));
    }
    // The row's menu: what its reset button does, and an unbind.
    if fixed.is_none() {
        crate::ui::menu::context(&line, look, palette, |menu| {
            let reset = skin.say("Reset to default");
            let none = skin.say("Not changed");
            let mut item = crate::ui::menu::Item::action(&reset);
            if !changed {
                item = item.disabled(&none);
            }
            if item.show(menu, look, palette).clicked() {
                actions.push(Action::ResetRow(row));
                menu.close();
            }
            let unbind = skin.say("Unbind");
            let unbound = skin.say("Not set");
            let mut item = crate::ui::menu::Item::action(&unbind);
            if keys.is_empty() {
                item = item.disabled(&unbound);
            }
            if item.show(menu, look, palette).clicked() {
                actions.push(Action::UnbindRow(row));
                menu.close();
            }
        });
    }
}

/// The line under a row that is recorded, while the last key pressed was
/// refused: why, then Swap where another row has the key and the exchange
/// would stand, and Cancel. A screen reader is told it as an alert, and the keyboard goes to its
/// first button once, as it appears.
fn refusal_line(
    ui: &mut Ui,
    (row, keys, why, swaps): (Row, &'static str, &Why, bool),
    skin: &Skin,
    actions: &mut Vec<Action>,
) {
    let Skin { look, palette, .. } = *skin;
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), REFUSAL), Sense::hover());
    let inner = rect.shrink2(vec2(0.0, 2.0));
    let ground = mix(palette.panel, palette.warning, 0.14);
    ui.painter()
        .rect_filled(inner, CornerRadius::same(6), ground);
    let middle = inner.center().y;
    let icon = Rect::from_center_size(pos2(inner.left() + 10.0 + 6.5, middle), vec2(13.0, 13.0));
    Icon::CircleAlert
        .image(palette.warning, 13.0)
        .paint_at(ui, icon);
    let said = keyboard::refusal(look, skin.locale, row, keys, why);
    let role = TextRole::Secondary;
    let swap = skin.say("Swap");
    // The key that gives up, as the keymap writes it: the screen's close.
    let close = crate::ui::keys::published(ui.ctx()).key_in(
        Layout::of(look),
        crate::keymap::Command::SettingsClose,
        crate::keymap::Scope::Prompt,
    );
    let cancel = format!("{} · {}", skin.say("Cancel"), close.as_str());
    let link_width = |text: &str| role.width(ui.ctx(), look.faces, text).ceil();
    let mut x = inner.right() - 10.0 - link_width(&cancel);
    let cancel_rect = Rect::from_min_size(pos2(x, middle - 9.0), vec2(link_width(&cancel), 18.0));
    let swap_rect = swaps.then(|| {
        x -= 12.0 + link_width(&swap);
        Rect::from_min_size(pos2(x, middle - 9.0), vec2(link_width(&swap), 18.0))
    });
    let room = x - 12.0 - (icon.right() + 8.0);
    let text = Text::one(look, role, &said, palette.secondary).wrap(room.max(40.0));
    let laid = text.layout(ui.ctx());
    laid.paint_left(ui.painter(), icon.right() + 8.0, middle);
    // An alert to a screen reader, whole.
    let alert = ui.interact(
        inner,
        ui.id().with(("keyboard-refusal", row)),
        Sense::hover(),
    );
    alert.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &said));
    ui.ctx().accesskit_node_builder(alert.id, |node| {
        node.set_role(egui::accesskit::Role::Alert);
    });
    let link = |rect: Rect, id: egui::Id, text: &str, color: egui::Color32| {
        let response = ui.interact(rect, id, Sense::click());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, text));
        widgets::paint_text(ui, rect.left(), middle, Text::one(look, role, text, color));
        focus::hint(ui, &response, rect, Ring::Outer { radius: 3 });
        response
    };
    let first = match swap_rect {
        Some(rect) => {
            let swap = link(rect, swap_id(), &swap, palette.accent);
            if swap.clicked() {
                actions.push(Action::SwapKeys);
            }
            swap_id()
        }
        None => cancel_id(),
    };
    if link(cancel_rect, cancel_id(), &cancel, palette.dim).clicked() {
        actions.push(Action::StopRecording);
    }
    // The keyboard to the first button, once for each refusal.
    let seen = egui::Id::new("keyboard-refusal-seen");
    let now = format!("{row:?}{keys}{why:?}");
    let before: Option<String> = ui.data(|data| data.get_temp(seen));
    if before.as_deref() != Some(now.as_str()) {
        ui.data_mut(|data| data.insert_temp(seen, now));
        ui.memory_mut(|memory| memory.request_focus(first));
    }
}
