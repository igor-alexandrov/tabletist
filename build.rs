//! Compiles the gettext catalogs in assets/i18n into Rust modules.

fn main() {
    fastframe_i18n::build::compile_catalogs("assets/i18n");
}
