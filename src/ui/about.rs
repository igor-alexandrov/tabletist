//! The About dialog: the icon, the version, and a link to where the project
//! lives.

use egui::{Sense, WidgetInfo, WidgetType, pos2, vec2};

use crate::app::App;
use crate::i18n::gettext;
use crate::model::{Action, Dialog};
use crate::typography::Text;
use crate::ui::widgets;

/// The art every icon size is made from.
const ICON: &[u8] = include_bytes!("../../assets/icon/tabletist.svg");

const ICON_SIZE: f32 = 72.0;

/// As LICENSE has it.
const COPYRIGHT: &str = "© 2026 Igor Alexandrov";

/// The fonts this build bundles, each with its licence as the font's
/// authors give it: the SIL Open Font License has its notice go with the
/// font. Windows draws in fastframe's fonts, and bundles none of its own.
const FONT_NOTICES: &[&str] = &[
    #[cfg(target_os = "macos")]
    include_str!("../../assets/fonts/IBMPlex-OFL.txt"),
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    include_str!("../../assets/fonts/JetBrainsMono-OFL.txt"),
    #[cfg(target_os = "macos")]
    include_str!("../../assets/fonts/NotoSansSymbols2-OFL.txt"),
];

/// Whether the fonts' notices are unfolded, in egui's memory.
fn notices_id() -> egui::Id {
    egui::Id::new("about-font-licenses")
}

/// How tall the fonts' notices are at most: they scroll.
const NOTICES_HEIGHT: f32 = 140.0;

/// The version and what it was built for: what a bug report needs.
pub fn version() -> String {
    format!(
        "{} ({} {})",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        std::env::consts::ARCH
    )
}

/// The repository without its scheme, as the dialog shows it.
pub fn repository() -> &'static str {
    let url = env!("CARGO_PKG_REPOSITORY");
    url.strip_prefix("https://").unwrap_or(url)
}

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !matches!(app.dialog, Some(Dialog::About)) {
        // The next About starts with the notices folded, whatever took
        // its place.
        ctx.data_mut(|data| data.remove::<bool>(notices_id()));
        return;
    }
    let locale = app.locale;
    let palette = app.palette;
    let look = app.look;
    let mut actions = Vec::new();
    let modal = widgets::modal(egui::Id::new("about"), &look, &palette).show(ctx, |ui| {
        ui.set_width(380.0);
        ui.vertical_centered(|ui| {
            ui.add(
                egui::Image::from_bytes("bytes://tabletist.svg", ICON)
                    .fit_to_exact_size(vec2(ICON_SIZE, ICON_SIZE))
                    .alt_text(gettext(locale, "Tabletist icon")),
            );
            ui.add_space(10.0);
            widgets::label(
                ui,
                widgets::dialog_title(&look),
                "Tabletist",
                palette.text,
                &look,
            );
            ui.add_space(2.0);
            // Selectable, to paste into a bug report.
            let version = format!("{} {}", gettext(locale, "Version"), version());
            let version = Text::one(&look, widgets::secondary(&look), &version, palette.dim);
            ui.add(egui::Label::new(version.layout(ui.ctx()).galley).selectable(true));
            ui.add_space(12.0);
            // Cargo.toml's description, so the two cannot drift.
            let about = gettext(locale, env!("CARGO_PKG_DESCRIPTION"));
            centered(
                ui,
                Text::one(&look, widgets::body(&look), &about, palette.text),
            );
            ui.add_space(8.0);
            // The source code, opened in the browser.
            let source = widgets::ButtonSpec::new(repository())
                .link()
                .role(widgets::code(&look))
                .padding(6.0);
            if source.show(ui, 24.0, &look, &palette).clicked() {
                ui.ctx()
                    .open_url(egui::OpenUrl::new_tab(env!("CARGO_PKG_REPOSITORY")));
            }
            let license = format!(
                "{COPYRIGHT} · {} {}",
                env!("CARGO_PKG_LICENSE"),
                gettext(locale, "License")
            );
            widgets::label(
                ui,
                widgets::secondary(&look),
                &license,
                palette.secondary,
                &look,
            );
            font_notices(ui, &look, &palette, locale);
        });
        ui.add_space(12.0);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if widgets::button(ui, &gettext(locale, "Close"), &look).clicked() {
                actions.push(Action::CloseDialog);
            }
        });
    });
    if modal.is_top_modal
        && ctx.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape))
    {
        actions.push(Action::CloseDialog);
    }
    app.actions.extend(actions);
}

/// The link that shows the bundled fonts' licences, and under it, once it
/// is pressed, their notices: selectable, to be copied whole. Whether they
/// show is the dialog's own, kept in egui's memory while it is open.
fn font_notices(
    ui: &mut egui::Ui,
    look: &crate::theme::Look,
    palette: &crate::theme::Palette,
    locale: crate::i18n::Locale,
) {
    if FONT_NOTICES.is_empty() {
        return;
    }
    let id = notices_id();
    let mut open = ui.data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
    ui.add_space(2.0);
    let label = gettext(locale, "Font licenses");
    let link = widgets::ButtonSpec::new(&label).link().padding(6.0);
    let size = vec2(link.width(ui, look), 24.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    if link.show_at(ui, rect, look, palette).clicked() {
        open = !open;
        ui.data_mut(|data| data.insert_temp(id, open));
    }
    if !open {
        return;
    }
    ui.add_space(4.0);
    let notices = FONT_NOTICES.join("\n\n");
    egui::ScrollArea::vertical()
        .max_height(NOTICES_HEIGHT)
        .show(ui, |ui| {
            let text = Text::one(look, widgets::secondary(look), &notices, palette.secondary)
                .wrap(ui.available_width())
                .layout(ui.ctx());
            ui.add(egui::Label::new(text.galley).selectable(true));
        });
}

/// `text` across the dialog, wrapped, with every line centred.
fn centered(ui: &mut egui::Ui, mut text: Text) {
    let width = ui.available_width();
    text.job_mut().halign = egui::Align::Center;
    let laid = text.wrap(width).layout(ui.ctx());
    let (rect, response) = ui.allocate_exact_size(vec2(width, laid.height()), Sense::hover());
    let label = laid.galley.text().to_owned();
    response.widget_info(|| WidgetInfo::labeled(WidgetType::Label, true, &label));
    if ui.is_rect_visible(rect) {
        // A centred galley is laid out around its position.
        laid.paint(ui.painter(), pos2(rect.center().x, rect.top()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::Harness;
    use egui::{Key, Modifiers};

    fn open(harness: &mut Harness) {
        harness.frame(vec![egui::Event::Text("?".into())]);
        harness.click("About Tabletist");
    }

    #[test]
    fn the_shortcuts_dialog_opens_about_in_every_look() {
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::new();
            harness.set_look(look);
            open(&mut harness);
            assert!(
                matches!(harness.app.dialog, Some(Dialog::About)),
                "{}",
                look.name
            );
            assert!(
                !harness.has("Keyboard shortcuts"),
                "{}: the shortcuts gave way",
                look.name
            );
            for label in [
                "Tabletist".to_owned(),
                format!("Version {}", version()),
                env!("CARGO_PKG_DESCRIPTION").to_owned(),
                "github.com/igor-alexandrov/tabletist".to_owned(),
                format!("{COPYRIGHT} · MIT License"),
            ] {
                assert!(harness.has(&label), "{}: {label}", look.name);
            }
        }
    }

    #[test]
    fn about_names_the_version_it_was_built_as() {
        let version = version();
        assert!(version.starts_with(env!("CARGO_PKG_VERSION")), "{version}");
        assert!(version.contains(std::env::consts::OS), "{version}");
    }

    #[test]
    fn about_and_the_way_to_it_fit_the_smallest_window() {
        use egui::accesskit::Role;
        let size = egui::vec2(720.0, 480.0);
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, size);
        for look in crate::theme::Look::ALL {
            let mut harness = Harness::with_size(size);
            harness.set_look(look);
            // The shortcuts are taller than the window: they scroll.
            harness.frame(vec![egui::Event::Text("?".into())]);
            let tree = harness.settle();
            let about = crate::testing::bounds(&tree, "About Tabletist", Role::Button)
                .expect("the About button");
            assert!(
                screen.contains_rect(about),
                "{}: About Tabletist at {about:?}",
                look.name
            );
            harness.click("About Tabletist");
            let tree = harness.settle();
            let close =
                crate::testing::bounds(&tree, "Close", Role::Button).expect("the Close button");
            assert!(
                screen.contains_rect(close),
                "{}: Close at {close:?}",
                look.name
            );
        }
    }

    /// Windows bundles no font of its own: it has no link.
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    #[test]
    fn about_shows_the_licenses_of_the_fonts_it_bundles() {
        let mut harness = Harness::new();
        open(&mut harness);
        harness.click("Font licenses");
        harness.settle();
        let shown = |harness: &Harness, words: &str| {
            harness.painted.iter().any(|(text, _)| text.contains(words))
        };
        // This build's: JetBrains Mono on Linux, with its notice whole.
        assert!(shown(&harness, "The JetBrains Mono Project Authors"));
        assert!(shown(&harness, "SIL OPEN FONT LICENSE Version 1.1"));
        // A second press folds them, and so does a close.
        harness.click("Font licenses");
        harness.settle();
        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
        harness.click("Font licenses");
        harness.click("Close");
        open(&mut harness);
        harness.settle();
        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
        // And when it closes another way.
        harness.click("Font licenses");
        harness.app.apply(Action::CloseDialog);
        harness.settle();
        harness.app.apply(Action::ShowAbout);
        harness.settle();
        assert!(!shown(&harness, "SIL OPEN FONT LICENSE"));
    }

    #[test]
    fn the_repository_opens_in_the_browser() {
        let mut harness = Harness::new();
        open(&mut harness);
        harness.click("github.com/igor-alexandrov/tabletist");
        assert_eq!(
            harness.opened.as_deref(),
            Some("https://github.com/igor-alexandrov/tabletist")
        );
        assert!(
            matches!(harness.app.dialog, Some(Dialog::About)),
            "About stays open"
        );
    }

    #[test]
    fn close_and_escape_close_about() {
        let mut harness = Harness::new();
        open(&mut harness);
        harness.click("Close");
        assert!(harness.app.dialog.is_none());
        open(&mut harness);
        harness.press(Key::Escape, Modifiers::NONE);
        assert!(harness.app.dialog.is_none());
    }

    #[test]
    fn about_does_not_replace_a_dialog_being_worked_in() {
        let mut harness = Harness::new();
        harness.press(Key::N, Modifiers::COMMAND);
        harness.app.apply(Action::ShowAbout);
        assert!(matches!(harness.app.dialog, Some(Dialog::Connection(_))));
    }
}
