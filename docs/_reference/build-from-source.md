---
title: Build from Source
description: Build Tabletist with Cargo on Linux, macOS or Windows.
nav_order: 3
---

## What you need

- [Rust](https://rustup.rs), installed with rustup. The toolchain is
  pinned in `rust-toolchain.toml`, and rustup installs it on the first
  build.
- On Linux, the Wayland, xkbcommon and GL development headers. On Ubuntu
  and Debian:

  ```sh
  sudo apt-get install -y libxkbcommon-dev libwayland-dev libgl1-mesa-dev
  ```

## Build

```sh
git clone https://github.com/igor-alexandrov/tabletist.git
cd tabletist
cargo build --release
```

The binary is `target/release/tabletist`. Run it with `--demo` for a
sample database:

```sh
./target/release/tabletist --demo
```

## Contributing

The test suites, the checks a change must pass and the release process are
described in
[AGENTS.md](https://github.com/igor-alexandrov/tabletist/blob/main/AGENTS.md).
Bugs and ideas are welcome in the
[issues](https://github.com/igor-alexandrov/tabletist/issues).
