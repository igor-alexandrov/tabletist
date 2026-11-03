//! Vim's normal, insert and visual modes for the SQL editor, as
//! Settings › Editor's Keybindings sets them.
//!
//! The engine knows nothing of egui: it reads a key with the script and
//! the cursor, and says what changes. The editor feeds it the keys of
//! normal and visual mode, and Esc in insert mode; insert mode's typing is
//! the text field's own. Places are character indices, as the field
//! counts them.
//!
//! What it has: counts; the motions `h j k l w b e W B E 0 ^ $ gg G f F t
//! T ; , % { }`; the ways into insert mode `i a I A o O`; `x X D C Y s S p
//! P J r ~`; the operators `d c y > < =` with a motion, or doubled for
//! lines; undo `u` and redo Ctrl+R; visual mode by character `v` and by
//! line `V`, which the same motions move and the operators act on. `:`
//! opens the editor's command line, and Esc with nothing typed leaves the
//! editor. Not: text objects, `.`, registers other than the one, search,
//! marks, macros.

use std::ops::Range;

/// Where the keys go.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Mode {
    /// Keys are commands; the cursor sits on a character.
    #[default]
    Normal,
    /// Keys type, as in the standard keybindings.
    Insert,
    /// A selection by character, from where `v` was typed to the cursor,
    /// both ends in.
    Visual,
    /// A selection of whole lines, from the line `V` was typed on to the
    /// cursor's.
    VisualLine,
}

impl Mode {
    /// What the status line says, as vim's: nothing in normal mode.
    pub fn label(self) -> Option<&'static str> {
        match self {
            Mode::Normal => None,
            Mode::Insert => Some("-- INSERT --"),
            Mode::Visual => Some("-- VISUAL --"),
            Mode::VisualLine => Some("-- VISUAL LINE --"),
        }
    }

    pub fn is_visual(self) -> bool {
        matches!(self, Mode::Visual | Mode::VisualLine)
    }
}

/// A key as the engine reads it: a character typed, or a key that types
/// none.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Input {
    Char(char),
    Escape,
    Enter,
    Backspace,
    Delete,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    /// Ctrl+R.
    Redo,
}

/// What a delete or a yank keeps and `p` puts back: vim's unnamed
/// register. Whole lines end with a line break.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Register {
    pub text: String,
    pub lines: bool,
}

/// What the editor does after a key, besides taking the text and the
/// cursor the step gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Then {
    /// Format the statements these characters (whole lines) touch; all of
    /// them where the range is the whole script.
    Format(Range<usize>),
    /// Open the command line, as `:` does with the keyboard out of a text
    /// field.
    Prompt,
    /// Esc with nothing typed: the keyboard leaves the editor.
    Leave,
}

/// What a key does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Step {
    /// The script after the key, where it changed.
    pub text: Option<String>,
    /// The cursor.
    pub cursor: usize,
    /// In visual mode, the selection's other end; the selection covers
    /// both ends' characters (by line, whole lines).
    pub anchor: Option<usize>,
    pub then: Option<Then>,
}

/// One editor's vim: its mode and the command typed so far.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Vim {
    pub mode: Mode,
    /// Where visual mode started.
    anchor: usize,
    /// The cursor in visual mode: the field's selection runs past it.
    head: usize,
    /// The column `j` and `k` keep to (`usize::MAX` after `$`), with the
    /// cursor it was kept at: it holds while the cursor is there, and an
    /// edit, an insert or a click that moves it starts from its own.
    column: Option<(usize, usize)>,
    /// The last `f`, `F`, `t` or `T`, for `;` and `,`.
    find: Option<(char, char)>,
    pending: Pending,
    history: History,
}

/// Vim's undo history: the script before each change, with the cursor to
/// put back. A change is a command of normal or visual mode, an insert
/// from the key that started it to Esc, or what changed from outside
/// between two keys (a paste, Format). Cursor moves are none.
#[derive(Clone, Default, PartialEq, Eq)]
struct History {
    undo: Vec<(String, usize)>,
    redo: Vec<(String, usize)>,
    /// The script as the last change left it, and the cursor then.
    seen: Option<(String, usize)>,
}

// The scripts stay out of logs, as `SqlTab`'s do.
impl std::fmt::Debug for History {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("History")
            .field("undo", &self.undo.len())
            .field("redo", &self.redo.len())
            .finish_non_exhaustive()
    }
}

impl History {
    /// The most steps kept, as egui's history keeps.
    const KEPT: usize = 100;
    /// The most bytes of scripts the steps hold together: each is a whole
    /// script, so a large one keeps fewer. The latest step stays whatever
    /// its size.
    const BYTES: usize = 16 << 20;

    /// `before` (a script and its cursor) as a step to undo to; nothing
    /// is left to redo.
    fn push(&mut self, before: (String, usize)) {
        self.undo.push(before);
        let mut bytes: usize = self.undo.iter().map(|(text, _)| text.len()).sum();
        while self.undo.len() > Self::KEPT || (bytes > Self::BYTES && self.undo.len() > 1) {
            bytes -= self.undo.remove(0).0.len();
        }
        self.redo.clear();
    }

    /// The script is `text` now: where it is not what the last change
    /// left, what changed since is a step of its own.
    fn settle(&mut self, text: &str, cursor: usize) {
        if let Some(seen) = self.seen.take()
            && seen.0 != text
        {
            self.push(seen);
        }
        self.seen = Some((text.to_owned(), cursor));
    }
}

/// A command typed in part.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Pending {
    count: Option<usize>,
    /// The operator and the count typed before it.
    operator: Option<(Operator, usize)>,
    /// `g` typed, waiting for the second `g`.
    g: bool,
    /// `f`, `F`, `t`, `T` or `r` typed, waiting for its character.
    wait: Option<char>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Operator {
    Delete,
    Change,
    Yank,
    Indent,
    Outdent,
    Format,
}

impl Operator {
    fn of(key: char) -> Option<Operator> {
        Some(match key {
            'd' => Operator::Delete,
            'c' => Operator::Change,
            'y' => Operator::Yank,
            '>' => Operator::Indent,
            '<' => Operator::Outdent,
            '=' => Operator::Format,
            _ => return None,
        })
    }
}

/// How a motion's range is taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// Up to the target, not it.
    Exclusive,
    /// Up to the target and it.
    Inclusive,
    /// The lines from the cursor's to the target's.
    Lines,
}

/// Where a motion goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Target {
    to: usize,
    kind: Kind,
    /// The column `j` and `k` keep to after it: kept where `None`.
    column: Option<usize>,
}

impl Vim {
    /// Vim in `mode`, as an editor opens. Its history starts with the
    /// script the editor first shows ([`Vim::start`]).
    pub fn new(mode: Mode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    /// Starts the history at `text` where it has not started: the oldest
    /// change undo goes back to. The editor calls it as it draws, so a tab
    /// opened with a query (from the history, say) starts at that query.
    pub fn start(&mut self, text: &str) {
        if self.history.seen.is_none() {
            self.history.seen = Some((text.to_owned(), 0));
        }
    }

    /// Starts the history over at `text`: the keybindings changed.
    pub fn start_over(&mut self, text: &str) {
        self.history = History::default();
        self.start(text);
    }

    /// Whether a command is typed in part: Esc then ends it, and the
    /// keyboard stays.
    pub fn is_pending(&self) -> bool {
        self.pending != Pending::default()
    }

    /// Reads `input` with `text` and the cursor at `cursor`. `indent` is
    /// what `>` puts in at the start of a line.
    pub fn key(
        &mut self,
        text: &str,
        cursor: usize,
        input: Input,
        register: &mut Register,
        indent: &str,
    ) -> Step {
        let chars: Vec<char> = text.chars().collect();
        let t = Text { c: &chars };
        let cursor = cursor.min(t.len());
        let mut step = Step {
            text: None,
            cursor,
            anchor: None,
            then: None,
        };
        // Out of insert mode, what changed since the last key is a step of
        // its own; in it, the insert is one, settled by Esc.
        let undo_point = match self.history.seen.as_ref() {
            Some((seen, at)) if self.mode == Mode::Insert => (seen.clone(), *at),
            _ => {
                self.history.settle(text, cursor);
                (text.to_owned(), cursor)
            }
        };
        let was = self.mode;
        match self.mode {
            Mode::Insert => {
                if input == Input::Escape {
                    // Back over the last character typed, as vim does.
                    self.mode = Mode::Normal;
                    let start = t.line_start(cursor);
                    step.cursor = t.clamp(cursor.saturating_sub(1).max(start));
                    self.history.seen = Some(undo_point.clone());
                    self.history.settle(text, step.cursor);
                }
            }
            Mode::Normal => {
                // Undo and redo, which are the history's own.
                let count = self.pending.count.unwrap_or(1);
                let undo = input == Input::Char('u')
                    && self.pending.operator.is_none()
                    && !self.pending.g
                    && self.pending.wait.is_none();
                if undo || input == Input::Redo {
                    self.pending = Pending::default();
                    self.history_steps(&t, count, undo, &mut step);
                    return step;
                }
                self.normal(&t, cursor, input, register, indent, &mut step);
            }
            Mode::Visual | Mode::VisualLine => {
                self.visual(&t, cursor, input, register, indent, &mut step);
            }
        }
        if let Some(new) = &step.text {
            if self.mode == Mode::Insert {
                // A change that goes on in insert mode: one step with the
                // typing, from before it.
                self.history.seen = Some(undo_point);
            } else if was != Mode::Insert {
                self.history.push(undo_point);
                self.history.seen = Some((new.clone(), step.cursor));
            }
        }
        if self.mode.is_visual() {
            step.anchor = Some(self.anchor);
            self.head = step.cursor;
        }
        step
    }

    /// `u` (or with `undo` false, Ctrl+R) `count` times: the script and
    /// the cursor of a step of the history, the one it leaves kept for
    /// the other way.
    fn history_steps(&mut self, t: &Text<'_>, count: usize, undo: bool, step: &mut Step) {
        let mut now = (t.slice(0..t.len()), step.cursor);
        let mut moved = false;
        for _ in 0..count {
            let (from, to) = if undo {
                (&mut self.history.undo, &mut self.history.redo)
            } else {
                (&mut self.history.redo, &mut self.history.undo)
            };
            let Some(then) = from.pop() else {
                break;
            };
            to.push(std::mem::replace(&mut now, then));
            moved = true;
        }
        if !moved {
            return;
        }
        let chars: Vec<char> = now.0.chars().collect();
        step.cursor = Text { c: &chars }.clamp(now.1);
        self.history.seen = Some((now.0.clone(), step.cursor));
        step.text = Some(now.0);
    }

    /// The cursor in visual mode, which the field's selection does not
    /// say: it runs one past it.
    pub fn head(&self) -> usize {
        self.head
    }

    /// Leaves visual mode for normal, or insert mode for normal (where
    /// the keybindings change, or the editor gives the keyboard up).
    pub fn to_normal(&mut self) {
        self.mode = Mode::Normal;
        self.pending = Pending::default();
    }

    fn normal(
        &mut self,
        t: &Text<'_>,
        cursor: usize,
        input: Input,
        register: &mut Register,
        indent: &str,
        step: &mut Step,
    ) {
        let pending = std::mem::take(&mut self.pending);
        // A character a command waits for.
        if let Some(wait) = pending.wait {
            let c = match input {
                Input::Char(c) => c,
                // `r` and Enter: a line break.
                Input::Enter if wait == 'r' => '\n',
                _ => return,
            };
            let count = pending.count.unwrap_or(1);
            if wait == 'r' {
                replace(t, cursor, c, count, step);
                return;
            }
            self.find = Some((wait, c));
            let target = find(t, cursor, wait, c, count, false);
            self.finish_motion(t, cursor, target, pending, register, indent, step);
            return;
        }
        let key = match input {
            Input::Char(c) => c,
            Input::Escape => {
                // Esc ends a command typed in part; with none it leaves.
                if pending == Pending::default() {
                    step.then = Some(Then::Leave);
                }
                return;
            }
            // Read as the history's, before this.
            Input::Redo => return,
            Input::Enter => '+',
            Input::Backspace | Input::Left => 'h',
            Input::Right => 'l',
            Input::Up => 'k',
            Input::Down => 'j',
            Input::Home => '0',
            Input::End => '$',
            Input::Delete => 'x',
        };
        // A count: digits, and 0 only after one.
        if key.is_ascii_digit() && (key != '0' || pending.count.is_some()) && !pending.g {
            let digit = key.to_digit(10).unwrap_or(0) as usize;
            let count = pending
                .count
                .unwrap_or(0)
                .saturating_mul(10)
                .saturating_add(digit);
            self.pending = Pending {
                count: Some(count.min(COUNTS)),
                ..pending
            };
            return;
        }
        if pending.g {
            if key == 'g' {
                let target = Some(line_target(t, pending.count.unwrap_or(1)));
                self.finish_motion(t, cursor, target, pending, register, indent, step);
            }
            return;
        }
        if let Some((operator, _)) = pending.operator {
            // A doubled operator acts on lines: `dd`, `cc`, `yy`, `>>`.
            if Operator::of(key) == Some(operator) {
                let count = total(&pending);
                let last = t.line_of(cursor) + count - 1;
                let to = t.line_at(last.min(t.lines() - 1));
                let target = Target {
                    to,
                    kind: Kind::Lines,
                    column: None,
                };
                self.finish_motion(t, cursor, Some(target), pending, register, indent, step);
                return;
            }
        }
        match key {
            'g' => {
                self.pending = Pending { g: true, ..pending };
                return;
            }
            'f' | 'F' | 't' | 'T' => {
                self.pending = Pending {
                    wait: Some(key),
                    ..pending
                };
                return;
            }
            'r' if pending.operator.is_none() => {
                self.pending = Pending {
                    wait: Some('r'),
                    ..pending
                };
                return;
            }
            _ => {}
        }
        if let Some(operator) = Operator::of(key)
            && pending.operator.is_none()
        {
            self.pending = Pending {
                count: None,
                operator: Some((operator, pending.count.unwrap_or(1))),
                ..Pending::default()
            };
            return;
        }
        let count = pending.count.unwrap_or(1);
        if let Some(target) = self.motion(t, cursor, key, &pending) {
            self.finish_motion(t, cursor, Some(target), pending, register, indent, step);
            return;
        }
        if pending.operator.is_some() {
            // An operator with no motion after it: dropped.
            return;
        }
        let (start, end) = (t.line_start(cursor), t.line_end(cursor));
        match key {
            ':' => step.then = Some(Then::Prompt),
            'i' => self.mode = Mode::Insert,
            'a' => {
                self.mode = Mode::Insert;
                step.cursor = (cursor + 1).min(end);
            }
            'I' => {
                self.mode = Mode::Insert;
                // On a line of blanks, after them.
                let at = t.first_non_blank(cursor);
                let blank = t.c.get(at).is_some_and(|c| *c == ' ' || *c == '\t');
                step.cursor = if blank { t.line_end(cursor) } else { at };
            }
            'A' => {
                self.mode = Mode::Insert;
                step.cursor = end;
            }
            'o' | 'O' => {
                self.mode = Mode::Insert;
                let lead: String = t.c[start..t.first_non_blank(cursor)].iter().collect();
                let (at, put) = if key == 'o' {
                    (end, format!("\n{lead}"))
                } else {
                    (start, format!("{lead}\n"))
                };
                let to = if key == 'o' {
                    end + put.chars().count()
                } else {
                    start + lead.chars().count()
                };
                splice(t, at..at, &put, to, step);
            }
            'v' | 'V' => {
                self.mode = if key == 'v' {
                    Mode::Visual
                } else {
                    Mode::VisualLine
                };
                self.anchor = cursor;
            }
            'x' | 'X' | 's' => {
                if start == end {
                    if key == 's' {
                        self.mode = Mode::Insert;
                    }
                    return;
                }
                let range = if key == 'X' {
                    cursor.saturating_sub(count).max(start)..cursor
                } else {
                    cursor..(cursor + count).min(end)
                };
                // `X` at a line's start: nothing to take, and the register
                // keeps what it has.
                if range.is_empty() {
                    return;
                }
                *register = Register {
                    text: t.slice(range.clone()),
                    lines: false,
                };
                let to = range.start;
                splice(t, range, "", to, step);
                if key == 's' {
                    self.mode = Mode::Insert;
                } else {
                    step.cursor = clamp_in(step, to);
                }
            }
            'D' | 'C' | 'Y' | 'S' => {
                // `d$`, `c$`, `yy` and `cc`, as vim reads them.
                let (operator, motion) = match key {
                    'D' => (Operator::Delete, '$'),
                    'C' => (Operator::Change, '$'),
                    'Y' => (Operator::Yank, 'y'),
                    _ => (Operator::Change, 'c'),
                };
                self.pending = Pending {
                    operator: Some((operator, 1)),
                    ..Pending::default()
                };
                self.pending.count = pending.count;
                self.normal(t, cursor, Input::Char(motion), register, indent, step);
            }
            'p' | 'P' => put(t, cursor, key == 'p', count, register, step),
            'J' => join(t, t.line_of(cursor), count.max(2) - 1, step),
            '~' => {
                if start == end {
                    return;
                }
                let range = cursor..(cursor + count).min(end);
                let flipped: String = t.c[range.clone()].iter().map(|c| flip(*c)).collect();
                let to = range.end.min(end - 1);
                splice(t, range, &flipped, to, step);
            }
            _ => {}
        }
    }

    fn visual(
        &mut self,
        t: &Text<'_>,
        cursor: usize,
        input: Input,
        register: &mut Register,
        indent: &str,
        step: &mut Step,
    ) {
        let pending = std::mem::take(&mut self.pending);
        if let Some(wait) = pending.wait {
            if let Input::Char(c) = input {
                self.find = Some((wait, c));
                if let Some(target) = find(t, cursor, wait, c, pending.count.unwrap_or(1), false) {
                    self.moved(t, target, step);
                }
            }
            return;
        }
        let key = match input {
            Input::Char(c) => c,
            Input::Escape => {
                self.mode = Mode::Normal;
                step.cursor = t.clamp(cursor);
                return;
            }
            Input::Enter => '+',
            Input::Backspace | Input::Left => 'h',
            Input::Right => 'l',
            Input::Up => 'k',
            Input::Down => 'j',
            Input::Home => '0',
            Input::End => '$',
            Input::Delete => 'x',
            Input::Redo => return,
        };
        if key.is_ascii_digit() && (key != '0' || pending.count.is_some()) && !pending.g {
            let digit = key.to_digit(10).unwrap_or(0) as usize;
            let count = pending
                .count
                .unwrap_or(0)
                .saturating_mul(10)
                .saturating_add(digit);
            self.pending = Pending {
                count: Some(count.min(COUNTS)),
                ..pending
            };
            return;
        }
        if pending.g {
            if key == 'g' {
                let target = line_target(t, pending.count.unwrap_or(1));
                self.moved(t, target, step);
            }
            return;
        }
        match key {
            'g' => {
                self.pending = Pending { g: true, ..pending };
                return;
            }
            'f' | 'F' | 't' | 'T' => {
                self.pending = Pending {
                    wait: Some(key),
                    ..pending
                };
                return;
            }
            _ => {}
        }
        if let Some(target) = self.motion(t, cursor, key, &pending) {
            self.moved(t, target, step);
            return;
        }
        let lines = self.mode == Mode::VisualLine;
        let (low, high) = (self.anchor.min(cursor), self.anchor.max(cursor));
        let range = if lines {
            t.line_start(low)..(t.line_end(high) + 1).min(t.len())
        } else {
            low..(high + 1).min(t.len())
        };
        match key {
            'v' | 'V' => {
                let mode = if key == 'v' {
                    Mode::Visual
                } else {
                    Mode::VisualLine
                };
                self.mode = if self.mode == mode {
                    Mode::Normal
                } else {
                    mode
                };
                step.cursor = t.clamp(cursor);
            }
            'o' => {
                step.cursor = self.anchor;
                self.anchor = cursor;
            }
            ':' => step.then = Some(Then::Prompt),
            'd' | 'x' | 'c' | 's' | 'y' | '>' | '<' | '=' => {
                let operator = match key {
                    'd' | 'x' => Operator::Delete,
                    'c' | 's' => Operator::Change,
                    'y' => Operator::Yank,
                    '>' => Operator::Indent,
                    '<' => Operator::Outdent,
                    _ => Operator::Format,
                };
                self.mode = Mode::Normal;
                let count = pending.count.unwrap_or(1);
                self.operate(
                    t, cursor, operator, range, lines, count, register, indent, step,
                );
            }
            'J' => {
                self.mode = Mode::Normal;
                let (first, last) = (t.line_of(low), t.line_of(high));
                join(t, first, (last - first).max(1), step);
            }
            '~' | 'u' | 'U' => {
                self.mode = Mode::Normal;
                let changed: String = t.c[range.clone()]
                    .iter()
                    .map(|c| match key {
                        '~' => flip(*c),
                        'u' => c.to_lowercase().next().unwrap_or(*c),
                        _ => c.to_uppercase().next().unwrap_or(*c),
                    })
                    .collect();
                let to = if lines { range.start } else { low };
                splice(t, range, &changed, to, step);
                step.cursor = clamp_in(step, to);
            }
            'p' | 'P' => {
                self.mode = Mode::Normal;
                // The selection goes; what was yanked goes in its place.
                let put = register.clone();
                let start = range.start;
                *register = Register {
                    text: t.slice(range.clone()),
                    lines,
                };
                let mut put_text = put.text.clone();
                if put.lines && !lines {
                    put_text = format!("\n{put_text}");
                } else if lines && !put.lines {
                    put_text.push('\n');
                }
                // The last line has no break after it, and gets none.
                let last = range.end == t.len() && t.c.last() != Some(&'\n');
                if lines && last && put_text.ends_with('\n') {
                    put_text.pop();
                }
                // On the last character put, or a line's first.
                let length = put_text.chars().count();
                let to = if put.lines || lines {
                    start
                } else {
                    start + length.saturating_sub(1)
                };
                splice(t, range, &put_text, to, step);
                step.cursor = clamp_in(step, to);
            }
            _ => self.pending = Pending::default(),
        }
    }

    /// A motion in visual mode: the cursor moves, the anchor stays.
    fn moved(&mut self, t: &Text<'_>, target: Target, step: &mut Step) {
        step.cursor = t.clamp(target.to);
        self.column = target.column.map(|column| (step.cursor, column));
    }

    /// The column `j` and `k` keep to from `cursor`, on a line that starts
    /// at `start`.
    fn kept_column(&self, cursor: usize, start: usize) -> usize {
        match self.column {
            Some((at, column)) if at == cursor => column,
            _ => cursor - start,
        }
    }

    /// The motion `key` from `cursor`, where it is one.
    fn motion(&self, t: &Text<'_>, cursor: usize, key: char, pending: &Pending) -> Option<Target> {
        let count = total(pending);
        let (start, end) = (t.line_start(cursor), t.line_end(cursor));
        let along = |to: usize, kind| Target {
            to,
            kind,
            column: Some(to - t.line_start(to)),
        };
        // `cw` on a word changes to its end, not the blanks after it: from
        // the end of this word, as `e` goes on from there.
        let change = matches!(pending.operator, Some((Operator::Change, _)));
        if change && matches!(key, 'w' | 'W') && t.c.get(cursor).is_some_and(|c| !c.is_whitespace())
        {
            let big = key == 'W';
            let own = class(t.c[cursor], big);
            let mut to = cursor;
            while to + 1 < t.len() && class(t.c[to + 1], big) == own {
                to += 1;
            }
            for _ in 1..count {
                to = word_end(t, to, big);
            }
            return Some(along(to, Kind::Inclusive));
        }
        Some(match key {
            'h' => along(cursor.saturating_sub(count).max(start), Kind::Exclusive),
            'l' | ' ' => along((cursor + count).min(end), Kind::Exclusive),
            '0' => along(start, Kind::Exclusive),
            '^' => along(t.first_non_blank(cursor), Kind::Exclusive),
            '$' => {
                let last = (t.line_of(cursor) + count - 1).min(t.lines() - 1);
                let line_end = t.line_end(t.line_at(last));
                Target {
                    to: line_end.saturating_sub(1).max(t.line_at(last)),
                    kind: Kind::Inclusive,
                    column: Some(usize::MAX),
                }
            }
            'j' | 'k' | '+' | '-' => {
                let line = t.line_of(cursor);
                let to_line = if matches!(key, 'j' | '+') {
                    (line + count).min(t.lines() - 1)
                } else {
                    line.saturating_sub(count)
                };
                if to_line == line {
                    return None;
                }
                let first = t.line_at(to_line);
                let to = if matches!(key, '+' | '-') {
                    t.first_non_blank(first)
                } else {
                    let column = self.kept_column(cursor, start);
                    let line_end = t.line_end(first);
                    first
                        .saturating_add(column)
                        .min(line_end.saturating_sub(1).max(first))
                };
                Target {
                    to,
                    kind: Kind::Lines,
                    column: Some(self.kept_column(cursor, start)),
                }
            }
            'G' => {
                let line = pending.count.unwrap_or(t.lines());
                line_target(t, line)
            }
            'w' | 'W' => {
                let (mut to, mut last) = (cursor, cursor);
                for _ in 0..count {
                    let next = word_start(t, to, key == 'W');
                    if next == to {
                        break;
                    }
                    (last, to) = (to, next);
                }
                // After an operator, the last word moved over ends the
                // text at its line's end, not at the next line's first
                // word, as in vim.
                let end = t.line_end(last);
                if pending.operator.is_some() && t.line_of(to) > t.line_of(last) && end > last {
                    to = end;
                }
                along(to.min(t.len()), Kind::Exclusive)
            }
            'e' | 'E' => {
                let mut to = cursor;
                for _ in 0..count {
                    let next = word_end(t, to, key == 'E');
                    if next == to {
                        break;
                    }
                    to = next;
                }
                along(to, Kind::Inclusive)
            }
            'b' | 'B' => {
                let mut to = cursor;
                for _ in 0..count {
                    let next = word_back(t, to, key == 'B');
                    if next == to {
                        break;
                    }
                    to = next;
                }
                along(to, Kind::Exclusive)
            }
            ';' | ',' => {
                let (kind, c) = self.find?;
                let kind = if key == ',' {
                    match kind {
                        'f' => 'F',
                        'F' => 'f',
                        't' => 'T',
                        _ => 't',
                    }
                } else {
                    kind
                };
                return find(t, cursor, kind, c, count, true);
            }
            '%' => along(matching(t, cursor)?, Kind::Inclusive),
            '}' => {
                let mut to = cursor;
                for _ in 0..count {
                    let next = paragraph(t, to, true);
                    if next == to {
                        break;
                    }
                    to = next;
                }
                along(to, Kind::Exclusive)
            }
            '{' => {
                let mut to = cursor;
                for _ in 0..count {
                    let next = paragraph(t, to, false);
                    if next == to {
                        break;
                    }
                    to = next;
                }
                along(to, Kind::Exclusive)
            }
            _ => return None,
        })
    }

    /// A motion typed, alone or after an operator.
    #[allow(clippy::too_many_arguments)] // the key, its script and cursor, and what it acts with
    fn finish_motion(
        &mut self,
        t: &Text<'_>,
        cursor: usize,
        target: Option<Target>,
        pending: Pending,
        register: &mut Register,
        indent: &str,
        step: &mut Step,
    ) {
        let Some(target) = target else {
            return;
        };
        let Some((operator, _)) = pending.operator else {
            step.cursor = t.clamp(target.to);
            self.column = target.column.map(|column| (step.cursor, column));
            return;
        };
        let (low, high) = (cursor.min(target.to), cursor.max(target.to));
        let (range, lines) = match target.kind {
            Kind::Exclusive => {
                // An exclusive motion that ends at the start of a later
                // line stops at the end of the line before, as in vim:
                // `dw` on a line's last word leaves the line break.
                let mut high = high;
                if high > low && t.column(high) == 0 && t.line_of(high) > t.line_of(low) {
                    high -= 1;
                }
                (low..high, false)
            }
            // Up to a line break, not it: `D` on an empty line takes
            // nothing.
            Kind::Inclusive if t.c.get(high) == Some(&'\n') => (low..high, false),
            Kind::Inclusive => (low..(high + 1).min(t.len()), false),
            Kind::Lines => (t.line_start(low)..(t.line_end(high) + 1).min(t.len()), true),
        };
        // A count before `>>` is of lines, which the range has: one shift.
        let count = match operator {
            Operator::Indent | Operator::Outdent => 1,
            _ => total(&pending),
        };
        self.operate(
            t, cursor, operator, range, lines, count, register, indent, step,
        );
    }

    /// `operator` over `range` (whole lines where `lines`).
    #[allow(clippy::too_many_arguments)] // the key, its script and cursor, and what it acts with
    fn operate(
        &mut self,
        t: &Text<'_>,
        cursor: usize,
        operator: Operator,
        range: Range<usize>,
        lines: bool,
        count: usize,
        register: &mut Register,
        indent: &str,
        step: &mut Step,
    ) {
        let first = range.start;
        match operator {
            Operator::Delete => {
                let mut taken = t.slice(range.clone());
                let mut range = range;
                if lines {
                    if !taken.ends_with('\n') {
                        taken.push('\n');
                        // The last line has no break after it: the one
                        // before it goes.
                        if range.start > 0 {
                            range.start -= 1;
                        }
                    }
                } else if range.is_empty() {
                    return;
                }
                *register = Register { text: taken, lines };
                let at = range.start;
                splice(t, range, "", at, step);
                let new: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
                let n = Text { c: &new };
                step.cursor = if lines {
                    n.first_non_blank(n.line_start(first.min(n.len())))
                } else {
                    n.clamp(at)
                };
            }
            Operator::Change => {
                self.mode = Mode::Insert;
                if lines {
                    // The lines' text goes; the first line's indent stays.
                    let mut taken = t.slice(range.clone());
                    if !taken.ends_with('\n') {
                        taken.push('\n');
                    }
                    *register = Register {
                        text: taken,
                        lines: true,
                    };
                    let lead: String = t.c[range.start..t.first_non_blank(range.start)]
                        .iter()
                        .collect();
                    let end = t.line_end(range.end.saturating_sub(1).max(range.start));
                    let to = range.start + lead.chars().count();
                    splice(t, range.start..end, &lead, to, step);
                } else {
                    *register = Register {
                        text: t.slice(range.clone()),
                        lines: false,
                    };
                    splice(t, range.clone(), "", range.start, step);
                }
            }
            Operator::Yank => {
                let mut taken = t.slice(range.clone());
                if lines && !taken.ends_with('\n') {
                    taken.push('\n');
                }
                *register = Register { text: taken, lines };
                step.cursor = if lines {
                    if t.line_of(cursor) == t.line_of(first) {
                        cursor
                    } else {
                        t.clamp(first + t.column(cursor).min(t.line_end(first) - first))
                    }
                } else {
                    t.clamp(first)
                };
            }
            Operator::Indent | Operator::Outdent => {
                let (first_line, last_line) = (
                    t.line_of(range.start),
                    t.line_of(range.end.saturating_sub(1).max(range.start)),
                );
                let mut out = String::new();
                for line in 0..t.lines() {
                    let (s, e) = (t.line_at(line), t.line_end(t.line_at(line)));
                    let body: String = t.c[s..e].iter().collect();
                    if (first_line..=last_line).contains(&line) && !body.trim().is_empty() {
                        if operator == Operator::Indent {
                            for _ in 0..count.clamp(1, SHIFTS) {
                                out.push_str(indent);
                            }
                            out.push_str(&body);
                        } else {
                            out.push_str(outdent(&body, indent, count.max(1)));
                        }
                    } else {
                        out.push_str(&body);
                    }
                    if e < t.len() {
                        out.push('\n');
                    }
                }
                let new: Vec<char> = out.chars().collect();
                let n = Text { c: &new };
                let to = n.first_non_blank(n.line_at(first_line));
                if out != t.slice(0..t.len()) {
                    step.text = Some(out);
                }
                step.cursor = to;
            }
            Operator::Format => {
                let start = t.line_start(range.start);
                let end = t.line_end(range.end.saturating_sub(1).max(range.start));
                // An empty line holds no statement to format.
                if start < end {
                    step.then = Some(Then::Format(start..end));
                }
                step.cursor = t.first_non_blank(start);
            }
        }
    }
}

/// The count of an operator and its motion: `2d3w` deletes six words.
fn total(pending: &Pending) -> usize {
    let before = pending.operator.map_or(1, |(_, count)| count);
    before
        .saturating_mul(pending.count.unwrap_or(1))
        .clamp(1, COUNTS)
}

/// The largest count: one typed, or an operator's times its motion's.
const COUNTS: usize = 100_000;

/// The most text one `p` puts in: its count gives way to it.
const PUT_BYTES: usize = 16 << 20;

/// The most indents one shift puts in.
const SHIFTS: usize = 100;

/// Line `line` (from 1, past the last is the last) as a motion to its
/// first character that is not blank.
fn line_target(t: &Text<'_>, line: usize) -> Target {
    let line = line.clamp(1, t.lines()) - 1;
    Target {
        to: t.first_non_blank(t.line_at(line)),
        kind: Kind::Lines,
        column: None,
    }
}

/// Replaces `range` of the script with `with`, the cursor then at `to`.
fn splice(t: &Text<'_>, range: Range<usize>, with: &str, to: usize, step: &mut Step) {
    let mut out: String = t.c[..range.start].iter().collect();
    out.push_str(with);
    out.extend(&t.c[range.end..]);
    step.text = Some(out);
    step.cursor = to;
}

/// `at` clamped to normal mode's places in the step's new script.
fn clamp_in(step: &Step, at: usize) -> usize {
    let chars: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
    Text { c: &chars }.clamp(at)
}

/// `r`: `count` characters from the cursor become `c`, where the line has
/// them.
fn replace(t: &Text<'_>, cursor: usize, c: char, count: usize, step: &mut Step) {
    let end = t.line_end(cursor);
    if cursor + count > end {
        return;
    }
    if c == '\n' {
        // The characters become one line break, as in vim.
        splice(t, cursor..cursor + count, "\n", cursor + 1, step);
        return;
    }
    let with: String = std::iter::repeat_n(c, count).collect();
    splice(t, cursor..cursor + count, &with, cursor + count - 1, step);
}

/// `p` (after the cursor) or `P` (before it), `count` times.
fn put(
    t: &Text<'_>,
    cursor: usize,
    after: bool,
    count: usize,
    register: &Register,
    step: &mut Step,
) {
    if register.text.is_empty() {
        return;
    }
    let count = count.clamp(1, (PUT_BYTES / register.text.len()).max(1));
    let text = register.text.repeat(count);
    if register.lines {
        if after {
            let end = t.line_end(cursor);
            if end >= t.len() {
                // The last line: a break before the lines, none after.
                let lines = text.strip_suffix('\n').unwrap_or(&text);
                splice(t, end..end, &format!("\n{lines}"), end + 1, step);
            } else {
                splice(t, end + 1..end + 1, &text, end + 1, step);
            }
        } else {
            let start = t.line_start(cursor);
            splice(t, start..start, &text, start, step);
        }
        let chars: Vec<char> = step.text.as_deref().unwrap_or("").chars().collect();
        step.cursor = Text { c: &chars }.first_non_blank(step.cursor);
    } else {
        let empty = t.line_start(cursor) == t.line_end(cursor);
        let at = if after && !empty { cursor + 1 } else { cursor };
        let length = text.chars().count();
        // On the last character put; at its start where it is of lines.
        let to = if text.contains('\n') {
            at
        } else {
            at + length - 1
        };
        splice(t, at..at, &text, to, step);
    }
}

/// `J`: `joins` line breaks from line `line` on go, each with the indent
/// after it, and a space goes between where neither side has one.
fn join(t: &Text<'_>, line: usize, joins: usize, step: &mut Step) {
    let mut chars: Vec<char> = t.c.to_vec();
    let mut at = None;
    for _ in 0..joins {
        let n = Text { c: &chars };
        let end = n.line_end(n.line_at(line));
        if end >= n.len() {
            break;
        }
        let mut next = end + 1;
        while next < n.len() && matches!(n.c[next], ' ' | '\t') {
            next += 1;
        }
        let space = end > n.line_at(line)
            && !n.c[end - 1].is_whitespace()
            && next < n.len()
            && n.c[next] != '\n'
            && n.c[next] != ')';
        chars.splice(end..next, if space { vec![' '] } else { vec![] });
        at = Some(end);
    }
    if let Some(at) = at {
        let text: String = chars.iter().collect();
        step.text = Some(text);
        step.cursor = Text { c: &chars }.clamp(at);
    }
}

/// `f`, `F`, `t` or `T` to the `count`th `c` on the line; `repeat` for
/// `;` and `,`, where a `t` does not stop where it stands.
fn find(
    t: &Text<'_>,
    cursor: usize,
    kind: char,
    c: char,
    count: usize,
    repeat: bool,
) -> Option<Target> {
    let (start, end) = (t.line_start(cursor), t.line_end(cursor));
    let forward = matches!(kind, 'f' | 't');
    let mut at = cursor;
    for n in 0..count {
        // A repeated `t` starts past the character it stopped before.
        let skip = if repeat && n == 0 && matches!(kind, 't' | 'T') {
            1
        } else {
            0
        };
        let found = if forward {
            (at + 1 + skip..end).find(|&i| t.c[i] == c)
        } else {
            (start..at.saturating_sub(skip))
                .rev()
                .find(|&i| t.c[i] == c)
        };
        at = found?;
    }
    let to = match kind {
        't' => at - 1,
        'T' => at + 1,
        _ => at,
    };
    Some(Target {
        to,
        kind: if forward {
            Kind::Inclusive
        } else {
            Kind::Exclusive
        },
        column: Some(to - start),
    })
}

/// `%`: the bracket that matches the first one at or after the cursor on
/// its line.
fn matching(t: &Text<'_>, cursor: usize) -> Option<usize> {
    let end = t.line_end(cursor);
    let at = (cursor..end).find(|&i| "()[]{}".contains(t.c[i]))?;
    let (open, close, forward) = match t.c[at] {
        '(' => ('(', ')', true),
        '[' => ('[', ']', true),
        '{' => ('{', '}', true),
        ')' => ('(', ')', false),
        ']' => ('[', ']', false),
        _ => ('{', '}', false),
    };
    let mut depth = 0usize;
    let mut i = at;
    loop {
        let c = t.c[i];
        if c == open {
            if forward {
                depth += 1;
            } else {
                depth -= 1;
            }
        } else if c == close {
            if forward {
                depth -= 1;
            } else {
                depth += 1;
            }
        }
        if depth == 0 {
            return Some(i);
        }
        if forward {
            i += 1;
            if i >= t.len() {
                return None;
            }
        } else {
            i = i.checked_sub(1)?;
        }
    }
}

/// `}` or `{`: the next or last empty line, or the script's end or start.
fn paragraph(t: &Text<'_>, cursor: usize, forward: bool) -> usize {
    let blank = |line: usize| {
        let s = t.line_at(line);
        t.c[s..t.line_end(s)].iter().all(|c| c.is_whitespace())
    };
    let line = t.line_of(cursor);
    if forward {
        // Past the empty lines the cursor is in, to the next one.
        let mut l = line;
        while l < t.lines() && blank(l) {
            l += 1;
        }
        while l < t.lines() && !blank(l) {
            l += 1;
        }
        if l >= t.lines() {
            // The script's end: a motion's cursor lands on its last
            // character, and an operator takes it.
            t.len()
        } else {
            t.line_at(l)
        }
    } else {
        let mut l = line;
        while l > 0 && blank(l) {
            l -= 1;
        }
        while l > 0 && !blank(l) {
            l -= 1;
        }
        t.line_at(l)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Class {
    Blank,
    Word,
    Other,
}

/// A character's class for the word motions: with `big`, all that is not
/// blank is one.
fn class(c: char, big: bool) -> Class {
    if c.is_whitespace() {
        Class::Blank
    } else if big || c.is_alphanumeric() || c == '_' {
        Class::Word
    } else {
        Class::Other
    }
}

/// Whether `at` is an empty line.
fn empty_line(t: &Text<'_>, at: usize) -> bool {
    t.c.get(at) == Some(&'\n') && (at == 0 || t.c[at - 1] == '\n')
}

/// `w`: the next word's start; an empty line is a word.
fn word_start(t: &Text<'_>, from: usize, big: bool) -> usize {
    let n = t.len();
    if from >= n {
        return n;
    }
    let mut i = from;
    let own = class(t.c[i], big);
    if own != Class::Blank {
        while i < n && class(t.c[i], big) == own {
            i += 1;
        }
    }
    while i < n && t.c[i].is_whitespace() {
        if i > from && empty_line(t, i) {
            return i;
        }
        i += 1;
    }
    i
}

/// `e`: the end of this word, or of the next where the cursor is at one.
fn word_end(t: &Text<'_>, from: usize, big: bool) -> usize {
    let n = t.len();
    let mut i = from + 1;
    while i < n && t.c[i].is_whitespace() {
        i += 1;
    }
    if i >= n {
        return n.saturating_sub(1).max(from.min(n.saturating_sub(1)));
    }
    let own = class(t.c[i], big);
    while i + 1 < n && class(t.c[i + 1], big) == own {
        i += 1;
    }
    i
}

/// `b`: the start of this word, or of the one before where the cursor is
/// at one; an empty line is a word.
fn word_back(t: &Text<'_>, from: usize, big: bool) -> usize {
    if from == 0 {
        return 0;
    }
    let mut i = from - 1;
    while i > 0 && t.c[i].is_whitespace() {
        if empty_line(t, i) {
            return i;
        }
        i -= 1;
    }
    let own = class(t.c[i], big);
    while i > 0 && class(t.c[i - 1], big) == own && own != Class::Blank {
        i -= 1;
    }
    i
}

/// `~`'s case change.
fn flip(c: char) -> char {
    if c.is_lowercase() {
        c.to_uppercase().next().unwrap_or(c)
    } else {
        c.to_lowercase().next().unwrap_or(c)
    }
}

/// `body` with `count` indents taken off its start, or what blanks it
/// has, where it has fewer.
fn outdent<'a>(body: &'a str, indent: &str, count: usize) -> &'a str {
    let mut rest = body;
    for _ in 0..count.min(SHIFTS) {
        if let Some(after) = rest.strip_prefix(indent) {
            rest = after;
        } else if let Some(after) = rest.strip_prefix('\t') {
            rest = after;
        } else {
            let width = indent.chars().count().max(1);
            let blanks = rest.chars().take(width).take_while(|c| *c == ' ').count();
            rest = &rest[blanks..];
        }
    }
    rest
}

/// The cursor that normal mode allows at `at` in `text`: on a character,
/// not after a line's last (an empty line has only its start).
pub fn clamp(text: &str, at: usize) -> usize {
    let chars: Vec<char> = text.chars().collect();
    Text { c: &chars }.clamp(at)
}

/// A script as the motions read it: characters and lines.
struct Text<'a> {
    c: &'a [char],
}

impl Text<'_> {
    fn len(&self) -> usize {
        self.c.len()
    }

    fn line_start(&self, at: usize) -> usize {
        let mut i = at.min(self.len());
        while i > 0 && self.c[i - 1] != '\n' {
            i -= 1;
        }
        i
    }

    fn line_end(&self, at: usize) -> usize {
        let mut i = at.min(self.len());
        while i < self.len() && self.c[i] != '\n' {
            i += 1;
        }
        i
    }

    fn column(&self, at: usize) -> usize {
        at - self.line_start(at)
    }

    fn first_non_blank(&self, at: usize) -> usize {
        let (start, end) = (self.line_start(at), self.line_end(at));
        let mut i = start;
        while i < end && matches!(self.c[i], ' ' | '\t') {
            i += 1;
        }
        // A line of blanks: its last.
        if i == end && end > start { end - 1 } else { i }
    }

    /// The line `at` is on, from 0.
    fn line_of(&self, at: usize) -> usize {
        self.c[..at.min(self.len())]
            .iter()
            .filter(|c| **c == '\n')
            .count()
    }

    fn lines(&self) -> usize {
        self.c.iter().filter(|c| **c == '\n').count() + 1
    }

    /// The start of line `line` (from 0).
    fn line_at(&self, line: usize) -> usize {
        if line == 0 {
            return 0;
        }
        let mut seen = 0;
        for (i, c) in self.c.iter().enumerate() {
            if *c == '\n' {
                seen += 1;
                if seen == line {
                    return i + 1;
                }
            }
        }
        self.line_start(self.len())
    }

    fn clamp(&self, at: usize) -> usize {
        let at = at.min(self.len());
        let (start, end) = (self.line_start(at), self.line_end(at));
        if end == start { start } else { at.min(end - 1) }
    }

    fn slice(&self, range: Range<usize>) -> String {
        self.c[range].iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Feeds `keys` to vim in normal mode over `text`, whose `|` marks the
    /// cursor, and gives the script back with the cursor marked, the mode
    /// and the last `Then`. In `keys`, `<esc>`, `<cr>`, `<bs>` and `<c-r>`
    /// name keys.
    fn run(text: &str, keys: &str) -> (String, Mode, Option<Then>) {
        let (text, _, mode, then) = run_with(text, keys, Register::default());
        (text, mode, then)
    }

    fn run_with(
        text: &str,
        keys: &str,
        mut register: Register,
    ) -> (String, Register, Mode, Option<Then>) {
        let at = text.chars().position(|c| c == '|').expect("a cursor");
        let mut script: String = text.chars().filter(|c| *c != '|').collect();
        let mut cursor = at;
        let mut vim = Vim::default();
        vim.start(&script);
        let mut then = None;
        let mut rest = keys;
        while !rest.is_empty() {
            let (input, len) = [
                ("<esc>", Input::Escape),
                ("<cr>", Input::Enter),
                ("<bs>", Input::Backspace),
                ("<c-r>", Input::Redo),
            ]
            .iter()
            .find(|(name, _)| rest.starts_with(name))
            .map(|(name, input)| (*input, name.len()))
            .unwrap_or_else(|| {
                let c = rest.chars().next().unwrap();
                (Input::Char(c), c.len_utf8())
            });
            rest = &rest[len..];
            if vim.mode == Mode::Insert && input != Input::Escape {
                // Insert mode's typing is the field's.
                if let Input::Char(c) = input {
                    let byte = script
                        .char_indices()
                        .nth(cursor)
                        .map_or(script.len(), |(b, _)| b);
                    script.insert(byte, c);
                    cursor += 1;
                }
                continue;
            }
            let step = vim.key(&script, cursor, input, &mut register, "  ");
            if let Some(text) = step.text {
                script = text;
            }
            cursor = step.cursor;
            if step.then.is_some() {
                then = step.then;
            }
        }
        let byte = script
            .char_indices()
            .nth(cursor)
            .map_or(script.len(), |(b, _)| b);
        script.insert(byte, '|');
        (script, register, vim.mode, then)
    }

    fn text(text: &str, keys: &str) -> String {
        run(text, keys).0
    }

    #[test]
    fn the_motions_move_along_a_line_and_across_lines() {
        assert_eq!(text("|select id from books", "w"), "select |id from books");
        assert_eq!(text("|select id from books", "3w"), "select id from |books");
        assert_eq!(text("select id from |books", "b"), "select id |from books");
        assert_eq!(text("|select id", "e"), "selec|t id");
        assert_eq!(text("|count(*) as n", "w"), "count|(*) as n");
        assert_eq!(text("|count(*) as n", "W"), "count(*) |as n");
        assert_eq!(text("se|lect", "$"), "selec|t");
        assert_eq!(text("  se|lect", "0"), "|  select");
        assert_eq!(text("  se|lect", "^"), "  |select");
        assert_eq!(text("|ab", "l"), "a|b");
        assert_eq!(text("a|b", "l"), "a|b", "not past the last character");
        assert_eq!(text("|a\nb\nc", "j"), "a\n|b\nc");
        assert_eq!(text("|a\nb\nc", "G"), "a\nb\n|c");
        assert_eq!(text("a\nb\n|c", "gg"), "|a\nb\nc");
        assert_eq!(text("a\nb\n|c", "2G"), "a\n|b\nc");
        // j and k keep the column, and `$` the end.
        assert_eq!(text("abc|d\nx\nabcdef", "jj"), "abcd\nx\nabc|def");
        assert_eq!(text("ab|c\nabcdef", "$j"), "abc\nabcde|f");
        // A word across a line break, and an empty line is a word.
        assert_eq!(text("|select\nfrom", "w"), "select\n|from");
        assert_eq!(text("|a\n\nb", "w"), "a\n|\nb");
        assert_eq!(text("a\n\n|b", "b"), "a\n|\nb");
    }

    #[test]
    fn find_and_match_jump_on_the_line() {
        assert_eq!(
            text("|where a = 1 and b = 2", "f="),
            "where a |= 1 and b = 2"
        );
        assert_eq!(
            text("|where a = 1 and b = 2", "2f="),
            "where a = 1 and b |= 2"
        );
        assert_eq!(
            text("|where a = 1 and b = 2", "f=;"),
            "where a = 1 and b |= 2"
        );
        assert_eq!(text("|where a = 1", "t="), "where a| = 1");
        assert_eq!(text("where a = |1", "F="), "where a |= 1");
        assert_eq!(text("|count(max(a))", "%"), "count(max(a)|)");
        assert_eq!(text("count(max(a)|)", "%"), "count|(max(a))");
    }

    #[test]
    fn delete_change_and_yank_take_what_their_motion_covers() {
        assert_eq!(text("|select id from books", "dw"), "|id from books");
        assert_eq!(text("select |id, title", "d2w"), "select |title");
        assert_eq!(text("select |id, title", "2dw"), "select |title");
        assert_eq!(
            text("select |id\nfrom", "dw"),
            "select| \nfrom",
            "the line break stays"
        );
        assert_eq!(text("select |id from", "d$"), "select| ");
        assert_eq!(text("select |id from", "D"), "select| ");
        assert_eq!(text("where a |= 1", "dt1"), "where a |1");
        assert_eq!(text("where a |= 1", "df1"), "where a| ");
        // `cw` is `ce` on a word.
        assert_eq!(
            run("select |id from", "cwname<esc>"),
            ("select nam|e from".into(), Mode::Normal, None)
        );
        assert_eq!(text("select |id from", "c$*<esc>"), "select |*");
        assert_eq!(text("|abc", "x"), "|bc");
        assert_eq!(text("ab|c", "x"), "a|b");
        assert_eq!(text("ab|c", "X"), "a|c");
        assert_eq!(text("|abc", "3x"), "|");
        assert_eq!(text("|abc", "sz<esc>"), "|zbc");
        // `cw` on a word of one character changes it alone.
        assert_eq!(text("select |2;", "cw3<esc>"), "select |3;");
        assert_eq!(text("|a b c", "c2wx<esc>"), "|x c");
    }

    #[test]
    fn a_doubled_operator_acts_on_whole_lines() {
        assert_eq!(text("a\n|b\nc", "dd"), "a\n|c");
        assert_eq!(
            text("a\nb\n|c", "dd"),
            "a\n|b",
            "the last line takes the break before it"
        );
        assert_eq!(text("|a\nb\nc", "2dd"), "|c");
        assert_eq!(text("|a\nb\nc", "dj"), "|c");
        assert_eq!(text("a\nb\n|c", "dk"), "|a");
        assert_eq!(text("  |a\nb", "ccx<esc>"), "  |x\nb", "the indent stays");
        assert_eq!(text("|a\nb", ">>"), "  |a\nb");
        assert_eq!(text("|a\nb", ">j"), "  |a\n  b");
        assert_eq!(text("    |a", "<<"), "  |a");
        assert_eq!(text("\t|a", "<<"), "|a");
    }

    #[test]
    fn a_yank_or_a_delete_puts_back_where_p_says() {
        assert_eq!(text("|a\nb", "yyp"), "a\n|a\nb");
        assert_eq!(text("a\n|b", "yyp"), "a\nb\n|b", "after the last line");
        assert_eq!(text("a\n|b", "yyP"), "a\n|b\nb");
        assert_eq!(text("|a\nb", "ddp"), "b\n|a");
        assert_eq!(text("|select id", "yw$p"), "select idselect| ");
        assert_eq!(text("|ab", "xp"), "b|a");
        assert_eq!(text("|ab", "x3p"), "baa|a");
        assert_eq!(text("|ab", "yl$P"), "a|ab");
    }

    #[test]
    fn insert_mode_is_entered_where_each_key_says_and_left_with_esc() {
        assert_eq!(run("se|lect", "i"), ("se|lect".into(), Mode::Insert, None));
        assert_eq!(
            text("se|lect", "ix<esc>"),
            "se|xlect",
            "Esc steps back over it"
        );
        assert_eq!(text("se|lect", "ax<esc>"), "sel|xect");
        assert_eq!(text("  se|lect", "Ix<esc>"), "  |xselect");
        assert_eq!(text("se|lect", "Ax<esc>"), "select|x");
        assert_eq!(text("  se|lect\nfrom", "ox<esc>"), "  select\n  |x\nfrom");
        assert_eq!(text("se|lect", "Ox<esc>"), "|x\nselect");
        assert_eq!(text("|", "ix<esc>"), "|x");
        assert_eq!(text("|", "i<esc>"), "|");
    }

    #[test]
    fn join_replace_and_case_change_the_text_in_place() {
        assert_eq!(text("|select\n  id", "J"), "select| id");
        assert_eq!(text("|a\nb\nc", "3J"), "a b| c");
        assert_eq!(text("|count(\n  )", "J"), "count(|)");
        assert_eq!(text("|abc", "rx"), "|xbc");
        assert_eq!(text("|abc", "2rx"), "x|xc");
        assert_eq!(text("|abc", "5rx"), "|abc", "not past the line");
        assert_eq!(text("|select", "3~"), "SEL|ect");
    }

    #[test]
    fn visual_mode_selects_both_ends_and_operates_on_them() {
        let (_, mode, _) = run("|select", "v");
        assert_eq!(mode, Mode::Visual);
        assert_eq!(text("|select id", "vd"), "|elect id");
        assert_eq!(text("|select id", "ved"), "| id");
        assert_eq!(text("|select id", "veyP"), "selec|tselect id");
        assert_eq!(text("select |id", "vbd"), "|d");
        assert_eq!(text("a\n|b\nc", "Vd"), "a\n|c");
        assert_eq!(text("|a\nb\nc", "Vjd"), "|c");
        assert_eq!(text("|a\nb", "Vj>"), "  |a\n  b");
        assert_eq!(text("|select id", "veU"), "|SELECT id");
        assert_eq!(text("|select id", "vecx<esc>"), "|x id");
        assert_eq!(text("|a\nb", "VjJ"), "a| b");
        // `o` moves the other end; Esc and v leave.
        assert_eq!(text("se|lect", "vlohd"), "s|ct");
        assert_eq!(run("|select", "v<esc>").1, Mode::Normal);
        assert_eq!(run("|select", "vv").1, Mode::Normal);
        assert_eq!(run("|select", "vV").1, Mode::VisualLine);
        // What is selected is replaced by what was yanked.
        assert_eq!(text("|ab cd", "yewvep"), "ab a|b");
    }

    #[test]
    fn the_editor_is_asked_for_undo_redo_format_the_prompt_and_leaving() {
        assert_eq!(run("|a", ":").2, Some(Then::Prompt));
        assert_eq!(run("|a", "<esc>").2, Some(Then::Leave));
        // Esc ends a command typed in part, and the keyboard stays.
        assert_eq!(run("|a", "d<esc>").2, None);
        assert_eq!(run("|a", "2<esc>").2, None);
        // `==` the cursor's line, `gg=G` every line, `=j` two.
        assert_eq!(run("a\n|bc\nd", "==").2, Some(Then::Format(2..4)));
        assert_eq!(run("a\n|bc\nd", "gg=G").2, Some(Then::Format(0..6)));
        assert_eq!(run("a\n|bc\nd", "=j").2, Some(Then::Format(2..6)));
        assert_eq!(run("|a\nbc\nd", "Vj=").2, Some(Then::Format(0..4)));
    }

    #[test]
    fn counts_multiply_and_an_unknown_key_drops_the_command() {
        assert_eq!(text("|a b c d e f g", "2d2w"), "|e f g");
        assert_eq!(
            text("|a b c d e f g e f g h i j k", "10w"),
            "a b c d e f g e f g |h i j k"
        );
        assert_eq!(text("|a b", "dqw"), "a |b", "`q` dropped the `d`");
        assert_eq!(text("|", "x"), "|");
        assert_eq!(text("|", "dd"), "|");
        assert_eq!(text("|", "p"), "|");
    }

    #[test]
    fn the_edges_a_reviewer_found_are_as_vim_has_them() {
        // `t` and `T` find a character right beside the cursor; `;` does
        // not stop where `t` stood.
        assert_eq!(text("|a=b=c", "dt="), "|=b=c");
        assert_eq!(text("count(|a)", "ct)x<esc>"), "count(|x)");
        assert_eq!(text("|a=b=c", "t=;"), "a=|b=c");
        // `dw` and `yw` on a line's last word stop at its end.
        assert_eq!(text("a |b\n  c", "dw"), "a| \n  c");
        let (_, register, _, _) = run_with("a |b\n  c", "yw", Register::default());
        assert_eq!(register.text, "b");
        // `D` and `C` on an empty line keep its break.
        assert_eq!(text("a\n|\nb", "D"), "a\n|\nb");
        assert_eq!(text("a\n|\nb", "Cx<esc>"), "a\n|x\nb");
        // A count before `>>` is of lines.
        assert_eq!(text("|a\nb\nc", "2>>"), "  |a\n  b\nc");
        assert_eq!(text("    |a\n    b\nc", "2<<"), "  |a\n  b\nc");
        // `j` after an insert starts from where the insert left it.
        assert_eq!(
            text("|abcdefghij\nabcdefghij", "llAx<esc>j"),
            "abcdefghijx\nabcdefghi|j"
        );
        assert_eq!(text("|a\nb", "yyjVp"), "a\n|a");
        assert_eq!(text("|a b", "d}"), "|");
        assert_eq!(text("|a b", "}"), "a |b");
        assert_eq!(text("|ab", "ylXp"), "a|ab");
        assert_eq!(text("a|bc", "r<cr>"), "a\n|c");
        assert_eq!(text("|  \nb", "Ix<esc>"), "  |x\nb");
        let register = Register {
            text: "x\ny".into(),
            lines: false,
        };
        assert_eq!(run_with("|ab", "p", register).0, "a|x\nyb");
    }

    #[test]
    fn undo_takes_back_a_change_or_an_insert_whole_and_redo_survives_moves() {
        // A command each, and counts.
        assert_eq!(text("|a b c", "xxu"), "| b c");
        assert_eq!(text("|a b c", "xx2u"), "|a b c");
        assert_eq!(text("|a b c", "xxuu<c-r>"), "| b c");
        assert_eq!(text("|a b c", "xx2u2<c-r>"), "|b c");
        // An insert, with the change that started it, is one step.
        assert_eq!(text("|a b", "cwxyz<esc>u"), "|a b");
        assert_eq!(text("|a", "oselect 1<esc>u"), "|a");
        // Moves are no steps, and leave the redo.
        assert_eq!(text("|ab\ncd", "xujl<c-r>"), "|b\ncd");
        // Nothing to undo: nothing changes.
        assert_eq!(text("a|b", "u"), "a|b");
        // A new change leaves nothing to redo.
        assert_eq!(text("|abc", "xu~<c-r>"), "A|bc");
        // `==` on an empty line formats nothing.
        assert_eq!(run("a\n|\nb", "==").2, None);
    }

    #[test]
    fn large_counts_are_cut_to_what_can_be_done_at_once() {
        // An operator's count times its motion's: capped, and the moves
        // stop at the script's end.
        let started = std::time::Instant::now();
        assert_eq!(text("|a b c", "99999d99999w"), "|");
        assert_eq!(text("a b |c", "99999b"), "|a b c");
        assert!(started.elapsed() < std::time::Duration::from_secs(2));
        // A put of a large yank, many times: no more than its bytes.
        let register = Register {
            text: "x".repeat(1 << 20),
            lines: false,
        };
        let (put, _, _, _) = run_with("|a", "99999p", register);
        assert!(put.len() <= PUT_BYTES + 2, "{}", put.len());
        // A shift: no more than its indents.
        assert_eq!(text("|a", "99999>>").len(), "  |a".len());
        let shifted = text("|a", "v99999>");
        assert_eq!(shifted.trim_start().len(), "|a".len());
        assert!(shifted.len() <= SHIFTS * 2 + 2, "{}", shifted.len());
    }

    #[test]
    fn the_history_starts_at_the_script_first_shown() {
        // Opened with a query: Esc and `u` find nothing older.
        let mut vim = Vim::new(Mode::Insert);
        vim.start("select 1");
        let mut register = Register::default();
        let step = vim.key("select 1", 8, Input::Escape, &mut register, "  ");
        let step = vim.key(
            "select 1",
            step.cursor,
            Input::Char('u'),
            &mut register,
            "  ",
        );
        assert_eq!(step.text, None);
        // Typed after it opened: `u` takes the typing out, not the query.
        let mut vim = Vim::new(Mode::Insert);
        vim.start("select 1");
        vim.start("select 12");
        let step = vim.key("select 12", 9, Input::Escape, &mut register, "  ");
        let step = vim.key(
            "select 12",
            step.cursor,
            Input::Char('u'),
            &mut register,
            "  ",
        );
        assert_eq!(step.text.as_deref(), Some("select 1"));
    }

    #[test]
    fn the_history_keeps_a_hundred_steps_and_no_more_than_its_bytes() {
        let mut history = History::default();
        for n in 0..150 {
            history.push((n.to_string(), 0));
        }
        assert_eq!(history.undo.len(), History::KEPT);
        assert_eq!(history.undo[0].0, "50", "the oldest go first");
        // Large scripts: as many as the bytes allow, and the latest always.
        let large = "x".repeat(History::BYTES / 3);
        for _ in 0..5 {
            history.push((large.clone(), 0));
        }
        let bytes: usize = history.undo.iter().map(|(text, _)| text.len()).sum();
        assert!(bytes <= History::BYTES, "{bytes}");
        assert_eq!(
            history.undo.last().map(|(text, _)| text.len()),
            Some(large.len())
        );
        let mut history = History::default();
        history.push(("x".repeat(History::BYTES + 1), 0));
        assert_eq!(history.undo.len(), 1);
    }

    #[test]
    fn normal_mode_keeps_the_cursor_on_a_character() {
        assert_eq!(clamp("abc", 3), 2);
        assert_eq!(clamp("abc\n", 3), 2);
        assert_eq!(clamp("abc\n", 4), 4, "the empty last line");
        assert_eq!(clamp("", 0), 0);
    }

    #[test]
    fn the_register_holds_lines_or_characters() {
        let (_, register, _, _) = run_with("|a\nb", "yy", Register::default());
        assert_eq!(
            register,
            Register {
                text: "a\n".into(),
                lines: true
            }
        );
        let (_, register, _, _) = run_with("a\n|b", "dd", Register::default());
        assert_eq!(
            register,
            Register {
                text: "b\n".into(),
                lines: true
            }
        );
        let (_, register, _, _) = run_with("|ab", "yl", Register::default());
        assert_eq!(
            register,
            Register {
                text: "a".into(),
                lines: false
            }
        );
    }
}
