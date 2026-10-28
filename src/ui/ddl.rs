//! The DDL view: a table's or a view's definition as read-only text, in
//! the code face with the SQL editor's colours and line numbers, and where
//! it comes from. In the looks with a pointer the text is a field, selected
//! and copied as any text is, with links to its parts over it; in the
//! terminal look it is lines with a cursor, selected by line, beside an
//! outline of its parts. A read that failed says the database's error, and
//! never shows part of a text.

use std::borrow::Cow;
use std::time::Duration;

use egui::{
    CornerRadius, Frame, Id, Margin, Rect, Sense, UiBuilder, WidgetInfo, WidgetType, pos2, vec2,
};
use tabletist_db::ddl::{PartKind, Source};
use tabletist_db::{Ddl, Driver, ObjectKind};

use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::keymap::Command;
use crate::model::{Action, ConnTabId, ObjectTab, TabId};
use crate::theme::{self, Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::{focus, format, sql_text, states, widgets};

/// The text's role, and its line numbers': the SQL editor's.
fn role(look: &Look) -> TextRole {
    TextRole::pick(look, TextRole::Code, TextRole::OCode)
}

/// The id of the text field of the DDL view of the tab `id`, and of the
/// terminal look's lines in its place.
pub fn text_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("ddl-text", tab.0, id.0))
}

/// What a part of the text is called where the view lists it.
fn part_name(kind: PartKind, object: ObjectKind, locale: Locale) -> Cow<'static, str> {
    let name = match kind {
        PartKind::Object => match object {
            ObjectKind::Table => "Table",
            ObjectKind::View => "View",
            ObjectKind::MaterializedView => "Materialized view",
        },
        PartKind::Indexes => "Indexes",
        PartKind::Triggers => "Triggers",
        PartKind::Comments => "Comments",
    };
    gettext(locale, name)
}

/// How much a part holds, where the view counts it: the statements of
/// the indexes, the triggers and the comments. The object's is one
/// statement, and the outline counts its lines instead.
fn part_count(ddl: &Ddl, kind: PartKind) -> Option<usize> {
    match kind {
        PartKind::Object => None,
        PartKind::Indexes | PartKind::Triggers | PartKind::Comments => {
            ddl.part(kind).map(|part| part.statements.len())
        }
    }
}

/// The part `line` is in: the last that starts at or before it.
fn part_at(placed: &[(PartKind, std::ops::Range<usize>)], line: usize) -> Option<PartKind> {
    placed
        .iter()
        .rev()
        .find(|(_, lines)| lines.start <= line)
        .or(placed.first())
        .map(|(kind, _)| *kind)
}

/// Copies the text of the DDL view of the tab `id`: what is selected in
/// it (the field's selection, or the terminal look's lines), or else all
/// of it, and says so. Nothing before the text is read.
pub fn copy(app: &App, ctx: &egui::Context, tab: ConnTabId, id: TabId) {
    let Some(object) = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
    else {
        return;
    };
    // Only the text on screen: not one a failed Reload keeps under its
    // error.
    let Some(ddl) = object.ddl_shown() else {
        return;
    };
    let text = ddl.text();
    let part = match app.look.terminal {
        true => object.ddl_selected().map(|lines| {
            text.lines()
                .skip(lines.start)
                .take(lines.len())
                .collect::<Vec<_>>()
                .join("\n")
        }),
        false => selected_text(ctx, text_id(tab, id), &text),
    };
    let locale = app.locale;
    let said = match &part {
        Some(part) => {
            let lines = part.lines().count().max(1);
            let count = u32::try_from(lines).unwrap_or(u32::MAX);
            crate::i18n::ngettext(locale, "Copied {n} line", "Copied {n} lines", count)
                .replace("{n}", &lines.to_string())
        }
        None => gettext(locale, "Copied DDL").into_owned(),
    };
    ctx.copy_text(part.unwrap_or(text));
    crate::ui::toast::say(ctx, &app.look.label(&said));
}

/// What the field `id` has selected of `text`, where it has anything.
fn selected_text(ctx: &egui::Context, id: Id, text: &str) -> Option<String> {
    let range = egui::TextEdit::load_state(ctx, id)?.cursor.char_range()?;
    let (one, other) = (range.primary.index.0, range.secondary.index.0);
    let (from, to) = (one.min(other), one.max(other));
    (from < to).then(|| text.chars().skip(from).take(to - from).collect())
}

/// What a click on the header's buttons asked for.
#[derive(Default)]
pub struct Asked {
    pub copy: bool,
    pub open: bool,
}

/// How the header's buttons draw: as the header's room runs short they
/// drop their keys, then their words (a screen reader still reads them),
/// before its title is cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Buttons {
    Keyed,
    Words,
    Icons,
}

impl Buttons {
    pub const ALL: [Buttons; 3] = [Buttons::Keyed, Buttons::Words, Buttons::Icons];
}

/// The macOS header's two buttons in the DDL view, Copy and Open in SQL
/// editor, drawn as `form` says.
fn header_buttons<'a>(
    words: &'a (String, String),
    keys: &'a (crate::keymap::Written, crate::keymap::Written),
    form: Buttons,
) -> [widgets::ButtonSpec<'a>; 2] {
    let shown = |word: &'a String| match form {
        Buttons::Icons => "",
        Buttons::Keyed | Buttons::Words => word.as_str(),
    };
    let copy = widgets::ButtonSpec::new(shown(&words.0))
        .label(&words.0)
        .icon(Icon::Copy)
        .role(TextRole::UiBodyStrong);
    let open = widgets::ButtonSpec::new(shown(&words.1))
        .label(&words.1)
        .icon(Icon::Code)
        .primary();
    match form {
        Buttons::Keyed => [copy.shortcut(&keys.0), open.shortcut(&keys.1)],
        Buttons::Words => [copy, open],
        Buttons::Icons => [copy.gap(0.0), open.gap(0.0)],
    }
}

/// The words and keys of the header's buttons.
fn header_words(
    ctx: &egui::Context,
    look: &Look,
    locale: Locale,
) -> (
    (String, String),
    (crate::keymap::Written, crate::keymap::Written),
) {
    (
        (
            gettext(locale, "Copy").into_owned(),
            gettext(locale, "Open in SQL editor").into_owned(),
        ),
        (
            crate::ui::keys::written(ctx, look, Command::CopyDdl),
            crate::ui::keys::written(ctx, look, Command::OpenDdlInEditor),
        ),
    )
}

/// How wide the header's buttons draw, with the 12 between them.
pub fn header_buttons_width(ui: &egui::Ui, look: &Look, locale: Locale, form: Buttons) -> f32 {
    let (words, keys) = header_words(ui.ctx(), look, locale);
    let [copy, open] = header_buttons(&words, &keys, form);
    copy.width(ui, look) + 12.0 + open.width(ui, look)
}

/// The header's buttons, ending at `right`, centred on `center`.
pub fn show_header_buttons(
    ui: &mut egui::Ui,
    right: f32,
    center: f32,
    form: Buttons,
    skin: Skin,
) -> Asked {
    let (look, palette, locale) = skin;
    let (words, keys) = header_words(ui.ctx(), look, locale);
    let [copy, open] = header_buttons(&words, &keys, form);
    let (copy_width, open_width) = (copy.width(ui, look), open.width(ui, look));
    let place = |x: f32, width: f32| Rect::from_min_size(pos2(x, center - 16.0), vec2(width, 32.0));
    let open_at = place(right - open_width, open_width);
    let copy_at = place(open_at.left() - 12.0 - copy_width, copy_width);
    Asked {
        copy: copy.show_at(ui, copy_at, look, palette).clicked(),
        open: open.show_at(ui, open_at, look, palette).clicked(),
    }
}

/// Where the text comes from, as the view says it: built from the
/// catalog, or as the database keeps it.
fn source_words(source: Source, locale: Locale) -> Cow<'static, str> {
    match source {
        Source::Catalog => gettext(locale, "built from the catalog"),
        Source::Stored => gettext(locale, "as stored in the database"),
    }
}

/// The engine as the footer names it.
fn engine(driver: Driver) -> &'static str {
    match driver {
        Driver::Postgres => "PostgreSQL",
        Driver::MySql => "MySQL",
        Driver::Sqlite => "SQLite",
    }
}

/// What the text leaves out, where the app builds it: what PostgreSQL's
/// catalog holds of the table besides its definition.
fn left_out(driver: Driver, locale: Locale) -> Option<Cow<'static, str>> {
    match driver {
        Driver::Postgres => Some(gettext(
            locale,
            "Not included: owner, grants, sequences owned by the table, row-level policies",
        )),
        Driver::MySql | Driver::Sqlite => None,
    }
}

/// "read just now", "read 12 s ago", "read 3 min ago", "read 2 h ago":
/// how long ago the definition arrived.
pub fn read_ago(waited: Duration, locale: Locale) -> String {
    let seconds = waited.as_secs();
    let ago = |count: u64, unit: &str| {
        gettext(locale, "read {count} {unit} ago")
            .replace("{count}", &count.to_string())
            .replace("{unit}", unit)
    };
    match seconds {
        0 => gettext(locale, "read just now").into_owned(),
        1..60 => ago(seconds, "s"),
        60..3600 => ago(seconds / 60, "min"),
        _ => ago(seconds / 3600, "h"),
    }
}

/// The terminal header's summary in the DDL view: "ddl · read-only ·
/// built from the catalog · postgresql 17.2", as far as it is known.
pub fn subtitle(object: &ObjectTab, locale: Locale) -> Vec<String> {
    let mut parts = vec![
        gettext(locale, "ddl").into_owned(),
        gettext(locale, "read-only").into_owned(),
    ];
    if let Some(ddl) = &object.ddl.value {
        parts.push(source_words(ddl.source(), locale).into_owned());
        parts.push(ddl.server.to_lowercase());
    }
    parts
}

/// Whether the read has gone on long enough to say so: the header says it
/// then, and the view shows where the text will be. A repaint is asked
/// for until it has.
fn waited_long(ui: &egui::Ui, waited: Option<Duration>) -> bool {
    let Some(waited) = waited else {
        return false;
    };
    if states::lasted(waited) {
        return true;
    }
    ui.ctx().request_repaint_after(states::DELAY - waited);
    false
}

/// How wide [`header_wait`] paints, for the header to keep the title
/// clear of it: 0 while nothing is said.
pub fn header_wait_width(ui: &egui::Ui, waited: Option<Duration>, skin: Skin) -> f32 {
    let (look, palette, locale) = skin;
    if !waited.is_some_and(states::lasted) {
        return 0.0;
    }
    let text = gettext(locale, "Reading the definition…");
    let said = Text::one(look, widgets::secondary(look), &text, palette.secondary);
    widgets::measure(ui, said) + 8.0 + 14.0
}

/// The macOS header's end in the DDL view while the definition is read: a
/// spinner and "Reading the definition…", ending at `right`. Its width,
/// or 0 when nothing is said.
pub fn header_wait(
    ui: &egui::Ui,
    waited: Option<Duration>,
    right: f32,
    center: f32,
    skin: Skin,
) -> f32 {
    let (look, palette, locale) = skin;
    if !waited_long(ui, waited) {
        return 0.0;
    }
    let text = gettext(locale, "Reading the definition…");
    let role = widgets::secondary(look);
    let laid = Text::one(look, role, &text, palette.secondary).layout(ui.ctx());
    let width = laid.paint_right(ui.painter(), right, center);
    let left = right - width - 8.0 - 14.0;
    let ring = Rect::from_center_size(pos2(left + 7.0, center), vec2(14.0, 14.0));
    states::spinner(ui, ring, palette.accent, palette);
    let named = Rect::from_min_max(pos2(left, center - 8.0), pos2(right, center + 8.0));
    widgets::announce_polite(ui, named, &text);
    width + 8.0 + 14.0
}

/// The look, the palette and the locale, as the view's parts take them.
pub type Skin<'a> = (&'a Look, &'a Palette, Locale);

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let skin = (&look, &palette, locale);
    let reload = crate::ui::keys::written(ui.ctx(), &look, Command::Reload);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let mut actions = Vec::new();
    let area = ui.max_rect();
    focus::region(ui, focus::Region::Grid, area);
    // A view that holds nothing and waits for nothing reads its
    // definition: one forgotten while its tab was behind (a new session, a
    // Reload elsewhere) is read when the tab comes in front again.
    if object.ddl.needs_load() && !object.ddl.asked {
        actions.push(Action::RetryDdl { tab, object_tab });
    }
    if let Some(error) = object.ddl.shown_error() {
        let failed = Failed {
            error,
            name: &object.object.name,
            driver: workspace.driver,
            kind: object.kind,
        };
        Frame::new()
            .inner_margin(Margin::same(theme::PAD))
            .show(ui, |ui| {
                if failed.show(ui, &reload, skin) {
                    actions.push(Action::RetryDdl { tab, object_tab });
                }
            });
    } else if let Some(ddl) = &object.ddl.value {
        let (text, placed) = ddl.laid_out();
        let name = gettext(locale, "DDL of {name}")
            .replace("{name}", &format::display_safe(&object.object.name));
        let id = text_id(tab, object_tab);
        let dialect = ddl.driver.dialect();
        if look.terminal {
            let lines = Lines {
                id,
                name: &name,
                dialect,
                cursor: object.ddl_cursor,
                selected: object.ddl_selected(),
            };
            let outline = Outline {
                ddl,
                placed: &placed,
                object: object.kind,
                cursor: object.ddl_cursor,
            };
            let (body, side) = match area.width() >= 700.0 {
                true => {
                    let side =
                        Rect::from_min_max(pos2(area.right() - OUTLINE, area.top()), area.max);
                    (
                        Rect::from_min_max(area.min, pos2(side.left(), area.bottom())),
                        Some(side),
                    )
                }
                false => (area, None),
            };
            if let Some(line) = lines.show(ui, body, &text, skin) {
                actions.push(Action::DdlCursorTo {
                    tab,
                    id: object_tab,
                    line,
                });
                actions.push(Action::GridKeys(tab));
            }
            if let Some(side) = side
                && let Some(line) = outline.show(ui, side, skin)
            {
                actions.push(Action::DdlCursorTo {
                    tab,
                    id: object_tab,
                    line,
                });
            }
            if lines_gained_keys(ui, id) {
                actions.push(Action::GridKeys(tab));
            }
            app.actions.extend(actions);
            return;
        }
        // The line over the text: where it comes from, and how long ago it
        // was read, with the key that reads it again; the links to its parts
        // before it.
        let mut body = area;
        let strip = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 44.0));
        body.min.y = strip.bottom();
        widgets::hline(
            ui,
            strip.x_range(),
            strip.bottom() - 0.5,
            palette.surface_hover,
        );
        let read = object.ddl_read.map(|at| at.elapsed());
        let source_left = source_line(ui, strip, ddl.source(), read, &reload, skin);
        if source_left.1 {
            actions.push(Action::Refresh(tab));
        }
        // Links to the parts, before the source line: the part at the
        // top of the text is marked.
        let first: usize = ui
            .data(|data| data.get_temp(id.with("first-line")))
            .unwrap_or(0);
        let links = Links {
            ddl,
            placed: &placed,
            object: object.kind,
            current: part_at(&placed, first),
        };
        let room = Rect::from_min_max(strip.min, pos2(source_left.0 - 16.0, strip.bottom()));
        if let Some(line) = links.show(ui, room, skin) {
            ui.data_mut(|data| data.insert_temp(id.with("jump"), line));
        }
        // The age changes once a second, then once a minute.
        let tick = match read {
            Some(read) if read.as_secs() >= 60 => 60,
            _ => 1,
        };
        ui.ctx().request_repaint_after(Duration::from_secs(tick));
        let field = Field {
            id,
            name: &name,
            dialect,
        };
        field.show(ui, body, &text, skin);
    } else if waited_long(ui, object.ddl.running_for()) {
        let text = gettext(locale, "Reading the definition…");
        if look.terminal {
            // One line: a spinner, and what it waits for.
            let text = text.to_lowercase();
            let center = area.top() + f32::from(theme::PAD) + 8.0;
            let left = area.left() + f32::from(theme::PAD);
            let ring = Rect::from_center_size(pos2(left + 6.0, center), vec2(12.0, 12.0));
            states::spinner(ui, ring, palette.accent, &palette);
            let said = Text::one(&look, TextRole::OBody, &text, palette.dim);
            let width = widgets::paint_text(ui, left + 20.0, center, said);
            let named = Rect::from_min_size(pos2(left, center - 8.0), vec2(width + 20.0, 16.0));
            widgets::announce_polite(ui, named, &text);
        } else {
            // The shape of the text without it: the header says what is
            // read.
            let lines = area.shrink2(vec2(10.0, 14.0));
            grey_lines(ui, lines, &palette);
        }
    }
    app.actions.extend(actions);
}

/// Lines of grey bars where the text will be: a gutter's number and the
/// line, of the widths a definition's lines have.
fn grey_lines(ui: &egui::Ui, rect: Rect, palette: &Palette) {
    const SHARES: [f32; 8] = [0.45, 0.3, 0.55, 0.4, 0.6, 0.35, 0.5, 0.2];
    let fill = palette.surface.gamma_multiply(0.7);
    let pitch = 20.0;
    let rows = ((rect.height() / pitch) as usize).min(SHARES.len());
    for (row, share) in SHARES.iter().enumerate().take(rows) {
        let center = rect.top() + pitch * (row as f32 + 0.5);
        let number = Rect::from_min_size(pos2(rect.left() + 18.0, center - 5.0), vec2(14.0, 10.0));
        let room = (rect.width() - 58.0).max(0.0);
        let line = Rect::from_min_size(
            pos2(rect.left() + 58.0, center - 5.0),
            vec2(room * share, 10.0),
        );
        for bar in [number, line] {
            ui.painter()
                .rect_filled(bar, egui::CornerRadius::same(5), fill);
        }
    }
}

/// "Read-only · built from the catalog · read 2 s ago · Reload ⌘R" at the
/// end of `strip`: Reload is a link. Returns where it starts, and whether
/// Reload was clicked.
fn source_line(
    ui: &mut egui::Ui,
    strip: Rect,
    source: Source,
    read: Option<Duration>,
    reload: &crate::keymap::Written,
    (look, palette, locale): Skin,
) -> (f32, bool) {
    let side = 24.0;
    let center = strip.center().y;
    let role = widgets::secondary(look);
    let mut words = vec![
        gettext(locale, "Read-only").into_owned(),
        source_words(source, locale).into_owned(),
    ];
    if let Some(read) = read {
        words.push(read_ago(read, locale));
    }
    let said = format!("{} · ", words.join(" · "));
    let label = gettext(locale, "Reload");
    let link = widgets::ButtonSpec::new(&label)
        .link()
        .shortcut(reload)
        .padding(4.0)
        .role(role)
        .shortcut_role(role);
    let width = link.width(ui, look);
    let at = Rect::from_min_size(
        pos2(strip.right() - side - width, center - 12.0),
        vec2(width, 24.0),
    );
    let clicked = link.show_at(ui, at, look, palette).clicked();
    let text = Text::one(look, role, &said, palette.dim);
    let laid = text.layout(ui.ctx());
    let left = at.left() - laid.paint_right(ui.painter(), at.left(), center);
    let named = Rect::from_min_max(pos2(left, center - 8.0), pos2(at.left(), center + 8.0));
    widgets::announce(ui, named, &words.join(" · "));
    (left, clicked)
}

/// The macOS links to the parts of the text, over it: Table, then each
/// other part with how many statements it holds, the one at the top of the
/// text marked.
struct Links<'a> {
    ddl: &'a Ddl,
    placed: &'a [(PartKind, std::ops::Range<usize>)],
    object: ObjectKind,
    current: Option<PartKind>,
}

impl Links<'_> {
    /// The links in `room`, from its left, as many as fit. Returns the
    /// line of the part whose link was clicked.
    fn show(&self, ui: &mut egui::Ui, room: Rect, (look, palette, locale): Skin) -> Option<usize> {
        let center = room.center().y;
        let role = widgets::body(look);
        let mut x = room.left() + 24.0;
        let mut asked = None;
        for (kind, lines) in self.placed {
            let name = part_name(*kind, self.object, locale);
            let count = part_count(self.ddl, *kind).map(|count| count.to_string());
            let current = self.current == Some(*kind);
            let (ink, faint) = match current {
                true => (palette.window, palette.window),
                false => (palette.accent, palette.dim),
            };
            let mut text = Text::new(look).add(role, &name, ink);
            if let Some(count) = &count {
                text = text.space(role, " ").add(role, count, faint);
            }
            let laid = text.layout(ui.ctx());
            let width = laid.width() + 20.0;
            if x + width > room.right() {
                break;
            }
            let rect = Rect::from_min_size(pos2(x, center - 13.0), vec2(width, 26.0));
            let label = match &count {
                Some(count) => format!("{name} {count}"),
                None => name.to_string(),
            };
            let response = ui.interact(
                rect,
                ui.id().with(("ddl-link", label.as_str())),
                Sense::click(),
            );
            response.widget_info(|| WidgetInfo::selected(WidgetType::Link, true, current, &label));
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_selected(current));
            let fill = match (current, response.hovered()) {
                (true, _) => Some(palette.text),
                (false, true) => Some(palette.surface_hover),
                (false, false) => None,
            };
            if let Some(fill) = fill {
                ui.painter().rect_filled(
                    rect,
                    CornerRadius::same(look.radius.saturating_sub(2)),
                    fill,
                );
            }
            laid.paint_left(ui.painter(), x + 10.0, center);
            if response.clicked() {
                asked = Some(lines.start);
            }
            x += width + 6.0;
        }
        asked
    }
}

/// The terminal look's outline beside the text: its width.
const OUTLINE: f32 = 300.0;

/// Whether the terminal look's lines just got the keyboard, by F6 or a
/// click: the rows' pane keys follow it.
fn lines_gained_keys(ui: &egui::Ui, id: Id) -> bool {
    let had = id.with("had-keyboard");
    let has = ui.memory(|memory| memory.has_focus(id));
    let came = has && !ui.data(|data| data.get_temp::<bool>(had)).unwrap_or(false);
    ui.data_mut(|data| data.insert_temp(had, has));
    came
}

/// The terminal look's text: lines with a cursor and a selection of
/// whole lines, beside a gutter of their numbers. One stop of the
/// keyboard, as the Structure view's columns are: the view's keys move in
/// it.
struct Lines<'a> {
    id: Id,
    name: &'a str,
    dialect: tabletist_db::Dialect,
    cursor: usize,
    selected: Option<std::ops::Range<usize>>,
}

impl Lines<'_> {
    /// `text` in `rect`. Returns the line a click put the cursor on.
    fn show(
        &self,
        ui: &mut egui::Ui,
        rect: Rect,
        text: &str,
        (look, palette, _): Skin,
    ) -> Option<usize> {
        let role = role(look);
        let lines = text.lines().count().max(1);
        let digits = lines.ilog10() as usize + 1;
        let digit = role.width(ui.ctx(), look.faces, "0");
        let gutter = (digits as f32 * digit + 14.0 + 8.0).ceil().max(52.0);
        let top = 10.0;
        let mut layouter = sql_text::layouter(look, palette, self.dialect, self.id, false);
        let galley = layouter(ui, &text, f32::INFINITY);
        let text_pane = Rect::from_min_max(pos2(rect.left() + gutter, rect.top()), rect.max);
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(text_pane));
        child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
        // The line the cursor was last brought into view on: a cursor that
        // moved is brought into view, at once.
        let seen = self.id.with("seen");
        let scrolled = egui::ScrollArea::both()
            .id_salt(("ddl-lines", self.id))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                let size = galley.size() + vec2(16.0, top + 16.0);
                let (area, _) = ui.allocate_exact_size(size, Sense::hover());
                let origin = area.min + vec2(0.0, top);
                let row = |line: usize| {
                    galley.rows.get(line).map(|row| {
                        let y = origin.y + row.min_y();
                        egui::Rangef::new(y, y + row.height())
                    })
                };
                let was: Option<usize> = ui.data(|data| data.get_temp(seen));
                if was != Some(self.cursor)
                    && let Some(y) = row(self.cursor)
                {
                    ui.data_mut(|data| data.insert_temp(seen, self.cursor));
                    let across = ui.clip_rect().x_range();
                    let jump = egui::style::ScrollAnimation::none();
                    ui.scroll_to_rect_animation(Rect::from_x_y_ranges(across, y), None, jump);
                }
                (
                    origin,
                    galley
                        .rows
                        .iter()
                        .map(|row| row.height())
                        .next()
                        .unwrap_or(16.0),
                )
            });
        let (origin, height) = scrolled.inner;
        let clip = rect.intersect(ui.clip_rect());
        let painter = ui.painter().with_clip_rect(clip);
        let band = |line: usize| {
            let y = origin.y + line as f32 * height;
            Rect::from_x_y_ranges(rect.x_range(), egui::Rangef::new(y, y + height))
        };
        // Behind the text: the selected lines, then the cursor's, with a
        // bar at its left. Painted before the text, whose glyphs stand
        // over them.
        if let Some(selected) = &self.selected {
            for line in selected.clone() {
                painter.rect_filled(band(line), CornerRadius::ZERO, palette.surface_active);
            }
        }
        let cursor = band(self.cursor);
        painter.rect_filled(cursor, CornerRadius::ZERO, palette.selection);
        painter.rect_filled(
            Rect::from_min_size(cursor.min, vec2(3.0, cursor.height())),
            CornerRadius::ZERO,
            palette.accent,
        );
        ui.painter()
            .with_clip_rect(text_pane.intersect(ui.clip_rect()))
            .galley(origin, galley.clone(), palette.text);
        // The numbers of the lines in sight; the cursor's stands out.
        let first = ((clip.top() - origin.y) / height).floor().max(0.0) as usize;
        for line in first..lines {
            let y = origin.y + line as f32 * height;
            if y > clip.bottom() {
                break;
            }
            let color = if line == self.cursor {
                palette.warning
            } else {
                palette.dim
            };
            let number = Text::one(look, role, &(line + 1).to_string(), color).layout(ui.ctx());
            number.paint_right(&painter, rect.left() + gutter - 14.0, y + height / 2.0);
        }
        // One stop for the keyboard, named as the text and holding it, for
        // a screen reader. A click puts the cursor on the line under it.
        let stop = ui.interact(clip, self.id, Sense::click() | Sense::FOCUSABLE);
        ui.ctx().accesskit_node_builder(stop.id, |node| {
            node.set_role(egui::accesskit::Role::MultilineTextInput);
            node.set_label(self.name);
            node.set_value(text);
            node.set_read_only();
        });
        focus::pane(ui, &stop);
        focus::hint(ui, &stop, clip, focus::Ring::Own);
        focus::claim(ui, focus::Region::Grid, &stop);
        let clicked = stop
            .clicked()
            .then(|| stop.interact_pointer_pos())
            .flatten();
        clicked.map(|at| (((at.y - origin.y) / height).floor().max(0.0) as usize).min(lines - 1))
    }
}

/// The terminal look's outline of the text: its parts, the cursor's
/// marked, what is not in it, and the view's keys.
struct Outline<'a> {
    ddl: &'a Ddl,
    placed: &'a [(PartKind, std::ops::Range<usize>)],
    object: ObjectKind,
    cursor: usize,
}

impl Outline<'_> {
    /// The outline in `rect`. Returns the line of a part that was clicked.
    fn show(&self, ui: &mut egui::Ui, rect: Rect, (look, palette, locale): Skin) -> Option<usize> {
        ui.painter()
            .rect_filled(rect, CornerRadius::ZERO, palette.panel);
        widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.outline);
        let role = TextRole::OBody;
        let pitch = 22.0;
        let left = rect.left() + 14.0;
        let right = rect.right() - 14.0;
        let mut y = rect.top() + 12.0 + pitch / 2.0;
        let key = |command| crate::ui::keys::written(ui.ctx(), look, command);
        let (next, previous) = (
            key(Command::NextDdlSection),
            key(Command::PreviousDdlSection),
        );
        let heading = gettext(locale, "sections · {next} next  {previous} prev")
            .replace("{next}", &next)
            .replace("{previous}", &previous);
        widgets::paint_label(ui, left, y, Text::one(look, role, &heading, palette.dim));
        y += pitch + 6.0;
        let current = part_at(self.placed, self.cursor);
        let mut asked = None;
        for (kind, lines) in self.placed {
            let name = part_name(*kind, self.object, locale).to_lowercase();
            let count = part_count(self.ddl, *kind).unwrap_or(lines.len());
            let on = current == Some(*kind);
            let color = if on { palette.accent } else { palette.text };
            let row = Rect::from_x_y_ranges(
                rect.x_range(),
                egui::Rangef::new(y - pitch / 2.0, y + pitch / 2.0),
            );
            let label = format!("{name} {count}");
            let response = ui.interact(
                row,
                ui.id().with(("ddl-outline", label.as_str())),
                Sense::click(),
            );
            response.widget_info(|| WidgetInfo::selected(WidgetType::Button, true, on, &label));
            ui.ctx()
                .accesskit_node_builder(response.id, |node| node.set_selected(on));
            let mark = if on { "▌ " } else { "  " };
            widgets::paint_text(
                ui,
                left,
                y,
                Text::one(look, role, &format!("{mark}{name}"), color),
            );
            widgets::paint_text_right(
                ui,
                right,
                y,
                Text::one(look, role, &count.to_string(), color),
            );
            if response.clicked() {
                asked = Some(lines.start);
            }
            y += pitch;
        }
        if let Some(left_out) = left_out_lines(self.ddl.driver, locale) {
            y += 14.0;
            widgets::paint_label(
                ui,
                left,
                y,
                Text::one(look, role, &gettext(locale, "not included"), palette.dim),
            );
            y += pitch;
            for line in left_out {
                widgets::paint_label(ui, left, y, Text::one(look, role, &line, palette.dim));
                y += pitch;
            }
        }
        y += 14.0;
        widgets::paint_label(
            ui,
            left,
            y,
            Text::one(look, role, &gettext(locale, "keys"), palette.dim),
        );
        y += pitch;
        let (copy, lines) = (key(Command::CopyDdl), key(Command::SelectDdlLines));
        let keys = [
            (copy.to_string(), "copy all"),
            (format!("{} {}", &*lines, &*copy), "copy lines"),
            (
                key(Command::OpenDdlInEditor).to_string(),
                "open in sql editor",
            ),
            (
                format!("{} {}", &*key(Command::DdlTop), &*key(Command::DdlBottom)),
                "top · bottom",
            ),
            (key(Command::OpenDdl).to_string(), "open from anywhere"),
            (key(Command::Reload).to_string(), "read again"),
        ];
        let column = keys
            .iter()
            .map(|(keys, _)| TextRole::OBody.width(ui.ctx(), look.faces, keys))
            .fold(0.0, f32::max)
            + 16.0;
        for (keys, word) in keys {
            if keys.trim().is_empty() {
                continue;
            }
            let word = gettext(locale, word);
            let said = Text::new(look).add(TextRole::OGroup, &keys, palette.accent);
            widgets::paint_text(ui, left, y, said);
            let width = widgets::paint_text(
                ui,
                left + column,
                y,
                Text::one(look, role, &word, palette.text),
            );
            // The key with what it does, for a screen reader: one line.
            let place = Rect::from_min_size(pos2(left, y - 8.0), vec2(column + width, 16.0));
            widgets::announce(ui, place, &format!("{keys} {word}"));
            y += pitch;
        }
        asked
    }
}

/// What PostgreSQL's text leaves out, as the outline lists it.
fn left_out_lines(driver: Driver, locale: Locale) -> Option<[String; 2]> {
    match driver {
        Driver::Postgres => Some([
            gettext(locale, "owner · grants · owned sequences").into_owned(),
            gettext(locale, "row-level policies").into_owned(),
        ]),
        Driver::MySql | Driver::Sqlite => None,
    }
}

/// The macOS footer of the DDL view: how many lines and whose, what the
/// text leaves out, and how to copy part of it.
pub fn footer(app: &App, ui: &mut egui::Ui, tab: ConnTabId, object_tab: TabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(object) = workspace.object_tab(object_tab) else {
        return;
    };
    let driver = workspace.driver;
    let lines = object
        .ddl
        .value
        .as_ref()
        .filter(|_| object.ddl.shown_error().is_none())
        .map(|ddl| ddl.text().lines().count());
    egui::Panel::bottom(Id::new(("ddl-footer", tab.0, object_tab.0)))
        .exact_size(36.0)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.surface_hover);
            let Some(lines) = lines else {
                return;
            };
            let center = rect.center().y;
            let role = widgets::secondary(&look);
            let count = u32::try_from(lines).unwrap_or(u32::MAX);
            let count = crate::i18n::ngettext(locale, "{n} line", "{n} lines", count)
                .replace("{n}", &lines.to_string());
            let count = format!("{count} · {}", engine(driver));
            let mut x = rect.left() + 24.0;
            x += widgets::paint_label(ui, x, center, Text::one(&look, role, &count, palette.dim));
            if let Some(left_out) = left_out(driver, locale) {
                let said = Text::one(&look, role, &left_out, palette.dim);
                x += 12.0 + widgets::paint_label(ui, x + 12.0, center, said);
            }
            // Where there is room for it after the rest.
            let hint = gettext(locale, "Select text to copy part of it");
            let hint = || Text::one(&look, role, &hint, palette.dim);
            let width = widgets::measure(ui, hint());
            if x + 24.0 + width < rect.right() - 24.0 {
                let right = rect.right() - 24.0;
                widgets::paint_text_right(ui, right, center, hint());
                let at = Rect::from_min_max(
                    pos2(right - width, center - 8.0),
                    pos2(right, center + 8.0),
                );
                widgets::announce(ui, at, &gettext(locale, "Select text to copy part of it"));
            }
        });
}

/// A read that failed: whose definition, the database's code and words,
/// what it means for the view, and the ways on.
struct Failed<'a> {
    error: &'a tabletist_db::Error,
    name: &'a str,
    driver: Driver,
    kind: ObjectKind,
}

impl Failed<'_> {
    /// "1142 · SHOW command denied …": MySQL's number where it gives one,
    /// else the code, then the database's words.
    fn said(&self) -> String {
        match self.error {
            tabletist_db::Error::Query {
                code,
                message,
                named,
                ..
            } => {
                let code = named
                    .number
                    .map(|number| number.to_string())
                    .or_else(|| code.clone());
                let message = format::capped(message);
                match code {
                    Some(code) => format!("{code} · {message}"),
                    None => message.into_owned(),
                }
            }
            other => format::capped(&other.to_string()).into_owned(),
        }
    }

    /// Whether the database refused the read to the connection's user:
    /// MySQL's numbers for a privilege it lacks, PostgreSQL's
    /// `insufficient_privilege`. SQLite has no users.
    fn refused(&self) -> bool {
        let tabletist_db::Error::Query { code, named, .. } = self.error else {
            return false;
        };
        match self.driver {
            Driver::MySql => matches!(named.number, Some(1044 | 1142 | 1143 | 1227)),
            Driver::Postgres => code.as_deref() == Some("42501"),
            Driver::Sqlite => false,
        }
    }

    /// What the error means for the view, where the database refused the
    /// read: what the user may not do, and that the rest of the tab still
    /// works. Nothing for any other error.
    fn meaning(&self, locale: Locale) -> Option<String> {
        if !self.refused() {
            return None;
        }
        let words = match (self.driver, self.kind) {
            (Driver::MySql, ObjectKind::View) => {
                "The connection's user can't run SHOW CREATE VIEW here. Rows and Structure still work."
            }
            (Driver::MySql, ObjectKind::Table | ObjectKind::MaterializedView) => {
                "The connection's user can't run SHOW CREATE TABLE here. Rows and Structure still work."
            }
            (Driver::Postgres | Driver::Sqlite, _) => {
                "The connection's user can't read this part of the catalog. Rows and Structure still work."
            }
        };
        Some(gettext(locale, words).into_owned())
    }

    /// All of it, for the clipboard: the code and the words, and what
    /// else the database said, each under its name.
    fn details(&self, locale: Locale) -> String {
        let mut details = self.said();
        if let tabletist_db::Error::Query { detail, hint, .. } = self.error {
            for (label, said) in [("Detail", detail), ("Hint", hint)] {
                if let Some(said) = said {
                    details.push_str(&format!("\n{}: {said}", gettext(locale, label)));
                }
            }
        }
        details
    }

    /// The card, and under it Try again and Copy error (the terminal's
    /// retry is its key). Returns whether Try again was asked for.
    fn show(&self, ui: &mut egui::Ui, reload: &crate::keymap::Written, skin: Skin) -> bool {
        let (look, palette, locale) = skin;
        let title = gettext(locale, "Couldn't read the definition of {name}")
            .replace("{name}", &format::display_safe(self.name));
        let title = look.label(&title);
        let said = self.said();
        let mut retried = false;
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing = vec2(8.0, 10.0);
            let card = states::Card {
                tone: states::Tone::Danger,
                icon: Icon::CircleAlert,
                title: &title,
                text: &said,
            };
            states::card(ui, &card, look, palette);
            if let Some(meaning) = self.meaning(locale) {
                let width = ui.available_width();
                Text::one(look, widgets::body(look), &meaning, palette.secondary)
                    .wrap(width)
                    .layout(ui.ctx())
                    .label(ui);
            }
            // Said once, as it appears: the title and the database's words.
            let rect = ui.min_rect();
            widgets::announce_polite(ui, rect, &format!("{title}. {said}"));
            let height = states::button_height(look);
            ui.horizontal(|ui| {
                let again = look.label(&gettext(locale, "Try again"));
                let again = if look.terminal {
                    states::key_button(&again, reload, look)
                } else {
                    states::button(&again, look)
                };
                if again
                    .label("Try again")
                    .show(ui, height, look, palette)
                    .clicked()
                {
                    retried = true;
                }
                let copy = look.label(&gettext(locale, "Copy error"));
                let copy = states::button(&copy, look).label("Copy error").quiet();
                if copy.show(ui, height, look, palette).clicked() {
                    ui.ctx().copy_text(self.details(locale));
                }
            });
        });
        retried
    }
}

/// The text's field: its id, its name for a screen reader, and the
/// dialect it is coloured in.
struct Field<'a> {
    id: Id,
    name: &'a str,
    dialect: tabletist_db::Dialect,
}

impl Field<'_> {
    /// `text` in `rect`, beside a gutter of its line numbers: read-only,
    /// selected and copied as any text, scrolled both ways (its lines do
    /// not wrap).
    fn show(&self, ui: &mut egui::Ui, rect: Rect, text: &str, (look, palette, _): Skin) {
        let role = role(look);
        let lines = text.lines().count().max(1);
        let digits = lines.ilog10() as usize + 1;
        let digit = role.width(ui.ctx(), look.faces, "0");
        // macOS: 44 pt of numbers, 14 of it after them, from 10 in. The
        // terminal: from the pane's edge.
        let inset = if look.terminal { 0.0 } else { 10.0 };
        let gutter = (digits as f32 * digit + 14.0 + 8.0).ceil().max(44.0);
        let top = if look.terminal { 8.0 } else { 14.0 };
        let text_pane =
            Rect::from_min_max(pos2(rect.left() + inset + gutter, rect.top()), rect.max);
        let mut child = ui.new_child(UiBuilder::new().id_salt(self.id).max_rect(text_pane));
        child.set_clip_rect(text_pane.intersect(ui.clip_rect()));
        // The keyboard that comes by F6 or Tab starts at the top: egui
        // puts the cursor at the end, from where nothing selects down. A
        // press puts it where it lands.
        let had = self.id.with("had-keyboard");
        let has = ui.memory(|memory| memory.has_focus(self.id));
        let came = has && !ui.data(|data| data.get_temp::<bool>(had)).unwrap_or(false);
        if came && !ui.input(|input| input.pointer.any_pressed()) {
            let mut state = egui::TextEdit::load_state(ui.ctx(), self.id).unwrap_or_default();
            let top = egui::text::CCursorRange::one(egui::text::CCursor::new(0));
            state.cursor.set_char_range(Some(top));
            egui::TextEdit::store_state(ui.ctx(), self.id, state);
        }
        ui.data_mut(|data| data.insert_temp(had, has));
        let font = role.font_id(look.faces);
        let mut layouter = sql_text::layouter(look, palette, self.dialect, self.id, false);
        // A link to a part asked for its line at the top: where the code's
        // own rows lay it.
        let jump: Option<usize> = ui.data_mut(|data| data.remove_temp(self.id.with("jump")));
        let mut area = egui::ScrollArea::both()
            .id_salt(("ddl-scroll", self.id))
            .auto_shrink([false, false]);
        if let Some(line) = jump {
            let galley = layouter(ui, &text, f32::INFINITY);
            // Under the field's margin, which shows over the first line
            // only.
            let row = galley.rows.get(line).or(galley.rows.last());
            let at = row
                .filter(|_| line > 0)
                .map_or(0.0, |row| row.min_y() + top);
            area = area.vertical_scroll_offset(at);
        }
        let scrolled = area.show(&mut child, |ui| {
            // The text the field reads and cannot change.
            let mut fixed: &str = text;
            egui::TextEdit::multiline(&mut fixed)
                .id(self.id)
                .font(font)
                .frame(Frame::new().inner_margin(Margin {
                    left: 0,
                    right: 16,
                    top: top as i8,
                    bottom: 16,
                }))
                .desired_width(f32::INFINITY)
                .desired_rows(lines)
                .layouter(&mut layouter)
                .show(ui)
        });
        let output = scrolled.inner;
        // The line at the top, for the links to mark its part. A part near
        // the end cannot come to the top: the link that asked for it stays
        // marked until the text is scrolled.
        let offset = scrolled.state.offset.y;
        let jumped = self.id.with("jumped");
        if let Some(line) = jump {
            ui.data_mut(|data| data.insert_temp(jumped, (line, offset)));
        }
        let held: Option<(usize, f32)> = ui.data(|data| data.get_temp(jumped));
        let first = match held {
            Some((line, at)) if (at - offset).abs() < 0.5 => line,
            _ => {
                ui.data_mut(|data| data.remove_temp::<(usize, f32)>(jumped));
                let rows = &output.galley.rows;
                rows.partition_point(|row| row.max_y() <= offset - top + 1.0)
            }
        };
        ui.data_mut(|data| data.insert_temp(self.id.with("first-line"), first));
        ui.ctx().accesskit_node_builder(self.id, |node| {
            node.set_label(self.name);
            node.set_read_only();
        });
        // The keyboard comes here with F6, as it goes to the rows. A field
        // that cannot be typed into draws no caret: a ring inside the
        // text's area, its numbers with it, says where the keyboard is.
        focus::claim(ui, focus::Region::Grid, &output.response);
        let ring = focus::Ring::Inset { radius: 0 };
        focus::hint(ui, &output.response, rect, ring);
        // The numbers of the lines in sight: one row each, as nothing
        // wraps.
        let clip = rect.intersect(ui.clip_rect());
        let rows = &output.galley.rows;
        let origin = output.galley_pos;
        let first = rows.partition_point(|row| origin.y + row.max_y() < clip.top());
        let right = rect.left() + inset + gutter - 14.0;
        let color = if look.terminal {
            palette.dim
        } else {
            palette.faint
        };
        let painter = ui.painter().with_clip_rect(clip);
        for (index, row) in rows.iter().enumerate().skip(first) {
            let top = origin.y + row.min_y();
            if top > clip.bottom() {
                break;
            }
            let number = Text::one(look, role, &(index + 1).to_string(), color).layout(ui.ctx());
            number.paint_right(&painter, right, top + row.height() / 2.0);
        }
    }
}
