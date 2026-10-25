# Promotional videos from scenarios

A promotional video is written as a scenario, a small TOML file that says
what to show and what to say, and one command turns it into an MP4 with
music and sound. It replaces the hard-coded video of the
`claude/promo-video` branch (spec of 2026-10-11), whose scenes were written
twice, once in Rust and once in JavaScript, each with its own timing.

Each video is 1920x1080 at 60 fps, H.264 with AAC sound, for social posts
and the website. Nothing a video needs is fetched or licensed: the voice is
macOS's own, the music and the sound effects are synthesized, and the data
is the Bookshop's unless the scenario names another file.

The videos follow 37signals' "What's new in Basecamp 5": a real voice
narrates, first person and casual ("Let's jump in."); the light app fills
the frame; the title and end cards are full-bleed brand blue with bold
white type; the camera is plain, the app at full size with small zooms at
most; light, upbeat music runs under the voice; and the video ends on a
call to action. The first cut, dark and spotlit with captions and no voice,
did not make anyone want to download the app.

## The scenario

Scenarios live in `contrib/promo/scenarios/`:

- `overview.toml`, the old branch's video: a table opened and walked, a
  filter, an edit and its review, and a query written in a SQL tab.
- `filtering.toml`, one feature explained: the filter bar's column,
  operator and value picked and typed, and the rows that are left walked.
- `whats-new.toml`, what is new since 0.2, as a 20 second reel (below):
  a hook ("More than 50 new features", true of the 0.2.0 to 0.3.0 release
  notes, which list about 56, fixes aside), seven cuts of the app on the
  beat (Quick open, editing, its review, rows added, duplicated and
  deleted, foreign keys searched, Format, history), seven flash cards
  faster and faster, and a hit into the call to action. Each feature gets
  a video of its own later.
- `smoke.toml`, a few seconds (an open, a click, a command and a hold),
  for checking the pipeline.

```toml
title    = "Filtering"     # the intro card says it; the video is named after the file
database = "bookshop"      # the Bookshop fixture, or a path to a SQLite file
tempo    = 128             # the music's beats per minute; a scene ends on a half bar
voice    = "Ava (Premium)" # the voice that says the lines, as `say -v` names it

[intro]
say = "Here's how to find the rows you need, without writing any SQL."

[[scene]]
caption = "Open a table. It's just there."
say = "Click a table, and it's right there."
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

[outro]
say = "Tabletist is free. Download it today."
```

- `database`: `bookshop` (the default) is the Bookshop the site's
  screenshots and the MCP tests use, written by
  `tabletist_db::fixtures::write_sqlite_bookshop` from
  `crates/tabletist-db/fixtures/bookshop/sqlite.sql`. A path, relative to the
  scenario, names another SQLite file, as the Fizzy video did by request.
  The file is opened read-only: nothing is written to it.
- `tempo`: 80 to 160.
- `voice`: the macOS voice that says the lines, `Ava (Premium)` unless the
  scenario names another. An empty one is an error.
- A scene has a `caption` (optional: a scene without one is a move between
  two), a `say` (optional) and its `steps`. The intro and outro cards are
  not scenes: every video has them. `[intro]` and `[outro]` take a
  `say`, the line said over the card, and `beats`; `[intro]` also takes a
  `title` and a `text`, which replace its card's title and tagline. None
  of them may be empty.
- `say`, on a shot, a slide, the intro or the outro, is a line the voice
  says: what you gain, said as a person would, not what the button is
  called. An empty one is an error. A shot that has one shows it as
  subtitles instead of its caption; a scenario with no line keeps its
  captions as before.
- A scene can be a slide instead: a feature named on a card of its own,
  with the app away. It has a `slide` (the headline) and may have a `text`
  (one line under it), and no caption and no steps:

  ```toml
  [[scene]]
  slide = "Keyboard first."
  text = "Every command has a key, and the keys are yours to change."
  ```

  A blue curtain rises over the app as for the outro, the headline rises
  word by word, its line follows a beat later, and the curtain lifts away
  for the next scene. Under a slide the drums rest, and a soft bell marks
  it. A slide with a `say` reads best with only its headline: the
  subtitles carry the rest.
- `beats`, on a shot, a slide, the intro or the outro: its exact length,
  in beats, a positive multiple of 0.5 (0, a negative number or 0.3 is an
  error). A shot with `beats` is a cut, a slide with `beats` a flash card.
- `label`, on a shot only, not empty: the feature it shows, in big type
  over the cut.

### A reel

The owner's brief for the 0.3 video: promotional, not explaining every
feature; 20 to 30 seconds; dynamic; "More than X features" first, then
the features one after another, the speed and the sound rising. A reel is
a scenario of fixed-length scenes (tempo 128, a beat 0.469 s):

1. The hook, 8 beats: the intro card, its `title` big ("More than 50 new
   features") and its `text` small ("Tabletist 0.3"); the voice says both,
   running on a little into the first cut. The pad and a riser.
2. The cuts, hard on the beat: Quick open and editing 4 beats each, the
   review, rows, foreign keys, Format and history 2 each. Each is the
   steps today's video played, in order, so the app's state carries from
   one to the next, and each has its label. The drums come in.
3. Flash cards, full blue, a phrase each from the 0.3.0 release notes:
   "Agents over MCP", "Vim mode", "Rebind any key" a beat each, "Show all
   rows", "Run without timeout", "AppImage for Linux" and "Notarized for
   macOS" half a beat each. The roll speeds up.
4. A hit on the outro's first beat and the end card ("Get it free."), 12
   beats, the pad ringing out.

That is 20.2 seconds. A scenario without `beats` is not a reel, and its
pictures and sound are what they were.

The look is macOS, light. The Omarchy look (its window, its vim SQL editor,
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
(`CHROME` names another one). A scenario with lines needs macOS, for
`say`, and its voice installed (System Settings > Accessibility > Spoken
Content > System voice > Manage Voices); without the voice, the render
stops and says where to add it.

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
  window's size and scale, the captions in `scenes` (a slide's `slide` and
  `text` there too, with no events: the runner plays nothing for it; a
  scene's `label` and `beats`), the intro's and the outro's `say`,
  `title`, `text` and `beats` where the scenario gives any, and the
  events.

```json
{
  "title": "Filtering", "tempo": 128, "size": [1200, 750], "scale": 2,
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

### Narration: `render.cjs` and `say`

Before the director runs, `render.cjs` speaks each line of the record
(the intro's, the scenes' in order, the outro's) with
`say -v <voice> -o <work>/voice/NN.aiff "<line>"`, makes each a 48 kHz WAV
with ffmpeg (Chrome decodes no AIFF), and measures it with ffprobe. Each
line's owner in the record gets `speech`, the WAV's `file://` URL and its
seconds, and the director times the scenes round them. `say` gives the same
samples for the same voice and line, so a render sounds the same each time.

### 2. The director: `contrib/promo/director.js`

A pure function from the events and the tempo to a timeline. Nothing else
decides time; the picture and the sound both read its output.

- The intro is a bar, or, with a line, long enough for the line (from
  0.6 s, once the title is in), its 0.4 s of room and a beat, on a half
  bar. A scene starts on a half bar and its first step
  half a beat later, so a caption's first word lands on a beat. The outro
  is a bar and a half after the last scene, and the music's last chord
  starts with it, in the middle of a bar if need be.
- Typing goes at about 18 characters a second, with a jitter drawn from a
  generator seeded by the scenario's title, and a longer gap at a space:
  the same scenario always gives the same video.
- The pointer takes 0.28 to 0.55 s to reach a point, by distance, and
  clicks 0.1 s after it arrives. A click, a command and a press each hold
  0.25 s (`STEP`). A key that follows typing (Enter, Run) waits 0.3 s more, as a
  hand does that has just typed. A result holds 0.4 s after a scene's last
  step.
- A scene lasts at least as long as its caption takes to read (0.3 s a word,
  1.6 s at least), and is rounded up to the next half bar of the tempo:
  a whole bar left too long a still tail.
- A line said starts 0.2 s into its shot, at the same time as the steps,
  which it does not move; a slide's starts once its headline is up (0.3 s,
  0.07 s a word, and 0.35 s for the last to rise). A scene with a line
  lasts at least until the line ends and 0.4 s more, rounded up to a half
  bar as ever. A short line changes nothing. The outro's line starts 0.6 s
  into it, and the video ends 1.2 s after the line, on a half bar, or a
  bar and a half after the outro starts, whichever is later. The timeline
  lists the lines as `voices: [{ at, file, seconds, text }]`.
- Subtitles: each line in pieces of four to nine words, about seven, the
  split chosen that costs least, a piece costing its distance from seven
  squared, less for ending a sentence or a clause, more for ending on a
  word that leads into the next ("and", "the"). Each piece lasts its share
  of the line's words; the last ends with the line. A shot with a line
  shows no caption.
- A slide lasts its words at 0.22 s each (`SLIDE_WORD`: read faster than a
  caption watched with the app) and 0.9 s for the card to come and go, 2.2 s
  at least, rounded up to a half bar. The camera shows the whole window
  through it, so the window comes back as it left.
- The camera holds the wide shot while the window rises in, until half a
  beat after the intro.
- Then it frames the union of the rects of the events from 0.4 s ago to
  0.85 s on, so it is there as the action starts. It zooms so that area
  fills about half the picture, between 1 and 1.25 times: gently, so the
  app stays whole enough to read. It holds still
  while that area stays inside the middle third at about the same zoom (a
  dead zone, so typing along a line does not drag it), and moves on a
  critically damped spring, which never overshoots.
- Of a wide field being typed in (the SQL editor), it frames only the left
  400 points (`TYPE_SPAN`), where the text goes.
- The spotlight does not wait for the camera: on its own spring, it lights
  the latest event with a rect, 0.1 s before it happens, with a little
  room round it. The picture no longer draws it (nothing is dimmed); the
  timeline keeps it as where the action is.
- The reveal: once the action has been quiet for `REVEAL` (0.6 s), and the
  next action is at least `QUIET` (1.6 s) after the last, the camera pulls
  back to the whole window and the spotlight lets go, so the result is seen
  whole. While it is back, the camera drifts in by `DRIFT` (1.5% a second),
  so a held shot is not a frozen one. From just before the outro, the
  camera shows the whole window and the spotlight is off.
- The pointer travels on a slight arc, eased in and out. It hides from the
  first character typed after its last move, as macOS hides it, and comes
  back with the next move.
- A scene with `beats` lasts exactly `beats` beats: no half bar, no
  reading time, no line lengthens it. A cut's events are placed as ever
  from its start, then their times (and the pointer's trips) are scaled
  into all but the last 15% (`TAIL`), so the result shows; events that
  take less keep their pace. An intro with `beats` lasts that, and its
  line starts at 0.2 s and may run on past it; an outro with `beats` lasts
  that.
- Hard cuts (`cuts` in the timeline): wherever a fixed-length intro or
  scene ends and a fixed-length scene or the outro starts. In a cut the
  camera is set straight, at its first frame, on the framing of all the
  cut's events (zoomed 1.18 at most) and pushes in by `PUSH` (6%) over
  it, so it never passes 1.25 and never springs across the cut.
- In a cut, typing is heard a key every 0.05 s at most, and a keycap goes
  with the cut. Flash cards ring no bell. A reel (`reel` in the timeline)
  has a `hit` on the outro.
- It is tested with `node --test` (`contrib/promo/director.test.js`):
  beats, bars, reading time, the typing's seed, the pointer, the dead zone,
  the wide shot, the spotlight, the reveal, the typing span, the pause
  after typing, the spring, the keycaps and their sounds, that the same
  input gives the same timeline, the lines' times and the scenes they
  lengthen or leave alone, the intro's and outro's lines, the
  subtitles' pieces and times, and of a reel: exact lengths, a cut's
  events inside it and in order, a cut never slowed, the camera with no
  spring across a cut and its push-in, flash cards in a row and the hit,
  the hook's length and words, a cut's keycap and its thinned keys, and
  that a scenario without beats is no reel.

### 3. The composer: `contrib/promo/index.html`, `render.cjs`, `audio.js`

`index.html` draws any time of the timeline with `renderAt(seconds)`, with
no clock of its own. `render.cjs` asks headless Chrome for every frame and
pipes them to ffmpeg, as on the old branch.

The picture:

- The stage: near-white, with a breath of blue at the top.
- The window: the app's frame for that moment, 1440 by 900 pixels at the
  top of the picture, above a 156 pixel band kept for the subtitles, so a
  subtitle never covers the app's bottom bar. It has a soft natural shadow
  and traffic lights, and is moved by the director's camera; a zoomed-in
  point comes to the middle of the space above the band.
- The pointer: a macOS-style cursor, drawn by the composer where the
  director puts it. It presses with a blue ripple.
- Keycaps: a `command`'s keys spring in with a small overshoot on its
  sound, and stay 0.8 s: brand blue with a white rim, which reads on the
  light app. A keycap hangs off the left of the caption, which holds
  still in the middle; without a caption, the keycap takes the middle,
  above the subtitles when there are some.
- Subtitles: white words in a dark, rounded, slightly translucent box at
  the bottom, a piece at a time, fading in with the line and out after it.
- Captions, in a scenario with no lines: the words rise one after another
  through a mask with a short blur, in the same dark box.
- Cards ride a full-bleed blue curtain, the icon's gradient (`#3b82f6` to
  `#1d4ed8`), with bold white type, and the icon in white. The intro (the
  icon, the scenario's title, "A small, fast, native database client.")
  builds piece by piece on beats, then its curtain lifts away and the
  window settles in under it. The outro's curtain rises over the app: the
  icon, "Download Tabletist. It's free.", the engines and the platforms
  smaller, and `igor.works/tabletist` in a white pill. Tabletist is
  MIT-licensed and free to download.
- In a reel: at a hard cut no curtain moves, and nothing (the camera, the
  pointer) is blended across it, even in `--final`'s subframes. The hook
  is the intro card in bigger type (172 px), with the intro's `title` and
  `text`. A label is 76 px bold white in a blue pill at the lower left,
  under the window (above a subtitle, in a scene that has one, so it hides
  none of the cut), snapping in from 1.12 and a blur to sharp in 0.15 s; a
  keycap stands beside it. A flash card is one 176 px phrase on the
  curtain, landing with a punch from 1.06. The outro lands on the hit: its
  icon and big line at once, the rest a quarter beat apart. The hook and
  the end card push in by 4% while they are up.
- Slides: the curtain rises over the app as for the outro; the headline
  rises word by word through masks, its line a beat later, both balanced
  on their lines; the curtain lifts away for the next scene.
- Motion blur, with `--final`: every frame is rendered four times, at four
  times the frame rate, and ffmpeg's `tmix` blends each four into one.

The sound is rendered by `audio.js` in the same Chrome, with an
`OfflineAudioContext` at 48 kHz, from the director's timeline, and written
as WAV:

- The voice: each line's WAV, played at its time, dry (not into the room),
  through a highpass at 90 Hz.
- The music, in the scenario's tempo, light and upbeat so it sits under a
  voice: I, V, vi, IV in D (D, A, Bm7, G add9) on an airy pad (detuned
  saws through a lowpass that breathes between 2.2 and 4.8 kHz, and a
  highpass at 280 Hz that leaves the low end alone), a round triangle bass
  on each beat, a crisp pluck (a square and a saw an octave apart, its
  filter snapping shut in a tenth of a second) in eighths up and down the
  chord, a light kick on 1 and 3, a clap on 2 and 4 (three quick bursts
  of noise and a short tail), and hats on the off-beats, through a room and
  a gentle compressor. The pad is alone under the intro, the rest comes in
  from the first scene, the drums and the plucks rest under a slide, and
  the last chord rings out over the outro.
- The effects, on the events: a key click for each character and key, each
  a little different in pitch and level; a mouse click; a low tick under a
  keycap; a soft bell as a slide comes in; a bell on the outro. The
  camera's moves make no sound. The clicks
  are loud enough to stand out over the music (their taps at 0.8 and 0.45,
  through effects at 0.9 against the music's 0.55).
- A reel builds from its first cut to the outro: the music grows from 3 to
  9 dB louder (and stays 7 dB up under the end card), the pad's lowpass
  opens to 2.6 times, from the first flash card the kick is on every beat
  and the hats in sixteenths, a snare roll runs through the flash cards
  (eighths, sixteenths in the last three beats, thirty-seconds in the
  last), a riser (noise through a band sweeping up) runs under the hook
  into the first cut and over the last two bars into the outro, stopping
  dead on it. The outro's hit: a deep kick, a crash of bright noise a side
  each, and the bell; then the last chord rings out. The voice sits 4 dB
  lower and the music goes only 5 dB down under it, and a fast, hard
  compressor keeps the louder mix's peaks under full scale before
  loudness is set.
- The music goes down 10 dB under the voice, ramping over a quarter of a
  second before a line and half a second after it, and stays down between
  lines less than a second apart. Before loudness is set, the voice sits
  about 8 dB over the music, 18 dB over it while ducked.
- The music ducks under each effect. ffmpeg normalizes the mix to −14 LUFS
  (`loudnorm`), the level the social platforms play at, and writes it as
  AAC at 192 kb/s into the MP4.
- Its random numbers come from the same seeded generator. No node's input
  takes more than two connections: Chrome sums the connections into one
  input in an order that changes from run to run, and a sum of three or
  more floating point numbers depends on it. So the same scenario always
  sounds the same, to the sample.

## The data

The runner writes the Bookshop to `/tmp/bookshop.sqlite` (the temporary
directory off Unix) with `tabletist_db::fixtures::write_sqlite_bookshop`
when the scenario asks for it, removing the last run's file first (the
helper refuses one that exists). The History tab shows a statement's
database as the file's whole path, so it is a short one that names no
person's checkout. There is one Bookshop to keep, and no `sqlite3`
binary is needed.

The query history is kept in memory for the run, not in a file: each
statement the app adds is kept, and the History tab reads a connection's
newest first, as from the file. Clearing and trimming it do nothing.

## Testing

- Rust, in the normal suite: the scenario's parsing and its errors (an
  empty `say`, `voice`, `label` or intro `title` or `text`, `beats` that
  is not a positive multiple of 0.5, a label on a slide, an unknown step, an unknown or unbuilt command id, a `key` that is not one
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

A recorded human voice, the Omarchy look, vertical (9:16) videos, a music track from a
file, and scenarios drafted by a model from prose.

## Order of the work

Three checkpoints, each tested before the next: the scenario and the
runner, writing frames and `events.json`; the director and its tests,
with the composer drawing the picture; the sound.
