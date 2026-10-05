//! Review SQL as it is read: the words of its comment lines, the colours of
//! its statements, its lines one to a row, and the text the clipboard gets.
//! Every place that shows a review draws it with what is here. The panel
//! itself is here too: the drawer above the pending bar on macOS and
//! Windows, and the terminal look's panel above its status line.

use std::hash::{Hash, Hasher};

use egui::{Color32, Frame, Id, Key, Modifiers, Rect, Sense, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::Access;
use tabletist_db::sql::TokenKind;

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, Dialog, TabId};
use crate::review::{Ink, Line, Review};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::pending_bar::counted;
use crate::ui::sql_text;
use crate::ui::states::Tone;
use crate::ui::widgets::{self, ButtonSpec};

/// The most lines a place shows before it scrolls.
pub const MAX_ROWS: usize = 12;

/// The room between two lines, over what the code face gives a line.
const LEADING: f32 = 4.0;

/// The panel's head.
const HEAD: f32 = 32.0;

/// The head's button: lower than the head, so its fill under the pointer
/// stands clear of the head's lines.
const BUTTON: f32 = 24.0;

/// The room above the lines and under them.
const PAD: f32 = 8.0;

/// The terminal look's foot under the lines.
const FOOT: f32 = 28.0;

/// The room between two things on the terminal look's head or foot, as
/// its status line keeps it.
const GAP: f32 = 18.0;

/// What a copied review opens with, after the comment's dashes: pasted
/// elsewhere, nothing checks a row and nothing wraps a transaction.
const COPIED: &str = "What Tabletist runs to save these changes, in one transaction. \
                      Each statement runs only while its row is still as the comment \
                      above it says.";

/// A comment line as it reads, its `--` included. `None` for a line of a
/// statement, which is drawn from its pieces. The app's own words are lower
/// case in every look, as a comment's are; a name, a value and the
/// builder's reason stand as the line holds them.
pub fn comment(line: &Line, locale: Locale) -> Option<String> {
    let say = |text: &'static str| gettext(locale, text);
    Some(match line {
        Line::Row(row) => format!("-- {} {row}", say("row")),
        Line::Check(loaded) => {
            let still: Vec<String> = loaded
                .iter()
                .map(|(column, value)| format!("{column} {} {value}", say("is still")))
                .collect();
            let and = format!(" {} ", say("and"));
            format!("-- {} {}", say("only if"), still.join(&and))
        }
        Line::Blocked { row, columns } => format!(
            "-- {} {row} · {} {} {}",
            say("row"),
            say("blocked: fix"),
            columns.join(", "),
            say("first")
        ),
        Line::Refused { row, reason } => {
            format!(
                "-- {} {row} · {} {reason}",
                say("row"),
                say("cannot be sent:")
            )
        }
        // The disabled Save's sentence, as a comment begins.
        Line::Unsendable => format!(
            "-- {}",
            say("these changes cannot be sent: the table's key is not known")
        ),
        Line::Sql(_) => return None,
    })
}

/// The colour a piece of a statement is drawn in: the SQL editor's for
/// the same kind of token.
pub fn ink_color(ink: Ink, palette: &Palette) -> Color32 {
    match ink {
        Ink::Plain => palette.text,
        Ink::Keyword => sql_text::color_of(TokenKind::Keyword, palette),
        Ink::Text => sql_text::color_of(TokenKind::String, palette),
        Ink::Number => sql_text::color_of(TokenKind::Number, palette),
    }
}

/// The colour of a comment line: a comment's in the SQL editor, and the
/// danger colour for a row that has no statement.
fn comment_color(line: &Line, palette: &Palette) -> Color32 {
    match line {
        Line::Blocked { .. } | Line::Refused { .. } | Line::Unsendable => {
            Tone::Danger.color(palette)
        }
        Line::Row(_) | Line::Check(_) | Line::Sql(_) => {
            sql_text::color_of(TokenKind::Comment, palette)
        }
    }
}

/// One line in the code face, in its colours, never wrapped.
pub fn line_text(line: &Line, look: &Look, palette: &Palette, locale: Locale) -> Text {
    let role = widgets::code(look);
    // A comment is laid as a statement's line is, as one run: the two sit
    // in their rows alike.
    let words = comment(line, locale).unwrap_or_default();
    let mut text = match line {
        Line::Sql(pieces) => Text::new(look).add_runs(
            role,
            pieces
                .iter()
                .map(|piece| (piece.text.as_str(), ink_color(piece.ink, palette))),
        ),
        _ => Text::new(look).add_runs(role, [(words.as_str(), comment_color(line, palette))]),
    };
    // One row whatever it holds: a line break would lay a second row over
    // the line under it, where it could pass for a line of its own.
    text.job_mut().break_on_newline = false;
    text
}

/// The height of one line.
pub fn row_height(ctx: &egui::Context, look: &Look) -> f32 {
    widgets::code(look).row_height(ctx, look.faces) + LEADING
}

/// The lines `range` of `lines`, one to a row: what a scroll area's
/// `show_rows` draws. The caller sets the item spacing to zero on the `ui`
/// that calls `show_rows`, before the call: `show_rows` adds that spacing
/// to the row height it is given, and the lines would drift from their
/// rows.
pub fn rows(
    ui: &mut egui::Ui,
    lines: &[Line],
    range: std::ops::Range<usize>,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) {
    let height = row_height(ui.ctx(), look);
    for line in lines.get(range).unwrap_or_default() {
        let laid = line_text(line, look, palette, locale).layout(ui.ctx());
        // As wide as the line, so a long one scrolls and is never cut by
        // its row; and no narrower than the place.
        let room = Some(ui.available_width()).filter(|room| room.is_finite());
        let width = room.map_or(laid.width(), |room| room.max(laid.width()));
        // Painted, not a label that can be selected: what is shown is cut,
        // and a copy of it would be pasted as it is.
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, laid.galley.text()));
        if ui.is_rect_visible(rect) {
            laid.paint_left(ui.painter(), rect.left(), rect.center().y);
        }
    }
}

#[cfg(test)]
thread_local! {
    /// How many lines this thread laid out for [`widest`].
    pub static MEASURED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// How wide the widest line of `review` is, laid out as [`rows`] draws it.
/// Measured once for a review and kept until another is asked about: a
/// frame lays out the lines in view and no more, and a large set has
/// thousands. What is kept is told by the lines themselves and by what
/// words and sizes them, so a review that changed is measured again.
pub fn widest(
    ctx: &egui::Context,
    review: &Review,
    look: &Look,
    palette: &Palette,
    locale: Locale,
) -> f32 {
    let mut hasher = std::hash::DefaultHasher::new();
    review.lines.hash(&mut hasher);
    look.name.hash(&mut hasher);
    std::mem::discriminant(&locale).hash(&mut hasher);
    ctx.pixels_per_point().to_bits().hash(&mut hasher);
    let key = hasher.finish();
    let id = Id::new("review-sql-widest");
    let kept: Option<(u64, f32)> = ctx.data(|data| data.get_temp(id));
    if let Some((_, width)) = kept.filter(|(of, _)| *of == key) {
        return width;
    }
    let width = review.lines.iter().fold(0.0, |widest: f32, line| {
        #[cfg(test)]
        MEASURED.with(|count| count.set(count.get() + 1));
        widest.max(line_text(line, look, palette, locale).layout(ctx).width())
    });
    ctx.data_mut(|data| data.insert_temp(id, (key, width)));
    width
}

/// A review as the clipboard gets it: the line that says what it is, then
/// one line of text for each of its lines, a line break after each.
pub fn text(review: &Review, locale: Locale) -> String {
    let mut text = format!("-- {}\n", gettext(locale, COPIED));
    for line in &review.lines {
        if let Some(words) = comment(line, locale).or_else(|| line.sql()) {
            text.push_str(&words);
            text.push('\n');
        }
    }
    text
}

/// The Review SQL of the table tab `id`, while it is open: a bottom panel.
/// Called after the pending bar it stands on it; in the terminal look it
/// stands on the status line, under the grid's error line. Under the
/// confirmation of this tab's save to production, which points at it there,
/// Page Up and Page Down scroll it: the pointer cannot reach it under a
/// dialog.
pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, id: TabId) {
    let (locale, palette, look) = (app.locale, app.palette, app.look);
    let workspace = app.workspace(tab);
    let edits = workspace
        .and_then(|workspace| workspace.object_tab(id))
        .map(|object| &object.edits)
        .filter(|edits| edits.reviewing);
    let Some(review) = edits.and_then(|edits| edits.review.as_ref()) else {
        // A panel takes one of its parent's ids. Passed over while there is
        // no drawer, so what is drawn after it (the grid, the filter's bar)
        // is the same widget with the drawer and without: a field that has
        // the keyboard keeps it as the drawer opens and closes.
        ui.skip_ahead_auto_ids(1);
        return;
    };
    let read_only = workspace.is_some_and(|workspace| workspace.access == Access::ReadOnly);
    // An editor that was typed into holds what a save would send with the
    // rest, and the lines are of the pending set alone: the head says so.
    // Not the lines themselves: they are made when the set changes, never
    // for a keystroke.
    let typing = edits
        .and_then(|edits| edits.editor.as_ref())
        .is_some_and(|editor| editor.touched);
    // Only under that confirmation: with no dialog up the two keys are the
    // grid's, and under any other they are not this panel's. Read before
    // the lines are drawn, and before the dialog's field sees them.
    let confirming = matches!(
        &app.dialog,
        Some(Dialog::ConfirmWrite(prompt)) if (prompt.tab, prompt.id) == (tab, id)
    );
    let pages = if confirming {
        ui.ctx().input_mut(|input| {
            let mut pressed = |key: Key| input.count_and_consume_key(Modifiers::NONE, key) as f32;
            pressed(Key::PageDown) - pressed(Key::PageUp)
        })
    } else {
        0.0
    };
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
    };
    let lines = &review.lines;
    let row = row_height(ui.ctx(), &look);
    let body = lines.len().min(MAX_ROWS) as f32 * row + 2.0 * PAD;
    // Only the terminal look's panel has a foot: the other looks hide the
    // drawer from the bar under it, and copy from its head.
    let foot = if look.terminal { FOOT } else { 0.0 };
    // As tall as its lines, to at most `MAX_ROWS` of them, and never more
    // than half of what the tab has for its grid and for this.
    let height = (HEAD + body + foot)
        .min(ui.available_height() / 2.0)
        .max(0.0);
    // The neighbour under it sets the lines' left edge: the bar's, or the
    // status line's.
    let side = if look.terminal { 12.0 } else { 20.0 };
    let mut asked = Asked::default();
    // How tall the lines stand, and how wide: nothing in a panel too low
    // for them.
    let mut shown = egui::Vec2::ZERO;
    let panel = egui::Panel::bottom(Id::new(("review-sql", tab.0, id.0)))
        .exact_size(height)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let full = ui.max_rect();
            let head = Rect::from_min_max(full.min, pos2(full.right(), full.top() + HEAD));
            ui.painter().rect_filled(head, 0, palette.panel);
            widgets::hline(ui, head.x_range(), head.top() + 0.5, palette.outline);
            widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.outline);
            if look.terminal {
                let counts = (review.changes, review.rows);
                terminal_head(ui, head, side, counts, typing, skin);
                let foot = Rect::from_min_max(pos2(full.left(), full.bottom() - FOOT), full.max);
                asked = terminal_foot(ui, foot, side, read_only, skin);
            } else {
                asked.copy = desktop_head(ui, head, side, typing, skin);
            }
            let place = Rect::from_min_max(
                pos2(full.left() + side, head.bottom() + PAD),
                pos2(full.right() - side, full.bottom() - foot - PAD),
            );
            if !place.is_positive() {
                return;
            }
            let mut body = ui.new_child(egui::UiBuilder::new().max_rect(place));
            // Before `show_rows`, which reads it from the `ui` it is given.
            body.spacing_mut().item_spacing = egui::Vec2::ZERO;
            // Both ways: a line is never wrapped into what could read as
            // two. As low as its place is: egui would keep 64 points of a
            // scroll area, over the bar under a drawer that has less.
            let lines_in = egui::ScrollArea::both()
                .id_salt(("review-sql-lines", tab.0, id.0))
                .auto_shrink([false, false])
                .min_scrolled_width(0.0)
                .min_scrolled_height(0.0)
                .show_rows(&mut body, row, lines.len(), |ui, range| {
                    // A page is the whole rows in view, so no line is
                    // passed over unseen. The lines move up for Page Down.
                    if pages != 0.0 {
                        let page = (place.height() / row).floor().max(1.0) * row;
                        ui.scroll_with_delta(vec2(0.0, -pages * page));
                    }
                    rows(ui, lines, range, &look, &palette, locale);
                })
                .inner_rect;
            // As wide as the lines are seen: without what a scroll bar
            // takes of the place, where a look gives its bars room.
            shown = vec2(lines_in.width(), place.height());
        });
    // The whole statements, never the lines as they are shown: those are
    // cut.
    if asked.copy
        && let Some(text) = copy_text(app, tab, id)
    {
        ui.ctx().copy_text(text);
    }
    let placed = Placed {
        tab,
        id,
        rect: panel.response.rect,
        lines: shown.y,
        width: shown.x,
    };
    let now = ui.ctx().cumulative_frame_nr();
    ui.ctx()
        .data_mut(|data| data.insert_temp(placed_id(), (placed, now)));
    if asked.hide {
        let show = false;
        app.actions.push(Action::ReviewEdits { tab, id, show });
    }
}

/// How the panel draws.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

impl Skin<'_> {
    /// `text` translated, in the look's case.
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }
}

/// What the panel's buttons were pressed for.
#[derive(Default)]
struct Asked {
    copy: bool,
    hide: bool,
}

/// What the head says while an editor holds typed text: a save would send
/// it too, and the lines under the head do not show it.
const TYPING: &str = "Without the cell being edited";

/// The head on macOS and Windows: what a save is, and Copy SQL at its
/// right. The counts are in the bar under the drawer. While a cell is
/// being edited (`typing`) it says that in the warning colour, where it
/// said what a save is. Returns whether the button was pressed.
fn desktop_head(ui: &mut egui::Ui, head: Rect, side: f32, typing: bool, skin: Skin<'_>) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let y = head.center().y;
    // The button first, from the right: what the head says gives way to
    // it.
    let name = gettext(locale, "Copy SQL");
    let button = ButtonSpec::new(&name).quiet();
    let width = button.width(ui, look);
    let at = Rect::from_min_size(
        pos2(head.right() - side - width, y - BUTTON / 2.0),
        vec2(width, BUTTON),
    );
    let copy = button.show_at(ui, at, look, palette).clicked();
    let (said, color) = if typing {
        (gettext(locale, TYPING), Tone::Warning.color(palette))
    } else {
        (
            gettext(locale, "Runs in one transaction"),
            palette.secondary,
        )
    };
    let said = || Text::one(look, widgets::body(look), &said, color);
    if head.left() + side + widgets::measure(ui, said()) <= at.left() - 8.0 {
        widgets::paint_label(ui, head.left() + side, y, said());
    }
    copy
}

/// The terminal look's head: how much is pending, since the look has no
/// bar to say it, and at the right what a save is, or, while a cell is
/// being edited (`typing`), that the lines are without it. Its buttons are
/// in the foot.
fn terminal_head(
    ui: &egui::Ui,
    head: Rect,
    side: f32,
    (changes, rows): (usize, usize),
    typing: bool,
    skin: Skin<'_>,
) {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let y = head.center().y;
    let counts = format!(
        "{} · {} · {}",
        skin.say("pending"),
        look.label(&counted(locale, changes, "change", "changes")),
        look.label(&counted(locale, rows, "row", "rows"))
    );
    let counts = Text::one(look, TextRole::OGroup, &counts, palette.text);
    let left = head.left() + side;
    let end = left + widgets::paint_label(ui, left, y, counts);
    // Where the head has the room for both.
    let (said, color) = if typing {
        (skin.say(TYPING), Tone::Warning.color(palette))
    } else {
        (skin.say("one transaction"), palette.dim)
    };
    let said = || Text::one(look, TextRole::OSecondary, &said, color);
    let start = head.right() - side - widgets::measure(ui, said());
    if end + GAP <= start {
        widgets::paint_label(ui, start, y, said());
    }
}

/// The terminal look's foot: the keys that close the panel and copy its
/// SQL at the left, the one that saves at the right, drawn as the status
/// line under it draws its keys. Over each of the two left hints lies a
/// button that is not drawn, so the pointer and a screen reader reach what
/// the keys do.
fn terminal_foot(
    ui: &mut egui::Ui,
    foot: Rect,
    side: f32,
    read_only: bool,
    skin: Skin<'_>,
) -> Asked {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    ui.painter().rect_filled(foot, 0, palette.panel);
    widgets::hline(ui, foot.x_range(), foot.top() + 0.5, palette.outline);
    let y = foot.top() + 1.0 + (FOOT - 1.0) / 2.0;
    let (close, copy, write) = (skin.say("close"), skin.say("copy sql"), skin.say("write"));
    let names = [gettext(locale, "Hide SQL"), gettext(locale, "Copy SQL")];
    let mut pressed = [false; 2];
    let mut x = foot.left() + side;
    for (index, hint) in [("esc", &*close, true), ("Y", &*copy, true)]
        .into_iter()
        .enumerate()
    {
        let width = widgets::key_hints(ui, (x, y), &[hint], GAP, look, palette);
        let place = Rect::from_min_max(pos2(x, foot.top() + 1.0), pos2(x + width, foot.bottom()));
        pressed[index] = ButtonSpec::new(&names[index])
            .hidden_at(ui, place)
            .clicked();
        x += width + GAP;
    }
    // Left out on a connection that cannot write, as the status line
    // strikes it out there; and where the foot is too narrow for it.
    let hint = [(":w", &*write, true)];
    let start = foot.right() - side - widgets::key_hints_width(ui, &hint, GAP, look, palette);
    if !read_only && x <= start {
        widgets::key_hints(ui, (start, y), &hint, GAP, look, palette);
    }
    Asked {
        hide: pressed[0],
        copy: pressed[1],
    }
}

/// What Copy SQL puts on the clipboard: the tab's review with every value
/// whole. `None` when nothing is pending.
pub fn copy_text(app: &App, tab: ConnTabId, id: TabId) -> Option<String> {
    app.review_whole(tab, id)
        .map(|review| text(&review, app.locale))
}

/// A panel that was drawn: whose it is, and where it stood.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Placed {
    pub tab: ConnTabId,
    pub id: TabId,
    pub rect: Rect,
    /// How tall its lines stand in it, between its head and its foot.
    pub lines: f32,
    /// How wide they stand: what is past it of a longer line is not seen
    /// until the lines are moved sideways.
    pub width: f32,
}

/// Where the last panel drawn is kept, with the number of its frame.
fn placed_id() -> Id {
    Id::new("review-sql-placed")
}

/// The last panel drawn, and the number of the frame it was drawn in.
fn last_placed(ctx: &egui::Context) -> Option<(Placed, u64)> {
    ctx.data(|data| data.get_temp(placed_id()))
}

/// The panel drawn in the frame being drawn, or in the one that just
/// ended: what a test asks once a frame is over. egui counts a frame at its
/// end, so read after a frame "this frame" is the last one.
pub fn placed(ctx: &egui::Context) -> Option<Placed> {
    let (placed, when) = last_placed(ctx)?;
    (ctx.cumulative_frame_nr() <= when + 1).then_some(placed)
}

/// The panel drawn earlier in the frame that is being drawn, and in no
/// other: what is drawn after it in a frame asks, to stand clear of it or
/// to count on it being on screen. A panel of the frame before is not on
/// screen.
pub fn placed_now(ctx: &egui::Context) -> Option<Placed> {
    let (placed, when) = last_placed(ctx)?;
    (ctx.cumulative_frame_nr() == when).then_some(placed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::{Values, of};
    use crate::testing::Harness;
    use tabletist_db::{CellChange, ChangeSet, Dialect, NewValue, ObjectRef, RowChange, Value};

    /// The fixture's `users` with `email` set in each of the rows `ids`.
    fn changes(ids: &[i64], email: &str) -> ChangeSet {
        let row = |id: &i64| RowChange {
            key: vec![("id".into(), Value::Int(*id))],
            set: vec![CellChange {
                column: "email".into(),
                type_name: "TEXT".into(),
                loaded: Value::Text(format!("user{id}@example.com").into()),
                new: NewValue::Text(email.into()),
            }],
        };
        ChangeSet {
            object: ObjectRef::new("main", "users"),
            rows: ids.iter().map(row).collect(),
        }
    }

    #[test]
    fn every_comment_reads_as_the_spec_writes_it() {
        let locale = Locale::English;
        let read = |line: Line| comment(&line, locale);
        assert_eq!(
            read(Line::Row("id 2".into())).as_deref(),
            Some("-- row id 2")
        );
        let check = Line::Check(vec![
            ("kind".into(), "'print'".into()),
            ("alt_text".into(), "NULL".into()),
        ]);
        assert_eq!(
            read(check).as_deref(),
            Some("-- only if kind is still 'print' and alt_text is still NULL")
        );
        let blocked = |columns: &[&str]| Line::Blocked {
            row: "id 4".into(),
            columns: columns.iter().map(|column| (*column).to_owned()).collect(),
        };
        assert_eq!(
            read(blocked(&["publisher_id"])).as_deref(),
            Some("-- row id 4 · blocked: fix publisher_id first")
        );
        assert_eq!(
            read(blocked(&["publisher_id", "pages"])).as_deref(),
            Some("-- row id 4 · blocked: fix publisher_id, pages first")
        );
        let refused = Line::Refused {
            row: "id 4".into(),
            reason: "pages: INTEGER expects a whole number".into(),
        };
        assert_eq!(
            read(refused).as_deref(),
            Some("-- row id 4 · cannot be sent: pages: INTEGER expects a whole number")
        );
        assert_eq!(
            read(Line::Unsendable).as_deref(),
            Some("-- these changes cannot be sent: the table's key is not known")
        );
        // A statement's line is drawn from its pieces.
        assert_eq!(read(Line::Sql(Vec::new())), None);
    }

    #[test]
    fn a_piece_is_drawn_in_the_sql_editors_colour() {
        for palette in [Palette::light(), Palette::dark()] {
            for (ink, kind) in [
                (Ink::Keyword, TokenKind::Keyword),
                (Ink::Text, TokenKind::String),
                (Ink::Number, TokenKind::Number),
            ] {
                assert_eq!(
                    ink_color(ink, &palette),
                    sql_text::color_of(kind, &palette),
                    "{ink:?}"
                );
            }
            assert_eq!(ink_color(Ink::Plain, &palette), palette.text);
        }
    }

    #[test]
    fn a_line_is_one_row_and_keeps_its_leading_spaces() {
        let review = of(
            Dialect::Sqlite,
            &changes(&[2], "bob@example.com"),
            &[],
            Values::Shown,
        );
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            // The faces come with the first frame.
            harness.settle();
            let palette = harness.app.palette;
            for line in &review.lines {
                let laid = line_text(line, &look, &palette, Locale::English).layout(&harness.ctx);
                assert_eq!(laid.galley.rows.len(), 1, "{}", look.name);
                assert!(
                    laid.height() <= row_height(&harness.ctx, &look),
                    "{}",
                    look.name
                );
            }
            let set =
                line_text(&review.lines[3], &look, &palette, Locale::English).layout(&harness.ctx);
            assert_eq!(
                set.galley.text(),
                r#"   SET "email" = 'bob@example.com'"#,
                "{}",
                look.name
            );
            // A comment is laid as it is worded.
            let row =
                line_text(&review.lines[0], &look, &palette, Locale::English).layout(&harness.ctx);
            assert_eq!(row.galley.text(), "-- row id 2", "{}", look.name);
            // The room between two lines is the row's, over the face's own.
            let face = widgets::code(&look).row_height(&harness.ctx, look.faces);
            assert_eq!(row_height(&harness.ctx, &look), face + 4.0, "{}", look.name);
            // Whatever a line holds, it is one row: a line break in it
            // would lay a second over the row under it.
            let broken = Line::Row("id 2\nDROP TABLE users".into());
            let laid = line_text(&broken, &look, &palette, Locale::English).layout(&harness.ctx);
            assert_eq!(laid.galley.rows.len(), 1, "{}", look.name);
        }
    }

    #[test]
    fn rows_stand_one_row_height_apart() {
        let review = of(
            Dialect::Sqlite,
            &changes(&[2, 4], "bob@example.com"),
            &[],
            Values::Shown,
        );
        // Comments and a statement's lines, the second row's first too.
        let lines = &review.lines[..6];
        let texts = [
            "-- row id 2",
            "-- only if email is still 'user2@example.com'",
            r#"UPDATE "main"."users""#,
            r#"   SET "email" = 'bob@example.com'"#,
            r#" WHERE "id" = 2;"#,
            "-- row id 4",
        ];
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let mut height = 0.0;
            let tree = harness.frame_with(|ui| {
                height = row_height(ui.ctx(), &look);
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                egui::ScrollArea::both().show_rows(ui, height, lines.len(), |ui, range| {
                    rows(ui, lines, range, &look, &palette, Locale::English);
                });
            });
            let tops: Vec<f32> = texts
                .iter()
                .map(|text| {
                    harness
                        .painted_rect(text)
                        .unwrap_or_else(|| panic!("{}: {text} is not painted", look.name))
                        .top()
                })
                .collect();
            // egui places a widget on a grid of a thirty-second of a
            // point: that much, and no more, may a row be off.
            for (index, pair) in tops.windows(2).enumerate() {
                assert!(
                    (pair[1] - pair[0] - height).abs() <= 1.0 / 16.0,
                    "{}: line {index} to the next is {}, a row is {height}",
                    look.name,
                    pair[1] - pair[0]
                );
            }
            assert!(
                (tops[5] - tops[0] - 5.0 * height).abs() <= 1.0 / 16.0,
                "{}: the first to the last",
                look.name
            );
            // Every line has the same left edge: a statement's leading
            // spaces are drawn, not taken as its indent.
            let lefts: Vec<f32> = texts
                .iter()
                .filter_map(|text| harness.painted_rect(text))
                .map(|rect| rect.left())
                .collect();
            assert!(lefts.iter().all(|left| *left == lefts[0]), "{lefts:?}");
            // Each row is read to a screen reader as it is written.
            let labels = crate::testing::labels(&tree);
            for text in texts {
                assert!(
                    labels.iter().any(|label| label == text),
                    "{}: {text} is not read: {labels:?}",
                    look.name
                );
            }
        }
    }

    #[test]
    fn the_clipboards_text_says_what_it_is_and_holds_the_whole_statements() {
        let long = "x".repeat(100);
        let review = of(Dialect::Sqlite, &changes(&[2], &long), &[], Values::Whole);
        let copied = text(&review, Locale::English);
        let expected = format!(
            "-- What Tabletist runs to save these changes, in one transaction. \
             Each statement runs only while its row is still as the comment above it says.\n\
             -- row id 2\n\
             -- only if email is still 'user2@example.com'\n\
             UPDATE \"main\".\"users\"\n   \
             SET \"email\" = '{long}'\n \
             WHERE \"id\" = 2;\n"
        );
        assert_eq!(copied, expected);
        // What says what the text is is one comment: nothing of it is on a
        // line of its own, where it would be read as SQL.
        let first = copied.lines().next().unwrap();
        assert!(first.starts_with("-- What Tabletist runs") && first.ends_with("says."));
        assert_eq!(copied.lines().count(), 6);
    }
}
