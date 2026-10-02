//! Empty, loading and error states: what a view shows in place of its
//! content, drawn the same wherever it appears. A view says what to say
//! (an icon, a title, a sentence, the buttons); this module places it.

use std::borrow::Cow;
use std::time::Duration;

use egui::{Align, Color32, CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, Ui};
use egui::{pos2, vec2};

use crate::env::{Platform, failure_tint, warning_tint};
use crate::theme::{Icon, Look, Palette};
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

/// A button of a state without a key beside it: a bordered one, its text
/// in the text colour. The terminal look mutes only [`key_button`]'s text:
/// muted and alone, it would read as a button that is off.
pub fn button<'a>(text: &'a str, look: &Look) -> ButtonSpec<'a> {
    if look.terminal {
        ButtonSpec::new(text).padding(11.0).gap(8.0)
    } else {
        ButtonSpec::new(text)
    }
}

/// A button of a state that shows its key: `keys` after its text, and in
/// the terminal look a hint (the text muted, the key in the text colour).
pub fn key_button<'a>(text: &'a str, keys: &'a str, look: &Look) -> ButtonSpec<'a> {
    let button = button(text, look).shortcut(keys);
    if look.terminal {
        button.hint().shortcut_role(TextRole::OBody)
    } else {
        button
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

    /// The tint of this tone and the line over it, lent by the module
    /// that owns them: a failure wears the production bar's red and a
    /// warning the staging bar's amber. [`Tone::color`] is too dark to
    /// tint with: it is text's.
    fn tints(self, look: &Look, palette: &Palette) -> (Color32, Color32) {
        let platform = Platform::of(look);
        match self {
            Self::Danger => failure_tint(platform, palette),
            Self::Warning => warning_tint(platform, palette),
        }
    }

    /// The fill of a card or a banner in this tone: its bar's tint.
    pub fn fill(self, look: &Look, palette: &Palette) -> Color32 {
        self.tints(look, palette).0
    }

    /// The line round a card or under a banner in this tone: the line a
    /// bar draws over its tint.
    pub fn line(self, look: &Look, palette: &Palette) -> Color32 {
        self.tints(look, palette).1
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
            Stroke::new(1.0, card.tone.line(look, palette)),
        )
    };
    let strong = TextRole::pick(look, TextRole::UiBodySemibold, TextRole::OGroup);
    // The terminal's title takes the tone: it has no icon to carry it.
    let title = if look.terminal { tone } else { palette.text };
    let shown = Frame::new()
        .fill(card.tone.fill(look, palette))
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

/// The width of a spinner's line.
const RING: f32 = 2.0;

/// A wait's spinner in `rect`: a ring, and over it a quarter of the ring
/// in `color` that goes round once a second. It asks for the next frame
/// while it shows, as `egui::Spinner` does: a wait's time ticks on those.
pub fn spinner(ui: &Ui, rect: Rect, color: Color32, palette: &Palette) {
    if !ui.is_rect_visible(rect) {
        return;
    }
    ui.request_repaint();
    // To the middle of the line, whose outer edge touches `rect`.
    let radius = (rect.width().min(rect.height()) - RING) / 2.0;
    if radius <= 0.0 {
        return;
    }
    let center = rect.center();
    ui.painter()
        .circle_stroke(center, radius, Stroke::new(RING, palette.border));
    let start = ui.input(|input| input.time).fract() * std::f64::consts::TAU;
    // A line through enough points to read as round.
    let pieces = (radius.round() as u32).clamp(8, 32);
    let quarter = (0..=pieces)
        .map(|piece| {
            let turned = std::f64::consts::FRAC_PI_2 * f64::from(piece) / f64::from(pieces);
            let (sin, cos) = (start + turned).sin_cos();
            center + radius * vec2(cos as f32, sin as f32)
        })
        .collect();
    ui.painter()
        .add(egui::Shape::line(quarter, Stroke::new(RING, color)));
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

/// `time` with every digit a 0: a stand-in that keeps its width while the
/// digits change.
fn steady(time: &str) -> String {
    time.chars()
        .map(|digit| if digit.is_ascii_digit() { '0' } else { digit })
        .collect()
}

/// The side of a step's mark.
const MARK: f32 = 14.0;

/// Between a step's mark and its line, and its line and its time.
const STEP_GAP: f32 = 10.0;

/// The roles of a step's text: of one that is done or still to come, and
/// of the one under way.
fn step_roles(look: &Look) -> [TextRole; 2] {
    [
        widgets::body(look),
        TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody),
    ]
}

/// The role of a step's time.
fn time_role(look: &Look) -> TextRole {
    widgets::secondary(look)
}

/// A step's line: what it does in `role`, then `detail`, what it does it
/// to, in the code face. [`steps`] paints it and [`steps_width`] measures
/// it. `colors` are those of the two pieces.
fn step_line(
    step: &Step<'_>,
    detail: &str,
    role: TextRole,
    look: &Look,
    colors: [Color32; 2],
) -> Text {
    let line = Text::one(look, role, step.text, colors[0]);
    if detail.is_empty() {
        return line;
    }
    line.space(role, " ")
        .add(widgets::code(look), detail, colors[1])
}

/// The room a row keeps at its end for a time: that of the longest time
/// running, and of ten seconds at least, so a time that comes or grows
/// moves nothing.
fn time_room(ui: &Ui, steps: &[Step<'_>], look: &Look) -> f32 {
    let running = steps.iter().filter_map(|step| match step.state {
        StepState::Running(elapsed) => elapsed,
        _ => None,
    });
    running
        .chain([Duration::from_secs(10)])
        .map(|elapsed| {
            let time = steady(&seconds(elapsed));
            widgets::measure(
                ui,
                Text::one(look, time_role(look), &time, Color32::PLACEHOLDER),
            )
        })
        .fold(0.0, f32::max)
}

/// What sums of widths may be off by: a row as wide as [`steps_width`]
/// asks for is not too narrow by a rounding error.
const ROUNDING: f32 = 0.01;

/// What a row `width` wide has for a step's line: the rest after its mark
/// and the room of a time, `time`.
fn line_room(width: f32, time: f32) -> f32 {
    width - MARK - STEP_GAP - STEP_GAP - time
}

/// The width [`steps`] needs to show `steps` in full: the widest one's
/// mark and line and, after a gap, the room for a time. Each line counts
/// in both of its roles, so the width is the same whichever step is under
/// way.
pub fn steps_width(ui: &Ui, steps: &[Step<'_>], look: &Look) -> f32 {
    let colors = [Color32::PLACEHOLDER; 2];
    let line = steps
        .iter()
        .flat_map(|step| {
            step_roles(look)
                .map(|role| widgets::measure(ui, step_line(step, step.detail, role, look, colors)))
        })
        .fold(0.0, f32::max);
    MARK + STEP_GAP + line + STEP_GAP + time_room(ui, steps, look)
}

/// `step`'s detail, cut with "…" where its line in `role` is wider than
/// `room`: a long host runs neither under the time nor out of the row.
fn fit_detail<'a>(
    ui: &Ui,
    step: &Step<'a>,
    role: TextRole,
    room: f32,
    look: &Look,
) -> Cow<'a, str> {
    if step.detail.is_empty() {
        return step.detail.into();
    }
    crate::ui::grid::ellipsize(step.detail, room + ROUNDING, false, |detail| {
        let colors = [Color32::PLACEHOLDER; 2];
        widgets::measure(ui, step_line(step, detail, role, look, colors))
    })
}

/// `steps` from the top of `rect`, one per line: a tick for a step that is
/// done, a spinner and the time for the one under way, a ring for those
/// still to come. A row too narrow for its step cuts the step's detail.
pub fn steps(ui: &mut Ui, rect: Rect, steps: &[Step<'_>], look: &Look, palette: &Palette) {
    let pitch = step_pitch(look);
    let [body, strong] = step_roles(look);
    let room = line_room(rect.width(), time_room(ui, steps, look));
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
            StepState::Running(_) => spinner(ui, mark, palette.accent, palette),
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
        let shown = fit_detail(ui, step, role, room, look);
        let line = step_line(step, &shown, role, look, [color, detail]);
        widgets::paint_text(ui, row.left() + MARK + STEP_GAP, center, line);
        if let StepState::Running(Some(elapsed)) = step.state {
            let time = Text::one(look, time_role(look), &seconds(elapsed), palette.dim);
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
    let side = if look.terminal { 12.0 } else { 14.0 };
    let height = if look.terminal { 24.0 } else { 28.0 };
    let gap = 12.0;
    let role = widgets::body(look);
    let time = seconds(elapsed);
    // The time's room is that of a stand-in with every digit a 0: the box
    // keeps its width and its place while the digits change.
    let widest = steady(&time);
    let line = Text::one(look, role, text, palette.text)
        .space(role, " ")
        .add(role, &widest, palette.dim);
    let width = widgets::measure(ui, line);
    let reserved = widgets::measure(ui, Text::one(look, role, &widest, palette.dim));
    let button_width = cancel.as_ref().map(|button| button.width(ui, look));
    let content = side + gap + width + button_width.map_or(0.0, |width| gap + width);
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
    // The box takes the clicks and the drags on its text and its padding,
    // and drops them: it lies over a live page, which would get them
    // otherwise. Registered before the button, so the button, which comes
    // after it, still wins inside it. It senses them without being
    // focusable, so the keyboard skips it.
    ui.interact(card, ui.id().with("running"), Sense::CLICK | Sense::DRAG);
    let (mut x, center) = (card.left() + 14.0, card.center().y);
    // The spinner asks for the frames that keep the time going.
    let ring = Rect::from_center_size(pos2(x + side / 2.0, center), vec2(side, side));
    spinner(ui, ring, palette.accent, palette);
    x += side + gap;
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

    /// A step under way on a host too long for a narrow row.
    fn long_step() -> Step<'static> {
        Step {
            state: StepState::Running(Some(Duration::from_millis(2400))),
            text: "Connect to",
            detail: "an-uncommonly-long-host-name.internal.example.com:5432 via bastion",
        }
    }

    #[test]
    fn a_step_too_long_for_its_row_is_cut_before_the_room_of_its_time() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.frame_with(|ui| {
                let list = [long_step()];
                let step = &list[0];
                let [_, role] = step_roles(&look);
                let time = time_room(ui, &list, &look);
                // A row as wide as the steps ask for shows the step in full.
                let asked = line_room(steps_width(ui, &list, &look), time);
                let shown = fit_detail(ui, step, role, asked, &look);
                assert_eq!(shown, step.detail, "{}", look.name);
                // A narrower one cuts the detail, and the line ends where
                // the room of the time starts.
                let narrow = line_room(220.0, time);
                let shown = fit_detail(ui, step, role, narrow, &look);
                assert!(shown.ends_with('…'), "{}: {shown}", look.name);
                assert!(step.detail.starts_with(shown.trim_end_matches('…')));
                let colors = [Color32::PLACEHOLDER; 2];
                let width = widgets::measure(ui, step_line(step, &shown, role, &look, colors));
                assert!(
                    width <= narrow + ROUNDING,
                    "{}: {width} in {narrow}",
                    look.name
                );
            });
        }
    }

    #[test]
    fn steps_ask_for_the_same_width_whichever_is_under_way() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            harness.frame_with(|ui| {
                let width = |state| {
                    let step = Step {
                        state,
                        ..long_step()
                    };
                    steps_width(ui, &[step], &look)
                };
                let waiting = width(StepState::Waiting);
                for state in [
                    // Before the connect is sent, as its time starts, and
                    // once it is answered.
                    StepState::Running(None),
                    StepState::Running(Some(Duration::from_millis(300))),
                    StepState::Running(Some(Duration::from_millis(9900))),
                    StepState::Done,
                ] {
                    assert_eq!(width(state), waiting, "{}", look.name);
                }
            });
        }
    }

    #[test]
    fn a_step_with_nothing_to_cut_is_left_as_it_is() {
        let mut harness = Harness::new();
        harness.frame_with(|ui| {
            let look = Look::standard();
            let step = Step {
                state: StepState::Waiting,
                text: "Load schema",
                detail: "",
            };
            let [role, _] = step_roles(&look);
            assert_eq!(fit_detail(ui, &step, role, 0.0, &look), "");
        });
    }

    /// A wait over a page that senses `sense`: the page's response, and
    /// whether the wait's button was clicked.
    fn wait_over(
        ui: &mut Ui,
        look: &Look,
        palette: &Palette,
        sense: Sense,
    ) -> (egui::Response, bool) {
        let rect = ui.max_rect();
        let page = ui.interact(rect, ui.id().with("page"), sense);
        let cancel = button("Cancel", look).label("Cancel query");
        let (text, elapsed) = ("Running query…", Duration::from_millis(4200));
        let cancelled = running(ui, rect, text, elapsed, Some(cancel), look, palette);
        (page, cancelled)
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
            let (page, cancelled) = wait_over(ui, look, &palette, Sense::click());
            clicked = (page.clicked(), cancelled);
        });
        (tree, clicked.0, clicked.1)
    }

    /// Drags down from `from` over a wait on a page that takes drags:
    /// whether the page was dragged.
    fn drag_over_a_page(harness: &mut Harness, look: &Look, from: egui::Pos2) -> bool {
        let palette = harness.app.palette;
        let to = from + vec2(0.0, 40.0);
        let button = |pos, pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        let mut dragged = false;
        for event in [
            egui::Event::PointerMoved(from),
            button(from, true),
            egui::Event::PointerMoved(to),
            button(to, false),
        ] {
            harness.frame_with_events(vec![event], |ui| {
                let (page, _) = wait_over(ui, look, &palette, Sense::drag());
                dragged |= page.dragged();
            });
        }
        dragged
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
    fn a_wait_keeps_clicks_and_drags_on_its_box_from_the_page_under_it() {
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
            // A drag is the box's or the page's by where it starts.
            let on_text = drag_over_a_page(&mut harness, &look, text.center());
            assert!(!on_text, "{}", look.name);
            let away = drag_over_a_page(&mut harness, &look, pos2(30.0, 30.0));
            assert!(away, "{}", look.name);
        }
    }

    /// Asserts nothing: drawing in no room must not panic, and that is all
    /// this guards.
    #[test]
    fn a_skeleton_and_a_progress_line_do_not_panic_in_no_room() {
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

    /// Asserts nothing: a spinner with no room to turn in must not panic,
    /// and that is all this guards.
    #[test]
    fn a_spinner_does_not_panic_in_no_room() {
        for look in Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            let palette = harness.app.palette;
            harness.frame_with(|ui| {
                let at = ui.max_rect().min;
                let none = Rect::from_min_size(at, vec2(0.0, 0.0));
                spinner(ui, none, palette.accent, &palette);
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
