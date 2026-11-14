//! Compiles the gettext catalogs in assets/i18n into Rust modules, which
//! `i18n` includes.

fn main() {
    fastframe_i18n::build::compile_catalogs("../../assets/i18n");
}
