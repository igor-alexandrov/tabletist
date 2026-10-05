//! The question about the rows a save found changed on the server: what
//! to do with each, one after another. On macOS and Windows a sheet: what
//! was loaded, what the server holds now and what the user typed, side by
//! side for every column the user changed, and the answers at its foot.
//! In the terminal look a box: the same three values in their colours,
//! without rules, and the answers as keys in its foot.

use egui::{
    Color32, CornerRadius, Frame, Id, Key, Margin, Modifiers, Rect, Sense, Stroke, WidgetInfo,
    WidgetType, pos2, vec2,
};

use crate::app::App;
use crate::edit::{Answer, LEAD, Shown, ShownLine};
use crate::i18n::{Locale, gettext};
use crate::model::{Action, Dialog, ObjectTab, Workspace};
use crate::theme::Look;
use crate::typography::{Text, TextRole};
use crate::ui::format::{self, Marks};
use crate::ui::grid;
use crate::ui::keys::{consume_press, drop_repeats};
use crate::ui::states::Tone;
use crate::ui::terminal_dialog;
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::write_prompts::{ROW, Skin, buttons_in, fitted};

/// How wide the sheet is, where the window has the room.
const WIDTH: f32 = 520.0;

/// How wide the terminal's box is, where the window has the room.
const BOX_WIDTH: f32 = 560.0;

/// A row of the sheet's table: its header, and each column the user
/// changed.
const LINE: f32 = 30.0;

/// The most a table's lines take of the question before they scroll: what
/// the production confirmation gives its statements.
const MOST: f32 = 220.0;

/// How a table of the question lays out a line and marks its values.
#[derive(Clone, Copy)]
struct Form {
    /// The first column, which names the column a line is about. The
    /// values share what is left.
    name: f32,
    /// What lies between two columns.
    gap: f32,
    /// How far into its cell a text starts, and how far from its end it
    /// stops.
    inset: f32,
    /// A value is marked by a tint behind it, and a hairline parts the
    /// lines. Without them a value's colour is its mark.
    tinted: bool,
}

/// The sheet's table: cells that touch, each with room inside it for its
/// tint.
const SHEET: Form = Form {
    name: 80.0,
    gap: 0.0,
    inset: 8.0,
    tinted: true,
};

/// The values of the terminal's box: columns apart, with no rules.
const BOX: Form = Form {
    name: 90.0,
    gap: 10.0,
    inset: 0.0,
    tinted: false,
};

impl Form {
    /// The cell of `row` for the value at `at` of the `of` a line shows,
    /// after the column's name: they share the row's width equally.
    fn cell(self, row: Rect, of: usize, at: usize) -> Rect {
        let width = ((row.width() - self.name) / of.max(1) as f32 - self.gap).max(0.0);
        let left = row.left() + self.name + self.gap + (width + self.gap) * at as f32;
        Rect::from_min_max(pos2(left, row.top()), pos2(left + width, row.bottom()))
    }
}

/// The keys of the terminal's box in the order its foot has them, as the
/// design writes them: the answer, the letter that gives it, and what its
/// hint says.
const KEYS: [(Answer, &str, &str); 4] = [
    (Answer::Overwrite, "o", "overwrite"),
    (Answer::UseServer, "s", "use server"),
    (Answer::KeepMine, "k", "keep mine, reload"),
    (Answer::Discard, "d", "discard my changes"),
];

/// The answer a letter typed in the terminal's box stands for.
fn lettered(typed: &str) -> Option<Answer> {
    let mut keys = KEYS.iter();
    keys.find(|(_, letter, _)| *letter == typed)
        .map(|(answer, ..)| *answer)
}

/// What the question about a row is headed with, in the parts its line is
/// laid out from: only the row's name gives way when the line is too long.
pub struct Title {
    /// "Row", in the look's case.
    pub before: String,
    /// The row by its key, as the row panel names it: `id 2`. The key is
    /// the database's, and stays as it is.
    pub name: String,
    /// What became of the row, in the look's case.
    pub after: String,
}

impl Title {
    /// The whole of it: what a screen reader and the pointer are told.
    pub fn whole(&self) -> String {
        self.with(&self.name)
    }

    /// The title with `name` for the row's name: the name cut, or whole.
    fn with(&self, name: &str) -> String {
        format!("{} {name} {}", self.before, self.after)
    }
}

/// The heading of the question about the page's row `row`.
pub fn title(object: &ObjectTab, row: usize, gone: bool, look: &Look, locale: Locale) -> Title {
    let say = |text: &'static str| look.label(&gettext(locale, text));
    Title {
        before: say("Row"),
        name: super::pending_bar::row_name(object, row),
        after: if gone {
            say("no longer exists on the server")
        } else {
            say("changed on the server")
        },
    }
}

/// Where the row is: the connection's name and the table's, since a table
/// of one name can be open on two connections and the question can come
/// up over either. With `of` rows found changed by the save, also which of
/// them this is: "· 1 of 2".
pub fn place(
    workspace: &Workspace,
    object: &ObjectTab,
    (at, of): (usize, usize),
    look: &Look,
    locale: Locale,
) -> String {
    let table = format::display_safe(&object.object.name);
    let mut place = format!("{} · {table}", workspace.name);
    if of > 1 {
        let word = look.label(&gettext(locale, "of"));
        place.push_str(&format!(" · {} {word} {of}", at + 1));
    }
    place
}

/// The answers the question about a row offers, by the names of their
/// buttons: one for a row that is gone, three for one that changed.
pub fn answers(gone: bool) -> &'static [(&'static str, Answer)] {
    if gone {
        &[("Discard my changes", Answer::Discard)]
    } else {
        &[
            ("Keep mine, reload row", Answer::KeepMine),
            ("Use server values", Answer::UseServer),
            ("Overwrite", Answer::Overwrite),
        ]
    }
}

/// The keys a question was given in a frame, taken before it is drawn.
#[derive(Clone, Copy)]
struct Pressed {
    /// Enter was pressed. It answers only as Keep mine, with the keyboard
    /// on that button.
    enter: bool,
    /// Page Down less Page Up, as often as each was pressed: how far the
    /// lines move, in pages of what is shown of them.
    pages: f32,
}

/// What the question about one row is drawn from.
struct Asked<'a> {
    title: Title,
    /// The line under the title: see [`place`].
    place: String,
    /// The row is gone from the server: there is nothing it holds now.
    gone: bool,
    /// Which of the save's rows this is. Each has its own place in its
    /// lines: the next row's question starts at its first.
    at: usize,
    /// The columns the user changed, as the reducer made them ready.
    lines: &'a [ShownLine],
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
    };
    let Some(Dialog::Conflict(prompt)) = &app.dialog else {
        return;
    };
    let at = prompt.at;
    let shown = app.workspace(prompt.tab).and_then(|workspace| {
        let object = workspace.object_tab(prompt.id)?;
        Some((workspace, object, prompt.rows.get(at)?))
    });
    let Some((workspace, object, conflict)) = shown else {
        // Nothing to draw, so nothing to answer it with: it closes, or it
        // would keep the keyboard for good.
        app.actions.push(Action::CloseDialog);
        return;
    };
    let gone = conflict.server.is_none();
    let asked = Asked {
        title: title(object, conflict.row, gone, &look, locale),
        place: place(workspace, object, (at, prompt.rows.len()), &look, locale),
        gone,
        at,
        lines: &prompt.lines,
    };
    // No answer in the question's first moment: what was on its way to
    // the grid when it came up is not one. It is dropped without a sign.
    let ripe = crate::edit::answers_taken(prompt.shown);
    // Taken before anything is drawn: a button that has the keyboard would
    // read Enter as a press of itself, and Overwrite is one of them. Space
    // presses a button. What the key answers is for the buttons to say,
    // where they are made.
    let enter = ctx.input_mut(|input| consume_press(input, Modifiers::NONE, Key::Enter));
    // A key that is held answers nothing, however long the question has
    // been up. The next row's question has its buttons where the last
    // one's were, with the keyboard still on the one that was pressed, and
    // a button reads what a held Space repeats as a press of itself: held
    // on Overwrite it would overwrite every row of the save, one each half
    // second. (What a held Enter repeats went with the Enter above.)
    ctx.input_mut(|input| drop_repeats(input, Key::Space));
    // Page Up and Page Down move the lines where there are more than are
    // shown at once: a question answered from the keyboard is read from
    // it. Never a list behind the question.
    let pages = ctx.input_mut(|input| {
        let mut pressed = |key: Key| input.count_and_consume_key(Modifiers::NONE, key) as f32;
        pressed(Key::PageDown) - pressed(Key::PageUp)
    });
    let pressed = Pressed { enter, pages };
    let mut answers = Vec::new();
    let top = if look.terminal {
        conflict_box(ctx, &asked, skin, pressed, &mut answers)
    } else {
        sheet(ctx, &asked, skin, pressed, &mut answers)
    };
    // Esc is Keep mine for the row shown: nothing of the user's is dropped
    // and nothing is written. Pressed, not held: what it repeats would
    // keep row after row.
    let escape = |input: &mut egui::InputState| consume_press(input, Modifiers::NONE, Key::Escape);
    if top && ctx.input_mut(escape) {
        answers.push(Answer::KeepMine);
    }
    if ripe {
        // Each names the row that was drawn: a click or a key a frame
        // behind is no answer to the row that comes after it.
        let answers = answers.into_iter();
        app.actions
            .extend(answers.map(|answer| Action::AnswerConflict { at, answer }));
    }
}

/// macOS and Windows: the row and where it is, the table of what was
/// loaded, what the server holds now and what the user typed, and the
/// answers. Of the keys `pressed`, Enter answers only as Keep mine, with
/// the keyboard on that button, and Page Up and Page Down move the
/// table's lines. Returns whether the question is the dialog on top.
fn sheet(
    ctx: &egui::Context,
    asked: &Asked<'_>,
    skin: Skin<'_>,
    pressed: Pressed,
    answers: &mut Vec<Answer>,
) -> bool {
    let Skin { look, palette, .. } = skin;
    let modal = widgets::modal(Id::new("conflict-prompt"), look, palette).show(ctx, |ui| {
        ui.set_width(fitted(ui.ctx(), WIDTH));
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        let width = ui.available_width();
        // Only the row's name gives way: what became of the row is what
        // the question is about. The whole title is measured with every
        // cut of the name, so what is painted is what was fitted.
        let role = widgets::dialog_title(look);
        let measure = |name: &str| role.width(ui.ctx(), look.faces, &asked.title.with(name));
        let name = grid::ellipsize(&asked.title.name, width, false, measure);
        let (title, whole) = (asked.title.with(&name), asked.title.whole());
        said(ui, &title, &whole, role, palette.text, look);
        told(ui, asked, palette.secondary, skin);
        ui.add_space(12.0);
        table(ui, asked, pressed.pages, skin);
        ui.add_space(14.0);
        foot(ui, asked.gone, skin, pressed.enter, answers);
    });
    modal.is_top_modal
}

/// Under the title, in `color`: where the row is, on one line, and what
/// became of it, wrapped.
fn told(ui: &mut egui::Ui, asked: &Asked<'_>, color: Color32, skin: Skin<'_>) {
    let Skin { look, .. } = skin;
    let sentence = skin.say(if asked.gone {
        "Someone deleted it after you loaded it. Nothing was written."
    } else {
        "Someone saved it after you loaded it. Nothing was written."
    });
    let width = ui.available_width();
    ui.add_space(4.0);
    let role = widgets::body(look);
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let place = grid::ellipsize(&asked.place, width, false, measure);
    said(ui, &place, &asked.place, role, color, look);
    ui.add_space(4.0);
    Text::one(look, role, &sentence, color)
        .wrap(width)
        .layout(ui.ctx())
        .label(ui);
}

/// The terminal look: `≠ conflict` and the row, where it is, the values
/// in their colours, and the answers as keys in the foot, each its button
/// too. The letters answer as typed text. Of the keys `pressed`, Enter
/// answers only as Keep mine, with the keyboard on that hint's button, and
/// Page Up and Page Down move the lines. Returns whether the question is
/// the dialog on top.
fn conflict_box(
    ctx: &egui::Context,
    asked: &Asked<'_>,
    skin: Skin<'_>,
    pressed: Pressed,
    answers: &mut Vec<Answer>,
) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    // The letters, as typed text. Not while a key is held: the question
    // comes up when the database answers, and the key that was moving
    // through the grid may still be down (`k` goes up a row there), so
    // its repeats are no answer. Nor while a field has the keyboard: one
    // behind the box keeps it for the frame the box opens in, and what is
    // typed then went into its text.
    let typing = ctx.text_edit_focused();
    let typed = ctx.input_mut(|input| {
        let held = input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Key {
                    pressed: true,
                    repeat: true,
                    ..
                }
            )
        });
        let mut typed = None;
        input.events.retain(|event| match event {
            egui::Event::Text(text) if lettered(text).is_some() => {
                typed = typed.or(lettered(text));
                false
            }
            _ => true,
        });
        typed.filter(|_| !held && !typing)
    });
    // A letter the row does not offer is no answer to it.
    answers.extend(typed.filter(|answer| answer.offered(asked.gone)));
    let (frame, radius) = skin.frame();
    let height = TextRole::OBody.row_height(ctx, look.faces) + 4.0;
    let modal = widgets::modal(Id::new("conflict-prompt"), look, palette)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(fitted(ui.ctx(), BOX_WIDTH));
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            Frame::new()
                .inner_margin(Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    headed(ui, &asked.title, skin);
                    told(ui, asked, palette.dim, skin);
                    ui.add_space(10.0);
                    values(ui, asked, height, pressed.pages, skin);
                });
            let foot = terminal_dialog::foot(ui, radius, palette);
            // The foot's keys: what each answers, how it is written, what
            // it does, and the name of its button.
            let mut keys = Vec::new();
            // How the lines are moved, where they do not all show. Said
            // first, so a narrow foot gives it up before a key that
            // answers, and no button: the keys themselves do it.
            if asked.lines.len() as f32 * height > MOST {
                keys.push((None, "pgup/pgdn".to_owned(), skin.say("scroll"), None));
            }
            // The answers this row has, by the names the sheet's buttons
            // have.
            let offered = self::answers(asked.gone);
            for (answer, letter, words) in KEYS {
                if let Some((name, _)) = offered.iter().find(|(_, of)| *of == answer) {
                    let name = gettext(locale, name);
                    keys.push((
                        Some(answer),
                        format!("[{letter}]"),
                        skin.say(words),
                        Some(name),
                    ));
                }
            }
            let hints: Vec<_> = keys
                .iter()
                .map(|(answer, key, label, button)| terminal_dialog::Key {
                    key,
                    label,
                    button: button.as_deref(),
                    // The design writes every letter in the accent colour:
                    // none of the answers is the one to take.
                    lead: answer.is_some(),
                    disabled: None,
                })
                .collect();
            let given = |index: usize| keys.get(index).and_then(|(answer, ..)| *answer);
            // Enter was taken before the buttons are drawn: it answers as
            // the one that has the keyboard only where that keeps the
            // user's changes and writes nothing.
            let entered = terminal_dialog::keyboard_on(ui, &hints)
                .filter(|&index| pressed.enter && given(index) == Some(Answer::KeepMine));
            let clicked = terminal_dialog::keys(ui, foot, 0.0, &hints, look, palette);
            answers.extend(clicked.or(entered).and_then(given));
        });
    modal.is_top_modal
}

/// The first line of the terminal's box: the mark and `conflict` in the
/// warning colour, then the title, whose name gives way as the sheet's
/// does. The whole title is what a screen reader reads and, where the
/// name was cut, what the pointer shows.
fn headed(ui: &mut egui::Ui, title: &Title, skin: Skin<'_>) {
    let Skin { look, palette, .. } = skin;
    let role = TextRole::OGroup;
    let height = role.row_height(ui.ctx(), look.faces);
    let size = vec2(ui.available_width(), height);
    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
    let y = row.center().y;
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let mut left = row.left();
    // Where the look's font has no such glyph the word stands alone, as in
    // the status line.
    if let Some(mark) = super::workspace::drawable(ui, role, look, &["≠"]) {
        let mark = Text::one(look, role, mark, palette.warning);
        left += widgets::paint_text(ui, left, y, mark) + measure(" ");
    }
    let word = Text::one(look, role, &skin.say("conflict"), palette.warning);
    left += widgets::paint_label(ui, left, y, word) + 10.0;
    let place = Rect::from_min_max(pos2(left.min(row.right()), row.top()), row.max);
    let name = grid::ellipsize(&title.name, place.width(), false, |name| {
        measure(&title.with(name))
    });
    let (shown, whole) = (title.with(&name), title.whole());
    let text = Text::one(look, role, &shown, palette.text);
    let id = ui.id().with("conflict-title");
    written(ui, place, 0.0, id, text, &whole, shown != whole);
}

/// The values of the terminal's box: a header in the dim colour and under
/// it a line for each column the user changed, each `height` high, with
/// no rules. More lines than [`MOST`] holds scroll under the header, and
/// `pages` moves them.
fn values(ui: &mut egui::Ui, asked: &Asked<'_>, height: f32, pages: f32, skin: Skin<'_>) {
    let Skin { look, palette, .. } = skin;
    let gone = asked.gone;
    let size = vec2(ui.available_width(), height);
    let (head, _) = ui.allocate_exact_size(size, Sense::hover());
    // A row that is gone has nothing on the server to show.
    let words: &[&'static str] = if gone {
        &["loaded", "yours"]
    } else {
        &["loaded", "server", "yours"]
    };
    for (at, word) in words.iter().enumerate() {
        let x = BOX.cell(head, words.len(), at).left();
        let text = Text::one(look, TextRole::OBody, &skin.say(word), palette.dim);
        widgets::paint_label(ui, x, head.center().y, text);
    }
    lines(ui, asked, height, pages, |ui, row, index, shown| {
        line(ui, row, (index, 0), shown, (gone, BOX), skin);
    });
}

/// The lines of a table under its header, each `height` high and drawn by
/// `draw` in its row, with its place among them. More of them than
/// [`MOST`] holds scroll, and `pages` moves them by as many pages of the
/// whole lines in view: Page Down and Page Up, for a hand that is on the
/// keyboard.
fn lines(
    ui: &mut egui::Ui,
    asked: &Asked<'_>,
    height: f32,
    pages: f32,
    draw: impl Fn(&mut egui::Ui, Rect, usize, &ShownLine),
) {
    let size = vec2(ui.available_width(), height);
    egui::ScrollArea::vertical()
        .id_salt(("conflict-lines", asked.at))
        .max_height(MOST)
        .auto_shrink([false, true])
        .show(ui, |ui| {
            // A page is the whole lines in view, so none is passed over
            // unseen. The lines move up for Page Down.
            if pages != 0.0 {
                let shown = MOST.min(asked.lines.len() as f32 * height);
                let page = (shown / height).floor().max(1.0) * height;
                ui.scroll_with_delta(vec2(0.0, -pages * page));
            }
            for (index, shown) in asked.lines.iter().enumerate() {
                let (row, _) = ui.allocate_exact_size(size, Sense::hover());
                if ui.is_rect_visible(row) {
                    draw(ui, row, index, shown);
                }
            }
        });
}

/// One line across `ui`: `shown` painted, and `whole` for a screen reader
/// and, where the line was cut, under the pointer.
fn said(ui: &mut egui::Ui, shown: &str, whole: &str, role: TextRole, color: Color32, look: &Look) {
    let laid = Text::one(look, role, shown, color).layout(ui.ctx());
    let size = vec2(ui.available_width(), laid.height());
    let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, whole));
    if ui.is_rect_visible(rect) {
        laid.paint(ui.painter(), rect.min);
    }
    if shown != whole {
        response.on_hover_text(whole);
    }
}

/// The sheet's table: a header, and under it a line for each column the
/// user changed. More lines than [`MOST`] holds scroll under the header,
/// and `pages` moves them.
fn table(ui: &mut egui::Ui, asked: &Asked<'_>, pages: f32, skin: Skin<'_>) {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let gone = asked.gone;
    // What lies along the frame's edge follows its corners, inside its
    // line.
    let radius = look.radius.saturating_sub(1);
    Frame::new()
        .stroke(Stroke::new(1.0, palette.border))
        .corner_radius(CornerRadius::same(look.radius))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            let size = vec2(ui.available_width(), LINE);
            let (head, _) = ui.allocate_exact_size(size, Sense::hover());
            let top = CornerRadius {
                nw: radius,
                ne: radius,
                sw: 0,
                se: 0,
            };
            ui.painter().rect_filled(head, top, palette.surface);
            // A row that is gone has nothing on the server to show.
            let words: &[&'static str] = if gone {
                &["loaded", "yours"]
            } else {
                &["loaded", "now on server", "yours"]
            };
            for (at, word) in words.iter().enumerate() {
                let x = SHEET.cell(head, words.len(), at).left() + SHEET.inset;
                let word = gettext(locale, word);
                let role = widgets::secondary(look);
                let text = Text::one(look, role, &word, palette.secondary);
                widgets::paint_label(ui, x, head.center().y, text);
            }
            lines(ui, asked, LINE, pages, |ui, row, index, shown| {
                // The last line's tint ends in the frame's corner.
                let last = index + 1 == asked.lines.len();
                let end = if last { radius } else { 0 };
                line(ui, row, (index, end), shown, (gone, SHEET), skin);
            });
        });
}

/// One line of a table in `row`, laid out as `form` has it: the column's
/// name, the value that was loaded, the one the server holds now (not for
/// a row that is `gone`) and the user's. In the sheet the server's is on a
/// red tint where it is another than the loaded one, and the user's on the
/// amber tint a pending cell has in the grid; in the terminal's box each
/// is written in that colour. `index` tells the line from the others, and
/// `end` is how round its last cell's lower right corner is.
fn line(
    ui: &mut egui::Ui,
    row: Rect,
    (index, end): (usize, u8),
    shown: &ShownLine,
    (gone, form): (bool, Form),
    skin: Skin<'_>,
) {
    let Skin { look, palette, .. } = skin;
    // Each value, the tone that marks it and the colour it is written in.
    let mut values = vec![(&shown.loaded, None, palette.text)];
    if let (false, Some(server)) = (gone, &shown.server) {
        // The tint says that it changed there; a NULL on it says to what.
        values.push(if shown.moved {
            (server, Some(Tone::Danger), Tone::Danger.color(palette))
        } else {
            (server, None, palette.text)
        });
    }
    // On its tint the user's value is plain.
    let mine = if form.tinted {
        palette.text
    } else {
        Tone::Warning.color(palette)
    };
    values.push((&shown.yours, Some(Tone::Warning), mine));
    let of = values.len();
    for (at, (value, tone, color)) in values.into_iter().enumerate() {
        let place = form.cell(row, of, at);
        let salt = (index, at);
        match tone {
            Some(tone) if form.tinted => {
                let corner = CornerRadius {
                    se: if at + 1 == of { end } else { 0 },
                    ..CornerRadius::ZERO
                };
                ui.painter()
                    .rect_filled(place, corner, tone.fill(look, palette));
            }
            // Its colour is all a look without tints has to say that the
            // value changed on the server, and the look's own NULL is
            // faint whatever became of it: the word, in that colour.
            Some(Tone::Danger) if matches!(value, Shown::Null) => {
                let text = Text::one(look, grid::data_role(look), "NULL", color);
                let id = ui.id().with(("conflict-value", salt));
                written(ui, place, form.inset, id, text, "NULL", false);
                continue;
            }
            _ => {}
        }
        self::value(ui, place, salt, value, (color, form.inset), skin);
    }
    if form.tinted {
        // The hairline above the line, over its tints.
        widgets::hline(ui, row.x_range(), row.top() + 0.5, palette.surface);
    }
    let name = format::display_safe(&shown.name);
    // The terminal writes the name as it writes the values beside it.
    let role = TextRole::pick(look, widgets::code(look), TextRole::OBody);
    let place = Rect::from_min_max(row.min, pos2(row.left() + form.name, row.bottom()));
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let cut = grid::ellipsize(&name, form.name - 2.0 * form.inset, false, measure);
    let text = Text::one(look, role, &cut, palette.text);
    let id = ui.id().with(("conflict-name", index));
    written(ui, place, form.inset, id, text, &name, cut != name);
}

/// What a cell of the question shows of `value`, whole: one line, as a
/// grid cell writes a text, behind a "…" where it starts inside the value.
/// `None` is NULL.
fn reads(value: &Shown, marks: Marks) -> Option<String> {
    match value {
        Shown::Null => None,
        Shown::Text { text, cut: false } => Some(one_line(text, marks)),
        Shown::Text { text, cut: true } => Some(format!("…{}", one_line(text, marks))),
    }
}

/// `text` on one line, as a grid cell shows a text: `''` for the empty
/// one, a mark for each character of one that is all white, a mark where
/// a line breaks, and a cell's worth of it at the most.
fn one_line(text: &str, marks: Marks) -> String {
    format::blank_text(text, marks).unwrap_or_else(|| format::cell_line(text, marks).into_owned())
}

/// The line of a value that starts inside itself (`text`, read whole as
/// `whole`), fitted to `room` as `width` measures: the [`LEAD`] characters
/// before the place it differs from its line's other values are only
/// there to find that place by, so they are given up first, one by one.
/// The end is cut only when the difference itself is too long, and never
/// before it: two values that differ never read alike for want of room.
fn from_inside(
    text: &str,
    whole: &str,
    marks: Marks,
    room: f32,
    width: impl Fn(&str) -> f32,
) -> String {
    if width(whole) <= room {
        return whole.to_owned();
    }
    let mut line = String::new();
    for dropped in 1..=LEAD {
        let rest = text
            .char_indices()
            .nth(dropped)
            .map_or("", |(start, _)| &text[start..]);
        // A value that ends where its lead does has nothing after the
        // mark: not the `''` of a text that is empty.
        line = if rest.is_empty() {
            "…".to_owned()
        } else {
            format!("…{}", one_line(rest, marks))
        };
        if width(&line) <= room {
            return line;
        }
    }
    grid::ellipsize(&line, room, false, width).into_owned()
}

/// A value of a line in its cell `place`: the look's NULL, or its text in
/// `color`, `inset` into the cell and cut with "…" to it. `salt` tells the
/// cell from the table's others.
fn value(
    ui: &mut egui::Ui,
    place: Rect,
    salt: (usize, usize),
    value: &Shown,
    (color, inset): (Color32, f32),
    skin: Skin<'_>,
) {
    let Skin { look, palette, .. } = skin;
    let role = grid::data_role(look);
    let marks = grid::marks(ui.ctx(), look);
    let Some(whole) = reads(value, marks) else {
        // Laid out where a text would start, on the row's middle.
        let height = role.row_height(ui.ctx(), look.faces);
        let top = place.center().y - height / 2.0;
        let at = Rect::from_min_max(
            pos2(place.left() + inset, top),
            pos2(place.right(), top + height),
        );
        let builder = egui::UiBuilder::new()
            .id_salt(("conflict-null", salt))
            .max_rect(at);
        grid::null_label(&mut ui.new_child(builder), look, palette);
        return;
    };
    let room = place.width() - 2.0 * inset;
    let measure = |text: &str| role.width(ui.ctx(), look.faces, text);
    let shown = match value {
        Shown::Text { text, cut: true } => from_inside(text, &whole, marks, room, measure),
        _ => grid::ellipsize(&whole, room, false, measure).into_owned(),
    };
    let text = Text::one(look, role, &shown, color);
    let cut = shown != whole;
    let id = ui.id().with(("conflict-value", salt));
    written(ui, place, inset, id, text, &whole, cut);
}

/// Paints `text` in the cell `place`, `inset` in and on its middle.
/// `whole` is what a screen reader reads there, and with `cut`, what the
/// pointer shows over the cell. `id` tells the cell from every other.
fn written(ui: &mut egui::Ui, place: Rect, inset: f32, id: Id, text: Text, whole: &str, cut: bool) {
    widgets::paint_text(ui, place.left() + inset, place.center().y, text);
    // Hovered and no more: nothing in the table is pressed.
    let response = ui.interact(place, id, Sense::hover());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, whole));
    if cut {
        response.on_hover_text(whole);
    }
}

/// The answers, [`ROW`] high. Keep mine reads as a link at the left and is
/// made first, so the Tab key comes to it first; the others stand at the
/// right, Overwrite as the primary button. `enter` answers only as Keep
/// mine, with the keyboard on it: Return never overwrites, never takes the
/// server's values and never discards.
fn foot(ui: &mut egui::Ui, gone: bool, skin: Skin<'_>, enter: bool, answers: &mut Vec<Answer>) {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let offered = self::answers(gone);
    let names: Vec<_> = offered
        .iter()
        .map(|(name, _)| gettext(locale, name))
        .collect();
    let (mut link, mut buttons, mut given) = (None, Vec::new(), Vec::new());
    for ((_, answer), name) in offered.iter().zip(&names) {
        match answer {
            Answer::KeepMine => link = Some(ButtonSpec::new(name).link()),
            Answer::Overwrite => {
                buttons.push(ButtonSpec::new(name).primary().padding(16.0));
                given.push(*answer);
            }
            Answer::UseServer | Answer::Discard => {
                buttons.push(ButtonSpec::new(name));
                given.push(*answer);
            }
        }
    }
    let size = vec2(ui.available_width(), ROW);
    let (row, _) = ui.allocate_exact_size(size, Sense::hover());
    if let Some(link) = link {
        if enter && link.has_keyboard(ui) {
            answers.push(Answer::KeepMine);
        }
        let place = Rect::from_min_size(row.min, vec2(link.width(ui, look), ROW));
        if link.show_at(ui, place, look, palette).clicked() {
            answers.push(Answer::KeepMine);
        }
    }
    if let Some(pressed) = buttons_in(ui, row, buttons, skin)
        && let Some(answer) = given.get(pressed)
    {
        answers.push(*answer);
    }
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;
    use egui::{Key, Modifiers};
    use tabletist_db::{Conflict, RowPage, Structure, Value, WriteOutcome};

    use crate::backend::Command;
    use crate::model::{Action, CellPos, ConnTabId, Dialog, EditStart, TabId};
    use crate::testing::Harness;
    use crate::theme::Look;
    use crate::typography::TextRole;
    use crate::ui::states::Tone;
    use crate::ui::tests::{
        click_at, click_dialog, focus_dialog, focused_name, hover, pressable, type_key,
    };

    /// The looks that ask the question: every one of them.
    fn looks() -> impl Iterator<Item = Look> {
        Look::ALL.into_iter()
    }

    /// The looks that ask with the sheet, for what is the sheet's alone:
    /// its table, its buttons, the order the Tab key takes them in. They
    /// stay these two when the terminal look asks too.
    fn sheets() -> impl Iterator<Item = Look> {
        Look::ALL.into_iter().filter(|look| !look.terminal)
    }

    /// Makes `text` the pending email of the row `id 2` of the table `id`.
    fn retype(harness: &mut Harness, tab: ConnTabId, id: TabId, text: &str) {
        pend(harness, (tab, id), (1, 1), text);
    }

    /// Makes `text` the pending value of the table's cell at `at` (row,
    /// column).
    fn pend(harness: &mut Harness, (tab, id): (ConnTabId, TabId), at: (usize, usize), text: &str) {
        let cell = CellPos {
            row: at.0,
            col: at.1,
        };
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start: EditStart::Replace(text.into()),
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
    }

    /// A writable table `users` with `structure` and `page`, in `look` and
    /// in a window of `size`: for a question about other columns or other
    /// values than the fixture's.
    fn table_of(
        look: Look,
        size: egui::Vec2,
        structure: Structure,
        page: RowPage,
    ) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::with_size(size);
        harness.set_look(look);
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_structure(structure);
        harness.answer_rows(page);
        let id = harness.app.workspace(tab).unwrap().active_tab.unwrap();
        (harness, tab, id)
    }

    /// The fixture's table in `look`, in the harness's own window.
    fn fixture_in(look: Look) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        (harness, tab, id)
    }

    /// Saves what is pending and answers the save with `conflicts`: the
    /// question is up, and has been for long enough to take an answer.
    fn saved(harness: &mut Harness, (tab, id): (ConnTabId, TabId), conflicts: Vec<Conflict>) {
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.answer_written(Ok(WriteOutcome::Conflicts(conflicts)));
        harness.finish_animations();
        assert!(asking(harness));
        shown(harness, true);
    }

    /// The set's row `row` as the server holds it now.
    fn changed(row: usize, server: Vec<Value>) -> Conflict {
        let server = Some(server);
        Conflict { row, server }
    }

    /// The fills a frame painted for the question: a dialog is painted over
    /// everything else, and the first thing it paints is what dims the
    /// window. What comes after that is the question's own, wherever the
    /// grid behind it painted the like.
    fn own_fills(harness: &Harness) -> &[(egui::Rect, egui::Color32)] {
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
        let mut fills = harness.fills.iter();
        let dimmed = fills.rposition(|(rect, _)| rect.contains_rect(window));
        &harness.fills[dimmed.expect("the window is dimmed") + 1..]
    }

    /// Where the question is drawn: the first of its fills that is a
    /// dialog's own.
    fn sheet(harness: &Harness) -> egui::Rect {
        let overlay = harness.app.palette.overlay;
        let mut fills = own_fills(harness).iter();
        let frame = fills.find(|(_, fill)| *fill == overlay);
        frame.map(|(rect, _)| *rect).expect("the sheet")
    }

    /// The pieces of text the question itself painted that `wanted` picks,
    /// each with its place and its colour. Its title is the first thing
    /// the sheet writes, and the word `conflict` the first of the box
    /// (after its mark): nothing behind either reads as one, while the
    /// grid there writes the same values, and may stand where the
    /// question does.
    fn pieces(
        harness: &Harness,
        wanted: impl Fn(&str) -> bool,
    ) -> Vec<(String, egui::Rect, egui::Color32)> {
        let painted = harness.painted.iter().zip(&harness.text_rects);
        let own = painted.skip_while(|((piece, _), _)| {
            let title = piece.starts_with("Row ") && piece.ends_with("on the server");
            !(title || piece == "conflict")
        });
        own.filter(|((piece, _), _)| wanted(piece))
            .map(|((piece, color), (_, rect))| (piece.clone(), *rect, *color))
            .collect()
    }

    /// The one piece `text` the question painted: where, and in what colour.
    fn piece(harness: &Harness, text: &str) -> (egui::Rect, egui::Color32) {
        let found = pieces(harness, |piece| piece == text);
        let [(_, rect, color)] = found[..] else {
            panic!(
                "{text:?} is painted {} times in the question: {:?}",
                found.len(),
                pieces(harness, |_| true)
            );
        };
        (rect, color)
    }

    /// Whether the question painted no piece `text`.
    fn unsaid(harness: &Harness, text: &str) -> bool {
        pieces(harness, |piece| piece == text).is_empty()
    }

    /// The fills of `color` the question painted.
    fn fills(harness: &Harness, color: egui::Color32) -> Vec<egui::Rect> {
        let fills = own_fills(harness).iter();
        fills
            .filter(|(_, fill)| *fill == color)
            .map(|(rect, _)| *rect)
            .collect()
    }

    /// Whether the question filled a cell with `color` behind `place`.
    fn tinted(harness: &Harness, place: egui::Rect, color: egui::Color32) -> bool {
        let mut fills = fills(harness, color).into_iter();
        fills.any(|rect| rect.contains_rect(place))
    }

    /// Whether a screen reader finds `name` at `place`: a node of that
    /// name whose bounds hold it.
    fn named(harness: &mut Harness, name: &str, place: egui::Rect) -> bool {
        let tree = harness.settle();
        let mut nodes = tree.nodes.iter();
        nodes.any(|(_, node)| {
            let Some(rect) = node.bounds() else {
                return false;
            };
            let rect = egui::Rect::from_min_max(
                egui::pos2(rect.x0 as f32, rect.y0 as f32),
                egui::pos2(rect.x1 as f32, rect.y1 as f32),
            );
            node.label().or_else(|| node.value()) == Some(name) && rect.contains(place.center())
        })
    }

    /// The fixture's row `id 2` as the server holds it with `email`.
    fn server(email: &str) -> Vec<Value> {
        vec![Value::Int(2), Value::Text(email.into()), Value::Null]
    }

    /// The fixture's table in `look`, the email of its row `id 2` pending
    /// as `bob@example.com` and saved, and the save answered: the row
    /// holds `email` now, or is gone. The question is up, and has been for
    /// long enough to take an answer.
    fn conflict_in(look: Look, email: Option<&str>) -> (Harness, ConnTabId, TabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let (tab, id) = harness.editable();
        retype(&mut harness, tab, id, "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        let conflict = Conflict {
            row: 0,
            server: email.map(server),
        };
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        assert!(asking(&harness), "{}", look.name);
        shown(&mut harness, true);
        (harness, tab, id)
    }

    /// Makes the question look as if it had been on screen for a while
    /// (`long`), or as if it had only just come up, however long the test
    /// has taken.
    fn shown(harness: &mut Harness, long: bool) {
        let now = std::time::Instant::now();
        let hour = std::time::Duration::from_secs(3600);
        if let Some(Dialog::Conflict(prompt)) = &mut harness.app.dialog {
            prompt.shown = if long {
                now.checked_sub(crate::edit::ANSWER_AFTER)
                    .expect("an earlier instant")
            } else {
                now + hour
            };
        }
    }

    fn asking(harness: &Harness) -> bool {
        matches!(harness.app.dialog, Some(Dialog::Conflict(_)))
    }

    fn writes(harness: &Harness) -> usize {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Write { .. }))
            .count()
    }

    /// How many cells are pending, and the email the page holds for the
    /// row `id 2`.
    fn state(harness: &Harness, tab: ConnTabId, id: TabId) -> (usize, Value) {
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let email = object.page().unwrap().rows[1][1].clone();
        (object.edits.cells.len(), email)
    }

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    fn painted(harness: &Harness, text: &str) -> bool {
        harness.painted.iter().any(|(piece, _)| piece == text)
    }

    #[test]
    fn each_answer_is_a_button_and_escape_keeps_mine() {
        for look in looks() {
            let said = look.name;
            let eve = || text("eve@example.com");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, eve()), "{said}");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Keep mine, reload row");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}: nothing is saved again");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            click_dialog(&mut harness, "Overwrite");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 2, "{said}: the save runs again");
            // Esc is Keep mine.
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, eve()), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            // A row that is gone has one answer. Esc leaves its change
            // pending, and the bar says that the row is not there (the
            // terminal look's status line, in its own words).
            let loaded = || text("user2@example.com");
            let (mut harness, tab, id) = conflict_in(look, None);
            assert!(!harness.has("Overwrite"), "{said}");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, loaded()), "{said}");
            let line = if look.terminal {
                "conflict row id 2 no longer exists on the server. nothing was written."
            } else {
                "Row id 2 no longer exists on the server. Nothing was written."
            };
            assert!(harness.has(line), "{said}");
            let (mut harness, tab, id) = conflict_in(look, None);
            click_dialog(&mut harness, "Discard my changes");
            assert!(!asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (0, loaded()), "{said}");
            let workspace = harness.app.workspace(tab).unwrap();
            let edits = &workspace.object_tab(id).unwrap().edits;
            assert!(edits.gone.contains(&1) && edits.note.is_none(), "{said}");
        }
    }

    #[test]
    fn an_answer_in_the_questions_first_moment_is_not_taken() {
        for look in looks() {
            let said = look.name;
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            shown(&mut harness, false);
            click_dialog(&mut harness, "Use server values");
            harness.press(egui::Key::Escape, egui::Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            let loaded = text("user2@example.com");
            assert_eq!(state(&harness, tab, id), (1, loaded), "{said}");
            // Once it has been on screen for a moment, it is.
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
        }
    }

    #[test]
    fn the_question_is_headed_with_the_row_by_its_key() {
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let title = |gone: bool, look: Look| super::title(object, 1, gone, &look, locale);
        let changed = title(false, Look::standard());
        assert_eq!(changed.whole(), "Row id 2 changed on the server");
        // The name is a part of its own: it is what gives way in a line
        // that is too long.
        assert_eq!(changed.name, "id 2");
        assert_eq!(
            title(true, Look::standard()).whole(),
            "Row id 2 no longer exists on the server"
        );
        // The terminal's lower case is for the app's own words.
        assert_eq!(
            title(false, Look::omarchy()).whole(),
            "row id 2 changed on the server"
        );
        assert!(
            painted(&harness, "Row id 2 changed on the server"),
            "{:?}",
            harness.painted
        );
    }

    #[test]
    fn the_question_says_which_connection_and_table_its_row_is_of() {
        for look in looks() {
            let said = look.name;
            // Two connections, each with a table `users` open.
            let mut harness = Harness::new();
            harness.set_look(look);
            let (first, users) = harness.editable();
            harness.app.apply(Action::ShowConnections);
            let (second, others) = harness.editable();
            harness.app.workspace_mut(second).unwrap().name = "Bookshop".into();
            // A save of the first is answered while the second shows.
            retype(&mut harness, first, users, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: first,
                id: users,
            });
            let changed = |row: usize, email: &str| Conflict {
                row,
                server: Some(server(email)),
            };
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Fixture · users"), "{said}");
            assert!(!painted(&harness, "Bookshop · users"), "{said}");
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            // The same row of the other connection's table.
            retype(&mut harness, second, others, "bob@example.com");
            harness.app.apply(Action::WriteEdits {
                tab: second,
                id: others,
            });
            let conflict = changed(0, "eve@example.com");
            harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
            harness.finish_animations();
            assert!(painted(&harness, "Bookshop · users"), "{said}");
            assert!(!painted(&harness, "Fixture · users"), "{said}");
        }
        // With several rows, which of them: in the look's case.
        let (harness, tab, id) = conflict_in(Look::standard(), Some("eve@example.com"));
        let workspace = harness.app.workspace(tab).unwrap();
        let object = workspace.object_tab(id).unwrap();
        let locale = harness.app.locale;
        let place =
            |at: (usize, usize)| super::place(workspace, object, at, &Look::standard(), locale);
        assert_eq!(place((0, 1)), "Fixture · users");
        assert_eq!(place((0, 2)), "Fixture · users · 1 of 2");
        assert_eq!(place((1, 2)), "Fixture · users · 2 of 2");
    }

    #[test]
    fn the_sheet_shows_what_was_loaded_what_the_server_holds_and_yours() {
        for look in sheets() {
            let said = look.name;
            let (harness, ..) = conflict_in(look, Some("eve@example.com"));
            let palette = harness.app.palette;
            for text in [
                "Row id 2 changed on the server",
                "Fixture · users",
                "Someone saved it after you loaded it. Nothing was written.",
                "loaded",
                "now on server",
                "yours",
                "email",
            ] {
                piece(&harness, text);
            }
            let (danger, warning) = (
                Tone::Danger.fill(&look, &palette),
                Tone::Warning.fill(&look, &palette),
            );
            // What was loaded is plain.
            let (loaded, color) = piece(&harness, "user2@example.com");
            assert_eq!(color, palette.text, "{said}");
            assert!(!tinted(&harness, loaded, danger), "{said}");
            assert!(!tinted(&harness, loaded, warning), "{said}");
            // What the server holds now is another value: red on a red tint.
            let (server, color) = piece(&harness, "eve@example.com");
            assert_eq!(color, Tone::Danger.color(&palette), "{said}");
            assert!(tinted(&harness, server, danger), "{said}");
            // The user's is on the tint a pending cell has in the grid.
            let (yours, color) = piece(&harness, "bob@example.com");
            assert_eq!(color, palette.text, "{said}");
            assert!(tinted(&harness, yours, warning), "{said}");
            // Each under its header, in the header's order.
            let under = |header: &str, value: egui::Rect| {
                let (header, _) = piece(&harness, header);
                assert_eq!(header.left(), value.left(), "{said}");
                assert!(header.bottom() <= value.top(), "{said}");
            };
            under("loaded", loaded);
            under("now on server", server);
            under("yours", yours);
            assert!(loaded.right() <= server.left(), "{said}");
            assert!(server.right() <= yours.left(), "{said}");
        }
    }

    #[test]
    fn only_the_columns_the_user_changed_are_listed_and_a_value_the_server_kept_is_not_marked() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = fixture_in(look);
            let palette = harness.app.palette;
            pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
            pend(&mut harness, (tab, id), (1, 2), "[1]");
            let conflict = changed(0, server("eve@example.com"));
            saved(&mut harness, (tab, id), vec![conflict]);
            // Two lines, in the page's order.
            let ((email, _), (meta, _)) = (piece(&harness, "email"), piece(&harness, "meta"));
            assert!(email.bottom() <= meta.top(), "{said}");
            // The email is another value on the server and is marked; the
            // `meta` it still holds as it was loaded is not.
            let danger = Tone::Danger.fill(&look, &palette);
            assert_eq!(fills(&harness, danger).len(), 1, "{said}");
            let (moved, _) = piece(&harness, "eve@example.com");
            assert!(tinted(&harness, moved, danger), "{said}");
            let nulls = pieces(&harness, |piece| piece == "NULL");
            assert_eq!(nulls.len(), 2, "{said}: loaded and server");
            for (_, null, _) in nulls {
                let apart = (null.center().y - meta.center().y).abs();
                assert!(apart < 15.0, "{said}: in the meta line");
                assert!(!tinted(&harness, null, danger), "{said}");
                assert!(named(&mut harness, "NULL", null), "{said}");
            }
            let (yours, _) = piece(&harness, "[1]");
            let warning = Tone::Warning.fill(&look, &palette);
            assert!(tinted(&harness, yours, warning), "{said}");
            // With only the email pending, `meta` has no line.
            let (harness, ..) = conflict_in(look, Some("eve@example.com"));
            piece(&harness, "email");
            assert!(pieces(&harness, |piece| piece == "meta").is_empty());
            assert!(unsaid(&harness, "NULL"), "{said}");
        }
    }

    #[test]
    fn a_value_that_became_null_on_the_server_is_the_null_chip_on_the_red_tint() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = fixture_in(look);
            let palette = harness.app.palette;
            // The fixture's first row holds a document in `meta`.
            pend(&mut harness, (tab, id), (0, 2), "[1]");
            let now = vec![Value::Int(1), text("user1@example.com"), Value::Null];
            saved(&mut harness, (tab, id), vec![changed(0, now)]);
            let danger = Tone::Danger.fill(&look, &palette);
            let (loaded, color) = piece(&harness, r#"{"plan":"pro"}"#);
            assert_eq!(color, palette.text, "{said}");
            assert!(!tinted(&harness, loaded, danger), "{said}");
            // The fill says it changed, the chip says to what.
            let (null, _) = piece(&harness, "NULL");
            assert!(named(&mut harness, "NULL", null), "{said}");
            assert!(tinted(&harness, null, danger), "{said}");
            assert!(loaded.right() <= null.left(), "{said}");
            assert!((null.center().y - loaded.center().y).abs() < 15.0, "{said}");
        }
    }

    #[test]
    fn two_long_values_that_differ_past_the_cut_are_shown_from_where_they_differ() {
        let long = |end: &str| format!("{}{end}", "x".repeat(300));
        // In the window the app is at least, and in one too narrow for a
        // cell to hold what stands before the difference.
        for (look, width) in sheets().flat_map(|look| [(look, 1280.0), (look, 420.0)]) {
            let said = format!("{}, {width} wide", look.name);
            let mut page = crate::testing::page(5, false);
            page.rows[1][1] = text(&long("loaded"));
            let structure = crate::testing::fixture_structure();
            let size = egui::vec2(width, 800.0);
            let (mut harness, tab, id) = table_of(look, size, structure, page);
            pend(&mut harness, (tab, id), (1, 1), &long("yours"));
            let now = vec![Value::Int(2), text(&long("server")), Value::Null];
            saved(&mut harness, (tab, id), vec![changed(0, now)]);
            // Read from their start, the three are the same 256 letters.
            let shown = pieces(&harness, |piece| piece.starts_with('…'));
            let shown: Vec<&str> = shown.iter().map(|(piece, ..)| piece.as_str()).collect();
            assert_eq!(shown.len(), 3, "{said}: {shown:?}");
            for end in ["loaded", "server", "yours"] {
                let ends = shown.iter().filter(|piece| piece.ends_with(end));
                assert_eq!(ends.count(), 1, "{said}: {end} in {shown:?}");
            }
            assert!(
                shown[0] != shown[1] && shown[0] != shown[2] && shown[1] != shown[2],
                "{said}: {shown:?}"
            );
            // None of them is the value from its start.
            assert!(pieces(&harness, |piece| piece.starts_with("xxx")).is_empty());
        }
    }

    #[test]
    fn a_value_that_differs_by_a_line_break_alone_shows_its_break() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = fixture_in(look);
            let palette = harness.app.palette;
            pend(&mut harness, (tab, id), (1, 1), "a\nb");
            saved(&mut harness, (tab, id), vec![changed(0, server("a b"))]);
            let marks = crate::ui::grid::marks(&harness.ctx, &look);
            let broken = format!("a{}b", marks.line);
            let (yours, _) = piece(&harness, &broken);
            let warning = Tone::Warning.fill(&look, &palette);
            assert!(tinted(&harness, yours, warning), "{said}");
            let (now, _) = piece(&harness, "a b");
            let danger = Tone::Danger.fill(&look, &palette);
            assert!(tinted(&harness, now, danger), "{said}");
            assert!(now.right() <= yours.left(), "{said}");
        }
    }

    /// The fixture's rows `id 2` and `id 4` with their emails pending and
    /// saved, and both found changed.
    fn two_rows(look: Look) -> (Harness, ConnTabId, TabId) {
        let (mut harness, tab, id) = fixture_in(look);
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        pend(&mut harness, (tab, id), (3, 1), "dan@example.com");
        let fourth = vec![Value::Int(4), text("fay@example.com"), Value::Null];
        let conflicts = vec![changed(0, server("eve@example.com")), changed(1, fourth)];
        saved(&mut harness, (tab, id), conflicts);
        (harness, tab, id)
    }

    /// The fixture's rows `id 2`, `id 4` and `id 5` with their emails
    /// pending and saved, and all three found changed.
    fn three_rows(look: Look) -> (Harness, ConnTabId, TabId) {
        let (mut harness, tab, id) = fixture_in(look);
        let mut conflicts = Vec::new();
        for (at, row) in [1, 3, 4].into_iter().enumerate() {
            pend(
                &mut harness,
                (tab, id),
                (row, 1),
                &format!("mine{row}@x.io"),
            );
            let now = Value::Int(row as i64 + 1);
            let email = text(&format!("theirs{row}@x.io"));
            conflicts.push(changed(at, vec![now, email, Value::Null]));
        }
        saved(&mut harness, (tab, id), conflicts);
        (harness, tab, id)
    }

    /// Which of the save's rows the question is about. `None` once every
    /// one is answered.
    fn asked_about(harness: &Harness) -> Option<usize> {
        match &harness.app.dialog {
            Some(Dialog::Conflict(prompt)) => Some(prompt.at),
            _ => None,
        }
    }

    /// One frame that brings `key` going down, with what it types. The
    /// question has been up for long enough to take an answer, whichever
    /// row it has come to. A key that is down already is a repeat to egui.
    fn down(harness: &mut Harness, key: Key, typed: Option<&str>) {
        shown(harness, true);
        let mut events = vec![crate::testing::key(key, Modifiers::NONE)];
        events.extend(typed.map(|typed| egui::Event::Text(typed.into())));
        harness.frame(events);
    }

    /// Lets `key` go.
    fn up(harness: &mut Harness, key: Key) {
        harness.frame(vec![crate::testing::release(key, Modifiers::NONE)]);
    }

    /// Answers the first row of three with a fresh press of `key` and
    /// keeps it down: its repeats answer no row, however long each row's
    /// question has been up. Let go and pressed again, it answers the row
    /// that is shown, and its repeats nothing once more.
    fn held_through_the_queue(harness: &mut Harness, key: Key, typed: Option<&str>, said: &str) {
        assert_eq!(asked_about(harness), Some(0), "{said}");
        for answered in 1..=2 {
            down(harness, key, typed);
            assert_eq!(asked_about(harness), Some(answered), "{said}: a press");
            // Well over the half second of each row's first moment.
            for _ in 0..40 {
                down(harness, key, typed);
                assert_eq!(asked_about(harness), Some(answered), "{said}: held");
            }
            up(harness, key);
            assert_eq!(asked_about(harness), Some(answered), "{said}: let go");
        }
    }

    #[test]
    fn a_key_held_since_one_rows_answer_answers_no_row_after_it() {
        for look in looks() {
            // Space on each button, which the next row's question has in
            // the same place with the keyboard still on it.
            for name in ["Overwrite", "Use server values", "Keep mine, reload row"] {
                let said = format!("{}, Space on {name}", look.name);
                let (mut harness, tab, id) = three_rows(look);
                focus_dialog(&mut harness, name);
                held_through_the_queue(&mut harness, Key::Space, None, &said);
                assert_eq!(focused_name(&harness.settle()), name, "{said}");
                // Two rows were answered, by the two presses.
                let left = pending_cells(&harness, tab, id).len();
                let dropped = name == "Use server values";
                assert_eq!(left, if dropped { 1 } else { 3 }, "{said}");
                assert_eq!(writes(&harness), 1, "{said}");
            }
            // Enter on the one button it presses.
            let said = format!("{}, Enter", look.name);
            let (mut harness, ..) = three_rows(look);
            focus_dialog(&mut harness, "Keep mine, reload row");
            held_through_the_queue(&mut harness, Key::Enter, None, &said);
            // Esc, wherever the keyboard is.
            let said = format!("{}, Esc", look.name);
            let (mut harness, tab, id) = three_rows(look);
            held_through_the_queue(&mut harness, Key::Escape, None, &said);
            assert_eq!(pending_cells(&harness, tab, id).len(), 3, "{said}");
            // The last row is answered by a press of its own.
            down(&mut harness, Key::Escape, None);
            assert_eq!(asked_about(&harness), None, "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
        }
    }

    #[test]
    fn a_letter_held_since_one_rows_answer_answers_no_row_after_it() {
        let look = Look::omarchy();
        for (key, letter) in [(Key::O, "o"), (Key::S, "s"), (Key::K, "k")] {
            let (mut harness, tab, id) = three_rows(look);
            held_through_the_queue(&mut harness, key, Some(letter), letter);
            let left = pending_cells(&harness, tab, id).len();
            assert_eq!(left, if letter == "s" { 1 } else { 3 }, "{letter}");
            assert_eq!(writes(&harness), 1, "{letter}");
        }
        // Two rows, the second of them gone: `k` held from the first keeps
        // nothing of the second.
        let (mut harness, tab, id) = fixture_in(look);
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        pend(&mut harness, (tab, id), (3, 1), "dan@example.com");
        let gone = Conflict {
            row: 1,
            server: None,
        };
        let conflicts = vec![changed(0, server("eve@example.com")), gone];
        saved(&mut harness, (tab, id), conflicts);
        down(&mut harness, Key::K, Some("k"));
        for _ in 0..40 {
            down(&mut harness, Key::K, Some("k"));
            assert_eq!(asked_about(&harness), Some(1));
        }
        up(&mut harness, Key::K);
        down(&mut harness, Key::D, Some("d"));
        assert_eq!(asked_about(&harness), None);
        assert_eq!(pending_cells(&harness, tab, id), [(1, 1)]);
    }

    #[test]
    fn the_place_line_counts_several_rows() {
        for look in sheets() {
            let said = look.name;
            // One row: where it is, on a line of its own under the title.
            let (harness, ..) = conflict_in(look, Some("eve@example.com"));
            let palette = harness.app.palette;
            let (title, _) = piece(&harness, "Row id 2 changed on the server");
            let (place, color) = piece(&harness, "Fixture · users");
            assert_eq!(color, palette.secondary, "{said}");
            assert!(title.bottom() <= place.top(), "{said}");
            assert_eq!(title.left(), place.left(), "{said}");
            assert!(pieces(&harness, |piece| piece.contains(" of ")).is_empty());
            // Two: which of them, too.
            let (mut harness, ..) = two_rows(look);
            let (_, color) = piece(&harness, "Fixture · users · 1 of 2");
            assert_eq!(color, palette.secondary, "{said}");
            piece(&harness, "Row id 2 changed on the server");
            click_dialog(&mut harness, "Use server values");
            shown(&mut harness, true);
            harness.settle();
            piece(&harness, "Fixture · users · 2 of 2");
            piece(&harness, "Row id 4 changed on the server");
            piece(&harness, "fay@example.com");
            assert!(unsaid(&harness, "eve@example.com"), "{said}");
        }
    }

    #[test]
    fn an_answer_names_the_row_the_sheet_drew() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = two_rows(look);
            let cells = |harness: &Harness| {
                let workspace = harness.app.workspace(tab).unwrap();
                let edits = &workspace.object_tab(id).unwrap().edits;
                edits.cells.keys().copied().collect::<Vec<_>>()
            };
            click_dialog(&mut harness, "Use server values");
            assert_eq!(cells(&harness), [(3, 1)], "{said}");
            // An answer that names the row before is none to this one:
            // Esc pressed twice, its second a frame behind.
            let late = crate::edit::Answer::KeepMine;
            let answer = Action::AnswerConflict {
                at: 0,
                answer: late,
            };
            harness.app.apply(answer);
            assert!(asking(&harness), "{said}");
            assert_eq!(cells(&harness), [(3, 1)], "{said}");
            // The sheet's own names the row it shows.
            shown(&mut harness, true);
            click_dialog(&mut harness, "Use server values");
            assert!(!asking(&harness), "{said}");
            assert!(cells(&harness).is_empty(), "{said}");
        }
    }

    #[test]
    fn a_long_key_gives_way_and_the_title_says_it_whole() {
        // The title as it is painted: the one line that ends as it does.
        let title = |harness: &Harness| {
            let found = pieces(harness, |piece| piece.ends_with("changed on the server"));
            let [(text, rect, _)] = &found[..] else {
                panic!("{found:?}");
            };
            (text.clone(), *rect)
        };
        for look in sheets() {
            let said = look.name;
            // A table keyed by `email`, holding a UUID and then a key as
            // long as a text key gets to be shown.
            for key in [
                "0199a3f2-7c1e-7abc-8def-0123456789ab".to_owned(),
                "k".repeat(120),
            ] {
                let mut structure = crate::testing::fixture_structure();
                structure.primary_key = vec!["email".into()];
                let mut page = crate::testing::page(5, false);
                page.rows[1][1] = text(&key);
                let size = egui::vec2(1280.0, 800.0);
                let (mut harness, tab, id) = table_of(look, size, structure, page);
                pend(&mut harness, (tab, id), (1, 2), "[1]");
                let now = vec![Value::Int(2), text(&key), text("[2]")];
                saved(&mut harness, (tab, id), vec![changed(0, now)]);
                let whole = format!("Row email {key} changed on the server");
                let (painted_as, rect) = title(&harness);
                // Inside the sheet's margins, on one line.
                assert!(sheet(&harness).shrink(19.0).contains_rect(rect), "{said}");
                assert!(rect.height() < 30.0, "{said}");
                assert!(painted_as.starts_with("Row email "), "{said}");
                assert_eq!(painted_as.contains('…'), painted_as != whole, "{said}");
                if key.len() == 120 {
                    assert!(painted_as.contains("k…"), "{said}: {painted_as}");
                }
                // A screen reader is told the whole of it.
                assert!(named(&mut harness, &whole, rect), "{said}");
                // And so is the pointer, where the name gave way: the
                // whole title is on screen once, as the line or over it.
                let whole_now = |harness: &Harness| pieces(harness, |piece| piece == whole).len();
                assert_eq!(whole_now(&harness), usize::from(painted_as == whole));
                hover(&mut harness, rect.center());
                assert_eq!(whole_now(&harness), 1, "{said}");
            }
            // A key of two columns.
            let mut structure = crate::testing::fixture_structure();
            structure.primary_key = vec!["id".into(), "email".into()];
            let page = crate::testing::page(5, false);
            let size = egui::vec2(1280.0, 800.0);
            let (mut harness, tab, id) = table_of(look, size, structure, page);
            pend(&mut harness, (tab, id), (1, 2), "[1]");
            let now = vec![Value::Int(2), text("user2@example.com"), text("[2]")];
            saved(&mut harness, (tab, id), vec![changed(0, now)]);
            let whole = "Row id 2, email user2@example.com changed on the server";
            assert!(harness.has(whole), "{said}");
            let (_, rect) = title(&harness);
            assert!(sheet(&harness).shrink(19.0).contains_rect(rect), "{said}");
        }
    }

    #[test]
    fn enter_answers_only_as_keep_mine() {
        for look in sheets() {
            let said = look.name;
            let loaded = || text("user2@example.com");
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            // With the keyboard on no button.
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            // On the primary button: Return does not press it.
            focus_dialog(&mut harness, "Overwrite");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            assert_eq!(state(&harness, tab, id), (1, loaded()), "{said}");
            focus_dialog(&mut harness, "Use server values");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            assert_eq!(state(&harness, tab, id), (1, loaded()), "{said}");
            // On Keep mine it is that button's answer.
            focus_dialog(&mut harness, "Keep mine, reload row");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(
                state(&harness, tab, id),
                (1, text("eve@example.com")),
                "{said}"
            );
            assert_eq!(writes(&harness), 1, "{said}");
            // Space presses the button that has the keyboard, as everywhere.
            let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
            focus_dialog(&mut harness, "Overwrite");
            harness.press(Key::Space, Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
            assert_eq!(writes(&harness), 2, "{said}");
        }
    }

    #[test]
    fn no_key_and_no_click_answers_in_the_questions_first_moment() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
            shown(&mut harness, false);
            let untouched = |harness: &Harness| {
                let loaded = text("user2@example.com");
                asking(harness) && writes(harness) == 1 && state(harness, tab, id) == (1, loaded)
            };
            // Enter and Space on the button that Enter does press.
            focus_dialog(&mut harness, "Keep mine, reload row");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(untouched(&harness), "{said}: Enter");
            harness.press(Key::Space, Modifiers::NONE);
            assert!(untouched(&harness), "{said}: Space");
            // Space on the two others, a click on each of the three, and
            // Esc.
            for name in ["Use server values", "Overwrite"] {
                focus_dialog(&mut harness, name);
                harness.press(Key::Space, Modifiers::NONE);
                assert!(untouched(&harness), "{said}: Space on {name}");
            }
            for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
                click_dialog(&mut harness, name);
                assert!(untouched(&harness), "{said}: a click on {name}");
            }
            harness.press(Key::Escape, Modifiers::NONE);
            assert!(untouched(&harness), "{said}: Esc");
            // Nothing was kept for later: the moment over, it is still
            // up, and the same key answers.
            shown(&mut harness, true);
            harness.settle();
            assert!(untouched(&harness), "{said}");
            focus_dialog(&mut harness, "Use server values");
            harness.press(Key::Space, Modifiers::NONE);
            assert!(!asking(&harness), "{said}");
        }
    }

    #[test]
    fn the_tab_key_comes_to_keep_mine_first() {
        for look in sheets() {
            let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
            harness.press(Key::Tab, Modifiers::NONE);
            let name = focused_name(&harness.settle());
            assert_eq!(name, "Keep mine, reload row", "{}", look.name);
            // Then the two at the right, as they are read.
            harness.press(Key::Tab, Modifiers::NONE);
            let name = focused_name(&harness.settle());
            assert_eq!(name, "Use server values", "{}", look.name);
            harness.press(Key::Tab, Modifiers::NONE);
            assert_eq!(
                focused_name(&harness.settle()),
                "Overwrite",
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn keep_mine_reads_as_a_link_and_overwrite_as_the_primary_button() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
            let palette = harness.app.palette;
            let tree = harness.settle();
            let place = |name: &str| {
                crate::testing::bounds(&tree, name, Role::Button)
                    .unwrap_or_else(|| panic!("{said}: no button {name}"))
            };
            let outlined = |place: egui::Rect| {
                let mut outlines = harness.outlines.iter();
                outlines.any(|(rect, _)| *rect == place)
            };
            let filled = |place: egui::Rect| {
                let mut fills = harness.fills.iter();
                fills
                    .find(|(rect, _)| *rect == place)
                    .map(|(_, fill)| *fill)
            };
            let (keep, server, overwrite) = (
                place("Keep mine, reload row"),
                place("Use server values"),
                place("Overwrite"),
            );
            // A link: its words in the accent colour, and nothing round or
            // behind them.
            let (_, color) = piece(&harness, "Keep mine, reload row");
            assert_eq!(color, palette.accent, "{said}");
            assert!(!outlined(keep) && filled(keep).is_none(), "{said}");
            // A bordered button, and the primary one: filled with the ink,
            // as the Leave prompt's Save is.
            assert!(outlined(server), "{said}");
            assert_eq!(filled(overwrite), Some(palette.text), "{said}");
            let (_, color) = piece(&harness, "Overwrite");
            assert_eq!(color, palette.window, "{said}");
            // Keep mine at the sheet's left, the two others at its right,
            // 8 apart, all on one row.
            let (title, _) = piece(&harness, "Row id 2 changed on the server");
            assert_eq!(keep.left(), title.left(), "{said}");
            assert_eq!(server.right() + 8.0, overwrite.left(), "{said}");
            assert!(keep.right() < server.left(), "{said}");
            assert_eq!(keep.y_range(), overwrite.y_range(), "{said}");
            assert_eq!(overwrite.height(), 32.0, "{said}");
        }
    }

    #[test]
    fn a_row_that_is_gone_has_its_own_words_and_one_button() {
        for look in sheets() {
            let said = look.name;
            let (mut harness, tab, id) = conflict_in(look, None);
            let palette = harness.app.palette;
            for text in [
                "Row id 2 no longer exists on the server",
                "Fixture · users",
                "Someone deleted it after you loaded it. Nothing was written.",
                "loaded",
                "yours",
                "email",
                "user2@example.com",
            ] {
                piece(&harness, text);
            }
            // Nothing is on the server to show, and nothing is marked as
            // changed there.
            assert!(unsaid(&harness, "now on server"), "{said}");
            let danger = Tone::Danger.fill(&look, &palette);
            assert!(fills(&harness, danger).is_empty(), "{said}");
            let (yours, _) = piece(&harness, "bob@example.com");
            let warning = Tone::Warning.fill(&look, &palette);
            assert!(tinted(&harness, yours, warning), "{said}");
            // The two columns share what three do.
            let ((loaded, _), (mine, _)) = (piece(&harness, "loaded"), piece(&harness, "yours"));
            let inner = sheet(&harness).width() - 40.0;
            let apart = mine.left() - loaded.left();
            // (The table's line takes a point at each side.)
            assert!(
                (apart - (inner - 80.0) / 2.0).abs() < 3.0,
                "{said}: {apart}"
            );
            // One answer, which is not the primary button and which Enter
            // does not give.
            assert_eq!(pressable(&mut harness, "Discard my changes").len(), 1);
            for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
                assert!(pressable(&mut harness, name).is_empty(), "{said}: {name}");
            }
            let tree = harness.settle();
            let place = crate::testing::bounds(&tree, "Discard my changes", Role::Button);
            let place = place.expect("the button");
            let mut outlines = harness.outlines.iter();
            assert!(outlines.any(|(rect, _)| *rect == place), "{said}");
            focus_dialog(&mut harness, "Discard my changes");
            harness.press(Key::Enter, Modifiers::NONE);
            assert!(asking(&harness), "{said}");
            assert_eq!(
                state(&harness, tab, id),
                (1, text("user2@example.com")),
                "{said}"
            );
        }
    }

    #[test]
    fn a_long_value_is_cut_to_its_cell_and_whole_under_the_pointer() {
        for look in sheets() {
            let said = look.name;
            let long = format!("{}@example.com", "b".repeat(108));
            assert_eq!(long.len(), 120);
            let (mut harness, tab, id) = fixture_in(look);
            let palette = harness.app.palette;
            pend(&mut harness, (tab, id), (1, 1), &long);
            let conflict = changed(0, server("eve@example.com"));
            saved(&mut harness, (tab, id), vec![conflict]);
            let found = pieces(&harness, |piece| piece.starts_with("bbb"));
            let [(yours, place, _)] = &found[..] else {
                panic!("{said}: {found:?}");
            };
            assert!(yours.ends_with('…') && yours.len() < 60, "{said}: {yours}");
            let warning = Tone::Warning.fill(&look, &palette);
            assert!(tinted(&harness, *place, warning), "{said}");
            // A screen reader is told the whole value; it is painted whole
            // only once the pointer is over it.
            assert!(named(&mut harness, &long, *place), "{said}");
            assert!(unsaid(&harness, &long), "{said}");
            hover(&mut harness, place.center());
            piece(&harness, &long);
            // A value that fits says nothing more under the pointer.
            let (server, _) = piece(&harness, "eve@example.com");
            hover(&mut harness, server.center());
            piece(&harness, "eve@example.com");
            assert!(unsaid(&harness, &long), "{said}");
        }
    }

    /// A table of twelve columns of text beside its key, all of them
    /// changed in the row `id 2` and found changed by the save, in `look`
    /// and in a window of `size`: more lines than the question shows at
    /// once. The column `c1` was `was c1`, is `now c1` on the server and
    /// `my c1` pending, and so on to `c12`.
    fn twelve_columns(look: Look, size: egui::Vec2) -> (Harness, ConnTabId, TabId) {
        let names: Vec<String> = (1..=12).map(|at| format!("c{at}")).collect();
        let mut structure = crate::testing::fixture_structure();
        structure.columns.truncate(1);
        let mut page = crate::testing::page(5, false);
        page.columns.truncate(1);
        for name in &names {
            structure.columns.push(tabletist_db::ColumnInfo {
                name: name.clone(),
                type_name: "TEXT".into(),
                nullable: true,
                ..tabletist_db::ColumnInfo::default()
            });
            page.columns.push(tabletist_db::ColumnMeta {
                name: name.clone(),
                type_name: "TEXT".into(),
                kind: tabletist_db::ValueKind::Text,
            });
        }
        for row in &mut page.rows {
            row.truncate(1);
            row.extend(names.iter().map(|name| text(&format!("was {name}"))));
        }
        let (mut harness, tab, id) = table_of(look, size, structure, page);
        for (at, name) in names.iter().enumerate() {
            pend(&mut harness, (tab, id), (1, at + 1), &format!("my {name}"));
        }
        let mut now = vec![Value::Int(2)];
        now.extend(names.iter().map(|name| text(&format!("now {name}"))));
        saved(&mut harness, (tab, id), vec![changed(0, now)]);
        (harness, tab, id)
    }

    #[test]
    fn the_sheet_fits_a_small_window() {
        for look in sheets() {
            let said = look.name;
            // Twelve columns of text beside the key, all of them changed
            // in one row.
            let size = egui::vec2(720.0, 480.0);
            let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let (mut harness, ..) = twelve_columns(look, size);
            assert!(window.contains_rect(sheet(&harness)), "{said}");
            let tree = harness.settle();
            for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
                let place = crate::testing::bounds(&tree, name, Role::Button);
                let place = place.unwrap_or_else(|| panic!("{said}: no button {name}"));
                assert!(window.contains_rect(place), "{said}: {name} at {place:?}");
                assert!(sheet(&harness).contains_rect(place), "{said}: {name}");
            }
            // More lines than fit scroll: the first is drawn, the last is
            // not.
            piece(&harness, "my c1");
            assert!(unsaid(&harness, "my c12"), "{said}");
        }
    }

    /// The cells pending in the table, as row and column.
    fn pending_cells(harness: &Harness, tab: ConnTabId, id: TabId) -> Vec<(usize, usize)> {
        let workspace = harness.app.workspace(tab).unwrap();
        let edits = &workspace.object_tab(id).unwrap().edits;
        edits.cells.keys().copied().collect()
    }

    /// The pieces the question painted on the line of `name`, from the
    /// left: what each says, and in what colour.
    fn line_of(harness: &Harness, name: &str) -> Vec<(String, egui::Color32)> {
        let (at, _) = piece(harness, name);
        let mut line = pieces(harness, |_| true);
        line.retain(|(_, rect, _)| (rect.center().y - at.center().y).abs() < 6.0);
        line.sort_by(|(_, a, _), (_, b, _)| a.left().total_cmp(&b.left()));
        line.into_iter()
            .map(|(piece, _, color)| (piece, color))
            .collect()
    }

    /// The hints of the box's foot for a row that changed, in their order.
    const HINTS: [&str; 3] = ["[o] overwrite", "[s] use server", "[k] keep mine, reload"];

    #[test]
    fn the_box_says_conflict_and_writes_the_values_in_their_colours() {
        let look = Look::omarchy();
        let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
        let palette = harness.app.palette;
        let frame = sheet(&harness);
        // The first line: the word in the warning colour, then the row.
        let (word, color) = piece(&harness, "conflict");
        assert_eq!(color, palette.warning);
        let (title, color) = piece(&harness, "row id 2 changed on the server");
        assert_eq!(color, palette.text);
        assert!(word.right() < title.left(), "{word:?} {title:?}");
        assert!((word.center().y - title.center().y).abs() < 1.0);
        // Its mark stands before the word, where the look's font has it.
        let mut texts = harness.painted.iter();
        let at = texts.position(|(piece, _)| piece == "conflict").unwrap();
        let font = TextRole::OGroup.font_id(look.faces);
        let mut left = word.left();
        if harness.ctx.fonts_mut(|fonts| fonts.has_glyphs(&font, "≠")) {
            assert_eq!(harness.painted[at - 1], ("≠".to_owned(), palette.warning));
            let (_, mark) = harness.text_rects[at - 1];
            assert!(mark.right() < word.left(), "{mark:?} {word:?}");
            assert!((mark.center().y - word.center().y).abs() < 3.0);
            left = mark.left();
        }
        // Where the row is and what happened, each on its own line under
        // that, dim.
        let (place, color) = piece(&harness, "Fixture · users");
        assert_eq!(color, palette.dim);
        assert!(title.bottom() <= place.top());
        assert_eq!(place.left(), left);
        let sentence = "someone saved it after you loaded it. nothing was written.";
        let (sentence, color) = piece(&harness, sentence);
        assert_eq!(color, palette.dim);
        assert!(place.bottom() <= sentence.top());
        // The values under their headers: the loaded one plain, the
        // server's in the danger colour, the user's in the warning colour.
        let (name, color) = piece(&harness, "email");
        assert_eq!((color, name.left()), (palette.text, left));
        let values = [
            ("loaded", "user2@example.com", palette.text),
            ("server", "eve@example.com", palette.danger),
            ("yours", "bob@example.com", palette.warning),
        ]
        .map(|(header, value, color)| {
            let (header, dim) = piece(&harness, header);
            assert_eq!(dim, palette.dim, "{value}");
            let (value, painted) = piece(&harness, value);
            assert_eq!(painted, color, "{value:?}");
            assert_eq!(header.left(), value.left());
            assert!(sentence.bottom() <= header.top());
            assert!(header.bottom() <= value.top());
            assert!((value.center().y - name.center().y).abs() < 3.0);
            value
        });
        assert!(unsaid(&harness, "now on server"));
        // A first column of 90, and the three others equal and 10 apart.
        assert!((values[0].left() - left - 100.0).abs() < 0.5);
        let step = values[1].left() - values[0].left();
        assert!((values[2].left() - values[1].left() - step).abs() < 0.5);
        let inner = frame.right() - 20.0 - left;
        assert!(
            (step - (inner - 90.0) / 3.0).abs() < 1.0,
            "{step} of {inner}"
        );
        // No rules and no fills: the colours say it all.
        for tone in [Tone::Danger, Tone::Warning] {
            assert!(fills(&harness, tone.fill(&look, &palette)).is_empty());
        }
        // The keys in the foot, under the values: each letter in the
        // accent colour, in the design's order, ending at the foot's right.
        let hints = HINTS.map(|hint| {
            let (place, color) = piece(&harness, hint);
            assert_eq!(color, palette.accent, "{hint}");
            assert!(values[2].bottom() < place.top(), "{hint}");
            assert!(frame.contains_rect(place), "{hint}");
            assert!(place.top() > frame.bottom() - 43.0, "{hint}");
            place
        });
        assert!(hints[0].right() < hints[1].left() && hints[1].right() < hints[2].left());
        let end = frame.right() - hints[2].right();
        assert!((14.0..=17.0).contains(&end), "{end}");
        // Each hint is its button too.
        let tree = harness.settle();
        for (name, hint) in ["Overwrite", "Use server values", "Keep mine, reload row"]
            .iter()
            .zip(hints)
        {
            let button = crate::testing::bounds(&tree, name, Role::Button);
            let button = button.unwrap_or_else(|| panic!("no button {name}"));
            assert!(button.contains(hint.center()), "{name}");
        }
        // Nothing says how to scroll lines that all fit.
        assert!(pieces(&harness, |piece| piece.starts_with("pgup")).is_empty());
    }

    #[test]
    fn a_value_the_server_kept_is_written_plain_and_one_that_became_null_in_the_danger_colour() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = fixture_in(look);
        let palette = harness.app.palette;
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        pend(&mut harness, (tab, id), (1, 2), "[1]");
        let conflict = changed(0, server("eve@example.com"));
        saved(&mut harness, (tab, id), vec![conflict]);
        // Two lines, in the page's order.
        let ((email, _), (meta, _)) = (piece(&harness, "email"), piece(&harness, "meta"));
        assert!(email.bottom() <= meta.top());
        // The `meta` the server still holds as it was loaded is the look's
        // own NULL, as the loaded one is.
        let plain = |text: &str, color: egui::Color32| (text.to_owned(), color);
        assert_eq!(
            line_of(&harness, "meta"),
            [
                plain("meta", palette.text),
                plain("NULL", palette.faint),
                plain("NULL", palette.faint),
                plain("[1]", palette.warning),
            ]
        );
        // A value that became NULL there: the word, in the danger colour.
        let (mut harness, tab, id) = fixture_in(look);
        pend(&mut harness, (tab, id), (0, 2), "[1]");
        let now = vec![Value::Int(1), text("user1@example.com"), Value::Null];
        saved(&mut harness, (tab, id), vec![changed(0, now)]);
        assert_eq!(
            line_of(&harness, "meta"),
            [
                plain("meta", palette.text),
                plain(r#"{"plan":"pro"}"#, palette.text),
                plain("NULL", palette.danger),
                plain("[1]", palette.warning),
            ]
        );
        let (null, _) = piece(&harness, "NULL");
        assert!(named(&mut harness, "NULL", null));
    }

    #[test]
    fn k_s_and_o_answer_and_d_discards_a_row_that_is_gone() {
        let look = Look::omarchy();
        let (eve, loaded) = (|| text("eve@example.com"), || text("user2@example.com"));
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        type_key(&mut harness, Key::K, "k");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, eve()));
        assert_eq!(writes(&harness), 1, "nothing is saved again");
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, eve()));
        assert_eq!(writes(&harness), 1);
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        type_key(&mut harness, Key::O, "o");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, eve()));
        assert_eq!(writes(&harness), 2, "the save runs again");
        // A row that is gone: `d` discards its changes.
        let (mut harness, tab, id) = conflict_in(look, None);
        type_key(&mut harness, Key::D, "d");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, loaded()));
        let workspace = harness.app.workspace(tab).unwrap();
        assert!(workspace.object_tab(id).unwrap().edits.gone.contains(&1));
        // And `k` keeps them, as Esc does: the line says the row is gone.
        let (mut harness, tab, id) = conflict_in(look, None);
        type_key(&mut harness, Key::K, "k");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, loaded()));
        assert!(
            harness.has("conflict row id 2 no longer exists on the server. nothing was written.")
        );
    }

    #[test]
    fn a_letter_the_row_does_not_offer_does_nothing() {
        let look = Look::omarchy();
        let changed: &[(Key, &str)] = &[(Key::D, "d"), (Key::J, "j"), (Key::X, "x")];
        let gone: &[(Key, &str)] = &[(Key::O, "o"), (Key::S, "s"), (Key::J, "j"), (Key::X, "x")];
        for (email, letters) in [(Some("eve@example.com"), changed), (None, gone)] {
            let (mut harness, tab, id) = conflict_in(look, email);
            let seen = |harness: &Harness| {
                let workspace = harness.app.workspace(tab).unwrap();
                let object = workspace.object_tab(id).unwrap();
                let gone = object.edits.gone.len();
                (state(harness, tab, id), object.selection, gone)
            };
            let before = seen(&harness);
            assert_eq!(before.0, (1, text("user2@example.com")));
            for (key, letter) in letters {
                type_key(&mut harness, *key, letter);
                assert!(asking(&harness), "{letter}");
                assert_eq!(writes(&harness), 1, "{letter}");
                assert_eq!(seen(&harness), before, "{letter}");
            }
        }
    }

    #[test]
    fn a_held_letter_is_no_answer() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = fixture_in(look);
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        // The key is down when the save is answered, and stays down: what
        // it repeats is typed into every frame, and is no answer. (egui
        // reads a press of a key that is down already as a repeat.)
        let held = |harness: &mut Harness, key: Key, letter: &str| {
            let down = crate::testing::key(key, Modifiers::NONE);
            harness.frame(vec![down, egui::Event::Text(letter.into())]);
        };
        harness.frame(vec![crate::testing::key(Key::O, Modifiers::NONE)]);
        let conflict = changed(0, server("eve@example.com"));
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        shown(&mut harness, true);
        for _ in 0..3 {
            held(&mut harness, Key::O, "o");
            assert!(asking(&harness));
            assert_eq!(writes(&harness), 1);
        }
        // Let go and pressed again, it answers nothing while it is held
        // either: the letter of another answer the same.
        harness.frame(vec![crate::testing::release(Key::O, Modifiers::NONE)]);
        harness.frame(vec![crate::testing::key(Key::S, Modifiers::NONE)]);
        for _ in 0..3 {
            held(&mut harness, Key::S, "s");
            assert!(asking(&harness));
            assert_eq!(state(&harness, tab, id), (1, text("user2@example.com")));
        }
        // Typed once it is let go, the letter is its answer.
        harness.frame(vec![crate::testing::release(Key::S, Modifiers::NONE)]);
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, text("eve@example.com")));
    }

    #[test]
    fn the_letters_wait_out_the_questions_first_moment() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        shown(&mut harness, false);
        for (key, letter) in [(Key::O, "o"), (Key::S, "s"), (Key::K, "k")] {
            type_key(&mut harness, key, letter);
            assert!(asking(&harness), "{letter}");
            assert_eq!(writes(&harness), 1, "{letter}");
            assert_eq!(state(&harness, tab, id), (1, text("user2@example.com")));
        }
        // Nothing was kept for later: the moment over, it is still up,
        // and the same letter answers.
        shown(&mut harness, true);
        harness.settle();
        assert!(asking(&harness));
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, text("eve@example.com")));
        // So with the one letter of a row that is gone.
        let (mut harness, tab, id) = conflict_in(look, None);
        shown(&mut harness, false);
        type_key(&mut harness, Key::D, "d");
        assert!(asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, text("user2@example.com")));
        shown(&mut harness, true);
        type_key(&mut harness, Key::D, "d");
        assert!(!asking(&harness));
    }

    /// The box about the fixture's row `id 2`, which holds another email
    /// now, as if it had only just come up. With it, whether nothing has
    /// come of what was done to it since.
    fn just_asked() -> (Harness, impl Fn(&Harness) -> bool) {
        let (mut harness, tab, id) = conflict_in(Look::omarchy(), Some("eve@example.com"));
        shown(&mut harness, false);
        let untouched = move |harness: &Harness| {
            let loaded = text("user2@example.com");
            asking(harness) && writes(harness) == 1 && state(harness, tab, id) == (1, loaded)
        };
        (harness, untouched)
    }

    #[test]
    fn enter_and_space_wait_out_the_boxs_first_moment() {
        let (mut harness, untouched) = just_asked();
        // Enter on the one button that Enter does press.
        focus_dialog(&mut harness, "Keep mine, reload row");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(untouched(&harness), "Enter");
        // Nothing was kept for later: the moment over, it is still up, and
        // the same key answers.
        shown(&mut harness, true);
        harness.settle();
        assert!(untouched(&harness));
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(!asking(&harness));
        // Space on each of the three.
        for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
            let (mut harness, untouched) = just_asked();
            focus_dialog(&mut harness, name);
            harness.press(Key::Space, Modifiers::NONE);
            assert!(untouched(&harness), "Space on {name}");
            shown(&mut harness, true);
            harness.press(Key::Space, Modifiers::NONE);
            assert!(!asking(&harness), "Space on {name}");
        }
    }

    #[test]
    fn a_click_on_a_key_hint_waits_out_the_boxs_first_moment() {
        // By the pointer, on the hint as it is painted.
        for hint in HINTS {
            let (mut harness, untouched) = just_asked();
            let (place, _) = piece(&harness, hint);
            click_at(&mut harness, place.center());
            assert!(untouched(&harness), "a click on {hint}");
            // The moment over, the same click answers.
            shown(&mut harness, true);
            harness.settle();
            assert!(untouched(&harness), "{hint}");
            click_at(&mut harness, place.center());
            assert!(!asking(&harness), "a click on {hint}");
        }
        // By a screen reader, on the hint's button.
        for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
            let (mut harness, untouched) = just_asked();
            click_dialog(&mut harness, name);
            assert!(untouched(&harness), "a press of {name}");
            shown(&mut harness, true);
            click_dialog(&mut harness, name);
            assert!(!asking(&harness), "a press of {name}");
        }
    }

    #[test]
    fn escape_waits_out_the_boxs_first_moment() {
        let (mut harness, untouched) = just_asked();
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(untouched(&harness));
        // The moment over, Esc keeps the user's changes over the row as
        // the server holds it.
        shown(&mut harness, true);
        harness.settle();
        assert!(untouched(&harness));
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(!asking(&harness));
        assert_eq!(writes(&harness), 1);
    }

    #[test]
    fn enter_in_the_box_answers_only_as_keep_mine() {
        let look = Look::omarchy();
        let loaded = || text("user2@example.com");
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        // With the keyboard on no button.
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(asking(&harness));
        assert_eq!(writes(&harness), 1);
        // The Tab key comes to the hints as they stand: `[o]` first, and
        // Enter does not overwrite.
        harness.press(Key::Tab, Modifiers::NONE);
        assert_eq!(focused_name(&harness.settle()), "Overwrite");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(asking(&harness));
        assert_eq!(writes(&harness), 1);
        assert_eq!(state(&harness, tab, id), (1, loaded()));
        focus_dialog(&mut harness, "Use server values");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, loaded()));
        // On Keep mine it is that button's answer.
        focus_dialog(&mut harness, "Keep mine, reload row");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, text("eve@example.com")));
        assert_eq!(writes(&harness), 1);
        // Space presses the button that has the keyboard, as everywhere.
        let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
        focus_dialog(&mut harness, "Overwrite");
        harness.press(Key::Space, Modifiers::NONE);
        assert!(!asking(&harness));
        assert_eq!(writes(&harness), 2);
        // Enter never discards either.
        let (mut harness, tab, id) = conflict_in(look, None);
        focus_dialog(&mut harness, "Discard my changes");
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, loaded()));
    }

    #[test]
    fn several_rows_are_asked_one_after_another_in_the_box() {
        let (mut harness, tab, id) = two_rows(Look::omarchy());
        let palette = harness.app.palette;
        let (_, color) = piece(&harness, "Fixture · users · 1 of 2");
        assert_eq!(color, palette.dim);
        piece(&harness, "row id 2 changed on the server");
        type_key(&mut harness, Key::S, "s");
        assert!(asking(&harness));
        shown(&mut harness, true);
        harness.settle();
        let (_, color) = piece(&harness, "Fixture · users · 2 of 2");
        assert_eq!(color, palette.dim);
        piece(&harness, "row id 4 changed on the server");
        piece(&harness, "fay@example.com");
        assert!(unsaid(&harness, "eve@example.com"));
        assert_eq!(pending_cells(&harness, tab, id), [(3, 1)]);
        type_key(&mut harness, Key::K, "k");
        assert!(!asking(&harness));
        assert_eq!(pending_cells(&harness, tab, id), [(3, 1)]);
        assert_eq!(writes(&harness), 1);
    }

    #[test]
    fn an_answer_names_the_row_the_box_drew() {
        let (mut harness, tab, id) = two_rows(Look::omarchy());
        type_key(&mut harness, Key::K, "k");
        assert!(asking(&harness));
        assert_eq!(pending_cells(&harness, tab, id), [(1, 1), (3, 1)]);
        // An answer that names the row before is none to this one: `k`
        // typed twice, its second a frame behind.
        let answer = crate::edit::Answer::KeepMine;
        harness.app.apply(Action::AnswerConflict { at: 0, answer });
        assert!(asking(&harness));
        // The box's own names the row it shows.
        shown(&mut harness, true);
        harness.settle();
        piece(&harness, "row id 4 changed on the server");
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(pending_cells(&harness, tab, id), [(1, 1)]);
    }

    #[test]
    fn a_long_key_gives_way_in_the_box_too() {
        let look = Look::omarchy();
        // The title as it is painted: the one line that ends as it does.
        let title = |harness: &Harness| {
            let found = pieces(harness, |piece| piece.ends_with("changed on the server"));
            let [(text, rect, _)] = &found[..] else {
                panic!("{found:?}");
            };
            (text.clone(), *rect)
        };
        for key in [
            "0199a3f2-7c1e-7abc-8def-0123456789ab".to_owned(),
            "k".repeat(120),
        ] {
            let mut structure = crate::testing::fixture_structure();
            structure.primary_key = vec!["email".into()];
            let mut page = crate::testing::page(5, false);
            page.rows[1][1] = text(&key);
            let size = egui::vec2(1280.0, 800.0);
            let (mut harness, tab, id) = table_of(look, size, structure, page);
            pend(&mut harness, (tab, id), (1, 2), "[1]");
            let now = vec![Value::Int(2), text(&key), text("[2]")];
            saved(&mut harness, (tab, id), vec![changed(0, now)]);
            // The key is the database's, and stays as it is.
            let whole = format!("row email {key} changed on the server");
            let (painted_as, rect) = title(&harness);
            // On the first line, after the word, inside the box's margins.
            let (word, _) = piece(&harness, "conflict");
            assert!(word.right() < rect.left(), "{key}");
            assert!((word.center().y - rect.center().y).abs() < 1.0, "{key}");
            // (The box's body is 18 in at the sides and 14 from the top.)
            let inner = sheet(&harness).shrink2(egui::vec2(19.0, 15.0));
            assert!(inner.contains_rect(rect), "{painted_as}: {rect:?}");
            assert!(rect.height() < 30.0, "{key}");
            // Only the name gave way.
            assert!(painted_as.starts_with("row email "), "{painted_as}");
            assert!(painted_as.contains('…'), "{painted_as}");
            if key.len() == 120 {
                assert!(painted_as.contains("k…"), "{painted_as}");
            }
            // A screen reader is told the whole of it, and so is the
            // pointer.
            assert!(named(&mut harness, &whole, rect), "{key}");
            assert!(unsaid(&harness, &whole), "{key}");
            hover(&mut harness, rect.center());
            piece(&harness, &whole);
        }
        // A title that fits is painted whole, and says no more under the
        // pointer.
        let (mut harness, ..) = conflict_in(look, Some("eve@example.com"));
        let (_, rect) = title(&harness);
        hover(&mut harness, rect.center());
        piece(&harness, "row id 2 changed on the server");
    }

    #[test]
    fn a_row_that_is_gone_has_its_own_words_and_one_key_in_the_box() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = conflict_in(look, None);
        let palette = harness.app.palette;
        for text in [
            "conflict",
            "row id 2 no longer exists on the server",
            "Fixture · users",
            "someone deleted it after you loaded it. nothing was written.",
            "email",
        ] {
            piece(&harness, text);
        }
        // Nothing is on the server to show: the two columns share what
        // three do.
        assert!(unsaid(&harness, "server"));
        let ((name, _), (loaded, _), (mine, _)) = (
            piece(&harness, "email"),
            piece(&harness, "loaded"),
            piece(&harness, "yours"),
        );
        let (was, color) = piece(&harness, "user2@example.com");
        assert_eq!((was.left(), color), (loaded.left(), palette.text));
        let (yours, color) = piece(&harness, "bob@example.com");
        assert_eq!((yours.left(), color), (mine.left(), palette.warning));
        assert!((loaded.left() - name.left() - 100.0).abs() < 0.5);
        let inner = sheet(&harness).right() - 20.0 - name.left();
        let step = mine.left() - loaded.left();
        assert!(
            (step - (inner - 90.0) / 2.0).abs() < 1.0,
            "{step} of {inner}"
        );
        // One key, which is its button, and none of the three others.
        let (hint, color) = piece(&harness, "[d] discard my changes");
        assert_eq!(color, palette.accent);
        for hint in HINTS {
            assert!(unsaid(&harness, hint), "{hint}");
        }
        assert_eq!(pressable(&mut harness, "Discard my changes").len(), 1);
        for name in ["Keep mine, reload row", "Use server values", "Overwrite"] {
            assert!(pressable(&mut harness, name).is_empty(), "{name}");
        }
        click_at(&mut harness, hint.center());
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, text("user2@example.com")));
    }

    #[test]
    fn a_long_value_is_cut_to_its_column_in_the_box_and_whole_under_the_pointer() {
        let look = Look::omarchy();
        let long = format!("{}@example.com", "b".repeat(108));
        let (mut harness, tab, id) = fixture_in(look);
        let palette = harness.app.palette;
        pend(&mut harness, (tab, id), (1, 1), &long);
        let conflict = changed(0, server("eve@example.com"));
        saved(&mut harness, (tab, id), vec![conflict]);
        let found = pieces(&harness, |piece| piece.starts_with("bbb"));
        let [(yours, place, color)] = &found[..] else {
            panic!("{found:?}");
        };
        assert!(yours.ends_with('…') && yours.len() < 60, "{yours}");
        assert_eq!(*color, palette.warning);
        // It starts under its header and ends inside the box's margin.
        let (header, _) = piece(&harness, "yours");
        assert_eq!(place.left(), header.left());
        assert!(place.right() <= sheet(&harness).right() - 20.0, "{place:?}");
        // The server's, in the column before, stops short of it.
        let (server, _) = piece(&harness, "eve@example.com");
        assert!(server.right() < place.left());
        // A screen reader is told the whole value; it is painted whole
        // only once the pointer is over it.
        assert!(named(&mut harness, &long, *place));
        assert!(unsaid(&harness, &long));
        hover(&mut harness, place.center());
        piece(&harness, &long);
    }

    #[test]
    fn a_letter_typed_into_a_field_as_the_box_comes_up_is_no_answer() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = fixture_in(look);
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        // While the save runs the user goes on to the WHERE line. The
        // question comes up with that field holding the keyboard, and the
        // field keeps it for the frame the box is first drawn in: what is
        // typed then is the field's, and answers nothing.
        type_key(&mut harness, Key::Slash, "/");
        assert!(
            harness.ctx.text_edit_focused(),
            "the WHERE line has the keys"
        );
        let conflict = changed(0, server("eve@example.com"));
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        assert!(asking(&harness));
        shown(&mut harness, true);
        harness.frame(vec![
            crate::testing::key(Key::S, Modifiers::NONE),
            egui::Event::Text("s".into()),
        ]);
        assert!(asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, text("user2@example.com")));
        harness.frame(vec![crate::testing::release(Key::S, Modifiers::NONE)]);
        // Once the box has the keyboard its letters answer.
        harness.finish_animations();
        assert!(!harness.ctx.text_edit_focused());
        shown(&mut harness, true);
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (0, text("eve@example.com")));
    }

    #[test]
    fn the_letters_answer_after_a_click_outside_the_box() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = conflict_in(look, Some("eve@example.com"));
        // A click beside the box takes the keyboard from whatever had it,
        // and answers nothing.
        focus_dialog(&mut harness, "Overwrite");
        let frame = sheet(&harness);
        click_at(&mut harness, frame.left_top() - egui::vec2(40.0, 40.0));
        assert!(asking(&harness));
        assert_eq!(focused_name(&harness.settle()), "");
        assert_eq!(state(&harness, tab, id), (1, text("user2@example.com")));
        // The letters are read as what is typed, wherever the keyboard is.
        type_key(&mut harness, Key::K, "k");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, text("eve@example.com")));
        assert_eq!(writes(&harness), 1);
    }

    #[test]
    fn the_box_is_drawn_and_answers_over_the_sql_panel() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = fixture_in(look);
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        // The tab's `:diff` panel is open on a pending cell, and the save
        // is asked for from insert mode in another row: it closes the
        // editor and takes its text with it.
        pend(&mut harness, (tab, id), (3, 1), "dan@example.com");
        let show = true;
        harness.app.apply(Action::ReviewEdits { tab, id, show });
        let cell = CellPos { row: 1, col: 1 };
        let start = EditStart::Replace("bob@example.com".into());
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell,
            start,
        });
        harness.settle();
        harness.settle();
        assert!(harness.ctx.text_edit_focused(), "insert mode");
        harness.press(Key::S, Modifiers::COMMAND);
        assert_eq!(writes(&harness), 1);
        let conflict = changed(0, server("eve@example.com"));
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        assert!(asking(&harness));
        // The panel is on screen, and the box over it and the window,
        // whole.
        let panel = crate::ui::review::placed(&harness.ctx).expect("the panel is drawn");
        assert_eq!((panel.tab, panel.id), (tab, id));
        let frame = sheet(&harness);
        let window = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
        assert!(window.contains_rect(frame), "{frame:?}");
        for text in [
            "conflict",
            "row id 2 changed on the server",
            "Fixture · users",
            "eve@example.com",
            "bob@example.com",
        ] {
            let (place, _) = piece(&harness, text);
            assert!(frame.contains_rect(place), "{text}");
        }
        for hint in HINTS {
            let (place, _) = piece(&harness, hint);
            assert!(frame.contains_rect(place), "{hint}");
        }
        // Its letters answer, with the panel open under it.
        shown(&mut harness, true);
        type_key(&mut harness, Key::S, "s");
        assert!(!asking(&harness));
        assert_eq!(state(&harness, tab, id), (1, text("eve@example.com")));
        assert_eq!(pending_cells(&harness, tab, id), [(3, 1)]);
    }

    #[test]
    fn overwrite_on_production_asks_again_with_the_row_as_the_server_holds_it() {
        let look = Look::omarchy();
        let (mut harness, tab, id) = fixture_in(look);
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        pend(&mut harness, (tab, id), (1, 1), "bob@example.com");
        // The save, confirmed with its word: asked for by the user, it
        // answers at once.
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.finish_animations();
        harness.frame(vec![egui::Event::Text("write".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        assert_eq!(writes(&harness), 1);
        // The row changed. The confirmation opened the tab's panel, and
        // the question stands over it.
        let conflict = changed(0, server("eve@example.com"));
        harness.answer_written(Ok(WriteOutcome::Conflicts(vec![conflict])));
        harness.finish_animations();
        assert!(asking(&harness));
        assert!(crate::ui::review::placed(&harness.ctx).is_some());
        piece(&harness, "row id 2 changed on the server");
        shown(&mut harness, true);
        type_key(&mut harness, Key::O, "o");
        harness.finish_animations();
        // Overwrite brings the confirmation up again, with the review of
        // the row as it is now: the check is on what the server holds.
        let checked = "-- only if email is still 'eve@example.com'";
        let Some(Dialog::ConfirmWrite(prompt)) = &harness.app.dialog else {
            panic!("expected the confirmation, got {:?}", harness.app.dialog);
        };
        let locale = harness.app.locale;
        let comments = prompt.review.lines.iter();
        let comments: Vec<String> = comments
            .filter_map(|line| crate::ui::review::comment(line, locale))
            .collect();
        assert_eq!(comments, ["-- row id 2", checked]);
        assert!(prompt.after_answer.is_some(), "an answer opened it");
        // It is the review the tab's panel shows, and the box points at
        // the panel, which lists it.
        let workspace = harness.app.workspace(tab).unwrap();
        let edits = &workspace.object_tab(id).unwrap().edits;
        assert!(edits.reviewing);
        assert_eq!(edits.review.as_ref(), Some(&prompt.review));
        let panel = crate::ui::review::placed(&harness.ctx).expect("the panel is drawn");
        assert!(
            painted(&harness, "sql shown with :diff"),
            "{:?}",
            harness.painted
        );
        let place = harness.painted_rect(checked).expect("the check's line");
        assert!(panel.rect.contains_rect(place));
        assert_eq!(writes(&harness), 1, "nothing is sent before the answer");
        // In the moment after the answer that opened it, it takes none:
        // the word stays typed, and Enter and Esc do nothing.
        let opened = |harness: &mut Harness, long: bool| {
            let now = std::time::Instant::now();
            let Some(Dialog::ConfirmWrite(prompt)) = &mut harness.app.dialog else {
                panic!("expected the confirmation, got {:?}", harness.app.dialog);
            };
            prompt.after_answer = Some(if long {
                now.checked_sub(crate::edit::ANSWER_AFTER)
                    .expect("an earlier instant")
            } else {
                now + std::time::Duration::from_secs(3600)
            });
        };
        opened(&mut harness, false);
        harness.frame(vec![egui::Event::Text("write".into())]);
        harness.press(Key::Enter, Modifiers::NONE);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(matches!(harness.app.dialog, Some(Dialog::ConfirmWrite(_))));
        assert_eq!(writes(&harness), 1);
        opened(&mut harness, true);
        harness.press(Key::Enter, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert_eq!(writes(&harness), 2, "the save runs again");
    }

    #[test]
    fn page_down_moves_the_lines_that_do_not_fit() {
        for look in looks() {
            let said = look.name;
            let size = egui::vec2(720.0, 480.0);
            let window = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
            let (mut harness, tab, id) = twelve_columns(look, size);
            // More lines than are shown at once: the first is drawn, the
            // last is not, and the question is whole in the window.
            assert!(window.contains_rect(sheet(&harness)), "{said}");
            piece(&harness, "my c1");
            assert!(unsaid(&harness, "my c12"), "{said}");
            // The keys bring the rest into view, and answer nothing.
            harness.press(Key::PageDown, Modifiers::NONE);
            harness.finish_animations();
            piece(&harness, "my c12");
            assert!(unsaid(&harness, "my c1"), "{said}");
            assert!(asking(&harness), "{said}");
            assert_eq!(pending_cells(&harness, tab, id).len(), 12, "{said}");
            assert_eq!(writes(&harness), 1, "{said}");
            // The header stays where it is, over them.
            piece(&harness, "loaded");
            harness.press(Key::PageUp, Modifiers::NONE);
            harness.finish_animations();
            piece(&harness, "my c1");
            assert!(unsaid(&harness, "my c12"), "{said}");
            // In the question's first moment too: reading answers nothing.
            shown(&mut harness, false);
            harness.press(Key::PageDown, Modifiers::NONE);
            harness.finish_animations();
            piece(&harness, "my c12");
            assert!(asking(&harness), "{said}");
            if !look.terminal {
                continue;
            }
            // The box says which keys, before the ones that answer, and
            // every key that answers is on screen with it.
            let (pages, _) = piece(&harness, "pgup/pgdn scroll");
            let (first, _) = piece(&harness, HINTS[0]);
            assert!(pages.right() < first.left(), "{said}");
            let tree = harness.settle();
            for name in ["Overwrite", "Use server values", "Keep mine, reload row"] {
                let place = crate::testing::bounds(&tree, name, Role::Button);
                let place = place.unwrap_or_else(|| panic!("no button {name}"));
                assert!(window.contains_rect(place), "{name} at {place:?}");
            }
        }
    }
}
