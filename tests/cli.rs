//! Runs the built binary, the way a launcher or a packager would.

use std::process::Command;

fn tabletist() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tabletist"))
}

#[test]
fn version_prints_the_package_version() {
    let output = tabletist()
        .arg("--version")
        .output()
        .expect("run tabletist");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        format!("tabletist {}", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn demo_flags_require_demo() {
    let output = tabletist()
        .args(["--demo-shot", "out.png"])
        .output()
        .expect("run tabletist");
    assert!(
        !output.status.success(),
        "--demo-shot without --demo must be rejected"
    );
}
