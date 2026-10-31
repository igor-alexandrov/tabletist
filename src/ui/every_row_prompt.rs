//! The question before a run of the SQL editor that changes every row of a
//! table: an `UPDATE` or a `DELETE` without `WHERE`, or a `TRUNCATE`. On
//! macOS and Windows a dialog with Cancel and Run anyway; in the terminal
//! look a line laid over the status line, answered with its letters. Each
//! says how many rows the tables hold, where the sidebar has the
//! database's count of them.

use egui::{Align2, Frame, Id, Key, Margin, Modifiers, Rect, Stroke, pos2};

use crate::i18n::{gettext, ngettext};
use crate::keymap::{Command, Layout, When};
use crate::model::{Action, EveryRowPrompt, WideStatement};
use crate::typography::{Text, TextRole};
use crate::ui::keys::{prompt_letter, take_enter};
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::workspace::STATUS_LINE;
use crate::ui::write_prompts::{Skin, button_row, fitted, keyboard_on};
use crate::ui::{Paint, format};
use tabletist_db::sql::Verb;

/// Where the terminal look's status line of `tab` was last drawn: the
/// question stands there.
pub(crate) fn status_line_id(tab: crate::model::ConnTabId) -> Id {
    Id::new(("status-line-rect", tab.0))
}

/// Asks `prompt` over the window, and returns the answer once there is
/// one. `editor_open` is whether the SQL tab it was asked from is still
/// open.
pub fn show(
    ctx: &egui::Context,
    paint: Paint,
    prompt: &EveryRowPrompt,
    editor_open: bool,
) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
    };
    // Whether the editor had the keyboard when the question came: it
    // still has it in the frame the question opens in, being drawn before
    // it. The keyboard goes back there after the answer, and nowhere else.
    // Kept under the question's own number, which no other question has.
    let kept = Id::new(("every-row-refocus", prompt.asked));
    let refocus = ctx
        .data(|data| data.get_temp::<bool>(kept))
        .unwrap_or_else(|| {
            let editor = crate::ui::sql_text::editor_id(prompt.tab, prompt.id);
            ctx.memory(|memory| memory.has_focus(editor))
        });
    ctx.data_mut(|data| data.insert_temp(kept, refocus));
    let mut answer = None;
    if !editor_open {
        // Its editor closed meanwhile: nothing to send, and nothing would
        // answer a question left on screen.
        answer = Some(false);
    } else {
        // Taken before anything is drawn: a button that has the keyboard
        // would read Enter as a press of itself. An Enter with a modifier
        // (the run key) answers nothing.
        let enter = ctx.input_mut(take_enter);
        let top = if look.terminal {
            line(ctx, prompt, skin, enter, &mut answer)
        } else {
            sheet(ctx, prompt, skin, enter, &mut answer)
        };
        if top && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
            answer = answer.or(Some(false));
        }
    }
    let mut actions = Vec::new();
    if let Some(run) = answer {
        ctx.data_mut(|data| data.remove::<bool>(kept));
        actions.push(match run {
            true => Action::RunEveryRow { refocus },
            false => Action::CancelEveryRow { refocus },
        });
    }
    actions
}

/// "book_covers", "books and covers", "authors, books and covers".
fn listed(tables: &[String], skin: Skin<'_>) -> String {
    let names: Vec<String> = tables
        .iter()
        .map(|name| format::display_safe(name).into_owned())
        .collect();
    match names.as_slice() {
        [] => String::new(),
        [one] => one.clone(),
        [rest @ .., last] => format!("{} {} {last}", rest.join(", "), skin.say("and")),
    }
}

/// What one statement does, as a question: "Update every row in
/// book_covers?", "Truncate books and covers?".
fn asked(wide: &WideStatement, skin: Skin<'_>) -> String {
    let words = match (wide.verb, wide.tables.is_empty()) {
        (Verb::Update, false) => "Update every row in {tables}?",
        (Verb::Update, true) => "Update every row of its table?",
        (Verb::Delete, false) => "Delete every row in {tables}?",
        (Verb::Delete, true) => "Delete every row of its table?",
        (Verb::Truncate, false) => "Truncate {tables}?",
        (Verb::Truncate, true) => "Truncate its tables?",
    };
    skin.say(words)
        .replace("{tables}", &listed(&wide.tables, skin))
}

/// The question: the one statement's, or how many there are.
fn question(prompt: &EveryRowPrompt, skin: Skin<'_>) -> String {
    match prompt.wide.as_slice() {
        [one] => asked(one, skin),
        many => skin
            .say("Run {n} statements that change every row?")
            .replace("{n}", &many.len().to_string()),
    }
}

/// "about 1,200 rows", where the statistics say.
fn rows(wide: &WideStatement, skin: Skin<'_>) -> Option<String> {
    let rows = wide.rows?;
    let plural = u32::try_from(rows).unwrap_or(u32::MAX);
    let words = ngettext(skin.locale, "about {n} row", "about {n} rows", plural);
    Some(
        skin.look
            .label(&words)
            .replace("{n}", &format::group_digits(rows)),
    )
}

/// "Line 3"
fn line_of(wide: &WideStatement, skin: Skin<'_>) -> String {
    skin.say("Line {n}").replace("{n}", &wide.line.to_string())
}

/// What the dialog says under its question: for one statement, its line
/// and why it reaches every row, then the rows; for several, a line each.
fn details(prompt: &EveryRowPrompt, skin: Skin<'_>) -> Vec<String> {
    match prompt.wide.as_slice() {
        [one] => {
            let why = match one.verb {
                Verb::Update | Verb::Delete => skin.say("has no WHERE."),
                Verb::Truncate => skin.say("takes out every row."),
            };
            let mut text = format!("{} {why}", line_of(one, skin));
            if let Some(rows) = rows(one, skin) {
                let by = skin.say("by the database's statistics.");
                text.push_str(&format!(" {}, {by}", capitalized(&rows, skin)));
            }
            vec![text]
        }
        many => many
            .iter()
            .map(|wide| {
                let what = asked(wide, skin);
                let mut text = format!("{}: {}", line_of(wide, skin), what.trim_end_matches('?'));
                if let Some(rows) = rows(wide, skin) {
                    text.push_str(&format!(", {rows}"));
                }
                text
            })
            .collect(),
    }
}

/// `text` with a capital first letter, where the look writes capitals.
fn capitalized(text: &str, skin: Skin<'_>) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) if !skin.look.terminal => first.to_uppercase().chain(chars).collect(),
        _ => text.to_owned(),
    }
}

/// macOS and Windows: the question, what it is about, and Cancel and Run
/// anyway. Enter answers as the button that has the keyboard, and cancels
/// from anywhere else: nothing that writes to every row is the default.
/// Puts the answer in `answer`: whether to run. Returns whether the dialog
/// is the one on top.
fn sheet(
    ctx: &egui::Context,
    prompt: &EveryRowPrompt,
    skin: Skin<'_>,
    enter: bool,
    answer: &mut Option<bool>,
) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let title = question(prompt, skin);
    let lines = details(prompt, skin);
    let modal = widgets::modal(Id::new("every-row-prompt"), look, palette).show(ctx, |ui| {
        ui.set_width(fitted(ui.ctx(), 440.0));
        widgets::label(ui, widgets::dialog_title(look), &title, palette.text, look);
        ui.add_space(4.0);
        let width = ui.available_width();
        for line in &lines {
            Text::one(look, widgets::body(look), line, palette.secondary)
                .wrap(width)
                .layout(ui.ctx())
                .label(ui);
        }
        ui.add_space(14.0);
        let (cancel, run) = (gettext(locale, "Cancel"), gettext(locale, "Run anyway"));
        let buttons = vec![ButtonSpec::new(&cancel), ButtonSpec::new(&run).danger()];
        if enter {
            *answer = Some(keyboard_on(ui, &buttons) == Some(1));
        }
        if let Some(pressed) = button_row(ui, buttons, skin).1 {
            *answer = Some(pressed == 1);
        }
    });
    modal.is_top_modal
}

/// The terminal look: the question where the status line was, as the
/// design's confirm prompt has it, `[y] yes  [N] no` after it. `y` runs;
/// `n`, Enter and Esc cancel. Each hint is its button too. Puts the answer
/// in `answer`: whether to run. Returns whether the line is the dialog on
/// top.
fn line(
    ctx: &egui::Context,
    prompt: &EveryRowPrompt,
    skin: Skin<'_>,
    enter: bool,
    answer: &mut Option<bool>,
) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    // The letters, as typed text. Not while a key is held: the run key may
    // still be down, and a repeat is no answer. Nor while a field has the
    // keyboard: the editor keeps it for the frame the question opens in,
    // and what is typed then went into its text.
    let typing = ctx.text_edit_focused();
    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(look));
    let letter = |text: &str| {
        prompt_letter(&keymap, layout, When::EveryRow, text)
            .filter(|command| matches!(command, Command::RunEveryRow | Command::CancelEveryRow))
    };
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
            egui::Event::Text(text) if letter(text).is_some() => {
                typed = typed.or(letter(text));
                false
            }
            _ => true,
        });
        typed.filter(|_| !held && !typing)
    });
    match typed {
        Some(command) => *answer = Some(command == Command::RunEveryRow),
        None if enter => *answer = Some(false),
        None => {}
    }
    let screen = ctx.content_rect();
    let at = ctx
        .data(|data| data.get_temp::<Rect>(status_line_id(prompt.tab)))
        .unwrap_or_else(|| {
            Rect::from_min_max(
                pos2(screen.left(), screen.bottom() - STATUS_LINE),
                screen.max,
            )
        });
    let role = TextRole::OSecondary;
    // "about 13 rows · line 1", "lines 1, 3, 4".
    let mut tail = match prompt.wide.as_slice() {
        [one] => rows(one, skin).into_iter().collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let numbers: Vec<String> = prompt
        .wide
        .iter()
        .map(|wide| wide.line.to_string())
        .collect();
    tail.push(match numbers.as_slice() {
        [one] => skin.say("line {n}").replace("{n}", one),
        many => skin.say("lines {n}").replace("{n}", &many.join(", ")),
    });
    let tail = tail.join(" · ");
    let said = question(prompt, skin);
    let (yes_key, no_key) = (
        crate::ui::keys::written(ctx, look, Command::RunEveryRow).bracketed(),
        crate::ui::keys::written(ctx, look, Command::CancelEveryRow).bracketed(),
    );
    let (yes, no) = (skin.say("yes"), skin.say("no"));
    let (run, cancel) = (gettext(locale, "Run anyway"), gettext(locale, "Cancel"));
    let modal = egui::Modal::new(Id::new("every-row-prompt"))
        .backdrop_color(egui::Color32::TRANSPARENT)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin::ZERO))
        .area(
            egui::Area::new(Id::new("every-row-prompt-line"))
                .order(egui::Order::Foreground)
                .anchor(Align2::LEFT_TOP, at.min.to_vec2()),
        )
        .show(ctx, |ui| {
            let (rect, _) = ui.allocate_exact_size(at.size(), egui::Sense::hover());
            // The design's confirm prompt: a red rule along its top.
            ui.painter().hline(
                rect.x_range(),
                rect.top() + 0.5,
                Stroke::new(1.0, palette.danger),
            );
            let y = rect.top() + 1.0 + 15.0;
            let mut x = rect.left() + 12.0;
            x += widgets::paint_text(ui, x, y, Text::one(look, role, &said, palette.text));
            let dim = Text::one(look, role, &format!(" {tail}"), palette.dim);
            x += widgets::paint_text(ui, x, y, dim);
            widgets::announce(ui, rect, &format!("{said} {tail}"));
            x += 18.0;
            for (key, word, name, runs) in
                [(&yes_key, &yes, &run, true), (&no_key, &no, &cancel, false)]
            {
                let hint = Text::new(look)
                    .add(role, key, palette.accent)
                    .space(role, " ")
                    .add(role, word, palette.secondary);
                let width = widgets::paint_text(ui, x, y, hint);
                let place =
                    Rect::from_min_max(pos2(x, rect.top() + 1.0), pos2(x + width, rect.bottom()));
                if ButtonSpec::new(name).hidden_at(ui, place).clicked() {
                    *answer = Some(runs);
                }
                x += width + 16.0;
            }
        });
    modal.is_top_modal
}
