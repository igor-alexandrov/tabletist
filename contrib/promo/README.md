# Promotional videos from scenarios

A scenario is a TOML file: captions and steps, with no times in it. The
command turns it into a 1920x1080, 60 fps MP4 with music and sound effects.
The design is `docs/superpowers/specs/2026-10-24-promo-video-scenarios-design.md`.

Scenarios live in `contrib/promo/scenarios/`.

## The command

    contrib/promo/make contrib/promo/scenarios/filtering.toml [--final] [--still <seconds>...] [--frames-only] [--reuse]

It writes `target/promo/filtering.mp4` and keeps its working files in
`target/promo/filtering/`. It installs `puppeteer-core` into
`target/promo/node_modules` on its first run, so the repository has no
`package.json` and Cargo has no new crate. It needs ffmpeg, Node and Chrome
(`CHROME` names another one).

- `--final`: motion blur, four frames rendered and blended into each. A
  draft takes about a quarter of the time.
- `--still 3.5 7`: those moments as PNGs, beside the video's path, instead
  of the video.
- `--frames-only`: stops after the app's frames.
- `--reuse`: keeps the app's frames from the last run, for work on motion or
  sound only. It never changes what the steps do: to try a changed step,
  run without it.

Renders need a GPU and Chrome, so they run on a Mac, by hand.

## The scenario

```toml
title    = "Filtering"     # the end card says it; the video is named after the file
database = "bookshop"      # the Bookshop fixture, or a path to a SQLite file
tempo    = 128             # the music's beats per minute; a scene ends on a half bar

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

The filter bar opens on the table's first column with `=`, so the second
scene picks `title` and `contains` from its two dropdowns first ("title"
is also a column header: the menu's item is taken, see Labels).
`scenarios/filtering.toml` is the whole video this example starts.

- `database`: `bookshop` (the default) is the Bookshop the site's
  screenshots and the MCP tests use. A path, relative to the scenario, names
  another SQLite file. Every SQLite session is read-only: nothing is written
  to it.
- `tempo`: 80 to 160.
- A scene has a `caption` (optional: a scene without one is a move between
  two) and its `steps`. The intro and outro cards are not scenes: every
  video has them.

The look is macOS, in the light theme. The Omarchy look is not in this.

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
| `command = "<id>"`, `key = "<keys>"`, `times = n` | The command's key goes down, `times` times (1 by default). The id is the one `settings.toml` uses. `key` picks one of the command's keys, spelled as `settings.toml` spells keys; without it, the first. The keycap is shown. |
| `press = "<keys>"`, `times = n` | A key that no command holds where the keyboard is (Enter in a field, Backspace), spelled as `settings.toml` spells keys. |
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
