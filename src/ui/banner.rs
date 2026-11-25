//! A banner with actions, as the Components boards draw it for Files: a
//! sentence of what went wrong, and under it the ways to mend it.
//!
//! macOS: a box in the tone's tint with a line round it and corners of 8,
//! the tone's triangle at its left, the lead in bold before the sentence,
//! and under them the actions as buttons 28 high, 8 apart: bordered, the
//! last one quiet (in the accent colour, no border) where it is marked so.
//!
//! The terminal look: a strip with a rule of 2 down its left in the tone's
//! colour, its fill the tone's at a tenth over the panel. The lead in
//! the tone's colour, `! returns.csv`, then the sentence; under them each
//! action as its key in the accent colour and its words, three spaces
//! apart.
//!
//! Its words are an alert for a screen reader, read out as they appear;
//! each action is a button named by its words.

use egui::{CornerRadius, Rect, Response, Sense, Stroke, StrokeKind, Ui, pos2, vec2};

use crate::keymap::Written;
use crate::style::Icon;
use crate::theme::{self, Look, Palette};
use crate::typography::{Laid, Text, TextRole};
use crate::ui::states::Tone;
use crate::ui::widgets::{self, ButtonSpec};

/// The room inside the macOS box, and the terminal strip's.
const PAD: egui::Vec2 = vec2(12.0, 10.0);
const TERMINAL_PAD: egui::Vec2 = vec2(12.0, 8.0);
/// The macOS icon, and the room after it.
const ICON: f32 = 12.0;
const ICON_GAP: f32 = 10.0;
/// A macOS button's height and the room at its sides; the gap between two
/// and above them.
const BUTTON: f32 = 28.0;
const BUTTON_PAD: f32 = 10.0;
const GAP: f32 = 8.0;
/// The most rows the sentence takes.
const ROWS: usize = 4;
/// The terminal look's room between two actions: three spaces.
const KEYS_GAP: &str = "   ";

/// What a banner says, and what it offers.
pub struct Banner<'a> {
    pub tone: Tone,
    /// What went wrong, in bold on macOS and in the tone's colour in the
    /// terminal look: "returns.csv couldn't be read." or "! returns.csv".
    pub lead: &'a str,
    /// The sentence after it.
    pub text: &'a str,
    pub actions: Vec<Action<'a>>,
    /// Tells the banner's buttons from others in its `Ui`.
    pub salt: &'a str,
}

/// One way to mend what the banner says.
pub struct Action<'a> {
    /// The words, which name its button too.
    pub label: &'a str,
    /// Its key, shown before its words in the terminal look.
    pub key: Option<Written>,
    /// macOS draws it quiet: the accent colour, no border.
    pub quiet: bool,
}

impl Banner<'_> {
    /// The words a screen reader hears: the lead, then the sentence.
    fn said(&self) -> String {
        format!("{} {}", self.lead, self.text)
    }

    fn sentence(&self, ctx: &egui::Context, wrap: f32, look: &Look, palette: &Palette) -> Laid {
        let color = self.tone.color(palette);
        let (lead_role, role, lead_color, text_color) = match look.terminal {
            true => (TextRole::OBody, TextRole::OBody, color, palette.text),
            false => (
                TextRole::UiBodySemibold,
                TextRole::UiBody,
                palette.text,
                palette.text,
            ),
        };
        let mut text = Text::new(look)
            .add(lead_role, self.lead, lead_color)
            .space(role, " ")
            .add(role, self.text, text_color)
            .wrap(wrap);
        // A sentence past its rows ends in an ellipsis: a value of any
        // length leaves the actions, and what is under the banner, in
        // reach. A screen reader hears it all.
        text.job_mut().wrap.max_rows = ROWS;
        text.layout(ctx)
    }

    /// The terminal look's line of keys and words.
    fn keys_line(&self, ctx: &egui::Context, look: &Look, palette: &Palette) -> Vec<(Laid, Laid)> {
        self.actions
            .iter()
            .map(|action| {
                let key = action.key.as_ref().map_or("", |key| key.as_str());
                (
                    Text::one(look, TextRole::OBody, key, palette.accent).layout(ctx),
                    Text::one(look, TextRole::OBody, action.label, palette.text).layout(ctx),
                )
            })
            .collect()
    }

    /// The width a macOS button takes, no wider than the row `width`.
    fn button_width(ctx: &egui::Context, label: &str, width: f32, look: &Look) -> f32 {
        let laid =
            Text::one(look, widgets::body(look), label, egui::Color32::PLACEHOLDER).layout(ctx);
        (laid.width() + 2.0 * BUTTON_PAD).ceil().min(width)
    }

    /// The width of an action in the terminal look: its key, a space, its
    /// words. No wider than the row `width`.
    fn key_width(ctx: &egui::Context, (key, words): &(Laid, Laid), width: f32, look: &Look) -> f32 {
        (key.width() + space(ctx, look, " ") + words.width()).min(width)
    }

    /// Where each action goes, from the left of a row `width` wide, `wide`
    /// each and `gap` apart: one that would pass the row's end starts the
    /// next, `step` below.
    fn flow(widths: &[f32], width: f32, gap: f32, step: f32) -> Vec<egui::Pos2> {
        let (mut x, mut y) = (0.0, 0.0);
        let mut places = Vec::with_capacity(widths.len());
        for wide in widths {
            if x > 0.0 && x + wide > width {
                x = 0.0;
                y += step;
            }
            places.push(pos2(x, y));
            x += wide + gap;
        }
        places
    }

    /// Where each macOS button goes, and its width.
    fn button_places(
        &self,
        ctx: &egui::Context,
        width: f32,
        look: &Look,
    ) -> Vec<(egui::Pos2, f32)> {
        let widths: Vec<f32> = self
            .actions
            .iter()
            .map(|action| Self::button_width(ctx, action.label, width, look))
            .collect();
        Self::flow(&widths, width, GAP, BUTTON + GAP)
            .into_iter()
            .zip(widths)
            .collect()
    }

    /// Where each of the terminal look's actions goes, by rows of `line`.
    fn key_places(
        &self,
        ctx: &egui::Context,
        width: f32,
        look: &Look,
        palette: &Palette,
    ) -> Vec<(egui::Pos2, (Laid, Laid))> {
        let line = TextRole::OBody.row_height(ctx, look.faces);
        let laid = self.keys_line(ctx, look, palette);
        let widths: Vec<f32> = laid
            .iter()
            .map(|pair| Self::key_width(ctx, pair, width, look))
            .collect();
        Self::flow(&widths, width, space(ctx, look, KEYS_GAP), line)
            .into_iter()
            .zip(laid)
            .collect()
    }

    /// Its height `width` wide.
    pub fn height(&self, ctx: &egui::Context, width: f32, look: &Look, palette: &Palette) -> f32 {
        if look.terminal {
            let inner = width - 2.0 - 2.0 * TERMINAL_PAD.x;
            let line = TextRole::OBody.row_height(ctx, look.faces);
            let keys = match self.key_places(ctx, inner, look, palette).last() {
                Some((last, _)) => last.y + line,
                None => 0.0,
            };
            return self.sentence(ctx, inner, look, palette).height() + keys + 2.0 * TERMINAL_PAD.y;
        }
        let inner = width - 2.0 * PAD.x - ICON - ICON_GAP;
        let text = self.sentence(ctx, inner, look, palette).height();
        let buttons = match self.button_places(ctx, inner, look).last() {
            Some((last, _)) => GAP + last.y + BUTTON,
            None => 0.0,
        };
        text + buttons + 2.0 * PAD.y
    }

    /// Draws it across the width `ui` has left. Returns the place in its
    /// actions of the one pressed.
    pub fn show(&self, ui: &mut Ui, look: &Look, palette: &Palette) -> Option<usize> {
        let width = ui.available_width();
        let height = self.height(ui.ctx(), width, look, palette);
        let (rect, _) = ui.allocate_exact_size(vec2(width, height), Sense::hover());
        let id = ui.id().with(("banner", self.salt));
        // An alert: a screen reader reads it out when it appears.
        let alert = ui.interact(rect, id, Sense::hover());
        let said = self.said();
        // Its role and words set on its node: a widget's would be a label's.
        ui.ctx().accesskit_node_builder(alert.id, |node| {
            node.set_role(egui::accesskit::Role::Alert);
            node.set_label(said);
            node.set_live(egui::accesskit::Live::Assertive);
            node.set_bounds(egui::accesskit::Rect {
                x0: rect.min.x.into(),
                y0: rect.min.y.into(),
                x1: rect.max.x.into(),
                y1: rect.max.y.into(),
            });
        });
        let color = self.tone.color(palette);
        let mut pressed = None;
        if look.terminal {
            let fill = theme::mix(palette.panel, color, 0.1);
            ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
            let rule = Rect::from_min_size(rect.min, vec2(2.0, rect.height()));
            ui.painter().rect_filled(rule, CornerRadius::ZERO, color);
            let inner = width - 2.0 - 2.0 * TERMINAL_PAD.x;
            let left = rect.left() + 2.0 + TERMINAL_PAD.x;
            let sentence = self.sentence(ui.ctx(), inner, look, palette);
            let top = rect.top() + TERMINAL_PAD.y;
            sentence.paint(ui.painter(), pos2(left, top));
            let line = TextRole::OBody.row_height(ui.ctx(), look.faces);
            let below = top + sentence.height();
            let space = space(ui.ctx(), look, " ");
            for (at, ((place, (key, words)), action)) in self
                .key_places(ui.ctx(), inner, look, palette)
                .into_iter()
                .zip(&self.actions)
                .enumerate()
            {
                let wide = Self::key_width(ui.ctx(), &(key.clone(), words.clone()), inner, look);
                let (x, y) = (left + place.x, below + place.y + line / 2.0);
                let place = Rect::from_min_size(pos2(x, y - line / 2.0), vec2(wide, line));
                let response = ui.interact(place, id.with(at), Sense::click());
                name(&response, action.label);
                if crate::ui::focus::shown(&response) || response.hovered() {
                    ui.painter()
                        .rect_filled(place, CornerRadius::ZERO, palette.selection);
                }
                key.paint_left(ui.painter(), x, y);
                words.paint_left(ui.painter(), x + key.width() + space, y);
                crate::ui::focus::hint(ui, &response, place, crate::ui::focus::Ring::Bar);
                if response.clicked() {
                    pressed = Some(at);
                }
            }
            return pressed;
        }
        ui.painter().rect(
            rect,
            CornerRadius::same(8),
            self.tone.fill(look, palette),
            Stroke::new(widgets::hairline(ui), self.tone.line(look, palette)),
            StrokeKind::Inside,
        );
        let icon = Rect::from_min_size(
            pos2(rect.left() + PAD.x, rect.top() + PAD.y + 3.0),
            vec2(ICON, ICON),
        );
        Icon::TriangleAlert.image(color, ICON).paint_at(ui, icon);
        let left = icon.right() + ICON_GAP;
        let inner = rect.right() - PAD.x - left;
        let sentence = self.sentence(ui.ctx(), inner, look, palette);
        sentence.paint(ui.painter(), pos2(left, rect.top() + PAD.y));
        let top = rect.top() + PAD.y + sentence.height() + GAP;
        let places = self.button_places(ui.ctx(), inner, look);
        for (at, (action, (place, wide))) in self.actions.iter().zip(places).enumerate() {
            let button =
                Rect::from_min_size(pos2(left + place.x, top + place.y), vec2(wide, BUTTON));
            let spec = ButtonSpec::new(action.label)
                .padding(BUTTON_PAD)
                .role(widgets::body(look))
                .salt(self.salt);
            let spec = if action.quiet { spec.link() } else { spec };
            let mut child = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(button)
                    .id_salt((self.salt, at)),
            );
            if spec.show_at(&mut child, button, look, palette).clicked() {
                pressed = Some(at);
            }
        }
        pressed
    }
}

/// Says `response` is a button named `name`.
fn name(response: &Response, name: &str) {
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, true, name));
}

/// The width of `text` in the terminal look's body.
fn space(ctx: &egui::Context, look: &Look, text: &str) -> f32 {
    Text::one(look, TextRole::OBody, text, egui::Color32::PLACEHOLDER)
        .layout(ctx)
        .width()
}
