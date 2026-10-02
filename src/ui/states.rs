//! Empty, loading and error states: what a view shows in place of its
//! content, drawn the same wherever it appears. A view says what to say
//! (an icon, a title, a sentence, the buttons); this module places it.

use std::time::Duration;

use egui::{Align, Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, Ui};
use egui::{pos2, vec2};

use crate::theme::{self, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets::{self, ButtonSpec};

/// A wait shows nothing of its own before this: an answer that comes at
/// once never flashes a spinner.
pub const DELAY: Duration = Duration::from_millis(300);

/// How much of stale content shows through: what a lost connection left
/// on screen stays readable and reads as old.
pub const STALE: f32 = 0.55;

/// The widest a state's sentence runs before it wraps.
const MEASURE: f32 = 340.0;

/// From the edge of its area to a state's text.
pub const INSET: f32 = 18.0;

/// Whether a wait has gone on long enough to show. This schedules
/// nothing: a caller that waits asks for a repaint after `DELAY - waited`.
pub fn lasted(waited: Duration) -> bool {
    waited >= DELAY
}

/// The height of a state's buttons.
pub fn button_height(look: &Look) -> f32 {
    if look.terminal { 24.0 } else { 32.0 }
}

/// A state's first line: why the area is empty, or what failed.
pub fn title_role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::StateTitle, TextRole::OGroup)
}

/// A button of a state: a bordered one, or in the terminal look a hint
/// (the text muted, its key in the text colour).
pub fn button<'a>(text: &'a str, look: &Look) -> ButtonSpec<'a> {
    if look.terminal {
        ButtonSpec::new(text)
            .hint()
            .shortcut_role(TextRole::OBody)
            .padding(11.0)
            .gap(8.0)
    } else {
        ButtonSpec::new(text)
    }
}

/// What an empty area says: why it is empty, in a title and a sentence
/// (which may be empty).
pub struct Notice<'a> {
    pub icon: Icon,
    pub title: &'a str,
    pub text: &'a str,
}

/// Which pieces of an empty state show beside its title and its buttons,
/// which always do.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Shown {
    tile: bool,
    text: bool,
}

/// What `room` has the height for: everything, or all but the icon tile,
/// or neither the tile nor the sentence. `tile` and `text` are the heights
/// of those two with the gap each brings, `rest` that of the title and the
/// buttons.
fn fit(tile: f32, text: f32, rest: f32, room: f32) -> Shown {
    let (tile, text) = if tile + text + rest <= room {
        (true, true)
    } else {
        (false, text + rest <= room)
    };
    Shown { tile, text }
}

/// `notice` in `rect` with `buttons` under it: centred under an icon tile,
/// or in the terminal look from the top left without one. An area too
/// short for it all loses the tile, then the sentence, and nothing shows
/// outside `rect`. Returns the place in `buttons` of the one clicked.
/// A button's id comes from its name, so one with a name already used in
/// the same `Ui` needs `.salt(...)`.
pub fn empty(
    ui: &mut Ui,
    rect: Rect,
    notice: &Notice<'_>,
    buttons: Vec<ButtonSpec<'_>>,
    look: &Look,
    palette: &Palette,
) -> Option<usize> {
    // `ui` is cut to the area while the state is drawn, the buttons too:
    // what the area has no room for never shows over its neighbours. A
    // child `Ui` would cut the same, and give all it holds another id.
    let clip = ui.clip_rect();
    ui.set_clip_rect(rect.intersect(clip));
    let clicked = place(ui, rect, notice, buttons, look, palette);
    ui.set_clip_rect(clip);
    clicked
}

/// The pieces of [`empty`] that fit, each where it goes.
fn place(
    ui: &mut Ui,
    rect: Rect,
    notice: &Notice<'_>,
    buttons: Vec<ButtonSpec<'_>>,
    look: &Look,
    palette: &Palette,
) -> Option<usize> {
    let ctx = ui.ctx().clone();
    let room = (rect.width() - 2.0 * INSET).max(0.0);
    let measure = if look.terminal {
        room
    } else {
        room.min(MEASURE)
    };
    let align = if look.terminal {
        Align::Min
    } else {
        Align::Center
    };
    let lay = |role: TextRole, text: &str, color: Color32| {
        let mut text = Text::one(look, role, text, color).wrap(measure);
        text.job_mut().halign = align;
        text.layout(&ctx)
    };
    let title = lay(title_role(look), notice.title, palette.text);
    let text =
        (!notice.text.is_empty()).then(|| lay(widgets::body(look), notice.text, palette.secondary));
    let widths: Vec<f32> = buttons
        .iter()
        .map(|button| button.width(ui, look))
        .collect();
    let height = button_height(look);
    // What sits between the pieces: under the tile, the title and the
    // sentence, and between the buttons.
    let (tile, under_tile, under_title, under_text, between) = if look.terminal {
        (0.0, 0.0, 4.0, 8.0, 8.0)
    } else {
        (44.0, 14.0, 8.0, 14.0, 8.0)
    };
    let tile_height = tile + under_tile;
    let text_height = text
        .as_ref()
        .map_or(0.0, |text| under_title + text.height());
    let buttons_height = if buttons.is_empty() {
        0.0
    } else {
        under_text + height
    };
    let rest = title.height() + buttons_height;
    let shown = fit(tile_height, text_height, rest, rect.height() - 2.0 * INSET);
    let text = text.filter(|_| shown.text);
    let mut total = rest;
    if shown.tile {
        total += tile_height;
    }
    if shown.text {
        total += text_height;
    }
    // The line the pieces hang on, and where they start: the middle of the
    // area, or its top left in the terminal look. Kept under the area's
    // top when there is too little room to centre.
    let (x, mut y) = if look.terminal {
        (rect.left() + INSET, rect.top() + INSET)
    } else {
        let top = rect.center().y - total / 2.0;
        (rect.center().x, top.max(rect.top() + INSET))
    };
    if shown.tile && !look.terminal {
        let tile_rect = Rect::from_center_size(pos2(x, y + tile / 2.0), vec2(tile, tile));
        ui.painter().rect_filled(
            tile_rect,
            CornerRadius::same(look.radius + 4),
            palette.surface,
        );
        notice.icon.image(palette.dim, 20.0).paint_at(
            ui,
            Rect::from_center_size(tile_rect.center(), vec2(20.0, 20.0)),
        );
        y += tile_height;
    }
    // A galley aligned to the centre hangs on its middle, one aligned to
    // the left on its left edge: `x` is that line either way.
    let paint = |laid: &crate::typography::Laid, said: &str, y: f32| {
        laid.paint(ui.painter(), pos2(x, y));
        let left = if look.terminal {
            x
        } else {
            x - laid.width() / 2.0
        };
        widgets::announce(ui, Rect::from_min_size(pos2(left, y), laid.size()), said);
    };
    paint(&title, notice.title, y);
    y += title.height();
    if let Some(text) = &text {
        y += under_title;
        paint(text, notice.text, y);
        y += text.height();
    }
    if buttons.is_empty() {
        return None;
    }
    y += under_text;
    let all: f32 = widths.iter().sum::<f32>() + between * (widths.len() - 1) as f32;
    let mut left = if look.terminal { x } else { x - all / 2.0 };
    let mut clicked = None;
    for (index, (button, width)) in buttons.into_iter().zip(widths).enumerate() {
        let at = Rect::from_min_size(pos2(left, y), vec2(width, height));
        if button.show_at(ui, at, look, palette).clicked() {
            clicked = Some(index);
        }
        left += width + between;
    }
    clicked
}

/// How a card reads: something failed, or something was held back.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Danger,
    Warning,
}

impl Tone {
    pub fn color(self, palette: &Palette) -> Color32 {
        match self {
            Self::Danger => palette.danger,
            Self::Warning => palette.warning,
        }
    }

    /// The fill of a card or a banner in this tone.
    pub fn fill(self, palette: &Palette) -> Color32 {
        theme::mix(palette.window, self.color(palette), 0.10)
    }

    /// The line round a card or under a banner in this tone.
    pub fn line(self, palette: &Palette) -> Color32 {
        theme::mix(palette.window, self.color(palette), 0.25)
    }
}

/// What failed, in plain words: a title and the sentence under it.
pub struct Card<'a> {
    pub tone: Tone,
    pub icon: Icon,
    pub title: &'a str,
    pub text: &'a str,
}

/// `card` across the width `ui` has left: a tinted box with a line round
/// it, or in the terminal look a square one with a bar down its left.
pub fn card(ui: &mut Ui, card: &Card<'_>, look: &Look, palette: &Palette) {
    let tone = card.tone.color(palette);
    let (corner, stroke) = if look.terminal {
        (CornerRadius::ZERO, Stroke::NONE)
    } else {
        (
            CornerRadius::same(look.radius + 2),
            Stroke::new(1.0, card.tone.line(palette)),
        )
    };
    let strong = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
    // The terminal's title takes the tone: it has no icon to carry it.
    let title = if look.terminal { tone } else { palette.text };
    let shown = Frame::new()
        .fill(card.tone.fill(palette))
        .stroke(stroke)
        .corner_radius(corner)
        .inner_margin(Margin::same(14))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = vec2(12.0, 0.0);
                if !look.terminal {
                    let (at, _) = ui.allocate_exact_size(vec2(18.0, 18.0), Sense::hover());
                    card.icon.image(tone, 18.0).paint_at(ui, at);
                }
                let width = ui.available_width();
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 4.0;
                    Text::one(look, strong, card.title, title)
                        .wrap(width)
                        .layout(ui.ctx())
                        .label(ui);
                    if !card.text.is_empty() {
                        Text::one(look, widgets::body(look), card.text, palette.secondary)
                            .wrap(width)
                            .layout(ui.ctx())
                            .label(ui);
                    }
                });
            });
        });
    if look.terminal {
        let rect = shown.response.rect;
        let bar = Rect::from_min_size(rect.min, vec2(3.0, rect.height()));
        ui.painter().rect_filled(bar, CornerRadius::ZERO, tone);
    }
}

/// How far a step of a wait has come.
#[derive(Clone, Copy, PartialEq)]
pub enum StepState {
    Done,
    /// Under way, and for how long when that is known.
    Running(Option<Duration>),
    Waiting,
}

/// One step of a wait: what it does, and to what (a host, a user), which
/// reads in the code face.
pub struct Step<'a> {
    pub state: StepState,
    pub text: &'a str,
    pub detail: &'a str,
}

/// The height of one step and the room under it.
fn step_pitch(look: &Look) -> f32 {
    if look.terminal { 21.0 } else { 30.0 }
}

/// The height [`steps`] takes for `count` of them.
pub fn steps_height(count: usize, look: &Look) -> f32 {
    step_pitch(look) * count as f32
}

/// "2.4 s": how long something has been going. A running time keeps one
/// unit so it does not jump: "0.3 s" where `format::elapsed` writes "300 ms".
pub fn seconds(elapsed: Duration) -> String {
    format!("{:.1} s", elapsed.as_secs_f64())
}

/// `steps` from the top of `rect`, one per line: a tick for a step that is
/// done, a spinner and the time for the one under way, a ring for those
/// still to come.
pub fn steps(ui: &mut Ui, rect: Rect, steps: &[Step<'_>], look: &Look, palette: &Palette) {
    const MARK: f32 = 14.0;
    let pitch = step_pitch(look);
    let body = widgets::body(look);
    let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
    let small = widgets::secondary(look);
    for (index, step) in steps.iter().enumerate() {
        let row = Rect::from_min_size(
            pos2(rect.left(), rect.top() + pitch * index as f32),
            vec2(rect.width(), pitch),
        );
        let center = row.top() + 10.0;
        let mark = Rect::from_center_size(pos2(row.left() + MARK / 2.0, center), vec2(MARK, MARK));
        let (role, color, detail) = match step.state {
            StepState::Done => (body, palette.text, palette.dim),
            StepState::Running(_) => (strong, palette.text, palette.dim),
            StepState::Waiting => (body, palette.faint, palette.faint),
        };
        match step.state {
            StepState::Done => Icon::Check.image(palette.success, MARK).paint_at(ui, mark),
            // The spinner asks for the frames that keep the time going.
            StepState::Running(_) => egui::Spinner::new()
                .size(MARK)
                .color(palette.accent)
                .paint_at(ui, mark),
            StepState::Waiting if look.terminal => {
                ui.painter()
                    .circle_filled(mark.center(), 1.5, palette.faint);
            }
            StepState::Waiting => {
                let ring = Stroke::new(2.0, palette.outline);
                ui.painter()
                    .circle_stroke(mark.center(), MARK / 2.0 - 1.0, ring);
            }
        }
        let mut text = Text::one(look, role, step.text, color);
        if !step.detail.is_empty() {
            text = text
                .space(role, " ")
                .add(widgets::code(look), step.detail, detail);
        }
        widgets::paint_text(ui, row.left() + MARK + 10.0, center, text);
        if let StepState::Running(Some(elapsed)) = step.state {
            let time = Text::one(look, small, &seconds(elapsed), palette.dim);
            widgets::paint_text_right(ui, row.right(), center, time);
        }
        let said = if step.detail.is_empty() {
            step.text.to_owned()
        } else {
            format!("{} {}", step.text, step.detail)
        };
        widgets::announce(ui, row, &said);
    }
}

/// A wait that has lasted: a spinner, what is running and for how long,
/// and the button that cancels it, in a box in the middle of `rect` so it
/// reads over whatever is under it (a page that a refresh will replace).
/// Returns whether the button was clicked.
pub fn running(
    ui: &mut Ui,
    rect: Rect,
    text: &str,
    elapsed: Duration,
    cancel: Option<ButtonSpec<'_>>,
    look: &Look,
    palette: &Palette,
) -> bool {
    let spinner = if look.terminal { 12.0 } else { 14.0 };
    let height = if look.terminal { 24.0 } else { 28.0 };
    let gap = 12.0;
    let role = widgets::body(look);
    let time = seconds(elapsed);
    // The time's room is that of a stand-in with every digit a 0: the box
    // keeps its width and its place while the digits change.
    let widest: String = time
        .chars()
        .map(|digit| if digit.is_ascii_digit() { '0' } else { digit })
        .collect();
    let line = Text::one(look, role, text, palette.text)
        .space(role, " ")
        .add(role, &widest, palette.dim);
    let width = widgets::measure(ui, line);
    let reserved = widgets::measure(ui, Text::one(look, role, &widest, palette.dim));
    let button_width = cancel.as_ref().map(|button| button.width(ui, look));
    let content = spinner + gap + width + button_width.map_or(0.0, |width| gap + width);
    // 14 before the spinner, 10 round the button.
    let card = Rect::from_center_size(rect.center(), vec2(14.0 + content + 10.0, height + 20.0));
    if look.terminal {
        // The terminal's box: square, a line round it, no shadow.
        ui.painter()
            .rect_filled(card, CornerRadius::ZERO, palette.window);
        ui.painter().rect_stroke(
            card,
            CornerRadius::ZERO,
            Stroke::new(1.0, palette.outline),
            StrokeKind::Inside,
        );
    } else {
        let corner = CornerRadius::same(look.radius + 2);
        let shadow = egui::epaint::Shadow {
            offset: [0, 6],
            blur: 20,
            spread: 0,
            color: palette.shadow.gamma_multiply(0.4),
        };
        ui.painter().add(shadow.as_shape(card, corner));
        ui.painter()
            .rect_filled(card, corner, widgets::raised_fill(palette));
        ui.painter().rect_stroke(
            card,
            corner,
            Stroke::new(widgets::hairline(ui), palette.border),
            StrokeKind::Inside,
        );
    }
    // The box takes the clicks on its text and its padding, and drops
    // them: it lies over a live page, which would get them otherwise.
    // Registered before the button, so the button, which comes after it,
    // still wins inside it. It does nothing, so the keyboard skips it.
    ui.interact(card, ui.id().with("running"), Sense::CLICK);
    let (mut x, center) = (card.left() + 14.0, card.center().y);
    // The spinner asks for the frames that keep the time going.
    egui::Spinner::new()
        .size(spinner)
        .color(palette.accent)
        .paint_at(
            ui,
            Rect::from_center_size(pos2(x + spinner / 2.0, center), vec2(spinner, spinner)),
        );
    x += spinner + gap;
    widgets::paint_text(ui, x, center, Text::one(look, role, text, palette.text));
    // The time is its own piece, from where its room starts.
    let time = Text::one(look, role, &time, palette.dim);
    widgets::paint_text(ui, x + width - reserved, center, time);
    // Named without the time, which changes every frame.
    let named = Rect::from_min_size(pos2(x, center - 8.0), vec2(width, 16.0));
    widgets::announce(ui, named, text);
    x += width + gap;
    match (cancel, button_width) {
        (Some(button), Some(width)) => {
            let at = Rect::from_min_size(pos2(x, center - height / 2.0), vec2(width, height));
            button.show_at(ui, at, look, palette).clicked()
        }
        _ => false,
    }
}

/// Rows of grey bars where rows are on their way: the shape of a grid
/// without its data. The terminal look draws none.
pub fn skeleton(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
    if look.terminal {
        return;
    }
    const BAR: f32 = 10.0;
    const GAP: f32 = 24.0;
    // The widths of a key, a number and a word; the last column takes a
    // share of what is left, a different one on each row.
    let fixed = [32.0, 76.0, 40.0];
    let shares = [1.0, 0.7, 0.85, 0.6];
    let fill = palette.surface.gamma_multiply(0.7);
    let rows = ((rect.height() / look.grid_row) as usize).min(6);
    for row in 0..rows {
        let center = rect.top() + look.grid_row * (row as f32 + 0.5);
        // The fixed bars, then the last one in the room that is left.
        let mut x = rect.left() + 12.0;
        let rest = (rect.right() - 12.0 - x - fixed.iter().sum::<f32>() - 3.0 * GAP).max(0.0);
        for width in fixed.into_iter().chain([rest * shares[row % shares.len()]]) {
            let at = Rect::from_min_size(pos2(x, center - BAR / 2.0), vec2(width, BAR));
            ui.painter().rect_filled(at, CornerRadius::same(5), fill);
            x += width + GAP;
        }
    }
}

/// A 2 pt line across `x` under `y` with a piece of it travelling: work
/// is going on, with no telling how much is left.
pub fn progress(ui: &Ui, x: egui::Rangef, y: f32, palette: &Palette) {
    let track = Rect::from_min_max(pos2(x.min, y), pos2(x.max, y + 2.0));
    ui.painter()
        .rect_filled(track, CornerRadius::ZERO, palette.selection);
    // A piece 30% of the line long crosses it every 1.6 s.
    let time = ui.input(|input| input.time);
    let at = ((time / 1.6).fract() as f32) * 1.3 - 0.3;
    let piece = Rect::from_min_max(
        pos2(x.min + x.span() * at, y),
        pos2(x.min + x.span() * (at + 0.3), y + 2.0),
    );
    ui.painter()
        .rect_filled(piece.intersect(track), CornerRadius::ZERO, palette.accent);
    ui.ctx().request_repaint();
}

#[cfg(test)]
mod tests {
    use egui::accesskit::Role;

    use super::*;
    use crate::testing::{Harness, bounds, node};

    fn notice() -> Notice<'static> {
        Notice {
            icon: Icon::Table,
            title: "No rows in fixture",
            text: "The table exists and is empty.",
        }
    }

    #[test]
    fn an_empty_state_says_why_and_offers_its_buttons_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let rect = ui.max_rect();
                let buttons = vec![button("Reload", &look), button("Clear", &look)];
                empty(ui, rect, &notice(), buttons, &look, &palette);
            });
            for label in ["No rows in fixture", "The table exists and is empty."] {
                assert!(node(&tree, label, Role::Label).is_some(), "{}", look.name);
            }
            let reload = bounds(&tree, "Reload", Role::Button).unwrap();
            let clear = bounds(&tree, "Clear", Role::Button).unwrap();
            assert!(reload.right() <= clear.left(), "{}", look.name);
        }
    }

    #[test]
    fn an_empty_state_stays_inside_a_small_area() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let area = Rect::from_min_size(pos2(40.0, 60.0), vec2(260.0, 90.0));
            let tree = harness.frame_with(|ui| {
                empty(ui, area, &notice(), Vec::new(), &look, &palette);
            });
            // The title always shows; the sentence only where it fits.
            let title = bounds(&tree, "No rows in fixture", Role::Label);
            assert!(title.is_some(), "{}", look.name);
            let text = bounds(&tree, "The table exists and is empty.", Role::Label);
            for at in title.into_iter().chain(text) {
                assert!(area.contains_rect(at), "{}: {at:?}", look.name);
            }
        }
    }

    #[test]
    fn a_short_area_loses_the_tile_then_the_sentence() {
        let shown = |tile, text| Shown { tile, text };
        let (all, no_tile, title) = (shown(true, true), shown(false, true), shown(false, false));
        // A tile of 50, a sentence of 30, a title and buttons of 60.
        assert_eq!(fit(50.0, 30.0, 60.0, 140.0), all);
        assert_eq!(fit(50.0, 30.0, 60.0, 139.0), no_tile);
        assert_eq!(fit(50.0, 30.0, 60.0, 90.0), no_tile);
        assert_eq!(fit(50.0, 30.0, 60.0, 89.0), title);
        // The title and the buttons stay, however little room there is.
        assert_eq!(fit(50.0, 30.0, 60.0, 0.0), title);
        assert_eq!(fit(50.0, 30.0, 60.0, -36.0), title);
        // The tile never comes back in place of the sentence.
        assert_eq!(fit(20.0, 30.0, 60.0, 85.0), title);
    }

    #[test]
    fn a_button_an_empty_state_has_no_room_for_takes_no_click() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            // Too short for even the title and the button under it.
            let area = Rect::from_min_size(pos2(40.0, 60.0), vec2(260.0, 30.0));
            let mut frame = |events: Vec<egui::Event>| {
                let mut clicked = None;
                let tree = harness.frame_with_events(events, |ui| {
                    let buttons = vec![button("Reload", &look)];
                    clicked = empty(ui, area, &notice(), buttons, &look, &palette);
                });
                (tree, clicked)
            };
            let (tree, _) = frame(Vec::new());
            let pos = bounds(&tree, "Reload", Role::Button).unwrap().center();
            assert!(!area.contains(pos), "{}", look.name);
            let press = |pressed| egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed,
                modifiers: egui::Modifiers::NONE,
            };
            for event in [egui::Event::PointerMoved(pos), press(true), press(false)] {
                assert_eq!(frame(vec![event]).1, None, "{}", look.name);
            }
        }
    }

    #[test]
    fn a_card_says_what_failed_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let failed = Card {
                    tone: Tone::Danger,
                    icon: Icon::Lock,
                    title: "Password rejected for reader",
                    text: "The server refused the login.",
                };
                card(ui, &failed, &look, &palette);
            });
            for label in [
                "Password rejected for reader",
                "The server refused the login.",
            ] {
                assert!(node(&tree, label, Role::Label).is_some(), "{}", look.name);
            }
        }
    }

    #[test]
    fn steps_are_named_with_what_they_act_on_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            let tree = harness.frame_with(|ui| {
                let rect = ui.max_rect();
                let list = [
                    Step {
                        state: StepState::Done,
                        text: "Open tunnel",
                        detail: "",
                    },
                    Step {
                        state: StepState::Running(Some(Duration::from_millis(2400))),
                        text: "Connect to",
                        detail: "db.example.com:5432",
                    },
                    Step {
                        state: StepState::Waiting,
                        text: "Load schema",
                        detail: "",
                    },
                ];
                steps(ui, rect, &list, &look, &palette);
            });
            for label in [
                "Open tunnel",
                "Connect to db.example.com:5432",
                "Load schema",
            ] {
                assert!(node(&tree, label, Role::Label).is_some(), "{}", look.name);
            }
        }
    }

    /// One frame of a wait drawn over a page that takes clicks, given
    /// `events`: the tree, whether the page was clicked, and whether the
    /// wait's button was.
    fn wait_over_a_page(
        harness: &mut Harness,
        look: &Look,
        events: Vec<egui::Event>,
    ) -> (egui::accesskit::TreeUpdate, bool, bool) {
        let palette = harness.app.palette;
        let mut clicked = (false, false);
        let tree = harness.frame_with_events(events, |ui| {
            let rect = ui.max_rect();
            let page = ui.interact(rect, ui.id().with("page"), Sense::click());
            let cancel = button("Cancel", look).label("Cancel query");
            let (text, elapsed) = ("Running query…", Duration::from_millis(4200));
            let cancelled = running(ui, rect, text, elapsed, Some(cancel), look, &palette);
            clicked = (page.clicked(), cancelled);
        });
        (tree, clicked.0, clicked.1)
    }

    /// Clicks at `pos` over [`wait_over_a_page`]: whether the page took
    /// the click, and whether the wait's button did.
    fn click_over_a_page(harness: &mut Harness, look: &Look, pos: egui::Pos2) -> (bool, bool) {
        let button = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut clicked = (false, false);
        let moved = egui::Event::PointerMoved(pos);
        for event in [moved, button(true), button(false)] {
            let (_, page, cancelled) = wait_over_a_page(harness, look, vec![event]);
            clicked = (clicked.0 || page, clicked.1 || cancelled);
        }
        clicked
    }

    #[test]
    fn a_wait_names_what_runs_and_its_button_in_every_look() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let (tree, ..) = wait_over_a_page(&mut harness, &look, Vec::new());
            assert!(
                node(&tree, "Running query…", Role::Label).is_some(),
                "{}",
                look.name
            );
            assert!(
                node(&tree, "Cancel query", Role::Button).is_some(),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_wait_keeps_clicks_on_its_box_from_the_page_under_it() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let (tree, ..) = wait_over_a_page(&mut harness, &look, Vec::new());
            let text = bounds(&tree, "Running query…", Role::Label).unwrap();
            let cancel = bounds(&tree, "Cancel query", Role::Button).unwrap();
            // On the box's text: neither the page nor the button.
            let on_text = click_over_a_page(&mut harness, &look, text.center());
            assert_eq!(on_text, (false, false), "{}", look.name);
            // On the button, which lies in the box and still cancels.
            let on_button = click_over_a_page(&mut harness, &look, cancel.center());
            assert_eq!(on_button, (false, true), "{}", look.name);
            // Away from the box the page is as live as before.
            let away = click_over_a_page(&mut harness, &look, pos2(30.0, 30.0));
            assert_eq!(away, (true, false), "{}", look.name);
        }
    }

    #[test]
    fn a_skeleton_and_a_progress_line_draw_in_no_room() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            harness.frame_with(|ui| {
                let at = ui.max_rect().min;
                skeleton(ui, Rect::from_min_size(at, vec2(0.0, 0.0)), &look, &palette);
                // Rows to draw, and no width to draw them in.
                skeleton(
                    ui,
                    Rect::from_min_size(at, vec2(0.0, 200.0)),
                    &look,
                    &palette,
                );
                progress(ui, egui::Rangef::point(at.x), at.y, &palette);
            });
        }
    }

    #[test]
    fn a_wait_shows_once_it_has_lasted() {
        assert!(!lasted(Duration::from_millis(299)));
        assert!(lasted(DELAY));
    }

    #[test]
    fn seconds_read_to_a_tenth() {
        assert_eq!(seconds(Duration::from_millis(4234)), "4.2 s");
        assert_eq!(seconds(Duration::from_millis(300)), "0.3 s");
    }
}
