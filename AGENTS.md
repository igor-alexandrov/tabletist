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
request themselves. No workflow reviews the change: Copilot's review comes
from the repository ruleset "Copilot review for default branch", and nothing
waits for it.

- The paths that start `packaging.yml` are listed twice: in its `pull_request`
  trigger and in its `push` trigger. Change them together.
- The ruleset asks for the review in the author's name, so it skips an author
  without a Copilot plan. `copilot-review.yml` asks in the owner's name for
  every pull request the owner did not open. It files the request and nothing
  else, and needs the `COPILOT_REVIEW_TOKEN` secret.

## Style

- Never use em dashes. Use a full stop, comma, colon, or parentheses.
- Work on `main`, linear history, one topic per commit, each passing checks.
- Report platform coverage honestly: say when something was only compiled.

## Releasing

1. Bump `version` in `Cargo.toml`, run `cargo update -p tabletist`, commit.
2. `git tag -s v0.1.0 -m v0.1.0 && git push origin v0.1.0`.
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
