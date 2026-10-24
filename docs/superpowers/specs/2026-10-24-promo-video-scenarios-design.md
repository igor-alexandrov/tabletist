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

Scenarios live in `contrib/promo/scenarios/`. The old branch's video becomes
`overview.toml`.

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
  Every SQLite session is `query_only`: nothing is written to it.
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
| `open = "<object>"` | The pointer goes to the object in the tree and clicks it. |
| `click = "<text>"` | The pointer goes to the AccessKit node with that label, or else to the painted text (a grid's cell is not a node), and clicks there with a pointer press and release. |
| `command = "<id>"`, `key = "<keys>"`, `times = n` | The command's key goes down, `times` times (1 by default). The id is the one `settings.toml` uses. `key` picks one of the command's keys, spelled as `settings.toml` spells keys; without it, the first. The keycap is shown. |
| `press = "<keys>"`, `times = n` | A key that no command holds where the keyboard is (Enter in a field, Backspace), spelled as `settings.toml` spells keys. |
| `type = "<text>"` | Typed into what has the keyboard, a character at a time. A newline is Enter. |
| `hold = <seconds>` | The picture stays, beyond the scene's own time. |

A key is parsed by the keymap's own parser (`src/keymap/overrides.rs`), so
`src/promo` spells none. The parse checks what it can without the app: the
step's shape, that a command id exists, is built and has a key on macOS,
that a `key` is one of its keys. Which scopes hold is worked out inside
`src/ui/keys.rs` and nothing outside it can ask, so the runner does not
check a command's scope: a command that does not hold changes nothing, and
that stops the run (below). A `press` the keymap happens to hold where the
keyboard is does what the app does with it, with no keycap; a scenario that
wants the keycap writes the `command`.

A scenario has no times in it: the director (below) decides them. Every key
the video shows is a `keymap::Written`, so a keycap says what the app's own
hints say.

## The command

    contrib/promo/make contrib/promo/scenarios/filtering.toml [--final] [--still <seconds>...] [--frames-only] [--reuse]

It writes `target/promo/filtering.mp4` and keeps its working files in
`target/promo/filtering/`. It installs `puppeteer-core` into
`target/promo/node_modules` on its first run, so the repository has no
`package.json` and Cargo has no new crate. It needs ffmpeg and Chrome
(`CHROME` names another one).

- `--final`: motion blur on fast camera moves (below). A draft takes about a
  quarter of the time.
- `--still 3.5 7`: those moments as PNGs, beside the video's path, instead
  of the video.
- `--frames-only`: stops after the app's frames.
- `--reuse`: keeps the app's frames from the last run, for work on motion or
  sound only.

## How it is made

Three pieces, each with one job. The app's frames, which take the longest to
make, depend on the scenario's steps only. Pacing, camera, sound and look can
be changed without making them again.

### 1. The runner: `src/promo.rs`

An ignored test behind the `shots` feature, as before:
`PROMO_SCENARIO=<file> cargo test --features shots --lib promo -- --ignored`.

- The scenario is parsed by `src/promo/scenario.rs`, which has no GPU
  dependency and is tested in the normal suite (it is compiled with `test`
  as well as `shots`).
- The database is opened with the real SQLite driver in a read-only
  session. What the app asks its backend is answered from it, as the old
  branch's `Live` did. Writes, saves and secrets are never answered.
- Each step is played as input through the harness: a click is a pointer
  move, press and release at the middle of its target, a command is its key
  from the keymap, typing is `egui::Event::Text`. No step applies an
  `Action` behind the views' back. The video shows what a person's input
  does.
- After each visible change it writes `target/promo/<name>/frames/NNNN.png`
  (window 1200x750 points, scale 2) and appends an event to
  `target/promo/<name>/events.json`:

```json
{ "scene": 1, "frame": 41, "kind": "type", "text": "f",
  "rect": [312, 96, 520, 120] }
```

  `kind` is `caption`, `click`, `command` (with its `keys`, the `Written`
  text), `press`, `type` or `hold`. `rect` is what the step acted on, in
  window points: the clicked node or text; in a grid, the cursor's cell
  (its row's node, across its column header's painted rect); else the node
  with the keyboard focus, from the AccessKit tree. The runner says what
  happened, never when.
- A step that cannot be played stops the run with the scenario's file,
  scene and step: a label or text that is not on screen, a step that
  changed nothing on screen (its frame's pixels are the same as the frame
  before it; `hold` is exempt), a database that does not open. No video
  comes out half right.

### 2. The director: `contrib/promo/director.js`

A pure function from the events and the tempo to a timeline. Nothing else
decides time; the picture and the sound both read its output.

- Typing goes at about 13 characters a second, with a jitter drawn from a
  generator seeded by the scenario's name: the same scenario always gives
  the same video.
- The pointer takes 0.35 to 0.7 s to reach a click, by distance. A command
  or a key holds 0.35 s. A result holds 0.6 s after the last step.
- A scene lasts at least as long as its caption takes to read (0.3 s a word,
  1.6 s at least), and is rounded up to the next bar of the tempo. A
  caption's first word lands on a beat.
- The camera frames the union of the rects of the events in the next
  0.6 s, and starts moving 0.25 s before them. It zooms so that area fills
  about half the picture, between 1 and 1.7 times. It holds still while
  that area stays inside the middle third (a dead zone, so typing along a
  line does not drag it), and moves on a critically damped spring, which
  never overshoots.
- It is tested with `node --test` (`contrib/promo/director.test.js`):
  beats, bars, the dead zone, the spring, and that the same input gives
  the same timeline.

### 3. The composer: `contrib/promo/index.html`, `render.cjs`, `audio.js`

`index.html` draws any time of the timeline with `renderAt(seconds)`, with
no clock of its own. `render.cjs` asks headless Chrome for every frame and
pipes them to ffmpeg, as on the old branch.

The picture:

- The window: the app's frame for that moment, with the window's shadow
  and traffic lights, moved by the director's camera.
- The pointer: a macOS-style cursor, drawn by the composer. It travels on
  a slight arc, eases in and out, slows as it arrives, and presses with a
  ripple. It hides while typing, as a real one does.
- The spotlight: the camera's target at full brightness, the rest of the
  window dimmed by about 15% with a feathered edge.
- Keycaps: a `command`'s keys spring in with a small overshoot on its
  sound, and stay 0.8 s.
- Captions: the words rise one after another through a mask with a short
  blur, under a soft scrim at the bottom.
- Cards: the intro (icon, name, "A small, fast, native database client.")
  and the outro (the title, the engines, the platforms,
  `igor.works/tabletist`) build piece by piece on beats. The window rises
  in from the intro and leaves into the outro with a slight blur.
- Motion blur, with `--final`: a frame whose camera moves faster than a
  threshold is the mean of four subframes.

The sound is rendered by `audio.js` in the same Chrome, with an
`OfflineAudioContext` at 48 kHz, from the director's timeline, and written
as WAV:

- The music, in the scenario's tempo: four chords on a warm pad (detuned
  saws through a slow lowpass), a soft plucked arpeggio, a felt kick and
  light noise hats. It is sparse under the intro, fills in from the first
  scene, and resolves on the outro with a tail.
- The effects, on the events: a key click for each character and key, each
  a little different in pitch and level; a mouse click; a whoosh for each
  camera move, louder and longer the farther it goes; a low tick under a
  keycap; a bell on the outro.
- The music ducks under each effect. ffmpeg normalizes the mix to −14 LUFS
  (`loudnorm`), the level the social platforms play at, and writes it as
  AAC at 192 kb/s into the MP4.
- Its random numbers come from the same seeded generator: the same scenario
  always sounds the same.

## The data

The runner writes the Bookshop to `target/promo/bookshop.sqlite` with
`tabletist_db::fixtures::write_sqlite_bookshop` when the scenario asks for
it, removing the last run's file first (the helper refuses one that
exists). There is one Bookshop to keep, and no `sqlite3` binary is needed.

## Testing

- Rust, in the normal suite: the scenario's parsing and its errors (an
  unknown step, an unknown or unbuilt command id, a `key` that is not one
  of the command's keys, a tempo out of range). A test that spells a key
  puts its file in `tests/keys.rs`'s `TESTS` list.
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
