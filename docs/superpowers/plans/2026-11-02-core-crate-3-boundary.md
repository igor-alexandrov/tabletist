# Moving the lower layer into a crate of its own, run 3: what the boundary breaks

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Fix, inside the one crate, what a crate boundary between the lower layer and the interface would break and `tests/layers.rs` cannot see, so the move (run 4) is a move. This is run 3 of `2026-11-02-core-crate.md`. Behaviour does not change.

**Files:**
- Modify: `Cargo.toml` (`[workspace.package] version`, `version.workspace = true`)
- Modify: `src/model.rs` (`impl TabName`; two doc links become code spans)
- Modify: `src/app/commands.rs` (`impl TabName` goes)
- Modify: `AGENTS.md` (Releasing), `.github/workflows/claude-review.yml` (the prompt's version-bump line)
- Modify: `docs/superpowers/plans/2026-11-02-core-crate.md` (run 3 done, two findings for run 4)

## Survey (2026-11-02, `main` at `99eb9ff3`)

What the plan listed, as `main` has it now:

| What | Where | Across the boundary | Now |
|---|---|---|---|
| The version | `update::CURRENT`, `relay.rs`, `backend/agents.rs` read `env!("CARGO_PKG_VERSION")` | the core's own version, so the update check would compare the wrong number with no error | `version = "0.4.1"` under `[workspace.package]`; `tabletist` takes `version.workspace = true`; the core takes it at the move |
| `impl TabName { fn of(tab: &Tab) }` | `src/app/commands.rs` | an inherent impl outside the type's crate does not compile | in `src/model.rs`, beside `TabName`, as `pub fn of` |
| Doc links to `App` | `model.rs`: `[`crate::app::App::start_matches`]`, `[`crate::app::App::quick_open_list`]` | unresolved, and `cargo doc` runs with `-D warnings` | code spans, as `model`'s other mentions of `App::…` are |

What else the re-survey read for, and what it found:

- **Every inherent impl and trait impl above the layer** (`src/` outside the lower modules, multi-line heads included) against every type and trait the lower modules define, `Command` from `keymap`'s `commands!` among them. 17 hits by name; all but `TabName` are the file's own type of the same name (`Window`, `Line`, `Key`, `Group`, …). No trait impl above names a lower type or trait.
- **Macros.** One in the lower layer, `keymap`'s `commands!`, used only where it is defined; no `#[macro_export]`, no `$crate`. `i18n`'s `gettext`, `ngettext` and `pgettext` are fastframe-i18n's functions, not macros.
- **`env!` in the lower layer.** `CARGO_PKG_VERSION` (above), `OUT_DIR` (`i18n`'s catalogs, which the plan moves with a `build.rs` of the core's) and `CARGO_MANIFEST_DIR` (below). No `CARGO_PKG_NAME`, `DESCRIPTION` or `REPOSITORY`, which would be the core's.
- **Features.** No `cfg(feature = …)` in the lower layer: `shots` is the interface's alone.
- **`#[path]`:** none in the lower layer.
- **Doc links naming what is above:** only `model`'s two.
- **What reads the version outside Rust.** `release.yml` reads the `tabletist` package's version with `cargo metadata`, which gives the inherited one (`0.4.1`, checked). `build.rs` reads `CARGO_PKG_VERSION`, which cargo sets from it. The README's badge reads `workspace.package.rust-version`. No script reads `version` from `Cargo.toml`'s text.
- **The log's filter.** `log_filter` says `tabletist=info` (`tabletist=debug` with `--verbose`), and a log line's target is its module path, so the core's lines will come from `tabletist_core::…`. env_filter 2.0.0 matches a directive by `target.starts_with(name)`, with no `::` boundary, so `tabletist` keeps covering them. Holds; nothing to change.
- **`include_str!`/`include_bytes!`.** As the plan lists, plus `src/fixtures.rs`'s Omarchy template, which run 1 made after the plan's survey. Each is relative to its file and changes only when the file moves: run 4's. Added to the plan's list.

One finding the plan had as a path, which is more than a path:

- **`env.rs`'s tests that read the whole crate.** `sources()` walks `CARGO_MANIFEST_DIR/src`, and four tests read every file through it and name files on both sides of the line: `environment_colours_are_written_only_in_this_module`, `no_view_writes_a_hex_colour`, `environments_are_matched_only_in_this_module` and `only_the_environment_surfaces_ask_for_its_colours` (`src/ui/value_tags.rs`, `src/ui/workspace.rs`, `src/ui/start.rs` and the rest, beside `src/theme.rs`, `src/model.rs`, `src/connections.rs`). In the core, `CARGO_MANIFEST_DIR` is the core's directory: they would read only the core's sources, find more than 20 files, and pass, with the interface no longer checked and no error. This is run 4's, since it depends on where the files land; the plan now says so (the tests go up into the root crate and read both crates' sources by their paths from the workspace root, or `sources()` reads both directories).

## What changed

- **`Cargo.toml`.** `version = "0.4.1"` under `[workspace.package]`, with a comment; the root package says `version.workspace = true`. `Cargo.lock` does not change: cargo resolves nothing for an inherited version, and every gate ran with `--locked`.
- **`AGENTS.md`, Releasing.** The version's one source is `[workspace.package]`, which each package whose version is the app's takes. A release's `Cargo.lock` diff is the `version` line of each such package: `tabletist`, and `tabletist-core` once it exists. The step runs `cargo update --workspace` in place of `cargo update -p tabletist`, so it holds before and after the move. Checked on a scratch bump to `0.4.2`, put back: `cargo update --workspace --offline` changed exactly the `tabletist` entry's `version` line.
- **`claude-review.yml`.** The prompt looks for "a version bump without its `Cargo.lock` lines (one per package that takes the workspace's version)", in place of its one line.
- **`TabName::of`** moved into `model` unchanged, with a doc line, and `pub`: `App` calls it, and after the move `App` is in another crate. `commands.rs` keeps importing `TabName` and `Tab`, which it still names.
- **`model`'s two doc links** to `App::start_matches` and `App::quick_open_list` are code spans.

## What did not change

- **No test was added, moved or removed;** the same 3,743 `#[test]` and `#[tokio::test]` lines under `src/` and `tests/` as `main`.
- **The version is 0.4.1** in `cargo metadata`, `--version` and `Cargo.lock`, as before.

## What this run teaches

- **A test that reads files reads a directory, and the directory is a crate's.** `tests/layers.rs` sees names, and the compiler will see names; neither sees a test that walks `CARGO_MANIFEST_DIR` and passes on what it finds. A boundary changes what such a test covers without failing it, so a survey looks for every `CARGO_MANIFEST_DIR`, not only the ones that build a path.
- **Write the release steps for both sides of the move.** `cargo update --workspace` and "one line per package that takes the workspace's version" are true now and after run 4, so the move does not have to remember the release notes.

## Tasks

### Task 1: the version

- [x] `[workspace.package] version`, `version.workspace = true`; `Cargo.lock` unchanged.
- [x] `AGENTS.md`'s Releasing and `claude-review.yml`'s prompt.

### Task 2: `model`

- [x] `impl TabName` into `model`.
- [x] The two doc links become code spans.

### Task 3: the re-survey

- [x] Impls, macros, `env!`, features, `#[path]`, doc links, the log filter, what reads the version.
- [x] `env.rs`'s source-reading tests and `fixtures.rs`'s include path noted in the plan for run 4.

### Task 4: CI's checks and the PR

- [x] `cargo fmt --all --check`
- [x] `cargo clippy --locked --workspace --all-targets -- -D warnings`
- [x] `cargo test --locked --workspace --all-targets`
- [x] `RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps`
- [ ] Push and open the PR.

## For later

Run 4, the move, comes next, and re-surveys from `main`.
