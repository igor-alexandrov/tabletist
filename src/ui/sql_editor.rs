//! A SQL editor tab: its toolbar, the editor over its results with a
//! splitter between them, and the footer. The editor (`sql_text`) and the
//! results (`sql_results`) draw their own panes.

use egui::{
    CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
    vec2,
};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, NoWrites, RunMode, SqlTab, TabId};
use crate::settings::Settings;
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::menu;
use crate::ui::states;
use crate::ui::widgets::{self, ButtonSpec};

/// Left and right padding of the toolbar.
const SIDE: f32 = 16.0;

/// The least height of the editor, and of the results under it.
const MIN_PANE: f32 = 80.0;

/// Why Run and Run all cannot be pressed while a read-write run is in
/// flight.
const WRITING: &str = "A read-write run is still going in this tab. Cancel it or wait for it.";

/// The height of what divides the editor from the results: a band to drag
/// between two rules, or the terminal's one rule.
fn splitter_height(look: &Look) -> f32 {
    if look.terminal { 1.0 } else { 9.0 }
}

/// The least a pane gets of `room` points shared by the two: 80, or half
/// the room when it is too small for both.
fn least_pane(room: f32) -> f32 {
    MIN_PANE.min(room.max(0.0) / 2.0)
}

/// The editor's height when it takes `split` of `room` points, with the
/// results keeping their least height and the editor its own.
fn editor_height(split: f32, room: f32) -> f32 {
    let least = least_pane(room);
    (split * room).round().clamp(least, room.max(0.0) - least)
}

/// The share of `room` an editor `height` points tall takes, once both
/// panes have their least height.
fn split_at(height: f32, room: f32) -> Option<f32> {
    let least = least_pane(room);
    (room > 0.0).then(|| height.clamp(least, room - least) / room)
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    toolbar(app, ui, tab, id);
    if !look.terminal {
        footer(app, ui, tab, id);
    }
    let Some(split) = app
        .workspace(tab)
        .and_then(|workspace| workspace.sql_tab(id))
        .map(|sql| sql.split)
    else {
        return;
    };
    // What the toolbar and the footer leave: the editor takes its share,
    // the band follows it, and the results take the rest.
    let room = ui.available_rect_before_wrap();
    let band = splitter_height(&look);
    let shared = room.height() - band;
    let editor = Rect::from_min_size(room.min, vec2(room.width(), editor_height(split, shared)));
    let divider = Rect::from_min_size(editor.left_bottom(), vec2(room.width(), band));
    let results = Rect::from_min_max(divider.left_bottom(), room.max);
    let mut pane = |rect: Rect, salt: &str, add: fn(&mut App, &mut Ui, ConnTabId, TabId)| {
        // An id of its own: one counted from the parent's widgets would
        // change when the row panel appears beside the panes, and the
        // widgets in them would lose the keyboard focus with it.
        let pane_id = Id::new(("sql-pane", salt, tab.0, id.0));
        let mut child = ui.new_child(egui::UiBuilder::new().id(pane_id).max_rect(rect));
        child.set_clip_rect(rect.intersect(ui.clip_rect()));
        add(app, &mut child, tab, id);
    };
    pane(editor, "sql-text", super::sql_text::show);
    pane(results, "sql-results", super::sql_results::show);
    // After the panes, so the band takes the pointer where they meet it.
    let place = Splitter {
        band: divider,
        top: room.top(),
        shared,
    };
    if let Some(split) = splitter(ui, &place, id, locale, &look, &palette) {
        app.actions.push(Action::SetSqlSplit {
            tab,
            sql_tab: id,
            split,
        });
    }
}

/// Where the divider under the editor sits.
struct Splitter {
    band: Rect,
    /// The editor's top.
    top: f32,
    /// The height the editor and the results share.
    shared: f32,
}

/// The divider under the editor, which is the handle that moves it: the
/// band and its grip (macOS), or the terminal's rule and 3 pt to each of
/// its sides (one point is too thin to catch). Returns the share the
/// editor takes where a drag puts the band.
fn splitter(
    ui: &Ui,
    place: &Splitter,
    id: TabId,
    locale: Locale,
    look: &Look,
    palette: &Palette,
) -> Option<f32> {
    let band = place.band;
    let handle = if look.terminal {
        band.expand2(vec2(0.0, 3.0))
    } else {
        band
    };
    // A drag only: the Tab key passes the band by.
    let handle_id = ui.id().with(("sql-split", id.0));
    let response = ui.interact(handle, handle_id, Sense::DRAG);
    response.widget_info(|| {
        WidgetInfo::labeled(
            WidgetType::Other,
            true,
            gettext(locale, "Resize the editor"),
        )
    });
    let held = response.hovered() || response.dragged();
    if held {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    if look.terminal {
        let rule = if held { palette.dim } else { palette.outline };
        widgets::hline(ui, band.x_range(), band.center().y, rule);
    } else {
        ui.painter()
            .rect_filled(band, CornerRadius::ZERO, palette.surface);
        widgets::hline(ui, band.x_range(), band.top() + 0.5, palette.outline);
        widgets::hline(ui, band.x_range(), band.bottom() - 0.5, palette.outline);
        let grip = if held {
            palette.dim
        } else {
            palette.border.lerp_to_gamma(palette.faint, 0.3)
        };
        ui.painter().rect_filled(
            Rect::from_center_size(band.center(), vec2(36.0, 3.0)),
            CornerRadius::same(2),
            grip,
        );
    }
    // The band keeps under the pointer the point it was caught by.
    let pointer = response.interact_pointer_pos()?;
    if response.drag_started() {
        let caught = pointer.y - band.top();
        ui.data_mut(|data| data.insert_temp(handle_id, caught));
    }
    if !response.dragged() {
        return None;
    }
    let caught = ui
        .data(|data| data.get_temp::<f32>(handle_id))
        .unwrap_or(band.height() / 2.0);
    split_at(pointer.y - caught - place.top, place.shared)
}

/// "Limit 1,000": a limit's name, the same in every look.
fn limit_name(limit: u32, locale: Locale) -> String {
    format!(
        "{} {}",
        gettext(locale, "Limit"),
        format::group_digits(u64::from(limit))
    )
}

/// A limit as the look reads it: "Limit 1,000", or the terminal's "limit
/// 1000". Where room is `short`, the number alone.
fn limit_text(limit: u32, short: bool, look: &Look, locale: Locale) -> String {
    let number = if look.terminal {
        limit.to_string()
    } else {
        format::group_digits(u64::from(limit))
    };
    if short {
        number
    } else {
        format!("{} {number}", look.label(&gettext(locale, "Limit")))
    }
}

/// "Timeout 30 s", or "No timeout": a timeout's name, the same in every
/// look.
fn timeout_name(secs: Option<u32>, locale: Locale) -> String {
    match secs {
        None => gettext(locale, "No timeout").into_owned(),
        Some(secs) => format!("{} {secs} s", gettext(locale, "Timeout")),
    }
}

/// A timeout as the look reads it: "Timeout 30 s", or the terminal's
/// "timeout 30s". Where room is `short`, the time alone, and "Off" for
/// none.
fn timeout_text(secs: Option<u32>, short: bool, look: &Look, locale: Locale) -> String {
    let Some(secs) = secs else {
        let none = if short { "Off" } else { "No timeout" };
        return look.label(&gettext(locale, none));
    };
    let time = if look.terminal {
        format!("{secs}s")
    } else {
        format!("{secs} s")
    };
    if short {
        time
    } else {
        format!("{} {time}", look.label(&gettext(locale, "Timeout")))
    }
}

/// The keys that run the statement at the cursor and the whole script, as
/// the look spells shortcuts.
pub(super) fn run_keys(look: &Look) -> (String, String) {
    let command = look.command_key();
    if look.terminal {
        ("ctrl+enter".to_owned(), "ctrl+shift+enter".to_owned())
    } else if command == "⌘" {
        (format!("{command}↩"), format!("⇧{command}↩"))
    } else {
        (format!("{command}Enter"), format!("{command}Shift+Enter"))
    }
}

/// The keys that format the script, as the macOS and the standard look
/// spell shortcuts.
fn format_keys(look: &Look) -> String {
    let command = look.command_key();
    if command == "⌘" {
        format!("⇧{command}F")
    } else {
        format!("{command}Shift+F")
    }
}

/// How a toolbar menu's button reads: worked out once a frame.
struct MenuLabel {
    /// What the menu sets ("Limit").
    name: String,
    /// What it is set to, for screen readers ("Limit 1,000").
    value: String,
    /// What the button reads, and what it reads where room is short.
    full: String,
    short: String,
}

impl MenuLabel {
    fn text(&self, short: bool) -> &str {
        if short { &self.short } else { &self.full }
    }
}

/// What the toolbar shows of one editor, and what it draws with.
#[derive(Clone, Copy)]
struct Bar<'a> {
    tab: ConnTabId,
    id: TabId,
    /// "Query 3", as the look names things.
    title: &'a str,
    limit: u32,
    /// The timeout in seconds.
    secs: Option<u32>,
    limit_label: &'a MenuLabel,
    timeout_label: &'a MenuLabel,
    /// The editor's read-write run is in flight: Run and Run all wait.
    writing: bool,
    /// The transaction the editor's runs are in now.
    mode: RunMode,
    /// Whether the badge switches that, or why the editor cannot write.
    writes: Result<(), NoWrites>,
    locale: Locale,
    look: &'a Look,
    palette: &'a Palette,
}

/// Run and Run all, the transaction the editor's runs are in, and the
/// limit and timeout they run with.
fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(sql) = workspace.sql_tab(id) else {
        return;
    };
    let (mode, writes) = (workspace.run_mode(sql), workspace.sql_writes());
    let title = look.label(&format!("{} {}", gettext(locale, "Query"), sql.number));
    let limit = sql.limit;
    let secs = sql
        .timeout
        .map(|timeout| u32::try_from(timeout.as_secs()).unwrap_or(u32::MAX));
    let limit_label = MenuLabel {
        name: gettext(locale, "Limit").into_owned(),
        value: limit_name(limit, locale),
        full: limit_text(limit, false, &look, locale),
        short: limit_text(limit, true, &look, locale),
    };
    let timeout_label = MenuLabel {
        name: gettext(locale, "Timeout").into_owned(),
        value: timeout_name(secs, locale),
        full: timeout_text(secs, false, &look, locale),
        short: timeout_text(secs, true, &look, locale),
    };
    let bar = Bar {
        tab,
        id,
        title: &title,
        limit,
        secs,
        limit_label: &limit_label,
        timeout_label: &timeout_label,
        writing: sql.is_writing(),
        mode,
        writes,
        locale,
        look: &look,
        palette: &palette,
    };
    let mut actions = Vec::new();
    // 48 (macOS) or 40 (terminal), and a rule.
    let height = if look.terminal { 41.0 } else { 49.0 };
    egui::Panel::top(Id::new(("sql-toolbar", tab.0, id.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let rule = if look.terminal {
                palette.outline
            } else {
                palette.surface_hover
            };
            widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, rule);
            if look.terminal {
                terminal_toolbar(ui, rect, &bar, &mut actions);
            } else {
                mac_toolbar(ui, rect, &bar, &mut actions);
            }
        });
    app.actions.extend(actions);
}

/// How the two menus draw: their labels short or in full, and (macOS)
/// with or without their chevrons.
#[derive(Clone, Copy)]
struct MenuShape {
    short: bool,
    chevron: bool,
}

/// The widths of the Limit and the Timeout menu's buttons.
fn menu_widths(ui: &Ui, shape: MenuShape, bar: &Bar<'_>) -> [f32; 2] {
    [bar.limit_label, bar.timeout_label]
        .map(|label| menu_width(ui, label.text(shape.short), shape.chevron, bar.look))
}

/// The Limit menu, then the Timeout menu, in `rects`.
fn menus(
    ui: &mut Ui,
    rects: [Rect; 2],
    shape: MenuShape,
    bar: &Bar<'_>,
    actions: &mut Vec<Action>,
) {
    let Bar {
        tab,
        id,
        locale,
        look,
        palette,
        ..
    } = *bar;
    let picked = menu(
        ui,
        rects[0],
        bar.limit_label,
        shape,
        (look, palette),
        || {
            Settings::SQL_LIMITS
                .iter()
                .map(|choice| menu::Choice {
                    text: limit_text(*choice, false, look, locale),
                    name: Some(limit_name(*choice, locale)),
                    selected: *choice == bar.limit,
                    disabled: None,
                })
                .collect()
        },
    );
    if let Some(limit) = picked.and_then(|index| Settings::SQL_LIMITS.get(index))
        && *limit != bar.limit
    {
        actions.push(Action::SetSqlLimit {
            tab,
            sql_tab: id,
            limit: *limit,
        });
    }
    let picked = menu(
        ui,
        rects[1],
        bar.timeout_label,
        shape,
        (look, palette),
        || {
            Settings::SQL_TIMEOUTS
                .iter()
                .map(|choice| menu::Choice {
                    text: timeout_text(*choice, false, look, locale),
                    name: Some(timeout_name(*choice, locale)),
                    selected: *choice == bar.secs,
                    disabled: None,
                })
                .collect()
        },
    );
    if let Some(secs) = picked.and_then(|index| Settings::SQL_TIMEOUTS.get(index))
        && *secs != bar.secs
    {
        actions.push(Action::SetSqlTimeout {
            tab,
            sql_tab: id,
            secs: *secs,
        });
    }
}

/// What the badge's menu is named for screen readers: its value is the
/// mode.
const TRANSACTION: &str = "Transaction";

/// Why a tab of a writable connection to production cannot be switched to
/// Read-write: its runs are to be confirmed first, with their statements
/// on screen, and that question is not built yet.
pub(super) const UNCONFIRMED: &str =
    "Read-write runs on a production connection are not available yet.";

/// A mode as the badge names it.
fn mode_name(mode: RunMode) -> &'static str {
    match mode {
        RunMode::ReadOnly => "Read-only transaction",
        RunMode::ReadWrite => "Read-write transaction",
    }
}

/// What the badge says on hover: what the mode does, and on a connection
/// whose editors cannot write, why not.
fn badge_tip(bar: &Bar<'_>) -> String {
    let say = |text: &'static str| gettext(bar.locale, text);
    let does = match bar.mode {
        RunMode::ReadOnly => say("Runs are rolled back. Nothing is changed."),
        RunMode::ReadWrite => {
            say("A run that changes data is committed when every statement succeeds.")
        }
    };
    match bar.writes {
        Ok(()) => does.into_owned(),
        Err(NoWrites::ReadOnlyConnection) => format!(
            "{}. {}",
            say("Every query runs in a read-only transaction that is rolled back"),
            say("This connection opens read-only.")
        ),
        Err(NoWrites::Unconfirmed) => format!("{does} {}", say(UNCONFIRMED)),
    }
}

/// Whether the badge opens its menu. On a connection that opens read-only
/// it is what it always was, a note.
fn badge_switches(bar: &Bar<'_>) -> bool {
    bar.writes != Err(NoWrites::ReadOnlyConnection)
}

/// The badge's width. macOS: 10 at its sides, an 11 pt mark, 6, the words
/// and, where it is a menu, 6 and a 10 pt chevron. The terminal: the
/// words.
fn badge_width(ui: &Ui, bar: &Bar<'_>) -> f32 {
    let look = bar.look;
    let words = look.label(&gettext(bar.locale, mode_name(bar.mode)));
    let words = menu_role(look).width(ui.ctx(), look.faces, &words);
    if look.terminal {
        return words;
    }
    let chevron = if badge_switches(bar) { 6.0 + 10.0 } else { 0.0 };
    10.0 + 11.0 + 6.0 + words + chevron + 10.0
}

/// The badge in `rect`: the transaction the editor's runs are in, and on
/// a connection that takes writes the menu that switches it. In
/// Read-write it reads in the warning tone. Returns the mode picked from
/// the menu this frame, when it is another than the one in use.
fn badge(ui: &mut Ui, rect: Rect, bar: &Bar<'_>) -> Option<RunMode> {
    let Bar {
        locale,
        look,
        palette,
        mode,
        ..
    } = *bar;
    let name = gettext(locale, mode_name(mode));
    let words = look.label(&name);
    let switches = badge_switches(bar);
    let sense = if switches {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(rect, ui.id().with("transaction-note"), sense);
    response.widget_info(|| {
        if switches {
            let menu = gettext(locale, TRANSACTION);
            let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, menu.as_ref());
            info.current_text_value = Some(name.to_string());
            info
        } else {
            WidgetInfo::labeled(WidgetType::Label, true, name.as_ref())
        }
    });
    let writing = mode == RunMode::ReadWrite;
    let lit = switches && (response.hovered() || response.has_focus());
    let center = rect.center().y;
    let role = menu_role(look);
    if look.terminal {
        // Muted words that light up under the pointer, as the menus
        // beside them.
        let color = match (writing, lit) {
            (true, _) => palette.warning,
            (false, true) => palette.text,
            (false, false) => palette.dim,
        };
        widgets::paint_text(
            ui,
            rect.left(),
            center,
            Text::one(look, role, &words, color),
        );
    } else {
        let tone = states::Tone::Warning;
        let (fill, ink) = match (writing, lit) {
            (true, _) => (tone.fill(look, palette), palette.warning),
            (false, true) => (palette.surface_hover, palette.secondary),
            (false, false) => (palette.surface, palette.secondary),
        };
        let corner = CornerRadius::same(13);
        ui.painter().rect_filled(rect, corner, fill);
        if writing {
            ui.painter().rect_stroke(
                rect,
                corner,
                Stroke::new(widgets::hairline(ui), tone.line(look, palette)),
                StrokeKind::Inside,
            );
        }
        let mark = if writing { Icon::Pencil } else { Icon::Lock };
        mark.image(ink, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.left() + 15.5, center), vec2(11.0, 11.0)),
        );
        widgets::paint_text(
            ui,
            rect.left() + 27.0,
            center,
            Text::one(look, role, &words, ink),
        );
        if switches {
            Icon::ChevronDown.image(ink, 10.0).paint_at(
                ui,
                Rect::from_center_size(pos2(rect.right() - 15.0, center), vec2(10.0, 10.0)),
            );
        }
    }
    let response = response.on_hover_text(badge_tip(bar));
    if !switches {
        return None;
    }
    let radius = if look.terminal { 0 } else { 13 };
    crate::ui::focus::hint(
        ui,
        &response,
        rect,
        crate::ui::focus::Ring::Outer { radius },
    );
    let unconfirmed = bar.writes == Err(NoWrites::Unconfirmed);
    let modes = [RunMode::ReadOnly, RunMode::ReadWrite];
    let picked = menu::choices(&response, rect.width(), look, palette, || {
        modes
            .iter()
            .map(|choice| {
                let name = gettext(locale, mode_name(*choice));
                let off = unconfirmed && *choice == RunMode::ReadWrite;
                menu::Choice {
                    text: look.label(&name),
                    name: Some(name.into_owned()),
                    selected: *choice == mode,
                    disabled: off.then(|| gettext(locale, UNCONFIRMED).into_owned()),
                }
            })
            .collect()
    });
    picked
        .and_then(|index| modes.get(index).copied())
        .filter(|picked| *picked != mode)
}

/// Run or Run all in `rect`, with what it does on hover. Neither can be
/// pressed while the editor's read-write run is in flight, and says so: a
/// new run would cancel a transaction that may be about to commit.
fn run_button(
    ui: &mut Ui,
    button: ButtonSpec<'_>,
    rect: Rect,
    all: bool,
    bar: &Bar<'_>,
    actions: &mut Vec<Action>,
) {
    let explained = gettext(
        bar.locale,
        if all {
            "Run every statement"
        } else {
            "Run the statement at the cursor"
        },
    );
    if bar.writing {
        let waits = gettext(bar.locale, WRITING);
        button
            .disabled(&waits)
            .show_at(ui, rect, bar.look, bar.palette);
        return;
    }
    let response = button.show_at(ui, rect, bar.look, bar.palette);
    if response.on_hover_text(explained).clicked() {
        actions.push(Action::RunSql {
            tab: bar.tab,
            sql_tab: bar.id,
            all,
        });
    }
}

/// macOS: Run and Run all, a divider and Format; at the right the
/// transaction badge, then the Limit and Timeout menus.
fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
    let Bar {
        locale,
        look,
        palette,
        ..
    } = *bar;
    let center = rect.top() + (rect.height() - 1.0) / 2.0;
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let labels = [
        gettext(locale, "Run"),
        gettext(locale, "Run all"),
        gettext(locale, "Format"),
    ];
    let (run_keys, all_keys) = run_keys(look);
    let format_keys = format_keys(look);
    // Run: 14 at its sides, a 12 pt arrow, 8 apart. Run all: 12 and 6.
    // Format: 10, and no border.
    let buttons = |keys: bool| {
        let run = ButtonSpec::new(&labels[0])
            .icon(Icon::Play)
            .icon_size(12.0)
            .primary()
            .padding(14.0)
            .gap(8.0);
        let all = ButtonSpec::new(&labels[1]);
        let format = ButtonSpec::new(&labels[2]).quiet().padding(10.0);
        if keys {
            [
                run.shortcut(&run_keys),
                all.shortcut(&all_keys),
                format.shortcut(&format_keys),
            ]
        } else {
            [run, all, format]
        }
    };
    let note_width = badge_width(ui, bar);
    // Everything 8 apart, and 16 between the two ends (8 at the tightest).
    // What gives way as the room runs out: the buttons' keys (the help
    // says them too), then the note, then Format (its key formats too),
    // and only then the menus' words, which alone say what "1,000" and
    // "30 s" are; then the menus' chevrons, and last the menus (the run
    // buttons stay).
    let shapes = [(false, true), (true, true), (true, false)]
        .map(|(short, chevron)| MenuShape { short, chevron });
    // Each width is measured once.
    let menu_sizes = shapes.map(|shape| menu_widths(ui, shape, bar));
    let widths = [false, true].map(|keys| buttons(keys).map(|b| b.width(ui, look)));
    let room = right - left;
    // The divider before Format: a rule 20 tall with 4 at its sides.
    let divider = 4.0 + 1.0 + 4.0;
    let needs = |shape: usize, badge: bool, keys: bool, format: bool| {
        let [run, all, format_width] = widths[usize::from(keys)];
        let format = if format {
            8.0 + divider + 8.0 + format_width
        } else {
            0.0
        };
        let badge = if badge { note_width + 8.0 } else { 0.0 };
        let between = if shapes[shape].chevron { 16.0 } else { 8.0 };
        run + 8.0 + all + format + between + badge + menu_sizes[shape].iter().sum::<f32>() + 8.0
    };
    let fit = [
        (0, true, true, true),
        (0, true, false, true),
        (0, false, false, true),
        (0, false, false, false),
        (1, false, false, false),
        (2, false, false, false),
    ]
    .into_iter()
    .find(|(shape, badge, keys, format)| needs(*shape, *badge, *keys, *format) <= room);
    let keys = fit.is_some_and(|(_, _, keys, _)| keys);
    // Where each piece sits, then the pieces from left to right: the Tab
    // key and screen readers meet them in the order they are made.
    let [run, all, format] = buttons(keys);
    let [run_width, all_width, format_width] = widths[usize::from(keys)];
    let place = |x: f32, width: f32| Rect::from_min_size(pos2(x, center - 16.0), vec2(width, 32.0));
    let run_place = place(left, run_width);
    let all_place = place(run_place.right() + 8.0, all_width);
    run_button(ui, run, run_place, false, bar, actions);
    run_button(ui, all, all_place, true, bar, actions);
    let Some((shape, badge, _, formats)) = fit else {
        return;
    };
    if formats {
        let rule = all_place.right() + 8.0 + divider / 2.0;
        widgets::vline(
            ui,
            rule,
            egui::Rangef::new(center - 10.0, center + 10.0),
            palette.outline,
        );
        let format_place = place(all_place.right() + 8.0 + divider + 8.0, format_width);
        let response = format.show_at(ui, format_place, look, palette);
        if response
            .on_hover_text(gettext(locale, "Format the SQL"))
            .clicked()
        {
            actions.push(Action::FormatSql {
                tab: bar.tab,
                sql_tab: bar.id,
            });
        }
    }
    let [limit_width, timeout_width] = menu_sizes[shape];
    let shape = shapes[shape];
    let menu_rect = |right: f32, width: f32| {
        Rect::from_min_size(pos2(right - width, center - 14.0), vec2(width, 28.0))
    };
    let timeout = menu_rect(right, timeout_width);
    let limit = menu_rect(timeout.left() - 8.0, limit_width);
    if badge {
        let pill = Rect::from_min_size(
            pos2(limit.left() - 8.0 - note_width, center - 13.0),
            vec2(note_width, 26.0),
        );
        if let Some(mode) = self::badge(ui, pill, bar) {
            actions.push(Action::SetSqlMode {
                tab: bar.tab,
                sql_tab: bar.id,
                mode,
            });
        }
    }
    menus(ui, [limit, timeout], shape, bar, actions);
}

/// Omarchy: the tab's title and a muted `read-only transaction · limit
/// 1000 · timeout 30s`, each part of which opens its menu (the first only
/// on a connection that takes writes); at the right `run` and `run all`
/// with their keys.
fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
    let Bar {
        locale,
        look,
        palette,
        ..
    } = *bar;
    let center = rect.top() + (rect.height() - 1.0) / 2.0;
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let labels = [gettext(locale, "Run"), gettext(locale, "Run all")];
    let texts = [look.label(&labels[0]), look.label(&labels[1])];
    let (run_keys, all_keys) = run_keys(look);
    // 10 at their sides inside a 1 pt border, and a space before the keys.
    let buttons = |keys: bool| {
        let run = ButtonSpec::new(&texts[0])
            .label(&labels[0])
            .icon(Icon::Play)
            .icon_size(10.0)
            .primary()
            .role(TextRole::OGroup);
        let all = ButtonSpec::new(&texts[1]).label(&labels[1]).hint();
        let (run, all) = if keys {
            (run.shortcut(&run_keys), all.shortcut(&all_keys))
        } else {
            (run, all)
        };
        [run, all].map(|button| button.shortcut_role(TextRole::OBody).padding(11.0).gap(8.0))
    };
    let role = TextRole::OBody;
    let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
    let (title_width, note_width, dot) = (
        width(TextRole::OTableTitle, bar.title),
        badge_width(ui, bar),
        width(role, " · "),
    );
    // Everything 14 apart. What gives way as the room runs out: the
    // buttons' keys (the status line says them too), then the note, then
    // the title (the tab says it too), and only then the menus' words,
    // which alone say what "1000" and "30s" are; last the menus (the
    // buttons stay).
    let shapes = [false, true].map(|short| MenuShape {
        short,
        chevron: false,
    });
    // Each width is measured once.
    let menu_sizes = shapes.map(|shape| menu_widths(ui, shape, bar));
    let run_widths = [false, true].map(|keys| buttons(keys).map(|b| b.width(ui, look)));
    let room = right - left;
    let needs = |shape: usize, keys: bool, note: bool, title: bool| {
        let buttons: f32 = run_widths[usize::from(keys)].iter().sum();
        let title = if title { title_width + 14.0 } else { 0.0 };
        let note = if note { note_width + dot } else { 0.0 };
        title + note + menu_sizes[shape].iter().sum::<f32>() + dot + 14.0 + buttons + 14.0
    };
    let fit = [
        (0, true, true, true),
        (0, false, true, true),
        (0, false, false, true),
        (0, false, false, false),
        (1, false, false, false),
    ]
    .into_iter()
    .find(|(shape, keys, note, title)| needs(*shape, *keys, *note, *title) <= room);
    let keys = fit.is_some_and(|(_, keys, ..)| keys);
    // The run buttons' places, from the right; they are made after the
    // menus, so the Tab key and screen readers meet the bar from left to
    // right.
    let mut x = right;
    let mut places = [Rect::NOTHING; 2];
    for (place, width) in places.iter_mut().zip(run_widths[usize::from(keys)]).rev() {
        *place = Rect::from_min_size(pos2(x - width, center - 13.0), vec2(width, 26.0));
        x -= width + 14.0;
    }
    if let Some((shape, _, noted, title)) = fit {
        let dot_text = || Text::one(look, role, " · ", palette.dim);
        let mut x = left;
        if title {
            x += widgets::paint_label(
                ui,
                x,
                center,
                Text::one(look, TextRole::OTableTitle, bar.title, palette.text),
            ) + 14.0;
        }
        let line = role.row_height(ui.ctx(), look.faces);
        if noted {
            let words = Rect::from_min_size(pos2(x, center - line / 2.0), vec2(note_width, line));
            if let Some(mode) = badge(ui, words, bar) {
                actions.push(Action::SetSqlMode {
                    tab: bar.tab,
                    sql_tab: bar.id,
                    mode,
                });
            }
            x += note_width;
            x += widgets::paint_text(ui, x, center, dot_text());
        }
        // The menus read as one line with the note: their words, a dot
        // apart.
        let [limit_width, timeout_width] = menu_sizes[shape];
        let shape = shapes[shape];
        let menu_rect = |left: f32, width: f32| {
            Rect::from_min_size(
                pos2(left, center - line / 2.0 - 2.0),
                vec2(width, line + 4.0),
            )
        };
        let limit = menu_rect(x, limit_width);
        let after = limit.right() + widgets::paint_text(ui, limit.right(), center, dot_text());
        let timeout = menu_rect(after, timeout_width);
        menus(ui, [limit, timeout], shape, bar, actions);
    }
    for ((button, place), all) in buttons(keys).into_iter().zip(places).zip([false, true]) {
        run_button(ui, button, place, all, bar, actions);
    }
}

/// The role a menu's button reads in.
fn menu_role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::Secondary, TextRole::OBody)
}

/// The width of a menu's button reading `text`. macOS: a 1 pt border, 10
/// of padding, the words and, with its chevron, 6 and the 10 pt chevron.
/// The terminal: the words.
fn menu_width(ui: &Ui, text: &str, chevron: bool, look: &Look) -> f32 {
    let text = menu_role(look).width(ui.ctx(), look.faces, text);
    if look.terminal {
        text
    } else {
        let chevron = if chevron { 6.0 + 10.0 } else { 0.0 };
        (1.0 + 10.0 + text + chevron + 10.0 + 1.0).ceil()
    }
}

/// A menu's button in `rect`, and its choices while it is open (`choices`
/// is asked for them only then). Returns the index of the choice picked
/// this frame.
fn menu(
    ui: &mut Ui,
    rect: Rect,
    label: &MenuLabel,
    shape: MenuShape,
    (look, palette): (&Look, &Palette),
    choices: impl FnOnce() -> Vec<menu::Choice>,
) -> Option<usize> {
    let response = ui.interact(rect, ui.id().with(("menu", &label.name)), Sense::click());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, &label.name);
        info.current_text_value = Some(label.value.clone());
        info
    });
    let center = rect.center().y;
    let role = menu_role(look);
    let text = label.text(shape.short);
    if look.terminal {
        // Muted words that light up under the pointer, as the bar's other
        // notes that answer a click.
        let color = if response.hovered() || response.has_focus() {
            palette.text
        } else {
            palette.dim
        };
        widgets::paint_text(ui, rect.left(), center, Text::one(look, role, text, color));
    } else {
        let corner = CornerRadius::same(look.radius);
        let fill = if response.hovered() {
            palette.panel
        } else {
            palette.window
        };
        ui.painter().rect_filled(rect, corner, fill);
        ui.painter().rect_stroke(
            rect,
            corner,
            Stroke::new(widgets::hairline(ui), palette.border),
            StrokeKind::Inside,
        );
        widgets::paint_text(
            ui,
            rect.left() + 11.0,
            center,
            Text::one(look, role, text, palette.text),
        );
        if shape.chevron {
            Icon::ChevronDown.image(palette.dim, 10.0).paint_at(
                ui,
                Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(10.0, 10.0)),
            );
        }
    }
    let radius = if look.terminal { 0 } else { look.radius };
    crate::ui::focus::hint(
        ui,
        &response,
        rect,
        crate::ui::focus::Ring::Outer { radius },
    );
    // Where its button reads short, the menu says what it sets on hover.
    let response = if shape.short {
        response.on_hover_text(label.value.as_str())
    } else {
        response
    };
    menu::choices(&response, rect.width(), look, palette, choices)
}

/// Whether the last run ended and ran something. A run that failed as a
/// whole ran nothing, and leaves no last run.
fn ran(sql: &SqlTab) -> bool {
    !sql.is_running()
        && sql
            .last_run()
            .is_some_and(|run| !run.outcome.results.is_empty())
}

/// "5 rows · 14 ms" for the result that shows. With none to show, in a run
/// sent to write, what its last statement changed: "12 rows affected ·
/// 14 ms".
fn run_summary(sql: &SqlTab, locale: Locale) -> Option<String> {
    use tabletist_db::StatementOutcome;
    if !ran(sql) {
        return None;
    }
    let counted = |count: u64, one: &'static str, many: &'static str, took| {
        format!(
            "{} {} · {}",
            format::group_digits(count),
            gettext(locale, if count == 1 { one } else { many }),
            format::elapsed(took)
        )
    };
    if let Some((_, result)) = sql.shown() {
        let StatementOutcome::Rows { rows, .. } = &result.outcome else {
            return None;
        };
        return Some(counted(rows.len() as u64, "row", "rows", result.elapsed));
    }
    let run = sql.last_run()?;
    let last = run.outcome.results.last()?;
    match &last.outcome {
        StatementOutcome::Done {
            affected: Some(count),
            ..
        } if run.mode == tabletist_db::ScriptMode::Write => Some(counted(
            *count,
            "row affected",
            "rows affected",
            last.elapsed,
        )),
        _ => None,
    }
}

/// What became of the last run's transaction: "rolled back" for every
/// read-only run, and for a run sent to write how it ended. Where the
/// database could not undo everything, that is what is said, whatever the
/// end; and a commit that failed after the database had committed part by
/// itself reads as what it left, partly committed.
fn end_words(run: &crate::model::SqlRun) -> &'static str {
    use tabletist_db::ScriptEnd;
    if run.mode == tabletist_db::ScriptMode::ReadOnly {
        return "rolled back";
    }
    if run.outcome.rollback_warning.is_some() {
        return "not fully rolled back";
    }
    match &run.outcome.end {
        ScriptEnd::Committed => "committed",
        ScriptEnd::RolledBack | ScriptEnd::Partly { committed: 0 } => "rolled back",
        ScriptEnd::CommitFailed { committed: 0, .. } => "commit failed",
        ScriptEnd::Partly { .. } | ScriptEnd::CommitFailed { .. } => "partly committed",
    }
}

/// "Read-only transaction · rolled back", "Read-write transaction ·
/// committed": the footer's note of the last run.
fn end_note(sql: &SqlTab, locale: Locale) -> Option<String> {
    let run = sql.last_run().filter(|_| ran(sql))?;
    let transaction = match run.mode {
        tabletist_db::ScriptMode::ReadOnly => "Read-only transaction",
        tabletist_db::ScriptMode::Write => "Read-write transaction",
    };
    Some(format!(
        "{} · {}",
        gettext(locale, transaction),
        gettext(locale, end_words(run))
    ))
}

/// The macOS and standard footer: the result's rows and time and what
/// became of the transaction; at the right the cursor and the server.
fn footer(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(sql) = workspace.sql_tab(id) else {
        return;
    };
    let (line, column) = sql.line_col();
    let cursor = format!(
        "{} {line}, {} {column}",
        gettext(locale, "Ln"),
        gettext(locale, "Col")
    );
    let summary = run_summary(sql, locale);
    let note = end_note(sql, locale);
    let version = workspace.server_version.value.clone();
    egui::Panel::bottom(Id::new(("sql-footer", tab.0, id.0)))
        .exact_size(33.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let center = rect.top() + 1.0 + (rect.height() - 1.0) / 2.0;
            // The status bar's grey, as a table's footer.
            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
            let role = widgets::secondary(&look);
            let width = |text: &str| role.width(ui.ctx(), look.faces, text);
            // 20 in from the ends, everything 16 apart. What gives way as
            // the room runs out: the transaction, the server, the result.
            let (left, right, gap) = (rect.left() + 20.0, rect.right() - 20.0, 16.0);
            let pieces = |keep: usize| {
                let start: Vec<&str> = [(&summary, 1), (&note, 3)]
                    .into_iter()
                    .filter(|(_, level)| keep >= *level)
                    .filter_map(|(text, _)| text.as_deref())
                    .collect();
                (start, version.as_deref().filter(|_| keep >= 2))
            };
            let needs = |keep: usize| {
                let (start, end) = pieces(keep);
                start
                    .into_iter()
                    .chain(end)
                    .map(|text| width(text) + gap)
                    .sum::<f32>()
                    + width(&cursor)
            };
            let keep = (0..=3)
                .rev()
                .find(|keep| needs(*keep) <= right - left)
                .unwrap_or(0);
            let (start, end) = pieces(keep);
            let text = |text: &str| Text::one(&look, role, text, status);
            // Left to right, as it reads: the right end's places first.
            let mut x = right;
            let mut places = Vec::new();
            for piece in end.into_iter().chain([cursor.as_str()]) {
                x -= width(piece);
                places.push((piece, x));
                x -= gap;
            }
            let mut x = left;
            for piece in start {
                x += widgets::paint_label(ui, x, center, text(piece)) + gap;
            }
            for (piece, x) in places.into_iter().rev() {
                widgets::paint_label(ui, x, center, text(piece));
            }
        });
}

/// The right end of the terminal's status line on an editor: "ln 12:21 ·
/// 5 rows · 14 ms · rolled back", or after a run sent to write "ln 3:1 ·
/// 12 rows affected · 14 ms · committed".
pub fn status_summary(app: &App, tab: ConnTabId) -> Option<String> {
    let locale = app.locale;
    let sql = app.workspace(tab)?.active_sql_tab()?;
    let (line, column) = sql.line_col();
    let mut parts = vec![format!("{} {line}:{column}", gettext(locale, "ln"))];
    parts.extend(run_summary(sql, locale));
    if let Some(run) = sql.last_run().filter(|_| ran(sql)) {
        parts.push(gettext(locale, end_words(run)).into_owned());
    }
    Some(parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use egui::{Key, Modifiers};

    use super::*;
    use crate::model::RunMode;
    use crate::testing::{Harness, bounds, done_outcome, node, write_outcome};

    /// A SQL editor on a connection that takes writes, drawn in `look`,
    /// with `text` typed into it.
    fn editor(look: Look, text: &str) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake_as(false);
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Paste(text.into())]);
        harness.settle();
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (harness, tab, id)
    }

    fn set_mode(harness: &mut Harness, tab: ConnTabId, id: TabId, mode: RunMode) {
        harness.app.apply(Action::SetSqlMode {
            tab,
            sql_tab: id,
            mode,
        });
    }

    /// Whether the button `name` cannot be pressed, and what it says of
    /// that.
    fn button(harness: &mut Harness, name: &str) -> (bool, Option<String>) {
        let tree = harness.settle();
        let id = node(&tree, name, Role::Button).unwrap_or_else(|| panic!("no {name}"));
        let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
        (node.is_disabled(), node.description().map(str::to_owned))
    }

    /// A SQL editor on a connection that opens read-only, drawn in `look`.
    fn read_only_editor(look: Look) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (harness, tab, id)
    }

    fn mode_of(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.run_mode(workspace.sql_tab(id).unwrap())
    }

    /// What the tab itself is set to, whatever its session lets it run as:
    /// where no editor can write, that is what a switch must not change.
    fn own_mode(harness: &Harness, tab: ConnTabId, id: TabId) -> RunMode {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.sql_tab(id).unwrap().mode
    }

    /// What the badge's menu is set to: none where the badge is no menu.
    fn badge_value(harness: &mut Harness) -> Option<String> {
        let tree = harness.settle();
        let id = node(&tree, TRANSACTION, Role::ComboBox)?;
        let (_, badge) = tree.nodes.iter().find(|(node, _)| *node == id)?;
        badge.value().map(str::to_owned)
    }

    /// The colour the badge's words are painted in, as `look` writes them.
    fn badge_color(harness: &Harness, look: &Look, mode: RunMode) -> Option<egui::Color32> {
        harness.painted_color(&look.label(mode_name(mode)))
    }

    #[test]
    fn the_badge_is_a_menu_that_switches_the_tabs_mode() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editor(look, "SELECT 1");
            assert_eq!(
                badge_value(&mut harness).as_deref(),
                Some("Read-only transaction"),
                "{}",
                look.name
            );
            let quiet = badge_color(&harness, &look, RunMode::ReadOnly);
            harness.click(TRANSACTION);
            harness.click("Read-write transaction");
            assert_eq!(
                mode_of(&harness, tab, id),
                RunMode::ReadWrite,
                "{}",
                look.name
            );
            assert_eq!(
                badge_value(&mut harness).as_deref(),
                Some("Read-write transaction"),
                "{}",
                look.name
            );
            // The menu closed on the pick, and the badge reads in the
            // warning colour, which the read-only one did not.
            assert!(!harness.has("Read-only transaction"), "{}", look.name);
            let warning = Some(harness.app.palette.warning);
            assert_eq!(badge_color(&harness, &look, RunMode::ReadWrite), warning);
            assert_ne!(quiet, warning, "{}", look.name);
            // Picking the mode in use changes nothing; the other one
            // switches back.
            harness.click(TRANSACTION);
            harness.click("Read-write transaction");
            assert_eq!(mode_of(&harness, tab, id), RunMode::ReadWrite);
            harness.click(TRANSACTION);
            harness.click("Read-only transaction");
            assert_eq!(
                mode_of(&harness, tab, id),
                RunMode::ReadOnly,
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn mod_shift_m_switches_the_mode_of_the_editor_on_screen() {
        let chord = Modifiers::COMMAND | Modifiers::SHIFT;
        for look in Look::ALL {
            let (mut harness, tab, id) = editor(look, "SELECT 1");
            // With the keyboard in the editor, where it is while typing.
            harness.press(Key::M, chord);
            assert_eq!(
                mode_of(&harness, tab, id),
                RunMode::ReadWrite,
                "{}",
                look.name
            );
            // The editor did not take the key for a letter.
            let text = &harness
                .app
                .workspace(tab)
                .unwrap()
                .sql_tab(id)
                .unwrap()
                .text;
            assert_eq!(text, "SELECT 1", "{}", look.name);
            harness.press(Key::M, chord);
            assert_eq!(
                mode_of(&harness, tab, id),
                RunMode::ReadOnly,
                "{}",
                look.name
            );
        }
        // The help lists it.
        let listed = crate::ui::keys::SHORTCUTS.iter().any(|(keys, what, _)| {
            *keys == "Mod+Shift+M" && *what == "Read-only or read-write runs in the SQL editor"
        });
        assert!(listed);
    }

    #[test]
    fn on_a_read_only_connection_the_badge_is_a_note_and_the_key_does_nothing() {
        for look in Look::ALL {
            let (mut harness, tab, id) = read_only_editor(look);
            // As it always was: a label, and no menu.
            assert_eq!(badge_value(&mut harness), None, "{}", look.name);
            let tree = harness.settle();
            assert!(
                node(&tree, "Read-only transaction", Role::Label).is_some(),
                "{}",
                look.name
            );
            harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
            assert_eq!(
                own_mode(&harness, tab, id),
                RunMode::ReadOnly,
                "{}",
                look.name
            );
            // On hover it says why.
            let at = bounds(&tree, "Read-only transaction", Role::Label).unwrap();
            let shown = crate::ui::tests::hover(&mut harness, at.center());
            let tip = "Every query runs in a read-only transaction that is rolled back. This \
                       connection opens read-only.";
            assert!(shown.iter().any(|label| label == tip), "{}", look.name);
        }
    }

    #[test]
    fn the_badge_says_on_hover_what_its_mode_does() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editor(look, "SELECT 1");
            for (mode, tip) in [
                (
                    RunMode::ReadOnly,
                    "Runs are rolled back. Nothing is changed.",
                ),
                (
                    RunMode::ReadWrite,
                    "A run that changes data is committed when every statement succeeds.",
                ),
            ] {
                set_mode(&mut harness, tab, id, mode);
                let tree = harness.settle();
                let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
                let shown = crate::ui::tests::hover(&mut harness, at.center());
                assert!(
                    shown.iter().any(|label| label == tip),
                    "{tip} in {}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn on_production_the_read_write_choice_is_shown_and_cannot_be_picked() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editor(look, "SELECT 1");
            harness.app.workspace_mut(tab).unwrap().environment =
                crate::env::Environment::Production;
            harness.click(TRANSACTION);
            let tree = harness.settle();
            let choice = node(&tree, "Read-write transaction", Role::Button).expect("the choice");
            let (_, choice) = tree.nodes.iter().find(|(node, _)| *node == choice).unwrap();
            assert!(choice.is_disabled(), "{}", look.name);
            harness.click("Read-write transaction");
            assert_eq!(
                own_mode(&harness, tab, id),
                RunMode::ReadOnly,
                "{}",
                look.name
            );
            harness.press(Key::Escape, Modifiers::NONE);
            harness.press(Key::M, Modifiers::COMMAND | Modifiers::SHIFT);
            assert_eq!(
                own_mode(&harness, tab, id),
                RunMode::ReadOnly,
                "{}",
                look.name
            );
            // The badge says why on hover.
            let tree = harness.settle();
            let at = bounds(&tree, TRANSACTION, Role::ComboBox).unwrap();
            let shown = crate::ui::tests::hover(&mut harness, at.center());
            let tip = format!("Runs are rolled back. Nothing is changed. {UNCONFIRMED}");
            assert!(shown.contains(&tip), "{}", look.name);
        }
    }

    #[test]
    fn run_and_run_all_wait_for_a_read_write_run_in_flight() {
        for look in Look::ALL {
            let (mut harness, tab, id) = editor(look, "DELETE FROM users WHERE id = 1");
            set_mode(&mut harness, tab, id, RunMode::ReadWrite);
            for name in ["Run", "Run all"] {
                assert!(!button(&mut harness, name).0, "{name} in {}", look.name);
            }
            harness.press(Key::Enter, Modifiers::COMMAND);
            let sent = harness.app.backend.sent.len();
            for name in ["Run", "Run all"] {
                assert_eq!(
                    button(&mut harness, name),
                    (true, Some(WRITING.to_owned())),
                    "{name} in {}",
                    look.name
                );
                // Nor does a press of it reach the run.
                harness.click(name);
            }
            harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
            assert_eq!(harness.app.backend.sent.len(), sent, "{}", look.name);
            // The way to stop it stays.
            assert!(harness.has("Cancel query"), "{}", look.name);
            let done = vec![done_outcome(Some(1))];
            let committed = write_outcome(done, tabletist_db::ScriptEnd::Committed);
            harness.answer_sql(Ok(committed), None);
            for name in ["Run", "Run all"] {
                assert!(!button(&mut harness, name).0, "{name} in {}", look.name);
            }
        }
    }

    #[test]
    fn the_footer_says_how_a_read_write_run_ended() {
        use tabletist_db::{Error, ScriptEnd};
        let failed = |committed| ScriptEnd::CommitFailed {
            error: Error::query("a deferred constraint"),
            committed,
        };
        let warned = Some("could not be rolled back".to_owned());
        for (end, warning, words) in [
            (ScriptEnd::Committed, None, "committed"),
            (ScriptEnd::RolledBack, None, "rolled back"),
            (ScriptEnd::Partly { committed: 1 }, None, "partly committed"),
            (failed(0), None, "commit failed"),
            // Part of it is written all the same: said as what it left.
            (failed(1), None, "partly committed"),
            (
                ScriptEnd::RolledBack,
                warned.clone(),
                "not fully rolled back",
            ),
            (
                ScriptEnd::Partly { committed: 1 },
                warned,
                "not fully rolled back",
            ),
        ] {
            for look in Look::ALL {
                let script = "CREATE TABLE notes (n);\nUPDATE notes SET n = 1";
                let (mut harness, tab, id) = editor(look, script);
                set_mode(&mut harness, tab, id, RunMode::ReadWrite);
                harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
                // Nothing is said of a run that is still going.
                assert!(!harness.has("Read-write transaction · committed"));
                let mut outcome = write_outcome(
                    vec![done_outcome(None), done_outcome(Some(12))],
                    end.clone(),
                );
                outcome.rollback_warning = warning.clone();
                harness.answer_sql(Ok(outcome), None);
                let context = format!("{words} in {}", look.name);
                if look.terminal {
                    // The terminal's status line, which has no footer.
                    let status = status_summary(&harness.app, tab).expect("an editor shows");
                    let (_, said) = status.split_once(" · ").expect("after the cursor");
                    assert_eq!(
                        said,
                        format!("12 rows affected · 14 ms · {words}"),
                        "{context}"
                    );
                } else {
                    assert!(harness.has("12 rows affected · 14 ms"), "{context}");
                    let note = format!("Read-write transaction · {words}");
                    assert!(harness.has(&note), "{note} in {}", look.name);
                    assert!(!harness.has("Read-only transaction · rolled back"));
                }
            }
        }
    }

    #[test]
    fn a_run_of_reads_in_a_read_write_tab_is_said_to_be_read_only() {
        let (mut harness, tab, id) = editor(Look::standard(), "SELECT 1");
        set_mode(&mut harness, tab, id, RunMode::ReadWrite);
        harness.press(Key::Enter, Modifiers::COMMAND);
        let rows = crate::testing::script_outcome(vec![crate::testing::rows_outcome(3)]);
        harness.answer_sql(Ok(rows), None);
        assert!(harness.has("3 rows · 14 ms"));
        assert!(harness.has("Read-only transaction · rolled back"));
        // A statement that counted rows in a read-only run is no change
        // that stayed: the footer does not count it.
        let (mut harness, _, _) = editor(Look::standard(), "UPDATE notes SET n = 1");
        harness.press(Key::Enter, Modifiers::COMMAND);
        let counted = crate::testing::script_outcome(vec![done_outcome(Some(12))]);
        harness.answer_sql(Ok(counted), None);
        assert!(!harness.has("12 rows affected · 14 ms"));
        assert!(harness.has("Read-only transaction · rolled back"));
    }

    #[test]
    fn run_stays_while_a_read_only_run_is_in_flight() {
        let (mut harness, _, _) = editor(Look::standard(), "SELECT 1");
        harness.press(Key::Enter, Modifiers::COMMAND);
        for name in ["Run", "Run all"] {
            assert!(!button(&mut harness, name).0, "{name}");
        }
    }

    #[test]
    fn the_editor_takes_its_share_of_the_room() {
        assert_eq!(editor_height(0.45, 600.0), 270.0);
        assert_eq!(editor_height(0.5, 401.0), 201.0);
        // The same share of more room is more of it.
        assert_eq!(editor_height(0.45, 800.0), 360.0);
    }

    #[test]
    fn both_panes_keep_their_least_height() {
        assert_eq!(editor_height(0.0, 600.0), MIN_PANE);
        assert_eq!(editor_height(1.0, 600.0), 600.0 - MIN_PANE);
        assert_eq!(editor_height(0.05, 600.0), MIN_PANE);
        // Room for no more than the two least heights: they take them.
        assert_eq!(editor_height(0.9, 160.0), MIN_PANE);
        // Less than that: the panes share what there is.
        assert_eq!(editor_height(0.9, 100.0), 50.0);
        assert_eq!(editor_height(0.1, 100.0), 50.0);
        assert_eq!(editor_height(0.45, 0.0), 0.0);
        assert_eq!(editor_height(0.45, -9.0), 0.0);
    }

    #[test]
    fn a_dragged_band_gives_a_share_that_keeps_the_least_heights() {
        assert_eq!(split_at(300.0, 600.0), Some(0.5));
        assert_eq!(split_at(-50.0, 600.0), Some(MIN_PANE / 600.0));
        assert_eq!(split_at(5_000.0, 600.0), Some((600.0 - MIN_PANE) / 600.0));
        assert_eq!(split_at(10.0, 0.0), None);
        // The share a drag gives puts the band where the drag did.
        for height in [80.0, 137.0, 300.0, 520.0] {
            let split = split_at(height, 600.0).unwrap();
            assert_eq!(editor_height(split, 600.0), height);
        }
    }
}
