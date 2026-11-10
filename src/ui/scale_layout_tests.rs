//! The window at its largest: macOS's and Windows's interface at 250% and
//! the terminal look's text at 32, in a 1920 × 1080 window. Nothing is
//! cut off, read from egui's layout and not from pixels: every piece of
//! text fits what shows it, and a control's own words stay inside it.

use egui::accesskit::{Node, TreeUpdate};
use egui::{Key, Modifiers, Rect, Vec2};

use crate::model::{Action, SettingsPane};
use crate::settings::{FontSize, OptionValue, UiScale};
use crate::testing::Harness;
use crate::theme::Look;

/// The window, in pixels, on a display of one pixel a point: the issue's
/// 17-inch laptop.
const WINDOW: Vec2 = egui::vec2(1920.0, 1080.0);

/// A window in `look` at its largest: the macOS look and Windows's at the
/// top scale, the terminal look at its top size. The harness lays out in
/// points, which the zoom makes fewer: the window's pixels over it.
fn largest(look: Look) -> Harness {
    let mut harness = Harness::with_size(WINDOW);
    harness.set_look(look);
    let value = match look.terminal {
        true => OptionValue::FontSize(FontSize::of(32.0)),
        false => OptionValue::InterfaceScale(*UiScale::ALL.last().unwrap()),
    };
    harness.app.apply(Action::SetOption(value));
    zoomed(&mut harness);
    harness
}

/// The zoom the app set, laid out: the logic's pass sets it, and the
/// window has as many points as its pixels over it.
fn zoomed(harness: &mut Harness) {
    let mut input = egui::RawInput::default();
    input.viewports.entry(egui::ViewportId::ROOT).or_default();
    let app = &mut harness.app;
    let _ = harness.ctx.run_logic(&input, |ctx| app.logic(ctx));
    // A zoom set there is the next frame's.
    harness.settle();
    harness.size = WINDOW / harness.ctx.zoom_factor();
    harness.settle();
}

fn looks() -> [Look; 3] {
    [Look::macos(), Look::standard(), Look::omarchy()]
}

/// Whether `rect` holds `inner`, give or take half a point.
fn holds(rect: Rect, inner: Rect) -> bool {
    rect.expand(0.5).contains_rect(inner)
}

/// Every piece of text the last frame painted that is cut off: wider or
/// taller than the place it is clipped to, which ends inside what holds
/// it. Text scrolled out of sight (clipped to nothing), or cut at the
/// edge of a scrolling pane or of the window (a clip that ends where one
/// holding it ends), is a scroll's: it shows whole once scrolled to.
fn cut(harness: &Harness) -> Vec<String> {
    let clips: Vec<Rect> = harness
        .clipped_text
        .iter()
        .map(|(_, _, clip)| *clip)
        .collect();
    let screen = Rect::from_min_size(egui::Pos2::ZERO, harness.size);
    let near = |a: f32, b: f32| (a - b).abs() < 0.5;
    // Whether `clip` ends, on a side `rect` goes past, where a larger
    // place holding it ends.
    let scrolled = |rect: Rect, clip: Rect| {
        clips.iter().chain([&screen]).any(|outer| {
            *outer != clip
                && holds(*outer, clip)
                && ((rect.right() > clip.right() + 0.5 && near(outer.right(), clip.right()))
                    || (rect.left() < clip.left() - 0.5 && near(outer.left(), clip.left()))
                    || (rect.bottom() > clip.bottom() + 0.5 && near(outer.bottom(), clip.bottom()))
                    || (rect.top() < clip.top() - 0.5 && near(outer.top(), clip.top())))
        })
    };
    harness
        .clipped_text
        .iter()
        .filter(|(text, _, _)| !text.trim().is_empty())
        .filter(|(_, _, clip)| clip.is_positive())
        .filter(|(_, rect, clip)| {
            let over = rect.width() > clip.width() + 0.5 || rect.height() > clip.height() + 0.5;
            over && !scrolled(*rect, *clip)
        })
        .map(|(text, rect, clip)| format!("{text:?} at {rect:?} in {clip:?}"))
        .collect()
}

fn rect_of(node: &Node) -> Option<Rect> {
    node.bounds().map(|bounds| {
        Rect::from_min_max(
            egui::pos2(bounds.x0 as f32, bounds.y0 as f32),
            egui::pos2(bounds.x1 as f32, bounds.y1 as f32),
        )
    })
}

/// Every control whose own words the last frame painted past its edges:
/// a button, a field, a menu or a tab whose label is drawn in it, and
/// does not end inside it.
fn overflowing(harness: &Harness, tree: &TreeUpdate) -> Vec<String> {
    use egui::accesskit::Role;
    let controls = [
        Role::Button,
        Role::CheckBox,
        Role::ComboBox,
        Role::RadioButton,
        Role::Switch,
        Role::Tab,
        Role::TextInput,
        Role::MenuItem,
    ];
    let mut found = Vec::new();
    for (_, node) in &tree.nodes {
        let (Some(label), Some(rect)) = (node.label(), rect_of(node)) else {
            continue;
        };
        if !controls.contains(&node.role()) || rect.width() < 1.0 || rect.height() < 1.0 {
            continue;
        }
        // The words it is called by, painted starting inside it: its own.
        for (text, at, clip) in &harness.clipped_text {
            let own = text == label && rect.contains(at.left_center() + egui::vec2(0.5, 0.0));
            // What its clip hides is a scroll's, told by `cut`, and what
            // is scrolled out of sight shows nothing.
            let shown = at.intersect(*clip);
            if own && shown.is_positive() && !holds(rect, shown) {
                found.push(format!("{label:?} drawn at {at:?} past {rect:?}"));
            }
        }
    }
    found
}

/// Neither kind of cut text in what the window shows now.
fn nothing_cut_here(harness: &mut Harness, what: &str) {
    let tree = harness.settle();
    let cut = cut(harness);
    assert!(cut.is_empty(), "{what}: cut off: {cut:#?}");
    let over = overflowing(harness, &tree);
    assert!(over.is_empty(), "{what}: past their controls: {over:#?}");
}

/// The wheel turned down by `delta` with the pointer at `at`, until it has
/// landed.
fn wheel_at(harness: &mut Harness, at: egui::Pos2, delta: f32) {
    harness.frame(vec![
        egui::Event::PointerMoved(at),
        egui::Event::MouseWheel {
            unit: egui::MouseWheelUnit::Point,
            delta: egui::vec2(0.0, -delta),
            modifiers: Modifiers::NONE,
            phase: egui::TouchPhase::Move,
        },
    ]);
    // Smooth scrolling ended, and a scroll asked for lands two frames on.
    harness.finish_animations();
    harness.frame(Vec::new());
    harness.frame(Vec::new());
}

/// Whether a piece of text the frames both painted moved between them:
/// whether the wheel scrolled anything.
fn moved(before: &[(String, Rect)], after: &[(String, Rect)]) -> bool {
    let before: std::collections::HashMap<&str, Rect> = before
        .iter()
        .rev()
        .map(|(text, rect)| (text.as_str(), *rect))
        .collect();
    let mut seen = std::collections::HashSet::new();
    after.iter().any(|(text, rect)| {
        seen.insert(text.as_str()) && before.get(text.as_str()).is_some_and(|was| was != rect)
    })
}

/// Nothing cut in what the window shows, scrolled down from the top to
/// the end of what is under `at` (a part of the window from 0 to 1 each
/// way), 40% of the window at a time: less than the smallest
/// pane that scrolls shows at 250% (the Settings window's, 195 of the 432
/// points), so a line cut at the bottom of one view is whole in the next. The terminal look's screens scroll by their
/// cursor, not the wheel, and paint every row: the first view has them.
fn nothing_cut(harness: &mut Harness, what: &str, at: Vec2) {
    if harness.app.look.terminal {
        return nothing_cut_here(harness, what);
    }
    let at = (harness.size * at).to_pos2();
    for step in 0..40 {
        nothing_cut_here(harness, &format!("{what}, scrolled {step} times"));
        let before = harness.text_rects.clone();
        let by = harness.size.y * 0.4;
        wheel_at(harness, at, by);
        if !moved(&before, &harness.text_rects) {
            return;
        }
    }
    panic!("{what}: the wheel never came to the end");
}

#[test]
fn the_largest_window_is_zoomed_and_its_points_are_fewer() {
    let harness = largest(Look::macos());
    assert_eq!(harness.ctx.zoom_factor(), 2.5);
    assert_eq!(harness.size, egui::vec2(768.0, 432.0));
    let harness = largest(Look::omarchy());
    assert_eq!(harness.ctx.zoom_factor(), 32.0 / 13.0);
}

#[test]
fn the_table_view_and_its_sidebar_cut_nothing_off_at_the_largest() {
    for look in looks() {
        let mut harness = largest(look);
        harness.book_covers();
        zoomed(&mut harness);
        nothing_cut(&mut harness, look.name, egui::vec2(0.6, 0.6));
    }
}

/// Every pane of the Settings window in `look` at its largest, from top
/// to bottom.
fn settings_cut_nothing(look: Look) {
    let mut harness = largest(look);
    harness.app.apply(Action::ShowSettings);
    for pane in [
        SettingsPane::General,
        SettingsPane::Appearance,
        SettingsPane::Editor,
        SettingsPane::Keyboard,
        SettingsPane::Agents,
        SettingsPane::Updates,
    ] {
        harness.app.apply(Action::ShowSettingsPane(pane));
        let what = format!("{} {pane:?}", look.name);
        nothing_cut(&mut harness, &what, egui::vec2(0.6, 0.5));
    }
}

#[test]
fn every_mac_settings_pane_cuts_nothing_off_at_250_percent() {
    settings_cut_nothing(Look::macos());
}

#[test]
fn every_windows_settings_pane_cuts_nothing_off_at_250_percent() {
    settings_cut_nothing(Look::standard());
}

#[test]
fn every_terminal_settings_pane_cuts_nothing_off_at_32() {
    settings_cut_nothing(Look::omarchy());
}

/// Where the node that has the keyboard is, as a screen reader has it.
fn focused(tree: &TreeUpdate) -> Option<(String, Rect)> {
    let id = tree.focus;
    let (_, node) = tree.nodes.iter().find(|(node, _)| *node == id)?;
    Some((node.label().unwrap_or_default().to_owned(), rect_of(node)?))
}

#[test]
fn tab_reaches_every_control_of_the_settings_in_sight_at_250_percent() {
    for look in [Look::macos(), Look::standard()] {
        let mut harness = largest(look);
        harness.app.apply(Action::ShowSettings);
        harness
            .app
            .apply(Action::ShowSettingsPane(SettingsPane::Appearance));
        harness.settle();
        let screen = Rect::from_min_size(egui::Pos2::ZERO, harness.size);
        let mut reached = Vec::new();
        for _ in 0..30 {
            harness.press(Key::Tab, Modifiers::NONE);
            // A control scrolled to lands two frames on.
            harness.settle();
            let tree = harness.settle();
            let Some((name, rect)) = focused(&tree) else {
                continue;
            };
            assert!(
                holds(screen, rect),
                "{}: {name:?} has the keyboard at {rect:?}, out of {screen:?}",
                look.name
            );
            reached.push(name);
        }
        // The pane's last controls too, below the first view.
        for name in ["Interface scale", "Data font size", "Table list"] {
            assert!(
                reached.iter().any(|reached| reached.starts_with(name)),
                "{}: {name} never had the keyboard: {reached:?}",
                look.name
            );
        }
    }
}

#[test]
fn the_selected_row_stays_in_sight_when_the_scale_changes() {
    for look in [Look::macos(), Look::standard()] {
        let mut harness = Harness::with_size(WINDOW);
        harness.set_look(look);
        let tab = harness.connect_fake_as(false);
        harness.app.apply(Action::OpenObject {
            tab,
            object: tabletist_db::ObjectRef::new("main", "book_covers"),
            kind: tabletist_db::ObjectKind::Table,
            pin: true,
        });
        harness.answer_structure(crate::testing::book_covers_structure());
        harness.answer_rows(crate::testing::book_covers_page(40));
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        harness.settle();
        // From the first row's cell to the twentieth's.
        harness.press(Key::ArrowDown, Modifiers::NONE);
        for _ in 0..19 {
            harness.press(Key::ArrowDown, Modifiers::NONE);
        }
        let selected = harness.app.workspace(tab).unwrap().active_object_tab();
        assert_eq!(
            selected
                .and_then(|object| object.selection)
                .map(|cell| cell.row),
            Some(19)
        );
        harness.settle();
        let row = |harness: &mut Harness| {
            let tree = harness.settle();
            crate::testing::bounds(&tree, "Row 20", egui::accesskit::Role::Button)
        };
        assert!(
            row(&mut harness).is_some(),
            "{}: row 20 in sight",
            look.name
        );
        let before = focused(&harness.settle());
        // Up to 250%: the keyboard stays on its row.
        for _ in 0..8 {
            harness.press(Key::Equals, Modifiers::COMMAND);
        }
        zoomed(&mut harness);
        let screen = Rect::from_min_size(egui::Pos2::ZERO, harness.size);
        // The grid scrolls to it smoothly.
        harness.finish_animations();
        let at = row(&mut harness).unwrap_or_else(|| panic!("{}: row 20 gone", look.name));
        // Its height in sight: the row runs on past the grid's right.
        let rows = screen.y_range().expand(0.5);
        assert!(
            rows.contains(at.top()) && rows.contains(at.bottom()),
            "{}: row 20 at {at:?} out of {screen:?}",
            look.name
        );
        assert_eq!(
            focused(&harness.settle()).map(|(name, _)| name),
            before.map(|(name, _)| name)
        );
    }
}

#[test]
fn egui_rounds_what_it_paints_to_whole_pixels() {
    // So a line keeps to whole pixels at 110%, 125% and 175%, where a
    // point is not a whole number of them.
    let harness = largest(Look::macos());
    harness.ctx.tessellation_options(|options| {
        assert!(options.round_text_to_pixels);
        assert!(options.round_rects_to_pixels);
        assert!(options.round_line_segments_to_pixels);
    });
}

#[test]
fn the_connection_dialog_cuts_nothing_off_at_the_largest() {
    for look in looks() {
        let mut harness = largest(look);
        harness.app.apply(Action::NewConnection);
        nothing_cut(&mut harness, look.name, egui::vec2(0.5, 0.5));
    }
}

#[test]
fn the_export_sheet_cuts_nothing_off_at_the_largest() {
    for look in looks() {
        let mut harness = largest(look);
        let (tab, _) = harness.book_covers();
        harness.app.workspace_mut(tab).unwrap().pane = crate::model::Pane::Grid;
        zoomed(&mut harness);
        match look.terminal {
            true => super::tests::type_key(&mut harness, Key::E, "E"),
            false => harness.press(Key::E, Modifiers::COMMAND | Modifiers::SHIFT),
        }
        assert!(
            matches!(harness.app.dialog, Some(crate::model::Dialog::Export(_))),
            "{}",
            look.name
        );
        nothing_cut(&mut harness, look.name, egui::vec2(0.5, 0.5));
    }
}
