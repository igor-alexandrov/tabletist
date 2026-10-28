---
title: Download
description: Download Tabletist for macOS, Windows or Linux, with the install steps for each.
nav_order: 1
---

The newest stable version is on the
[latest release](https://github.com/igor-alexandrov/tabletist/releases/latest)
page. Every version, pre-releases included, is on the
[releases page](https://github.com/igor-alexandrov/tabletist/releases).
Each release lists its files with a `checksums.txt`, which you can use to
check that a download was not damaged or changed, and `checksums.txt.sig`,
its signature by the Tabletist release key.

## macOS

Download `tabletist-v<version>-macos-universal.dmg`. It is one file for both
Apple Silicon and Intel. Open it and drag **Tabletist** to **Applications**.

### First open on macOS

Releases are signed with a Developer ID and notarized by Apple, so macOS
opens them like any other app. Older releases were not, and macOS blocks
them on first launch. For one of those: try to open it once, then click
**Open Anyway** in **System Settings → Privacy & Security**, or run:

```sh
xattr -dr com.apple.quarantine /Applications/Tabletist.app
```

## Windows

Download and run `tabletist-v<version>-x86_64-pc-windows-msvc-setup.exe`, or
the `aarch64` one on an ARM PC. The installer does not need administrator
rights.

To run Tabletist without an installer, download the `.zip` for your PC,
extract it and open `tabletist.exe`.

## Linux

### Arch Linux and Omarchy

The AUR packages (`tabletist-bin` and `tabletist`) are not published yet.
Until they are, use the AppImage to try Tabletist, and the release archive
to install it with an entry in the app launcher.

### Try it: the AppImage

Download `tabletist-<version>-x86_64.AppImage`, or the
`aarch64` one. It is one file that runs without installing anything. Make it
executable and run it:

```sh
chmod +x tabletist-<version>-x86_64.AppImage
./tabletist-<version>-x86_64.AppImage
```

An AppImage needs FUSE, which most desktops have. Without it, run it with
`--appimage-extract-and-run`.

The AppImage adds nothing to the app launcher. To start Tabletist from
there, install it from the release archive instead.

### Install it: the release archive

Download `tabletist-v<version>-x86_64-unknown-linux-gnu.tar.gz`, or the
`aarch64` one. It holds the `tabletist` binary, a `.desktop` file and the
icon. Unpack it and put the three where your desktop looks for them:

```sh
tar xzf tabletist-v<version>-x86_64-unknown-linux-gnu.tar.gz
cd tabletist-v<version>-x86_64-unknown-linux-gnu
install -Dm755 tabletist ~/.local/bin/tabletist
install -Dm644 packaging/linux/dev.tabletist.Tabletist.desktop -t ~/.local/share/applications
install -Dm644 packaging/linux/dev.tabletist.Tabletist.svg -t ~/.local/share/icons/hicolor/scalable/apps
```

With the `aarch64` archive, write `aarch64` for `x86_64` in the first two
lines.

Tabletist is then in the app launcher (Super+Space on Omarchy), with its
icon. The launcher entry runs `tabletist` from your `PATH`, which on Omarchy
holds `~/.local/bin`. On a desktop whose launcher does not look there, put
the binary in `/usr/local/bin` instead, which needs root:

```sh
sudo install -Dm755 tabletist /usr/local/bin/tabletist
```

To update, install the new binary the same way: **Settings › Updates**
(`:update` in the Omarchy look) checks GitHub for a newer version, every
day unless you choose every week or only when you ask, and says when
there is one. To remove Tabletist, delete the three files.

The Linux binaries are built on Ubuntu 24.04 and need glibc 2.39 or newer
(Ubuntu 24.04, Debian 13, Fedora 40 or later). On an older system,
[build from source]({% link _reference/build-from-source.md %}).

## Next

[Open Tabletist and make your first connection]({% link _guide/getting-started.md %}).
