//! A toast: one line that says what was just done ("Copied SQL"), for a
//! moment and then no longer. On macOS and Windows it floats over the
//! bottom of the window, centred; the terminal look says it in its status
//! line. It is the frame's own, as a tooltip is: nothing of the app's
//! state, and nothing a save or a reload has to know of.
//!
//! One kind takes a click: a copy over the clipboard's limit offers
//! Export… (spec 3b, R6). It stays while the pointer is over it or its
//! button has the keyboard.

use egui::{Align2, Area, Frame, Id, Margin, Order, vec2};

use crate::i18n::gettext;
use crate::keymap::Layout;
use crate::model::ExportOffer;
use crate::ui::Paint;
use crate::ui::widgets::{self, ButtonSpec};

/// How long a toast is said, in seconds.
pub const SECONDS: f64 = 4.0;

/// How far above the window's bottom edge it floats.
const LIFT: f32 = 16.0;

/// What is being said, since when in egui's seconds, and for how long.
#[derive(Clone)]
struct Said {
    text: String,
    when: f64,
    seconds: f64,
    /// The sheet Export… opens.
    offer: Option<ExportOffer>,
}

fn id() -> Id {
    Id::new("toast")
}

/// Says `text`, already in the user's language, for [`SECONDS`]. It takes
/// the place of what was being said.
pub fn say(ctx: &egui::Context, text: &str) {
    say_for(ctx, text, SECONDS);
}

/// [`say`], for `seconds`: what is told of rather than what was done (an
/// update available) stays longer.
pub fn say_for(ctx: &egui::Context, text: &str, seconds: f64) {
    keep(ctx, text, seconds, None);
}

/// [`say`], with Export… after the words, which opens `offer`'s sheet.
/// The terminal look's status line says the words alone: they name the
/// key.
pub fn offer(ctx: &egui::Context, text: &str, offer: ExportOffer) {
    keep(ctx, text, SECONDS, Some(offer));
}

fn keep(ctx: &egui::Context, text: &str, seconds: f64, offer: Option<ExportOffer>) {
    let said = Said {
        text: text.to_owned(),
        when: ctx.input(|input| input.time),
        seconds,
        offer,
    };
    ctx.data_mut(|data| data.insert_temp(id(), said));
    ctx.request_repaint();
}

/// The sheet a toast that is being said offers: the terminal look's `E`
/// opens it while its line says so.
pub fn offered(ctx: &egui::Context) -> Option<ExportOffer> {
    said(ctx)?;
    kept(ctx)?.offer
}

/// Takes the offer off the toast where `holds` says it no longer stands:
/// the words stay, Export… goes.
pub fn keep_offer_if(ctx: &egui::Context, holds: impl FnOnce(&ExportOffer) -> bool) {
    let Some(mut said) = kept(ctx) else {
        return;
    };
    if said.offer.as_ref().is_some_and(|offer| !holds(offer)) {
        said.offer = None;
        ctx.data_mut(|data| data.insert_temp(id(), said));
    }
}

fn kept(ctx: &egui::Context) -> Option<Said> {
    ctx.data(|data| data.get_temp(id()))
}

/// What is being said. While something is, a frame is asked for when its
/// moment ends: it goes without waiting for the pointer to move.
pub fn said(ctx: &egui::Context) -> Option<String> {
    let said: Said = ctx.data(|data| data.get_temp(id()))?;
    let left = said.seconds - (ctx.input(|input| input.time) - said.when);
    if left <= 0.0 {
        ctx.data_mut(|data| data.remove::<Said>(id()));
        return None;
    }
    ctx.request_repaint_after(std::time::Duration::from_secs_f64(left));
    Some(said.text)
}

/// The toast of macOS and Windows, over everything else in the window, a
/// dialog too: the text colour filled, the window's colour written on it,
/// as the primary button is. It takes neither the pointer nor the
/// keyboard, but for an offer's Export…, whose click it returns. The
/// terminal look draws none: its status line says it.
pub fn show(ctx: &egui::Context, paint: Paint, dialog: bool) -> Option<ExportOffer> {
    if paint.look.terminal {
        return None;
    }
    let text = said(ctx)?;
    let offer = kept(ctx).and_then(|said| said.offer);
    let offering = offer.is_some();
    let (look, palette) = (&paint.look, &paint.palette);
    let mut clicked = None;
    let shown = Area::new(id())
        .order(match offering {
            true => Order::Foreground,
            false => Order::Tooltip,
        })
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -LIFT))
        .interactable(offering)
        .show(ctx, |ui| {
            Frame::new()
                .fill(palette.text)
                .corner_radius(9)
                .inner_margin(Margin::symmetric(12, 8))
                .shadow(egui::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: palette.shadow,
                })
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let said =
                            widgets::label(ui, widgets::body(look), &text, palette.window, look);
                        // Read out as it appears, without taking the user's
                        // place.
                        ui.ctx().accesskit_node_builder(said.id, |node| {
                            node.set_live(egui::accesskit::Live::Polite);
                        });
                        let Some(offer) = offer else {
                            return false;
                        };
                        ui.add_space(8.0);
                        let words = gettext(paint.locale, "Export…");
                        let button = ButtonSpec::new(&words).link().show(ui, 22.0, look, palette);
                        self::button(ui.ctx(), &button, true);
                        // Under a dialog the click is the dialog's to ignore.
                        if button.clicked() && !dialog {
                            clicked = Some(offer);
                        }
                        button.has_focus()
                    })
                    .inner
                })
                .inner
        });
    // Under the pointer or with the keyboard, the offer waits; once
    // asked, it goes.
    let waits = shown.response.contains_pointer() || shown.inner || clicked.is_some();
    if offering
        && waits
        && let Some(mut said) = kept(ctx)
    {
        said.when = ctx.input(|input| input.time);
        if clicked.is_some() {
            said.seconds = 0.0;
        }
        ctx.data_mut(|data| data.insert_temp(id(), said));
    }
    clicked
}

/// What ⌘⇧' left for the toasts: the keyboard asked for, and what had it.
#[derive(Clone, Default)]
struct Keyboard {
    /// The frame ⌘⇧' was pressed in: the next toast drawn (in that frame
    /// or the one after) gives its first button the keyboard.
    asked: Option<u64>,
    /// What had the keyboard before: Esc gives it back.
    behind: Option<Id>,
    /// The toasts' buttons the frame before drew, and this frame's.
    before: Vec<Id>,
    drawn: Vec<Id>,
    frame: u64,
    /// The button that had the keyboard as it was drawn: egui moves the
    /// keyboard on Tab before the frame's keys are read, so where it was
    /// is kept here.
    had: Option<Id>,
}

fn keyboard_id() -> Id {
    id().with("keyboard")
}

/// Says that `response` is a button of a toast (the offer's Export…, an
/// export's Cancel, its pill, Show in Finder, a failure's action and ×),
/// the first of its toast where `first`: ⌘⇧' gives that one the keyboard.
pub fn button(ctx: &egui::Context, response: &egui::Response, first: bool) {
    let now = ctx.cumulative_frame_nr();
    let focused = response.has_focus();
    let take = ctx.data_mut(|data| {
        let keyboard: &mut Keyboard = data.get_temp_mut_or_default(keyboard_id());
        if keyboard.frame != now {
            keyboard.before = std::mem::take(&mut keyboard.drawn);
            keyboard.frame = now;
            keyboard.had = None;
        }
        if focused {
            keyboard.had = Some(response.id);
        }
        if !keyboard.drawn.contains(&response.id) {
            keyboard.drawn.push(response.id);
        }
        let asked = keyboard.asked.is_some_and(|when| when + 1 >= now);
        if first && asked {
            keyboard.asked = None;
        }
        first && asked
    });
    if take {
        response.request_focus();
    }
    // Tab and Esc are the toast's own while one of its buttons has the
    // keyboard ([`keys`]): egui would take Tab out of the toast.
    if focused {
        let filter = egui::EventFilter {
            tab: true,
            escape: true,
            ..Default::default()
        };
        ctx.memory_mut(|memory| memory.set_focus_lock_filter(response.id, filter));
    }
}

/// The toasts' buttons on screen: the last frame drawn's, and this one's
/// so far.
fn buttons(ctx: &egui::Context) -> Vec<Id> {
    let keyboard: Keyboard = ctx.data(|data| data.get_temp(keyboard_id()).unwrap_or_default());
    let now = ctx.cumulative_frame_nr();
    // A toast gone since draws nothing: what it drew is not there.
    if keyboard.frame + 1 < now {
        return Vec::new();
    }
    let mut shown = keyboard.drawn;
    if keyboard.frame == now {
        shown.extend(keyboard.before);
    }
    shown
}

/// The places of `rects` as they are read: by line, top to bottom (two
/// whose middles are within a toast's half height share a line), then
/// left to right.
fn reading_order(rects: &[egui::Rect]) -> Vec<usize> {
    const LINE: f32 = 12.0;
    let mut order: Vec<usize> = (0..rects.len()).collect();
    order.sort_by(|one, two| rects[*one].center().y.total_cmp(&rects[*two].center().y));
    // Lines, each from its first's middle.
    let mut lines: Vec<(f32, Vec<usize>)> = Vec::new();
    for at in order {
        let y = rects[at].center().y;
        match lines.last_mut() {
            Some((top, line)) if y - *top <= LINE => line.push(at),
            _ => lines.push((y, vec![at])),
        }
    }
    lines
        .into_iter()
        .flat_map(|(_, mut line)| {
            line.sort_by(|one, two| rects[*one].left().total_cmp(&rects[*two].left()));
            line
        })
        .collect()
}

/// Whether a button of a toast has the keyboard.
pub fn has_keyboard(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.focused())
        .is_some_and(|focused| buttons(ctx).contains(&focused))
}

/// The toast's button that had the keyboard as the last frame drew it.
fn had(ctx: &egui::Context) -> Option<Id> {
    let keyboard: Keyboard = ctx.data(|data| data.get_temp(keyboard_id()).unwrap_or_default());
    (keyboard.frame + 1 >= ctx.cumulative_frame_nr())
        .then_some(keyboard.had)
        .flatten()
}

/// The toasts' keys, before anything else reads the frame's (spec 3b, R9):
/// ⌘⇧' gives the first button of the toast that has one the keyboard,
/// and Esc on a toast's button gives it back to what had it. Nothing
/// under a dialog, and nothing where no toast has a button.
pub fn keys(ctx: &egui::Context, keymap: &crate::keymap::Keymap, layout: Layout, dialog: bool) {
    use crate::keymap::{Command, Scope};
    use crate::ui::keys::asked;
    let had = had(ctx);
    let on_toast = had.is_some() || has_keyboard(ctx);
    let any = !buttons(ctx).is_empty();
    let (focus, leave, step) = ctx.input_mut(|input| {
        let focus = !dialog
            && any
            && asked(
                input,
                keymap,
                layout,
                Command::FocusToast,
                Scope::Global,
                true,
            )
            .count
                > 0;
        let on = |input: &mut egui::InputState, command| {
            on_toast && asked(input, keymap, layout, command, Scope::Prompt, true).count > 0
        };
        let leave = on(input, Command::LeaveToast);
        // Shift+Tab before Tab: Tab's chord would take it too.
        let step = match (
            on(input, Command::ToastPrevious),
            on(input, Command::ToastNext),
        ) {
            (true, _) => -1,
            (false, true) => 1,
            (false, false) => 0,
        };
        (focus, leave, step)
    });
    let focused = ctx.memory(|memory| memory.focused());
    // ⇥ and ⇧⇥ go round every toast's buttons (spec 4, S13): top to
    // bottom, and left to right along a toast.
    if step != 0
        && let Some(focused) = had.or(focused)
    {
        let mut shown: Vec<egui::Rect> = Vec::new();
        let mut ids: Vec<Id> = Vec::new();
        for response in buttons(ctx)
            .into_iter()
            .filter_map(|id| ctx.read_response(id))
        {
            if !ids.contains(&response.id) {
                ids.push(response.id);
                shown.push(response.rect);
            }
        }
        let order = reading_order(&shown);
        let ordered: Vec<Id> = order.into_iter().map(|at| ids[at]).collect();
        if let Some(at) = ordered.iter().position(|id| *id == focused) {
            let next = (at as isize + step).rem_euclid(ordered.len() as isize) as usize;
            ctx.memory_mut(|memory| {
                memory.request_focus(ordered[next]);
                memory.move_focus(egui::FocusDirection::None);
            });
        }
    }
    if focus {
        let now = ctx.cumulative_frame_nr();
        ctx.data_mut(|data| {
            let keyboard: &mut Keyboard = data.get_temp_mut_or_default(keyboard_id());
            keyboard.asked = Some(now);
            if !on_toast {
                keyboard.behind = focused;
            }
        });
        ctx.request_repaint();
    }
    if leave {
        let behind = ctx.data_mut(|data| {
            let keyboard: &mut Keyboard = data.get_temp_mut_or_default(keyboard_id());
            keyboard.behind.take()
        });
        match behind.filter(|id| ctx.read_response(*id).is_some()) {
            Some(id) => ctx.memory_mut(|memory| memory.request_focus(id)),
            None => {
                if let Some(id) = focused {
                    ctx.memory_mut(|memory| memory.surrender_focus(id));
                }
            }
        }
    }
}
