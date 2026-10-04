//! Headless UI test harness: runs frames without a window and inspects the
//! AccessKit tree, the same tree screen readers see.

use egui::accesskit::{self, NodeId, Role, TreeUpdate};

use crate::app::App;
use crate::paths::AppDirs;
use crate::settings::Settings;

pub struct Harness {
    pub app: App,
    pub ctx: egui::Context,
    pub size: egui::Vec2,
    /// The last text a frame put on the clipboard.
    pub copied: Option<String>,
    /// Physical pixels per point (2.0 is a Retina Mac).
    pub scale: f32,
    /// Window commands the last frame sent (move, zoom...).
    pub viewport_commands: Vec<egui::ViewportCommand>,
    /// Whether the window reports itself fullscreen.
    pub fullscreen: bool,
    /// Every piece of text the last frame painted, with its color.
    pub painted: Vec<(String, egui::Color32)>,
    /// Where the last frame painted each piece of text, in points.
    pub text_rects: Vec<(String, egui::Rect)>,
    /// Every rectangle, circle and outline the last frame filled, and where.
    pub fills: Vec<(egui::Rect, egui::Color32)>,
    /// The colour of every line and outline the last frame drew.
    pub strokes: Vec<egui::Color32>,
    /// Every rectangle the last frame outlined, and with what.
    pub outlines: Vec<(egui::Rect, egui::Stroke)>,
    /// How soon the last frame asked to be drawn again: at once is zero.
    pub repaint_after: std::time::Duration,
    #[cfg(feature = "shots")]
    renderer: Option<egui_kittest::wgpu::WgpuTestRenderer>,
    #[cfg(feature = "shots")]
    last: Option<egui::FullOutput>,
    _dir: tempfile::TempDir,
}

impl Default for Harness {
    fn default() -> Self {
        Self::new()
    }
}

impl Harness {
    pub fn new() -> Self {
        Self::with_size(egui::vec2(1280.0, 800.0))
    }

    pub fn with_size(size: egui::Vec2) -> Self {
        Self::with_backend(size, crate::backend::Backend::recording())
    }

    pub fn frame(&mut self, events: Vec<egui::Event>) -> TreeUpdate {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.scale);
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .fullscreen = Some(self.fullscreen);
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        #[cfg(feature = "shots")]
        {
            use egui_kittest::TestRenderer as _;
            if let Some(renderer) = &mut self.renderer {
                renderer.handle_delta(&mut output.textures_delta);
            }
        }
        output.textures_delta.clear();
        self.painted.clear();
        self.text_rects.clear();
        self.fills.clear();
        self.strokes.clear();
        self.outlines.clear();
        for clipped in &output.shapes {
            collect_text(&clipped.shape, &mut self.painted, &mut self.text_rects);
            collect_paint(&clipped.shape, &mut self.fills, &mut self.strokes);
            collect_outlines(&clipped.shape, &mut self.outlines);
        }
        let viewport = output.viewport_output.get(&egui::ViewportId::ROOT);
        self.viewport_commands = viewport
            .map(|viewport| viewport.commands.clone())
            .unwrap_or_default();
        self.repaint_after =
            viewport.map_or(std::time::Duration::MAX, |viewport| viewport.repaint_delay);
        #[cfg(feature = "shots")]
        {
            self.last = Some(output.clone());
        }
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied = Some(text.clone());
            }
        }
        output
            .platform_output
            .accesskit_update
            .expect("AccessKit is enabled")
    }

    /// Runs one frame that draws only `add` in a central panel, for
    /// testing a widget on its own.
    pub fn frame_with(&mut self, add: impl FnOnce(&mut egui::Ui)) -> TreeUpdate {
        self.frame_with_events(Vec::new(), add)
    }

    /// [`Self::frame_with`], given `events`: for a widget on its own that
    /// answers the pointer or the keyboard.
    pub fn frame_with_events(
        &mut self,
        events: Vec<egui::Event>,
        add: impl FnOnce(&mut egui::Ui),
    ) -> TreeUpdate {
        let mut input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            events,
            ..Default::default()
        };
        input
            .viewports
            .entry(egui::ViewportId::ROOT)
            .or_default()
            .native_pixels_per_point = Some(self.scale);
        let mut add = Some(add);
        let mut output = self.ctx.run_ui(input, |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                if let Some(add) = add.take() {
                    add(ui);
                }
            });
        });
        output.textures_delta.clear();
        output
            .platform_output
            .accesskit_update
            .expect("AccessKit is enabled")
    }

    pub fn copy(&mut self, shift: bool) {
        self.settle();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, self.size)),
            // egui 0.36 tracks modifiers through events, not a RawInput field.
            events: vec![
                egui::Event::ModifiersChanged(egui::Modifiers {
                    shift,
                    ..egui::Modifiers::COMMAND
                }),
                egui::Event::Copy,
            ],
            ..Default::default()
        };
        let app = &mut self.app;
        let mut output = self.ctx.run_ui(input, |ui| app.frame_ui(ui));
        output.textures_delta.clear();
        for command in &output.platform_output.commands {
            if let egui::OutputCommand::CopyText(text) = command {
                self.copied = Some(text.clone());
            }
        }
        self.frame(vec![egui::Event::ModifiersChanged(egui::Modifiers::NONE)]);
    }

    /// Two frames: egui lays out on the first and settles sizes on the second.
    pub fn settle(&mut self) -> TreeUpdate {
        self.frame(Vec::new());
        self.frame(Vec::new())
    }

    /// Runs frames until fades are over (a dialog opening), so painted
    /// colours are the final ones. Each frame is 1/60 s; the longest
    /// animation is 0.12 s.
    pub fn finish_animations(&mut self) -> TreeUpdate {
        for _ in 0..20 {
            self.frame(Vec::new());
        }
        self.settle()
    }

    /// Clicks the widget labelled `label` through AccessKit: a button if
    /// there is one, else any node with that label (selectable labels and
    /// toggles get other roles).
    pub fn click(&mut self, label: &str) {
        let tree = self.settle();
        let target = node(&tree, label, Role::Button)
            .or_else(|| {
                tree.nodes
                    .iter()
                    .find(|(_, node)| node.label() == Some(label))
                    .map(|(id, _)| *id)
            })
            .unwrap_or_else(|| panic!("nothing labelled {label:?}: {:?}", labels(&tree)));
        self.frame(vec![egui::Event::AccessKitActionRequest(
            accesskit::ActionRequest {
                target_tree: accesskit::TreeId::ROOT,
                target_node: target,
                action: accesskit::Action::Click,
                data: None,
            },
        )]);
        self.settle();
    }

    /// Presses and releases a key: one frame with the key down, one with it
    /// up, so a second `press` of the same key is a new press, not a repeat
    /// of a held one.
    pub fn press(&mut self, key_: egui::Key, modifiers: egui::Modifiers) {
        self.settle();
        self.frame(vec![key(key_, modifiers)]);
        self.frame(vec![release(key_, modifiers)]);
        self.settle();
    }

    /// The color the last frame painted `text` in.
    pub fn painted_color(&self, text: &str) -> Option<egui::Color32> {
        self.painted
            .iter()
            .find(|(painted, _)| painted == text)
            .map(|(_, color)| *color)
    }

    /// Where the last frame painted `text`.
    pub fn painted_rect(&self, text: &str) -> Option<egui::Rect> {
        self.text_rects
            .iter()
            .find(|(painted, _)| painted == text)
            .map(|(_, rect)| *rect)
    }

    pub fn has(&mut self, label: &str) -> bool {
        let tree = self.settle();
        labels(&tree).iter().any(|found| found == label)
    }
}

fn collect_text(
    shape: &egui::Shape,
    into: &mut Vec<(String, egui::Color32)>,
    rects: &mut Vec<(String, egui::Rect)>,
) {
    match shape {
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .for_each(|shape| collect_text(shape, into, rects)),
        egui::Shape::Text(text) => {
            let color = text.override_text_color.unwrap_or_else(|| {
                let section = text.galley.job.sections.first();
                match section.map(|section| section.format.color) {
                    Some(color) if color != egui::Color32::PLACEHOLDER => color,
                    _ => text.fallback_color,
                }
            });
            into.push((text.galley.text().to_owned(), color));
            rects.push((
                text.galley.text().to_owned(),
                egui::Rect::from_min_size(text.pos, text.galley.size()),
            ));
        }
        _ => {}
    }
}

fn collect_outlines(shape: &egui::Shape, into: &mut Vec<(egui::Rect, egui::Stroke)>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .for_each(|shape| collect_outlines(shape, into)),
        egui::Shape::Rect(rect) if rect.stroke.width > 0.0 && rect.stroke.color.a() > 0 => {
            into.push((rect.rect, rect.stroke));
        }
        _ => {}
    }
}

fn collect_paint(
    shape: &egui::Shape,
    fills: &mut Vec<(egui::Rect, egui::Color32)>,
    strokes: &mut Vec<egui::Color32>,
) {
    let mut stroke = |stroke: egui::Stroke| {
        if stroke.width > 0.0 && stroke.color.a() > 0 {
            strokes.push(stroke.color);
        }
    };
    match shape {
        egui::Shape::Vec(shapes) => shapes
            .iter()
            .for_each(|shape| collect_paint(shape, fills, strokes)),
        egui::Shape::Rect(rect) => {
            if rect.fill.a() > 0 {
                fills.push((rect.rect, rect.fill));
            }
            stroke(rect.stroke);
        }
        egui::Shape::Circle(circle) => {
            if circle.fill.a() > 0 {
                let size = egui::Vec2::splat(2.0 * circle.radius);
                fills.push((
                    egui::Rect::from_center_size(circle.center, size),
                    circle.fill,
                ));
            }
            stroke(circle.stroke);
        }
        egui::Shape::LineSegment { stroke: line, .. } => stroke(*line),
        // A filled outline (the connection dialog's stripe), by the
        // rectangle round it.
        egui::Shape::Path(path) if path.fill.a() > 0 => {
            let bounds = egui::Rect::from_points(&path.points);
            fills.push((bounds, path.fill));
        }
        _ => {}
    }
}

/// The node named `label` with `role`. Plain labels carry their text as the
/// value (see [`labels`]), so that counts as the name too.
pub fn node(tree: &TreeUpdate, label: &str, role: Role) -> Option<NodeId> {
    tree.nodes
        .iter()
        .find(|(_, node)| {
            node.label().or_else(|| node.value()) == Some(label) && node.role() == role
        })
        .map(|(id, _)| *id)
}

/// Where the node labelled `label` with `role` sits, in points.
pub fn bounds(tree: &TreeUpdate, label: &str, role: Role) -> Option<egui::Rect> {
    let id = node(tree, label, role)?;
    let (_, found) = tree.nodes.iter().find(|(node, _)| *node == id)?;
    let rect = found.bounds()?;
    Some(egui::Rect::from_min_max(
        egui::pos2(rect.x0 as f32, rect.y0 as f32),
        egui::pos2(rect.x1 as f32, rect.y1 as f32),
    ))
}

/// Every node's accessible name. egui gives plain labels (role `Label`)
/// their text as the node's value, not its label, so read both.
pub fn labels(tree: &TreeUpdate) -> Vec<String> {
    tree.nodes
        .iter()
        .filter_map(|(_, node)| node.label().or_else(|| node.value()).map(str::to_owned))
        .collect()
}

pub fn key(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers,
    }
}

/// The key coming back up.
pub fn release(key: egui::Key, modifiers: egui::Modifiers) -> egui::Event {
    egui::Event::Key {
        key,
        physical_key: None,
        pressed: false,
        repeat: false,
        modifiers,
    }
}

use crate::backend::{Command, Event, RequestId, SessionId};
use crate::connections::{ConnectionId, SavedConnection};
use crate::model::{Action, ConnTabId, TabId, Workspace};
use tabletist_db::{ConnectSpec, Driver, ObjectInfo, ObjectKind};

pub fn last_sent(app: &App) -> &Command {
    app.backend.sent.last().expect("a command was sent")
}

impl Harness {
    /// Puts an empty SQL editor at the end of `tab`'s strip without showing
    /// it, numbered like the ones the app opens.
    pub fn add_sql_tab(&mut self, tab: ConnTabId) -> TabId {
        let id = TabId(self.app.next_id());
        let workspace = self.app.workspace_mut(tab).expect("a workspace");
        workspace.push_sql_tab(id, 1_000, Some(std::time::Duration::from_secs(30)));
        id
    }
}

/// The saved connection the fake session stands for.
fn fixture_connection() -> SavedConnection {
    SavedConnection {
        id: ConnectionId::new(),
        name: "Fixture".into(),
        environment: crate::env::Environment::Dev,
        // Read-only, as every connection was when most tests were written.
        // A test of a writable connection says so itself.
        read_only: Some(true),
        password: crate::connections::PasswordMode::None,
        ssh_secret: crate::connections::PasswordMode::None,
        spec: ConnectSpec::sqlite("/tmp/fixture.db"),
    }
}

/// What the newest Connect asked its session to be opened as. A faked
/// `Connected` says so, as the backend says what the session it opened is.
pub fn asked_access(app: &App) -> tabletist_db::Access {
    let sent = app.backend.sent.iter().rev();
    sent.filter_map(|command| match command {
        Command::Connect { access, .. } => Some(*access),
        _ => None,
    })
    .next()
    .expect("a Connect was sent")
}

/// A workspace for the fixture connection, still connecting and with no
/// tabs, for tests that need one without an app.
pub fn workspace() -> Workspace {
    Workspace::new(
        SessionId(1),
        RequestId(2),
        fixture_connection(),
        tabletist_db::Secrets::default(),
    )
}

impl Harness {
    /// Connects the active tab through the recording backend and answers the
    /// tree's first requests: schema `main` with `users`, `orders` and the
    /// view `active_users`.
    pub fn connect_fake(&mut self) -> ConnTabId {
        self.connect_fake_as(true)
    }

    /// [`Self::connect_fake`], with the saved connection's "Open read-only"
    /// box as given. The session's access is read from that box at every
    /// connect, so a writable fixture comes back writable from a reconnect.
    pub fn connect_fake_as(&mut self, read_only: bool) -> ConnTabId {
        let mut saved = fixture_connection();
        saved.read_only = Some(read_only);
        let conn = saved.id.clone();
        self.app.connections.upsert(saved);
        let tab = self.app.active_tab_id();
        self.app.apply(Action::Connect { tab, conn });
        let Command::Connect {
            session, request, ..
        } = *last_sent(&self.app)
        else {
            panic!("expected Connect");
        };
        self.app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
            access: crate::testing::asked_access(&self.app),
        }));
        let Command::ListSchemas { request, .. } = *last_sent(&self.app) else {
            panic!("expected ListSchemas");
        };
        self.app.apply(Action::Backend(Event::Schemas {
            session,
            request,
            result: Ok(vec!["main".into()]),
        }));
        let Command::ListObjects { request, .. } = *last_sent(&self.app) else {
            panic!("expected ListObjects for the default schema");
        };
        let objects = vec![
            ObjectInfo {
                name: "active_users".into(),
                kind: ObjectKind::View,
                estimated_rows: None,
            },
            ObjectInfo {
                name: "orders".into(),
                kind: ObjectKind::Table,
                estimated_rows: Some(3),
            },
            ObjectInfo {
                name: "users".into(),
                kind: ObjectKind::Table,
                estimated_rows: Some(1_200_000),
            },
        ];
        self.app.apply(Action::Backend(Event::Objects {
            session,
            request,
            schema: "main".into(),
            result: Ok(objects),
        }));
        tab
    }
}

use tabletist_db::{ColumnMeta, RowPage, Value, ValueKind};

/// A page shaped like the fixture's users table.
pub fn page(rows: usize, has_more: bool) -> RowPage {
    RowPage {
        columns: vec![
            ColumnMeta {
                name: "id".into(),
                type_name: "INTEGER".into(),
                kind: ValueKind::Numeric,
            },
            ColumnMeta {
                name: "email".into(),
                type_name: "TEXT".into(),
                kind: ValueKind::Text,
            },
            ColumnMeta {
                name: "meta".into(),
                type_name: "JSON".into(),
                kind: ValueKind::Json,
            },
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Text(format!("user{}@example.com", index + 1).into()),
                    if index == 0 {
                        Value::Text(r#"{"plan":"pro"}"#.into())
                    } else {
                        Value::Null
                    },
                ]
            })
            .collect(),
        has_more,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(12),
    }
}

/// The structure of the table `page` holds: `id` is the primary key,
/// `email` is NOT NULL, `meta` is JSON and may be NULL.
pub fn fixture_structure() -> tabletist_db::Structure {
    let column = |name: &str, type_name: &str, nullable: bool| tabletist_db::ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..tabletist_db::ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("email", "TEXT", false),
            column("meta", "JSON", true),
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

impl Harness {
    /// A writable connection with `users` open and pinned, its structure
    /// and a page of five rows loaded: a table whose cells can be edited.
    /// The saved connection itself is writable, so a reconnect comes back
    /// writable too.
    pub fn editable(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "users"),
            kind: ObjectKind::Table,
            pin: true,
        });
        // The describe was sent before the rows were asked for.
        self.answer_structure(fixture_structure());
        self.answer_rows(page(5, false));
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }

    /// Answers the newest FetchRows command with `page`.
    pub fn answer_rows(&mut self, page: RowPage) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::FetchRows {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a FetchRows was sent");
        self.app.apply(Action::Backend(Event::Rows {
            session,
            request,
            result: Ok(page),
        }));
    }

    /// Answers the latest `Describe` with `structure`.
    pub fn answer_structure(&mut self, structure: tabletist_db::Structure) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Describe {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a Describe was sent");
        self.app.apply(Action::Backend(Event::Structure {
            session,
            request,
            result: Ok(structure),
        }));
    }

    /// Answers the newest `Write` sent.
    pub fn answer_written(
        &mut self,
        result: Result<tabletist_db::WriteOutcome, tabletist_db::Error>,
    ) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Write {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a Write was sent");
        self.app.apply(Action::Backend(Event::Written {
            session,
            request,
            result,
        }));
    }
}

/// A SQL editor statement's result, shaped like the fixture's users table.
pub fn rows_outcome(rows: usize) -> tabletist_db::StatementOutcome {
    let page = page(rows, false);
    tabletist_db::StatementOutcome::Rows {
        columns: page.columns,
        rows: page.rows,
        truncated: false,
    }
}

/// A SQL editor statement that failed with `message`, at the 1-based
/// character `position` of its text when the database gives one.
pub fn error_outcome(message: &str, position: Option<usize>) -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Error {
        error: tabletist_db::Error::query(message),
        position,
    }
}

/// What a script did, as a driver reports it: one outcome per statement
/// that started, each taking 14 ms. A `Cancelled` outcome is a stopped run,
/// as it is for every driver.
pub fn script_outcome(
    outcomes: Vec<tabletist_db::StatementOutcome>,
) -> tabletist_db::ScriptOutcome {
    let stopped = outcomes.contains(&tabletist_db::StatementOutcome::Cancelled);
    tabletist_db::ScriptOutcome {
        results: outcomes
            .into_iter()
            .map(|outcome| tabletist_db::StatementResult {
                elapsed: std::time::Duration::from_millis(14),
                outcome,
            })
            .collect(),
        stopped,
    }
}

/// A script stopped before it began: cancelled while it was queued, or
/// while its transaction opened. No statement has a result.
pub fn stopped_before_it_began() -> tabletist_db::ScriptOutcome {
    tabletist_db::ScriptOutcome {
        results: Vec::new(),
        stopped: true,
    }
}

/// Types `text` into `sql`, runs all of it (split as SQLite does) and
/// finishes the run with `outcomes`, without an app.
pub fn run_script(
    sql: &mut crate::model::SqlTab,
    text: &str,
    outcomes: Vec<tabletist_db::StatementOutcome>,
) {
    sql.text = text.into();
    let request = RequestId(sql.run.loaded.map_or(9, |last| last.0 + 1));
    let statements = tabletist_db::sql::statements(Driver::Sqlite.dialect(), text);
    let _ = sql.start_run(request, statements);
    assert!(sql.finish_run(request, Ok(script_outcome(outcomes)), None));
}

impl Harness {
    /// Answers the newest `RunSql`, as the backend does when the script
    /// ends: with what it did (or why it failed as a whole) and who stopped
    /// it, if someone did.
    pub fn answer_sql(
        &mut self,
        result: Result<tabletist_db::ScriptOutcome, tabletist_db::Error>,
        cancel: Option<crate::backend::CancelReason>,
    ) {
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::RunSql {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a RunSql was sent");
        self.app.apply(Action::Backend(Event::SqlRan {
            session,
            request,
            result,
            cancel,
        }));
    }
}

impl Harness {
    pub fn with_backend(size: egui::Vec2, backend: crate::backend::Backend) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let ctx = egui::Context::default();
        ctx.enable_accesskit();
        let mut app = App::new(AppDirs::at(dir.path()), Settings::default().into(), backend);
        // Tests stay deterministic on every OS.
        app.look = crate::theme::Look::standard();
        app.attach(&ctx, false);
        Self {
            app,
            ctx,
            size,
            copied: None,
            scale: 1.0,
            viewport_commands: Vec::new(),
            fullscreen: false,
            painted: Vec::new(),
            text_rects: Vec::new(),
            fills: Vec::new(),
            strokes: Vec::new(),
            outlines: Vec::new(),
            repaint_after: std::time::Duration::MAX,
            #[cfg(feature = "shots")]
            renderer: None,
            #[cfg(feature = "shots")]
            last: None,
            _dir: dir,
        }
    }

    /// Runs frames until `done` holds or `timeout` passes. Returns whether it held.
    pub fn run_until(&mut self, timeout: std::time::Duration, done: impl Fn(&App) -> bool) -> bool {
        let deadline = std::time::Instant::now() + timeout;
        while std::time::Instant::now() < deadline {
            self.frame(Vec::new());
            if done(&self.app) {
                self.settle();
                return true;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        false
    }

    /// Draws with `look` from the next frame on.
    pub fn set_look(&mut self, look: crate::theme::Look) {
        self.app.look = look;
        crate::theme::install(&self.ctx, false, &look);
        crate::theme::apply(&self.ctx, &self.app.palette, &look);
    }
}

#[cfg(feature = "shots")]
impl Harness {
    /// A harness that can render screenshots: `size` in points at `scale`
    /// pixels per point, in the light or dark theme and drawn with `look`.
    pub fn for_shots(size: egui::Vec2, scale: f32, light: bool, look: crate::theme::Look) -> Self {
        let mut harness = Self::with_size(size);
        harness.scale = scale;
        harness.renderer = Some(egui_kittest::wgpu::WgpuTestRenderer::new());
        harness.app.palette = if light {
            crate::theme::Palette::light()
        } else {
            crate::theme::Palette::dark()
        };
        harness.set_look(look);
        harness.settle();
        harness
    }

    /// Settles, renders the last frame, and writes it to `path` as PNG.
    pub fn shot(&mut self, path: &std::path::Path) {
        use egui_kittest::TestRenderer as _;
        self.settle();
        self.settle();
        let output = self.last.take().expect("a frame ran");
        let image = self
            .renderer
            .as_mut()
            .expect("made with for_shots")
            .render(&self.ctx, &output)
            .expect("rendered");
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("shots directory");
        }
        image.save(path).expect("PNG written");
    }
}

#[cfg(test)]
mod tests {
    use super::Harness;

    #[test]
    fn a_pressed_key_is_released() {
        let mut harness = Harness::new();
        harness.press(egui::Key::J, egui::Modifiers::NONE);
        // egui reads a press of a key that is still down as a repeat.
        assert!(
            !harness.ctx.input(|input| input.key_down(egui::Key::J)),
            "a key left down makes its next press a repeat"
        );
    }
}
