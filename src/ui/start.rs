//! The start screen: what a connection's window shows with no tab open.
//! The connection in one line, what was opened lately on it (on a first
//! visit, its largest tables) and three ways to start. On macOS a column
//! in the middle of the pane with a status bar under it; in the terminal
//! look a dashboard whose keys the status line lists: while the screen has
//! the keyboard, a number opens a row and a letter starts something.

use egui::{CornerRadius, Frame, Margin, Rect, Sense, Stroke, StrokeKind, WidgetInfo, WidgetType};
use egui::{pos2, vec2};
use tabletist_db::{Driver, ObjectKind, ObjectRef};

use crate::app::App;
use crate::env::{Platform, env_colors};
use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Scope, Written};
use crate::model::{Action, ConnTabId, Pane, StartRecent};
use crate::theme::{Icon, Look, Palette};
use crate::typography::{Text, TextRole};
use crate::ui::focus::{self, Region};
use crate::ui::format::display_safe;
use crate::ui::widgets::{self, ButtonSpec};
use crate::ui::workspace::{Badge, BarInfo, bar_info, database_menu, database_menu_id, env_badge};

/// The column's width, and the room above it.
fn column(look: &Look) -> (f32, f32) {
    if look.terminal {
        (760.0, 96.0)
    } else {
        (620.0, 72.0)
    }
}

/// Between the column's parts.
fn gap(look: &Look) -> f32 {
    if look.terminal { 20.0 } else { 22.0 }
}

/// A row of the lists.
fn row_height(look: &Look) -> f32 {
    if look.terminal { 24.0 } else { 36.0 }
}

/// A row's inset, and a list's: 10 at each side.
const INSET: f32 = 10.0;

/// The macOS status bar's height.
const STATUS_BAR: f32 = 32.0;

/// The terminal status line's height.
const STATUS_LINE: f32 = 31.0;

/// What the screen shows of the window, gathered before anything is
/// drawn.
struct Shown {
    conn: crate::connections::ConnectionId,
    /// The database's name, or a SQLite file's.
    database: String,
    info: BarInfo,
    driver: Driver,
    /// The server as it names itself, once it has.
    server: Option<String>,
    /// How many tables and views the shown schema has, once it is listed.
    objects: Option<usize>,
    recents: Vec<StartRecent>,
    /// On a first visit, the shown schema's largest tables.
    largest: Vec<(ObjectRef, Option<u64>)>,
    agents: Option<crate::model::AgentsHere>,
    /// Whether the connection is still saved: Edit connection needs it.
    saved: bool,
    /// Whether the keys are the screen's: the keyboard is in the pane,
    /// and on no control of it.
    has_keys: bool,
}

fn shown(app: &App, ctx: &egui::Context, tab: ConnTabId) -> Option<Shown> {
    let workspace = app.workspace(tab)?;
    let info = bar_info(app, tab)?;
    // A SQLite connection is on a file, which names it.
    let database = match workspace.driver {
        Driver::Sqlite => workspace
            .spec
            .sqlite_path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default(),
        Driver::Postgres | Driver::MySql => info.database.clone(),
    };
    let objects = app
        .start_schema(tab)
        .and_then(|schema| workspace.tree.nodes.get(&schema))
        .and_then(|node| node.objects.value.as_ref())
        .map(Vec::len);
    let recents = app.start_recents(tab);
    let largest = if recents.is_empty() {
        app.start_largest(tab)
    } else {
        Vec::new()
    };
    Some(Shown {
        conn: workspace.conn_id.clone(),
        database,
        info,
        driver: workspace.driver,
        server: workspace.server_version.value.clone(),
        objects,
        recents,
        largest,
        agents: app.agents_here(tab),
        saved: app.connections.get(&workspace.conn_id).is_some(),
        has_keys: workspace.pane == Pane::Grid && !focus::on_control(ctx),
    })
}

/// The keys the screen names, as the keymap writes them for the look.
struct Keys {
    /// New SQL query's key everywhere, and the one the screen reads.
    sql: Written,
    sql_here: Written,
    /// Switch database's key everywhere, and the one the screen reads.
    database: Written,
    database_here: Written,
    edit_here: Written,
    /// Show the Agents tab's key on a table's rows (`ga`), and the one
    /// the screen reads.
    agents_rows: Written,
    agents_here: Written,
    /// The row numbers, and the keys the status line lists after them.
    recent: Written,
    sidebar: Written,
    help: Written,
    /// Quick open's key, written at the macOS field's end: search goes on
    /// from anywhere with it. Empty in the Omarchy look, which has none.
    quick_open: Written,
    /// The search: the keys that move among the matches (`↑↓`, `ctrl+n/p`),
    /// open one, open one in a SQL editor, clear the search, and (Omarchy)
    /// focus the field.
    search_focus: Written,
    search_move: Written,
    search_open: Written,
    search_editor: Written,
    search_clear: Written,
}

/// Where the start screen's search field keeps the keyboard, by window.
pub(crate) fn search_field_id(tab: ConnTabId) -> egui::Id {
    egui::Id::new(("start-search", tab.0))
}

fn keys(app: &App) -> Keys {
    let layout = app.layout();
    let here = |command: Command| app.keymap.key_in(layout, command, Scope::Start);
    let key = |command: Command| app.keymap.key(layout, command);
    Keys {
        sql: app.keymap.key_in(layout, Command::NewSqlTab, Scope::Global),
        sql_here: here(Command::NewSqlTab),
        database: app
            .keymap
            .key_in(layout, Command::SwitchDatabase, Scope::Global),
        database_here: here(Command::SwitchDatabase),
        edit_here: here(Command::EditConnection),
        agents_rows: app.keymap.key_in(layout, Command::ShowAgents, Scope::Grid),
        agents_here: here(Command::ShowAgents),
        recent: here(Command::OpenRecent),
        sidebar: key(Command::ToggleSidebar),
        help: key(Command::Help),
        quick_open: key(Command::FindObject),
        search_focus: here(Command::StartSearchFocus),
        search_move: app.keymap.label(layout, Command::StartSearchMove),
        search_open: here(Command::StartSearchOpen),
        search_editor: here(Command::StartSearchOpenInEditor),
        search_clear: here(Command::StartSearchClear),
    }
}

/// What the screen draws with.
#[derive(Clone, Copy)]
struct Skin<'a> {
    look: &'a Look,
    palette: &'a Palette,
    locale: Locale,
    tab: ConnTabId,
    /// Seconds since the Unix epoch, for how long ago a row was opened.
    now: u64,
    /// Whether numbers are written in threes.
    grouped: bool,
}

impl Skin<'_> {
    fn say(&self, text: &'static str) -> String {
        self.look.label(&gettext(self.locale, text))
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let Some(shown) = shown(app, ui.ctx(), tab) else {
        return;
    };
    let keys = keys(app);
    // The matches of what is typed, grouped (tables, then views); empty
    // where nothing is typed. The window left the field two one-shot
    // flags, and holds what is typed and which match is picked.
    let matches = app.start_matches(tab);
    let (focus, blur, mut query, selected) = app
        .workspace_mut(tab)
        .map(|workspace| {
            let focus = std::mem::take(&mut workspace.focus_start_search);
            let blur = std::mem::take(&mut workspace.blur_start_search);
            let selected = workspace
                .start_selected
                .min(matches.len().saturating_sub(1));
            (focus, blur, workspace.start_search.clone(), selected)
        })
        .unwrap_or_default();
    let searching = !query.trim().is_empty();
    let skin = Skin {
        look: &look,
        palette: &palette,
        locale,
        tab,
        now: crate::util::now_secs(),
        grouped: app.settings.grouped(app.layout()),
    };
    let mut actions = Vec::new();
    if !look.terminal {
        status_bar(ui, skin, searching);
    }
    // The boards' `surface`: the content's white (the window's colour
    // here), as a table's grid and the Agents tab have it.
    egui::CentralPanel::default()
        .frame(Frame::new().fill(palette.window))
        .show(ui, |ui| {
            let full = ui.max_rect();
            focus::region(ui, Region::Start, full);
            // The terminal look's pane: one Tab stop whose keys are the
            // screen's, lit while the keyboard is in it. The other looks
            // put the keyboard on the screen's controls.
            if look.terminal {
                let pane = ui.interact(full, egui::Id::new(("start-pane", tab.0)), Sense::click());
                let name = gettext(locale, "Start");
                pane.widget_info(|| WidgetInfo::labeled(WidgetType::Other, true, name.clone()));
                focus::claim(ui, Region::Start, &pane);
                focus::pane(ui, &pane);
                if pane.clicked() {
                    pane.request_focus();
                }
                if pane.gained_focus() {
                    actions.push(Action::GridKeys(tab));
                }
                if shown.has_keys {
                    focus::pane_border(ui, full, &look, &palette);
                }
            }
            let (width, top) = column(&look);
            let width = width.min(full.width() - 48.0).max(0.0);
            let column = Rect::from_min_size(
                pos2(full.center().x - width / 2.0, full.top() + top),
                vec2(width, (full.height() - top).max(0.0)),
            );
            let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(column));
            ui.set_clip_rect(full.intersect(ui.clip_rect()));
            ui.spacing_mut().item_spacing.y = 0.0;
            connection_line(&mut ui, &shown, skin);
            ui.add_space(gap(&look));
            search_field(&mut ui, &mut query, &keys, skin, (focus, blur));
            ui.add_space(gap(&look));
            // On macOS the first row, or the first button where there is
            // none, is where F6 brings the keyboard. While searching the
            // matches replace the recents, and the footer of keys the
            // three ways to start.
            let mut claimed = look.terminal;
            if searching {
                results(&mut ui, &matches, selected, &query, skin, &mut actions);
                if !look.terminal {
                    ui.add_space(gap(&look));
                    search_footer(&mut ui, &keys, skin);
                }
            } else {
                lists(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
                ui.add_space(gap(&look));
                starts(&mut ui, &shown, &keys, skin, &mut claimed, &mut actions);
            }
        });
    // What the field edited goes back; a change picks the first match anew.
    if let Some(workspace) = app.workspace_mut(tab)
        && workspace.start_search != query
    {
        workspace.start_search = query;
        actions.push(Action::StartSearchChanged(tab));
    }
    app.actions.extend(actions);
}

/// The words after the database's name: `Read-only` on a production
/// connection that is, the server as it names itself (or its kind, until
/// it has), and how many tables and views the shown schema has.
fn line_words(shown: &Shown, skin: Skin<'_>) -> String {
    let mut parts = Vec::new();
    if shown.info.env.is_production() && shown.info.read_only {
        parts.push(skin.say("Read-only"));
    }
    parts.push(
        shown
            .server
            .clone()
            .unwrap_or_else(|| shown.driver.label().to_owned()),
    );
    if let Some(count) = shown.objects {
        let word = if skin.look.terminal {
            "objects"
        } else {
            "tables and views"
        };
        parts.push(format!("{count} {}", skin.say(word)));
    }
    skin.look.label(&parts.join(" · "))
}

/// The connection in one line: its environment's dot (the terminal's
/// tag), the database in the mono face, then [`line_words`]. On a
/// production connection the line is the production strip, whatever the
/// Environment color setting says of the bar.
fn connection_line(ui: &mut egui::Ui, shown: &Shown, skin: Skin<'_>) {
    let (look, palette) = (skin.look, skin.palette);
    let platform = Platform::of(look);
    let env = shown.info.env;
    let colors = env_colors(env, platform, palette);
    let name = display_safe(&shown.database).into_owned();
    let words = line_words(shown, skin);
    let whole = format!("{name} · {words}");
    let (name_role, words_role) = if look.terminal {
        (TextRole::OBody, TextRole::OBody)
    } else {
        (TextRole::MonoSecondary, widgets::secondary(look))
    };
    let height = if look.terminal { 20.0 } else { 18.0 };
    if env.is_production() {
        // The strip: the production bar's tint and rule, its tag, and the
        // words in its ink.
        let (corner, stroke) = if look.terminal {
            (CornerRadius::ZERO, Stroke::NONE)
        } else {
            (
                CornerRadius::same(look.radius),
                Stroke::new(1.0, colors.bar_border()),
            )
        };
        let shown_at = Frame::new()
            .fill(colors.bar_bg())
            .stroke(stroke)
            .corner_radius(corner)
            .inner_margin(Margin::symmetric(12, 8))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
                let y = rect.center().y;
                let style = if look.terminal {
                    Badge::Tracked
                } else {
                    Badge::Mac
                };
                let mut x = rect.left();
                x += env_badge(ui, x, y, env, &colors, style, look) + 10.0;
                let name = Text::one(look, name_role, &name, palette.text);
                x += widgets::paint_text(ui, x, y, name) + 8.0;
                let ink = if look.terminal {
                    palette.dim
                } else {
                    colors.base()
                };
                // The board's strip has no dot after the name; the
                // terminal's has, as its line does.
                let said = if look.terminal {
                    format!("· {words}")
                } else {
                    words.clone()
                };
                let words = Text::one(look, words_role, &said, ink);
                widgets::paint_text(ui, x, y, words);
                widgets::announce(ui, rect, &whole);
            });
        if look.terminal {
            // The terminal's strip has a rule in the production red above.
            let rect = shown_at.response.rect;
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.danger);
        }
        return;
    }
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
    let y = rect.center().y;
    let mut x = rect.left();
    if look.terminal {
        x += env_badge(ui, x, y, env, &colors, Badge::Tracked, look) + 10.0;
    } else {
        // The dot in the environment's colour.
        ui.painter()
            .circle_filled(pos2(x + 4.0, y), 4.0, colors.base());
        x += 16.0;
    }
    let name_ink = if look.terminal {
        palette.text
    } else {
        palette.secondary
    };
    let name = Text::one(look, name_role, &name, name_ink);
    x += widgets::paint_text(ui, x, y, name) + 8.0;
    let words = Text::one(look, words_role, &format!("· {words}"), palette.dim);
    widgets::paint_text(ui, x, y, words);
    widgets::announce(ui, rect, &whole);
}

/// One row of a list: what it shows, what a screen reader says of it, and
/// what a click does.
struct Row {
    icon: Icon,
    name: String,
    /// The muted words after the name: the kind and the schema, or a
    /// script's first comment.
    detail: String,
    /// At the right: when it was opened, or a table's estimate.
    meta: String,
    label: String,
    action: Action,
}

/// The recents, or on a first visit the largest tables.
fn lists(
    ui: &mut egui::Ui,
    shown: &Shown,
    keys: &Keys,
    skin: Skin<'_>,
    claimed: &mut bool,
    actions: &mut Vec<Action>,
) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let zone = jiff::tz::TimeZone::system();
    let kind_word = |kind: ObjectKind| {
        skin.say(match kind {
            ObjectKind::Table => "table",
            ObjectKind::View => "view",
            ObjectKind::MaterializedView => "materialized view",
        })
    };
    let rows: Vec<Row> = if shown.recents.is_empty() {
        shown
            .largest
            .iter()
            .map(|(object, estimate)| {
                let name = display_safe(&object.name).into_owned();
                let schema = display_safe(&object.schema).into_owned();
                let meta = estimate.map_or_else(String::new, |rows| estimate_text(rows, skin));
                let detail = if look.terminal {
                    kind_word(ObjectKind::Table)
                } else {
                    schema.clone()
                };
                let mut label = format!("{name}, {}, {schema}", kind_word(ObjectKind::Table));
                if !meta.is_empty() {
                    label = format!("{label}, {meta}");
                }
                Row {
                    icon: Icon::Table,
                    name,
                    detail,
                    meta,
                    label,
                    action: Action::OpenObject {
                        tab: skin.tab,
                        object: object.clone(),
                        kind: ObjectKind::Table,
                        pin: true,
                    },
                }
            })
            .collect()
    } else {
        shown
            .recents
            .iter()
            .enumerate()
            .map(|(index, recent)| {
                let when = look.label(&crate::recents::when(recent.opened(), skin.now, &zone));
                let action = Action::OpenRecent {
                    tab: skin.tab,
                    index,
                };
                let opened = skin.say("opened");
                match recent {
                    StartRecent::Object { object, kind, .. } => {
                        let name = display_safe(&object.name).into_owned();
                        let schema = display_safe(&object.schema).into_owned();
                        let kind_word = kind_word(*kind);
                        let detail = if look.terminal {
                            kind_word.clone()
                        } else {
                            format!("{kind_word} · {schema}")
                        };
                        Row {
                            icon: match kind {
                                ObjectKind::Table => Icon::Table,
                                ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
                            },
                            label: format!("{name}, {kind_word}, {schema}, {opened} {when}"),
                            name,
                            detail,
                            meta: when,
                            action,
                        }
                    }
                    StartRecent::Sql {
                        number, comment, ..
                    } => {
                        let name = format!("{} {number}", skin.say("Query"));
                        let editor = skin.say("SQL editor");
                        let detail = if look.terminal {
                            skin.say("sql")
                        } else {
                            comment.clone().unwrap_or_else(|| editor.clone())
                        };
                        let label = match comment {
                            Some(comment) => {
                                format!("{name}, {editor}, {comment}, {opened} {when}")
                            }
                            None => format!("{name}, {editor}, {opened} {when}"),
                        };
                        Row {
                            icon: Icon::Code,
                            name,
                            detail,
                            meta: when,
                            label,
                            action,
                        }
                    }
                }
            })
            .collect()
    };
    // The heading, and what stands at its right: Clear, or the note that
    // the estimates cost nothing.
    let first_visit = shown.recents.is_empty();
    // The terminal's board heads the list `recent`, the Mac's in full.
    let title = if !first_visit {
        if look.terminal {
            gettext(locale, "Recent").into_owned()
        } else {
            gettext(locale, "Recent on this connection").into_owned()
        }
    } else {
        match shown.driver {
            Driver::Postgres | Driver::MySql => gettext(locale, "Largest tables").into_owned(),
            Driver::Sqlite => gettext(locale, "Tables").into_owned(),
        }
    };
    let note = if first_visit {
        Some(match shown.driver {
            Driver::Postgres | Driver::MySql => {
                skin.say("From table statistics. Nothing is queried.")
            }
            Driver::Sqlite => skin.say("Nothing is queried."),
        })
    } else {
        None
    };
    let width = ui.available_width();
    let laid = widgets::section_label(&title, look, palette).layout(ui.ctx());
    let head_height = laid.height() + if look.terminal { 2.0 } else { 6.0 };
    let (head, _) = ui.allocate_exact_size(vec2(width, head_height), Sense::hover());
    let inset = if look.terminal { 2.0 } else { INSET };
    let y = head.top() + laid.height() / 2.0;
    let x = head.left() + inset + laid.paint_left(ui.painter(), head.left() + inset, y);
    // Said as the title is written, whatever case the look paints it in.
    widgets::announce(
        ui,
        Rect::from_min_size(head.min, vec2(x - head.left(), head_height)),
        &title,
    );
    if let Some(note) = note {
        let role = TextRole::pick(look, TextRole::Secondary, TextRole::OCaption);
        if look.terminal {
            let note = Text::one(look, role, &format!(" · {note}"), palette.dim);
            widgets::paint_label(ui, x, y, note);
        } else {
            let right = head.right() - inset;
            let text = || Text::one(look, role, &note, palette.dim);
            widgets::paint_text_right(ui, right, y, text());
            let width = widgets::measure(ui, text());
            widgets::announce(
                ui,
                Rect::from_min_size(pos2(right - width, head.top()), vec2(width, head_height)),
                &note,
            );
        }
    } else if !look.terminal {
        // Clear empties the connection's list without asking: nothing is
        // lost, the objects stay where they are.
        let clear = gettext(locale, "Clear");
        let button = ButtonSpec::new(&clear).link();
        let button_width = button.width(ui, look);
        let place = Rect::from_min_size(
            pos2(head.right() - inset - button_width, head.top() - 2.0),
            vec2(button_width, head_height + 2.0),
        );
        if button.show_at(ui, place, look, palette).clicked() {
            actions.push(Action::ClearRecents(skin.tab));
        }
    }
    for (index, row) in rows.into_iter().enumerate() {
        let response = list_row(ui, index, row, keys, skin, actions);
        if !*claimed {
            focus::claim(ui, Region::Start, &response);
            *claimed = true;
        }
    }
}

/// A table's estimate, as the look writes numbers: `≈ 48,200 rows` on
/// macOS, `≈ 48.2k` in the terminal look.
fn estimate_text(rows: u64, skin: Skin<'_>) -> String {
    if skin.look.terminal {
        skin.look
            .label(&format!("≈ {}", crate::ui::format::compact_count(rows)))
    } else {
        let count = if skin.grouped {
            crate::ui::format::group_digits(rows)
        } else {
            rows.to_string()
        };
        format!("≈ {count} {}", skin.say("rows"))
    }
}

/// One row: on macOS its icon, name, detail and what stands at the right;
/// in the terminal look its number (the key that opens it), name, detail
/// and the right-hand words.
fn list_row(
    ui: &mut egui::Ui,
    index: usize,
    row: Row,
    keys: &Keys,
    skin: Skin<'_>,
    actions: &mut Vec<Action>,
) -> egui::Response {
    let (look, palette) = (skin.look, skin.palette);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, row.label.clone()));
    let hovered = response.hovered();
    if look.terminal {
        if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
        }
    } else {
        if hovered {
            ui.painter()
                .rect_filled(rect, CornerRadius::same(look.radius), palette.surface_hover);
        }
        let radius = look.radius.saturating_sub(2);
        focus::hint(ui, &response, rect, focus::Ring::Inset { radius });
    }
    let y = rect.center().y;
    let left = rect.left() + INSET;
    let right = rect.right() - INSET;
    if look.terminal {
        // The number is the key: the keymap's digits, this one of them.
        let number = if keys.recent.as_str().is_empty() {
            String::new()
        } else {
            (index + 1).to_string()
        };
        widgets::paint_text(
            ui,
            left,
            y,
            Text::one(look, TextRole::OGroup, &number, palette.accent),
        );
        let meta = || Text::one(look, TextRole::OBody, &row.meta, palette.dim);
        let meta_width = widgets::measure(ui, meta());
        widgets::paint_text_right(ui, right, y, meta());
        // The kind in a column of its own, before the right-hand words.
        let detail_x = right - 110.0_f32.max(meta_width + 10.0) - 120.0;
        widgets::paint_text(
            ui,
            detail_x,
            y,
            Text::one(look, TextRole::OBody, &row.detail, palette.dim),
        );
        let room = (detail_x - 10.0 - (left + 28.0)).max(0.0);
        let name = crate::ui::grid::ellipsize(&row.name, room, false, |text| {
            TextRole::OBody.width(ui.ctx(), look.faces, text)
        });
        widgets::paint_text(
            ui,
            left + 28.0,
            y,
            Text::one(look, TextRole::OBody, &name, palette.text),
        );
    } else {
        let icon = Rect::from_center_size(pos2(left + 7.0, y), vec2(14.0, 14.0));
        row.icon.image(palette.dim, 14.0).paint_at(ui, icon);
        let meta = || Text::one(look, widgets::secondary(look), &row.meta, palette.dim);
        let meta_width = widgets::measure(ui, meta());
        widgets::paint_text_right(ui, right, y, meta());
        let mut x = left + 18.0 + INSET;
        let room = (right - meta_width - 10.0 - x).max(0.0);
        let name_role = TextRole::UiBodyStrong;
        let name = crate::ui::grid::ellipsize(&row.name, room, false, |text| {
            name_role.width(ui.ctx(), look.faces, text)
        });
        x += widgets::paint_text(ui, x, y, Text::one(look, name_role, &name, palette.text)) + 8.0;
        let room = (right - meta_width - 10.0 - x).max(0.0);
        let detail = crate::ui::grid::ellipsize(&row.detail, room, false, |text| {
            widgets::secondary(look).width(ui.ctx(), look.faces, text)
        });
        widgets::paint_text(
            ui,
            x,
            y,
            Text::one(look, widgets::secondary(look), &detail, palette.dim),
        );
    }
    if response.clicked() {
        actions.push(row.action);
    }
    response
}

/// The ways to start: three buttons on macOS; in the terminal look the
/// `start` list, each line its key, what it does, and the key that does
/// the same anywhere (or, for agents, who is here).
fn starts(
    ui: &mut egui::Ui,
    shown: &Shown,
    keys: &Keys,
    skin: Skin<'_>,
    claimed: &mut bool,
    actions: &mut Vec<Action>,
) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let switchable = shown.info.databases.len() > 1;
    let no_other = gettext(locale, "This connection has no other database to switch to");
    let not_saved = gettext(locale, "This connection is no longer saved");
    if !look.terminal {
        let (sql, database, edit) = (
            gettext(locale, "New SQL query"),
            gettext(locale, "Switch database"),
            gettext(locale, "Edit connection"),
        );
        let height = 30.0;
        let (row, _) = ui.allocate_exact_size(vec2(ui.available_width(), height), Sense::hover());
        let mut x = row.left() + INSET;
        let mut place = |ui: &mut egui::Ui, button: ButtonSpec<'_>| {
            let width = button.width(ui, look);
            let rect = Rect::from_min_size(pos2(x, row.top()), vec2(width, height));
            x += width + 8.0;
            button.show_at(ui, rect, look, palette)
        };
        let mut button = ButtonSpec::new(&sql).icon(Icon::Code);
        if !keys.sql.as_str().is_empty() {
            button = button.shortcut(&keys.sql);
        }
        let response = place(ui, button);
        if !*claimed {
            focus::claim(ui, Region::Start, &response);
            *claimed = true;
        }
        if response.clicked() {
            actions.push(Action::NewSqlTab(skin.tab));
        }
        let mut button = ButtonSpec::new(&database).icon(Icon::Database);
        if !keys.database.as_str().is_empty() {
            button = button.shortcut(&keys.database);
        }
        if !switchable {
            button = button.disabled(&no_other);
        }
        let response = place(ui, button);
        if switchable {
            let id = database_menu_id(skin.tab, true);
            database_menu(
                &response,
                id,
                skin.tab,
                &shown.info,
                (look, palette),
                actions,
            );
        }
        let mut button = ButtonSpec::new(&edit).icon(Icon::Settings);
        if !shown.saved {
            button = button.disabled(&not_saved);
        }
        if place(ui, button).clicked() {
            actions.push(Action::EditConnection(shown.conn.clone()));
        }
        return;
    }
    let start = gettext(locale, "start");
    let title = widgets::section_label(&start, look, palette).layout(ui.ctx());
    let (head, _) = ui.allocate_exact_size(
        vec2(ui.available_width(), title.height() + 2.0),
        Sense::hover(),
    );
    title.paint_left(
        ui.painter(),
        head.left() + 2.0,
        head.top() + title.height() / 2.0,
    );
    widgets::announce(ui, head, &start);
    // Each line: its key, what it does, and the muted words after.
    let agents_words = shown.agents.as_ref().map(|here| {
        let (said, _, _) = crate::ui::sidebar::agents_said(here, look, palette, locale);
        let key = keys.agents_rows.as_str();
        if key.is_empty() {
            said
        } else {
            format!("{key} · {said}")
        }
    });
    let database_hint = if switchable {
        keys.database.as_str().to_owned()
    } else {
        look.label(&no_other)
    };
    let edit_hint = if shown.saved {
        String::new()
    } else {
        look.label(&not_saved)
    };
    let lines: Vec<(&Written, String, String, Option<Action>)> = vec![
        (
            &keys.sql_here,
            skin.say("new sql query"),
            keys.sql.as_str().to_owned(),
            Some(Action::NewSqlTab(skin.tab)),
        ),
        (
            &keys.database_here,
            skin.say("switch database"),
            database_hint,
            None,
        ),
        (
            &keys.edit_here,
            skin.say("edit connection"),
            edit_hint,
            shown
                .saved
                .then(|| Action::EditConnection(shown.conn.clone())),
        ),
    ];
    let agents = agents_words.map(|words| {
        (
            &keys.agents_here,
            skin.say("agents"),
            words,
            Some(Action::ShowAgents(skin.tab)),
        )
    });
    for (key, words, hint, action) in lines.into_iter().chain(agents) {
        let width = ui.available_width();
        let (rect, response) =
            ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
        let label = if hint.is_empty() {
            words.clone()
        } else {
            format!("{words}, {hint}")
        };
        response.widget_info(|| WidgetInfo::labeled(WidgetType::Button, true, label.clone()));
        if response.hovered() {
            ui.painter()
                .rect_filled(rect, CornerRadius::ZERO, palette.text.gamma_multiply(0.08));
        }
        let y = rect.center().y;
        let left = rect.left() + INSET;
        widgets::paint_text(
            ui,
            left,
            y,
            Text::one(look, TextRole::OGroup, key.as_str(), palette.accent),
        );
        widgets::paint_text(
            ui,
            left + 28.0,
            y,
            Text::one(look, TextRole::OBody, &words, palette.text),
        );
        widgets::paint_text(
            ui,
            left + 28.0 + 220.0,
            y,
            Text::one(look, TextRole::OBody, &hint, palette.dim),
        );
        if words == skin.say("switch database") {
            let id = database_menu_id(skin.tab, true);
            if switchable {
                database_menu(
                    &response,
                    id,
                    skin.tab,
                    &shown.info,
                    (look, palette),
                    actions,
                );
            }
        } else if response.clicked()
            && let Some(action) = action
        {
            actions.push(action);
        }
    }
}

/// The search field under the connection line. macOS: a 44 pt capsule
/// with a magnifier, the placeholder, and the `⌘P` key written at its end
/// (quick open is where search goes on from anywhere). Omarchy: a `/`-led
/// line, as the sidebar's filter. It edits what is typed; `focus` gives it
/// the keyboard when `/` asks (the Omarchy look), `blur` takes it off (esc
/// there). On macOS a letter typed asks for it in `keys.rs`. Its accessible
/// name is the placeholder.
fn search_field(
    ui: &mut egui::Ui,
    query: &mut String,
    keys: &Keys,
    skin: Skin<'_>,
    (focus, blur): (bool, bool),
) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let id = search_field_id(skin.tab);
    // What the search finds, and no more: the index has no functions and
    // the app no saved queries, which the brief's words named too.
    let hint = look.label(&gettext(locale, "Find a table or view"));
    let full = ui.available_width();
    let height = if look.terminal { 30.0 } else { 44.0 };
    let (rect, _) = ui.allocate_exact_size(vec2(full, height), Sense::hover());
    let corner = if look.terminal {
        CornerRadius::same(3)
    } else {
        CornerRadius::same((height / 2.0) as u8)
    };
    let role = TextRole::pick(look, TextRole::UiBody, TextRole::OField);
    let pad = if look.terminal { 8.0 } else { 14.0 };
    // The lead mark: an accent `/` in the terminal look, a magnifier on
    // macOS. The `⌘P` hint stands at the right on macOS; the Omarchy look
    // has no quick open, so its line carries none.
    let slash = Text::one(look, TextRole::OGroup, "/", palette.accent).layout(ui.ctx());
    let mark = if look.terminal { slash.width() } else { 14.0 };
    let shortcut = (!look.terminal && !keys.quick_open.as_str().is_empty()).then(|| {
        let text = format!(
            "{} {}",
            keys.quick_open.as_str(),
            gettext(locale, "from anywhere")
        );
        Text::one(look, widgets::secondary(look), &text, palette.dim).layout(ui.ctx())
    });
    let right_room = shortcut.as_ref().map_or(0.0, |laid| laid.width() + 10.0);
    let left = rect.left() + pad + mark + 8.0;
    let right = rect.right() - pad - right_room;
    let line = role.row_height(ui.ctx(), look.faces);
    let top = rect.center().y - role.middle(ui.ctx(), look.faces);
    let inner = Rect::from_min_max(pos2(left, top), pos2(right.max(left), top + line));
    let backdrop = ui.painter().add(egui::Shape::Noop);
    let mut child = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    let placeholder = Text::one(look, role, &hint, palette.dim).layout(child.ctx());
    let response = child.add(
        egui::TextEdit::singleline(query)
            .id(id)
            .font(role.font_id(look.faces))
            .frame(Frame::NONE)
            .margin(Margin::ZERO)
            .desired_width(inner.width())
            .text_color(palette.text)
            .hint_text(placeholder.galley),
    );
    // The accessible name is the placeholder, so a screen reader names
    // the field whether it is empty or holds a query.
    response.widget_info(|| WidgetInfo::labeled(WidgetType::TextEdit, true, &hint));
    // Esc is the search's: it clears the search (`keys.rs`), where egui
    // would take the field's keyboard away first and leave the key unread.
    // The field set its own filter while it drew (the arrows); egui
    // replaces the whole filter, so those are named again. Tab still steps
    // on. It holds from the frame after the field first had the keyboard.
    if response.has_focus() {
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                id,
                egui::EventFilter {
                    tab: false,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: true,
                },
            );
        });
    }
    if focus {
        response.request_focus();
    }
    if blur {
        response.surrender_focus();
    }
    // The ring of the keyboard is `focus.rs`'s to draw, round the whole
    // box rather than the line of text inside it: the accent border and,
    // on macOS, the halo, while the keyboard put the focus there.
    focus::hint(
        ui,
        &response,
        rect,
        focus::Ring::Field { radius: corner.nw },
    );
    // The boards': the content's own white on macOS, set apart by its
    // border and halo; the sunken tone of an input on Omarchy.
    let fill = if look.terminal {
        palette.panel
    } else {
        palette.window
    };
    ui.painter().set(
        backdrop,
        egui::epaint::RectShape::filled(rect, corner, fill),
    );
    let border = if look.terminal {
        palette.outline
    } else {
        palette.border
    };
    ui.painter()
        .rect_stroke(rect, corner, Stroke::new(1.0, border), StrokeKind::Inside);
    let y = rect.center().y;
    if look.terminal {
        slash.paint_left(ui.painter(), rect.left() + pad, y);
    } else {
        Icon::Search.image(palette.dim, 14.0).paint_at(
            ui,
            Rect::from_center_size(pos2(rect.left() + pad + 7.0, y), vec2(14.0, 14.0)),
        );
    }
    if let Some(shortcut) = shortcut {
        shortcut.paint_right(ui.painter(), rect.right() - pad, y);
    }
}

/// The matches of what is typed, in the place of the recents: the tables
/// first and the views after, each under its heading, the picked match on
/// the selection's ground and the run of the name that matched in the
/// accent. Opening a row opens its object.
fn results(
    ui: &mut egui::Ui,
    matches: &[(ObjectRef, ObjectKind)],
    selected: usize,
    query: &str,
    skin: Skin<'_>,
    actions: &mut Vec<Action>,
) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    if matches.is_empty() {
        let text = gettext(locale, "No matches");
        let laid = Text::one(look, widgets::secondary(look), &text, palette.dim);
        let (rect, _) =
            ui.allocate_exact_size(vec2(ui.available_width(), row_height(look)), Sense::hover());
        let inset = if look.terminal { 2.0 } else { INSET };
        widgets::paint_label(ui, rect.left() + inset, rect.center().y, laid);
        widgets::announce(ui, rect, &look.label(&text));
        return;
    }
    if look.terminal {
        // One list, headed `matches`, the kind of each in a column of its
        // own: the terminal board's dashboard, not the Mac's sections.
        group_header(ui, &gettext(locale, "matches"), skin);
        for (index, entry) in matches.iter().enumerate() {
            result_row(ui, index, entry, query, index == selected, skin, actions);
        }
    } else {
        // The matches under the kind they are, tables first, then views,
        // as `start_matches` already orders them.
        let tables = matches
            .iter()
            .take_while(|(_, k)| *k == ObjectKind::Table)
            .count();
        for (title, range) in [("Tables", 0..tables), ("Views", tables..matches.len())] {
            if range.is_empty() {
                continue;
            }
            group_header(ui, &gettext(locale, title), skin);
            for index in range {
                result_row(
                    ui,
                    index,
                    &matches[index],
                    query,
                    index == selected,
                    skin,
                    actions,
                );
            }
        }
    }
}

/// A plain section heading over a group of matches: "Tables" or "Views",
/// as the look writes a section label.
fn group_header(ui: &mut egui::Ui, title: &str, skin: Skin<'_>) {
    let (look, palette) = (skin.look, skin.palette);
    let laid = widgets::section_label(title, look, palette).layout(ui.ctx());
    let head_height = laid.height() + if look.terminal { 2.0 } else { 6.0 };
    let (head, _) = ui.allocate_exact_size(vec2(ui.available_width(), head_height), Sense::hover());
    let inset = if look.terminal { 2.0 } else { INSET };
    let y = head.top() + laid.height() / 2.0;
    let width = laid.paint_left(ui.painter(), head.left() + inset, y);
    widgets::announce(
        ui,
        Rect::from_min_size(head.min, vec2(width, head_height)),
        title,
    );
}

/// One match: on macOS its kind's icon, the name with the matched run in
/// the accent, and its schema; in the terminal look a cursor bar on the
/// picked one, the name and the kind. A click opens it.
fn result_row(
    ui: &mut egui::Ui,
    index: usize,
    entry: &(ObjectRef, ObjectKind),
    query: &str,
    picked: bool,
    skin: Skin<'_>,
    actions: &mut Vec<Action>,
) {
    let (object, kind) = (&entry.0, entry.1);
    let (look, palette) = (skin.look, skin.palette);
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(vec2(width, row_height(look)), Sense::click());
    let name = display_safe(&object.name).into_owned();
    let schema = display_safe(&object.schema).into_owned();
    let kind_word = skin.say(match kind {
        ObjectKind::Table => "table",
        ObjectKind::View | ObjectKind::MaterializedView => "view",
    });
    let label = format!("{name}, {kind_word}, {schema}");
    response
        .widget_info(|| WidgetInfo::selected(WidgetType::SelectableLabel, true, picked, &label));
    widgets::selection(ui, rect, picked, response.hovered(), look, palette);
    if !look.terminal {
        focus::hint(
            ui,
            &response,
            widgets::selection_rect(rect, look),
            focus::Ring::Inset {
                radius: look.radius.saturating_sub(2),
            },
        );
    }
    let [before, hit, after] = highlight(&name, query);
    let y = rect.center().y;
    let left = rect.left() + INSET;
    let right = rect.right() - INSET;
    if look.terminal {
        let marker = if picked { "▌" } else { "" };
        widgets::paint_text(
            ui,
            left,
            y,
            Text::one(look, TextRole::OGroup, marker, palette.accent),
        );
        let detail_x = right - 120.0;
        widgets::paint_text(
            ui,
            detail_x,
            y,
            Text::one(look, TextRole::OBody, &kind_word, palette.dim),
        );
        let room = (detail_x - 10.0 - (left + 28.0)).max(0.0);
        let name = Text::new(look)
            .add(TextRole::OBody, &before, palette.text)
            .add(TextRole::OGroup, &hit, palette.accent)
            .add(TextRole::OBody, &after, palette.text);
        name.wrap(room)
            .layout(ui.ctx())
            .paint_left(ui.painter(), left + 28.0, y);
    } else {
        let icon = match kind {
            ObjectKind::Table => Icon::Table,
            ObjectKind::View | ObjectKind::MaterializedView => Icon::Eye,
        };
        let ink = if picked { palette.accent } else { palette.dim };
        icon.image(ink, 14.0).paint_at(
            ui,
            Rect::from_center_size(pos2(left + 7.0, y), vec2(14.0, 14.0)),
        );
        let x = left + 18.0 + INSET;
        let name_ink = if picked {
            palette.accent_hover
        } else {
            palette.text
        };
        let name = Text::new(look)
            .add(TextRole::UiBodyStrong, &before, name_ink)
            .add(TextRole::UiBodyStrong, &hit, palette.accent)
            .add(TextRole::UiBodyStrong, &after, name_ink);
        let laid = name.layout(ui.ctx());
        let after_name = x + laid.width() + 8.0;
        laid.paint_left(ui.painter(), x, y);
        widgets::paint_text(
            ui,
            after_name,
            y,
            Text::one(look, widgets::secondary(look), &schema, palette.dim),
        );
    }
    if response.clicked() {
        actions.push(Action::StartSearchOpen {
            tab: skin.tab,
            index: Some(index),
            in_editor: false,
        });
    }
}

/// Splits `name` into what stands before the run of it that matched
/// `query`, that run, and what stands after. A contiguous, case-ignoring
/// run only, and only where lowercasing leaves the bytes in place (ASCII
/// names); a scattered or qualified query highlights nothing.
fn highlight(name: &str, query: &str) -> [String; 3] {
    let needle = query.trim();
    let nothing = || [name.to_owned(), String::new(), String::new()];
    if needle.is_empty() {
        return nothing();
    }
    let lower = name.to_lowercase();
    let found = lower.find(&needle.to_lowercase());
    if lower.len() == name.len()
        && let Some(at) = found
    {
        let end = at + needle.to_lowercase().len();
        if name.is_char_boundary(at) && name.is_char_boundary(end) {
            return [
                name[..at].to_owned(),
                name[at..end].to_owned(),
                name[end..].to_owned(),
            ];
        }
    }
    nothing()
}

/// The macOS footer under the matches: the keys that move among them, open
/// one, open one in a SQL editor, and clear the search.
fn search_footer(ui: &mut egui::Ui, keys: &Keys, skin: Skin<'_>) {
    let (look, palette, locale) = (skin.look, skin.palette, skin.locale);
    let words = ["move", "open", "open in SQL editor", "clear"].map(|word| gettext(locale, word));
    let hints: Vec<widgets::Hint<'_>> = vec![
        (&keys.search_move, &*words[0], true),
        (&keys.search_open, &*words[1], true),
        (&keys.search_editor, &*words[2], true),
        (&keys.search_clear, &*words[3], true),
    ];
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 20.0), Sense::hover());
    widgets::key_hints(
        ui,
        (rect.left() + INSET, rect.center().y),
        &hints,
        14.0,
        look,
        palette,
    );
}

/// The macOS status bar under the screen: that no table is open.
fn status_bar(ui: &mut egui::Ui, skin: Skin<'_>, searching: bool) {
    let (look, palette) = (skin.look, skin.palette);
    egui::Panel::bottom(egui::Id::new(("start-status", skin.tab.0)))
        .exact_size(STATUS_BAR)
        .resizable(false)
        .show_separator_line(false)
        .frame(
            Frame::new()
                .fill(palette.panel)
                .inner_margin(Margin::symmetric(20, 0)),
        )
        .show(ui, |ui| {
            let full = ui.max_rect().expand2(vec2(20.0, 0.0));
            widgets::hline(ui, full.x_range(), full.top() + 0.5, palette.outline);
            // The status bar's grey, as a table's footer has it.
            let status = palette.secondary.lerp_to_gamma(palette.dim, 0.5);
            let text = gettext(skin.locale, "No table open");
            let rect = ui.max_rect();
            let y = rect.center().y + 0.5;
            widgets::paint_label(
                ui,
                rect.left(),
                y,
                Text::one(look, widgets::secondary(look), &text, status),
            );
            // The right side invites a search, while nothing is typed.
            if !searching {
                let invite = gettext(skin.locale, "Start typing to search");
                widgets::paint_text_right(
                    ui,
                    rect.right(),
                    y,
                    Text::one(look, widgets::secondary(look), &invite, status),
                );
            }
        });
}

/// The terminal look's status line with no tab open: the mode, the
/// screen's keys (those that fit), and that no table is open.
pub fn status_line(app: &mut App, ui: &mut egui::Ui, tab: ConnTabId) {
    let (look, palette, locale) = (app.look, app.palette, app.locale);
    let keys = keys(app);
    // The search is on while its field has the keyboard, as `keys.rs` reads
    // it: the line then names the keys that move among the matches, open
    // one and go back, in step with those live keys.
    let searching = ui
        .ctx()
        .memory(|memory| memory.has_focus(search_field_id(tab)));
    // Otherwise the line names the ways to start, `/` for the search among
    // them.
    let words = [
        "open recent",
        "search",
        "new sql",
        "database",
        "edit connection",
        "tables",
        "all keys",
        "move",
        "open",
        "back",
    ]
    .map(|word| gettext(locale, word));
    let hints: Vec<widgets::Hint<'_>> = if searching {
        vec![
            (&keys.search_move, &*words[7], true),
            (&keys.search_open, &*words[8], true),
            (&keys.search_clear, &*words[9], true),
        ]
    } else {
        vec![
            (&keys.recent, &*words[0], true),
            (&keys.search_focus, &*words[1], true),
            (&keys.sql_here, &*words[2], true),
            (&keys.database_here, &*words[3], true),
            (&keys.edit_here, &*words[4], true),
            (&keys.sidebar, &*words[5], true),
            (&keys.help, &*words[6], true),
        ]
    };
    egui::Panel::bottom(egui::Id::new(("status-line", tab.0)))
        .exact_size(STATUS_LINE)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel))
        .show(ui, |ui| {
            let rect = ui.max_rect();
            widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
            let y = rect.top() + 1.0 + 15.0;
            let role = TextRole::OSecondary;
            let gap = 18.0;
            let mode = gettext(locale, "-- START --");
            let mode = Text::one(&look, TextRole::OModeLine, &mode, palette.dim);
            let left = rect.left() + 12.0;
            let x = left + widgets::paint_label(ui, left, y, mode) + gap;
            let right = rect.right() - 12.0;
            let said = gettext(locale, "no table open");
            let summary = || Text::one(&look, role, &said, palette.dim);
            let summary_width = widgets::measure(ui, summary());
            // Everything 18 apart. What does not fit before the summary
            // gives way from the end.
            let limit = right - summary_width - gap;
            let shown = (0..=hints.len())
                .rev()
                .find(|&count| {
                    x + widgets::key_hints_width(ui, &hints[..count], gap, &look, &palette) <= limit
                })
                .unwrap_or(0);
            widgets::key_hints(ui, (x, y), &hints[..shown], gap, &look, &palette);
            let width = widgets::paint_text_right(ui, right, y, summary());
            widgets::announce(
                ui,
                Rect::from_min_size(pos2(right - width, y - 8.0), vec2(width.max(1.0), 16.0)),
                &said,
            );
        });
}
