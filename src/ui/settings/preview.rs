//! The Appearance pane's preview: two rows of the Bookshop's `book_covers`
//! under the bar of a dev connection named Bookshop, drawn by the grid the
//! tables are drawn with and in the bar's colours, as the settings have
//! them now. The rows are made here: nothing is asked of a database, and
//! nothing is sent to the backend for them.

use egui::emath::GuiRounding as _;
use egui::{CornerRadius, Rect, Sense, Stroke, StrokeKind, Ui, pos2, vec2};
use tabletist_db::{ColumnMeta, Value, ValueKind};

use crate::env::{Environment, Platform};
use crate::settings::Settings;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::data_view::{self, Shown};
use crate::ui::grid::{self, Column};
use crate::ui::value_tags::Tags;
use crate::ui::workspace::{Badge, env_badge};

/// The connection the bar names, its environment and its database.
const NAME: &str = "Bookshop";
const ENV: Environment = Environment::Dev;
const DATABASE: &str = "bookshop_development";
/// The kinds `book_covers` allows, in the database's order: each value
/// takes its tag's slot by its place.
const KINDS: [&str; 3] = ["print", "ebook", "audio"];
/// The bar's line, and the macOS stripe over it.
const BAR: f32 = 30.0;
const STRIPE: f32 = 3.0;

/// The preview's columns, as a server would describe them.
fn columns() -> [ColumnMeta; 4] {
    let meta = |name: &str, type_name: &str, kind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    [
        meta("id", "int8", ValueKind::Numeric),
        meta("publisher_id", "int8", ValueKind::Numeric),
        meta("kind", "varchar", ValueKind::Text),
        meta("created_at", "timestamp", ValueKind::Temporal),
    ]
}

/// The preview's two rows.
fn rows() -> [[Value; 4]; 2] {
    [
        [
            Value::Int(2),
            Value::Int(9_100_000_000_000_000_001),
            Value::Text("print".into()),
            Value::Text("2026-01-12 09:14:03".into()),
        ],
        [
            Value::Int(3),
            Value::Int(9_100_000_000_000_000_002),
            Value::Text("ebook".into()),
            Value::Text("2026-01-12 17:02:41".into()),
        ],
    ]
}

/// The preview across `ui`'s width, `side` in from each of its edges, in a
/// group named `name`: the bar, then the grid's header and its two rows.
pub(super) fn show(
    ui: &mut Ui,
    (name, side): (&str, f32),
    settings: &Settings,
    look: &Look,
    palette: &Palette,
) {
    // The grid's rows as dense and as striped, and its header as tall, as
    // the settings have them.
    let grid_look = look.with_grid(settings);
    let bar = if look.terminal {
        BAR + 1.0
    } else {
        STRIPE + BAR + 1.0
    };
    let rows_height = grid::header_height(&grid_look) + 2.0 * grid_look.row();
    // The last row's rule, which the macOS grid draws under each row, is
    // the card's bottom border: the border is drawn over it, not under.
    let height = 1.0 + bar + rows_height;
    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    // On whole pixels, as the grid puts its rows: its last row ends on the
    // card's bottom.
    let card = place
        .shrink2(vec2(side, 0.0))
        .round_to_pixels(ui.pixels_per_point());
    if !card.is_positive() {
        return;
    }
    let inside = card.shrink(1.0);
    let mut group = ui.new_child(egui::UiBuilder::new().id_salt("preview").max_rect(card));
    // A group with the section's name, in the card's place: what is in it
    // is told to a screen reader as a picture's parts, disabled.
    group
        .ctx()
        .accesskit_node_builder(group.unique_id(), |node| {
            node.set_role(egui::accesskit::Role::Group);
            node.set_label(name);
            node.set_bounds(egui::accesskit::Rect {
                x0: card.min.x.into(),
                y0: card.min.y.into(),
                x1: card.max.x.into(),
                y1: card.max.y.into(),
            });
        });
    let top = Rect::from_min_size(inside.min, vec2(inside.width(), bar));
    paint_bar(&group, top, settings, look, palette);
    let below = Rect::from_min_max(
        pos2(inside.left(), top.bottom()),
        pos2(inside.right(), card.bottom()),
    );
    paint_grid(&mut group, below, settings, &grid_look, palette);
    // The card's rounded corners over the square ones of what it holds:
    // the window's colour outside them, held to the card so that it paints
    // only its corners, then its border.
    let radius = if look.terminal { 3 } else { 10 };
    let mask = (f32::from(radius) * (std::f32::consts::SQRT_2 - 1.0)).ceil() + 1.0;
    group
        .painter()
        .with_clip_rect(card.intersect(group.clip_rect()))
        .rect_stroke(
            card,
            CornerRadius::same(radius),
            Stroke::new(mask, palette.window),
            StrokeKind::Outside,
        );
    let painter = group.painter();
    let border = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    painter.rect_stroke(
        card,
        CornerRadius::same(radius),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
}

/// The connection bar in `rect`, its rule at the bottom: its fill as the
/// settings have it, the macOS stripe over it, and the connection named
/// with its environment, as the bar's own chip names it.
fn paint_bar(ui: &Ui, rect: Rect, settings: &Settings, look: &Look, palette: &Palette) {
    let platform = Platform::of(look);
    let env = crate::env::env_colors(ENV, platform, palette).on_bar(
        ENV,
        settings.env_bar,
        platform,
        palette,
    );
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::ZERO, env.bar_bg());
    crate::ui::widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, env.bar_border());
    if look.terminal {
        // The environment's tag, the name, and the database dim after it.
        let center = rect.top() + BAR / 2.0;
        let mut x = rect.left() + 10.0;
        x += env_badge(ui, x, center, ENV, &env, Badge::Tracked, look) + 8.0;
        let name = Text::one(look, TextRole::OGroup, NAME, palette.text).layout(ui.ctx());
        name.paint_left(painter, x, center);
        x += name.width() + 8.0;
        Text::one(look, TextRole::OBody, DATABASE, palette.dim)
            .layout(ui.ctx())
            .paint_left(painter, x, center);
    } else {
        // The stripe, then the bar's dot, the name and the environment's
        // pill.
        let stripe = Rect::from_min_size(rect.min, vec2(rect.width(), STRIPE));
        painter.rect_filled(stripe, CornerRadius::ZERO, env.base());
        let center = rect.top() + STRIPE + BAR / 2.0;
        let mut x = rect.left() + 12.0;
        painter.circle_filled(pos2(x + 3.5, center), 3.5, env.base());
        x += 7.0 + 8.0;
        let role = TextRole::UiBodySemibold;
        let name = Text::one(look, role, NAME, palette.text).layout(ui.ctx());
        name.paint_left(painter, x, center);
        x += name.width() + 8.0;
        env_badge(ui, x, center, ENV, &env, Badge::Chip, look);
    }
}

/// The grid in `rect`: the header and the two rows, drawn by the tables'
/// grid as a picture of it. It takes no click, no Tab and no wheel, and
/// fits its columns again each frame, to the faces and the header the
/// settings have now.
fn paint_grid(ui: &mut Ui, rect: Rect, settings: &Settings, look: &Look, palette: &Palette) {
    let mut still = ui.new_child(egui::UiBuilder::new().id_salt("grid").max_rect(rect));
    // Disabled at full strength: egui draws a disabled Ui faded.
    still.visuals_mut().disabled_alpha = 1.0;
    still.disable();
    let id = still.id().with("rows");
    grid::forget(still.ctx(), id);
    let metas = columns();
    let rows = rows();
    let kinds = KINDS.map(String::from);
    let tags = |col: usize| match col {
        2 => Tags::Values(&kinds).when(settings.value_tags),
        _ => Tags::None,
    };
    let columns: Vec<Column> = metas
        .iter()
        .map(|meta| {
            let (type_line, _) =
                data_view::type_line(&meta.name, &meta.type_name, meta.kind, None, look);
            Column {
                name: &meta.name,
                type_line,
                numeric: meta.kind == ValueKind::Numeric,
                sort: None,
                key: false,
                // The last takes what is left, as the boards' grids have it.
                flexible: meta.name == "created_at",
                sortable: false,
                required: false,
                comment: None,
            }
        })
        .collect();
    let ctx = still.ctx().clone();
    // The ids name rows: not in threes, as a table's keys are not.
    let shown = Shown::default();
    grid::show(
        &mut still,
        id,
        &columns,
        rows.len(),
        None,
        false,
        palette,
        look,
        &|row| grid::Row {
            number: Some(row as u64 + 1),
            ..grid::Row::default()
        },
        None,
        None,
        |row, col| data_view::cell(&ctx, &rows[row][col], &metas[col], &tags(col), look, shown),
    );
}
