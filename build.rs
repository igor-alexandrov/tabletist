//! Compiles the gettext catalogs in assets/i18n into Rust modules and, for a
//! Windows program built on Windows, embeds its icon and version details.

fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");
    embed_windows_resources();
}

/// Explorer, the shortcuts the installer creates and the Apps list all read
/// the icon from the exe, and Properties > Details reads the version details;
/// the icon eframe sets at runtime only reaches the window and the taskbar.
///
/// The fixed file and product versions are the package's major, minor and
/// patch numbers (a pre-release's `-rc1` has no place in them); the version
/// strings keep the whole package version.
///
/// Only on a Windows host: winresource is a build dependency there alone, so
/// a cross-build from Linux or macOS gets a program without resources.
#[cfg(windows)]
fn embed_windows_resources() {
    use std::{env, path::PathBuf};

    println!("cargo:rerun-if-changed=packaging/windows/tabletist.ico");
    // Where winresource looks for rc.exe before it asks the registry.
    println!("cargo:rerun-if-env-changed=RC_PATH");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("cargo sets CARGO_MANIFEST_DIR");
    let description = env::var("CARGO_PKG_DESCRIPTION").expect("cargo sets CARGO_PKG_DESCRIPTION");
    // Absolute, so rc.exe does not have to resolve it against the generated
    // .rc file in OUT_DIR.
    let icon = PathBuf::from(manifest_dir)
        .join("packaging")
        .join("windows")
        .join("tabletist.ico");

    let mut resources = winresource::WindowsResource::new();
    resources
        .set_icon(&icon.to_string_lossy())
        .set("ProductName", "Tabletist")
        .set("FileDescription", &description)
        // As in LICENSE.
        .set("LegalCopyright", "Copyright (c) 2026 Igor Alexandrov");
    if let Err(error) = resources.compile() {
        // A program without its icon must not pass for a good build.
        panic!(
            "could not embed the icon and version details: {error}. This needs rc.exe from the \
             Windows SDK; set RC_PATH to it if it is installed somewhere unusual."
        );
    }
}

#[cfg(not(windows))]
fn embed_windows_resources() {}
