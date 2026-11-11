//! The keyboard shortcuts dialog (`?`): the keymap, as the look in use
//! has it, and a search over it. The search is Settings › Keyboard's
//! (`key_search`): words name a command, its group or its keys, and a
//! chord pressed in the field finds the commands it is a key of.

use egui::{Key, Modifiers, Rect, Sense, pos2, vec2};

use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Group, Keymap, Layout, Written, key_written};
use crate::model::{Action, HelpSearch};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::key_search::{self, Search};
use crate::ui::{Paint, widgets};

/// The groups the list is in, in its order, as the Keyboard page of the
/// design names them.
const GROUPS: [(Group, &str); 6] = [
    (Group::Window, "Window and connections"),
    (Group::Navigation, "Navigation"),
    (Group::Editing, "Grid and editing"),
    (Group::Sql, "SQL editor"),
    (Group::Ai, "AI assistant"),
    (Group::Prompts, "Prompts and dialogs"),
];

/// What the list holds for `layout`: every command that is built and has
/// a key there, with its keys as the layout writes them, in the keymap's
/// order. None that is only claimed, and none the layout has no key for.
pub(crate) fn rows(keymap: &Keymap, layout: Layout) -> Vec<(Group, Command, Written)> {
    let mut rows = Vec::new();
    for (group, _) in GROUPS {
        for command in Command::ALL {
            let info = command.info();
            let keys = keymap.label(layout, *command);
            if info.group == group && info.built && !keys.is_empty() {
                rows.push((group, *command, keys));
            }
        }
    }
    rows
}

/// A group's heading, in English: the caller translates.
fn heading(group: Group) -> &'static str {
    GROUPS
        .iter()
        .find(|(of, _)| *of == group)
        .map_or("", |(_, name)| name)
}

/// A command the dialog lists while it is searched: its group, its keys
/// as the look writes them, and what of them and of its name the search
/// matched, to mark.
pub(crate) struct Listed {
    pub group: Group,
    pub command: Command,
    pub keys: Written,
    /// Where in `keys` the cap the typed keys named is.
    pub keys_hit: Option<std::ops::Range<usize>>,
    /// Where the words are in the command's name, as translated.
    pub spans: Vec<std::ops::Range<usize>>,
}

/// The rows the search leaves, in [`rows`]' order: all of them while
/// nothing is typed or pressed.
pub(crate) fn listed(
    keymap: &Keymap,
    layout: Layout,
    locale: Locale,
    search: &HelpSearch,
) -> Vec<Listed> {
    let finder = Search::new(&search.search, search.found);
    rows(keymap, layout)
        .into_iter()
        .filter_map(|(group, command, keys)| {
            let name = gettext(locale, command.info().name);
            let group_name = gettext(locale, heading(group));
            let caps = keymap.caps(layout, command);
            let caps: Vec<&str> = caps.iter().map(|cap| cap.as_str()).collect();
            let spelled = keymap.command_keys(layout, command);
            if !finder.keeps(layout, (&name, &group_name), &spelled, &caps) {
                return None;
            }
            // The caps are written one after another, ", " between them.
            let keys_hit = finder.cap_hit(layout, &spelled, &caps).and_then(|at| {
                let start: usize = caps[..at].iter().map(|cap| cap.len() + 2).sum();
                let end = start + caps[at].len();
                (end <= keys.len()).then_some(start..end)
            });
            Some(Listed {
                group,
                command,
                spans: finder.name_spans(&name),
                keys_hit,
                keys,
            })
        })
        .collect()
}

/// Space between the keys and what they do.
const GAP: f32 = 16.0;

/// Space between the rows of the list.
const ROW_GAP: f32 = 6.0;

/// The height of the line a chord looked for shows.
const FOUND_LINE: f32 = 20.0;

/// The search field of the macOS and Windows looks.
fn field_id() -> egui::Id {
    egui::Id::new("help-search")
}

/// Whether the field was given the keyboard since the dialog opened.
fn opened_id() -> egui::Id {
    field_id().with("opened")
}

/// The field the keyboard was on last frame: Omarchy's `/` line names
/// its own.
fn last_field_id() -> egui::Id {
    field_id().with("last")
}

/// Whether the dialog's field had the keyboard last frame: a chord pressed
/// there is one to find, which no shortcut takes first.
pub(crate) fn field_has_keyboard(ctx: &egui::Context) -> bool {
    let last: Option<egui::Id> = ctx.data(|data| data.get_temp(last_field_id()));
    last.is_some_and(|id| ctx.memory(|memory| memory.has_focus(id)))
}

/// Forgets the dialog's frame state once it is closed: the next one gives
/// its field the keyboard again.
pub fn fold(ctx: &egui::Context) {
    ctx.data_mut(|data| {
        data.remove::<bool>(opened_id());
        data.remove::<egui::Id>(last_field_id());
    });
}

/// Draws the dialog over the window, the keys `keymap` has, cut to what
/// `search` leaves, and returns what it asks for.
pub fn show(
    ctx: &egui::Context,
    paint: Paint,
    keymap: &Keymap,
    search: &HelpSearch,
) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let layout = paint.layout();
    let all = rows(keymap, layout);
    let listed = listed(keymap, layout, locale, search);
    let heading_role = TextRole::pick(&look, TextRole::UiBodySemibold, TextRole::OGroup);
    let say = |words: &'static str| look.label(&gettext(locale, words));
    let searching = !search.search.is_empty() || search.found.is_some();
    let mut actions = Vec::new();
    crate::ui::widgets::modal(egui::Id::new("help"), &look, &palette).show(ctx, |ui| {
        // Esc takes back the search first, then closes; read before the
        // field, which would take it.
        let top = ui
            .ctx()
            .memory(|memory| memory.top_modal_layer())
            .is_none_or(|layer| layer == ui.layer_id());
        if top && ui.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            actions.push(if searching {
                Action::HelpSearch(String::new())
            } else {
                Action::CloseDialog
            });
        }
        // A chord pressed in the field finds what it does, in place of
        // being typed: read before the field reads the frame's keys.
        if field_has_keyboard(ui.ctx())
            && let Some(keys) = key_search::find_chord(ui.ctx(), layout)
        {
            actions.push(Action::HelpFind(Some(keys)));
        }
        // As wide as its widest keys and description side by side, and no
        // wider or taller than the window leaves room for: the list scrolls
        // when the window is small, so the buttons stay on screen. Measured
        // on the whole list, so that nothing moves while it is searched.
        let (mut keys_width, mut what_width) = (0.0_f32, 0.0_f32);
        // The look's own keys: none that only another look has.
        for (_, command, keys) in &all {
            let keys = Text::one(&look, widgets::code(&look), keys, palette.text);
            let what = Text::one(
                &look,
                widgets::body(&look),
                &gettext(locale, command.info().name),
                palette.text,
            );
            keys_width = keys_width.max(widgets::measure(ui, keys));
            what_width = what_width.max(widgets::measure(ui, what));
        }
        let screen = ui.ctx().content_rect();
        let width = (keys_width + GAP + what_width).ceil();
        let width = width.min(screen.width() - 80.0).max(120.0);
        ui.set_width(width);
        widgets::label(
            ui,
            widgets::dialog_title(&look),
            &gettext(locale, "Keyboard shortcuts"),
            palette.text,
            &look,
        );
        ui.add_space(8.0);
        field(ui, paint, search, listed.len(), &mut actions);
        if let Some(keys) = search.found {
            ui.add_space(6.0);
            found_line(ui, paint, keys, &mut actions);
        }
        ui.add_space(8.0);
        // The whole list's height, so that the dialog keeps its size.
        let groups = {
            let mut groups: Vec<Group> = all.iter().map(|(group, ..)| *group).collect();
            groups.dedup();
            groups.len()
        };
        let line = widgets::body(&look).row_height(ui.ctx(), look.faces) + ROW_GAP;
        let full = (all.len() + groups) as f32 * line;
        // The found line takes its place from the list's.
        let found = if search.found.is_some() {
            6.0 + FOUND_LINE + ui.spacing().item_spacing.y
        } else {
            0.0
        };
        let height = full.min((screen.height() - 240.0).max(120.0)) - found;
        // The list's place is kept whatever the search leaves in it.
        let (place, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let mut list = ui.new_child(egui::UiBuilder::new().max_rect(place));
        egui::ScrollArea::both()
            .max_height(height)
            .auto_shrink([false, false])
            .show(&mut list, |ui| {
                if listed.is_empty() {
                    nothing_listed(ui, paint, search);
                    return;
                }
                egui::Grid::new("shortcuts")
                    .num_columns(2)
                    .spacing([GAP, ROW_GAP])
                    .show(ui, |ui| {
                        let mut last = None;
                        for row in &listed {
                            // Each group under its name, in the first
                            // column: the keys' own.
                            if last != Some(row.group) {
                                last = Some(row.group);
                                let name = look.label(&gettext(locale, heading(row.group)));
                                widgets::label(ui, heading_role, &name, palette.dim, &look);
                                ui.end_row();
                            }
                            let code = widgets::code(&look);
                            let hit: Vec<_> = row.keys_hit.iter().cloned().collect();
                            let marks = (code, palette.secondary, code, palette.accent_hover);
                            crate::ui::start::marked(&look, &row.keys, &hit, marks)
                                .layout(ui.ctx())
                                .label(ui);
                            let body = widgets::body(&look);
                            let name = gettext(locale, row.command.info().name);
                            let marks = (body, palette.text, heading_role, palette.accent_hover);
                            crate::ui::start::marked(&look, &name, &row.spans, marks)
                                .layout(ui.ctx())
                                .label(ui);
                            ui.end_row();
                        }
                    });
            });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if widgets::button(ui, &gettext(locale, "About Tabletist"), &look).clicked() {
                actions.push(Action::ShowAbout);
            }
            if widgets::button(ui, &gettext(locale, "Settings"), &look).clicked() {
                actions.push(Action::ShowSettings);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if widgets::button(ui, &gettext(locale, "Close"), &look).clicked() {
                    actions.push(Action::CloseDialog);
                }
            });
        });
        // The terminal look's dialogs end in their keys: what Esc does now.
        if look.terminal {
            ui.add_space(8.0);
            let (rect, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
            let esc = key_written(layout, "esc");
            let does = say(if searching { "clear" } else { "close" });
            let typing = say("type to filter, or press keys to find them");
            let none = Written::default();
            let hints = [(&none, typing.as_str(), true), (&esc, does.as_str(), true)];
            widgets::key_hints(
                ui,
                (rect.left(), rect.center().y),
                &hints,
                16.0,
                &look,
                &palette,
            );
        }
    });
    actions
}

/// The search field: the look's search field, or Omarchy's `/` line with
/// the count of what it leaves at its end. It has the keyboard as the
/// dialog opens, and keeps it through Esc, which the dialog reads.
fn field(
    ui: &mut egui::Ui,
    paint: Paint,
    search: &HelpSearch,
    count: usize,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let mut text = search.search.clone();
    let width = ui.available_width();
    let response = if look.terminal {
        let hint = look.label(&gettext(
            locale,
            "filter commands, or press a key to find it",
        ));
        let rect = Rect::from_min_size(ui.cursor().min, vec2(width, look.control_height));
        let style = widgets::FieldStyle {
            fill: None,
            boxed: true,
            role: TextRole::OBody,
        };
        let response =
            widgets::filter_field(ui, &mut text, &hint, rect.size(), style, &look, &palette);
        if !search.search.is_empty() {
            let ink = if count == 0 {
                palette.danger
            } else {
                palette.dim
            };
            let said = Text::one(&look, TextRole::OBody, &count.to_string(), ink);
            widgets::paint_text_right(ui, rect.right() - 8.0, rect.center().y, said);
        }
        response
    } else {
        let hint = gettext(locale, "Search commands or press a shortcut");
        let field = widgets::search_field(ui, &mut text, &hint, &look)
            .id(field_id())
            .desired_width(width);
        widgets::add_search(ui, field, &look)
    };
    if response.changed() {
        actions.push(Action::HelpSearch(text));
    }
    let id = response.id;
    ui.ctx()
        .data_mut(|data| data.insert_temp(last_field_id(), id));
    let opened: bool = ui
        .ctx()
        .data(|data| data.get_temp(opened_id()))
        .unwrap_or(false);
    if !opened {
        response.request_focus();
        ui.ctx()
            .data_mut(|data| data.insert_temp(opened_id(), true));
    }
    // Esc is the dialog's: the field keeps the keyboard through it. Tab
    // still goes on to the buttons.
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                tab: false,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
}

/// What the list is cut to while a chord is looked for, and the link that
/// lists every command again: Settings › Keyboard's line.
fn found_line(ui: &mut egui::Ui, paint: Paint, found: &'static str, actions: &mut Vec<Action>) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let top = ui.cursor().min;
    let role = TextRole::pick(&look, TextRole::Secondary, TextRole::OSecondary);
    let height = FOUND_LINE;
    let middle = top.y + height / 2.0;
    let key = key_written(paint.layout(), found);
    let said = look
        .label(&gettext(locale, "Commands bound to {keys}"))
        .replace("{keys}", key.as_str());
    let width = widgets::paint_label(
        ui,
        top.x,
        middle,
        Text::one(&look, role, &said, palette.secondary),
    );
    let all = look.label(&gettext(locale, "Show all"));
    let all_width = role.width(ui.ctx(), look.faces, &all).ceil();
    let rect = Rect::from_min_size(pos2(top.x + width + 12.0, top.y), vec2(all_width, height));
    let link = ui.interact(rect, ui.id().with("help-show-all"), Sense::click());
    link.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Link, true, &all));
    let color = if link.hovered() {
        palette.accent_hover
    } else {
        palette.accent
    };
    widgets::paint_text(ui, rect.left(), middle, Text::one(&look, role, &all, color));
    focus::hint(ui, &link, rect, Ring::Outer { radius: 3 });
    if link.clicked() {
        actions.push(Action::HelpFind(None));
    }
    ui.advance_cursor_after_rect(Rect::from_min_size(top, vec2(ui.available_width(), height)));
}

/// What the list says when the search leaves nothing, and how to have it
/// all again.
fn nothing_listed(ui: &mut egui::Ui, paint: Paint, search: &HelpSearch) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let said = match search.found {
        Some(keys) => gettext(locale, "No command is bound to {keys}")
            .replace("{keys}", key_written(paint.layout(), keys).as_str()),
        None => {
            gettext(locale, "No commands match “{words}”").replace("{words}", search.search.trim())
        }
    };
    let then = if look.terminal {
        "Esc clears the filter"
    } else {
        "Esc clears the search"
    };
    ui.vertical_centered(|ui| {
        ui.add_space(24.0);
        widgets::label(
            ui,
            widgets::body(&look),
            &look.label(&said),
            palette.dim,
            &look,
        );
        ui.add_space(6.0);
        let then = look.label(&gettext(locale, then));
        widgets::label(ui, widgets::secondary(&look), &then, palette.dim, &look);
    });
}
