# Automatic updates, step 1: signed releases and the updater's groundwork Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every release something an updater can install safely (a signed `checksums.txt`, marker files, an AppImage name the updater knows), and make every build from here on accept an update relaunch, so the release that later turns updates on can be followed by updates.

**Architecture:** Tabletist takes its updater from `fastframe-update` (crmne/fastframe, the crate Spotifast, ZapFast, Solco and TonePush update with) at the tag the other fastframe crates use. Part A changes only the release: a new `sign` job signs `checksums.txt` with an Ed25519 key that only it can read, the archives and the installer carry the marker files the updater looks for, and the AppImage gets the name the updater downloads. Part B adds `src/update.rs` (the updater's configuration and an HTTP client over ureq), takes the update flags out of the command line before clap reads it, and confirms an update's receipt once the window is up. Nothing in this plan checks for, downloads or offers an update: that is step 2, after the design canvas has the screens.

**Tech Stack:** Rust 1.98, fastframe-update (fastframe v0.13.0), ureq 3.4 on rustls with ring, OpenSSL 3 in the release job, GitHub Actions, Inno Setup 6.

**Spec:** none yet. The decisions below stand in for one; each needs a yes before code.

---

## Before Task 1

Commit this plan:

```bash
git add docs/superpowers/plans/2026-10-21-automatic-updates-1-releases-and-groundwork.md
git commit -m "Plan automatic updates step 1: signed releases and the updater's groundwork"
```

## Where this sits

- **Step 1 (this plan).** Part A, Tasks 1 to 5: the release. Part B, Tasks 6 to 9: the app's groundwork. Each part ships on its own. After Part A, a release carries `checksums.txt.sig` and the markers. After Part B, a build accepts `--update-receipt` and `--update-error` and confirms the receipt. The app still never looks for an update.
- **Step 2 (a later plan, after the design).** When to check (once a day, the `CHECK_INTERVAL`), downloading in the background, the "Restart to update" offer and what it does with unsaved edits and open transactions, what a copy that cannot update itself says (a package manager's, one run from the disk image), the Settings switch, a "Check for Updates…" item in the macOS app menu, and the website. The design canvas has no artboard for any of it yet (see the memory note on the canvas); step 2 starts by asking for one.

Why step 1 first: the version that installs an update is the old one, and the version it installs must answer the old helper. A build without Part B, installed by an update, would be rolled back after a minute because nothing confirms its receipt. So Part B has to be in every release from the one before updates are turned on.

## How fastframe-update installs an update (for the engineer)

1. `Updater::check` reads `https://api.github.com/repos/igor-alexandrov/tabletist/releases/latest` and compares its tag with `CARGO_PKG_VERSION`. Pre-releases are never offered; `0.3.0-rc1` hears about `0.3.0`.
2. `Updater::installation` refuses a copy a package manager owns (pacman and the AUR, Homebrew, Flatpak, Snap, apt, dnf, Nix, `cargo install`) and, on Linux and Windows, any copy without a marker file beside it. The Inno Setup default folder (`%LOCALAPPDATA%\Programs\Tabletist\tabletist.exe` with `unins000.exe`) counts without a marker.
3. `Updater::download` needs the asset for this platform plus `checksums.txt` and `checksums.txt.sig`. It checks the signature against the key built into the app before it reads anything else, then the SHA-256 of the download, then runs the download's `--version`, which must print `tabletist <version>`. On macOS it also checks the bundle id, the version and, when the running app is signed by a team, the same team and Gatekeeper.
4. `Updater::handoff` starts a copy of the app as `tabletist --apply-update <staging>/handoff.json`, which waits for the app to quit, keeps the old copy as `previous`, installs (replaces the binary, runs the Inno setup with `/VERYSILENT /DIR=…`, swaps `Tabletist.app`, or renames the AppImage), and relaunches with `--update-receipt <job>`.
5. The new app writes `started` beside the job (`Receipt::acknowledge`). Without it within a minute, the helper puts `previous` back and starts the old app with `--update-error "<message>"`.

The asset names it expects, all but the AppImage already Tabletist's:

| Kind | Name |
| --- | --- |
| Linux portable | `tabletist-v<version>-<x86_64\|aarch64>-unknown-linux-gnu.tar.gz`, holding `tabletist-v<version>-<target>/tabletist` |
| Windows portable | `tabletist-v<version>-<target>-pc-windows-msvc.zip`, holding `…/tabletist.exe` |
| Windows installer | `tabletist-v<version>-<x86_64\|aarch64>-pc-windows-msvc-setup.exe` |
| macOS | `tabletist-v<version>-macos-universal.dmg`, holding `Tabletist.app` |
| AppImage | `tabletist-<version>-<x86_64\|aarch64>.AppImage` (Task 5 renames it to this) |
| Markers | `tabletist-portable.txt` holding `tabletist-portable-v1`; `tabletist-installer.txt` holding `tabletist-installer-v1` |
| Signature | `checksums.txt.sig`: the raw 64-byte Ed25519 signature over the exact bytes of `checksums.txt` |

## Decisions, for a yes or a no before code

| Decision | Instead of | Why |
| --- | --- | --- |
| fastframe-update at tag `v0.13.0` | `v0.14.0`, which Spotifast uses | `AGENTS.md`: fastframe crates share one tag. The crate's files are byte for byte the same at both tags. |
| ureq 3.4 (`rustls` on ring, Mozilla's roots through webpki-roots) and a `Transport` of our own, about 40 lines | The crate's `reqwest` feature | Tabletist has no HTTP client. ureq adds `ureq`, `ureq-proto`, `http`, `httparse`, `utf8-zero` and a second `base64` (0.23, beside the tree's 0.22), plus `fastframe-dirs` and `fastframe-update` themselves; rustls, ring, webpki-roots and `url` are already in the tree. reqwest would add hyper, h2 and tower. No aws-lc. |
| Mozilla's roots for GitHub | The system's roots, as `tabletist-db` uses for databases | GitHub's certificates chain to Mozilla's roots. A proxy that re-signs TLS would need the system's roots; that can come with step 2's proxy question if anyone asks. ureq reads `HTTPS_PROXY` already. |
| The AppImage renamed to `tabletist-<version>-<arch>.AppImage` | Keeping `tabletist-v<version>-<target>.AppImage` and turning AppImage updates off | It is the only name the crate downloads. The pages name the pattern, never a file, so only the pattern changes. |
| A release without the signing key fails | Publishing unsigned with a notice, as the Apple and AUR secrets do | Once step 2 ships, an unsigned release cannot be installed by any updater, and nobody would notice until people stopped getting updates. The `sign` job fails before `publish` runs. |
| Signing in a job of its own, in a `release-signing` environment that only `v*` tags may use | Signing in `publish` | `publish` holds the only write token. The job that reads the key runs no cargo, no third-party action that sees the key, and has a read-only token. |
| OpenSSL on the runner signs | native-packages' `sign-checksums`, which Spotifast uses | No new tool in the release. OpenSSL 3 signs Ed25519 over raw bytes (`pkeyutl -rawin`), which is what ring verifies; Task 2's test proves it. |
| The installer installs `tabletist-installer.txt`, and its uninstaller removes `.tabletist-update-*` | Relying on the default folder alone | A person who chose another folder still updates, and the last update's staging folder (kept until the next one) goes with the uninstall. |
| Omastore copies carry the portable marker too, and will update themselves once step 2 ships | A second archive without the marker for omastore | `omastore.toml` installs the same `.tar.gz` the download page offers. Open question for step 2: does omastore update its apps itself? If it does, step 2 refuses that install path (a check on where the executable lives) so two updaters never race. Step 1 changes nothing for omastore users. |

The portable marker changes nothing else: Tabletist keeps its data where `directories` says (`src/paths.rs`), not beside a portable copy, since it does not use fastframe-dirs for its paths.

## File structure

| File | Part | What it is for |
| --- | --- | --- |
| `assets/update-public-key.hex` | A, created | The public half of the release signing key, 64 hex digits. Built into the app (Part B) and checked against by the `sign` job. |
| `packaging/sign-checksums.sh` | A, created | Signs `checksums.txt` with the key in `TABLETIST_UPDATE_SIGNING_KEY` and checks the signature against `assets/update-public-key.hex`. |
| `tests/fixtures/update/{checksums.txt,checksums.txt.sig,public-key.hex}` | A, created | A signature the script made with a throwaway key. |
| `tests/update_release.rs` | A, created | The script's signature verifies under ring; the key file is a key; the markers hold what the updater reads. |
| `.gitattributes` | A, created | Keeps the fixtures' bytes as they are on Windows checkouts. |
| `packaging/tabletist-portable.txt` | A, created | The portable marker, packed into the `.zip` and `.tar.gz`. |
| `packaging/windows/tabletist-installer.txt` | A, created | The installer marker, installed beside `tabletist.exe`. |
| `packaging/windows/tabletist.iss` | A, modified | Installs the marker; removes staging folders on uninstall. |
| `.github/workflows/release.yml` | A, modified | The `sign` job; markers in the archives; the AppImage name; `publish` after `sign`. |
| `.github/workflows/packaging.yml` | A, modified | The installed marker and the uninstall of a staging folder are checked. |
| `Cargo.toml`, `Cargo.lock` | A and B, modified | `ring` as a dev-dependency (A); `fastframe-update` and `ureq` (B). |
| `src/update.rs` | B, created | `CONFIG`, `updater()`, and the ureq `Transport`. |
| `src/lib.rs` | B, modified | `pub mod update;` |
| `src/entrypoint.rs` | B, modified | `intercept` before clap; the receipt confirmed after the first frame; a restored update logged. |
| `tests/cli.rs` | B, modified | The update flags are taken out before clap; `--apply-update` with no job exits 1 without a window. |
| `AGENTS.md`, `README.md`, `docs/_guide/download.md` | A and B, modified | The signing secret, the AppImage name, and what a later release relies on. |

---

# Part A: the release

### Task 1: The release signing key (Igor, by hand)

The private key never passes through an agent, a commit or a log. The engineer stops here and asks Igor to run these steps, then waits for the public key to be committed.

**Files:**
- Create: `assets/update-public-key.hex`

- [ ] **Step 1: Igor makes the key pair on his own machine**

```bash
umask 077
# Outside the repository, so no `git add` can pick it up.
openssl genpkey -algorithm ed25519 -out ~/tabletist-update-signing.pem
openssl pkey -in ~/tabletist-update-signing.pem -pubout -outform DER | tail -c 32 | od -An -tx1 -v | tr -d ' \n' > assets/update-public-key.hex
echo >> assets/update-public-key.hex
cat assets/update-public-key.hex
```

Expected: one line of 64 lowercase hex digits.

- [ ] **Step 2: Igor stores the private key**

1. Save `~/tabletist-update-signing.pem` in 1Password. Losing it means every installed copy has to be updated by hand once, to a build that trusts a new key (`additional_publisher_keys` exists for planned rotation, not for a lost key).
2. On GitHub: Settings › Environments › New environment `release-signing`. Under "Deployment branches and tags" choose "Selected branches and tags" and add the tag rule `v*`. Add the environment secret `TABLETIST_UPDATE_SIGNING_KEY` with the whole PEM file, `-----BEGIN PRIVATE KEY-----` and `-----END PRIVATE KEY-----` lines included.
3. Delete the local file: `rm ~/tabletist-update-signing.pem`.

- [ ] **Step 3: Commit the public key**

```bash
git add assets/update-public-key.hex
git commit -m "Add the public key that release checksums are signed with"
```

### Task 2: The signing script, and a test that ring accepts what it signs

**Files:**
- Create: `packaging/sign-checksums.sh`
- Create: `tests/fixtures/update/checksums.txt`, `tests/fixtures/update/checksums.txt.sig`, `tests/fixtures/update/public-key.hex`
- Create: `tests/update_release.rs`
- Create: `.gitattributes`
- Modify: `Cargo.toml` (`[dev-dependencies]`, line 169)

- [ ] **Step 1: Write the failing test**

Create `tests/update_release.rs`:

```rust
//! What a release gives the updater (fastframe-update): a `checksums.txt`
//! signed with the key the app trusts, and the markers that tell a copy
//! from the download page from one a package manager installed.

// `allow-unwrap-in-tests` does not cover helpers in an integration-test crate.
#![allow(clippy::unwrap_used)]

use ring::signature::{ED25519, UnparsedPublicKey};

/// A public key from its 64 hex digits, as `assets/update-public-key.hex`
/// holds it and as fastframe-update reads it.
fn key(hex: &str) -> Vec<u8> {
    let hex = hex.trim();
    assert_eq!(hex.len(), 64, "an Ed25519 public key is 64 hex digits");
    (0..hex.len())
        .step_by(2)
        .map(|at| u8::from_str_radix(&hex[at..at + 2], 16).unwrap())
        .collect()
}

/// `packaging/sign-checksums.sh` signs with OpenSSL; the updater verifies
/// with ring. The fixture was signed by the script with a throwaway key.
#[test]
fn a_signature_from_the_release_script_verifies() {
    let manifest = include_bytes!("fixtures/update/checksums.txt");
    let signature = include_bytes!("fixtures/update/checksums.txt.sig");
    let public = key(include_str!("fixtures/update/public-key.hex"));
    let verifier = UnparsedPublicKey::new(&ED25519, &public);
    assert_eq!(signature.len(), 64);
    verifier.verify(manifest, signature).unwrap();
    let mut changed = manifest.to_vec();
    changed[0] ^= 1;
    assert!(verifier.verify(&changed, signature).is_err());
}

#[test]
fn the_published_key_is_a_key() {
    assert_eq!(key(include_str!("../assets/update-public-key.hex")).len(), 32);
}
```

Add to `Cargo.toml` under `[dev-dependencies]`, after the `tokio` entry:

```toml
# Checks that what packaging/sign-checksums.sh signs with OpenSSL verifies
# under ring, as fastframe-update verifies it (tests/update_release.rs).
# Already in the tree through rustls and fastframe-update.
ring = "0.17"
```

- [ ] **Step 2: Run the test to see it fail**

Run: `~/.cargo/bin/cargo test --test update_release`
Expected: FAIL to compile, `couldn't read …/tests/fixtures/update/checksums.txt`.

- [ ] **Step 3: Write the script**

Create `packaging/sign-checksums.sh` and make it executable (`chmod 755`):

```bash
#!/bin/bash
# Sign a release's checksums.txt for the app's updater, and check the
# signature against the public key the app has built in.
#
#   TABLETIST_UPDATE_SIGNING_KEY="$(cat key.pem)" \
#     packaging/sign-checksums.sh dist/checksums.txt assets/update-public-key.hex
#
# Writes dist/checksums.txt.sig: the raw 64-byte Ed25519 signature over the
# file's exact bytes, which fastframe-update checks before it downloads
# anything. Fails when the key is not set or is not the public key's pair:
# no updater would install a release signed with another key. The signature
# is written only once it verifies. Needs OpenSSL 3 and xxd.
set -euo pipefail

checksums="$1"
public_hex="$2"
: "${TABLETIST_UPDATE_SIGNING_KEY:?TABLETIST_UPDATE_SIGNING_KEY is not set}"

work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
# The key is never readable by anyone else, not even briefly.
(umask 077 && printf '%s\n' "$TABLETIST_UPDATE_SIGNING_KEY" > "$work/key.pem")
openssl pkeyutl -sign -inkey "$work/key.pem" -rawin -in "$checksums" -out "$work/signature"

# The built-in key as DER: Ed25519's SubjectPublicKeyInfo prefix, then the
# 32 bytes the hex file holds.
{ printf '302a300506032b6570032100'; tr -d '[:space:]' < "$public_hex"; } |
    xxd -r -p > "$work/public.der"
openssl pkeyutl -verify -pubin -keyform DER -inkey "$work/public.der" -rawin \
    -in "$checksums" -sigfile "$work/signature"
[ "$(wc -c < "$work/signature")" -eq 64 ]
mv "$work/signature" "$checksums.sig"
```

- [ ] **Step 4: Make the fixture with a throwaway key**

```bash
scratch="$(mktemp -d)"
mkdir -p tests/fixtures/update
openssl genpkey -algorithm ed25519 -out "$scratch/key.pem"
openssl pkey -in "$scratch/key.pem" -pubout -outform DER | tail -c 32 | od -An -tx1 -v | tr -d ' \n' > tests/fixtures/update/public-key.hex
echo >> tests/fixtures/update/public-key.hex
printf '%s  %s\n' \
  0000000000000000000000000000000000000000000000000000000000000000 tabletist-v0.0.0-x86_64-unknown-linux-gnu.tar.gz \
  1111111111111111111111111111111111111111111111111111111111111111 tabletist-v0.0.0-macos-universal.dmg \
  > tests/fixtures/update/checksums.txt
TABLETIST_UPDATE_SIGNING_KEY="$(cat "$scratch/key.pem")" \
  bash packaging/sign-checksums.sh tests/fixtures/update/checksums.txt tests/fixtures/update/public-key.hex
```

Expected: `Signature Verified Successfully`, and `tests/fixtures/update/checksums.txt.sig` is 64 bytes.

Then check that the script refuses a key that is not the public key's pair, and leaves no signature behind:

```bash
openssl genpkey -algorithm ed25519 -out "$scratch/other.pem"
cp tests/fixtures/update/checksums.txt "$scratch/checksums.txt"
TABLETIST_UPDATE_SIGNING_KEY="$(cat "$scratch/other.pem")" \
  bash packaging/sign-checksums.sh "$scratch/checksums.txt" tests/fixtures/update/public-key.hex
echo "exit $?"; ls "$scratch"
rm -rf "$scratch"
```

Expected: `Signature Verification Failure`, `exit 1`, and no `checksums.txt.sig` in the listing. The throwaway private key is deleted with the folder and never committed.

- [ ] **Step 5: Keep the fixture's bytes on Windows**

The signature covers exact bytes, and a Windows checkout would otherwise turn `checksums.txt` into CRLF. Create `.gitattributes`:

```gitattributes
# Signed bytes: a checkout must not change line endings (tests/update_release.rs).
tests/fixtures/update/** -text
```

- [ ] **Step 6: Run the test to see it pass**

Run: `~/.cargo/bin/cargo test --locked --test update_release`
Expected: PASS, 2 tests. If `--locked` fails because `Cargo.lock` lacks the `ring` edge, run without it once, then check `git diff Cargo.lock` shows only `"ring"` added to the `tabletist` package's dependency list (see the memory note on minimal `Cargo.lock` edits; hand-revert any other change) and run with `--locked` again.

- [ ] **Step 7: Commit**

```bash
git add packaging/sign-checksums.sh tests/fixtures/update tests/update_release.rs .gitattributes Cargo.toml Cargo.lock
git commit -m "Sign release checksums for the updater, and test ring accepts the signature"
```

### Task 3: The `sign` job in the release

**Files:**
- Modify: `.github/workflows/release.yml` (the `publish` job, and a new `sign` job before it)

- [ ] **Step 1: Add the `sign` job**

Insert before `publish:` in `.github/workflows/release.yml`:

```yaml
  # Signs checksums.txt for the app's updater (fastframe-update), which
  # downloads nothing from a release whose signature does not verify. The
  # only job that sees the signing key: it runs no cargo and has a read-only
  # token, and its environment gives the key to v* tags alone. Without the
  # key it fails, and nothing is published: no updater could install an
  # unsigned release.
  sign:
    name: Checksums and signature
    needs: [build, macos]
    if: github.event_name == 'push'
    runs-on: ubuntu-latest
    environment: release-signing
    steps:
      - uses: actions/checkout@v4
        with:
          persist-credentials: false
      - uses: actions/download-artifact@v4
        with:
          path: dist
          # Everything but the unsigned macOS binary, which the DMG replaces.
          pattern: "!macos-universal-binary"
          merge-multiple: true
      # The source archive, so source packages check the same checksums.
      - name: Source archive and checksums
        run: |
          curl --fail --location --retry 3 \
            "https://github.com/${GITHUB_REPOSITORY}/archive/refs/tags/${TAG}.tar.gz" \
            --output "dist/tabletist-${TAG}-source.tar.gz"
          (cd dist && sha256sum * > checksums.txt && cat checksums.txt)
      - name: Sign checksums
        env:
          TABLETIST_UPDATE_SIGNING_KEY: ${{ secrets.TABLETIST_UPDATE_SIGNING_KEY }}
        run: bash packaging/sign-checksums.sh dist/checksums.txt assets/update-public-key.hex
      - uses: actions/upload-artifact@v4
        with:
          name: checksums
          path: |
            dist/checksums.txt
            dist/checksums.txt.sig
            dist/tabletist-*-source.tar.gz
          if-no-files-found: error
```

- [ ] **Step 2: Make `publish` take the signed files**

In the `publish` job, change `needs: [build, macos]` to `needs: sign`, and replace the whole "Source archive and checksums" step with:

```yaml
      # From the sign job, with the source archive.
      - name: The checksums are signed
        run: |
          test "$(wc -c < dist/checksums.txt.sig)" -eq 64
          cat dist/checksums.txt
```

`publish` keeps downloading every artifact but `macos-universal-binary`, which now includes `checksums`.

- [ ] **Step 3: Check the workflow parses**

Run: `python3 -I -c "import sys, yaml; yaml.safe_load(open('.github/workflows/release.yml'))" && echo ok`
Expected: `ok`. If `actionlint` is installed, also run `actionlint .github/workflows/release.yml` and expect no output.

A dry run (`gh workflow run release.yml -f tag=v0.0.0-dry`) skips `sign` and `publish` as before, so the first tag is the first real run of `sign`. It cannot publish a bad signature: `sign-checksums.sh` verifies against the committed key, and `publish` waits for `sign`.

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "Sign checksums.txt in a release job of its own, before publishing"
```

### Task 4: Marker files in the archives and the installer

**Files:**
- Create: `packaging/tabletist-portable.txt`, `packaging/windows/tabletist-installer.txt`
- Modify: `tests/update_release.rs`
- Modify: `.github/workflows/release.yml` ("Archive" step, line 82)
- Modify: `packaging/windows/tabletist.iss` (`[Files]`, and a new `[UninstallDelete]`)
- Modify: `.github/workflows/packaging.yml` ("Install, run and uninstall" step)

- [ ] **Step 1: Write the failing test**

Append to `tests/update_release.rs`:

```rust
/// fastframe-dirs reads `<slug>-<kind>.txt` and compares its trimmed text
/// with `<slug>-<kind>-v1`. Without a marker, a copy on Linux or Windows
/// never updates itself.
#[test]
fn the_markers_hold_what_the_updater_reads() {
    assert_eq!(
        include_str!("../packaging/tabletist-portable.txt").trim(),
        "tabletist-portable-v1"
    );
    assert_eq!(
        include_str!("../packaging/windows/tabletist-installer.txt").trim(),
        "tabletist-installer-v1"
    );
}
```

- [ ] **Step 2: Run the test to see it fail**

Run: `~/.cargo/bin/cargo test --locked --test update_release`
Expected: FAIL to compile, `couldn't read …/packaging/tabletist-portable.txt`.

- [ ] **Step 3: Add the markers**

```bash
printf 'tabletist-portable-v1\n' > packaging/tabletist-portable.txt
printf 'tabletist-installer-v1\n' > packaging/windows/tabletist-installer.txt
```

- [ ] **Step 4: Run the test to see it pass**

Run: `~/.cargo/bin/cargo test --locked --test update_release`
Expected: PASS, 3 tests.

- [ ] **Step 5: Pack the portable marker**

In `.github/workflows/release.yml`, "Archive" step, change

```bash
          cp README.md LICENSE "dist/$name/"
```

to

```bash
          # The marker tells the updater this copy came from the download
          # page and may replace itself. The AUR's tabletist-bin unpacks the
          # same archive but does not install it.
          cp README.md LICENSE packaging/tabletist-portable.txt "dist/$name/"
```

- [ ] **Step 6: Install the installer marker, and remove staging folders on uninstall**

In `packaging/windows/tabletist.iss`, add to `[Files]` after the `LICENSE` line:

```ini
; Tells the updater this copy came from the installer, wherever it was put.
Source: "tabletist-installer.txt"; DestDir: "{app}"; Flags: ignoreversion
```

and add after the `[Files]` section:

```ini
[UninstallDelete]
; The updater keeps the last update's staging folder beside the app until
; the next update (fastframe-update); it goes with the app.
Type: filesandordirs; Name: "{app}\.tabletist-update-*"
```

- [ ] **Step 7: Check both in the packaging workflow**

In `.github/workflows/packaging.yml`, step "Install, run and uninstall", after the `--version` check and before the uninstaller runs, add:

```powershell
          $marker = "$app\tabletist-installer.txt"
          if (-not (Test-Path $marker)) { throw "no installer marker" }
          if ((Get-Content $marker -Raw).Trim() -ne 'tabletist-installer-v1') { throw "the installer marker says $(Get-Content $marker -Raw)" }
          # What an update leaves beside the app goes with the uninstall.
          $staging = "$app\.tabletist-update-0123456789abcdef"
          New-Item -ItemType Directory $staging | Out-Null
          Set-Content "$staging\result.txt" 'ok'
```

and after the existing `still installed after uninstall` check:

```powershell
          if (Test-Path $staging) { throw "an update's staging folder is left after uninstall" }
```

This runs on the pull request (`packaging/**` is in its paths). It is the only check of the Inno wildcard; if Inno leaves the folder, the step says so.

- [ ] **Step 8: Commit**

```bash
git add packaging/tabletist-portable.txt packaging/windows/tabletist-installer.txt packaging/windows/tabletist.iss tests/update_release.rs .github/workflows/release.yml .github/workflows/packaging.yml
git commit -m "Mark portable and installed copies for the updater, and uninstall its staging folders"
```

### Task 5: The AppImage name the updater downloads, and the pages

**Files:**
- Modify: `.github/workflows/release.yml` ("AppImage" step, line 97)
- Modify: `docs/_guide/download.md`, `README.md:108-111`, `AGENTS.md` ("Releasing")

- [ ] **Step 1: Rename the AppImage**

In `.github/workflows/release.yml`, "AppImage" step, change

```bash
          bash packaging/linux/appimage.sh "target/${{ matrix.target }}/release/tabletist" \
            "dist/tabletist-${TAG}-${{ matrix.target }}.AppImage" "${TAG#v}"
```

to

```bash
          # tabletist-0.2.3-x86_64.AppImage: the name fastframe-update
          # downloads for an AppImage, with no v and the processor alone.
          target="${{ matrix.target }}"
          bash packaging/linux/appimage.sh "target/${target}/release/tabletist" \
            "dist/tabletist-${TAG#v}-${target%%-*}.AppImage" "${TAG#v}"
```

- [ ] **Step 2: The download page**

In `docs/_guide/download.md`:

1. Replace "Each release lists its files with a `checksums.txt`, which you can use to check that a download was not damaged or changed." with:

   ```markdown
   Each release lists its files with a `checksums.txt`, which you can use to
   check that a download was not damaged or changed, and `checksums.txt.sig`,
   its signature by the Tabletist release key.
   ```

2. Under "Other distributions", replace `tabletist-v<version>-x86_64-unknown-linux-gnu.AppImage` with `tabletist-<version>-x86_64.AppImage`, and in the code block replace both `tabletist-v<version>-x86_64-unknown-linux-gnu.AppImage` with `tabletist-<version>-x86_64.AppImage`.

- [ ] **Step 3: The README**

In `README.md`, "Other Linux", replace `tabletist-v<version>-x86_64-unknown-linux-gnu.AppImage` with `tabletist-<version>-x86_64.AppImage`.

- [ ] **Step 4: AGENTS.md, "Releasing"**

1. In step 3 of the list, replace "Linux (x86_64, aarch64 `.tar.gz` and `.AppImage`)" with "Linux (x86_64, aarch64 `.tar.gz`, and `.AppImage` named `tabletist-<version>-<arch>.AppImage`)", and "publishes a GitHub release with `checksums.txt`" with "signs `checksums.txt` (`checksums.txt.sig`), publishes a GitHub release with both".
2. Before "Optional secrets", add:

   ```markdown
   Required: `TABLETIST_UPDATE_SIGNING_KEY`, an Ed25519 private key (PEM) in
   the `release-signing` environment, which only `v*` tags may use. The
   `sign` job signs `checksums.txt` with it and checks the signature against
   `assets/update-public-key.hex`, the key the app has built in; without it,
   or with another key, the release stops before anything is published. The
   key is in 1Password. Never commit it, and never replace
   `assets/update-public-key.hex` alone: installed copies trust only the key
   they were built with.
   ```

- [ ] **Step 5: Check the site builds**

Run: `cd docs && bin/check`
Expected: the check passes. (If Ruby is not set up here, say so and leave it to `docs.yml` on the pull request.)

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release.yml docs/_guide/download.md README.md AGENTS.md
git commit -m "Name the AppImage as the updater downloads it, and document the signing key"
```

**Part A ships here.** Push, open the pull request, and let `packaging.yml` run the Windows install check. The next tag is the first signed release; watch its `sign` job.

---

# Part B: the app's groundwork

### Task 6: The two crates

**Files:**
- Modify: `Cargo.toml` (`[dependencies]`, after the `fastframe-log` entry at line 77)
- Modify: `Cargo.lock`

- [ ] **Step 1: Add the dependencies**

In `Cargo.toml`, after the `fastframe-log` line:

```toml
# Self-update from GitHub releases (src/update.rs): checks the signature on
# checksums.txt, detects how this copy was installed, hands the update to a
# helper that installs it and rolls back if the new version does not start.
fastframe-update = { git = "https://github.com/crmne/fastframe", tag = "v0.13.0" }
```

and after the `tabletist-mcp` entry (line 82):

```toml
# The updater's HTTP client: blocking GETs to GitHub, on rustls with ring
# and Mozilla's roots (webpki-roots), all already in the tree. Adds ureq,
# ureq-proto, http, httparse, utf8-zero and base64 0.23 (beside the tree's
# 0.22); reqwest would add hyper, h2 and tower. No gzip: release files are
# compressed already.
ureq = { version = "3.4", default-features = false, features = ["rustls"] }
```

- [ ] **Step 2: Resolve, and keep the lock diff to the new crates**

Run: `~/.cargo/bin/cargo build`
Then: `git diff --stat Cargo.lock && git diff Cargo.lock | grep '^[-+]version' | sort | uniq -c`
Expected: new entries for `fastframe-dirs`, `fastframe-update`, `http`, `httparse`, `ureq`, `ureq-proto`, `utf8-zero` and a second `base64` (0.23.1), dependency lines added to existing packages, and existing `"base64"` dependency lines turned into `"base64 0.22.1"` now that two versions are in the tree. That rename is expected; do not revert it. No `-version` line: if any existing crate's version moved (the memory note names `windows-sys` pins), put the old version back by hand and run `~/.cargo/bin/cargo build --locked` until it passes.

Then: `~/.cargo/bin/cargo tree --locked -i aws-lc-rs`
Expected: `error: package ID specification `aws-lc-rs` did not match any packages` (no aws-lc).

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "Add fastframe-update and ureq for automatic updates"
```

### Task 7: `src/update.rs`: what the updater knows, and its HTTP client

**Files:**
- Create: `src/update.rs`
- Modify: `src/lib.rs` (`pub mod update;` between `pub mod ui;` and `pub mod util;`)

- [ ] **Step 1: Write the failing tests**

Create `src/update.rs` with the tests only, and add `pub mod update;` to `src/lib.rs` between `pub mod ui;` and `pub mod util;`:

```rust
//! Automatic updates from GitHub releases, through fastframe-update.

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::thread::JoinHandle;

    use fastframe_update::{Request, Transport};

    use super::*;

    /// Answers one request on a local port with `answer`, and gives back
    /// the request's header lines.
    fn serve_once(answer: &'static str) -> (String, JoinHandle<Vec<String>>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/releases/latest", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let (stream, _) = listener.accept().unwrap();
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut head = Vec::new();
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).unwrap();
                if line == "\r\n" || line.is_empty() {
                    break;
                }
                head.push(line.trim_end().to_owned());
            }
            (&stream).write_all(answer.as_bytes()).unwrap();
            head
        });
        (url, server)
    }

    #[test]
    fn the_configuration_is_valid() {
        CONFIG.validate().unwrap();
        assert_eq!(CONFIG.current_version, env!("CARGO_PKG_VERSION"));
    }

    /// Without a key the updater trusts checksums.txt as GitHub serves it;
    /// with one, a release must be signed (packaging/sign-checksums.sh).
    #[test]
    fn updates_need_the_release_signature() {
        assert!(CONFIG.publisher_key.is_some());
    }

    /// The release names its AppImages for the updater (release.yml), so
    /// an AppImage may replace itself.
    #[test]
    fn appimages_update_themselves() {
        const { assert!(CONFIG.appimage) };
    }

    /// The updater follows redirects itself, and only to GitHub's hosts: a
    /// client that followed them would let a redirect go anywhere.
    #[test]
    fn a_redirect_is_handed_back_not_followed() {
        let (url, server) = serve_once(
            "HTTP/1.1 302 Found\r\nLocation: https://objects.githubusercontent.com/x\r\nContent-Length: 0\r\nConnection: close\r\n\r\n",
        );
        let response = UreqTransport::new()
            .get(&Request {
                url: &url,
                accept: "application/octet-stream",
                user_agent: "Tabletist/0.2.2",
            })
            .unwrap();
        assert_eq!(response.status, 302);
        assert_eq!(
            response.location.as_deref(),
            Some("https://objects.githubusercontent.com/x")
        );
        let head = server.join().unwrap();
        let sent = |line: &str| head.iter().any(|sent| sent.eq_ignore_ascii_case(line));
        assert!(sent("accept: application/octet-stream"), "{head:?}");
        assert!(sent("user-agent: Tabletist/0.2.2"), "{head:?}");
    }

    /// A 404 (no release yet) is an answer for the updater to read, not a
    /// transport error.
    #[test]
    fn an_error_status_is_an_answer() {
        let (url, server) = serve_once(
            "HTTP/1.1 404 Not Found\r\nContent-Length: 9\r\nConnection: close\r\n\r\nnot found",
        );
        let mut response = UreqTransport::new()
            .get(&Request {
                url: &url,
                accept: "application/json",
                user_agent: "Tabletist/0.2.2",
            })
            .unwrap();
        assert_eq!(response.status, 404);
        let mut body = String::new();
        response.body.read_to_string(&mut body).unwrap();
        assert_eq!(body, "not found");
        server.join().unwrap();
    }
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --lib update::`
Expected: FAIL to compile: `cannot find value CONFIG`, `cannot find type UreqTransport`.

- [ ] **Step 3: Write the module**

Put above the `#[cfg(test)]` block in `src/update.rs`, under the module's doc line, which becomes:

```rust
//! Automatic updates from GitHub releases, through fastframe-update.
//!
//! What a later release's helper relies on from this build, which must not
//! change: `tabletist --version` prints `tabletist <version>`;
//! `--update-receipt <job>` and `--update-error <message>` are taken out
//! before clap reads the command line ([`fastframe_update::intercept`] in
//! `entrypoint::run`); and the receipt is confirmed once the window is up.
//! The release side of the contract (asset names, markers, the signed
//! checksums) is in `release.yml` and `tests/update_release.rs`.

use std::time::Duration;

use fastframe_update::{MacConfig, Request, Response, Transport, UpdateConfig, Updater};

/// What the updater knows about Tabletist.
pub const CONFIG: UpdateConfig = UpdateConfig {
    macos: MacConfig {
        bundle_ids: &["dev.tabletist.Tabletist"],
        // The slug: Contents/MacOS/tabletist.
        executable_names: &[],
        legacy_bundle_names: &[],
    },
    // Releases sign checksums.txt (packaging/sign-checksums.sh); nothing is
    // downloaded from a release whose signature does not verify.
    publisher_key: Some(include_str!("../assets/update-public-key.hex")),
    // Releases list tabletist-<version>-<arch>.AppImage in the signed
    // checksums, so an AppImage replaces itself with the next one.
    appimage: true,
    ..UpdateConfig::new(
        "igor-alexandrov/tabletist",
        "Tabletist",
        "tabletist",
        env!("CARGO_PKG_VERSION"),
    )
};

/// An updater, on an HTTP client of its own. Its calls block: run them on
/// a thread of their own, never on the interface thread.
pub fn updater() -> Updater {
    Updater::new(CONFIG, UreqTransport::new())
}

/// The updater's HTTP client: one GET at a time, redirects handed back
/// rather than followed (the updater follows them, to GitHub's hosts
/// only), any status returned as an answer, 15 seconds to connect and 15
/// minutes for a whole download, as fastframe-update asks of a transport.
struct UreqTransport(ureq::Agent);

impl UreqTransport {
    fn new() -> Self {
        let config = ureq::Agent::config_builder()
            .max_redirects(0)
            .http_status_as_error(false)
            .timeout_connect(Some(Duration::from_secs(15)))
            .timeout_global(Some(Duration::from_secs(15 * 60)))
            .build();
        Self(config.into())
    }
}

impl Transport for UreqTransport {
    fn get(&self, request: &Request<'_>) -> anyhow::Result<Response> {
        let response = self
            .0
            .get(request.url)
            .header("Accept", request.accept)
            .header("User-Agent", request.user_agent)
            .call()?;
        let location = response
            .headers()
            .get("location")
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        Ok(Response {
            status: response.status().as_u16(),
            location,
            body: Box::new(response.into_body().into_reader()),
        })
    }
}
```

- [ ] **Step 4: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --lib update::`
Expected: PASS, 5 tests. (This code was compiled and its tests run against ureq 3.4.2 and fastframe-update v0.13.0 in a scratch crate while the plan was written.)

- [ ] **Step 5: Commit**

```bash
git add src/update.rs src/lib.rs
git commit -m "Configure the updater for Tabletist, on a ureq client of its own"
```

### Task 8: Take the update flags out before clap, and confirm a receipt

**Files:**
- Modify: `src/entrypoint.rs` (`run` at line 61, `main` at line 100, `Window` at line 316, `Window::ui` at line 436)
- Modify: `tests/cli.rs` (after `demo_flags_require_demo`)

- [ ] **Step 1: Write the failing tests**

In `tests/cli.rs`, after `demo_flags_require_demo`:

```rust
/// An update's helper relaunches the app with `--update-receipt` (or, after
/// a rollback, `--update-error`) beside the arguments it was started with.
/// Those flags are taken out before clap reads the rest; clap alone would
/// refuse them.
#[test]
fn update_flags_are_taken_out_before_the_arguments_are_read() {
    for flags in [
        [
            "--update-receipt",
            "/missing/.tabletist-update-0000000000000000/handoff.json",
        ],
        [
            "--update-error",
            "The update could not start. The previous version has been restored.",
        ],
    ] {
        let output = tabletist()
            .args(flags)
            .arg("--version")
            .output()
            .expect("run tabletist");
        assert!(output.status.success(), "{flags:?}: {output:?}");
        assert_eq!(
            String::from_utf8_lossy(&output.stdout).trim(),
            format!("tabletist {}", env!("CARGO_PKG_VERSION"))
        );
    }
}

/// `--apply-update <job>` makes the process the helper, before anything
/// else: with no job to read it fails and exits, and never opens a window.
#[test]
fn the_helper_flag_runs_the_helper_and_nothing_else() {
    let output = tabletist()
        .args([
            "--apply-update",
            "/missing/.tabletist-update-0000000000000000/handoff.json",
        ])
        .output()
        .expect("run tabletist");
    assert_eq!(output.status.code(), Some(1), "{output:?}");
}
```

- [ ] **Step 2: Run the tests to see them fail**

Run: `~/.cargo/bin/cargo test --locked --test cli -- update_flags_are_taken_out_before_the_arguments_are_read the_helper_flag_runs_the_helper_and_nothing_else`
Expected: both FAIL. The first because clap rejects `--update-receipt` (exit 2), the second because clap rejects `--apply-update` (exit 2, not 1).

- [ ] **Step 3: `intercept` first in `run`**

In `src/entrypoint.rs`, replace the start of `run`:

```rust
pub fn run() -> std::process::ExitCode {
    let cli = Cli::parse();
```

with:

```rust
pub fn run() -> std::process::ExitCode {
    // First of all: `--apply-update <job>` makes this process an update's
    // helper, which installs it and exits. Otherwise the receipt or the
    // error an update's relaunch carries is taken out of the arguments
    // before clap reads them (src/update.rs).
    let launch = fastframe_update::intercept(&crate::update::CONFIG);
    let cli = Cli::parse_from(&launch.arguments);
```

and in the same function change `match main(cli) {` to `match main(cli, launch) {`.

- [ ] **Step 4: Hand the receipt to the window, and log a restored update**

Change `main`'s signature and add, right after `logging.init()?;`:

```rust
pub fn main(cli: Cli, launch: fastframe_update::Launch) -> anyhow::Result<()> {
```

```rust
    // The helper put the previous version back: this one. Step 2 shows it
    // in the window; until then the log has it.
    if let Some(message) = &launch.error {
        log::error!("{message}");
    }
    let update_receipt = launch.receipt;
```

In the `Box::new(Window { … })` literal, add `update_receipt,` after `demo,`. In `struct Window`, after `demo: bool,`:

```rust
    /// Set when an update's helper started this version. Confirmed after
    /// the first frame: until then the helper stands ready to put the
    /// previous version back.
    update_receipt: Option<fastframe_update::Receipt>,
```

The closure passed to `eframe::run_native` is `move`, so `update_receipt` moves into it with `dirs` and `settings`.

- [ ] **Step 5: Confirm the receipt after the first frame**

In `impl eframe::App for Window`, `fn ui`, right after `self.app.frame_ui(ui);`:

```rust
        // The window is up: tell the update's helper this version started,
        // so it keeps it. Off the interface thread, as the receipt asks.
        if let Some(receipt) = self.update_receipt.take() {
            std::thread::spawn(move || {
                if let Err(error) = receipt.acknowledge() {
                    log::error!("could not confirm the update: {error:#}");
                }
            });
        }
```

- [ ] **Step 6: Run the tests to see them pass**

Run: `~/.cargo/bin/cargo test --locked --test cli`
Expected: PASS, the two new tests and every test that was there. The `mcp::` tests still pass: `intercept` leaves `tabletist mcp` alone.

- [ ] **Step 7: Commit**

```bash
git add src/entrypoint.rs tests/cli.rs
git commit -m "Take the update flags out before clap, and confirm an update's receipt"
```

### Task 9: What a later release relies on, written down, and the checks

**Files:**
- Modify: `AGENTS.md` (a new "Updates" section after "Releasing")

- [ ] **Step 1: AGENTS.md**

Add after the "Releasing" section's last paragraph:

```markdown
## Updates

`src/update.rs` configures fastframe-update. The version that installs an
update is the old one, and its helper runs the new one, so these hold for
every release, and a change to any of them needs a release that accepts
both ways first:

- `tabletist --version` prints `tabletist <version>`, and
  `--update-receipt <job>`, `--update-error <message>` and
  `--apply-update <job>` are handled by `fastframe_update::intercept` before
  clap reads the command line (`tests/cli.rs`).
- The receipt is confirmed after the first frame (`entrypoint.rs`).
- The release's file names, the markers (`packaging/tabletist-portable.txt`
  in the archives, `packaging/windows/tabletist-installer.txt` installed
  beside the app) and `checksums.txt.sig` stay as `release.yml` makes them
  (`tests/update_release.rs`).
- `assets/update-public-key.hex` changes only through
  `additional_publisher_keys`: ship the next key there, keep signing with
  the old one, and switch once installed copies trust the next.
```

- [ ] **Step 2: The full checks**

Run each, from `AGENTS.md`:

```bash
cargo fmt --all --check
~/.cargo/bin/cargo clippy --locked --workspace --all-targets -- -D warnings
~/.cargo/bin/cargo test --locked --workspace --all-targets
RUSTDOCFLAGS='-D warnings' ~/.cargo/bin/cargo doc --locked --workspace --no-deps
```

Expected: all pass. The database suites print "skipped" without their URLs, which is fine here: nothing in this plan touches them.

- [ ] **Step 3: Other platforms**

The workspace does not cross-compile here (ring; see the memory note). `src/update.rs` and the `entrypoint.rs` change have no `cfg`, and fastframe-update builds on all three in its own CI and in Spotifast's. Say in the pull request that Linux was built and tested here, and Windows and macOS are left to `ci.yml` (which runs `cargo test` on all three) and `packaging.yml`.

- [ ] **Step 4: Commit**

```bash
git add AGENTS.md
git commit -m "Write down what a later release's updater relies on"
```

**Part B ships here, and step 1 ends.** From this release on, every build accepts an update's relaunch. Step 2's plan starts from the design canvas.

---

## By hand, after the first signed release

Not tests: one look each, after the first tag that has Part A.

1. The release page lists `checksums.txt.sig` and `tabletist-<version>-x86_64.AppImage`.
2. Verify the published signature from a clean folder:

   ```bash
   gh release download v<version> -p checksums.txt -p checksums.txt.sig
   { printf '302a300506032b6570032100'; tr -d '[:space:]' < assets/update-public-key.hex; } | xxd -r -p > public.der
   openssl pkeyutl -verify -pubin -keyform DER -inkey public.der -rawin -in checksums.txt -sigfile checksums.txt.sig
   ```

   Expected: `Signature Verified Successfully`.
3. `tar -tzf tabletist-v<version>-x86_64-unknown-linux-gnu.tar.gz` lists `tabletist-v<version>-x86_64-unknown-linux-gnu/tabletist-portable.txt`.
