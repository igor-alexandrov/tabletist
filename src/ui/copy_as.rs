//! Copy as: a grid's selection, or the row panel's row, on the clipboard
//! in a format. ⌥⌘C and `gy` open its menu at the cursor's cell, one
//! letter a format; a secondary click on a cell opens the grid's menu,
//! whose Copy as › holds the same rows. The copy itself is the app's
//! (`Action::CopyAs`): this draws and reads the keys.

use egui::{Area, Id, Order, Rect, Ui, pos2, vec2};

use crate::i18n::gettext;
use crate::keymap::{Command, Layout, Scope, When};
use crate::model::{Action, CopyAsMenu, CopyHow, CopyPick};
use crate::typography::{Text, TextRole};
use crate::ui::{Paint, menu, widgets};

/// What a row of the grid's menu asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Chosen {
    /// Copy: as ⌘C.
    Copy,
    /// Copy rows as TSV: as ⇧⌘C.
    CopyRows,
    /// A row of Copy as ›.
    As(CopyHow),
    /// Export selection…: a table's grid's, and a SQL result's (spec 4).
    Export,
}

impl Chosen {
    /// The copy it asks for, of the grid in front in `tab`.
    pub fn action(self, tab: crate::model::ConnTabId) -> Action {
        let (pick, how) = match self {
            Self::Copy => (CopyPick::Selection, CopyHow::Plain),
            Self::CopyRows => (CopyPick::Rows, CopyHow::Plain),
            Self::As(how) => (CopyPick::Selection, how),
            Self::Export => {
                return Action::ExportSelection(crate::model::ExportOffer {
                    tab,
                    table: None,
                    format: None,
                    whole_rows: false,
                    copied: None,
                    row_only: false,
                });
            }
        };
        Action::CopyAs { tab, pick, how }
    }
}

fn anchor_id() -> Id {
    Id::new("copy-as-anchor")
}

/// Keeps where the cursor's cell is drawn: the menu opens under it.
pub fn keep_anchor(ctx: &egui::Context, cell: Rect) {
    ctx.data_mut(|data| data.insert_temp(anchor_id(), cell));
}

/// A row's name: the format's ("Markdown"), or "Column names". On the row
/// panel each reads "Copy row as …".
pub fn row_name(how: CopyHow, row: bool, locale: crate::i18n::Locale) -> String {
    let name = match how {
        CopyHow::As(format) => format.name().to_owned(),
        CopyHow::Plain => crate::export::Format::Tsv.name().to_owned(),
        CopyHow::ColumnNames => gettext(locale, "Column names").into_owned(),
    };
    if row {
        format!("{} {name}", gettext(locale, "Copy row as"))
    } else {
        name
    }
}

/// Reads the menu's keys, before anything else in the frame reads them: a
/// field under it would take Enter. Returns what they ask for.
pub fn keys(ctx: &egui::Context, paint: Paint, open: &CopyAsMenu) -> Vec<Action> {
    keep_keyboard(ctx);
    let (tab, pick) = (open.tab, open.pick);
    let rows = CopyHow::menu(pick);
    let lit = open.lit.min(rows.len() - 1);
    let copy = |how: CopyHow| vec![Action::CloseDialog, Action::CopyAs { tab, pick, how }];
    let (keymap, layout) = (crate::ui::keys::published(ctx), Layout::of(&paint.look));
    // The letters, as typed text, by the keymap.
    let typed = ctx.input_mut(|input| {
        let mut typed = Vec::new();
        input.events.retain(|event| match event {
            egui::Event::Text(text) => {
                match crate::ui::keys::prompt_letter(&keymap, layout, When::CopyAs, text) {
                    Some(command) => {
                        typed.push(command);
                        false
                    }
                    None => true,
                }
            }
            _ => true,
        });
        typed
    });
    let picked = typed
        .iter()
        .find_map(|command| rows.iter().find(|(_, letter)| letter == command))
        .map(|(how, _)| *how);
    let moved = |command: Command| {
        ctx.input_mut(|input| {
            crate::ui::keys::asked(input, &keymap, layout, command, Scope::Prompt, true).count
        })
    };
    let step = moved(Command::CopyAsNext) as isize - moved(Command::CopyAsPrevious) as isize;
    let actions = if let Some(how) = picked {
        copy(how)
    } else if moved(Command::CopyAsPick) > 0 {
        copy(rows[lit].0)
    } else if moved(Command::CloseCopyAs) > 0 {
        vec![Action::CloseDialog]
    } else if step != 0 {
        vec![Action::MoveInCopyAs(step)]
    } else {
        Vec::new()
    };
    give_keyboard_back(ctx, &actions);
    actions
}

/// Draws the Copy as menu at the cursor's cell, and returns what a click
/// asks for. `size` is what it copies, in words ("4 rows × 2 columns"),
/// for the terminal look's title.
pub fn show(ctx: &egui::Context, paint: Paint, open: &CopyAsMenu, size: &str) -> Vec<Action> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let (tab, pick) = (open.tab, open.pick);
    let rows = CopyHow::menu(pick);
    let lit = open.lit.min(rows.len() - 1);
    // Over the row panel's row it is the row's.
    let name = if pick == CopyPick::CursorRow {
        "Copy row as"
    } else {
        "Copy as"
    };
    let mut actions = Vec::new();
    let anchor = ctx
        .data(|data| data.get_temp::<Rect>(anchor_id()))
        .unwrap_or_else(|| Rect::from_center_size(ctx.content_rect().center(), vec2(0.0, 0.0)));
    let area = Area::new(Id::new("copy-as"))
        .order(Order::Foreground)
        .kind(egui::UiKind::Menu)
        .fixed_pos(pos2(anchor.left(), anchor.bottom() + 4.0))
        .show(ctx, |ui| {
            menu::frame(&look, &palette).show(ui, |ui| {
                ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
                ui.set_min_width(if look.terminal { 300.0 } else { 222.0 });
                if look.terminal {
                    let title = look.label(&gettext(locale, name));
                    title_line(ui, paint, &title, size);
                }
                for (index, (how, letter)) in rows.iter().enumerate() {
                    let key = crate::ui::keys::written(ctx, &look, *letter);
                    let name = row_name(*how, false, locale);
                    let shown = if look.terminal {
                        format!("{key}  {}", look.label(&name))
                    } else {
                        name.clone()
                    };
                    let mut item = menu::Item::action(&shown)
                        .name(&name)
                        .kept_out(index == lit);
                    let upper = key.as_str().to_uppercase();
                    if !look.terminal {
                        item = item.detail(&upper);
                    }
                    if item.show(ui, &look, &palette).clicked() {
                        actions.push(Action::CloseDialog);
                        actions.push(Action::CopyAs {
                            tab,
                            pick,
                            how: *how,
                        });
                    }
                }
                if look.terminal {
                    foot_line(ui, paint);
                }
            });
            let place = ui.min_rect();
            widgets::announce(ui, place, &gettext(locale, name));
        });
    if area.response.clicked_elsewhere() && actions.is_empty() {
        actions.push(Action::CloseDialog);
    }
    give_keyboard_back(ctx, &actions);
    actions
}

/// Keeps where the keyboard was as the menu opened.
fn keep_keyboard(ctx: &egui::Context) {
    if ctx
        .data(|data| data.get_temp::<Option<Id>>(keyboard_id()))
        .is_none()
    {
        let focused = ctx.memory(|memory| memory.focused());
        ctx.data_mut(|data| data.insert_temp(keyboard_id(), focused));
    }
}

/// Gives the keyboard back where it was, as `actions` close the menu.
fn give_keyboard_back(ctx: &egui::Context, actions: &[Action]) {
    if actions
        .iter()
        .any(|action| matches!(action, Action::CloseDialog))
    {
        let back = ctx.data_mut(|data| data.remove_temp::<Option<Id>>(keyboard_id()));
        if let Some(Some(id)) = back {
            ctx.memory_mut(|memory| memory.request_focus(id));
        }
    }
}

/// Forgets where the keyboard was, with no menu open.
pub fn fold(ctx: &egui::Context) {
    ctx.data_mut(|data| data.remove_temp::<Option<Id>>(keyboard_id()));
}

fn keyboard_id() -> Id {
    Id::new("copy-as-keyboard")
}

/// The terminal look's title: "copy as", then what it copies.
fn title_line(ui: &mut Ui, paint: Paint, title: &str, size: &str) {
    let Paint { palette, look, .. } = paint;
    let role = TextRole::OBody;
    let height = role.row_height(ui.ctx(), look.faces) + 2.0;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    let y = rect.center().y;
    let width = widgets::paint_text(
        ui,
        rect.left() + 10.0,
        y,
        Text::one(&look, TextRole::OGroup, title, palette.text),
    );
    widgets::paint_text(
        ui,
        rect.left() + 10.0 + width + 10.0,
        y,
        Text::one(&look, role, size, palette.dim),
    );
    widgets::hline(ui, rect.x_range(), rect.bottom() - 0.5, palette.outline);
}

/// The terminal look's keys under the rows: "one key picks · ctrl+n/p
/// enter · esc close".
fn foot_line(ui: &mut Ui, paint: Paint) {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let ctx = ui.ctx().clone();
    let moves = crate::ui::keys::written_together(
        &ctx,
        &look,
        &[Command::CopyAsNext, Command::CopyAsPrevious],
    );
    let enter = crate::ui::keys::written(&ctx, &look, Command::CopyAsPick);
    let esc = crate::ui::keys::written(&ctx, &look, Command::CloseCopyAs);
    let picks = look.label(&gettext(locale, "one key picks"));
    let close = look.label(&gettext(locale, "close"));
    let role = widgets::secondary(&look);
    let height = TextRole::OBody.row_height(&ctx, look.faces) + 2.0;
    let (rect, _) =
        ui.allocate_exact_size(vec2(ui.available_width(), height), egui::Sense::hover());
    widgets::hline(ui, rect.x_range(), rect.top() + 0.5, palette.outline);
    let y = rect.center().y;
    let x = rect.left() + 10.0;
    let width = widgets::paint_text(ui, x, y, Text::one(&look, role, &picks, palette.dim));
    let hints: [widgets::Hint<'_>; 2] = [(&moves, enter.as_str(), true), (&esc, &close, true)];
    widgets::key_hints(ui, (x + width + 14.0, y), &hints, 14.0, &look, &palette);
}

/// The grid's menu: Copy, Copy as › (its rows, with their letters), Copy
/// rows as TSV, and (`export`) Export selection…, in a table's grid and a
/// SQL result's (spec 4). Returns what was picked.
pub fn context_rows(ui: &mut Ui, paint: Paint, export: bool) -> Option<Chosen> {
    let Paint {
        locale,
        palette,
        look,
    } = paint;
    let ctx = ui.ctx().clone();
    let mut chosen = None;
    let label = |text: &'static str| look.label(&gettext(locale, text));
    let copy_key = crate::ui::keys::written_in(&ctx, &look, Command::CopyCells, Scope::Grid);
    let copy = label("Copy");
    let mut item = menu::Item::action(&copy);
    if !look.terminal {
        item = item.detail(copy_key.as_str());
    }
    if item.show(ui, &look, &palette).clicked() {
        chosen = Some(Chosen::Copy);
    }
    let copy_as = label("Copy as");
    let button = menu::Item::action(&copy_as)
        .detail("›")
        .show(ui, &look, &palette);
    egui::containers::menu::SubMenu::new().show(ui, &button, |ui| {
        ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
        ui.set_min_width(180.0);
        for (how, letter) in CopyHow::MENU {
            let key = crate::ui::keys::written(&ctx, &look, letter);
            // As the look writes a key: `C` on macOS, `c` in the terminal.
            let letter = if look.terminal {
                key.to_string()
            } else {
                key.as_str().to_uppercase()
            };
            let name = look.label(&row_name(how, false, locale));
            let item = menu::Item::action(&name).detail(&letter);
            if item.show(ui, &look, &palette).clicked() {
                chosen = Some(Chosen::As(how));
            }
        }
    });
    let rows = label("Copy rows as TSV");
    let rows_key = crate::ui::keys::written_in(&ctx, &look, Command::CopyRow, Scope::Grid);
    let mut item = menu::Item::action(&rows);
    if !rows_key.as_str().is_empty() {
        item = item.detail(rows_key.as_str());
    }
    if item.show(ui, &look, &palette).clicked() {
        chosen = Some(Chosen::CopyRows);
    }
    if export {
        menu::separator(ui, &palette);
        let words = label("Export selection…");
        let key = crate::ui::keys::written_in(&ctx, &look, Command::Export, Scope::Grid);
        let mut item = menu::Item::action(&words);
        if !look.terminal && !key.as_str().is_empty() {
            item = item.detail(key.as_str());
        }
        if item.show(ui, &look, &palette).clicked() {
            chosen = Some(Chosen::Export);
        }
    }
    chosen
}
