//! Command line, logging, and the native window.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// Command-line flags.
#[derive(Debug, Parser)]
#[command(name = "tabletist", version, about = "A native database client")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Sub>,
    /// Log at debug level.
    #[arg(long)]
    pub verbose: bool,
    /// Run with sample data in a throwaway profile.
    #[arg(long)]
    pub demo: bool,
    /// With --demo: save a screenshot of the settled window to this PNG, then quit.
    #[arg(long, requires = "demo")]
    pub demo_shot: Option<PathBuf>,
    /// With --demo: the window's inner size, as WIDTHxHEIGHT.
    #[arg(long, requires = "demo", value_parser = parse_size)]
    pub demo_size: Option<[f32; 2]>,
}

/// What `tabletist` can be asked to do besides opening its window.
#[derive(Debug, Subcommand)]
pub enum Sub {
    /// Connect an agent to the running Tabletist, over stdin and stdout.
    Mcp {
        /// The folder of the app's socket and key, for tests.
        #[arg(long, hide = true)]
        endpoint: Option<PathBuf>,
    },
}

fn parse_size(text: &str) -> Result<[f32; 2], String> {
    let (width, height) = text
        .split_once('x')
        .ok_or_else(|| format!("expected WIDTHxHEIGHT, got {text:?}"))?;
    let parse = |part: &str| {
        part.trim()
            .parse::<f32>()
            .map_err(|_| format!("not a number: {part:?}"))
    };
    Ok([parse(width)?, parse(height)?])
}

use crate::app::App;
use crate::paths::AppDirs;
use crate::settings::Settings;

/// The Wayland app-id and X11 WM class, matching the `.desktop` file so
/// Hyprland window rules and launchers find the window.
pub const APP_ID: &str = "dev.tabletist.Tabletist";

/// Runs the app, and tells the user why when it cannot start. A release
/// build on Windows has no console, and an app launched from a menu has no
/// terminal, so stderr alone would be a silent exit.
pub fn run() -> std::process::ExitCode {
    let cli = Cli::parse();
    // Before the logger, the settings and the window: the relay writes
    // nothing but the protocol to stdout and shows no dialog.
    if let Some(Sub::Mcp { endpoint }) = cli.command {
        return crate::relay::main(endpoint);
    }
    match main(cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            report_startup_error(&error);
            std::process::ExitCode::FAILURE
        }
    }
}

/// The text of the startup error dialog, with every cause in the chain.
pub fn startup_error_message(error: &anyhow::Error) -> String {
    format!("Tabletist could not start: {error:#}")
}

/// Logs the error and shows it in a native dialog. rfd's message dialog
/// needs no running event loop, so it works after eframe gave up.
fn report_startup_error(error: &anyhow::Error) {
    let message = startup_error_message(error);
    // The logger prints to stderr too; before it is up, print directly.
    if log::max_level() == log::LevelFilter::Off {
        eprintln!("{message}");
    } else {
        log::error!("{message}");
    }
    rfd::MessageDialog::new()
        .set_level(rfd::MessageLevel::Error)
        .set_title("Tabletist")
        .set_description(message)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

pub fn main(cli: Cli) -> anyhow::Result<()> {
    // Demo runs use a throwaway profile: nothing touches the user's files.
    // It lives until the window closes and is removed when `main` returns.
    let demo_root = if cli.demo {
        Some(demo_profile()?)
    } else {
        None
    };
    let dirs = match &demo_root {
        Some(root) => AppDirs::at(root.path()),
        None => AppDirs::discover(),
    };
    let mut logging = fastframe_log::Logging::new("tabletist", env!("CARGO_PKG_VERSION"))
        .filter(log_filter(cli.verbose))
        .redact(without_agents_words);
    // fastframe-log would create the log's folder itself, but with default
    // permissions; made here first, it is private to the user.
    if let Err(error) = dirs.ensure() {
        eprintln!("could not create {}: {error}", dirs.state.display());
    }
    if !cli.demo {
        logging = logging.file(dirs.log_file()).panic_log(dirs.panic_log());
    }
    logging.init()?;

    let settings = Settings::load(&dirs);
    let demo = cli.demo;
    let shot = cli.demo_shot.clone().map(Shot::new);
    eframe::run_native(
        "Tabletist",
        native_options(cli.demo_size, !demo),
        Box::new(move |cc| {
            let repaint = cc.egui_ctx.clone();
            let backend = crate::backend::Backend::start_with(
                crate::backend::Waker::new(move || repaint.request_repaint()),
                keyring_for(demo),
            );
            let mut app = App::new(dirs, settings, backend);
            app.attach(&cc.egui_ctx, !demo);
            // Not for a demo: its profile and its keyring are throwaway.
            if !demo {
                app.serve_agents();
            }
            if demo {
                demo_setup(&mut app);
            }
            // The menu bar is up by now: winit builds its own before the
            // window, and this one takes its place.
            let menu = crate::menu::Bar::attach(cc, &app, &cc.egui_ctx);
            // macOS only for now: tabletist-menubar does not attach on
            // Windows yet, and a warning at every launch there would say
            // nothing. Windows joins once it attaches.
            if menu.is_none() && cfg!(target_os = "macos") {
                log::warn!("the menu bar could not be attached");
            }
            Ok(Box::new(Window {
                app,
                shot,
                demo,
                #[cfg(target_os = "macos")]
                title_bar: None,
                menu,
            }))
        }),
    )
    .map_err(|error| anyhow::anyhow!("could not open the window: {error}"))
}

/// The keyring for saved passwords. Demo runs keep secrets in memory, so a
/// throwaway profile never reads or writes the user's OS keyring.
fn keyring_for(demo: bool) -> crate::secrets::Keyring {
    if demo {
        crate::secrets::Keyring::memory()
    } else {
        crate::secrets::Keyring::native()
    }
}

/// A fresh directory under the system temp dir for a demo profile. The name
/// is random and the directory is created exclusively (0700 on Unix), so a
/// directory someone else prepared in a shared temp dir is never reused. It
/// is removed when dropped.
fn demo_profile() -> anyhow::Result<tempfile::TempDir> {
    let mut builder = tempfile::Builder::new();
    builder.prefix("tabletist-demo-");
    #[cfg(unix)]
    builder.permissions(std::os::unix::fs::PermissionsExt::from_mode(0o700));
    Ok(builder.tempdir()?)
}

/// Fills a demo profile: the shared SQLite fixture as a saved "Demo"
/// connection, connected in the first tab with the users table open.
pub fn demo_setup(app: &mut App) {
    let path = app.dirs.state.join("demo.sqlite");
    if let Err(error) = std::fs::create_dir_all(&app.dirs.state) {
        log::error!("could not create the demo profile: {error}");
        return;
    }
    if !path.exists()
        && let Err(error) = tabletist_db::fixtures::write_sqlite_demo(&path)
    {
        log::error!("could not write the demo database: {error}");
        return;
    }
    let saved = crate::connections::SavedConnection {
        id: crate::connections::ConnectionId::new(),
        name: "Demo".into(),
        environment: crate::env::Environment::Dev,
        read_only: None,
        password: crate::connections::PasswordMode::None,
        ssh_secret: crate::connections::PasswordMode::None,
        spec: tabletist_db::ConnectSpec::sqlite(&path),
        agents: Default::default(),
    };
    let conn = saved.id.clone();
    app.connections.upsert(saved);
    let tab = app.active_tab_id();
    app.apply(crate::model::Action::Connect { tab, conn });
    if let Some(workspace) = app.workspace_mut(tab) {
        workspace.pending_open = Some((
            tabletist_db::ObjectRef::new("main", "users"),
            tabletist_db::ObjectKind::Table,
        ));
    }
}

pub fn native_options(size: Option<[f32; 2]>, persist: bool) -> eframe::NativeOptions {
    let viewport = egui::ViewportBuilder::default()
        .with_title("Tabletist")
        .with_app_id(APP_ID)
        .with_inner_size(size.unwrap_or([1280.0, 800.0]))
        .with_min_inner_size([720.0, 480.0]);
    // The window, taskbar and Dock icon. macOS gets the plate on Apple's icon
    // grid, the same art as the bundle's .icns: eframe hands this icon to
    // setApplicationIconImage, and a binary run outside Tabletist.app (cargo
    // run) has no .icns, so without it the Dock shows the generic "exec" icon.
    let png: &[u8] = if cfg!(target_os = "macos") {
        include_bytes!("../packaging/macos/icon-1024.png")
    } else {
        include_bytes!("../assets/icon/tabletist-256.png")
    };
    let viewport = match eframe::icon_data::from_png_bytes(png) {
        Ok(icon) => viewport.with_icon(icon),
        Err(error) => {
            log::warn!("could not load the window icon: {error}");
            viewport
        }
    };
    // macOS: no separate title strip. The connection bar sits in the title
    // bar next to the window buttons, as Safari's tabs do.
    let viewport = if cfg!(target_os = "macos") {
        viewport
            .with_fullsize_content_view(true)
            .with_titlebar_shown(false)
            .with_title_shown(false)
    } else {
        viewport
    };
    eframe::NativeOptions {
        viewport,
        persist_window: persist,
        ..Default::default()
    }
}

/// A pending --demo-shot: when to take it and where to write it.
struct Shot {
    path: PathBuf,
    due: std::time::Instant,
    asked: bool,
}

impl Shot {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            // Enough frames for fonts, icons and (later) demo queries to land.
            due: std::time::Instant::now() + std::time::Duration::from_millis(1500),
            asked: false,
        }
    }
}

struct Window {
    app: App,
    shot: Option<Shot>,
    demo: bool,
    /// macOS: the window whose unified title bar the tabs share, once it
    /// exists.
    #[cfg(target_os = "macos")]
    title_bar: Option<crate::macos::UnifiedTitleBar>,
    /// The menu bar, where the platform has one.
    menu: Option<crate::menu::Bar>,
}

impl Window {
    fn drive_shot(&mut self, ctx: &egui::Context) {
        let Some(shot) = self.shot.as_mut() else {
            return;
        };
        ctx.request_repaint();
        if !shot.asked && std::time::Instant::now() >= shot.due {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
            shot.asked = true;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            return;
        };
        let [width, height] = [image.size[0] as u32, image.size[1] as u32];
        let pixels: Vec<u8> = image
            .pixels
            .iter()
            .flat_map(|pixel| pixel.to_srgba_unmultiplied())
            .collect();
        match ::image::RgbaImage::from_raw(width, height, pixels) {
            Some(buffer) => match buffer.save(&shot.path) {
                Ok(()) => log::info!("wrote {width}x{height} to {}", shot.path.display()),
                Err(error) => log::error!("could not write {}: {error}", shot.path.display()),
            },
            None => log::error!("the frame buffer did not match {width}x{height}"),
        }
        self.shot = None;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }
}

impl eframe::App for Window {
    fn persist_egui_memory(&self) -> bool {
        !self.demo
    }

    fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
        // Before the frame's `logic`: a key a menu took goes back into
        // this frame's input.
        if let Some(menu) = &mut self.menu {
            menu.input(raw, &self.app);
        }
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let clicks = self
            .menu
            .as_mut()
            .map(crate::menu::Bar::clicks)
            .unwrap_or_default();
        for item in clicks {
            match crate::menu::window_command(item) {
                Some(command) => {
                    ctx.send_viewport_cmd(command);
                    // A paste lands in the next frame's input.
                    ctx.request_repaint();
                }
                None => self.app.choose(item),
            }
        }
        self.app.logic(ctx);
        self.drive_shot(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // What is open, for the next launch.
        self.app.save_session();
        // Connections and known hosts saved just before quitting still land.
        self.app.backend.flush(std::time::Duration::from_secs(5));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        #[cfg(target_os = "macos")]
        {
            if self.title_bar.is_none() {
                self.title_bar = crate::macos::UnifiedTitleBar::attach();
            }
            if let Some(title_bar) = &self.title_bar {
                self.app.titlebar = title_bar.measure();
            }
        }
        self.app.frame_ui(ui);
        #[cfg(target_os = "macos")]
        if let Some(title_bar) = &self.title_bar {
            let zoom = ui.ctx().zoom_factor();
            title_bar.place_buttons(crate::ui::window_buttons_line(&self.app, zoom) * zoom);
        }
        // After the frame's actions (`frame_ui` applies them): what is
        // enabled now, and the keys if the keymap or the look changed.
        if let Some(menu) = &mut self.menu {
            menu.sync(&self.app);
        }
    }
}

/// What the app's log takes, at its usual level and at `--verbose`.
///
/// Never a line of the MCP server's SDK (`rmcp`, and `tracing`, which
/// writes lines of its own for the SDK's spans), of `sqlparser`, which
/// reads an agent's statement, or of `tokio_postgres`, which writes the
/// statement it runs and its values at debug, and a server's notice at
/// info: they write what a client sent, or what its statement made the
/// server say, and `tabletist.log` holds none of that (`AGENTS.md`). `RUST_LOG`, where it is set, takes the filter's
/// place as a whole: [`without_agents_words`] is what holds then.
fn log_filter(verbose: bool) -> &'static str {
    match verbose {
        true => {
            "info,tabletist=debug,tabletist_db=debug,rmcp=off,tracing=off,sqlparser=off,tokio_postgres=off"
        }
        false => "warn,tabletist=info,rmcp=off,tracing=off,sqlparser=off,tokio_postgres=off",
    }
}

/// The crates whose lines can hold what an agent sent. `mysql_async`
/// writes no line of its own: its spans are behind a feature that is not
/// built.
const AGENTS_WORDS: [&str; 4] = ["rmcp", "tracing", "sqlparser", "tokio_postgres"];

/// A line of one of [`AGENTS_WORDS`] without its words, for when a
/// `RUST_LOG` lets it through: that the line was there, and nothing of it.
fn without_agents_words(
    record: &log::Record<'_>,
    _message: &str,
) -> Option<std::borrow::Cow<'static, str>> {
    let target = record.target();
    let theirs = AGENTS_WORDS
        .iter()
        .any(|crate_name| target == *crate_name || target.starts_with(&format!("{crate_name}::")));
    theirs.then_some(std::borrow::Cow::Borrowed(
        "(a line left out: it can hold what an agent sent)",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_that_can_hold_an_agents_words_is_written_without_them() {
        let line = |target: &str| {
            let record = log::Record::builder()
                .target(target)
                .level(log::Level::Debug)
                .args(format_args!("received request SELECT email FROM customers"))
                .build();
            without_agents_words(&record, "received request SELECT email FROM customers")
        };
        for target in [
            "rmcp",
            "rmcp::service",
            "tracing::span",
            "sqlparser::parser",
            "tokio_postgres::query",
            "tokio_postgres::connection",
        ] {
            let written = line(target).expect(target);
            assert!(!written.contains("SELECT"), "{target}: {written}");
        }
        // Every other crate's line is as it was, the app's own among them.
        for target in [
            "tabletist",
            "tabletist_mcp::server",
            "zbus",
            "rmcpx",
            "tracingfoo",
            "tokio_postgres_rustls",
        ] {
            assert_eq!(line(target), None, "{target}");
        }
    }

    #[test]
    fn the_log_takes_nothing_of_what_writes_an_agents_words() {
        for verbose in [false, true] {
            let filter = log_filter(verbose);
            for theirs in AGENTS_WORDS {
                let off = format!("{theirs}=off");
                assert!(filter.split(',').any(|said| said == off), "{filter}");
            }
            // And the app's own crates are as they were.
            assert!(filter.contains("tabletist="), "{filter}");
        }
    }

    #[test]
    fn demo_mode_never_uses_the_os_keyring() {
        assert!(!keyring_for(true).is_native());
        assert!(keyring_for(false).is_native());
    }

    #[test]
    fn each_demo_profile_is_fresh_private_and_removed_afterwards() {
        let first = demo_profile().unwrap();
        let second = demo_profile().unwrap();
        assert_ne!(first.path(), second.path());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(first.path())
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o777, 0o700);
        }
        let path = first.path().to_path_buf();
        drop(first);
        assert!(!path.exists());
    }

    #[test]
    fn sizes_parse_as_width_by_height() {
        assert_eq!(parse_size("1280x800"), Ok([1280.0, 800.0]));
        assert!(parse_size("1280").is_err());
        assert!(parse_size("wide x 800").is_err());
    }

    #[test]
    fn a_startup_error_names_the_app_and_every_cause() {
        let error =
            anyhow::anyhow!("no suitable adapter found").context("could not open the window");
        assert_eq!(
            startup_error_message(&error),
            "Tabletist could not start: could not open the window: no suitable adapter found"
        );
    }

    #[test]
    fn native_options_carry_the_app_id_and_size() {
        let options = native_options(Some([900.0, 600.0]), false);
        assert_eq!(options.viewport.app_id.as_deref(), Some(APP_ID));
        assert_eq!(options.viewport.inner_size, Some(egui::vec2(900.0, 600.0)));
        assert!(!options.persist_window);
    }

    #[test]
    fn the_desktop_file_names_the_app_id() {
        let desktop = include_str!("../packaging/linux/dev.tabletist.Tabletist.desktop");
        assert!(desktop.contains("StartupWMClass=dev.tabletist.Tabletist"));
        assert!(desktop.contains("Exec=tabletist"));
    }

    #[test]
    fn the_macos_icon_sits_on_apples_icon_grid() {
        // Apple's template centres an 824 px plate in the 1024 px canvas; a
        // plate that fills the canvas looks too big in the Dock and Finder.
        let icon = image::load_from_memory(include_bytes!("../packaging/macos/icon-1024.png"))
            .expect("the icon is a PNG")
            .to_rgba8();
        assert_eq!(icon.dimensions(), (1024, 1024));
        // The opaque plate's bounds; the shadow around it is translucent.
        let (left, right, top, bottom) = icon
            .enumerate_pixels()
            .filter(|(_, _, pixel)| pixel[3] == 255)
            .fold((u32::MAX, 0, u32::MAX, 0), |(l, r, t, b), (x, y, _)| {
                (l.min(x), r.max(x), t.min(y), b.max(y))
            });
        for (edge, value, expected) in [
            ("left", left, 100),
            ("right", right, 923),
            ("top", top, 100),
            ("bottom", bottom, 923),
        ] {
            assert!(
                value.abs_diff(expected) <= 2,
                "the plate's {edge} edge is at {value}, not {expected}"
            );
        }
    }

    #[test]
    fn demo_mode_browses_the_fixture_end_to_end_at_every_size() {
        use crate::testing::Harness;
        for size in [
            egui::vec2(720.0, 480.0),
            egui::vec2(1280.0, 800.0),
            egui::vec2(2560.0, 1440.0),
        ] {
            let backend = crate::backend::Backend::start_with(
                crate::backend::Waker::default(),
                keyring_for(true),
            );
            let mut harness = Harness::with_backend(size, backend);
            demo_setup(&mut harness.app);
            let loaded = harness.run_until(std::time::Duration::from_secs(20), |app| {
                app.workspace(app.active_tab_id())
                    .and_then(|workspace| workspace.active_object_tab())
                    .and_then(|object| object.page())
                    .is_some_and(|page| page.rows.len() == 5)
            });
            assert!(loaded, "the demo users table loads at {size:?}");
            assert!(harness.has("users tab"));
            assert!(harness.has("email"));
            assert!(harness.has("Rows 1–5 of 5"));
            harness.click("Row 3");
            assert!(harness.has("Copy email"));
            harness.click("Structure");
            assert!(
                harness.run_until(std::time::Duration::from_secs(10), |app| {
                    app.workspace(app.active_tab_id())
                        .and_then(|workspace| workspace.active_object_tab())
                        .is_some_and(|object| object.structure.value.is_some())
                })
            );
            assert!(harness.has("users_name_idx"));
        }
    }

    #[test]
    fn a_session_closed_while_idle_connects_again_for_the_next_fetch() {
        use crate::testing::Harness;
        let backend = crate::backend::Backend::start_with(
            crate::backend::Waker::default(),
            keyring_for(true),
        );
        let mut harness = Harness::with_backend(egui::vec2(1280.0, 800.0), backend);
        demo_setup(&mut harness.app);
        let rows = |app: &crate::app::App| {
            app.workspace(app.active_tab_id())
                .and_then(|workspace| workspace.active_object_tab())
                .and_then(|object| object.page())
                .map(|page| page.rows.len())
        };
        let wait = std::time::Duration::from_secs(20);
        // And nothing else waited for: a session still asked for the
        // structure or the sidebar's lists is in use, and not closed.
        let settled = |app: &crate::app::App| {
            rows(app) == Some(5)
                && app
                    .workspace(app.active_tab_id())
                    .is_some_and(|workspace| !workspace.waits())
        };
        assert!(harness.run_until(wait, settled));
        let tab = harness.app.active_tab_id();
        let old = harness.app.workspace(tab).unwrap().session;
        let later = std::time::Instant::now() + std::time::Duration::from_secs(31 * 60);
        harness.app.close_idle(later);
        assert!(matches!(
            harness.app.workspace(tab).unwrap().status,
            crate::model::SessionStatus::Idle
        ));
        harness.app.apply(crate::model::Action::Refresh(tab));
        let answered = harness.run_until(wait, |app| {
            let workspace = app.workspace(tab).unwrap();
            matches!(workspace.status, crate::model::SessionStatus::Connected)
                && workspace.session != old
                && workspace
                    .active_object_tab()
                    .is_some_and(|object| !object.rows.is_loading() && object.rows.error.is_none())
        });
        assert!(answered, "the new session answers the fetch");
        assert_eq!(rows(&harness.app), Some(5));
    }

    #[test]
    fn the_desktop_entry_matches_the_app_id() {
        let entry = include_str!("../packaging/linux/dev.tabletist.Tabletist.desktop");
        assert!(entry.contains(&format!("StartupWMClass={APP_ID}")));
        assert!(entry.contains(&format!("Icon={APP_ID}")));
        assert!(entry.contains("Exec=tabletist"));
    }

    #[test]
    fn the_window_has_the_app_icon() {
        let options = native_options(None, false);
        let icon = options.viewport.icon.expect("an icon");
        // macOS gets the plate on Apple's icon grid, like the .icns.
        let size = if cfg!(target_os = "macos") { 1024 } else { 256 };
        assert_eq!((icon.width, icon.height), (size, size));
    }
}
