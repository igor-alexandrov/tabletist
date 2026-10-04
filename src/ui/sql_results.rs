//! A SQL editor's results. Under a header with the Results and Messages
//! tabs: the rows of the last statement that returned some, in the grid a
//! table uses, or a line for every statement of the run. With neither to
//! show, what there is to say: how to run, that a run is on its way, that
//! it was cancelled, or why it failed.

use std::time::Duration;

use egui::{CornerRadius, Id, Rect, Sense, Ui, WidgetInfo, WidgetType, pos2, vec2};
use tabletist_db::{Error, StatementOutcome, ValueKind};

use crate::app::App;
use crate::backend::{CancelReason, RequestId};
use crate::i18n::{Locale, gettext};
use crate::model::{Action, ConnTabId, ResultPane, SqlRun, SqlTab, TabId};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Laid, Text, TextRole};
use crate::ui::data_view;
use crate::ui::format;
use crate::ui::grid::{self, Column};
use crate::ui::states;
use crate::ui::value_tags::Tags;
use crate::ui::widgets::{self, ButtonSpec};

/// The header's height: 40 and a rule, or the terminal's 34 and a rule
/// (the rule over it is the splitter's).
fn header_height(look: &Look) -> f32 {
    if look.terminal { 35.0 } else { 41.0 }
}

/// Left and right padding of the header, the messages and the notes.
fn side(look: &Look) -> f32 {
    if look.terminal { 16.0 } else { 12.0 }
}

/// What the panes draw with: the look, its colours and the user's language.
#[derive(Clone, Copy)]
struct Env<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
}

/// Our own words as a look writes them: lower case in the terminal's. What
/// a database said is never passed through here: its names keep their case.
#[derive(Clone, Copy)]
struct Words {
    locale: Locale,
    lower: bool,
}

impl Words {
    fn say(self, text: &'static str) -> String {
        let text = gettext(self.locale, text);
        if self.lower {
            text.to_lowercase()
        } else {
            text.into_owned()
        }
    }
}

/// A text as screen readers are told it and as the look paints it. The
/// two differ only in the terminal look.
struct Said {
    name: String,
    painted: String,
}

impl Env<'_> {
    /// `text` in the name's words and in the look's.
    fn said(&self, text: impl Fn(Words) -> String) -> Said {
        let name = text(Words {
            locale: self.locale,
            lower: false,
        });
        let painted = if self.look.terminal {
            text(Words {
                locale: self.locale,
                lower: true,
            })
        } else {
            name.clone()
        };
        Said { name, painted }
    }
}

/// What the panes have to show.
enum State<'a> {
    /// Nothing ran yet.
    Idle,
    /// A run is on its way and none before it left a result.
    Waiting,
    /// The last run failed as a whole: nothing ran.
    Failed(&'a Error),
    /// The last run. It stays up while the next one is on its way.
    Ran(&'a SqlRun),
}

fn state(sql: &SqlTab) -> State<'_> {
    // A run that failed as a whole ran nothing, and left no last run.
    if let Some(error) = &sql.run.error {
        return State::Failed(error);
    }
    match sql.last_run() {
        Some(run) => State::Ran(run),
        None if sql.is_running() => State::Waiting,
        None => State::Idle,
    }
}

/// The id of the grid that shows the result of `run`: one per run and per
/// fit, so the widths that fit one result's columns are not another's.
fn grid_id(tab: ConnTabId, id: TabId, run: Option<RequestId>, fit: data_view::Fit) -> Id {
    Id::new(("sql-grid", tab.0, id.0, run.map(|run| run.0), fit))
}

/// Where egui's memory keeps what the results of the editor `id` were
/// last drawn with.
fn results_id(tab: ConnTabId, id: TabId) -> Id {
    Id::new(("sql-results", tab.0, id.0))
}

/// The grid an editor's result was last drawn with. Each run has its own,
/// so the one before it is forgotten when the next is drawn.
#[derive(Clone, Copy, PartialEq)]
struct LastGrid(Id);

/// The scroll area an editor's messages were last drawn in, one per run
/// as well: a new run's messages start at their top.
#[derive(Clone, Copy, PartialEq)]
struct LastMessages(Id);

/// Notes that the results kept under `key` now draw the grid `grid`, and
/// drops what egui kept for the grid before it.
fn keep_grid(ctx: &egui::Context, key: Id, grid: Id) {
    let before: Option<LastGrid> = ctx.data(|data| data.get_temp(key));
    if before == Some(LastGrid(grid)) {
        return;
    }
    if let Some(LastGrid(old)) = before {
        grid::forget(ctx, old);
    }
    ctx.data_mut(|data| data.insert_temp(key, LastGrid(grid)));
}

/// [`keep_grid`] for the scroll area of the messages.
fn keep_messages(ctx: &egui::Context, key: Id, area: Id) {
    ctx.data_mut(|data| {
        let before: Option<LastMessages> = data.get_temp(key);
        if before == Some(LastMessages(area)) {
            return;
        }
        if let Some(LastMessages(old)) = before {
            data.remove::<egui::scroll_area::State>(old);
        }
        data.insert_temp(key, LastMessages(area));
    });
}

/// Drops what egui's memory keeps for the results of a closed editor: its
/// grid's widths and where it and the messages were scrolled to.
pub fn forget(ctx: &egui::Context, tab: ConnTabId, id: TabId) {
    let key = results_id(tab, id);
    let grid: Option<LastGrid> = ctx.data(|data| data.get_temp(key));
    if let Some(LastGrid(grid)) = grid {
        grid::forget(ctx, grid);
    }
    ctx.data_mut(|data| {
        if let Some(LastMessages(area)) = data.get_temp(key) {
            data.remove::<egui::scroll_area::State>(area);
        }
        data.remove::<LastGrid>(key);
        data.remove::<LastMessages>(key);
    });
}

pub fn show(app: &mut App, ui: &mut Ui, tab: ConnTabId, id: TabId) {
    let mut actions = Vec::new();
    draw(app, ui, tab, id, &mut actions);
    app.actions.extend(actions);
}

/// The editor a pane is drawn for.
#[derive(Clone, Copy)]
struct Place<'a> {
    tab: ConnTabId,
    sql: &'a SqlTab,
    /// How the cells are drawn, as the workspace's tables draw them.
    fit: data_view::Fit,
    /// Whose error codes the results read.
    driver: tabletist_db::Driver,
    /// Whether the arrow keys move in the result's grid.
    keys: bool,
}

fn draw(app: &App, ui: &mut Ui, tab: ConnTabId, id: TabId, actions: &mut Vec<Action>) {
    let (look, palette) = (app.look, app.palette);
    let env = Env {
        look: &look,
        palette: &palette,
        locale: app.locale,
    };
    let Some(workspace) = app.workspace(tab) else {
        return;
    };
    let Some(sql) = workspace.sql_tab(id) else {
        return;
    };
    let place = Place {
        tab,
        sql,
        fit: data_view::Fit::of(workspace, &app.settings),
        driver: workspace.driver,
        keys: workspace.pane == crate::model::Pane::Grid,
    };
    let state = state(sql);
    let pane = ui.max_rect();
    let top = Rect::from_min_size(pane.min, vec2(pane.width(), header_height(&look)));
    header(ui, top, &head(sql, &state, &env), &place, &env, actions);
    let rest = Rect::from_min_max(top.left_bottom(), pane.max);
    let mut body = ui.new_child(egui::UiBuilder::new().id_salt("body").max_rect(rest));
    body.set_clip_rect(rest.intersect(ui.clip_rect()));
    match (state, sql.pane) {
        (State::Idle, _) => {
            let (keys, _) = super::sql_editor::run_keys(&look);
            let hint = env.said(|words| format!("{} {keys}", words.say("Run a statement with")));
            note(&body, rest, &hint, palette.secondary, &env);
        }
        (State::Waiting, _) => {
            egui::Spinner::new().size(20.0).color(palette.dim).paint_at(
                &body,
                Rect::from_center_size(rest.center(), vec2(20.0, 20.0)),
            );
        }
        (State::Failed(error), ResultPane::Results) => {
            if format::refuses_writes(error, place.driver) {
                blocked(&mut body, rest, error, &env);
            } else {
                note(&body, rest, &whole(error), palette.danger, &env);
            }
        }
        (State::Failed(error), ResultPane::Messages) => {
            let lines: Vec<Line<'_>> = std::iter::once(Line::Whole(error))
                .chain(more(error))
                .collect();
            messages(&mut body, &lines, None, &place, &env);
        }
        (State::Ran(run), ResultPane::Results) => results(&mut body, run, &place, &env, actions),
        (State::Ran(run), ResultPane::Messages) => {
            messages(&mut body, &lines(run), Some(run), &place, &env);
        }
    }
}

/// What the header says beside its tabs.
struct Head {
    /// The rows of the result that shows, beside "Results".
    count: Option<usize>,
    /// "Statement at line 2 · 14 ms" for it. A run in flight is said in
    /// its place.
    info: Option<Said>,
    /// How long the run in flight has been going, from when it was queued.
    running: Option<Duration>,
    /// The statements of the last run that failed (1 for a run that failed
    /// as a whole), beside "Messages".
    errors: usize,
}

fn head(sql: &SqlTab, state: &State<'_>, env: &Env<'_>) -> Head {
    let result = match state {
        State::Ran(run) => sql
            .shown()
            .and_then(|(index, result)| Some((run.statements.get(index)?, result))),
        _ => None,
    };
    let running = sql.running_for();
    Head {
        count: result.and_then(|(_, result)| match &result.outcome {
            StatementOutcome::Rows { rows, .. } => Some(rows.len()),
            _ => None,
        }),
        info: result.map(|(statement, result)| {
            env.said(|words| {
                format!(
                    "{} {} · {}",
                    words.say("Statement at line"),
                    statement.first_line,
                    format::elapsed(result.elapsed)
                )
            })
        }),
        running,
        errors: match state {
            State::Failed(_) => 1,
            State::Ran(run) => run
                .outcome
                .results
                .iter()
                .filter(|result| matches!(result.outcome, StatementOutcome::Error { .. }))
                .count(),
            State::Idle | State::Waiting => 0,
        },
    }
}

/// One of the header's tabs, laid out.
struct PaneTab {
    pane: ResultPane,
    /// Its name, in every look.
    name: String,
    /// The name as the look writes it, and the count after it.
    text: String,
    count: Option<String>,
    /// What the count is painted in: the errors' is not the rows'.
    count_color: egui::Color32,
    width: f32,
}

/// The tabs' roles: the selected one's and the others'. Both are measured
/// as the selected one, so picking a tab moves nothing.
fn tab_roles(look: &Look) -> (TextRole, TextRole) {
    if look.terminal {
        (TextRole::OBody, TextRole::OBody)
    } else {
        (TextRole::UiBodyStrong, TextRole::UiBody)
    }
}

/// The header: the Results and Messages tabs, then what the result that
/// shows is, or that a run is on its way with a button to cancel it.
fn header(
    ui: &mut Ui,
    rect: Rect,
    head: &Head,
    place: &Place<'_>,
    env: &Env<'_>,
    actions: &mut Vec<Action>,
) {
    let Env { look, palette, .. } = *env;
    let (fill, rule) = if look.terminal {
        (palette.panel, palette.outline)
    } else {
        (palette.window, palette.surface_hover)
    };
    ui.painter().rect_filled(rect, CornerRadius::ZERO, fill);
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, rule);
    let center = rect.top() + (rect.height() - 1.0) / 2.0;
    let (left, right) = (rect.left() + side(look), rect.right() - side(look));
    // What follows the tabs starts 16 after them, and reads a step
    // quieter than they do (in the terminal, by its colour alone).
    let start = pane_tabs(
        ui,
        (left, center),
        (head.count, head.errors),
        place,
        env,
        actions,
    ) + 16.0;
    let role = TextRole::pick(look, TextRole::Secondary, TextRole::OBody);
    let spot = Spot {
        start,
        right,
        center,
        role,
    };
    if let Some(running) = head.running {
        if run_state(ui, &spot, running, env) {
            actions.push(Action::CancelQuery(place.tab));
        }
    } else if let Some(info) = &head.info {
        let cut = grid::ellipsize(&info.painted, (right - start).max(0.0), false, |text| {
            role.width(ui.ctx(), look.faces, text)
        });
        // Cut to nothing but its dots, it says nothing.
        if cut.chars().count() > 1 {
            let text = Text::one(look, role, &cut, palette.dim);
            data_view::paint_named(ui, start, center, text, &info.name);
        }
    }
}

/// The Results and Messages tabs from `left`, centred on `center`: a click
/// on the one not shown asks for its pane. Returns where they end.
fn pane_tabs(
    ui: &mut Ui,
    (left, center): (f32, f32),
    (count, errors): (Option<usize>, usize),
    place: &Place<'_>,
    env: &Env<'_>,
    actions: &mut Vec<Action>,
) -> f32 {
    let Env {
        look,
        palette,
        locale,
    } = *env;
    let (strong, plain) = tab_roles(look);
    let width = |role: TextRole, text: &str| role.width(ui.ctx(), look.faces, text);
    let space = width(plain, " ");
    let tabs = [
        (ResultPane::Results, "Results", count, palette.dim),
        (
            ResultPane::Messages,
            "Messages",
            (errors > 0).then_some(errors),
            palette.danger,
        ),
    ]
    .map(|(pane, name, count, count_color)| {
        let name = gettext(locale, name).into_owned();
        let text = look.label(&name);
        let count = count.map(|count| format::group_digits(count as u64));
        let counted = count
            .as_deref()
            .map_or(0.0, |count| space + width(plain, count));
        PaneTab {
            pane,
            width: width(strong, &text) + counted,
            name,
            text,
            count,
            count_color,
        }
    });
    // macOS: 28 pt tabs, 12 at their sides, 2 apart. The terminal: the
    // words, 14 apart. Either way clear of the header's top, where the
    // terminal's splitter takes the pointer.
    let (pad, gap, height) = if look.terminal {
        let line = strong.row_height(ui.ctx(), look.faces);
        (0.0, 14.0, line + 8.0)
    } else {
        (12.0, 2.0, 28.0)
    };
    let mut x = left;
    let mut end = left;
    for tab in &tabs {
        let cell = Rect::from_min_size(
            pos2(x, center - height / 2.0),
            vec2(tab.width + 2.0 * pad, height),
        );
        end = cell.right();
        x = end + gap;
        // The terminal's words take the pointer a little past their ends.
        let hit = cell.expand2(vec2(if look.terminal { 4.0 } else { 0.0 }, 0.0));
        let response = ui.interact(hit, ui.id().with(("pane", &tab.name)), Sense::click());
        let selected = place.sql.pane == tab.pane;
        // Named alike with and without rows: the count is its value.
        response.widget_info(|| {
            let mut info = WidgetInfo::selected(WidgetType::Button, true, selected, &tab.name);
            info.current_text_value = tab.count.clone();
            info
        });
        let role = if selected { strong } else { plain };
        let color = match (selected, look.terminal) {
            (true, true) => palette.accent,
            (true, false) => palette.text,
            (false, true) if response.hovered() => palette.text,
            (false, true) => palette.dim,
            (false, false) => palette.secondary,
        };
        if !look.terminal && (selected || response.hovered()) {
            let fill = if selected {
                palette.surface
            } else {
                palette.panel
            };
            ui.painter()
                .rect_filled(cell, CornerRadius::same(look.radius), fill);
        }
        let mut text = Text::new(look).add(role, &tab.text, color);
        if let Some(count) = &tab.count {
            text = text.space(plain, " ").add(plain, count, tab.count_color);
        }
        widgets::paint_text(ui, cell.left() + pad, center, text);
        if selected && look.terminal {
            // 2 under the word's line, 2 thick.
            let y = cell.bottom() - 2.0;
            ui.painter().rect_filled(
                Rect::from_min_max(pos2(cell.left(), y), pos2(cell.right(), y + 2.0)),
                CornerRadius::ZERO,
                palette.accent,
            );
        }
        let radius = if look.terminal { 0 } else { look.radius };
        crate::ui::focus::hint(ui, &response, hit, crate::ui::focus::Ring::Outer { radius });
        if response.clicked() && !selected {
            actions.push(Action::SetResultPane {
                tab: place.tab,
                sql_tab: place.sql.id,
                pane: tab.pane,
            });
        }
    }
    end
}

/// Where the header's text after its tabs goes: from `start` to `right`,
/// centred on `center`, in `role`.
#[derive(Clone, Copy)]
struct Spot {
    start: f32,
    right: f32,
    center: f32,
    role: TextRole,
}

/// The run in flight: a spinner and how long it has been going, and at the
/// right the button that cancels it. As the room runs out the button's
/// keys give way (the help says them too), then the time. Returns whether
/// the button was clicked.
fn run_state(ui: &mut Ui, spot: &Spot, running: Duration, env: &Env<'_>) -> bool {
    const SPINNER: f32 = 12.0;
    let Env {
        look,
        palette,
        locale,
    } = *env;
    let Spot {
        start,
        right,
        center,
        role,
    } = *spot;
    let time =
        env.said(|words| format!("{} · {:.1} s", words.say("Running"), running.as_secs_f64()));
    let label = gettext(locale, "Cancel query");
    let cancel = look.label(&gettext(locale, "Cancel"));
    let keys = format!("{}.", look.label(look.command_key()));
    let button = |with_keys: bool| {
        let button = ButtonSpec::new(&cancel).label(&label);
        let button = if look.terminal {
            button
                .hint()
                .shortcut_role(TextRole::OBody)
                .padding(11.0)
                .gap(8.0)
        } else {
            button.role(TextRole::Secondary).padding(10.0)
        };
        if with_keys {
            button.shortcut(&keys)
        } else {
            button
        }
    };
    let widths = [false, true].map(|keys| button(keys).width(ui, look));
    let time_width = SPINNER + 6.0 + role.width(ui.ctx(), look.faces, &time.painted);
    let fit = [(true, true), (false, true), (false, false)]
        .into_iter()
        .find(|(keys, time)| {
            let time = if *time { time_width + 16.0 } else { 0.0 };
            time + widths[usize::from(*keys)] <= right - start
        });
    let (with_keys, with_time) = fit.unwrap_or((false, false));
    if with_time {
        // The spinner asks for the frames that keep the time going.
        egui::Spinner::new()
            .size(SPINNER)
            .color(palette.dim)
            .paint_at(
                ui,
                Rect::from_center_size(pos2(start + SPINNER / 2.0, center), vec2(SPINNER, SPINNER)),
            );
        let text = Text::one(look, role, &time.painted, palette.dim);
        data_view::paint_named(ui, start + SPINNER + 6.0, center, text, &time.name);
    }
    // At the right end, and never over the tabs. Shorter than the header
    // by more than the 3 pt of it the terminal's splitter takes.
    let width = widths[usize::from(with_keys)];
    let height = if look.terminal { 24.0 } else { 28.0 };
    let at = Rect::from_min_size(
        pos2((right - width).max(start), center - height / 2.0),
        vec2(width, height),
    );
    button(with_keys).show_at(ui, at, look, palette).clicked()
}

/// A message laid out: its place (a line, or what a detail is), then its
/// text beside it. Two galleys, so that nothing in what a database said (a
/// right-to-left override, say) can move the place or change how it reads.
struct LaidMessage {
    place: Option<Laid>,
    text: Laid,
    /// Where the text starts, from the message's left.
    text_left: f32,
}

impl LaidMessage {
    /// Lays `place` and `text` out in `role` within `room` points. Text
    /// that `wraps` (what a database said) takes the rows it needs beside
    /// the place; text of ours is one row, cut with "…" where it is too
    /// long.
    fn new(
        ctx: &egui::Context,
        (place, place_color): (&str, egui::Color32),
        (text, color): (&str, egui::Color32),
        (role, room, wraps): (TextRole, f32, bool),
        look: &Look,
    ) -> Self {
        let place =
            (!place.is_empty()).then(|| Text::one(look, role, place, place_color).layout(ctx));
        let space = role.width(ctx, look.faces, " ");
        let text_left = place.as_ref().map_or(0.0, |place| place.width() + space);
        let room = (room - text_left).max(0.0);
        let text = if wraps {
            Text::one(look, role, text, color).wrap(room).layout(ctx)
        } else {
            let cut = grid::ellipsize(text, room, false, |text| role.width(ctx, look.faces, text));
            Text::one(look, role, &cut, color).layout(ctx)
        };
        Self {
            place,
            text,
            text_left,
        }
    }

    fn size(&self) -> egui::Vec2 {
        let place = self.place.as_ref().map_or(0.0, Laid::height);
        vec2(
            self.text_left + self.text.width(),
            self.text.height().max(place),
        )
    }

    /// Paints with the top-left corner at `at`.
    fn paint(&self, painter: &egui::Painter, at: egui::Pos2) {
        if let Some(place) = &self.place {
            place.paint(painter, at);
        }
        self.text.paint(painter, at + vec2(self.text_left, 0.0));
    }
}

/// `said` in the middle of `rect`, wrapped to fit it.
fn note(ui: &Ui, rect: Rect, said: &Said, color: egui::Color32, env: &Env<'_>) {
    note_at(ui, rect, ("", &said.painted), &said.name, color, env);
}

/// A note that leads with a `place`: what statement `text` is said of.
fn note_at(
    ui: &Ui,
    rect: Rect,
    (place, text): (&str, &str),
    name: &str,
    color: egui::Color32,
    env: &Env<'_>,
) {
    let room = (rect.width() - 2.0 * 24.0).max(0.0);
    let laid = LaidMessage::new(
        ui.ctx(),
        (place, color),
        (text, color),
        (widgets::body(env.look), room, true),
        env.look,
    );
    // Kept under the header when there is too little room to centre it.
    let at = rect.center() - laid.size() / 2.0;
    let at = pos2(at.x, at.y.max(rect.top() + 8.0));
    laid.paint(ui.painter(), at);
    widgets::announce(ui, Rect::from_min_size(at, laid.size()), name);
}

/// An error in its own words, cut as [`format::capped`] cuts.
fn error_text(error: &Error) -> String {
    match error {
        // The message itself, without a copy of all of it first.
        Error::Query { message, .. } => format::capped(message).into_owned(),
        other => format::capped(&other.to_string()).into_owned(),
    }
}

/// A whole run's error, in its own words: they name the line when there
/// is one and say what Tabletist refused or why the session closed.
fn whole(error: &Error) -> Said {
    let text = error_text(error);
    Said {
        painted: text.clone(),
        name: text,
    }
}

/// "Cancelled", or "Cancelled after 30 s (timeout)" when the timeout
/// stopped the run. A statement cancelled for no reason of ours (the
/// server's own `statement_timeout`) is cancelled all the same.
fn cancel_text(cancel: Option<CancelReason>, words: Words) -> String {
    match cancel {
        Some(CancelReason::Timeout(after)) => format!(
            "{} {} s ({})",
            words.say("Cancelled after"),
            after.as_secs(),
            words.say("timeout")
        ),
        Some(CancelReason::User) | None => words.say("Cancelled"),
    }
}

/// "5 rows", "1 row".
fn rows_text(count: u64, words: Words) -> String {
    let noun = if count == 1 { "row" } else { "rows" };
    format!("{} {}", format::group_digits(count), words.say(noun))
}

/// The statement that reads as cancelled though it has no result: the
/// first one, when the run was stopped before it began. A run stopped
/// later has a `Cancelled` result of its own (every driver gives the
/// statement it was stopped in or before one), and whatever has no result
/// after that did not run.
fn stopped_at(run: &SqlRun) -> Option<usize> {
    let outcome = &run.outcome;
    let cancelled = outcome
        .results
        .iter()
        .any(|result| result.outcome == StatementOutcome::Cancelled);
    let started = outcome.results.len();
    (outcome.stopped && !cancelled && started < run.statements.len()).then_some(started)
}

/// How a line of the messages reads.
#[derive(Clone, Copy, PartialEq)]
enum Tone {
    Plain,
    /// A statement that did not run.
    Muted,
    Cancelled,
    Failed,
}

impl Tone {
    fn color(self, palette: &Palette) -> egui::Color32 {
        match self {
            Self::Plain => palette.text,
            Self::Muted => palette.dim,
            Self::Cancelled => palette.warning,
            Self::Failed => palette.danger,
        }
    }
}

/// One message: its place (a line, or what a detail is), what it says
/// and how that reads.
struct Message {
    place: String,
    text: String,
    tone: Tone,
}

impl Message {
    fn new(place: String, text: String, tone: Tone) -> Self {
        Self { place, text, tone }
    }

    /// The place and the text as one line.
    fn line(&self) -> String {
        if self.place.is_empty() {
            self.text.clone()
        } else {
            format!("{} {}", self.place, self.text)
        }
    }
}

/// What statement `index` of `run` did, after its line (the error's own
/// line and column when the database gave a position).
fn statement_message(run: &SqlRun, index: usize, words: Words) -> Message {
    let Some(statement) = run.statements.get(index) else {
        return Message::new(String::new(), String::new(), Tone::Plain);
    };
    let at = format!("{} {}:", words.say("Line"), statement.first_line);
    let Some(result) = run.outcome.results.get(index) else {
        return if stopped_at(run) == Some(index) {
            Message::new(at, cancel_text(run.cancel, words), Tone::Cancelled)
        } else {
            Message::new(at, words.say("Not run"), Tone::Muted)
        };
    };
    let time = format::elapsed(result.elapsed);
    match &result.outcome {
        StatementOutcome::Rows {
            rows, truncated, ..
        } => {
            let cut = if *truncated {
                format!(" ({})", words.say("limit reached"))
            } else {
                String::new()
            };
            let rows = rows_text(rows.len() as u64, words);
            Message::new(at, format!("{rows}{cut} · {time}"), Tone::Plain)
        }
        StatementOutcome::Done {
            affected: Some(count),
        } => {
            let noun = if *count == 1 {
                "row affected"
            } else {
                "rows affected"
            };
            let count = format::group_digits(*count);
            let text = format!("{count} {} · {time}", words.say(noun));
            Message::new(at, text, Tone::Plain)
        }
        StatementOutcome::Done { affected: None } => {
            let text = format!("{} · {time}", words.say("Statement ran"));
            Message::new(at, text, Tone::Plain)
        }
        StatementOutcome::Error { error, position } => {
            let at = match position {
                Some(position) => {
                    let (line, column) = statement.line_col(*position);
                    let (line_word, col) = (words.say("Line"), words.say("col"));
                    format!("{line_word} {line}, {col} {column}:")
                }
                None => at,
            };
            Message::new(at, error_text(error), Tone::Failed)
        }
        StatementOutcome::Cancelled => {
            Message::new(at, cancel_text(run.cancel, words), Tone::Cancelled)
        }
    }
}

/// One line of the messages.
#[derive(Clone, Copy)]
enum Line<'a> {
    /// What the statement at this index of the run did.
    Statement(usize),
    /// More of the error above it: its code, detail or hint.
    More(&'static str, &'a str),
    /// A run that failed as a whole, in the error's own words.
    Whole(&'a Error),
}

impl Line<'_> {
    /// Whether the line holds what a database said, which can be long:
    /// such a line wraps. The others are ours, and short.
    fn wraps(&self, run: Option<&SqlRun>) -> bool {
        match self {
            Self::More(..) | Self::Whole(_) => true,
            Self::Statement(index) => run
                .and_then(|run| run.outcome.results.get(*index))
                .is_some_and(|result| matches!(result.outcome, StatementOutcome::Error { .. })),
        }
    }
}

/// The code, detail and hint a database gave with `error`.
fn more(error: &Error) -> impl Iterator<Item = Line<'_>> {
    let parts = match error {
        Error::Query {
            code, detail, hint, ..
        } => [("Code", code), ("Detail", detail), ("Hint", hint)]
            .map(|(label, text)| text.as_deref().map(|text| Line::More(label, text))),
        _ => [None, None, None],
    };
    parts.into_iter().flatten()
}

/// A line for every statement of `run`, in the script's order.
fn lines(run: &SqlRun) -> Vec<Line<'_>> {
    let mut lines = Vec::with_capacity(run.statements.len());
    for index in 0..run.statements.len() {
        lines.push(Line::Statement(index));
        if let Some(result) = run.outcome.results.get(index)
            && let StatementOutcome::Error { error, .. } = &result.outcome
        {
            lines.extend(more(error));
        }
    }
    lines
}

/// What `line` says.
fn message(line: Line<'_>, run: Option<&SqlRun>, words: Words) -> Message {
    match line {
        Line::Statement(index) => match run {
            Some(run) => statement_message(run, index, words),
            None => Message::new(String::new(), String::new(), Tone::Plain),
        },
        Line::More(label, text) => Message::new(
            format!("{}:", words.say(label)),
            format::capped(text).into_owned(),
            Tone::Muted,
        ),
        Line::Whole(error) => Message::new(String::new(), error_text(error), Tone::Failed),
    }
}

/// The messages: one line per statement of `run`, or why the run failed
/// as a whole. Only the lines in view are written and laid out.
fn messages(
    ui: &mut Ui,
    lines: &[Line<'_>],
    run: Option<&SqlRun>,
    place: &Place<'_>,
    env: &Env<'_>,
) {
    let Env {
        look,
        palette,
        locale,
    } = *env;
    let role = widgets::code(look);
    // 3 above and below each line; 8 above the first.
    let (above, row) = (3.0, role.row_height(ui.ctx(), look.faces) + 6.0);
    let pad = side(look);
    let room = (ui.available_width() - 2.0 * pad).max(0.0);
    let painted = Words {
        locale,
        lower: look.terminal,
    };
    let ctx = ui.ctx().clone();
    // More of an error sits 16 in from the line it is more of.
    let indent = |line: Line<'_>| {
        if matches!(line, Line::More(..)) {
            16.0
        } else {
            0.0
        }
    };
    let lay = |line: Line<'_>| -> LaidMessage {
        let message = message(line, run, painted);
        LaidMessage::new(
            &ctx,
            (&message.place, palette.dim),
            (&message.text, message.tone.color(palette)),
            (role, (room - indent(line)).max(0.0), line.wraps(run)),
            look,
        )
    };
    // A line of ours is one row, cut where it is too long for the pane.
    // What a database said is as tall as it wraps to: laid out here for
    // its height, and few (a run stops at its first error).
    let heights: Vec<f32> = lines
        .iter()
        .map(|line| {
            if line.wraps(run) {
                lay(*line).size().y + 6.0
            } else {
                row
            }
        })
        .collect();
    let area = egui::ScrollArea::vertical()
        .id_salt(("sql-messages", place.sql.run.loaded.map(|run| run.0)))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add_space(8.0);
            widgets::virtual_rows_varying(ui, &heights, |ui, index| {
                let line = lines[index];
                let size = vec2(ui.available_width(), heights[index]);
                let (rect, response) = ui.allocate_exact_size(size, Sense::hover());
                // Named in every look's case and in full, whatever the
                // look paints and however the pane cuts it.
                let named = Words {
                    locale,
                    lower: false,
                };
                let name = message(line, run, named).line();
                response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &name));
                if ui.is_rect_visible(rect) {
                    let at = pos2(rect.left() + pad + indent(line), rect.top() + above);
                    lay(line).paint(ui.painter(), at);
                }
            });
            ui.add_space(8.0);
        });
    keep_messages(ui.ctx(), results_id(place.tab, place.sql.id), area.id);
}

/// A write the read-only session refused: said as what it is, a limit of
/// this version and not a mistake in the statement. The exact error
/// stays under the card. A pane too short for it all scrolls.
fn blocked(ui: &mut Ui, rect: Rect, error: &Error, env: &Env<'_>) {
    let Env { look, palette, .. } = *env;
    let inner = rect.shrink2(vec2(16.0, 14.0));
    let mut pane = ui.new_child(
        egui::UiBuilder::new()
            .id_salt("blocked")
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );
    // As short as the pane is: a scroll area keeps 64 pt by itself, which
    // would run out under the pane.
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .min_scrolled_height(0.0)
        .show(&mut pane, |column| {
            column.spacing_mut().item_spacing = vec2(8.0, 10.0);
            let title = env.said(|words| words.say("This version only reads data"));
            let text = env.said(|words| {
                words.say(
                    "Every query runs in a read-only transaction, so this statement was \
                     refused. Nothing changed.",
                )
            });
            let card = states::Card {
                tone: states::Tone::Warning,
                icon: Icon::Lock,
                title: &title.painted,
                text: &text.painted,
            };
            states::card(column, &card, look, palette);
            // The database's own words, after its code when it gave one.
            let raw = match error {
                Error::Query {
                    code: Some(code), ..
                } => format!("{code} · {}", error_text(error)),
                other => error_text(other),
            };
            Text::one(look, widgets::code(look), &raw, palette.secondary)
                .wrap(column.available_width())
                .layout(column.ctx())
                .label(column);
            let later = env.said(|words| words.say("Editing arrives in a later version."));
            Text::one(
                look,
                widgets::secondary(look),
                &later.painted,
                palette.secondary,
            )
            .layout(column.ctx())
            .label(column);
        });
}

/// The Results pane of a run that ran: the rows of its last statement
/// that returned some, in the grid a table uses. With none, whether the
/// run was stopped, where it failed, or that it ran.
fn results(ui: &mut Ui, run: &SqlRun, place: &Place<'_>, env: &Env<'_>, actions: &mut Vec<Action>) {
    let Env {
        look,
        palette,
        locale,
    } = *env;
    let rect = ui.max_rect();
    let Place {
        tab,
        sql,
        fit,
        keys,
        ..
    } = *place;
    let Some((columns, rows, truncated)) = sql.shown_rows() else {
        // A run someone stopped never reads as one that ran.
        let failed = run
            .outcome
            .results
            .iter()
            .position(|result| matches!(result.outcome, StatementOutcome::Error { .. }));
        if run.outcome.was_cancelled() {
            let said = env.said(|words| cancel_text(run.cancel, words));
            note(ui, rect, &said, palette.warning, env);
        } else if let Some(index) = failed {
            if let StatementOutcome::Error { error, .. } = &run.outcome.results[index].outcome
                && format::refuses_writes(error, place.driver)
            {
                blocked(ui, rect, error, env);
                return;
            }
            // The statement's line, then what the database said of it.
            let named = Words {
                locale,
                lower: false,
            };
            let painted = Words {
                locale,
                lower: look.terminal,
            };
            let name = statement_message(run, index, named).line();
            let message = statement_message(run, index, painted);
            let parts = (message.place.as_str(), message.text.as_str());
            note_at(ui, rect, parts, &name, palette.danger, env);
        } else {
            let said = env.said(|words| words.say("Statement ran · no rows returned"));
            note(ui, rect, &said, palette.secondary, env);
        }
        return;
    };
    // Under the grid, when the limit cut the rows off: 28 and a rule.
    let mut area = rect;
    if truncated {
        let strip = Rect::from_min_max(pos2(rect.left(), rect.bottom() - 29.0), rect.max);
        area.max.y = strip.top();
        ui.painter()
            .rect_filled(strip, CornerRadius::ZERO, palette.panel);
        widgets::hline(ui, strip.x_range(), strip.top() + 0.5, palette.outline);
        // The rows there are, not the limit the editor has by now.
        let said = env.said(|words| {
            format!(
                "{} {} ({})",
                words.say("First"),
                rows_text(rows.len() as u64, words),
                words.say("limit reached")
            )
        });
        let text = Text::one(
            look,
            widgets::secondary(look),
            &said.painted,
            palette.secondary,
        );
        let center = strip.top() + 1.0 + (strip.height() - 1.0) / 2.0;
        data_view::paint_named(ui, strip.left() + side(look), center, text, &said.name);
    }
    let grid_columns: Vec<Column<'_>> = columns
        .iter()
        .map(|column| {
            // No structure: a result's column is no table's key.
            let (type_line, _) =
                data_view::type_line(&column.name, &column.type_name, column.kind, None, look);
            Column {
                name: &column.name,
                type_line,
                numeric: column.kind == ValueKind::Numeric,
                sort: None,
                key: false,
                flexible: column.kind == ValueKind::Json,
                // A result is in the order its statement gave it.
                sortable: false,
            }
        })
        .collect();
    let id = grid_id(tab, sql.id, sql.run.loaded, fit);
    keep_grid(ui.ctx(), results_id(tab, sql.id), id);
    let mut child = ui.new_child(egui::UiBuilder::new().id_salt("grid").max_rect(area));
    child.set_clip_rect(area.intersect(ui.clip_rect()));
    let ctx = ui.ctx().clone();
    // A result has no catalog to name a column's allowed values: only its
    // booleans draw as tags, as a table's do.
    let tags: Vec<Tags<'_>> = columns
        .iter()
        .map(|column| Tags::of(column, None).when(fit.value_tags))
        .collect();
    // The grid asks for the cells in view only.
    let output = grid::show(
        &mut child,
        id,
        &grid_columns,
        rows.len(),
        0,
        sql.selection,
        keys,
        palette,
        look,
        |row, col| {
            data_view::cell(
                &ctx,
                &rows[row][col],
                &columns[col],
                &tags[col],
                look,
                data_view::Shown {
                    full_precision: fit.full_precision,
                    // A result has no key to leave alone.
                    grouped: fit.grouped,
                },
            )
        },
    );
    if let Some(cell) = output.clicked {
        actions.push(Action::SelectCell {
            tab,
            id: sql.id,
            cell,
        });
    }
    // The Tab key came to the grid: the arrows are its own now.
    if output.focused {
        actions.push(Action::GridKeys(tab));
    }
    if rows.is_empty() {
        let under = Rect::from_min_max(
            pos2(area.left(), area.top() + grid::header_height(look)),
            area.max,
        );
        let said = env.said(|_| gettext(locale, "No rows").into_owned());
        note(ui, under, &said, palette.secondary, env);
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use egui::accesskit::Role;
    use egui::{Key, Modifiers};
    use tabletist_db::{Error, StatementOutcome};

    use super::*;
    use crate::backend::{CancelReason, Command};
    use crate::model::{Action, CellPos, ResultPane, SqlTab};
    use crate::testing::{
        Harness, bounds, error_outcome, labels, node, rows_outcome, script_outcome,
        stopped_before_it_began,
    };
    use crate::theme::Look;

    /// A SQL editor drawn in `look`, with `text` typed into it.
    fn editor(look: Look, text: &str) -> (Harness, ConnTabId) {
        let mut harness = Harness::new();
        harness.set_look(look);
        let tab = harness.connect_fake();
        harness.press(Key::T, Modifiers::COMMAND);
        harness.frame(vec![egui::Event::Paste(text.into())]);
        harness.settle();
        (harness, tab)
    }

    /// Runs the statement at the cursor, as the key does.
    fn run(harness: &mut Harness) {
        harness.press(Key::Enter, Modifiers::COMMAND);
    }

    fn run_all(harness: &mut Harness) {
        harness.press(Key::Enter, Modifiers::COMMAND | Modifiers::SHIFT);
    }

    fn sql(harness: &Harness, tab: ConnTabId) -> &SqlTab {
        let workspace = harness.app.workspace(tab).unwrap();
        workspace.active_sql_tab().unwrap()
    }

    fn show_pane(harness: &mut Harness, tab: ConnTabId, pane: ResultPane) {
        let sql_tab = sql(harness, tab).id;
        harness
            .app
            .apply(Action::SetResultPane { tab, sql_tab, pane });
    }

    /// The count beside "Results": the tab's value.
    fn count(harness: &mut Harness) -> Option<String> {
        let tree = harness.settle();
        let id = node(&tree, "Results", Role::Button).expect("the Results tab");
        let (_, tab) = tree.nodes.iter().find(|(node, _)| *node == id)?;
        tab.value().map(str::to_owned)
    }

    /// The hint an editor that has run nothing shows.
    fn hint(look: &Look) -> String {
        let (keys, _) = crate::ui::sql_editor::run_keys(look);
        format!("Run a statement with {keys}")
    }

    /// Whether the last frame painted `text` as one piece.
    fn painted(harness: &Harness, text: &str) -> bool {
        harness.painted.iter().any(|(piece, _)| piece == text)
    }

    /// A statement that ran and gave no result set.
    fn done(affected: Option<u64>) -> StatementOutcome {
        StatementOutcome::Done { affected }
    }

    #[test]
    fn before_a_run_the_results_say_how_to_run() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            let hint = hint(&look);
            assert!(harness.has(&hint), "{hint} in {}", look.name);
            assert_eq!(count(&mut harness), None, "{}", look.name);
            let tree = harness.settle();
            for name in ["Results", "Messages"] {
                assert!(node(&tree, name, Role::Button).is_some(), "{name}");
            }
            // The messages have nothing to list yet either.
            show_pane(&mut harness, tab, ResultPane::Messages);
            assert!(harness.has(&hint), "{}", look.name);
            // What later versions add is absent, not disabled.
            for name in ["Explain", "History", "Copy", "Export"] {
                assert!(!harness.has(name), "{name} in {}", look.name);
            }
            let reads = if look.terminal {
                hint.to_lowercase()
            } else {
                hint
            };
            assert!(painted(&harness, &reads), "{reads}: {:?}", harness.painted);
        }
    }

    #[test]
    fn a_script_with_nothing_to_run_leaves_the_hint() {
        let look = Look::standard();
        let (mut harness, tab) = editor(look, "-- only a note");
        run(&mut harness);
        run_all(&mut harness);
        assert!(!sql(&harness, tab).is_running());
        assert!(harness.has(&hint(&look)));
        assert!(!harness.has("Cancel query"));
    }

    #[test]
    fn a_result_fills_the_grid() {
        for look in Look::ALL {
            let (mut harness, _tab) = editor(look, "-- the users\nSELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
            let tree = harness.settle();
            for (name, role) in [
                // A result's columns are labels: there is no sorting it.
                ("email", Role::Label),
                ("Row 1", Role::Button),
                ("Row 5", Role::Button),
                ("Statement at line 2 · 14 ms", Role::Label),
            ] {
                assert!(
                    node(&tree, name, role).is_some(),
                    "{name} in {}: {:?}",
                    look.name,
                    labels(&tree)
                );
            }
            assert!(node(&tree, "Row 6", Role::Button).is_none());
            assert_eq!(count(&mut harness).as_deref(), Some("5"), "{}", look.name);
            assert!(!harness.has(&hint(&look)));
            // A column's name over its type, and the cells as a table's.
            for text in ["TEXT", "user5@example.com", "NULL"] {
                assert!(painted(&harness, text), "{text} in {}", look.name);
            }
            let info = if look.terminal {
                "statement at line 2 · 14 ms"
            } else {
                "Statement at line 2 · 14 ms"
            };
            assert!(painted(&harness, info), "{info}: {:?}", harness.painted);
        }
    }

    #[test]
    fn a_results_booleans_read_as_a_tables_do() {
        use tabletist_db::{ColumnMeta, Value, ValueKind};
        let (mut harness, _tab) = editor(Look::standard(), "SELECT 1");
        run(&mut harness);
        let flags = StatementOutcome::Rows {
            columns: vec![ColumnMeta {
                name: "in_print".into(),
                type_name: "bool".into(),
                kind: ValueKind::Bool,
            }],
            rows: vec![vec![Value::Bool(false)], vec![Value::Text("false".into())]],
            truncated: false,
        };
        harness.answer_sql(Ok(script_outcome(vec![flags])), None);
        harness.settle();
        // A false flag is the second tag; the same word as text is no flag.
        let palette = harness.app.palette;
        let colors: Vec<egui::Color32> = harness
            .painted
            .iter()
            .filter(|(piece, _)| piece == "false")
            .map(|(_, color)| *color)
            .collect();
        let (tag, _) = crate::ui::value_tags::slot_colors(1, &harness.app.look, &palette);
        assert_eq!(colors, [tag, palette.text]);
    }

    #[test]
    fn clicking_a_row_selects_its_cell() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT 1");
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
        harness.click("Row 3");
        assert_eq!(
            sql(&harness, tab).selection,
            Some(CellPos { row: 2, col: 0 })
        );
        assert_eq!(
            harness.app.workspace(tab).unwrap().pane,
            crate::model::Pane::Grid
        );
    }

    #[test]
    fn an_empty_result_shows_its_columns_and_says_it_has_no_rows() {
        for look in Look::ALL {
            let (mut harness, _tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(0)])), None);
            let tree = harness.settle();
            assert!(node(&tree, "email", Role::Label).is_some());
            assert!(harness.has("No rows"), "{}", look.name);
            assert_eq!(count(&mut harness).as_deref(), Some("0"));
            assert!(!harness.has("Statement ran · no rows returned"));
        }
    }

    #[test]
    fn the_tabs_switch_between_the_rows_and_the_messages() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
            assert_eq!(sql(&harness, tab).pane, ResultPane::Results);
            harness.click("Messages");
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            assert!(harness.has("Line 1: 5 rows · 14 ms"), "{}", look.name);
            assert!(!harness.has("Row 1"), "{}", look.name);
            // The count stays on its tab.
            assert_eq!(count(&mut harness).as_deref(), Some("5"));
            harness.click("Results");
            assert_eq!(sql(&harness, tab).pane, ResultPane::Results);
            assert!(harness.has("Row 1"), "{}", look.name);
            assert!(!harness.has("Line 1: 5 rows · 14 ms"));
        }
    }

    #[test]
    fn the_messages_list_every_statement() {
        for look in Look::ALL {
            let script = "SELECT 1;\nUPDATE notes SET seen = 1;\nSELECT Total;\nSELECT 2";
            let (mut harness, tab) = editor(look, script);
            run_all(&mut harness);
            harness.answer_sql(
                Ok(script_outcome(vec![
                    rows_outcome(1),
                    done(Some(3)),
                    error_outcome("no such column: Total", Some(8)),
                ])),
                None,
            );
            // A failed statement opens the messages by itself.
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            let tree = harness.settle();
            for line in [
                "Line 1: 1 row · 14 ms",
                "Line 2: 3 rows affected · 14 ms",
                "Line 3, col 8: no such column: Total",
                "Line 4: Not run",
            ] {
                assert!(
                    node(&tree, line, Role::Label).is_some(),
                    "{line} in {}: {:?}",
                    look.name,
                    labels(&tree)
                );
            }
            // The terminal writes our words in lower case, never the
            // database's.
            let reads = if look.terminal {
                ["line 3, col 8:", "line 4:", "not run"]
            } else {
                ["Line 3, col 8:", "Line 4:", "Not run"]
            };
            for text in reads {
                assert!(painted(&harness, text), "{text}: {:?}", harness.painted);
            }
            // What the database said is a piece of its own, in the
            // error's colour.
            assert_eq!(
                harness.painted_color("no such column: Total"),
                Some(harness.app.palette.danger),
                "{}",
                look.name
            );
            // The rows of the statement before the one that failed stay.
            harness.click("Results");
            assert!(harness.has("Row 1") && !harness.has("Row 2"));
            assert!(harness.has("Statement at line 1 · 14 ms"));
        }
    }

    #[test]
    fn an_error_says_its_code_detail_and_hint() {
        let (mut harness, _tab) = editor(Look::standard(), "SELECT kindd FROM notes");
        run(&mut harness);
        let failed = StatementOutcome::Error {
            error: Error::Query {
                code: Some("42703".into()),
                message: "column \"kindd\" does not exist".into(),
                detail: None,
                hint: Some("Perhaps you meant to reference the column \"kind\".".into()),
            },
            position: None,
        };
        harness.answer_sql(Ok(script_outcome(vec![failed])), None);
        for line in [
            "Line 1: column \"kindd\" does not exist",
            "Code: 42703",
            "Hint: Perhaps you meant to reference the column \"kind\".",
        ] {
            assert!(harness.has(line), "{line}");
        }
        // Results has no rows to show for it, and does not say it ran.
        harness.click("Results");
        assert!(harness.has("Line 1: column \"kindd\" does not exist"));
        assert!(!harness.has("Statement ran · no rows returned"));
        assert_eq!(count(&mut harness), None);
    }

    #[test]
    fn a_statement_without_a_result_set_says_it_ran() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "CREATE TEMP TABLE notes (seen int)");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![done(None)])), None);
            assert_eq!(sql(&harness, tab).pane, ResultPane::Results);
            assert!(
                harness.has("Statement ran · no rows returned"),
                "{}",
                look.name
            );
            assert_eq!(count(&mut harness), None);
            assert!(!harness.has("Statement at line 1 · 14 ms"));
            harness.click("Messages");
            assert!(
                harness.has("Line 1: Statement ran · 14 ms"),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_truncated_result_says_the_limit_was_reached() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            let page = crate::testing::page(1_000, false);
            let cut = StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: true,
            };
            harness.answer_sql(Ok(script_outcome(vec![cut])), None);
            let note = "First 1,000 rows (limit reached)";
            assert!(harness.has(note), "{}", look.name);
            // The limit the editor has now is the next run's, not this
            // result's.
            let sql_tab = sql(&harness, tab).id;
            harness.app.apply(Action::SetSqlLimit {
                tab,
                sql_tab,
                limit: 100,
            });
            assert!(harness.has(note), "{}", look.name);
            harness.click("Messages");
            assert!(harness.has("Line 1: 1,000 rows (limit reached) · 14 ms"));
            // A whole result has no note.
            harness.click("Results");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
            assert!(harness.has("Row 3") && !harness.has(note));
        }
    }

    #[test]
    fn a_timeout_reads_as_cancelled_after_its_limit() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            let timeout = Some(CancelReason::Timeout(Duration::from_secs(30)));
            harness.answer_sql(
                Ok(script_outcome(vec![StatementOutcome::Cancelled])),
                timeout,
            );
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            assert!(
                harness.has("Line 1: Cancelled after 30 s (timeout)"),
                "{}",
                look.name
            );
            harness.click("Results");
            assert!(
                harness.has("Cancelled after 30 s (timeout)"),
                "{}",
                look.name
            );
            assert!(!harness.has("Statement ran · no rows returned"));
            let reads = if look.terminal {
                "cancelled after 30 s (timeout)"
            } else {
                "Cancelled after 30 s (timeout)"
            };
            assert!(painted(&harness, reads), "{reads}: {:?}", harness.painted);
        }
    }

    #[test]
    fn a_cancel_reads_as_cancelled_and_earlier_rows_stay() {
        for cancel in [Some(CancelReason::User), None] {
            let (mut harness, tab) = editor(Look::standard(), "SELECT 1;\nSELECT 2;\nSELECT 3");
            run_all(&mut harness);
            harness.answer_sql(
                Ok(script_outcome(vec![
                    rows_outcome(2),
                    StatementOutcome::Cancelled,
                ])),
                cancel,
            );
            // Cancelled by hand with rows to show: the rows show.
            assert_eq!(sql(&harness, tab).pane, ResultPane::Results);
            assert!(harness.has("Row 2") && !harness.has("Cancelled"));
            harness.click("Messages");
            for line in [
                "Line 1: 2 rows · 14 ms",
                "Line 2: Cancelled",
                "Line 3: Not run",
            ] {
                assert!(harness.has(line), "{line} with {cancel:?}");
            }
        }
        // With no rows before it, Results says so too.
        let (mut harness, _tab) = editor(Look::standard(), "SELECT 1");
        run(&mut harness);
        harness.answer_sql(
            Ok(script_outcome(vec![StatementOutcome::Cancelled])),
            Some(CancelReason::User),
        );
        assert!(harness.has("Line 1: Cancelled"));
        harness.click("Results");
        assert!(harness.has("Cancelled"));
        assert!(!harness.has("Statement ran · no rows returned"));
    }

    /// Whether anything on screen says a statement ran.
    fn says_it_ran(harness: &mut Harness) -> bool {
        let tree = harness.settle();
        labels(&tree)
            .iter()
            .any(|label| label.contains("Statement ran"))
    }

    #[test]
    fn a_run_stopped_before_it_began_reads_as_cancelled() {
        let timeout = CancelReason::Timeout(Duration::from_secs(30));
        for look in Look::ALL {
            for (cancel, text) in [
                (Some(CancelReason::User), "Cancelled"),
                (Some(timeout), "Cancelled after 30 s (timeout)"),
                (None, "Cancelled"),
            ] {
                let (mut harness, tab) = editor(look, "SELECT 1;\nSELECT 2");
                run_all(&mut harness);
                harness.answer_sql(Ok(stopped_before_it_began()), cancel);
                assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
                for line in [format!("Line 1: {text}"), "Line 2: Not run".to_owned()] {
                    assert!(harness.has(&line), "{line} in {}", look.name);
                }
                assert!(!says_it_ran(&mut harness), "{}", look.name);
                harness.click("Results");
                assert!(harness.has(text), "{text} in {}", look.name);
                assert!(!says_it_ran(&mut harness), "{}", look.name);
                assert_eq!(count(&mut harness), None);
            }
        }
    }

    #[test]
    fn a_run_stopped_between_statements_says_where() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT 1;\nSELECT 2;\nSELECT 3");
        run_all(&mut harness);
        // As every driver reports it: the statement the run was stopped
        // before is its last result, and it is the cancelled one.
        let outcome = script_outcome(vec![done(None), StatementOutcome::Cancelled]);
        assert!(outcome.stopped);
        harness.answer_sql(Ok(outcome), Some(CancelReason::User));
        assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
        for line in [
            "Line 1: Statement ran · 14 ms",
            "Line 2: Cancelled",
            "Line 3: Not run",
        ] {
            assert!(harness.has(line), "{line}");
        }
        // No statement returned rows and the run was stopped: Results
        // says that, not that the statement ran.
        harness.click("Results");
        assert!(harness.has("Cancelled"));
        assert!(!says_it_ran(&mut harness));
    }

    #[test]
    fn the_statements_after_a_cancelled_one_did_not_run() {
        let timeout = CancelReason::Timeout(Duration::from_secs(30));
        for (cancel, text) in [
            (Some(CancelReason::User), "Cancelled"),
            (Some(timeout), "Cancelled after 30 s (timeout)"),
        ] {
            let script = "SELECT 1;\nSELECT 2;\nSELECT 3;\nSELECT 4";
            let (mut harness, _tab) = editor(Look::standard(), script);
            run_all(&mut harness);
            // A cancel in the middle of the second statement of four.
            let outcome = script_outcome(vec![rows_outcome(2), StatementOutcome::Cancelled]);
            assert!(outcome.stopped);
            harness.answer_sql(Ok(outcome), cancel);
            harness.click("Messages");
            let tree = harness.settle();
            let mut lines: Vec<String> = labels(&tree)
                .into_iter()
                .filter(|label| label.starts_with("Line ") && label.contains(':'))
                .collect();
            lines.sort();
            assert_eq!(
                lines,
                [
                    "Line 1: 2 rows · 14 ms".to_owned(),
                    format!("Line 2: {text}"),
                    "Line 3: Not run".to_owned(),
                    "Line 4: Not run".to_owned(),
                ]
            );
        }
    }

    #[test]
    fn a_run_that_failed_as_a_whole_shows_its_own_text_and_no_older_rows() {
        let errors = [
            Error::Refused {
                line: 1,
                what: "COMMIT".into(),
            },
            Error::Unsupported("SQL editor on MySQL before 5.7"),
            Error::LeftReadOnly,
            Error::ConnectionLost("the server went away".into()),
        ];
        for look in Look::ALL {
            for error in &errors {
                let (mut harness, tab) = editor(look, "SELECT 1");
                run(&mut harness);
                harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
                assert!(harness.has("Row 5"));
                run(&mut harness);
                harness.answer_sql(Err(error.clone()), None);
                let text = error.to_string();
                let context = format!("{text} in {}", look.name);
                // The messages open and say what failed, as it says it.
                assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
                assert!(harness.has(&text), "{context}");
                assert!(!harness.has("Line 1: 5 rows · 14 ms"), "{context}");
                // Results says the same: the older run's rows are gone.
                harness.click("Results");
                assert_eq!(sql(&harness, tab).dims(), (0, 0));
                assert!(harness.has(&text), "{context}");
                assert!(!harness.has("Row 1"), "{context}");
                assert!(!harness.has("Statement at line 1 · 14 ms"), "{context}");
                assert_eq!(count(&mut harness), None, "{context}");
                assert!(painted(&harness, &text), "as the database said it");
                // Nor do they come back while the next run is on its way.
                run(&mut harness);
                assert!(sql(&harness, tab).is_running());
                assert!(!harness.has("Row 1"), "{context}");
                assert!(!harness.has(&text), "{context}");
                show_pane(&mut harness, tab, ResultPane::Messages);
                assert!(!harness.has("Line 1: 5 rows · 14 ms"), "{context}");
                harness.answer_sql(Ok(script_outcome(vec![rows_outcome(2)])), None);
                assert!(harness.has("Row 2") && !harness.has("Row 3"), "{context}");
            }
        }
    }

    /// The server's refusal of a write in a read-only transaction.
    fn read_only_refusal() -> StatementOutcome {
        StatementOutcome::Error {
            error: Error::Query {
                code: Some("25006".into()),
                message: "cannot execute UPDATE in a read-only transaction".into(),
                detail: None,
                hint: None,
            },
            position: None,
        }
    }

    #[test]
    fn a_refused_write_reads_as_a_limit_of_this_version() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
            // The fixture's session is SQLite's: the code is PostgreSQL's.
            harness.app.workspace_mut(tab).unwrap().driver = tabletist_db::Driver::Postgres;
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![read_only_refusal()])), None);
            show_pane(&mut harness, tab, ResultPane::Results);
            let title = look.label("This version only reads data");
            assert!(harness.has(&title), "{title} in {}", look.name);
            assert!(
                harness.has("25006 · cannot execute UPDATE in a read-only transaction"),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn a_refused_write_in_a_short_pane_scrolls_to_its_last_line() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "UPDATE users SET email = 'x'");
            let workspace = harness.app.workspace_mut(tab).unwrap();
            workspace.driver = tabletist_db::Driver::Postgres;
            // The editor takes most of the height: the results have room
            // for the card's first lines and not for its last.
            let id = workspace.active_tab.unwrap();
            workspace.sql_tab_mut(id).unwrap().split = 0.8;
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![read_only_refusal()])), None);
            show_pane(&mut harness, tab, ResultPane::Results);
            let title = look.label("This version only reads data");
            let last = look.label("Editing arrives in a later version.");
            // The card's title and its last line, and how far down the
            // pane shows anything: to the footer under it, or to the
            // window's end in the terminal look, which has none.
            let places = |harness: &mut Harness| {
                let tree = harness.settle();
                let place = |label: &str| bounds(&tree, label, Role::Label);
                let footer = labels(&tree)
                    .into_iter()
                    .find(|label| label.starts_with("Ln "));
                let bottom = match footer.and_then(|footer| place(&footer)) {
                    Some(footer) => footer.top(),
                    None => harness.size.y,
                };
                let [title, last] = [&title, &last].map(|label| place(label).expect("a label"));
                (title, last, bottom)
            };
            let (title, before, bottom) = places(&mut harness);
            assert!(before.bottom() > bottom, "{}: {before:?}", look.name);
            // The wheel over the card brings the rest of it up.
            harness.frame(vec![egui::Event::PointerMoved(title.center())]);
            harness.frame(vec![egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0.0, -600.0),
                modifiers: Modifiers::NONE,
                phase: egui::TouchPhase::Move,
            }]);
            for _ in 0..60 {
                harness.frame(Vec::new());
            }
            let (_, after, bottom) = places(&mut harness);
            assert!(after.bottom() <= bottom, "{}: {after:?}", look.name);
        }
    }

    #[test]
    fn another_error_keeps_its_own_words_in_the_results() {
        let look = Look::macos();
        let (mut harness, tab) = editor(look, "SELECT nope");
        run(&mut harness);
        let failed = error_outcome("no such column: nope", None);
        harness.answer_sql(Ok(script_outcome(vec![failed])), None);
        show_pane(&mut harness, tab, ResultPane::Results);
        assert!(!harness.has("This version only reads data"));
    }

    #[test]
    fn the_messages_tab_counts_the_errors_of_the_last_run() {
        let value = |harness: &mut Harness| {
            let tree = harness.settle();
            let id = node(&tree, "Messages", Role::Button).expect("the Messages tab");
            let (_, tab) = tree.nodes.iter().find(|(node, _)| *node == id).unwrap();
            tab.value().map(str::to_owned)
        };
        let (mut harness, _tab) = editor(Look::macos(), "SELECT nope");
        assert_eq!(value(&mut harness), None);
        run(&mut harness);
        let failed = error_outcome("no such column: nope", None);
        harness.answer_sql(Ok(script_outcome(vec![failed])), None);
        assert_eq!(value(&mut harness).as_deref(), Some("1"));
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![rows_outcome(2)])), None);
        assert_eq!(value(&mut harness), None);
    }

    #[test]
    fn arrows_do_not_move_in_rows_a_failed_run_hides() {
        for look in [Look::standard(), Look::omarchy()] {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(5)])), None);
            run(&mut harness);
            harness.answer_sql(Err(Error::LeftReadOnly), None);
            show_pane(&mut harness, tab, ResultPane::Results);
            harness.press(Key::Escape, Modifiers::NONE);
            for key in [Key::ArrowDown, Key::End, Key::J] {
                harness.press(key, Modifiers::NONE);
            }
            assert_eq!(sql(&harness, tab).selection, None, "{}", look.name);
        }
    }

    /// The request of the newest run sent to the backend.
    fn run_request(harness: &Harness) -> RequestId {
        let sent = harness.app.backend.sent.iter().rev();
        sent.filter_map(|command| match command {
            Command::RunSql { request, .. } => Some(*request),
            _ => None,
        })
        .next()
        .expect("a RunSql was sent")
    }

    #[test]
    fn a_running_script_shows_its_time_and_a_cancel_button() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            assert!(sql(&harness, tab).is_running());
            let tree = harness.settle();
            let running = labels(&tree)
                .into_iter()
                .find(|label| label.starts_with("Running · "))
                .unwrap_or_else(|| panic!("no time in {}: {:?}", look.name, labels(&tree)));
            assert!(running.ends_with(" s"), "{running}");
            assert!(node(&tree, "Cancel query", Role::Button).is_some());
            // Nothing ran before: no hint, no rows, no count.
            assert!(!harness.has(&hint(&look)), "{}", look.name);
            assert_eq!(count(&mut harness), None);
            let request = run_request(&harness);
            harness.click("Cancel query");
            assert!(
                matches!(
                    crate::testing::last_sent(&harness.app),
                    Command::Cancel { request: cancelled, .. } if *cancelled == request
                ),
                "{}",
                look.name
            );
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
            assert!(harness.has("Row 3") && !harness.has("Cancel query"));
            assert!(harness.has("Statement at line 1 · 14 ms"));
            // The next run leaves the rows up until it answers, and says
            // it is on its way in their place's header.
            run(&mut harness);
            let tree = harness.settle();
            assert!(node(&tree, "Row 3", Role::Button).is_some());
            assert!(node(&tree, "Cancel query", Role::Button).is_some());
            assert!(node(&tree, "Statement at line 1 · 14 ms", Role::Label).is_none());
        }
    }

    /// The width of the editor's panes: the splitter spans them.
    fn pane_width(harness: &mut Harness) -> f32 {
        let tree = harness.settle();
        let band = bounds(&tree, "Resize the editor", Role::Unknown).expect("the splitter");
        band.width()
    }

    /// Narrows the window until the editor's panes are `width` points
    /// wide. 320 is what the smallest window leaves beside the widest
    /// sidebar.
    fn narrow_to(harness: &mut Harness, width: f32) {
        harness.size.x -= pane_width(harness) - width;
        let now = pane_width(harness);
        assert!((now - width).abs() <= 1.0, "the pane is {now} wide");
    }

    #[test]
    fn the_header_keeps_its_buttons_apart_in_the_narrowest_pane() {
        for look in Look::ALL {
            let (mut harness, _tab) = editor(look, "SELECT 1");
            narrow_to(&mut harness, 320.0);
            // A count beside Results is the widest the tabs get.
            run(&mut harness);
            let page = crate::testing::page(10_000, false);
            let many = StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: true,
            };
            harness.answer_sql(Ok(script_outcome(vec![many])), None);
            assert_eq!(count(&mut harness).as_deref(), Some("10,000"));
            run(&mut harness);
            let tree = harness.settle();
            let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, harness.size);
            let parts: Vec<(&str, egui::Rect)> = ["Results", "Messages", "Cancel query"]
                .into_iter()
                .map(|name| {
                    let rect = bounds(&tree, name, Role::Button)
                        .unwrap_or_else(|| panic!("{name} missing in {}", look.name));
                    assert!(
                        screen.contains_rect(rect),
                        "{name} at {rect:?} in {}",
                        look.name
                    );
                    (name, rect)
                })
                .collect();
            for (index, (name, rect)) in parts.iter().enumerate() {
                for (other, other_rect) in &parts[index + 1..] {
                    assert!(
                        !rect.intersects(*other_rect),
                        "{name} overlaps {other} in {}",
                        look.name
                    );
                }
            }
        }
    }

    #[test]
    fn the_messages_keep_their_lines_apart_in_the_narrowest_pane() {
        for look in Look::ALL {
            let script = "SELECT 1;\nSELECT a_column_that_is_not_there;\nSELECT 2";
            let (mut harness, tab) = editor(look, script);
            narrow_to(&mut harness, 320.0);
            run_all(&mut harness);
            let page = crate::testing::page(1_000, false);
            let cut = StatementOutcome::Rows {
                columns: page.columns,
                rows: page.rows,
                truncated: true,
            };
            // Longer than the pane is wide, as a database's words can be.
            let said = "no such column: a_column_that_is_not_there, \
                        and nothing like it in any table of the query";
            harness.answer_sql(
                Ok(script_outcome(vec![cut, error_outcome(said, None)])),
                None,
            );
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            let tree = harness.settle();
            // Named in full, however the pane cuts or wraps them.
            let names = [
                "Line 1: 1,000 rows (limit reached) · 14 ms".to_owned(),
                format!("Line 2: {said}"),
                "Line 3: Not run".to_owned(),
            ];
            let lines: Vec<egui::Rect> = names
                .iter()
                .map(|name| {
                    bounds(&tree, name, Role::Label)
                        .unwrap_or_else(|| panic!("{name} in {}: {:?}", look.name, labels(&tree)))
                })
                .collect();
            // A line of ours is one row, its text a piece of its own that
            // is cut where the pane ends: it is not painted as it would
            // wrap, over the line under it.
            let whole = "1,000 rows (limit reached) · 14 ms";
            let ours = |harness: &Harness| {
                let pieces = harness.painted.iter().map(|(piece, _)| piece.clone());
                let mut ours = pieces.filter(|piece| piece.starts_with("1,000 rows ("));
                ours.next()
                    .unwrap_or_else(|| panic!("{}: {:?}", look.name, harness.painted))
            };
            let is_cut = |piece: &str| {
                piece.ends_with('…') && whole.starts_with(piece.trim_end_matches('…'))
            };
            let piece = ours(&harness);
            assert!(
                piece == whole || is_cut(&piece),
                "{piece:?} in {}",
                look.name
            );
            // What the database said takes the rows it needs, and the
            // line after it starts under them.
            assert!(
                lines[1].height() > lines[0].height() * 1.5,
                "{:?} in {}",
                lines[1],
                look.name
            );
            for pair in lines.windows(2) {
                assert!(
                    pair[1].top() >= pair[0].bottom() - 0.5,
                    "{pair:?} in {}",
                    look.name
                );
            }
            assert_eq!(
                harness.painted_color(said),
                Some(harness.app.palette.danger),
                "the whole of it is painted in {}",
                look.name
            );
            // Narrower than any window leaves it, the line of ours is
            // cut for certain, and still named in full.
            narrow_to(&mut harness, 240.0);
            let tree = harness.settle();
            let piece = ours(&harness);
            assert!(is_cut(&piece), "{piece:?} in {}", look.name);
            assert!(node(&tree, &names[0], Role::Label).is_some());
        }
    }

    #[test]
    fn the_terminal_header_keeps_clear_of_the_splitters_handle() {
        let (mut harness, _tab) = editor(Look::omarchy(), "SELECT 1");
        run(&mut harness);
        let tree = harness.settle();
        // The rule's handle reaches 3 pt into the results and takes the
        // pointer there.
        let handle = bounds(&tree, "Resize the editor", Role::Unknown).expect("the splitter");
        for name in ["Results", "Messages", "Cancel query"] {
            let rect = bounds(&tree, name, Role::Button).expect(name);
            assert!(
                rect.top() >= handle.bottom(),
                "{name} at {rect:?} is under the handle at {handle:?}"
            );
        }
    }

    /// The name of what has the keyboard.
    fn focused(harness: &mut Harness) -> String {
        let tree = harness.settle();
        let node = tree.nodes.iter().find(|(id, _)| *id == tree.focus);
        node.and_then(|(_, node)| node.label().or_else(|| node.value()))
            .unwrap_or_default()
            .to_owned()
    }

    #[test]
    fn tab_passes_a_results_column_headers_by() {
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT 1");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![rows_outcome(3)])), None);
            // From the Results tab on (the editor before it keeps Tab to
            // indent with), as a screen reader puts the keyboard there.
            let tree = harness.settle();
            let results = node(&tree, "Results", Role::Button).expect("the Results tab");
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: results,
                    action: egui::accesskit::Action::Focus,
                    data: None,
                },
            )]);
            let mut stops = vec![focused(&mut harness)];
            for _ in 0..12 {
                harness.press(Key::Tab, Modifiers::NONE);
                stops.push(focused(&mut harness));
            }
            // It goes on to Messages and the rows (one stop for all of
            // them), not the headers.
            for stop in ["Results", "Messages", "Rows"] {
                assert!(stops.iter().any(|name| name == stop), "{stop}: {stops:?}");
            }
            for header in ["id", "email", "meta"] {
                assert!(
                    !stops.iter().any(|name| name == header),
                    "{header} in {}: {stops:?}",
                    look.name
                );
            }
            // Nor does a click on one ask for anything.
            let sent = harness.app.backend.sent.len();
            let tree = harness.settle();
            let email = node(&tree, "email", Role::Label).expect("the header");
            harness.frame(vec![egui::Event::AccessKitActionRequest(
                egui::accesskit::ActionRequest {
                    target_tree: egui::accesskit::TreeId::ROOT,
                    target_node: email,
                    action: egui::accesskit::Action::Click,
                    data: None,
                },
            )]);
            harness.settle();
            assert_eq!(harness.app.backend.sent.len(), sent);
            assert_eq!(sql(&harness, tab).selection, None);
        }
    }

    #[test]
    fn a_huge_message_builds_a_bounded_amount_of_text() {
        // PostgreSQL repeats a literal it cannot read: 200 KB of it.
        let literal = "9".repeat(200_000);
        let huge = format!("invalid input syntax for type integer: \"{literal}\"");
        let failed = StatementOutcome::Error {
            error: Error::Query {
                code: Some("22P02".into()),
                message: huge.clone(),
                detail: Some(huge.clone()),
                hint: Some(huge.clone()),
            },
            position: Some(8),
        };
        let lost = Error::ConnectionLost(huge.clone());
        // What a frame hands to layout and to screen readers: no piece of
        // text longer than a message may be, with its line before it.
        let most = format::MESSAGE_MAX_CHARS + 64;
        let bounded = |harness: &mut Harness, state: &str| {
            let tree = harness.settle();
            for label in labels(&tree) {
                let length = label.chars().count();
                assert!(length <= most, "a name of {length} characters, {state}");
            }
            for (piece, _) in &harness.painted {
                let length = piece.chars().count();
                assert!(length <= most, "{length} characters painted, {state}");
            }
        };
        for look in Look::ALL {
            let (mut harness, tab) = editor(look, "SELECT '9'::int");
            run(&mut harness);
            harness.answer_sql(Ok(script_outcome(vec![failed.clone()])), None);
            assert_eq!(sql(&harness, tab).pane, ResultPane::Messages);
            bounded(&mut harness, "in Messages");
            // Its start is there, and says what went wrong.
            let start = "Line 1, col 8: invalid input syntax for type integer: \"999";
            let tree = harness.settle();
            let line = labels(&tree)
                .into_iter()
                .find(|label| label.starts_with(start))
                .unwrap_or_else(|| panic!("no line in {}", look.name));
            assert!(line.ends_with('…'), "cut in {}", look.name);
            show_pane(&mut harness, tab, ResultPane::Results);
            bounded(&mut harness, "in Results");
            // A run that failed as a whole with as much to say.
            run(&mut harness);
            harness.answer_sql(Err(lost.clone()), None);
            bounded(&mut harness, "in Messages, of a whole run");
            show_pane(&mut harness, tab, ResultPane::Results);
            bounded(&mut harness, "in Results, of a whole run");
            assert!(
                harness
                    .painted
                    .iter()
                    .any(|(piece, _)| piece.starts_with("the connection was lost: invalid")),
                "{}",
                look.name
            );
        }
    }

    #[test]
    fn the_time_a_run_shows_counts_from_when_it_was_queued() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT 1");
        run(&mut harness);
        // As if queued a minute and a half ago.
        let id = sql(&harness, tab).id;
        let workspace = harness.app.workspace_mut(tab).unwrap();
        let in_flight = workspace.sql_tab_mut(id).unwrap().in_flight.as_mut();
        let earlier = std::time::Instant::now().checked_sub(Duration::from_secs(90));
        in_flight.unwrap().started = earlier.expect("a clock 90 s old");
        let tree = harness.settle();
        let running = labels(&tree)
            .into_iter()
            .find(|label| label.starts_with("Running · "))
            .expect("the time");
        let seconds: f64 = running
            .trim_start_matches("Running · ")
            .trim_end_matches(" s")
            .parse()
            .unwrap_or_else(|_| panic!("{running}"));
        assert!((90.0..120.0).contains(&seconds), "{running}");
    }

    /// The scroll area the editor's messages were last drawn in.
    fn messages_area(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<Id> {
        let kept: Option<LastMessages> =
            harness.ctx.data(|data| data.get_temp(results_id(tab, id)));
        kept.map(|LastMessages(area)| area)
    }

    #[test]
    fn the_messages_of_an_older_run_are_forgotten() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT x");
        let id = sql(&harness, tab).id;
        let scrolled =
            |harness: &Harness, area| egui::scroll_area::State::load(&harness.ctx, area).is_some();
        let fail = |harness: &mut Harness| {
            run(harness);
            let failed = error_outcome("no such column: x", None);
            harness.answer_sql(Ok(script_outcome(vec![failed])), None);
            harness.settle();
        };
        assert_eq!(messages_area(&harness, tab, id), None);
        fail(&mut harness);
        let first = messages_area(&harness, tab, id).expect("the messages were drawn");
        assert!(scrolled(&harness, first));
        // Each run's messages scroll on their own, from their top; what
        // egui kept for the run before goes when the next is drawn.
        fail(&mut harness);
        let second = messages_area(&harness, tab, id).expect("drawn again");
        assert_ne!(first, second);
        assert!(!scrolled(&harness, first) && scrolled(&harness, second));
        // And the last one's when the editor closes.
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
        assert_eq!(messages_area(&harness, tab, id), None);
        assert!(!scrolled(&harness, second));
    }

    /// A result of three columns, the second named `name`.
    fn result_with(name: &str) -> StatementOutcome {
        let mut page = crate::testing::page(3, false);
        page.columns[1].name = name.into();
        StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        }
    }

    #[test]
    fn a_new_run_fits_its_own_columns() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT 1");
        let width = |harness: &mut Harness, name: &str| {
            let tree = harness.settle();
            bounds(&tree, name, Role::Label)
                .expect("the column")
                .width()
        };
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![result_with("email")])), None);
        let narrow = width(&mut harness, "email");
        let fit = data_view::Fit::of(harness.app.workspace(tab).unwrap(), &harness.app.settings);
        let first = sql(&harness, tab).run.loaded;
        let first = grid_id(tab, sql(&harness, tab).id, first, fit);
        assert!(crate::ui::grid::remembered(&harness.ctx, first));
        // As many columns, one with a far longer name: widths kept from
        // the run before would cut it.
        let long = "the_address_this_user_signed_up_with";
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![result_with(long)])), None);
        let wide = width(&mut harness, long);
        assert!(wide > narrow + 40.0, "{narrow} then {wide}");
        // What egui kept for the older run's grid is dropped.
        let second = sql(&harness, tab).run.loaded;
        let second = grid_id(tab, sql(&harness, tab).id, second, fit);
        assert_ne!(first, second);
        assert!(!crate::ui::grid::remembered(&harness.ctx, first));
        assert!(crate::ui::grid::remembered(&harness.ctx, second));
        // And for the last one when its editor closes.
        harness.press(Key::W, Modifiers::COMMAND);
        assert!(harness.app.workspace(tab).unwrap().tabs.is_empty());
        assert!(!crate::ui::grid::remembered(&harness.ctx, second));
    }

    /// The grid the editor's result was last drawn with.
    fn result_grid(harness: &Harness, tab: ConnTabId, id: TabId) -> Option<Id> {
        let kept: Option<LastGrid> = harness.ctx.data(|data| data.get_temp(results_id(tab, id)));
        kept.map(|LastGrid(grid)| grid)
    }

    #[test]
    fn a_result_on_screen_is_fitted_again_when_the_file_changes_value_tags() {
        let (mut harness, tab) = editor(Look::standard(), "SELECT 1");
        // A boolean column with no type under its name: its values decide
        // how wide it is.
        let mut page = crate::testing::page(3, false);
        page.columns[2].name = "ok".into();
        page.columns[2].type_name = String::new();
        page.columns[2].kind = ValueKind::Bool;
        for row in &mut page.rows {
            row[2] = tabletist_db::Value::Bool(false);
        }
        let result = StatementOutcome::Rows {
            columns: page.columns,
            rows: page.rows,
            truncated: false,
        };
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![result])), None);
        let id = sql(&harness, tab).id;
        let ran = sql(&harness, tab).run.loaded;
        // The grid the result is drawn with once the file reads `text`, and
        // how wide that grid makes the boolean column.
        let drawn = |harness: &mut Harness, text: Option<&str>| {
            if let Some(text) = text {
                let text = text.to_owned();
                let file = crate::backend::Event::SettingsFile { text };
                harness.app.apply(Action::Backend(file));
            }
            let tree = harness.settle();
            let grid = result_grid(harness, tab, id).expect("the result was drawn");
            let column = bounds(&tree, "ok", Role::Label).expect("the column");
            (grid, column.width())
        };
        let remembered = |harness: &Harness, grid| crate::ui::grid::remembered(&harness.ctx, grid);
        let (tagged, padded) = drawn(&mut harness, None);
        assert!(remembered(&harness, tagged));
        // The same run under the other option is another grid, fitted on
        // its own: a tag pads its text, and the plain text needs less. What
        // egui kept for the grid before is dropped.
        let (plain, bare) = drawn(&mut harness, Some("[data]\nvalue_tags = false\n"));
        assert_eq!(sql(&harness, tab).run.loaded, ran, "nothing ran again");
        assert_ne!(tagged, plain);
        assert!(bare < padded, "{padded} then {bare}");
        assert!(!remembered(&harness, tagged) && remembered(&harness, plain));
        // And the other way: the widths of the plain text would cut a tag.
        let (again, wide) = drawn(&mut harness, Some("[data]\nvalue_tags = true\n"));
        assert_eq!(again, tagged);
        assert_eq!(wide, padded);
        assert!(remembered(&harness, tagged) && !remembered(&harness, plain));
    }

    #[test]
    fn a_big_result_builds_only_the_rows_in_view() {
        use tabletist_db::{ColumnMeta, Value, ValueKind};
        let columns: Vec<ColumnMeta> = (0..60)
            .map(|index| ColumnMeta {
                name: format!("column_{index}"),
                type_name: "TEXT".into(),
                kind: ValueKind::Text,
            })
            .collect();
        let rows: Vec<Vec<Value>> = (0..10_000)
            .map(|row| {
                (0..60)
                    .map(|col| Value::Text(format!("r{row}c{col}").into()))
                    .collect()
            })
            .collect();
        let big = StatementOutcome::Rows {
            columns,
            rows,
            truncated: true,
        };
        let (mut harness, _tab) = editor(Look::standard(), "SELECT 1");
        run(&mut harness);
        harness.answer_sql(Ok(script_outcome(vec![big])), None);
        assert_eq!(count(&mut harness).as_deref(), Some("10,000"));
        let tree = harness.settle();
        let built = labels(&tree)
            .iter()
            .filter(|label| label.starts_with("Row "))
            .count();
        assert!(built > 2 && built < 40, "{built} rows built");
        // Cells out of view, down or across, are not laid out.
        let cells = harness
            .painted
            .iter()
            .filter(|(piece, _)| piece.starts_with('r') && piece.contains('c'))
            .count();
        assert!(cells > 2 && cells < 40 * 20, "{cells} cells painted");
        assert!(!painted(&harness, "r9999c0") && !painted(&harness, "r0c59"));
    }

    #[test]
    fn a_long_script_builds_only_the_messages_in_view() {
        let script = "SELECT 1;\n".repeat(3_000);
        let (mut harness, tab) = editor(Look::standard(), &script);
        run_all(&mut harness);
        let outcomes = (0..3_000).map(|_| done(None)).collect();
        harness.answer_sql(Ok(script_outcome(outcomes)), None);
        show_pane(&mut harness, tab, ResultPane::Messages);
        let tree = harness.settle();
        let lines: Vec<String> = labels(&tree)
            .into_iter()
            .filter(|label| label.starts_with("Line "))
            .collect();
        assert!(lines.len() > 2 && lines.len() < 60, "{} lines", lines.len());
        assert!(
            lines
                .iter()
                .any(|line| line == "Line 1: Statement ran · 14 ms")
        );
    }
}
