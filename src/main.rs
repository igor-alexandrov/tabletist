// A GUI program on Windows: no console window next to the app in release
// builds (debug builds keep it for the log).
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    tabletist::entrypoint::run()
}
