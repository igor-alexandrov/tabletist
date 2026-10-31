//! The calendar under the line of a date's editor: the month the text is
//! in, with its time fields, its zone and Now, and on macOS a footer that
//! offers NULL, DEFAULT and PostgreSQL's infinities. It never takes the
//! keyboard: the field keeps it, and every pick rewrites the field's text,
//! as typing would. In the terminal look it is painted under the float's
//! line, which says all it shows, and no screen reader hears it.

use egui::accesskit::Role;
use egui::{CornerRadius, Id, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use jiff::civil::{Date, Time, Weekday};

use crate::dates::{Column, Kind, Moment, Refusal};
use crate::i18n::Locale;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::cell_editor::Calendar;
use crate::ui::date_line::{full_month_name, tr, year_words};
use crate::ui::widgets;

/// What the calendar shows of the editor's text, made each frame.
#[derive(Debug, Clone, PartialEq)]
pub struct Model {
    /// The text it is of: a new one takes the month to the one it names.
    pub typed: String,
    /// The month the text is in (its first day), where the column keeps a
    /// day and the text names one. The calendar shows it unless its
    /// arrows took it to another since the text last named it.
    pub follows: Option<Date>,
    /// The month shown where the text has named none yet: the value's
    /// before the edit, else today's.
    pub start: Date,
    /// The day the text reads as: filled.
    pub selected: Option<Date>,
    /// The day the cell had before this edit: marked with a dot.
    pub was: Option<Date>,
    /// Today, in the zone the column's days are counted in: outlined.
    pub today: Date,
    /// The text reads as no day: the month is dimmed.
    pub dim: bool,
    /// The time a day picked keeps, and the offset the text has, which a
    /// pick keeps where the time is a repeated hour's.
    pub keeps: Option<Time>,
    pub offset: Option<jiff::tz::Offset>,
    /// The time fields, where the column keeps a time of day.
    pub parts: Vec<Part>,
    /// What the zone chip says, where the time row has one.
    pub zone: Option<Zone>,
    /// The zone's abbreviation then and its offset, as the terminal's time
    /// row says them: `EST · UTC-05`.
    pub abbreviation: Option<String>,
    /// The text Now writes, where the time row has Now.
    pub now: Option<String>,
    /// What the footer offers: NULL where the column takes it, DEFAULT
    /// where it has one, and PostgreSQL's infinities for a day.
    pub null: bool,
    pub default: bool,
    pub infinity: bool,
    /// The day of the week the weeks start on.
    pub week: Weekday,
    pub kind: Kind,
}

/// What the zone chip of the time row says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Zone {
    /// Where the zone a moment is shown in is: "New York".
    Place(String),
    /// A timestamp with no zone, stored as typed.
    None,
    /// A MySQL `TIMESTAMP` whose session's zone is not known.
    Server,
    /// A `timetz`'s offset, as typed: `-05`; a zone with no name, by the
    /// offset it keeps.
    Offset(String),
}

impl Zone {
    /// The chip's words.
    fn words(&self, locale: Locale) -> String {
        match self {
            Self::Place(place) | Self::Offset(place) => place.clone(),
            Self::None => tr(locale, "no zone"),
            Self::Server => tr(locale, "the server's zone"),
        }
    }
}

/// One of the time fields: what it is, what it shows, and its range.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    pub name: &'static str,
    /// The fraction of a second, after a point: the others are after a
    /// colon.
    pub fraction: bool,
    /// As the field shows it: `09`, or `--` where the text reads as no
    /// time.
    pub shown: String,
    pub value: Option<f64>,
    pub most: f64,
}

/// The time fields of `time` for `column`: hour, minute, second, and the
/// fraction where the type states one or the time has one.
fn parts(time: Option<Time>, column: &Column) -> Vec<Part> {
    let part = |name, value: Option<i64>, most: f64, width: usize| Part {
        name,
        fraction: false,
        shown: value.map_or_else(|| "-".repeat(width), |value| format!("{value:0width$}")),
        value: value.map(|value| value as f64),
        most,
    };
    let mut parts = vec![
        part("Hour", time.map(|time| i64::from(time.hour())), 23.0, 2),
        part("Minute", time.map(|time| i64::from(time.minute())), 59.0, 2),
        part("Second", time.map(|time| i64::from(time.second())), 59.0, 2),
    ];
    let digits = usize::from(column.digits.min(9));
    let fraction = time.is_some_and(|time| time.subsec_nanosecond() != 0);
    if digits > 0 && (column.fraction || fraction) {
        let value = time.map(|time| {
            let nanos = i64::from(time.subsec_nanosecond());
            nanos / 10_i64.pow(9 - digits as u32)
        });
        let most = 10_f64.powi(digits as i32) - 1.0;
        parts.push(Part {
            fraction: true,
            ..part("Fraction of a second", value, most, digits)
        });
    }
    parts
}

/// The calendar of the editor whose text is `text`, which reads as
/// `moment`, in the column `column`, whose cell had `was` before the
/// edit. `offers` is what the footer may offer: NULL, then DEFAULT.
pub fn model(
    (text, moment, was): (
        &str,
        &Result<crate::dates::Reading, Refusal>,
        Option<&Moment>,
    ),
    column: &Column,
    calendar: &Calendar,
    offers: (bool, bool),
) -> Model {
    let clock = &calendar.clock;
    let today = clock.today_in(column.zone.as_ref().unwrap_or(&clock.zone));
    let was = match was {
        Some(Moment::At { date, .. }) => *date,
        _ => None,
    };
    let (mut selected, mut keeps, mut dim, mut follows) = (None, None, true, None);
    let (mut offset, mut abbreviation) = (None, None);
    match moment {
        Ok(reading) => match &reading.moment {
            Moment::At {
                date,
                time,
                offset: typed,
            } => {
                (selected, keeps, dim) = (*date, *time, false);
                follows = *date;
                offset = *typed;
                abbreviation = reading
                    .zone_name
                    .as_ref()
                    .zip(*typed)
                    .map(|(name, offset)| {
                        format!("{name} · UTC{}", crate::dates::written_offset(offset))
                    });
            }
            // Dimmed, on today's month.
            Moment::Infinity | Moment::MinusInfinity => follows = Some(today),
            Moment::EndOfDay | Moment::Span(_) => dim = false,
        },
        // A time the clocks skip is a day all the same: the line offers
        // the time after it.
        Err(Refusal::Gap { at, .. }) => {
            (selected, keeps, dim) = (Some(at.date()), Some(at.time()), false);
            follows = selected;
        }
        Err(_) => {
            if let Some(sofar) = crate::dates::so_far(text, column, clock) {
                (follows, keeps) = (Some(sofar.month), sofar.time);
            }
        }
    }
    let zone = match column.kind {
        Kind::Zoned => Some(match &column.zone {
            Some(zone) => match crate::dates::place(zone) {
                // A fixed offset, or a POSIX rule, has no name to tell
                // its place by: the offset it keeps now is its words.
                place if place.is_empty() => Zone::Offset(crate::dates::written_offset(
                    zone.to_offset(calendar.clock.now),
                )),
                place => Zone::Place(place),
            },
            None => Zone::Server,
        }),
        Kind::Timestamp => Some(Zone::None),
        Kind::Time { offset: true } => {
            offset.map(|offset| Zone::Offset(crate::dates::written_offset(offset)))
        }
        Kind::Time { offset: false } | Kind::Date => None,
    };
    let postgres = match column.dialect {
        // DuckDB keeps PostgreSQL's infinities.
        tabletist_db::Dialect::Postgres | tabletist_db::Dialect::DuckDb => true,
        tabletist_db::Dialect::MySql | tabletist_db::Dialect::Sqlite => false,
    };
    Model {
        typed: text.to_owned(),
        offset,
        follows: follows.map(Date::first_of_month),
        start: was.unwrap_or(today).first_of_month(),
        selected,
        was,
        today,
        dim,
        keeps,
        parts: if column.kind.has_time() {
            parts(keeps, column)
        } else {
            Vec::new()
        },
        zone,
        abbreviation,
        now: column
            .kind
            .has_time()
            .then(|| crate::dates::now_text(column, clock))
            .flatten(),
        null: offers.0,
        default: offers.1,
        infinity: postgres && column.kind.has_date(),
        week: calendar.week,
        kind: column.kind,
    }
}

/// What a frame of the calendar asks for.
#[derive(Debug, Default, PartialEq)]
pub struct Asked {
    /// The text a pick rewrites the field to.
    pub text: Option<String>,
    /// The pick ends the edit: NULL and DEFAULT, which are no day.
    pub commit: bool,
    /// A press on one of its controls.
    pub pressed: bool,
}

/// The heights of the macOS calendar's parts.
const HEAD: f32 = 32.0;
const WEEKDAYS: f32 = 22.0;
const DAY: f32 = 30.0;
const DAY_GAP: f32 = 2.0;
const TIME_ROW: f32 = 42.0;
const FOOTER: f32 = 38.0;
/// The terminal's rows, and its days' columns.
const TERMINAL_ROW: f32 = 22.0;
const TERMINAL_DAY: f32 = 32.0;
/// The room at the sides.
const PAD: f32 = 10.0;

/// How tall the calendar of `model` is, under the line.
pub fn height(model: &Model, look: &Look) -> f32 {
    let month = model.kind.has_date();
    let time = model.kind.has_time();
    if look.terminal {
        // The terminal's time row is a timestamp's: a time's line says
        // it, as its board has it.
        let month = if month {
            2.0 * TERMINAL_ROW + 6.0 * TERMINAL_ROW + 8.0
        } else {
            0.0
        };
        let time = if time && model.kind.has_date() {
            TERMINAL_ROW + 4.0
        } else {
            0.0
        };
        return month + time;
    }
    let month = if month {
        HEAD + WEEKDAYS + 6.0 * DAY + 5.0 * DAY_GAP + 8.0
    } else {
        0.0
    };
    let time = if time { TIME_ROW } else { 0.0 };
    month + time + FOOTER
}

/// Where the calendar of the editor `memory` names (its field and its
/// cell) keeps which month it shows: the text it last showed it for, the
/// month, and the frame it was drawn in.
fn shown_id(memory: Id) -> Id {
    memory.with("calendar-month")
}

/// The month to show: the one the text names, unless the arrows moved off
/// it since the text last changed. A text that names none leaves it where
/// it is. An editor drawn after a frame without one, or on another cell,
/// starts again.
fn month_shown(ui: &Ui, memory: Id, model: &Model) -> Date {
    let frame = ui.ctx().cumulative_frame_nr();
    let kept: Option<(String, Date, u64)> = ui.ctx().data(|data| data.get_temp(shown_id(memory)));
    let shown = match kept {
        Some((typed, shown, at)) if frame.saturating_sub(at) <= 1 => {
            match model.follows.filter(|_| typed != model.typed) {
                Some(follows) => follows,
                None => shown,
            }
        }
        _ => model.follows.unwrap_or(model.start),
    };
    remember(ui, memory, model, shown);
    shown
}

fn remember(ui: &Ui, memory: Id, model: &Model, shown: Date) {
    let frame = ui.ctx().cumulative_frame_nr();
    let kept = (model.typed.clone(), shown, frame);
    ui.ctx()
        .data_mut(|data| data.insert_temp(shown_id(memory), kept));
}

/// The first day of the month `by` months from `month`, where a date
/// holds it.
fn moved(month: Date, by: i64) -> Option<Date> {
    month.checked_add(jiff::Span::new().months(by)).ok()
}

const INITIALS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];
const WEEKDAY_NAMES: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

/// The weekdays in the order the weeks run, from `first`.
fn weekdays(first: Weekday) -> [Weekday; 7] {
    std::array::from_fn(|at| first.wrapping_add(at as i64))
}

fn initial(day: Weekday, locale: Locale) -> String {
    let at = usize::try_from(day.to_monday_zero_offset()).unwrap_or_default();
    tr(locale, INITIALS[at % 7])
}

/// "January 2026", "March 44 BC".
pub fn month_words(month: Date, locale: Locale) -> String {
    format!(
        "{} {}",
        full_month_name(month.month(), locale),
        year_words(month.year(), locale)
    )
}

/// "Thursday 15 January 2026": a day as a screen reader says it.
pub fn day_name(date: Date, locale: Locale) -> String {
    let at = usize::try_from(date.weekday().to_monday_zero_offset()).unwrap_or_default();
    format!(
        "{} {} {} {}",
        tr(locale, WEEKDAY_NAMES[at % 7]),
        date.day(),
        full_month_name(date.month(), locale),
        year_words(date.year(), locale)
    )
}

/// Draws the calendar of `model` from `top`, `width` wide, for the editor
/// `memory` names (its field and its cell) in `column`: on macOS with the
/// controls that pick, in the terminal look painted. Says what was picked.
pub fn show(
    ui: &mut Ui,
    (top, width): (egui::Pos2, f32),
    memory: Id,
    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Asked {
    // Its controls' ids are the editor's: one cell's are no other's. A
    // child that allocates nothing: the popover is no taller than it is.
    let room = Rect::from_min_max(top, pos2(top.x + width, ui.max_rect().bottom().max(top.y)));
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt(memory).max_rect(room));
    drawn(
        &mut child,
        (top, width),
        memory,
        (model, column, clock),
        (look, palette, locale),
    )
}

/// [`show`], in the editor's own scope.
fn drawn(
    ui: &mut Ui,
    (top, width): (egui::Pos2, f32),
    memory: Id,
    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Asked {
    let mut asked = Asked::default();
    let skin = (look, palette, locale);
    let mut y = top.y;
    let left = top.x;
    if model.kind.has_date() {
        let shown = month_shown(ui, memory, model);
        if look.terminal {
            y = terminal_month(ui, (left, y, width), shown, model, skin);
        } else {
            let at = (left, y, width);
            y = month(
                ui,
                (at, memory),
                shown,
                (model, column, clock),
                skin,
                &mut asked,
            );
        }
    }
    if look.terminal {
        if model.kind.has_time() && model.kind.has_date() {
            terminal_time(ui, (left, y, width), model, skin);
        }
        return asked;
    }
    if model.kind.has_time() {
        y = time_row(ui, (left, y, width), model, skin, &mut asked);
    }
    footer(ui, (left, y, width), model, skin, &mut asked);
    asked
}

/// The macOS month: its head, its weekdays and its six weeks. Returns
/// where it ends.
fn month(
    ui: &mut Ui,
    ((left, top, width), memory): ((f32, f32, f32), Id),
    shown: Date,
    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
    (look, palette, locale): (&Look, &Palette, Locale),
    asked: &mut Asked,
) -> f32 {
    let skin = (look, palette, locale);
    let right = left + width;
    widgets::hline(ui, egui::Rangef::new(left, right), top, palette.border);
    let middle = top + 4.0 + 24.0 / 2.0 + 4.0;
    let title = month_words(shown, locale);
    let laid = Text::one(look, TextRole::UiBodySemibold, &title, palette.text).layout(ui.ctx());
    laid.paint_left(ui.painter(), left + PAD, middle);
    // ‹ › and Today, at the right.
    let today_label = tr(locale, "Today");
    let today = widgets::ButtonSpec::new(&today_label)
        .link()
        .padding(6.0)
        .role(widgets::secondary(look));
    let today_width = today.width(ui, look);
    let today_rect = Rect::from_min_size(
        pos2(right - PAD - today_width, middle - 12.0),
        vec2(today_width, 24.0),
    );
    let mut go = None;
    let response = today.show_at(ui, today_rect, look, palette);
    asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
    if response.clicked() {
        go = Some(model.today.first_of_month());
    }
    let mut x = today_rect.left() - 6.0;
    for (by, glyph, name, last) in [
        (
            1,
            "›",
            tr(locale, "Next month"),
            tr(locale, "No date comes later"),
        ),
        (
            -1,
            "‹",
            tr(locale, "Previous month"),
            tr(locale, "No date comes earlier"),
        ),
    ] {
        let place = Rect::from_min_size(pos2(x - 24.0, middle - 12.0), vec2(24.0, 24.0));
        x -= 28.0;
        let target = moved(shown, by);
        let spec = widgets::ButtonSpec::new(glyph)
            .quiet()
            .label(&name)
            .padding(0.0);
        let spec = match target {
            Some(_) => spec,
            None => spec.disabled(&last),
        };
        let response = spec.show_at(ui, place, look, palette);
        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
        if response.clicked() {
            go = target.or(go);
        }
    }
    if let Some(month) = go {
        remember(ui, memory, model, month);
    }
    let shown = go.unwrap_or(shown);
    let mut y = top + HEAD;
    let column_width = (width - 16.0) / 7.0;
    let first = left + 8.0;
    for (at, day) in weekdays(model.week).into_iter().enumerate() {
        let center = pos2(first + column_width * (at as f32 + 0.5), y + WEEKDAYS / 2.0);
        Text::one(
            look,
            TextRole::FieldLabel,
            &initial(day, locale),
            palette.faint,
        )
        .layout(ui.ctx())
        .paint_center(ui.painter(), center);
    }
    y += WEEKDAYS;
    // The month is a grid of its days, each a button.
    let grid_id = Id::new("calendar-grid");
    ui.scope_builder(egui::UiBuilder::new().id_salt(grid_id), |ui| {
        ui.ctx().accesskit_node_builder(ui.unique_id(), |node| {
            node.set_role(Role::Grid);
            node.set_label(month_words(shown, locale));
        });
        for (at, day) in crate::dates::month_days(shown, model.week)
            .into_iter()
            .enumerate()
        {
            let (row, col) = (at / 7, at % 7);
            let cell = Rect::from_min_size(
                pos2(
                    first + column_width * col as f32,
                    y + (DAY + DAY_GAP) * row as f32,
                ),
                vec2(column_width, DAY),
            );
            let Some(day) = day else {
                continue;
            };
            if let Some(text) = day_cell(ui, cell, day, shown, (model, column, clock), skin, asked)
            {
                asked.text = Some(text);
            }
        }
    });
    y + 6.0 * DAY + 5.0 * DAY_GAP + 8.0
}

/// One day of the macOS month in `cell`: its number, its marks, and a
/// click that picks it. Returns the text a click rewrites the field to.
fn day_cell(
    ui: &mut Ui,
    cell: Rect,
    day: Date,
    shown: Date,
    (model, column, clock): (&Model, &Column, &crate::dates::Clock),
    (look, palette, locale): (&Look, &Palette, Locale),
    asked: &mut Asked,
) -> Option<String> {
    let holds = crate::dates::holds(day, model.keeps, model.offset, column, clock);
    let selected = model.selected == Some(day);
    let today = model.today == day;
    let was = model.was == Some(day) && !selected;
    let id = ui
        .id()
        .with(("calendar-day", day.year(), day.month(), day.day()));
    let sense = if holds {
        Sense::click()
    } else {
        Sense::hover()
    };
    let response = ui.interact(cell, id, sense);
    let name = day_name(day, locale);
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, holds, &name));
    let said: Vec<String> = [(today, "today"), (was, "current value")]
        .into_iter()
        .filter(|(on, _)| *on)
        .map(|(_, words)| tr(locale, words))
        .collect();
    ui.ctx().accesskit_node_builder(response.id, |node| {
        node.set_selected(selected);
        if !said.is_empty() {
            node.set_description(said.join(", "));
        }
    });
    let inside = day.year() == shown.year() && day.month() == shown.month();
    // The month dims while the text reads as no day; a day the type does
    // not hold, and another month's, are drawn as disabled.
    let fade = |color: egui::Color32| {
        if model.dim {
            color.gamma_multiply(0.45)
        } else {
            color
        }
    };
    let painter = ui.painter();
    let radius = CornerRadius::same(7);
    if selected && !model.dim {
        painter.rect_filled(cell.shrink2(vec2(1.0, 0.0)), radius, palette.accent);
    } else if response.hovered() && holds {
        painter.rect_filled(cell.shrink2(vec2(1.0, 0.0)), radius, palette.hover_fill());
    }
    if today && !(selected && !model.dim) {
        let outline = palette.window.lerp_to_gamma(palette.accent, 0.45);
        painter.rect_stroke(
            cell.shrink(3.0),
            CornerRadius::same(6),
            Stroke::new(1.0, fade(outline)),
            StrokeKind::Inside,
        );
    }
    let (role, color) = if selected && !model.dim {
        (TextRole::UiBodySemibold, palette.on_accent)
    } else if !holds || !inside {
        (TextRole::UiBody, palette.faint)
    } else if today {
        (TextRole::UiBodySemibold, palette.accent_hover)
    } else {
        (TextRole::UiBody, palette.text)
    };
    Text::one(look, role, &day.day().to_string(), fade(color))
        .layout(ui.ctx())
        .paint_center(ui.painter(), cell.center());
    if was {
        ui.painter().circle_filled(
            pos2(cell.center().x, cell.bottom() - 6.0),
            2.0,
            fade(palette.warning),
        );
    }
    asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
    (response.clicked() && holds)
        .then(|| crate::dates::picked(day, model.keeps, model.offset, column))
}

/// The macOS time row: the fields, then the zone chip and Now. Returns
/// where it ends.
fn time_row(
    ui: &mut Ui,
    (left, top, width): (f32, f32, f32),
    model: &Model,
    (look, palette, locale): (&Look, &Palette, Locale),
    asked: &mut Asked,
) -> f32 {
    widgets::hline(
        ui,
        egui::Rangef::new(left, left + width),
        top,
        palette.border,
    );
    let middle = top + TIME_ROW / 2.0;
    let mono = TextRole::GridCell;
    let mut x = left + PAD;
    for (at, part) in model.parts.iter().enumerate() {
        if at > 0 {
            let between = if part.fraction { "." } else { ":" };
            let laid = Text::one(look, mono, between, palette.faint).layout(ui.ctx());
            x += 3.0;
            x += laid.paint_left(ui.painter(), x, middle) + 3.0;
        }
        let text_width = mono.width(ui.ctx(), look.faces, &part.shown);
        let field = Rect::from_min_size(
            pos2(x, middle - 13.0),
            vec2((text_width + 8.0).max(30.0), 26.0),
        );
        x = field.right();
        // A spin button with its range: the keys that step it come with
        // the picker's keys.
        let id = ui.id().with(("calendar-time", at));
        let response = ui.interact(field, id, Sense::hover());
        let name = tr(locale, part.name);
        response
            .widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::DragValue, true, &name));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_min_numeric_value(0.0);
            node.set_max_numeric_value(part.most);
            if let Some(value) = part.value {
                node.set_numeric_value(value);
            }
            node.set_value(part.shown.clone());
        });
        ui.painter().rect_stroke(
            field,
            CornerRadius::same(6),
            Stroke::new(1.0, palette.border),
            StrokeKind::Inside,
        );
        Text::one(look, mono, &part.shown, palette.text)
            .layout(ui.ctx())
            .paint_center(ui.painter(), field.center());
    }
    let mut right = left + width - PAD;
    if let Some(now) = &model.now {
        let label = tr(locale, "Now");
        let spec = widgets::ButtonSpec::new(&label)
            .padding(8.0)
            .role(widgets::secondary(look));
        let place = Rect::from_min_size(
            pos2(right - spec.width(ui, look), middle - 12.0),
            vec2(spec.width(ui, look), 24.0),
        );
        let response = spec.show_at(ui, place, look, palette);
        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
        if response.clicked() {
            asked.text = Some(now.clone());
        }
        right = place.left() - 6.0;
    }
    if let Some(zone) = &model.zone {
        let words = zone.words(locale);
        let laid = Text::one(look, TextRole::Shortcut, &words, palette.secondary).layout(ui.ctx());
        let chip = Rect::from_min_size(
            pos2(right - laid.width() - 14.0, middle - 11.0),
            vec2(laid.width() + 14.0, 22.0),
        );
        if chip.left() > x + 6.0 {
            ui.painter()
                .rect_filled(chip, CornerRadius::same(11), palette.surface);
            laid.paint_center(ui.painter(), chip.center());
            widgets::announce(ui, chip, &words);
        }
    }
    top + TIME_ROW
}

/// The macOS footer: the keys that apply and cancel, or PostgreSQL's
/// infinities, and NULL and DEFAULT where the column takes them.
fn footer(
    ui: &mut Ui,
    (left, top, width): (f32, f32, f32),
    model: &Model,
    (look, palette, locale): (&Look, &Palette, Locale),
    asked: &mut Asked,
) {
    let rect = Rect::from_min_size(pos2(left, top), vec2(width, FOOTER));
    ui.painter().rect_filled(
        rect.shrink(1.0),
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: look.dialog_radius,
            se: look.dialog_radius,
        },
        palette.panel,
    );
    widgets::hline(ui, rect.x_range(), top, palette.border);
    let middle = rect.center().y;
    let mut button = |ui: &mut Ui, x: f32, text: &str, name: &str, right: bool| {
        let spec = widgets::ButtonSpec::new(text)
            .label(name)
            .padding(8.0)
            .role(widgets::secondary(look));
        let width = spec.width(ui, look);
        let left = if right { x - width } else { x };
        let place = Rect::from_min_size(pos2(left, middle - 12.0), vec2(width, 24.0));
        let response = spec.show_at(ui, place, look, palette);
        asked.pressed |= response.is_pointer_button_down_on() || response.clicked();
        (response.clicked(), place)
    };
    if model.infinity {
        // The keys' hints give way to the infinities, as the board has it.
        let mut x = left + PAD;
        for (text, written, name) in [
            ("−∞", "-infinity", tr(locale, "Minus infinity")),
            ("∞", "infinity", tr(locale, "Infinity")),
        ] {
            let (clicked, place) = button(ui, x, text, &name, false);
            if clicked {
                asked.text = Some(written.to_owned());
            }
            x = place.right() + 6.0;
        }
    } else {
        let apply = crate::ui::keys::written_in(
            ui.ctx(),
            look,
            crate::keymap::Command::CommitDown,
            crate::keymap::Scope::CellEditor,
        );
        let cancel = crate::ui::keys::written_in(
            ui.ctx(),
            look,
            crate::keymap::Command::CancelEdit,
            crate::keymap::Scope::CellEditor,
        );
        let (apply_words, cancel_words) = (tr(locale, "Apply"), tr(locale, "Cancel"));
        let hints = [
            (&apply, apply_words.as_str(), true),
            (&cancel, cancel_words.as_str(), true),
        ];
        widgets::key_hints(ui, (left + PAD, middle), &hints, 10.0, look, palette);
    }
    let mut x = left + width - PAD;
    for (offered, text) in [(model.default, "DEFAULT"), (model.null, "NULL")] {
        if !offered {
            continue;
        }
        let (clicked, place) = button(ui, x, text, text, true);
        if clicked {
            asked.text = Some(text.to_owned());
            asked.commit = true;
        }
        x = place.left() - 6.0;
    }
}

/// The terminal's month: its name and its six weeks, painted. Returns
/// where it ends.
fn terminal_month(
    ui: &Ui,
    (left, top, width): (f32, f32, f32),
    shown: Date,
    model: &Model,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> f32 {
    widgets::hline(
        ui,
        egui::Rangef::new(left, left + width),
        top,
        palette.border,
    );
    let role = TextRole::OBody;
    let mut y = top + TERMINAL_ROW / 2.0;
    Text::one(
        look,
        TextRole::OGroup,
        &look.label(&month_words(shown, locale)),
        palette.text,
    )
    .layout(ui.ctx())
    .paint_left(ui.painter(), left + PAD, y);
    y += TERMINAL_ROW;
    let fade = |color: egui::Color32| {
        if model.dim {
            color.gamma_multiply(0.45)
        } else {
            color
        }
    };
    for (at, day) in weekdays(model.week).into_iter().enumerate() {
        let center = pos2(left + PAD + TERMINAL_DAY * (at as f32 + 0.5), y);
        Text::one(
            look,
            role,
            &look.label(&initial(day, locale)),
            fade(palette.dim),
        )
        .layout(ui.ctx())
        .paint_center(ui.painter(), center);
    }
    y += TERMINAL_ROW / 2.0 + 2.0;
    for (at, day) in crate::dates::month_days(shown, model.week)
        .into_iter()
        .enumerate()
    {
        let Some(day) = day else {
            continue;
        };
        let (row, col) = (at / 7, at % 7);
        let cell = Rect::from_min_size(
            pos2(
                left + PAD + TERMINAL_DAY * col as f32,
                y + TERMINAL_ROW * row as f32,
            ),
            vec2(TERMINAL_DAY, TERMINAL_ROW),
        );
        let inside = day.year() == shown.year() && day.month() == shown.month();
        let selected = model.selected == Some(day) && !model.dim;
        let number = format!("{:>2}", day.day());
        let (role, color) = if selected {
            ui.painter()
                .rect_filled(cell, CornerRadius::ZERO, palette.accent);
            (TextRole::OGroup, palette.on_accent)
        } else if !inside {
            (role, palette.faint)
        } else if model.today == day {
            (role, palette.accent)
        } else {
            (role, palette.text)
        };
        let laid = Text::one(look, role, &number, fade(color)).layout(ui.ctx());
        laid.paint_center(ui.painter(), cell.center());
        // Today is underlined, as the terminal marks a link.
        if model.today == day && !selected {
            let under = cell.center().y + 8.0;
            let half = laid.width() / 2.0;
            ui.painter().hline(
                egui::Rangef::new(cell.center().x - half, cell.center().x + half),
                under,
                Stroke::new(1.0, fade(palette.accent)),
            );
        }
    }
    y + 6.0 * TERMINAL_ROW + 6.0
}

/// The terminal's time row of a timestamp: the time, and its zone.
fn terminal_time(
    ui: &Ui,
    (left, top, width): (f32, f32, f32),
    model: &Model,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    widgets::hline(
        ui,
        egui::Rangef::new(left, left + width),
        top,
        palette.border,
    );
    let role = TextRole::OBody;
    let middle = top + (TERMINAL_ROW + 4.0) / 2.0;
    let time: Vec<&str> = model.parts.iter().map(|part| part.shown.as_str()).collect();
    let mut time = time[..time.len().min(3)].join(":");
    if let Some(fraction) = model.parts.get(3) {
        time.push('.');
        time.push_str(&fraction.shown);
    }
    let zone = match (&model.abbreviation, &model.zone) {
        (Some(abbreviation), _) => abbreviation.clone(),
        (None, Some(zone)) => zone.words(locale),
        (None, None) => String::new(),
    };
    let mut x = left + PAD;
    for (text, color) in [
        (look.label(&tr(locale, "time")), palette.dim),
        (time, palette.text),
        (look.label(&zone), palette.dim),
    ] {
        let laid = Text::one(look, role, &text, color).layout(ui.ctx());
        x += laid.paint_left(ui.painter(), x, middle) + 10.0;
    }
}
