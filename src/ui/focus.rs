//! Keyboard focus, shown: a ring round whatever has the keyboard, once the
//! keyboard (not the pointer) put it there. One painter draws it for every
//! widget, egui's own included, so no Tab stop is ever without one; a
//! widget that wants another form than the default says so with [`hint`].

use egui::{Color32, CornerRadius, EventFilter, Id, Key, Modifiers, Rect, Response, Sense, Stroke};
use egui::{StrokeKind, Ui};

use crate::theme::{Look, Palette};

/// How far off a control its ring starts, and how wide a ring is.
const GAP: f32 = 2.0;
const WIDTH: f32 = 2.0;
/// A field's halo: this wide, the accent at a quarter.
const HALO: f32 = 3.0;
/// A field's border.
const BORDER: f32 = 1.0;

/// How the widget that has the keyboard shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ring {
    /// Off the control, with a gap that shows what the control sits on:
    /// buttons, chips, links. The default.
    Outer { radius: u8 },
    /// Inside the item's edge, where a ring outside it would be cut by the
    /// list it scrolls in: rows, tabs, cells.
    Inset { radius: u8 },
    /// On the item's edge, with no gap: a segment in its track.
    Edge { radius: u8 },
    /// A text field: its border in the accent and a soft halo round it.
    Field { radius: u8 },
    /// The widget shows it by itself (a caret in text that has no box, the
    /// terminal's reversed button).
    Own,
}

/// What the focused widget said of its ring this frame.
#[derive(Clone, Copy)]
struct Hint {
    id: Id,
    rect: Rect,
    ring: Ring,
    clip: Rect,
}

fn visible_id() -> Id {
    Id::new("focus-visible")
}

fn hint_id() -> Id {
    Id::new("focus-hint")
}

fn pane_id() -> Id {
    Id::new("focus-pane")
}

fn regions_id() -> Id {
    Id::new("focus-regions")
}

/// The parts of the window F6 steps between, in the order it takes them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Region {
    /// The connection bar: Connections, then a chip for each connection.
    Header,
    /// The sidebar's filter.
    Search,
    /// The object tree.
    Tree,
    /// The open tables and editors.
    Tabs,
    /// A table's header: the Data and Structure switch.
    Toolbar,
    /// A SQL editor's script.
    Editor,
    /// The rows.
    Grid,
    /// The row panel.
    Panel,
}

impl Region {
    /// The panes the terminal look steps between with ctrl+h and ctrl+l.
    pub const PANES: [Region; 3] = [Region::Tree, Region::Grid, Region::Panel];
}

/// Where the regions are, and which one F6 asked for.
#[derive(Clone, Default)]
struct Regions {
    /// The regions drawn so far this frame.
    drawn: Vec<(Region, Rect)>,
    /// The ones the frame before drew: what a key pressed now steps among.
    before: Vec<(Region, Rect)>,
    /// The region asked for, and in which frame.
    wanted: Option<(Region, u64)>,
}

/// Says where `region` is this frame, so F6 can step to it and from it.
pub fn region(ui: &Ui, region: Region, rect: Rect) {
    ui.data_mut(|data| {
        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
        regions.drawn.push((region, rect));
    });
}

/// Gives `response`'s widget the keyboard when F6 asked for `region`: call
/// it for the control a region starts with.
pub fn claim(ui: &Ui, region: Region, response: &Response) {
    let asked = ui.data_mut(|data| {
        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
        let asked = matches!(regions.wanted, Some((wanted, _)) if wanted == region);
        if asked {
            regions.wanted = None;
        }
        asked
    });
    if asked {
        response.request_focus();
    }
}

/// F6: asks for the region after (or, not `forward`, before) the one the
/// keyboard is in, among the ones on screen and in `among` if given. With
/// the keyboard nowhere it counts from `from`.
pub fn step(ctx: &egui::Context, forward: bool, among: Option<&[Region]>, from: Option<Region>) {
    let at = ctx
        .memory(|memory| memory.focused())
        .and_then(|id| ctx.read_response(id))
        .map(|response| response.rect.center());
    let now = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| {
        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
        let mut stops: Vec<Region> = regions
            .before
            .iter()
            .map(|(region, _)| *region)
            .filter(|region| among.is_none_or(|among| among.contains(region)))
            .collect();
        stops.sort();
        stops.dedup();
        // The region the keyboard is in: the smallest one round it.
        let current = at
            .and_then(|at| {
                regions
                    .before
                    .iter()
                    .filter(|(_, rect)| rect.contains(at))
                    .min_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
                    .map(|(region, _)| *region)
            })
            .or(from);
        let next = match (current, forward) {
            (Some(current), true) => stops.iter().find(|stop| **stop > current),
            (Some(current), false) => stops.iter().rev().find(|stop| **stop < current),
            (None, _) => None,
        };
        // Past the last one it starts over.
        let next = next.or(if forward { stops.first() } else { stops.last() });
        regions.wanted = next.map(|region| (*region, now));
    });
    // The region takes the keyboard when it is next drawn.
    ctx.request_repaint();
}

/// Makes `response`'s widget a pane: one Tab stop for a list or a grid
/// whose keys the app's shortcuts handle. With the keyboard on it Enter,
/// Space and the arrows act on what it shows: egui neither presses it like
/// a button nor moves focus off it with the arrows.
pub fn pane(ui: &Ui, response: &Response) {
    if !response.has_focus() {
        return;
    }
    let arrows = EventFilter {
        horizontal_arrows: true,
        vertical_arrows: true,
        ..Default::default()
    };
    ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, arrows));
    ui.data_mut(|data| data.insert_temp(pane_id(), response.id));
}

/// Whether the keyboard is on a control of its own (a button, a field, a
/// tab): it takes Enter and Space itself, and the arrows move focus from
/// it. On a pane, or nowhere, the shortcuts have those keys.
pub fn on_control(ctx: &egui::Context) -> bool {
    ctx.memory(|memory| memory.focused())
        .is_some_and(|id| ctx.data(|data| data.get_temp::<Id>(pane_id())) != Some(id))
}

/// The terminal look's mark of the pane the keys go to: a 2 pt accent line
/// round it, inside its edge. The other looks mark the item, not the pane.
pub fn pane_border(ui: &Ui, rect: Rect, look: &Look, palette: &Palette) {
    if look.terminal {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::ZERO,
            Stroke::new(WIDTH, palette.accent),
            StrokeKind::Inside,
        );
    }
}

/// One of the `count` choices of a segmented control, the one at `index`:
/// the chosen one is the control's only Tab stop, and with the keyboard on
/// it ← and → choose its neighbours. Returns its response and the choice
/// the arrows made, if any; the keyboard follows that choice once the
/// caller has made it the chosen one.
pub fn segment(
    ui: &Ui,
    rect: Rect,
    id: Id,
    group: Id,
    (index, count): (usize, usize),
    chosen: bool,
) -> (Response, Option<usize>) {
    let sense = if chosen { Sense::click() } else { Sense::CLICK };
    let response = ui.interact(rect, id, sense);
    let now = ui.ctx().cumulative_frame_nr();
    // The choice the arrows made, and when: taken up within two frames or
    // not at all (the caller may have refused it).
    let moved: Option<(usize, u64)> = ui.data(|data| data.get_temp(group));
    if let Some((target, when)) = moved
        && chosen
        && target == index
    {
        if now <= when + 2 {
            response.request_focus();
        }
        ui.data_mut(|data| data.remove::<(usize, u64)>(group));
    }
    let mut picked = None;
    if response.has_focus() {
        let arrows = EventFilter {
            horizontal_arrows: true,
            ..Default::default()
        };
        ui.memory_mut(|memory| memory.set_focus_lock_filter(response.id, arrows));
        let (right, left) = ui.input_mut(|input| {
            (
                input.consume_key(Modifiers::NONE, Key::ArrowRight),
                input.consume_key(Modifiers::NONE, Key::ArrowLeft),
            )
        });
        let next = match (right, left) {
            (true, false) => (index + 1).min(count.saturating_sub(1)),
            (false, true) => index.saturating_sub(1),
            _ => index,
        };
        if next != index {
            picked = Some(next);
            ui.data_mut(|data| data.insert_temp(group, (next, now)));
        }
    }
    (response, picked)
}

/// Notes what this frame's input says of how the user works: a key makes
/// focus visible, a pointer press hides it. A screen reader that moves
/// focus counts as a key. Call before anything reads the frame's keys.
pub fn begin_frame(ctx: &egui::Context) {
    let keyboard = ctx.input(|input| {
        input.events.iter().rev().find_map(|event| match event {
            egui::Event::Key { pressed: true, .. } => Some(true),
            egui::Event::AccessKitActionRequest(request)
                if request.action == egui::accesskit::Action::Focus =>
            {
                Some(true)
            }
            egui::Event::PointerButton { pressed: true, .. } => Some(false),
            _ => None,
        })
    });
    let now = ctx.cumulative_frame_nr();
    ctx.data_mut(|data| {
        if let Some(keyboard) = keyboard {
            data.insert_temp(visible_id(), keyboard);
        }
        let regions: &mut Regions = data.get_temp_mut_or_default(regions_id());
        regions.before = std::mem::take(&mut regions.drawn);
        // A region asked for and not drawn since is not there to take it.
        if regions.wanted.is_some_and(|(_, when)| now > when + 1) {
            regions.wanted = None;
        }
    });
}

/// Whether focus is shown: the keyboard was used last, not the pointer.
pub fn visible(ctx: &egui::Context) -> bool {
    ctx.data(|data| data.get_temp(visible_id()))
        .unwrap_or(false)
}

/// Whether `response`'s widget has the keyboard and should show it: for a
/// widget that draws its own focus ([`Ring::Own`]).
pub fn shown(response: &Response) -> bool {
    response.has_focus() && visible(&response.ctx)
}

/// Says how `response`'s widget shows focus: `ring` round `rect`. Without a
/// hint a widget gets [`Ring::Outer`] round its own rectangle (a text field
/// [`Ring::Field`]), at the look's radius.
pub fn hint(ui: &Ui, response: &Response, rect: Rect, ring: Ring) {
    if !response.has_focus() {
        return;
    }
    let hint = Hint {
        id: response.id,
        rect,
        ring,
        clip: ui.clip_rect(),
    };
    ui.data_mut(|data| data.insert_temp(hint_id(), hint));
}

/// Draws the ring of whatever has the keyboard. Call once the frame's
/// widgets are drawn.
pub fn paint(ctx: &egui::Context, look: &Look, palette: &Palette) {
    let Some(id) = ctx.memory(|memory| memory.focused()) else {
        return;
    };
    if !visible(ctx) {
        return;
    }
    let Some(response) = ctx.read_response(id) else {
        return;
    };
    let hint = ctx
        .data(|data| data.get_temp::<Hint>(hint_id()))
        .filter(|hint| hint.id == id);
    let (rect, ring, clip) = match hint {
        Some(hint) => (hint.rect, hint.ring, hint.clip),
        None => {
            let ring = if ctx.text_edit_focused() {
                Ring::Field {
                    radius: look.radius,
                }
            } else {
                Ring::Outer {
                    radius: look.radius,
                }
            };
            (response.rect, ring, Rect::EVERYTHING)
        }
    };
    let painter = ctx.layer_painter(response.layer_id);
    let accent = Stroke::new(WIDTH, palette.accent);
    // Concentric with the control: a ring further out is rounder. A
    // square control keeps a square ring.
    let round = |radius: u8, out: f32| {
        if radius == 0 {
            CornerRadius::ZERO
        } else {
            CornerRadius::same(radius.saturating_add(out as u8))
        }
    };
    match ring {
        Ring::Outer { radius } => {
            // The ring stands outside the control, so it may pass the edge
            // of what clips the control by its own reach.
            painter
                .with_clip_rect(clip.expand(GAP + WIDTH))
                .rect_stroke(
                    rect.expand(GAP),
                    round(radius, GAP),
                    accent,
                    StrokeKind::Outside,
                );
        }
        Ring::Inset { radius } => {
            painter.with_clip_rect(clip).rect_stroke(
                rect,
                CornerRadius::same(radius),
                accent,
                StrokeKind::Inside,
            );
        }
        Ring::Edge { radius } => {
            painter.with_clip_rect(clip.expand(WIDTH)).rect_stroke(
                rect,
                CornerRadius::same(radius),
                accent,
                StrokeKind::Outside,
            );
        }
        Ring::Field { radius } => {
            let painter = painter.with_clip_rect(clip.expand(HALO));
            // The terminal's fields are square and take the border alone.
            if !look.terminal {
                painter.rect_stroke(
                    rect,
                    round(radius, 0.0),
                    Stroke::new(HALO, halo(palette)),
                    StrokeKind::Outside,
                );
            }
            painter.rect_stroke(
                rect,
                CornerRadius::same(radius),
                Stroke::new(BORDER, palette.accent),
                StrokeKind::Inside,
            );
        }
        Ring::Own => {}
    }
}

/// A field's halo: the accent at a quarter.
pub fn halo(palette: &Palette) -> Color32 {
    palette.accent.gamma_multiply(0.25)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Key, Modifiers, vec2};

    /// One frame of two buttons and a text field, with `events`; the
    /// second button asks for an inset ring. Returns what was painted.
    fn frame(
        ctx: &egui::Context,
        look: &Look,
        palette: &Palette,
        events: Vec<egui::Event>,
    ) -> Vec<egui::epaint::ClippedShape> {
        let input = egui::RawInput {
            events,
            ..Default::default()
        };
        let mut output = ctx.run_ui(input, |ui| {
            begin_frame(ui.ctx());
            egui::CentralPanel::default().show(ui, |ui| {
                let _ = ui.button("First");
                let (rect, second) = ui.allocate_exact_size(vec2(80.0, 24.0), egui::Sense::click());
                hint(ui, &second, rect, Ring::Inset { radius: 0 });
                let mut text = String::new();
                ui.text_edit_singleline(&mut text);
            });
            paint(ui.ctx(), look, palette);
        });
        output.textures_delta.clear();
        output.shapes
    }

    /// The kinds of the accent outlines in `shapes`, by their width.
    fn rings(shapes: &[egui::epaint::ClippedShape], palette: &Palette) -> Vec<(StrokeKind, f32)> {
        shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(rect)
                    if rect.fill == Color32::TRANSPARENT
                        && (rect.stroke.color == palette.accent
                            || rect.stroke.color == halo(palette)) =>
                {
                    Some((rect.stroke_kind, rect.stroke.width))
                }
                _ => None,
            })
            .collect()
    }

    fn context(look: &Look, palette: &Palette) -> egui::Context {
        let ctx = egui::Context::default();
        crate::theme::install(&ctx, false, look);
        crate::theme::apply(&ctx, palette, look);
        ctx
    }

    fn tab() -> Vec<egui::Event> {
        vec![
            crate::testing::key(Key::Tab, Modifiers::NONE),
            crate::testing::release(Key::Tab, Modifiers::NONE),
        ]
    }

    #[test]
    fn the_keyboard_shows_focus_and_the_pointer_hides_it() {
        for look in Look::ALL {
            for palette in [Palette::light(), Palette::dark()] {
                let ctx = context(&look, &palette);
                let draw = |events| rings(&frame(&ctx, &look, &palette, events), &palette);
                assert_eq!(draw(Vec::new()), [], "{}: nothing focused", look.name);
                // Tab lands on the first button: the default, an outer ring.
                draw(tab());
                assert_eq!(
                    draw(Vec::new()),
                    [(StrokeKind::Outside, WIDTH)],
                    "{}",
                    look.name
                );
                // The second asked for an inset one.
                draw(tab());
                assert_eq!(
                    draw(Vec::new()),
                    [(StrokeKind::Inside, WIDTH)],
                    "{}",
                    look.name
                );
                // A press of the pointer elsewhere hides the ring.
                let press = egui::Event::PointerButton {
                    pos: egui::pos2(600.0, 500.0),
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: Modifiers::NONE,
                };
                draw(vec![
                    egui::Event::PointerMoved(egui::pos2(600.0, 500.0)),
                    press,
                ]);
                assert_eq!(draw(Vec::new()), [], "{}: the pointer hides it", look.name);
            }
        }
    }

    #[test]
    fn a_text_field_takes_the_accent_border_and_a_halo() {
        for look in Look::ALL {
            let palette = Palette::light();
            let ctx = context(&look, &palette);
            let draw = |events| rings(&frame(&ctx, &look, &palette, events), &palette);
            for _ in 0..3 {
                draw(tab());
            }
            let rings = draw(Vec::new());
            assert!(
                rings.contains(&(StrokeKind::Inside, BORDER)),
                "{}: {rings:?}",
                look.name
            );
            assert_eq!(
                rings.contains(&(StrokeKind::Outside, HALO)),
                !look.terminal,
                "{}: {rings:?}",
                look.name
            );
        }
    }

    #[test]
    fn focus_the_pointer_gave_is_not_shown() {
        let (look, palette) = (Look::macos(), Palette::light());
        let ctx = context(&look, &palette);
        let draw = |ctx: &egui::Context| {
            let mut id = Id::NULL;
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                begin_frame(ui.ctx());
                egui::CentralPanel::default().show(ui, |ui| {
                    id = ui.button("First").id;
                });
                paint(ui.ctx(), &look, &palette);
            });
            output.textures_delta.clear();
            (id, rings(&output.shapes, &palette))
        };
        let (id, _) = draw(&ctx);
        // Focus with no key behind it: code asked for it after a click.
        ctx.memory_mut(|memory| memory.request_focus(id));
        draw(&ctx);
        assert_eq!(draw(&ctx).1, []);
        assert!(!visible(&ctx));
    }
}
