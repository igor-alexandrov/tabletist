# Tabletist

A fast, native database client for **PostgreSQL**, **MySQL** and
**SQLite**. Written in Rust with egui; runs on
Linux (Omarchy and Hyprland first), macOS and Windows.

## What it does

- Saved connections grouped in a picker, each tagged with its environment
  (dev, staging, production) and when it was last used.
  Open more than one and they share a tab bar.
- Direct connections, TLS (libpq's `sslmode` values, with `allow` read as
  `prefer`; `verify-ca` needs a CA file, and MySQL has no `verify-ca` yet),
  and SSH tunnels (password, key file or agent) with host keys trusted on
  first use. The agent is the one `~/.ssh/config` names for the host
  (`IdentityAgent`, as for 1Password), else `SSH_AUTH_SOCK`.
- Passwords live in the system keyring, or are asked for once per
  connection tab (reconnecting in that tab reuses them).
- A sidebar of recent objects and one schema's tables and views, folded into
  prefix groups (`book_`) or listed flat; each table opens with a data grid
  (keys, foreign keys, value tags, JSON at a glance), a row panel showing
  every field in full with a jump along foreign keys, and a Structure view
  (columns, indexes, foreign keys).
- Server-side sorting and paging, a filter bar with a raw WHERE option, exact
  counts on demand, and cancel for any running query.
- Quick open (Cmd/Ctrl+P) and a full keyboard map: press `?` in the app.
- Looks native on each platform: a macOS look in IBM Plex, and on Linux the
  Omarchy look (square, keyboard first, vim keys, the desktop's monospace
  font throughout). It follows the Omarchy theme live on Omarchy, and the
  system light/dark setting elsewhere.

## Install

- **Arch Linux / Omarchy:** `yay -S tabletist-bin` (release binary) or
  `yay -S tabletist` (built from source).
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

The Linux release binaries, in the `.tar.gz` and in `tabletist-bin`, are
built on Ubuntu 24.04 and need glibc 2.39 or newer (Ubuntu 24.04, Debian 13,
Fedora 40 or later). On an older system, [build from
source](#build-from-source); on an Arch-based one, `yay -S tabletist` does
that for you.

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
