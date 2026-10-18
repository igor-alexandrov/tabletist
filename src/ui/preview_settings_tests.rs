//! Settings › Appearance, its preview: two Bookshop rows under a dev
//! connection's bar, drawn as the options have them as they change, and
//! nothing asked of the backend for them.

use egui::accesskit::Role;

use super::appearance_settings_tests::{on_the_pane, shows};
use crate::backend::{Command, StateFile};
use crate::env::{Environment, Platform};
use crate::model::{Action, SettingsPane};
use crate::settings::{
    EnvBar, FontSize, InterfaceSize, OptionValue, RowHeight, TableList, ThemeChoice,
};
use crate::testing::{Harness, bounds};
use crate::theme::Look;

/// The looks that draw the sheet: Windows draws the macOS one, with its
/// own grid.
fn looks() -> [Look; 2] {
    [Look::standard(), Look::macos()]
}

/// Settings on its Appearance pane, in a window as large as the look's
/// board, with the sheet's rows scrolled to their end: the preview.
fn shown(look: Look) -> Harness {
    shown_over(look, false)
}

/// [`shown`], over the Bookshop's `book_covers` open in a table where
/// `table` says.
fn shown_over(look: Look, table: bool) -> Harness {
    let size = if look.terminal {
        egui::vec2(1920.0, 1080.0)
    } else {
        egui::vec2(1440.0, 1000.0)
    };
    let mut harness = Harness::with_size(size);
    harness.set_look(look);
    if table {
        harness.book_covers();
    }
    on_the_pane(&mut harness);
    // The board's window shows all of the pane; the rows scroll where a
    // larger text or taller rows make them taller than the sheet leaves.
    if !look.terminal {
        wheel(&mut harness, -2000.0);
    }
    harness
}

/// The wheel turned by `delta` over the middle of the window.
fn wheel(harness: &mut Harness, delta: f32) {
    let middle = (harness.size / 2.0).to_pos2();
    wheel_at(harness, middle, delta);
}

/// The wheel turned by `delta` with the pointer at `at`.
fn wheel_at(harness: &mut Harness, at: egui::Pos2, delta: f32) {
    harness.frame(vec![
        egui::Event::PointerMoved(at),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, delta),
            modifiers: egui::Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        },
    ]);
    // The wheel scrolls smoothly: until it has stopped.
    for _ in 0..5 {
        harness.finish_animations();
    }
    // A scroll asked for lands two frames later.
    harness.settle();
    harness.settle();
}

/// An option set, as the pane's controls set it, and the app's logic run
/// after it, as the window runs it between frames: a theme is put in
/// place there.
fn set(harness: &mut Harness, value: OptionValue) {
    harness.app.apply(Action::SetOption(value));
    let mut input = egui::RawInput::default();
    input.viewports.entry(egui::ViewportId::ROOT).or_default();
    let app = &mut harness.app;
    let _ = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
    harness.settle();
    harness.settle();
}

/// Whether the preview shows `text`, the whole of a piece of text.
fn in_preview(harness: &mut Harness, text: &str) -> bool {
    let preview = preview(harness);
    harness
        .text_rects
        .iter()
        .any(|(painted, at)| painted == text && preview.contains_rect(*at))
}

/// Where the preview is: the group a screen reader is told of.
fn preview(harness: &mut Harness) -> egui::Rect {
    let tree = harness.settle();
    let name = harness.app.look.label("Preview");
    bounds(&tree, &name, Role::Group).expect("the preview")
}

/// The preview's two rows, as drawn.
fn rows(harness: &mut Harness) -> [egui::Rect; 2] {
    let preview = preview(harness);
    let tree = harness.settle();
    // In the preview: a table open under the sheet has rows of these names.
    let row = |number: u64| {
        let name = format!("Row {number}");
        tree.nodes
            .iter()
            .filter(|(_, node)| node.role() == Role::Button && node.label() == Some(&name))
            .filter_map(|(_, node)| node.bounds())
            .map(|rect| {
                egui::Rect::from_min_max(
                    egui::pos2(rect.x0 as f32, rect.y0 as f32),
                    egui::pos2(rect.x1 as f32, rect.y1 as f32),
                )
            })
            .find(|rect| preview.contains_rect(*rect))
            .expect("a row in the preview")
    };
    [row(1), row(2)]
}

/// Whether the preview has a fill of `color` over all of `rect`.
fn filled(harness: &mut Harness, rect: egui::Rect, color: egui::Color32) -> bool {
    let preview = preview(harness);
    harness.fills.iter().any(|(fill, painted)| {
        *painted == color && preview.contains_rect(*fill) && fill.contains_rect(rect)
    })
}

#[test]
fn the_sheet_says_what_the_preview_does_under_its_title() {
    let mut harness = shown(Look::macos());
    let words = "Theme, type and how dense the grid is. The preview updates as you change things.";
    assert!(shows(&mut harness, words));
}

#[test]
fn the_preview_draws_two_bookshop_rows_under_a_dev_bar() {
    for look in looks() {
        let mut harness = shown(look);
        let preview = preview(&mut harness);
        for words in [
            "Bookshop",
            Environment::Dev.label(Platform::of(&look)),
            "publisher_id",
            "created_at",
            "9100000000000000001",
            "9100000000000000002",
            "print",
            "ebook",
            "2026-01-12 09:14:03",
            "2026-01-12 17:02:41",
        ] {
            assert!(shows(&mut harness, words), "{} {words}", look.name);
            let at = harness.painted_rect(words).expect("painted");
            assert!(preview.contains_rect(at), "{} {words}", look.name);
        }
        // The terminal's bar names the database too, as its chip does.
        assert_eq!(
            shows(&mut harness, "bookshop_development"),
            look.terminal,
            "{}",
            look.name
        );
        for row in rows(&mut harness) {
            assert!(preview.contains_rect(row), "{}", look.name);
        }
        // The kinds are value tags, each in its slot's colour.
        for (kind, slot) in [("print", 0), ("ebook", 1)] {
            let (ink, _) =
                crate::ui::value_tags::slot_colors(slot, &harness.app.look, &harness.app.palette);
            assert_eq!(
                harness.painted_color(kind),
                Some(ink),
                "{} {kind}",
                look.name
            );
        }
    }
}

#[test]
fn the_preview_s_rows_follow_the_grid_options() {
    for look in looks() {
        let mut harness = shown(look);
        // Each density's rows, as the table grid has them.
        let mut heights = Vec::new();
        for height in [RowHeight::Compact, RowHeight::Regular, RowHeight::Roomy] {
            set(&mut harness, OptionValue::RowHeight(height));
            if !look.terminal {
                wheel(&mut harness, -2000.0);
            }
            let expected = harness.app.look.with_grid(&harness.app.settings).row();
            for row in rows(&mut harness) {
                assert_eq!(row.height(), expected, "{} {height:?}", look.name);
            }
            heights.push(expected);
        }
        assert!(
            heights.is_sorted() && heights[0] < heights[2],
            "{heights:?}"
        );
        // The second row is striped where the rows are.
        let panel = harness.app.palette.panel;
        for zebra in [true, false] {
            set(&mut harness, OptionValue::Zebra(Some(zebra)));
            let [first, second] = rows(&mut harness);
            assert_eq!(filled(&mut harness, second, panel), zebra, "{}", look.name);
            assert!(!filled(&mut harness, first, panel), "{}", look.name);
        }
        // Without the types the header is one line, and says no type.
        let header = |harness: &mut Harness| rows(harness)[0].top() - preview(harness).top();
        let two = header(&mut harness);
        assert!(in_preview(&mut harness, "varchar"), "{}", look.name);
        set(&mut harness, OptionValue::ColumnTypes(false));
        let one = header(&mut harness);
        assert!(one < two, "{} {one} {two}", look.name);
        assert!(!in_preview(&mut harness, "varchar"), "{}", look.name);
    }
}

#[test]
fn the_preview_s_bar_follows_the_environment_option() {
    for look in looks() {
        let mut harness = shown(look);
        let platform = Platform::of(&look);
        let palette = harness.app.palette;
        let (tint, _) = crate::env::bar_colors(Environment::Dev, EnvBar::Tint, platform, &palette);
        let bar = |harness: &mut Harness| {
            let preview = preview(harness);
            let name = harness.painted_rect("Bookshop").expect("the name");
            egui::Rect::from_x_y_ranges(preview.shrink(1.0).x_range(), name.y_range())
        };
        let at = bar(&mut harness);
        assert!(filled(&mut harness, at, tint), "{}", look.name);
        set(&mut harness, OptionValue::EnvBar(EnvBar::Label));
        let at = bar(&mut harness);
        assert!(!filled(&mut harness, at, tint), "{}", look.name);
        let panel = harness.app.palette.panel;
        assert!(filled(&mut harness, at, panel), "{}", look.name);
        // The environment is still said.
        let label = Environment::Dev.label(platform);
        assert!(in_preview(&mut harness, label), "{}", look.name);
    }
}

#[test]
fn the_preview_follows_the_theme_and_the_text() {
    let mut harness = shown(Look::macos());
    set(&mut harness, OptionValue::Theme(ThemeChoice::Light));
    let light = harness.painted_color("Bookshop");
    assert_eq!(light, Some(harness.app.palette.text));
    set(&mut harness, OptionValue::Theme(ThemeChoice::Dark));
    assert!(harness.app.palette.dark);
    let dark = harness.painted_color("Bookshop");
    assert_eq!(dark, Some(harness.app.palette.text));
    assert_ne!(light, dark);
    // The data's size widens a cell's text, the interface's the header.
    let wide = |harness: &mut Harness, text: &str| {
        harness.settle();
        harness.painted_rect(text).expect("painted").width()
    };
    let (cell, name) = (
        wide(&mut harness, "9100000000000000001"),
        wide(&mut harness, "publisher_id"),
    );
    let left = |harness: &mut Harness| {
        harness.settle();
        let text = harness.painted_rect("2026-01-12 09:14:03");
        text.expect("painted").left()
    };
    let after = left(&mut harness);
    set(&mut harness, OptionValue::FontSize(FontSize::of(16.0)));
    assert!(wide(&mut harness, "9100000000000000001") > cell);
    assert_eq!(wide(&mut harness, "publisher_id"), name);
    // The columns are fitted again: the ones after the ids move right.
    assert!(left(&mut harness) > after);
    set(
        &mut harness,
        OptionValue::InterfaceSize(InterfaceSize::Large),
    );
    assert!(wide(&mut harness, "publisher_id") > name);
    // Value tags off: the kinds are plain text.
    set(&mut harness, OptionValue::ValueTags(false));
    assert_eq!(
        harness.painted_color("print"),
        Some(harness.app.palette.text)
    );
}

#[test]
fn the_preview_sends_the_backend_nothing() {
    for look in looks() {
        // A table open under the sheet, which a frame could ask rows for.
        let mut harness = shown_over(look, true);
        let before = harness.app.backend.sent.len();
        let mut values = vec![
            OptionValue::RowHeight(RowHeight::Roomy),
            OptionValue::Zebra(Some(!look.terminal)),
            OptionValue::ColumnTypes(false),
            OptionValue::TableList(TableList::Flat),
            OptionValue::EnvBar(EnvBar::Label),
            OptionValue::ValueTags(false),
            OptionValue::FontSize(FontSize::of(15.0)),
        ];
        if !look.terminal {
            values.push(OptionValue::Theme(ThemeChoice::Dark));
            values.push(OptionValue::InterfaceSize(InterfaceSize::Small));
            values.push(OptionValue::Font(Some("JetBrains Mono".into())));
        }
        let changes = values.len();
        for value in values {
            set(&mut harness, value);
            wheel(&mut harness, -2000.0);
        }
        // And every option put back at once.
        harness.app.apply(Action::ResetSettings);
        harness.app.apply(Action::ConfirmResetSettings(true));
        wheel(&mut harness, -2000.0);
        assert!(harness.painted_rect("Bookshop").is_some(), "{}", look.name);
        rows(&mut harness);
        // Each change is written to the file, and that is all.
        let sent = &harness.app.backend.sent[before..];
        assert_eq!(sent.len(), changes + 1, "{} {sent:?}", look.name);
        for command in sent {
            assert!(
                matches!(
                    command,
                    Command::Save {
                        file: StateFile::Settings(_),
                        ..
                    }
                ),
                "{} {command:?}",
                look.name
            );
        }
    }
}

#[test]
fn the_preview_takes_no_tab_pointer_or_wheel() {
    let mut harness = shown(Look::macos());
    // Tab goes round the sheet's controls and never into the preview.
    for _ in 0..40 {
        harness.press(egui::Key::Tab, egui::Modifiers::NONE);
        let preview = preview(&mut harness);
        let focused = harness
            .ctx
            .memory(|memory| memory.focused())
            .and_then(|id| harness.ctx.read_response(id));
        if let Some(focused) = focused {
            assert!(!preview.intersects(focused.rect), "{:?}", focused.rect);
        }
    }
    // A row under the pointer is not lit, as a table's is, and a click on
    // it takes the keyboard nowhere and asks for nothing.
    wheel(&mut harness, -2000.0);
    let [row, _] = rows(&mut harness);
    harness.frame(vec![egui::Event::PointerMoved(row.center())]);
    let palette = harness.app.palette;
    let hover = palette.panel.lerp_to_gamma(palette.surface, 0.5);
    assert!(!filled(&mut harness, row, hover));
    let sent = harness.app.backend.sent.len();
    let press = |pressed| egui::Event::PointerButton {
        pos: row.center(),
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    };
    harness.frame(vec![press(true)]);
    harness.frame(vec![press(false)]);
    harness.settle();
    assert_eq!(harness.ctx.memory(|memory| memory.focused()), None);
    assert_eq!(harness.app.backend.sent.len(), sent);
    assert!(harness.app.dialog.is_some(), "the sheet stays");
    // In a window too short for all of the pane, the wheel over the
    // preview scrolls the sheet: the preview moves down.
    let mut harness = Harness::new();
    harness.set_look(Look::macos());
    on_the_pane(&mut harness);
    wheel(&mut harness, -2000.0);
    let before = preview(&mut harness);
    wheel_at(&mut harness, before.center(), 300.0);
    assert!(preview(&mut harness).top() > before.top());
}

#[test]
fn the_line_under_a_pane_s_title_wraps_in_the_smallest_sheet() {
    // The sheet ends 40 in from the window's edge.
    for (pane, words) in [
        (SettingsPane::General, "How Tabletist shows data"),
        (SettingsPane::Appearance, "Theme, type and how dense"),
    ] {
        let mut harness = Harness::with_size(egui::vec2(720.0, 480.0));
        harness.set_look(Look::macos());
        harness.app.apply(Action::ShowSettings);
        harness.app.apply(Action::ShowSettingsPane(pane));
        harness.settle();
        let line = harness
            .text_rects
            .iter()
            .find(|(text, _)| text.starts_with(words))
            .map(|(_, at)| *at)
            .expect("the line");
        assert!(line.right() <= 680.0, "{pane:?} {line:?}");
    }
}
