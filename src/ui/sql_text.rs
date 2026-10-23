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
use crate::i18n::{Locale, gettext};
use crate::model::{Action, Completion, ConnTabId, Pane, SqlTab, TabId, TextPrint};
use crate::settings::Indent;
use crate::theme::{Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::widgets;
use crate::vim;

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
pub(crate) struct Parsed {
    /// What it was worked out from: the script, the dialect that reads it
    /// and the colours.
    of: (TextPrint, Dialect, Palette),
    runs: Vec<(Range<usize>, Color32)>,
    statements: Vec<Statement>,
    /// The script's tokens, for the completion list.
    pub(crate) tokens: Vec<Token>,
}

/// `text` parsed, from egui's memory for the editor `id` when that is of
/// the same text, dialect and colours: a frame that changes none of them
/// tokenizes nothing.
pub(crate) fn parsed(
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
        tokens,
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
/// worked out from the script, where it was scrolled to, and what its
/// completion list kept.
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
    super::sql_complete::forget(ctx, editor);
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
        ("list", super::sql_complete::remembered(ctx, editor)),
    ]
    .into_iter()
    .filter_map(|(name, kept)| kept.then_some(name))
    .collect()
}

/// How far the editor `id` is scrolled.
#[cfg(test)]
pub fn scroll_offset(ctx: &egui::Context, tab: ConnTabId, id: TabId) -> Option<egui::Vec2> {
    let ScrollId(area) = ctx.data(|data| data.get_temp(editor_id(tab, id)))?;
    egui::scroll_area::State::load(ctx, area).map(|state| state.offset)
}

/// A text edit layouter for the editor `id` that colours `dialect`'s
/// tokens. It wraps a long line at the width it is given where `wraps`
/// (Settings › Editor's Wrap), and otherwise never: one galley row per
/// line, and the lines scroll.
pub fn layouter(
    look: &Look,
    palette: &Palette,
    dialect: Dialect,
    id: Id,
    wraps: bool,
) -> impl FnMut(&Ui, &dyn egui::TextBuffer, f32) -> Arc<Galley> + use<> {
    let (faces, role, palette) = (look.faces, role(look), *palette);
    move |ui, buffer, width| {
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
        if wraps && width.is_finite() {
            laid = laid.wrap(width);
        }
        laid.layout(ui.ctx()).galley
    }
}

/// `text` in `dialect`'s colours, as an editor draws it, wrapped at
/// `width`: a statement shown and not edited.
pub fn highlighted(
    ctx: &egui::Context,
    look: &Look,
    palette: &Palette,
    dialect: Dialect,
    text: &str,
    width: f32,
) -> crate::typography::Laid {
    let tokens = sql::tokenize(dialect, text);
    let runs = runs(&tokens, text, palette);
    Text::in_faces(look.faces)
        .add_runs(
            role(look),
            runs.iter()
                .map(|(range, color)| (&text[range.clone()], *color)),
        )
        .wrap(width)
        .layout(ctx)
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

/// The character index, as egui counts a cursor, of the byte offset `byte`
/// of `text`.
fn char_index(text: &str, byte: usize) -> usize {
    text.get(..byte)
        .map_or_else(|| text.chars().count(), |before| before.chars().count())
}

/// A field's cursor and script, as its undo history keeps them.
type Edit = (CCursorRange, String);

/// Puts an edit made from outside the field into its undo history as a
/// step of its own, and the field's cursor where the edit left it:
/// `before` and `after` are the cursor and the script on either side of
/// the edit, and `time` is the frame's.
///
/// egui groups undo by time and would merge the edit with the typing
/// around it, so both go in as undo points: undo gives the first back
/// whole, and redo the second. Between the two the edited script is fed
/// as an edit of the field's own is, which empties what an earlier undo
/// left to redo: adding an undo point alone would keep it. (The time fed
/// does not matter: the undo point after it settles the state.)
fn outside_edit(
    state: &mut egui::text_edit::TextEditState,
    time: f64,
    before: &Edit,
    after: &Edit,
) {
    let mut undoer = state.undoer();
    undoer.add_undo(before);
    undoer.feed_state(time, after);
    undoer.add_undo(after);
    state.set_undoer(undoer);
    state.cursor.set_char_range(Some(after.0));
}

/// Carries out an accepted completion before the field handles the
/// frame's events: replaces the word with the highlighted row's text and
/// puts the cursor after it. The list is taken off the tab either way.
/// Returns whether a row went in.
///
/// The insertion is an undo step of its own (see `outside_edit`): undo
/// takes back what was typed after the insertion, then the insertion
/// alone, and redo puts each back. As after any edit, nothing an earlier
/// undo took back is left to redo.
fn insert_completion(ctx: &egui::Context, id: Id, sql_tab: &mut SqlTab) -> bool {
    let Some(list) = sql_tab.completion.take_if(|list| list.accept) else {
        return false;
    };
    let Some(candidate) = list.highlighted() else {
        return false;
    };
    let word = list.site.word.clone();
    // The list must be of this very script, and the row must change it.
    let changes = sql_tab
        .text
        .get(word.clone())
        .is_some_and(|old| old != candidate.insert);
    if !list.is_of_text(&sql_tab.text) || !changes {
        return false;
    }
    let mut state = egui::TextEdit::load_state(ctx, id).unwrap_or_default();
    let before = state.cursor.char_range().unwrap_or_else(|| {
        CCursorRange::one(CCursor::new(char_index(&sql_tab.text, sql_tab.cursor)))
    });
    let before = (before, sql_tab.text.clone());
    sql_tab.text.replace_range(word.clone(), &candidate.insert);
    sql_tab.cursor = word.start + candidate.insert.len();
    let after = CCursorRange::one(CCursor::new(char_index(&sql_tab.text, sql_tab.cursor)));
    let after = (after, sql_tab.text.clone());
    outside_edit(&mut state, ctx.input(|input| input.time), &before, &after);
    egui::TextEdit::store_state(ctx, id, state);
    true
}

/// Whether `event` is text typed into the field, as it takes it: it
/// ignores an empty text and a line ending (Enter is a key), and an empty
/// commit of an input method.
pub(crate) fn is_typed(event: &egui::Event) -> bool {
    match event {
        egui::Event::Text(text) => !text.is_empty() && text != "\n" && text != "\r",
        egui::Event::Ime(egui::ImeEvent::Commit(text)) => !text.is_empty(),
        _ => false,
    }
}

/// Formats the script, or the statements the field's selection overlaps,
/// as one step of the field's undo history. Returns whether it changed
/// the text: it does not when Format has nothing to change.
fn format_script(ui: &Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
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
    let formatted =
        sql::format::format_with(field.dialect, &sql_tab.text, selection, cursor, field.style);
    // Vim's `=` selects the lines it formats: its cursor, kept, is what
    // stays, not the lines.
    let vim_cursor: Option<usize> =
        ui.data_mut(|data| data.remove_temp(field.id.with("vim format")));
    let Some(formatted) = formatted else {
        if let Some(at) = vim_cursor {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
            egui::TextEdit::store_state(ui.ctx(), field.id, state);
            sql_tab.selected = None;
        }
        return false;
    };
    let at = |text: &str, byte: usize| CCursorRange::one(CCursor::new(char_index(text, byte)));
    let before = match vim_cursor {
        Some(cursor) => CCursorRange::one(CCursor::new(cursor)),
        None => typed.unwrap_or_else(|| at(&sql_tab.text, sql_tab.cursor)),
    };
    let after = at(&formatted.text, formatted.cursor);
    // The text as typed and the text as formatted, both: undo gives the
    // first back whole, and redo the second.
    let before = (before, std::mem::take(&mut sql_tab.text));
    let after = (after, formatted.text);
    outside_edit(&mut state, ui.input(|input| input.time), &before, &after);
    egui::TextEdit::store_state(ui.ctx(), field.id, state);
    sql_tab.text = after.1;
    sql_tab.cursor = formatted.cursor;
    true
}

/// The number the gutter shows for `line`: its own, or (relative numbers)
/// how far it is from the cursor's line, which keeps its own.
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
    /// with the digits of `lines`. Without numbers (`numbered` false), the
    /// bar is that 12 or 10 in.
    fn new(ui: &Ui, look: &Look, lines: usize, numbered: bool) -> Self {
        let (column, pad, bar, text_left) = if look.terminal {
            (48.0, 10.0, 3.0, 12)
        } else {
            (52.0, 12.0, 4.0, 14)
        };
        let digits = lines.max(1).ilog10() as usize + 1;
        let digit = role(look).width(ui.ctx(), look.faces, "0");
        let column = if numbered {
            (digits as f32 * digit + pad + 8.0).ceil().max(column)
        } else {
            pad
        };
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
    /// The caret on screen, as tall as its line. A field never focused
    /// has none.
    caret: Option<Rect>,
    focused: bool,
    /// The field's text changed this frame by typing: not by a paste, an
    /// undo, a deletion or an input method's text while it is composed.
    /// Only text the field takes counts (see `edit`).
    typed: bool,
    /// `:` typed in vim's normal or visual mode.
    prompt: bool,
    /// The field's own.
    response: egui::Response,
}

impl Edited {
    /// The screen's vertical range of the 1-based `lines`, each with every
    /// row it wraps to.
    /// `starts` is `line_rows` of the galley.
    fn span(&self, starts: &[usize], lines: &RangeInclusive<usize>) -> Option<egui::Rangef> {
        let first = *starts.get(lines.start().checked_sub(1)?)?;
        // The row before the next line's first, or the galley's last.
        let last = match starts.get(*lines.end()) {
            Some(next) => next.checked_sub(1)?,
            None if *lines.end() == starts.len() => self.galley.rows.len().checked_sub(1)?,
            None => return None,
        };
        let (first, last) = (self.galley.rows.get(first)?, self.galley.rows.get(last)?);
        Some(egui::Rangef::new(
            self.origin.y + first.min_y(),
            self.origin.y + last.max_y(),
        ))
    }
}

/// The first row of each line of `galley`, in order: a line wrapped at
/// the editor's width is more than one row.
fn line_rows(galley: &Galley) -> Vec<usize> {
    let mut starts = vec![0];
    for (index, row) in galley.rows.iter().enumerate() {
        if row.ends_with_newline && index + 1 < galley.rows.len() {
            starts.push(index + 1);
        }
    }
    starts
}

/// How the editor draws, as the settings and the look have them.
struct Shown {
    look: Look,
    palette: Palette,
    locale: Locale,
    numbering: crate::settings::LineNumbers,
    style: sql::format::Style,
    indent: Indent,
    wraps: bool,
    highlight: bool,
    pairs: bool,
    vim: bool,
}

impl Shown {
    fn of(
        settings: &crate::settings::Settings,
        look: Look,
        palette: Palette,
        locale: Locale,
    ) -> Self {
        Self {
            numbering: settings.line_numbers_in(look.terminal),
            style: format_style(settings, &look),
            indent: settings.indent,
            wraps: settings.wrap,
            highlight: settings.highlight_line,
            pairs: settings.close_pairs,
            vim: settings.editor_keys_in(look.terminal) == crate::settings::EditorKeys::Vim,
            look,
            palette,
            locale,
        }
    }

    /// The text field named `name` beside `gutter`: one that edits, or
    /// with `read_only` the preview's.
    fn field<'a>(
        &'a self,
        (id, name): (Id, &'a str),
        gutter: &Gutter,
        dialect: Dialect,
        hold_escape: bool,
        read_only: bool,
    ) -> Field<'a> {
        Field {
            id,
            name,
            left: gutter.text_left,
            dialect,
            look: &self.look,
            palette: &self.palette,
            style: self.style,
            indent: self.indent,
            wraps: self.wraps,
            hold_escape,
            pairs: self.pairs,
            vim: self.vim,
            read_only,
        }
    }
}

/// Where the editor draws: its pane, the text's part of it beside the
/// gutter, and the slot behind the text the bands go in.
struct Place {
    pane: Rect,
    text_pane: Rect,
    backdrop: egui::layers::ShapeIdx,
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let shown = Shown::of(&app.settings, app.look, app.palette, app.locale);
    let locale = shown.locale;
    let Some(workspace) = app.workspace_mut(tab) else {
        return;
    };
    let dialect = workspace.driver.dialect();
    let tree_has_arrows = workspace.pane == Pane::Tree;
    let Some(sql_tab) = workspace.sql_tab_mut(id) else {
        return;
    };
    let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
    let gutter = Gutter::new(ui, &shown.look, lines, shown.numbering.shown());
    let editor = editor_id(tab, id);
    let name = gettext(locale, "SQL");
    // An open completion list takes Esc: the editor keeps the keyboard.
    // Not one whose row this frame inserts: Esc is the editor's again.
    let hold_escape = sql_tab.completion.as_ref().is_some_and(|list| !list.accept);
    let field = shown.field((editor, &name), &gutter, dialect, hold_escape, false);
    let (mut edited, place) = draw_text(ui, sql_tab, (&field, &gutter), locale);
    // The open completion list, and the characters its word starts and
    // ends at. (After the field drew: a list whose row was accepted went
    // in there and is off the tab, so the one here is open.)
    let completing = sql_tab.completion.as_ref().map(|list| {
        let word = &list.site.word;
        let start = char_index(&sql_tab.text, word.start);
        let end = char_index(&sql_tab.text, word.end);
        (list.clone(), start..end)
    });
    let (cursor_line, _) = sql_tab.line_col();
    let error_line = sql_tab.error_mark().map(|(line, _)| line);
    // The statement Run executes, from the tokens the colours came from.
    let parsed = parsed(ui.ctx(), editor, dialect, &shown.palette, &sql_tab.text);
    // The first row of each line: a wrapped line has more than one.
    let starts = line_rows(&edited.galley);
    let statement = sql::statement_at(&parsed.statements, sql_tab.cursor)
        .and_then(|statement| edited.span(&starts, &statement_lines(statement)));
    if edited.focused && tree_has_arrows {
        app.actions
            .push(Action::SqlEditorFocused { tab, sql_tab: id });
    }
    if edited.typed {
        app.actions.push(Action::SqlTyped { tab, sql_tab: id });
    }
    // Vim's `:`: the prompt the terminal look has. (The macOS look has
    // none.)
    if edited.prompt && shown.look.terminal {
        app.actions.push(Action::EditorCommand { tab, sql_tab: id });
    }
    let spot = Spot {
        tab,
        sql_tab: id,
        text_pane: place.text_pane,
        locale,
    };
    let asked = completion_list(ui, completing.as_ref(), &mut edited, &field, &spot);
    app.actions.extend(asked);
    let lines = Lines {
        starts: &starts,
        cursor: cursor_line,
        error: error_line,
        statement,
    };
    draw_lines(ui, &shown, &gutter, &place, &edited, &lines, false);
}

/// Settings › Editor's preview: `sql_tab`'s script in the editor, as the
/// settings have it now, read-only and named "Editor preview". Its cursor's
/// line is lit whether or not it has the keyboard, and it has no statement
/// bar and no gutter to press: one Tab stop. Nothing of it reaches a
/// database. Returns how tall the field is that shows all of the script
/// (the preview's height from the next frame on), and the field's
/// response, for the ring that shows it has the keyboard.
pub fn preview(
    ui: &mut Ui,
    sql_tab: &mut SqlTab,
    id: Id,
    settings: &crate::settings::Settings,
    (look, palette, locale): (&Look, &Palette, Locale),
) -> (f32, egui::Response) {
    let shown = Shown::of(settings, *look, *palette, locale);
    let lines = sql_tab.text.bytes().filter(|b| *b == b'\n').count() + 1;
    let gutter = Gutter::new(ui, &shown.look, lines, shown.numbering.shown());
    let name = gettext(locale, "Editor preview");
    let field = shown.field((id, &name), &gutter, Dialect::Postgres, false, true);
    let (edited, place) = draw_text(ui, sql_tab, (&field, &gutter), locale);
    let starts = line_rows(&edited.galley);
    let (cursor_line, _) = sql_tab.line_col();
    let lines = Lines {
        starts: &starts,
        cursor: cursor_line,
        error: None,
        statement: None,
    };
    draw_lines(ui, &shown, &gutter, &place, &edited, &lines, true);
    // The rows as laid out, with the field's 8 above and under them.
    (edited.galley.rect.height() + 16.0, edited.response)
}

/// How tall the preview is that shows all of `text` before it has drawn:
/// its lines in the field's font, with the field's 8 above and under them.
pub fn preview_height(ui: &Ui, look: &Look, text: &str) -> f32 {
    let font = role(look).font_id(look.faces);
    let line = ui.fonts_mut(|fonts| fonts.row_height(&font)) + ui.spacing().extra_text_line_spacing;
    let lines = text.bytes().filter(|b| *b == b'\n').count() + 1;
    lines as f32 * line + 16.0
}

/// The editor's text field, scrolling beside the gutter, and the gutter's
/// press (not the preview's).
fn draw_text(
    ui: &mut Ui,
    sql_tab: &mut SqlTab,
    (field, gutter): (&Field<'_>, &Gutter),
    locale: Locale,
) -> (Edited, Place) {
    let pane = ui.max_rect();
    let editor = field.id;
    // Behind the text: filled in once the rows are laid out.
    let backdrop = ui.painter().add(Shape::Noop);
    // The text scrolls beside the gutter, which stays put: both ways, or
    // down alone where it wraps.
    let text_pane = Rect::from_min_max(pos2(pane.left() + gutter.width, pane.top()), pane.max);
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt("text").max_rect(text_pane));
    child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
    // Wrapped, the lines never run past the pane: it scrolls down alone.
    let scrolled = egui::ScrollArea::new([!field.wraps, true])
        .id_salt("scroll")
        .auto_shrink([false, false])
        .show(&mut child, |ui| edit(ui, sql_tab, field));
    ui.data_mut(|data| data.insert_temp(editor, ScrollId(scrolled.id)));
    let mut edited = scrolled.inner;
    let place = Place {
        pane,
        text_pane,
        backdrop,
    };
    if field.read_only {
        return (edited, place);
    }
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
        let y = pointer.y - edited.origin.y;
        let row = edited.galley.cursor_from_pos(vec2(0.0, y));
        // Beside a row, the start of its line, not of a row the line wraps
        // to. Under the last, the end of the script.
        let at = if y <= edited.galley.rect.max.y {
            let byte = byte_offset(&sql_tab.text, row.index.0);
            let start = sql_tab.text[..byte].rfind('\n').map_or(0, |at| at + 1);
            CCursor::new(char_index(&sql_tab.text, start))
        } else {
            row
        };
        let mut state = egui::TextEdit::load_state(ui.ctx(), editor).unwrap_or_default();
        state.cursor.set_char_range(Some(CCursorRange::one(at)));
        egui::TextEdit::store_state(ui.ctx(), editor, state);
        sql_tab.cursor = byte_offset(&sql_tab.text, at.index.0);
        // A cursor put there ends vim's visual selection.
        if sql_tab.vim.mode.is_visual() {
            sql_tab.vim.to_normal();
        }
        ui.memory_mut(|memory| memory.request_focus(editor));
        edited.focused = true;
    }
    (edited, place)
}

/// What the gutter and the bands mark: each line's first row, the
/// cursor's line, the line of the last run's error, and the rows of the
/// statement Run executes.
struct Lines<'a> {
    starts: &'a [usize],
    cursor: usize,
    error: Option<usize>,
    statement: Option<egui::Rangef>,
}

/// Behind the text, the statement's band and bar and the cursor's line;
/// in the gutter, the numbers of the rows in sight. With `always`, the
/// cursor's line is lit without the keyboard too (the preview).
fn draw_lines(
    ui: &Ui,
    shown: &Shown,
    gutter: &Gutter,
    place: &Place,
    edited: &Edited,
    lines: &Lines<'_>,
    always: bool,
) {
    let (look, palette, pane) = (&shown.look, &shown.palette, place.pane);
    let across = |x: egui::Rangef, y| Rect::from_x_y_ranges(x, y);
    let fill = |rect, color| Shape::rect_filled(rect, CornerRadius::ZERO, color);
    let mut behind = Vec::new();
    // macOS: a faint band behind the statement, the gutter included, and a
    // bar beside it. The terminal: the bar alone, in the accent.
    if let Some(y) = lines.statement
        && !look.terminal
    {
        behind.push(fill(across(pane.x_range(), y), palette.panel));
    }
    // The cursor's line, over the band, while the editor has the keys,
    // unless the settings turn it off.
    if (edited.focused || always)
        && shown.highlight
        && let Some(y) = edited.span(lines.starts, &(lines.cursor..=lines.cursor))
    {
        let color = if look.terminal {
            palette.selection
        } else {
            palette.window.lerp_to_gamma(palette.selection, 0.45)
        };
        behind.push(fill(across(pane.x_range(), y), color));
    }
    if let Some(y) = lines.statement {
        let bar = egui::Rangef::new(pane.left() + gutter.bar.min, pane.left() + gutter.bar.max);
        behind.push(fill(across(bar, y), bar_color(look, palette)));
    }
    ui.painter().set(place.backdrop, Shape::Vec(behind));
    // The numbers of the rows in sight, where the gutter has numbers.
    if !shown.numbering.shown() {
        return;
    }
    let clip = ui.clip_rect();
    let rows = &edited.galley.rows;
    // A line's number stands on its first row: the rows a long line wraps
    // to have none.
    let first = rows.partition_point(|row| edited.origin.y + row.max_y() < clip.top());
    for (index, row) in rows.iter().enumerate().skip(first) {
        let top = edited.origin.y + row.min_y();
        if top > clip.bottom() {
            break;
        }
        let Ok(line) = lines.starts.binary_search(&index) else {
            continue;
        };
        let line = line + 1;
        let color = if lines.error == Some(line) {
            palette.danger
        } else if line == lines.cursor {
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
        let relative = shown.numbering == crate::settings::LineNumbers::Relative;
        let number = gutter_number(line, lines.cursor, relative);
        widgets::paint_text_right(
            ui,
            pane.left() + gutter.numbers_right,
            top + row.height() / 2.0,
            Text::one(look, role(look), &number.to_string(), color),
        );
    }
}

/// Where an editor's completion list is drawn, and whose it is.
struct Spot {
    tab: ConnTabId,
    sql_tab: TabId,
    /// The text's pane: the editor without its gutter.
    text_pane: Rect,
    locale: Locale,
}

/// Draws the open completion list of an editor under its word:
/// `completing` is the list and the characters its word starts and ends
/// at, or nothing when no list is open. Returns what the frame asks of
/// the list: a row picked, or that it closes.
///
/// A press on the list hands the keyboard back to the editor, and
/// `edited` says so.
fn completion_list(
    ui: &Ui,
    completing: Option<&(Completion, Range<usize>)>,
    edited: &mut Edited,
    field: &Field<'_>,
    spot: &Spot,
) -> Option<Action> {
    let (tab, sql_tab, pane) = (spot.tab, spot.sql_tab, spot.text_pane);
    let Some((list, chars)) = completing else {
        super::sql_complete::forget(ui.ctx(), field.id);
        return None;
    };
    let close = Action::CloseCompletion { tab, sql_tab };
    // A character's place on screen, as tall as its line.
    // `next`: on the row after where a wrapped line breaks, as a word's
    // start is.
    let on_screen = |at: usize, next: bool| {
        let mut cursor = CCursor::new(at);
        cursor.prefer_next_row = next;
        let place = edited.galley.pos_from_cursor(cursor);
        place.translate(edited.origin.to_vec2())
    };
    let (word, end) = (on_screen(chars.start, true), on_screen(chars.end, false));
    // The list hangs under its word while some of the word is in the
    // pane: its line is, and across the word starts before the pane's
    // right edge and ends after its left one.
    let in_sight = pane.y_range().contains(word.center().y)
        && word.left() <= pane.right()
        && end.right() >= pane.left();
    if !in_sight {
        // Scrolled out of the pane, the list has nothing to hang under.
        // One that has not drawn yet waits instead, while the editor
        // scrolls its caret into view (see `edit`) and no longer: with the
        // caret in the pane and still no place for the list, it closes,
        // rather than stay open with its keys live and nothing to see.
        let drawn = super::sql_complete::was_shown(ui.ctx(), field.id, list);
        let caret_on_its_way = edited
            .caret
            .is_some_and(|caret| !pane.contains(caret.center()));
        let waits = !drawn && caret_on_its_way && edited.focused;
        return (!waits).then_some(close);
    }
    // A start left of the pane (the line is scrolled, and the pane shows
    // the word from further along): the list is drawn from the pane's
    // left edge.
    let word = word.translate(vec2((pane.left() - word.left()).max(0.0), 0.0));
    let (look, palette) = (field.look, field.palette);
    let shown = super::sql_complete::show(ui, list, word, field.id, look, palette, spot.locale);
    // A press on the list lands outside the field, which gives the keys
    // up: they stay the editor's. Only once it lost them: asking for the
    // keyboard again lets go of the keys the field holds (Tab, the arrows,
    // Esc) for a frame.
    if shown.pressed && !edited.focused {
        ui.memory_mut(|memory| memory.request_focus(field.id));
        edited.focused = true;
    }
    if let Some(row) = shown.picked {
        let row = Some(row);
        return Some(Action::AcceptCompletion { tab, sql_tab, row });
    }
    // Checked after the list drew, so a press on a row does not count as
    // the editor losing the keyboard.
    (!edited.focused).then_some(close)
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
    /// Keep the keyboard on Esc (a completion list is open and takes it).
    hold_escape: bool,
    /// A bracket or a quote typed gets its closing one.
    pairs: bool,
    /// How Format writes the script.
    style: sql::format::Style,
    /// What the Tab key puts in, and takes out with Shift.
    indent: Indent,
    /// Long lines wrap at the pane's width.
    wraps: bool,
    /// The keybindings are vim's: the field has normal, insert and visual
    /// modes.
    vim: bool,
    /// The preview's: the text cannot change, and Tab moves on.
    read_only: bool,
}

/// How Format writes, as the settings have it in `look`: its keyword case
/// and commas, and a `CASE`'s parts in by the indent (four spaces for a
/// tab: the river is set in spaces).
pub(crate) fn format_style(
    settings: &crate::settings::Settings,
    look: &Look,
) -> sql::format::Style {
    use crate::settings::{Commas, KeywordCase};
    use sql::format::Case;
    sql::format::Style {
        keywords: match settings.keywords_in(look.terminal) {
            KeywordCase::Upper => Case::Upper,
            KeywordCase::Lower => Case::Lower,
            KeywordCase::Keep => Case::Keep,
        },
        commas: match settings.commas {
            Commas::Trailing => sql::format::Commas::Trailing,
            Commas::Leading => sql::format::Commas::Leading,
        },
        case_indent: indent_width(settings.indent),
    }
}

/// What an indent puts in.
fn indent_unit(indent: Indent) -> &'static str {
    match indent {
        Indent::Two => "  ",
        Indent::Four => "    ",
        Indent::Tab => "\t",
    }
}

/// How many columns an indent is: a tab counts four.
fn indent_width(indent: Indent) -> usize {
    match indent {
        Indent::Two => 2,
        Indent::Four | Indent::Tab => 4,
    }
}

/// `text` with an indent put in or taken out (`outdent`), and the bytes
/// `selection` covers after it. Without a selection an indent goes in at
/// the cursor (`selection`'s end); with one, at the start of every line it
/// touches but blank ones; an outdent takes one indent (a tab, or up to
/// an indent's width of spaces) off the start of every line the selection
/// or the cursor touches. A line the selection only reaches the start of
/// is not touched. `None` when nothing changes.
fn indented(
    text: &str,
    selection: Range<usize>,
    indent: Indent,
    outdent: bool,
) -> Option<(String, Range<usize>)> {
    let unit = indent_unit(indent);
    if !outdent && selection.is_empty() {
        let at = selection.end;
        let mut out = text.to_owned();
        out.insert_str(at, unit);
        let to = at + unit.len();
        return Some((out, to..to));
    }
    // The starts of the lines touched, in order.
    let first = text[..selection.start].rfind('\n').map_or(0, |at| at + 1);
    let mut starts = vec![first];
    for (at, byte) in text.bytes().enumerate().skip(first) {
        let next = at + 1;
        let touched = next < selection.end;
        if byte == b'\n' && touched && next < text.len() {
            starts.push(next);
        } else if next >= selection.end {
            break;
        }
    }
    // Each line's change: where, what goes out, what comes in.
    let mut edits: Vec<(usize, usize, &str)> = Vec::new();
    for start in starts {
        let line = &text[start
            ..text[start..]
                .find('\n')
                .map_or(text.len(), |end| start + end)];
        if outdent {
            let cut = if line.starts_with('\t') {
                1
            } else {
                line.bytes()
                    .take(indent_width(indent))
                    .take_while(|byte| *byte == b' ')
                    .count()
            };
            if cut > 0 {
                edits.push((start, cut, ""));
            }
        } else if !line.trim().is_empty() {
            edits.push((start, 0, unit));
        }
    }
    if edits.is_empty() {
        return None;
    }
    // Where an offset of the old text lands in the new one.
    let moved = |offset: usize| -> usize {
        let mut to = offset;
        for (at, cut, put) in &edits {
            if *at >= offset {
                break;
            }
            let gone = (*cut).min(offset - at);
            to = to - gone + put.len();
        }
        to
    };
    let mut out = String::with_capacity(text.len() + edits.len() * unit.len());
    let mut from = 0;
    for (at, cut, put) in &edits {
        out.push_str(&text[from..*at]);
        out.push_str(put);
        from = at + cut;
    }
    out.push_str(&text[from..]);
    Some((out, moved(selection.start)..moved(selection.end)))
}

/// Takes the frame's Tab and Shift+Tab, as the keymap has them in the
/// editor (`IndentLines`, `OutdentLines`), before the field reads its
/// events: Tab with nothing selected becomes the indent's text, typed in
/// its place among the frame's events; Tab over a selection and Shift+Tab
/// are made here on the lines (`indented`), each one step of the field's
/// undo history, the selection kept as it ran. Only while the field has
/// the keyboard: the completion list took its Tab before. Returns whether
/// the text changed here.
fn indent_script(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
    use crate::keymap::{Command, Layout, Scope};
    if !ui.memory(|memory| memory.has_focus(field.id)) {
        return false;
    }
    let keymap = crate::ui::keys::published(ui.ctx());
    let layout = Layout::of(field.look);
    let chords = |command| -> Vec<&'static str> {
        keymap
            .chords_now(layout, command, Scope::SqlEditor, false)
            .collect()
    };
    let (indents, outdents) = (chords(Command::IndentLines), chords(Command::OutdentLines));
    let unit = indent_unit(field.indent);
    let selected = egui::TextEdit::load_state(ui.ctx(), field.id)
        .and_then(|state| state.cursor.char_range())
        .is_some_and(|range| !range.is_empty());
    // The line edits asked for, in order: `true` to outdent.
    let mut lines = Vec::new();
    ui.input_mut(|input| {
        let held = input.modifiers;
        let pressed = |event: &egui::Event, chords: &[&'static str]| {
            chords
                .iter()
                .any(|keys| crate::ui::keys::press_of(layout, held, event, keys).is_some())
        };
        let events = std::mem::take(&mut input.events);
        input.events = events
            .into_iter()
            .filter_map(|event| {
                if pressed(&event, &outdents) {
                    lines.push(true);
                    None
                } else if pressed(&event, &indents) {
                    if selected || !lines.is_empty() {
                        lines.push(false);
                        None
                    } else {
                        Some(egui::Event::Text(unit.to_owned()))
                    }
                } else {
                    Some(event)
                }
            })
            .collect();
    });
    let mut changed = false;
    for outdent in lines {
        let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
        let range = state.cursor.char_range().unwrap_or_else(|| {
            CCursorRange::one(CCursor::new(char_index(&sql_tab.text, sql_tab.cursor)))
        });
        let [start, end] = range.sorted_cursors();
        let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
        let selection = byte(start)..byte(end);
        let Some((text, after)) = indented(&sql_tab.text, selection, field.indent, outdent) else {
            continue;
        };
        let at = |byte: usize| CCursor::new(char_index(&text, byte));
        // The end that moves stays the one that moved: a selection made
        // backwards is still one.
        let backwards = range.primary.index < range.secondary.index;
        let after_range = match (after.is_empty(), backwards) {
            (true, _) => CCursorRange::one(at(after.end)),
            (false, false) => CCursorRange::two(at(after.start), at(after.end)),
            (false, true) => CCursorRange::two(at(after.end), at(after.start)),
        };
        let cursor = if backwards { after.start } else { after.end };
        let before = (range, std::mem::take(&mut sql_tab.text));
        let after_edit = (after_range, text);
        outside_edit(
            &mut state,
            ui.input(|input| input.time),
            &before,
            &after_edit,
        );
        egui::TextEdit::store_state(ui.ctx(), field.id, state);
        sql_tab.text = after_edit.1;
        sql_tab.cursor = cursor;
        changed = true;
    }
    changed
}

/// The closing character of a bracket or a quote that `typed` opens in
/// `dialect`: a backtick only where it quotes names (MySQL, SQLite).
fn closer_of(typed: char, dialect: Dialect) -> Option<char> {
    match typed {
        '(' => Some(')'),
        '[' => Some(']'),
        '{' => Some('}'),
        '\'' | '"' => Some(typed),
        '`' => dialect.backtick_names().then_some('`'),
        _ => None,
    }
}

/// Whether the quote `typed` at byte `at` of `text` would close the
/// string or the quoted name the cursor is in: the literal's own last
/// character, its closing quote, is right after the cursor. A quote of an
/// escaped pair, or one inside a literal of another kind, closes nothing.
fn closes_literal(text: &str, dialect: Dialect, at: usize, typed: char) -> bool {
    sql::tokenize(dialect, text).iter().any(|token| {
        let quoted = matches!(token.kind, TokenKind::String | TokenKind::QuotedIdentifier);
        let range = &token.range;
        quoted
            && range.start < at
            && range.end == at + typed.len_utf8()
            && text[range.clone()].ends_with(typed)
            && text[range.clone()]
                .trim_start_matches(char::is_alphabetic)
                .starts_with(typed)
    })
}

/// Whether `at` is inside a string, a quoted name or a comment of `text`
/// (one left open counts to its end): a quote typed there closes one or
/// is text, and is never paired. Told by the tokenizer, of the text up to
/// `at` with a letter after it: a literal that takes the letter in goes
/// on past `at`. So a line comment with lines under it, a dollar quote, an
/// escaped quote at an open string's end are read as the dialect reads
/// them.
fn in_literal(text: &str, dialect: Dialect, at: usize) -> bool {
    // A letter, not a space: MySQL reads `--` as a comment only before a
    // space, and the text after the cursor is not known to be one.
    let probe = format!("{}x", &text[..at]);
    sql::tokenize(dialect, &probe).iter().any(|token| {
        let literal = matches!(
            token.kind,
            TokenKind::String
                | TokenKind::QuotedIdentifier
                | TokenKind::Comment
                | TokenKind::ExecutableComment
        );
        literal && token.range.start < at && at < token.range.end
    })
}

/// The edit Close brackets and quotes makes of a character typed with
/// nothing selected, at byte `at` of `text`: its pair put in (`Pair`), a
/// closing character stepped over (`Over`), or none.
#[derive(Debug, PartialEq, Eq)]
enum Paired {
    Pair(char),
    Over,
}

fn paired(text: &str, dialect: Dialect, at: usize, typed: char) -> Option<Paired> {
    let next = text[at..].chars().next();
    let before = text[..at].chars().next_back();
    // Its own closing character already there: stepped over. A quote
    // only where it closes the literal the cursor is in: one that opens
    // the next string is typed before it.
    let bracket = matches!(typed, ')' | ']' | '}');
    let quote = closer_of(typed, dialect) == Some(typed);
    let closes = bracket || (quote && closes_literal(text, dialect, at, typed));
    if closes && next == Some(typed) {
        return Some(Paired::Over);
    }
    let closer = closer_of(typed, dialect)?;
    // Only where nothing follows that the pair would split off.
    let room = next.is_none_or(|next| next.is_whitespace() || ")]},;".contains(next));
    if !room {
        return None;
    }
    if closer == typed {
        // A quote: not inside a word (`it's`), after another quote (`''`
        // escapes), or inside a string or a comment.
        let wordy = before
            .is_some_and(|before| before.is_alphanumeric() || before == '_' || before == typed);
        if wordy || in_literal(text, dialect, at) {
            return None;
        }
    }
    Some(Paired::Pair(closer))
}

/// Close brackets and quotes, before the field reads the frame's events:
/// a bracket or a quote typed gets its closing one after the caret, a
/// closing one typed before its twin steps over it, Backspace between an
/// empty pair takes both out, and a bracket or a quote typed over a
/// selection puts the selection between the two (one step of the undo
/// history, the selection kept). The other edits are the field's own
/// (the pair typed and the caret moved back), so they undo as typing
/// does. Only in a frame of one edit, while the field has the keyboard.
/// Returns whether the text changed here.
fn close_pairs(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> bool {
    if !field.pairs || !ui.memory(|memory| memory.has_focus(field.id)) {
        return false;
    }
    let key = |key: egui::Key| egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    };
    // The frame's one edit: a character typed, or a plain Backspace. A
    // character comes with the press of its key too, which is no edit of
    // its own; the keys that edit or move the caret are.
    let edit = ui.input(|input| {
        use egui::Key as K;
        let edits: Vec<&egui::Event> = input
            .events
            .iter()
            .filter(|event| match event {
                egui::Event::Text(_)
                | egui::Event::Paste(_)
                | egui::Event::Cut
                | egui::Event::Ime(egui::ImeEvent::Commit(_)) => true,
                // The text field's own editing keys, which the keymap does
                // not hold, and any chord (undo, redo); not Ctrl+Alt, which
                // is AltGr on Windows, typing `[`, `{` or a backtick on
                // German, French or Polish keyboards.
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers,
                    ..
                } => {
                    *pressed
                        && (modifiers.command || modifiers.ctrl)
                        && !(modifiers.ctrl && modifiers.alt)
                        || *pressed
                            && matches!(
                                key,
                                K::Backspace
                                    | K::Delete
                                    | K::Enter
                                    | K::Tab
                                    | K::ArrowLeft
                                    | K::ArrowRight
                                    | K::ArrowUp
                                    | K::ArrowDown
                                    | K::Home
                                    | K::End
                                    | K::PageUp
                                    | K::PageDown
                            )
                }
                _ => false,
            })
            .collect();
        match edits.as_slice() {
            [egui::Event::Text(text)] if text.chars().count() == 1 => text.chars().next(),
            [
                egui::Event::Key {
                    key: egui::Key::Backspace,
                    modifiers,
                    ..
                },
            ] if modifiers.is_none() => Some('\u{8}'),
            _ => None,
        }
    });
    let Some(typed) = edit else {
        return false;
    };
    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
    let Some(range) = state.cursor.char_range() else {
        return false;
    };
    let [start, end] = range.sorted_cursors();
    let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
    let (start, end) = (byte(start), byte(end));
    let text = &sql_tab.text;
    if typed == '\u{8}' {
        // Backspace between an empty pair: the closing one goes with it.
        let before = text[..start].chars().next_back();
        let next = text[start..].chars().next();
        // A quote's pair only where the first one opens a literal: the two
        // of an escaped quote (`'it''s'`) are inside one.
        let empty = start == end
            && before.is_some_and(|before| {
                let opens = closer_of(before, field.dialect).is_some_and(|c| Some(c) == next);
                let escaped = before == next.unwrap_or_default()
                    && in_literal(text, field.dialect, start - before.len_utf8());
                opens && !escaped
            });
        if empty {
            ui.input_mut(|input| input.events.push(key(egui::Key::Delete)));
        }
        return false;
    }
    if start != end {
        // Over a selection: the selection between the two.
        let Some(closer) = closer_of(typed, field.dialect) else {
            return false;
        };
        ui.input_mut(|input| {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Text(_)));
        });
        let inner = &text[start..end];
        let out = [
            &text[..start],
            &typed.to_string(),
            inner,
            &closer.to_string(),
            &text[end..],
        ]
        .concat();
        let (from, to) = (start + typed.len_utf8(), end + typed.len_utf8());
        let at = |byte: usize| CCursor::new(char_index(&out, byte));
        let backwards = range.primary.index < range.secondary.index;
        let after = if backwards {
            CCursorRange::two(at(to), at(from))
        } else {
            CCursorRange::two(at(from), at(to))
        };
        let before = (range, std::mem::take(&mut sql_tab.text));
        let after = (after, out);
        outside_edit(&mut state, ui.input(|input| input.time), &before, &after);
        egui::TextEdit::store_state(ui.ctx(), field.id, state);
        sql_tab.text = after.1;
        sql_tab.cursor = if backwards { from } else { to };
        return true;
    }
    let events = match paired(text, field.dialect, start, typed) {
        Some(Paired::Pair(closer)) => vec![
            egui::Event::Text(format!("{typed}{closer}")),
            key(egui::Key::ArrowLeft),
        ],
        Some(Paired::Over) => vec![key(egui::Key::ArrowRight)],
        None => return false,
    };
    ui.input_mut(|input| {
        input
            .events
            .retain(|event| !matches!(event, egui::Event::Text(_)));
        input.events.extend(events);
    });
    false
}

/// What vim does with one of the frame's events.
enum Read {
    /// The field's, as it is.
    Keep,
    /// Nobody's: a key's press whose character comes as text.
    Drop,
    /// Vim's.
    Take(Vec<vim::Input>),
}

/// Vim's chords as the keymap has them in the SQL editor (`VimRedo`,
/// `VimNormal`): its letters, and the field's own keys, are the editor's.
struct VimChords {
    layout: crate::keymap::Layout,
    /// What the frame holds.
    held: egui::Modifiers,
    redo: Vec<&'static str>,
    normal: Vec<&'static str>,
}

impl VimChords {
    fn of(ui: &Ui, look: &Look) -> Self {
        use crate::keymap::{Command, Layout, Scope};
        let keymap = crate::ui::keys::published(ui.ctx());
        let layout = Layout::of(look);
        let chords = |command, normal| -> Vec<&'static str> {
            keymap
                .chords_now(layout, command, Scope::SqlEditor, normal)
                .collect()
        };
        Self {
            layout,
            held: ui.input(|input| input.modifiers),
            redo: chords(Command::VimRedo, true),
            normal: chords(Command::VimNormal, false),
        }
    }

    fn pressed(&self, event: &egui::Event, chords: &[&'static str]) -> bool {
        chords
            .iter()
            .any(|keys| crate::ui::keys::press_of(self.layout, self.held, event, keys).is_some())
    }
}

/// How vim in `mode` reads `event`. In insert mode it takes Esc (and the
/// keymap's `VimNormal`, Ctrl+[) alone, unless `held` (an open completion
/// list takes it). Out of it, the characters typed and the keys that move
/// or edit, and `VimRedo`; the field keeps its chords (copy, paste, select
/// all, undo), but not vim's deletions of insert mode nor Tab, which type
/// nothing here. A held Esc is not normal mode's: the keyboard does not
/// leave with it.
fn vim_input(event: &egui::Event, mode: vim::Mode, held: bool, chords: &VimChords) -> Read {
    use egui::{Event, Key};
    use vim::Input;
    let escape = |key: &Key| *key == Key::Escape || chords.pressed(event, &chords.normal);
    if mode == vim::Mode::Insert {
        return match event {
            Event::Key {
                key, pressed: true, ..
            } if escape(key) && !held => Read::Take(vec![Input::Escape]),
            _ => Read::Keep,
        };
    }
    match event {
        Event::Text(text) | Event::Ime(egui::ImeEvent::Commit(text)) => Read::Take(
            text.chars()
                .filter(|c| !matches!(c, '\n' | '\r'))
                .map(Input::Char)
                .collect(),
        ),
        Event::Ime(_) => Read::Drop,
        Event::Key {
            key,
            pressed,
            repeat,
            modifiers,
            ..
        } => {
            if !pressed {
                return Read::Keep;
            }
            if escape(key) {
                return if *repeat {
                    Read::Drop
                } else {
                    Read::Take(vec![Input::Escape])
                };
            }
            // Before AltGr's drop: a redo may be set to a chord with Ctrl
            // and Alt.
            if chords.pressed(event, &chords.redo) {
                return Read::Take(vec![Input::Redo]);
            }
            if modifiers.ctrl && modifiers.alt {
                // AltGr on Windows: its character comes as text.
                return Read::Drop;
            }
            // The field's chords that delete (a word, to a line's end) have
            // nothing to delete in normal mode.
            let deletes = matches!(key, Key::Backspace | Key::Delete)
                || (modifiers.ctrl && matches!(key, Key::W | Key::H | Key::U | Key::K));
            if modifiers.any() && deletes {
                return Read::Drop;
            }
            if modifiers.command || modifiers.ctrl {
                return Read::Keep;
            }
            let input = match key {
                Key::Enter => Input::Enter,
                Key::Backspace => Input::Backspace,
                Key::Delete => Input::Delete,
                Key::ArrowLeft => Input::Left,
                Key::ArrowRight => Input::Right,
                Key::ArrowUp => Input::Up,
                Key::ArrowDown => Input::Down,
                Key::Home => Input::Home,
                Key::End => Input::End,
                // The field pages, as in every mode.
                Key::PageUp | Key::PageDown => return Read::Keep,
                _ => return Read::Drop,
            };
            Read::Take(vec![input])
        }
        _ => Read::Keep,
    }
}

/// Whether the field, reading `event`, may change the script or the
/// cursor.
fn edits(event: &egui::Event) -> bool {
    matches!(
        event,
        egui::Event::Text(_)
            | egui::Event::Paste(_)
            | egui::Event::Cut
            | egui::Event::Ime(_)
            | egui::Event::Key { pressed: true, .. }
    )
}

/// Where the keybindings are vim's, before the field reads the frame: in
/// normal and visual mode the frame's keys go to vim, whose edits go in
/// as edits from outside, and whose format Format carries out; in insert
/// mode Esc goes back to normal mode. Normal mode's cursor stays on a
/// character. Returns whether the script or the cursor changed, and
/// whether `:` was typed. (Vim keeps its own undo history: the field's
/// counts cursor moves.)
///
/// The frame's events are read in order, each in the mode the ones before
/// it left. A key of vim's after one the field is to read (typing, or a
/// chord), or after a format, which Format carries out after this, would
/// read the script before them: it and what follows wait for the next
/// frame, where the keymap has read its own already. After Esc out of the
/// editor or `:`, the frame's keys are nobody's.
fn vim_keys(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> (bool, bool) {
    let later_id = field.id.with("vim later");
    // A press on the script leaves visual mode, with the keyboard here or
    // coming back with the press: the pointer selects as it does anywhere,
    // and what visual mode selected goes (the press sets its own). A press
    // anywhere else (Run, the results) leaves it be: Run runs the visual
    // selection. The gutter's press leaves it too, in `show`.
    let pane = ui.clip_rect();
    let pressed = field.vim
        && sql_tab.vim.mode.is_visual()
        && ui.input(|input| {
            input.events.iter().any(|event| {
                matches!(event, egui::Event::PointerButton { pos, pressed: true, .. }
                    if pane.contains(*pos))
            })
        });
    if pressed {
        sql_tab.vim.to_normal();
        let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
        let head = CCursorRange::one(CCursor::new(sql_tab.vim.head()));
        state.cursor.set_char_range(Some(head));
        egui::TextEdit::store_state(ui.ctx(), field.id, state);
    }
    if !field.vim || !ui.memory(|memory| memory.has_focus(field.id)) {
        ui.data_mut(|data| data.remove_temp::<Vec<egui::Event>>(later_id));
        return (pressed, false);
    }
    let waited: Vec<egui::Event> = ui
        .data_mut(|data| data.remove_temp(later_id))
        .unwrap_or_default();
    let events = ui.input_mut(|input| {
        let mut events = waited;
        events.append(&mut input.events);
        events
    });
    let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
    let mut changed = pressed;
    if sql_tab.vim.mode == vim::Mode::Normal
        && let Some(range) = state.cursor.char_range()
        && range.is_empty()
    {
        // A click or an undo may leave it after a line's last character.
        let at = vim::clamp(&sql_tab.text, range.primary.index.0);
        if at != range.primary.index.0 {
            state
                .cursor
                .set_char_range(Some(CCursorRange::one(CCursor::new(at))));
            changed = true;
        }
    }
    let mut prompt = false;
    let register_id = Id::new("vim register");
    // The preview's yanks and deletes are its own: the editors' register
    // is not theirs to fill.
    let mut register: vim::Register = if field.read_only {
        vim::Register::default()
    } else {
        ui.data(|data| data.get_temp(register_id))
            .unwrap_or_default()
    };
    let time = ui.input(|input| input.time);
    let unit = indent_unit(field.indent);
    let chords = VimChords::of(ui, field.look);
    let (mut kept, mut later) = (Vec::new(), Vec::new());
    // The field has events of this frame to read first.
    let mut field_first = false;
    let mut queue = events.into_iter();
    while let Some(event) = queue.next() {
        let mode = sql_tab.vim.mode;
        // A cut or a paste ends visual mode: the selection is the field's
        // to take.
        if mode.is_visual() && matches!(event, egui::Event::Cut | egui::Event::Paste(_)) {
            sql_tab.vim.to_normal();
        }
        let inputs = match vim_input(&event, sql_tab.vim.mode, field.hold_escape, &chords) {
            Read::Keep => {
                field_first |= edits(&event);
                kept.push(event);
                continue;
            }
            Read::Drop => continue,
            Read::Take(inputs) => inputs,
        };
        if field_first {
            later.push(event);
            later.extend(queue.by_ref());
            break;
        }
        let (mut waits, mut done) = (false, false);
        for input in inputs {
            let cursor = if sql_tab.vim.mode.is_visual() {
                sql_tab.vim.head()
            } else {
                state.cursor.char_range().map_or_else(
                    || char_index(&sql_tab.text, sql_tab.cursor),
                    |range| range.primary.index.0,
                )
            };
            let step = sql_tab
                .vim
                .key(&sql_tab.text, cursor, input, &mut register, unit);
            if field.read_only && (step.text.is_some() || sql_tab.vim.mode == vim::Mode::Insert) {
                // The preview's text does not change: what would change it
                // is dropped, and so is insert mode, which has nothing to
                // type; vim stays in normal mode.
                sql_tab.vim.to_normal();
                continue;
            }
            if let Some(text) = &step.text {
                // Undo puts the cursor back, not a selection.
                let before = CCursorRange::one(CCursor::new(cursor));
                let range = vim_range(text, &step, sql_tab.vim.mode);
                let before = (before, std::mem::replace(&mut sql_tab.text, text.clone()));
                outside_edit(&mut state, time, &before, &(range, text.clone()));
            } else {
                let range = vim_range(&sql_tab.text, &step, sql_tab.vim.mode);
                state.cursor.set_char_range(Some(range));
            }
            changed = true;
            match step.then {
                Some(vim::Then::Format(_)) if field.read_only => {}
                Some(vim::Then::Format(lines)) => {
                    // Format formats the statements a selection touches,
                    // or with none every one.
                    let whole = lines.start == 0 && lines.end >= sql_tab.text.chars().count();
                    let range = if whole {
                        CCursorRange::one(CCursor::new(lines.start))
                    } else {
                        CCursorRange::two(CCursor::new(lines.end), CCursor::new(lines.start))
                    };
                    state.cursor.set_char_range(Some(range));
                    sql_tab.format = true;
                    // Format's undo, and where it leaves nothing to do,
                    // put the cursor back, not the lines.
                    ui.data_mut(|data| data.insert_temp(field.id.with("vim format"), cursor));
                    waits = true;
                }
                Some(vim::Then::Prompt) => {
                    prompt = true;
                    done = true;
                }
                Some(vim::Then::Leave) => {
                    ui.memory_mut(|memory| memory.surrender_focus(field.id));
                    done = true;
                }
                None => {}
            }
        }
        if done {
            sql_tab.vim.to_normal();
            break;
        }
        if waits {
            later.extend(queue.by_ref());
            break;
        }
    }
    if !field.read_only {
        ui.data_mut(|data| data.insert_temp(register_id, register));
    }
    if !later.is_empty() {
        ui.data_mut(|data| data.insert_temp(later_id, later));
        ui.ctx().request_repaint();
    }
    if changed {
        if let Some(range) = state.cursor.char_range() {
            sql_tab.cursor = byte_offset(&sql_tab.text, range.primary.index.0);
        }
        egui::TextEdit::store_state(ui.ctx(), field.id, state);
    }
    ui.input_mut(|input| input.events = kept);
    (changed, prompt)
}

/// The field's cursor for vim's `step` in `text`, now in `mode`: the
/// cursor, or in visual mode the selection, whose ends' characters are in
/// it (by line, whole lines).
fn vim_range(text: &str, step: &vim::Step, mode: vim::Mode) -> CCursorRange {
    let one = |at: usize| CCursorRange::one(CCursor::new(at));
    let Some(anchor) = step.anchor else {
        return one(step.cursor);
    };
    let chars: Vec<char> = text.chars().collect();
    let length = chars.len();
    let (cursor, forward) = (step.cursor, step.cursor >= anchor);
    let (low, high) = (anchor.min(cursor), anchor.max(cursor));
    let (start, end) = if mode == vim::Mode::VisualLine {
        let start = chars[..low.min(length)]
            .iter()
            .rposition(|c| *c == '\n')
            .map_or(0, |at| at + 1);
        let end = chars[high.min(length)..]
            .iter()
            .position(|c| *c == '\n')
            .map_or(length, |at| high + at);
        (start, end)
    } else {
        (low, (high + 1).min(length))
    };
    if forward {
        CCursorRange::two(CCursor::new(start), CCursor::new(end))
    } else {
        CCursorRange::two(CCursor::new(end), CCursor::new(start))
    }
}

/// The text field itself: edits the tab's text and reports its cursor.
fn edit(ui: &mut Ui, sql_tab: &mut SqlTab, field: &Field<'_>) -> Edited {
    let inserted = insert_completion(ui.ctx(), field.id, sql_tab);
    let focus = std::mem::take(&mut sql_tab.focus_editor);
    // Vim's normal and visual modes take the frame's keys before the
    // field, and the edits that follow, before Tab and the pairs read it.
    // Its history starts at the script the editor first shows.
    if field.vim {
        sql_tab.vim.start(&sql_tab.text);
    }
    let (vimmed, prompt) = vim_keys(ui, sql_tab, field);
    // The preview's Esc, with the standard keys, gives the keyboard up and
    // is taken: the next closes the window. (Vim's is `vim_keys`'.)
    if field.read_only
        && !field.vim
        && ui.memory(|memory| memory.has_focus(field.id))
        && ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        ui.memory_mut(|memory| memory.surrender_focus(field.id));
    }
    // Before the field is drawn, so this frame shows the formatted text.
    // An indent is an outside edit too: what the field shows, and where
    // its caret goes, follow as after Format.
    // The preview's text does not change.
    let indented = !field.read_only && indent_script(ui, sql_tab, field);
    let paired = !field.read_only && close_pairs(ui, sql_tab, field);
    let formatted = std::mem::take(&mut sql_tab.format) && format_script(ui, sql_tab, field);
    let formatted = formatted || indented || paired || vimmed;
    // The cursor put somewhere from outside (an error's mark): set before
    // the field is drawn, which then draws it there.
    let moved = sql_tab.go_to.take().is_some_and(|at| {
        let at = sql_tab.text.floor_char_boundary(at);
        let mut state = egui::TextEdit::load_state(ui.ctx(), field.id).unwrap_or_default();
        let cursor = CCursor::new(char_index(&sql_tab.text, at));
        state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
        egui::TextEdit::store_state(ui.ctx(), field.id, state);
        sql_tab.cursor = at;
        sql_tab.selected = None;
        true
    });
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
    let mut layouter = layouter(
        field.look,
        field.palette,
        field.dialect,
        field.id,
        field.wraps,
    );
    // Wrapped, as wide as the pane, which the layouter wraps at.
    let width = if field.wraps {
        ui.available_width()
    } else {
        f32::INFINITY
    };
    // Out of insert mode vim's cursor is a block, drawn below: egui's
    // caret is not.
    let block = field.vim && sql_tab.vim.mode != vim::Mode::Insert;
    let caret_stroke = ui.visuals().text_cursor.stroke;
    if block {
        ui.visuals_mut().text_cursor.stroke.color = Color32::TRANSPARENT;
    }
    // The preview's text, which the field reads and cannot change.
    let kept = field.read_only.then(|| sql_tab.text.clone());
    let mut fixed: &str = kept.as_deref().unwrap_or_default();
    let buffer: &mut dyn egui::TextBuffer = if field.read_only {
        &mut fixed
    } else {
        &mut sql_tab.text
    };
    let output = egui::TextEdit::multiline(buffer)
        .id(field.id)
        .font(font)
        // Tab indents; Esc gives the keys up unless `hold_escape`. The
        // preview's Tab moves on: it is one stop.
        .lock_focus(!field.read_only)
        .frame(egui::Frame::new().inner_margin(margin))
        .desired_width(width)
        .desired_rows(rows as usize)
        .layouter(&mut layouter)
        .show(ui);
    ui.visuals_mut().text_cursor.stroke = caret_stroke;
    // The preview shows vim's cursor without the keyboard too: the mode
    // the keybindings give.
    if block && (output.response.has_focus() || field.read_only) {
        let at = if sql_tab.vim.mode.is_visual() {
            sql_tab.vim.head()
        } else {
            output
                .state
                .cursor
                .char_range()
                .map_or(0, |range| range.primary.index.0)
        };
        crate::ui::cell_editor::block_cursor_at(
            ui,
            &output,
            CCursor::new(at),
            role(field.look),
            field.look,
            field.palette,
        );
    }
    // The workspace's editor region, which F6 and the pane keys move to:
    // not the preview's.
    if !field.read_only {
        crate::ui::focus::region(ui, crate::ui::focus::Region::Editor, output.response.rect);
        crate::ui::focus::claim(ui, crate::ui::focus::Region::Editor, &output.response);
    }
    // The script fills its pane: the caret says where the keyboard is.
    // (The preview's card is its ring: `editor_preview` asks for it.)
    if !field.read_only {
        crate::ui::focus::hint(
            ui,
            &output.response,
            output.response.rect,
            crate::ui::focus::Ring::Own,
        );
    }
    // The name only: the field keeps the role and the value egui gave it.
    ui.ctx().accesskit_node_builder(field.id, |node| {
        node.set_label(field.name);
        if field.read_only {
            node.set_read_only();
        }
    });
    if focus {
        output.response.request_focus();
        // The keys arrive with the next frame: ask for it, since no event
        // need follow the one that opened the tab.
        ui.ctx().request_repaint();
    }
    // The caret on screen.
    let caret = output.state.cursor.range(&output.galley).map(|range| {
        let caret = output.galley.pos_from_cursor(range.primary);
        caret.translate(output.galley_pos.to_vec2())
    });
    // The field scrolls to its caret when its own events move it. A row
    // put in from outside moves it too, and so does Format. And a
    // completion list that has not drawn yet hangs under a word that may
    // be out of the pane (it was asked for after scrolling away): the
    // caret comes into view, and the list with it. (The list here is an
    // open one: `insert_completion` took one whose row was accepted. One
    // that still has no place once the caret is in the pane is closed by
    // `completion_list`, so this ends.)
    let list_waits = sql_tab
        .completion
        .as_ref()
        .is_some_and(|list| !super::sql_complete::was_shown(ui.ctx(), field.id, list));
    if (inserted || list_waits)
        && let Some(caret) = caret
    {
        ui.scroll_to_rect(caret.expand(1.5), None);
    } else if (formatted || moved)
        && let Some(caret) = caret
    {
        // The cursor's line moved with the layout. The field scrolls to
        // its cursor after an edit of its own, not after this one, so
        // bring it into view here.
        ui.scroll_to_rect(caret, None);
    }
    if inserted || formatted || moved {
        // One more frame: what is drawn before the editor (the footer's
        // line and column) and the gutter read the script and the cursor
        // before the row went in or the script was formatted.
        ui.ctx().request_repaint();
    }
    // Vim's Esc is its own: insert mode's goes to normal mode, and normal
    // mode's leaves the editor when `vim_keys` says so.
    if (field.hold_escape || field.vim || field.read_only) && output.response.has_focus() {
        // The field set its own filter while it drew (the arrows and Tab).
        // egui replaces the whole filter, so those are named again. The
        // filter is read at the start of the next pass, so holding Esc takes
        // effect a frame after the flag is set and ends a frame after it is
        // cleared.
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                field.id,
                egui::EventFilter {
                    // The preview's Tab moves on: it is one stop.
                    tab: !field.read_only,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
    }
    // egui counts the cursor in characters; a field never focused has
    // none, and the tab keeps the one it has. The footer and the status
    // line, drawn before the editor, say where the cursor is a frame later
    // (egui draws one after any event).
    if let Some(range) = output.state.cursor.range(&output.galley) {
        // In vim's visual mode the selection runs one past the cursor.
        let cursor = if field.vim && sql_tab.vim.mode.is_visual() {
            sql_tab.vim.head()
        } else {
            range.primary.index.0
        };
        sql_tab.cursor = byte_offset(&sql_tab.text, cursor);
        let [start, end] = range.sorted_cursors();
        let byte = |cursor: CCursor| byte_offset(&sql_tab.text, cursor.index.0);
        sql_tab.selected = (start != end).then(|| byte(start)..byte(end));
    }
    let typed = output.response.changed() && ui.input(|input| input.events.iter().any(is_typed));
    Edited {
        typed,
        prompt,
        response: output.response.response.clone(),
        focused: output.response.has_focus(),
        origin: output.galley_pos,
        caret,
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
            let mut layouter = layouter(&look, &palette, Dialect::Postgres, id, false);
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
            assert_eq!(section.format.line_height, role.line_height(look.faces));
        }
        // An empty script is one line, as tall as any other.
        let empty = &galleys[1];
        assert_eq!(empty.rows.len(), 1);
        assert_eq!(empty.rows[0].height(), galley.rows[0].height());
    }

    #[test]
    fn vim_s_redo_set_to_a_chord_with_ctrl_and_alt_is_read_and_altgr_is_not() {
        let held = egui::Modifiers::CTRL | egui::Modifiers::ALT;
        let chords = VimChords {
            layout: crate::keymap::Layout::Omarchy,
            held,
            redo: vec!["ctrl+alt+r"],
            normal: vec!["ctrl+["],
        };
        let press = |key| crate::testing::key(key, held);
        let read = |key| vim_input(&press(key), vim::Mode::Normal, false, &chords);
        assert!(matches!(read(egui::Key::R), Read::Take(inputs) if inputs == [vim::Input::Redo]));
        // AltGr with another key: its character comes as text.
        assert!(matches!(read(egui::Key::Num8), Read::Drop));
    }

    #[test]
    fn a_line_starts_its_rows_and_a_wrapped_line_has_more() {
        let (look, palette) = (Look::standard(), Palette::light());
        let long = format!("SELECT {}\nFROM t\n", "title, ".repeat(40));
        let mut galleys = Vec::new();
        let mut harness = crate::testing::Harness::new();
        harness.frame_with(|ui| {
            let id = Id::new("an editor");
            let mut wrapping = layouter(&look, &palette, Dialect::Postgres, id, true);
            galleys.push(wrapping(ui, &long, 300.0));
            galleys.push(wrapping(ui, &String::new(), 300.0));
            galleys.push(wrapping(ui, &"a\nb\n".to_owned(), 300.0));
        });
        let wrapped = line_rows(&galleys[0]);
        // Three lines (the last empty, after the break), the first of
        // several rows.
        assert_eq!(wrapped.len(), 3);
        assert_eq!(wrapped[0], 0);
        assert!(wrapped[1] > 1, "{wrapped:?}");
        assert_eq!(wrapped[2], galleys[0].rows.len() - 1);
        assert_eq!(line_rows(&galleys[1]), [0]);
        assert_eq!(line_rows(&galleys[2]), [0, 1, 2]);
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
        // Past the end, or inside a character: the end.
        assert_eq!(char_index("żółw", 99), 4);
        assert_eq!(char_index("żółw", 1), 4);
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
    fn a_pair_goes_in_where_nothing_follows_that_it_would_split_off() {
        let pair = |text: &str, typed| paired(text, Dialect::Postgres, text.len(), typed);
        assert_eq!(pair("SELECT count", '('), Some(Paired::Pair(')')));
        assert_eq!(pair("SELECT ", '\''), Some(Paired::Pair('\'')));
        assert_eq!(pair("SELECT ", '"'), Some(Paired::Pair('"')));
        // A backtick quotes names on MySQL alone.
        assert_eq!(pair("SELECT ", '`'), None);
        let mysql = paired("SELECT ", Dialect::MySql, 7, '`');
        assert_eq!(mysql, Some(Paired::Pair('`')));
        // Before a word, after a word or a quote, inside a literal: none.
        assert_eq!(paired("SELECT x", Dialect::Postgres, 7, '('), None);
        assert_eq!(pair("SELECT it", '\''), None);
        assert_eq!(pair("SELECT ''", '\''), None);
        assert_eq!(pair("SELECT 'a ", '\''), None);
        assert_eq!(pair("-- a ", '"'), None);
        assert_eq!(pair("/* a ", '"'), None);
        // Closed, a comment or a string is behind the cursor.
        assert_eq!(pair("/* a */ ", '"'), Some(Paired::Pair('"')));
        assert_eq!(pair("SELECT 'a' ", '"'), Some(Paired::Pair('"')));
        // Before a closing one or a comma: room.
        assert_eq!(
            paired("f()", Dialect::Postgres, 2, '['),
            Some(Paired::Pair(']'))
        );
        // Its twin after the cursor: stepped over.
        assert_eq!(paired("f()", Dialect::Postgres, 2, ')'), Some(Paired::Over));
        assert_eq!(paired("''", Dialect::Postgres, 1, '\''), Some(Paired::Over));
        // A closing one with nothing to step over is typed.
        assert_eq!(pair("f(", ')'), None);
    }

    #[test]
    fn a_literal_is_told_as_the_dialect_reads_it() {
        use Dialect::{MySql, Postgres};
        let at = |text: &str, dialect, at, typed| paired(text, dialect, at, typed);
        // A line comment with a line under it, `--` and MySQL's `#`.
        assert_eq!(at("-- note \nSELECT 1", Postgres, 8, '"'), None);
        assert_eq!(at("SELECT 1 # x \nSELECT 2", MySql, 13, '\''), None);
        // Left open at the end: dollar quotes, an escaped quote, a quoted
        // name, a block comment that only looks closed.
        assert_eq!(at("SELECT $$", Postgres, 9, '\''), None);
        assert_eq!(at("SELECT $f$", Postgres, 10, '\''), None);
        assert_eq!(at("SELECT 'it''", Postgres, 12, '"'), None);
        assert_eq!(at("SELECT \"a\"\"", Postgres, 11, '\''), None);
        assert_eq!(at("/*/", Postgres, 3, '\''), None);
        // Closed, whatever its first character: room for a pair.
        assert_eq!(
            at("SELECT E'a' ", Postgres, 12, '"'),
            Some(Paired::Pair('"'))
        );
        // A quote before the next string's opening one is typed, not
        // stepped over.
        let two = "SELECT 'a',  'b'";
        assert_eq!(at(two, Postgres, two.find("'b'").unwrap(), '\''), None);
        // In a string, before its closing quote: stepped over.
        assert_eq!(at("SELECT 'a'", Postgres, 9, '\''), Some(Paired::Over));
        // Before a quote that closes nothing it is typed: one inside a
        // string of the other quote, one of an escaped pair.
        assert_eq!(at("'{\"a\":1}'", Postgres, 4, '"'), None);
        assert_eq!(at("'it''s'", Postgres, 3, '\''), None);
        // Backticks quote names on MySQL and SQLite, and step over there.
        assert_eq!(at("``", MySql, 1, '`'), Some(Paired::Over));
        assert_eq!(
            at("SELECT ", Dialect::Sqlite, 7, '`'),
            Some(Paired::Pair('`'))
        );
        // MySQL's `--` is a comment only before a space: right after it,
        // a quote pairs.
        assert_eq!(at("SELECT 1 --", MySql, 11, '\''), Some(Paired::Pair('\'')));
    }

    #[test]
    fn an_indent_goes_in_at_the_cursor_or_on_each_line_selected() {
        let tab = |text: &str, selection: Range<usize>, indent, outdent| {
            indented(text, selection, indent, outdent)
        };
        // No selection: at the cursor, in the middle of a line too.
        assert_eq!(
            tab("SELECT 1", 6..6, Indent::Two, false),
            Some(("SELECT   1".to_owned(), 8..8))
        );
        assert_eq!(
            tab("x", 0..0, Indent::Tab, false),
            Some(("\tx".to_owned(), 1..1))
        );
        // A selection: each line it touches, not one it only reaches the
        // start of, and no blank one; it covers the same text after.
        assert_eq!(
            tab("a\nb\n\nc\nd", 2..7, Indent::Four, false),
            Some(("a\n    b\n\n    c\nd".to_owned(), 2..15))
        );
        // Characters of more than one byte keep their place.
        assert_eq!(
            tab("é\nü", 0..5, Indent::Two, false),
            Some(("  é\n  ü".to_owned(), 0..9))
        );
    }

    #[test]
    fn an_outdent_takes_one_indent_off_each_line_it_touches() {
        let tab =
            |text: &str, selection: Range<usize>, indent| indented(text, selection, indent, true);
        // The cursor's line: up to an indent's width of spaces.
        assert_eq!(
            tab("    x", 5..5, Indent::Two),
            Some(("  x".to_owned(), 3..3))
        );
        assert_eq!(tab(" x", 2..2, Indent::Four), Some(("x".to_owned(), 1..1)));
        // A tab, whatever the indent is.
        assert_eq!(tab("\tx", 1..1, Indent::Two), Some(("x".to_owned(), 0..0)));
        // Nothing to take: nothing changes.
        assert_eq!(tab("x\n  y", 0..1, Indent::Two), None);
        // A cursor inside the indent stays at the line's new start.
        assert_eq!(
            tab("    x", 1..1, Indent::Four),
            Some(("x".to_owned(), 0..0))
        );
        // Every line selected.
        assert_eq!(
            tab("  a\n    b\nc", 0..10, Indent::Two),
            Some(("a\n  b\nc".to_owned(), 0..6))
        );
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

    /// One frame of an editor on its own, with `events`. Returns whether
    /// it has the keyboard afterwards.
    fn editor_frame(
        harness: &mut crate::testing::Harness,
        sql: &mut SqlTab,
        hold_escape: bool,
        events: Vec<egui::Event>,
    ) -> bool {
        let (look, palette) = (Look::standard(), Palette::light());
        let input = egui::RawInput {
            screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, harness.size)),
            events,
            ..Default::default()
        };
        let mut focused = false;
        let mut output = harness.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                // A neighbour for the focus to move to, were the arrows
                // not held by the editor.
                let _ = ui.button("above");
                let field = Field {
                    id: Id::new("held editor"),
                    name: "SQL",
                    left: 14,
                    dialect: Dialect::Sqlite,
                    look: &look,
                    palette: &palette,
                    hold_escape,
                    pairs: false,
                    vim: false,
                    read_only: false,
                    style: sql::format::Style::default(),
                    // A tab character: what Tab put in before the setting.
                    indent: Indent::Tab,
                    wraps: false,
                };
                focused = edit(ui, sql, &field).focused;
            });
        });
        // As `Harness::frame_with` does: a delta dropped unapplied panics.
        output.textures_delta.clear();
        focused
    }

    #[test]
    fn an_editor_told_to_hold_escape_keeps_the_keyboard() {
        use crate::testing::{key, release};
        use egui::{Key, Modifiers};
        let mut harness = crate::testing::Harness::new();
        let mut sql = SqlTab::new(TabId(1), 1, 1_000, None);
        sql.text = "SELECT 1\nFROM users".into();
        // A new editor asks for the keyboard and has it a frame later; the
        // lock filter takes one frame more, since `set_focus_lock_filter`
        // does nothing until the widget had focus on the previous frame.
        editor_frame(&mut harness, &mut sql, true, Vec::new());
        editor_frame(&mut harness, &mut sql, true, Vec::new());
        assert!(editor_frame(&mut harness, &mut sql, true, Vec::new()));
        // Esc is held: the editor still has the keyboard.
        let esc = || vec![key(Key::Escape, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, esc()));
        let up = vec![release(Key::Escape, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, up));
        // The arrows are still the editor's: the cursor moves up a line
        // and the keyboard stays.
        assert_eq!(sql.cursor, sql.text.len());
        let before = sql.cursor;
        let arrow = vec![key(Key::ArrowUp, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, arrow));
        assert!(editor_frame(&mut harness, &mut sql, true, Vec::new()));
        assert!(sql.cursor < before, "{} then {}", before, sql.cursor);
        // So is Tab: it indents.
        let tab = vec![key(Key::Tab, Modifiers::NONE)];
        assert!(editor_frame(&mut harness, &mut sql, true, tab));
        assert!(sql.text.contains('\t'));
        // Not told to hold it: Esc leaves the editor, as before.
        assert!(editor_frame(&mut harness, &mut sql, false, Vec::new()));
        assert!(!editor_frame(&mut harness, &mut sql, false, esc()));
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
