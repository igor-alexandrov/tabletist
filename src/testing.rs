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
    /// Asks the window to close in the next frame, as its close button or
    /// the desktop does. Taken by that frame.
    pub close_requested: bool,
    /// Every piece of text the last frame painted, with its color.
    pub painted: Vec<(String, egui::Color32)>,
    /// Where the last frame painted each piece of text, in points.
    pub text_rects: Vec<(String, egui::Rect)>,
    /// Every piece of text the last frame painted slanted.
    pub slanted: Vec<String>,
    /// Every piece of text the last frame painted struck through.
    pub struck: Vec<String>,
    /// Every rectangle, circle and outline the last frame filled, and where.
    pub fills: Vec<(egui::Rect, egui::Color32)>,
    /// The colour of every line and outline the last frame drew.
    pub strokes: Vec<egui::Color32>,
    /// Every rectangle the last frame outlined, and with what.
    pub outlines: Vec<(egui::Rect, egui::Stroke)>,
    /// How soon the last frame asked to be drawn again: at once is zero.
    pub repaint_after: std::time::Duration,
    /// How many of the `Lookup` commands sent so far were answered by
    /// [`Harness::answer_lookups`].
    pub lookups_answered: usize,
    /// Whether the window has the keyboard, which every frame is told:
    /// [`Harness::window_focus`] changes it.
    window_focused: bool,
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
            focused: self.window_focused,
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
        if std::mem::take(&mut self.close_requested) {
            // What `ViewportInfo::close_requested` reads.
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .events
                .push(egui::ViewportEvent::Close);
        }
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
        self.collect(&output);
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
            focused: self.window_focused,
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
        self.collect(&output);
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

    /// Keeps what a frame painted: its texts, its fills and its lines.
    fn collect(&mut self, output: &egui::FullOutput) {
        self.painted.clear();
        self.text_rects.clear();
        self.slanted.clear();
        self.struck.clear();
        self.fills.clear();
        self.strokes.clear();
        self.outlines.clear();
        for clipped in &output.shapes {
            collect_text(&clipped.shape, &mut self.painted, &mut self.text_rects);
            collect_slanted(&clipped.shape, &mut self.slanted);
            collect_struck(&clipped.shape, &mut self.struck);
            collect_paint(&clipped.shape, &mut self.fills, &mut self.strokes);
            collect_outlines(&clipped.shape, &mut self.outlines);
        }
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

    /// The window gives the keyboard to another app, or has it again, as
    /// the window says it: an event, and the state of every frame after.
    pub fn window_focus(&mut self, focused: bool) {
        self.window_focused = focused;
        self.frame(vec![egui::Event::WindowFocused(focused)]);
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

/// The texts of `shape` that are written slanted, every piece of them.
fn collect_slanted(shape: &egui::Shape, into: &mut Vec<String>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect_slanted(shape, into)),
        egui::Shape::Text(text) => {
            let sections = &text.galley.job.sections;
            if !sections.is_empty() && sections.iter().all(|section| section.format.italics) {
                into.push(text.galley.text().to_owned());
            }
        }
        _ => {}
    }
}

/// The pieces of `shape`'s texts that are written struck through: each
/// run of a text that is, without what stands beside it unstruck.
fn collect_struck(shape: &egui::Shape, into: &mut Vec<String>) {
    match shape {
        egui::Shape::Vec(shapes) => shapes.iter().for_each(|shape| collect_struck(shape, into)),
        egui::Shape::Text(text) => {
            let job = &text.galley.job;
            for section in &job.sections {
                if section.format.strikethrough != egui::Stroke::NONE
                    && let Some(piece) = job
                        .text
                        .get(section.byte_range.start.0..section.byte_range.end.0)
                {
                    into.push(piece.to_owned());
                }
            }
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

impl Harness {
    /// Sets the "Open read-only" box of `tab`'s saved connection and
    /// reconnects, as a user who changed the box and pressed Reconnect: the
    /// session comes back as the box says. The tree's requests are left
    /// unanswered.
    pub fn reconnect_fake_as(&mut self, tab: ConnTabId, read_only: bool) {
        let conn = self
            .app
            .workspace(tab)
            .expect("a workspace")
            .conn_id
            .clone();
        let mut saved = self.app.connections.get(&conn).expect("saved").clone();
        saved.read_only = Some(read_only);
        self.app.connections.upsert(saved);
        self.app.apply(Action::Reconnect(tab));
        let (session, request) = self
            .app
            .backend
            .sent
            .iter()
            .rev()
            .find_map(|command| match command {
                Command::Connect {
                    session, request, ..
                } => Some((*session, *request)),
                _ => None,
            })
            .expect("a Connect was sent");
        self.app.apply(Action::Backend(Event::Connected {
            session,
            request,
            driver: Driver::Sqlite,
            encrypted: false,
            access: asked_access(&self.app),
        }));
    }
}

use tabletist_db::{ColumnMeta, RowPage, Value, ValueKind};

/// What a view sends when the tab's open editor loses the keyboard: the
/// action names the editor it is about, by its cell and its place.
pub fn leave_edit(app: &App, tab: ConnTabId, id: TabId) -> Action {
    let editor = app
        .workspace(tab)
        .and_then(|workspace| workspace.object_tab(id))
        .and_then(|object| object.edits.editor.as_ref());
    let (cell, place) = editor.map_or(
        (
            crate::model::CellPos { row: 0, col: 0 },
            crate::edit::EditorPlace::Grid,
        ),
        |editor| (editor.cell, editor.place),
    );
    Action::LeaveEdit {
        tab,
        id,
        cell,
        place,
    }
}

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

/// The structure of the Bookshop's `book_covers`: the database numbers
/// `id`, `publisher_id` must be given, `kind` is one of three and has a
/// default, `image_data` may be NULL, and `created_at` has an expression
/// for a default.
pub fn book_covers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            ColumnInfo {
                identity: true,
                ..column("id", "INTEGER")
            },
            column("publisher_id", "INTEGER"),
            ColumnInfo {
                default: Some("'print'".into()),
                allowed_values: Some(vec!["print".into(), "ebook".into(), "audio".into()]),
                ..column("kind", "TEXT")
            },
            ColumnInfo {
                nullable: true,
                ..column("image_data", "JSON")
            },
            ColumnInfo {
                default: Some("CURRENT_TIMESTAMP".into()),
                ..column("created_at", "TEXT")
            },
        ],
        primary_key: vec!["id".into()],
        ..tabletist_db::Structure::default()
    }
}

/// A page of `book_covers`, `rows` long.
pub fn book_covers_page(rows: usize) -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("publisher_id", "INTEGER", ValueKind::Numeric),
            meta("kind", "TEXT", ValueKind::Text),
            meta("image_data", "JSON", ValueKind::Json),
            meta("created_at", "TEXT", ValueKind::Text),
        ],
        rows: (0..rows)
            .map(|index| {
                vec![
                    Value::Int(index as i64 + 1),
                    Value::Int(9_100_000_000_000_000_001 + (index as i64 % 3)),
                    Value::Text(if index % 2 == 0 { "print" } else { "ebook" }.into()),
                    Value::Null,
                    Value::Text("2026-01-12 09:14:03".into()),
                ]
            })
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// The Bookshop's publishers, as (id, name).
pub const PUBLISHERS: [(i64, &str); 3] = [
    (9_100_000_000_000_000_001, "Northlight Books"),
    (9_100_000_000_000_000_004, "Harbor Press"),
    (9_100_000_000_000_000_007, "Harbor Lights"),
];

/// The structure of the Bookshop's `publishers`.
pub fn publishers_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        ..ColumnInfo::default()
    };
    // The keys that point at the table: `book_covers` loses its covers
    // with a publisher, `books` is cleared of it, and `book_reviews`
    // holds it.
    let key = |table: &str, on_delete: &str| tabletist_db::ReferencingKey {
        name: Some(format!("{table}_publisher_id_fkey")),
        schema: "main".into(),
        table: table.into(),
        columns: vec!["publisher_id".into()],
        ref_columns: vec!["id".into()],
        on_delete: on_delete.into(),
    };
    tabletist_db::Structure {
        columns: vec![column("id", "INTEGER"), column("name", "TEXT")],
        primary_key: vec!["id".into()],
        referenced_by: vec![
            key("book_covers", "CASCADE"),
            key("books", "SET NULL"),
            key("book_reviews", "NO ACTION"),
        ],
        ..tabletist_db::Structure::default()
    }
}

/// A page of `publishers`: the three of [`PUBLISHERS`].
pub fn publishers_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: PUBLISHERS
            .iter()
            .map(|(id, name)| vec![Value::Int(*id), Value::Text((*name).into())])
            .collect(),
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// `book_covers` with its foreign key: `publisher_id` points at
/// `publishers.id`. The tests that type into `publisher_id` as a number
/// keep [`book_covers_structure`], which has no key.
pub fn keyed_covers_structure() -> tabletist_db::Structure {
    tabletist_db::Structure {
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["publisher_id".into()],
            ref_schema: "main".into(),
            ref_table: "publishers".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..book_covers_structure()
    }
}

/// `genres`: a table that points at itself. `parent_id` is a foreign key
/// to `genres.id`, and `id` is typed by hand: no database numbers it.
pub fn genres_structure() -> tabletist_db::Structure {
    use tabletist_db::ColumnInfo;
    let column = |name: &str, type_name: &str, nullable: bool| ColumnInfo {
        name: name.into(),
        type_name: type_name.into(),
        nullable,
        ..ColumnInfo::default()
    };
    tabletist_db::Structure {
        columns: vec![
            column("id", "INTEGER", false),
            column("parent_id", "INTEGER", true),
            column("name", "TEXT", false),
        ],
        primary_key: vec!["id".into()],
        foreign_keys: vec![tabletist_db::ForeignKeyInfo {
            columns: vec!["parent_id".into()],
            ref_schema: "main".into(),
            ref_table: "genres".into(),
            ref_columns: vec!["id".into()],
            ..tabletist_db::ForeignKeyInfo::default()
        }],
        ..tabletist_db::Structure::default()
    }
}

/// The two genres the tests hold: Fiction (1), and Crime (2) under it.
pub fn genres_page() -> RowPage {
    let meta = |name: &str, type_name: &str, kind: ValueKind| ColumnMeta {
        name: name.into(),
        type_name: type_name.into(),
        kind,
    };
    RowPage {
        columns: vec![
            meta("id", "INTEGER", ValueKind::Numeric),
            meta("parent_id", "INTEGER", ValueKind::Numeric),
            meta("name", "TEXT", ValueKind::Text),
        ],
        rows: vec![
            vec![Value::Int(1), Value::Null, Value::Text("Fiction".into())],
            vec![Value::Int(2), Value::Int(1), Value::Text("Crime".into())],
        ],
        has_more: false,
        ordered_by_key: true,
        elapsed: std::time::Duration::from_millis(2),
    }
}

/// The genres, as (id, name), for [`Harness::answer_lookups`].
pub const GENRES: [(i64, &str); 2] = [(1, "Fiction"), (2, "Crime")];

/// A page of `book_covers` whose three rows point at the three
/// publishers in turn: Northlight Books, Harbor Press, Harbor Lights.
/// [`book_covers_page`] numbers its publishers on from the first, and
/// two of those are no publisher.
pub fn keyed_covers_page() -> RowPage {
    let mut page = book_covers_page(3);
    for (row, (key, _)) in page.rows.iter_mut().zip(PUBLISHERS) {
        row[1] = Value::Int(key);
    }
    page
}

/// What a database holding `rows` as a table of (key, label) answers
/// `lookup`: by the rules of `Connection::lookup`, folding case as Rust's
/// `to_lowercase` does. It differs from the real statement in one thing:
/// it sorts every match before it applies the limit, where the database
/// limits first.
pub fn fake_lookup(
    lookup: &tabletist_db::Lookup,
    rows: &[(i64, &str)],
) -> tabletist_db::LookupPage {
    use tabletist_db::{LookupPart, LookupRow, Matched};
    let rank = |matched: Matched| match matched {
        Matched::Key => 0,
        Matched::Label => 1,
        Matched::KeyStart => 2,
    };
    let mut found: Vec<(Matched, i64, &str)> = rows
        .iter()
        .filter_map(|&(key, label)| {
            let written = key.to_string();
            let matched = match &lookup.part {
                LookupPart::Pinned(Value::Int(pinned)) => (*pinned == key).then_some(Matched::Key),
                LookupPart::Pinned(Value::Text(pinned)) => {
                    (**pinned == *written).then_some(Matched::Key)
                }
                LookupPart::Pinned(_) => None,
                LookupPart::Matching(text) if text.is_empty() => Some(Matched::KeyStart),
                LookupPart::Matching(text) if written == *text => Some(Matched::Key),
                LookupPart::Matching(text)
                    if lookup.label.is_some()
                        && label.to_lowercase().contains(&text.to_lowercase()) =>
                {
                    Some(Matched::Label)
                }
                LookupPart::Matching(text) => written
                    .starts_with(text.as_str())
                    .then_some(Matched::KeyStart),
            };
            Some((matched?, key, label))
        })
        .collect();
    found.sort_by_key(|&(matched, key, _)| (rank(matched), key));
    let limit = lookup.limit as usize;
    let more = found.len() > limit;
    found.truncate(limit);
    tabletist_db::LookupPage {
        rows: found
            .into_iter()
            .map(|(matched, key, label)| LookupRow {
                key: Value::Int(key),
                label: lookup.label.as_ref().map(|_| Value::Text(label.into())),
                matched,
            })
            .collect(),
        more,
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

    /// As [`Self::editable`], with the Bookshop's `book_covers` and three of
    /// its rows: a table a row can be added to.
    pub fn book_covers(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(book_covers_structure());
        self.answer_rows(book_covers_page(3));
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

impl Harness {
    /// As [`Self::book_covers`], with the key of `publisher_id` to
    /// `publishers`: a table whose `publisher_id` cells open the picker.
    pub fn keyed_covers(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(keyed_covers_structure());
        self.answer_rows(keyed_covers_page());
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }

    /// `genres` open, writable, its structure and its two rows loaded.
    pub fn genres(&mut self) -> (ConnTabId, TabId) {
        let tab = self.connect_fake_as(false);
        self.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "genres"),
            kind: ObjectKind::Table,
            pin: true,
        });
        self.answer_structure(genres_structure());
        self.answer_rows(genres_page());
        let id = self
            .app
            .workspace(tab)
            .and_then(|workspace| workspace.active_tab)
            .expect("the table's tab is open");
        (tab, id)
    }

    /// The `Lookup` commands sent so far, with their requests.
    pub fn lookups(&self) -> Vec<(SessionId, RequestId, tabletist_db::Lookup)> {
        self.app
            .backend
            .sent
            .iter()
            .filter_map(|command| match command {
                Command::Lookup {
                    session,
                    request,
                    lookup,
                } => Some((*session, *request, lookup.clone())),
                _ => None,
            })
            .collect()
    }

    /// Answers every lookup not answered yet, in the order they were
    /// sent, as a table holding `rows` would. One that was superseded is
    /// answered too: the app drops what it no longer waits for. Returns
    /// how many it answered.
    pub fn answer_lookups(&mut self, rows: &[(i64, &str)]) -> usize {
        let waiting: Vec<_> = self.lookups().split_off(self.lookups_answered);
        for (session, request, lookup) in &waiting {
            self.app.apply(Action::Backend(Event::Lookup {
                session: *session,
                request: *request,
                result: Ok(fake_lookup(lookup, rows)),
            }));
        }
        self.lookups_answered += waiting.len();
        waiting.len()
    }

    /// Answers the lookup at `index` of [`Self::lookups`] alone, as a table
    /// holding `rows` would, and leaves `lookups_answered` as it is: a test
    /// that uses it manages the order of the answers itself, and answers
    /// the rest by hand (or by [`Self::answer_lookups`], which then
    /// answers this one again, and the app drops it).
    pub fn answer_lookup_at(&mut self, index: usize, rows: &[(i64, &str)]) {
        let (session, request, lookup) = self.lookups()[index].clone();
        self.app.apply(Action::Backend(Event::Lookup {
            session,
            request,
            result: Ok(fake_lookup(&lookup, rows)),
        }));
    }

    /// Answers the newest lookup with `error`, and counts every lookup
    /// before it as answered.
    pub fn fail_lookup(&mut self, error: tabletist_db::Error) {
        let all = self.lookups();
        let (session, request, _) = all.last().expect("a Lookup was sent").clone();
        self.lookups_answered = all.len();
        self.app.apply(Action::Backend(Event::Lookup {
            session,
            request,
            result: Err(error),
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

/// A write refused in a read-only transaction, as PostgreSQL and MySQL say
/// it (SQLSTATE 25006).
pub fn refused_write() -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Error {
        error: tabletist_db::Error::Query {
            code: Some("25006".into()),
            message: "cannot execute UPDATE in a read-only transaction".into(),
            detail: None,
            hint: None,
            named: Box::default(),
        },
        position: None,
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
        ..Default::default()
    }
}

/// What a script sent to write did: [`script_outcome`], with how its
/// transaction ended.
pub fn write_outcome(
    outcomes: Vec<tabletist_db::StatementOutcome>,
    end: tabletist_db::ScriptEnd,
) -> tabletist_db::ScriptOutcome {
    tabletist_db::ScriptOutcome {
        end,
        ..script_outcome(outcomes)
    }
}

/// A statement that ran and gave no result set: how many rows it changed,
/// where it counts them.
pub fn done_outcome(affected: Option<u64>) -> tabletist_db::StatementOutcome {
    tabletist_db::StatementOutcome::Done {
        affected,
        warnings: 0,
    }
}

/// A script stopped before it began: cancelled while it was queued, or
/// while its transaction opened. No statement has a result.
pub fn stopped_before_it_began() -> tabletist_db::ScriptOutcome {
    tabletist_db::ScriptOutcome {
        results: Vec::new(),
        stopped: true,
        ..Default::default()
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
    let _ = sql.start_run(request, statements, tabletist_db::ScriptMode::ReadOnly);
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
        // A search is sent on the frame after the keystroke, not 150 ms
        // later: no test waits.
        app.lookup_delay = std::time::Duration::ZERO;
        app.attach(&ctx, false);
        Self {
            app,
            ctx,
            size,
            copied: None,
            scale: 1.0,
            viewport_commands: Vec::new(),
            fullscreen: false,
            close_requested: false,
            painted: Vec::new(),
            text_rects: Vec::new(),
            slanted: Vec::new(),
            struck: Vec::new(),
            fills: Vec::new(),
            strokes: Vec::new(),
            outlines: Vec::new(),
            repaint_after: std::time::Duration::MAX,
            lookups_answered: 0,
            window_focused: true,
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
