//! Settings › Editor's Format section and indent, in both looks: what
//! Format writes, what Tab and Shift+Tab put in and take out, the case
//! completion offers keywords in, and the rows that set them.

use crate::keymap::{Command as Keyed, Scope};
use crate::model::{Action, SettingsPane};
use crate::settings::{Commas, Indent, KeywordCase};
use crate::testing::Harness;
use crate::theme::Look;

use super::appearance_settings_tests::{settings_saves, shows};

/// Settings on its Editor pane.
fn on_the_pane(harness: &mut Harness) {
    harness.app.apply(Action::ShowSettings);
    harness
        .app
        .apply(Action::ShowSettingsPane(SettingsPane::Editor));
    harness.settle();
}

#[test]
fn the_sheet_has_the_indent_and_the_format_section_and_sets_them() {
    let look = Look::macos();
    let mut harness = Harness::with_size(egui::vec2(1440.0, 1400.0));
    harness.set_look(look);
    on_the_pane(&mut harness);
    // The heading names the Format key, as the keymap writes it.
    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
    let heading = format!("FORMAT · {}", key.as_str()).to_uppercase();
    for words in [heading.as_str(), "Indent", "Keywords", "Commas"] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // Indent leads the Typing section, as the board has it.
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    assert!(top(&harness, "TYPING") < top(&harness, "Indent"));
    assert!(top(&harness, "Indent") < top(&harness, "Autocomplete"));
    assert!(top(&harness, "Autocomplete") < top(&harness, "Keywords"));
    let saves = settings_saves(&harness);
    harness.click("Indent: Tab");
    harness.click("Keywords: lowercase");
    harness.click("Commas: Leading");
    harness.settle();
    let settings = &harness.app.settings;
    assert_eq!(settings.indent, Indent::Tab);
    assert_eq!(settings.keywords, Some(KeywordCase::Lower));
    assert_eq!(settings.commas, Commas::Leading);
    assert_eq!(settings_saves(&harness), saves + 3);
    // UPPERCASE is the look's own: picked while lower is set, it is set.
    harness.click("Keywords: UPPERCASE");
    assert_eq!(harness.app.settings.keywords, Some(KeywordCase::Upper));
}

#[test]
fn the_screen_has_the_indent_and_the_format_section() {
    let look = Look::omarchy();
    let mut harness = Harness::with_size(egui::vec2(1920.0, 1080.0));
    harness.set_look(look);
    on_the_pane(&mut harness);
    let key = crate::ui::keys::written_in(&harness.ctx, &look, Keyed::Format, Scope::SqlEditor);
    let heading = format!("format · {} in normal mode", key.as_str());
    for words in [heading.as_str(), "indent", "2 spaces", "keywords", "commas"] {
        assert!(shows(&mut harness, words), "{words}");
    }
    // As the board orders them: line numbers, indent, autocomplete.
    let top = |harness: &Harness, words: &str| harness.painted_rect(words).expect(words).top();
    assert!(top(&harness, "line numbers") < top(&harness, "indent"));
    assert!(top(&harness, "indent") < top(&harness, "autocomplete"));
    // The case is the choice: UPPER as it is, lower chosen.
    assert!(shows(&mut harness, "UPPER"));
    let tree = harness.settle();
    let chosen = tree.nodes.iter().any(|(_, node)| {
        node.label() == Some("Keywords: lower")
            && node.toggled() == Some(egui::accesskit::Toggled::True)
    });
    assert!(chosen, "the look's lower case is the one chosen");
}
