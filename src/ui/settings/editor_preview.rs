//! Settings › Editor's preview: the SQL editor itself, over the boards'
//! Bookshop query (publishers and `book_covers`) as Format lays it out with
//! the settings, read-only. It shows every setting as it is set: the
//! keybindings' cursor, the line numbers, the cursor's line, wrap, and the
//! keywords' case and the commas. Its own state (the cursor, vim's mode) is
//! the window's, in egui's memory: nothing of it reaches the backend.

use std::sync::{Arc, Mutex, PoisonError};

use egui::text::{CCursor, CCursorRange};
use egui::{CornerRadius, Id, Sense, Stroke, StrokeKind, Ui, vec2};
use tabletist_db::Dialect;
use tabletist_db::sql;

use crate::i18n::Locale;
use crate::model::{SqlTab, TabId};
use crate::settings::Settings;
use crate::theme::{Look, Palette};

/// The boards' query, as typed: Format lays it out. Its keywords are of
/// both cases, so that Keep shows otherwise than upper and lower case.
const QUERY: &str = "SELECT p.name AS publisher, count(*) as covers from book_covers c \
                     JOIN publishers p on p.id = c.publisher_id GROUP BY p.name;";

/// The preview's text field: the editor's state is kept by it.
pub(crate) fn field_id() -> Id {
    Id::new("editor preview")
}

/// The query as Format writes it with `settings` in `look`.
fn text(settings: &Settings, look: &Look) -> String {
    let style = crate::ui::sql_text::format_style(settings, look);
    sql::format::format_with(Dialect::Postgres, QUERY, None, 0, style)
        .map_or_else(|| QUERY.to_owned(), |formatted| formatted.text)
}

/// The preview's editor, as the window keeps it: one for every frame, so
/// the cursor and vim's mode stay between them. A preview not drawn the
/// frame before (the window closed, or another pane shown) starts again,
/// with the cursor where the boards have it.
fn tab(ui: &Ui) -> Arc<Mutex<SqlTab>> {
    let frame = ui.ctx().cumulative_frame_nr();
    let drawn = field_id().with("drawn");
    let last: Option<u64> = ui.data(|data| data.get_temp(drawn));
    ui.data_mut(|data| {
        data.insert_temp(drawn, frame);
        // A new tab's text is empty: `show` sets its query and cursor.
        if last.is_some_and(|last| last + 1 < frame) {
            data.remove::<Arc<Mutex<SqlTab>>>(field_id().with("tab"));
        }
    });
    ui.data_mut(|data| {
        data.get_temp_mut_or_insert_with(field_id().with("tab"), || {
            let mut tab = SqlTab::new(TabId(0), 0, 0, None);
            // Nothing to type: vim starts in normal mode.
            tab.vim = crate::vim::Vim::new(crate::vim::Mode::Normal);
            tab.focus_editor = false;
            Arc::new(Mutex::new(tab))
        })
        .clone()
    })
}

/// The preview under its heading, `side` in from either side: the editor
/// in a card as tall as the query.
pub(super) fn show(
    ui: &mut Ui,
    side: f32,
    settings: &Settings,
    (look, palette, locale): (&Look, &Palette, Locale),
) {
    let shared = tab(ui);
    let mut tab = shared.lock().unwrap_or_else(PoisonError::into_inner);
    let text = text(settings, look);
    if tab.text != text {
        // The settings changed the query's layout: the cursor goes back
        // to where the boards have it, the start of the third line's
        // words (`FROM book_covers c`).
        let third = text.match_indices('\n').nth(1).map_or(0, |(at, _)| at + 1);
        let words = third + text[third..].len() - text[third..].trim_start_matches(' ').len();
        // Out of visual mode too: the cursor is where it is put now.
        tab.vim.to_normal();
        tab.vim.start_over(&text);
        tab.text = text;
        tab.cursor = words;
        tab.selected = None;
        let mut state = egui::TextEdit::load_state(ui.ctx(), field_id()).unwrap_or_default();
        let at = tab.text[..words].chars().count();
        state
            .cursor
            .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
        egui::TextEdit::store_state(ui.ctx(), field_id(), state);
    }
    // As tall as the field showed the query last frame, the card's border
    // round it; before it has drawn, as its lines would be.
    let height_id = field_id().with("height");
    let height = ui
        .data(|data| data.get_temp::<f32>(height_id))
        .unwrap_or_else(|| crate::ui::sql_text::preview_height(ui, look, &tab.text))
        + 2.0;
    let (place, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let card = place.shrink2(vec2(side, 0.0));
    if !card.is_positive() {
        return;
    }
    let mut inside = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("editor preview")
            .max_rect(card.shrink(1.0)),
    );
    inside.set_clip_rect(card.shrink(1.0).intersect(ui.clip_rect()));
    let (shown, response) = crate::ui::sql_text::preview(
        &mut inside,
        &mut tab,
        field_id(),
        settings,
        (look, palette, locale),
    );
    if ui.data(|data| data.get_temp::<f32>(height_id)) != Some(shown) {
        ui.data_mut(|data| data.insert_temp(height_id, shown));
        ui.ctx().request_repaint();
    }
    // The card's border, as the Appearance pane's preview has it. While
    // the preview has the keyboard, which its letters then move, the card
    // is a text field's ring: `focus` paints it.
    let radius = if look.terminal { 3 } else { 10 };
    crate::ui::focus::hint(
        ui,
        &response,
        card,
        crate::ui::focus::Ring::Field { radius },
    );
    let border = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    ui.painter().rect_stroke(
        card,
        CornerRadius::same(radius),
        Stroke::new(1.0, border),
        StrokeKind::Inside,
    );
}
