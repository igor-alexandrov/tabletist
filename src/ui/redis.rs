//! A Redis connection's window: the keys in the sidebar, grouped by their
//! separator, with the scan's progress under them, and the value of the
//! key open in front. It takes the window's [`Browser`] and edits only its
//! filter's text in place; everything else it asks for as actions.

use egui::{
    CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::redis::{KeyInfo, KeyType};

use crate::i18n::gettext;
use crate::keymap::{Command, Keymap, Scope};
use crate::keyspace::{Browser, KeyTab, Place, RedisAction, Row, ScanState, Shown, lossy};
use crate::model::{Action, ConnTabId};
use crate::style::Icon;
use crate::theme::Look;
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::{Paint, focus};

/// The sidebar's width, as the boards draw it.
fn sidebar_width(look: &Look) -> f32 {
    if look.terminal { 300.0 } else { 301.0 }
}

/// How often the scan's progress is said to a screen reader, at most.
const ANNOUNCE_EVERY: std::time::Duration = std::time::Duration::from_secs(2);

/// The value tag slot of each type: the colours the grid's tags have, a
/// fixed one each.
fn tag_slot(kind: &KeyType) -> usize {
    match kind {
        KeyType::Hash => 0,
        KeyType::List => 1,
        KeyType::Set => 3,
        KeyType::ZSet | KeyType::Other(_) | KeyType::Missing => 5,
        KeyType::Stream => 6,
        KeyType::String | KeyType::Json => 7,
    }
}

/// The id of the sidebar's filter field, by window.
pub fn filter_id(tab: ConnTabId) -> Id {
    Id::new(("redis-filter", tab.0))
}

/// The id of the tree's one keyboard stop, by window.
pub fn tree_id(tab: ConnTabId) -> Id {
    Id::new(("redis-tree", tab.0))
}

/// Draws the window, and returns what it asks for. `stale` dims what a
/// lost connection left on screen.
pub fn show(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    browser: &mut Browser,
    keymap: &Keymap,
    stale: bool,
) -> Vec<Action> {
    let mut actions = Vec::new();
    let Paint { look, palette, .. } = paint;
    if look.terminal {
        terminal_footer(ui, paint, tab, browser, keymap);
    }
    egui::Panel::left(Id::new(("redis-sidebar", tab.0)))
        .resizable(true)
        .default_size(sidebar_width(&look))
        .size_range(200.0..=520.0)
        .frame(
            egui::Frame::new()
                .fill(palette.panel)
                .inner_margin(egui::Margin {
                    right: 1,
                    ..Default::default()
                }),
        )
        .show_separator_line(false)
        .show(ui, |ui| {
            if stale {
                ui.multiply_opacity(super::states::STALE);
            }
            sidebar(ui, paint, tab, browser, keymap, &mut actions);
        });
    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(palette.window))
        .show(ui, |ui| {
            if stale {
                ui.multiply_opacity(super::states::STALE);
            }
            if !look.terminal {
                key_tabs(ui, paint, tab, browser, &mut actions);
            }
            main(ui, paint, tab, browser, &mut actions);
        });
    actions
}

/// The sidebar: the filter, the database's line, the tree, and how the
/// scan stands.
fn sidebar(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    browser: &mut Browser,
    keymap: &Keymap,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let full = ui.max_rect();
    widgets::vline(ui, full.right() + 0.5, full.y_range(), palette.outline);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    let pad = if look.terminal { 10.0 } else { 12.0 };
    ui.add_space(pad);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let size = vec2(
            full.width() - 2.0 * pad,
            if look.terminal { 32.0 } else { 34.0 },
        );
        let hint = if look.terminal {
            gettext(locale, "filter keys · book:*").into_owned()
        } else {
            gettext(locale, "Filter keys…").into_owned()
        };
        let style = widgets::FieldStyle {
            fill: Some(palette.window),
            boxed: true,
            role: TextRole::pick(&look, TextRole::UiBody, TextRole::OField),
        };
        let field =
            widgets::filter_field(ui, &mut browser.filter, &hint, size, style, &look, &palette);
        let name = gettext(locale, "Filter keys");
        field.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &*name));
        focus::region(ui, focus::Region::Search, field.rect.expand(8.0));
        focus::claim(ui, focus::Region::Search, &field);
        if std::mem::take(&mut browser.focus_filter) {
            field.request_focus();
        }
        // The key that brings the keyboard here, at the field's right.
        if !look.terminal && !field.has_focus() && browser.filter.is_empty() {
            let key = keymap.key_in(paint.layout(), Command::FilterKeys, Scope::Redis);
            if !key.as_str().is_empty() {
                let text = Text::one(&look, TextRole::Shortcut, key.as_str(), palette.dim);
                widgets::paint_text_right(
                    ui,
                    field.rect.right() - 10.0,
                    field.rect.center().y,
                    text,
                );
            }
        }
        let entered = field.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if entered {
            actions.push(Action::Redis(tab, RedisAction::Filter));
        }
    });
    ui.add_space(8.0);
    database_line(ui, paint, browser, full);
    let footer = footer_height(browser, &look);
    let body = (ui.available_height() - footer).max(0.0);
    ui.allocate_ui(vec2(full.width(), body), |ui| {
        tree(ui, paint, tab, browser, actions);
    });
    footer_of(ui, paint, tab, browser, keymap, full, actions);
}

/// "db 0" over the tree, with the keys the database holds.
fn database_line(ui: &mut Ui, paint: Paint, browser: &Browser, full: Rect) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let (rect, _) = ui.allocate_exact_size(vec2(full.width(), 24.0), Sense::hover());
    let role = TextRole::pick(&look, TextRole::SectionLabel, TextRole::OSecondary);
    let name = format!("db {}", browser.database);
    let name = if look.terminal {
        name
    } else {
        name.to_uppercase()
    };
    let y = rect.center().y;
    widgets::paint_text(
        ui,
        rect.left() + 16.0,
        y,
        Text::one(&look, role, &name, palette.secondary),
    );
    if let Some(keys) = browser.keys() {
        let said = keys_words(keys, locale);
        widgets::paint_text_right(
            ui,
            rect.right() - 16.0,
            y,
            Text::one(&look, widgets::secondary(&look), &said, palette.dim),
        );
    }
}

/// "1,204 keys", "1 key", "empty".
fn keys_words(keys: u64, locale: crate::i18n::Locale) -> String {
    match keys {
        0 => gettext(locale, "empty").into_owned(),
        1 => gettext(locale, "1 key").into_owned(),
        keys => format!(
            "{} {}",
            crate::format::group_digits(keys),
            gettext(locale, "keys")
        ),
    }
}

/// A group's count as the tree shows it: growing (`31+`) while the scan
/// runs, from a sample (`≈13`) when it stopped short.
fn count_words(count: usize, browser: &Browser) -> String {
    let count = crate::format::group_digits(count as u64);
    match browser.scan.state {
        ScanState::Running { .. } => format!("{count}+"),
        ScanState::Limited | ScanState::Stopped | ScanState::Failed(_) => format!("≈{count}"),
        ScanState::Done | ScanState::Idle => count,
    }
}

fn row_height(row: &Row, look: &Look) -> f32 {
    match (row, look.terminal) {
        (Row::Group { .. }, true) => 24.0,
        (Row::Group { .. }, false) => 26.0,
        (_, true) => 22.0,
        (_, false) => 24.0,
    }
}

/// The tree of keys: one keyboard stop, its rows built as they come into
/// view, and the types asked for the keys on screen.
fn tree(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    browser: &mut Browser,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let rows = browser.rows();
    let area = Rect::from_min_size(
        ui.cursor().min,
        vec2(ui.available_width(), ui.available_height()),
    );
    let stop = ui.interact(area, tree_id(tab), Sense::focusable_noninteractive());
    let name = gettext(locale, "Keys");
    stop.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &*name));
    ui.ctx().accesskit_node_builder(stop.id, |node| {
        node.set_role(egui::accesskit::Role::Tree);
    });
    focus::pane(ui, &stop);
    focus::hint(ui, &stop, area, focus::Ring::Own);
    focus::region(ui, focus::Region::Tree, area);
    focus::claim(ui, focus::Region::Tree, &stop);
    if stop.gained_focus() && browser.cursor.is_none() {
        actions.push(Action::Redis(tab, RedisAction::Move(0)));
    }
    if rows.is_empty() {
        let said = match &browser.scan.state {
            ScanState::Running { .. } | ScanState::Idle => gettext(locale, "Scanning…"),
            ScanState::Failed(error) => std::borrow::Cow::Owned(error.to_string()),
            ScanState::Done | ScanState::Stopped | ScanState::Limited => {
                gettext(locale, "No keys match.")
            }
        };
        ui.add_space(12.0);
        ui.horizontal(|ui| {
            ui.add_space(16.0);
            let width = ui.available_width() - 16.0;
            Text::one(&look, widgets::secondary(&look), &said, palette.dim)
                .wrap(width)
                .layout(ui.ctx())
                .label(ui);
        });
        return;
    }
    let reveal = std::mem::take(&mut browser.reveal)
        .then(|| {
            let place = browser.cursor.as_ref()?;
            rows.iter().position(|row| row.place() == *place)
        })
        .flatten();
    let keys = stop.has_focus();
    let active = browser.active_tab().map(|open| open.key.clone());
    let mut visible = Vec::new();
    let mut parent = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(area)
            .accessibility_parent(stop.id),
    );
    egui::ScrollArea::vertical()
        .id_salt(("redis-tree-scroll", tab.0))
        .auto_shrink([false, false])
        .show(&mut parent, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            let heights: Vec<f32> = rows.iter().map(|row| row_height(row, &look)).collect();
            if let Some(index) = reveal {
                let top = ui.cursor().top() + heights[..index].iter().sum::<f32>();
                let rect =
                    Rect::from_min_size(pos2(ui.cursor().left(), top), vec2(1.0, heights[index]));
                ui.scroll_to_rect_animation(rect, None, egui::style::ScrollAnimation::none());
            }
            widgets::virtual_rows_varying(ui, &heights, |ui, index| {
                let row = &rows[index];
                if let Row::Key { key, .. } = row {
                    visible.push(key.clone());
                }
                let cursor = keys && browser.cursor.as_ref() == Some(&row.place());
                let selected = matches!(row, Row::Key { key, .. } if active.as_ref() == Some(key));
                tree_row(ui, paint, tab, row, browser, (cursor, selected), actions);
            });
            ui.add_space(8.0);
        });
    let unknown: Vec<_> = visible
        .into_iter()
        .filter(|key| !browser.types.contains_key(key) && !browser.asking.contains(key))
        .collect();
    if !unknown.is_empty() {
        actions.push(Action::Redis(tab, RedisAction::Visible(unknown)));
    }
}

/// One row of the tree: a group with its chevron and count, a key with
/// its type, or the keys of a group not shown yet.
fn tree_row(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    row: &Row,
    browser: &Browser,
    (cursor, selected): (bool, bool),
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let height = row_height(row, &look);
    let (full, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let inset = if look.terminal { 4.0 } else { 8.0 };
    let rect = full.shrink2(vec2(inset, 0.0));
    let id = Id::new(("redis-row", tab.0)).with(row.place());
    let response = ui.interact(rect, id, Sense::click());
    let painter = ui.painter();
    let radius = CornerRadius::same(if look.terminal { 0 } else { 6 });
    if selected {
        painter.rect_filled(rect, radius, palette.selection);
    } else if response.hovered() {
        painter.rect_filled(rect, radius, palette.surface_hover);
    }
    if cursor {
        painter.rect_stroke(
            rect,
            radius,
            Stroke::new(1.0, palette.accent),
            StrokeKind::Inside,
        );
    }
    let indent = 18.0 * row.depth() as f32;
    let y = rect.center().y;
    let mono = TextRole::pick(&look, TextRole::MonoGroup, TextRole::OBody);
    let key_role = TextRole::pick(&look, TextRole::MonoSecondary, TextRole::OBody);
    let count_role = widgets::secondary(&look);
    match row {
        Row::Group {
            name,
            prefix,
            count,
            open,
            ..
        } => {
            let left = rect.left() + 8.0 + indent;
            let chevron = Rect::from_center_size(pos2(left + 5.0, y), vec2(10.0, 10.0));
            let icon = if *open {
                Icon::ChevronDown
            } else {
                Icon::ChevronRight
            };
            icon.image(palette.dim, 10.0).paint_at(ui, chevron);
            let name = lossy(name);
            widgets::paint_text(
                ui,
                left + 16.0,
                y,
                Text::one(&look, mono, &name, palette.text),
            );
            let counted = count_words(*count, browser);
            widgets::paint_text_right(
                ui,
                rect.right() - 8.0,
                y,
                Text::one(&look, count_role, &counted, palette.dim),
            );
            let keys = gettext(locale, "keys");
            let said = format!(
                "{name}, {} {keys}",
                crate::format::group_digits(*count as u64)
            );
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &said));
            let open = *open;
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::TreeItem);
                node.set_expanded(open);
            });
            if response.clicked() {
                actions.push(Action::Redis(tab, RedisAction::ToggleGroup(prefix.clone())));
            }
        }
        Row::Key { key, name, .. } => {
            let left = rect.left() + 8.0 + indent + if row.depth() == 0 { 16.0 } else { 0.0 };
            let kind = browser.types.get(key);
            let tag_width = kind.map_or(0.0, |kind| {
                paint_tag(ui, paint, kind, rect.right() - 8.0, y)
            });
            let room = (rect.right() - 8.0 - tag_width - 8.0 - left).max(0.0);
            let name = lossy(name);
            let shown = crate::ui::grid::ellipsize(&name, room, false, |text| {
                key_role.width(ui.ctx(), look.faces, text)
            });
            let color = if selected {
                palette.accent
            } else {
                palette.text
            };
            widgets::paint_text(ui, left, y, Text::one(&look, key_role, &shown, color));
            let said = match kind {
                Some(kind) => format!("{name}, {}", kind.tag()),
                None => name.clone(),
            };
            response.widget_info(|| WidgetInfo::selected(WidgetType::Other, true, selected, &said));
            ui.ctx().accesskit_node_builder(response.id, |node| {
                node.set_role(egui::accesskit::Role::TreeItem);
            });
            if response.clicked() {
                actions.push(Action::Redis(tab, RedisAction::OpenKey(key.clone())));
            }
        }
        Row::More { prefix, left, .. } => {
            let x = rect.left() + 8.0 + indent + 16.0;
            let said = format!(
                "{} {}",
                crate::format::group_digits(*left as u64),
                gettext(locale, "more")
            );
            let color = if response.hovered() {
                palette.accent
            } else {
                palette.dim
            };
            widgets::paint_text(ui, x, y, Text::one(&look, count_role, &said, color));
            response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &said));
            if response.clicked() {
                actions.push(Action::Redis(tab, RedisAction::More(prefix.clone())));
                actions.push(Action::Redis(
                    tab,
                    RedisAction::Cursor(Place::More(prefix.clone())),
                ));
            }
        }
    }
}

/// A type's tag ending at `right`, centred on `y`: its width.
fn paint_tag(ui: &Ui, paint: Paint, kind: &KeyType, right: f32, y: f32) -> f32 {
    let Paint { look, palette, .. } = paint;
    let (ink, fill) = super::value_tags::slot_colors(tag_slot(kind), &look, &palette);
    let role = TextRole::pick(&look, TextRole::ValueTag, TextRole::OSecondary);
    let laid = Text::one(&look, role, kind.tag(), ink).layout(ui.ctx());
    let pad = if fill.is_some() { 6.0 } else { 0.0 };
    let width = laid.width() + 2.0 * pad;
    if let Some(fill) = fill {
        let rect = Rect::from_min_max(
            pos2(right - width, y - laid.height() / 2.0 - 1.0),
            pos2(right, y + laid.height() / 2.0 + 1.0),
        );
        ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    }
    laid.paint_left(ui.painter(), right - width + pad, y);
    width
}

/// How tall the sidebar's footer is: taller with the Scan all question.
fn footer_height(browser: &Browser, look: &Look) -> f32 {
    let base = if look.terminal { 30.0 } else { 40.0 };
    let limited = matches!(browser.scan.state, ScanState::Limited);
    if browser.confirm_scan_all {
        base + 74.0
    } else if limited {
        base + 22.0
    } else {
        base
    }
}

/// What the scan says under the tree, and its buttons: Reload when it has
/// ended, Stop while it runs, Scan all when it stopped at the limit.
fn footer_of(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    browser: &mut Browser,
    keymap: &Keymap,
    full: Rect,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    let height = footer_height(browser, &look);
    let rect = Rect::from_min_max(pos2(full.left(), full.bottom() - height), full.max);
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let mut child = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(vec2(16.0, 8.0)))
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    let ui = &mut child;
    let role = widgets::secondary(&look);
    let held = browser.tree.len() as u64;
    let total = browser.scan.total;
    let of = |count: u64| crate::format::group_digits(count);
    let took = crate::format::elapsed(browser.scan.took);
    let (said, button): (String, Option<(String, RedisAction)>) = match &browser.scan.state {
        ScanState::Idle => (say("Scanning…"), None),
        ScanState::Running { .. } => (
            format!(
                "{} {}+ {} {} {}",
                say("Scanned"),
                of(held),
                say("of"),
                total.map_or_else(|| "?".into(), of),
                say("keys")
            ),
            Some((say("Stop"), RedisAction::Stop)),
        ),
        ScanState::Done => (
            format!(
                "{} {} {} {} {} · {took}",
                say("Scanned"),
                of(held),
                say("of"),
                of(total.unwrap_or(held)),
                say("keys")
            ),
            Some((reload_words(paint, keymap), RedisAction::Reload)),
        ),
        ScanState::Stopped => (
            format!(
                "{} {} {} {} {}",
                say("Stopped at"),
                of(held),
                say("of"),
                total.map_or_else(|| "?".into(), of),
                say("keys")
            ),
            Some((reload_words(paint, keymap), RedisAction::Reload)),
        ),
        ScanState::Limited => (
            format!(
                "{} {} {} {} {}",
                say("Showing groups from the first"),
                of(held),
                say("of"),
                total.map_or_else(|| "?".into(), crate::format::compact_count),
                say("keys")
            ),
            Some((say("Scan all"), RedisAction::ScanAll)),
        ),
        ScanState::Failed(error) => (
            error.to_string(),
            Some((say("Reload"), RedisAction::Reload)),
        ),
    };
    let tooltip = gettext(
        locale,
        "Keys added or removed while the scan runs may or may not appear until the next reload.",
    );
    let width = ui.available_width();
    let line = ui
        .horizontal_wrapped(|ui| {
            ui.set_width(width);
            ui.spacing_mut().item_spacing.x = 6.0;
            let color = match browser.scan.state {
                ScanState::Failed(_) => palette.danger,
                _ => palette.dim,
            };
            Text::one(&look, role, &said, color)
                .layout(ui.ctx())
                .label(ui);
            if let Some((text, action)) = button {
                let link = ButtonSpec::new(&text).quiet().link();
                if link.show(ui, 18.0, &look, &palette).clicked() {
                    actions.push(Action::Redis(tab, action));
                }
            }
        })
        .response
        .on_hover_text(tooltip);
    // A status region, said at most every two seconds while it changes.
    ui.ctx().accesskit_node_builder(line.id, |node| {
        node.set_role(egui::accesskit::Role::Status);
    });
    let due = browser
        .scan
        .announced
        .is_none_or(|at| at.elapsed() >= ANNOUNCE_EVERY);
    if due {
        widgets::announce_polite(ui, line.rect, &said);
        browser.scan.announced = Some(std::time::Instant::now());
    }
    if matches!(browser.scan.state, ScanState::Limited) && !browser.confirm_scan_all {
        widgets::label(
            ui,
            role,
            &say("or filter with a pattern"),
            palette.dim,
            &look,
        );
    }
    if browser.confirm_scan_all {
        let keys = total.unwrap_or(held);
        let bytes = browser.bytes_for(keys);
        let warning = gettext(
            locale,
            "Scanning {keys} keys may take a while and hold about {size} in Tabletist.",
        )
        .replace("{keys}", &crate::format::compact_count(keys))
        .replace(
            "{size}",
            &crate::format::human_size(usize::try_from(bytes).unwrap_or(usize::MAX)),
        );
        ui.add_space(6.0);
        Text::one(&look, role, &warning, palette.warning)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ButtonSpec::new(&say("Scan all"))
                .primary()
                .show(ui, 26.0, &look, &palette)
                .clicked()
            {
                actions.push(Action::Redis(tab, RedisAction::ConfirmScanAll(true)));
            }
            if ButtonSpec::new(&say("Cancel"))
                .show(ui, 26.0, &look, &palette)
                .clicked()
            {
                actions.push(Action::Redis(tab, RedisAction::ConfirmScanAll(false)));
            }
        });
    }
}

/// "Reload ⌘R", as the keymap writes the key.
fn reload_words(paint: Paint, keymap: &Keymap) -> String {
    let reload = paint.look.label(&gettext(paint.locale, "Reload"));
    let key = keymap.key_in(paint.layout(), Command::ReloadKeys, Scope::Redis);
    match key.as_str() {
        "" => reload,
        key => format!("{reload} {key}"),
    }
}

/// macOS: the keys open, as tabs over the value.
fn key_tabs(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    browser: &Browser,
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    if browser.tabs.is_empty() {
        return;
    }
    let (strip, _) = ui.allocate_exact_size(vec2(ui.available_width(), 36.0), Sense::hover());
    ui.painter().rect_filled(strip, 0.0, palette.panel);
    widgets::hline(ui, strip.x_range(), strip.bottom() - 0.5, palette.outline);
    let role = TextRole::pick(&look, TextRole::MonoSecondary, TextRole::OBody);
    let mut left = strip.left();
    let close = gettext(locale, "Close");
    for (at, open) in browser.tabs.iter().enumerate() {
        let name = lossy(&open.key);
        let shown = crate::ui::grid::ellipsize(&name, 220.0, false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        let laid = Text::one(&look, role, &shown, palette.text).layout(ui.ctx());
        let width = laid.width() + 14.0 + 8.0 + 16.0 + 10.0;
        let rect = Rect::from_min_size(pos2(left, strip.top()), vec2(width, strip.height()));
        left += width;
        let front = browser.active == Some(at);
        let response = ui.interact(rect, Id::new(("redis-key-tab", tab.0, at)), Sense::click());
        response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, front, &name));
        if front {
            ui.painter().rect_filled(rect, 0.0, palette.window);
            ui.painter().hline(
                rect.x_range(),
                rect.top() + 1.0,
                Stroke::new(2.0, palette.accent),
            );
        }
        widgets::vline(ui, rect.right() - 0.5, rect.y_range(), palette.outline);
        laid.paint_left(ui.painter(), rect.left() + 14.0, rect.center().y);
        let x =
            Rect::from_center_size(pos2(rect.right() - 18.0, rect.center().y), vec2(16.0, 16.0));
        let close_response = ui.interact(
            x,
            Id::new(("redis-key-tab-close", tab.0, at)),
            Sense::click(),
        );
        let named = format!("{close} {name}");
        close_response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, &named));
        let tint = if close_response.hovered() {
            palette.text
        } else {
            palette.dim
        };
        Icon::X.image(tint, 12.0).paint_at(ui, x.shrink(2.0));
        if close_response.clicked() {
            actions.push(Action::Redis(tab, RedisAction::CloseTab(at)));
        } else if response.clicked() {
            actions.push(Action::Redis(tab, RedisAction::SelectTab(at)));
        }
    }
}

/// The key in front: its header, its value, and how it was read.
fn main(ui: &mut Ui, paint: Paint, tab: ConnTabId, browser: &Browser, actions: &mut Vec<Action>) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let Some(open) = browser.active_tab() else {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 20.0);
            let said = gettext(locale, "Choose a key to see its value.");
            widgets::label(ui, widgets::body(&look), &said, palette.dim, &look);
        });
        return;
    };
    egui::Frame::new()
        .inner_margin(egui::Margin {
            left: 24,
            right: 24,
            top: 14,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            header(ui, paint, tab, open, actions);
        });
    let rect = ui.available_rect_before_wrap();
    widgets::hline(ui, rect.x_range(), rect.top(), palette.outline);
    egui::Frame::new()
        .inner_margin(egui::Margin::symmetric(24, 18))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            value(ui, paint, tab, open, actions);
        });
}

/// "6 fields · 412 B · listpack" and the expiry: what the header says
/// under the key's name.
fn facts(info: &KeyInfo, locale: crate::i18n::Locale) -> (String, String) {
    let say = |text: &'static str| gettext(locale, text).into_owned();
    let mut parts = Vec::new();
    if let Some(length) = info.length {
        let unit = match (&info.kind, length) {
            (KeyType::Hash, 1) => say("field"),
            (KeyType::Hash, _) => say("fields"),
            (KeyType::List, 1) | (KeyType::Set, 1) => say("item"),
            (KeyType::List, _) => say("items"),
            (KeyType::Set, _) | (KeyType::ZSet, _) => say("members"),
            (KeyType::Stream, 1) => say("entry"),
            (KeyType::Stream, _) => say("entries"),
            (KeyType::String, _) => "B".to_owned(),
            (KeyType::Json | KeyType::Other(_) | KeyType::Missing, _) => String::new(),
        };
        match info.kind {
            KeyType::String => parts.push(crate::format::human_size(
                usize::try_from(length).unwrap_or(usize::MAX),
            )),
            _ => parts.push(format!("{} {unit}", crate::format::group_digits(length))),
        }
    }
    if let Some(memory) = info.memory {
        parts.push(crate::format::human_size(
            usize::try_from(memory).unwrap_or(usize::MAX),
        ));
    }
    if let Some(encoding) = &info.encoding {
        parts.push(encoding.clone());
    }
    let expiry = match info.ttl_ms {
        None => say("No expiry"),
        Some(ms) => {
            let seconds = (ms / 1000).max(0);
            let span = match seconds {
                0..=59 => format!("{seconds} s"),
                60..=3599 => format!("{} min", seconds / 60),
                3600..=86_399 => format!("{} h", seconds / 3600),
                _ => format!("{} d", seconds / 86_400),
            };
            format!("{} {span}", say("Expires in"))
        }
    };
    (parts.join(" · "), expiry)
}

/// The key's name, its type, size, encoding and expiry, and Copy value
/// and Copy key at the right.
fn header(ui: &mut Ui, paint: Paint, tab: ConnTabId, open: &KeyTab, actions: &mut Vec<Action>) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            let title = TextRole::pick(&look, TextRole::TableTitle, TextRole::OTableTitle);
            let name = lossy(&open.key);
            Text::one(&look, title, &name, palette.text)
                .layout(ui.ctx())
                .label(ui);
            if let Some(info) = &open.info.value {
                let (line, expiry) = facts(info, locale);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    let (rect, _) = ui.allocate_exact_size(vec2(44.0, 18.0), Sense::hover());
                    let width =
                        paint_tag(ui, paint, &info.kind, rect.left() + 44.0, rect.center().y);
                    let _ = width;
                    let role = widgets::secondary(&look);
                    let text = [line, expiry]
                        .into_iter()
                        .filter(|part| !part.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ");
                    let text = look.label(&text);
                    widgets::label(ui, role, &text, palette.dim, &look);
                });
            } else if open.loading.is_some() {
                widgets::label(
                    ui,
                    widgets::secondary(&look),
                    &say("Reading…"),
                    palette.dim,
                    &look,
                );
            }
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if ButtonSpec::new(&say("Copy key"))
                .show(ui, 30.0, &look, &palette)
                .clicked()
            {
                actions.push(Action::Redis(tab, RedisAction::CopyKey));
            }
            let can_copy = open.copied().is_some();
            ui.add_enabled_ui(can_copy, |ui| {
                if ButtonSpec::new(&say("Copy value"))
                    .show(ui, 30.0, &look, &palette)
                    .clicked()
                {
                    actions.push(Action::Redis(tab, RedisAction::CopyValue));
                }
            });
        });
    });
}

/// The value: a string or JSON as text, the others as a table, then Load
/// more and the line that says how it was read.
fn value(ui: &mut Ui, paint: Paint, tab: ConnTabId, open: &KeyTab, actions: &mut Vec<Action>) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let say = |text: &'static str| look.label(&gettext(locale, text));
    if let Some(error) = &open.error {
        let width = ui.available_width();
        Text::one(
            &look,
            widgets::body(&look),
            &error.to_string(),
            palette.danger,
        )
        .wrap(width)
        .layout(ui.ctx())
        .label(ui);
        return;
    }
    let footer_room = 40.0;
    let height = (ui.available_height() - footer_room).max(60.0);
    if let Some(unsupported) = &open.rows.unsupported {
        let said = format!(
            "{} ({unsupported})",
            gettext(locale, "This type isn't shown yet")
        );
        widgets::label(ui, widgets::body(&look), &said, palette.dim, &look);
    } else if let Some(text) = &open.rows.text {
        let mono = TextRole::pick(&look, TextRole::Code, TextRole::OCode);
        let (shown, note) = match text {
            Shown::Text(text) | Shown::Json(text) => (text.as_str(), None),
            Shown::Binary { hex, length } => (
                hex.as_str(),
                Some(format!(
                    "{}, {}",
                    gettext(locale, "Binary"),
                    crate::format::human_size(usize::try_from(*length).unwrap_or(usize::MAX))
                )),
            ),
        };
        if let Some(note) = note {
            widgets::label(ui, widgets::secondary(&look), &note, palette.dim, &look);
            ui.add_space(6.0);
        }
        if let Some((read, length)) = open.text_length
            && read < length
        {
            let cut = gettext(locale, "Showing the first 256 KiB");
            widgets::label(ui, widgets::secondary(&look), &cut, palette.warning, &look);
            ui.add_space(6.0);
        }
        egui::ScrollArea::both()
            .id_salt(("redis-text", tab.0))
            .max_height(height)
            .auto_shrink([false, true])
            .show(ui, |ui| {
                let width = ui.available_width();
                let laid = Text::one(&look, mono, shown, palette.text)
                    .wrap(width)
                    .layout(ui.ctx());
                let response = laid.label(ui);
                let name = gettext(locale, "Value");
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &*name));
            });
    } else if !open.rows.columns.is_empty() {
        table(ui, paint, tab, open, height, actions);
    } else if open.loading.is_some() {
        ui.spinner();
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        if let Some(line) = read_line(open, paint) {
            widgets::label(ui, widgets::secondary(&look), &line, palette.dim, &look);
        }
        if open.next.is_some() {
            let total = open.info.value.as_ref().and_then(|info| info.length);
            let more = match total {
                Some(total) => format!(
                    "{} ({} {} {})",
                    say("Load more"),
                    crate::format::group_digits(open.rows.cells.len() as u64),
                    say("of"),
                    crate::format::group_digits(total)
                ),
                None => say("Load more"),
            };
            let busy = open.loading.is_some();
            ui.add_enabled_ui(!busy, |ui| {
                if ButtonSpec::new(&more)
                    .show(ui, 26.0, &look, &palette)
                    .clicked()
                {
                    actions.push(Action::Redis(tab, RedisAction::LoadMore));
                }
            });
        }
    });
}

/// "HGETALL · 6 fields · 1 ms": how the value shown was read.
fn read_line(open: &KeyTab, paint: Paint) -> Option<String> {
    let (command, took) = open.command?;
    let say = |text: &'static str| gettext(paint.locale, text).into_owned();
    let rows = open.rows.cells.len() as u64;
    let unit = match open.info.value.as_ref().map(|info| &info.kind) {
        Some(KeyType::Hash) => say("fields"),
        Some(KeyType::Stream) => say("entries"),
        Some(KeyType::List | KeyType::Set | KeyType::ZSet) => say("items"),
        Some(KeyType::String | KeyType::Json | KeyType::Other(_) | KeyType::Missing) | None => {
            String::new()
        }
    };
    let mut line = command.to_owned();
    if !unit.is_empty() {
        line = format!("{line} · {} {unit}", crate::format::group_digits(rows));
    }
    line = format!("{line} · {}", crate::format::elapsed(took));
    Some(paint.look.label(&line))
}

/// The rows of a hash, list, set, sorted set or stream, under their
/// columns' headers: a table to a screen reader, as the grid is.
fn table(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    open: &KeyTab,
    height: f32,
    actions: &mut Vec<Action>,
) {
    let Paint { locale, look, .. } = paint;
    let columns = &open.rows.columns;
    let width = ui.available_width();
    // The first column as wide as the board's, the rest share what is left.
    let first = match columns.len() {
        1 => width,
        _ => (width * 0.3).clamp(80.0, 260.0),
    };
    let rest = if columns.len() > 1 {
        (width - first) / (columns.len() - 1) as f32
    } else {
        0.0
    };
    let widths: Vec<f32> = (0..columns.len())
        .map(|at| if at == 0 { first } else { rest })
        .collect();
    let row_height = if look.terminal { 24.0 } else { 34.0 };
    let name = format!("{} {}", gettext(locale, "Value of"), lossy(&open.key));
    let (frame, _) = ui.allocate_exact_size(vec2(width, 0.0), Sense::hover());
    let table = ui.interact(
        Rect::from_min_size(frame.min, vec2(width, 1.0)),
        Id::new(("redis-table", tab.0)),
        Sense::hover(),
    );
    table.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, &name));
    ui.ctx().accesskit_node_builder(table.id, |node| {
        node.set_role(egui::accesskit::Role::Table);
        node.set_row_count(open.rows.cells.len() + 1);
        node.set_column_count(columns.len());
    });
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(ui.available_rect_before_wrap())
            .accessibility_parent(table.id)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    table_rows(
        &mut inside,
        paint,
        tab,
        open,
        (height, row_height),
        &widths,
        actions,
    );
    // What the table took, so the line under it follows it.
    ui.advance_cursor_after_rect(inside.min_rect());
}

/// The table's headers and its rows, scrolled under them. A click selects
/// a cell, and the copy key copies it, as a read-only grid's does.
fn table_rows(
    ui: &mut Ui,
    paint: Paint,
    tab: ConnTabId,
    open: &KeyTab,
    (height, row_height): (f32, f32),
    widths: &[f32],
    actions: &mut Vec<Action>,
) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let columns = &open.rows.columns;
    let width: f32 = widths.iter().sum();
    let cell_role = TextRole::pick(&look, TextRole::GridCell, TextRole::OBody);
    let head_role = TextRole::pick(&look, TextRole::ColumnType, TextRole::OSecondary);
    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
    if open.selected.is_some()
        && !ui.ctx().text_edit_focused()
        && ui.input(|input| {
            input
                .events
                .iter()
                .any(|event| matches!(event, egui::Event::Copy))
        })
    {
        actions.push(Action::Redis(tab, RedisAction::CopyCell));
    }
    // The headers.
    let (head, _) = ui.allocate_exact_size(vec2(width, 32.0), Sense::hover());
    ui.painter().rect_filled(head, 0.0, palette.panel);
    let mut x = head.left();
    for (at, column) in columns.iter().enumerate() {
        let rect = Rect::from_min_size(pos2(x, head.top()), vec2(widths[at], head.height()));
        let id = Id::new(("redis-head", tab.0, at));
        let response = ui.interact(rect, id, Sense::hover());
        let said = gettext(locale, column);
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &*said));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::ColumnHeader);
        });
        let text = look.label(&said);
        widgets::paint_text(
            ui,
            rect.left() + 12.0,
            rect.center().y,
            Text::one(&look, head_role, &text, palette.dim),
        );
        x += widths[at];
    }
    widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.outline);
    egui::ScrollArea::vertical()
        .id_salt(("redis-rows", tab.0))
        .max_height((height - 32.0).max(40.0))
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
            widgets::virtual_rows(ui, open.rows.cells.len(), row_height, |ui, index| {
                let cells = &open.rows.cells[index];
                let (rect, _) = ui.allocate_exact_size(vec2(width, row_height), Sense::hover());
                let mut x = rect.left();
                for (at, cell) in cells.iter().enumerate() {
                    let place =
                        Rect::from_min_size(pos2(x, rect.top()), vec2(widths[at], row_height));
                    x += widths[at];
                    let text = lossy(cell);
                    let id = Id::new(("redis-cell", tab.0, index, at));
                    let response = ui.interact(place, id, Sense::click());
                    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &text));
                    ui.ctx().accesskit_node_builder(response.id, |node| {
                        node.set_role(egui::accesskit::Role::Cell);
                    });
                    let selected = open.selected == Some((index, at));
                    if selected {
                        ui.painter().rect_filled(place, 0.0, palette.selection);
                        ui.painter().rect_stroke(
                            place,
                            0.0,
                            Stroke::new(1.0, palette.accent),
                            StrokeKind::Inside,
                        );
                    } else if response.hovered() {
                        ui.painter().rect_filled(place, 0.0, palette.surface_hover);
                    }
                    let room = (place.width() - 24.0).max(0.0);
                    let line = text.lines().next().unwrap_or_default();
                    let shown = crate::ui::grid::ellipsize(line, room, false, |text| {
                        cell_role.width(ui.ctx(), look.faces, text)
                    });
                    widgets::paint_text(
                        ui,
                        place.left() + 12.0,
                        place.center().y,
                        Text::one(&look, cell_role, &shown, palette.text),
                    );
                    if response.clicked() {
                        actions.push(Action::Redis(tab, RedisAction::SelectCell(index, at)));
                    }
                }
                widgets::hline(
                    ui,
                    rect.x_range(),
                    rect.bottom() - 0.5,
                    palette.surface_hover,
                );
            });
        });
}

/// The terminal look's line of keys at the window's foot, and how the
/// value in front was read at its right.
fn terminal_footer(ui: &mut Ui, paint: Paint, tab: ConnTabId, browser: &Browser, keymap: &Keymap) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let layout = paint.layout();
    let hints: [(Command, Scope, &str); 8] = [
        (Command::MoveInKeys, Scope::Redis, "move"),
        (Command::OpenKey, Scope::Redis, "open"),
        (Command::FoldKeyGroup, Scope::Redis, "fold"),
        (Command::FilterKeys, Scope::Redis, "filter"),
        (Command::ReloadKeys, Scope::Redis, "reload"),
        (Command::CopyKeyValue, Scope::Redis, "copy value"),
        (Command::CopyKeyName, Scope::Redis, "copy key"),
        (Command::SwitchDatabase, Scope::Global, "database"),
    ];
    egui::Panel::bottom(Id::new(("redis-footer", tab.0)))
        .resizable(false)
        .show_separator_line(false)
        .frame(
            egui::Frame::new()
                .fill(palette.panel)
                .inner_margin(egui::Margin::symmetric(12, 0)),
        )
        .exact_size(30.0)
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
            let role = TextRole::OSecondary;
            let mut x = full.left();
            let y = full.center().y;
            for (command, scope, said) in hints {
                let key = keymap.key_in(layout, command, scope);
                if key.as_str().is_empty() {
                    continue;
                }
                let words = gettext(locale, said);
                let laid = Text::new(&look)
                    .add(role, key.as_str(), palette.text)
                    .space(role, " ")
                    .add(role, &words, palette.dim)
                    .layout(ui.ctx());
                if x + laid.width() > full.right() - 200.0 {
                    break;
                }
                laid.paint_left(ui.painter(), x, y);
                x += laid.width() + 16.0;
            }
            if let Some(line) = browser.active_tab().and_then(|open| read_line(open, paint)) {
                widgets::paint_text_right(
                    ui,
                    full.right(),
                    y,
                    Text::one(&look, role, &line, palette.dim),
                );
            }
        });
}
