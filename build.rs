//! Compiles the gettext catalogs in assets/i18n into Rust modules and, for a
//! Windows program built on Windows, embeds its icon and version details;
//! a macOS program gets the name its menu bar shows.

fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");
    embed_windows_resources();
    embed_macos_name();
}

/// The bold name in the macOS menu bar is the app's own, not a menu title:
/// `CFBundleName` from Tabletist.app's Info.plist, or for a binary run on
/// its own (`cargo run`) the executable's name, `tabletist` in lower case.
/// An Info.plist in the executable's `__TEXT,__info_plist` section is what
/// macOS reads for a binary outside a bundle; inside Tabletist.app the
/// bundle's Info.plist is read instead. It names the app and nothing else:
/// a bundle identifier would move the binary's defaults and keychain items.
fn embed_macos_name() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("macos") {
        return;
    }
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    let plist = out.join("Info.plist");
    let text = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>Tabletist</string>
    <key>CFBundleDisplayName</key><string>Tabletist</string>
</dict>
</plist>
"#;
    std::fs::write(&plist, text).expect("could not write the embedded Info.plist");
    println!(
        "cargo:rustc-link-arg-bins=-Wl,-sectcreate,__TEXT,__info_plist,{}",
        plist.display()
    );
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
