//! The two questions about pending changes: before they are dropped, and
//! before they are saved to production. Each as its look asks it: a sheet
//! with buttons on macOS and Windows, a box with its keys in the terminal
//! look. The confirmation lists every statement it would send, in every
//! look, as Review SQL draws them: no save to production is offered
//! without them on screen. The terminal's box leaves them to its panel
//! only where that panel is on screen for it and shows the whole of every
//! line, and stands clear of it.

use egui::{
    Align2, Color32, CornerRadius, Frame, Id, Key, Margin, Modifiers, Rect, Sense, Stroke, pos2,
    vec2,
};

use crate::app::App;
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{Action, Dialog, Held, LeavePrompt};
use crate::review::{Line, Review, Values};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::format;
use crate::ui::keys::consume_press;
use crate::ui::pending_bar::counted;
use crate::ui::review;
use crate::ui::states::Tone;
use crate::ui::terminal_dialog;
use crate::ui::widgets::{self, ButtonSpec};

/// A row of buttons at a prompt's foot.
const ROW: f32 = 32.0;

/// The sheet's Copy SQL: lower than the buttons that answer, beside what
/// the row says.
const COPY: f32 = 24.0;

/// The band along the top of the macOS and Windows confirmation.
const BAND: f32 = 4.0;

/// What the terminal's box needs above its panel to point at it and
/// stand clear: the box as it is then, and 24 clear above and below it.
/// The box that points is the same height whatever it is asked about: each
/// of its lines is one row.
pub(crate) const POINTING_ROOM: f32 = 300.0;

/// What the sheet of macOS and Windows takes of the window besides its
/// statements, with 24 clear above and below it: what its statements may
/// not have of a low window.
const SHEET_REST: f32 = 220.0;

/// The fewest lines a confirmation shows at once, however low the window.
const FEWEST_ROWS: f32 = 3.0;

/// The word the terminal's confirmation takes.
const WORD: &str = "write";

/// What the Leave prompt says of a save that is running when the window is
/// asked to close.
const SAVING: &str = "A save is still running. It writes everything or nothing, \
                      and closing now means not seeing which.";

pub fn show(app: &mut App, ctx: &egui::Context) {
    leave(app, ctx);
    confirm_write(app, ctx);
}

/// How a prompt draws.
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

    /// The frame of a prompt whose parts fill it edge to edge, and the
    /// corners of what lies along its edge.
    fn frame(&self) -> (egui::Frame, u8) {
        let frame = widgets::modal_frame(self.look, self.palette).inner_margin(Margin::ZERO);
        let radius = self.look.dialog_radius;
        (frame, radius.saturating_sub(frame.stroke.width as u8))
    }
}

/// What was asked for and is held, as the question words it. The terminal
/// says `closing` of a tab, as its design does.
fn doing(held: &Held, terminal: bool) -> &'static str {
    match held {
        Held::Action(action) => match **action {
            Action::CloseTab { .. } if terminal => "closing",
            Action::CloseTab { .. } => "closing the tab",
            Action::Refresh(_) | Action::RetryRows { .. } | Action::RetryStructure { .. } => {
                "reloading"
            }
            Action::NextPage { .. } | Action::PrevPage { .. } => "changing the page",
            Action::SortBy { .. } | Action::ClearSort { .. } => "sorting",
            Action::ApplyFilters { .. }
            | Action::ClearFilters { .. }
            | Action::DropFilter { .. }
            | Action::FollowForeignKey { .. } => "filtering",
            Action::SwitchDatabase { .. } => "switching database",
            Action::CloseConnTab(_) | Action::Disconnect(_) | Action::Connect { .. } => {
                "disconnecting"
            }
            _ => "going on",
        },
        Held::CloseWindow => "closing the window",
    }
}

/// `width`, or what a small window leaves of it: 24 clear at each side.
fn fitted(ctx: &egui::Context, width: f32) -> f32 {
    width.min(ctx.content_rect().width() - 48.0).max(240.0)
}

/// Buttons at the right end of a row, 8 apart. They are made
/// from the left, so the keyboard reaches them in the order they are read.
/// Returns the row and the place in `buttons` of the one pressed.
fn button_row(
    ui: &mut egui::Ui,
    buttons: Vec<ButtonSpec<'_>>,
    skin: Skin<'_>,
) -> (Rect, Option<usize>) {
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
    (row, buttons_in(ui, row, buttons, skin))
}

/// What `buttons` take of a row, the 8 between them included.
fn buttons_width(ui: &egui::Ui, buttons: &[ButtonSpec<'_>], look: &Look) -> f32 {
    let widths = buttons.iter().map(|button| button.width(ui, look));
    widths.sum::<f32>() + 8.0 * buttons.len().saturating_sub(1) as f32
}

/// [`button_row`] in a row that is there already. Returns the place in
/// `buttons` of the one pressed.
fn buttons_in(
    ui: &mut egui::Ui,
    row: Rect,
    buttons: Vec<ButtonSpec<'_>>,
    skin: Skin<'_>,
) -> Option<usize> {
    let Skin { look, palette, .. } = skin;
    let mut left = row.right() - buttons_width(ui, &buttons, look);
    let mut pressed = None;
    for (index, button) in buttons.into_iter().enumerate() {
        let width = button.width(ui, look);
        let place = Rect::from_min_size(pos2(left, row.top()), vec2(width, row.height()));
        left += width + 8.0;
        if button.show_at(ui, place, look, palette).clicked() {
            pressed = Some(index);
        }
    }
    pressed
}

/// The place in `buttons` of the one that has the keyboard. Asked before
/// [`button_row`] draws them in `ui`.
fn keyboard_on(ui: &egui::Ui, buttons: &[ButtonSpec<'_>]) -> Option<usize> {
    buttons.iter().position(|button| button.has_keyboard(ui))
}

/// Asks before pending changes are dropped. Enter never discards: it
/// answers as the button that has the keyboard where that is Cancel or
/// Save, saves where Save is offered and the keyboard is on no button, and
/// answers nothing anywhere else.
fn leave(app: &mut App, ctx: &egui::Context) {
    let (look, palette) = (app.look, app.palette);
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale: app.locale,
    };
    let Some(Dialog::Leave(prompt)) = &app.dialog else {
        return;
    };
    // Taken before anything is drawn: a button that has the keyboard would
    // read Enter as a press of itself, and Discard is one of them. Space
    // presses a button. What the key answers is for the buttons to say,
    // where they are made.
    let enter = ctx.input_mut(|input| consume_press(input, Modifiers::NONE, Key::Enter));
    let mut actions = Vec::new();
    let top = if look.terminal {
        leave_box(ctx, prompt, skin, enter, &mut actions)
    } else {
        leave_sheet(ctx, prompt, skin, enter, &mut actions)
    };
    if top && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        actions.push(Action::LeaveStay);
    }
    app.actions.extend(actions);
}

/// macOS and Windows: what is asked, and Cancel, Discard and Save. `enter`
/// says Enter was pressed: it follows the button that has the keyboard,
/// and saves from anywhere else. Returns whether the prompt is the dialog
/// on top.
fn leave_sheet(
    ctx: &egui::Context,
    prompt: &LeavePrompt,
    skin: Skin<'_>,
    enter: bool,
    actions: &mut Vec<Action>,
) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let changes = counted(locale, prompt.changes, "change", "changes");
    let title = match (prompt.can_save, prompt.tabs.len()) {
        (true, _) => format!(
            "{} {changes} {} {}?",
            gettext(locale, "Save"),
            gettext(locale, "before"),
            gettext(locale, doing(&prompt.held, false))
        ),
        (false, 0 | 1) => format!(
            "{} {changes} {} {}?",
            gettext(locale, "Discard"),
            gettext(locale, "before"),
            gettext(locale, doing(&prompt.held, false))
        ),
        (false, tabs) => format!(
            "{} {changes} {} {tabs} {}?",
            gettext(locale, "Discard"),
            gettext(locale, "in"),
            gettext(locale, "tabs")
        ),
    };
    let plural = u32::try_from(prompt.changes).unwrap_or(u32::MAX);
    let text = if prompt.saving {
        gettext(locale, SAVING)
    } else {
        ngettext(
            locale,
            "It has not been written.",
            "They have not been written.",
            plural,
        )
    };
    let modal = widgets::modal(Id::new("leave-prompt"), look, palette).show(ctx, |ui| {
        ui.set_width(fitted(ui.ctx(), 420.0));
        widgets::label(ui, widgets::dialog_title(look), &title, palette.text, look);
        ui.add_space(4.0);
        let width = ui.available_width();
        Text::one(look, widgets::body(look), &text, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        ui.add_space(14.0);
        let (cancel, discard, save) = (
            gettext(locale, "Cancel"),
            gettext(locale, "Discard"),
            gettext(locale, "Save"),
        );
        let mut buttons = vec![ButtonSpec::new(&cancel), ButtonSpec::new(&discard)];
        if prompt.can_save {
            buttons.push(ButtonSpec::new(&save).primary().padding(16.0));
        }
        if enter {
            match keyboard_on(ui, &buttons) {
                Some(0) => actions.push(Action::LeaveStay),
                // Enter never discards.
                Some(1) => {}
                _ if prompt.can_save => actions.push(Action::LeaveSave),
                _ => {}
            }
        }
        match button_row(ui, buttons, skin).1 {
            Some(0) => actions.push(Action::LeaveStay),
            Some(1) => actions.push(Action::LeaveDiscard),
            Some(_) => actions.push(Action::LeaveSave),
            None => {}
        }
    });
    modal.is_top_modal
}

/// The terminal look: what is being left, how much is pending, and the
/// answers as keys in the foot, each its button too. `enter` says Enter
/// was pressed: it presses Write or Stay when the keyboard is on that
/// button, and answers nothing anywhere else. Returns whether the prompt
/// is the dialog on top.
fn leave_box(
    ctx: &egui::Context,
    prompt: &LeavePrompt,
    skin: Skin<'_>,
    enter: bool,
    actions: &mut Vec<Action>,
) -> bool {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    // The letters, as typed text. Not while a key is held: the key that
    // asked may still be down (the `d` of `gd`), and its repeats are no
    // answer. Nor while a field has the keyboard: one behind the box keeps
    // it for the frame the box opens in, and what is typed then went into
    // its text.
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
            egui::Event::Text(text) if matches!(text.as_str(), "w" | "d") => {
                typed = typed.take().or_else(|| Some(text.clone()));
                false
            }
            _ => true,
        });
        typed.filter(|_| !held && !typing)
    });
    match typed.as_deref() {
        Some("w") if prompt.can_save => actions.push(Action::LeaveSave),
        Some("d") => actions.push(Action::LeaveDiscard),
        _ => {}
    }
    let mut line = format!(
        "{} {}",
        skin.say(doing(&prompt.held, true)),
        skin.say("with pending edits")
    );
    if prompt.tabs.len() > 1 {
        line.push_str(&format!(
            " {} {} {}",
            skin.say("in"),
            prompt.tabs.len(),
            skin.say("tabs")
        ));
    }
    // Under a save there is no telling what is written: the box says
    // what closing comes to.
    let count = if prompt.saving {
        skin.say(SAVING)
    } else {
        format!(
            "{} {}",
            look.label(&counted(locale, prompt.changes, "change", "changes")),
            skin.say("not written")
        )
    };
    let (frame, radius) = skin.frame();
    let modal = widgets::modal(Id::new("leave-prompt"), look, palette)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(fitted(ui.ctx(), 460.0));
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            Frame::new()
                .inner_margin(Margin::symmetric(18, 14))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    widgets::label(ui, TextRole::OGroup, &line, palette.text, look);
                    ui.add_space(4.0);
                    let width = ui.available_width();
                    Text::one(look, TextRole::OBody, &count, palette.dim)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                });
            let foot = terminal_dialog::foot(ui, radius, palette);
            let (write, discard, stay) = (skin.say("write"), skin.say("discard"), skin.say("stay"));
            let names = [
                gettext(locale, "Write"),
                gettext(locale, "Discard"),
                gettext(locale, "Stay"),
            ];
            let key = |key, label, name, lead| terminal_dialog::Key {
                key,
                label,
                button: Some(name),
                lead,
                disabled: None,
            };
            let mut keys = vec![
                (
                    key("[d]", discard.as_str(), &*names[1], false),
                    Action::LeaveDiscard,
                ),
                (
                    key("[esc]", stay.as_str(), &*names[2], false),
                    Action::LeaveStay,
                ),
            ];
            // Saving is offered only where a save can run.
            if prompt.can_save {
                let write = key("[w]", write.as_str(), &*names[0], true);
                keys.insert(0, (write, Action::LeaveSave));
            }
            let (hints, mut answers): (Vec<_>, Vec<_>) = keys.into_iter().unzip();
            // Enter never discards, on its button either.
            let entered = terminal_dialog::keyboard_on(ui, &hints)
                .filter(|&index| enter && !matches!(answers[index], Action::LeaveDiscard));
            let pressed = terminal_dialog::keys(ui, foot, 0.0, &hints, look, palette);
            if let Some(index) = pressed.or(entered) {
                actions.push(answers.swap_remove(index));
            }
        });
    modal.is_top_modal
}

/// The keys a confirmation was given in a frame, taken before it is drawn.
#[derive(Clone, Copy)]
struct Asked {
    /// Enter was pressed.
    enter: bool,
    /// Page Down less Page Up, as often as each was pressed: how far the
    /// list of statements moves, in pages of what it shows.
    pages: f32,
}

/// What the confirmation says its save is of.
struct Facts {
    /// The connection's name and where it points.
    connection: String,
    /// "2 changes".
    changes: String,
    /// "1 row in book_covers".
    rows: String,
    /// The columns the save sets, in the order it first names them.
    columns: Vec<String>,
    /// The environment as its tag says it.
    tag: &'static str,
}

/// Asks before a save to production, with every statement it would send.
fn confirm_write(app: &mut App, ctx: &egui::Context) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
    };
    let Some(Dialog::ConfirmWrite(prompt)) = &app.dialog else {
        return;
    };
    let Some(workspace) = app.workspace(prompt.tab) else {
        // Nothing to draw, so nothing to answer it with: it closes, or it
        // would keep the keyboard for good.
        app.actions.push(Action::CancelWrite);
        return;
    };
    let table = format::display_safe(&prompt.changeset.object.name).into_owned();
    let mut columns: Vec<String> = Vec::new();
    for change in prompt.changeset.rows.iter().flat_map(|row| &row.set) {
        let name = format::display_safe(&change.column);
        if !columns.iter().any(|column| *column == name) {
            columns.push(name.into_owned());
        }
    }
    let facts = Facts {
        connection: format!(
            "{} · {}",
            workspace.name,
            format::display_safe(&super::workspace::target(workspace))
        ),
        changes: counted(locale, prompt.changes, "change", "changes"),
        rows: format!(
            "{} {} {table}",
            counted(locale, prompt.rows, "row", "rows"),
            gettext(locale, "in")
        ),
        columns,
        tag: workspace.environment.label(crate::env::Platform::of(&look)),
    };
    let dialect = workspace.driver.dialect();
    // What the tab's own panel shows: the box leaves its statements to no
    // other lines than its own.
    let shown = workspace
        .object_tab(prompt.id)
        .and_then(|object| object.edits.review.as_ref());
    let panel = look
        .terminal
        .then(|| pointed_at(ctx, prompt, shown, skin))
        .flatten();
    // Taken before anything is drawn, as the other prompt takes it: the
    // button that sends is pressed, never reached by a stray Enter.
    let enter = ctx.input_mut(|input| consume_press(input, Modifiers::NONE, Key::Enter));
    // Page Up and Page Down move the statements that are asked about,
    // wherever they stand: the panel's where the box points at it, and
    // the prompt's own list everywhere else. Never a list behind the
    // prompt, which is not what is confirmed. Taken before the field sees
    // them.
    let mut pages = ctx.input_mut(|input| {
        let mut pressed = |key: Key| input.count_and_consume_key(Modifiers::NONE, key) as f32;
        pressed(Key::PageDown) - pressed(Key::PageUp)
    });
    if panel.is_some() {
        review::turn(ctx, prompt.tab, prompt.id, pages);
        pages = 0.0;
    }
    let mut actions = Vec::new();
    let Some(Dialog::ConfirmWrite(prompt)) = &mut app.dialog else {
        return;
    };
    let mut copy = false;
    let top = if look.terminal {
        let asked = Asked { enter, pages };
        confirm_box(ctx, prompt, panel, &facts, skin, asked, &mut actions)
    } else {
        let lines = &prompt.review.lines;
        let asked = Asked { enter, pages };
        let sheet = confirm_sheet(ctx, lines, &facts, skin, asked, &mut actions);
        copy = sheet.copy;
        sheet.top
    };
    if top && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        actions.push(Action::CancelWrite);
    }
    if copy {
        // The whole statements, never the lines as the sheet shows them:
        // those are cut. Of the set the sheet was made with, which is what
        // it shows and all it would send: the tab's own need not be the
        // one in front, nor, were it to change, the one that was read.
        let whole = crate::review::of(dialect, &prompt.changeset, &[], Values::Whole);
        ctx.copy_text(review::text(&whole, locale));
    }
    app.actions.extend(actions);
}

/// The statements a save would send, as Review SQL draws them: its lines
/// one to a row in the code face, in a bordered box as tall as they are, to
/// at most [`review::MAX_ROWS`] of them. It scrolls both ways: a line
/// longer than the box is cut by it, never wrapped into what could read as
/// another. The lines are painted and cannot be selected: a value is shown
/// cut, and a copy of it would be pasted as it is. `rest` is what the
/// prompt takes of the window besides them: in a low window they stand
/// lower, so the question and its answers stay on screen with them.
/// `pages` moves them, by as many pages of the rows in view: Page Down and
/// Page Up, for a hand that is on the keyboard.
fn statements(ui: &mut egui::Ui, lines: &[Line], rest: f32, pages: f32, skin: Skin<'_>) {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let (fill, line, radius) = if look.terminal {
        (palette.panel, palette.outline, 3)
    } else {
        (palette.window, palette.border, look.radius)
    };
    Frame::new()
        .fill(fill)
        .stroke(Stroke::new(1.0, line))
        .corner_radius(CornerRadius::same(radius))
        .inner_margin(Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Before `show_rows`, which reads it from the `ui` it is given.
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let row = review::row_height(ui.ctx(), look);
            let most = (ui.ctx().content_rect().height() - rest)
                .min(review::MAX_ROWS as f32 * row)
                .max(FEWEST_ROWS * row);
            egui::ScrollArea::both()
                .max_height(most)
                .auto_shrink([false, true])
                .min_scrolled_height(0.0)
                .show_rows(ui, row, lines.len(), |ui, range| {
                    // A page is the whole rows in view, so no line is
                    // passed over unseen. The lines move up for Page Down.
                    if pages != 0.0 {
                        let shown = most.min(lines.len() as f32 * row);
                        let page = (shown / row).floor().max(1.0) * row;
                        ui.scroll_with_delta(vec2(0.0, -pages * page));
                    }
                    review::rows(ui, lines, range, look, palette, locale);
                });
        });
}

/// What the sheet says of itself once it is drawn.
struct Sheet {
    /// It is the dialog on top.
    top: bool,
    /// Its Copy SQL was pressed.
    copy: bool,
}

/// macOS and Windows: a band of the production red along the top, what is
/// saved and where, the statements, and a foot with what the save is, Copy
/// SQL, Cancel and the button that sends. Of the keys `asked`, Enter
/// cancels with the keyboard on Cancel, copies with it on Copy SQL, and
/// never confirms; Page Up and Page Down move the statements.
fn confirm_sheet(
    ctx: &egui::Context,
    lines: &[Line],
    facts: &Facts,
    skin: Skin<'_>,
    asked: Asked,
    actions: &mut Vec<Action>,
) -> Sheet {
    let Asked { enter, pages } = asked;
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let title = format!(
        "{} {} {}?",
        gettext(locale, "Save"),
        facts.changes,
        gettext(locale, "to production")
    );
    let about = format!("{} · {}", facts.connection, facts.rows);
    let (frame, radius) = skin.frame();
    let mut copy = false;
    let modal = widgets::modal(Id::new("write-prompt"), look, palette)
        .frame(frame)
        .show(ctx, |ui| {
            ui.set_width(fitted(ui.ctx(), 560.0));
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            let (band, _) =
                ui.allocate_exact_size(vec2(ui.available_width(), BAND), Sense::hover());
            ui.painter().add(egui::Shape::convex_polygon(
                widgets::top_cap(band, f32::from(radius), BAND),
                Tone::Danger.color(palette),
                Stroke::NONE,
            ));
            Frame::new()
                .inner_margin(Margin {
                    left: 20,
                    right: 20,
                    top: 16,
                    bottom: 20,
                })
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    widgets::label(ui, widgets::dialog_title(look), &title, palette.text, look);
                    ui.add_space(4.0);
                    let width = ui.available_width();
                    Text::one(look, widgets::body(look), &about, palette.secondary)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    ui.add_space(12.0);
                    statements(ui, lines, SHEET_REST, pages, skin);
                    ui.add_space(14.0);
                    let (cancel, save, copy_sql) = (
                        gettext(locale, "Cancel"),
                        gettext(locale, "Save to production"),
                        gettext(locale, "Copy SQL"),
                    );
                    let buttons = vec![
                        ButtonSpec::new(&cancel),
                        ButtonSpec::new(&save).danger().padding(16.0),
                    ];
                    let (row, _) =
                        ui.allocate_exact_size(vec2(ui.available_width(), ROW), Sense::hover());
                    let y = row.center().y;
                    // What the save is, at the row's left, where it has the
                    // room beside the buttons: it gives way to them.
                    let said = gettext(locale, "One transaction");
                    let said = || Text::one(look, widgets::body(look), &said, palette.secondary);
                    let copier = ButtonSpec::new(&copy_sql).quiet();
                    let copier_width = copier.width(ui, look);
                    let end = row.right() - buttons_width(ui, &buttons, look) - 8.0;
                    let mut left = row.left();
                    if left + widgets::measure(ui, said()) + 8.0 + copier_width <= end {
                        left += widgets::paint_label(ui, left, y, said()) + 8.0;
                    }
                    let place =
                        Rect::from_min_size(pos2(left, y - COPY / 2.0), vec2(copier_width, COPY));
                    // Enter was taken before the buttons are drawn: it does
                    // what the one that has the keyboard does, and with the
                    // keyboard on the one that sends, or nowhere, nothing.
                    if enter {
                        if copier.has_keyboard(ui) {
                            copy = true;
                        } else if keyboard_on(ui, &buttons) == Some(0) {
                            actions.push(Action::CancelWrite);
                        }
                    }
                    // Made first, as it is read first: the keyboard reaches
                    // it before Cancel.
                    copy |= copier.show_at(ui, place, look, palette).clicked();
                    match buttons_in(ui, row, buttons, skin) {
                        Some(0) => actions.push(Action::CancelWrite),
                        Some(_) => actions.push(Action::ConfirmWrite),
                        None => {}
                    }
                });
        });
    Sheet {
        top: modal.is_top_modal,
        copy,
    }
}

/// Where the Review SQL panel of the prompt's own tab stands, when the
/// box may leave the statements to it: the panel was drawn in this very
/// frame (it is drawn before the dialogs), it is that tab's and no
/// other's, the review it draws (`shown`) is the prompt's own, it shows
/// its lines (all of them, or as many at once as the box itself would at
/// the least, and the whole width of the widest), and above it the window
/// has the room the box needs to stand clear of it. `None` everywhere
/// else, and the box then lists the statements itself: its tab is not the
/// one in front (a save asked for by the question about leaving a tab, a
/// connection or the window), another tab's panel is on screen, or the
/// window is too low or too narrow. Too narrow counts as much as too low:
/// under the box neither the pointer nor a key moves the panel's lines
/// sideways, and what is past its edge of a statement would be confirmed
/// unread; in the box the lines scroll both ways under the pointer.
fn pointed_at(
    ctx: &egui::Context,
    prompt: &crate::model::WritePrompt,
    shown: Option<&Review>,
    skin: Skin<'_>,
) -> Option<Rect> {
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let placed = review::placed_now(ctx)?;
    let own = (placed.tab, placed.id) == (prompt.tab, prompt.id);
    let room = placed.rect.top() - ctx.content_rect().top();
    let lines = prompt.review.lines.len() as f32;
    let fewest = lines.min(FEWEST_ROWS) * review::row_height(ctx, look);
    let stands = own && room >= POINTING_ROOM && placed.lines >= fewest;
    // Asked last: the lines are compared, and measured, only for a panel
    // that would be pointed at.
    let whole = stands
        && shown == Some(&prompt.review)
        && review::widest(ctx, &prompt.review, look, palette, locale) <= placed.width;
    whole.then_some(placed.rect)
}

/// One row of `text` across the width `ui` has left, cut with `…` where it
/// is longer; a screen reader gets the whole of it.
fn one_row(ui: &mut egui::Ui, role: TextRole, text: &str, color: Color32, look: &Look) {
    let height = role.row_height(ui.ctx(), look.faces);
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let measure = |text: &str| widgets::measure(ui, Text::one(look, role, text, color));
    let shown = crate::ui::grid::ellipsize(text, row.width(), false, measure);
    widgets::paint_text(
        ui,
        row.left(),
        row.center().y,
        Text::one(look, role, &shown, color),
    );
    widgets::announce(ui, row, text);
}

/// The terminal look: the box in the danger colour, its head with the
/// environment's tag and the question, what is saved and where, the
/// statements, and the field that takes the word. Where the panel of its
/// own tab is on screen with room above it and shows its lines (`panel`,
/// see [`pointed_at`]), the box stands in that room, says the panel shows
/// the statements, and draws no backdrop over them; everywhere else it
/// lists them itself. Of the keys `asked`, Enter confirms once the field
/// holds the word, and cancels with the keyboard on Cancel; Page Up and
/// Page Down move the statements the box lists (the panel's are moved by
/// whoever read the keys). Returns whether the prompt is the dialog on
/// top.
fn confirm_box(
    ctx: &egui::Context,
    prompt: &mut crate::model::WritePrompt,
    panel: Option<Rect>,
    facts: &Facts,
    skin: Skin<'_>,
    asked: Asked,
    actions: &mut Vec<Action>,
) -> bool {
    let Asked { enter, pages } = asked;
    let Skin {
        look,
        palette,
        locale,
    } = skin;
    let danger = Tone::Danger.color(palette);
    let (frame, _) = skin.frame();
    let frame = frame.stroke(Stroke::new(2.0, danger));
    let radius = look.dialog_radius.saturating_sub(2);
    let armed = prompt.typed == WORD;
    // What the field asks for, and why the button that sends cannot be
    // pressed before it has it.
    let ask = format!("{} {WORD} {}", skin.say("type"), skin.say("to confirm"));
    let id = Id::new("write-prompt");
    let modal = widgets::modal(id, look, palette).frame(frame);
    let modal = match panel {
        // In the middle of what the panel leaves above it. No backdrop: it
        // would dim the statements the user is asked to read. The box's
        // edge marks it, and it takes every click all the same.
        Some(panel) => {
            let lift = (ctx.content_rect().bottom() - panel.top()) / 2.0;
            let area =
                egui::Modal::default_area(id).anchor(Align2::CENTER_CENTER, vec2(0.0, -lift));
            modal.area(area).backdrop_color(Color32::TRANSPARENT)
        }
        None => modal,
    };
    let modal = modal.show(ctx, |ui| {
        ui.set_width(fitted(ui.ctx(), 600.0));
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        let head = terminal_dialog::head(ui, radius, Tone::Danger.fill(look, palette), palette);
        let y = terminal_dialog::head_line(head);
        // The tag, filled with the danger colour.
        let ink = widgets::ink_on(danger, palette);
        let tag = Text::one(look, TextRole::OGroup, facts.tag, ink).layout(ui.ctx());
        let chip = Rect::from_min_size(
            pos2(head.left() + 14.0, y - tag.height() / 2.0 - 1.0),
            vec2(tag.width() + 12.0, tag.height() + 2.0),
        );
        ui.painter()
            .rect_filled(chip, CornerRadius::same(2), danger);
        tag.paint_left(ui.painter(), chip.left() + 6.0, y);
        let title = format!("{} {}?", skin.say("write"), look.label(&facts.changes));
        widgets::paint_label(
            ui,
            chip.right() + 10.0,
            y,
            Text::one(look, TextRole::OScreenTitle, &title, palette.text),
        );
        Frame::new()
            .inner_margin(Margin::symmetric(18, 14))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let role = TextRole::OBody;
                let about = format!("{} · {}", look.label(&facts.rows), facts.columns.join(", "));
                if panel.is_some() {
                    // Each on one row, so the box is as tall as the room
                    // it was given allows, whatever it is asked about.
                    one_row(ui, role, &facts.connection, palette.text, look);
                    ui.add_space(2.0);
                    one_row(ui, role, &about, palette.dim, look);
                    ui.add_space(10.0);
                    // In place of the statements: where they are.
                    Text::new(look)
                        .add(role, &skin.say("sql shown with"), palette.dim)
                        .space(role, " ")
                        .add(role, ":diff", palette.accent)
                        .layout(ui.ctx())
                        .label(ui);
                } else {
                    widgets::label(ui, role, &facts.connection, palette.text, look);
                    ui.add_space(2.0);
                    let width = ui.available_width();
                    Text::one(look, role, &about, palette.dim)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    ui.add_space(10.0);
                    // The box that lists is the box that points with
                    // the statements in place of one line: what that one
                    // needs of the window is what these may not have.
                    statements(ui, &prompt.review.lines, POINTING_ROOM, pages, skin);
                }
                ui.add_space(12.0);
                let ask = widgets::label(ui, role, &ask, palette.dim, look);
                ui.add_space(6.0);
                // The field's border in the danger colour, with the
                // keyboard and without.
                let visuals = ui.visuals_mut();
                let edge = Stroke::new(1.0, danger);
                visuals.selection.stroke = edge;
                for state in [
                    &mut visuals.widgets.inactive,
                    &mut visuals.widgets.hovered,
                    &mut visuals.widgets.active,
                ] {
                    state.bg_stroke = edge;
                }
                let field = ui
                    .add(
                        widgets::single_in(ui, &mut prompt.typed, look, role)
                            .desired_width(f32::INFINITY),
                    )
                    .labelled_by(ask.id);
                focus::hint(ui, &field, field.rect, Ring::Failing { radius: 3 });
                // Once: a frame that only sizes the box keeps no focus.
                if prompt.focus {
                    field.request_focus();
                    prompt.focus = ui.is_sizing_pass();
                }
            });
        let foot = terminal_dialog::foot(ui, radius, palette);
        let (confirm, cancel, scroll) = (
            skin.say("confirm"),
            skin.say("cancel"),
            skin.say("scroll sql"),
        );
        let names = [
            gettext(locale, "Save to production"),
            gettext(locale, "Cancel"),
        ];
        let mut keys = vec![
            terminal_dialog::Key {
                key: "enter",
                label: &confirm,
                button: Some(&names[0]),
                // Its key answers once the word is typed, and so does
                // its button.
                lead: armed,
                disabled: (!armed).then_some(ask.as_str()),
            },
            terminal_dialog::Key {
                key: "esc",
                label: &cancel,
                button: Some(&names[1]),
                lead: false,
                disabled: None,
            },
        ];
        // The pointer cannot reach the panel under the box: its keys do.
        // Last, and no button: the two before it are told by their place.
        if panel.is_some() {
            keys.push(terminal_dialog::Key {
                key: "pgup/pgdn",
                label: &scroll,
                button: None,
                lead: false,
                disabled: None,
            });
        }
        // The terminal's Enter confirms, once the field holds the
        // word: from the field and from the button that sends, not
        // from Cancel.
        let entered = match terminal_dialog::keyboard_on(ui, &keys) {
            _ if !enter => None,
            Some(1) => Some(1),
            _ => Some(0),
        };
        let pressed = terminal_dialog::keys(ui, foot, 0.0, &keys, look, palette);
        match pressed.or(entered) {
            // The button confirms what its key does, and no more.
            Some(0) if armed => actions.push(Action::ConfirmWrite),
            Some(1) => actions.push(Action::CancelWrite),
            _ => {}
        }
    });
    modal.is_top_modal
}

#[cfg(test)]
mod tests {
    use crate::backend::Command;
    use crate::model::{Action, CellPos, ConnTabId, Dialog, EditStart, TabId};
    use crate::testing::Harness;
    use crate::ui::tests::{click_dialog, pressable as buttons};

    /// Makes `text` the pending value of column `col` in row `row`, as an
    /// editor that was typed into and left does.
    fn change(harness: &mut Harness, at: (ConnTabId, TabId), row: usize, col: usize, text: &str) {
        let (tab, id) = at;
        harness.app.apply(Action::EditCell {
            tab,
            id,
            cell: CellPos { row, col },
            start: EditStart::Replace(text.into()),
        });
        harness.app.apply(Action::LeaveEdit { tab, id });
    }

    fn writes(harness: &Harness) -> usize {
        let sent = harness.app.backend.sent.iter();
        sent.filter(|command| matches!(command, Command::Write { .. }))
            .count()
    }

    fn pending(harness: &Harness, (tab, id): (ConnTabId, TabId)) -> usize {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.object_tab(id).unwrap().edits.cells.len()
    }

    #[test]
    fn the_production_confirmation_lists_every_statement_and_sends_only_when_confirmed() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        harness.app.workspace_mut(tab).unwrap().environment = crate::env::Environment::Production;
        change(&mut harness, (tab, id), 1, 1, "bob@example.com");
        change(&mut harness, (tab, id), 3, 1, "dan@example.com");
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.settle();
        let lines: Vec<String> = match &harness.app.dialog {
            Some(Dialog::ConfirmWrite(prompt)) => {
                let lines = prompt.review.lines.iter();
                lines
                    .filter_map(|line| {
                        crate::ui::review::comment(line, harness.app.locale).or_else(|| line.sql())
                    })
                    .collect()
            }
            other => panic!("expected the confirmation, got {other:?}"),
        };
        // Two comments and a statement's three lines for each of the rows.
        assert_eq!(lines.len(), 10);
        assert_eq!(lines[0], "-- row id 2");
        assert_eq!(lines[7], r#"UPDATE "main"."users""#);
        for line in &lines {
            assert!(
                harness.painted.iter().any(|(text, _)| text == line),
                "{line} is on screen"
            );
        }
        assert_eq!(writes(&harness), 0, "nothing is sent before the answer");
        harness.click("Cancel");
        assert!(harness.app.dialog.is_none());
        assert_eq!((writes(&harness), pending(&harness, (tab, id))), (0, 2));
        // Esc cancels too.
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.press(egui::Key::Escape, egui::Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
        assert_eq!((writes(&harness), pending(&harness, (tab, id))), (0, 2));
        harness.app.apply(Action::WriteEdits { tab, id });
        harness.click("Save to production");
        assert!(harness.app.dialog.is_none());
        assert_eq!(writes(&harness), 1);
    }

    #[test]
    fn a_line_of_the_box_that_points_is_one_row_however_long() {
        let look = crate::theme::Look::omarchy();
        let mut harness = Harness::new();
        harness.set_look(look);
        let palette = harness.app.palette;
        let role = crate::typography::TextRole::OBody;
        let long = format!("1 row in users · {}", ["a_column"; 60].join(", "));
        let mut heights = Vec::new();
        let tree = harness.frame_with(|ui| {
            let place = egui::Rect::from_min_size(ui.cursor().min, egui::vec2(400.0, 200.0));
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(place));
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            for text in ["1 row in users · email", long.as_str()] {
                let top = ui.cursor().top();
                super::one_row(&mut ui, role, text, palette.dim, &look);
                heights.push(ui.cursor().top() - top);
            }
        });
        // A long one takes the row a short one takes, and no more.
        assert_eq!(heights[0], heights[1]);
        assert!(heights[0] > 0.0);
        // The short one is whole; the long one is cut with its mark, and
        // fits the width.
        let painted = |start: &str| {
            let mut texts = harness.text_rects.iter();
            texts
                .find(|(text, _)| text.starts_with(start))
                .cloned()
                .unwrap_or_else(|| panic!("nothing begins {start:?}"))
        };
        assert_eq!(
            painted("1 row in users · email").0,
            "1 row in users · email"
        );
        let (cut, place) = painted("1 row in users · a_column");
        assert!(cut.ends_with('…') && cut.len() < long.len(), "{cut}");
        assert!(place.width() <= 400.0, "{place:?}");
        // A screen reader gets the whole of both.
        let labels = crate::testing::labels(&tree);
        assert!(labels.contains(&long));
        assert!(labels.iter().any(|label| label == "1 row in users · email"));
    }

    #[test]
    fn the_leave_prompt_offers_save_only_where_a_save_can_run() {
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        change(&mut harness, (tab, id), 1, 1, "bob@example.com");
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(harness.has("Discard") && harness.has("Cancel"));
        // The dialog's Save, and the pending bar's behind it.
        assert_eq!(buttons(&mut harness, "Save").len(), 2);
        click_dialog(&mut harness, "Save");
        assert!(harness.app.dialog.is_none());
        assert_eq!(writes(&harness), 1);
        // A value to fix disables the save, and the prompt has no Save.
        let mut harness = Harness::new();
        let (tab, id) = harness.editable();
        change(&mut harness, (tab, id), 1, 2, "{oops");
        harness.app.apply(Action::CloseTab { tab, id });
        assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
        // No Save in the dialog, and the bar's cannot be pressed.
        assert!(buttons(&mut harness, "Save").is_empty());
        assert!(harness.has("Discard") && harness.has("Cancel"));
        // Enter never discards.
        harness.press(egui::Key::Enter, egui::Modifiers::NONE);
        assert!(matches!(harness.app.dialog, Some(Dialog::Leave(_))));
        assert_eq!(pending(&harness, (tab, id)), 1);
    }
}
