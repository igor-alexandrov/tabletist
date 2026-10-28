# Promotional videos from scenarios Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `contrib/promo/make <scenario.toml>` turns a scenario (captions
and steps, no times) into a 1920x1080, 60 fps MP4 with synthesized music
and sound effects, and with camera, pointer and captions that move as a
produced video's do.

**Architecture:** There are three pieces, each with one job.
1. A Rust runner (`src/promo/`, an ignored test behind `shots`) plays the
   scenario as input through the headless harness, against a read-only
   SQLite session. It writes a PNG per change on screen and `events.json`,
   what happened and where.
2. A Node director (`contrib/promo/director.js`) turns those events into a
   timeline. It is the only place a time is decided.
3. A composer (`index.html`, `audio.js`) draws and sounds any moment of
   that timeline in headless Chrome. `render.cjs` hands the frames and the
   WAV to ffmpeg.

The design is
`docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md`.

**Tech stack:**
- Rust 2024 with the egui harness (`src/testing.rs`), `toml` and `serde`
  (already dependencies), and `tabletist_db::fixtures`.
- Node 22 (`node:test`), with `puppeteer-core` installed into
  `target/promo/` by `make`, so there is no `package.json`.
- Web Audio `OfflineAudioContext`, ffmpeg (libx264, aac, loudnorm) and
  Google Chrome.

No new crate. One branch, one commit per task.

**Before you start:**
- Read `AGENTS.md`.
- Never use em dashes, in code or prose.
- Commits are signed through 1Password. If signing fails, stage the
  change and stop: never commit unsigned.
- Renders need a GPU and Chrome, so they run only on the Mac, by hand.
- CI runs the Rust checks without `shots`. The scenario's parsing and its
  tests must compile and pass there too.

## Files

| File | Job |
|---|---|
| `src/lib.rs` | `#[cfg(test)] mod promo;` |
| `src/testing.rs` | `Harness::render`, which returns the frame's image. `shot` uses it. |
| `src/promo.rs` | The module's doc, and its three modules. |
| `src/promo/scenario.rs` | A scenario file read and checked. No GPU, so it runs in the normal suite. |
| `src/promo/live.rs` | The backend answered by a real SQLite session (`shots`). Ported from `claude/promo-video:src/promo.rs`. |
| `src/promo/runner.rs` | The ignored test `film`: the setup, the steps, frames and `events.json` (`shots`). |
| `contrib/promo/make` | The one command. |
| `contrib/promo/README.md` | The scenario format, for whoever writes one. |
| `contrib/promo/director.js` | Events to timeline. Pure, and usable from Node and the page. |
| `contrib/promo/director.test.js` | `node --test` tests for the director. |
| `contrib/promo/index.html` | The picture at any time `t`. |
| `contrib/promo/audio.js` | The sound of a timeline, as a WAV in base64. |
| `contrib/promo/render.cjs` | Chrome, the frames, ffmpeg. |
| `contrib/promo/scenarios/smoke.toml` | A few seconds, for checking the pipeline. |
| `contrib/promo/scenarios/overview.toml` | The old branch's video, on the Bookshop. |
| `contrib/promo/scenarios/filtering.toml` | One feature explained. |

### `events.json`, the runner's output and the director's input

```json
{
  "title": "Filtering", "tempo": 112, "size": [1200, 750], "scale": 2,
  "scenes": [{ "caption": "Open a table. It's just there." }],
  "events": [
    { "scene": null, "frame": 0, "kind": "start" },
    { "scene": 0, "frame": 1, "kind": "point", "rect": [20, 180, 140, 200] },
    { "scene": 0, "frame": 2, "kind": "click", "rect": [20, 180, 140, 200] },
    { "scene": 0, "frame": 3, "kind": "command", "keys": "⌘F", "rect": [0, 80, 1200, 700] },
    { "scene": 0, "frame": 4, "kind": "type", "text": "t", "rect": [300, 96, 520, 120] },
    { "scene": 0, "frame": 4, "kind": "hold", "seconds": 1.0 }
  ]
}
```

Its kinds:
- `start`: the frame the video opens on.
- `point`: the pointer has arrived, and the frame shows the hover.
- `click`, `command`, `press`, `type` and `hold`: one per step, and one
  per repetition of a `command` or a `press`.

`frame` is the PNG that shows the event (`frames/NNNN.png`). `rect` is in
window points. Captions sit in `scenes`, not in `events`. The spec's
`caption` kind became this array, and Task 10 says so in the spec.

---

## Phase 1: the scenario and the runner

### Task 1: `Harness::render`

The runner compares a frame's pixels with the one before it, so it needs
the image, not only a file.

**Files:**
- Modify: `src/testing.rs` (the `#[cfg(feature = "shots")] impl Harness` block, `shot`)

- [ ] **Step 1: Split `shot`**

Replace `shot` with:

```rust
    /// Settles and renders the last frame.
    pub fn render(&mut self) -> image::RgbaImage {
        use egui_kittest::TestRenderer as _;
        self.settle();
        self.settle();
        let output = self.last.take().expect("a frame ran");
        self.renderer
            .as_mut()
            .expect("made with for_shots")
            .render(&self.ctx, &output)
            .expect("rendered")
    }

    /// Settles, renders the last frame, and writes it to `path` as PNG.
    pub fn shot(&mut self, path: &std::path::Path) {
        let image = self.render();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("shots directory");
        }
        image.save(path).expect("PNG written");
    }
```

If the compiler says `render` returns another type than
`image::RgbaImage`, use the type it names: it is what `shot` already
saved.

- [ ] **Step 2: Check that it compiles with and without `shots`**

Run: `cargo test --locked --features shots --lib --no-run && cargo test --locked --lib --no-run`
Expected: both finish with no error and no warning.

- [ ] **Step 3: Check that the screenshots still render**

Run: `cargo test --locked --features shots --lib shots -- --ignored 2>&1 | tail -3`
Expected: `test result: ok`, and the PNGs in `target/shots/` are fresh (`ls -lt target/shots | head`).

- [ ] **Step 4: Commit**

```bash
git add src/testing.rs
git commit -m "The harness renders a frame to an image, not only to a file

A promotional video's runner compares each frame with the one before it.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 2: the scenario

**Files:**
- Modify: `src/lib.rs` (beside `mod shots;`)
- Create: `src/promo.rs`
- Create: `src/promo/scenario.rs`

- [ ] **Step 1: The module, and the tests first**

In `src/lib.rs`, next to the `shots` declaration:

```rust
#[cfg(test)]
mod promo;
```

`src/promo.rs`:

```rust
//! Promotional videos, made from scenarios (`contrib/promo/scenarios/`):
//! `contrib/promo/make <scenario>` runs [`runner`] for the app's frames and
//! what happened in each, then `contrib/promo/render.cjs` for the video.
//! The design: docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md.

mod scenario;

#[cfg(feature = "shots")]
mod live;
#[cfg(feature = "shots")]
mod runner;
```

Comment out the two `shots` modules until Tasks 3 and 4 create them.

`src/promo/scenario.rs`, the tests with the API stubbed:

```rust
//! A scenario, as its file says it: what a promotional video shows and
//! what it says. It has no times: `contrib/promo/director.js` decides
//! them. The format: contrib/promo/README.md.

use std::path::{Path, PathBuf};

use crate::keymap::{Command, Layout, Mods};

/// The look a video is drawn in: the macOS one, the only one so far.
pub const LAYOUT: Layout = Layout::Mac;

#[derive(Debug)]
pub struct Scenario {
    /// What the end card says.
    pub title: String,
    pub database: Database,
    /// The music's beats per minute; a scene ends on a bar.
    pub tempo: u32,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, PartialEq)]
pub enum Database {
    /// The Bookshop fixture, written afresh for each run.
    Bookshop,
    /// A SQLite file, opened read-only.
    File(PathBuf),
}

#[derive(Debug)]
pub struct Scene {
    pub caption: Option<String>,
    pub steps: Vec<Step>,
}

/// One press of a key with what is held, as the keymap spells it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Keys {
    pub spelled: &'static str,
    pub key: egui::Key,
    pub mods: Mods,
}

#[derive(Debug, PartialEq)]
pub enum Step {
    /// The object in the tree, clicked.
    Open(String),
    /// The node with this label, or else this painted text, clicked.
    Click(String),
    /// The command's key, `times` times, with its keycap.
    Command {
        command: Command,
        keys: Keys,
        times: u32,
    },
    /// A key the field has to itself (Enter in a filter's value).
    Press { keys: Keys, times: u32 },
    /// Typed a character at a time.
    Type(String),
    /// The picture stays this many seconds more.
    Hold(f64),
}

pub fn load(_path: &Path) -> Result<Scenario, String> {
    Err("not yet".into())
}

pub fn parse(_text: &str, _dir: &Path) -> Result<Scenario, String> {
    Err("not yet".into())
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Database, Keys, Step, load, parse};
    use crate::keymap::{Command, Mods};

    fn read(text: &str) -> Result<super::Scenario, String> {
        parse(text, Path::new("/scenarios"))
    }

    /// The error a scenario with `step` as its one step gives.
    fn refused(step: &str) -> String {
        read(&format!(
            "title = \"T\"\n[[scene]]\nsteps = [{step}]\n"
        ))
        .unwrap_err()
    }

    #[test]
    fn a_scenario_reads_its_scenes_and_steps() {
        let scenario = read(
            r#"
title = "Filtering"
tempo = 100

[[scene]]
caption = "Open a table."
steps = [
  { open = "books" },
  { click = "Lanterns" },
  { command = "move_row", key = "down", times = 3 },
  { press = "enter" },
  { type = "the" },
  { hold = 1.5 },
]

[[scene]]
steps = [{ command = "filter_bar" }]
"#,
        )
        .unwrap();
        assert_eq!(scenario.title, "Filtering");
        assert_eq!(scenario.tempo, 100);
        assert_eq!(scenario.database, Database::Bookshop);
        assert_eq!(scenario.scenes.len(), 2);
        let first = &scenario.scenes[0];
        assert_eq!(first.caption.as_deref(), Some("Open a table."));
        let down = Keys {
            spelled: "down",
            key: egui::Key::ArrowDown,
            mods: Mods::default(),
        };
        let enter = Keys {
            spelled: "enter",
            key: egui::Key::Enter,
            mods: Mods::default(),
        };
        assert_eq!(
            first.steps,
            vec![
                Step::Open("books".into()),
                Step::Click("Lanterns".into()),
                Step::Command {
                    command: Command::MoveRow,
                    keys: down,
                    times: 3,
                },
                Step::Press {
                    keys: enter,
                    times: 1,
                },
                Step::Type("the".into()),
                Step::Hold(1.5),
            ]
        );
        assert_eq!(scenario.scenes[1].caption, None);
    }

    #[test]
    fn a_command_without_a_key_takes_its_first() {
        let scenario = read("title = \"T\"\n[[scene]]\nsteps = [{ command = \"filter_bar\" }]\n")
            .unwrap();
        let Step::Command { command, keys, times } = &scenario.scenes[0].steps[0] else {
            panic!("not a command: {:?}", scenario.scenes[0].steps[0]);
        };
        assert_eq!(*command, Command::FilterBar);
        assert_eq!(*times, 1);
        assert!(keys.mods.cmd, "the filter bar's first key on macOS holds Cmd");
        assert_eq!(keys.key, egui::Key::F);
    }

    #[test]
    fn the_tempo_is_112_unless_the_scenario_says() {
        let scenario = read("title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n").unwrap();
        assert_eq!(scenario.tempo, 112);
    }

    #[test]
    fn a_database_path_is_from_the_scenario_s_directory() {
        let scenario = read(
            "title = \"T\"\ndatabase = \"../fizzy.sqlite\"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
        )
        .unwrap();
        assert_eq!(
            scenario.database,
            Database::File(Path::new("/scenarios/../fizzy.sqlite").into())
        );
    }

    #[test]
    fn a_file_is_read_and_its_errors_name_it() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("demo.toml");
        std::fs::write(&path, "title = \"Demo\"\n[[scene]]\nsteps = [{ hold = 1 }]\n").unwrap();
        assert_eq!(load(&path).unwrap().title, "Demo");
        std::fs::write(&path, "title = \"Demo\"\n").unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.starts_with(&path.display().to_string()), "{error}");
    }

    #[test]
    fn what_a_scenario_cannot_say_is_refused_with_where() {
        for (step, said) in [
            ("{ command = \"no_such\" }", "no command is called \"no_such\""),
            ("{ command = \"reopen_closed_tab\" }", "reopen_closed_tab is not built yet"),
            ("{ command = \"filter_bar\", key = \"f5\" }", "\"f5\" is not a key of filter_bar"),
            ("{ open = \"books\", click = \"x\" }", "a step is one of"),
            ("{ key = \"down\", press = \"down\" }", "only a command takes a key"),
            ("{ open = \"books\", times = 2 }", "only a command or a press takes times"),
            ("{ press = \"down\", times = 0 }", "times is at least 1"),
            ("{ press = \"g g\" }", "is not one press of a key"),
            ("{ press = \"nonsense\" }", "nonsense"),
            ("{ type = \"\" }", "type needs text"),
            ("{ hold = 0 }", "hold 0 is not above 0"),
            ("{ wait = 1 }", "unknown field"),
        ] {
            let error = refused(step);
            assert!(error.starts_with("scene 1, step 1: ") || said == "unknown field", "{step}: {error}");
            assert!(error.contains(said), "{step}: {error}");
        }
    }

    #[test]
    fn a_scenario_needs_scenes_steps_and_a_tempo_it_can_play() {
        for (text, said) in [
            ("title = \"T\"\n", "a scenario needs a [[scene]]"),
            ("title = \"T\"\n[[scene]]\ncaption = \"x\"\n", "scene 1: a scene needs steps"),
            ("title = \"T\"\ntempo = 200\n[[scene]]\nsteps = [{ hold = 1 }]\n", "tempo 200 is not between 80 and 160"),
        ] {
            let error = read(text).unwrap_err();
            assert!(error.contains(said), "{text}: {error}");
        }
    }
}
```

Two of those messages come from outside this module. The words for
`nonsense` are the keymap parser's (`keymap::Unread`), and a field serde
does not know is reported as `unknown field`. If either test fails only
because those words differ, change the expected text to the words the
parser actually gives, not the code.

- [ ] **Step 2: Run the tests and see them fail**

Run: `cargo test --locked --lib promo::scenario`
Expected: the seven tests FAIL with `not yet`.

- [ ] **Step 3: Write the parser**

Replace the two stubs with this. Replace the stub's `use crate::keymap::{Command, Layout, Mods};`
with `use crate::keymap::{self, Command, Keymap, Kind, Layout, Mods, Press, Spelled, Stroke};`,
and add `use serde::Deserialize;`:

```rust
/// The tempo when the scenario names none, and the ones it may name.
const TEMPO: u32 = 112;
const TEMPOS: std::ops::RangeInclusive<u32> = 80..=160;

/// The longest `hold`, in seconds.
const LONGEST_HOLD: f64 = 10.0;

/// Reads the scenario at `path`. A database it names is from its directory.
pub fn load(path: &Path) -> Result<Scenario, String> {
    let at = |error| format!("{}: {error}", path.display());
    let text = std::fs::read_to_string(path).map_err(|error| at(error.to_string()))?;
    let dir = path.parent().unwrap_or(Path::new("."));
    parse(&text, dir).map_err(at)
}

/// Reads a scenario's text; `dir` is where a database it names is from.
pub fn parse(text: &str, dir: &Path) -> Result<Scenario, String> {
    let file: File = toml::from_str(text).map_err(|error| error.to_string())?;
    if !TEMPOS.contains(&file.tempo) {
        return Err(format!(
            "tempo {} is not between {} and {}",
            file.tempo,
            TEMPOS.start(),
            TEMPOS.end()
        ));
    }
    if file.scenes.is_empty() {
        return Err("a scenario needs a [[scene]]".into());
    }
    let database = match file.database.as_deref() {
        None | Some("bookshop") => Database::Bookshop,
        Some(path) => Database::File(dir.join(path)),
    };
    let scenes = file
        .scenes
        .into_iter()
        .enumerate()
        .map(|(index, scene)| {
            let at = format!("scene {}", index + 1);
            if scene.steps.is_empty() {
                return Err(format!("{at}: a scene needs steps"));
            }
            let steps = scene
                .steps
                .into_iter()
                .enumerate()
                .map(|(step, raw)| raw.read().map_err(|error| format!("{at}, step {}: {error}", step + 1)))
                .collect::<Result<_, _>>()?;
            Ok(Scene {
                caption: scene.caption,
                steps,
            })
        })
        .collect::<Result<_, String>>()?;
    Ok(Scenario {
        title: file.title,
        database,
        tempo: file.tempo,
        scenes,
    })
}

/// The file as it is written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    title: String,
    database: Option<String>,
    #[serde(default = "tempo")]
    tempo: u32,
    #[serde(rename = "scene", default)]
    scenes: Vec<RawScene>,
}

fn tempo() -> u32 {
    TEMPO
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawScene {
    caption: Option<String>,
    #[serde(default)]
    steps: Vec<RawStep>,
}

/// A step as the file writes it: one of its names, and what that one takes.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawStep {
    open: Option<String>,
    click: Option<String>,
    command: Option<String>,
    key: Option<String>,
    press: Option<String>,
    times: Option<u32>,
    #[serde(rename = "type")]
    typed: Option<String>,
    hold: Option<f64>,
}

impl RawStep {
    fn read(self) -> Result<Step, String> {
        let RawStep {
            open,
            click,
            command,
            key,
            press,
            times,
            typed,
            hold,
        } = self;
        let names = [
            open.is_some(),
            click.is_some(),
            command.is_some(),
            press.is_some(),
            typed.is_some(),
            hold.is_some(),
        ];
        if names.iter().filter(|named| **named).count() != 1 {
            return Err("a step is one of open, click, command, press, type or hold".into());
        }
        if key.is_some() && command.is_none() {
            return Err("only a command takes a key".into());
        }
        if times.is_some() && command.is_none() && press.is_none() {
            return Err("only a command or a press takes times".into());
        }
        let times = times.unwrap_or(1);
        if times == 0 {
            return Err("times is at least 1".into());
        }
        if let Some(object) = open {
            return Ok(Step::Open(object));
        }
        if let Some(text) = click {
            return Ok(Step::Click(text));
        }
        if let Some(id) = command {
            return command_step(&id, key.as_deref(), times);
        }
        if let Some(keys) = press {
            return Ok(Step::Press {
                keys: chord(&keys)?,
                times,
            });
        }
        if let Some(text) = typed {
            if text.is_empty() {
                return Err("type needs text".into());
            }
            return Ok(Step::Type(text));
        }
        match hold {
            Some(seconds) if seconds > 0.0 && seconds <= LONGEST_HOLD => Ok(Step::Hold(seconds)),
            Some(seconds) => Err(format!(
                "hold {seconds} is not above 0 and up to {LONGEST_HOLD}"
            )),
            None => Err("a step is one of open, click, command, press, type or hold".into()),
        }
    }
}

/// The command `id` names, pressed with `key` (one of its own keys on
/// macOS) or else with its first.
fn command_step(id: &str, key: Option<&str>, times: u32) -> Result<Step, String> {
    let command = Command::from_id(id).ok_or_else(|| format!("no command is called {id:?}"))?;
    if !command.info().built {
        return Err(format!("{id} is not built yet"));
    }
    let keymap = Keymap::default();
    let own: Vec<&'static str> = keymap
        .of(command)
        .flat_map(|binding| binding.bound(LAYOUT).chords.iter())
        .filter(|chord| chord.kind == Kind::Own)
        .map(|chord| chord.keys)
        .collect();
    let spelled = match key {
        None => own
            .first()
            .copied()
            .ok_or_else(|| format!("{id} has no key on macOS"))?,
        Some(key) => own.iter().copied().find(|own| *own == key).ok_or_else(|| {
            format!("{key:?} is not a key of {id}: its keys are {}", own.join(", "))
        })?,
    };
    Ok(Step::Command {
        command,
        keys: chord(spelled)?,
        times,
    })
}

/// `keys`, as `settings.toml` spells a key, read as one press of a key
/// with what is held: the only kind a video plays.
fn chord(keys: &str) -> Result<Keys, String> {
    let spelled = keymap::intern(keys);
    let read = keymap::spell(spelled).map_err(|keymap::Unread(why)| why)?;
    let Spelled::Strokes(strokes) = &read else {
        return Err(format!("{spelled} is not one press of a key"));
    };
    match (Press::of(&read), strokes.as_slice()) {
        (Some(Press::Chord), [Stroke::Key { mods, key }]) => Ok(Keys {
            spelled,
            key: *key,
            mods: *mods,
        }),
        _ => Err(format!("{spelled} is not one press of a key")),
    }
}
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `cargo test --locked --lib promo::scenario`
Expected: 7 passed.

If `filter_bar`'s first macOS key is not `cmd+f`, look it up in
`src/keymap.rs` (`Command::FilterBar`) and correct that test's expected key,
not the code.

- [ ] **Step 5: Format, lint, and the keys test**

`reopen_closed_tab` is `.reserved()` (not built) in `Command::info`. If
that has changed, take another `.reserved()` command for that test case.

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo test --locked --test keys`
Expected: clean, and the keys test passes. These tests spell no `cmd+`.
If the keys test does flag `src/promo/scenario.rs`, it is a test file in
its words: add it to `TESTS` in `tests/keys.rs`, keeping the list sorted,
and raise the array's length.

- [ ] **Step 6: Commit**

```bash
git add src/lib.rs src/promo.rs src/promo/scenario.rs
git commit -m "A promotional video's scenario, read and checked

A small TOML file: a title, the database, the music's tempo, and scenes
of a caption and steps. A step is open, click, command, press, type or
hold. A command's key is one of its own on macOS and a press is one
chord, both read by the keymap's parser. Nothing in a scenario is timed.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 3: the live backend

**Files:**
- Create: `src/promo/live.rs`
- Modify: `src/promo.rs` (uncomment `mod live;`)

- [ ] **Step 1: Port `Live`**

Port it from `git show claude/promo-video:src/promo.rs`, against today's
`backend::Command`. The match names every variant. The new variants are
`Lookup` (answered), and `LoadZones`, `OpenKeysList`, `ReadTheme`,
`ReadFont`, `ListFonts`, `Agents` and `History` (not answered).

```rust
//! The app's backend, answered by a real SQLite session reading the
//! scenario's file. Every SQLite session is `query_only`: nothing is
//! written, saved or stored.

#![allow(clippy::unwrap_used)]

use std::path::Path;

use tabletist_db::{Access, ConnectSpec, Connection, Secrets, StopFlag};

use crate::backend::{Command, Event};
use crate::model::Action;
use crate::testing::Harness;

/// The file's connection and how many of the app's commands it answered.
pub struct Live {
    runtime: tokio::runtime::Runtime,
    connection: Connection,
    answered: usize,
}

impl Live {
    pub fn open(path: &Path) -> Self {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let spec = ConnectSpec::sqlite(path);
        let connection = runtime
            .block_on(Connection::connect(&spec, &Secrets::default()))
            .unwrap();
        Self {
            runtime,
            connection,
            answered: 0,
        }
    }

    pub fn access(&self) -> Access {
        self.connection.access()
    }

    /// Answers every command the app sent since the last call, and those
    /// the answers make it send.
    pub fn pump(&mut self, harness: &mut Harness) {
        while let Some(command) = harness.app.backend.sent.get(self.answered) {
            self.answered += 1;
            let connection = &self.connection;
            let run = &self.runtime;
            let event = match command {
                Command::Connect {
                    session,
                    request,
                    access,
                    ..
                } => Some(Event::Connected {
                    session: *session,
                    request: *request,
                    driver: connection.driver(),
                    encrypted: false,
                    access: *access,
                }),
                Command::ListSchemas { session, request } => Some(Event::Schemas {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.list_schemas()),
                }),
                Command::ListObjects {
                    session,
                    request,
                    schema,
                } => Some(Event::Objects {
                    session: *session,
                    request: *request,
                    schema: schema.clone(),
                    result: run.block_on(connection.list_objects(schema)),
                }),
                Command::Describe {
                    session,
                    request,
                    object,
                } => Some(Event::Structure {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.describe(object)),
                }),
                Command::FetchRows {
                    session,
                    request,
                    query,
                } => Some(Event::Rows {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.fetch_rows(query)),
                }),
                Command::CountRows {
                    session,
                    request,
                    query,
                } => Some(Event::Count {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.count_rows(query)),
                }),
                Command::Lookup {
                    session,
                    request,
                    lookup,
                } => Some(Event::Lookup {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.lookup(lookup)),
                }),
                Command::ListDatabases { session, request } => Some(Event::Databases {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.list_databases()),
                }),
                Command::ServerVersion { session, request } => Some(Event::ServerVersion {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.server_version()),
                }),
                Command::RunSql {
                    session,
                    request,
                    statements,
                    limit,
                    mode,
                    ..
                } => Some(Event::SqlRan {
                    session: *session,
                    request: *request,
                    result: run.block_on(connection.run_script(
                        statements,
                        *limit,
                        *mode,
                        &StopFlag::new(),
                    )),
                    cancel: None,
                }),
                // Nothing is written, saved, stored, watched or closed here.
                Command::Write { .. }
                | Command::Test { .. }
                | Command::Close { .. }
                | Command::Cancel { .. }
                | Command::LoadSecret { .. }
                | Command::StoreSecret { .. }
                | Command::Save { .. }
                | Command::WatchSettings { .. }
                | Command::LoadZones
                | Command::EditSettingsFile { .. }
                | Command::RevealSettingsFile { .. }
                | Command::OpenKeysList { .. }
                | Command::ReadTheme { .. }
                | Command::ReadFont { .. }
                | Command::ListFonts
                | Command::Flush { .. }
                | Command::Agents(_)
                | Command::History(_) => None,
            };
            if let Some(event) = event {
                harness.app.apply(Action::Backend(event));
            }
        }
    }
}
```

`run_script`'s arguments may have changed since the old branch. If the
compiler disagrees, call it the way `src/backend.rs` does for `RunSql`.

- [ ] **Step 2: Compile with `shots`**

Uncomment `mod live;` in `src/promo.rs`.
Run: `cargo test --locked --features shots --lib --no-run 2>&1 | grep -E '^(warning|error)' | sort | uniq -c`
Expected: errors, none. Warnings: only `never used` ones for `Live`,
which Task 4 uses. Task 3 has no commit of its own: Task 4 commits
`live.rs` with the runner, so that every commit passes clippy.

### Task 4: the runner

**Files:**
- Create: `src/promo/runner.rs`
- Create: `contrib/promo/scenarios/smoke.toml`
- Modify: `src/promo.rs` (uncomment `mod runner;`)

- [ ] **Step 1: The smoke scenario**

`contrib/promo/scenarios/smoke.toml`:

```toml
# A few seconds that go through every kind of step: for checking that the
# pipeline works, not for posting.
title = "Smoke"
tempo = 120

[[scene]]
caption = "Open a table."
steps = [
  { open = "books" },
  { click = "Lanterns" },
  { command = "move_row", key = "down", times = 2 },
  { hold = 0.5 },
]
```

- [ ] **Step 2: Write the runner**

`src/promo/runner.rs`:

```rust
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
use super::scenario::{self, Database, Keys, Step};
use crate::connections::{ConnectionId, PasswordMode, SavedConnection};
use crate::env::Environment;
use crate::keymap;
use crate::model::Action;
use crate::testing::{self, Harness};

/// The window, in points, and its pixels to a point.
const SIZE: egui::Vec2 = egui::vec2(1200.0, 750.0);
const SCALE: f32 = 2.0;

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
    /// and none of it filmed.
    fn start(database: &Path, name: &str, dir: PathBuf) -> Self {
        let mut harness = Harness::for_shots(SIZE, SCALE, false, crate::theme::Look::macos());
        // Measured on a macOS 26 window with a unified compact toolbar.
        harness.app.titlebar = crate::app::TitleBar {
            height: 38.0,
            inset: 80.0,
        };
        let mut live = Live::open(database);
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
        assert_eq!(live.access(), tabletist_db::Access::ReadOnly);
        // The grid has the window's width.
        if harness.app.workspace(tab).is_some_and(|workspace| workspace.row_panel) {
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
            at: "the start".into(),
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
        assert!(self.take(), "{}: the step changed nothing on screen", self.at);
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
            Step::Open(label) | Step::Click(label) => self.click(label),
            Step::Command { keys, times, .. } => self.keys("command", keys, *times),
            Step::Press { keys, times } => self.keys("press", keys, *times),
            Step::Type(text) => {
                for character in text.chars() {
                    if character == '\n' {
                        self.harness.press(egui::Key::Enter, egui::Modifiers::NONE);
                    } else {
                        self.harness.frame(vec![egui::Event::Text(character.into())]);
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

    /// The pointer moved to the middle of what is called `label`, then a
    /// press and a release there: a frame of the hover, one of the click.
    fn click(&mut self, label: &str) {
        let tree = self.harness.settle();
        let rect = labelled(&tree, label)
            .or_else(|| self.harness.painted_rect(label))
            .unwrap_or_else(|| {
                panic!(
                    "{}: nothing on screen is called {label:?}; the labels are {:?}",
                    self.at,
                    testing::labels(&tree)
                )
            });
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
    /// its column's header, while the focus holds that row.
    fn cell(&self, tree: &TreeUpdate, focused: Option<egui::Rect>) -> Option<egui::Rect> {
        let app = &self.harness.app;
        let workspace = app.workspace(app.active_tab_id())?;
        let tab = workspace.object_tab(workspace.active_tab?)?;
        let cell = tab.selection?;
        let page = tab.rows.value.as_ref()?;
        let row = testing::bounds(tree, &format!("Row {}", cell.row + 1), Role::Button)?;
        if !focused?.contains_rect(row) {
            return None;
        }
        let header = |col: usize| {
            let column = page.columns.get(col)?;
            self.harness.painted_rect(&column.name)
        };
        let left = header(cell.col)?.left() - 6.0;
        let right = header(cell.col + 1).map_or(row.right(), |next| next.left() - 6.0);
        Some(egui::Rect::from_x_y_ranges(left..=right, row.y_range()))
    }
}

/// Where the node called `label` is, whatever its role. Plain labels carry
/// their text as the value, so that counts as the name too.
fn labelled(tree: &TreeUpdate, label: &str) -> Option<egui::Rect> {
    tree.nodes
        .iter()
        .filter(|(_, node)| node.label().or_else(|| node.value()) == Some(label))
        .find_map(|(_, node)| node.bounds().map(rect_of))
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

/// The SQLite file the scenario names, and the connection's name.
fn database(database: &Database) -> (PathBuf, String) {
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
            assert!(path.is_file(), "{} is not a file", path.display());
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
    let (path, connection) = database(&scenario.database);
    let mut film = Film::start(&path, &connection, dir.clone());
    for (index, scene) in scenario.scenes.iter().enumerate() {
        film.scene = Some(index);
        for (number, step) in scene.steps.iter().enumerate() {
            film.at = format!("{}, scene {}, step {}", file.display(), index + 1, number + 1);
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
```

`Step::Command`'s `command` field is read by no one: the keys stand for it.
If clippy or rustc says so, keep the field, since the scenario's tests and
its errors are about the command, and write `Step::Command { command: _,
keys, times }` in the match.

- [ ] **Step 3: Run it on the smoke scenario**

Uncomment `mod runner;` in `src/promo.rs`.
Run: `PROMO_SCENARIO=$PWD/contrib/promo/scenarios/smoke.toml cargo test --locked --features shots --lib promo::runner::film -- --ignored --exact --nocapture 2>&1 | tail -5`
Expected: `N frames, M events in .../target/promo/smoke` and
`test result: ok`. That is 5 to 7 frames and 6 events: `start`, `point`,
`click`, `command`, `command`, `hold`.

If it panics with "nothing on screen is called", the message lists the
labels the tree has. Fix the scenario, or the tree item's label, by what
it shows, and read the frames written so far (next step) to see where the
app stood.

- [ ] **Step 4: Look at what it filmed**

Read `target/promo/smoke/frames/0000.png` and the last frame with the Read
tool, and read `target/promo/smoke/events.json`.

Expected:
- frame 0 is the connected window, dark, macOS chrome inset, the tree
  showing `books`, and no row panel;
- the last frame shows the books table with the cursor two rows below
  "Lanterns";
- the `command` events' `rect` is a cell-sized rect (about 20 points
  tall), not the whole grid.

Then check for flicker. The harness clock moves 1/60 s a frame, so a
blinking text caret could differ between frames that show the same thing.
Run the smoke scenario twice and compare the frames' hashes
(`shasum target/promo/smoke/frames/*`). If they differ, or a later
scenario's frames show the caret coming and going while typing, find where
the app reads the caret's blink. Keep it steady for the harness, through
a setting the harness sets, the way `lookup_delay` is set in `with_backend`.

- [ ] **Step 5: Checks, and commit**

Run: `cargo fmt --all && cargo clippy --locked --workspace --all-targets -- -D warnings && cargo clippy --locked --all-targets --features shots -- -D warnings && cargo test --locked --lib promo && cargo test --locked --features shots --lib promo`
Expected: all clean, and the scenario's 7 tests pass. Without `shots`,
`film` is not compiled. With it, `film` is listed as ignored.

```bash
git add src/promo.rs src/promo/live.rs src/promo/runner.rs contrib/promo/scenarios/smoke.toml
git commit -m "The promotional video's runner plays a scenario as input

An ignored test behind shots sets up a connected window on the
Bookshop (or the scenario's SQLite file, read-only). It then plays each
step as a person's input: a pointer press on a node or painted text, a
command's key from the keymap, typed characters. It writes a PNG for
each change on screen and events.json, saying what happened in each
frame and where, and stops at a step that changed nothing.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 5: the command and the format

**Files:**
- Create: `contrib/promo/make` (executable)
- Create: `contrib/promo/README.md`

- [ ] **Step 1: Find the puppeteer-core version to pin**

Run: `npm view puppeteer-core time --json | node -e 'const t=JSON.parse(require("fs").readFileSync(0));const old=Date.now()-14*864e5;console.log(Object.keys(t).filter(v=>/^\d+\.\d+\.\d+$/.test(v)&&Date.parse(t[v])<old).pop())'`
Expected: a version such as `24.x.y`. Pin that one below, in place of
`PUPPETEER`.

- [ ] **Step 2: Write `make`**

```bash
#!/usr/bin/env bash
# Makes a promotional video from a scenario:
#
#   contrib/promo/make contrib/promo/scenarios/overview.toml [--final] [--still <seconds>...] [--frames-only] [--reuse]
#
# It writes target/promo/<name>.mp4 (stills beside it) and keeps its
# working files in target/promo/<name>/. It needs ffmpeg, Node and Google
# Chrome (CHROME names another); puppeteer-core is installed into
# target/promo/ on the first run. The scenario's format: contrib/promo/README.md.
set -euo pipefail

root=$(cd "$(dirname "$0")/../.." && pwd)
if [[ $# -lt 1 ]]; then
  sed -n '2,9p' "$0"
  exit 2
fi
scenario=$(cd "$(dirname "$1")" && pwd)/$(basename "$1")
shift
name=$(basename "$scenario" .toml)
out=$root/target/promo

reuse=false
frames_only=false
render=()
for arg in "$@"; do
  case $arg in
    --reuse) reuse=true ;;
    --frames-only) frames_only=true ;;
    *) render+=("$arg") ;;
  esac
done

if ! $reuse || [[ ! -f $out/$name/events.json ]]; then
  (cd "$root" && PROMO_SCENARIO=$scenario cargo test --locked --features shots --lib \
    promo::runner::film -- --ignored --exact --nocapture)
fi
$frames_only && exit 0

puppeteer=PUPPETEER
installed=$out/node_modules/puppeteer-core/package.json
if [[ ! -f $installed ]] || ! grep -q "\"version\": \"$puppeteer\"" "$installed"; then
  npm install --prefix "$out" --no-audit --no-fund "puppeteer-core@$puppeteer"
fi
NODE_PATH=$out/node_modules node "$root/contrib/promo/render.cjs" \
  "$out/$name" "$out/$name.mp4" ${render[@]+"${render[@]}"}
```

`${render[@]+"${render[@]}"}` is there because macOS's bash 3.2, under
`set -u`, calls an empty array unbound.

Run: `chmod +x contrib/promo/make`

- [ ] **Step 3: Write the README**

`contrib/promo/README.md`. It is the format reference, taken from the
spec's "The scenario", "Where a video starts", "The steps" and "The
command" sections, in the same words. It adds an example of a feature
explained in three scenes, and these notes:
- what a failing step's message says (file, scene, step) and how to read
  the frames written up to it;
- `--reuse` while tuning a scenario's look and sound, and that it never
  changes what the steps do.

- [ ] **Step 4: Run it, frames only**

Run: `contrib/promo/make contrib/promo/scenarios/smoke.toml --frames-only 2>&1 | tail -3`
Expected: the runner's `N frames, M events` line, and exit status 0.

Run: `contrib/promo/make 2>&1 | head -3; echo "status $?"`
Expected: the usage lines, and `status 2`.

- [ ] **Step 5: Commit**

```bash
git add contrib/promo/make contrib/promo/README.md
git commit -m "One command makes a promotional video from a scenario

contrib/promo/make runs the runner, installs a pinned puppeteer-core
into target/promo/ the first time, and hands the frames to the renderer.
The README is the scenario format.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

(`render.cjs` comes in Task 7. Until then `make` without `--frames-only`
stops at Node, saying it is missing.)

---

## Phase 2: the director and the picture

### Task 6: the director

**Files:**
- Create: `contrib/promo/director.js`
- Create: `contrib/promo/director.test.js`

- [ ] **Step 1: Write the tests**

`contrib/promo/director.test.js`:

```js
// The director's tests: node --test contrib/promo/director.test.js

"use strict";

const test = require("node:test");
const assert = require("node:assert/strict");
const { schedule, place, film, FPS } = require("./director.js");

// A record as the runner writes one.
const record = (events, scenes = [{ caption: "Open a table." }], tempo = 120) => ({
  title: "Test",
  tempo,
  size: [1200, 750],
  scale: 2,
  scenes,
  events: [{ scene: null, frame: 0, kind: "start" }, ...events],
});

const rect = [100, 100, 200, 120];
const typing = (text, from = 1) =>
  [...text].map((character, i) => ({
    scene: 0, frame: from + i, kind: "type", text: character,
    rect: [300 + 8 * i, 96, 308 + 8 * i, 116],
  }));

test("a scene starts after the intro's bar and ends on a bar", () => {
  const { scenes, bar } = place(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(bar, 2);
  assert.equal(scenes[0].from, 2);
  assert.equal(scenes[0].to % bar, 0);
  assert.ok(scenes[0].to > scenes[0].from);
});

test("a caption gets the time to be read", () => {
  const caption = "one two three four five six seven eight nine ten";
  const { scenes } = place(record([{ scene: 0, frame: 1, kind: "press", keys: "↩", rect }], [{ caption }]));
  assert.ok(scenes[0].to - scenes[0].from >= 3, `${scenes[0].to - scenes[0].from} s`);
});

test("typing is human, and the same every time", () => {
  const events = typing("select title from books");
  const first = schedule(record(events));
  assert.deepEqual(schedule(record(events)), first);
  const times = place(record(events)).timed.map(e => e.at);
  const gaps = times.slice(1).map((t, i) => t - times[i]);
  assert.ok(new Set(gaps.map(g => g.toFixed(3))).size > 5, "the gaps differ");
  const rate = (times.length - 1) / (times[times.length - 1] - times[0]);
  assert.ok(rate > 9 && rate < 17, `${rate} characters a second`);
});

test("the pointer arrives when its point is due, and the click follows", () => {
  const timeline = schedule(record([
    { scene: 0, frame: 1, kind: "point", rect: [100, 200, 300, 220] },
    { scene: 0, frame: 2, kind: "click", rect: [100, 200, 300, 220] },
  ]));
  const [point] = place(record([{ scene: 0, frame: 1, kind: "point", rect: [100, 200, 300, 220] }])).timed;
  const arrived = timeline.pointer[Math.ceil(point.at * FPS)];
  assert.ok(Math.abs(arrived.x - 200) < 1 && Math.abs(arrived.y - 210) < 1, JSON.stringify(arrived));
  assert.equal(timeline.clicks.length, 1);
  assert.ok(timeline.clicks[0].at > point.at);
});

test("the camera holds still while typing goes along a line", () => {
  const { timed, outro } = place(record(typing("select title from books")));
  const { moves } = film(timed, outro, outro + 1);
  // One move to the line, one back to the whole window for the outro.
  assert.ok(moves.length <= 3, `${moves.length} moves`);
});

test("the spring never overshoots", () => {
  const timed = [{ kind: "click", at: 1, rect: [1000, 600, 1100, 640] }];
  const { camera } = film(timed, 10, 4);
  const xs = camera.map(c => c.x);
  for (let i = 1; i < xs.length; i++) assert.ok(xs[i] >= xs[i - 1] - 1e-9, `x went back at frame ${i}`);
  const target = camera[camera.length - 1].x;
  assert.ok(xs.every(x => x <= target + 1e-6), "x went past where it stopped");
});

test("the camera shows the whole window for the outro", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "click", rect: [1000, 600, 1100, 640] }]));
  const last = timeline.camera[timeline.camera.length - 1];
  assert.ok(Math.abs(last.zoom - 1) < 0.01 && Math.abs(last.x - 600) < 2 && Math.abs(last.y - 375) < 2, JSON.stringify(last));
});

test("every frame shown is one the runner wrote, in order", () => {
  const timeline = schedule(record([...typing("abc"), { scene: 0, frame: 3, kind: "hold", seconds: 1 }]));
  assert.deepEqual(timeline.frames.map(f => f[1]), [0, 1, 2, 3]);
  const at = timeline.frames.map(f => f[0]);
  assert.deepEqual(at, [...at].sort((a, b) => a - b));
});

test("a command shows its keycap, and makes its sounds", () => {
  const timeline = schedule(record([{ scene: 0, frame: 1, kind: "command", keys: "⌘F", rect }]));
  assert.equal(timeline.keycaps.length, 1);
  assert.equal(timeline.keycaps[0].keys, "⌘F");
  const kinds = timeline.sounds.map(s => s.kind);
  assert.ok(kinds.includes("key") && kinds.includes("tick") && kinds.includes("bell"), kinds.join());
});

test("an event the director does not know stops it", () => {
  assert.throws(() => place(record([{ scene: 0, frame: 1, kind: "teleport" }])), /teleport/);
});
```

- [ ] **Step 2: Run them and see them fail**

Run: `node --test contrib/promo/director.test.js`
Expected: FAIL, `Cannot find module './director.js'`.

- [ ] **Step 3: Write the director**

`contrib/promo/director.js`:

```js
// The director: what the runner saw (events.json) made into a timeline.
// It is the one place a time is decided. The picture (index.html) and the
// sound (audio.js) only read what it returns. Pure: the same events give
// the same timeline. The design:
// docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md.
//
// render.cjs requires it; index.html loads it as a script (Director).

(function () {
  "use strict";

  const FPS = 60;
  // The window, in points: every rect is in them.
  const WIDTH = 1200, HEIGHT = 750;
  // Characters typed a second, on average.
  const TYPE_RATE = 13;
  // The camera frames what happens from BEHIND seconds ago to
  // LEAD + AHEAD seconds on, so it is there as the action starts.
  const LEAD = 0.25, AHEAD = 0.6, BEHIND = 0.4;
  const ZOOM = [1, 1.7];
  // The spring's angular frequency: it settles in about half a second.
  const OMEGA = 2 * Math.PI * 1.1;
  const READ_WORD = 0.3, READ_LEAST = 1.6;
  // How long a result stays after a scene's last step.
  const SETTLE = 0.6;
  const KEYCAP = 0.8;

  const clamp = (x, a, b) => Math.min(b, Math.max(a, x));

  // Numbers in [0, 1) that `text` always starts the same (FNV-1a, then
  // mulberry32).
  function seeded(text) {
    let h = 2166136261;
    for (const c of text) h = Math.imul(h ^ c.codePointAt(0), 16777619) >>> 0;
    return () => {
      h = (h + 0x6d2b79f5) >>> 0;
      let t = h;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }

  // The first bar line at or after `t`.
  const onBar = (t, bar) => Math.ceil(t / bar - 1e-9) * bar;

  const centre = ([x0, y0, x1, y1]) => ({ x: (x0 + x1) / 2, y: (y0 + y1) / 2 });

  // The last of `list` (in time order) that `ok` holds for.
  function latest(list, ok) {
    let found = null;
    for (const item of list) if (ok(item)) found = item;
    return found;
  }

  // When each event happens, scene by scene, and when the scenes are.
  function place(record) {
    const beat = 60 / record.tempo, bar = 4 * beat;
    const random = seeded(record.title);
    const timed = [], scenes = [];
    let pointer = { x: WIDTH * 0.7, y: HEIGHT * 0.75 };
    // The intro is a bar.
    let t = bar;
    record.scenes.forEach((scene, index) => {
      const from = t;
      let now = from + beat / 2;
      for (const event of record.events.filter(e => e.scene === index)) {
        switch (event.kind) {
          case "point": {
            const to = centre(event.rect);
            const far = Math.hypot(to.x - pointer.x, to.y - pointer.y);
            const travel = clamp(0.35 + far / 1800, 0.35, 0.7);
            timed.push({ ...event, at: now + travel, leaves: now, origin: pointer, target: to });
            pointer = to;
            now += travel;
            break;
          }
          case "click":
            now += 0.12;
            timed.push({ ...event, at: now, where: pointer });
            now += 0.35;
            break;
          case "command":
          case "press":
            timed.push({ ...event, at: now });
            now += 0.35;
            break;
          case "type": {
            const pause = /\s/.test(event.text) ? 1.4 : 1;
            now += (pause * (0.6 + 0.9 * random())) / TYPE_RATE;
            timed.push({ ...event, at: now });
            break;
          }
          case "hold":
            now += event.seconds;
            break;
          default:
            throw new Error(`an event the director does not know: ${event.kind}`);
        }
      }
      now += SETTLE;
      const words = scene.caption ? scene.caption.split(/\s+/).length : 0;
      const reading = scene.caption ? Math.max(READ_LEAST, READ_WORD * words) : 0;
      const to = onBar(Math.max(now, from + reading), bar);
      scenes.push({ from, to, caption: scene.caption || null });
      t = to;
    });
    return { timed, scenes, outro: t, beat, bar };
  }

  // What the camera looks at to frame `rects`: their middle, zoomed so they
  // fill about half the picture, kept inside the window.
  function view(rects) {
    const x0 = Math.min(...rects.map(r => r[0])), y0 = Math.min(...rects.map(r => r[1]));
    const x1 = Math.max(...rects.map(r => r[2])), y1 = Math.max(...rects.map(r => r[3]));
    const zoom = clamp(0.5 / Math.max((x1 - x0) / WIDTH, (y1 - y0) / HEIGHT, 1e-3), ZOOM[0], ZOOM[1]);
    const half = { x: WIDTH / 2 / zoom, y: HEIGHT / 2 / zoom };
    return {
      x: clamp((x0 + x1) / 2, half.x, WIDTH - half.x),
      y: clamp((y0 + y1) / 2, half.y, HEIGHT - half.y),
      zoom,
      area: { x0: x0 - 12, y0: y0 - 8, x1: x1 + 12, y1: y1 + 8, alpha: 1 },
    };
  }

  const WHOLE = {
    x: WIDTH / 2, y: HEIGHT / 2, zoom: 1,
    area: { x0: 0, y0: 0, x1: WIDTH, y1: HEIGHT, alpha: 0 },
  };

  // Whether `want` is near enough where the camera is going that it stays:
  // inside the middle third of the view, at about the same zoom.
  function resting(target, want) {
    const third = { x: WIDTH / target.zoom / 6, y: HEIGHT / target.zoom / 6 };
    return Math.abs(want.x - target.x) < third.x
      && Math.abs(want.y - target.y) < third.y
      && Math.abs(want.zoom - target.zoom) / target.zoom < 0.15;
  }

  const distance = (a, b) =>
    Math.hypot((a.x - b.x) / WIDTH, (a.y - b.y) / HEIGHT) + Math.abs(Math.log(b.zoom / a.zoom));

  // One frame of a critically damped spring, solved exactly: from rest it
  // never overshoots.
  function spring(pos, vel, target, dt) {
    const e = Math.exp(-OMEGA * dt);
    for (const k of Object.keys(target)) {
      const x0 = pos[k] - target[k], v0 = vel[k], c = v0 + OMEGA * x0;
      pos[k] = target[k] + (x0 + c * dt) * e;
      vel[k] = (v0 - OMEGA * c * dt) * e;
    }
  }

  // Where the camera and the spotlight are, a frame at a time, and when
  // the camera set off for somewhere new (for the whooshes).
  function film(timed, outro, duration) {
    const framed = timed.filter(e => e.rect);
    let target = WHOLE;
    const pos = { x: WHOLE.x, y: WHOLE.y, zoom: 1 }, vel = { x: 0, y: 0, zoom: 0 };
    const spot = { ...WHOLE.area }, spotVel = { x0: 0, y0: 0, x1: 0, y1: 0, alpha: 0 };
    const camera = [], spots = [], moves = [];
    for (let i = 0, n = Math.round(duration * FPS); i < n; i++) {
      const t = i / FPS;
      let want = null;
      if (t >= outro - LEAD) want = WHOLE;
      else {
        const near = framed.filter(e => e.at > t - BEHIND && e.at <= t + LEAD + AHEAD);
        if (near.length) want = view(near.map(e => e.rect));
      }
      if (want && want !== target && !(want !== WHOLE && target !== WHOLE && resting(target, want))) {
        moves.push({ at: t, far: distance(target, want) });
        target = want;
      }
      spring(pos, vel, { x: target.x, y: target.y, zoom: target.zoom }, 1 / FPS);
      spring(spot, spotVel, target.area, 1 / FPS);
      camera.push({ ...pos });
      spots.push({ ...spot });
    }
    return { camera, spot: spots, moves };
  }

  // Where the pointer is, frame by frame: it travels to each point on a
  // slight arc, eased in and out, presses for a click, and hides while
  // the keyboard types.
  function point(timed, duration) {
    const trips = timed.filter(e => e.kind === "point");
    const typing = timed.filter(e => e.kind === "type");
    const clicks = timed.filter(e => e.kind === "click");
    const frames = [];
    for (let i = 0, n = Math.round(duration * FPS); i < n; i++) {
      const t = i / FPS;
      const trip = latest(trips, e => e.leaves <= t);
      if (!trip) {
        frames.push({ x: 0, y: 0, shown: 0, press: 0 });
        continue;
      }
      const k = clamp((t - trip.leaves) / (trip.at - trip.leaves), 0, 1);
      const e = k < 0.5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2;
      const a = trip.origin, b = trip.target;
      const dx = b.x - a.x, dy = b.y - a.y, length = Math.hypot(dx, dy) || 1;
      const bend = 0.12 * length;
      const control = { x: (a.x + b.x) / 2 + (dy / length) * bend, y: (a.y + b.y) / 2 - (dx / length) * bend };
      const u = 1 - e;
      const x = u * u * a.x + 2 * u * e * control.x + e * e * b.x;
      const y = u * u * a.y + 2 * u * e * control.y + e * e * b.y;
      const index = trips.indexOf(trip);
      const before = index === 0 ? -Infinity : trips[index - 1].at;
      const returns = index === 0 || typing.some(e => e.at > before && e.at <= trip.leaves);
      const appear = returns ? clamp((t - trip.leaves) / 0.15, 0, 1) : 1;
      const typed = latest(typing, e => e.at <= t);
      const hidden = typed && typed.at > trip.leaves ? clamp((t - typed.at) / 0.15, 0, 1) : 0;
      const click = latest(clicks, e => e.at - 0.06 <= t);
      const press = click && t < click.at + 0.1 ? 1 : 0;
      frames.push({ x, y, shown: appear * (1 - hidden), press });
    }
    return frames;
  }

  // What is heard when: a key for each character and key, a click, a
  // whoosh for each camera move by how far it goes, a tick under a keycap,
  // a bell on the outro.
  function sounds(timed, moves, outro, random) {
    const all = [];
    for (const e of timed) {
      if (e.kind === "type") all.push({ at: e.at, kind: "key", gain: 0.55 + 0.35 * random() });
      if (e.kind === "press") all.push({ at: e.at, kind: "key", gain: 0.9 });
      if (e.kind === "command") all.push({ at: e.at, kind: "key", gain: 1 }, { at: e.at, kind: "tick", gain: 0.8 });
      if (e.kind === "click") all.push({ at: e.at, kind: "click", gain: 1 });
    }
    for (const move of moves) {
      if (move.far > 0.08) all.push({ at: move.at, kind: "whoosh", gain: clamp(move.far, 0.2, 1) });
    }
    all.push({ at: outro, kind: "bell", gain: 1 });
    return all.sort((a, b) => a.at - b.at);
  }

  // The timeline of a record: everything index.html and audio.js read.
  function schedule(record) {
    if (record.size[0] !== WIDTH || record.size[1] !== HEIGHT) {
      throw new Error(`the window is ${record.size.join("x")} points, not ${WIDTH}x${HEIGHT}`);
    }
    const { timed, scenes, outro, beat, bar } = place(record);
    const duration = outro + 2 * bar;
    const { camera, spot, moves } = film(timed, outro, duration);
    return {
      title: record.title,
      tempo: record.tempo,
      fps: FPS,
      beat,
      bar,
      duration,
      size: [WIDTH, HEIGHT],
      intro: { from: 0, to: bar },
      outro,
      frames: [[0, 0], ...timed.map(e => [e.at, e.frame])].sort((a, b) => a[0] - b[0]),
      scenes,
      camera,
      spot,
      pointer: point(timed, duration),
      clicks: timed.filter(e => e.kind === "click").map(e => ({ at: e.at, x: e.where.x, y: e.where.y })),
      keycaps: timed.filter(e => e.kind === "command").map(e => ({ at: e.at, until: e.at + KEYCAP, keys: e.keys })),
      sounds: sounds(timed, moves, outro, seeded(`${record.title} keys`)),
    };
  }

  const api = { schedule, place, film, seeded, FPS };
  if (typeof module !== "undefined") module.exports = api;
  else globalThis.Director = api;
})();
```

- [ ] **Step 4: Run the tests and see them pass**

Run: `node --test contrib/promo/director.test.js`
Expected: 10 passed.

A tuning number may make a test fail: `TYPE_RATE`, `LEAD` or the 0.15 of
`resting`. If so, look at what the test measures before changing either
side. The test says what the spec asks for.

- [ ] **Step 5: Commit**

```bash
git add contrib/promo/director.js contrib/promo/director.test.js
git commit -m "The promotional video's director decides every time

It turns the runner's events into a timeline. It times typing at a
seeded human rhythm and the pointer's trips by distance. Scenes end on
the music's bars, with time to read their caption. The camera frames
what is about to happen on a critically damped spring, and holds still
inside a dead zone. Tested with node --test.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 7: the picture

**Files:**
- Create: `contrib/promo/index.html`
- Create: `contrib/promo/render.cjs` (silent for now; Task 8 adds the sound)

- [ ] **Step 1: Write the composer**

Start from `git show claude/promo-video:contrib/promo/index.html` and keep
these as they are:
- its `<style>`: fonts, `:root`, `#stage`, `#grid`, `#window`, `#shot`,
  `#lights`, `#scrim`, `#caption`, `#key`, `.card` and the rest;
- the intro card;
- the icon's `<svg><defs>`.

Make these changes.

Add to the style:

```css
  #spot { position: absolute; border-radius: 12px; pointer-events: none; }
  #pointer { position: absolute; left: 0; top: 0; width: 26px; height: 38px;
    transform-origin: 2.3px 2.3px; filter: drop-shadow(0 2px 3px rgba(0,0,0,.35)); }
  #ripple { position: absolute; border-radius: 50%; pointer-events: none;
    border: 3px solid rgba(255,255,255,.9); box-shadow: 0 0 0 1px rgba(0,0,0,.25); }
  #line .word { display: inline-block; overflow: hidden; vertical-align: bottom; padding-bottom: .08em; }
  #line .word > span { display: inline-block; }
```

Inside `#window`, after `#lights`:

```html
    <div id="spot"></div>
    <div id="ripple"></div>
    <svg id="pointer" viewBox="0 0 17 25"><path d="M1.5 1.5 L1.5 20.5 L6 16.2 L9.2 23.2 L12.4 21.8 L9.3 15 L15.2 15 Z"
      fill="#000" stroke="#fff" stroke-width="1.5" stroke-linejoin="round"/></svg>
```

The outro card gets the scenario's title as a line under the name:

```html
  <div class="card" id="outro">
    <svg class="icon" viewBox="0 0 1024 1024"><use href="#icon"/></svg>
    <div class="name">Tabletist</div>
    <div class="tag" id="title"></div>
    <div class="tag"><b>PostgreSQL</b> · <b>MySQL</b> · <b>SQLite</b></div>
    <div class="tag" style="margin-top:10px">macOS · Linux · Windows</div>
    <div class="url">igor.works/tabletist</div>
  </div>
```

Before the inline script, load the director (for `seeded`) and the sound:

```html
<script src="director.js"></script>
<script src="audio.js"></script>
```

Replace the old inline script with:

```js
// What the page draws: the director's timeline (T) and the app's frames,
// given by load(). renderAt(seconds) draws that moment, and nothing here
// runs on a clock.

const $ = id => document.getElementById(id);
const clamp = (x, a = 0, b = 1) => Math.min(b, Math.max(a, x));
const mix = (a, b, k) => a + (b - a) * k;
const inOut = k => (k < .5 ? 4 * k * k * k : 1 - Math.pow(-2 * k + 2, 3) / 2);
const out = k => 1 - Math.pow(1 - k, 4);
const back = k => 1 + 2.2 * Math.pow(k - 1, 3) + 1.2 * Math.pow(k - 1, 2);
// How far `t` is through the `length` seconds that start at `from`.
const span = (t, from, length) => clamp((t - from) / length);
const escape = text => text.replace(/[&<>]/g, c => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);

// The window's pixels to a point of the app, and where it rests.
const S = 1400 / 1200, LEFT = 260, TOP = 40;

let T = null, frames = [], shown = null, said = undefined;

// A per-frame list at `t`: between two of its frames, the mix of both (so
// --final's subframes move smoothly).
function sample(list, t) {
  const at = clamp(t * T.fps, 0, list.length - 1), i = Math.floor(at), k = at - i;
  const a = list[i], b = list[Math.min(i + 1, list.length - 1)];
  const value = {};
  for (const key in a) value[key] = mix(a[key], b[key], k);
  return value;
}

// The last of `list` (in time order) that has begun by `t`.
function latest(list, t, at = item => item.at) {
  let found = null;
  for (const item of list) {
    if (at(item) > t + 1e-6) break;
    found = item;
  }
  return found;
}

function drawWindow(t) {
  const el = $("window");
  const rise = out(span(t, T.intro.to - T.beat, .8));
  const leave = inOut(span(t, T.outro - .1, .6));
  const hidden = rise === 0 || leave === 1;
  el.style.visibility = hidden ? "hidden" : "visible";
  if (hidden) return;
  const { x, y, zoom } = sample(T.camera, t);
  // Zoomed in, the point looked at comes to the middle of the picture.
  const pull = clamp((zoom - 1) * 3);
  const sx = mix(LEFT + x * S, 960, pull), sy = mix(TOP + y * S, 470, pull);
  const size = zoom * mix(.92, 1, rise) * mix(1, .86, leave);
  const dx = sx - x * S * size, dy = sy - y * S * size + (1 - rise) * 160 + leave * 40;
  el.style.opacity = rise * (1 - leave);
  el.style.filter = leave ? `blur(${leave * 18}px)` : "none";
  el.style.transform = `translate(${dx}px, ${dy}px) scale(${size})`;
}

// The part of the window that matters stays bright; the rest dims.
function drawSpot(t) {
  const { x0, y0, x1, y1, alpha } = sample(T.spot, t);
  const el = $("spot");
  el.style.left = `${x0 * S}px`;
  el.style.top = `${y0 * S}px`;
  el.style.width = `${(x1 - x0) * S}px`;
  el.style.height = `${(y1 - y0) * S}px`;
  el.style.boxShadow = `0 0 70px 3000px rgba(6, 8, 12, ${(.16 * alpha).toFixed(3)})`;
}

function drawPointer(t) {
  const { x, y, shown, press } = sample(T.pointer, t);
  const el = $("pointer");
  el.style.opacity = shown;
  el.style.transform = `translate(${x * S - 2.3}px, ${y * S - 2.3}px) scale(${mix(1, .86, press)})`;
  const click = latest(T.clicks, t);
  const ripple = $("ripple");
  const k = click ? span(t, click.at, .45) : 1;
  ripple.style.opacity = k < 1 ? (1 - k) * .55 : 0;
  if (click && k < 1) {
    const size = mix(12, 64, out(k));
    ripple.style.width = ripple.style.height = `${size}px`;
    ripple.style.left = `${click.x * S - size / 2}px`;
    ripple.style.top = `${click.y * S - size / 2}px`;
  }
}

function drawCaption(t) {
  const scene = T.scenes.find(s => s.caption && t >= s.from && t < s.to) || null;
  const line = $("line"), key = $("key");
  if (said !== scene) {
    said = scene;
    line.innerHTML = scene
      ? scene.caption.split(" ").map(w => `<span class="word"><span>${escape(w)}</span></span>`).join(" ")
      : "";
  }
  let words = 0;
  if (scene) {
    const gone = span(t, scene.to - .35, .25);
    words = span(t, scene.from, .3) * (1 - gone);
    [...line.children].forEach((word, i) => {
      const k = out(span(t, scene.from + .05 + i * .06, .45));
      word.firstChild.style.transform = `translateY(${(1 - k) * 105}%)`;
      word.firstChild.style.filter = `blur(${(1 - k) * 8}px)`;
      word.style.opacity = (k > 0 ? 1 : 0) * (1 - gone);
    });
  }
  const cap = latest(T.keycaps, t);
  const on = cap && t < cap.until + .2;
  key.style.display = on ? "block" : "none";
  let keys = 0;
  if (on) {
    const pop = span(t, cap.at, .35);
    keys = clamp(pop * 4) * (1 - span(t, cap.until, .2));
    key.textContent = cap.keys;
    key.style.opacity = keys;
    key.style.transform = `scale(${back(pop)})`;
  }
  $("scrim").style.opacity = Math.max(words, keys);
}

// A card's pieces come in a half beat apart, and leave together.
function drawCard(el, t, from, to) {
  const gone = to === null ? 0 : inOut(span(t, to, .4));
  const on = t >= from - .01 && gone < 1;
  el.style.visibility = on ? "visible" : "hidden";
  if (!on) return;
  [...el.children].forEach((piece, i) => {
    const k = span(t, from + i * T.beat / 2, .55);
    const size = i === 0 ? back(k) : 1;
    piece.style.opacity = clamp(k * 2.5) * (1 - gone);
    piece.style.transform =
      `translateY(${(1 - out(k)) * (i ? 50 : 0) - gone * 90}px) scale(${size * mix(1, .94, gone)})`;
  });
}

async function renderAt(t) {
  const now = latest(T.frames, t, frame => frame[0]);
  const file = frames[now ? now[1] : 0];
  if (file !== shown) {
    shown = file;
    $("shot").src = file;
    await $("shot").decode();
  }
  drawWindow(t);
  drawSpot(t);
  drawPointer(t);
  drawCaption(t);
  drawCard($("intro"), t, .08, T.intro.to - .35);
  drawCard($("outro"), t, T.outro + .1, null);
  $("grid").style.transform = `translateY(${-t * 4}px)`;
}

// `timeline` is the director's; `files` are the app's frames, in order.
async function load(timeline, files) {
  T = timeline;
  frames = files;
  $("title").textContent = T.title;
  $("title").style.display = T.title === "Tabletist" ? "none" : "";
  await document.fonts.ready;
  await renderAt(0);
}
```

Create `contrib/promo/audio.js` now, holding only this, so the page loads.
Task 8 fills it in:

```js
// The sound of a promotional video: Task 8.
(function () {
  "use strict";
  globalThis.renderAudio = async () => null;
})();
```

- [ ] **Step 2: Write the renderer (silent)**

`contrib/promo/render.cjs`:

```js
// Renders a promotional video: the director's timeline drawn by index.html
// in a headless Chrome, a screenshot a frame, encoded by ffmpeg.
// contrib/promo/make runs it:
//
//   node contrib/promo/render.cjs <work dir> <out.mp4> [--final] [--still <seconds>...]
//
// The work dir is what the runner (src/promo/runner.rs) wrote: events.json
// and frames/. --final renders four frames for each one and has ffmpeg
// blend them (motion blur). --still writes those moments as PNG beside the
// video's path instead.

"use strict";

const { spawn } = require("node:child_process");
const fs = require("node:fs");
const path = require("node:path");
const puppeteer = require("puppeteer-core");
const { schedule } = require("./director.js");

const chrome =
  process.env.CHROME || "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome";

async function main() {
  const args = process.argv.slice(2);
  const still = args.indexOf("--still");
  const times = still < 0 ? [] : args.splice(still).slice(1).map(Number);
  const final = args.includes("--final");
  const [work, video] = args.filter(arg => !arg.startsWith("--")).map(arg => path.resolve(arg));
  if (!work || !video) throw new Error("usage: render.cjs <work dir> <out.mp4> [--final] [--still <seconds>...]");

  const record = JSON.parse(fs.readFileSync(path.join(work, "events.json"), "utf8"));
  const timeline = schedule(record);
  fs.writeFileSync(path.join(work, "timeline.json"), JSON.stringify(timeline));
  const frames = fs
    .readdirSync(path.join(work, "frames"))
    .filter(name => /^\d{4}\.png$/.test(name))
    .sort()
    .map(name => "file://" + path.join(work, "frames", name));
  console.log(`${frames.length} app frames, ${timeline.duration.toFixed(2)} s`);

  const browser = await puppeteer.launch({
    executablePath: chrome,
    args: ["--allow-file-access-from-files", "--force-color-profile=srgb", "--hide-scrollbars"],
  });
  const page = await browser.newPage();
  page.on("pageerror", error => console.error(error));
  await page.setViewport({ width: 1920, height: 1080, deviceScaleFactor: 1 });
  await page.goto("file://" + path.join(__dirname, "index.html"));
  await page.evaluate((timeline, frames) => load(timeline, frames), timeline, frames);

  if (times.length > 0) {
    for (const time of times) {
      await page.evaluate(time => renderAt(time), time);
      const name = video.replace(/\.mp4$/, `-${time.toFixed(2)}.png`);
      await page.screenshot({ path: name });
      console.log(name);
    }
    await browser.close();
    return;
  }

  const rate = final ? 4 * timeline.fps : timeline.fps;
  const ffmpeg = spawn(
    "ffmpeg",
    [
      "-loglevel", "error", "-y",
      "-f", "image2pipe", "-framerate", String(rate), "-c:v", "png", "-i", "-",
      ...(final ? ["-vf", `tmix=frames=4,fps=${timeline.fps}`] : []),
      "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p",
      "-movflags", "+faststart", video,
    ],
    { stdio: ["pipe", "inherit", "inherit"] },
  );
  const total = Math.round(timeline.duration * rate);
  for (let frame = 0; frame < total; frame++) {
    await page.evaluate(time => renderAt(time), frame / rate);
    const png = await page.screenshot({ type: "png", optimizeForSpeed: true });
    if (!ffmpeg.stdin.write(png)) await new Promise(done => ffmpeg.stdin.once("drain", done));
    if (frame % rate === 0) process.stdout.write(`\r${frame / rate} s of ${timeline.duration.toFixed(1)}`);
  }
  ffmpeg.stdin.end();
  const code = await new Promise(done => ffmpeg.on("close", done));
  await browser.close();
  if (code !== 0) throw new Error(`ffmpeg stopped with ${code}`);
  console.log(`\n${video}`);
}

main().catch(error => {
  console.error(error);
  process.exit(1);
});
```

- [ ] **Step 3: Stills of the smoke scenario**

Run: `contrib/promo/make contrib/promo/scenarios/smoke.toml --reuse --still 0.6 2.2 3.0 4.0 9.0`
Then: `jq '{duration, outro, scenes, clicks}' target/promo/smoke/timeline.json`

Read each PNG under `target/promo/` with the Read tool. Expected:
- 0.6 s: the intro card coming in: icon, name, tagline;
- 2.2 s: the window risen, the pointer on its way to `books`;
- 3.0 s and 4.0 s: zoomed in on the table, the spotlight dimming the
  rest, the caption's words up, the cursor in the grid;
- the last still, past `outro`: the outro card with "Smoke" under the name.

Change times to match the timeline if they fall elsewhere. Fix what looks
wrong before going on. Typical things to fix:
- the pointer tip offset;
- the spot's shadow too strong or too weak;
- a caption clipped by the word mask (`padding-bottom`).

- [ ] **Step 4: The whole smoke video**

Run: `contrib/promo/make contrib/promo/scenarios/smoke.toml --reuse && ffprobe -v error -show_entries format=duration:stream=codec_name,width,height,r_frame_rate -of compact target/promo/smoke.mp4`
Expected:
- `codec_name=h264|width=1920|height=1080|r_frame_rate=60/1`;
- the duration equals the timeline's, within a frame.

Look at the motion in a contact sheet:
`ffmpeg -v error -y -i target/promo/smoke.mp4 -vf "fps=4,scale=480:-1,tile=4x4" -frames:v 1 target/promo/smoke-sheet.png`
Then read the sheet. The camera should glide between moments, never jump.

- [ ] **Step 5: Commit**

```bash
git add contrib/promo/index.html contrib/promo/render.cjs contrib/promo/audio.js
git commit -m "The promotional video's composer draws a timeline

index.html draws any moment of the director's timeline over the app's
frames: the window on the camera, a spotlight on what matters, a
pointer that glides and presses, keycaps that spring in, captions
revealed word by word, and the intro and outro cards on the beat.
render.cjs screenshots each frame in headless Chrome for ffmpeg. With
--final it renders four frames for each one and ffmpeg blends them.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Phase 3: the sound

### Task 8: music and effects

**Files:**
- Modify: `contrib/promo/audio.js` (replace the placeholder)
- Modify: `contrib/promo/render.cjs` (the sound, and its mix into the MP4)

- [ ] **Step 1: Write the synthesizer**

`contrib/promo/audio.js`:

```js
// The sound of a promotional video: music in the timeline's tempo and the
// effects on its events, rendered offline (no clock, no speakers) and
// returned as a 16-bit WAV in base64. index.html loads it after
// director.js; render.cjs calls renderAudio(timeline). Seeded: a timeline
// always sounds the same.

(function () {
  "use strict";

  const RATE = 48000;
  // The music's level, which the effects duck.
  const MUSIC = 0.55;
  // D major: Dmaj7, Bm7, Gmaj7, A6, a bar each, as MIDI notes; the outro
  // rings on Dmaj9.
  const CHORDS = [[50, 54, 57, 61], [47, 50, 54, 57], [43, 47, 50, 54], [45, 49, 52, 54]];
  const LAST = [50, 57, 61, 64];
  const hz = note => 440 * Math.pow(2, (note - 69) / 12);

  async function renderAudio(timeline) {
    const ctx = new OfflineAudioContext(2, Math.ceil(timeline.duration * RATE), RATE);
    const random = Director.seeded(`${timeline.title} sound`);
    const noise = ctx.createBuffer(1, RATE, RATE);
    const white = noise.getChannelData(0);
    for (let i = 0; i < white.length; i++) white[i] = random() * 2 - 1;
    const tools = { ctx, noise, random };

    const bus = ctx.createDynamicsCompressor();
    bus.threshold.value = -12;
    bus.ratio.value = 3;
    bus.attack.value = 0.005;
    bus.release.value = 0.2;
    bus.connect(ctx.destination);
    const room = ctx.createConvolver();
    room.buffer = impulse(ctx, 2.6, random);
    const wet = ctx.createGain();
    wet.gain.value = 0.3;
    room.connect(wet).connect(bus);

    const music = ctx.createGain();
    music.gain.value = MUSIC;
    music.connect(bus);
    music.connect(room);
    const fx = ctx.createGain();
    fx.gain.value = 0.9;
    fx.connect(bus);
    const fxRoom = ctx.createGain();
    fxRoom.gain.value = 0.15;
    fx.connect(fxRoom).connect(room);

    band(tools, music, timeline);
    for (const sound of timeline.sounds) {
      EFFECTS[sound.kind](tools, fx, sound.at, sound.gain);
      duck(music.gain, sound.at, sound.kind);
    }
    return wav(await ctx.startRendering());
  }

  // The music ducks a little under each effect, and more under the big ones.
  function duck(gain, at, kind) {
    const depth = kind === "key" ? 0.92 : 0.7;
    gain.setTargetAtTime(MUSIC * depth, at, 0.01);
    gain.setTargetAtTime(MUSIC, at + 0.06, 0.15);
  }

  // A room: noise that dies away over `seconds`, each side its own.
  function impulse(ctx, seconds, random) {
    const length = Math.round(seconds * RATE);
    const buffer = ctx.createBuffer(2, length, RATE);
    for (let c = 0; c < 2; c++) {
      const data = buffer.getChannelData(c);
      for (let i = 0; i < length; i++) data[i] = (random() * 2 - 1) * Math.pow(1 - i / length, 3);
    }
    return buffer;
  }

  // The pad alone under the intro; bass, kick, hats and the arpeggio from
  // the first scene; the last chord ringing out over the outro.
  function band(tools, into, timeline) {
    const { ctx, random } = tools;
    const { beat, bar, duration, outro } = timeline;
    const bed = ctx.createGain();
    bed.connect(into);
    bed.gain.setValueAtTime(0, 0);
    bed.gain.linearRampToValueAtTime(1, timeline.intro.to * 0.8);
    bed.gain.setValueAtTime(1, duration - bar);
    bed.gain.linearRampToValueAtTime(0, duration - 0.05);
    for (let at = 0; at < duration - 1e-6; at += bar) {
      if (at >= outro - 1e-6) {
        pad(ctx, bed, LAST, at, duration - at);
        break;
      }
      const chord = CHORDS[Math.round(at / bar) % CHORDS.length];
      pad(ctx, bed, chord, at, bar);
      if (at < timeline.intro.to - 1e-6) continue;
      bass(ctx, bed, chord[0] - 12, at, bar);
      arp(ctx, bed, chord, at, beat);
      for (let k = 0; k < 4; k++) {
        if (k % 2 === 0) kick(ctx, bed, at + k * beat);
        hat(tools, bed, at + k * beat + beat / 2, random());
      }
    }
  }

  // Detuned saws through a slow lowpass: warm, wide, quiet.
  function pad(ctx, into, chord, at, length) {
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.Q.value = 0.7;
    filter.frequency.setValueAtTime(700, at);
    filter.frequency.linearRampToValueAtTime(1500, at + length * 0.6);
    filter.frequency.linearRampToValueAtTime(900, at + length);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.05, at + 0.35);
    gain.gain.setValueAtTime(0.05, at + length - 0.1);
    gain.gain.linearRampToValueAtTime(0, at + length + 0.6);
    filter.connect(gain).connect(into);
    for (const note of chord) {
      for (const cents of [-7, 7]) {
        const osc = ctx.createOscillator();
        osc.type = "sawtooth";
        osc.frequency.value = hz(note);
        osc.detune.value = cents;
        osc.connect(filter);
        osc.start(at);
        osc.stop(at + length + 0.7);
      }
    }
  }

  function bass(ctx, into, note, at, length) {
    const osc = ctx.createOscillator();
    osc.type = "sine";
    osc.frequency.value = hz(note);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0, at);
    gain.gain.linearRampToValueAtTime(0.18, at + 0.02);
    gain.gain.setValueAtTime(0.18, at + length - 0.08);
    gain.gain.linearRampToValueAtTime(0, at + length);
    osc.connect(gain).connect(into);
    osc.start(at);
    osc.stop(at + length + 0.01);
  }

  // A felt kick: a sine falling in pitch, gone in a third of a second.
  function kick(ctx, into, at) {
    const osc = ctx.createOscillator();
    const gain = ctx.createGain();
    osc.frequency.setValueAtTime(110, at);
    osc.frequency.exponentialRampToValueAtTime(42, at + 0.12);
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.5, at + 0.004);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.38);
    osc.connect(gain).connect(into);
    osc.start(at);
    osc.stop(at + 0.4);
  }

  function hat({ ctx, noise }, into, at, random) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "highpass";
    filter.frequency.value = 7000;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.035 + 0.02 * random, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.05);
    source.connect(filter).connect(gain).connect(into);
    source.start(at, random * 0.5, 0.06);
  }

  // Eighths over the chord, an octave up, left and right in turn.
  function arp(ctx, into, chord, at, beat) {
    [0, 1, 2, 3, 1, 2, 3, 2].forEach((n, i) =>
      pluck(ctx, into, hz(chord[n] + 12), at + (i * beat) / 2, i % 2 ? 0.35 : -0.35));
  }

  function pluck(ctx, into, freq, at, pan) {
    const osc = ctx.createOscillator();
    osc.type = "triangle";
    osc.frequency.value = freq;
    const filter = ctx.createBiquadFilter();
    filter.type = "lowpass";
    filter.frequency.setValueAtTime(4000, at);
    filter.frequency.exponentialRampToValueAtTime(600, at + 0.25);
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(0.0001, at);
    gain.gain.exponentialRampToValueAtTime(0.07, at + 0.005);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.4);
    const panner = ctx.createStereoPanner();
    panner.pan.value = pan;
    osc.connect(filter).connect(gain).connect(panner).connect(into);
    osc.start(at);
    osc.stop(at + 0.45);
  }

  // A short burst of noise around `freq`: half of a mouse click.
  function tap({ ctx, noise, random }, into, at, freq, level) {
    const source = ctx.createBufferSource();
    source.buffer = noise;
    const filter = ctx.createBiquadFilter();
    filter.type = "bandpass";
    filter.frequency.value = freq;
    filter.Q.value = 2;
    const gain = ctx.createGain();
    gain.gain.setValueAtTime(level, at);
    gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.012);
    source.connect(filter).connect(gain).connect(into);
    source.start(at, random() * 0.9, 0.02);
  }

  const EFFECTS = {
    // Each key a little different: a click of noise and a body under it.
    key({ ctx, noise, random }, into, at, gain) {
      const source = ctx.createBufferSource();
      source.buffer = noise;
      const filter = ctx.createBiquadFilter();
      filter.type = "bandpass";
      filter.frequency.value = 2500 + 1500 * random();
      filter.Q.value = 1.2;
      const level = ctx.createGain();
      level.gain.setValueAtTime(0.22 * gain, at);
      level.gain.exponentialRampToValueAtTime(0.0001, at + 0.03);
      const panner = ctx.createStereoPanner();
      panner.pan.value = (random() - 0.5) * 0.3;
      source.connect(filter).connect(level).connect(panner).connect(into);
      source.start(at, random() * 0.9, 0.04);
      const body = ctx.createOscillator();
      body.frequency.value = 180 + 60 * random();
      const thud = ctx.createGain();
      thud.gain.setValueAtTime(0.12 * gain, at);
      thud.gain.exponentialRampToValueAtTime(0.0001, at + 0.025);
      body.connect(thud).connect(into);
      body.start(at);
      body.stop(at + 0.03);
    },
    // Down, and up again.
    click(tools, into, at, gain) {
      tap(tools, into, at, 1300, 0.3 * gain);
      tap(tools, into, at + 0.07, 1800, 0.18 * gain);
    },
    // Air sweeping past, from one side to the other: longer for a longer move.
    whoosh({ ctx, noise, random }, into, at, gain) {
      const length = 0.4 + 0.35 * gain;
      const source = ctx.createBufferSource();
      source.buffer = noise;
      source.loop = true;
      const filter = ctx.createBiquadFilter();
      filter.type = "bandpass";
      filter.Q.value = 0.9;
      filter.frequency.setValueAtTime(300, at);
      filter.frequency.exponentialRampToValueAtTime(2200, at + length * 0.55);
      filter.frequency.exponentialRampToValueAtTime(500, at + length);
      const level = ctx.createGain();
      level.gain.setValueAtTime(0.0001, at);
      level.gain.exponentialRampToValueAtTime(0.16 * gain, at + length * 0.5);
      level.gain.exponentialRampToValueAtTime(0.0001, at + length);
      const panner = ctx.createStereoPanner();
      const side = random() < 0.5 ? -1 : 1;
      panner.pan.setValueAtTime(-0.4 * side, at);
      panner.pan.linearRampToValueAtTime(0.4 * side, at + length);
      source.connect(filter).connect(level).connect(panner).connect(into);
      source.start(at, random() * 0.5);
      source.stop(at + length + 0.05);
    },
    // A low, soft note under a keycap.
    tick({ ctx }, into, at, gain) {
      for (const [times, level] of [[1, 0.14], [2, 0.05], [3, 0.025]]) {
        const osc = ctx.createOscillator();
        osc.frequency.value = 220 * times;
        const amp = ctx.createGain();
        amp.gain.setValueAtTime(0.0001, at);
        amp.gain.exponentialRampToValueAtTime(level * gain, at + 0.004);
        amp.gain.exponentialRampToValueAtTime(0.0001, at + 0.15);
        osc.connect(amp).connect(into);
        osc.start(at);
        osc.stop(at + 0.16);
      }
    },
    // An FM bell on D, and its octave below.
    bell({ ctx }, into, at, gain) {
      const f = hz(86);
      const carrier = ctx.createOscillator();
      carrier.frequency.value = f;
      const modulator = ctx.createOscillator();
      modulator.frequency.value = f * 3.5;
      const depth = ctx.createGain();
      depth.gain.setValueAtTime(f * 2, at);
      depth.gain.exponentialRampToValueAtTime(1, at + 1.5);
      modulator.connect(depth).connect(carrier.frequency);
      const level = ctx.createGain();
      level.gain.setValueAtTime(0.0001, at);
      level.gain.exponentialRampToValueAtTime(0.18 * gain, at + 0.005);
      level.gain.exponentialRampToValueAtTime(0.0001, at + 2.8);
      carrier.connect(level).connect(into);
      const low = ctx.createOscillator();
      low.frequency.value = hz(74);
      const lowLevel = ctx.createGain();
      lowLevel.gain.setValueAtTime(0.0001, at);
      lowLevel.gain.exponentialRampToValueAtTime(0.08 * gain, at + 0.01);
      lowLevel.gain.exponentialRampToValueAtTime(0.0001, at + 2.5);
      low.connect(lowLevel).connect(into);
      for (const osc of [carrier, modulator, low]) {
        osc.start(at);
        osc.stop(at + 3);
      }
    },
  };

  // The rendered sound as a 16-bit PCM WAV, in base64.
  function wav(buffer) {
    const channels = buffer.numberOfChannels, frames = buffer.length, size = frames * channels * 2;
    const view = new DataView(new ArrayBuffer(44 + size));
    const text = (at, s) => [...s].forEach((c, i) => view.setUint8(at + i, c.charCodeAt(0)));
    text(0, "RIFF");
    view.setUint32(4, 36 + size, true);
    text(8, "WAVE");
    text(12, "fmt ");
    view.setUint32(16, 16, true);
    view.setUint16(20, 1, true);
    view.setUint16(22, channels, true);
    view.setUint32(24, RATE, true);
    view.setUint32(28, RATE * channels * 2, true);
    view.setUint16(32, channels * 2, true);
    view.setUint16(34, 16, true);
    text(36, "data");
    view.setUint32(40, size, true);
    const data = Array.from({ length: channels }, (_, c) => buffer.getChannelData(c));
    for (let i = 0, at = 44; i < frames; i++) {
      for (let c = 0; c < channels; c++, at += 2) {
        view.setInt16(at, Math.max(-1, Math.min(1, data[c][i])) * 32767, true);
      }
    }
    const bytes = new Uint8Array(view.buffer);
    let binary = "";
    for (let i = 0; i < bytes.length; i += 0x8000) {
      binary += String.fromCharCode(...bytes.subarray(i, i + 0x8000));
    }
    return btoa(binary);
  }

  globalThis.renderAudio = renderAudio;
})();
```

- [ ] **Step 2: Mix it into the video**

In `render.cjs`, after the `--still` branch and before `rate`, render the
sound:

```js
  const audio = path.join(work, "audio.wav");
  const sound = await page.evaluate(timeline => renderAudio(timeline), timeline);
  fs.writeFileSync(audio, Buffer.from(sound, "base64"));
```

Then make ffmpeg take it as a second input, normalized to −14 LUFS:

```js
      "-f", "image2pipe", "-framerate", String(rate), "-c:v", "png", "-i", "-",
      "-i", audio,
      "-map", "0:v", "-map", "1:a",
      ...(final ? ["-vf", `tmix=frames=4,fps=${timeline.fps}`] : []),
      "-c:v", "libx264", "-preset", "slow", "-crf", "16", "-pix_fmt", "yuv420p",
      "-af", "loudnorm=I=-14:TP=-1.5:LRA=11", "-ar", "48000", "-c:a", "aac", "-b:a", "192k",
      "-shortest", "-movflags", "+faststart", video,
```

Update the header comment: "...its sound by audio.js, encoded and mixed by
ffmpeg."

- [ ] **Step 3: Render the smoke video, and check its sound**

Run: `contrib/promo/make contrib/promo/scenarios/smoke.toml --reuse`

Loudness:
`ffmpeg -nostats -i target/promo/smoke.mp4 -af ebur128=framelog=quiet -f null - 2>&1 | grep -A3 'Integrated loudness'`
Expected: `I:` between −15 and −13 LUFS.

The waveform:
`ffmpeg -v error -y -i target/promo/smoke.mp4 -filter_complex "showwavespic=s=1600x300:split_channels=1" -frames:v 1 target/promo/smoke-wave.png`
Read the PNG. Expected:
- quiet under the intro;
- the beat clear from the first scene, with spikes on the clicks and keys;
- a long tail over the outro;
- no flat-topped clipping.

Determinism:
`shasum target/promo/smoke/audio.wav; contrib/promo/make contrib/promo/scenarios/smoke.toml --reuse >/dev/null; shasum target/promo/smoke/audio.wav`
Expected: the same hash twice.

Have the person listen to `target/promo/smoke.mp4`, since you cannot.
Note what they say for Task 9.

- [ ] **Step 4: Commit**

```bash
git add contrib/promo/audio.js contrib/promo/render.cjs
git commit -m "The promotional video has music and sound

audio.js renders the sound offline in the same Chrome, from the
timeline, seeded so it never changes. The music is a pad, bass,
arpeggio, kick and hats in the scenario's tempo, opening sparse and
resolving on the outro. The effects sit on the events: keys, clicks,
whooshes for the camera's moves, a tick under each keycap, a bell at the
end. ffmpeg normalizes the mix to -14 LUFS and muxes it as AAC.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Phase 4: the videos, and the checks

### Task 9: `overview` and `filtering`

**Files:**
- Create: `contrib/promo/scenarios/overview.toml`
- Create: `contrib/promo/scenarios/filtering.toml`
- Modify: whatever tuning the renders call for, in `director.js`,
  `index.html` or `audio.js`

- [ ] **Step 1: Write the overview**

The old branch's storyboard, on the Bookshop:

```toml
# What Tabletist is, in four scenes: the video for a release's posts.
title = "Tabletist"
tempo = 112

[[scene]]
caption = "Open a table. It's just there."
steps = [
  { open = "books" },
  { click = "Lanterns" },
  { command = "move_row", key = "down", times = 3 },
]

[[scene]]
caption = "Filter without writing SQL."
steps = [
  { command = "filter_bar" },
  { click = "Filter column" },
  { click = "title" },
  { click = "Filter operator" },
  { click = "contains" },
  { click = "Value" },
  { type = "the" },
  { press = "enter" },
  { hold = 0.6 },
]

[[scene]]
caption = "Edit. Review. Then save."
steps = [
  { click = "The Quiet Ledger" },
  { command = "edit_cell" },
  { type = " (2nd ed.)" },
  { press = "enter" },
  { command = "review_sql" },
  { hold = 0.8 },
]

[[scene]]
caption = "Or just write the SQL."
steps = [
  { command = "new_sql_tab" },
  { type = "SELECT title, price\n  FROM books\n WHERE in_print\n ORDER BY price DESC;" },
  { command = "run_statement" },
  { hold = 1.0 },
]
```

- [ ] **Step 2: Play it until every step does what it says**

Run: `contrib/promo/make contrib/promo/scenarios/overview.toml --frames-only`

A step that fails names its scene and step. Read the frames written so far
to see why. These are likely, and here is what to do about each:
- **The filter bar's row starts on the table's first column (`id`) with
  `=`** (`ToggleFilterBar` in `src/app.rs`). That is why the scene clicks
  the "Filter column" and "Filter operator" dropdowns, both AccessKit
  labels from `src/ui/filter_bar.rs`, and picks `title` and `contains`.
  `Value` is the field's painted hint.
- **`click = "title"` hits the grid's column header instead of the
  menu's item.** Make `labelled` in `src/promo/runner.rs` take the last
  node with that label rather than the first: the open menu is drawn last.
  Add a line to its doc comment saying why.
- **A later scene clicks a row the filter hides.** Every row clicked after
  the filter must contain "the" (here "The Quiet Ledger").
- **Typing in the cell replaces the title instead of adding to it**
  (`edit_cell` selects it). Type the whole new title instead.
- **The SQL editor's completion list takes a newline.** Close it before
  the line ends with `{ press = "esc" }`, or end that line's word with a
  space.
- **`review_sql` has no key on macOS.** Use `{ click = "Review SQL" }`
  instead (the scenario's parser says so if the key is missing).

Change the scenario, never the app, unless the app is wrong. Keep the
captions' promises: the filter scene must not write SQL.

- [ ] **Step 3: Write `filtering.toml`, one feature explained**

```toml
# One feature, explained: filtering a table without SQL.
title = "Filter without SQL"
tempo = 104

[[scene]]
caption = "Every table opens ready to filter."
steps = [
  { open = "books" },
  { command = "filter_bar" },
]

[[scene]]
caption = "Pick a column, type what you're after."
steps = [
  { click = "Filter column" },
  { click = "title" },
  { click = "Filter operator" },
  { click = "contains" },
  { click = "Value" },
  { type = "the" },
  { press = "enter" },
  { hold = 0.8 },
]

[[scene]]
caption = "The rows narrow as you go. No SQL written."
steps = [
  { click = "The Salt Road" },
  { command = "move_row", key = "up" },
  { hold = 0.8 },
]
```

Make it play as in Step 2. The second scene's steps follow from what the
first showed of the filter bar.

- [ ] **Step 4: Render both, final quality, and look**

Run: `contrib/promo/make contrib/promo/scenarios/overview.toml --final && contrib/promo/make contrib/promo/scenarios/filtering.toml --final`

For each:
- a contact sheet (as in Task 7, Step 4);
- stills at the middle of each scene (`timeline.json`'s `scenes`) with
  `--reuse --still`;
- the loudness check (as in Task 8, Step 3).

Read them. Tune whatever reads as amateur, in the director or the composer
and not per scenario:
- a camera that zooms on something too small (raise the 0.5 fill, or
  lower `ZOOM[1]`);
- a caption hidden by the zoom;
- a keycap that covers the action;
- a scene that drags (lower `SETTLE`).

Rerun the director's tests after any tuning: `node --test contrib/promo/director.test.js`.

- [ ] **Step 5: Hand the videos to the person**

Hand `target/promo/overview.mp4` and `target/promo/filtering.mp4` to the
person (as a file they can open, with whatever tool sends one), and ask for
their verdict on picture and sound. Fold in what
they say, then commit.

```bash
git add contrib/promo/scenarios/overview.toml contrib/promo/scenarios/filtering.toml contrib/promo/director.js contrib/promo/index.html contrib/promo/audio.js
git commit -m "Two promotional videos: the overview and filtering

The overview is the old branch's storyboard, played on the Bookshop.
Filtering explains one feature in three scenes. The tuning their renders
asked for is in the director and the composer, for every scenario.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

### Task 10: the spec as built, and the checks

**Files:**
- Modify: `docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md`

- [ ] **Step 1: Bring the spec to what was built**

Make these changes:
- **The runner.** `events.json` has its captions in `scenes` and the
  kinds `start` and `point`. Replace the spec's `caption` kind with that.
- **The composer.** `--final` renders every frame four times and ffmpeg's
  `tmix` blends them. It is not only fast moves, and a still moment comes
  out unchanged.
- **The command.** `puppeteer-core` is pinned in `make`.
- **Anything Task 9's tuning changed:** the step list, the starting
  state, the numbers.

- [ ] **Step 2: Run every check**

Run:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo clippy --locked --all-targets --features shots -- -D warnings
cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
node --test contrib/promo/director.test.js
```

Expected: all clean and passing. The `film` test is ignored in the suite.

- [ ] **Step 3: Commit**

```bash
git add docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md
git commit -m "The promotional video's spec says what was built

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

- [ ] **Step 4: Report**

Tell the person:
- what was rendered and where;
- that the renders ran on macOS only, and that CI runs only the Rust
  checks, not `node --test` or a render;
- that `claude/promo-video`, the old branch, is now superseded and can be
  deleted when they are ready (do not delete it yourself).
