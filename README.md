<p align="center">
  <img src="assets/icon/tabletist.svg" width="128" height="128" alt="Tabletist icon">
</p>

<h1 align="center">Tabletist</h1>

<p align="center">
  <a href="https://github.com/igor-alexandrov/tabletist/actions/workflows/ci.yml"><img src="https://github.com/igor-alexandrov/tabletist/actions/workflows/ci.yml/badge.svg?branch=main" alt="CI"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-blue" alt="License: MIT"></a>
  <a href="rust-toolchain.toml"><img src="https://img.shields.io/badge/dynamic/toml?url=https%3A%2F%2Fraw.githubusercontent.com%2Figor-alexandrov%2Ftabletist%2Fmain%2FCargo.toml&query=%24.workspace.package.rust-version&label=rust&logo=rust&color=orange" alt="Rust version"></a>
  <img src="https://img.shields.io/badge/platform-Linux%20%7C%20macOS%20%7C%20Windows-lightgrey" alt="Platforms: Linux, macOS, Windows">
  <img src="https://img.shields.io/badge/databases-PostgreSQL%20%7C%20MySQL%20%7C%20SQLite-336791" alt="Databases: PostgreSQL, MySQL, SQLite">
</p>

A fast, native database client for **PostgreSQL**, **MySQL** and
**SQLite**. Written in Rust with egui; runs on
Linux (Omarchy and Hyprland first), macOS and Windows.

<p align="center">
  <img src="assets/screenshots/macos.png" width="900" alt="Tabletist 0.1.0, the macOS look: a table's data grid with the sidebar of tables on the left and the row panel on the right">
</p>

<p align="center">
  <img src="assets/screenshots/omarchy.png" width="900" alt="Tabletist 0.1.0, the Omarchy look: the same table in a dark, square, keyboard-first layout with key hints along the bottom">
</p>

## What it does

- Saved connections grouped in a picker, each tagged with its environment
  (dev, staging, production) and when it was last used.
  Open more than one and each is a chip in the window's header: click one
  to switch.
- Direct connections, TLS (libpq's `sslmode` values, with `allow` read as
  `prefer`; `verify-ca` needs a CA file, and MySQL has no `verify-ca` yet),
  and SSH tunnels (password, key file or agent) with host keys trusted on
  first use. The agent is the one `~/.ssh/config` names for the host
  (`IdentityAgent`, as for 1Password), else `SSH_AUTH_SOCK`.
- Passwords live in the system keyring, or are asked for once per
  connection tab (reconnecting in that tab reuses them).
- A connection opens read-only when its "Open read-only" box says so, which
  is the default for production. A writable one opens a read-write session,
  in which browsing, a raw WHERE and the SQL editor still only read.
- On a writable connection a table's values are edited in its grid: changes
  stay pending until Save writes them in one transaction, which never
  overwrites a row someone else changed. Before saving, Review SQL shows
  the statements that will run (`:diff` on Omarchy, Cmd/Ctrl+Shift+D
  everywhere) and copies them, and a save to production asks first, with
  them on screen.
- A sidebar of recent objects and one schema's tables and views, folded into
  prefix groups (`book_`) or listed flat; each table opens with a data grid
  (keys, foreign keys, value tags, JSON at a glance), a row panel showing
  every field in full with a jump along foreign keys, and a Structure view
  (columns, indexes, foreign keys).
- Server-side sorting and paging, a filter bar with a raw WHERE option, exact
  counts on demand, and cancel for any running query. MySQL sessions run in
  utf8mb4, with `ANSI_QUOTES`, the combination modes that imply it and
  `NO_BACKSLASH_ESCAPES` turned off: in a raw WHERE `"..."` is a string and
  names take backticks.
- A SQL editor per connection (Cmd/Ctrl+T): run the statement at the cursor
  (Cmd/Ctrl+Return) or the whole script, with a row limit and a timeout, and
  read a result row in full in the row panel.
  Every run happens in a read-only transaction that is rolled back, and
  statements that would leave it are refused. Format (Cmd/Ctrl+Shift+F)
  lays queries out in river style and uppercases reserved words, in the
  selection's statements or the whole script. Keywords, schemas, tables,
  views and the columns of a statement's tables are completed while typing
  (Ctrl+Space or Cmd/Ctrl+I asks for the list anywhere).
- Quick open (Cmd/Ctrl+P) and a full keyboard map: press `?` in the app.
- Looks native on each platform: a macOS look in IBM Plex, and on Linux the
  Omarchy look (square, keyboard first, vim keys, the desktop's monospace
  font throughout). It follows the Omarchy theme live on Omarchy, and the
  system light/dark setting elsewhere.

## Install

- **Arch Linux / Omarchy:** the AUR packages (`tabletist-bin` and
  `tabletist`) are not published yet. Until they are, use the release
  `.tar.gz` (see Other Linux below).
- **macOS:** download `tabletist-v<version>-macos-universal.dmg` from the
  [releases](https://github.com/igor-alexandrov/tabletist/releases) and drag
  Tabletist to Applications. Releases are signed and notarized only when
  they are built with the Apple signing secrets; macOS blocks an unsigned
  build on first launch. For an unsigned build only: try to open it once,
  then click Open Anyway in System Settings → Privacy & Security, or run
  `xattr -dr com.apple.quarantine /Applications/Tabletist.app`.
- **Windows:** run `tabletist-v<version>-x86_64-pc-windows-msvc-setup.exe`
  (or the `aarch64` one on ARM). No administrator rights needed.
- **Other Linux:** the release `.tar.gz` holds the binary, a `.desktop` file
  and the icon.

The Linux release binaries are built on Ubuntu 24.04 and need glibc 2.39 or
newer (Ubuntu 24.04, Debian 13, Fedora 40 or later). On an older system,
[build from source](#build-from-source).

## Build from source

Requires the Rust toolchain pinned in `rust-toolchain.toml` (rustup installs
it on the first build). On Linux you also need the Wayland, xkbcommon and GL
development headers.

```bash
cargo build --release
./target/release/tabletist           # or: --demo for a sample database
```

## Omarchy

On Omarchy the app follows the current theme and recolors when you switch
themes. `contrib/omarchy/tabletist.json.tpl` maps an Omarchy theme onto
Tabletist's palette.

## Development

See [AGENTS.md](AGENTS.md) for the test suites (including PostgreSQL, MySQL
and SSH integration tests against `compose.yaml`) and the release process.

## License

MIT, see [LICENSE](LICENSE).
