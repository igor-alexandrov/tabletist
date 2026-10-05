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
use crate::ui::states::{self, Tone};
use crate::ui::widgets::{self, ButtonSpec};

/// The bar's height.
const HEIGHT: f32 = 48.0;

/// Left and right padding, as the footer under it has.
const SIDE: f32 = 20.0;

/// The dot that leads the bar.
const DOT: f32 = 8.0;

/// `count` with its noun: "1 change", "3 changes".
pub fn counted(locale: Locale, count: usize, one: &'static str, many: &'static str) -> String {
    let plural = u32::try_from(count).unwrap_or(u32::MAX);
    format!("{count} {}", ngettext(locale, one, many, plural))
}

/// "3 changes in 2 rows".
fn counts_text(changes: usize, rows: usize, locale: Locale) -> String {
    format!(
        "{} {} {}",
        counted(locale, changes, "change", "changes"),
        gettext(locale, "in"),
        counted(locale, rows, "row", "rows")
    )
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
        // The database's own words, after its code when it gave one.
        Note::Failed { error, .. } => match error {
            Error::Query {
                code: Some(code),
                message,
                ..
            } => format!("{code} · {} {nothing}", sentence(&format::capped(message))),
            other => format!(
                "{} {nothing}",
                sentence(&format::capped(&other.to_string()))
            ),
        },
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
    format!(
        "{} {} · {} · {}",
        gettext(locale, "written"),
        counted(locale, saved.changes, "change", "changes"),
        counted(locale, saved.rows, "row", "rows"),
        format::elapsed(saved.elapsed)
    )
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
        SaveBlock::Disconnected => gettext(locale, "Not connected").into_owned(),
        SaveBlock::ReadOnly => gettext(locale, "This connection opens read-only").into_owned(),
        SaveBlock::Unsendable => gettext(
            locale,
            "These changes cannot be sent: the table's key is not known",
        )
        .into_owned(),
    }
}

/// The bar of the table tab `id`, while it has something to say. A bottom
/// panel: called after the footer, it stands above it.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let object = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
        .filter(|object| {
            let edits = &object.edits;
            !edits.cells.is_empty() || edits.saving.is_some() || edits.note.is_some()
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
    let pending = !edits.cells.is_empty();
    let reviewing = edits.reviewing;
    let counts = edits.counts();
    let note = edits
        .note
        .as_ref()
        .map(|note| note_text(note, object, locale));
    let blocked = app
        .save_blocked(tab, id)
        .map(|block| block_text(block, counts.to_fix, locale));
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
            // The buttons first, from the right: what the bar says is cut
            // where they begin.
            let mut right = full.right() - SIDE;
            let mut place = |ui: &egui::Ui, button: &ButtonSpec<'_>| {
                let width = button.width(ui, &look);
                let rect =
                    Rect::from_min_size(pos2(right - width, y - height / 2.0), vec2(width, height));
                right -= width + 8.0;
                rect
            };
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
            if pending || saving {
                let keys = format!("{}S", look.command_key());
                let button = ButtonSpec::new(&save).primary().shortcut(&keys);
                let at = place(ui, &button);
                let button = match &blocked {
                    Some(reason) => button.disabled(reason),
                    None => button,
                };
                if button.show_at(ui, at, &look, &palette).clicked() {
                    actions.push(Action::WriteEdits { tab, id });
                }
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
                // too: the drawer then shows what was sent. One button
                // under two names, so the keyboard stays on it.
                if pending {
                    let name = if reviewing { &hide } else { &review };
                    let button = ButtonSpec::new(name).quiet().keyed("review-sql");
                    let at = place(ui, &button);
                    if button.show_at(ui, at, &look, &palette).clicked() {
                        let show = !reviewing;
                        actions.push(Action::ReviewEdits { tab, id, show });
                    }
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
            if saving {
                // A save is one of the tab's requests: the query's cancel
                // stops it.
                let button = ButtonSpec::new(&cancel).label(&cancel_save).quiet();
                let at = place(ui, &button);
                if button.show_at(ui, at, &look, &palette).clicked() {
                    actions.push(Action::CancelQuery(tab));
                }
            }
            let limit = right - 8.0;
            let mut x = full.left() + SIDE;
            ui.painter().circle_filled(
                pos2(x + DOT / 2.0, y),
                DOT / 2.0,
                Tone::Warning.color(&palette),
            );
            x += DOT + 10.0;
            let body = widgets::body(&look);
            if pending {
                let text = counts_text(counts.changes, counts.rows, locale);
                let strong = Text::one(&look, TextRole::UiBodyStrong, &text, palette.text);
                x += widgets::paint_label(ui, x, y, strong) + 14.0;
            }
            if counts.to_fix > 0 {
                let mark = Rect::from_center_size(pos2(x + 6.5, y), vec2(13.0, 13.0));
                Icon::CircleAlert
                    .image(palette.danger, 13.0)
                    .paint_at(ui, mark);
                x += 13.0 + 5.0;
                let text = format!("{} {}", counts.to_fix, gettext(locale, "to fix"));
                x += widgets::paint_label(ui, x, y, Text::one(&look, body, &text, palette.danger))
                    + 14.0;
            }
            if saving {
                let ring = Rect::from_center_size(pos2(x + 7.0, y), vec2(14.0, 14.0));
                states::spinner(ui, ring, Tone::Warning.color(&palette), &palette);
                x += 14.0 + 6.0;
                x += widgets::paint_label(
                    ui,
                    x,
                    y,
                    Text::one(&look, body, &busy, palette.secondary),
                ) + 14.0;
            }
            if let Some(note) = &note {
                said(ui, (x, y), (limit - x).max(0.0), note, &look, &palette);
            }
        });
    app.actions.extend(actions);
}

/// What the last save came to, from `x` on the line centred at `y`: cut
/// with "…" to `room`, whole under the pointer and to a screen reader.
fn said(
    ui: &mut egui::Ui,
    (x, y): (f32, f32),
    room: f32,
    text: &str,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
) {
    let role = widgets::body(look);
    let shown = crate::ui::grid::ellipsize(text, room, false, |text| {
        role.width(ui.ctx(), look.faces, text)
    });
    let laid = Text::one(look, role, &shown, palette.text).layout(ui.ctx());
    let width = laid.paint_left(ui.painter(), x, y);
    let rect = Rect::from_min_size(
        pos2(x, y - laid.height() / 2.0),
        vec2(width.max(1.0), laid.height()),
    );
    let response = ui.interact(rect, ui.id().with("pending-note"), Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, text));
    if shown != text {
        response.on_hover_text(text);
    }
}
