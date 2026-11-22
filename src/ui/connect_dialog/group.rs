//! The connection's Group: a dropdown of the groups it can be put in, the
//! Connections list's, with New group… under them, which turns it into a
//! field where a new one is named. Escape there goes back to the dropdown.

use egui::{Response, Ui, WidgetInfo, WidgetType, vec2};

use super::{Skin, input};
use crate::connections::groups;
use crate::i18n::msgid;
use crate::model::ConnectionForm;
use crate::typography::TextRole;
use crate::ui::menu;

/// Draws Group `width` wide, as tall as Name beside or above it. `name`
/// is what screen readers call it; the field the caller labels.
pub(super) fn control(
    ui: &mut Ui,
    form: &mut ConnectionForm,
    name: &str,
    width: f32,
    skin: &Skin,
) -> Response {
    let Skin { look, palette, .. } = *skin;
    let focus = std::mem::take(&mut form.focus_group);
    // As Name is: 34 on macOS, the terminal's fields' 30.
    let role = TextRole::pick(look, TextRole::UiBody, TextRole::OBody);
    let height = if look.terminal {
        skin.field_height()
    } else {
        34.0
    };
    if let Some(typed) = form.group_typed.as_mut() {
        let placeholder = super::hint(ui, &skin.say(msgid("Group name")), role, skin);
        let field = ui.add_sized(
            vec2(width, height),
            input(ui, typed, role, height, skin)
                .id(ui.id().with("group-typed"))
                .hint_text(placeholder),
        );
        if focus {
            field.request_focus();
        }
        if super::agents::cancel_asked(ui, &field, skin) {
            field.surrender_focus();
            form.group_typed = None;
            form.focus_group = true;
        }
        return field;
    }
    let choices = &form.group_choices;
    let shown = choices.shown(form.group.as_deref()).to_owned();
    let titles = choices.titles.clone();
    let automatic = choices.automatic.clone();
    let mut chosen = shown.clone();
    let mut new = false;
    let dropdown = menu::Dropdown::new("group", &shown, width)
        .height(height)
        .fill(ui.visuals().extreme_bg_color);
    let response = dropdown
        .show(ui, look, palette, |ui| {
            for title in &titles {
                menu::value(ui, &mut chosen, title.clone(), title, look, palette);
            }
            menu::separator(ui, palette);
            let more = skin.say(msgid("New group…"));
            if menu::Item::action(&more).show(ui, look, palette).clicked() {
                new = true;
                ui.close();
            }
        })
        .response;
    response.widget_info(|| {
        let mut info = WidgetInfo::labeled(WidgetType::ComboBox, true, name);
        info.current_text_value = Some(shown.clone());
        info
    });
    if focus {
        response.request_focus();
    }
    if new {
        form.group_typed = Some(String::new());
        form.focus_group = true;
    } else if chosen != shown {
        // The rule's own group is no group of the connection's: it then
        // follows the rule.
        form.group = (!groups::same(&chosen, &automatic)).then_some(chosen);
    }
    response
}
