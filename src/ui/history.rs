//! The History tab: every statement run on the window's connection, from
//! the SQL editor, a grid's save, a filter or sort set on a table, and the
//! agents, the newest first and grouped by day, with the selected one's
//! pane at the right. A read runs again, any statement opens in a SQL
//! editor, a filter is set again, and an agent's entry links to its call in
//! the Agents Log, which alone keeps what the agent sent.

use egui::{Frame, Id, Rect, pos2, vec2};

use crate::history::{Entry, Outcome, Source};
use crate::i18n::{Locale, gettext, ngettext};
use crate::keymap::Keymap;
use crate::model::{Action, ConnTabId, HistoryFilterSet, HistoryRow, HistoryTab, Workspace};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::{Paint, menu, states, widgets};

/// The macOS look's header and its row of filters; the terminal look's
/// line at the top.
const HEADER: f32 = 48.0;
const FILTERS: f32 = 52.0;
const LINE: f32 = 32.0;
/// The list's head, and a day's heading.
const HEAD: f32 = 32.0;
const DAY: f32 = 28.0;
/// The selected entry's pane, at the right.
const PANE: f32 = 360.0;
const PANE_TERMINAL: f32 = 380.0;

/// What the person said of the copy.
const COPIED: &str = crate::ui::review::COPIED_SQL;

/// What the tab says while the history is off, as its title.
pub const OFF: &str = "History is off";
/// An agent's entry whose call the Agents Log has dropped.
pub const GONE: &str = "No longer in the Agents Log";

/// The height of an entry's row.
fn row_height(look: &Look) -> f32 {
    if look.terminal { 24.0 } else { 35.0 }
}

/// The keys the tab names, as the keymap writes them for the look.
struct Keys {
    run: crate::keymap::Written,
    open: crate::keymap::Written,
    call: crate::keymap::Written,
    reapply: crate::keymap::Written,
    copy: crate::keymap::Written,
    filter: crate::keymap::Written,
    close: crate::keymap::Written,
    yes: crate::keymap::Written,
    no: crate::keymap::Written,
}

/// What the tab asks of the app, and what it puts on the clipboard.
enum Ask {
    Do(Box<Action>),
    Copy(String),
}

fn ask(action: Action) -> Ask {
    Ask::Do(Box::new(action))
}

/// Draws the History tab of `workspace`, the connection tab `tab`, and
/// returns what it asks for. `name` is the window's connection as the tab
/// names it (`App::history_name`), `keep` the entries kept per connection.
pub fn show(
    ui: &mut egui::Ui,
    paint: Paint,
    tab: ConnTabId,
    workspace: &Workspace,
    keymap: &Keymap,
    name: &str,
    keep: u32,
) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let layout = paint.layout();
    let key = |command: crate::keymap::Command| {
        keymap.key_in(layout, command, crate::keymap::Scope::History)
    };
    use crate::keymap::Command;
    let keys = Keys {
        run: key(Command::RunHistoryAgain),
        open: key(Command::OpenHistoryEntry),
        // On macOS the Agents tab's own chord, which shows the call.
        call: match look.terminal {
            true => key(Command::ShowHistoryCall),
            false => keymap.key(layout, Command::ShowAgents),
        },
        reapply: key(Command::ReapplyHistoryFilter),
        copy: key(Command::CopyHistorySql),
        filter: key(Command::FilterHistory),
        close: key(Command::CloseHistoryLine),
        yes: key(Command::ConfirmClearHistory),
        no: key(Command::KeepHistory),
    };
    let Some(history) = workspace.history_tab() else {
        return Vec::new();
    };
    let dialect = workspace.driver.dialect();
    let production = workspace.environment.is_production();
    let database = crate::session::database_of(&workspace.spec);
    let mut asks = Vec::new();
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            let view = View {
                history,
                keys: &keys,
                look: &look,
                palette: &palette,
                locale,
                name,
                keep,
                dialect,
                production,
                database: &database,
                tab,
            };
            view.draw(ui, rect, &mut asks);
        });
    let mut actions = Vec::new();
    for ask in asks {
        match ask {
            Ask::Do(action) => actions.push(*action),
            Ask::Copy(sql) => {
                ui.ctx().copy_text(sql);
                crate::ui::toast::say(ui.ctx(), &gettext(locale, COPIED));
            }
        }
    }
    actions
}

/// What the tab is drawn from.
struct View<'a> {
    history: &'a HistoryTab,
    keys: &'a Keys,
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
    /// The window's connection: "Bookshop · dev".
    name: &'a str,
    /// The entries kept per connection; 0 keeps none.
    keep: u32,
    dialect: tabletist_db::Dialect,
    production: bool,
    database: &'a str,
    tab: ConnTabId,
}

impl View<'_> {
    /// What an entry was of: its tab or its table, or an agent's name as
    /// the Agents Log says it.
    fn detail_of<'e>(&'e self, entry: &'e Entry) -> &'e str {
        match entry.source {
            Source::Agent => self.history.agent_of(entry).unwrap_or_default(),
            Source::Sql | Source::Save | Source::Filter => &entry.detail,
        }
    }

    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }

    fn draw(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) {
        let mut top = rect.top();
        if self.look.terminal {
            top = self.terminal_line(ui, rect, asks);
        } else {
            top = self.header(
                ui,
                Rect::from_min_max(pos2(rect.left(), top), rect.max),
                asks,
            );
            top = self.filters(
                ui,
                Rect::from_min_max(pos2(rect.left(), top), rect.max),
                asks,
            );
        }
        let width = match self.look.terminal {
            true => PANE_TERMINAL,
            false => PANE,
        };
        let width = width.min(rect.width() / 2.0);
        let pane = Rect::from_min_max(pos2(rect.right() - width, top), rect.max);
        let list = Rect::from_min_max(pos2(rect.left(), top), pos2(pane.left(), rect.bottom()));
        self.list(ui, list, asks);
        self.pane(ui, pane, asks);
    }

    /// The macOS look's header: whose history it is and what it holds,
    /// and Clear history…, or the question it asks.
    fn header(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) -> f32 {
        let (look, palette) = (self.look, self.palette);
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), HEADER));
        widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.border);
        let body = widgets::body(look);
        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OBody);
        let mut right = bar.right() - 16.0;
        if self.history.clearing {
            let (clear, cancel) = (self.say("Clear history"), self.say("Cancel"));
            for (label, clears) in [(&cancel, false), (&clear, true)] {
                let mut button = states::button(label, look).salt("history-clear-question");
                if clears {
                    button = button.danger();
                }
                let width = button.width(ui, look);
                let at = Rect::from_center_size(
                    pos2(right - width / 2.0, bar.center().y),
                    vec2(width, 28.0),
                );
                if button.show_at(ui, at, look, palette).clicked() {
                    asks.push(ask(Action::ConfirmClearHistory {
                        tab: self.tab,
                        clear: clears,
                    }));
                }
                right = at.left() - 8.0;
            }
            let question = self.clear_question();
            let laid = Text::one(look, body, &question, palette.text)
                .wrap((right - bar.left() - 24.0).max(40.0))
                .layout(ui.ctx());
            laid.paint_left(ui.painter(), bar.left() + 16.0, bar.center().y);
            widgets::announce(ui, bar, &question);
            return bar.bottom();
        }
        let label = self.say("Clear history…");
        let button = states::button(&label, look);
        let width = button.width(ui, look);
        let at =
            Rect::from_center_size(pos2(right - width / 2.0, bar.center().y), vec2(width, 28.0));
        if button.show_at(ui, at, look, palette).clicked() {
            asks.push(ask(Action::ClearHistory(self.tab)));
        }
        right = at.left() - 12.0;
        let of = self.say("History of");
        let what = self.say("every statement run here: SQL editor, grid saves, filters and agents");
        let room = (right - bar.left() - 16.0).max(0.0);
        let line = Text::new(look)
            .add(body, &of, palette.secondary)
            .space(body, " ")
            .add(strong, self.name, palette.text)
            .add(body, &format!(" · {what}"), palette.secondary);
        let laid = line.layout(ui.ctx());
        if laid.width() <= room {
            laid.paint_left(ui.painter(), bar.left() + 16.0, bar.center().y);
        } else {
            let line = Text::new(look)
                .add(body, &of, palette.secondary)
                .space(body, " ")
                .add(strong, self.name, palette.text);
            widgets::paint_text(ui, bar.left() + 16.0, bar.center().y, line);
        }
        widgets::announce(ui, bar, &format!("{of} {} · {what}", self.name));
        bar.bottom()
    }

    /// "Clear history for Bookshop · dev? This can't be undone…"
    fn clear_question(&self) -> String {
        let words = "Clear history for {connection}? This can't be undone. It holds the values typed into grid saves, on this computer.";
        self.look
            .label(&gettext(self.locale, words).replace("{connection}", self.name))
    }

    /// The count, and what the history keeps: "184 statements · keeps the
    /// last 1,000 · on this computer".
    fn count(&self, shown: usize) -> String {
        if self.keep == 0 {
            return self.say(OFF);
        }
        let n = u32::try_from(shown).unwrap_or(u32::MAX);
        let statements = match self.history.filter.is_set() {
            false => ngettext(self.locale, "{n} statement", "{n} statements", n),
            true => ngettext(
                self.locale,
                "{n} statement matches",
                "{n} statements match",
                n,
            ),
        }
        .replace("{n}", &crate::ui::format::group_digits(shown as u64));
        let keeps = gettext(self.locale, "keeps the last {n}").replace(
            "{n}",
            &crate::ui::format::group_digits(u64::from(self.keep)),
        );
        self.look.label(&format!(
            "{statements} · {keeps} · {}",
            gettext(self.locale, "on this computer")
        ))
    }

    /// The macOS look's filters: the source, the result, a search of the
    /// statements, and the count at the right.
    fn filters(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) -> f32 {
        let (look, palette, locale) = (self.look, self.palette, self.locale);
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), FILTERS));
        widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.border);
        let row = Rect::from_min_max(
            pos2(bar.left() + 16.0, bar.center().y - 14.0),
            pos2(bar.right() - 16.0, bar.center().y + 14.0),
        );
        let mut child = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(row)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        child.spacing_mut().item_spacing.x = 8.0;
        let named = |response: &egui::Response, name: &str, value: &str| {
            response.widget_info(|| {
                let mut info = egui::WidgetInfo::labeled(egui::WidgetType::ComboBox, true, name);
                info.current_text_value = Some(value.to_owned());
                info
            });
        };
        let tab = self.tab;
        let filter = &self.history.filter;
        let every_source = gettext(locale, "All sources");
        let source = filter.source.map_or_else(
            || every_source.to_string(),
            |source| source_menu_word(source, locale),
        );
        let menu = menu::Dropdown::new("history-source", &source, 150.0)
            .height(28.0)
            .show(&mut child, look, palette, |ui| {
                let choices = std::iter::once((None, every_source.to_string())).chain(
                    Source::ALL
                        .into_iter()
                        .map(|source| (Some(source), source_menu_word(source, locale))),
                );
                for (value, word) in choices {
                    let item =
                        menu::Item::choice(&word, filter.source == value).show(ui, look, palette);
                    if item.clicked() {
                        asks.push(ask(Action::FilterHistory {
                            tab,
                            filter: HistoryFilterSet::Source(value),
                        }));
                        ui.close();
                    }
                }
            });
        named(&menu.response, &gettext(locale, "Source"), &source);
        let every_result = gettext(locale, "All results");
        let result = filter.outcome.map_or_else(
            || every_result.to_string(),
            |outcome| outcome_word(outcome, locale),
        );
        let menu = menu::Dropdown::new("history-result", &result, 150.0)
            .height(28.0)
            .show(&mut child, look, palette, |ui| {
                let choices = std::iter::once((None, every_result.to_string())).chain(
                    Outcome::ALL
                        .into_iter()
                        .map(|outcome| (Some(outcome), outcome_word(outcome, locale))),
                );
                for (value, word) in choices {
                    let item =
                        menu::Item::choice(&word, filter.outcome == value).show(ui, look, palette);
                    if item.clicked() {
                        asks.push(ask(Action::FilterHistory {
                            tab,
                            filter: HistoryFilterSet::Outcome(value),
                        }));
                        ui.close();
                    }
                }
            });
        named(&menu.response, &gettext(locale, "Result"), &result);
        let mut text = filter.search.clone();
        let placeholder = gettext(locale, "Search SQL…");
        let role = widgets::body(look);
        let field = child.add_sized(
            vec2(220.0, 28.0),
            widgets::field(&child, &mut text, look, role, 28.0, 8)
                .id(search_id(tab))
                .hint_text(placeholder.as_ref()),
        );
        ui.ctx().accesskit_node_builder(field.id, |node| {
            node.set_label(placeholder.trim_end_matches('…'));
        });
        if field.changed() {
            asks.push(ask(Action::FilterHistory {
                tab,
                filter: HistoryFilterSet::Search(text),
            }));
        }
        // ⌘F gives the search the keyboard: each ask once.
        let seen_id = Id::new(("history-search-seen", tab.0));
        let seen: Option<u64> = ui.data(|data| data.get_temp(seen_id));
        ui.data_mut(|data| data.insert_temp(seen_id, self.history.line_asked));
        if seen.is_some_and(|seen| seen != self.history.line_asked) {
            field.request_focus();
        }
        let filters = child.min_rect().right();
        let count = self.count(self.history.shown().count());
        let small = widgets::secondary(look);
        let laid = Text::one(look, small, &count, palette.secondary).layout(ui.ctx());
        let left = bar.right() - 16.0 - laid.width();
        if left >= filters + 12.0 {
            laid.paint_left(ui.painter(), left, bar.center().y);
        }
        widgets::announce(ui, bar, &count);
        bar.bottom()
    }

    /// The terminal look's line at the top: the filter line (open, or what
    /// it says, its unknown tokens in red), or the hint of its tokens; the
    /// count at the right. While Clear history asks, its question.
    fn terminal_line(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) -> f32 {
        let (look, palette, locale) = (self.look, self.palette, self.locale);
        let bar = Rect::from_min_size(rect.min, vec2(rect.width(), LINE));
        widgets::hline(ui, bar.x_range(), bar.bottom() - 0.5, palette.border);
        let body = widgets::body(look);
        let y = bar.center().y;
        let left = bar.left() + 14.0;
        if self.history.clearing {
            let question = self.clear_question();
            let text = Text::new(look)
                .add(body, &question, palette.warning)
                .space(body, "  ")
                .add(body, self.keys.yes.as_str(), palette.accent)
                .add(body, &format!(" {}", self.say("yes")), palette.secondary)
                .space(body, "  ")
                .add(body, self.keys.no.as_str(), palette.accent)
                .add(body, &format!(" {}", self.say("no")), palette.secondary);
            widgets::paint_text(ui, left, y, text);
            widgets::announce(ui, bar, &question);
            return bar.bottom();
        }
        // The count at the right: "184 kept · last 1000 · bookshop_development".
        let shown = self.history.shown().count();
        let count = match self.keep {
            0 => self.say(OFF),
            keep => format!(
                "{} · {} · {}",
                look.label(
                    &ngettext(
                        locale,
                        "{n} kept",
                        "{n} kept",
                        u32::try_from(shown).unwrap_or(u32::MAX)
                    )
                    .replace("{n}", &shown.to_string())
                ),
                look.label(&gettext(locale, "last {n}").replace("{n}", &keep.to_string())),
                self.database,
            ),
        };
        let count_width = widgets::paint_text_right(
            ui,
            bar.right() - 14.0,
            y,
            Text::one(look, body, &count, palette.dim),
        );
        widgets::announce(ui, bar, &count);
        let room = (bar.right() - 14.0 - count_width - 24.0 - left).max(0.0);
        // The key, then the field while open, else what it says.
        let slash = Text::one(look, body, self.keys.filter.as_str(), palette.accent);
        let used = widgets::paint_text(ui, left, y, slash);
        let at = left + used + 8.0;
        let read = crate::history::Line::read(&self.history.line);
        if self.history.line_open {
            let field_rect = Rect::from_min_max(
                pos2(at, bar.top() + 4.0),
                pos2((at + room * 0.6).max(at + 120.0), bar.bottom() - 4.0),
            );
            let mut text = self.history.line.clone();
            let mut child = ui.new_child(egui::UiBuilder::new().max_rect(field_rect));
            let field = child.add_sized(
                field_rect.size(),
                widgets::field(&child, &mut text, look, body, field_rect.height(), 6)
                    .id(search_id(self.tab)),
            );
            ui.ctx().accesskit_node_builder(field.id, |node| {
                node.set_label(look.label(&gettext(locale, "filter")).as_str());
            });
            let seen_id = Id::new(("history-line-seen", self.tab.0));
            let seen: Option<u64> = ui.data(|data| data.get_temp(seen_id));
            ui.data_mut(|data| data.insert_temp(seen_id, self.history.line_asked));
            if seen != Some(self.history.line_asked) {
                field.request_focus();
            }
            if field.changed() {
                asks.push(ask(Action::FilterHistory {
                    tab: self.tab,
                    filter: HistoryFilterSet::Line(text),
                }));
            }
            // Esc closes the line, and keeps what it says.
            // egui's field gives the keyboard up on Esc by itself.
            if line_closed(ui, look) {
                field.surrender_focus();
                asks.push(ask(Action::ShowHistoryLine {
                    tab: self.tab,
                    open: false,
                }));
            }
            let mut x = field_rect.right() + 10.0;
            for token in &read.unknown {
                x += widgets::paint_text(ui, x, y, Text::one(look, body, token, palette.danger));
                x += 8.0;
            }
            if !read.unknown.is_empty() {
                let said = format!("{} {}", self.say("ignored:"), read.unknown.join(" "));
                widgets::announce(
                    ui,
                    Rect::from_min_max(pos2(field_rect.right(), bar.top()), bar.max),
                    &said,
                );
            }
            let hint = Text::new(look)
                .add(body, self.keys.close.as_str(), palette.accent)
                .add(
                    body,
                    &format!(" {}", self.say("closes, and keeps it")),
                    palette.secondary,
                );
            let laid = hint.layout(ui.ctx());
            if x + 8.0 + laid.width() <= bar.right() - 14.0 - count_width - 24.0 {
                laid.paint_left(ui.painter(), x + 8.0, y);
            }
            return bar.bottom();
        }
        if self.history.line.trim().is_empty() {
            // "/ filter · source:agent source:save result:error  plain words search the sql".
            let hint = Text::new(look)
                .add(body, &self.say("filter"), palette.secondary)
                .add(body, " · ", palette.secondary)
                .add(body, "source:agent source:save result:error", palette.text)
                .space(body, "  ")
                .add(body, &self.say("plain words search the sql"), palette.dim);
            let laid = hint.layout(ui.ctx());
            if laid.width() <= room {
                laid.paint_left(ui.painter(), at, y);
            }
            return bar.bottom();
        }
        // What the line says, its unknown tokens in red.
        let mut text = Text::new(look);
        for word in self.history.line.split_whitespace() {
            let color = match read.unknown.iter().any(|token| token == word) {
                true => palette.danger,
                false => palette.text,
            };
            text = text.add(body, word, color).space(body, " ");
        }
        widgets::paint_text(ui, at, y, text);
        widgets::announce(
            ui,
            Rect::from_min_max(pos2(at, bar.top()), pos2(at + room, bar.bottom())),
            &format!("{} {}", self.say("filter"), self.history.line),
        );
        bar.bottom()
    }

    /// The entries that pass the filters, by day, under the list's head.
    fn list(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) {
        let (look, palette, locale) = (self.look, self.palette, self.locale);
        let small = widgets::secondary(look);
        let body = widgets::body(look);
        let row = row_height(look);
        let history = self.history;
        let shown: Vec<&HistoryRow> = history.shown().collect();
        let empty = match (&history.failed, history.read) {
            _ if self.keep == 0 => Some(self.say("Nothing is recorded while History is off.")),
            (Some(failed), _) => Some(format!(
                "{} {failed}",
                self.say("The history could not be read:")
            )),
            (None, false) => Some(self.say("Reading the history…")),
            (None, true) if !shown.is_empty() => None,
            (None, true) if history.filter.is_set() => {
                Some(self.say("No statements in the history match these filters."))
            }
            (None, true) => Some(self.say("No statements run on this connection yet.")),
        };
        // Time, source, statement, rows, ms, result: the statement takes
        // what is left.
        let room = (rect.width() - 32.0).max(0.0);
        let fixed = |width: f32, share: f32| width.min(room * share);
        let mut widths = [
            fixed(76.0, 0.14),
            fixed(if look.terminal { 180.0 } else { 170.0 }, 0.24),
            0.0,
            fixed(54.0, 0.09),
            fixed(54.0, 0.09),
            fixed(92.0, 0.15),
        ];
        widths[2] = (room - widths.iter().sum::<f32>()).max(0.0);
        let names =
            ["Time", "Source", "Statement", "Rows", "ms", "Result"].map(|name| self.say(name));
        let head = Rect::from_min_size(rect.min, vec2(rect.width(), HEAD));
        if !look.terminal {
            ui.painter().rect_filled(head, 0.0, palette.panel);
        }
        widgets::hline(ui, head.x_range(), head.bottom() - 0.5, palette.border);
        let mut x = head.left() + 16.0;
        for (index, (name, width)) in names.iter().zip(widths).enumerate() {
            let text = Text::one(look, small, name, palette.secondary);
            if index == 3 || index == 4 {
                widgets::paint_text_right(ui, x + width - 8.0, head.center().y, text);
            } else {
                widgets::paint_text(ui, x, head.center().y, text);
            }
            x += width;
        }
        widgets::announce(ui, head, &names.join(" · "));
        let area_rect = Rect::from_min_max(pos2(rect.left(), head.bottom()), rect.max);
        // The list, as one stop of the keyboard: a table of the entries.
        let stop = ui.interact(
            area_rect,
            Id::new(("history-list", self.tab.0)),
            egui::Sense::focusable_noninteractive(),
        );
        let label = self.say("Statements");
        stop.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &label));
        ui.ctx().accesskit_node_builder(stop.id, |node| {
            node.set_role(egui::accesskit::Role::Table);
        });
        crate::ui::focus::pane(ui, &stop);
        crate::ui::focus::hint(
            ui,
            &stop,
            area_rect,
            crate::ui::focus::Ring::Inset { radius: 0 },
        );
        // New entries are said politely while the list has the keyboard,
        // and not otherwise.
        if stop.has_focus() {
            let count = self.count(shown.len());
            widgets::announce_polite(
                ui,
                Rect::from_min_size(area_rect.min, vec2(1.0, 1.0)),
                &count,
            );
        }
        if let Some(said) = empty {
            let line = Rect::from_min_size(
                pos2(area_rect.left(), area_rect.top() + 8.0),
                vec2(area_rect.width(), row),
            );
            widgets::paint_text(
                ui,
                line.left() + 16.0,
                line.center().y,
                Text::one(look, small, &said, palette.dim),
            );
            widgets::announce(ui, line, &said);
            return;
        }
        // The lines: a day's heading before its first entry.
        let now = std::time::SystemTime::now();
        let mut lines: Vec<Line<'_>> = Vec::with_capacity(shown.len() + 8);
        let mut day = None;
        for entry in &shown {
            let on = crate::clock::day_of(crate::clock::of_millis(entry.entry.at_ms), now);
            if day.as_ref() != Some(&on) {
                lines.push(Line::Day(day_word(&on, locale, look)));
                day = Some(on);
            }
            lines.push(Line::Entry(entry));
        }
        let heights: Vec<f32> = lines
            .iter()
            .map(|line| match line {
                Line::Day(_) => DAY,
                Line::Entry(_) => row,
            })
            .collect();
        let mut area = ui.new_child(egui::UiBuilder::new().max_rect(area_rect));
        area.spacing_mut().item_spacing.y = 0.0;
        let mono = TextRole::pick(look, TextRole::GridCell, TextRole::OBody);
        // A selection that moved since the last frame is brought into
        // sight; a new entry at the top moves neither it nor the keyboard.
        let seen_id = Id::new(("history-selected-seen", self.tab.0));
        let seen: Option<u64> = area.data(|data| data.get_temp(seen_id)).flatten();
        egui::ScrollArea::vertical()
            .id_salt(("history-list", self.tab.0))
            .auto_shrink([false, false])
            .show(&mut area, |ui| {
                for (line, height) in lines.iter().zip(&heights) {
                    let (place, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), *height),
                        egui::Sense::hover(),
                    );
                    if !ui.is_rect_visible(place) {
                        if let Line::Entry(entry) = line
                            && history.selected == Some(entry.key)
                            && seen != history.selected
                        {
                            ui.scroll_to_rect(place, None);
                        }
                        continue;
                    }
                    match line {
                        Line::Day(word) => {
                            let text = Text::one(look, small, word, palette.secondary);
                            widgets::paint_text(
                                ui,
                                place.left() + 16.0,
                                place.center().y + 3.0,
                                text,
                            );
                            let id = ui.id().with(("history-day", word.as_str()));
                            let response = ui.interact(place, id, egui::Sense::hover());
                            response.widget_info(|| {
                                egui::WidgetInfo::labeled(egui::WidgetType::Label, true, word)
                            });
                            ui.ctx().accesskit_node_builder(response.id, |node| {
                                node.set_role(egui::accesskit::Role::RowGroup);
                            });
                        }
                        Line::Entry(entry) => {
                            let clicked = ui
                                .interact(
                                    place,
                                    ui.id().with(("history-entry", entry.key)),
                                    egui::Sense::click(),
                                )
                                .clicked();
                            if clicked {
                                asks.push(ask(Action::SelectHistoryEntry {
                                    tab: self.tab,
                                    key: entry.key,
                                }));
                            }
                            if history.selected == Some(entry.key) {
                                ui.painter().rect_filled(place, 0.0, palette.selection);
                                let edge =
                                    Rect::from_min_size(place.min, vec2(3.0, place.height()));
                                ui.painter().rect_filled(edge, 0.0, palette.accent);
                                if seen != history.selected {
                                    ui.scroll_to_rect(place, None);
                                }
                            }
                            self.entry_row(ui, place, &entry.entry, &widths, (body, small, mono));
                            if !look.terminal {
                                widgets::hline(
                                    ui,
                                    place.x_range(),
                                    place.bottom() - 0.5,
                                    palette.border,
                                );
                            }
                        }
                    }
                }
            });
        area.data_mut(|data| data.insert_temp(seen_id, history.selected));
    }

    /// One entry's row, in its columns.
    fn entry_row(
        &self,
        ui: &egui::Ui,
        place: Rect,
        entry: &Entry,
        widths: &[f32; 6],
        (body, small, mono): (TextRole, TextRole, TextRole),
    ) {
        let (look, palette, locale) = (self.look, self.palette, self.locale);
        let y = place.center().y;
        let mut x = place.left() + 16.0;
        let fit = |text: &str, width: f32, role: TextRole| {
            crate::ui::grid::ellipsize(text, (width - 12.0).max(0.0), false, |text| {
                role.width(ui.ctx(), look.faces, text)
            })
            .into_owned()
        };
        let time = crate::clock::time(crate::clock::of_millis(entry.at_ms));
        widgets::paint_text(ui, x, y, Text::one(look, mono, &time, palette.secondary));
        x += widths[0];
        // The source: its icon and word, and what it was of.
        let agent = entry.source == Source::Agent;
        let (word, detail) = source_words(entry, self.detail_of(entry), locale, look);
        let ink = if agent { palette.accent } else { palette.text };
        let source = joined(&word, &detail);
        if look.terminal {
            let said = fit(&source, widths[1], body);
            widgets::paint_text(ui, x, y, Text::one(look, body, &said, ink));
        } else {
            source_icon(entry.source)
                .image(
                    if agent {
                        palette.accent
                    } else {
                        palette.secondary
                    },
                    13.0,
                )
                .paint_at(
                    ui,
                    Rect::from_center_size(pos2(x + 6.0, y), vec2(13.0, 13.0)),
                );
            let text = Text::new(look).add(body, &word, ink).space(body, " ").add(
                small,
                &detail,
                palette.secondary,
            );
            let laid = text.layout(ui.ctx());
            if laid.width() <= widths[1] - 30.0 {
                laid.paint_left(ui.painter(), x + 19.0, y);
            } else {
                widgets::paint_text(
                    ui,
                    x + 19.0,
                    y,
                    Text::one(look, body, &fit(&word, widths[1] - 19.0, body), ink),
                );
            }
        }
        x += widths[1];
        let gone = self.history.call_gone(entry);
        let statement = statement_line(entry, gone, locale, look);
        let role = if entry.source == Source::Agent {
            body
        } else {
            mono
        };
        let color = if agent {
            palette.secondary
        } else {
            palette.text
        };
        widgets::paint_text(
            ui,
            x,
            y,
            Text::one(look, role, &fit(&statement, widths[2], role), color),
        );
        x += widths[2];
        let rows = entry
            .rows
            .map(crate::ui::format::group_digits)
            .unwrap_or_default();
        let took = entry
            .took_ms
            .map(crate::ui::format::group_digits)
            .unwrap_or_default();
        for (number, width) in [(&rows, widths[3]), (&took, widths[4])] {
            widgets::paint_text_right(
                ui,
                x + width - 8.0,
                y,
                Text::one(look, small, number, palette.secondary),
            );
            x += width;
        }
        let (result, tone) = result_words(entry, locale, look, palette);
        result_pill(ui, (x + 8.0, y), &result, tone, look, palette);
        // What a screen reader and the tests read of the row.
        let said = [time, source, statement, rows, took, result].join(" · ");
        let id = ui.id().with(("history-row", entry.at_ms, said.as_str()));
        let response = ui.interact(place, id, egui::Sense::hover());
        response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Label, true, &said));
        ui.ctx().accesskit_node_builder(response.id, |node| {
            node.set_role(egui::accesskit::Role::Row);
        });
    }

    /// The selected entry's pane: the region named Statement.
    fn pane(&self, ui: &mut egui::Ui, rect: Rect, asks: &mut Vec<Ask>) {
        let (look, palette) = (self.look, self.palette);
        ui.painter().rect_filled(rect, 0.0, palette.panel);
        widgets::vline(ui, rect.left() + 0.5, rect.y_range(), palette.border);
        let region = ui.interact(
            rect,
            Id::new(("history-pane", self.tab.0)),
            egui::Sense::hover(),
        );
        let named = self.say("Statement");
        region.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Other, true, &named));
        ui.ctx().accesskit_node_builder(region.id, |node| {
            node.set_role(egui::accesskit::Role::Region);
        });
        let inner = rect.shrink2(vec2(16.0, 16.0));
        let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
        egui::ScrollArea::vertical()
            .id_salt(("history-pane", self.tab.0))
            .auto_shrink([false, false])
            .show(&mut child, |ui| {
                ui.spacing_mut().item_spacing.y = 12.0;
                if self.keep == 0 {
                    self.off(ui, asks);
                    return;
                }
                match self.history.chosen() {
                    Some(chosen) => self.detail(ui, chosen, asks),
                    None => {
                        let said = self.say("Select a statement to see it whole.");
                        Text::one(look, widgets::secondary(look), &said, palette.dim)
                            .wrap(ui.available_width())
                            .layout(ui.ctx())
                            .label(ui);
                    }
                }
            });
    }

    /// History is off: what the pane says, and where it is turned on.
    fn off(&self, ui: &mut egui::Ui, asks: &mut Vec<Ask>) {
        let (look, palette) = (self.look, self.palette);
        let width = ui.available_width();
        ui.add_space(24.0);
        if !look.terminal {
            let (place, _) = ui.allocate_exact_size(vec2(14.0, 14.0), egui::Sense::hover());
            Icon::Clock
                .image(palette.secondary, 14.0)
                .paint_at(ui, place);
        }
        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
        Text::one(look, strong, &self.say(OFF), palette.text)
            .layout(ui.ctx())
            .label(ui);
        let note = self.say(
            "Tabletist isn't recording statements on this computer. Turn it on to see every statement run on this connection.",
        );
        Text::one(look, widgets::secondary(look), &note, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        let label = self.say("Open Settings › Editor");
        if states::button(&label, look)
            .show(ui, states::button_height(look), look, palette)
            .clicked()
        {
            asks.push(ask(Action::ShowSettingsPane(
                crate::model::SettingsPane::Editor,
            )));
        }
    }

    /// What the pane says of `chosen`, by its source.
    fn detail(&self, ui: &mut egui::Ui, chosen: &HistoryRow, asks: &mut Vec<Ask>) {
        let (look, palette, locale) = (self.look, self.palette, self.locale);
        let entry = &chosen.entry;
        let width = ui.available_width();
        let body = widgets::body(look);
        let small = widgets::secondary(look);
        let strong = TextRole::pick(look, TextRole::UiBodyStrong, TextRole::OGroup);
        let (word, detail) = source_words(entry, self.detail_of(entry), locale, look);
        let agent = entry.source == Source::Agent;
        let detail = match detail.is_empty() {
            true => String::new(),
            false => format!(" · {detail}"),
        };
        // "SQL editor · Query 1", with production's pill.
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            if agent && !look.terminal {
                let (place, _) = ui.allocate_exact_size(vec2(13.0, 13.0), egui::Sense::hover());
                Icon::SquareTerminal
                    .image(palette.accent, 13.0)
                    .paint_at(ui, place);
            }
            let ink = if agent { palette.accent } else { palette.text };
            Text::new(look)
                .add(strong, &word, ink)
                .add(body, &detail, palette.secondary)
                .layout(ui.ctx())
                .label(ui);
            if self.production {
                let pill = self.say("production");
                let laid = Text::one(look, small, &pill, palette.on_accent).layout(ui.ctx());
                let (place, _) =
                    ui.allocate_exact_size(vec2(laid.width() + 14.0, 18.0), egui::Sense::hover());
                ui.painter().rect_filled(place, 9.0, palette.danger);
                laid.paint_left(ui.painter(), place.left() + 7.0, place.center().y);
                widgets::announce(ui, place, &pill);
            }
        });
        // When, and how it ran.
        let moment = moment_words(entry.at_ms, locale, look);
        let how = self.how(entry);
        let sub = match how.is_empty() {
            true => moment,
            false => format!("{moment} · {how}"),
        };
        Text::one(look, body, &sub, palette.secondary)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
        if agent {
            self.agent_detail(ui, chosen, asks);
            return;
        }
        if let Some(sql) = &entry.sql {
            let frame = egui::Frame::new()
                .fill(palette.window)
                .stroke(egui::Stroke::new(1.0, palette.border))
                .corner_radius(egui::CornerRadius::same(look.radius))
                .inner_margin(egui::Margin::symmetric(12, 10));
            frame.show(ui, |ui| {
                ui.set_width(width - 24.0);
                let laid = crate::ui::sql_text::highlighted(
                    ui.ctx(),
                    look,
                    palette,
                    self.dialect,
                    sql,
                    width - 24.0,
                );
                laid.label(ui);
            });
        }
        for (label, value) in self.facts(entry) {
            ui.horizontal_top(|ui| {
                let name = Text::one(look, small, &label, palette.secondary).layout(ui.ctx());
                ui.allocate_ui(vec2(110.0, name.galley.size().y), |ui| {
                    ui.set_min_width(110.0);
                    name.label(ui);
                });
                Text::one(look, body, &value, palette.text)
                    .wrap((width - 110.0).max(40.0))
                    .layout(ui.ctx())
                    .label(ui);
            });
        }
        let key = chosen.key;
        let tab = self.tab;
        if look.terminal {
            // The keys, as the board has them.
            let mut hints: Vec<(&crate::keymap::Written, String)> = Vec::new();
            if entry.runs_again() {
                hints.push((&self.keys.run, self.say("run again")));
            }
            if entry.source == Source::Filter {
                hints.push((&self.keys.reapply, self.say("reapply filter")));
            }
            hints.push((&self.keys.open, self.say("open in sql")));
            hints.push((&self.keys.copy, self.say("copy sql")));
            let mut text = Text::new(look);
            for (index, (written, word)) in hints.iter().enumerate() {
                if index > 0 {
                    text = text.space(body, "  ");
                }
                text = text.add(body, written.as_str(), palette.accent).add(
                    body,
                    &format!(" {word}"),
                    palette.secondary,
                );
            }
            text.wrap(width).layout(ui.ctx()).label(ui);
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = vec2(8.0, 8.0);
                let height = 30.0;
                if entry.runs_again() && entry.source == Source::Sql {
                    let label = gettext(locale, "Run again");
                    let button = states::button(&label, look)
                        .primary()
                        .shortcut(&self.keys.run);
                    if button.show(ui, height, look, palette).clicked() {
                        asks.push(ask(Action::RunHistoryAgain { tab, key }));
                    }
                }
                if entry.source == Source::Filter {
                    let label = gettext(locale, "Reapply filter");
                    if states::button(&label, look)
                        .primary()
                        .show(ui, height, look, palette)
                        .clicked()
                    {
                        asks.push(ask(Action::ReapplyHistoryFilter { tab, key }));
                    }
                }
                let label = gettext(locale, "Open in SQL editor");
                if states::button(&label, look)
                    .show(ui, height, look, palette)
                    .clicked()
                {
                    asks.push(ask(Action::OpenHistoryEntry { tab, key }));
                }
                if entry.source != Source::Filter {
                    let label = gettext(locale, "Copy SQL");
                    let button = widgets::ButtonSpec::new(&label).link();
                    if button.show(ui, height, look, palette).clicked()
                        && let Some(sql) = &entry.sql
                    {
                        asks.push(Ask::Copy(sql.clone()));
                    }
                }
            });
        }
        let note = self.note(entry);
        Text::one(look, small, &note, palette.dim)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
    }

    /// An agent's entry: that what it sent is the Agents Log's, and the
    /// link to its call there, or that the log has dropped it.
    fn agent_detail(&self, ui: &mut egui::Ui, chosen: &HistoryRow, asks: &mut Vec<Ask>) {
        let (look, palette) = (self.look, self.palette);
        let width = ui.available_width();
        let body = widgets::body(look);
        let small = widgets::secondary(look);
        let kept = self.say(
            "What an agent sent is kept only in the Agents Log. History shows when it ran and the result, and links to the call.",
        );
        let frame = egui::Frame::new()
            .fill(palette.window)
            .stroke(egui::Stroke::new(1.0, palette.border))
            .corner_radius(egui::CornerRadius::same(look.radius))
            .inner_margin(egui::Margin::symmetric(12, 10));
        frame.show(ui, |ui| {
            ui.set_width(width - 24.0);
            Text::one(look, body, &kept, palette.secondary)
                .wrap(width - 24.0)
                .layout(ui.ctx())
                .label(ui);
        });
        if self.history.call_gone(&chosen.entry) {
            let gone = self.say(GONE);
            Text::one(look, body, &gone, palette.text)
                .wrap(width)
                .layout(ui.ctx())
                .label(ui);
            return;
        }
        let (tab, key) = (self.tab, chosen.key);
        if look.terminal {
            Text::new(look)
                .add(body, self.keys.call.as_str(), palette.accent)
                .add(
                    body,
                    &format!(" {}", self.say("agents log")),
                    palette.secondary,
                )
                .layout(ui.ctx())
                .label(ui);
        } else {
            let label = gettext(self.locale, "Show in Agents Log");
            let button = states::button(&label, look).shortcut(&self.keys.call);
            if button.show(ui, 30.0, look, palette).clicked() {
                asks.push(ask(Action::ShowHistoryCall { tab, key }));
            }
        }
        let note = self.say(
            "Run again and Open in SQL editor are on the call in the Agents Log. After the log drops the call, this entry says it is no longer there.",
        );
        Text::one(look, small, &note, palette.dim)
            .wrap(width)
            .layout(ui.ctx())
            .label(ui);
    }

    /// How the entry ran, after its time: "read-only transaction", "one
    /// transaction · saved", the filter and sort, an agent's tool, rows,
    /// time and result, or how it ended where that was not well.
    fn how(&self, entry: &Entry) -> String {
        let locale = self.locale;
        let (result, _) = result_words(entry, locale, self.look, self.palette);
        let said = match (entry.source, entry.outcome) {
            (_, Outcome::Cancelled) => match entry.took_ms {
                Some(took) => gettext(locale, "Cancelled after {time}")
                    .replace("{time}", &duration_words(took)),
                None => result,
            },
            (_, Outcome::Error) => result,
            (Source::Sql, _) => match entry.read_only {
                true => gettext(locale, "read-only transaction").into_owned(),
                false => gettext(locale, "read-write").into_owned(),
            },
            (Source::Save, _) => format!(
                "{} · {}",
                gettext(locale, "one transaction"),
                result.to_lowercase()
            ),
            (Source::Filter, _) => entry.query.as_ref().map(filter_words).unwrap_or_default(),
            (Source::Agent, _) => {
                let mut said = vec![entry.tool.clone().unwrap_or_default()];
                if let Some(rows) = entry.rows {
                    said.push(
                        ngettext(
                            locale,
                            "{n} row",
                            "{n} rows",
                            u32::try_from(rows).unwrap_or(u32::MAX),
                        )
                        .replace("{n}", &crate::ui::format::group_digits(rows)),
                    );
                }
                if let Some(took) = entry.took_ms {
                    said.push(format!("{} ms", crate::ui::format::group_digits(took)));
                }
                said.push(result);
                said.join(" · ")
            }
        };
        self.look.label(&said)
    }

    /// The entry's facts, line by line, as the pane says them.
    fn facts(&self, entry: &Entry) -> Vec<(String, String)> {
        let locale = self.locale;
        let say = |text: &'static str| self.say(text);
        let mut facts = Vec::new();
        let rows = |rows: Option<u64>| match rows {
            Some(rows) => crate::ui::format::group_digits(rows),
            None => say("none"),
        };
        match entry.source {
            Source::Save => facts.push((say("Rows changed"), rows(entry.rows))),
            Source::Sql | Source::Filter | Source::Agent => {
                facts.push((say("Rows"), rows(entry.rows)))
            }
        }
        if let Some(took) = entry.took_ms {
            facts.push((
                say("Took"),
                format!("{} ms", crate::ui::format::group_digits(took)),
            ));
        }
        if entry.source == Source::Sql || entry.outcome != Outcome::Ok {
            let (result, _) = result_words(entry, locale, self.look, self.palette);
            let result = match (entry.outcome, entry.message.as_deref()) {
                // Why it was stopped, or that what it wrote was undone.
                (Outcome::Cancelled | Outcome::Ok, Some(why)) => format!("{result} · {why}"),
                _ => result,
            };
            facts.push((say("Result"), result));
        }
        if let (Outcome::Error, Some(message)) = (entry.outcome, &entry.message) {
            facts.push((say("Message"), message.clone()));
        }
        if entry.source == Source::Sql {
            facts.push((say("Database"), entry.database.clone()));
        }
        facts
    }

    /// The pane's small print, by what the entry offers.
    fn note(&self, entry: &Entry) -> String {
        let words = match (entry.source, entry.runs_again(), entry.outcome) {
            (Source::Filter, _, _) => {
                "Reapply filter opens the table with this filter and sort, the way you set them."
            }
            (_, true, _) if self.production => {
                "On production, Run again opens a read-only tab, whatever the statement. It runs on the connection it came from."
            }
            (_, true, Outcome::Error | Outcome::Cancelled) => {
                "Errors and cancelled reads can be run again too. The timeout from Settings › Editor applies."
            }
            (_, true, _) if self.look.terminal => {
                return self
                    .say("{key} opens a new sql tab and runs it on the current data. results are never stored.")
                    .replace("{key}", self.keys.run.as_str());
            }
            (_, true, _) => {
                "Run again opens a new SQL tab and runs this on the current data. Results are never stored, only the statement."
            }
            (_, false, _) if self.look.terminal => {
                return self
                    .say("writes: {key} only, into a read-only tab.")
                    .replace("{key}", self.keys.open.as_str());
            }
            (_, false, _) => {
                "Writes are never run again from History. Open in SQL editor puts the statements in a new read-only tab; switch the tab to read-write to run them."
            }
        };
        self.say(words)
    }
}

/// A line of the list.
enum Line<'a> {
    Day(String),
    Entry(&'a HistoryRow),
}

/// The search field's id, which ⌘F and `/` give the keyboard.
pub fn search_id(tab: ConnTabId) -> Id {
    Id::new(("history-search", tab.0))
}

/// Whether the keymap's close of the filter line was pressed this frame.
fn line_closed(ui: &egui::Ui, look: &Look) -> bool {
    let keymap = crate::ui::keys::published(ui.ctx());
    let layout = crate::keymap::Layout::of(look);
    ui.input_mut(|input| {
        crate::ui::keys::asked(
            input,
            &keymap,
            layout,
            crate::keymap::Command::CloseHistoryLine,
            crate::keymap::Scope::History,
            true,
        )
        .count
            > 0
    })
}

fn source_icon(source: Source) -> Icon {
    match source {
        Source::Sql => Icon::Code,
        Source::Save => Icon::Table,
        Source::Filter => Icon::Funnel,
        Source::Agent => Icon::SquareTerminal,
    }
}

/// An entry's source, in a word, and what it was of: "SQL editor" and
/// "Query 1"; an agent's `detail` is its name as the Agents Log says it,
/// where it says one.
fn source_words(entry: &Entry, detail: &str, locale: Locale, look: &Look) -> (String, String) {
    let word = match (entry.source, look.terminal) {
        (Source::Sql, false) => gettext(locale, "SQL editor"),
        (Source::Sql, true) => gettext(locale, "sql"),
        (Source::Save, false) => gettext(locale, "Grid save"),
        (Source::Save, true) => gettext(locale, "save"),
        (Source::Filter, _) => gettext(locale, "Filter"),
        (Source::Agent, _) => gettext(locale, "Agent"),
    };
    let detail = match look.terminal {
        true => detail.to_lowercase(),
        false => detail.to_owned(),
    };
    (look.label(&word), detail)
}

/// "SQL editor · Query 1", or the word alone where it was of nothing
/// named.
fn joined(word: &str, detail: &str) -> String {
    match detail.is_empty() {
        true => word.to_owned(),
        false => format!("{word} · {detail}"),
    }
}

/// A source as the Source menu offers it.
fn source_menu_word(source: Source, locale: Locale) -> String {
    match source {
        Source::Sql => gettext(locale, "SQL editor"),
        Source::Save => gettext(locale, "Grid saves"),
        Source::Filter => gettext(locale, "Filters"),
        Source::Agent => gettext(locale, "Agents"),
    }
    .into_owned()
}

/// A result, in a word: "OK", "Saved", "Error".
fn outcome_word(outcome: Outcome, locale: Locale) -> String {
    match outcome {
        Outcome::Ok => gettext(locale, "OK"),
        Outcome::Saved => gettext(locale, "Saved"),
        Outcome::Error => gettext(locale, "Error"),
        Outcome::Cancelled => gettext(locale, "Cancelled"),
        Outcome::Approved => gettext(locale, "Approved"),
        Outcome::Denied => gettext(locale, "Denied"),
        Outcome::Refused => gettext(locale, "Refused"),
    }
    .into_owned()
}

/// An entry's result as its pill says it, an error with its code, and the
/// pill's colour.
fn result_words(
    entry: &Entry,
    locale: Locale,
    look: &Look,
    palette: &Palette,
) -> (String, egui::Color32) {
    let word = outcome_word(entry.outcome, locale);
    let word = match (entry.outcome, &entry.code) {
        (Outcome::Error, Some(code)) => format!("{word} {code}"),
        _ => word,
    };
    let color = match entry.outcome {
        Outcome::Ok | Outcome::Saved => states::Tone::Success.color(palette),
        Outcome::Error | Outcome::Denied | Outcome::Refused => states::Tone::Danger.color(palette),
        Outcome::Cancelled => states::Tone::Warning.color(palette),
        Outcome::Approved => palette.accent,
    };
    (look.label(&word), color)
}

/// A result's pill at `(x, y)`: its word on a wash of its colour, or the
/// word alone in the terminal look.
fn result_pill(
    ui: &egui::Ui,
    (x, y): (f32, f32),
    word: &str,
    color: egui::Color32,
    look: &Look,
    palette: &Palette,
) {
    let small = widgets::secondary(look);
    let laid = Text::one(look, small, word, color).layout(ui.ctx());
    if !look.terminal {
        let wash = Rect::from_min_size(pos2(x - 7.0, y - 9.0), vec2(laid.width() + 14.0, 18.0));
        let fill = color.gamma_multiply(if palette.dark { 0.22 } else { 0.14 });
        ui.painter().rect_filled(wash, 9.0, fill);
    }
    laid.paint_left(ui.painter(), x, y);
}

/// The statement as its row says it, on one line: a save's count and its
/// first statement, an agent's tool and where the call is.
fn statement_line(entry: &Entry, gone: bool, locale: Locale, look: &Look) -> String {
    let one = |text: &str| text.split_whitespace().collect::<Vec<_>>().join(" ");
    match entry.source {
        Source::Agent => {
            let tool = entry.tool.clone().unwrap_or_default();
            let place = match (gone, look.terminal) {
                (true, _) => gettext(locale, GONE),
                (false, true) => gettext(locale, "in the Agents Log"),
                (false, false) => gettext(locale, "in the Agents Log →"),
            };
            look.label(&format!("{tool} · {place}"))
        }
        Source::Save => {
            let sql = entry.sql.as_deref().unwrap_or_default();
            let statements: Vec<&str> = sql
                .split(";\n")
                .map(str::trim)
                .filter(|part| {
                    !part.is_empty() && *part != "BEGIN" && *part != "COMMIT;" && *part != "COMMIT"
                })
                .collect();
            let first = statements
                .first()
                .map(|first| one(first))
                .unwrap_or_default();
            match statements.len() {
                0 | 1 => first,
                n => format!(
                    "{} · {first}",
                    look.label(&gettext(locale, "{n} statements").replace("{n}", &n.to_string()))
                ),
            }
        }
        Source::Sql | Source::Filter => one(entry.sql.as_deref().unwrap_or_default()),
    }
}

/// A day's heading: "Today", "Yesterday", "Oct 13".
fn day_word(day: &crate::clock::Day, locale: Locale, look: &Look) -> String {
    let word = match day {
        crate::clock::Day::Today => gettext(locale, "Today").into_owned(),
        crate::clock::Day::Yesterday => gettext(locale, "Yesterday").into_owned(),
        crate::clock::Day::On(date) => date.clone(),
    };
    match look.terminal {
        true => word.to_lowercase(),
        false => word.to_uppercase(),
    }
}

/// "Today 10:42:18".
fn moment_words(at_ms: i64, locale: Locale, look: &Look) -> String {
    let at = crate::clock::of_millis(at_ms);
    let day = match crate::clock::day_of(at, std::time::SystemTime::now()) {
        crate::clock::Day::Today => gettext(locale, "Today").into_owned(),
        crate::clock::Day::Yesterday => gettext(locale, "Yesterday").into_owned(),
        crate::clock::Day::On(date) => date,
    };
    look.label(&format!("{day} {}", crate::clock::time(at)))
}

/// "30 s", "1 min 5 s", "450 ms".
fn duration_words(took_ms: u64) -> String {
    match took_ms {
        ms if ms < 1_000 => format!("{ms} ms"),
        ms if ms < 60_000 => format!("{} s", ms / 1_000),
        ms => match (ms / 1_000) % 60 {
            0 => format!("{} min", ms / 60_000),
            s => format!("{} min {s} s", ms / 60_000),
        },
    }
}

/// A filter's conditions and sort, as the pane says them:
/// "kind = 'print' · sort created_at ↑".
fn filter_words(kept: &crate::session::KeptTab) -> String {
    let bar = kept.bar();
    let mut said: Vec<String> = bar
        .rows
        .iter()
        .map(|row| match row.op.takes_value() {
            true => format!("{} {} '{}'", row.column, row.op.label(), row.value),
            false => format!("{} {}", row.column, row.op.label()),
        })
        .collect();
    if bar.raw && !bar.raw_text.trim().is_empty() {
        said.push(
            bar.raw_text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    for sort in kept.sorts() {
        let arrow = match sort.dir {
            tabletist_db::SortDir::Asc => "↑",
            tabletist_db::SortDir::Desc => "↓",
        };
        said.push(format!("sort {} {arrow}", sort.column));
    }
    said.join(" · ")
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_duration_is_said_in_its_unit() {
        assert_eq!(super::duration_words(450), "450 ms");
        assert_eq!(super::duration_words(30_000), "30 s");
        assert_eq!(super::duration_words(65_000), "1 min 5 s");
        assert_eq!(super::duration_words(120_000), "2 min");
    }
}
