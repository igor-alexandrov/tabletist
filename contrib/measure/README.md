# Measuring startup, memory and size

`measure.py` takes Tabletist's startup time, memory use and release sizes
the same way each time, so a number quoted anywhere can be taken again by
anyone with the same kind of machine. It needs Python 3 (standard library
only). Startup and memory need a Hyprland session, `hyprctl` and `grim`
1.5 or later; sizes need `gh`.

The app opens on a headless output the script adds, far from the real
monitors, through a window rule that keeps it off the screen, away from
the keyboard and without Hyprland's open animation. The output and the rule
are removed when the run ends, and so is every demo profile the run left
behind. The pictures the script compares are the window's own buffer
(`grim -T`, through `ext-foreign-toplevel-list`), not the output: what the
app drew, with no wallpaper, animation or lock screen in it. So it works
with the session locked too, as it was for the results below.

## What each number is

**Startup** (`measure.py startup -- <command>`) launches the command again
and again and reports, for each launch:

- *first frame*: from the moment the script starts the process to
  Hyprland's `openwindow` event. A Wayland window is mapped when its first
  frame is committed, so this is launch to first frame.
- *settled*: from the launch to the first frame of the picture the window
  then keeps. The script captures the window over and over (about every
  17 ms, printed with each run) and takes the first capture whose picture
  stays the same for the next 0.5 s. With `--demo` that picture is the
  demo's `users` table, loaded and drawn. Captures start once the window is
  mapped and found (a few milliseconds after its first frame), so when the
  first capture is already the final picture (`settled_on_first_capture`),
  the window settled then or earlier.

Each run's first launch is a warm-up and is not counted (`--warmup`). The
report is the median, the minimum and the maximum of the counted runs, in
JSON on standard output, with the command and the SHA-256 of the file it
ran. `--shot` saves the last run's settled picture, to check it is the app.

`--demo` does more before its first frame than a normal start: it writes a
fresh SQLite file holding the demo's data (a 100,000-row table among them)
into a throwaway profile, and connects to it.

*Warm* runs one launch after another, so the app's files are in the page
cache. *Cold* (`--cold`) drops the launched file from the page cache before
each run (`posix_fadvise(POSIX_FADV_DONTNEED)`, no root needed; `fincore
<file>` shows it worked). It cannot drop the shared libraries other
programs have mapped (Mesa, Wayland, fontconfig) or the system fonts, so a
cold run is "the app's file is not cached", not a first start after a
reboot. Keep the files on a disk: a file on tmpfs cannot be dropped.

**AppImage mount** (`measure.py mount <appimage>`) times the AppImage
runtime alone: from launching it with `--appimage-mount` to the line where
it prints its mount point. An AppImage's startup is roughly that plus the
app's startup from the mounted image, where each page of the binary is
read through FUSE and decompressed on first use.

**Memory** (`measure.py memory -- <command> --demo`) samples the window's
process (for an AppImage, the app, not the runtime) at three points, each
after the picture settles and two more seconds:

1. idle on the demo, its `users` table open;
2. after opening the 100,000-row table `big` (quick open, `big`, Enter),
   which loads its first page of 1,000 rows;
3. after 99 pages forward (Ctrl+Alt+Right, one page of 1,000 rows each,
   0.2 s apart), at rows 99,001 to 100,000. The largest RSS seen while
   paging is kept too (`peak_rss_while_paging`).

The keys are sent to the window with Hyprland's `send_shortcut`, which does
not move the keyboard focus. Each sample is read from
`/proc/<pid>/smaps_rollup`:

- *RSS*: resident memory, shared library pages counted in full.
- *PSS*: resident memory with each shared page divided among the processes
  that map it. The fairest single number for "how much this app costs".
- *private*: pages only this process has (`Private_Clean + Private_Dirty`).

and GPU memory from the `fdinfo` of the process's `/dev/dri` files
(`drm-memory-*`, counted once per DRM client), which the three above do not
include. On an integrated GPU that memory is system memory too (`gtt`).

**Other apps.** `startup` and `memory` take any Wayland or XWayland app:
`--class` names its window class (a regular expression), `--env NAME=VALUE`
adds to its environment, and `--fresh-profile` gives each launch empty XDG
config, data, cache and state folders, removed afterwards, so every app
starts as on its first run and none writes to your own. `--seed DIR` copies
settings into each of those profiles first (`profiles/` holds the ones used
below). `--evict` takes a folder too, for an app of many files. `memory
--idle-only` takes one sample at the first screen, with no keys; `memory
--steps` drives an app with keys, typing, waits, samples and pictures, as
`run_steps` in the script describes. Each memory sample also adds up the
whole process tree (`tree`): an Electron app is several processes. Add up
PSS, not RSS: RSS counts a shared page once in every process that maps it.
A process that leaves the tree (a daemon that forks away, such as the
AppImage runtime's FUSE server or a crash reporter) is not counted.

**Sizes** (`measure.py sizes <tag>`) downloads a release's assets with `gh`
and reports each one's size, and the size of the binary inside each Linux
`.tar.gz` and Windows `.zip`. The macOS `.dmg` is not opened.

## Running it

    ~/.cargo/bin/cargo build --locked --release
    contrib/measure/measure.py startup --runs 20 -- target/release/tabletist --demo
    contrib/measure/measure.py startup --runs 20 --cold -- target/release/tabletist --demo
    contrib/measure/measure.py startup --runs 20 -- ./tabletist-0.5.0-x86_64.AppImage --demo
    contrib/measure/measure.py mount --runs 20 ./tabletist-0.5.0-x86_64.AppImage
    contrib/measure/measure.py memory -- target/release/tabletist --demo
    contrib/measure/measure.py sizes v0.5.0

Run nothing else heavy meanwhile: a build on every core doubles the spread.
`python3 -m unittest discover contrib/measure` tests the script's
arithmetic and parsing.

## Results, 2026-11-09

The raw reports are in `results/2026-11-09/`, one JSON file per command.

The machine: AMD Ryzen AI MAX+ 395 (16 cores, 32 threads) with its Radeon
8060S graphics, 128 GB of memory, a WD_BLACK SN7100 NVMe disk (btrfs),
Omarchy on Linux 7.2.15, Hyprland 0.56.2, Mesa 26.2.5. The headless output
was 1440x900 at scale 1, so the window was 1416x850. The session was locked
(Omarchy locks after its automatic login), which the window captures do not
see. Load average 0.1 to 0.8 throughout.

The builds:

- v0.5.0 from its release (published 2026-11-07): the x86_64 AppImage
  (SHA-256 `82d05d49…`) and the binary of the x86_64 `.tar.gz`
  (`ba1116f9…`), as `checksums.txt` lists them.
- `main` at `e6b1577f`, built here with `cargo build --locked --release`
  (`814c085d…`).

### Startup, `--demo`

20 counted runs each after one warm-up; median, then minimum to maximum.

| Build | Cache | First frame | Settled |
|---|---|---|---|
| v0.5.0 AppImage | warm | 163 ms (158 to 169) | 191 ms (186 to 198) |
| v0.5.0 AppImage | cold | 174 ms (166 to 178) | 202 ms (194 to 206) |
| v0.5.0 binary (`.tar.gz`) | warm | 108 ms (105 to 113) | 136 ms (132 to 140) |
| v0.5.0 binary (`.tar.gz`) | cold | 126 ms (123 to 129) | 154 ms (151 to 157) |
| `main` at `e6b1577f` | warm | 108 ms (103 to 116) | 136 ms (131 to 144) |
| `main` at `e6b1577f` | cold | 125 ms (121 to 422) | 153 ms (148 to 460) |

In no run was the first capture already the final picture: the demo's
table lands one or two frames after the first. The settled picture is the
`users` table with its five rows. The warm AppImage and `.tar.gz` runs read
their files from tmpfs; the cold ones, and a second warm AppImage run
(`startup-appimage-disk-warm.json`: 161 ms and 189 ms), from the disk.

The AppImage costs about 55 ms more than the same binary unpacked. Its
runtime mounts the image in 3.6 ms warm (2.8 to 4.5) and 7.0 ms cold (6.4
to 11.0), 20 runs each, so most of the difference is the binary being read
through FUSE and decompressed as it starts.

An earlier figure, "about 1.6 s to the first settled frame" for v0.4.0's
AppImage, came from `--demo-shot`, which takes its picture on a fixed
1.5 s timer. It measured the timer, not the startup.

### Memory, `--demo`

Three runs each; the median of the three. MiB is 1,048,576 bytes.

| Build | Step | RSS | PSS | Private | GPU (GTT) |
|---|---|---|---|---|---|
| v0.5.0 AppImage | idle on the demo | 129.3 MiB | 85.6 MiB | 76.0 MiB | 62.6 MiB |
| v0.5.0 AppImage | `big` opened (rows 1 to 1,000) | 129.9 MiB | 86.2 MiB | 76.7 MiB | 62.6 MiB |
| v0.5.0 AppImage | `big` after 99 pages (rows 99,001 to 100,000) | 132.8 MiB | 89.1 MiB | 79.6 MiB | 55.3 MiB |
| v0.5.0 binary | idle on the demo | 131.7 MiB | 87.9 MiB | 78.4 MiB | 62.6 MiB |
| v0.5.0 binary | `big` opened (rows 1 to 1,000) | 132.3 MiB | 88.6 MiB | 79.1 MiB | 62.6 MiB |
| v0.5.0 binary | `big` after 99 pages (rows 99,001 to 100,000) | 135.2 MiB | 91.5 MiB | 81.9 MiB | 55.3 MiB |
| `main` | idle on the demo | 132.3 MiB | 88.6 MiB | 79.1 MiB | 62.6 MiB |
| `main` | `big` opened (rows 1 to 1,000) | 132.9 MiB | 89.2 MiB | 79.7 MiB | 62.6 MiB |
| `main` | `big` after 99 pages (rows 99,001 to 100,000) | 135.6 MiB | 91.9 MiB | 82.4 MiB | 55.3 MiB |

The grid holds one page of 1,000 rows at a time, so paging through the
100,000-row table adds about 3 MiB; the largest RSS seen while paging was
the one at the end in every run. VRAM was 0: on this integrated GPU the
app's GPU memory is GTT, system memory the driver maps for it. The AppImage
figures are the app's process; the AppImage runtime, which serves the
image over FUSE, is a process of its own and not counted.

### Sizes, v0.5.0

| Asset | Download | Binary inside |
|---|---|---|
| `tabletist-v0.5.0-x86_64-unknown-linux-gnu.tar.gz` | 19,286,377 bytes (18.4 MiB) | 45,268,096 bytes (43.2 MiB) |
| `tabletist-0.5.0-x86_64.AppImage` | 18,606,584 bytes (17.7 MiB) | |
| `tabletist-v0.5.0-aarch64-unknown-linux-gnu.tar.gz` | 18,045,583 bytes (17.2 MiB) | 38,910,992 bytes (37.1 MiB) |
| `tabletist-0.5.0-aarch64.AppImage` | 17,476,104 bytes (16.7 MiB) | |
| `tabletist-v0.5.0-x86_64-pc-windows-msvc.zip` | 15,479,905 bytes (14.8 MiB) | 35,911,168 bytes (34.2 MiB) |
| `tabletist-v0.5.0-x86_64-pc-windows-msvc-setup.exe` | 13,591,997 bytes (13.0 MiB) | |
| `tabletist-v0.5.0-aarch64-pc-windows-msvc.zip` | 14,414,653 bytes (13.7 MiB) | 31,119,872 bytes (29.7 MiB) |
| `tabletist-v0.5.0-aarch64-pc-windows-msvc-setup.exe` | 12,556,123 bytes (12.0 MiB) | |
| `tabletist-v0.5.0-macos-universal.dmg` | 36,571,091 bytes (34.9 MiB) | |

The Linux binaries are stripped. A release build of `main` at `e6b1577f`
made here is 45,496,208 bytes.

### First start, against Beekeeper Studio and TablePro

The same machine and method, the same day: each app's first start in an
empty profile (`--fresh-profile`), to its first frame, to its settled first
screen, and its memory idle there. No app opened a database: Tabletist and
TablePro show their empty connection lists, Beekeeper its welcome page
(with its bundled demo connection listed, not opened) and a welcome toast.
The raw reports are in `results/2026-11-09-first-start/`.

The builds, each the latest release on 2026-11-09:

- Tabletist 0.5.0: the AppImage, and the binary of the `.tar.gz`, as above,
  without `--demo`.
- TablePro for Linux 0.1.0-beta.3 (Rust, GTK 4 and libadwaita; a public
  beta), from `tablepro-0.1.0-beta.3-ubuntu-26.10-amd64.deb`, SHA-256
  checked against its `SHA256SUMS`. Its Flatpak needs Flatpak installed,
  which needs root, so the deb was unpacked and its binary run against the
  system's GTK 4.24.1 and libadwaita 1.10.0, with its GSettings schema
  compiled beside it (`--env GSETTINGS_SCHEMA_DIR=...`).
- Beekeeper Studio 6.2.2 (Electron, Chrome 152), from
  `Beekeeper-Studio-6.2.2.AppImage`, SHA-512 checked against its
  `latest-linux.yml`. Its AppImage runtime needs libfuse2, which this
  system does not have, so the AppImage was unpacked once
  (`--appimage-extract`) and its `AppRun` run from the disk: the files its
  `.deb` and Arch `.pacman` packages install. So compare it with Tabletist's
  unpacked binary. It runs on X11 by default, here through XWayland
  (`--env DISPLAY=:0`); it was also run on Wayland
  (`--ozone-platform=wayland`). A cold run drops all 1,002 of its files
  from the page cache.

Startup, 20 counted runs each; median, then minimum to maximum:

| App | Cache | First frame | Settled |
|---|---|---|---|
| Tabletist 0.5.0, binary | warm | 88 ms (86 to 93) | 89 ms (87 to 93) |
| Tabletist 0.5.0, binary | cold | 108 ms (102 to 120) | 108 ms (103 to 120) |
| Tabletist 0.5.0, AppImage | warm | 135 ms (129 to 142) | 136 ms (129 to 142) |
| Tabletist 0.5.0, AppImage | cold | 146 ms (143 to 155) | 146 ms (144 to 155) |
| TablePro 0.1.0-beta.3 | warm | 228 ms (209 to 328) | 230 ms (221 to 329) |
| TablePro 0.1.0-beta.3 | cold | 257 ms (245 to 270) | 259 ms (248 to 273) |
| Beekeeper Studio 6.2.2, X11 | warm | 1,229 ms (1,220 to 1,252) | 2,137 ms (2,115 to 2,184) |
| Beekeeper Studio 6.2.2, X11 | cold | 1,385 ms (1,364 to 1,405) | 2,305 ms (2,277 to 2,347) |
| Beekeeper Studio 6.2.2, Wayland | warm | 1,336 ms (1,275 to 1,353) | 2,175 ms (2,145 to 2,196) |
| Beekeeper Studio 6.2.2, Wayland | cold | 1,492 ms (1,471 to 1,525) | 2,344 ms (2,318 to 2,369) |

Tabletist's first frame was already its settled picture in every run
(TablePro's in 36 of 40); Beekeeper's window opens and then fills in.

Memory idle at the first screen, three runs each, the median of the three;
the process tree added up:

| App | Processes | PSS | RSS (summed) | Private | GPU (VRAM + GTT) |
|---|---|---|---|---|---|
| Tabletist 0.5.0, binary | 1 | 83.4 MiB | 128.7 MiB | 74.3 MiB | 50.6 MiB |
| Tabletist 0.5.0, AppImage | 1 | 80.5 MiB | 125.8 MiB | 71.5 MiB | 50.6 MiB |
| TablePro 0.1.0-beta.3 | 1 | 111.5 MiB | 147.2 MiB | 105.7 MiB | 32.5 MiB |
| Beekeeper Studio 6.2.2, X11 | 7 | 845.7 MiB | 1,336.3 MiB | 710.9 MiB | 119.7 MiB |
| Beekeeper Studio 6.2.2, Wayland | 7 | 851.1 MiB | 1,362.7 MiB | 714.0 MiB | 185.3 MiB |

The GPU column is the median of each run's VRAM plus GTT; on this
integrated GPU both are system memory.

Sizes, for the same builds:

| App | Download (x86_64) | Unpacked |
|---|---|---|
| Tabletist 0.5.0 | 18,606,584 bytes AppImage; 19,286,377 bytes `.tar.gz` | 45,268,096 bytes (one binary) |
| TablePro 0.1.0-beta.3 | 31,945,920 bytes Flatpak; 35,241,790 bytes `.deb` (Ubuntu 26.10) | 121,383,952 bytes (`du -sb` of the deb's files; the binary is 119,453,688) |
| Beekeeper Studio 6.2.2 | 362,351,631 bytes AppImage; 276,178,024 bytes `.deb`; 242,804,048 bytes `.pacman` | 1,154,776,007 bytes (`du -sb` of the unpacked AppImage) |

What this does not show: how the apps compare with a database open, or on
another machine. TablePro for Linux is a beta, and ran here from its deb,
not as its Flatpak.

### 100,000 rows, against Beekeeper Studio and TablePro

The same builds, machine and day. Each app holds the same 100,000 rows in
its grid, and memory is taken before and after. The rows come from one
SQLite script, `crates/tabletist-db/fixtures/sqlite.sql`, the one
Tabletist's demo uses: its table `big` has 100,000 rows of three columns
(`id`, `label`, `bucket`). TablePro and Beekeeper open a file made from it
(`target/bench/bench.sqlite`, written with Python's `sqlite3` from that
script), given on their command line; Tabletist's `--demo` writes the same
script into a new file of its own.

Each app loads the rows where it can hold all of them, with keys only, and
its settings for that come from a seed in `profiles/` (`--seed`), copied
into each throwaway profile:

- Tabletist 0.5.0 (`--demo`): a SQL tab (Ctrl+T), `select id, label,
  bucket from big`, Ctrl+Enter, then Escape and `a` to show all rows, past
  the editor's limit of 1,000. Its table view pages at most 10,000 rows.
- TablePro 0.1.0-beta.3: its table view, `big` opened from the sidebar
  (Down five times, Enter), with `default-page-size` set to 100,000 (the
  default is 1,000, the most 100,000). Its query results stop at "10,000+".
  The seed also turns off `share-usage-data`, which sends usage events to
  PostHog; it was on, as by default, for the first-start runs above.
- Beekeeper Studio 6.2.2, on Wayland: the query editor it opens with, the
  same query, Ctrl+Enter, with `maxResults` set to 100,000 (the default is
  50,000). Its table view pages 100 rows, and its next page is a click. On
  X11 it did not take the keys sent to it, so it ran on Wayland here. The
  seed also turns off its update check.

After the last key each waits 6 seconds, then for the picture to hold
still, then 2 more seconds, and the memory is taken. The picture of every
run shows all 100,000 rows: Tabletist's footer reads "100,000 rows",
TablePro's "Rows 1 – 100,000", Beekeeper's "100000". Moving to the last
row is left out: the apps' grids could not all be focused with keys from
outside, and where they could (TablePro, Tab three times then Ctrl+End,
during the trials), PSS did not change, as the grids draw only the rows in
view. The raw reports are in `results/2026-11-09-100000-rows/`.

Five runs each; the median of the five, the process tree added up:

| App | Step | PSS | RSS (summed) | Private | GPU (VRAM + GTT) |
|---|---|---|---|---|---|
| Tabletist 0.5.0 | connected, `users` open | 87.9 MiB | 132.7 MiB | 78.8 MiB | 62.6 MiB |
| Tabletist 0.5.0 | 100,000 rows loaded | 103.7 MiB | 148.5 MiB | 94.6 MiB | 57.4 MiB |
| TablePro 0.1.0-beta.3 | connected | 114.7 MiB | 150.6 MiB | 108.9 MiB | 32.5 MiB |
| TablePro 0.1.0-beta.3 | 100,000 rows loaded | 203.8 MiB | 240.0 MiB | 197.6 MiB | 32.6 MiB |
| Beekeeper Studio 6.2.2 | connected | 863.5 MiB | 1,382.5 MiB | 725.4 MiB | 166.1 MiB |
| Beekeeper Studio 6.2.2 | 100,000 rows loaded | 1,449.8 MiB | 1,971.4 MiB | 1,310.4 MiB | 93.0 MiB |

Holding the 100,000 rows added 15.8 MiB of PSS in Tabletist, 89.1 MiB in
TablePro and 586.3 MiB in Beekeeper. The five runs of each, loaded: Tabletist
103.0 to 104.7 MiB, TablePro 203.3 to 204.1 MiB, Beekeeper 1,446.7 to
1,452.4 MiB.

The commands, with `Q` the query typed key by key:

    Q="type:select id, label, bucket from big"
    contrib/measure/measure.py memory --fresh-profile \
      --steps "sample:connected, users table open;CTRL+t;wait:0.8;$Q;CTRL+Return;wait:2;Escape;wait:0.3;a;wait:6;settle;sample:all 100,000 rows loaded" \
      -- ./tabletist --demo
    contrib/measure/measure.py memory --fresh-profile --seed contrib/measure/profiles/tablepro \
      --class '.*[Tt]able[Pp]ro.*' --env GSETTINGS_SCHEMA_DIR=<deb>/usr/share/glib-2.0/schemas --env GSETTINGS_BACKEND=keyfile \
      --steps "sample:connected;wait:1;Down*5;Return;wait:6;settle;sample:all 100,000 rows loaded" \
      -- <deb>/usr/bin/tablepro bench.sqlite
    contrib/measure/measure.py memory --fresh-profile --seed contrib/measure/profiles/beekeeper \
      --class '.*[Bb]eekeeper.*' \
      --steps "sample:connected;wait:1;$Q;CTRL+Return;wait:6;settle;sample:all 100,000 rows loaded" \
      -- <unpacked AppImage>/AppRun bench.sqlite --ozone-platform=wayland
