//! A toast: one line that says what was just done ("Copied SQL"), for a
//! moment and then no longer. On macOS and Windows it floats over the
//! bottom of the window, centred; the terminal look says it in its status
//! line. It is the frame's own, as a tooltip is: nothing of the app's
//! state, and nothing a save or a reload has to know of.

use egui::{Align2, Area, Frame, Id, Margin, Order, vec2};

use crate::app::App;
use crate::ui::widgets;

/// How long a toast is said, in seconds.
pub const SECONDS: f64 = 4.0;

/// How far above the window's bottom edge it floats.
const LIFT: f32 = 16.0;

/// What is being said, and since when, in egui's seconds.
#[derive(Clone)]
struct Said {
    text: String,
    when: f64,
}

fn id() -> Id {
    Id::new("toast")
}

/// Says `text`, already in the user's language, for [`SECONDS`]. It takes
/// the place of what was being said.
pub fn say(ctx: &egui::Context, text: &str) {
    let said = Said {
        text: text.to_owned(),
        when: ctx.input(|input| input.time),
    };
    ctx.data_mut(|data| data.insert_temp(id(), said));
    ctx.request_repaint();
}

/// What is being said. While something is, a frame is asked for when its
/// moment ends: it goes without waiting for the pointer to move.
pub fn said(ctx: &egui::Context) -> Option<String> {
    let said: Said = ctx.data(|data| data.get_temp(id()))?;
    let left = SECONDS - (ctx.input(|input| input.time) - said.when);
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
/// keyboard. The terminal look draws none: its status line says it.
pub fn show(app: &App, ctx: &egui::Context) {
    if app.look.terminal {
        return;
    }
    let Some(text) = said(ctx) else {
        return;
    };
    let (look, palette) = (&app.look, &app.palette);
    Area::new(id())
        .order(Order::Tooltip)
        .anchor(Align2::CENTER_BOTTOM, vec2(0.0, -LIFT))
        .interactable(false)
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
                    widgets::label(ui, widgets::body(look), &text, palette.window, look);
                });
        });
}
