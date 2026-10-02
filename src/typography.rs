//! Every text style the interface draws with, and the fonts behind them.
//!
//! Views never name a font, a family or a size: they draw through a
//! [`TextRole`], one per style in the 0.1.0 typography spec, plus the styles
//! the design's CSS uses that the spec's table leaves out (the CSS wins).
//! A role is a family, a weight, a size, letter spacing, line height and a
//! case; [`TextRole::format`] turns it into egui's [`TextFormat`] in the
//! look's faces (Plex on macOS, the terminal's monospace on Omarchy, Inter
//! elsewhere).
//!
//! Text is built with [`Text`] and drawn from the [`Laid`] it lays out.

pub mod desktop_font;
mod fonts;

pub use fonts::configure;

use std::sync::Arc;

use egui::text::LayoutJob;
use egui::{
    Color32, FontFamily, FontId, Galley, Painter, Pos2, Response, Sense, TextFormat, Ui, Vec2,
};

use crate::theme::{Faces, Look};

/// Sans or monospace: what a role asks for. The Omarchy look has one
/// monospace face for both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Sans,
    Mono,
}

/// A text style: what [`TextRole::spec`] returns.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Spec {
    pub kind: Kind,
    /// CSS weight: 400, 500, 600 or 700.
    pub weight: u16,
    /// Points (CSS px at 1x).
    pub size: f32,
    /// Letter spacing in em.
    pub tracking: f32,
    /// Line height as a multiple of the size; `None` is the font's own.
    pub line_height: Option<f32>,
    pub uppercase: bool,
}

/// One text style. The first thirteen are the spec's macOS styles, the
/// `O`-prefixed ones its Omarchy styles; the rest are styles the design's
/// CSS uses that the spec's tables leave out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TextRole {
    // macOS, from the spec.
    UiBody,
    UiBodyStrong,
    TableTitle,
    ScreenTitle,
    DialogTitle,
    SectionLabel,
    Secondary,
    FieldLabel,
    ColumnType,
    GridCell,
    Json,
    Tag,
    Shortcut,
    // macOS, from the design's CSS.
    /// Connection names, column names, the inspector's title.
    UiBodySemibold,
    /// The picker's group headings.
    GroupLabel,
    /// The picker's environment tags.
    EnvTag,
    /// The environment tag in the connection bar's chips.
    ChipEnv,
    /// View and trigger tags in the object tree.
    TagSmall,
    /// The database name, the sort chip's column, a connection's database.
    MonoSecondary,
    /// Prefix groups in the object tree.
    MonoGroup,
    /// Tag values in grid cells.
    ValueTag,
    /// The `{ 3 }` chip in JSON cells.
    JsonChip,
    /// Values in the inspector.
    InspectorValue,
    /// The SQL editor's script and its line numbers.
    Code,
    /// Labels over the connection dialog's own fields (name, type, environment, the URL).
    FormLabel,
    /// The connection dialog's group headings (Database, Server, Security).
    Legend,
    /// What an empty area or a failed connection says first.
    StateTitle,
    // Omarchy, from the spec.
    OBody,
    OTableTitle,
    OScreenTitle,
    OGroup,
    OSecondary,
    OCaption,
    OColumnType,
    OJson,
    OEnvLabel,
    OModeLine,
    // Omarchy, from the design's CSS.
    /// The object filter.
    OField,
    /// The picker's environment badges (upper case in the data, untracked).
    OBadge,
    /// The SQL editor's script and its line numbers.
    OCode,
}

impl TextRole {
    pub const ALL: [TextRole; 40] = [
        Self::UiBody,
        Self::UiBodyStrong,
        Self::TableTitle,
        Self::ScreenTitle,
        Self::DialogTitle,
        Self::SectionLabel,
        Self::Secondary,
        Self::FieldLabel,
        Self::ColumnType,
        Self::GridCell,
        Self::Json,
        Self::Tag,
        Self::Shortcut,
        Self::UiBodySemibold,
        Self::GroupLabel,
        Self::EnvTag,
        Self::ChipEnv,
        Self::TagSmall,
        Self::MonoSecondary,
        Self::MonoGroup,
        Self::ValueTag,
        Self::JsonChip,
        Self::InspectorValue,
        Self::Code,
        Self::FormLabel,
        Self::Legend,
        Self::StateTitle,
        Self::OBody,
        Self::OTableTitle,
        Self::OScreenTitle,
        Self::OGroup,
        Self::OSecondary,
        Self::OCaption,
        Self::OColumnType,
        Self::OJson,
        Self::OEnvLabel,
        Self::OModeLine,
        Self::OField,
        Self::OBadge,
        Self::OCode,
    ];

    /// The style's name in the 0.1.0 typography spec.
    pub const fn id(self) -> &'static str {
        match self {
            Self::UiBody => "ui-body",
            Self::UiBodyStrong => "ui-body-strong",
            Self::TableTitle => "table-title",
            Self::ScreenTitle => "screen-title",
            Self::DialogTitle => "dialog-title",
            Self::SectionLabel => "section-label",
            Self::Secondary => "secondary",
            Self::FieldLabel => "field-label",
            Self::ColumnType => "column-type",
            Self::GridCell => "grid-cell",
            Self::Json => "json",
            Self::Tag => "tag",
            Self::Shortcut => "shortcut",
            Self::UiBodySemibold => "ui-body-semibold",
            Self::GroupLabel => "group-label",
            Self::EnvTag => "env-tag",
            Self::ChipEnv => "chip-env",
            Self::TagSmall => "tag-small",
            Self::MonoSecondary => "mono-secondary",
            Self::MonoGroup => "mono-group",
            Self::ValueTag => "value-tag",
            Self::JsonChip => "json-chip",
            Self::InspectorValue => "inspector-value",
            Self::Code => "code",
            Self::FormLabel => "form-label",
            Self::Legend => "legend",
            Self::StateTitle => "state-title",
            Self::OBody => "o-body",
            Self::OTableTitle => "o-table-title",
            Self::OScreenTitle => "o-screen-title",
            Self::OGroup => "o-group",
            Self::OSecondary => "o-secondary",
            Self::OCaption => "o-caption",
            Self::OColumnType => "o-column-type",
            Self::OJson => "o-json",
            Self::OEnvLabel => "o-env-label",
            Self::OModeLine => "o-mode-line",
            Self::OField => "o-field",
            Self::OBadge => "o-badge",
            Self::OCode => "o-code",
        }
    }

    pub const fn spec(self) -> Spec {
        use Kind::{Mono, Sans};
        const fn style(kind: Kind, weight: u16, size: f32) -> Spec {
            Spec {
                kind,
                weight,
                size,
                tracking: 0.0,
                line_height: None,
                uppercase: false,
            }
        }
        const fn capitals(kind: Kind, weight: u16, size: f32, tracking: f32) -> Spec {
            Spec {
                kind,
                weight,
                size,
                tracking,
                line_height: None,
                uppercase: true,
            }
        }
        const fn lines(kind: Kind, weight: u16, size: f32, line_height: f32) -> Spec {
            Spec {
                kind,
                weight,
                size,
                tracking: 0.0,
                line_height: Some(line_height),
                uppercase: false,
            }
        }
        match self {
            Self::UiBody => style(Sans, 400, 13.0),
            Self::UiBodyStrong => style(Sans, 500, 13.0),
            Self::TableTitle => style(Sans, 600, 17.0),
            Self::ScreenTitle => style(Sans, 600, 20.0),
            Self::DialogTitle => style(Sans, 600, 17.0),
            Self::SectionLabel => capitals(Sans, 600, 11.0, 0.06),
            Self::Secondary => style(Sans, 400, 12.0),
            Self::FieldLabel => style(Sans, 400, 11.5),
            Self::ColumnType => style(Sans, 400, 11.0),
            Self::GridCell => style(Mono, 400, 12.5),
            Self::Json => lines(Mono, 400, 11.5, 1.6),
            Self::Tag => style(Sans, 500, 11.5),
            Self::Shortcut => style(Sans, 400, 11.0),
            Self::UiBodySemibold => style(Sans, 600, 13.0),
            Self::GroupLabel => capitals(Sans, 600, 12.0, 0.06),
            Self::EnvTag => style(Sans, 500, 11.0),
            Self::ChipEnv => style(Sans, 500, 10.5),
            Self::TagSmall => style(Sans, 400, 10.5),
            Self::MonoSecondary => style(Mono, 400, 12.0),
            Self::MonoGroup => style(Mono, 500, 12.0),
            Self::ValueTag => style(Mono, 400, 11.5),
            Self::JsonChip => style(Mono, 400, 11.0),
            Self::InspectorValue => style(Mono, 400, 13.0),
            // 22 pt lines.
            Self::Code => lines(Mono, 400, 13.0, 22.0 / 13.0),
            Self::FormLabel => style(Sans, 500, 12.0),
            Self::Legend => style(Sans, 600, 12.0),
            Self::StateTitle => style(Sans, 600, 15.0),
            Self::OBody => style(Mono, 400, 13.0),
            Self::OTableTitle => style(Mono, 700, 15.0),
            Self::OScreenTitle => style(Mono, 700, 14.0),
            Self::OGroup => style(Mono, 700, 13.0),
            Self::OSecondary => style(Mono, 400, 12.0),
            Self::OCaption => style(Mono, 400, 11.5),
            Self::OColumnType => style(Mono, 400, 11.0),
            Self::OJson => lines(Mono, 400, 12.0, 1.7),
            Self::OEnvLabel => capitals(Mono, 700, 11.5, 0.04),
            Self::OModeLine => style(Mono, 700, 12.0),
            Self::OField => style(Mono, 400, 12.5),
            Self::OBadge => style(Mono, 700, 11.5),
            Self::OCode => lines(Mono, 400, 13.0, 22.0 / 13.0),
        }
    }

    /// The Omarchy role when the look is a terminal's, else the macOS one.
    pub fn pick(look: &Look, mac: Self, omarchy: Self) -> Self {
        if look.terminal { omarchy } else { mac }
    }

    pub fn size(self) -> f32 {
        self.spec().size
    }

    /// The role's font in `faces`.
    pub fn font_id(self, faces: Faces) -> FontId {
        let spec = self.spec();
        FontId::new(spec.size, family(faces, spec.kind, spec.weight))
    }

    /// Letter spacing in points.
    pub fn tracking(self) -> f32 {
        let spec = self.spec();
        spec.tracking * spec.size
    }

    /// Line height in points, when the role sets one.
    pub fn line_height(self) -> Option<f32> {
        let spec = self.spec();
        spec.line_height.map(|factor| factor * spec.size)
    }

    /// egui's format for the role in `faces`.
    pub fn format(self, faces: Faces, color: Color32) -> TextFormat {
        TextFormat {
            font_id: self.font_id(faces),
            extra_letter_spacing: self.tracking(),
            line_height: self.line_height(),
            color,
            ..Default::default()
        }
    }

    /// `text` in the role's case.
    pub fn transform(self, text: &str) -> std::borrow::Cow<'_, str> {
        if self.spec().uppercase {
            text.to_uppercase().into()
        } else {
            text.into()
        }
    }

    /// The width of `text` in the role, for fitting it (draws nothing).
    pub fn width(self, ctx: &egui::Context, faces: Faces, text: &str) -> f32 {
        let mut job = LayoutJob::default();
        job.append(
            &self.transform(text),
            0.0,
            self.format(faces, Color32::PLACEHOLDER),
        );
        ctx.fonts_mut(|fonts| fonts.layout_job(job)).size().x
    }

    /// The height of the role's capitals in points, in faces whose line box
    /// does not centre on them (Plex).
    fn capitals(self, faces: Faces) -> Option<f32> {
        fonts::capitals(faces).map(|ems| ems * self.size())
    }

    /// Where a line of the role centres, below its top (see
    /// [`Laid::middle`]): for placing text that egui paints itself, such as a
    /// field's.
    pub fn middle(self, ctx: &egui::Context, faces: Faces) -> f32 {
        Text::in_faces(faces)
            .add(self, "H", Color32::PLACEHOLDER)
            .layout(ctx)
            .middle()
    }

    /// The height of one line in the role, in `faces`.
    pub fn row_height(self, ctx: &egui::Context, faces: Faces) -> f32 {
        self.line_height()
            .unwrap_or_else(|| ctx.fonts_mut(|fonts| fonts.row_height(&self.font_id(faces))))
    }
}

/// The weights `faces` has for `kind`.
fn weights(faces: Faces, kind: Kind) -> &'static [u16] {
    match (faces, kind) {
        (Faces::Plex, Kind::Sans) => &[400, 500, 600],
        (Faces::Plex, Kind::Mono) => &[400, 500],
        (Faces::Terminal, _) => &[400, 500, 700],
        (Faces::Inter, Kind::Sans) => &[400, 500, 600, 700],
        (Faces::Inter, Kind::Mono) => &[400],
    }
}

/// The name of the family holding `faces`' `kind` at `weight`.
pub fn family_name(faces: Faces, kind: Kind, weight: u16) -> String {
    let face = match (faces, kind) {
        (Faces::Plex, Kind::Sans) => "plex-sans",
        (Faces::Plex, Kind::Mono) => "plex-mono",
        (Faces::Terminal, _) => "omarchy-mono",
        (Faces::Inter, Kind::Sans) => "inter",
        (Faces::Inter, Kind::Mono) => "standard-mono",
    };
    format!("{face}-{weight}")
}

/// The family for `kind` at `weight` in `faces`, at the nearest weight the
/// faces have (ties go heavier).
pub fn family(faces: Faces, kind: Kind, weight: u16) -> FontFamily {
    let nearest = weights(faces, kind)
        .iter()
        .copied()
        .min_by_key(|have| (have.abs_diff(weight), u16::MAX - have))
        .unwrap_or(400);
    FontFamily::Name(family_name(faces, kind, nearest).into())
}

/// Text in one or more roles, before layout.
pub struct Text {
    job: LayoutJob,
    faces: Faces,
    chars: usize,
    /// The height of the first piece's capitals ([`TextRole::capitals`]).
    capitals: Option<f32>,
}

impl Text {
    pub fn new(look: &Look) -> Self {
        Self::in_faces(look.faces)
    }

    pub fn in_faces(faces: Faces) -> Self {
        Self {
            job: LayoutJob::default(),
            faces,
            chars: 0,
            capitals: None,
        }
    }

    /// `text` alone, in `role`.
    pub fn one(look: &Look, role: TextRole, text: &str, color: Color32) -> Self {
        Self::new(look).add(role, text, color)
    }

    /// Appends `text` in `role` and `color`.
    pub fn add(self, role: TextRole, text: &str, color: Color32) -> Self {
        self.add_with(role, text, 0.0, |format| format.color = color)
    }

    /// Appends `runs` in `role`, each in its own colour: for text cut
    /// into many pieces (highlighted code), which share one format.
    pub fn add_runs<'a>(
        mut self,
        role: TextRole,
        runs: impl IntoIterator<Item = (&'a str, Color32)>,
    ) -> Self {
        let format = role.format(self.faces, Color32::PLACEHOLDER);
        for (text, color) in runs {
            let text = role.transform(text);
            self.chars += text.chars().count();
            let format = TextFormat {
                color,
                ..format.clone()
            };
            self.job.append(&text, 0.0, format);
        }
        self
    }

    /// Appends white space in `role`: room between pieces.
    pub fn space(mut self, role: TextRole, space: &str) -> Self {
        debug_assert!(space.trim().is_empty(), "only white space: {space:?}");
        self.chars += space.chars().count();
        self.job
            .append(space, 0.0, role.format(self.faces, Color32::PLACEHOLDER));
        self
    }

    /// Appends `text` in `role`, `leading` points after what came before,
    /// with `style` changing the format's colour or decoration (never its
    /// font, size, spacing or line height).
    pub fn add_with(
        mut self,
        role: TextRole,
        text: &str,
        leading: f32,
        style: impl FnOnce(&mut TextFormat),
    ) -> Self {
        let text = role.transform(text);
        let mut format = role.format(self.faces, Color32::PLACEHOLDER);
        style(&mut format);
        if self.chars == 0 {
            self.capitals = role.capitals(self.faces);
        }
        self.chars += text.chars().count();
        self.job.append(&text, leading, format);
        self
    }

    /// Wraps at `width` points.
    pub fn wrap(mut self, width: f32) -> Self {
        self.job.wrap.max_width = width;
        self
    }

    /// The job, for alignment and wrapping options.
    pub fn job_mut(&mut self) -> &mut LayoutJob {
        &mut self.job
    }

    pub fn is_empty(&self) -> bool {
        self.chars == 0
    }

    pub fn layout(self, ctx: &egui::Context) -> Laid {
        let galley = ctx.fonts_mut(|fonts| fonts.layout_job(self.job));
        Laid {
            galley,
            capitals: self.capitals,
        }
    }
}

/// Laid-out text, ready to paint.
#[derive(Clone)]
pub struct Laid {
    pub galley: Arc<Galley>,
    capitals: Option<f32>,
}

impl Laid {
    pub fn size(&self) -> Vec2 {
        self.galley.size()
    }

    pub fn width(&self) -> f32 {
        self.galley.size().x
    }

    pub fn height(&self) -> f32 {
        self.galley.size().y
    }

    /// Paints with the top-left corner at `pos`.
    pub fn paint(&self, painter: &Painter, pos: Pos2) {
        painter.galley(pos, self.galley.clone(), Color32::PLACEHOLDER);
    }

    /// How far below its top the text centres: on the middle of its first
    /// line's capitals, which is where an icon beside it reads as level
    /// with it. Faces whose line box centres on the capitals by itself
    /// (all but Plex) use the middle of the box.
    pub fn middle(&self) -> f32 {
        let half = self.height() / 2.0;
        let (Some(capitals), Some(row)) = (self.capitals, self.galley.rows.first()) else {
            return half;
        };
        // How far a glyph's own baseline lies above the leading face's:
        // epaint centres a fallback face's line box (a key symbol's) on the
        // leading face's instead of sharing its baseline.
        let lift = |glyph: &egui::epaint::text::Glyph| {
            glyph.font_ascent
                - glyph.font_face_ascent
                - (glyph.font_height - glyph.font_face_height) / 2.0
        };
        let leading = row.glyphs.iter().find(|glyph| lift(glyph) == 0.0);
        let Some(glyph) = leading.or_else(|| row.glyphs.first()) else {
            return half;
        };
        let baseline = glyph.pos.y + lift(glyph);
        half - row.size.y / 2.0 + baseline - capitals / 2.0
    }

    /// Paints with the left edge at `x`, centred on `y`. Returns the width.
    pub fn paint_left(&self, painter: &Painter, x: f32, y: f32) -> f32 {
        self.paint(painter, egui::pos2(x, y - self.middle()));
        self.width()
    }

    /// Paints with the right edge at `right`, centred on `y`. Returns the width.
    pub fn paint_right(&self, painter: &Painter, right: f32, y: f32) -> f32 {
        self.paint(painter, egui::pos2(right - self.width(), y - self.middle()));
        self.width()
    }

    /// Paints centred on `center`.
    pub fn paint_center(&self, painter: &Painter, center: Pos2) {
        self.paint(
            painter,
            egui::pos2(center.x - self.width() / 2.0, center.y - self.middle()),
        );
    }

    /// Adds the text as a label widget in `ui`'s layout.
    pub fn label(self, ui: &mut Ui) -> Response {
        self.label_sense(ui, Sense::hover())
    }

    /// [`Self::label`] that answers `sense`.
    pub fn label_sense(self, ui: &mut Ui, sense: Sense) -> Response {
        let (rect, response) = ui.allocate_exact_size(self.size(), sense);
        let text = self.galley.text().to_owned();
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &text));
        if ui.is_rect_visible(rect) {
            self.paint(ui.painter(), rect.min);
        }
        response
    }
}

/// A text edit layouter drawing its contents in `role`.
pub fn layouter(
    look: &Look,
    role: TextRole,
    color: Color32,
) -> impl FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> + use<> {
    let faces = look.faces;
    move |ui, buffer, wrap| {
        Text::in_faces(faces)
            .add(role, buffer.as_str(), color)
            .wrap(wrap)
            .layout(ui.ctx())
            .galley
    }
}

/// egui's own text styles (what its built-in widgets draw with), from the
/// roles.
pub fn text_styles(faces: Faces) -> std::collections::BTreeMap<egui::TextStyle, FontId> {
    use egui::TextStyle;
    let (small, body, heading, mono) = if faces == Faces::Terminal {
        (
            TextRole::OSecondary,
            TextRole::OBody,
            TextRole::OScreenTitle,
            TextRole::OSecondary,
        )
    } else {
        (
            TextRole::Secondary,
            TextRole::UiBody,
            TextRole::ScreenTitle,
            TextRole::MonoSecondary,
        )
    };
    [
        (TextStyle::Small, small.font_id(faces)),
        (TextStyle::Body, body.font_id(faces)),
        (TextStyle::Button, body.font_id(faces)),
        (TextStyle::Heading, heading.font_id(faces)),
        (TextStyle::Monospace, mono.font_id(faces)),
    ]
    .into()
}
