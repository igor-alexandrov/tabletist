# Promotional videos from scenarios

A scenario is a TOML file: what a voice says, captions and steps, with no
times in it. The command turns it into a 1920x1080, 60 fps MP4: the app in
the light theme, large on a bright stage, a voice saying each scene's line
with subtitles under it, light music beneath, and blue cards to open and
close it, the last one asking you to download Tabletist. The design is
`docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md`.

Scenarios live in `contrib/promo/scenarios/`.

## The command

    contrib/promo/make contrib/promo/scenarios/filtering.toml [--final] [--still <seconds>...] [--frames-only] [--reuse]

It writes `target/promo/filtering.mp4` and keeps its working files in
`target/promo/filtering/`. It installs `puppeteer-core` into
`target/promo/node_modules` on its first run, so the repository has no
`package.json` and Cargo has no new crate. It needs ffmpeg, Node and Chrome
(`CHROME` names another one).

The voice is macOS's own: `say` speaks each line into
`target/promo/<name>/voice/`, so only a Mac renders a scenario's lines.
Elsewhere the video is made without them, as if they were not written. Its voice has to be installed. If it is not, the render stops and
says so: add it in System Settings > Accessibility > Spoken Content >
System voice > Manage Voices. `say -v '?'` lists the installed ones.

- `--final`: motion blur, four frames rendered and blended into each,
  never across a hard cut: a frame whose four moments would cross one
  takes them all from the cut on. A draft takes about a quarter of the
  time.
- `--still 3.5 7`: those moments as PNGs, beside the video's path, instead
  of the video.
- `--frames-only`: stops after the app's frames.
- `--reuse`: keeps the app's frames from the last run, for work on motion or
  sound only. It never changes what the steps do: to try a changed step,
  run without it.

Renders need a GPU and Chrome, and run by hand. On Linux, name the browser:
`CHROME=/usr/bin/chromium`.

## The scenario

```toml
title    = "Filtering"     # the intro card says it; the video is named after the file
database = "bookshop"      # the Bookshop fixture, or a path to a SQLite file
look     = "macos"         # or "omarchy" (see The look)
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

The filter bar opens on the table's first column with `=`, so the second
scene picks `title` and `contains` from its two dropdowns first ("title"
is also a column header: the menu's item is taken, see Labels).
`scenarios/filtering.toml` is the whole video this example starts.

- `database`: `bookshop` (the default) is the Bookshop the site's
  screenshots and the MCP tests use. A path, relative to the scenario, names
  another SQLite file. Every SQLite session is read-only: nothing is written
  to it.
- `tempo`: 80 to 160.
- `voice`: `Ava (Premium)` unless the scenario names another.
- A scene has a `caption` (optional: a scene without one is a move between
  two), a `say` (optional) and its `steps`. The intro and outro cards are
  not scenes: every video has them, and `[intro]` and `[outro]` may each
  give one a `say` and `beats` (see A reel). `[intro]` may also give its
  card a `title` and a `text`.
- `say` is a line the voice says, first person and casual: what you gain,
  not what the button is called. It starts 0.2 s into its shot, as the
  steps start, and the scene lasts until it is said, with 0.4 s to spare.
  It shows as subtitles at the bottom while it is said (not in a reel,
  below), a piece of six to nine words at a time, and a shot that has one
  shows no caption. A scenario with no lines at all keeps its captions,
  and has no voice.
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
  for the next scene. A slide's `say` starts once the headline is up, and
  a slide that has one reads best with no `text`: the subtitles say it.
  Under a slide the drums rest, and a soft bell marks it.

## A reel

A scene can last a fixed number of beats: `beats = 4`, a positive multiple
of 0.5. That is its exact length, whatever its steps, its caption or its
line would take. A scenario made of such scenes is a reel, fast and cut on
the beat, as `scenarios/whats-new.toml` is:

```toml
[intro]
title = "More than 50 new features" # the card's title, instead of the scenario's
text  = "Tabletist 0.3"             # and its tagline
say   = "More than fifty new features."
beats = 8                           # its line may run on into the first cut

[[scene]]
label = "Quick open"                # the feature, big, under the app
beats = 4
steps = [{ command = "find_object" }, { type = "boo" }, { press = "enter" }]

[[scene]]
slide = "Vim mode"                  # a flash card
beats = 1

[outro]
say   = "Get it free."
beats = 8
```

- A shot with `beats` is a cut. Its steps are played as ever, then
  squeezed into it, leaving the last 15% for the result to show: typing
  becomes a flurry, and the pointer moves faster. Steps that take less
  than that keep their pace. A `hold` is squeezed with the rest, so a
  longer one leaves more of the cut to the result. The camera frames the
  result, close: what the steps did in the cut's last 60% (their rects,
  of the SQL editor its left, where the text is), so the end state fills
  the picture. Where none of them has a rect, it frames what they did
  earlier in the cut, and where no step has one (a key with nothing
  focused, as History's), the last change on screen. It is on that
  framing from the cut's first frame, zoomed up to 1.9 times (a narrated
  video stays at 1.25), and pushes in slowly (6%), showing nothing but
  the window. Put a `hold` where the thing the label names should stay
  in sight past the cut's middle.
- A shot's `label` names the feature in big white words in a blue pill at
  the lower left, under the window. It snaps in on the cut, alone: a cut
  shows no keycap.
- A slide with `beats` is a flash card: one big phrase on the blue,
  landing with a small punch.
- Between two scenes of fixed length, and from one to the outro, the
  picture cuts hard: no curtain moves. An intro with `title` is the hook,
  in bigger type; with `beats` it cuts hard to the first scene. The
  hook's words rise through masks one after another, a number in its
  title counts up from 0 and lands just before the voice says it, its
  `text` fades up a beat later, and the card pushes in 8%.
- A reel's lines are heard, not captioned: its cards say the same.
- The sound builds: the drums come in at the first cut, the music grows
  louder to the outro, the pad's filter opens, the kick goes to every beat
  and the hats to sixteenths at the first flash card, a snare roll runs
  through the flash cards (eighths, sixteenths in the last three beats,
  thirty-seconds in the last), and a noise riser climbs into the outro.
  The outro lands on a hit: a deep kick, a crash and the bell, and the
  last chord rings out. The music goes 14 dB down under the voice, and
  the crash has died away 30 dB by the time the end card's line starts,
  0.6 s after the hit.

A scenario with no `beats` sounds and moves as before.

## The look

`look = "macos"` (the default) films the app in the macOS look, in the
light theme, under its title bar. `look = "omarchy"` films it in the
Omarchy look, in Tokyo Night as the site's screenshots show it: no title
bar, and the window is square, in Hyprland's 2 px border of the accent, on
a dark desk. `scenarios/whats-new-omarchy.toml` is the What's new reel in
it.

The steps press the look's keys. On Omarchy many are vim's letters or a
line of the `:` prompt, and a `command` presses them as they are spelled:
`delete_row` is `dd`, `review_sql` types `:diff` and Enter. A `press`
takes the same: `{ press = "==" }` formats the line under the cursor in
the SQL editor's normal mode. A new SQL tab's editor takes typing at once;
Esc goes to normal mode (a second Esc where the completions are open).

## What a video looks and sounds like

- The intro is a full-bleed blue card (the icon's gradient) with the icon,
  the `title` and the tagline in bold white. Its curtain lifts away to the
  app.
- The app fills most of the picture, about 1700 pixels wide, on a light
  stage with a soft shadow. The camera stays plain: it moves in gently
  (1.25 times at most) on what is happening, and back out to the whole
  window when the action stops. Nothing is dimmed.
- Keycaps spring in for a `command` (not in a reel's cuts), above the
  subtitles when there are some. The pointer moves on a slight arc and clicks with a blue ripple.
- The outro is the blue card again: "Download Tabletist. It's free.", the
  engines and the platforms, and `igor.works/tabletist` in a white pill.
- The voice is evened out by a slow, gentle compressor, lifted a little
  round 3 kHz, and given a short, quiet room of its own.
- The music is synthesized, light and upbeat, in the scenario's tempo: an
  airy pad, a round bass on the beat, a crisp pluck, a light kick, a clap
  on beats 2 and 4, and hats. It goes down 10 dB under the voice, and a
  little under each click and key. The mix is set to -14 LUFS in two
  passes (measured, then one gain for all of it), with true peaks at
  -1.5 dBTP at most: where the gain would lift a peak past that, a
  limiter takes the few peaks down first. The same scenario always gives
  the same sound, to the sample.

## Where a video starts

Before the first step, the runner sets the scene up, and none of it is
filmed: a saved connection named after the database (`Bookshop`, or the
file's stem), Dev environment, connected; the tree showing the main schema;
nothing open; the row panel hidden, so the grid has the window's width. The
window is 1200x750 points at scale 2. The setup may apply actions; the steps
never do.

## The steps

| Step | What happens |
|---|---|
| `open = "<object>"` | The pointer goes to the object in the tree and clicks it. |
| `click = "<text>"` | The pointer goes to the node with that label, or else to the painted text (a grid's cell is not a node), and clicks there. |
| `command = "<id>"`, `key = "<keys>"`, `times = n` | The command's keys are pressed, `times` times (1 by default): one chord, a few keys one after another (`dd`), or a `:` line and Enter. The id is the one `settings.toml` uses. `key` picks one of the command's keys in the look, spelled as `settings.toml` spells keys; without it, the first. The keycap is shown. |
| `press = "<keys>"`, `times = n` | Keys that no command holds where the keyboard is (Enter in a field, Backspace, vim's `==`), spelled as `settings.toml` spells keys. |
| `type = "<text>"` | Typed into what has the keyboard, a character at a time. A newline is Enter. |
| `hold = <seconds>` | The picture stays, beyond the scene's own time. |

A scenario has no times in it. The director decides them.

A `press` that the keymap holds where the keyboard is does what the app does
with it, with no keycap. If you want the keycap, write a `command`.

### Labels

- A `click` names an accessibility node's label, or else text painted on
  screen, for example a grid cell's value.
- An `open` takes only an item of the sidebar tree.
- When several nodes share a label, a click takes the only one that sits in
  a popup or menu over the window. Otherwise the run stops, and lists each
  candidate's role and rect. Name another label to tell them apart.

### A step that fails

Every click, key and typed character has to change what is on screen. A
command that does not hold where the keyboard is, a `type` with nothing to
take it, or a label that is not there stops the run.

The message starts with the file, the scene and the step, for example:

    /path/to/contrib/promo/scenarios/overview.toml, scene 2, step 3: the step changed nothing on screen

(Scenes and steps count from 1.) The frames written up to the failing step
are in `target/promo/<name>/frames/`. Open the last one to see where the app
stood.

## Checking a scenario

Start with `--frames-only`: it plays the steps and writes the frames, and
that is where a scenario goes wrong. Once it passes, render. While tuning
the look and the sound, add `--reuse` to skip the steps.

`scenarios/smoke.toml` is a few seconds: an open, a click, a command and a
hold. Use it to check that the pipeline works.
