//! The DDL view: a table's or a view's definition as read-only text, in
//! the code face with the SQL editor's colours and line numbers, and where
//! it comes from. The text is selected and copied as any text is. A read
//! that failed says the database's error, and never shows part of a text.

use std::borrow::Cow;
use std::time::Duration;

use egui::{Frame, Id, Margin, Rect, UiBuilder, pos2, vec2};
use tabletist_db::ddl::Source;
use tabletist_db::{Driver, ObjectKind};

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

/// The id of the text field of the DDL view of the tab `id`.
pub fn text_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("ddl-text", tab.0, id.0))
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
        let text = ddl.text();
        let name = gettext(locale, "DDL of {name}")
            .replace("{name}", &format::display_safe(&object.object.name));
        let mut body = area;
        if !look.terminal {
            // The line over the text: where it comes from, and how long ago
            // it was read, with the key that reads it again.
            let strip = Rect::from_min_max(area.min, pos2(area.right(), area.top() + 44.0));
            body.min.y = strip.bottom();
            widgets::hline(
                ui,
                strip.x_range(),
                strip.bottom() - 0.5,
                palette.surface_hover,
            );
            let read = object.ddl_read.map(|at| at.elapsed());
            if source_line(ui, strip, ddl.source(), read, &reload, skin) {
                actions.push(Action::Refresh(tab));
            }
            // The age changes once a second, then once a minute.
            let tick = match read {
                Some(read) if read.as_secs() >= 60 => 60,
                _ => 1,
            };
            ui.ctx().request_repaint_after(Duration::from_secs(tick));
        }
        let dialect = ddl.driver.dialect();
        let id = text_id(tab, object_tab);
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
/// end of `strip`: Reload is a link. Returns whether it was clicked.
fn source_line(
    ui: &mut egui::Ui,
    strip: Rect,
    source: Source,
    read: Option<Duration>,
    reload: &crate::keymap::Written,
    (look, palette, locale): Skin,
) -> bool {
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
    clicked
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
        let scrolled = egui::ScrollArea::both()
            .id_salt(("ddl-scroll", self.id))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
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
