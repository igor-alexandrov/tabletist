//! The person's keys, laid over the keymap's: what `settings.toml` says
//! under `[keys.<scope>]` (`"cmd+shift+f" = "format"`) and `[keys.unbind]`
//! (`grid = ["gd"]`). The keymap stays the one list: these are changes to
//! it, and [`Keymap::with`] makes the keymap in force from them, saying
//! which lines it could not take and why. The Keyboard settings ask the
//! same function whether a key can be recorded, so the file and the page
//! cannot judge a key differently.
//!
//! A line gives a row ([`Row`]) its keys of one kind ([`Press`]): a chord
//! recorded for `Save changes` on Omarchy leaves its `:w` as it is. An
//! unbind takes a default key off the rows of its scope.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

use super::{
    BINDINGS, Binding, Bound, Chord, Command, Fixed, Keymap, Kind, Layout, Mode, ONE_LETTER, Owner,
    Press, Row, Scope, Spelled, Stroke, VIM_INSERT, When, clash, defaults, entries, fixed,
    reserved_by, same, spell, takes,
};

/// A spelling made while the app runs, kept for as long as it runs: the
/// keymap's spellings are `&'static`, and a key read from the file or
/// recorded joins them. Each is kept once, so a file read again and again
/// holds no more than it names.
pub fn intern(text: &str) -> &'static str {
    static KEPT: OnceLock<Mutex<HashSet<&'static str>>> = OnceLock::new();
    let mut kept = KEPT
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = kept.get(text) {
        return found;
    }
    let leaked: &'static str = Box::leak(text.to_owned().into_boxed_str());
    kept.insert(leaked);
    leaked
}

/// A line's keys as the keymap holds them, kept once as [`intern`] keeps a
/// spelling.
fn intern_chords(chords: Vec<Chord>) -> &'static [Chord] {
    static KEPT: OnceLock<Mutex<HashSet<&'static [Chord]>>> = OnceLock::new();
    let mut kept = KEPT
        .get_or_init(Mutex::default)
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(found) = kept.get(chords.as_slice()) {
        return found;
    }
    let leaked: &'static [Chord] = Box::leak(chords.into_boxed_slice());
    kept.insert(leaked);
    leaked
}

/// The person's changes to the keymap, in the order the file has them.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Overrides {
    pub binds: Vec<Bind>,
    pub unbinds: Vec<Unbind>,
}

/// A line of `[keys.<table>]`: a row's keys of one kind.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bind {
    pub scope: Scope,
    /// The table it is written in: as the file names it, or as the look
    /// names the scope where the app adds it.
    pub table: &'static str,
    pub keys: &'static str,
    pub command: Command,
    /// What the file says beside it (`after swap · was set_default`).
    pub note: Option<String>,
}

/// A key of `[keys.unbind]`: a default key that does nothing in its scope.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unbind {
    pub scope: Scope,
    pub table: &'static str,
    pub keys: &'static str,
}

/// A line of the overrides, by its place among its kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Line {
    Bind(usize),
    Unbind(usize),
}

/// A line the keymap did not take, and why: the default stays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Refused {
    pub line: Line,
    pub why: Why,
}

/// What Omarchy never binds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Never {
    /// Super is Hyprland's.
    Super,
    /// The colon opens the prompt.
    Colon,
    /// Esc gives up what is under way.
    Escape,
}

/// Why a key cannot be a row's.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Why {
    /// The command has no line in that scope.
    NoRow,
    /// The row's keys cannot be recorded.
    Fixed(Fixed),
    /// The handler reads the row's command by keys of other kinds.
    Kind(Vec<Press>),
    /// On macOS and Windows a key needs Cmd, Ctrl or Alt, or is a
    /// function key.
    Alone,
    /// The system keeps the key.
    Reserved(Owner),
    /// Omarchy never binds it.
    Never(Never),
    /// Vim's own chord where text is typed.
    Vim,
    /// Typed keys that start with a digit, which is a count.
    Count,
    /// The command is read by one typed letter.
    OneLetter,
    /// Another row has the key where both are read.
    Taken(Row),
    /// A row whose keys cannot be recorded has the key: Tabletist's.
    Tabletist(Command),
    /// An unbind of a key no row of its scope has by default.
    NotBound,
}

impl Why {
    /// What is said of the refusal, in English, with `{keys}` for the key,
    /// `{name}` for the row's command (or the other row's, for a key that
    /// is taken) and `{scope}` for its scope: the caller translates it and
    /// fills them in.
    pub fn text(&self, layout: Layout) -> &'static str {
        let omarchy = layout == Layout::Omarchy;
        match self {
            Self::NoRow => "{name} has no keys in {scope}",
            Self::Fixed(_) => "{name} keeps its keys",
            Self::Kind(takes) => match takes.as_slice() {
                [Press::Typed] => "{name} takes typed keys",
                [Press::Line] => "{name} takes a : line",
                [Press::Chord] if omarchy => "{name} takes a chord",
                [Press::Chord] if layout == Layout::Windows => "Add Ctrl or Alt",
                [Press::Chord] => "Add ⌘, ⌃ or ⌥",
                _ => "{name} takes other keys",
            },
            Self::Alone => match layout {
                Layout::Mac => "Add ⌘, ⌃ or ⌥",
                Layout::Windows | Layout::Omarchy => "Add Ctrl or Alt",
            },
            Self::Reserved(Owner::MacOs) => "{keys} is used by macOS",
            Self::Reserved(Owner::Windows) => "{keys} is used by Windows",
            Self::Never(Never::Super) => "super is never bound: hyprland owns it",
            Self::Never(Never::Colon) => ": opens the prompt and is never bound",
            Self::Never(Never::Escape) => "esc cancels and is never bound",
            Self::Vim => "{keys} is vim's in insert mode",
            Self::Count => "a digit first is a count",
            Self::OneLetter => "{name} takes one letter",
            Self::Taken(_) if omarchy => "{keys} is {name}",
            Self::Taken(_) => "{keys} is already “{name}”.",
            Self::Tabletist(_) => "{keys} is used by Tabletist",
            Self::NotBound => "{keys} is no default key in {scope}",
        }
    }
}

/// How a spelling is read, if it reads.
fn press_of(keys: &str) -> Option<Press> {
    let keys = intern(keys);
    spell(keys).ok().and_then(|spelled| Press::of(&spelled))
}

/// Whether two spellings are the same keys in `layout`.
fn same_keys(layout: Layout, a: &str, b: &str) -> bool {
    match (spell(intern(a)), spell(intern(b))) {
        (Ok(a), Ok(b)) => same(layout, &a, &b),
        _ => a == b,
    }
}

/// What `layout` says of `spelled` as a key of `row`, before anything else
/// has it: a key the look never gives a command, or one the row's command
/// would not be read by. None where it can be the row's.
pub fn refuse_key(layout: Layout, row: Row, spelled: &Spelled) -> Option<Why> {
    let Some(press) = Press::of(spelled) else {
        return Some(Why::Kind(takes(layout, row)));
    };
    let strokes: &[Stroke] = match spelled {
        Spelled::Strokes(strokes) => strokes,
        Spelled::Digits(_) | Spelled::Ex(_) => &[],
    };
    match layout {
        Layout::Mac | Layout::Windows => {
            if press == Press::Chord && !super::alone_will_do(spelled) {
                return Some(Why::Alone);
            }
            if let Some(owner) = reserved_by(layout, spelled) {
                return Some(Why::Reserved(owner));
            }
        }
        Layout::Omarchy => {
            if let [Stroke::Key { mods, key }] = strokes {
                if mods.cmd {
                    return Some(Why::Never(Never::Super));
                }
                if mods.none() && *key == egui::Key::Escape {
                    return Some(Why::Never(Never::Escape));
                }
            }
            if press == Press::Typed {
                if matches!(strokes.first(), Some(Stroke::Text(char)) if char.is_ascii_digit()) {
                    return Some(Why::Count);
                }
                if strokes.len() > 1 && ONE_LETTER.contains(&row.command) {
                    return Some(Why::OneLetter);
                }
            }
            let vims = VIM_INSERT
                .iter()
                .filter_map(|keys| spell(keys).ok())
                .any(|vim| same(layout, &vim, spelled));
            let typed_in = BINDINGS
                .iter()
                .filter(|binding| Row::of(binding) == row)
                .any(|binding| {
                    binding.scopes.iter().any(|scope| {
                        *scope != Scope::Prompt
                            && binding.mode(*scope, layout, spelled).overlaps(Mode::Insert)
                    })
                });
            if vims && typed_in {
                return Some(Why::Vim);
            }
        }
    }
    let takes = takes(layout, row);
    if !takes.contains(&press) {
        return Some(Why::Kind(takes));
    }
    None
}

/// Whether `spelled` is one of `row`'s keys by default.
fn own_default(layout: Layout, row: Row, spelled: &Spelled) -> bool {
    defaults(layout, row)
        .iter()
        .filter_map(|keys| spell(keys).ok())
        .any(|keys| same(layout, &keys, spelled))
}

impl Binding {
    fn bound_mut(&mut self, layout: Layout) -> &mut Bound {
        match layout {
            Layout::Mac => &mut self.mac,
            Layout::Windows => match &mut self.windows {
                Some(windows) => windows,
                None => &mut self.mac,
            },
            Layout::Omarchy => &mut self.omarchy,
        }
    }
}

/// A bind the keymap took: its place among the binds, its row, its keys.
struct Taken {
    index: usize,
    row: Row,
    keys: &'static str,
    spelled: Spelled,
    press: Press,
}

impl Keymap {
    /// The keymap in force: the defaults with `overrides` laid over them,
    /// in `layout`, and the lines it did not take. Every line is judged
    /// alone first (its row, its key), then all that pass are laid
    /// together and judged against one another and against the defaults:
    /// a key another row has where both are read is refused, the later
    /// line of two giving way, until none is left. So the two lines of a
    /// swap stand together, and a line that takes a default key from
    /// another row does not.
    pub fn with(layout: Layout, overrides: &Overrides) -> (Keymap, Vec<Refused>) {
        if overrides.binds.is_empty() && overrides.unbinds.is_empty() {
            return (Keymap::default(), Vec::new());
        }
        let defaults_map = Keymap::default();
        let rows = defaults_map.rows(layout);
        let mut refused = Vec::new();
        let mut unbinds: Vec<(Scope, Spelled)> = Vec::new();
        for (index, unbind) in overrides.unbinds.iter().enumerate() {
            let why = match spell(unbind.keys) {
                Ok(spelled) => {
                    let mut owners = rows.iter().filter(|row| {
                        row.scope == unbind.scope
                            && row.when == When::Always
                            && defaults(layout, **row).iter().any(|keys| {
                                spell(keys).is_ok_and(|default| same(layout, &default, &spelled))
                            })
                    });
                    match owners.next() {
                        Some(row) => match fixed(layout, *row) {
                            Some(fixed) => Some(Why::Fixed(fixed)),
                            None => {
                                unbinds.push((unbind.scope, spelled));
                                None
                            }
                        },
                        None => Some(Why::NotBound),
                    }
                }
                Err(_) => Some(Why::NotBound),
            };
            if let Some(why) = why {
                refused.push(Refused {
                    line: Line::Unbind(index),
                    why,
                });
            }
        }
        let mut taken: Vec<Taken> = Vec::new();
        for (index, bind) in overrides.binds.iter().enumerate() {
            let row = Row {
                command: bind.command,
                scope: bind.scope,
                when: When::Always,
            };
            let why = if !rows.contains(&row) {
                Some(Why::NoRow)
            } else if let Some(fixed) = fixed(layout, row) {
                Some(Why::Fixed(fixed))
            } else {
                match spell(bind.keys) {
                    Err(_) => Some(Why::Kind(takes(layout, row))),
                    // A key the row has by default is its own whatever the
                    // look says of new keys (Return edits a cell on macOS).
                    Ok(spelled) => {
                        let why = if own_default(layout, row, &spelled) {
                            None
                        } else {
                            refuse_key(layout, row, &spelled)
                        };
                        if why.is_none() {
                            let press = Press::of(&spelled).expect("a key of a kind");
                            taken.push(Taken {
                                index,
                                row,
                                keys: bind.keys,
                                spelled,
                                press,
                            });
                        }
                        why
                    }
                }
            };
            if let Some(why) = why {
                refused.push(Refused {
                    line: Line::Bind(index),
                    why,
                });
            }
        }
        loop {
            let keymap = lay(layout, &unbinds, &taken);
            let given_way = conflicts(layout, &keymap, &taken);
            if given_way.is_empty() {
                refused.sort_by_key(|refused| match refused.line {
                    Line::Bind(index) => (0, index),
                    Line::Unbind(index) => (1, index),
                });
                return (keymap, refused);
            }
            for (index, why) in given_way {
                taken.retain(|bind| bind.index != index);
                refused.push(Refused {
                    line: Line::Bind(index),
                    why,
                });
            }
        }
    }
}

/// The defaults with the unbinds and the binds laid over them.
fn lay(layout: Layout, unbinds: &[(Scope, Spelled)], taken: &[Taken]) -> Keymap {
    let mut bindings = BINDINGS.to_vec();
    for binding in &mut bindings {
        let row = Row::of(binding);
        if row.when != When::Always || fixed(layout, row).is_some() {
            continue;
        }
        let mine: Vec<&Taken> = taken.iter().filter(|bind| bind.row == row).collect();
        let bound = binding.bound_mut(layout);
        let mut chords: Vec<Chord> = bound.chords.to_vec();
        let before = chords.len();
        for (scope, spelled) in unbinds {
            if *scope == row.scope {
                chords.retain(|chord| {
                    chord.kind == Kind::Via
                        || !spell(chord.keys).is_ok_and(|keys| same(layout, &keys, spelled))
                });
            }
        }
        let mut changed = chords.len() != before;
        if !mine.is_empty() {
            let kinds: Vec<Press> = mine.iter().map(|bind| bind.press).collect();
            let recorded = |chord: &Chord| {
                let press = spell(chord.keys).ok().and_then(|keys| Press::of(&keys));
                press.is_some_and(|press| kinds.contains(&press))
            };
            // The new keys stand where the first they replace stood: a
            // line of the prompt recorded stays before a chord.
            let mut at = chords.iter().position(recorded).unwrap_or(chords.len());
            // A key of a kind recorded goes, another command's written
            // beside the row's among them.
            chords.retain(|chord| !recorded(chord));
            at = at.min(chords.len());
            for bind in &mine {
                if !chords.iter().any(|chord| chord.keys == bind.keys) {
                    chords.insert(
                        at,
                        Chord {
                            keys: bind.keys,
                            kind: Kind::Own,
                        },
                    );
                    at += 1;
                }
            }
            changed = true;
        }
        if changed {
            bound.chords = intern_chords(chords);
            // What the defaults wrote together (`j/k`) is no longer what
            // the line has.
            bound.shown = "";
        }
    }
    Keymap { bindings }
}

/// The binds that give way to a key another row has where both are read,
/// each with the row it gives way to: of two binds, the later one.
fn conflicts(layout: Layout, keymap: &Keymap, taken: &[Taken]) -> Vec<(usize, Why)> {
    let all = entries(keymap, layout);
    // The bind a key of a line comes from, if it does.
    let from_bind = |line: usize, keys: &str| -> Option<&Taken> {
        let row = Row::of(&keymap.bindings()[line]);
        taken
            .iter()
            .find(|bind| bind.row == row && bind.keys == keys)
    };
    let mut given_way: Vec<(usize, Why)> = Vec::new();
    for bind in taken {
        let mine = all
            .iter()
            .filter(|entry| Row::of(&keymap.bindings()[entry.line]) == bind.row)
            .filter(|entry| entry.keys == bind.keys);
        for entry in mine {
            let other = all.iter().find(|other| {
                other.command != entry.command
                    && entry.scope.alongside(other.scope)
                    && entry.when == other.when
                    && entry.mode.overlaps(other.mode)
                    && clash(layout, &bind.spelled, &other.spelled)
            });
            let Some(other) = other else {
                continue;
            };
            let other_row = Row::of(&keymap.bindings()[other.line]);
            let (index, why) = match from_bind(other.line, other.keys) {
                // The later of two binds gives way to the earlier one.
                Some(earlier) if earlier.index < bind.index => {
                    (bind.index, Why::Taken(earlier.row))
                }
                Some(later) => (later.index, Why::Taken(bind.row)),
                None => match fixed(layout, other_row) {
                    Some(_) => (bind.index, Why::Tabletist(other_row.command)),
                    None => (bind.index, Why::Taken(other_row)),
                },
            };
            if !given_way.iter().any(|(given, _)| *given == index) {
                given_way.push((index, why));
            }
            break;
        }
    }
    given_way
}

impl Overrides {
    pub fn is_empty(&self) -> bool {
        self.binds.is_empty() && self.unbinds.is_empty()
    }

    /// Whether a bind is a line of `row`.
    fn of_row(bind: &Bind, row: Row) -> bool {
        bind.scope == row.scope && bind.command == row.command && row.when == When::Always
    }

    /// The overrides with `row`'s keys of the kind `press` set to `keys`:
    /// its lines of that kind and the unbinds of its defaults of that kind
    /// go, and lines are written only where `keys` are not its defaults of
    /// that kind. No keys: the defaults of that kind are unbound.
    fn set_kind(&self, layout: Layout, row: Row, press: Press, keys: &[&'static str]) -> Self {
        let mut next = self.clone();
        next.binds
            .retain(|bind| !(Self::of_row(bind, row) && press_of(bind.keys) == Some(press)));
        let olds: Vec<&'static str> = defaults(layout, row)
            .into_iter()
            .filter(|keys| press_of(keys) == Some(press))
            .collect();
        next.unbinds.retain(|unbind| {
            !(unbind.scope == row.scope
                && olds.iter().any(|old| same_keys(layout, old, unbind.keys)))
        });
        let back = keys.len() == olds.len()
            && keys
                .iter()
                .all(|key| olds.iter().any(|old| same_keys(layout, old, key)));
        if back {
            return next;
        }
        let table = row.scope.table(layout);
        if keys.is_empty() {
            for old in olds {
                next.unbinds.push(Unbind {
                    scope: row.scope,
                    table,
                    keys: old,
                });
            }
            return next;
        }
        for key in keys {
            next.binds.push(Bind {
                scope: row.scope,
                table,
                keys: key,
                command: row.command,
                note: None,
            });
        }
        next
    }

    /// The overrides with `keys` recorded for `row`: they take the place of
    /// its keys of their kind.
    pub fn record(&self, layout: Layout, row: Row, keys: &'static str) -> Self {
        match press_of(keys) {
            Some(press) => self.set_kind(layout, row, press, &[intern(keys)]),
            None => self.clone(),
        }
    }

    /// The overrides with `row` given no keys: its lines go, and each of
    /// its defaults is unbound.
    pub fn unbind(&self, layout: Layout, row: Row) -> Self {
        let mut next = self.clone();
        next.binds.retain(|bind| !Self::of_row(bind, row));
        let table = row.scope.table(layout);
        for keys in defaults(layout, row) {
            let unbound = next
                .unbinds
                .iter()
                .any(|unbind| unbind.scope == row.scope && same_keys(layout, unbind.keys, keys));
            if !unbound {
                next.unbinds.push(Unbind {
                    scope: row.scope,
                    table,
                    keys,
                });
            }
        }
        next
    }

    /// The overrides with `row` back at its defaults: its lines go, and
    /// the unbinds of its defaults.
    pub fn reset(&self, layout: Layout, row: Row) -> Self {
        let mut next = self.clone();
        next.binds.retain(|bind| !Self::of_row(bind, row));
        let olds = defaults(layout, row);
        next.unbinds.retain(|unbind| {
            !(unbind.scope == row.scope
                && olds.iter().any(|old| same_keys(layout, old, unbind.keys)))
        });
        next
    }

    /// The overrides with `keys` recorded for `row`, where `other` has
    /// them: an exchange. `other` loses `keys` and gets the keys `row` had
    /// of their kind in their place; its other keys stay. The line of `row`
    /// says whose the key was.
    pub fn swap(&self, layout: Layout, row: Row, keys: &'static str, other: Row) -> Self {
        let Some(press) = press_of(keys) else {
            return self.clone();
        };
        let (now, _) = Keymap::with(layout, self);
        let of_kind = |row: Row| -> Vec<&'static str> {
            now.keys_of(layout, row)
                .into_iter()
                .filter(|keys| press_of(keys) == Some(press))
                .collect()
        };
        let old = of_kind(row);
        let mut given: Vec<&'static str> = Vec::new();
        for kept in of_kind(other) {
            if same_keys(layout, kept, keys) {
                for old in &old {
                    if !given.contains(old) {
                        given.push(old);
                    }
                }
            } else if !given.contains(&kept) {
                given.push(kept);
            }
        }
        let mut next = self.record(layout, row, keys);
        let note = format!("after swap · was {}", other.command.id());
        if let Some(bind) = next
            .binds
            .iter_mut()
            .rev()
            .find(|bind| Self::of_row(bind, row) && same_keys(layout, bind.keys, keys))
        {
            bind.note = Some(note);
        }
        next.set_kind(layout, other, press, &given)
    }

    /// Whether the swap of `keys` from `other` to `row` stands: the keymap
    /// takes both rows' lines. Where `row`'s old keys are a third row's,
    /// it would not, and none is offered.
    pub fn swaps(&self, layout: Layout, row: Row, keys: &'static str, other: Row) -> bool {
        let next = self.swap(layout, row, keys, other);
        let (_, refused) = Keymap::with(layout, &next);
        let (_, before) = Keymap::with(layout, self);
        refused.iter().all(|refusal| {
            let Line::Bind(index) = refusal.line else {
                return true;
            };
            let Some(bind) = next.binds.get(index) else {
                return true;
            };
            let touched = [row, other]
                .iter()
                .any(|row| bind.scope == row.scope && bind.command == row.command);
            // A line refused before the swap is no worse for it.
            let refused_before = before.iter().any(|old| match old.line {
                Line::Bind(at) => self.binds.get(at) == Some(bind),
                Line::Unbind(_) => false,
            });
            !touched || refused_before
        })
    }

    /// The overrides with `row` back at its defaults, but those another
    /// row's line has taken since, which stay unbound: a reset never
    /// takes a key from a row the person gave it to.
    pub fn reset_free(&self, layout: Layout, row: Row) -> Self {
        let mut next = self.reset(layout, row);
        let (_, before) = Keymap::with(layout, self);
        for _ in 0..defaults(layout, row).len() {
            let (_, refused) = Keymap::with(layout, &next);
            let taken: Vec<&'static str> = refused
                .iter()
                .filter(|refusal| matches!(refusal.why, Why::Taken(by) if by == row))
                .filter(|refusal| !before.iter().any(|old| old.line == refusal.line))
                .filter_map(|refusal| match refusal.line {
                    Line::Bind(index) => next.binds.get(index).map(|bind| bind.keys),
                    Line::Unbind(_) => None,
                })
                .collect();
            let defaults = defaults(layout, row);
            let lost: Vec<&'static str> = defaults
                .into_iter()
                .filter(|default| taken.iter().any(|keys| same_keys(layout, keys, default)))
                .filter(|default| {
                    !next.unbinds.iter().any(|unbind| {
                        unbind.scope == row.scope && same_keys(layout, unbind.keys, default)
                    })
                })
                .collect();
            if lost.is_empty() {
                break;
            }
            for keys in lost {
                next.unbinds.push(Unbind {
                    scope: row.scope,
                    table: row.scope.table(layout),
                    keys,
                });
            }
        }
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(command: Command, scope: Scope) -> Row {
        Row {
            command,
            scope,
            when: When::Always,
        }
    }

    fn bind(scope: Scope, keys: &str, command: Command) -> Bind {
        Bind {
            scope,
            table: scope.table(Layout::Mac),
            keys: intern(keys),
            command,
            note: None,
        }
    }

    fn unbind(scope: Scope, keys: &str) -> Unbind {
        Unbind {
            scope,
            table: scope.table(Layout::Omarchy),
            keys: intern(keys),
        }
    }

    fn binds(list: Vec<Bind>) -> Overrides {
        Overrides {
            binds: list,
            unbinds: Vec::new(),
        }
    }

    fn keys(keymap: &Keymap, layout: Layout, row: Row) -> Vec<&'static str> {
        keymap.keys_of(layout, row)
    }

    #[test]
    fn no_lines_are_the_defaults() {
        let (keymap, refused) = Keymap::with(Layout::Mac, &Overrides::default());
        assert!(refused.is_empty());
        let format = row(Command::Format, Scope::SqlEditor);
        assert_eq!(keys(&keymap, Layout::Mac, format), ["cmd+shift+f"]);
        assert!(!keymap.changed(Layout::Mac, format));
    }

    #[test]
    fn a_free_chord_takes_the_place_of_the_rows_chords_and_answers() {
        let format = row(Command::Format, Scope::SqlEditor);
        let lines = binds(vec![bind(Scope::SqlEditor, "cmd+alt+f", Command::Format)]);
        let (keymap, refused) = Keymap::with(Layout::Mac, &lines);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keys(&keymap, Layout::Mac, format), ["cmd+alt+f"]);
        assert!(keymap.changed(Layout::Mac, format));
        let chords: Vec<&str> = keymap
            .chords(Layout::Mac, Command::Format, Scope::SqlEditor)
            .collect();
        assert_eq!(chords, ["cmd+alt+f"]);
        assert_eq!(
            keymap.key_in(Layout::Mac, Command::Format, Scope::SqlEditor),
            "⌥⌘F"
        );
        // The large editor's line of the same command is another row.
        assert_eq!(
            keymap.key_in(Layout::Mac, Command::Format, Scope::CellEditor),
            "⇧⌘F"
        );
    }

    #[test]
    fn a_chord_leaves_a_line_of_the_prompt_beside_it() {
        let save = row(Command::SaveChanges, Scope::Grid);
        let lines = binds(vec![bind(Scope::Grid, "ctrl+alt+s", Command::SaveChanges)]);
        let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keys(&keymap, Layout::Omarchy, save), [":w", "ctrl+alt+s"]);
    }

    #[test]
    fn the_same_keys_in_two_scopes_stand_and_in_one_scope_or_global_conflict() {
        let mac = Layout::Mac;
        // ⌘N is New connection's in Connections and Add row's on a table.
        let (_, refused) = Keymap::with(
            mac,
            &binds(vec![bind(Scope::SqlEditor, "cmd+n", Command::Format)]),
        );
        assert!(refused.is_empty(), "{refused:?}");
        // In one scope: Run all has ⇧⌘↩ in the editor.
        let (keymap, refused) = Keymap::with(
            mac,
            &binds(vec![bind(
                Scope::SqlEditor,
                "cmd+shift+enter",
                Command::Format,
            )]),
        );
        assert_eq!(
            refused,
            [Refused {
                line: Line::Bind(0),
                why: Why::Taken(row(Command::RunAll, Scope::SqlEditor)),
            }]
        );
        // The default stays.
        let format = row(Command::Format, Scope::SqlEditor);
        assert_eq!(keys(&keymap, mac, format), ["cmd+shift+f"]);
        // Against the global scope: ⌘P is Find any object's everywhere.
        let (_, refused) = Keymap::with(
            mac,
            &binds(vec![bind(Scope::SqlEditor, "cmd+p", Command::Format)]),
        );
        assert_eq!(
            refused[0].why,
            Why::Taken(row(Command::FindObject, Scope::Global))
        );
        // And a global key against a scope's.
        let (_, refused) = Keymap::with(
            mac,
            &binds(vec![bind(
                Scope::Global,
                "cmd+shift+f",
                Command::FindObject,
            )]),
        );
        assert_eq!(refused[0].why, Why::Taken(format));
    }

    #[test]
    fn of_two_lines_with_one_key_the_later_gives_way() {
        let lines = binds(vec![
            bind(Scope::SqlEditor, "cmd+j", Command::Format),
            bind(Scope::SqlEditor, "cmd+j", Command::RunAll),
        ]);
        let (keymap, refused) = Keymap::with(Layout::Mac, &lines);
        assert_eq!(
            refused,
            [Refused {
                line: Line::Bind(1),
                why: Why::Taken(row(Command::Format, Scope::SqlEditor)),
            }]
        );
        let run_all = row(Command::RunAll, Scope::SqlEditor);
        assert_eq!(keys(&keymap, Layout::Mac, run_all), ["cmd+shift+enter"]);
    }

    #[test]
    fn the_keys_of_a_fixed_row_are_tabletists() {
        let (_, refused) = Keymap::with(
            Layout::Omarchy,
            &binds(vec![bind(Scope::Grid, "j", Command::SetNull)]),
        );
        assert_eq!(refused[0].why, Why::Tabletist(Command::MoveRow));
    }

    #[test]
    fn a_look_refuses_the_keys_it_never_gives() {
        let format = row(Command::Format, Scope::SqlEditor);
        let refused =
            |layout, keys: &str| refuse_key(layout, format, &spell(intern(keys)).unwrap());
        assert_eq!(
            refused(Layout::Mac, "cmd+q"),
            Some(Why::Reserved(Owner::MacOs))
        );
        assert_eq!(
            refused(Layout::Mac, "cmd+w"),
            Some(Why::Reserved(Owner::MacOs))
        );
        for keys in ["cmd+c", "cmd+v", "cmd+x", "cmd+z", "cmd+shift+z", "cmd+a"] {
            assert_eq!(
                refused(Layout::Mac, keys),
                Some(Why::Reserved(Owner::MacOs)),
                "{keys}"
            );
        }
        assert_eq!(refused(Layout::Mac, "shift+f"), Some(Why::Alone));
        assert_eq!(refused(Layout::Mac, "enter"), Some(Why::Alone));
        assert_eq!(refused(Layout::Mac, "f5"), None);
        assert_eq!(refused(Layout::Mac, "cmd+j"), None);
        assert_eq!(
            refused(Layout::Windows, "ctrl+c"),
            Some(Why::Reserved(Owner::Windows))
        );
        let omarchy = Layout::Omarchy;
        assert_eq!(refused(omarchy, "cmd+j"), Some(Why::Never(Never::Super)));
        assert_eq!(refused(omarchy, "esc"), Some(Why::Never(Never::Escape)));
        // The SQL editor's text keeps vim's insert keys.
        assert_eq!(refused(omarchy, "ctrl+r"), Some(Why::Vim));
        assert_eq!(refused(omarchy, "ctrl+w"), Some(Why::Vim));
        // Format is `=` there, typed: a chord is of another kind.
        assert_eq!(
            refused(omarchy, "ctrl+j"),
            Some(Why::Kind(vec![Press::Typed]))
        );
        let delete = row(Command::DeleteRow, Scope::Grid);
        let refused = |keys: &str| refuse_key(omarchy, delete, &spell(intern(keys)).unwrap());
        assert_eq!(refused("3x"), Some(Why::Count));
        assert_eq!(refused("gx"), None);
        let reload = row(Command::Reload, Scope::Grid);
        assert_eq!(
            refuse_key(omarchy, reload, &spell("gr").unwrap()),
            Some(Why::OneLetter)
        );
        assert_eq!(refuse_key(omarchy, reload, &spell("r").unwrap()), None);
    }

    #[test]
    fn a_line_of_a_row_that_does_not_exist_or_is_fixed_is_refused() {
        let (_, refused) = Keymap::with(
            Layout::Mac,
            &binds(vec![
                bind(Scope::Connections, "cmd+j", Command::RunAll),
                bind(Scope::Grid, "cmd+j", Command::MoveRow),
                bind(Scope::Grid, "cmd+alt+j", Command::OpenReferencedRow),
            ]),
        );
        let whys: Vec<Why> = refused.into_iter().map(|refused| refused.why).collect();
        assert_eq!(
            whys,
            [
                Why::NoRow,
                Why::Fixed(Fixed::Direction),
                // Not listed on macOS: nothing would read it there.
                Why::NoRow,
            ]
        );
    }

    #[test]
    fn an_unbind_takes_a_default_off_its_scope() {
        let lines = Overrides {
            binds: Vec::new(),
            unbinds: vec![unbind(Scope::Grid, "gd"), unbind(Scope::Grid, "zz")],
        };
        let (keymap, refused) = Keymap::with(Layout::Omarchy, &lines);
        assert_eq!(
            refused,
            [Refused {
                line: Line::Unbind(1),
                why: Why::NotBound,
            }]
        );
        let follow = row(Command::OpenReferencedRow, Scope::Grid);
        assert!(keys(&keymap, Layout::Omarchy, follow).is_empty());
        let nothing = |_: When| false;
        assert_eq!(
            keymap.typed(Layout::Omarchy, Scope::Grid, &nothing, "gd"),
            crate::keymap::Typed::Nothing
        );
    }

    #[test]
    fn a_recorded_key_is_the_menu_s_key() {
        // The sidebar's toggle has one line (GLOBAL), so the recorded chord
        // is its only one. (Format would not do: its large editor's line
        // keeps its own chord, and two chords show none.)
        let mac = Layout::Mac;
        let sidebar = row(Command::ToggleSidebar, Scope::Global);
        let recorded = Overrides::default().record(mac, sidebar, "cmd+alt+b");
        let (keymap, refused) = Keymap::with(mac, &recorded);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(
            keymap.menu_key(Command::ToggleSidebar, mac),
            Some(crate::keymap::MenuKey {
                mods: crate::keymap::Mods {
                    cmd: true,
                    alt: true,
                    ..Default::default()
                },
                key: egui::Key::B,
            })
        );
    }

    #[test]
    fn two_recorded_chords_show_no_menu_key() {
        // Format's SQL editor line takes a new chord and its cell editor
        // line keeps `cmd+shift+f`: two chords, so the menu shows none.
        let mac = Layout::Mac;
        let format = row(Command::Format, Scope::SqlEditor);
        let recorded = Overrides::default().record(mac, format, "cmd+alt+f");
        let (keymap, refused) = Keymap::with(mac, &recorded);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.menu_key(Command::Format, mac), None);
    }

    #[test]
    fn a_recorded_chord_without_cmd_or_ctrl_shows_no_menu_key() {
        let mac = Layout::Mac;
        let sidebar = row(Command::ToggleSidebar, Scope::Global);
        let recorded = Overrides::default().record(mac, sidebar, "alt+b");
        let (keymap, refused) = Keymap::with(mac, &recorded);
        assert!(refused.is_empty(), "{refused:?}");
        // The key is the command's (the line took it), yet no menu shows it.
        assert!(
            keymap
                .of(Command::ToggleSidebar)
                .flat_map(|binding| binding.bound(mac).chords.iter())
                .any(|chord| chord.keys == "alt+b"),
            "the recorded key stands"
        );
        assert_eq!(keymap.menu_key(Command::ToggleSidebar, mac), None);
    }

    #[test]
    fn recording_gives_a_row_a_key_and_its_default_back_writes_nothing() {
        let mac = Layout::Mac;
        let format = row(Command::Format, Scope::SqlEditor);
        let recorded = Overrides::default().record(mac, format, "cmd+alt+f");
        assert_eq!(
            recorded.binds,
            [Bind {
                scope: Scope::SqlEditor,
                table: "editor",
                keys: "cmd+alt+f",
                command: Command::Format,
                note: None,
            }]
        );
        // Again: the line is replaced, not added to.
        let again = recorded.record(mac, format, "cmd+alt+g");
        assert_eq!(again.binds.len(), 1);
        assert_eq!(again.binds[0].keys, "cmd+alt+g");
        // The default again: no line.
        assert!(again.record(mac, format, "cmd+shift+f").is_empty());
        // Spelled another way, the default is the default.
        let error = row(Command::NextError, Scope::Grid);
        assert!(
            Overrides::default()
                .record(mac, error, "alt+cmd+down")
                .is_empty()
        );
    }

    #[test]
    fn unbinding_writes_the_defaults_and_resetting_takes_every_line_away() {
        let omarchy = Layout::Omarchy;
        let follow = row(Command::OpenReferencedRow, Scope::Grid);
        let unbound = Overrides::default().unbind(omarchy, follow);
        assert_eq!(unbound.unbinds, [unbind(Scope::Grid, "gd")]);
        assert_eq!(unbound.unbinds[0].table, "grid");
        let (keymap, refused) = Keymap::with(omarchy, &unbound);
        assert!(refused.is_empty());
        assert!(keymap.keys_of(omarchy, follow).is_empty());
        assert!(keymap.changed(omarchy, follow));
        assert!(unbound.reset(omarchy, follow).is_empty());
        // Recorded, then unbound: the line goes with it.
        let save = row(Command::SaveChanges, Scope::Grid);
        let recorded = Overrides::default().record(omarchy, save, "ctrl+alt+s");
        let unbound = recorded.unbind(omarchy, save);
        assert!(unbound.binds.is_empty());
        assert_eq!(unbound.unbinds.len(), 2);
        assert!(unbound.reset(omarchy, save).is_empty());
    }

    #[test]
    fn a_swap_moves_the_keys_both_ways_and_says_so() {
        let omarchy = Layout::Omarchy;
        let delete = row(Command::DeleteRow, Scope::Grid);
        let default = row(Command::SetDefault, Scope::Grid);
        // `D` is Set DEFAULT's: recorded for Delete row it is taken.
        let recorded = Overrides::default().record(omarchy, delete, "D");
        let (_, refused) = Keymap::with(omarchy, &recorded);
        assert_eq!(refused[0].why, Why::Taken(default));
        let swapped = Overrides::default().swap(omarchy, delete, "D", default);
        let (keymap, refused) = Keymap::with(omarchy, &swapped);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.keys_of(omarchy, delete), ["D"]);
        assert_eq!(keymap.keys_of(omarchy, default), ["dd"]);
        let lines: Vec<(&str, Command, Option<&str>)> = swapped
            .binds
            .iter()
            .map(|bind| (bind.keys, bind.command, bind.note.as_deref()))
            .collect();
        assert_eq!(
            lines,
            [
                (
                    "D",
                    Command::DeleteRow,
                    Some("after swap · was set_default")
                ),
                ("dd", Command::SetDefault, None),
            ]
        );
        // A row with no keys of the kind leaves the other with none.
        let mac = Layout::Mac;
        let compare = row(Command::ComparePreviousRun, Scope::SqlEditor);
        let analyze = row(Command::ExplainAnalyze, Scope::SqlEditor);
        let swapped = Overrides::default().swap(mac, compare, "cmd+alt+e", analyze);
        let (keymap, refused) = Keymap::with(mac, &swapped);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.keys_of(mac, compare), ["cmd+alt+e"]);
        assert!(keymap.keys_of(mac, analyze).is_empty());
        assert_eq!(
            swapped.unbinds,
            [Unbind {
                scope: Scope::SqlEditor,
                table: "editor",
                keys: "cmd+alt+e",
            }]
        );
    }

    #[test]
    fn a_swap_is_an_exchange_and_the_other_rows_other_keys_stay() {
        let mac = Layout::Mac;
        let reload = row(Command::Reload, Scope::Grid);
        let edit = row(Command::EditCell, Scope::Grid);
        let recorded = Overrides::default().record(mac, reload, "f2");
        let (_, refused) = Keymap::with(mac, &recorded);
        assert_eq!(refused[0].why, Why::Taken(edit));
        assert!(Overrides::default().swaps(mac, reload, "f2", edit));
        let swapped = Overrides::default().swap(mac, reload, "f2", edit);
        let (keymap, refused) = Keymap::with(mac, &swapped);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.keys_of(mac, reload), ["f2"]);
        // Return still edits a cell; ⌘R takes F2's place.
        assert_eq!(keymap.keys_of(mac, edit), ["enter", "cmd+r"]);
    }

    #[test]
    fn a_swap_that_would_not_stand_is_not_offered() {
        let mac = Layout::Mac;
        let add = row(Command::AddRow, Scope::Grid);
        let sidebar = row(Command::ToggleSidebar, Scope::Global);
        let (_, refused) = Keymap::with(mac, &Overrides::default().record(mac, add, "cmd+b"));
        assert_eq!(refused[0].why, Why::Taken(sidebar));
        // Toggle sidebar would get ⌘N, which is New connection's in the
        // picker: the global scope is read there too.
        assert!(!Overrides::default().swaps(mac, add, "cmd+b", sidebar));
    }

    #[test]
    fn a_reset_leaves_a_default_another_row_was_given() {
        let mac = Layout::Mac;
        let format = row(Command::Format, Scope::SqlEditor);
        let run_all = row(Command::RunAll, Scope::SqlEditor);
        let lines = Overrides::default()
            .unbind(mac, format)
            .record(mac, run_all, "cmd+shift+f");
        let (_, refused) = Keymap::with(mac, &lines);
        assert!(refused.is_empty());
        let reset = lines.reset_free(mac, format);
        let (keymap, refused) = Keymap::with(mac, &reset);
        assert!(refused.is_empty(), "{refused:?}");
        assert_eq!(keymap.keys_of(mac, run_all), ["cmd+shift+f"]);
        assert!(keymap.keys_of(mac, format).is_empty());
        // With nothing taken, a reset is a reset.
        let free = Overrides::default()
            .unbind(mac, format)
            .reset_free(mac, format);
        assert!(free.is_empty());
    }

    #[test]
    fn a_refusal_names_the_looks_own_keys() {
        let kind = Why::Kind(vec![Press::Chord]);
        assert_eq!(kind.text(Layout::Mac), "Add ⌘, ⌃ or ⌥");
        assert_eq!(kind.text(Layout::Windows), "Add Ctrl or Alt");
        assert_eq!(kind.text(Layout::Omarchy), "{name} takes a chord");
    }

    #[test]
    fn a_spelling_is_kept_once() {
        let a = intern(&String::from("ctrl+alt+q"));
        let b = intern(&String::from("ctrl+alt+q"));
        assert!(std::ptr::eq(a, b));
    }
}
