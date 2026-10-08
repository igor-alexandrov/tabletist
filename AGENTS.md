# Tabletist agent guide

Tabletist is a small, fast, native database client (PostgreSQL,
MySQL, SQLite) on egui/eframe and fastframe. The design lives in
`docs/superpowers/specs/2026-09-27-tabletist-design.md`; batch plans live in
`docs/superpowers/plans/`.

## Architecture

- `src/ui/` draws views and pushes `Action`s onto `app.actions`. `App::apply`
  in `src/app.rs` applies them after drawing. Do not mutate application state
  from inside a view, apart from text a field is editing and the cursor
  position that field reports.
- Database, network, and disk work runs on the backend runtime
  (`src/backend.rs`), never on the UI thread.
- `crates/tabletist-db` has no UI dependencies.
- Code that depends on the database engine matches on `Driver` or `Dialect`
  and names every variant: no `==`, `!=`, `matches!` or `_` arm. A new engine
  then fails to compile wherever nobody has decided for it.
  `tests/engines.rs` finds `==`, `!=` and `matches!`. Nothing finds a `_`
  arm, or a comparison written with `Self::` in the enums' own methods.
- The workspace forbids `unsafe`. AppKit calls that cannot be made without it
  go in `crates/tabletist-appkit`, and SQLite calls in
  `crates/tabletist-sqlite-ffi`, behind a safe API, each with a SAFETY note;
  `src/macos.rs`, `crates/tabletist-db` and everything else stay free of it.
- Platform code sits behind `cfg`. A fix for one platform keeps Linux, macOS,
  and Windows compiling.
- Settings and state files stay readable, backward compatible, and atomically
  written. Never log passwords, passphrases, or connection URLs with secrets.
- Prefer existing dependencies. Explain every crate in `Cargo.toml`.
- egui and winit come from the crmne forks spotifast uses; fastframe crates
  share one tag. Move each group together.

## Checks

    cargo fmt --all --check
    cargo clippy --locked --workspace --all-targets -- -D warnings
    cargo test --locked --workspace --all-targets
    RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps

Database integration tests need servers: `docker compose up -d --build --wait postgres mysql ssh`, then
`TABLETIST_TEST_PG_URL=postgres://tabletist:tabletist@localhost:55432/tabletist TABLETIST_TEST_MYSQL_URL=mysql://tabletist:tabletist@localhost:53306/tabletist TABLETIST_TEST_SSH_URL=ssh://tabletist:tabletist@localhost:52222 cargo test --workspace`.
SSH agent tests also need `TABLETIST_TEST_SSH_AGENT=1` and the test key in a running agent:
`chmod 600 crates/tabletist-db/tests/ssh/id_ed25519 && ssh-add crates/tabletist-db/tests/ssh/id_ed25519`
(then `cargo test -p tabletist-db --lib ssh::tests::the_agent_lists_its_keys -- --ignored`).
Without the variables those tests print "skipped" and pass; CI runs all three suites on Linux.
The native keyring test is `#[ignore]`d; run it by hand on a desktop session.

Views draw text only through `TextRole`s (`src/typography.rs`); a view never
names a font, a family or a size.

Keyboard focus is drawn in one place, `src/ui/focus.rs`, for whatever has the
keyboard. A view never paints a control's focus ring: a widget that needs
another form than the default says so with `focus::hint`. Only a pane (the
tree, a grid) marks where its arrows are by itself: its cursor, its cell.

Add a focused regression test for every behaviour change. UI behaviour is
tested headlessly through `src/testing.rs` (AccessKit tree + events). Do not
weaken a lint, delete a test, or add an `allow` to make CI green without
saying why the rule does not apply.

## Pull requests

`ci.yml`, and `packaging.yml` when packaging paths changed, start on the pull
request themselves. `claude-review.yml` has Claude review the change. The
review is advice: it never approves or requests changes, and nothing waits
for it.

- The paths that start `packaging.yml` are listed twice: in its `pull_request`
  trigger and in its `push` trigger. Change them together.
- A branch of this repository is reviewed when its pull request opens and on
  each push, drafts excepted. A fork is reviewed only when the owner comments
  `@claude review <sha>`, naming the commit they read. `@claude review` alone
  reviews a branch of this repository again.
- The review posts inline comments and one summary comment, which each later
  review rewrites. It reads `AGENTS.md` from the base branch, so a rule the
  review should hold a change to belongs here. `Cargo.lock` and
  `docs/superpowers/` are not reviewed.
- Claude has no shell there and reads nothing outside the workspace. The
  pull request's code is never run. Keep it so: do not give the job another
  secret, a tool that runs commands, or a permission beyond
  `pull-requests: write`.
- It needs the `CLAUDE_CODE_OAUTH_TOKEN` secret (`claude setup-token`), and
  skips with a notice without it. The comment trigger runs the workflow of
  the default branch, so a change to it can be tried only after it is merged.

## Website

The website is a Jekyll site in `docs/`, on jekyll-vitepress-theme. `docs.yml`
builds and checks it on a pull request that touches it, and publishes it to
GitHub Pages from `main`. `docs/superpowers/` is not part of the site:
`docs/_config.yml` excludes it.

    cd docs && bundle install
    bin/check                                 # build and check, as docs.yml does
    bundle exec jekyll serve --livereload     # http://localhost:4000/tabletist/

- Update the pages when user-visible behaviour, settings, files or keys
  change. `_reference/keyboard-shortcuts.md` follows `SHORTCUTS` in
  `src/ui/keys.rs`, and `_guide/macos.md` writes the same keys with Cmd.
  `_guide/omarchy.md` lists the keys of the Omarchy look,
  the ones `SHORTCUTS` leaves out too: the letters in `keys.rs` and the key
  hints its dialogs draw. `_reference/settings-and-files.md` follows
  `src/settings.rs` and `src/paths.rs`.
- The site is served under `/tabletist`. Link to a page with
  `{% link _guide/name.md %}` and to a file with the `relative_url` filter,
  never with a bare `/path/`.
- The icon and the screenshots stay in `assets/`: `docs/_plugins/repo_assets.rb`
  publishes the icon and every picture in `assets/screenshots/`. Do not copy
  them into `docs/`. A page shows one with `{% include shot.html %}`. The
  site's are lossless WebP, rendered from the scenes of `src/shots.rs` (the
  Bookshop data, never a real project's).
- The pages name no version, so a release does not touch them.
- The paths that start `docs.yml` are listed twice, as in `packaging.yml`.
  Change them together.

## Style

- Never use em dashes. Use a full stop, comma, colon, or parentheses.
- Work on `main`, linear history, one topic per commit, each passing checks.
- Report platform coverage honestly: say when something was only compiled.

## Releasing

The version has one source: `version` under `[package]` in the root
`Cargo.toml`. The About window, `tabletist --version`, the log and the
Windows version details all read it at build time, and `release.yml` takes the
package file names from the tag. So the tag follows `Cargo.toml`, never the
other way round.

- The bump lands on `main` before the tag exists. Tag the commit that carries
  it, nothing earlier.
- The tag is `v` plus the version in `Cargo.toml`, character for character
  (`0.2.0-rc1` is tagged `v0.2.0-rc1`). `release.yml` checks this first and
  builds nothing for a tag that disagrees.
- `Cargo.toml` and the `tabletist` entry in `Cargo.lock` change in the same
  commit: every release build is `--locked`. That one line is the whole
  `Cargo.lock` diff. The crates under `crates/` keep their own versions.
- Never create the release, or its tag, in the GitHub UI or with
  `gh release create`. `release.yml` creates the release from the pushed tag.
  One made by hand has no packages, and its tag points at whatever `main` was.
- Never move a tag a release was built from. If a tag went out on the wrong
  commit and nothing was built from it, delete it and tag again; otherwise
  bump to the next patch version.
- A user-facing string that names a version ("read-only in 0.1.0") is not
  bumped with it. Check on each release whether it is still true.

1. Bump `version` in `Cargo.toml`, run `cargo update -p tabletist`, commit,
   and get the commit onto `main`.
2. On that commit: `git tag -s v0.2.2 -m v0.2.2 && git push origin v0.2.2`.
3. `release.yml` builds Linux (x86_64, aarch64 `.tar.gz`), Windows (x64,
   arm64 `.zip` and `-setup.exe`) and macOS (universal `.dmg`), publishes a
   GitHub release with `checksums.txt`, then updates the AUR. A tag with a
   `-` (`v0.2.0-rc1`) is a pre-release and skips the AUR.

Optional secrets (each step skips with a notice without them):

- `APPLE_CERTIFICATE_P12` (base64), `APPLE_CERTIFICATE_PASSWORD`,
  `APPLE_SIGNING_IDENTITY`: sign the app and DMG with a Developer ID.
- `APPLE_ID`, `APPLE_TEAM_ID`, `APPLE_APP_PASSWORD`: notarize and staple.
- `AUR_SSH_KEY`: push `tabletist-bin` and `tabletist` to the AUR. The AUR's
  host key is pinned in `release.yml`; update it there if the AUR rotates it.

Dry run without publishing: `gh workflow run release.yml -f tag=v0.0.0-dry`.
`packaging.yml` builds and installs the DMG, the Windows installer and the
source AUR package whenever `packaging/` changes. Locally,
`packaging/arch/render.sh` fills a PKGBUILD from a release's checksums.
