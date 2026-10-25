//! A scenario, as its file says it: what a promotional video shows and
//! what it says. It has no times: `contrib/promo/director.js` decides
//! them. The format: contrib/promo/README.md.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::keymap::{self, Command, Keymap, Kind, Layout, Mods, Press, Spelled, Stroke};

/// The look a video is drawn in: the macOS one, the only one so far.
pub const LAYOUT: Layout = Layout::Mac;

#[derive(Debug)]
pub struct Scenario {
    /// What the end card says.
    pub title: String,
    pub database: Database,
    /// The music's beats per minute; a scene ends on a half bar.
    pub tempo: u32,
    /// The voice that says the lines, as macOS `say -v` names it.
    pub voice: String,
    /// The line said over the intro card, and the one over the outro's.
    pub intro: Option<String>,
    pub outro: Option<String>,
    pub scenes: Vec<Scene>,
}

#[derive(Debug, PartialEq)]
pub enum Database {
    /// The Bookshop fixture, written afresh for each run.
    Bookshop,
    /// A SQLite file, opened read-only.
    File(PathBuf),
}

#[derive(Debug, PartialEq)]
pub enum Scene {
    /// The app, played: a caption under it, the line the voice says, and
    /// the steps that move it.
    Shot {
        caption: Option<String>,
        say: Option<String>,
        steps: Vec<Step>,
    },
    /// A feature named on a card of its own, the app away: a headline, a
    /// line under it, and the line the voice says.
    Slide {
        title: String,
        text: Option<String>,
        say: Option<String>,
    },
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

/// The tempo when the scenario names none, and the ones it may name.
const TEMPO: u32 = 128;
const TEMPOS: std::ops::RangeInclusive<u32> = 80..=160;

/// The voice when the scenario names none: installed with macOS, in its
/// Premium quality from System Settings.
const VOICE: &str = "Ava (Premium)";

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
    let voice = file.voice.unwrap_or_else(|| VOICE.into());
    if voice.trim().is_empty() {
        return Err("voice is empty".into());
    }
    let card = |card: Option<RawCard>, name: &str| match card {
        Some(RawCard { say }) if say.trim().is_empty() => Err(format!("the {name}'s say is empty")),
        Some(RawCard { say }) => Ok(Some(say)),
        None => Ok(None),
    };
    let intro = card(file.intro, "intro")?;
    let outro = card(file.outro, "outro")?;
    let database = match file.database.as_deref() {
        None | Some("bookshop") => Database::Bookshop,
        Some(path) => Database::File(dir.join(path)),
    };
    let scenes = file
        .scenes
        .into_iter()
        .enumerate()
        .map(|(index, scene)| {
            scene
                .read()
                .map_err(|error| format!("scene {}{error}", index + 1))
        })
        .collect::<Result<_, String>>()?;
    Ok(Scenario {
        title: file.title,
        database,
        tempo: file.tempo,
        voice,
        intro,
        outro,
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
    voice: Option<String>,
    intro: Option<RawCard>,
    outro: Option<RawCard>,
    #[serde(rename = "scene", default)]
    scenes: Vec<RawScene>,
}

/// The intro or the outro card, which only takes a line to say.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawCard {
    say: String,
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
    slide: Option<String>,
    text: Option<String>,
    say: Option<String>,
}

impl RawScene {
    /// The scene, or what is wrong with it, the error after its place
    /// (": ..." for the scene, ", step N: ..." for a step).
    fn read(self) -> Result<Scene, String> {
        let RawScene {
            caption,
            steps,
            slide,
            text,
            say,
        } = self;
        if say.as_deref().is_some_and(|say| say.trim().is_empty()) {
            return Err(": say is empty".into());
        }
        let Some(title) = slide else {
            if text.is_some() {
                return Err(": only a slide has text".into());
            }
            if steps.is_empty() {
                return Err(": a scene needs steps".into());
            }
            let steps = steps
                .into_iter()
                .enumerate()
                .map(|(step, raw)| {
                    raw.read()
                        .map_err(|error| format!(", step {}: {error}", step + 1))
                })
                .collect::<Result<_, _>>()?;
            return Ok(Scene::Shot {
                caption,
                say,
                steps,
            });
        };
        if !steps.is_empty() || caption.is_some() {
            return Err(": a slide has no steps and no caption".into());
        }
        if title.trim().is_empty() {
            return Err(": a slide needs a title".into());
        }
        if text.as_deref().is_some_and(|text| text.trim().is_empty()) {
            return Err(": a slide's text is empty".into());
        }
        Ok(Scene::Slide { title, text, say })
    }
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
            format!(
                "{key:?} is not a key of {id}: its keys are {}",
                own.join(", ")
            )
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

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Database, Keys, Scene, Step, load, parse};
    use crate::keymap::{Command, Mods};

    fn read(text: &str) -> Result<super::Scenario, String> {
        parse(text, Path::new("/scenarios"))
    }

    /// A scene of the app played: its caption and its steps.
    fn shot(scene: &Scene) -> (Option<&str>, &[Step]) {
        let Scene::Shot { caption, steps, .. } = scene else {
            panic!("not a shot: {scene:?}");
        };
        (caption.as_deref(), steps)
    }

    /// The error a scenario with `step` as its one step gives.
    fn refused(step: &str) -> String {
        read(&format!("title = \"T\"\n[[scene]]\nsteps = [{step}]\n")).unwrap_err()
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
        let (caption, steps) = shot(&scenario.scenes[0]);
        assert_eq!(caption, Some("Open a table."));
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
            steps,
            [
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
        assert_eq!(shot(&scenario.scenes[1]).0, None);
    }

    #[test]
    fn a_command_without_a_key_takes_its_first() {
        let scenario =
            read("title = \"T\"\n[[scene]]\nsteps = [{ command = \"filter_bar\" }]\n").unwrap();
        let Step::Command {
            command,
            keys,
            times,
        } = &shot(&scenario.scenes[0]).1[0]
        else {
            panic!("not a command: {:?}", scenario.scenes[0]);
        };
        assert_eq!(*command, Command::FilterBar);
        assert_eq!(*times, 1);
        assert!(
            keys.mods.cmd,
            "the filter bar's first key on macOS holds Cmd"
        );
        assert_eq!(keys.key, egui::Key::F);
    }

    #[test]
    fn the_tempo_is_128_unless_the_scenario_says() {
        let scenario = read("title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n").unwrap();
        assert_eq!(scenario.tempo, 128);
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
        std::fs::write(
            &path,
            "title = \"Demo\"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
        )
        .unwrap();
        assert_eq!(load(&path).unwrap().title, "Demo");
        std::fs::write(&path, "title = \"Demo\"\n").unwrap();
        let error = load(&path).unwrap_err();
        assert!(error.starts_with(&path.display().to_string()), "{error}");
    }

    #[test]
    fn what_a_scenario_cannot_say_is_refused_with_where() {
        for (step, said) in [
            (
                "{ command = \"no_such\" }",
                "no command is called \"no_such\"",
            ),
            (
                "{ command = \"reopen_closed_tab\" }",
                "reopen_closed_tab is not built yet",
            ),
            (
                "{ command = \"filter_bar\", key = \"f5\" }",
                "\"f5\" is not a key of filter_bar",
            ),
            ("{ open = \"books\", click = \"x\" }", "a step is one of"),
            (
                "{ key = \"down\", press = \"down\" }",
                "only a command takes a key",
            ),
            (
                "{ open = \"books\", times = 2 }",
                "only a command or a press takes times",
            ),
            ("{ press = \"down\", times = 0 }", "times is at least 1"),
            ("{ press = \"g g\" }", "is not one press of a key"),
            ("{ press = \"nonsense\" }", "nonsense"),
            ("{ type = \"\" }", "type needs text"),
            ("{ hold = 0 }", "hold 0 is not above 0"),
            ("{ wait = 1 }", "unknown field"),
        ] {
            let error = refused(step);
            assert!(
                error.starts_with("scene 1, step 1: ") || said == "unknown field",
                "{step}: {error}"
            );
            assert!(error.contains(said), "{step}: {error}");
        }
    }

    #[test]
    fn a_scenario_needs_scenes_steps_and_a_tempo_it_can_play() {
        for (text, said) in [
            ("title = \"T\"\n", "a scenario needs a [[scene]]"),
            (
                "title = \"T\"\n[[scene]]\ncaption = \"x\"\n",
                "scene 1: a scene needs steps",
            ),
            (
                "title = \"T\"\ntempo = 200\n[[scene]]\nsteps = [{ hold = 1 }]\n",
                "tempo 200 is not between 80 and 160",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \"S\"\nsteps = [{ hold = 1 }]\n",
                "scene 1: a slide has no steps and no caption",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \"S\"\ncaption = \"x\"\n",
                "scene 1: a slide has no steps and no caption",
            ),
            (
                "title = \"T\"\n[[scene]]\ntext = \"x\"\nsteps = [{ hold = 1 }]\n",
                "scene 1: only a slide has text",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \" \"\n",
                "scene 1: a slide needs a title",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \"S\"\ntext = \"\"\n",
                "scene 1: a slide's text is empty",
            ),
        ] {
            let error = read(text).unwrap_err();
            assert!(error.contains(said), "{text}: {error}");
        }
    }

    #[test]
    fn a_scene_may_say_a_line_in_the_voice_the_scenario_names() {
        let scenario = read(
            r#"
title = "T"
voice = "Samantha"

[intro]
say = "Here is what is new."

[[scene]]
caption = "Open a table."
say = "Open a table, and it's just there."
steps = [{ hold = 1 }]

[[scene]]
slide = "Keyboard first."
say = "Every command has a key."

[[scene]]
steps = [{ hold = 1 }]

[outro]
say = "Download it today."
"#,
        )
        .unwrap();
        assert_eq!(scenario.voice, "Samantha");
        assert_eq!(scenario.intro.as_deref(), Some("Here is what is new."));
        assert_eq!(scenario.outro.as_deref(), Some("Download it today."));
        let Scene::Shot { say, .. } = &scenario.scenes[0] else {
            panic!("not a shot: {:?}", scenario.scenes[0]);
        };
        assert_eq!(say.as_deref(), Some("Open a table, and it's just there."));
        assert_eq!(
            scenario.scenes[1],
            Scene::Slide {
                title: "Keyboard first.".into(),
                text: None,
                say: Some("Every command has a key.".into()),
            }
        );
        let Scene::Shot { say, .. } = &scenario.scenes[2] else {
            panic!("not a shot: {:?}", scenario.scenes[2]);
        };
        assert_eq!(*say, None);
    }

    #[test]
    fn the_voice_is_ava_unless_the_scenario_says() {
        let scenario = read("title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n").unwrap();
        assert_eq!(scenario.voice, "Ava (Premium)");
        assert_eq!(scenario.intro, None);
        assert_eq!(scenario.outro, None);
    }

    #[test]
    fn a_line_to_say_and_a_voice_are_not_empty() {
        for (text, said) in [
            (
                "title = \"T\"\n[[scene]]\nsay = \"\"\nsteps = [{ hold = 1 }]\n",
                "scene 1: say is empty",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \"S\"\nsay = \"  \"\n",
                "scene 1: say is empty",
            ),
            (
                "title = \"T\"\n[intro]\nsay = \" \"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
                "the intro's say is empty",
            ),
            (
                "title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n[outro]\nsay = \"\"\n",
                "the outro's say is empty",
            ),
            (
                "title = \"T\"\nvoice = \"\"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
                "voice is empty",
            ),
            (
                "title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n[outro]\ncaption = \"x\"\n",
                "unknown field",
            ),
        ] {
            let error = read(text).unwrap_err();
            assert!(error.contains(said), "{text}: {error}");
        }
    }

    #[test]
    fn a_slide_is_a_scene_of_its_own() {
        let scenario = read(
            r#"
title = "T"

[[scene]]
slide = "Keyboard first."
text = "Every command has a key."

[[scene]]
slide = "Small."

[[scene]]
steps = [{ hold = 1 }]
"#,
        )
        .unwrap();
        assert_eq!(
            scenario.scenes[0],
            Scene::Slide {
                title: "Keyboard first.".into(),
                text: Some("Every command has a key.".into()),
                say: None,
            }
        );
        assert_eq!(
            scenario.scenes[1],
            Scene::Slide {
                title: "Small.".into(),
                text: None,
                say: None,
            }
        );
        assert_eq!(shot(&scenario.scenes[2]).1, [Step::Hold(1.0)]);
    }
}
