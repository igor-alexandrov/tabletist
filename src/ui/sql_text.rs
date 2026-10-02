//! A SQL editor's text: the script, highlighted through the tokenizer,
//! beside a gutter of line numbers, with a band behind the statement Run
//! executes.

use std::ops::{Range, RangeInclusive};
use std::sync::Arc;

use egui::text::{CCursor, CCursorRange};
use egui::{
    Color32, CornerRadius, Galley, Id, Margin, Rect, Sense, Shape, Ui, WidgetInfo, WidgetType,
    pos2, vec2,
};
use tabletist_db::Dialect;
use tabletist_db::sql::{self, Statement, Token, TokenKind};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, ConnTabId, Pane, SqlTab, TabId, TextPrint};
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;

/// The script's and the line numbers' role: monospace in every look.
fn role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::Code, TextRole::OCode)
}

/// The colour a token of `kind` is drawn in. An executable comment is SQL
/// the server runs, so it does not read as a comment.
pub fn color_of(kind: TokenKind, palette: &Palette) -> Color32 {
    match kind {
        TokenKind::Keyword => palette.magenta,
        TokenKind::String => palette.success,
        TokenKind::Number => palette.orange,
        TokenKind::Comment => palette.dim,
        TokenKind::Identifier
        | TokenKind::QuotedIdentifier
        | TokenKind::ExecutableComment
        | TokenKind::Operator
        | TokenKind::Punctuation
        | TokenKind::Semicolon
        | TokenKind::Whitespace => palette.text,
    }
}

/// `text` as runs of one colour, in order and without gaps: neighbouring
/// `tokens` that share a colour are one run.
fn runs(tokens: &[Token], text: &str, palette: &Palette) -> Vec<(Range<usize>, Color32)> {
    let mut runs: Vec<(Range<usize>, Color32)> = Vec::new();
    let mut push = |range: Range<usize>, color: Color32| {
        if range.is_empty() {
            return;
        }
        match runs.last_mut() {
            Some((last, shared)) if *shared == color && last.end == range.start => {
                last.end = range.end;
            }
            _ => runs.push((range, color)),
        }
    };
    let mut at = 0;
    for token in tokens {
        // The tokens cover the text; were one missing, its text is plain.
        push(at..token.range.start, palette.text);
        at = token.range.end;
        push(token.range.clone(), color_of(token.kind, palette));
    }
    push(at..text.len(), palette.text);
    runs
}

#[cfg(test)]
thread_local! {
    /// How many times this thread tokenized an editor's script.
    pub static TOKENIZED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// What an editor works out from its script, tokenized once for both: the
/// colours it is drawn in and the statements it holds.
struct Parsed {
    /// What it was worked out from: the script, the dialect that reads it
    /// and the colours.
    of: (TextPrint, Dialect, Palette),
    runs: Vec<(Range<usize>, Color32)>,
    statements: Vec<Statement>,
}

/// `text` parsed, from egui's memory for the editor `id` when that is of
/// the same text, dialect and colours: a frame that changes none of them
/// tokenizes nothing.
fn parsed(
    ctx: &egui::Context,
    id: Id,
    dialect: Dialect,
    palette: &Palette,
    text: &str,
) -> Arc<Parsed> {
    let of = (TextPrint::of(text), dialect, *palette);
    let kept: Option<Arc<Parsed>> = ctx.data(|data| data.get_temp(id));
    if let Some(kept) = kept
        && kept.of == of
    {
        return kept;
    }
    #[cfg(test)]
    TOKENIZED.with(|count| count.set(count.get() + 1));
    let tokens = sql::tokenize(dialect, text);
    let parsed = Arc::new(Parsed {
        of,
        runs: runs(&tokens, text, palette),
        statements: sql::statements_from(text, &tokens),
    });
    ctx.data_mut(|data| data.insert_temp(id, Arc::clone(&parsed)));
    parsed
}

/// The id egui keeps the text field of the editor `id` under.
pub fn editor_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("sql-text", tab.0, id.0))
}

/// The scroll area of an editor, kept under the editor's id so `forget`
/// finds it.
#[derive(Clone, Copy)]
struct ScrollId(Id);

/// Drops what egui's memory keeps for a closed editor: its text field's
/// cursor and undo history (which holds copies of the script), what was
/// worked out from the script, and where it was scrolled to.
pub fn forget(ctx: &egui::Context, tab: ConnTabId, id: TabId) {
    let editor = editor_id(tab, id);
    ctx.data_mut(|data| {
        data.remove::<egui::text_edit::TextEditState>(editor);
        data.remove::<Arc<Parsed>>(editor);
        if let Some(ScrollId(area)) = data.get_temp(editor) {
            data.remove::<egui::scroll_area::State>(area);
        }
        data.remove::<ScrollId>(editor);
    });
}

/// What egui's memory keeps for the editor `id`, by name.
#[cfg(test)]
pub fn remembered(ctx: &egui::Context, tab: ConnTabId, id: TabId) -> Vec<&'static str> {
    let editor = editor_id(tab, id);
    let area: Option<ScrollId> = ctx.data(|data| data.get_temp(editor));
    let scrolled =
        area.is_some_and(|ScrollId(area)| egui::scroll_area::State::load(ctx, area).is_some());
    [
        ("text", egui::TextEdit::load_state(ctx, editor).is_some()),
        (
            "parsed",
            ctx.data(|data| data.get_temp::<Arc<Parsed>>(editor).is_some()),
        ),
        ("scroll id", area.is_some()),
        ("scroll", scrolled),
    ]
    .into_iter()
    .filter_map(|(name, kept)| kept.then_some(name))
    .collect()
}

/// A text edit layouter for the editor `id` that colours `dialect`'s
/// tokens and never wraps: one galley row per line, so the gutter can
/// number the rows.
pub fn layouter(
    look: &Look,
    palette: &Palette,
    dialect: Dialect,
    id: Id,
) -> impl FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> + use<> {
    let (faces, role, palette) = (look.faces, role(look), *palette);
    move |ui, buffer, _wrap| {
        let text = buffer.as_str();
        let parsed = parsed(ui.ctx(), id, dialect, &palette, text);
        let runs = parsed.runs.iter();
        let mut laid = Text::in_faces(faces).add_runs(
            role,
            runs.map(|(range, color)| (&text[range.clone()], *color)),
        );
        if text.is_empty() {
            // An empty script still has a line, as tall as any other.
            laid = laid.add(role, "", palette.text);
        }
        laid.layout(ui.ctx()).galley
    }
}

/// The colour of the bar beside the statement Run executes: the accent in
/// the terminal, the splitter grip's grey elsewhere.
pub fn bar_color(look: &Look, palette: &Palette) -> Color32 {
    if look.terminal {
        palette.accent
    } else {
        palette.border.lerp_to_gamma(palette.faint, 0.3)
    }
}

/// The byte offset in `text` of the character index `chars`, as egui
/// counts a cursor.
fn byte_offset(text: &str, chars: usize) -> usize {
    text.char_indices()
        .nth(chars)
        .map_or(text.len(), |(byte, _)| byte)
}

/// The character index, as egui counts a cursor, of the byte offset
/// `bytes` in `text`.
fn char_index(text: &str, bytes: usize) -> usize {
    text.char_indices()
        .take_while(|(byte, _)| *byte < bytes)
        .count()
}

/// Formats the script, or the statements the field's selection overlaps,
/// as one step of the field's undo history. Changes nothing when Format
/// has nothing to change.
fn format(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) {
    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
    // A field that never had the keys has no cursor of its own.
    let typed = state.cursor.char_range();
    let (selection, cursor) = match typed {
        Some(range) => {
            let [start, end] = range.sorted_cursors();
            let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
            (Some(byte(start)..byte(end)), byte(range.primary))
        }
        None => (None, sql_tab.cursor),
    };
    let Some(formatted) = sql::format::format(field.dialect, &sql_tab.text, selection, cursor)
    else {
        return;
    };
    let cursor = CCursor::new(char_index(&formatted.text, formatted.cursor));
    let (before, after) = (typed.unwrap_or_default(), CCursorRange::one(cursor));
    // The text as typed and the text as formatted, both: undo gives the
    // first back whole, and redo the second.
    let mut undoer = state.undoer();
    undoer.add_undo(&(before, sql_tab.text.clone()));
    undoer.add_undo(&(after, formatted.text.clone()));
    state.set_undoer(undoer);
    state.cursor.set_char_range(Some(after));
    egui::TextEdit::store_state(ui.ctx(), field.id, state);
    sql_tab.text = formatted.text;
    sql_tab.cursor = formatted.cursor;
}

/// The number the gutter shows for `line`: its own, or (the terminal's
/// relative numbers) how far it is from the cursor's line, which keeps
/// its own.
fn gutter_number(line: usize, cursor_line: usize, relative: bool) -> usize {
    if relative && line != cursor_line {
        line.abs_diff(cursor_line)
    } else {
        line
    }
}

/// The lines of a statement's SQL: from its first token that is not a
/// comment to its last line.
fn statement_lines(statement: &Statement) -> RangeInclusive<usize> {
    let newlines = statement.text.bytes().filter(|b| *b == b'\n').count();
    statement.first_line..=statement.start_line + newlines
}

/// Where the gutter's pieces sit, from the pane's left edge.
struct Gutter {
    /// Where the line numbers end.
    numbers_right: f32,
    /// The bar beside the statement Run executes.
    bar: egui::Rangef,
    /// The whole gutter: the text's pane starts here.
    width: f32,
    /// The text's padding after the bar.
    text_left: i8,
}

impl Gutter {
    /// macOS: 52 pt of numbers ending 12 pt before a 4 pt bar, then 14 pt.
    /// The terminal: 48, 10, a 3 pt bar and 12. The numbers' column grows
    /// with the digits of `lines`.
    fn new(ui: &Ui, look: &Look, lines: usize) -> Self {
        let (column, pad, bar, text_left) = if look.terminal {
            (48.0, 10.0, 3.0, 12)
        } else {
            (52.0, 12.0, 4.0, 14)
        };
        let digits = lines.max(1).ilog10() as usize + 1;
        let digit = role(look).width(ui.ctx(), look.faces, "0");
        let column = (digits as f32 * digit + pad + 8.0).ceil().max(column);
        Self {
            numbers_right: column - pad,
            bar: egui::Rangef::new(column, column + bar),
            width: column + bar,
            text_left,
        }
    }
}

/// What a frame of the text edit leaves for the gutter and the band.
struct Edited {
    galley: Arc<Galley>,
    /// Where the galley's top left corner is on screen.
    origin: egui::Pos2,
    focused: bool,
}

impl Edited {
    /// The screen's vertical range of the 1-based `lines`, each line being
    /// a row of the galley.
    fn span(&self, lines: &RangeInclusive<usize>) -> Option<egui::Rangef> {
        let first = self.galley.rows.get(lines.start().checked_sub(1)?)?;
        let last = self.galley.rows.get(lines.end().checked_sub(1)?)?;
        Some(egui::Rangef::new(
            self.origin.y + first.min_y(),
            self.origin.y + last.max_y(),
        ))
    }
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(workspace) = app.workspace_mut(tab) else {
        return;
    };
    let dialect = workspace.driver.dialect();
    let tree_has_arrows = workspace.pane == Pane::Tree;
    let Some(sql_tab) = workspace.sql_tab_mut(id) else {
        return;
    };
    let pane = ui.max_rect();
    let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
    let gutter = Gutter::new(ui, &look, lines);
    let editor = editor_id(tab, id);
    // Behind the text: filled in once the rows are laid out.
    let backdrop = ui.painter().add(Shape::Noop);
    // The text scrolls both ways beside the gutter, which stays put.
    let text_pane = Rect::from_min_max(pos2(pane.left() + gutter.width, pane.top()), pane.max);
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
    child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
    let scrolled = egui::ScrollArea::both()
        .id_salt("scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| {
            let field = Field {
                id: editor,
                name: &gettext(locale, "SQL"),
                left: gutter.text_left,
                dialect,
                look: &look,
                palette: &palette,
            };
            edit(ui, sql_tab, &field)
        });
    ui.data_mut(|data| data.insert_temp(editor, ScrollId(scrolled.id)));
    let mut edited = scrolled.inner;
    // The gutter is the editor's too: a press on it gives the editor the
    // keys, with the cursor at the start of the line pressed. (After the
    // field, which gives the keys up when a press lands outside it.)
    let numbers = Rect::from_min_max(pane.min, pos2(text_pane.left(), pane.bottom()));
    let pressed = ui.interact(numbers, ui.id().with("gutter"), Sense::CLICK);
    let name = gettext(locale, "Line numbers");
    pressed.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, name.as_ref()));
    // From the press to the click that ends it: the field gives the keys
    // up on either, whichever egui counts as a click elsewhere.
    if (pressed.is_pointer_button_down_on() || pressed.clicked())
        && let Some(pointer) = pressed.interact_pointer_pos()
    {
        let at = edited
            .galley
            .cursor_from_pos(vec2(0.0, pointer.y - edited.origin.y));
        let mut state = egui::TextEdit::load_state(ui.ctx(), editor).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::one(at)));
        egui::TextEdit::store_state(ui.ctx(), editor, state);
        sql_tab.cursor = byte_offset(&sql_tab.text, at.index.0);
        ui.memory_mut(|memory| memory.request_focus(editor));
        edited.focused = true;
    }
    let (cursor_line, _) = sql_tab.line_col();
    let error_line = sql_tab.error_mark().map(|(line, _)| line);
    // The statement Run executes, from the tokens the colours came from.
    let parsed = parsed(ui.ctx(), editor, dialect, &palette, &sql_tab.text);
    let statement = sql::statement_at(&parsed.statements, sql_tab.cursor)
        .and_then(|statement| edited.span(&statement_lines(statement)));
    if edited.focused && tree_has_arrows {
        app.actions
            .push(Action::SqlEditorFocused { tab, sql_tab: id });
    }
    let across = |x: egui::Rangef, y| Rect::from_x_y_ranges(x, y);
    let fill = |rect, color| Shape::rect_filled(rect, CornerRadius::ZERO, color);
    let mut behind = Vec::new();
    // macOS: a faint band behind the statement, the gutter included, and a
    // bar beside it. The terminal: the bar alone, in the accent.
    if let Some(y) = statement
        && !look.terminal
    {
        behind.push(fill(across(pane.x_range(), y), palette.panel));
    }
    // The cursor's line, over the band, while the editor has the keys.
    if edited.focused
        && let Some(y) = edited.span(&(cursor_line..=cursor_line))
    {
        let color = if look.terminal {
            palette.selection
        } else {
            palette.window.lerp_to_gamma(palette.selection, 0.45)
        };
        behind.push(fill(across(pane.x_range(), y), color));
    }
    if let Some(y) = statement {
        let bar = egui::Rangef::new(pane.left() + gutter.bar.min, pane.left() + gutter.bar.max);
        behind.push(fill(across(bar, y), bar_color(&look, &palette)));
    }
    ui.painter().set(backdrop, Shape::Vec(behind));
    // The numbers of the rows in sight.
    let clip = ui.clip_rect();
    let rows = &edited.galley.rows;
    let first = rows.partition_point(|row| edited.origin.y + row.max_y() < clip.top());
    for (index, row) in rows.iter().enumerate().skip(first) {
        let top = edited.origin.y + row.min_y();
        if top > clip.bottom() {
            break;
        }
        let line = index + 1;
        let color = if error_line == Some(line) {
            palette.danger
        } else if line == cursor_line {
            // The terminal's one absolute number stands out.
            if look.terminal {
                palette.warning
            } else {
                palette.text
            }
        } else if look.terminal {
            palette.dim
        } else {
            palette.faint
        };
        let number = gutter_number(line, cursor_line, look.terminal);
        widgets::paint_text_right(
            ui,
            pane.left() + gutter.numbers_right,
            top + row.height() / 2.0,
            Text::one(&look, role(&look), &number.to_string(), color),
        );
    }
}

/// What the text field is, and what it draws with.
struct Field<'a> {
    id: Id,
    /// Its accessible name.
    name: &'a str,
    /// The padding before the text.
    left: i8,
    dialect: Dialect,
    look: &'a Look,
    palette: &'a Palette,
}

/// The text field itself: edits the tab's text and reports its cursor.
fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
    let focus = std::mem::take(&mut sql_tab.focus_editor);
    // Before the field is drawn, so this frame shows the formatted text.
    if std::mem::take(&mut sql_tab.format) {
        format(ui, sql_tab, field);
    }
    // The field fills the pane, so a click under the last line lands in
    // it: whole lines, and the rest as bottom padding. (egui sizes a text
    // edit in lines of its font.)
    let font = role(field.look).font_id(field.look.faces);
    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let (top, least_bottom) = (8.0, 8.0);
    let room = (ui.available_height() - top - least_bottom).max(line);
    let rows = (room / line).floor();
    let bottom = (least_bottom + room - rows * line).floor();
    let margin = Margin {
        left: field.left,
        right: 16,
        top: top as i8,
        bottom: bottom as i8,
    };
    let mut layouter = layouter(field.look, field.palette, field.dialect, field.id);
    let output = egui::TextEdit::multiline(&mut sql_tab.text)
        .id(field.id)
        .font(font)
        // Tab indents; Esc gives the keys up.
        .lock_focus(true)
        .frame(egui::Frame::new().inner_margin(margin))
        .desired_width(f32::INFINITY)
        .desired_rows(rows as usize)
        .layouter(&mut layouter)
        .show(ui);
    // The name only: the field keeps the role and the value egui gave it.
    ui.ctx().accesskit_node_builder(field.id, |node| {
        node.set_label(field.name);
    });
    if focus {
        output.response.request_focus();
        // The keys arrive with the next frame: ask for it, since no event
        // need follow the one that opened the tab.
        ui.ctx().request_repaint();
    }
    // egui counts the cursor in characters; a field never focused has
    // none, and the tab keeps the one it has. The footer and the status
    // line, drawn before the editor, say where the cursor is a frame later
    // (egui draws one after any event).
    if let Some(range) = output.state.cursor.range(&output.galley) {
        sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index.0);
    }
    Edited {
        focused: output.response.has_focus(),
        origin: output.galley_pos,
        galley: output.galley,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_token_kind_takes_its_palette_colour() {
        for palette in [Palette::light(), Palette::dark()] {
            assert_eq!(color_of(TokenKind::Keyword, &palette), palette.magenta);
            assert_eq!(color_of(TokenKind::String, &palette), palette.success);
            assert_eq!(color_of(TokenKind::Number, &palette), palette.orange);
            assert_eq!(color_of(TokenKind::Comment, &palette), palette.dim);
            for plain in [
                TokenKind::Identifier,
                TokenKind::QuotedIdentifier,
                TokenKind::Operator,
                TokenKind::Punctuation,
                TokenKind::Semicolon,
                TokenKind::Whitespace,
                // MySQL runs it: it is not dimmed as a comment is.
                TokenKind::ExecutableComment,
            ] {
                assert_eq!(color_of(plain, &palette), palette.text, "{plain:?}");
            }
        }
    }

    #[test]
    fn a_script_is_coloured_in_runs_that_cover_it() {
        let palette = Palette::light();
        let text = "SELECT name, 42 FROM books -- all\nWHERE kind = 'é'";
        let runs = runs(&sql::tokenize(Dialect::Postgres, text), text, &palette);
        let pieces: Vec<(&str, Color32)> = runs
            .iter()
            .map(|(range, color)| (&text[range.clone()], *color))
            .collect();
        assert_eq!(
            pieces,
            [
                ("SELECT", palette.magenta),
                // Names, punctuation and the spaces between are one run.
                (" name, ", palette.text),
                ("42", palette.orange),
                (" ", palette.text),
                ("FROM", palette.magenta),
                (" books ", palette.text),
                ("-- all", palette.dim),
                ("\n", palette.text),
                ("WHERE", palette.magenta),
                (" kind = ", palette.text),
                ("'é'", palette.success),
            ]
        );
        // Nothing is left out, whatever the text.
        for text in ["", " ", "'unterminated", "/* open", "ż ó\r\n\tł;;"] {
            let tokens = sql::tokenize(Dialect::MySql, text);
            let runs = super::runs(&tokens, text, &palette);
            let joined: String = runs.iter().map(|(range, _)| &text[range.clone()]).collect();
            assert_eq!(joined, text);
        }
    }

    #[test]
    fn the_layouter_colours_tokens_and_keeps_a_row_for_each_line() {
        let (look, palette) = (Look::standard(), Palette::light());
        let long = format!("SELECT {}\n-- done", "a_column, ".repeat(400));
        let mut galleys = Vec::new();
        let mut harness = crate::testing::Harness::new();
        harness.frame_with(|ui| {
            let id = Id::new("an editor");
            let mut layouter = layouter(&look, &palette, Dialect::Postgres, id);
            // A width to wrap at is not taken: lines scroll instead.
            galleys.push(layouter(ui, &long, 100.0));
            galleys.push(layouter(ui, &String::new(), 100.0));
        });
        let galley = &galleys[0];
        assert_eq!(galley.rows.len(), 2);
        let sections = &galley.job.sections;
        let keyword = &sections[0].byte_range;
        assert_eq!((keyword.start.0, keyword.end.0), (0, 6));
        assert_eq!(sections[0].format.color, palette.magenta);
        assert_eq!(sections.last().unwrap().format.color, palette.dim);
        // Every run is the code role's, whatever its colour.
        let role = role(&look);
        let format = role.format(look.faces, palette.magenta);
        assert_eq!(sections[0].format, format);
        for section in sections {
            assert_eq!(section.format.font_id, format.font_id);
            assert_eq!(section.format.line_height, role.line_height());
        }
        // An empty script is one line, as tall as any other.
        let empty = &galleys[1];
        assert_eq!(empty.rows.len(), 1);
        assert_eq!(empty.rows[0].height(), galley.rows[0].height());
    }

    #[test]
    fn a_byte_offset_becomes_a_character_index() {
        assert_eq!(char_index("SELECT 1", 0), 0);
        assert_eq!(char_index("SELECT 1", 8), 8);
        // Two bytes each.
        assert_eq!(char_index("żółw 1", 7), 4);
        assert_eq!(char_index("żółw 1", 9), 6);
        // There and back.
        for chars in 0..=6 {
            assert_eq!(char_index("żółw 1", byte_offset("żółw 1", chars)), chars);
        }
        // Past the end: the end.
        assert_eq!(char_index("żółw", 99), 4);
    }

    #[test]
    fn a_character_index_becomes_a_byte_offset() {
        assert_eq!(byte_offset("SELECT 1", 0), 0);
        assert_eq!(byte_offset("SELECT 1", 8), 8);
        // Two bytes each.
        assert_eq!(byte_offset("żółw 1", 4), 7);
        assert_eq!(byte_offset("żółw 1", 6), 9);
        // A CRLF line ending is two characters and two bytes.
        assert_eq!(byte_offset("é\r\nx", 3), 4);
        // Past the end (the text shrank under the cursor): the end.
        assert_eq!(byte_offset("żółw", 9), 7);
        assert_eq!(byte_offset("", 3), 0);
    }

    #[test]
    fn the_terminal_gutter_is_relative_to_the_cursors_line() {
        let numbers = |cursor, relative| -> Vec<usize> {
            (1..=5)
                .map(|line| gutter_number(line, cursor, relative))
                .collect()
        };
        assert_eq!(numbers(3, false), [1, 2, 3, 4, 5]);
        assert_eq!(numbers(3, true), [2, 1, 3, 1, 2]);
        assert_eq!(numbers(1, true), [1, 1, 2, 3, 4]);
        assert_eq!(numbers(5, true), [4, 3, 2, 1, 5]);
    }

    #[test]
    fn the_band_covers_a_statements_sql_not_the_comment_above_it() {
        let script =
            "SELECT 1;\r\n\r\n-- the count\r\nSELECT count(*)\r\n  FROM books\r\n;\nSELECT 2";
        let statements = sql::statements(Dialect::Postgres, script);
        let lines: Vec<_> = statements.iter().map(statement_lines).collect();
        assert_eq!(lines, [1..=1, 4..=5, 7..=7]);
    }
}
