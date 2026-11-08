//! Each look this system can choose draws in the app as it ships. The
//! app's own tests build every bundled face (`cfg(test)`), so a face a
//! look names but this system's build lacks shows only here, where the
//! app is built as it ships: the first label of that look would panic.

use tabletist::app::App;
use tabletist::backend::Backend;
use tabletist::paths::AppDirs;
use tabletist::settings::{LookChoice, Settings};

#[test]
fn each_look_this_system_can_choose_draws() {
    let root = std::env::temp_dir().join(format!("tabletist-looks-{}", std::process::id()));
    for choice in [None, Some(LookChoice::Classic), Some(LookChoice::Omarchy)] {
        let dir = root.join(choice.map_or("none", LookChoice::name));
        std::fs::create_dir_all(&dir).expect("a settings folder");
        let settings = Settings {
            look: choice,
            ..Settings::default()
        };
        let mut app = App::new(AppDirs::at(&dir), settings.into(), Backend::recording());
        let ctx = egui::Context::default();
        app.attach(&ctx, false);
        // The start draws the look the settings choose.
        for _ in 0..3 {
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| app.frame_ui(ui));
            // No renderer takes the textures.
            output.textures_delta.clear();
        }
    }
    let _ = std::fs::remove_dir_all(&root);
}
