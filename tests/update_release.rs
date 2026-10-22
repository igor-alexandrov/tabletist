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
    assert_eq!(
        key(include_str!("../assets/update-public-key.hex")).len(),
        32
    );
}

/// The refusals of `packaging/sign-checksums.sh`: it signs only with the
/// built-in key's pair, leaves no signature behind when it fails, and
/// needs the key set. Linux only: macOS ships a LibreSSL that lacks
/// `-rawin`, and Windows has no bash; the release job runs on Linux.
#[cfg(target_os = "linux")]
#[test]
fn the_release_script_signs_only_with_the_built_in_keys_pair() {
    use std::path::Path;
    use std::process::Command;

    fn generate(dir: &Path, name: &str) -> String {
        let pem = dir.join(name);
        let made = Command::new("openssl")
            .args(["genpkey", "-algorithm", "ed25519", "-out"])
            .arg(&pem)
            .status()
            .unwrap();
        assert!(made.success());
        std::fs::read_to_string(pem).unwrap()
    }

    let dir = tempfile::tempdir().unwrap();
    let signing = generate(dir.path(), "signing.pem");
    let other = generate(dir.path(), "other.pem");

    // The last 32 bytes of the key's DER are the raw public key.
    let der = Command::new("openssl")
        .args(["pkey", "-pubout", "-outform", "DER", "-in"])
        .arg(dir.path().join("signing.pem"))
        .output()
        .unwrap()
        .stdout;
    let raw = &der[der.len() - 32..];
    let hex: String = raw.iter().map(|byte| format!("{byte:02x}")).collect();
    let hex_file = dir.path().join("public-key.hex");
    std::fs::write(&hex_file, format!("{hex}\n")).unwrap();

    let checksums = dir.path().join("checksums.txt");
    std::fs::write(&checksums, "00  tabletist-v0.0.0.tar.gz\n").unwrap();
    let signature = dir.path().join("checksums.txt.sig");
    let script = concat!(env!("CARGO_MANIFEST_DIR"), "/packaging/sign-checksums.sh");
    let run_with = |key: Option<&str>, trace: bool| {
        let mut command = Command::new("bash");
        if trace {
            command.arg("-x");
        }
        command.arg(script).arg(&checksums).arg(&hex_file);
        match key {
            Some(key) => command.env("TABLETIST_UPDATE_SIGNING_KEY", key),
            None => command.env_remove("TABLETIST_UPDATE_SIGNING_KEY"),
        };
        command.output().unwrap()
    };
    let run = |key: Option<&str>| run_with(key, false);

    let signed = run(Some(&signing));
    assert!(signed.status.success(), "{signed:?}");
    let bytes = std::fs::read(&signature).unwrap();
    assert_eq!(bytes.len(), 64);
    UnparsedPublicKey::new(&ED25519, raw)
        .verify(&std::fs::read(&checksums).unwrap(), &bytes)
        .unwrap();

    // A stale signature does not survive a refused run.
    std::fs::write(&signature, b"stale").unwrap();
    assert!(!run(Some(&other)).status.success());
    assert!(!signature.exists());

    std::fs::write(&signature, b"stale").unwrap();
    assert!(!run(None).status.success());
    assert!(!signature.exists());

    // Tracing the script never shows the key.
    let traced = run_with(Some(&signing), true);
    assert!(traced.status.success(), "{traced:?}");
    let trace = String::from_utf8_lossy(&traced.stderr);
    assert!(!trace.contains("PRIVATE KEY"), "{trace}");

    // A key file with more than 64 digits is refused, though OpenSSL would
    // ignore the extra bytes.
    std::fs::write(&hex_file, format!("{hex}00\n")).unwrap();
    assert!(!run(Some(&signing)).status.success());
    assert!(!signature.exists());
}

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
