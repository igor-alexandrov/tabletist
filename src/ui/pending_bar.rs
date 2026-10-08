//! The bar above a table's footer while its tab holds pending changes: how
//! many, how many are to fix, what the last save came to, and Review SQL,
//! Discard all and Save. The words of a save's end are here too, for the
//! terminal's lines.

use egui::{Frame, Id, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::Error;

use crate::app::App;
use crate::edit::{Note, Saved};
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{Action, ConnTabId, ObjectTab, SaveBlock, TabId};
use crate::theme::Icon;
use crate::typography::{Text, TextRole};
use crate::ui::format;
use crate::ui::menu;
use crate::ui::states::{self, Tone};
use crate::ui::widgets::{self, ButtonSpec};

/// The bar's height.
const HEIGHT: f32 = 48.0;

/// Left and right padding, as the footer under it has.
const SIDE: f32 = 20.0;

/// The dot that leads the bar.
const DOT: f32 = 8.0;

/// The mark before what is to fix, and the ring of a save that runs.
const MARK: f32 = 13.0;
const RING: f32 = 14.0;

/// Room between two buttons, and after each thing the bar says.
const BETWEEN: f32 = 8.0;
const APART: f32 = 14.0;

/// What the last save came to keeps this much room, or its own width,
/// before the counts take theirs.
const NOTE_LEAST: f32 = 120.0;

/// `count` with its noun: "1 change", "3 changes".
pub fn counted(locale: Locale, count: usize, one: &'static str, many: &'static str) -> String {
    let plural = u32::try_from(count).unwrap_or(u32::MAX);
    format!("{count} {}", ngettext(locale, one, many, plural))
}

/// "1 new row · 3 changes in 2 rows", each part where there is any. Short,
/// the changes go without their rows.
fn counts_text(counts: crate::edit::Counts, short: bool, locale: Locale) -> String {
    let mut parts = Vec::new();
    if counts.added > 0 {
        parts.push(counted(locale, counts.added, "new row", "new rows"));
    }
    if counts.changes > 0 {
        let changes = counted(locale, counts.changes, "change", "changes");
        parts.push(if short {
            changes
        } else {
            format!(
                "{changes} {} {}",
                gettext(locale, "in"),
                counted(locale, counts.rows, "row", "rows")
            )
        });
    }
    parts.join(" · ")
}

/// `text` as a sentence: with one full stop at its end.
fn sentence(text: &str) -> String {
    format!("{}.", text.trim_end().trim_end_matches('.'))
}

/// The page's row `row` by its key, as the row panel's title names a row:
/// `id 2`, several columns joined by a comma. By its number where the key
/// is not known.
pub(crate) fn row_name(object: &ObjectTab, row: usize) -> String {
    let named = object.page().and_then(|page| {
        let key = object.structure.value.as_ref()?.row_key()?;
        let parts = super::row_panel::key_parts(&page.columns, page.rows.get(row)?, &key)?;
        let parts: Vec<String> = parts
            .into_iter()
            .map(|(column, value)| format!("{column} {value}"))
            .collect();
        Some(parts.join(", "))
    });
    named.unwrap_or_else(|| (object.query.offset + row as u64 + 1).to_string())
}

/// What a save that wrote nothing came to, as the user reads it.
pub fn note_text(note: &Note, object: &ObjectTab, locale: Locale) -> String {
    note_said(note, object, locale, str::to_owned)
}

/// [`note_text`] as the terminal's line says it: the app's own words in the
/// look's lower case, and the row's key and the database's words as they
/// are.
pub fn note_line(
    note: &Note,
    object: &ObjectTab,
    look: &crate::theme::Look,
    locale: Locale,
) -> String {
    note_said(note, object, locale, |words| look.label(words))
}

/// The database's own words about a statement that failed, after its code
/// when it gave one.
fn database_said(error: &Error) -> String {
    match error {
        Error::Query {
            code: Some(code),
            message,
            ..
        } => format!("{code} · {}", sentence(&format::capped(message))),
        other => sentence(&format::capped(&other.to_string())),
    }
}

/// What `note` says, the app's own words put through `own`: the values of
/// a row's key and what the database said are theirs, and stay as they are.
fn note_said(
    note: &Note,
    object: &ObjectTab,
    locale: Locale,
    own: impl Fn(&str) -> String,
) -> String {
    let say = |text: &'static str| own(&gettext(locale, text));
    let nothing = say("Nothing was written.");
    match note {
        Note::Conflict { row, gone, others } => {
            let what = if *gone {
                say("no longer exists on the server.")
            } else {
                say("changed on the server.")
            };
            let mut text = format!("{} {} {what} {nothing}", say("Row"), row_name(object, *row));
            if *others > 0 {
                let plural = u32::try_from(*others).unwrap_or(u32::MAX);
                let more = own(&ngettext(locale, "more row too.", "more rows too.", plural));
                text.push_str(&format!(" {others} {more}"));
            }
            text
        }
        Note::Failed { error, .. } => format!("{} {nothing}", database_said(error)),
        // No row of the page to name: the new row says it is the one.
        Note::FailedInsert { error } => format!(
            "{} {}",
            say("Nothing was saved. 1 new row failed, so the whole transaction rolled back."),
            database_said(error)
        ),
        Note::Lost => say("The connection was lost while saving. Reload to see what was written."),
        Note::NotSent => say("Not connected. Nothing was sent."),
        Note::Cancelled => format!("{} {nothing}", say("Save cancelled.")),
        Note::Refused(error) => format!(
            "{} {nothing}",
            sentence(&format::capped(&error.to_string()))
        ),
    }
}

/// What a save that wrote says once it is done: `written 2 changes · 1 row
/// · 14 ms`.
pub fn written_text(saved: &Saved, locale: Locale) -> String {
    let mut parts = Vec::new();
    if saved.added > 0 {
        parts.push(counted(locale, saved.added, "new row", "new rows"));
    }
    // A save of new rows alone changed none.
    if saved.changes > 0 || saved.added == 0 {
        parts.push(counted(locale, saved.changes, "change", "changes"));
        parts.push(counted(locale, saved.rows, "row", "rows"));
    }
    parts.push(format::elapsed(saved.elapsed));
    format!("{} {}", gettext(locale, "written"), parts.join(" · "))
}

/// Why Save cannot be pressed, as its tooltip says it, and the terminal's
/// status line.
pub(crate) fn block_text(block: SaveBlock, to_fix: usize, locale: Locale) -> String {
    match block {
        SaveBlock::Saving => gettext(locale, "Saving…").into_owned(),
        SaveBlock::ToFix => {
            let plural = u32::try_from(to_fix).unwrap_or(u32::MAX);
            format!(
                "{} {to_fix} {}",
                gettext(locale, "Fix"),
                ngettext(locale, "value to save", "values to save", plural)
            )
        }
        SaveBlock::Required => gettext(locale, "Fill required fields to save").into_owned(),
        SaveBlock::Disconnected => gettext(locale, "Not connected").into_owned(),
        SaveBlock::ReadOnly => gettext(locale, "This connection opens read-only").into_owned(),
        SaveBlock::Unsendable => gettext(
            locale,
            "These changes cannot be sent: the table's key is not known",
        )
        .into_owned(),
    }
}

/// How many of its buttons the bar shows as they are, in the room it has.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Buttons {
    /// Each one, Save with its key.
    Whole,
    /// Each one, Save without its key.
    Plain,
    /// Save, and the others in the menu under "…".
    Folded,
}

/// The bar of the table tab `id`, while it has something to say. A bottom
/// panel: called after the footer, it stands above it.
///
/// It is as wide as the grid, which a small window and the row panel leave
/// little of. Save's key goes first, then what the bar says, and the
/// buttons beside Save fold into a menu once they have no room themselves.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let object = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
        .filter(|object| {
            let edits = &object.edits;
            edits.pending() || edits.saving.is_some() || edits.note.is_some()
        });
    let Some(object) = object else {
        // A panel takes one of its parent's ids. Passed over while there is
        // no bar, so what is drawn after it is the same widget with the bar
        // and without: a field that has the keyboard when the first change
        // is made keeps it.
        ui.skip_ahead_auto_ids(1);
        return;
    };
    let edits = &object.edits;
    let saving = edits.saving.is_some();
    let pending = edits.pending();
    let reviewing = edits.reviewing;
    let counts = edits.counts();
    let note = edits
        .note
        .as_ref()
        .map(|note| note_text(note, object, locale));
    let blocked = app
        .save_blocked(tab, id)
        .map(|block| block_text(block, counts.to_fix, locale));
    // What the tab's new rows still need: the column, or how many.
    let lacking = app.lacking(tab, id);
    let needs = match lacking.as_slice() {
        [] => None,
        [one] => Some(format!("{one} {}", gettext(locale, "is required"))),
        more => Some(format!(
            "{} {}",
            more.len(),
            gettext(locale, "values are required")
        )),
    };
    let mut actions = Vec::new();
    egui::Panel::bottom(Id::new(("pending-bar", tab.0, id.0)))
        .exact_size(HEIGHT)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(Tone::Warning.fill(&look, &palette)))
        .show(ui, |ui| {
            let full = ui.max_rect();
            widgets::hline(
                ui,
                full.x_range(),
                full.top() + 0.5,
                Tone::Warning.line(&look, &palette),
            );
            let y = full.top() + 1.0 + (HEIGHT - 1.0) / 2.0;
            let height = states::button_height(&look);
            let (save, discard, dismiss, cancel, busy) = (
                gettext(locale, "Save"),
                gettext(locale, "Discard all"),
                gettext(locale, "Dismiss"),
                gettext(locale, "Cancel"),
                gettext(locale, "Saving…"),
            );
            let (review, hide) = (gettext(locale, "Review SQL"), gettext(locale, "Hide SQL"));
            // Told apart from a dialog's Cancel by a screen reader.
            let cancel_save = gettext(locale, "Cancel save");
            let more = gettext(locale, "More actions");
            let keys = format!("{}S", look.command_key());
            let body = widgets::body(&look);
            let ctx = ui.ctx().clone();
            let width = move |role: TextRole, text: &str| role.width(&ctx, look.faces, text);

            // What the bar says, and the room each part of it asks for.
            let whole = pending.then(|| counts_text(counts, false, locale));
            let short = pending.then(|| counts_text(counts, true, locale));
            // What stands against a save, in one place: the values to fix,
            // and what a new row still needs.
            let fix = (counts.to_fix > 0)
                .then(|| format!("{} {}", counts.to_fix, gettext(locale, "to fix")));
            let fix = match (fix, &needs) {
                (Some(fix), Some(needs)) => Some(format!("{fix} · {needs}")),
                (fix, needs) => fix.or_else(|| needs.clone()),
            };
            let fix_width = fix.as_ref().map(|fix| MARK + 5.0 + width(body, fix));
            let busy_width = saving.then(|| RING + 6.0 + width(body, &busy));
            let note_width = note.as_ref().map(|note| width(body, note));
            let note_least = note_width.map(|width| width.min(NOTE_LEAST));
            // All of it, whole: Save keeps its key only beside that.
            let said_width = {
                let counts = whole
                    .as_ref()
                    .map(|whole| width(TextRole::UiBodyStrong, whole));
                let parts = [counts, fix_width, busy_width, note_width];
                let gaps = parts.iter().flatten().count().saturating_sub(1);
                parts.iter().flatten().sum::<f32>() + APART * gaps as f32
            };

            let save_button = |keyed: bool| {
                let button = ButtonSpec::new(&save).primary();
                if keyed {
                    button.shortcut(&keys)
                } else {
                    button
                }
            };
            // One button under two names, so the keyboard stays on it.
            let review_name = if reviewing { &hide } else { &review };
            let review_button = || ButtonSpec::new(review_name).quiet().keyed("review-sql");
            let cancel_button = || ButtonSpec::new(&cancel).label(&cancel_save).quiet();
            let room = full.width() - 2.0 * SIDE;
            let lead = DOT + 10.0;
            let buttons = {
                let mut beside = BETWEEN + ButtonSpec::new(&discard).width(ui, &look);
                if pending {
                    beside += BETWEEN + review_button().width(ui, &look);
                }
                if saving {
                    beside += BETWEEN + cancel_button().width(ui, &look);
                }
                let keyed = save_button(true).width(ui, &look) + beside;
                let plain = save_button(false).width(ui, &look) + beside;
                if !(pending || saving) || lead + said_width + 2.0 * BETWEEN + keyed <= room {
                    Buttons::Whole
                } else if lead + plain <= room {
                    Buttons::Plain
                } else {
                    Buttons::Folded
                }
            };

            // The buttons first, from the right: what the bar says is cut
            // where they begin.
            let mut right = full.right() - SIDE;
            let mut place = |ui: &egui::Ui, button: &ButtonSpec<'_>| {
                let width = button.width(ui, &look);
                let rect =
                    Rect::from_min_size(pos2(right - width, y - height / 2.0), vec2(width, height));
                right -= width + BETWEEN;
                rect
            };
            if pending || saving {
                let button = save_button(buttons == Buttons::Whole);
                let at = place(ui, &button);
                let button = match &blocked {
                    Some(reason) => button.disabled(reason),
                    None => button,
                };
                if button.show_at(ui, at, &look, &palette).clicked() {
                    actions.push(Action::WriteEdits { tab, id });
                }
            } else {
                // Only what the last save came to is left: the button takes
                // the line away and nothing else. An editor may be open,
                // and what is typed in it is not the line's to drop.
                let button = ButtonSpec::new(&dismiss);
                let at = place(ui, &button);
                if button.show_at(ui, at, &look, &palette).clicked() {
                    actions.push(Action::DismissNote { tab, id });
                }
            }
            let show = !reviewing;
            if buttons == Buttons::Folded {
                // The same three, by their names, under one button.
                let row = |text: &str, disabled: Option<&str>| menu::Choice {
                    text: text.to_owned(),
                    name: None,
                    selected: false,
                    disabled: disabled.map(str::to_owned),
                };
                let mut rows = Vec::new();
                if pending {
                    let action = Action::ReviewEdits { tab, id, show };
                    rows.push((row(review_name, None), action));
                }
                // The reducer ignores it under a save: it is not offered.
                let unoffered = saving.then_some(&*busy);
                rows.push((row(&discard, unoffered), Action::DiscardEdits { tab, id }));
                if saving {
                    rows.push((row(&cancel_save, None), Action::CancelQuery(tab)));
                }
                let (rows, mut then): (Vec<_>, Vec<_>) = rows.into_iter().unzip();
                let button = ButtonSpec::new("")
                    .icon(Icon::Ellipsis)
                    .gap(0.0)
                    .padding(8.0)
                    .label(&more);
                let at = place(ui, &button);
                let response = button.show_at(ui, at, &look, &palette);
                if let Some(picked) = menu::actions(&response, 0.0, &look, &palette, || rows) {
                    actions.push(then.swap_remove(picked));
                }
            } else if pending || saving {
                // The reducer ignores it under a save: it is not offered.
                let button = ButtonSpec::new(&discard);
                let at = place(ui, &button);
                let button = if saving {
                    button.disabled(&busy)
                } else {
                    button
                };
                if button.show_at(ui, at, &look, &palette).clicked() {
                    actions.push(Action::DiscardEdits { tab, id });
                }
                // What a save would run, to read before it. Under a save
                // too: the drawer then shows what was sent.
                if pending {
                    let button = review_button();
                    let at = place(ui, &button);
                    if button.show_at(ui, at, &look, &palette).clicked() {
                        actions.push(Action::ReviewEdits { tab, id, show });
                    }
                }
                if saving {
                    // A save is one of the tab's requests: the query's
                    // cancel stops it.
                    let button = cancel_button();
                    let at = place(ui, &button);
                    if button.show_at(ui, at, &look, &palette).clicked() {
                        actions.push(Action::CancelQuery(tab));
                    }
                }
            }

            let limit = right - BETWEEN;
            let mut x = full.left() + SIDE;
            let dot = Rect::from_min_size(pos2(x, y - DOT / 2.0), vec2(DOT, DOT));
            if dot.right() <= right {
                ui.painter()
                    .circle_filled(dot.center(), DOT / 2.0, Tone::Warning.color(&palette));
            }
            x += lead;
            // The room is given out by what the bar alone says: that a
            // save runs, then what is to fix, then some of what the last
            // save came to. The counts take what those leave, down to the
            // changes alone, and what the save came to takes the rest.
            // Save says why it cannot be pressed whatever is left out here.
            let mut left = (limit - x).max(0.0);
            let busy_said = busy_width.is_some_and(|width| take(&mut left, width));
            let spins = busy_said || (saving && take(&mut left, RING));
            let fixes = fix_width.is_some_and(|width| take(&mut left, width));
            let note_kept = note_least.map_or(0.0, |least| {
                let kept = least.min(left);
                take(&mut left, kept);
                kept
            });
            let strong = |text: &str| width(TextRole::UiBodyStrong, text);
            let counts_shown = match (&whole, &short) {
                (Some(whole), _) if take(&mut left, strong(whole)) => Some(whole),
                (_, Some(short)) if take(&mut left, strong(short)) => Some(short),
                _ => None,
            };
            let note_room = note_kept + left;

            if let Some(whole) = &whole {
                let cut = counts_shown != Some(whole);
                let place = match counts_shown {
                    Some(text) => {
                        let text = Text::one(&look, TextRole::UiBodyStrong, text, palette.text);
                        let laid = text.layout(ui.ctx());
                        let width = laid.paint_left(ui.painter(), x, y);
                        let place = Rect::from_min_size(
                            pos2(x, y - laid.height() / 2.0),
                            vec2(width.max(1.0), laid.height()),
                        );
                        x += width + APART;
                        place
                    }
                    // The dot stands for them.
                    None => dot,
                };
                named(ui, place, "pending-counts", whole, cut);
            }
            if let Some(fix) = fix.as_ref().filter(|_| fixes) {
                let mark = Rect::from_center_size(pos2(x + MARK / 2.0, y), vec2(MARK, MARK));
                Icon::CircleAlert
                    .image(palette.danger, MARK)
                    .paint_at(ui, mark);
                x += MARK + 5.0;
                x += widgets::paint_label(ui, x, y, Text::one(&look, body, fix, palette.danger))
                    + APART;
            }
            if spins {
                let ring = Rect::from_center_size(pos2(x + RING / 2.0, y), vec2(RING, RING));
                states::spinner(ui, ring, Tone::Warning.color(&palette), &palette);
                x += RING + if busy_said { 6.0 } else { APART };
            }
            if busy_said {
                let busy = Text::one(&look, body, &busy, palette.secondary);
                x += widgets::paint_label(ui, x, y, busy) + APART;
            }
            if let Some(note) = &note {
                said(ui, (x, y), note_room, note, &look, &palette);
            }
        });
    app.actions.extend(actions);
}

/// Takes `width` of the `left` room and the gap after it. Says whether
/// there was that much.
fn take(left: &mut f32, width: f32) -> bool {
    let fits = width <= *left;
    if fits {
        *left = (*left - width - APART).max(0.0);
    }
    fits
}

/// Names `rect` as `whole` to a screen reader, and under the pointer too
/// where the bar drew less than that there (`cut`).
fn named(ui: &egui::Ui, rect: Rect, salt: &str, whole: &str, cut: bool) {
    let response = ui.interact(rect, ui.id().with(salt), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, whole));
    if cut {
        response.on_hover_text(whole);
    }
}

/// What the last save came to, from `x` on the line centred at `y`: cut
/// with "…" to `room`, whole under the pointer and to a screen reader.
fn said(
    ui: &egui::Ui,
    (x, y): (f32, f32),
    room: f32,
    text: &str,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
) {
    let role = widgets::body(look);
    let width = |text: &str| role.width(ui.ctx(), look.faces, text);
    let shown = crate::ui::grid::ellipsize(text, room, false, width);
    let laid = Text::one(look, role, &shown, palette.text).layout(ui.ctx());
    // With no room for "…" either, nothing is drawn: it is named all the
    // same.
    let drawn = if laid.width() <= room {
        laid.paint_left(ui.painter(), x, y)
    } else {
        0.0
    };
    let rect = Rect::from_min_size(
        pos2(x, y - laid.height() / 2.0),
        vec2(drawn.max(1.0), laid.height()),
    );
    named(ui, rect, "pending-note", text, shown != text);
}
