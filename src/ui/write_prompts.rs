//! The two questions about pending changes: before they are dropped, and
//! before they are saved to production. Each as its look asks it: a sheet
//! with buttons on macOS and Windows, a box with its keys in the terminal
//! look. The confirmation lists every statement it would send, in every
//! look: no save to production is offered without them on screen.

use egui::{CornerRadius, Frame, Id, Key, Margin, Modifiers, Rect, Sense, Stroke, pos2, vec2};

use crate::app::App;
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{Action, Dialog, Held, LeavePrompt};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::format;
use crate::ui::keys::consume_press;
use crate::ui::pending_bar::counted;
use crate::ui::states::Tone;
use crate::ui::terminal_dialog;
use crate::ui::widgets::{self, ButtonSpec};

/// The tallest the confirmation's statements stand before they scroll, so
/// what is under them stays on screen.
const STATEMENTS_HEIGHT: f32 = 220.0;

/// The band along the top of the macOS and Windows confirmation.
const BAND: f32 = 4.0;

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
    let Skin { look, palette, .. } = skin;
    let height = 32.0;
    let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let widths: Vec<f32> = buttons
        .iter()
        .map(|button| button.width(ui, look))
        .collect();
    let total = widths.iter().sum::<f32>() + 8.0 * widths.len().saturating_sub(1) as f32;
    let mut left = row.right() - total;
    let mut pressed = None;
    for (index, (button, width)) in buttons.into_iter().zip(widths).enumerate() {
        let place = Rect::from_min_size(pos2(left, row.top()), vec2(width, height));
        left += width + 8.0;
        if button.show_at(ui, place, look, palette).clicked() {
            pressed = Some(index);
        }
    }
    (row, pressed)
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
    // Taken before anything is drawn, as the other prompt takes it: the
    // button that sends is pressed, never reached by a stray Enter.
    let enter = ctx.input_mut(|input| consume_press(input, Modifiers::NONE, Key::Enter));
    let mut actions = Vec::new();
    let Some(Dialog::ConfirmWrite(prompt)) = &mut app.dialog else {
        return;
    };
    let top = if look.terminal {
        confirm_box(ctx, prompt, &facts, skin, enter, &mut actions)
    } else {
        confirm_sheet(ctx, &prompt.statements, &facts, skin, enter, &mut actions)
    };
    if top && ctx.input_mut(|input| input.consume_key(Modifiers::NONE, Key::Escape)) {
        actions.push(Action::CancelWrite);
    }
    app.actions.extend(actions);
}

/// The statements a save would send, one to a line in the code face, in a
/// bordered box that scrolls both ways: a line longer than the box is cut
/// by it, never wrapped into what could read as another statement.
fn statements(ui: &mut egui::Ui, list: &[String], skin: Skin<'_>) {
    let Skin { look, palette, .. } = skin;
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
            egui::ScrollArea::both()
                .max_height(STATEMENTS_HEIGHT)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 4.0);
                    for statement in list {
                        let laid = Text::one(look, widgets::code(look), statement, palette.text)
                            .layout(ui.ctx());
                        ui.add(egui::Label::new(laid.galley).selectable(true).extend());
                    }
                });
        });
}

/// macOS and Windows: a band of the production red along the top, what is
/// saved and where, the statements, and Cancel and the button that sends.
/// `enter` says Enter was pressed: it cancels with the keyboard on Cancel,
/// and never confirms. Returns whether the prompt is the dialog on top.
fn confirm_sheet(
    ctx: &egui::Context,
    list: &[String],
    facts: &Facts,
    skin: Skin<'_>,
    enter: bool,
    actions: &mut Vec<Action>,
) -> bool {
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
                    statements(ui, list, skin);
                    ui.add_space(14.0);
                    let (cancel, save) = (
                        gettext(locale, "Cancel"),
                        gettext(locale, "Save to production"),
                    );
                    let buttons = vec![
                        ButtonSpec::new(&cancel),
                        ButtonSpec::new(&save).danger().padding(16.0),
                    ];
                    if enter && keyboard_on(ui, &buttons) == Some(0) {
                        actions.push(Action::CancelWrite);
                    }
                    let (row, pressed) = button_row(ui, buttons, skin);
                    match pressed {
                        Some(0) => actions.push(Action::CancelWrite),
                        Some(_) => actions.push(Action::ConfirmWrite),
                        None => {}
                    }
                    widgets::paint_label(
                        ui,
                        row.left(),
                        row.center().y,
                        Text::one(
                            look,
                            widgets::body(look),
                            &gettext(locale, "One transaction"),
                            palette.secondary,
                        ),
                    );
                });
        });
    modal.is_top_modal
}

/// The terminal look: the box in the danger colour, its head with the
/// environment's tag and the question, what is saved and where, the
/// statements, and the field that takes the word. `enter` says Enter was
/// pressed: it confirms once the field holds the word, and cancels with
/// the keyboard on Cancel. Returns whether the prompt is the dialog on top.
fn confirm_box(
    ctx: &egui::Context,
    prompt: &mut crate::model::WritePrompt,
    facts: &Facts,
    skin: Skin<'_>,
    enter: bool,
    actions: &mut Vec<Action>,
) -> bool {
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
    let modal = widgets::modal(Id::new("write-prompt"), look, palette)
        .frame(frame)
        .show(ctx, |ui| {
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
                    widgets::label(ui, role, &facts.connection, palette.text, look);
                    ui.add_space(2.0);
                    let about =
                        format!("{} · {}", look.label(&facts.rows), facts.columns.join(", "));
                    let width = ui.available_width();
                    Text::one(look, role, &about, palette.dim)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    ui.add_space(10.0);
                    statements(ui, &prompt.statements, skin);
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
            let (confirm, cancel) = (skin.say("confirm"), skin.say("cancel"));
            let names = [
                gettext(locale, "Save to production"),
                gettext(locale, "Cancel"),
            ];
            let keys = [
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
        let statements = match &harness.app.dialog {
            Some(Dialog::ConfirmWrite(prompt)) => prompt.statements.clone(),
            other => panic!("expected the confirmation, got {other:?}"),
        };
        assert_eq!(statements.len(), 2);
        for statement in &statements {
            assert!(statement.starts_with("UPDATE"), "{statement}");
            assert!(
                harness.painted.iter().any(|(text, _)| text == statement),
                "{statement} is on screen"
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
