//! Quick open (`⌘P`, Omarchy `ctrl+p`): a panel over the window that finds
//! a table or view of the window's connection by name, or of every open
//! connection (`⇥`, `tab`), and in the macOS and Windows looks starts a
//! few commands. What it lists is [`App::quick_open_list`]'s; this draws
//! it and reads its keys, which are its own while it is open (the window's
//! handler does not run under a dialog). macOS: a floating panel 640 wide,
//! 84 from the window's top, the window behind it not dimmed. Omarchy: the
//! picker, 780 wide, an accent border on the sunken surface.

use std::collections::HashMap;

use egui::accesskit::Role;
use egui::{Align2, Color32, CornerRadius, Rect, Sense, Stroke, StrokeKind, UiBuilder};
use egui::{WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::ObjectKind;

use crate::app::{
    App, Found, QuickCommand, QuickHeading, QuickItem, QuickList, QuickRow, StartRow,
};
use crate::env::{Environment, Platform, env_colors};
use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Keymap, Layout, Scope, Written};
use crate::model::{Action, AgentsHere, ConnTabId, Dialog, QuickCaret, QuickGroup, StartRecent};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format::display_safe;
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::workspace::{Badge, env_badge};

/// The panel's id: its area's, and the root of its parts' ids.
fn panel_id() -> egui::Id {
    egui::Id::new("quick-open")
}

/// Where quick open's field keeps the keyboard.
pub(crate) fn field_id() -> egui::Id {
    panel_id().with("field")
}

/// The list's id: the listbox the field controls.
fn list_id() -> egui::Id {
    panel_id().with("list")
}

/// The panel's width and how far below the window's top it floats.
fn place(look: &Look) -> (f32, f32) {
    if look.terminal {
        (780.0, 120.0)
    } else {
        (640.0, 84.0)
    }
}

/// A result's row, and a heading's. The terminal look's headings are a
/// row's height, so that its eight rows are eight lines of either.
fn heights(look: &Look) -> (f32, f32) {
    if look.terminal {
        (24.0, 24.0)
    } else {
        (36.0, 28.0)
    }
}

/// How many rows show before the list scrolls.
fn rows_shown(look: &Look) -> usize {
    if look.terminal { 8 } else { 9 }
}

/// A window quick open names: its connection, environment and database.
struct Window {
    name: String,
    env: Environment,
    database: String,
}

/// The keys the panel names and reads, as the keymap writes them.
struct Keys {
    moves: Written,
    open: Written,
    editor: Written,
    scope: Written,
    number: Written,
    close: Written,
}

fn keys(keymap: &Keymap, layout: Layout) -> Keys {
    let key = |command| keymap.key_in(layout, command, Scope::Prompt);
    Keys {
        moves: keymap.label(layout, Command::QuickOpenMove),
        open: key(Command::QuickOpenPick),
        editor: key(Command::QuickOpenPickInEditor),
        scope: key(Command::QuickOpenScope),
        number: key(Command::QuickOpenNumber),
        close: key(Command::QuickOpenClose),
    }
}

/// What the panel draws with, gathered before the dialog is borrowed.
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
    /// Seconds since the Unix epoch, for how long ago a recent was opened.
    now: u64,
    /// Whether numbers are written in threes.
    grouped: bool,
    windows: &'a HashMap<ConnTabId, Window>,
    /// The window quick open was opened in.
    here: ConnTabId,
    /// Who is connected to the window's Agents tab: the Agents command
    /// says it after the connection's name.
    agents: Option<String>,
    /// The commands' keys, as the window reads them.
    command_keys: &'a HashMap<Command, Written>,
    /// The key that opens the picked result, written on its row (macOS).
    open_key: String,
}

impl Skin<'_> {
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }

    /// A window's connection as the chip and the headings name it: its
    /// name and its environment (`Bookshop · dev`).
    fn window_name(&self, tab: ConnTabId) -> String {
        let Some(window) = self.windows.get(&tab) else {
            return String::new();
        };
        let name = display_safe(&window.name);
        if window.env == Environment::None {
            name.into_owned()
        } else {
            format!("{name} · {}", window.env.label(Platform::Native))
        }
    }
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(Dialog::QuickOpen(open)) = &app.dialog else {
        return;
    };
    let here = open.tab;
    // Quick open goes with the window it was opened in.
    if app.workspace(here).is_none() {
        app.actions.push(Action::CloseDialog);
        return;
    }
    let list = app.quick_open_list(open);
    let layout = app.layout();
    let keymap = app.keymap.clone();
    let keys = keys(&keymap, layout);
    let windows: HashMap<ConnTabId, Window> = app
        .open_connections()
        .map(|(tab, workspace)| {
            let database = crate::ui::workspace::bar_info(app, tab)
                .map(|info| info.database)
                .unwrap_or_default();
            let window = Window {
                name: workspace.name.clone(),
                env: workspace.environment,
                database,
            };
            (tab, window)
        })
        .collect();
    let agents = app.agents_here(here).map(|state| {
        let said = match &state {
            AgentsHere::ServerOff => gettext(locale, "server off").into_owned(),
            AgentsHere::OffHere => gettext(locale, "off here").into_owned(),
            AgentsHere::NoneConnected => gettext(locale, "none connected").into_owned(),
            AgentsHere::Connected { agent, .. } => agent.clone(),
        };
        look.label(&said)
    });
    let command_keys: HashMap<Command, Written> = [
        Command::NewSqlTab,
        Command::SwitchDatabase,
        Command::ShowAgents,
    ]
    .into_iter()
    .map(|command| (command, keymap.key_in(layout, command, Scope::Global)))
    .collect();
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
        now: crate::util::now_secs(),
        grouped: app.settings.grouped(layout),
        windows: &windows,
        here,
        agents,
        command_keys: &command_keys,
        open_key: keys.open.as_str().to_owned(),
    };
    let mut actions = Vec::new();
    // The panel's keys come before its field sees the frame: Tab, the
    // arrows, Enter and Esc are quick open's, and so are the digits while
    // nothing is typed (Omarchy).
    // Nothing typed but spaces lists what nothing typed lists, numbers
    // and all.
    let nothing_typed = open.query.trim().is_empty();
    ctx.input_mut(|input| read_keys(input, &keymap, layout, nothing_typed, &mut actions));
    let Some(Dialog::QuickOpen(open)) = &mut app.dialog else {
        return;
    };
    let (width, top) = place(&look);
    let width = width.min(ctx.content_rect().width() - 32.0).max(200.0);
    let area = egui::Modal::default_area(panel_id()).anchor(Align2::CENTER_TOP, vec2(0.0, top));
    let modal = egui::Modal::new(panel_id())
        .area(area)
        .frame(frame(&look, &palette))
        // The window behind is not dimmed.
        .backdrop_color(Color32::TRANSPARENT)
        .show(ctx, |ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let name = if look.terminal {
                gettext(locale, "Find object")
            } else {
                gettext(locale, "Quick open")
            };
            ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
                node.set_role(Role::Dialog);
                node.set_label(name.into_owned());
            });
            let caret = open.caret.take();
            let changed = field(ui, open, &list, &skin, caret);
            if changed {
                actions.push(Action::QuickOpenChanged);
            }
            let selected = open.selected.min(list.items().count().saturating_sub(1));
            if list.is_empty() {
                no_match(
                    ui,
                    (&open.query, open.everywhere),
                    &keys,
                    &skin,
                    &mut actions,
                );
            } else {
                // Scroll to the pick only when it moved, so a wheel scroll
                // is not undone on the next frame.
                let reveal = open.scrolled_to != Some(selected);
                open.scrolled_to = Some(selected);
                let offset = rows(ui, &list, selected, reveal, &skin, &mut actions);
                open.scroll_offset = offset;
            }
            footer(ui, open.everywhere, nothing_typed, &keys, &skin);
        });
    if modal.backdrop_response.clicked() {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}

/// The panel's frame: the dialog's corner and shadow, its parts edge to
/// edge (macOS); Omarchy's accent border on the sunken surface.
fn frame(look: &Look, palette: &Palette) -> egui::Frame {
    if look.terminal {
        egui::Frame::new()
            .fill(palette.panel)
            .stroke(Stroke::new(1.0, palette.accent))
            .corner_radius(CornerRadius::same(3))
    } else {
        widgets::modal_frame(look, palette)
            .fill(palette.window)
            .inner_margin(egui::Margin::ZERO)
    }
}

/// Reads the panel's keys out of the frame: Esc closes; the arrows (and
/// Omarchy's ctrl+n/p) move the pick; Enter opens it, `⌥↩` in a SQL
/// editor; Tab switches the scope; a digit opens that numbered row while
/// nothing is typed (Omarchy), and then is no text for the field; `⌘P`
/// again selects what is typed.
fn read_keys(
    input: &mut egui::InputState,
    keymap: &Keymap,
    layout: Layout,
    nothing_typed: bool,
    actions: &mut Vec<Action>,
) {
    use crate::ui::keys::{asked, digit_asked, presses};
    let scope = Scope::Prompt;
    let on = |input: &mut egui::InputState, command| {
        asked(input, keymap, layout, command, scope, true).count > 0
    };
    if on(input, Command::QuickOpenClose) {
        actions.push(Action::CloseDialog);
        return;
    }
    // The keymap lists the keys that move up first, then as many that
    // move down.
    let moves: Vec<&'static str> = keymap
        .chords_now(layout, Command::QuickOpenMove, scope, true)
        .collect();
    let half = moves.len() / 2;
    for (nth, keys) in moves.into_iter().enumerate() {
        let step = if nth < half { -1 } else { 1 };
        for _ in 0..presses(input, layout, keys).count {
            actions.push(Action::QuickOpenMove(step));
        }
    }
    if on(input, Command::QuickOpenPickInEditor) {
        actions.push(Action::QuickOpenPick {
            index: None,
            in_editor: true,
        });
    }
    if on(input, Command::QuickOpenPick) {
        actions.push(Action::QuickOpenPick {
            index: None,
            in_editor: false,
        });
    }
    if on(input, Command::QuickOpenScope) {
        actions.push(Action::QuickOpenScope);
    }
    if nothing_typed
        && let Some(index) =
            digit_asked(input, keymap, layout, Command::QuickOpenNumber, scope, true)
    {
        // The digit's text goes with its key: the field must not get it.
        input.events.retain(|event| {
            !matches!(event, egui::Event::Text(text)
                if text.len() == 1 && text.chars().all(|char| char.is_ascii_digit()))
        });
        actions.push(Action::QuickOpenPick {
            index: Some(index),
            in_editor: false,
        });
    }
    if asked(
        input,
        keymap,
        layout,
        Command::FindObject,
        Scope::Global,
        true,
    )
    .count
        > 0
    {
        actions.push(Action::OpenQuickOpen);
    }
}

/// The field's line. macOS: a magnifier, the scope's chip (the window's
/// connection with its environment's dot, the production pill, or "All
/// open connections") and the field with its placeholder. Omarchy: the
/// `>` prompt, the scope's label (`DEV`, `PROD`, `ALL`), the field with a
/// block cursor, and the count of matches at the right. The field is a
/// combobox that controls the list. Returns whether what is typed changed.
fn field(
    ui: &mut egui::Ui,
    open: &mut crate::model::QuickOpen,
    list: &QuickList,
    skin: &Skin<'_>,
    caret: Option<QuickCaret>,
) -> bool {
    let (look, palette) = (skin.look, skin.palette);
    let id = field_id();
    let height = if look.terminal { 32.0 } else { 52.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let pad = if look.terminal { 10.0 } else { 14.0 };
    let y = rect.center().y;
    let mut x = rect.left() + pad;
    if look.terminal {
        let prompt = Text::one(look, TextRole::OGroup, ">", palette.accent);
        x += widgets::paint_text(ui, x, y, prompt) + 10.0;
        x += scope_label(ui, (x, y), open.everywhere, skin) + 10.0;
    } else {
        let icon = Rect::from_center_size(pos2(x + 8.0, y), vec2(16.0, 16.0));
        Icon::Search.image(palette.dim, 16.0).paint_at(ui, icon);
        x += 16.0 + 10.0;
        x += chip(ui, (x, y), open.everywhere, skin) + 10.0;
    }
    // The count at the right (Omarchy): how many match, and how many of
    // them the list's height hides.
    let mut right = rect.right() - pad;
    if look.terminal && !open.query.trim().is_empty() {
        let count = list.matches;
        // The results below the lines the list shows at once.
        let hidden = list
            .rows
            .iter()
            .skip(rows_shown(look))
            .filter(|row| matches!(row, QuickRow::Item(QuickItem::Object { .. })))
            .count();
        let mut said = if count == 0 {
            "0".to_owned()
        } else {
            let words = if count == 1 { "match" } else { "matches" };
            format!("{count} {}", skin.say(words))
        };
        if hidden > 0 {
            said = format!("{said} · {hidden} {}", skin.say("hidden"));
        }
        let text = Text::one(look, TextRole::OBody, &said, palette.dim);
        right -= widgets::paint_text_right(ui, right, y, text) + 10.0;
    }
    let role = widgets::body(look);
    let line = role.row_height(ui.ctx(), look.faces);
    let inner = Rect::from_min_max(
        pos2(x, y - role.middle(ui.ctx(), look.faces)),
        pos2(right.max(x), y - role.middle(ui.ctx(), look.faces) + line),
    );
    if let Some(caret) = caret {
        use egui::text::{CCursor, CCursorRange};
        let mut state = egui::text_edit::TextEditState::load(ui.ctx(), id).unwrap_or_default();
        let end = CCursor::new(open.query.chars().count());
        let range = match caret {
            QuickCaret::All => CCursorRange::two(CCursor::new(0), end),
            QuickCaret::End => CCursorRange::one(end),
        };
        state.cursor.set_char_range(Some(range));
        state.store(ui.ctx(), id);
    }
    // What the field finds, and no more: tables, views and (macOS) the
    // commands. Omarchy's prompt shows no placeholder, as its board.
    let hint = if look.terminal {
        skin.say("Find a table or view")
    } else {
        skin.say("Find a table, view or command")
    };
    let placeholder = if look.terminal {
        String::new()
    } else {
        hint.clone()
    };
    let mut child = ui.new_child(UiBuilder::new().max_rect(inner));
    let placeholder = Text::one(look, role, &placeholder, palette.dim).layout(child.ctx());
    let output = egui::TextEdit::singleline(&mut open.query)
        .id(id)
        .font(role.font_id(look.faces))
        .frame(egui::Frame::NONE)
        .margin(egui::Margin::ZERO)
        .desired_width(inner.width())
        .text_color(palette.text)
        .hint_text(placeholder.galley)
        .show(&mut child);
    let response = &output.response;
    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &hint));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(Role::EditableComboBox);
        node.push_controlled(list_id().accesskit_id());
        node.set_expanded(true);
    });
    // The panel's one field: its caret (Omarchy's block) says where the
    // keyboard is, and no ring is drawn round it, as the boards draw it.
    crate::ui::focus::hint(ui, response, inner, crate::ui::focus::Ring::Own);
    if look.terminal {
        crate::ui::cell_editor::block_cursor(ui, &output, role, look, palette);
    }
    // The field keeps the keyboard while quick open is open: Tab, the
    // arrows and Esc are the panel's, read before the field.
    response.request_focus();
    ui.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
    response.changed()
}

/// The macOS chip of what is searched, at `x`: the window's connection
/// with its environment's dot, on production the red pill, or "All open
/// connections" on the accent's soft ground. Returns its width.
fn chip(ui: &egui::Ui, (x, y): (f32, f32), everywhere: bool, skin: &Skin<'_>) -> f32 {
    let (look, palette) = (skin.look, skin.palette);
    let env = skin
        .windows
        .get(&skin.here)
        .map_or(Environment::None, |window| window.env);
    let role = TextRole::Secondary;
    let (words, fill, border, ink, dot) = if everywhere {
        let words = skin.say("All open connections");
        let border = palette.accent.gamma_multiply(0.35);
        (words, palette.selection, border, palette.accent_hover, None)
    } else if env.is_production() {
        // The token `on-danger`: white on the danger red, in both modes.
        let words = env.label(Platform::Native).to_owned();
        (words, palette.danger, palette.danger, Color32::WHITE, None)
    } else {
        let colors = env_colors(env, Platform::of(look), palette);
        let words = skin.window_name(skin.here);
        let dot = (env != Environment::None).then(|| colors.base());
        (
            words,
            Color32::TRANSPARENT,
            palette.border,
            palette.secondary,
            dot,
        )
    };
    let laid = Text::one(look, role, &words, ink).layout(ui.ctx());
    let dot_room = if dot.is_some() { 7.0 + 6.0 } else { 0.0 };
    let width = 8.0 + dot_room + laid.width() + 8.0;
    let rect = Rect::from_min_size(pos2(x, y - 12.0), vec2(width, 24.0));
    let corner = CornerRadius::same(12);
    ui.painter().rect_filled(rect, corner, fill);
    ui.painter()
        .rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
    let mut left = rect.left() + 8.0;
    if let Some(dot) = dot {
        ui.painter().circle_filled(pos2(left + 3.5, y), 3.5, dot);
        left += dot_room;
    }
    laid.paint_left(ui.painter(), left, y);
    widgets::announce(ui, rect, &words);
    width
}

/// The Omarchy prompt's label of what is searched, at `x`: the window's
/// environment (`DEV`, `PROD` in the danger red) or `ALL` on the accent.
/// Returns its width.
fn scope_label(ui: &egui::Ui, (x, y): (f32, f32), everywhere: bool, skin: &Skin<'_>) -> f32 {
    let (look, palette) = (skin.look, skin.palette);
    if everywhere {
        let laid = Text::one(look, TextRole::OBadge, "ALL", palette.on_accent).layout(ui.ctx());
        let rect = Rect::from_min_size(
            pos2(x, y - laid.height() / 2.0 - 1.0),
            vec2(laid.width() + 12.0, laid.height() + 2.0),
        );
        ui.painter()
            .rect_filled(rect, CornerRadius::same(2), palette.accent);
        laid.paint_left(ui.painter(), x + 6.0, y);
        widgets::announce(ui, rect, "ALL");
        return rect.width();
    }
    let env = skin
        .windows
        .get(&skin.here)
        .map_or(Environment::None, |window| window.env);
    let colors = env_colors(env, Platform::Omarchy, palette);
    let width = env_badge(ui, x, y, env, &colors, Badge::Plain, look);
    let rect = Rect::from_min_size(pos2(x, y - 8.0), vec2(width, 16.0));
    widgets::announce(ui, rect, env.label(Platform::Omarchy));
    width
}

/// The words of a result: its icon, name with what matched marked, the
/// words after it (macOS) or its kind (Omarchy), what stands at its right,
/// and what a screen reader says of it.
struct Shown {
    icon: Icon,
    name: String,
    spans: Vec<std::ops::Range<usize>>,
    /// macOS: the kind and the schema, or what a command does. Omarchy:
    /// the kind.
    detail: String,
    meta: String,
    label: String,
    /// A group's "more" row: the name in the accent, no icon.
    more: bool,
}

fn kind_word(kind: ObjectKind, skin: &Skin<'_>) -> String {
    skin.say(match kind {
        ObjectKind::Table => "table",
        ObjectKind::View => "view",
        ObjectKind::MaterializedView => "materialized view",
    })
}

fn kind_icon(kind: ObjectKind) -> Icon {
    match kind {
        ObjectKind::Table => Icon::Table,
        ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
    }
}

/// An object's result: `name`, `kind · schema` (Omarchy: the kind), and
/// at its right `meta`.
fn object_shown(
    object: &tabletist_db::ObjectRef,
    kind: ObjectKind,
    spans: Vec<std::ops::Range<usize>>,
    meta: String,
    skin: &Skin<'_>,
) -> Shown {
    let name = object.name.clone();
    let schema = display_safe(&object.schema).into_owned();
    let kind_word = kind_word(kind, skin);
    let detail = if skin.look.terminal {
        kind_word.clone()
    } else {
        format!("{kind_word} · {schema}")
    };
    let mut label = format!("{}, {kind_word}, {schema}", display_safe(&name));
    if !meta.is_empty() {
        label = format!("{label}, {meta}");
    }
    Shown {
        icon: kind_icon(kind),
        name,
        spans,
        detail,
        meta,
        label,
        more: false,
    }
}

fn shown(item: &QuickItem, skin: &Skin<'_>) -> Shown {
    let look = skin.look;
    let estimate = |rows: Option<u64>| {
        rows.map_or_else(String::new, |rows| {
            crate::ui::start::estimate_words(rows, look, skin.grouped, skin.locale)
        })
    };
    match item {
        QuickItem::Object { found, .. } => {
            let Found {
                object,
                kind,
                estimate: rows,
                spans,
                ..
            } = found;
            object_shown(object, *kind, spans.clone(), estimate(*rows), skin)
        }
        QuickItem::Start { row, .. } => {
            let zone = jiff::tz::TimeZone::system();
            match row {
                StartRow::Largest(object, rows) => {
                    object_shown(object, ObjectKind::Table, Vec::new(), estimate(*rows), skin)
                }
                StartRow::Recent(StartRecent::Object {
                    object,
                    kind,
                    opened,
                }) => {
                    let when = look.label(&crate::recents::when(*opened, skin.now, &zone));
                    object_shown(object, *kind, Vec::new(), when, skin)
                }
                StartRow::Recent(StartRecent::Sql {
                    number,
                    comment,
                    opened,
                    ..
                }) => {
                    let when = look.label(&crate::recents::when(*opened, skin.now, &zone));
                    let name = format!("{} {number}", skin.say("Query"));
                    let editor = skin.say("SQL editor");
                    let detail = if look.terminal {
                        skin.say("sql")
                    } else {
                        comment.clone().unwrap_or_else(|| editor.clone())
                    };
                    let label = match comment {
                        Some(comment) => format!("{name}, {editor}, {comment}, {when}"),
                        None => format!("{name}, {editor}, {when}"),
                    };
                    Shown {
                        icon: Icon::Code,
                        name,
                        spans: Vec::new(),
                        detail,
                        meta: when,
                        label,
                        more: false,
                    }
                }
            }
        }
        QuickItem::Command { command, spans } => {
            let name = gettext(skin.locale, command.name()).into_owned();
            let key = skin
                .command_keys
                .get(&command.command())
                .map(|key| key.as_str().to_owned())
                .unwrap_or_default();
            let (icon, detail) = match command {
                QuickCommand::NewSql { read_only } => {
                    let detail = if *read_only {
                        skin.say("opens read-only")
                    } else {
                        String::new()
                    };
                    (Icon::Code, detail)
                }
                QuickCommand::SwitchDatabase => (Icon::Database, String::new()),
                QuickCommand::Agents => {
                    let mut detail = skin.window_name(skin.here);
                    if let Some(said) = &skin.agents {
                        detail = format!("{detail} · {said}");
                    }
                    (Icon::SquareTerminal, detail)
                }
            };
            let mut label = format!("{name}, {}", skin.say("command"));
            for words in [&detail, &key] {
                if !words.is_empty() {
                    label = format!("{label}, {words}");
                }
            }
            Shown {
                icon,
                name,
                spans: spans.clone(),
                detail,
                meta: key,
                label,
                more: false,
            }
        }
        QuickItem::More { group, hidden } => {
            let name = match group {
                QuickGroup::Tables if *hidden == 1 => skin.say("1 more table"),
                QuickGroup::Tables => format!("{hidden} {}", skin.say("more tables")),
                QuickGroup::Views if *hidden == 1 => skin.say("1 more view"),
                QuickGroup::Views => format!("{hidden} {}", skin.say("more views")),
                QuickGroup::Connection(tab) => format!(
                    "{hidden} {} {}",
                    skin.say("more in"),
                    skin.window_name(*tab)
                ),
            };
            Shown {
                icon: Icon::Ellipsis,
                label: name.clone(),
                name,
                spans: Vec::new(),
                detail: String::new(),
                meta: String::new(),
                more: true,
            }
        }
    }
}

/// The list: its headings and results, scrolling past the first rows
/// that fit, only the rows in view drawn. The picked result is on the
/// selection's ground. A click opens a result. Returns the list's scroll
/// offset.
fn rows(
    ui: &mut egui::Ui,
    list: &QuickList,
    selected: usize,
    reveal: bool,
    skin: &Skin<'_>,
    actions: &mut Vec<Action>,
) -> f32 {
    let look = skin.look;
    let (row, heading) = heights(look);
    let pad = if look.terminal { 0.0 } else { 6.0 };
    // Each row's top and height in the list.
    let mut y = pad;
    let placed: Vec<(f32, f32)> = list
        .rows
        .iter()
        .map(|entry| {
            let height = match entry {
                QuickRow::Heading(_) => heading,
                QuickRow::Item(_) => row,
            };
            let top = y;
            y += height;
            (top, height)
        })
        .collect();
    let total = y + pad;
    let count = list.items().count();
    let max = rows_shown(look) as f32 * row + 2.0 * pad;
    let width = ui.available_width();
    let output = egui::ScrollArea::vertical()
        .id_salt(panel_id().with("scroll"))
        .max_height(max)
        .auto_shrink([false, true])
        .show_viewport(ui, |ui, viewport| {
            ui.set_height(total);
            ui.set_width(width);
            let origin = ui.max_rect().min;
            let whole = ui.interact(
                Rect::from_min_size(origin, vec2(width, total)),
                list_id(),
                Sense::hover(),
            );
            whole.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, "Results"));
            ui.ctx().accesskit_node_builder(list_id(), |node| {
                node.set_role(Role::ListBox);
                node.set_size_of_set(count);
            });
            let mut index = 0;
            let mut inner = ui.new_child(
                UiBuilder::new()
                    .max_rect(ui.max_rect())
                    .accessibility_parent(list_id()),
            );
            for (entry, (top, height)) in list.rows.iter().zip(&placed) {
                let rect =
                    Rect::from_min_size(origin + vec2(pad, *top), vec2(width - 2.0 * pad, *height));
                let seen = *top + height >= viewport.min.y && *top <= viewport.max.y;
                match entry {
                    QuickRow::Heading(title) => {
                        if seen {
                            heading_row(&mut inner, rect, *title, skin);
                        }
                    }
                    QuickRow::Item(item) => {
                        let picked = index == selected;
                        if picked && reveal {
                            inner.scroll_to_rect(rect, None);
                        }
                        if seen || picked {
                            let response =
                                result_row(&mut inner, rect, item, (index, count), picked, skin);
                            if response.clicked() {
                                actions.push(Action::QuickOpenPick {
                                    index: Some(index),
                                    in_editor: false,
                                });
                            }
                        }
                        index += 1;
                    }
                }
            }
        });
    output.state.offset.y
}

/// A heading over a group: Recent, Tables, Views, Commands, or a
/// connection's name with its environment (Omarchy: in the danger red on
/// production). A label, never a result.
fn heading_row(ui: &mut egui::Ui, rect: Rect, title: QuickHeading, skin: &Skin<'_>) {
    let (look, palette) = (skin.look, skin.palette);
    let words = |text: &'static str| gettext(skin.locale, text).into_owned();
    let (text, env) = match title {
        QuickHeading::Recent => (words("Recent"), Environment::None),
        QuickHeading::Largest => (words("Largest tables"), Environment::None),
        QuickHeading::Tables => (words("Tables"), Environment::None),
        QuickHeading::Views => (words("Views"), Environment::None),
        QuickHeading::Commands => (words("Commands"), Environment::None),
        QuickHeading::Connection(tab) => skin.windows.get(&tab).map_or_else(
            || (String::new(), Environment::None),
            |window| (display_safe(&window.name).into_owned(), window.env),
        ),
    };
    let inset = 10.0;
    let y = rect.bottom() - 4.0 - 8.0;
    if look.terminal {
        // As the brief writes them: `bookshop · dev`, and `bookshop ·
        // PROD` in the danger red.
        let ink = if env.is_production() {
            palette.danger
        } else {
            palette.dim
        };
        let text = if env == Environment::None {
            text.to_lowercase()
        } else if env.is_production() {
            format!("{} · {}", text.to_lowercase(), env.label(Platform::Omarchy))
        } else {
            format!("{} · {}", text.to_lowercase(), env.label(Platform::Native))
        };
        let laid = Text::one(look, TextRole::OGroup, &text, ink);
        widgets::paint_label(ui, rect.left() + inset, y, laid);
        return;
    }
    let laid = widgets::section_label(&text, look, palette).layout(ui.ctx());
    let width = laid.paint_left(ui.painter(), rect.left() + inset, y);
    widgets::announce(
        ui,
        Rect::from_min_size(rect.min, vec2(width + inset, rect.height())),
        &text,
    );
    if env != Environment::None {
        let colors = env_colors(env, Platform::Native, palette);
        env_badge(
            ui,
            rect.left() + inset + width + 8.0,
            y,
            env,
            &colors,
            Badge::Mac,
            look,
        );
    }
}

/// One result in `rect`: number `index` of `count`. macOS: its icon, the
/// name with what matched in the strong accent, the words after it, what
/// stands at its right, and `↩` on the picked one. Omarchy: its number,
/// the name with what matched in the accent, its kind in a column of its
/// own and what stands at its right; the picked one on the accent's soft
/// ground with a bar of the accent at its left. Never focused: the field
/// keeps the keyboard, and its node names the picked option.
fn result_row(
    ui: &mut egui::Ui,
    rect: Rect,
    item: &QuickItem,
    (index, count): (usize, usize),
    picked: bool,
    skin: &Skin<'_>,
) -> egui::Response {
    let (look, palette) = (skin.look, skin.palette);
    let id = panel_id().with(("result", index));
    let response = ui.interact(rect, id, Sense::click());
    let shown = shown(item, skin);
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &shown.label));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_role(Role::ListBoxOption);
        node.set_selected(picked);
        node.set_position_in_set(index + 1);
        node.set_size_of_set(count);
    });
    if picked {
        ui.ctx().accesskit_node_builder(field_id(), |node| {
            node.set_active_descendant(id.accesskit_id());
        });
    }
    let y = rect.center().y;
    if look.terminal {
        if picked {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, palette.selection);
            let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
            ui.painter()
                .rect_filled(bar, CornerRadius::ZERO, palette.accent);
        } else if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
        }
        let left = rect.left() + 10.0;
        let right = rect.right() - 10.0;
        let number = (index + 1).to_string();
        let number = Text::one(look, TextRole::OGroup, &number, palette.accent);
        widgets::paint_text(ui, left, y, number);
        let meta = Text::one(look, TextRole::OBody, &shown.meta, palette.dim);
        widgets::paint_text_right(ui, right, y, meta);
        let kind_x = right - 150.0 - 110.0;
        let detail = Text::one(look, TextRole::OBody, &shown.detail, palette.dim);
        widgets::paint_text(ui, kind_x, y, detail);
        let room = (kind_x - 10.0 - (left + 28.0)).max(0.0);
        let ink = if shown.more {
            palette.accent
        } else {
            palette.text
        };
        let runs = (TextRole::OBody, ink, TextRole::OGroup, palette.accent);
        let name = crate::ui::start::marked(look, &shown.name, &shown.spans, runs);
        let laid = one_line(name, room).layout(ui.ctx());
        laid.paint_left(ui.painter(), left + 28.0, y);
        full_on_hover(&response, &laid, &shown.name);
        return response;
    }
    let corner = CornerRadius::same(6);
    if picked {
        ui.painter().rect_filled(rect, corner, palette.selection);
    } else if response.hovered() {
        ui.painter()
            .rect_filled(rect, corner, palette.surface_hover);
    }
    let left = rect.left() + 10.0;
    let mut right = rect.right() - 10.0;
    let strong = palette.accent_hover;
    if !shown.more {
        let ink = if picked { strong } else { palette.dim };
        let icon = Rect::from_center_size(pos2(left + 9.0, y), vec2(16.0, 16.0));
        shown.icon.image(ink, 16.0).paint_at(ui, icon);
    }
    // The column of the key that opens it, on the picked row.
    let enter = Text::one(look, TextRole::Shortcut, &skin.open_key, palette.dim);
    if picked {
        widgets::paint_text_right(ui, right, y, enter);
    }
    right -= 20.0 + 10.0;
    let meta = || Text::one(look, widgets::secondary(look), &shown.meta, palette.dim);
    let meta_width = widgets::measure(ui, meta());
    widgets::paint_text_right(ui, right, y, meta());
    let x = left + 18.0 + 10.0;
    let room = (right - meta_width - 10.0 - x).max(0.0);
    let (role, ink) = if shown.more {
        (TextRole::UiBody, palette.accent)
    } else if picked {
        (TextRole::UiBodyStrong, strong)
    } else {
        (TextRole::UiBodyStrong, palette.text)
    };
    let marks = (role, ink, TextRole::UiBodySemibold, strong);
    let name = crate::ui::start::marked(look, &shown.name, &shown.spans, marks);
    let laid = one_line(name, room).layout(ui.ctx());
    let after = x + laid.width() + 8.0;
    laid.paint_left(ui.painter(), x, y);
    full_on_hover(&response, &laid, &shown.name);
    let room = (right - meta_width - 10.0 - after).max(0.0);
    let detail = crate::ui::grid::ellipsize(&shown.detail, room, false, |text| {
        widgets::secondary(look).width(ui.ctx(), look.faces, text)
    });
    widgets::paint_text(
        ui,
        after,
        y,
        Text::one(look, widgets::secondary(look), &detail, palette.dim),
    );
    response
}

/// `text` on one line `room` wide: what does not fit ends in "…".
fn one_line(mut text: Text, room: f32) -> Text {
    let wrap = &mut text.job_mut().wrap;
    wrap.max_width = room;
    wrap.max_rows = 1;
    wrap.break_anywhere = true;
    wrap.overflow_character = Some('…');
    text
}

/// A name cut short shows in full on hover.
fn full_on_hover(response: &egui::Response, laid: &crate::typography::Laid, name: &str) {
    if laid.galley.elided {
        response.clone().on_hover_text(display_safe(name));
    }
}

/// What the panel says when nothing matches: where it looked, and in this
/// connection's scope a way to look in every open one.
fn no_match(
    ui: &mut egui::Ui,
    (query, everywhere): (&str, bool),
    keys: &Keys,
    skin: &Skin<'_>,
    actions: &mut Vec<Action>,
) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let query = query.trim();
    let place = if everywhere {
        gettext(locale, "open connections").into_owned()
    } else if look.terminal {
        skin.windows
            .get(&skin.here)
            .map(|window| display_safe(&window.database).into_owned())
            .unwrap_or_default()
    } else {
        skin.window_name(skin.here)
    };
    let said = gettext(locale, "No objects match “{query}” in {place}")
        .replace("{query}", &display_safe(query))
        .replace("{place}", &place);
    let said = look.label(&said);
    let link = gettext(locale, "Search all open connections");
    let height = if look.terminal { 52.0 } else { 96.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if look.terminal {
        let y = rect.center().y;
        let x = rect.left() + 10.0;
        let text = Text::one(look, TextRole::OBody, &said, palette.secondary);
        let width = widgets::paint_label(ui, x, y, text);
        if !everywhere {
            let key = Text::one(look, TextRole::OBody, keys.scope.as_str(), palette.accent);
            let x = x + width + 24.0;
            let key_width = widgets::paint_text(ui, x, y, key);
            let words = look.label(&link);
            let words = Text::one(look, TextRole::OBody, &words, palette.dim);
            widgets::paint_label(ui, x + key_width + 8.0, y, words);
        }
        return;
    }
    let text = Text::one(look, TextRole::UiBody, &said, palette.secondary);
    let laid = text.layout(ui.ctx());
    let y = rect.top() + 36.0;
    laid.paint_left(ui.painter(), rect.center().x - laid.width() / 2.0, y);
    widgets::announce(ui, rect, &said);
    if !everywhere {
        let words = format!("{link} {}", keys.scope.as_str());
        let words = words.trim();
        let button = ButtonSpec::new(words).link();
        let button_width = button.width(ui, look);
        let place =
            Rect::from_center_size(pos2(rect.center().x, y + 28.0), vec2(button_width, 24.0));
        if button.show_at(ui, place, look, palette).clicked() {
            actions.push(Action::QuickOpenScope);
        }
    }
}

/// The key line under the list. macOS: `↑↓ move`, `↩ open`, `⌥↩ in SQL
/// editor`, `⇥` and the scope it switches to, `esc close` at the right, on
/// the sunken surface. Omarchy: the same in its words, `1..9 pick` only
/// while nothing is typed.
fn footer(ui: &mut egui::Ui, everywhere: bool, nothing_typed: bool, keys: &Keys, skin: &Skin<'_>) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let height = if look.terminal { 30.0 } else { 34.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    if !look.terminal {
        let corner = CornerRadius {
            sw: look.dialog_radius,
            se: look.dialog_radius,
            ..CornerRadius::ZERO
        };
        ui.painter().rect_filled(rect, corner, palette.panel);
    }
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let word = |text: &'static str| look.label(&gettext(locale, text));
    let scope = if everywhere {
        word("This connection")
    } else {
        word("All connections")
    };
    let (moves, open, editor, pick, close) = (
        word("move"),
        word("open"),
        word("in SQL editor"),
        word("pick"),
        word("close"),
    );
    let mut hints: Vec<widgets::Hint<'_>> =
        vec![(&keys.moves, &moves, true), (&keys.open, &open, true)];
    if !keys.editor.as_str().is_empty() {
        hints.push((&keys.editor, &editor, true));
    }
    if look.terminal && nothing_typed && !keys.number.as_str().is_empty() {
        hints.push((&keys.number, &pick, true));
    }
    hints.push((&keys.scope, &scope, true));
    let pad = if look.terminal { 10.0 } else { 14.0 };
    let gap = if look.terminal { 14.0 } else { 12.0 };
    let y = rect.center().y;
    if look.terminal {
        hints.push((&keys.close, &close, true));
        widgets::key_hints(ui, (rect.left() + pad, y), &hints, gap, look, palette);
    } else {
        widgets::key_hints(ui, (rect.left() + pad, y), &hints, gap, look, palette);
        let last = [(&keys.close, close.as_str(), true)];
        let width = widgets::key_hints_width(ui, &last, gap, look, palette);
        widgets::key_hints(
            ui,
            (rect.right() - pad - width, y),
            &last,
            gap,
            look,
            palette,
        );
    }
}
