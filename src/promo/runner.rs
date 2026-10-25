//! Plays a scenario (`super::scenario`) through the harness, as a
//! person's input, and writes what it filmed to target/promo/<name>/: a
//! PNG for each change on screen (frames/) and events.json, what happened
//! in each and where. contrib/promo/make runs it:
//! `PROMO_SCENARIO=<file> cargo test --features shots --lib promo::runner::film -- --ignored --exact`.

#![allow(clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use egui::accesskit::{Role, TreeUpdate};
use serde::Serialize;

use super::live::Live;
use super::pick;
use super::scenario::{self, Database, Keys, Step};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::keymap;
use crate::model::{Action, Pane};
use crate::testing::{self, Harness};

/// The window, in points, and its pixels to a point.
const SIZE: egui::Vec2 = egui::vec2(1200.0, 750.0);
const SCALE: f32 = 2.0;

/// The macOS look is filmed in the light theme, as a Mac shows it by default.
const LIGHT: bool = true;

fn promo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target/promo")
}

/// What events.json holds: the scenario's words, and what its steps did.
#[derive(Serialize)]
struct Record {
    title: String,
    tempo: u32,
    size: [f32; 2],
    scale: f32,
    scenes: Vec<SceneRecord>,
    events: Vec<Event>,
}

#[derive(Serialize)]
struct SceneRecord {
    caption: Option<String>,
}

/// One thing that happened, the frame that shows it, and where.
#[derive(Serialize)]
struct Event {
    /// None for the frame the video opens on.
    scene: Option<usize>,
    frame: usize,
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    /// The keys as the keymap writes them, for a keycap.
    #[serde(skip_serializing_if = "Option::is_none")]
    keys: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    seconds: Option<f64>,
    /// x0, y0, x1, y1, in window points.
    #[serde(skip_serializing_if = "Option::is_none")]
    rect: Option<[f32; 4]>,
}

/// The harness, the database that answers it, and what has been filmed.
struct Film {
    harness: Harness,
    live: Live,
    dir: PathBuf,
    /// How many frames are written, and the last one's pixels.
    frames: usize,
    last: Vec<u8>,
    scene: Option<usize>,
    events: Vec<Event>,
    /// The step being played, for its errors.
    at: String,
}

impl Film {
    /// The window as a video opens on it: connected to `database`, its
    /// tree showing, nothing open, the row panel hidden. Set up directly,
    /// and none of it filmed. `file` is the scenario's, for its errors.
    fn start(file: &Path, database: &Path, name: &str, dir: PathBuf) -> Self {
        let at = format!("{}, the start", file.display());
        let mut harness = Harness::for_shots(SIZE, SCALE, LIGHT, crate::theme::Look::macos());
        // Measured on a macOS 26 window with a unified compact toolbar.
        harness.app.titlebar = crate::app::TitleBar {
            height: 38.0,
            inset: 80.0,
        };
        let mut live = Live::open(database)
            .unwrap_or_else(|error| panic!("{at}: {} does not open: {error}", database.display()));
        let saved = SavedConnection {
            id: ConnectionId::new(),
            name: name.into(),
            environment: Environment::Dev,
            read_only: Some(false),
            password: PasswordMode::None,
            ssh_secret: PasswordMode::None,
            spec: tabletist_db::ConnectSpec::sqlite(database),
            agents: Default::default(),
        };
        let conn = saved.id.clone();
        harness.app.connections.upsert(saved);
        let tab = harness.app.active_tab_id();
        harness.app.apply(Action::Connect { tab, conn });
        live.pump(&mut harness);
        assert_eq!(
            live.access(),
            tabletist_db::Access::ReadOnly,
            "{at}: the session is not read-only"
        );
        // The grid has the window's width.
        if harness
            .app
            .workspace(tab)
            .is_some_and(|workspace| workspace.row_panel)
        {
            harness.app.apply(Action::ToggleRowPanel(tab));
        }
        let mut film = Self {
            harness,
            live,
            dir,
            frames: 0,
            last: Vec::new(),
            scene: None,
            events: Vec::new(),
            at,
        };
        film.must_take();
        film.record("start");
        film
    }

    /// Answers what the app asked, draws, and writes the frame unless it
    /// is the last one again. Whether it was new.
    fn take(&mut self) -> bool {
        self.live.pump(&mut self.harness);
        self.harness.settle();
        self.live.pump(&mut self.harness);
        let image = self.harness.render();
        if *image.as_raw() == self.last {
            return false;
        }
        let path = self.dir.join(format!("frames/{:04}.png", self.frames));
        image.save(path).unwrap();
        self.last = image.into_raw();
        self.frames += 1;
        true
    }

    /// [`Self::take`] after a step that must change what is on screen.
    fn must_take(&mut self) {
        assert!(
            self.take(),
            "{}: the step changed nothing on screen",
            self.at
        );
    }

    /// An event of the last frame written.
    fn record(&mut self, kind: &'static str) -> &mut Event {
        self.events.push(Event {
            scene: self.scene,
            frame: self.frames - 1,
            kind,
            text: None,
            keys: None,
            seconds: None,
            rect: None,
        });
        self.events.last_mut().unwrap()
    }

    fn play(&mut self, step: &Step) {
        match step {
            Step::Open(object) => self.open(object),
            Step::Click(label) => self.click(label),
            Step::Command { keys, times, .. } => self.keys("command", keys, *times),
            Step::Press { keys, times } => self.keys("press", keys, *times),
            Step::Type(text) => {
                for character in text.chars() {
                    if character == '\n' {
                        self.harness.press(egui::Key::Enter, egui::Modifiers::NONE);
                    } else {
                        self.harness
                            .frame(vec![egui::Event::Text(character.into())]);
                    }
                    self.must_take();
                    let rect = self.focus_rect();
                    let event = self.record("type");
                    event.text = Some(character.into());
                    event.rect = rect;
                }
            }
            Step::Hold(seconds) => self.record("hold").seconds = Some(*seconds),
        }
    }

    /// The sidebar tree's item called `object`, clicked. Only the tree's
    /// items count: once a table is open its name is a tab and a heading
    /// too.
    fn open(&mut self, object: &str) {
        let tree = self.harness.settle();
        let rect = self
            .picked(object, pick::tree_items(&tree, object))
            .unwrap_or_else(|| {
                panic!(
                    "{}: the tree has no item called {object:?}; the labels are {:?}",
                    self.at,
                    testing::labels(&tree)
                )
            });
        self.press_at(rect);
    }

    /// What is called `label`, clicked: the one node with that name (of
    /// several, the one in a popup, a menu or a dialog: [`pick::choose`]),
    /// or else, where no node has it, the first painted text that reads it.
    fn click(&mut self, label: &str) {
        let tree = self.harness.settle();
        let rect = self
            .picked(label, pick::candidates(&tree, label))
            .or_else(|| self.harness.painted_rect(label))
            .unwrap_or_else(|| {
                panic!(
                    "{}: nothing on screen is called {label:?}; the labels are {:?}",
                    self.at,
                    testing::labels(&tree)
                )
            });
        self.press_at(rect);
    }

    /// The rect of the candidate [`pick::choose`] takes, None where there
    /// is none. Where several could be meant, the step stops and names
    /// each, for the scenario to say another label.
    fn picked(&self, label: &str, candidates: Vec<pick::Candidate>) -> Option<egui::Rect> {
        match pick::choose(candidates) {
            pick::Pick::Nothing => None,
            pick::Pick::One(rect) => Some(rect),
            pick::Pick::Ambiguous(candidates) => {
                let each: Vec<String> = candidates
                    .iter()
                    .map(|one| {
                        let place = if one.over { ", over the window" } else { "" };
                        format!("{:?} at {:?}{place}", one.role, corners(one.rect))
                    })
                    .collect();
                panic!(
                    "{}: {} nodes are called {label:?}, and none is the only one in a popup: {}",
                    self.at,
                    candidates.len(),
                    each.join("; ")
                )
            }
        }
    }

    /// The pointer moved to the middle of `rect`, then a press and a
    /// release there: a frame of the hover, one of the click.
    fn press_at(&mut self, rect: egui::Rect) {
        let pos = rect.center();
        self.harness.frame(vec![egui::Event::PointerMoved(pos)]);
        // A hover may change nothing on screen.
        self.take();
        self.record("point").rect = Some(corners(rect));
        let press = |pressed| egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::NONE,
        };
        self.harness.frame(vec![press(true)]);
        self.harness.frame(vec![press(false)]);
        self.must_take();
        self.record("click").rect = Some(corners(rect));
    }

    /// The chord, `times` times, as macOS reports what it holds.
    fn keys(&mut self, kind: &'static str, keys: &Keys, times: u32) {
        let mods = keys.mods;
        let modifiers = egui::Modifiers {
            alt: mods.alt,
            ctrl: mods.ctrl,
            shift: mods.shift,
            mac_cmd: mods.cmd,
            command: mods.cmd,
        };
        let written = keymap::key_written(scenario::LAYOUT, keys.spelled);
        for _ in 0..times {
            self.harness.press(keys.key, modifiers);
            self.must_take();
            let rect = self.focus_rect();
            let event = self.record(kind);
            event.keys = Some(written.as_str().to_owned());
            event.rect = rect;
        }
    }

    /// What the keyboard acts on: the cursor's cell where a grid has the
    /// keyboard, else the node that has the focus.
    fn focus_rect(&mut self) -> Option<[f32; 4]> {
        let tree = self.harness.settle();
        let focused = tree
            .nodes
            .iter()
            .find(|(id, _)| *id == tree.focus)
            .and_then(|(_, node)| node.bounds())
            .map(rect_of);
        self.cell(&tree, focused).or(focused).map(corners)
    }

    /// The cursor's cell of the table on screen: its row's node, across
    /// its column's header, while the grid has the keyboard: the focus
    /// holds that row, or nothing has the focus and the arrows go to the
    /// grid, the pane last worked in.
    fn cell(&self, tree: &TreeUpdate, focused: Option<egui::Rect>) -> Option<egui::Rect> {
        let app = &self.harness.app;
        let workspace = app.workspace(app.active_tab_id())?;
        let tab = workspace.object_tab(workspace.active_tab?)?;
        let cell = tab.selection?;
        let page = tab.rows.value.as_ref()?;
        let row = testing::bounds(tree, &format!("Row {}", cell.row + 1), Role::Button)?;
        let keyboard = match focused {
            Some(focused) => focused.contains_rect(row),
            None => workspace.pane == Pane::Grid,
        };
        if !keyboard {
            return None;
        }
        // A header is a button as wide as its column (its text may sit at
        // the right), above the row: not a tree item of the same name.
        let name = page.columns.get(cell.col)?.name.as_str();
        let header = tree
            .nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(name))
            .filter_map(|(_, node)| node.bounds().map(rect_of))
            .find(|header| {
                header.bottom() <= row.top() && row.x_range().contains(header.center().x)
            })?;
        Some(egui::Rect::from_x_y_ranges(header.x_range(), row.y_range()))
    }
}

fn rect_of(rect: egui::accesskit::Rect) -> egui::Rect {
    egui::Rect::from_min_max(
        egui::pos2(rect.x0 as f32, rect.y0 as f32),
        egui::pos2(rect.x1 as f32, rect.y1 as f32),
    )
}

fn corners(rect: egui::Rect) -> [f32; 4] {
    [rect.min.x, rect.min.y, rect.max.x, rect.max.y]
}

/// The SQLite file the scenario at `file` names, and the connection's
/// name.
fn database(file: &Path, database: &Database) -> (PathBuf, String) {
    match database {
        Database::Bookshop => {
            let path = promo().join("bookshop.sqlite");
            std::fs::create_dir_all(promo()).unwrap();
            // The fixture refuses a file that is there.
            let _ = std::fs::remove_file(&path);
            tabletist_db::fixtures::write_sqlite_bookshop(&path).unwrap();
            (path, "Bookshop".into())
        }
        Database::File(path) => {
            assert!(
                path.is_file(),
                "{}: the database {} is not a file",
                file.display(),
                path.display()
            );
            let name = path.file_stem().unwrap().to_string_lossy().into_owned();
            (path.clone(), name)
        }
    }
}

#[test]
#[ignore = "renders with wgpu; contrib/promo/make runs it"]
fn film() {
    let Ok(file) = std::env::var("PROMO_SCENARIO") else {
        println!("skipped: PROMO_SCENARIO names no scenario");
        return;
    };
    let file = PathBuf::from(file);
    let scenario = scenario::load(&file).unwrap_or_else(|error| panic!("{error}"));
    let name = file.file_stem().unwrap().to_string_lossy().into_owned();
    let dir = promo().join(&name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("frames")).unwrap();
    let (path, connection) = database(&file, &scenario.database);
    let mut film = Film::start(&file, &path, &connection, dir.clone());
    for (index, scene) in scenario.scenes.iter().enumerate() {
        film.scene = Some(index);
        for (number, step) in scene.steps.iter().enumerate() {
            film.at = format!(
                "{}, scene {}, step {}",
                file.display(),
                index + 1,
                number + 1
            );
            film.play(step);
        }
    }
    let record = Record {
        title: scenario.title.clone(),
        tempo: scenario.tempo,
        size: [SIZE.x, SIZE.y],
        scale: SCALE,
        scenes: scenario
            .scenes
            .iter()
            .map(|scene| SceneRecord {
                caption: scene.caption.clone(),
            })
            .collect(),
        events: std::mem::take(&mut film.events),
    };
    let json = serde_json::to_string_pretty(&record).unwrap();
    std::fs::write(dir.join("events.json"), json).unwrap();
    println!(
        "{} frames, {} events in {}",
        film.frames,
        record.events.len(),
        dir.display()
    );
}
