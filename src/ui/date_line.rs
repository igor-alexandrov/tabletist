//! The line under the editor of a date or time: what the text will be
//! stored as, or why it will not be. A popover under the cell, or under
//! the row panel's field, on macOS; a float under it in the terminal look,
//! headed by the column and its zone. It never takes the keyboard: the
//! field keeps it, and every choice it offers rewrites the field's text.

use egui::accesskit::{Live, Role};
use egui::{Id, Rect, Sense, Ui, pos2, vec2};
use jiff::civil::{Date, Time, Weekday};

use crate::dates::{Column, Kind, Moment, Range, Reading, Refusal, Stored};
use crate::i18n::{Locale, gettext};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::{sql_complete, widgets};

/// `text` in the person's language.
pub(super) fn tr(locale: Locale, text: &'static str) -> String {
    gettext(locale, text).into_owned()
}

/// How the line reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// What will be stored.
    Plain,
    /// Stored only once a choice is made (a change of the clocks), or a
    /// value the cell holds that nothing should be given.
    Warning,
    /// Not stored: the text is no value of the column.
    Danger,
}

/// A choice the line offers, as a button: what it says, the text it
/// rewrites the field to, and whether the value is that one now.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub label: String,
    pub text: String,
    pub chosen: bool,
}

/// The line's words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub tone: Tone,
    pub words: String,
    /// The literal a save sends, said after the words where it is not
    /// what was typed: "sent as `2026-01-15 09:30:00-05`".
    pub sent: Option<String>,
    pub offers: Vec<Offer>,
}

impl Line {
    /// All of it as one text: what a screen reader is told, and what a
    /// test reads.
    pub fn said(&self, look: &Look, locale: Locale) -> String {
        match &self.sent {
            Some(sent) => format!(
                "{} · {sent}",
                self.words,
                sent = sent_words(sent, look, locale)
            ),
            None => self.words.clone(),
        }
    }
}

/// "sent as" before `literal`, in the look's case: the literal as it is.
fn sent_words(literal: &str, look: &Look, locale: Locale) -> String {
    format!("{} {literal}", look.label(&tr(locale, "sent as")))
}

const WEEKDAYS: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const FULL_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

fn weekday_name(day: Weekday, locale: Locale) -> String {
    let at = usize::try_from(day.to_monday_zero_offset()).unwrap_or_default();
    tr(locale, WEEKDAYS[at % 7])
}

fn month_name(month: i8, locale: Locale) -> String {
    let at = usize::try_from(month - 1).unwrap_or_default();
    tr(locale, MONTHS[at % 12])
}

/// "January".
pub(super) fn full_month_name(month: i8, locale: Locale) -> String {
    let at = usize::try_from(month - 1).unwrap_or_default();
    tr(locale, FULL_MONTHS[at % 12])
}

/// The year as a person reads it: `2026`, and `44 BC` before 1 AD.
pub(super) fn year_words(year: i16, locale: Locale) -> String {
    if year < 1 {
        format!("{} {}", 1 - i32::from(year), tr(locale, "BC"))
    } else {
        year.to_string()
    }
}

/// "Thu 15 Jan 2026".
pub fn day_words(date: Date, locale: Locale) -> String {
    format!(
        "{} {} {} {}",
        weekday_name(date.weekday(), locale),
        date.day(),
        month_name(date.month(), locale),
        year_words(date.year(), locale)
    )
}

/// "15 Jan" or "8 Mar": a day of the year the line is about.
fn short_day(date: Date, locale: Locale) -> String {
    format!("{} {}", date.day(), month_name(date.month(), locale))
}

/// "09:30", with its seconds where they are not nothing (always for a
/// time of day, `seconds`), and its fraction to `digits` where it has one.
fn time_words(time: Time, digits: u8, seconds: bool) -> String {
    let mut out = format!("{:02}:{:02}", time.hour(), time.minute());
    let fraction = time.subsec_nanosecond() != 0 && digits > 0;
    if seconds || time.second() != 0 || fraction {
        out.push_str(&format!(":{:02}", time.second()));
    }
    if fraction {
        let nanos = format!("{:09}", time.subsec_nanosecond());
        out.push('.');
        out.push_str(&nanos[..usize::from(digits.min(9))]);
    }
    out
}

/// "today", "tomorrow", "in 6 days", "3 days ago".
fn days_words(days: i64, locale: Locale) -> String {
    match days {
        0 => tr(locale, "today"),
        1 => tr(locale, "tomorrow"),
        -1 => tr(locale, "yesterday"),
        days if days > 0 => tr(locale, "in {count} days").replace("{count}", &days.to_string()),
        days => tr(locale, "{count} days ago").replace("{count}", &days.abs().to_string()),
    }
}

/// What the type a column of `kind` keeps is called, in a sentence:
/// "date", "time", "date and time".
pub fn kind_words(kind: Kind, locale: Locale) -> String {
    match kind {
        Kind::Date => tr(locale, "date"),
        Kind::Time { .. } => tr(locale, "time"),
        Kind::Timestamp | Kind::Zoned => tr(locale, "date and time"),
    }
}

/// Why `refusal` keeps a text from its column, of the type `type_name`
/// (as the grid's header names it).
pub fn refusal_text(refusal: &Refusal, type_name: &str, locale: Locale) -> String {
    let say = |text: &'static str| tr(locale, text);
    match refusal {
        Refusal::NotRead(Kind::Date) => say("Not a date. Try 2026-01-15, 15 Jan 2026 or tomorrow."),
        Refusal::NotRead(Kind::Time { .. }) => say("Not a time. Try 9:30, 9am or 21:30."),
        Refusal::NotRead(Kind::Timestamp | Kind::Zoned) => {
            say("Not a date and time. Try 2026-01-15 9:30 or tomorrow 9am.")
        }
        Refusal::NoMonth(month) => {
            say("There is no month {month}.").replace("{month}", &month.to_string())
        }
        Refusal::NoDay { day, month } => say("There is no {day} {month}.")
            .replace("{day}", &day.to_string())
            .replace("{month}", &full_month_name(*month, locale)),
        Refusal::NoTime(typed) => say("{time} is not a time of day.").replace("{time}", typed),
        Refusal::Range(range) => match range {
            Range::PgDate => say("PostgreSQL date holds 4714-11-24 BC to 5874897 AD."),
            Range::PgTimestamp => say("PostgreSQL timestamp holds 4714-11-24 BC to 294276 AD."),
            Range::MySqlDate => say("MySQL DATE holds 1000-01-01 to 9999-12-31."),
            Range::MySqlDatetime => say("MySQL DATETIME holds 1000-01-01 to 9999-12-31."),
            Range::MySqlTimestamp => say("MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."),
            Range::MySqlTime => say("MySQL TIME holds -838:59:59 to 838:59:59."),
            Range::PgOffset => say("PostgreSQL keeps offsets from -15:59:59 to +15:59:59."),
        },
        Refusal::Years => say("Tabletist reads years up to 9999."),
        Refusal::YearZero => say("There is no year 0: 1 BC comes just before 1 AD."),
        Refusal::Gap { at, place, .. } => {
            let words = match place.is_empty() {
                true => say("{time} doesn't exist on {day} in this zone; clocks go forward."),
                false => say("{time} doesn't exist on {day} in {place}; clocks go forward."),
            };
            words
                .replace("{time}", &time_words(at.time(), 0, false))
                .replace("{day}", &short_day(at.date(), locale))
                .replace("{place}", place)
        }
        Refusal::Zero => say("MySQL zero date · pick a date or set NULL"),
        Refusal::NoZone => {
            say("{type} keeps no zone: leave the offset out.").replace("{type}", type_name)
        }
        Refusal::DayOnly => say("A date keeps no time of day."),
        Refusal::TimeOnly => say("A time keeps no day."),
        Refusal::ZoneUnknown => say(
            "The server's zone is not known: type the day and the time as the server keeps them, with no offset.",
        ),
    }
}

/// What a value read is said as: the day, its time and zone, how far it is
/// from today, and how it is kept.
fn reading_words(reading: &Reading, column: &Column, locale: Locale) -> String {
    let say = |text: &'static str| tr(locale, text);
    let (date, time, offset) = match &reading.moment {
        Moment::Infinity => return say("No end date · stored as infinity"),
        Moment::MinusInfinity => return say("No start date · stored as -infinity"),
        Moment::EndOfDay => {
            return format!("24:00:00 · {}", say("the end of the day"));
        }
        Moment::Span(written) => {
            return format!("{written} · {}", say("a span of time, not a time of day"));
        }
        Moment::At { date, time, offset } => (*date, *time, *offset),
    };
    let digits = column.digits;
    let mut head = match (date, time) {
        (Some(date), Some(time)) if column.kind.has_time() => {
            format!(
                "{}, {}",
                day_words(date, locale),
                time_words(time, digits, false)
            )
        }
        (Some(date), _) => day_words(date, locale),
        (None, Some(time)) => {
            let mut words = time_words(time, digits, true);
            if let Some(offset) = offset {
                words.push_str(&crate::dates::written_offset(offset));
            }
            words
        }
        (None, None) => String::new(),
    };
    if let Some(zone) = &reading.zone_name {
        head.push(' ');
        head.push_str(zone);
    }
    let mut parts = vec![head];
    if let Some(days) = reading
        .days
        .filter(|days| reading.relative || days.abs() <= 7)
    {
        parts.push(days_words(days, locale));
    }
    if column.kind == Kind::Timestamp && offset.is_none() {
        parts.push(say("no zone"));
    }
    let sqlite = match column.dialect {
        tabletist_db::Dialect::Sqlite => column.kind.has_date(),
        tabletist_db::Dialect::Postgres | tabletist_db::Dialect::MySql => false,
    };
    if sqlite {
        parts.push(match column.stored {
            Stored::Fresh => say("stored as text"),
            Stored::Text { .. } => say("stored as text, like the other rows"),
            Stored::Seconds => say("stored as seconds since 1970, like the other rows"),
            Stored::Millis => say("stored as milliseconds since 1970, like the other rows"),
            Stored::Days => say("stored as a Julian day, like the other rows"),
            Stored::Compact => say("stored as a number such as 20260115, like the other rows"),
            Stored::Unknown => {
                say("stored as text: the other rows hold numbers of a form not known here")
            }
        });
    }
    parts.join(" · ")
}

/// The line for the text `typed`, which reads as `moment`, in a column
/// `column` of the type `type_name` (as the grid's header names it).
/// `was` is the day the cell had before the edit, where the line names it
/// (the terminal look's, under its calendar): said where the text reads
/// as another.
pub fn line(
    typed: &str,
    moment: &Result<Reading, Refusal>,
    (column, type_name, was): (&Column, &str, Option<Date>),
    (look, locale): (&Look, Locale),
) -> Line {
    let plain = |tone: Tone, words: String| Line {
        tone,
        words: look.label(&words),
        sent: None,
        offers: Vec::new(),
    };
    if typed.trim().is_empty() {
        let hint = match column.kind {
            Kind::Date => tr(locale, "Type a date: 2026-01-15, 15 Jan 2026 or tomorrow."),
            Kind::Time { .. } => tr(locale, "Type a time: 9:30, 9am or 21:30."),
            Kind::Timestamp | Kind::Zoned => tr(
                locale,
                "Type a date and time: 2026-01-15 9:30 or tomorrow 9am.",
            ),
        };
        return plain(Tone::Plain, hint);
    }
    match moment {
        Ok(reading) => {
            let mut words = reading_words(reading, column, locale);
            let day = match &reading.moment {
                Moment::At { date, .. } => *date,
                Moment::Infinity | Moment::MinusInfinity | Moment::Span(_) | Moment::EndOfDay => {
                    None
                }
            };
            if let Some(was) = was.filter(|was| Some(*was) != day) {
                let same_year = day.is_some_and(|day| day.year() == was.year());
                let said = if same_year {
                    short_day(was, locale)
                } else {
                    format!(
                        "{} {}",
                        short_day(was, locale),
                        year_words(was.year(), locale)
                    )
                };
                words.push_str(&format!(
                    " · {}",
                    tr(locale, "was {day}").replace("{day}", &said)
                ));
            }
            let sent = (reading.literal != typed.trim()).then(|| reading.literal.clone());
            let Some(fold) = &reading.fold else {
                return Line {
                    tone: Tone::Plain,
                    words: look.label(&words),
                    sent,
                    offers: Vec::new(),
                };
            };
            let words = match fold.place.is_empty() {
                true => tr(
                    locale,
                    "{time} happens twice on {day} in this zone; clocks go back.",
                ),
                false => tr(
                    locale,
                    "{time} happens twice on {day} in {place}; clocks go back.",
                ),
            }
            .replace("{time}", &time_words(fold.at.time(), 0, false))
            .replace("{day}", &short_day(fold.at.date(), locale))
            .replace("{place}", &fold.place);
            let chosen = match &reading.moment {
                Moment::At { offset, .. } => *offset,
                Moment::Infinity | Moment::MinusInfinity | Moment::Span(_) | Moment::EndOfDay => {
                    None
                }
            };
            let offers = [&fold.earlier, &fold.later]
                .into_iter()
                .map(|(offset, name)| Offer {
                    label: look.label(&format!("{} {name}", time_words(fold.at.time(), 0, false))),
                    text: crate::dates::rewritten(fold.at, Some(*offset), column),
                    chosen: chosen == Some(*offset),
                })
                .collect();
            Line {
                tone: Tone::Warning,
                words: look.label(&words),
                sent,
                offers,
            }
        }
        Err(Refusal::Gap { offered, .. }) => {
            let refusal = moment.as_ref().err();
            let words = refusal.map(|refusal| refusal_text(refusal, type_name, locale));
            let time = time_words(offered.time(), 0, false);
            Line {
                tone: Tone::Warning,
                words: look.label(&words.unwrap_or_default()),
                sent: None,
                offers: vec![Offer {
                    label: look.label(&tr(locale, "Use {time}").replace("{time}", &time)),
                    text: crate::dates::rewritten(*offered, None, column),
                    chosen: false,
                }],
            }
        }
        Err(Refusal::Zero) => plain(
            Tone::Warning,
            refusal_text(&Refusal::Zero, type_name, locale),
        ),
        Err(refusal) => {
            let words = refusal_text(refusal, type_name, locale);
            // The terminal marks what is refused, as its status line does.
            let words = if look.terminal {
                format!("! {words}")
            } else {
                words
            };
            plain(Tone::Danger, words)
        }
    }
}

/// What the pointer did to the line this frame.
#[derive(Default)]
pub struct Shown {
    /// The line was pressed or clicked: the press landed outside the
    /// field, which keeps the keyboard.
    pub pressed: bool,
    /// The text an offer rewrites the field to, once clicked.
    pub picked: Option<String>,
    /// The pick ends the edit: the calendar's NULL or DEFAULT.
    pub commit: bool,
}

/// The widths of the macOS popover and of the terminal's float.
const WIDTH: f32 = 330.0;
const TERMINAL_WIDTH: f32 = 420.0;
/// The room inside the frame.
const PAD: egui::Vec2 = vec2(10.0, 8.0);
/// The gap between the cell and the line.
const GAP: f32 = 4.0;

/// The calendar under the line, and what it is read at.
pub struct Sheet<'a> {
    pub model: &'a crate::ui::calendar::Model,
    /// What its month is kept by: the field and the cell it edits.
    pub memory: Id,
    pub column: &'a Column,
    pub clock: &'a crate::dates::Clock,
}

/// The float's head in the terminal look: the column and its type, and the
/// zone a zoned value is in.
pub struct Head<'a> {
    pub column: &'a str,
    pub type_name: &'a str,
    pub zone: Option<String>,
}

/// Draws `line` under `anchor`, the cell or field the editor `field` is
/// on (over it where the window has no room below), named after the
/// column as a dialog is. `head` heads the terminal look's float.
pub fn show(
    ui: &Ui,
    anchor: Rect,
    field: Id,
    (line, head, kind): (&Line, &Head<'_>, Kind),
    sheet: Option<Sheet<'_>>,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Shown {
    let mut shown = Shown::default();
    let screen = ui.ctx().content_rect();
    // The macOS calendar is as wide as its board draws it.
    let width = match (&sheet, look.terminal) {
        (Some(_), false) => WIDTH,
        (_, true) => TERMINAL_WIDTH.max(anchor.width().min(WIDTH * 1.5)),
        (None, false) => WIDTH.max(anchor.width().min(WIDTH * 1.5)),
    }
    .min(screen.width() - 16.0);
    let inner = width - 2.0 * PAD.x;
    let small = widgets::secondary(look);
    let color = match line.tone {
        Tone::Plain => palette.secondary,
        Tone::Warning => palette.warning,
        Tone::Danger => palette.danger,
    };
    let mut words = Text::one(look, small, &line.words, color);
    if let Some(sent) = &line.sent {
        let mono = TextRole::pick(look, TextRole::MonoSecondary, TextRole::OSecondary);
        words = words
            .add(
                small,
                &format!(" · {} ", look.label(&tr(locale, "sent as"))),
                color,
            )
            .add(mono, sent, color);
    }
    let laid = words.wrap(inner).layout(ui.ctx());
    let head_laid = look.terminal.then(|| {
        let name = format!("{} · {}", head.column, head.type_name);
        Text::one(look, small, &name, palette.dim).layout(ui.ctx())
    });
    let zone_laid = head
        .zone
        .as_ref()
        .filter(|_| look.terminal)
        .map(|zone| Text::one(look, small, &zone.to_lowercase(), palette.dim).layout(ui.ctx()));
    let head_height = head_laid
        .as_ref()
        .map_or(0.0, |laid| laid.height() + 2.0 * PAD.y);
    let offers_height = if line.offers.is_empty() {
        0.0
    } else {
        look.control_height + PAD.y
    };
    let calendar_height = sheet
        .as_ref()
        .map_or(0.0, |sheet| crate::ui::calendar::height(sheet.model, look));
    let height = head_height + laid.height() + 2.0 * PAD.y + offers_height + calendar_height;
    let below = anchor.bottom() + GAP;
    let top = if below + height > screen.bottom() && anchor.top() - GAP - height >= screen.top() {
        anchor.top() - GAP - height
    } else {
        below
    };
    let left = anchor.left().min(screen.right() - width).max(screen.left());
    let rect = Rect::from_min_size(pos2(left, top), vec2(width, height));
    // One area for the lines of every editor: only one editor is open.
    let id = Id::new("date-line");
    egui::Area::new(id)
        .order(egui::Order::Foreground)
        .fade_in(false)
        .constrain(false)
        .fixed_pos(rect.min)
        .show(ui.ctx(), |ui| {
            let (rect, _) = ui.allocate_exact_size(rect.size(), Sense::hover());
            let whole = ui.interact(rect, id.with("whole"), Sense::CLICK);
            let name = format!("{}, {}", head.column, kind_words(kind, locale));
            whole.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &name));
            ui.ctx().accesskit_node_builder(whole.id, |node| {
                node.set_role(Role::Dialog);
            });
            // The dialog is the field's: it describes what is typed there.
            ui.ctx().accesskit_node_builder(field, |node| {
                node.push_controlled(whole.id.accesskit_id());
            });
            let clicked_over =
                ui.input(|input| input.pointer.any_click()) && ui.rect_contains_pointer(rect);
            shown.pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over;
            let corner = if look.terminal { 0 } else { look.dialog_radius };
            let border = sql_complete::panel(ui, rect, corner, look, palette);
            let mut y = rect.top();
            if let Some(head_laid) = head_laid {
                let center = y + head_height / 2.0;
                head_laid.paint_left(ui.painter(), rect.left() + PAD.x, center);
                if let Some(zone) = zone_laid {
                    zone.paint_right(ui.painter(), rect.right() - PAD.x, center);
                }
                y += head_height;
                ui.painter()
                    .hline(rect.x_range(), y, egui::Stroke::new(1.0, palette.border));
            }
            let words_rect = Rect::from_min_size(
                pos2(rect.left() + PAD.x, y + PAD.y),
                vec2(inner, laid.height()),
            );
            let said = line.said(look, locale);
            // A live region: a screen reader reads the line out as it
            // changes, at once where the text is refused.
            // One node whatever it says: a screen reader reads a region
            // out as it changes, not one that is made again.
            let live = ui.interact(words_rect, id.with("said"), Sense::hover());
            live.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
            ui.ctx().accesskit_node_builder(live.id, |node| {
                node.set_live(match line.tone {
                    Tone::Danger => Live::Assertive,
                    Tone::Plain | Tone::Warning => Live::Polite,
                });
            });
            laid.paint_left(ui.painter(), words_rect.left(), words_rect.center().y);
            y = words_rect.bottom() + PAD.y;
            let mut x = rect.left() + PAD.x;
            for (at, offer) in line.offers.iter().enumerate() {
                let spec = widgets::ButtonSpec::new(&offer.label)
                    .padding(8.0)
                    .role(small);
                // The one the value is now stands out as the default does.
                let spec = if offer.chosen { spec.primary() } else { spec };
                let button = Rect::from_min_size(
                    pos2(x, y),
                    vec2(spec.width(ui, look), look.control_height),
                );
                let mut child = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(button)
                        .id_salt(("offer", at)),
                );
                let response = spec.show_at(&mut child, button, look, palette);
                if response.clicked() {
                    shown.picked = Some(offer.text.clone());
                }
                shown.pressed |= response.is_pointer_button_down_on() || response.clicked();
                x = button.right() + 6.0;
            }
            if let Some(sheet) = &sheet {
                let top = pos2(rect.left(), rect.bottom() - calendar_height);
                let asked = crate::ui::calendar::show(
                    ui,
                    (top, rect.width()),
                    sheet.memory,
                    (sheet.model, sheet.column, sheet.clock),
                    (look, palette, locale),
                );
                shown.pressed |= asked.pressed;
                if let Some(text) = asked.text {
                    shown.picked = Some(text);
                    shown.commit = asked.commit;
                }
            }
            ui.painter().add(border);
        });
    shown
}

/// The line of `editor`, where its column keeps dates or times and no
/// foreign key's list hangs from it, under `anchor`. A press on it is the
/// editor's: the field `target.id` keeps the keyboard, and the frame does
/// not read as an edit that was left. An offer clicked rewrites the text,
/// as typing would.
pub fn under(
    ui: &Ui,
    anchor: Rect,
    editor: &mut crate::edit::Editor,
    target: &crate::ui::cell_editor::Target,
    outcome: &mut crate::ui::cell_editor::Outcome,
    skin: (&Look, &Palette, Locale),
) {
    let (Some(column), Some(moment), None) = (&target.dates, &editor.moment, &editor.picker) else {
        return;
    };
    let (look, _, locale) = skin;
    let model = target.calendar.as_ref().map(|calendar| {
        let offers = (target.nullable, target.default);
        let was = editor.was.as_ref();
        crate::ui::calendar::model((&editor.text, moment, was), column, calendar, offers)
    });
    // The terminal look names the day before the edit on its line, where
    // macOS marks it in the month.
    let was = model
        .as_ref()
        .filter(|_| look.terminal)
        .and_then(|model| model.was);
    let said = line(
        &editor.text,
        moment,
        (column, &target.type_name, was),
        (look, locale),
    );
    let zone = column
        .zone
        .as_ref()
        .and_then(|zone| zone.iana_name().map(str::to_owned));
    let head = Head {
        column: &target.name,
        type_name: &target.type_name,
        zone,
    };
    let sheet = model
        .as_ref()
        .zip(target.calendar.as_ref())
        .map(|(model, calendar)| Sheet {
            model,
            memory: target.id.with((editor.cell.row, editor.cell.col)),
            column,
            clock: &calendar.clock,
        });
    let shown = show(
        ui,
        anchor,
        target.id,
        (&said, &head, column.kind),
        sheet,
        skin,
    );
    if shown.pressed {
        ui.memory_mut(|memory| memory.request_focus(target.id));
        outcome.left = false;
        outcome.kept = false;
    }
    if let Some(text) = shown.picked {
        editor.text = text;
        outcome.changed = true;
        if shown.commit {
            outcome.commit = Some(crate::model::Advance::Stay);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dates::{Clock, Order, read};
    use jiff::civil::date;
    use jiff::tz::TimeZone;
    use tabletist_db::Dialect;

    fn new_york() -> TimeZone {
        TimeZone::get("America/New_York").unwrap()
    }

    fn clock() -> Clock {
        let now = date(2026, 10, 24)
            .at(10, 0, 0, 0)
            .to_zoned(new_york())
            .unwrap()
            .timestamp();
        Clock {
            now,
            zone: new_york(),
            order: Order::DayMonth,
        }
    }

    fn line_of(dialect: Dialect, type_name: &str, typed: &str, look: &Look) -> Line {
        let column = Column::of(dialect, type_name, Some(&new_york()), []).unwrap();
        let moment = read(typed, &column, &clock());
        line(
            typed,
            &moment,
            (&column, type_name, None),
            (look, Locale::English),
        )
    }

    fn said(dialect: Dialect, type_name: &str, typed: &str) -> String {
        let look = Look::macos();
        line_of(dialect, type_name, typed, &look).said(&look, Locale::English)
    }

    #[test]
    fn the_line_says_what_is_stored_as_the_boards_word_it() {
        let pg = Dialect::Postgres;
        assert_eq!(
            said(pg, "timestamptz", "2026-01-15 09:30"),
            "Thu 15 Jan 2026, 09:30 EST · sent as 2026-01-15 09:30:00-05"
        );
        assert_eq!(said(pg, "date", "2026-01-15"), "Thu 15 Jan 2026");
        assert_eq!(
            said(pg, "date", "2026-10-30"),
            "Fri 30 Oct 2026 · in 6 days"
        );
        assert_eq!(
            said(pg, "date", "+6d"),
            "Fri 30 Oct 2026 · in 6 days · sent as 2026-10-30"
        );
        assert_eq!(said(pg, "time", "9:30"), "09:30:00 · sent as 09:30:00");
        assert_eq!(
            said(pg, "timestamp(6)", "2026-01-15 09:30:12.250"),
            "Thu 15 Jan 2026, 09:30:12.250000 · no zone"
        );
        assert_eq!(
            said(pg, "date", "infinity"),
            "No end date · stored as infinity"
        );
        assert_eq!(
            said(Dialect::MySql, "time", "100:30:00"),
            "100:30:00 · a span of time, not a time of day"
        );
        assert_eq!(said(pg, "date", "0044-03-15 BC"), "Fri 15 Mar 44 BC");
        assert_eq!(
            said(Dialect::Sqlite, "DATE", "2026-10-24"),
            "Sat 24 Oct 2026 · today · stored as text"
        );
    }

    #[test]
    fn a_refused_text_says_why() {
        let pg = Dialect::Postgres;
        let refused = line_of(pg, "date", "2026-02-30", &Look::macos());
        assert_eq!(refused.tone, Tone::Danger);
        assert_eq!(refused.words, "There is no 30 February.");
        assert_eq!(said(pg, "time", "25:00"), "25:00 is not a time of day.");
        assert_eq!(
            said(Dialect::MySql, "timestamp", "2040-05-01 08:00"),
            "MySQL TIMESTAMP holds 1970-01-01 to 2038-01-19 UTC."
        );
        let terminal = line_of(pg, "date", "2026-02-30", &Look::omarchy());
        assert_eq!(terminal.words, "! there is no 30 february.");
    }

    #[test]
    fn a_change_of_the_clocks_offers_its_way_out() {
        let pg = Dialect::Postgres;
        let gap = line_of(pg, "timestamptz", "2026-03-08 02:30", &Look::macos());
        assert_eq!(gap.tone, Tone::Warning);
        assert_eq!(
            gap.words,
            "02:30 doesn't exist on 8 Mar in New York; clocks go forward."
        );
        assert_eq!(
            gap.offers,
            vec![Offer {
                label: "Use 03:30".into(),
                text: "2026-03-08 03:30".into(),
                chosen: false
            }]
        );
        let fold = line_of(pg, "timestamptz", "2026-11-01 01:30", &Look::macos());
        assert_eq!(
            fold.words,
            "01:30 happens twice on 1 Nov in New York; clocks go back."
        );
        let labels: Vec<_> = fold
            .offers
            .iter()
            .map(|offer| (offer.label.as_str(), offer.chosen))
            .collect();
        assert_eq!(labels, [("01:30 EDT", true), ("01:30 EST", false)]);
        assert_eq!(fold.offers[1].text, "2026-11-01 01:30-05");
        // The terminal look's offers are in its case, as its words are.
        let terminal = line_of(pg, "timestamptz", "2026-11-01 01:30", &Look::omarchy());
        let labels: Vec<_> = terminal
            .offers
            .iter()
            .map(|offer| offer.label.as_str())
            .collect();
        assert_eq!(labels, ["01:30 edt", "01:30 est"]);
    }
}
