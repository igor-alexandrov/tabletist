//! A SQL editor tab: its toolbar, the editor over its results with a
//! splitter between them, and the footer. The editor (`sql_text`) and the
//! results (`sql_results`) draw their own panes.

use egui::{
    CornerRadius, Frame, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2,
    vec2,
};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, SqlTab, TabId};
use crate::settings::Settings;
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::widgets::{self, ButtonSpec, MenuChoice};

/// Left and right padding of the toolbar.
const SIDE: f32 = 16.0;

/// The least height of the editor, and of the results under it.
const MIN_PANE: f32 = 80.0;

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
        let mut child = ui.new_child(egui::UiBuilder::new().id_salt((salt, id.0)).max_rect(rect));
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
    locale: Locale,
    look: &'a Look,
    palette: &'a Palette,
}

/// Run and Run all, the transaction every run is wrapped in, and the
/// limit and timeout it runs with.
fn toolbar(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(sql) = app.workspace(tab).and_then(|w| w.sql_tab(id)) else {
        return;
    };
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
                .map(|choice| MenuChoice {
                    text: limit_text(*choice, false, look, locale),
                    name: Some(limit_name(*choice, locale)),
                    selected: *choice == bar.limit,
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
                .map(|choice| MenuChoice {
                    text: timeout_text(*choice, false, look, locale),
                    name: Some(timeout_name(*choice, locale)),
                    selected: *choice == bar.secs,
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

/// Names the transaction note for screen readers and says, on hover, why
/// it is there.
fn explain_note(ui: &Ui, rect: Rect, locale: Locale) {
    let response = ui.interact(rect, ui.id().with("transaction-note"), Sense::hover());
    let name = gettext(locale, "Read-only transaction");
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, name.as_ref()));
    let _ = response.on_hover_text(gettext(
        locale,
        "Every query runs in a read-only transaction that is rolled back",
    ));
}

/// Run or Run all in `rect`, with what it does on hover.
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
/// transaction note, then the Limit and Timeout menus.
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
    // The note: 10 at its sides, an 11 pt lock, 6, the words.
    let note = gettext(locale, "Read-only transaction");
    let note_width =
        10.0 + 11.0 + 6.0 + TextRole::Secondary.width(ui.ctx(), look.faces, &note) + 10.0;
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
        ui.painter()
            .rect_filled(pill, CornerRadius::same(13), palette.surface);
        Icon::Lock.image(palette.secondary, 11.0).paint_at(
            ui,
            Rect::from_center_size(pos2(pill.left() + 15.5, center), vec2(11.0, 11.0)),
        );
        widgets::paint_text(
            ui,
            pill.left() + 27.0,
            center,
            Text::one(look, TextRole::Secondary, &note, palette.secondary),
        );
        explain_note(ui, pill, locale);
    }
    menus(ui, [limit, timeout], shape, bar, actions);
}

/// Omarchy: the tab's title and a muted `read-only transaction · limit
/// 1000 · timeout 30s`, whose limit and timeout open the menus; at the
/// right `run` and `run all` with their keys.
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
    let note = look.label(&gettext(locale, "Read-only transaction"));
    let (title_width, note_width, dot) = (
        width(TextRole::OTableTitle, bar.title),
        width(role, &note),
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
            let width =
                widgets::paint_text(ui, x, center, Text::one(look, role, &note, palette.dim));
            explain_note(
                ui,
                Rect::from_min_size(pos2(x, center - line / 2.0), vec2(width, line)),
                locale,
            );
            x += width;
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
    choices: impl FnOnce() -> Vec<MenuChoice>,
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
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(if look.terminal { 0 } else { look.radius }),
            widgets::primary_focus_ring(palette),
            StrokeKind::Outside,
        );
    }
    // Where its button reads short, the menu says what it sets on hover.
    let response = if shape.short {
        response.on_hover_text(label.value.as_str())
    } else {
        response
    };
    widgets::popup_menu(&response, rect.width(), look, choices)
}

/// Whether the last run ended and ran something. A run that failed as a
/// whole ran nothing, and leaves no last run.
fn ran(sql: &SqlTab) -> bool {
    !sql.is_running()
        && sql
            .last_run()
            .is_some_and(|run| !run.outcome.results.is_empty())
}

/// "5 rows · 14 ms" for the result that shows.
fn run_summary(sql: &SqlTab, locale: Locale) -> Option<String> {
    if !ran(sql) {
        return None;
    }
    let (_, result) = sql.shown()?;
    let tabletist_db::StatementOutcome::Rows { rows, .. } = &result.outcome else {
        return None;
    };
    let noun = if rows.len() == 1 { "row" } else { "rows" };
    Some(format!(
        "{} {} · {}",
        format::group_digits(rows.len() as u64),
        gettext(locale, noun),
        format::elapsed(result.elapsed)
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
    let note =
        ran(sql).then(|| gettext(locale, "Read-only transaction · rolled back").into_owned());
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
/// 5 rows · 14 ms · rolled back".
pub fn status_summary(app: &App, tab: ConnTabId) -> Option<String> {
    let locale = app.locale;
    let sql = app.workspace(tab)?.active_sql_tab()?;
    let (line, column) = sql.line_col();
    let mut parts = vec![format!("{} {line}:{column}", gettext(locale, "ln"))];
    parts.extend(run_summary(sql, locale));
    if ran(sql) {
        parts.push(gettext(locale, "rolled back").into_owned());
    }
    Some(parts.join(" · "))
}

#[cfg(test)]
mod tests {
    use super::*;

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
