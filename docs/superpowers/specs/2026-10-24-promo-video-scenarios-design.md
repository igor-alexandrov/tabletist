# Promotional videos from scenarios

A promotional video is written as a scenario, a small TOML file that says
what to show and what to say, and one command turns it into an MP4 with
music and sound. It replaces the hard-coded video of the
`claude/promo-video` branch (spec of 2026-10-11), whose scenes were written
twice, once in Rust and once in JavaScript, each with its own timing.

Each video is 1920x1080 at 60 fps, H.264 with AAC sound, for social posts
and the website. Nothing a video needs is fetched or licensed: the music and
the sound effects are synthesized, and the data is the Bookshop's unless the
scenario names another file.

## The scenario

Scenarios live in `contrib/promo/scenarios/`:

- `overview.toml`, the old branch's video: a table opened and walked, a
  filter, an edit and its review, and a query written in a SQL tab.
- `filtering.toml`, one feature explained: the filter bar's column,
  operator and value picked and typed, and the rows that are left walked.
- `smoke.toml`, a few seconds (an open, a click, a command and a hold),
  for checking the pipeline.

```toml
title    = "Filtering"     # the end card says it; the video is named after the file
database = "bookshop"      # the Bookshop fixture, or a path to a SQLite file
tempo    = 112             # the music's beats per minute; a scene ends on a bar

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
  { hold = 1.0 },
]
```

- `database`: `bookshop` (the default) is the Bookshop the site's
  screenshots and the MCP tests use, written by
  `tabletist_db::fixtures::write_sqlite_bookshop` from
  `crates/tabletist-db/fixtures/bookshop/sqlite.sql`. A path, relative to the
  scenario, names another SQLite file, as the Fizzy video did by request.
  The file is opened read-only: nothing is written to it.
- `tempo`: 80 to 160.
- A scene has a `caption` (optional: a scene without one is a move between
  two) and its `steps`. The intro and outro cards are not scenes: every
  video has them.

The look is macOS, dark. The Omarchy look (its window, its vim SQL editor,
its multi-stroke and `:` keys) is not in this.

### Where a video starts

Before the first step, the runner sets the scene up directly, and none of
it is filmed: a saved connection named after the database (`Bookshop`, or
the file's stem), Dev environment, connected; the tree showing the main
schema; nothing open; the row panel hidden, so the grid has the window's
width. The window is 1200x750 points at scale 2 with the macOS 26 title bar
the old branch measured (`TitleBar { height: 38, inset: 80 }`). The setup
may apply `Action`s; the steps never do.

### The steps

| Step | What happens |
|---|---|
| `open = "<object>"` | The pointer goes to the object in the sidebar tree and clicks it. |
| `click = "<text>"` | The pointer goes to the AccessKit node with that label, or else to the painted text (a grid's cell is not a node), and clicks there with a pointer press and release. |
| `command = "<id>"`, `key = "<keys>"`, `times = n` | The command's key goes down, `times` times (1 by default). The id is the one `settings.toml` uses. `key` picks one of the command's keys, spelled as `settings.toml` spells keys; without it, the first. The keycap is shown. |
| `press = "<keys>"`, `times = n` | A key that no command holds where the keyboard is (Enter in a field, Backspace), spelled as `settings.toml` spells keys. |
| `type = "<text>"` | Typed into what has the keyboard, a character at a time. A newline is Enter. |
| `hold = <seconds>` | The picture stays, beyond the scene's own time. Up to 10 s. |

A key is parsed by the keymap's own parser (`keymap::spell`, which reads
`settings.toml`'s keys too), so `src/promo` spells none. The parse checks
what it can without the app: the step's shape, that a command id exists,
is built and has a key on macOS, that a `key` is one of its keys, that a
key is one press of one key. Which scopes hold is worked out inside
`src/ui/keys.rs` and nothing outside it can ask, so the runner does not
check a command's scope: a command that does not hold changes nothing, and
that stops the run (below). A `press` the keymap happens to hold where the
keyboard is does what the app does with it, with no keycap; a scenario that
wants the keycap writes the `command`.

A scenario has no times in it: the director (below) decides them. Every key
the video shows is a `keymap::Written`, so a keycap says what the app's own
hints say.

### Which node a label means

A tree holds several nodes of one name: a table's is a tree item, a tab and
a heading once it is open, and a column's is its header and a choice in the
filter's menu. egui hands its nodes over in no set order, so the runner
chooses (`src/promo/pick.rs`):

- `open` takes only the sidebar tree's items: the buttons inside its
  "Objects" group.
- `click` takes the only node with the label (or with it as its value, as
  a plain label has), whatever its role. Of several, it takes the only one
  in an area over the window: a menu or a popup, which a click there would
  reach.
- Otherwise the run stops and lists every candidate, its role and its rect,
  for the scenario to name another label.
- Where no node has the label, a `click` takes the painted text that reads
  it.

## The command

    contrib/promo/make contrib/promo/scenarios/filtering.toml [--final] [--still <seconds>...] [--frames-only] [--reuse]

It writes `target/promo/filtering.mp4` and keeps its working files in
`target/promo/filtering/`. It installs `puppeteer-core` into
`target/promo/node_modules` on its first run, at the version `make` pins,
and again whenever the installed one is another, so the repository has no
`package.json` and Cargo has no new crate. It needs ffmpeg, Node and Chrome
(`CHROME` names another one).

- `--final`: motion blur, four frames rendered and blended into each (below).
  A draft takes about a quarter of the time.
- `--still 3.5 7`: those moments as PNGs, beside the video's path, instead
  of the video.
- `--frames-only`: stops after the app's frames.
- `--reuse`: keeps the app's frames from the last run, for work on motion or
  sound only.

## How it is made

Three pieces, each with one job. The app's frames, which take the longest to
make, depend on the scenario's steps only. Pacing, camera, sound and look can
be changed without making them again.

### 1. The runner: `src/promo/runner.rs`

An ignored test behind the `shots` feature:
`PROMO_SCENARIO=<file> cargo test --features shots --lib promo::runner::film -- --ignored --exact`.

- `src/promo` is compiled for tests only. The scenario's parser
  (`scenario.rs`) and the choice of a label's node (`pick.rs`) have no GPU
  dependency and are tested in the normal suite; the runner and its
  database (`live.rs`) need `shots` as well.
- The database is opened with the real SQLite driver, read-only, and the
  runner stops unless the session says it is. What the app asks its backend
  is answered from it, as the old branch's `Live` did. Writes, saves and
  secrets are never answered, and a script that writes is refused by the
  read-only session.
- Each step is played as input through the harness: a click is a pointer
  move, press and release at the middle of its target, a command is its key
  from the keymap, typing is `egui::Event::Text`. No step applies an
  `Action` behind the views' back. The video shows what a person's input
  does.
- The harness's text caret does not blink (`App::caret_blinks` is false
  there, and each restyle keeps it so). Its caret holds still, so two
  frames of the same state are the same pixels, and a blink cannot pass for
  a step's change.
- After each visible change it writes `target/promo/<name>/frames/NNNN.png`
  (window 1200x750 points, scale 2). At the end it writes
  `target/promo/<name>/events.json`: the scenario's title and tempo, the
  window's size and scale, the captions in `scenes`, and the events.

```json
{
  "title": "Filtering", "tempo": 112, "size": [1200, 750], "scale": 2,
  "scenes": [{ "caption": "Open a table. It's just there." }],
  "events": [
    { "scene": null, "frame": 0, "kind": "start" },
    { "scene": 0, "frame": 1, "kind": "point", "rect": [20, 180, 140, 200] },
    { "scene": 0, "frame": 2, "kind": "click", "rect": [20, 180, 140, 200] },
    { "scene": 0, "frame": 3, "kind": "command", "keys": "⌘F", "rect": [249, 96, 1200, 132] },
    { "scene": 0, "frame": 4, "kind": "type", "text": "t", "rect": [312, 96, 520, 120] },
    { "scene": 0, "frame": 4, "kind": "hold", "seconds": 1.0 }
  ]
}
```

  `kind` is `start` (the frame the video opens on), `point` (the pointer
  has arrived, and the frame shows the hover, which may change nothing),
  then one `click`, `command` (with its `keys`, the `Written` text),
  `press`, `type` or `hold` for each step, each repetition of a key and
  each character typed. `frame` is the PNG that shows it. `rect` is in
  window points:
  - of a click or a point, the clicked node or text;
  - of a key or a character, while the grid has the keyboard, the cursor's
    cell: its column header's node across its row's node. The grid has the
    keyboard when the focused node holds that row, or when egui's focus is
    the window root and the workspace's pane is the grid;
  - otherwise the node with the keyboard focus, from the AccessKit tree;
  - absent when there is none of these.

  The runner says what happened, never when.
- A step that cannot be played stops the run with the scenario's file,
  scene and step: a label or text that is not on screen, a label several
  nodes share, a step that changed nothing on screen (its frame's pixels
  are the same as the frame before it; `hold` and the pointer's hover are
  exempt). A database that is not a file, does not open or is not
  read-only stops it with the scenario's file. No video comes out half
  right.

### 2. The director: `contrib/promo/director.js`

A pure function from the events and the tempo to a timeline. Nothing else
decides time; the picture and the sound both read its output.

- The intro is a bar. A scene starts on a bar and its first step half a
  beat later, so a caption's first word lands on a beat. The outro is two
  bars after the last scene.
- Typing goes at about 13 characters a second, with a jitter drawn from a
  generator seeded by the scenario's title, and a longer gap at a space:
  the same scenario always gives the same video.
- The pointer takes 0.35 to 0.7 s to reach a point, by distance, and
  clicks 0.12 s after it arrives. A click, a command and a press each hold
  0.35 s. A key that follows typing (Enter, Run) waits 0.3 s more, as a
  hand does that has just typed. A result holds 0.6 s after a scene's last
  step.
- A scene lasts at least as long as its caption takes to read (0.3 s a word,
  1.6 s at least), and is rounded up to the next bar of the tempo.
- The camera holds the wide shot while the window rises in, until half a
  beat after the intro.
- Then it frames the union of the rects of the events from 0.4 s ago to
  0.85 s on, so it is there as the action starts. It zooms so that area
  fills about half the picture, between 1 and 1.7 times. It holds still
  while that area stays inside the middle third at about the same zoom (a
  dead zone, so typing along a line does not drag it), and moves on a
  critically damped spring, which never overshoots.
- Of a wide field being typed in (the SQL editor), it frames only the left
  400 points (`TYPE_SPAN`), where the text goes.
- The spotlight does not wait for the camera: on its own spring, it lights
  the latest event with a rect, 0.1 s before it happens, with a little
  room round it.
- The reveal: once the action has been quiet for `REVEAL` (0.9 s), and the
  next action is at least `QUIET` (2.4 s) after the last, the camera pulls
  back to the whole window and the spotlight lets go, so the result is seen
  whole. While it is back, the camera drifts in by `DRIFT` (1.5% a second),
  so a held shot is not a frozen one. From just before the outro, the
  camera shows the whole window and the spotlight is off.
- The pointer travels on a slight arc, eased in and out. It hides from the
  first character typed after its last move, as macOS hides it, and comes
  back with the next move.
- It is tested with `node --test` (`contrib/promo/director.test.js`):
  beats, bars, reading time, the typing's seed, the pointer, the dead zone,
  the wide shot, the spotlight, the reveal, the typing span, the pause
  after typing, the spring, the keycaps and their sounds, and that the same
  input gives the same timeline.

### 3. The composer: `contrib/promo/index.html`, `render.cjs`, `audio.js`

`index.html` draws any time of the timeline with `renderAt(seconds)`, with
no clock of its own. `render.cjs` asks headless Chrome for every frame and
pipes them to ffmpeg, as on the old branch.

The picture:

- The window: the app's frame for that moment, with the window's shadow
  and traffic lights, moved by the director's camera.
- The pointer: a macOS-style cursor, drawn by the composer where the
  director puts it. It presses with a ripple.
- The spotlight: a feathered mask over the window that dims all but the
  lit area by 30%.
- Keycaps: a `command`'s keys spring in with a small overshoot on its
  sound, and stay 0.8 s. A keycap hangs off the left of the caption, which
  holds still in the middle; without a caption, the keycap takes the
  middle.
- Captions: the words rise one after another through a mask with a short
  blur, under a soft scrim at the bottom.
- Cards: the intro (icon, name, "A small, fast, native database client.")
  and the outro (the title, the engines, the platforms,
  `igor.works/tabletist`) build piece by piece on beats. The window rises
  in from the intro and leaves into the outro with a slight blur.
- Motion blur, with `--final`: every frame is rendered four times, at four
  times the frame rate, and ffmpeg's `tmix` blends each four into one.

The sound is rendered by `audio.js` in the same Chrome, with an
`OfflineAudioContext` at 48 kHz, from the director's timeline, and written
as WAV:

- The music, in the scenario's tempo: four chords on a warm pad (detuned
  saws through a slow lowpass), a sine bass, a soft plucked arpeggio, a
  felt kick and light noise hats, through a room and a gentle compressor.
  The pad is alone under the intro, the rest comes in from the first scene,
  and the last chord rings out over the outro.
- The effects, on the events: a key click for each character and key, each
  a little different in pitch and level; a mouse click; a whoosh for each
  camera move, louder and longer the farther it goes; a low tick under a
  keycap; a bell on the outro. The clicks and whooshes are loud enough to
  stand out over the music (a click's taps at 0.8 and 0.45, a whoosh at up
  to 0.45, through effects at 0.9 against the music's 0.55).
- The music ducks under each effect. ffmpeg normalizes the mix to −14 LUFS
  (`loudnorm`), the level the social platforms play at, and writes it as
  AAC at 192 kb/s into the MP4.
- Its random numbers come from the same seeded generator. No node's input
  takes more than two connections: Chrome sums the connections into one
  input in an order that changes from run to run, and a sum of three or
  more floating point numbers depends on it. So the same scenario always
  sounds the same, to the sample.

## The data

The runner writes the Bookshop to `target/promo/bookshop.sqlite` with
`tabletist_db::fixtures::write_sqlite_bookshop` when the scenario asks for
it, removing the last run's file first (the helper refuses one that
exists). There is one Bookshop to keep, and no `sqlite3` binary is needed.

## Testing

- Rust, in the normal suite: the scenario's parsing and its errors (an
  unknown step, an unknown or unbuilt command id, a `key` that is not one
  of the command's keys, a key that is not one press, a tempo out of
  range, a file's errors naming it); the choice of a label's node; and
  that the app's caret blinks and the harness's does not. The tests spell
  keys as `settings.toml` does (`down`, `enter`), which is not what
  `tests/keys.rs` takes for a key written by hand.
- Node: the director's tests.
- By hand: `overview.toml` and a 3-second scenario rendered, stills looked
  at, and the sound checked as a waveform and its loudness.

The renders need a GPU and Chrome, and are not part of CI.

## Not in this

Voiceover, the Omarchy look, vertical (9:16) videos, a music track from a
file, and scenarios drafted by a model from prose.

## Order of the work

Three checkpoints, each tested before the next: the scenario and the
runner, writing frames and `events.json`; the director and its tests,
with the composer drawing the picture; the sound.
