//! What the terminal look's dialogs are made of: a head of one tinted line
//! and a foot whose key hints are its buttons. The connection dialog and
//! the prompts about pending changes are drawn with them.

use egui::{Color32, CornerRadius, Rect, Sense, Ui, pos2, vec2};

use crate::theme::{Look, Palette};
use crate::typography::Text;
use crate::ui::widgets::{self, ButtonSpec};

/// The height of a head or a foot: 40 and its rule.
const BAND: f32 = 41.0;

/// The head across the width `ui` has left: `fill` under the dialog's top
/// corners (rounded at `radius`) and a rule under it. Returns its place.
/// What it says is painted by the caller, on [`head_line`].
pub fn head(ui: &mut Ui, radius: u8, fill: Color32, palette: &Palette) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAND), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: radius,
            ne: radius,
            sw: 0,
            se: 0,
        },
        fill,
    );
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
    rect
}

/// The line a head's text centres on.
pub fn head_line(head: Rect) -> f32 {
    head.top() + 20.0
}

/// The foot across the width `ui` has left: the panel's tone over the
/// dialog's bottom corners (rounded at `radius`), under a rule. Returns
/// its place. What it says is painted by the caller, on [`foot_line`].
pub fn foot(ui: &mut Ui, radius: u8, palette: &Palette) -> Rect {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), BAND), Sense::hover());
    ui.painter().rect_filled(
        rect,
        CornerRadius {
            nw: 0,
            ne: 0,
            sw: radius,
            se: radius,
        },
        palette.panel,
    );
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    rect
}

/// The line a foot's text centres on, under its rule.
pub fn foot_line(foot: Rect) -> f32 {
    foot.top() + 1.0 + 20.0
}

/// One key of a foot.
pub struct Key<'a> {
    /// The key as the keymap wrote it: `ctrl+s`, `[w]`.
    pub key: &'a crate::keymap::Written,
    /// What it does.
    pub label: &'a str,
    /// The name of the button the hint stands for, when it has one: a
    /// click on the hint presses it, and a screen reader reads it.
    pub button: Option<&'a str>,
    /// The key the dialog is for: it takes the accent.
    pub lead: bool,
    /// Why its button cannot be pressed, while it cannot.
    pub disabled: Option<&'a str>,
}

impl<'a> Key<'a> {
    /// Its button, as [`keys`] makes it.
    fn spec(&self) -> Option<ButtonSpec<'a>> {
        let button = ButtonSpec::new(self.button?);
        Some(match self.disabled {
            Some(reason) => button.disabled(reason),
            None => button,
        })
    }
}

/// The place in `keys` of the one whose button has the keyboard. Asked
/// before [`keys`] draws them, by a box that reads Enter itself: a button
/// that has the keyboard would take the key as a press of itself.
pub fn keyboard_on(ui: &Ui, keys: &[Key<'_>]) -> Option<usize> {
    let has = |key: &Key<'_>| key.spec().is_some_and(|button| button.has_keyboard(ui));
    keys.iter().position(has)
}

/// The keys of a foot, ending 14 in from its right edge, 16 apart. Each
/// hint is its button too. `taken` is the width something at the foot's
/// left needs: the hints that do not fit beside it are left out, from the
/// left, and their buttons stay for screen readers. Returns the place in
/// `keys` of the one whose button was pressed.
pub fn keys(
    ui: &mut Ui,
    foot: Rect,
    taken: f32,
    keys: &[Key<'_>],
    look: &Look,
    palette: &Palette,
) -> Option<usize> {
    let role = widgets::secondary(look);
    let y = foot_line(foot);
    let hint = |key: &Key<'_>| {
        let color = if key.lead {
            palette.accent
        } else {
            palette.text
        };
        Text::new(look)
            .add(role, key.key, color)
            .space(role, " ")
            .add(role, key.label, palette.dim)
    };
    let widths: Vec<f32> = keys
        .iter()
        .map(|key| widgets::measure(ui, hint(key)))
        .collect();
    let room = foot.width() - 28.0 - taken;
    let mut total = widths.iter().sum::<f32>() + 16.0 * keys.len().saturating_sub(1) as f32;
    let mut dropped = 0;
    while dropped < keys.len() && total > room {
        total -= widths[dropped] + 16.0;
        dropped += 1;
    }
    let mut left = foot.right() - 14.0 - total.max(0.0);
    // Where a hint that is left out keeps its button.
    let edge = Rect::from_min_size(pos2(foot.right() - 1.0, foot.top() + 1.0), vec2(1.0, 1.0));
    let mut pressed = None;
    for (index, (key, width)) in keys.iter().zip(widths).enumerate() {
        let place = if index < dropped {
            edge
        } else {
            widgets::paint_text(ui, left, y, hint(key));
            let place = Rect::from_min_max(
                pos2(left, foot.top() + 1.0),
                pos2(left + width, foot.bottom()),
            );
            left += width + 16.0;
            place
        };
        if let Some(button) = key.spec()
            && button.hidden_at(ui, place).clicked()
        {
            pressed = Some(index);
        }
    }
    pressed
}
