//! A scenario, as its file says it: what a promotional video shows and
//! what it says. It has no times: `contrib/promo/director.js` decides
//! them. The format: contrib/promo/README.md.

use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::env::Environment;
use crate::keymap::{self, Command, Keymap, Kind, Layout, Spelled, Stroke};

/// The look a video is filmed in, and so the keys its steps press.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Look {
    /// macOS, in the light theme, with its title bar.
    #[default]
    Macos,
    /// Omarchy, in Tokyo Night: vim's keys and the `:` prompt.
    Omarchy,
}

impl Look {
    /// The keymap's layout for the look.
    pub fn layout(self) -> Layout {
        match self {
            Self::Macos => Layout::Mac,
            Self::Omarchy => Layout::Omarchy,
        }
    }

    /// The look as a scenario names it.
    pub fn name(self) -> &'static str {
        match self {
            Self::Macos => "macos",
            Self::Omarchy => "omarchy",
        }
    }
}

#[derive(Debug)]
pub struct Scenario {
    /// What the end card says.
    pub title: String,
    pub database: Database,
    pub look: Look,
    /// The music's beats per minute; a scene ends on a half bar.
    pub tempo: u32,
    /// The voice that says the lines, as macOS `say -v` names it.
    pub voice: String,
    /// The intro card and the outro card.
    pub intro: Intro,
    pub outro: Outro,
    /// The other saved connections to the database, none of them
    /// connected at the start.
    pub connections: Vec<Saved>,
    /// SQL that someone else ran on the server once the video's rows were
    /// loaded: a save finds what it changed.
    pub meanwhile: Option<String>,
    pub scenes: Vec<Scene>,
}

/// Another saved connection to the scenario's database, for a scene to
/// switch to: its name and its environment. It opens writable.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Saved {
    pub name: String,
    pub environment: Environment,
}

/// The intro card: the line said over it, the words that replace its
/// title and its tagline, and how many beats it lasts, if it is fixed.
#[derive(Debug, Default, PartialEq)]
pub struct Intro {
    pub say: Option<String>,
    pub title: Option<String>,
    pub text: Option<String>,
    pub beats: Option<f64>,
}

/// The outro card: the line said over it, and how many beats it lasts.
#[derive(Debug, Default, PartialEq)]
pub struct Outro {
    pub say: Option<String>,
    pub beats: Option<f64>,
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
    /// the steps that move it. With `beats` it is a cut of exactly that
    /// many beats, and its `label` names the feature over it.
    Shot {
        caption: Option<String>,
        say: Option<String>,
        steps: Vec<Step>,
        label: Option<String>,
        beats: Option<f64>,
    },
    /// A feature named on a card of its own, the app away: a headline, a
    /// line under it, and the line the voice says. With `beats` it is a
    /// flash card of exactly that many beats.
    Slide {
        title: String,
        text: Option<String>,
        say: Option<String>,
        beats: Option<f64>,
    },
}

/// Keys as the keymap spells them, and what they press one after another:
/// a key with what is held, or a character typed (a vim letter). A line
/// of the `:` prompt is its colon, its letters and Enter.
#[derive(Clone, Debug, PartialEq)]
pub struct Keys {
    pub spelled: &'static str,
    pub strokes: Vec<Stroke>,
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
    /// Omarchy switches to the theme of this name, as Omarchy shows it
    /// (`Retro 82`): the app recolours as it follows the desktop.
    Omarchy(String),
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
    let intro = match file.intro {
        None => Intro::default(),
        Some(RawIntro {
            say,
            title,
            text,
            beats,
        }) => Intro {
            say: words(say, "the intro's say")?,
            title: words(title, "the intro's title")?,
            text: words(text, "the intro's text")?,
            beats: length(beats).map_err(|error| format!("the intro's {error}"))?,
        },
    };
    let outro = match file.outro {
        None => Outro::default(),
        Some(RawOutro { say, beats }) => Outro {
            say: words(say, "the outro's say")?,
            beats: length(beats).map_err(|error| format!("the outro's {error}"))?,
        },
    };
    let look = match file.look.as_deref() {
        None | Some("macos") => Look::Macos,
        Some("omarchy") => Look::Omarchy,
        Some(other) => return Err(format!("look {other:?} is not macos or omarchy")),
    };
    let database = match file.database.as_deref() {
        None | Some("bookshop") => Database::Bookshop,
        Some(path) => Database::File(dir.join(path)),
    };
    for (index, saved) in file.connections.iter().enumerate() {
        if saved.name.trim().is_empty() {
            return Err(format!("connection {} has no name", index + 1));
        }
        if file.connections[..index]
            .iter()
            .any(|earlier| earlier.name == saved.name)
        {
            return Err(format!("two connections are called {:?}", saved.name));
        }
    }
    let meanwhile = words(file.meanwhile, "meanwhile")?;
    let scenes = file
        .scenes
        .into_iter()
        .enumerate()
        .map(|(index, scene)| {
            scene
                .read(look)
                .map_err(|error| format!("scene {}{error}", index + 1))
        })
        .collect::<Result<_, String>>()?;
    Ok(Scenario {
        title: file.title,
        database,
        look,
        tempo: file.tempo,
        voice,
        intro,
        outro,
        connections: file.connections,
        meanwhile,
        scenes,
    })
}

/// The file as it is written.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct File {
    title: String,
    database: Option<String>,
    look: Option<String>,
    #[serde(default = "tempo")]
    tempo: u32,
    voice: Option<String>,
    intro: Option<RawIntro>,
    outro: Option<RawOutro>,
    #[serde(rename = "connection", default)]
    connections: Vec<Saved>,
    meanwhile: Option<String>,
    #[serde(rename = "scene", default)]
    scenes: Vec<RawScene>,
}

/// The intro card: a line to say, its words, and its length.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawIntro {
    say: Option<String>,
    title: Option<String>,
    text: Option<String>,
    beats: Option<f64>,
}

/// The outro card: a line to say, and its length.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RawOutro {
    say: Option<String>,
    beats: Option<f64>,
}

/// `text`, which may be absent but not empty: `what` is named if it is.
fn words(text: Option<String>, what: &str) -> Result<Option<String>, String> {
    match text {
        Some(text) if text.trim().is_empty() => Err(format!("{what} is empty")),
        text => Ok(text),
    }
}

/// A length in beats: above 0, in half beats.
fn length(beats: Option<f64>) -> Result<Option<f64>, String> {
    match beats {
        Some(beats) if !(beats > 0.0 && beats.is_finite() && (beats * 2.0).fract() == 0.0) => {
            Err(format!("beats {beats} is not a positive multiple of 0.5"))
        }
        beats => Ok(beats),
    }
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
    label: Option<String>,
    beats: Option<f64>,
}

impl RawScene {
    /// The scene, or what is wrong with it, the error after its place
    /// (": ..." for the scene, ", step N: ..." for a step). Its keys are
    /// `look`'s.
    fn read(self, look: Look) -> Result<Scene, String> {
        let RawScene {
            caption,
            steps,
            slide,
            text,
            say,
            label,
            beats,
        } = self;
        let say = words(say, ": say")?;
        let beats = length(beats).map_err(|error| format!(": {error}"))?;
        let Some(title) = slide else {
            let label = words(label, ": label")?;
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
                    raw.read(look)
                        .map_err(|error| format!(", step {}: {error}", step + 1))
                })
                .collect::<Result<_, _>>()?;
            return Ok(Scene::Shot {
                caption,
                say,
                steps,
                label,
                beats,
            });
        };
        if label.is_some() {
            return Err(": only a shot has a label".into());
        }
        if !steps.is_empty() || caption.is_some() {
            return Err(": a slide has no steps and no caption".into());
        }
        if title.trim().is_empty() {
            return Err(": a slide needs a title".into());
        }
        if text.as_deref().is_some_and(|text| text.trim().is_empty()) {
            return Err(": a slide's text is empty".into());
        }
        Ok(Scene::Slide {
            title,
            text,
            say,
            beats,
        })
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
    omarchy: Option<String>,
}

/// What a step that names none, or more than one, is told.
const ONE_STEP: &str = "a step is one of open, click, command, press, type, hold or omarchy";

impl RawStep {
    fn read(self, look: Look) -> Result<Step, String> {
        let RawStep {
            open,
            click,
            command,
            key,
            press,
            times,
            typed,
            hold,
            omarchy,
        } = self;
        let names = [
            open.is_some(),
            click.is_some(),
            command.is_some(),
            press.is_some(),
            typed.is_some(),
            hold.is_some(),
            omarchy.is_some(),
        ];
        if names.iter().filter(|named| **named).count() != 1 {
            return Err(ONE_STEP.into());
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
            return command_step(look, &id, key.as_deref(), times);
        }
        if let Some(keys) = press {
            return Ok(Step::Press {
                keys: strokes(&keys)?,
                times,
            });
        }
        if let Some(text) = typed {
            if text.is_empty() {
                return Err("type needs text".into());
            }
            return Ok(Step::Type(text));
        }
        if let Some(theme) = omarchy {
            if look != Look::Omarchy {
                return Err("only the omarchy look switches Omarchy's theme".into());
            }
            if theme.trim().is_empty() {
                return Err("omarchy needs a theme's name".into());
            }
            return Ok(Step::Omarchy(theme));
        }
        match hold {
            Some(seconds) if seconds > 0.0 && seconds <= LONGEST_HOLD => Ok(Step::Hold(seconds)),
            Some(seconds) => Err(format!(
                "hold {seconds} is not above 0 and up to {LONGEST_HOLD}"
            )),
            None => Err(ONE_STEP.into()),
        }
    }
}

/// The command `id` names, pressed with `key` (one of its own keys in
/// `look`) or else with its first.
fn command_step(look: Look, id: &str, key: Option<&str>, times: u32) -> Result<Step, String> {
    let command = Command::from_id(id).ok_or_else(|| format!("no command is called {id:?}"))?;
    if !command.info().built {
        return Err(format!("{id} is not built yet"));
    }
    let keymap = Keymap::default();
    let own: Vec<&'static str> = keymap
        .of(command)
        .flat_map(|binding| binding.bound(look.layout()).chords.iter())
        .filter(|chord| chord.kind == Kind::Own)
        .map(|chord| chord.keys)
        .collect();
    let spelled = match key {
        None => own
            .first()
            .copied()
            .ok_or_else(|| format!("{id} has no key on {}", look.name()))?,
        Some(key) => own.iter().copied().find(|own| *own == key).ok_or_else(|| {
            format!(
                "{key:?} is not a key of {id}: its keys are {}",
                own.join(", ")
            )
        })?,
    };
    Ok(Step::Command {
        command,
        keys: strokes(spelled)?,
        times,
    })
}

/// `keys`, as `settings.toml` spells them, read as what they press one
/// after another: one press of a key, a few (`dd`, `yy p`), or a line of
/// the `:` prompt (`:diff`).
fn strokes(keys: &str) -> Result<Keys, String> {
    let spelled = keymap::intern(keys);
    let strokes = match keymap::spell(spelled).map_err(|keymap::Unread(why)| why)? {
        Spelled::Strokes(strokes) => strokes,
        Spelled::Ex(line) => std::iter::once(':')
            .chain(line.chars())
            .map(Stroke::Text)
            .chain([Stroke::Key {
                mods: keymap::Mods::default(),
                key: egui::Key::Enter,
            }])
            .collect(),
        Spelled::Digits(_) => return Err(format!("{spelled} is nine keys, not one")),
    };
    Ok(Keys { spelled, strokes })
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Database, Intro, Keys, Look, Outro, Saved, Scene, Step, load, parse};
    use crate::env::Environment;
    use crate::keymap::{Command, Mods, Stroke};

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
            strokes: vec![Stroke::Key {
                key: egui::Key::ArrowDown,
                mods: Mods::default(),
            }],
        };
        let enter = Keys {
            spelled: "enter",
            strokes: vec![Stroke::Key {
                key: egui::Key::Enter,
                mods: Mods::default(),
            }],
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
        let [Stroke::Key { mods, key }] = keys.strokes.as_slice() else {
            panic!("not one press: {keys:?}");
        };
        assert!(mods.cmd, "the filter bar's first key on macOS holds Cmd");
        assert_eq!(*key, egui::Key::F);
    }

    #[test]
    fn an_omarchy_scenario_presses_vim_s_keys_and_the_prompt_s_lines() {
        let scenario = read(
            r#"
title = "T"
look = "omarchy"
[[scene]]
steps = [
  { command = "delete_row" },
  { command = "review_sql" },
  { command = "find_object" },
  { press = "i" },
]
"#,
        )
        .unwrap();
        assert_eq!(scenario.look, Look::Omarchy);
        let typed = |text: &str| text.chars().map(Stroke::Text).collect::<Vec<_>>();
        let enter = Stroke::Key {
            key: egui::Key::Enter,
            mods: Mods::default(),
        };
        let pressed: Vec<_> = shot(&scenario.scenes[0])
            .1
            .iter()
            .map(|step| match step {
                Step::Command { keys, .. } | Step::Press { keys, .. } => {
                    (keys.spelled, keys.strokes.clone())
                }
                other => panic!("not keys: {other:?}"),
            })
            .collect();
        let ctrl = Mods {
            ctrl: true,
            ..Mods::default()
        };
        assert_eq!(
            pressed,
            [
                ("dd", typed("dd")),
                (":diff", [typed(":diff"), vec![enter]].concat()),
                (
                    "ctrl+p",
                    vec![Stroke::Key {
                        key: egui::Key::P,
                        mods: ctrl,
                    }]
                ),
                ("i", typed("i")),
            ]
        );
        assert_eq!(
            read("title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n")
                .unwrap()
                .look,
            Look::Macos
        );
        assert!(
            read("title = \"T\"\nlook = \"windows\"\n[[scene]]\nsteps = [{ hold = 1 }]\n")
                .unwrap_err()
                .contains("look \"windows\" is not macos or omarchy")
        );
    }

    #[test]
    fn an_omarchy_scenario_may_switch_omarchy_s_theme() {
        let scenario = read(
            "title = \"T\"\nlook = \"omarchy\"\n[[scene]]\nsteps = [{ omarchy = \"Retro 82\" }]\n",
        )
        .unwrap();
        assert_eq!(
            shot(&scenario.scenes[0]).1,
            [Step::Omarchy("Retro 82".into())]
        );
        assert!(
            read("title = \"T\"\nlook = \"omarchy\"\n[[scene]]\nsteps = [{ omarchy = \" \" }]\n")
                .unwrap_err()
                .contains("omarchy needs a theme's name")
        );
    }

    #[test]
    fn a_scenario_may_save_other_connections_and_say_what_changed_meanwhile() {
        let scenario = read(
            r#"
title = "T"
meanwhile = "UPDATE books SET price = 17 WHERE id = 1"

[[connection]]
name = "Bookshop production"
environment = "production"

[[scene]]
steps = [{ hold = 1 }]
"#,
        )
        .unwrap();
        assert_eq!(
            scenario.connections,
            [Saved {
                name: "Bookshop production".into(),
                environment: Environment::Production,
            }]
        );
        assert_eq!(
            scenario.meanwhile.as_deref(),
            Some("UPDATE books SET price = 17 WHERE id = 1")
        );
        let plain = read("title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n").unwrap();
        assert!(plain.connections.is_empty());
        assert_eq!(plain.meanwhile, None);
        let scene = "[[scene]]\nsteps = [{ hold = 1 }]\n";
        let connection = |name: &str, environment: &str| {
            format!("[[connection]]\nname = \"{name}\"\nenvironment = \"{environment}\"\n")
        };
        for (head, said) in [
            (
                connection(" ", "dev"),
                "connection 1 has no name".to_owned(),
            ),
            (
                connection("Shop", "dev") + &connection("Shop", "staging"),
                "two connections are called \"Shop\"".to_owned(),
            ),
            (
                connection("Shop", "moon"),
                "unknown variant `moon`".to_owned(),
            ),
            (
                "meanwhile = \" \"\n".to_owned(),
                "meanwhile is empty".to_owned(),
            ),
        ] {
            let error = read(&format!("title = \"T\"\n{head}{scene}")).unwrap_err();
            assert!(error.contains(&said), "{error}");
        }
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
            ("{ press = \"alt+1..9\" }", "is nine keys, not one"),
            ("{ press = \"ctrl+nonsense\" }", "nonsense"),
            ("{ type = \"\" }", "type needs text"),
            ("{ hold = 0 }", "hold 0 is not above 0"),
            (
                "{ omarchy = \"Nord\" }",
                "only the omarchy look switches Omarchy's theme",
            ),
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
        assert_eq!(scenario.intro.say.as_deref(), Some("Here is what is new."));
        assert_eq!(scenario.outro.say.as_deref(), Some("Download it today."));
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
                beats: None,
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
        assert_eq!(scenario.intro, Intro::default());
        assert_eq!(scenario.outro, Outro::default());
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
                beats: None,
            }
        );
        assert_eq!(
            scenario.scenes[1],
            Scene::Slide {
                title: "Small.".into(),
                text: None,
                say: None,
                beats: None,
            }
        );
        assert_eq!(shot(&scenario.scenes[2]).1, [Step::Hold(1.0)]);
    }

    #[test]
    fn a_scene_may_last_a_number_of_beats_and_a_shot_may_have_a_label() {
        let scenario = read(
            r#"
title = "T"

[[scene]]
label = "Quick open"
beats = 4
steps = [{ hold = 1 }]

[[scene]]
slide = "Vim mode"
beats = 0.5

[[scene]]
steps = [{ hold = 1 }]
"#,
        )
        .unwrap();
        let Scene::Shot { label, beats, .. } = &scenario.scenes[0] else {
            panic!("not a shot: {:?}", scenario.scenes[0]);
        };
        assert_eq!(label.as_deref(), Some("Quick open"));
        assert_eq!(*beats, Some(4.0));
        let Scene::Slide { beats, .. } = &scenario.scenes[1] else {
            panic!("not a slide: {:?}", scenario.scenes[1]);
        };
        assert_eq!(*beats, Some(0.5));
        let Scene::Shot { label, beats, .. } = &scenario.scenes[2] else {
            panic!("not a shot: {:?}", scenario.scenes[2]);
        };
        assert_eq!((label, beats), (&None, &None));
    }

    #[test]
    fn the_intro_may_name_its_card_and_the_cards_may_last_a_number_of_beats() {
        let scenario = read(
            r#"
title = "T"

[intro]
title = "More than 50 new features"
text = "Tabletist 0.3"
beats = 8

[[scene]]
steps = [{ hold = 1 }]

[outro]
say = "Get it free."
beats = 12
"#,
        )
        .unwrap();
        assert_eq!(
            scenario.intro,
            Intro {
                say: None,
                title: Some("More than 50 new features".into()),
                text: Some("Tabletist 0.3".into()),
                beats: Some(8.0),
            }
        );
        assert_eq!(
            scenario.outro,
            Outro {
                say: Some("Get it free.".into()),
                beats: Some(12.0),
            }
        );
    }

    #[test]
    fn beats_are_a_positive_multiple_of_a_half() {
        for beats in ["0", "-1", "0.3", "1.25", "nan", "inf"] {
            let error = read(&format!(
                "title = \"T\"\n[[scene]]\nbeats = {beats}\nsteps = [{{ hold = 1 }}]\n"
            ))
            .unwrap_err();
            assert!(
                error.starts_with("scene 1: beats ")
                    && error.contains("is not a positive multiple of 0.5"),
                "{beats}: {error}"
            );
            let error = read(&format!(
                "title = \"T\"\n[[scene]]\nslide = \"S\"\nbeats = {beats}\n"
            ))
            .unwrap_err();
            assert!(error.starts_with("scene 1: beats "), "{beats}: {error}");
            let error = read(&format!(
                "title = \"T\"\n[intro]\nbeats = {beats}\n[[scene]]\nsteps = [{{ hold = 1 }}]\n"
            ))
            .unwrap_err();
            assert!(error.starts_with("the intro's beats "), "{beats}: {error}");
            let error = read(&format!(
                "title = \"T\"\n[[scene]]\nsteps = [{{ hold = 1 }}]\n[outro]\nbeats = {beats}\n"
            ))
            .unwrap_err();
            assert!(error.starts_with("the outro's beats "), "{beats}: {error}");
        }
    }

    #[test]
    fn a_label_is_a_shot_s_and_not_empty_and_the_intro_s_words_are_not_empty() {
        for (text, said) in [
            (
                "title = \"T\"\n[[scene]]\nlabel = \" \"\nsteps = [{ hold = 1 }]\n",
                "scene 1: label is empty",
            ),
            (
                "title = \"T\"\n[[scene]]\nslide = \"S\"\nlabel = \"L\"\n",
                "scene 1: only a shot has a label",
            ),
            (
                "title = \"T\"\n[intro]\ntitle = \"\"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
                "the intro's title is empty",
            ),
            (
                "title = \"T\"\n[intro]\ntext = \" \"\n[[scene]]\nsteps = [{ hold = 1 }]\n",
                "the intro's text is empty",
            ),
            (
                "title = \"T\"\n[[scene]]\nsteps = [{ hold = 1 }]\n[outro]\ntitle = \"x\"\n",
                "unknown field",
            ),
        ] {
            let error = read(text).unwrap_err();
            assert!(error.contains(said), "{text}: {error}");
        }
    }
}
