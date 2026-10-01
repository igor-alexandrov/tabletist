//! A SQL editor tab: its toolbar, the editor over its results with a
//! splitter between them, and the footer. The editor (`sql_text`) and the
//! results (`sql_results`) draw their own panes.

use egui::{
    Color32, CornerRadius, Frame, Id, Margin, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, SqlTab, TabId};
use crate::settings::Settings;
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::widgets::{self, ButtonSpec};

/// Left and right padding of the toolbar.
const SIDE: f32 = 16.0;

/// The least height of the editor, and of the results under it.
const MIN_PANE: f32 = 80.0;

/// The height of what divides the editor from the results: a band to drag
/// between two rules, or the terminal's one rule.
fn splitter_height(look: &Look) -> f32 {
    if look.terminal { 1.0 } else { 9.0 }
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette) = (app.look, app.palette);
    toolbar(app, ui, tab, id);
    if !look.terminal {
        footer(app, ui, tab, id);
    }
    // The editor takes 45% of the room until its edge is dragged; egui
    // keeps the height it was dragged to, for each tab.
    let available = ui.available_height();
    let band = splitter_height(&look);
    egui::Panel::top(Id::new(("sql-editor", tab.0, id.0)))
        .resizable(true)
        .show_separator_line(false)
        .default_size((available * 0.45).round())
        .size_range(MIN_PANE..=(available - MIN_PANE).max(MIN_PANE))
        .frame(Frame::new().fill(palette.window).inner_margin(Margin {
            bottom: band as i8,
            ..Default::default()
        }))
        .show(ui, |ui| {
            // A panel is as tall as its contents; fill it, so it is as
            // tall as it was dragged.
            ui.take_available_space();
            let edge = ui.max_rect();
            splitter(
                ui,
                Rect::from_min_size(edge.left_bottom(), vec2(edge.width(), band)),
                &look,
                &palette,
            );
            super::sql_text::show(app, ui, tab, id);
        });
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| super::sql_results::show(app, ui, tab, id));
}

/// The divider under the editor. The panel's own edge, at its bottom,
/// takes the drag.
fn splitter(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
    if look.terminal {
        widgets::hline(ui, rect.x_range(), rect.center().y, palette.outline);
        return;
    }
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.surface);
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
    let grip = Rect::from_center_size(rect.center(), vec2(36.0, 3.0));
    ui.painter().rect_filled(
        grip,
        CornerRadius::same(2),
        palette.border.lerp_to_gamma(palette.faint, 0.3),
    );
}

/// "Limit 1,000", or the terminal's "limit 1000".
fn limit_text(limit: u32, look: &Look, locale: Locale) -> String {
    let label = look.label(&gettext(locale, "Limit"));
    if look.terminal {
        format!("{label} {limit}")
    } else {
        format!("{label} {}", format::group_digits(u64::from(limit)))
    }
}

/// "Timeout 30 s", or the terminal's "timeout 30s"; "No timeout" for none.
fn timeout_text(secs: Option<u32>, look: &Look, locale: Locale) -> String {
    let label = look.label(&gettext(locale, "Timeout"));
    match secs {
        None => look.label(&gettext(locale, "No timeout")),
        Some(secs) if look.terminal => format!("{label} {secs}s"),
        Some(secs) => format!("{label} {secs} s"),
    }
}

/// The keys that run the statement at the cursor and the whole script, as
/// the look spells shortcuts.
fn run_keys(look: &Look) -> (String, String) {
    let command = look.command_key();
    if look.terminal {
        ("ctrl+enter".to_owned(), "ctrl+shift+enter".to_owned())
    } else if command == "⌘" {
        (format!("{command}↩"), format!("⇧{command}↩"))
    } else {
        (format!("{command}Enter"), format!("{command}Shift+Enter"))
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
    let bar = Bar {
        tab,
        id,
        title: &title,
        limit: sql.limit,
        secs: sql
            .timeout
            .map(|timeout| u32::try_from(timeout.as_secs()).unwrap_or(u32::MAX)),
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

/// The Limit and Timeout menus, `gap` apart and ending at `right`, on the
/// line `center` and `height` tall. Returns the left edge of the first.
fn menus(
    ui: &mut Ui,
    (right, center, height): (f32, f32, f32),
    gap: f32,
    bar: &Bar<'_>,
    actions: &mut Vec<Action>,
) -> f32 {
    let Bar {
        tab,
        id,
        locale,
        look,
        palette,
        ..
    } = *bar;
    let limit = limit_text(bar.limit, look, locale);
    let timeout = timeout_text(bar.secs, look, locale);
    let limits = Settings::SQL_LIMITS.map(|choice| (choice, limit_text(choice, look, locale)));
    let timeouts =
        Settings::SQL_TIMEOUTS.map(|choice| (choice, timeout_text(choice, look, locale)));
    let place = |right: f32, width: f32| {
        Rect::from_min_size(
            pos2(right - width, center - height / 2.0),
            vec2(width, height),
        )
    };
    let rect = place(right, menu_width(ui, &timeout, look));
    let picked = menu(
        ui,
        rect,
        &Menu {
            name: &gettext(locale, "Timeout"),
            current: &timeout,
            choices: &timeouts,
            selected: bar.secs,
        },
        look,
        palette,
    );
    if let Some(secs) = picked {
        actions.push(Action::SetSqlTimeout {
            tab,
            sql_tab: id,
            secs,
        });
    }
    let rect = place(rect.left() - gap, menu_width(ui, &limit, look));
    let picked = menu(
        ui,
        rect,
        &Menu {
            name: &gettext(locale, "Limit"),
            current: &limit,
            choices: &limits,
            selected: bar.limit,
        },
        look,
        palette,
    );
    if let Some(limit) = picked {
        actions.push(Action::SetSqlLimit {
            tab,
            sql_tab: id,
            limit,
        });
    }
    rect.left()
}

/// The width of the two menus `gap` apart.
fn menus_width(ui: &Ui, gap: f32, bar: &Bar<'_>) -> f32 {
    menu_width(ui, &limit_text(bar.limit, bar.look, bar.locale), bar.look)
        + gap
        + menu_width(ui, &timeout_text(bar.secs, bar.look, bar.locale), bar.look)
}

/// Why the transaction note is there, on hover; and the note's name for
/// screen readers.
fn explain_note(ui: &Ui, rect: Rect, text: &str, locale: Locale) {
    let response = ui.interact(rect, ui.id().with("transaction-note"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    let _ = response.on_hover_text(gettext(
        locale,
        "Every query runs in a read-only transaction that is rolled back",
    ));
}

/// What each run button does, on hover.
fn explain_run(response: egui::Response, all: bool, locale: Locale) -> egui::Response {
    response.on_hover_text(gettext(
        locale,
        if all {
            "Run every statement"
        } else {
            "Run the statement at the cursor"
        },
    ))
}

/// macOS: Run and Run all; at the right the transaction note, then the
/// Limit and Timeout menus.
fn mac_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
    let Bar {
        tab,
        id,
        locale,
        look,
        palette,
        ..
    } = *bar;
    let center = rect.top() + (rect.height() - 1.0) / 2.0;
    let (left, right) = (rect.left() + SIDE, rect.right() - SIDE);
    let labels = [gettext(locale, "Run"), gettext(locale, "Run all")];
    let (run_keys, all_keys) = run_keys(look);
    // Run: 14 at its sides, a 12 pt arrow, 8 apart. Run all: 12 and 6.
    let buttons = |keys: bool| {
        let run = ButtonSpec::new(&labels[0])
            .icon(Icon::Play)
            .icon_size(12.0)
            .primary()
            .padding(14.0)
            .gap(8.0);
        let all = ButtonSpec::new(&labels[1]);
        if keys {
            [run.shortcut(&run_keys), all.shortcut(&all_keys)]
        } else {
            [run, all]
        }
    };
    // The note: 10 at its sides, an 11 pt lock, 6, the words.
    let note = gettext(locale, "Read-only transaction");
    let note_width =
        10.0 + 11.0 + 6.0 + TextRole::Secondary.width(ui.ctx(), look.faces, &note) + 10.0;
    // Everything 8 apart, and at least 16 between the two ends. What gives
    // way as the room runs out: the note, then the buttons' keys, then the
    // menus (the buttons stay).
    let room = right - left;
    let needs = |badge: bool, keys: bool| {
        let buttons: f32 = buttons(keys).iter().map(|b| b.width(ui, look)).sum();
        let badge = if badge { note_width + 8.0 } else { 0.0 };
        buttons + 8.0 + 16.0 + badge + menus_width(ui, 8.0, bar)
    };
    let (badge, keys, with_menus) = [(true, true), (false, true), (false, false)]
        .into_iter()
        .find(|(badge, keys)| needs(*badge, *keys) <= room)
        .map_or((false, false, false), |(badge, keys)| (badge, keys, true));
    let mut x = left;
    for (button, all) in buttons(keys).into_iter().zip([false, true]) {
        let width = button.width(ui, look);
        let place = Rect::from_min_size(pos2(x, center - 16.0), vec2(width, 32.0));
        let response = button.show_at(ui, place, look, palette);
        if explain_run(response, all, locale).clicked() {
            actions.push(Action::RunSql {
                tab,
                sql_tab: id,
                all,
            });
        }
        x += width + 8.0;
    }
    if !with_menus {
        return;
    }
    let menus_left = menus(ui, (right, center, 28.0), 8.0, bar, actions);
    if badge {
        let pill = Rect::from_min_size(
            pos2(menus_left - 8.0 - note_width, center - 13.0),
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
        explain_note(ui, pill, &note, locale);
    }
}

/// Omarchy: the tab's title and a muted `read-only transaction · limit
/// 1000 · timeout 30s`, whose limit and timeout open the menus; at the
/// right `run` and `run all` with their keys.
fn terminal_toolbar(ui: &mut Ui, rect: Rect, bar: &Bar<'_>, actions: &mut Vec<Action>) {
    let Bar {
        tab,
        id,
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
    // Everything 14 apart. What gives way as the room runs out: the note,
    // then the buttons' keys, then the title (the tab says it too), then
    // the menus (the buttons stay).
    let room = right - left;
    let needs = |title: bool, note: bool, keys: bool| {
        let buttons: f32 = buttons(keys).iter().map(|b| b.width(ui, look)).sum();
        let title = if title { title_width + 14.0 } else { 0.0 };
        let note = if note { note_width + dot } else { 0.0 };
        title + note + menus_width(ui, dot, bar) + 14.0 + buttons + 14.0
    };
    let (title, noted, keys, with_menus) = [
        (true, true, true),
        (true, false, true),
        (true, false, false),
        (false, false, false),
    ]
    .into_iter()
    .find(|(title, note, keys)| needs(*title, *note, *keys) <= room)
    .map_or((false, false, false, false), |(title, note, keys)| {
        (title, note, keys, true)
    });
    let mut x = right;
    for (button, all) in buttons(keys).into_iter().zip([false, true]).rev() {
        let width = button.width(ui, look);
        let place = Rect::from_min_size(pos2(x - width, center - 13.0), vec2(width, 26.0));
        let response = button.show_at(ui, place, look, palette);
        if explain_run(response, all, locale).clicked() {
            actions.push(Action::RunSql {
                tab,
                sql_tab: id,
                all,
            });
        }
        x -= width + 14.0;
    }
    if !with_menus {
        return;
    }
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
        let width = widgets::paint_text(ui, x, center, Text::one(look, role, &note, palette.dim));
        explain_note(
            ui,
            Rect::from_min_size(pos2(x, center - line / 2.0), vec2(width, line)),
            &note,
            locale,
        );
        x += width;
        x += widgets::paint_text(ui, x, center, dot_text());
    }
    // The menus read as one line with the note: their words, a dot apart.
    let menus_right = x + menus_width(ui, dot, bar);
    let menus_left = menus(ui, (menus_right, center, line + 4.0), dot, bar, actions);
    let limit_right = menus_left + menu_width(ui, &limit_text(bar.limit, look, locale), look);
    widgets::paint_text(ui, limit_right, center, dot_text());
}

/// A toolbar menu: what it sets, what it is set to, and its choices.
struct Menu<'a, T> {
    /// What the menu sets, for screen readers ("Limit").
    name: &'a str,
    /// The choice in use, as the menu's button reads ("Limit 1,000").
    current: &'a str,
    choices: &'a [(T, String)],
    selected: T,
}

/// The role a menu's button reads in.
fn menu_role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::Secondary, TextRole::OBody)
}

/// The width of a menu's button reading `current`. macOS: a 1 pt border,
/// 10 of padding, the words, 6, a 10 pt chevron. The terminal: the words.
fn menu_width(ui: &Ui, current: &str, look: &Look) -> f32 {
    let text = menu_role(look).width(ui.ctx(), look.faces, current);
    if look.terminal {
        text
    } else {
        (1.0 + 10.0 + text + 6.0 + 10.0 + 10.0 + 1.0).ceil()
    }
}

/// A menu's button in `rect`, and its choices while it is open. Returns
/// the choice picked this frame, if it is another than the one in use.
fn menu<T: Copy + PartialEq>(
    ui: &mut Ui,
    rect: Rect,
    spec: &Menu<'_, T>,
    look: &Look,
    palette: &Palette,
) -> Option<T> {
    let response = ui.interact(rect, ui.id().with(("menu", spec.name)), Sense::click());
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, spec.name);
        info.current_text_value = Some(spec.current.to_owned());
        info
    });
    let center = rect.center().y;
    let role = menu_role(look);
    if look.terminal {
        // Muted words that light up under the pointer, as the bar's other
        // notes that answer a click.
        let color = if response.hovered() || response.has_focus() {
            palette.text
        } else {
            palette.dim
        };
        widgets::paint_text(
            ui,
            rect.left(),
            center,
            Text::one(look, role, spec.current, color),
        );
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
            Text::one(look, role, spec.current, palette.text),
        );
        Icon::ChevronDown.image(palette.dim, 10.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.right() - 16.0, center), vec2(10.0, 10.0)),
        );
    }
    if response.has_focus() {
        ui.painter().rect_stroke(
            rect.expand(1.0),
            CornerRadius::same(if look.terminal { 0 } else { look.radius }),
            widgets::primary_focus_ring(palette),
            StrokeKind::Outside,
        );
    }
    let mut picked = None;
    egui::Popup::menu(&response).show(|ui| {
        ui.set_min_width(rect.width());
        for (choice, text) in spec.choices {
            let selected = *choice == spec.selected;
            let text = widgets::galley(ui, text, Color32::PLACEHOLDER, look);
            if ui.add(egui::Button::selectable(selected, text)).clicked() {
                picked = (!selected).then_some(*choice);
                // A pointer's click closes the menu by itself; a key's or a
                // screen reader's does not.
                ui.close();
            }
        }
    });
    picked
}

/// Whether the last run ended and ran something. A run that failed as a
/// whole ran nothing, and the result still held is an older run's.
fn ran(sql: &SqlTab) -> bool {
    !sql.is_running()
        && sql.run.error.is_none()
        && sql
            .run
            .value
            .as_ref()
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
            let mut x = left;
            for piece in start {
                x += widgets::paint_label(ui, x, center, text(piece)) + gap;
            }
            let mut x = right;
            for piece in end.into_iter().chain([cursor.as_str()]) {
                x -= width(piece);
                widgets::paint_label(ui, x, center, text(piece));
                x -= gap;
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
