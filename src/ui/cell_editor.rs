//! Editing a cell: the field that sits on the cell, the popover for a
//! value too large for it, the keys that end an edit, and the words for
//! what a typed value fails and for why a cell cannot be edited.

use egui::text::{CCursor, CCursorRange};
use egui::{CornerRadius, Id, Key, Margin, Modifiers, Rect, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::edit::{Editor, Lock, MAX_EDIT_BYTES, Problem};
use crate::i18n::{Locale, gettext, ngettext};
use crate::model::{Advance, ConnTabId, TabId};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Ring};
use crate::ui::format::display_safe;
use crate::ui::keys::{consume_press, copy_is_ctrl_c, is_press};
use crate::ui::states::Tone;
use crate::ui::{grid, sql_complete, widgets};

/// What a frame of an open editor asks for.
#[derive(Debug, Default, PartialEq)]
pub struct Outcome {
    /// The text changed.
    pub changed: bool,
    /// Enter, Tab or Shift+Tab ended the edit.
    pub commit: Option<Advance>,
    /// Esc, or the terminal look's Ctrl+C: the edit is dropped.
    pub cancel: bool,
    /// The keyboard went elsewhere, or the terminal look's Esc left insert
    /// mode: the text is kept.
    pub left: bool,
    /// It was that Esc: the keys go back to where the edit was asked for,
    /// which a keyboard that went elsewhere of itself does not.
    pub kept: bool,
    /// Alt+Enter.
    pub large: bool,
    /// The now button: the cell takes the time of the save, and the text
    /// is dropped.
    pub now: bool,
}

/// The cell an editor is open on, as its field needs it.
pub struct Target {
    /// The field's id: one for a table's tab, whatever cell it is on.
    pub id: Id,
    /// The column's name.
    pub name: String,
    /// The column's type as the grid's header shows it.
    pub type_name: String,
    /// The most characters the column holds, where its type says.
    pub max_chars: Option<u32>,
    /// The column holds JSON: the large editor numbers its lines, colours
    /// its syntax and says when it is valid.
    pub json: bool,
    /// No dialog is up: an editor that is open has the keyboard.
    pub hold: bool,
    /// The column keeps a date or a time: the editor offers now.
    pub now: bool,
}

/// The now button beside a date or time cell's editor, at `place`, in the
/// looks that have one: the terminal look's is a key. Says whether it was
/// pressed, and keeps the press from reading as the keyboard gone
/// elsewhere: the button is the editor's own, and the field `id` has the
/// keyboard through the press.
fn now_button(
    ui: &mut Ui,
    place: Rect,
    id: Id,
    (look, palette, locale): (&Look, &Palette, Locale),
    outcome: &mut Outcome,
) {
    let label = gettext(locale, "now");
    let hint = gettext(locale, "The time of the save, by the database's clock");
    let response = widgets::ButtonSpec::new(&label)
        .padding(8.0)
        .role(widgets::secondary(look))
        .show_at(ui, place, look, palette)
        .on_hover_text(hint);
    if response.clicked() {
        outcome.now = true;
    } else if response.is_pointer_button_down_on() {
        // Between the press and its release the edit is not left.
        ui.memory_mut(|memory| memory.request_focus(id));
        outcome.left = false;
        outcome.kept = false;
    }
    if outcome.now {
        outcome.left = false;
        outcome.kept = false;
    }
}

/// How wide the now button is, and the gap before it.
fn now_room(ui: &Ui, look: &Look, locale: Locale) -> (f32, f32) {
    let label = gettext(locale, "now");
    let width = widgets::ButtonSpec::new(&label)
        .padding(8.0)
        .role(widgets::secondary(look))
        .width(ui, look);
    (width, 6.0)
}

/// The id of the field that edits a cell of the table `id` shows.
pub fn field_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("cell-editor", tab.0, id.0))
}

/// Where egui's memory keeps whether the field `id` had the keyboard when
/// it was last drawn. egui says a widget lost the keyboard only when it
/// had it a frame ago, and only when nothing else asked for it meanwhile;
/// and a field is not drawn every frame (its tab may be behind another).
fn had_id(id: Id) -> Id {
    id.with("had-keyboard")
}

/// Gives the field `id` the keyboard, its cursor at the end of `text` and
/// nothing to undo: it is one field for every cell, and each edit is its
/// own. An editor that opens at its text's top passes none of it.
fn take_keyboard(ctx: &egui::Context, id: Id, text: &str) {
    let mut state = egui::text_edit::TextEditState::default();
    let end = CCursor::new(text.chars().count());
    state.cursor.set_char_range(Some(CCursorRange::one(end)));
    state.store(ctx, id);
    ctx.memory_mut(|memory| memory.request_focus(id));
    // The keys a field holds (Tab, Esc) are its own from the frame after
    // it has the keyboard: ask for that frame, since no event need follow
    // the one that opened the editor.
    ctx.request_repaint();
}

/// Where it is kept that a pointer asked for the editor of the field `id`
/// at a point of its value's text.
fn asked_at_id(id: Id) -> Id {
    id.with("asked-at")
}

/// Says that the editor of the field `id` is about to be asked for by a
/// click at `pos`, on its value's text: it opens with its cursor under
/// the pointer, where the text is read, and not at its end.
pub fn open_at(ctx: &egui::Context, id: Id, pos: egui::Pos2) {
    let frame = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| data.insert_temp(asked_at_id(id), (pos, frame)));
}

/// Takes the point the editor of the field `id` was asked for at, if a
/// click asked for it in the last frames: one that was refused (a locked
/// field) is no point of a later editor's.
fn asked_at(ctx: &egui::Context, id: Id) -> Option<egui::Pos2> {
    let asked: Option<(egui::Pos2, u64)> = ctx.data_mut(|data| data.remove_temp(asked_at_id(id)));
    let (pos, frame) = asked?;
    (ctx.cumulative_frame_nr().saturating_sub(frame) <= 3).then_some(pos)
}

/// Puts the cursor of the field `id`, whose text `output` drew, at the
/// character under `pos`.
fn caret_at(
    ctx: &egui::Context,
    id: Id,
    output: &egui::text_edit::TextEditOutput,
    pos: egui::Pos2,
) {
    let cursor = output.galley.cursor_from_pos(pos - output.galley_pos);
    let mut state = egui::text_edit::TextEditState::load(ctx, id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
    state.store(ctx, id);
    // The cursor was drawn where the field opened it.
    ctx.request_repaint();
}

/// The keys that end an edit, read before the field is added: it would
/// take Enter as giving the keyboard up, and Alt+Enter with it. Tab and
/// Esc are egui's to move and drop the keyboard with, unless the field
/// holds them (see `hold_keys`).
fn ending_keys(ui: &Ui, has: bool, had: bool, terminal: bool, outcome: &mut Outcome) {
    ui.input_mut(|input| {
        if has {
            // Alt first: a match lets an extra Alt and Shift through.
            if consume_press(input, Modifiers::ALT, Key::Enter) {
                outcome.large = true;
            } else if consume_press(input, Modifiers::NONE, Key::Enter) {
                outcome.commit = Some(Advance::Down);
            } else if consume_press(input, Modifiers::SHIFT, Key::Tab) {
                outcome.commit = Some(Advance::Left);
            } else if consume_press(input, Modifiers::NONE, Key::Tab) {
                outcome.commit = Some(Advance::Right);
            }
        }
        leaving_keys(input, has || had, terminal, outcome);
    });
}

/// Esc and, in the terminal look, Ctrl+C, for a field that has the
/// keyboard or `had` it a frame ago: in the first frames of a field egui
/// still drops the keyboard on Esc before any of this runs, and the key is
/// the editor's all the same. Esc drops the edit. In the terminal look it
/// leaves insert mode with the text kept, and Ctrl+C drops it.
fn leaving_keys(input: &mut egui::InputState, mine: bool, terminal: bool, outcome: &mut Outcome) {
    if !mine {
        return;
    }
    if !terminal {
        outcome.cancel |= consume_press(input, Modifiers::NONE, Key::Escape);
        return;
    }
    // Ctrl+C comes as a copy where Ctrl is the command key, and is taken
    // before the field sees it: nothing is copied. Where Cmd is, the copy
    // is Cmd+C and stays the field's, and Ctrl+C is a key. A copy asked
    // for any other way is the field's too, and copies what is selected:
    // Ctrl+Shift+C, a Copy key and a menu's Copy, which come with nothing
    // held.
    let copy = copy_is_ctrl_c(input);
    let mut ended = false;
    input.events.retain(|event| {
        if ended {
            // What is typed after the key that left is not the text's:
            // insert mode ended there. Nor is it normal mode's, whose keys
            // were read before the field was drawn.
            return !matches!(
                event,
                egui::Event::Text(_) | egui::Event::Paste(_) | egui::Event::Copy | egui::Event::Cut
            );
        }
        match event {
            egui::Event::Copy if copy => outcome.cancel = true,
            event if is_press(event, Modifiers::CTRL, Key::C) => outcome.cancel = true,
            egui::Event::Key {
                key: Key::Escape,
                pressed: true,
                repeat,
                modifiers,
                ..
            } if modifiers.matches_logically(Modifiers::NONE) => {
                // A held key's repeats are taken and end nothing.
                if *repeat {
                    return false;
                }
                outcome.left = true;
                outcome.kept = true;
            }
            _ => return true,
        }
        ended = true;
        false
    });
}

/// Keeps Tab and Esc for the field `id`, which has the keyboard. The field
/// set its own filter while it drew (the arrows, Tab); egui replaces the
/// whole filter, so those are named again. It holds from the frame after
/// the field first had the keyboard.
fn hold_keys(ctx: &egui::Context, id: Id) {
    ctx.memory_mut(|memory| {
        memory.set_focus_lock_filter(
            id,
            egui::EventFilter {
                tab: true,
                horizontal_arrows: true,
                vertical_arrows: true,
                escape: true,
            },
        );
    });
}

/// Keeps the keyboard on a field that has it, for this frame. egui decides
/// where Tab and the arrows move the keyboard when a pass begins, by what
/// the widget that has it holds, and a field holds them only from the frame
/// after it first had the keyboard (see `hold_keys`). In its first frames
/// the move is called off here, before the field is added: the keys are the
/// field's from the start.
fn keep_keyboard(ctx: &egui::Context) {
    ctx.memory_mut(|memory| memory.move_focus(egui::FocusDirection::None));
}

/// How much of `space` a field's frame takes as a margin, which egui keeps
/// in whole points and up to 127 of them, and what is left over: the field
/// begins or ends that much inside the cell, so its text is where the
/// cell's was.
fn margin(space: f32) -> (i8, f32) {
    let whole = space.clamp(0.0, f32::from(i8::MAX)).floor();
    (whole as i8, space - whole)
}

/// The editor on its cell: a one-line field over `rect`, the cell's. It
/// edits `editor.text` and takes `editor.focus`; everything else it says in
/// its [`Outcome`].
pub fn field(
    ui: &mut Ui,
    rect: Rect,
    editor: &mut Editor,
    target: &Target,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome {
    let mut outcome = Outcome::default();
    let id = target.id;
    if std::mem::take(&mut editor.focus) {
        take_keyboard(ui.ctx(), id, &editor.text);
    }
    let has = ui.memory(|memory| memory.has_focus(id));
    let had = had_keyboard(ui.ctx(), id);
    if has {
        keep_keyboard(ui.ctx());
    }
    ending_keys(ui, has, had, look.terminal, &mut outcome);

    // Over the cell: the row's fill would show through a field with none.
    ui.painter()
        .rect_filled(rect, CornerRadius::ZERO, palette.window);
    // A date or time cell offers now at its right end, where the cell has
    // the room for it beside some of its text: the field is that much
    // narrower.
    let cell = rect;
    let now = (target.now && !look.terminal)
        .then(|| now_room(ui, look, locale))
        .filter(|(width, gap)| cell.width() >= 2.0 * (width + gap))
        .map(|(width, _)| {
            let height = (cell.height() - 8.0).max(0.0);
            Rect::from_center_size(
                pos2(cell.right() - 4.0 - width / 2.0, cell.center().y),
                vec2(width, height),
            )
        });
    let rect = match now {
        Some(button) => Rect::from_min_max(cell.min, pos2(button.left() - 2.0, cell.bottom())),
        None => cell,
    };
    let role = grid::data_role(look);
    let pad = grid::cell_pad(look);
    let center = rect.center().y;
    // How much of the column's length the text takes, at the field's right.
    let mut right = rect.right() - pad;
    if let Some(max) = target.max_chars {
        let count = format!("{} / {max}", editor.text.chars().count());
        let laid = Text::one(look, widgets::secondary(look), &count, palette.dim).layout(ui.ctx());
        right -= laid.paint_right(ui.painter(), right, center) + pad;
    }
    let line = role.row_height(ui.ctx(), look.faces);
    // Where the text is: where the cell had it.
    let inner = Rect::from_min_max(
        pos2(rect.left() + pad, center - line / 2.0),
        pos2(right.max(rect.left() + pad), center + line / 2.0),
    );
    // The field is the whole cell, the text inside its margins: a click
    // beside the text, in the cell's own padding or on the count, places
    // the cursor as one on the text does. It is no click on the row, which
    // would end the edit.
    let (left, shift_x) = margin(inner.left() - rect.left());
    let (top, shift_y) = margin(inner.top() - rect.top());
    let (right, _) = margin(rect.right() - inner.right());
    let (bottom, _) = margin(rect.bottom() - inner.bottom());
    let inside = Margin {
        left,
        right,
        top,
        bottom,
    };
    let place = Rect::from_min_max(
        rect.min + vec2(shift_x, shift_y),
        inner.max + vec2(f32::from(right), f32::from(bottom)),
    );
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(place));
    // One line, whatever its length: a text wider than the cell (a time
    // with all its digits in a column sized for the second) is scrolled in
    // the field, not wrapped under the row.
    let mut laid = crate::typography::layouter(look, role, palette.text);
    let mut layouter =
        |ui: &Ui, text: &dyn egui::TextBuffer, _wrap: f32| laid(ui, text, f32::INFINITY);
    let output = egui::TextEdit::singleline(&mut editor.text)
        .id(id)
        .font(role.font_id(look.faces))
        .frame(egui::Frame::new().inner_margin(inside))
        .desired_width(place.width())
        // Tab ends the edit; it is not egui's to move the keyboard with.
        .lock_focus(true)
        // A paste of any size is cut here, in characters, which is how
        // egui counts; `bound` then holds it in bytes. One more than the
        // limit is let in, so a text that was cut is over the limit and
        // says so: one cut to the limit itself would pass for a value.
        .char_limit(MAX_EDIT_BYTES + 1)
        .layouter(&mut layouter)
        .show(&mut child);
    bound(&mut editor.text);
    if look.terminal && output.response.has_focus() {
        block_cursor(ui, &output, role, look, palette);
    }
    let response: egui::Response = output.response.response;
    // The name only: the field keeps the role and the value egui gave it.
    let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(name);
    });
    let has = response.has_focus();
    if has {
        hold_keys(ui.ctx(), id);
    }
    // The ring is the cell's: red while the text fails its check.
    let failing = editor.problem.is_some();
    if look.terminal {
        // The cursor's line, as the grid draws it round the cell its keys
        // are on, and no halo: the grid marks its own cell.
        let color = if failing {
            palette.danger
        } else {
            palette.accent
        };
        ui.painter().rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(2.0, color),
            StrokeKind::Inside,
        );
        focus::hint(ui, &response, rect, Ring::Own);
    } else if failing {
        focus::hint(ui, &response, rect, Ring::Failing { radius: 0 });
    } else {
        focus::hint(ui, &response, rect, Ring::Field { radius: 0 });
    }
    if let (Some(problem), true) = (&editor.problem, target.hold) {
        let message = problem_text(problem, &target.type_name, Some(&editor.text), locale);
        say_under(ui, rect, id, &message, look, palette);
    }
    outcome.changed = response.changed();
    keyboard_after(ui.ctx(), target, has, had, &mut outcome);
    if let Some(place) = now {
        now_button(ui, place, id, (look, palette, locale), &mut outcome);
    }
    outcome
}

/// A terminal's cursor is a block: one character wide from where the next
/// one goes, over the caret's line of the field `output` is of. Tinted, so
/// the character under it reads through.
fn block_cursor(
    ui: &Ui,
    output: &egui::text_edit::TextEditOutput,
    role: TextRole,
    look: &Look,
    palette: &Palette,
) {
    let Some(cursor) = output.cursor_range else {
        return;
    };
    let caret = output.galley.pos_from_cursor(cursor.primary);
    let block = Rect::from_min_size(
        output.galley_pos + caret.min.to_vec2(),
        vec2(role.width(ui.ctx(), look.faces, "0"), caret.height()),
    );
    ui.painter()
        .with_clip_rect(output.text_clip_rect.intersect(ui.clip_rect()))
        .rect_filled(
            block,
            CornerRadius::ZERO,
            palette.accent.gamma_multiply(0.5),
        );
}

/// The editor in the row panel: a field in the place of its field's
/// value, `height` tall for one line of text and `outset` wider at each
/// side than the room it is given (the box the value shows under the
/// pointer, so the text stays where it was read), its text in `role`, the
/// role the value is read in. Its text wraps where the value wrapped, and
/// the field grows with it: it is one line of text all the same, which
/// Enter commits. Its keys are the keys of the field on a cell, but a
/// commit moves no selection: Enter and Tab take the text and ask for the
/// row's next field, Shift+Tab for the one before. Where a click on the
/// value's text asked for it, its cursor opens under the pointer. Under
/// the field it says what the text fails, and how much of its column's
/// length the text takes.
pub fn in_panel(
    ui: &mut Ui,
    editor: &mut Editor,
    target: &Target,
    (role, (height, outset)): (TextRole, (f32, f32)),
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome {
    let mut outcome = Outcome::default();
    let id = target.id;
    let opened = std::mem::take(&mut editor.focus);
    if opened {
        take_keyboard(ui.ctx(), id, &editor.text);
    }
    let has = ui.memory(|memory| memory.has_focus(id));
    let had = had_keyboard(ui.ctx(), id);
    if has {
        keep_keyboard(ui.ctx());
    }
    ending_keys(ui, has, had, look.terminal, &mut outcome);
    // In the panel a commit walks the row's fields, not the grid's cells.
    outcome.commit = outcome.commit.map(|then| match then {
        Advance::Stay => Advance::Stay,
        Advance::Left | Advance::PrevField => Advance::PrevField,
        Advance::Down | Advance::Right | Advance::NextField => Advance::NextField,
    });
    // A click on the value's text asked for it: the cursor goes under the
    // pointer, once the text is laid out.
    let at = opened.then(|| asked_at(ui.ctx(), id)).flatten();
    // A date or time field offers now beside it, as the design's row has
    // it: the field is that much narrower.
    let now = (target.now && !look.terminal).then(|| now_room(ui, look, locale));
    let width = ui.available_width() - now.map_or(0.0, |(width, gap)| width + gap);
    let mut layouter = crate::typography::layouter(look, role, palette.text);
    // The text wraps where its value wrapped, at the room's width, and
    // the field is a line taller for each line it wraps to: nothing of
    // it is out of sight, and it stands where it was read.
    let wraps = layouter(ui, &editor.text.as_str(), width).rows.len().max(1);
    let grown = height + (wraps - 1) as f32 * role.row_height(ui.ctx(), look.faces);
    // The field's lines are the room's; the field itself stands out of
    // them at both sides, and its text is as far in as it stands out.
    let (line, _) = ui.allocate_exact_size(vec2(width, grown), egui::Sense::hover());
    let place = line.expand2(vec2(outset, 0.0));
    let now = now.map(|(width, gap)| {
        Rect::from_min_size(pos2(place.right() + gap, line.top()), vec2(width, height))
    });
    let mut within = ui.new_child(egui::UiBuilder::new().max_rect(place));
    // On the panel's own tone, as the design's field is: the panel is the
    // window's colour, and a field's usual fill would read as the tint of
    // a value under the pointer. The terminal look's field keeps its fill.
    let fill = if look.terminal {
        ui.visuals().extreme_bg_color
    } else {
        palette.window
    };
    let pad = outset as i8;
    let output = widgets::wrapping_field(&within, &mut editor.text, look, role, height, pad)
        .id(id)
        .background_color(fill)
        .desired_width(place.width())
        // Tab ends the edit; it is not egui's to move the keyboard with.
        .lock_focus(true)
        // As the field on a cell: a paste is cut, and left over the limit.
        .char_limit(MAX_EDIT_BYTES + 1)
        .layouter(&mut layouter)
        .show(&mut within);
    bound(&mut editor.text);
    if let Some(pos) = at {
        caret_at(ui.ctx(), id, &output, pos);
    }
    if look.terminal && output.response.has_focus() {
        block_cursor(ui, &output, role, look, palette);
    }
    let response: egui::Response = output.response.response;
    // The name only: the field keeps the role and the value egui gave it.
    let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(name);
    });
    let has = response.has_focus();
    if has {
        hold_keys(ui.ctx(), id);
    }
    // A field's own ring, red while the text fails its check. The ring
    // shows once the keyboard is in use; the red border does not wait for
    // that, since a pasted text can fail before any key is pressed.
    let radius = look.radius;
    let ring = if editor.problem.is_some() {
        ui.painter().rect_stroke(
            response.rect,
            CornerRadius::same(radius),
            Stroke::new(1.0, palette.danger),
            StrokeKind::Inside,
        );
        Ring::Failing { radius }
    } else {
        Ring::Field { radius }
    };
    focus::hint(ui, &response, response.rect, ring);
    // The field can open far down a long row: it is brought into view
    // once, when it takes the keyboard.
    if opened {
        response.scroll_to_me(None);
    }
    let problem = editor
        .problem
        .as_ref()
        .map(|problem| problem_text(problem, &target.type_name, Some(&editor.text), locale));
    let count = target
        .max_chars
        .map(|max| format!("{} / {max}", editor.text.chars().count()));
    if problem.is_some() || count.is_some() {
        let small = widgets::secondary(look);
        ui.add_space(3.0);
        ui.horizontal_top(|ui| {
            let taken = count
                .as_ref()
                .map_or(0.0, |count| small.width(ui.ctx(), look.faces, count) + 8.0);
            if let Some(problem) = &problem {
                Text::one(look, small, problem, palette.danger)
                    .wrap((width - taken).max(0.0))
                    .layout(ui.ctx())
                    .label(ui);
            }
            if let Some(count) = &count {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                    Text::one(look, small, count, palette.dim)
                        .layout(ui.ctx())
                        .label(ui);
                });
            }
        });
    }
    outcome.changed = response.changed();
    keyboard_after(ui.ctx(), target, has, had, &mut outcome);
    if let Some(place) = now {
        now_button(ui, place, id, (look, palette, locale), &mut outcome);
    }
    outcome
}

/// Whether the field `id` had the keyboard when it was last drawn.
fn had_keyboard(ctx: &egui::Context, id: Id) -> bool {
    ctx.data(|data| data.get_temp(had_id(id))).unwrap_or(false)
}

/// What a field's frame comes to for the keyboard, once the field is
/// drawn: `has` it now, `had` it when last drawn. Says whether it went
/// elsewhere, and takes it back for an editor that is still open after its
/// leaving was not taken.
fn keyboard_after(
    ctx: &egui::Context,
    target: &Target,
    has: bool,
    had: bool,
    outcome: &mut Outcome,
) {
    let ended = outcome.commit.is_some() || outcome.cancel || outcome.large;
    // The keyboard went elsewhere: to a click, to another field. Or the
    // terminal's Esc left the field already.
    outcome.left = !ended && (outcome.left || (had && !has));
    if target.hold && !has && !had {
        // A prompt was up when it left, and the editor is open still: it
        // has the keyboard again once nothing else is asked.
        ctx.memory_mut(|memory| memory.request_focus(target.id));
        ctx.request_repaint();
    }
    ctx.data_mut(|data| data.insert_temp(had_id(target.id), has));
}

/// The large editor's size, and the height of the band under its text.
const LARGE: egui::Vec2 = vec2(420.0, 180.0);
const BAND: f32 = 28.0;
/// The room round the large editor's text, and at the band's sides.
const LARGE_PAD: f32 = 10.0;

/// The large editor: a popover anchored to `anchor`, the cell, for a value
/// of several lines, a long one, or a document. Its text is a field of
/// several lines, where Enter and Tab are the text's own; Mod+Enter
/// applies it and Esc drops it. Under the text a band counts what it holds
/// and names those keys, or says what the text fails.
pub fn large(
    ctx: &egui::Context,
    anchor: Rect,
    editor: &mut Editor,
    target: &Target,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome {
    let mut outcome = Outcome::default();
    let id = target.id;
    // One area for the editors of every table: only the table on screen
    // has one, and egui keeps each area's place for good.
    let area = Id::new("cell-editor-large");
    egui::Area::new(area)
        .order(egui::Order::Foreground)
        .fade_in(false)
        // Placed inside the screen by the size it has, not by the one
        // egui remembers.
        .constrain(false)
        .fixed_pos(sql_complete::place(anchor, LARGE, ctx.content_rect()))
        .show(ctx, |ui| {
            // egui sizes a new area in a pass that paints nothing and
            // whose widgets take no keyboard: the field drawn in it would
            // give up the one it is handed. The editor starts with the
            // frame after.
            let sizing = ui.is_sizing_pass();
            if sizing {
                ui.ctx().request_repaint();
            } else if std::mem::take(&mut editor.focus) {
                let before = if editor.top { "" } else { &editor.text };
                take_keyboard(ui.ctx(), id, before);
            }
            let had = large_keys(ui, id, look.terminal, &mut outcome);
            let (rect, _) = ui.allocate_exact_size(LARGE, egui::Sense::hover());
            let corner = if look.terminal { 3 } else { 8 };
            // The panel's own border gives way to the editor's line.
            let _ = sql_complete::panel(ui, rect, corner, look, palette);
            let skin = (look, palette, locale);
            let (response, has) = large_body(ui, rect, area, editor, target, (had, None), skin);
            outcome.changed = response.changed();
            if !sizing {
                keyboard_after(ui.ctx(), target, has, had, &mut outcome);
            }
        });
    outcome
}

/// The large editor in the row panel: the popover's body in the place of
/// its field's value, `outset` wider at each side than the room it is
/// given, as the panel's one-line field is, and as tall as its text from
/// three lines to twelve (it scrolls past that). Its keys are the
/// popover's: Enter and Tab are the text's own, Mod+Enter applies, and Esc
/// drops the edit (the terminal look's keeps it, and Ctrl+C drops it).
pub fn tall(
    ui: &mut Ui,
    editor: &mut Editor,
    target: &Target,
    outset: f32,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> Outcome {
    let mut outcome = Outcome::default();
    let id = target.id;
    let opened = std::mem::take(&mut editor.focus);
    if opened {
        let before = if editor.top { "" } else { &editor.text };
        take_keyboard(ui.ctx(), id, before);
    }
    // A click on the value's text asked for it: the cursor goes under the
    // pointer, once the text is laid out.
    let at = opened.then(|| asked_at(ui.ctx(), id)).flatten();
    let had = large_keys(ui, id, look.terminal, &mut outcome);
    // As tall as the text is at the width it gets, within its bounds.
    let role = grid::data_role(look);
    let line = role.row_height(ui.ctx(), look.faces);
    let width = ui.available_width();
    let wrap = (width + 2.0 * outset - 2.0 * LARGE_PAD).max(1.0);
    let mut layouter = crate::typography::layouter(look, role, palette.text);
    let rows = layouter(ui, &editor.text.as_str(), wrap).rows.len();
    let rows = rows.clamp(TALL_ROWS.0, TALL_ROWS.1) as f32;
    let height = LARGE_PAD + rows * line + LARGE_PAD / 2.0 + BAND;
    // Its line is the room's; the box stands out of it at both sides, as
    // the box of a value under the pointer does.
    let (place, _) = ui.allocate_exact_size(vec2(width, height), egui::Sense::hover());
    let rect = place.expand2(vec2(outset, 0.0));
    let corner = if look.terminal { 3 } else { 8 };
    // On the panel's own tone, as the one-line field is.
    let fill = if look.terminal {
        ui.visuals().extreme_bg_color
    } else {
        palette.window
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(corner), fill);
    let mut within = ui.new_child(egui::UiBuilder::new().max_rect(rect));
    let salt = Id::new("row-panel-tall");
    let skin = (look, palette, locale);
    let (response, has) = large_body(&mut within, rect, salt, editor, target, (had, at), skin);
    // The field can open far down a long row: it is brought into view
    // once, when it takes the keyboard.
    if opened {
        ui.scroll_to_rect(rect, None);
    }
    outcome.changed = response.changed();
    keyboard_after(ui.ctx(), target, has, had, &mut outcome);
    outcome
}

/// The fewest and the most lines of its text the row panel's tall field
/// shows.
const TALL_ROWS: (usize, usize) = (3, 12);

/// The large editor's keys, read before its text is added, which would
/// take Esc as the keyboard given up: Mod+Enter applies, and Esc and the
/// terminal look's Ctrl+C leave. No popup is open for the other owners of
/// Esc to see: the key is taken here, so none of them acts on it. Returns
/// whether the text had the keyboard when it was last drawn.
fn large_keys(ui: &Ui, id: Id, terminal: bool, outcome: &mut Outcome) -> bool {
    let has = ui.memory(|memory| memory.has_focus(id));
    let had = had_keyboard(ui.ctx(), id);
    if has {
        keep_keyboard(ui.ctx());
    }
    ui.input_mut(|input| {
        if has && consume_press(input, Modifiers::COMMAND, Key::Enter) {
            outcome.commit = Some(Advance::Stay);
        }
        leaving_keys(input, has || had, terminal, outcome);
    });
    had
}

/// Where a press that is the open editor's own is noted, for the frame.
fn own_press_id(id: Id) -> Id {
    id.with("own-press")
}

/// Says that a press outside the large editor of the field `id` is the
/// editor's all the same: a control that acts on its text (Format) stands
/// beside it, and the text keeps the keyboard through the press.
pub fn press_is_the_editors(ctx: &egui::Context, id: Id) {
    ctx.data_mut(|data| data.insert_temp(own_press_id(id), true));
}

/// The large editor's body, in `rect`: a text of several lines, the
/// editor's line round it, and the band under it. The popover at a cell
/// and the row panel's tall field both draw it, under ids told apart by
/// `salt`. `had` says its text had the keyboard when it was last drawn,
/// and `at` is the point its cursor goes to, where a click on the value's
/// text asked for it; returns the text's response and whether it has the
/// keyboard now.
fn large_body(
    ui: &mut Ui,
    rect: Rect,
    salt: Id,
    editor: &mut Editor,
    target: &Target,
    (had, at): (bool, Option<egui::Pos2>),
    (look, palette, locale): (&Look, &Palette, Locale),
) -> (egui::Response, bool) {
    let id = target.id;
    // A press beside the text (the band, the padding) is the editor's: it
    // is no click on what is under it, and the text keeps the keyboard.
    let whole = ui.interact(rect, salt.with("panel"), egui::Sense::CLICK);
    let clicked_over =
        ui.input(|input| input.pointer.any_click()) && ui.rect_contains_pointer(rect);
    // So is a press on a control beside it that says so.
    let beside: bool = ui
        .data_mut(|data| data.remove_temp(own_press_id(id)))
        .unwrap_or(false);
    let pressed = whole.is_pointer_button_down_on() || whole.clicked() || clicked_over || beside;
    let corner = if look.terminal { 3 } else { 8 };
    let role = grid::data_role(look);
    let text_rect = Rect::from_min_max(
        rect.min + vec2(LARGE_PAD, LARGE_PAD),
        pos2(
            rect.right() - LARGE_PAD,
            rect.bottom() - BAND - LARGE_PAD / 2.0,
        ),
    );
    let line = role.row_height(ui.ctx(), look.faces);
    let rows = (text_rect.height() / line).floor().max(1.0) as usize;
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(text_rect));
    let faces = look.faces;
    // A document is written in its tree's colours, piece by piece; any
    // other text in the text's.
    let mut layouter = |ui: &Ui, buffer: &dyn egui::TextBuffer, wrap: f32| {
        let text = buffer.as_str();
        let mut job = if target.json {
            let format = |color| role.format(faces, color);
            crate::ui::json_text::job(text, format, look, palette)
        } else {
            let mut job = egui::text::LayoutJob::default();
            job.append(text, 0.0, role.format(faces, palette.text));
            job
        };
        job.wrap.max_width = wrap;
        ui.fonts_mut(|fonts| fonts.layout_job(job))
    };
    // A document's lines are numbered, in a gutter at the text's left
    // that scrolls with it.
    let lines = editor.text.matches('\n').count() + 1;
    let gutter = if target.json {
        let digits = lines.to_string().len().max(2) as f32;
        GUTTER_PAD + digits * role.width(ui.ctx(), faces, "0") + GUTTER_PAD
    } else {
        0.0
    };
    let response = egui::ScrollArea::vertical()
        .id_salt(salt.with("text"))
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let numbers = ui.painter().add(egui::Shape::Noop);
                ui.add_space(gutter);
                let output = egui::TextEdit::multiline(&mut editor.text)
                    .id(id)
                    .font(role.font_id(faces))
                    .frame(egui::Frame::NONE)
                    .margin(Margin::ZERO)
                    .desired_width(f32::INFINITY)
                    .desired_rows(rows)
                    // Tab is the text's own.
                    .lock_focus(true)
                    // As the field on the cell: what is laid out is
                    // bounded, and a text that was cut is over the limit.
                    .char_limit(MAX_EDIT_BYTES + 1)
                    .layouter(&mut layouter)
                    .show(ui);
                if let Some(pos) = at {
                    caret_at(ui.ctx(), id, &output, pos);
                }
                if target.json {
                    let column = Rect::from_min_max(
                        pos2(text_rect.left() - LARGE_PAD + 1.0, ui.clip_rect().top()),
                        pos2(
                            text_rect.left() + gutter - GUTTER_PAD,
                            ui.clip_rect().bottom(),
                        ),
                    );
                    let behind = gutter_numbers(ui, column, &output, role, look, palette);
                    ui.painter().set(numbers, behind);
                }
                output.response.response
            })
            .inner
        })
        .inner;
    bound(&mut editor.text);
    // The name only: the field keeps the role and the value egui gave it.
    let name = format!("{} {}", gettext(locale, "Edit"), display_safe(&target.name));
    ui.ctx().accesskit_node_builder(id, |node| {
        node.set_label(name);
    });
    let mut has = response.has_focus();
    if !has && had && pressed {
        // The press landed outside the field, which gave the keys up:
        // they stay the editor's.
        ui.memory_mut(|memory| memory.request_focus(id));
        has = true;
    }
    if has {
        hold_keys(ui.ctx(), id);
    }
    // The editor's line: the accent of a field being typed in, red while
    // the text fails its check.
    let (color, ring) = if editor.problem.is_some() {
        (palette.danger, Ring::Failing { radius: corner })
    } else {
        (palette.accent, Ring::Field { radius: corner })
    };
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(corner),
        Stroke::new(1.0, color),
        StrokeKind::Inside,
    );
    focus::hint(ui, &response, rect, ring);
    band(ui, rect, editor, target, (look, palette, locale));
    (response, has)
}

/// The room at each side of a document's line numbers.
const GUTTER_PAD: f32 = 6.0;

/// Paints the numbers of a document's lines in `column`, beside the rows
/// of the text `output` drew: a line's number on its first row, none on
/// the rows it wraps to. Returns the gutter's ground, to go behind them.
fn gutter_numbers(
    ui: &Ui,
    column: Rect,
    output: &egui::text_edit::TextEditOutput,
    role: TextRole,
    look: &Look,
    palette: &Palette,
) -> egui::Shape {
    // The tone of a row under the pointer, as the design's gutter has.
    let ground = grid::row_fill(false, true, false, look, palette).unwrap_or(palette.surface);
    let clip = ui.clip_rect();
    let middle = role.middle(ui.ctx(), look.faces);
    let mut line = 1;
    let mut starts = true;
    for row in &output.galley.rows {
        let top = output.galley_pos.y + row.min_y();
        if starts && top <= clip.bottom() && top + row.height() >= clip.top() {
            let number = Text::one(look, role, &line.to_string(), palette.faint);
            widgets::paint_text_right(ui, column.right() - GUTTER_PAD, top + middle, number);
        }
        starts = row.ends_with_newline;
        if starts {
            line += 1;
        }
    }
    egui::Shape::from(egui::epaint::RectShape::filled(
        column,
        CornerRadius::ZERO,
        ground,
    ))
}

/// The band under the large editor's text: how much it holds, or what it
/// fails, and at the right the keys that end the edit.
fn band(
    ui: &Ui,
    rect: Rect,
    editor: &Editor,
    target: &Target,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let top = rect.bottom() - BAND;
    let line = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    widgets::hline(ui, rect.x_range().shrink(1.0), top, line);
    let y = top + BAND / 2.0;
    let role = widgets::secondary(look);
    // The terminal's Esc keeps the text, and its keys are spelled out.
    let keys = if look.terminal {
        format!(
            "{}enter {} · esc {}",
            look.label(look.command_key()),
            gettext(locale, "apply"),
            gettext(locale, "keep")
        )
    } else {
        format!(
            "{}↩ {} · esc {}",
            look.command_key(),
            gettext(locale, "apply"),
            gettext(locale, "cancel")
        )
    };
    let right = rect.right() - LARGE_PAD;
    let taken = widgets::paint_text_right(ui, right, y, Text::one(look, role, &keys, palette.dim));
    let left = rect.left() + LARGE_PAD;
    let room = (right - taken - LARGE_PAD - left).max(0.0);
    let (said, color) = match &editor.problem {
        Some(problem) => (
            problem_text(problem, &target.type_name, Some(&editor.text), locale),
            palette.danger,
        ),
        None => {
            let count = |count: usize, one: &'static str, many: &'static str| {
                let noun = ngettext(locale, one, many, u32::try_from(count).unwrap_or(u32::MAX));
                let count = crate::ui::format::group_digits(count as u64);
                format!("{count} {noun}")
            };
            let chars = editor.text.chars().count();
            let lines = editor.text.matches('\n').count() + 1;
            (
                format!(
                    "{} · {}",
                    count(chars, "char", "chars"),
                    count(lines, "line", "lines")
                ),
                palette.dim,
            )
        }
    };
    let width = |text: &str| role.width(ui.ctx(), look.faces, text);
    // A document that is one says so, before its counts. Not an empty
    // text, which is none and fails nothing yet: a NULL opens with it,
    // and nobody typed it.
    let valid = gettext(locale, "Valid JSON");
    let written = !editor.text.trim().is_empty();
    let lead = if target.json && written && editor.problem.is_none() {
        format!("{valid} · ")
    } else {
        String::new()
    };
    let said = format!("{lead}{said}");
    let shown = grid::ellipsize(&said, room, false, width);
    let text = match shown
        .strip_prefix(valid.as_ref())
        .filter(|_| !lead.is_empty())
    {
        Some(rest) => Text::one(look, role, &valid, palette.success).add(role, rest, color),
        None => Text::one(look, role, &shown, color),
    };
    let wide = widgets::paint_text(ui, left, y, text);
    // The whole of it for a screen reader, cut or not.
    let place = Rect::from_min_size(pos2(left, top), vec2(wide.max(1.0), BAND));
    widgets::announce(ui, place, &said);
}

/// Paints `message` under the field at `rect`, or over it where the grid
/// ends under it: what the text fails, in red. Over the rows, which are
/// drawn after the field's.
fn say_under(ui: &Ui, rect: Rect, id: Id, message: &str, look: &Look, palette: &Palette) {
    let clip = ui.clip_rect();
    let laid = Text::one(look, widgets::secondary(look), message, palette.danger).layout(ui.ctx());
    let size = laid.size() + vec2(12.0, 6.0);
    // Clear of the field's halo.
    let gap = 4.0;
    let below = rect.bottom() + gap;
    let top = if below + size.y > clip.bottom() {
        rect.top() - gap - size.y
    } else {
        below
    };
    let place = Rect::from_min_size(pos2(rect.left(), top), size);
    let layer = egui::LayerId::new(egui::Order::Foreground, id.with("problem"));
    let painter = ui.ctx().layer_painter(layer).with_clip_rect(clip);
    let corner = CornerRadius::same(look.radius.min(4));
    painter.rect_filled(place, corner, Tone::Danger.fill(look, palette));
    painter.rect_stroke(
        place,
        corner,
        Stroke::new(1.0, Tone::Danger.line(look, palette)),
        StrokeKind::Inside,
    );
    laid.paint_left(&painter, place.left() + 6.0, place.center().y);
    widgets::announce(ui, place, message);
}

/// Holds an editor's text to just over `MAX_EDIT_BYTES`, in bytes, at a
/// character's edge: characters of several bytes would otherwise make the
/// text several times the limit, and all of it is laid out every frame. It
/// is left over the limit, never at it: the check then says the value is
/// too large, where a text cut to the limit would be taken for the value
/// that was pasted.
fn bound(text: &mut String) {
    if text.len() <= MAX_EDIT_BYTES {
        return;
    }
    let mut end = MAX_EDIT_BYTES + 1;
    while !text.is_char_boundary(end) {
        end += 1;
    }
    text.truncate(end);
}

/// What a typed value fails, as the user reads it. `type_name` is the
/// column's type as the grid's header shows it (the page's `ColumnMeta`:
/// `int8`, where the structure says `bigint`); `typed` is the text that
/// failed, where the caller has it.
pub fn problem_text(
    problem: &Problem,
    type_name: &str,
    typed: Option<&str>,
    locale: Locale,
) -> String {
    let say = |text: &'static str| gettext(locale, text);
    let counted = |count: u32, one: &'static str, many: &'static str| {
        format!("{count} {}", ngettext(locale, one, many, count))
    };
    // As the check read it: without the spaces a database overlooks.
    let typed = typed.map(str::trim_ascii).filter(|typed| !typed.is_empty());
    // What the database would have kept in place of the typed number.
    let stored_as = |stored: &str| match typed {
        Some(typed) => format!("{typed} {} {stored}", say("would be stored as")),
        None => format!("{} {stored}", say("Would be stored as")),
    };
    match problem {
        Problem::WholeNumber => format!("{type_name} {}", say("expects a whole number")),
        Problem::OutOfRange { min, max } => {
            format!("{type_name} {} {min} {} {max}", say("holds"), say("to"))
        }
        Problem::Number => format!("{type_name} {}", say("expects a number")),
        Problem::Decimals { scale, stored } => {
            // What the type keeps: so many decimals, none, or, for a scale
            // below zero, zeros in the last places before the point.
            let limit = match (u32::try_from(*scale), scale) {
                (Ok(0), _) => format!("{}.", say("No decimals")),
                (Ok(decimals), _) => format!(
                    "{} {}.",
                    say("Up to"),
                    counted(decimals, "decimal", "decimals")
                ),
                (Err(_), -1) => format!("{}.", say("Whole tens only")),
                (Err(_), -2) => format!("{}.", say("Whole hundreds only")),
                (Err(_), -3) => format!("{}.", say("Whole thousands only")),
                (Err(_), _) => format!(
                    "{} 1{} {}.",
                    say("Whole multiples of"),
                    "0".repeat(scale.unsigned_abs() as usize),
                    say("only")
                ),
            };
            match typed {
                Some(_) => format!("{limit} {}.", stored_as(stored)),
                None => limit,
            }
        }
        Problem::Digits { whole } => format!(
            "{} {} {}",
            say("At most"),
            counted(*whole, "digit", "digits"),
            say("before the point")
        ),
        // Both ways from zero, so it reads right of a value below it.
        // Neither end is a value the type holds.
        Problem::Under { limit } => format!(
            "{type_name} {} -{limit} {} {limit}",
            say("holds values between"),
            say("and")
        ),
        Problem::Inexact { stored } => stored_as(stored),
        Problem::Boolean => format!("{type_name} {}", say("expects true or false")),
        Problem::NotOneOf(allowed) => {
            // The values come from the server: nothing hidden in them.
            let values: Vec<_> = allowed.iter().map(|value| display_safe(value)).collect();
            format!("{} {}", say("Not one of:"), values.join(", "))
        }
        Problem::Json {
            message,
            line,
            column,
        } => {
            // The parser starts its sentence in lower case.
            let mut letters = message.chars();
            let message: String = match letters.next() {
                Some(first) => first.to_uppercase().chain(letters).collect(),
                None => String::new(),
            };
            format!("{message} {} {line}:{column}", say("at"))
        }
        Problem::TooLong { max } => format!(
            "{} {}",
            say("At most"),
            counted(*max, "character", "characters")
        ),
        Problem::TooLarge => {
            say("Over 256 KiB: values this large cannot be edited yet").into_owned()
        }
    }
}

/// Why a cell cannot be edited. `table` is the table's name as it is
/// shown. A cell that is not there has no reason to give.
pub fn lock_text(lock: Lock, table: &str, locale: Locale) -> String {
    let say = |text: &'static str| gettext(locale, text).into_owned();
    match lock {
        Lock::ReadOnly => say("This connection opens read-only"),
        Lock::NotATable => say("Views cannot be edited"),
        Lock::StructureLoading => say("The table's structure is still loading"),
        Lock::NoKey => format!(
            "{table} {}",
            say("has no primary key or unique index, so a row can't be targeted safely")
        ),
        Lock::KeyType => format!(
            "{table}{}",
            say("'s key cannot be matched exactly, so a row can't be targeted safely")
        ),
        Lock::Saving => say("A save is running"),
        Lock::Refreshing => say("The page is loading"),
        Lock::Gone => say("This row no longer exists on the server"),
        Lock::KeyIsNull => say("This row's key is NULL"),
        Lock::KeyInexact => say("This row's key holds text that was not read exactly"),
        Lock::UnknownColumn => say("This column cannot be told apart in the table"),
        Lock::Generated => say("Computed by the database"),
        Lock::Assigned => say("Assigned by the database on save"),
        Lock::KeyColumn => say("Part of the row's key"),
        Lock::Binary => say("Binary values cannot be edited yet"),
        Lock::TooLarge => say("Values over 256 KiB cannot be edited yet"),
        Lock::NotNull => say("This column cannot be NULL"),
        Lock::NoSuchCell => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_text_is_held_just_over_the_limit_at_a_characters_edge() {
        // Under the limit and at it, nothing is cut.
        for len in [0, 5, MAX_EDIT_BYTES] {
            let mut text = "x".repeat(len);
            bound(&mut text);
            assert_eq!(text.len(), len);
        }
        // Over it, one byte more than the limit is kept, so the check
        // still says the value is too large: a text cut to the limit
        // itself would pass for the value that was pasted.
        let mut text = "x".repeat(MAX_EDIT_BYTES * 2);
        bound(&mut text);
        assert_eq!(text.len(), MAX_EDIT_BYTES + 1);
        // Characters of several bytes are not split, and the text is no
        // larger than the limit and one character.
        for letter in ["é", "☺", "😀"] {
            let mut text = letter.repeat(MAX_EDIT_BYTES);
            bound(&mut text);
            assert!(text.len() > MAX_EDIT_BYTES, "{letter}");
            assert!(text.len() <= MAX_EDIT_BYTES + letter.len(), "{letter}");
            assert!(
                text.chars().all(|held| letter.starts_with(held)),
                "{letter}"
            );
        }
        // And what is left is refused as too large.
        let column = tabletist_db::ColumnInfo {
            name: "note".into(),
            type_name: "text".into(),
            ..tabletist_db::ColumnInfo::default()
        };
        assert_eq!(
            crate::edit::check(tabletist_db::Dialect::Postgres, &column, &text),
            Some(Problem::TooLarge)
        );
    }

    #[test]
    fn every_problem_and_every_lock_has_words() {
        let locale = Locale::English;
        let say =
            |problem: &Problem, typed: Option<&str>| problem_text(problem, "int8", typed, locale);
        assert_eq!(
            say(&Problem::WholeNumber, None),
            "int8 expects a whole number"
        );
        assert_eq!(
            say(
                &Problem::OutOfRange {
                    min: -128,
                    max: 127
                },
                None
            ),
            "int8 holds -128 to 127"
        );
        assert_eq!(say(&Problem::Number, None), "int8 expects a number");
        let decimals = Problem::Decimals {
            scale: 2,
            stored: "12.51".into(),
        };
        assert_eq!(
            say(&decimals, Some("12.505")),
            "Up to 2 decimals. 12.505 would be stored as 12.51."
        );
        // The spaces a database overlooks round a value are not the value.
        assert_eq!(
            say(&decimals, Some(" 12.505 ")),
            "Up to 2 decimals. 12.505 would be stored as 12.51."
        );
        assert_eq!(say(&decimals, None), "Up to 2 decimals.");
        let one = Problem::Decimals {
            scale: 1,
            stored: "0.3".into(),
        };
        assert_eq!(say(&one, None), "Up to 1 decimal.");
        // A scale of zero keeps none, and one below zero keeps zeros in
        // the places before the point.
        let none = Problem::Decimals {
            scale: 0,
            stored: "13".into(),
        };
        assert_eq!(
            say(&none, Some("12.5")),
            "No decimals. 12.5 would be stored as 13."
        );
        assert_eq!(say(&none, None), "No decimals.");
        let rounds = |scale: i32, typed: &str, stored: &str| {
            let problem = Problem::Decimals {
                scale,
                stored: stored.into(),
            };
            say(&problem, Some(typed))
        };
        assert_eq!(
            rounds(-1, "12345", "12350"),
            "Whole tens only. 12345 would be stored as 12350."
        );
        assert_eq!(
            rounds(-2, "12345", "12300"),
            "Whole hundreds only. 12345 would be stored as 12300."
        );
        assert_eq!(
            rounds(-3, "12345", "12000"),
            "Whole thousands only. 12345 would be stored as 12000."
        );
        assert_eq!(
            rounds(-4, "12345", "10000"),
            "Whole multiples of 10000 only. 12345 would be stored as 10000."
        );
        let hundreds = Problem::Decimals {
            scale: -2,
            stored: "12300".into(),
        };
        assert_eq!(say(&hundreds, None), "Whole hundreds only.");
        assert_eq!(
            say(
                &Problem::Under {
                    limit: "0.01".into()
                },
                Some("0.01234")
            ),
            "int8 holds values between -0.01 and 0.01"
        );
        // As true of a value below zero as of one above it.
        assert_eq!(
            say(&Problem::Under { limit: "1".into() }, Some("-1.5")),
            "int8 holds values between -1 and 1"
        );
        assert_eq!(
            say(&Problem::Digits { whole: 8 }, None),
            "At most 8 digits before the point"
        );
        assert_eq!(
            say(&Problem::Digits { whole: 1 }, None),
            "At most 1 digit before the point"
        );
        let inexact = Problem::Inexact {
            stored: "0.30000000000000004".into(),
        };
        assert_eq!(
            say(&inexact, Some("0.300000000000000044")),
            "0.300000000000000044 would be stored as 0.30000000000000004"
        );
        assert_eq!(
            say(&inexact, None),
            "Would be stored as 0.30000000000000004"
        );
        assert_eq!(say(&Problem::Boolean, None), "int8 expects true or false");
        let allowed = vec!["print".to_owned(), "ebook".to_owned(), "audio".to_owned()];
        assert_eq!(
            say(&Problem::NotOneOf(allowed), None),
            "Not one of: print, ebook, audio"
        );
        let json = Problem::Json {
            message: "expected , or }".into(),
            line: 3,
            column: 23,
        };
        assert_eq!(say(&json, None), "Expected , or } at 3:23");
        assert_eq!(
            say(&Problem::TooLong { max: 200 }, None),
            "At most 200 characters"
        );
        assert_eq!(
            say(&Problem::TooLong { max: 1 }, None),
            "At most 1 character"
        );
        assert_eq!(
            say(&Problem::TooLarge, None),
            "Over 256 KiB: values this large cannot be edited yet"
        );

        let why = |lock| lock_text(lock, "book_covers", locale);
        assert_eq!(why(Lock::ReadOnly), "This connection opens read-only");
        assert_eq!(why(Lock::NotATable), "Views cannot be edited");
        assert_eq!(
            why(Lock::NoKey),
            "book_covers has no primary key or unique index, so a row can't be targeted safely"
        );
        assert_eq!(
            why(Lock::KeyType),
            "book_covers's key cannot be matched exactly, so a row can't be targeted safely"
        );
        assert_eq!(why(Lock::KeyIsNull), "This row's key is NULL");
        assert_eq!(why(Lock::Gone), "This row no longer exists on the server");
        assert_eq!(why(Lock::Generated), "Computed by the database");
        assert_eq!(why(Lock::Assigned), "Assigned by the database on save");
        assert_eq!(why(Lock::NotNull), "This column cannot be NULL");
        assert_eq!(why(Lock::KeyColumn), "Part of the row's key");
        // A cell that is not there has nothing to say; every other has.
        assert_eq!(why(Lock::NoSuchCell), "");
        for lock in [
            Lock::ReadOnly,
            Lock::NotATable,
            Lock::StructureLoading,
            Lock::NoKey,
            Lock::KeyType,
            Lock::Saving,
            Lock::Refreshing,
            Lock::Gone,
            Lock::KeyIsNull,
            Lock::KeyInexact,
            Lock::UnknownColumn,
            Lock::Generated,
            Lock::Assigned,
            Lock::NotNull,
            Lock::KeyColumn,
            Lock::Binary,
            Lock::TooLarge,
        ] {
            assert!(!why(lock).is_empty(), "{lock:?}");
        }
    }
}
