//! The terminal look's status line names the keys of the pane that has
//! the keyboard: the tree's, its filter field's, the row panel's. The
//! grid's, a SQL editor's and the views' sets are drawn by the line itself,
//! with the pane keys added where they move.

use crate::i18n::{Locale, gettext};
use crate::keymap::{Command, Keymap, Layout, Scope, Written};
use crate::model::{ConnTabId, Pane, Workspace};
use crate::ui::focus::{self, Region};
use crate::ui::widgets::Hint;

/// A hint the line draws: the key, and what it does.
pub type Owned = (Written, String);

/// The pane that has the keyboard, as the status line reads it, in the
/// order `keys.rs` asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Held {
    /// The filter field above the tree.
    Filter,
    /// A field of the row panel of the table on screen.
    Panel,
    /// The tree.
    Tree,
    /// Anything else: the view in front says.
    View,
}

impl Held {
    pub fn of(ctx: &egui::Context, tab: ConnTabId, workspace: &Workspace) -> Self {
        if crate::ui::sidebar::filter_focused(ctx, tab) {
            return Self::Filter;
        }
        let panel = workspace.active_tab.and_then(|id| {
            let object = workspace.object_tab(id)?;
            let columns = object.rows.value.as_ref()?.columns.len();
            crate::ui::row_panel::focused_field(ctx, tab, id, columns)
        });
        if panel.is_some() {
            return Self::Panel;
        }
        if workspace.pane == Pane::Tree && !ctx.text_edit_focused() {
            return Self::Tree;
        }
        Self::View
    }
}

/// What the pane keys' hint says they do.
const PANES: &str = "panes";

/// `ctrl+h/l panes`, where those keys would move the keyboard from `from`.
pub fn panes(ctx: &egui::Context, keymap: &Keymap, layout: Layout, from: Region) -> Option<Owned> {
    focus::can_step(ctx, &Region::PANES, from).then(|| {
        let keys = keymap.together(layout, &[Command::PaneLeft, Command::PaneRight]);
        (keys, PANES.to_owned())
    })
}

/// What of `hints` the line has room for, where `fits` says: all of them,
/// or without the last (the export), then without the pane keys too,
/// wherever they stand, then fewer from the end.
pub fn giving_way<'a>(hints: &[Hint<'a>], fits: impl Fn(&[Hint<'a>]) -> bool) -> Vec<Hint<'a>> {
    if fits(hints) {
        return hints.to_vec();
    }
    let mut shown = hints[..hints.len().saturating_sub(1)].to_vec();
    if fits(&shown) {
        return shown;
    }
    shown.retain(|(_, word, _)| *word != PANES);
    while !shown.is_empty() && !fits(&shown) {
        shown.pop();
    }
    shown
}

/// What is pending in the table on screen, which the prompt saves and
/// reviews from the tree too.
#[derive(Clone, Copy, Debug, Default)]
pub struct Pending {
    /// `:w`: something is pending, on a connection that writes.
    pub write: bool,
    /// `:diff`: something is pending.
    pub review: bool,
}

/// The tree's: j/k move · enter open · / filter · t tree · za fold ·
/// ctrl+h/l panes · (:w write · :diff review, while something is pending)
/// · E export table. Export gives way first, then the pending keys.
pub fn tree(
    ctx: &egui::Context,
    keymap: &Keymap,
    layout: Layout,
    locale: Locale,
    pending: Pending,
) -> Vec<Owned> {
    let tree = |command| keymap.key_in(layout, command, Scope::Sidebar);
    let word = |word| gettext(locale, word).into_owned();
    let mut hints = vec![
        // The pair as the keymap shows it (`j/k`), not its first key.
        (keymap.key(layout, Command::MoveInTree), word("move")),
        (tree(Command::OpenObject), word("open")),
        (tree(Command::FilterObjects), word("filter")),
        (tree(Command::ToggleTree), word("tree")),
        (tree(Command::FoldGroup), word("fold")),
    ];
    hints.extend(panes(ctx, keymap, layout, Region::Tree));
    if pending.write {
        hints.push((keymap.key(layout, Command::SaveChanges), word("write")));
    }
    if pending.review {
        hints.push((keymap.key(layout, Command::ReviewSql), word("review")));
    }
    hints.push((tree(Command::Export), word("export table")));
    hints.retain(|(key, _)| !key.is_empty());
    hints
}

/// The filter field's: its Enter and Esc both leave it for the tree.
pub fn filter(locale: Locale) -> Vec<Owned> {
    vec![(
        Written::plain("enter/esc"),
        gettext(locale, "tree").into_owned(),
    )]
}

/// The row panel's, while one of its fields has the keyboard:
/// j/k field · [ ] row · i edit · gy copy as · esc grid · ctrl+h/l panes.
/// The edit key only where the field can be edited.
pub fn panel(
    ctx: &egui::Context,
    keymap: &Keymap,
    layout: Layout,
    locale: Locale,
    editable: bool,
) -> Vec<Owned> {
    let panel = |command| keymap.key_in(layout, command, Scope::Inspector);
    let word = |word| gettext(locale, word).into_owned();
    let rows = keymap.together(layout, &[Command::PreviousRow, Command::NextRow]);
    let mut hints = vec![
        (keymap.key(layout, Command::MoveField), word("field")),
        (rows, word("row")),
    ];
    if editable {
        // `i`, as the panel's header and the grid's line write it.
        hints.push((keymap.key(layout, Command::EditCell), word("edit")));
    }
    hints.extend([
        (panel(Command::CopyRowAs), word("copy as")),
        (
            keymap.own_key_in(layout, Command::BackToGrid, Scope::Inspector),
            word("grid"),
        ),
    ]);
    hints.extend(panes(ctx, keymap, layout, Region::Panel));
    hints.retain(|(key, _)| !key.is_empty());
    hints
}

/// The line's hints of `owned`, to draw.
pub fn drawn(owned: &[Owned]) -> Vec<Hint<'_>> {
    owned
        .iter()
        .map(|(key, word)| (key, word.as_str(), true))
        .collect()
}
