//! The person's keys in `settings.toml`: a table for each scope that holds
//! a change (`[keys.grid]`, `"D" = "delete_row"`), and `[keys.unbind]`
//! (`grid = ["gd"]`) for the default keys that do nothing. Only changes
//! from the keymap's defaults live here: the keymap itself is in code.
//!
//! What is read here is what a line says: a table this version names, a
//! key that spells, a command it knows. Whether the line can be laid over
//! the keymap (its row, its kind, a key another row has) depends on the
//! look, and is the keymap's to say ([`crate::keymap::Keymap::with`]): the
//! app marks those lines too.

use toml::de::{DeTable, DeValue};

use super::{
    HEADER, MAX_BYTES, MAX_LINES, comment_after, is_subtable, item_at, line_end, line_of,
    line_start, own_lines, readable, without_empty_table,
};
use crate::keymap::{Bind, Command, Line, Overrides, Scope, Unbind, intern, spell};

/// The table of the default keys that do nothing.
const UNBIND: &str = "unbind";

/// What the keys' tables of a text say.
#[derive(Clone, Debug, Default, PartialEq)]
pub(super) struct Read {
    pub overrides: Overrides,
    /// The line each of them is on, counted from 1.
    pub lines: Vec<(Line, usize)>,
    /// The lines that say nothing this version can read, counted from 1.
    pub invalid: Vec<usize>,
}

/// A table's name, as the file has it.
fn name<'a>(key: &'a toml::Spanned<toml::de::DeString<'_>>) -> &'a str {
    key.get_ref().as_ref()
}

/// The keys `keys` of `[keys.<table>]` would give: the scope the table
/// names, and a key that spells.
fn spelled(keys: &str) -> Option<&'static str> {
    let keys = intern(keys);
    spell(keys).is_ok().then_some(keys)
}

/// Reads the keys' tables of `document`, the parse of `working` (the
/// file's text with its rejected lines blanked).
pub(super) fn read(document: &DeTable<'_>, working: &str) -> Read {
    let mut read = Read::default();
    let Some(keys) = document.get("keys") else {
        return read;
    };
    let line = |span: std::ops::Range<usize>| line_of(working, span.start);
    let Some(tables) = keys.get_ref().as_table() else {
        read.invalid.push(line(keys.span()));
        return read;
    };
    for (key, item) in tables {
        let table = name(key);
        if table == UNBIND {
            read_unbinds(item, working, &mut read);
            continue;
        }
        let Some(scope) = Scope::from_table(table) else {
            read.invalid.push(line(key.span()));
            continue;
        };
        let Some(items) = item.get_ref().as_table() else {
            read.invalid.push(line(item.span()));
            continue;
        };
        let table = intern(table);
        for (keys, value) in items {
            let at = line(keys.span());
            let command = value.get_ref().as_str().and_then(Command::from_id);
            let (Some(spelled), Some(command)) = (spelled(name(keys)), command) else {
                read.invalid.push(at);
                continue;
            };
            let note = comment_after(working, value.span().end);
            read.lines
                .push((Line::Bind(read.overrides.binds.len()), at));
            read.overrides.binds.push(Bind {
                scope,
                table,
                keys: spelled,
                command,
                note: (!note.is_empty()).then_some(note),
            });
        }
    }
    // In the order the file has them: the parser's tables keep theirs by
    // name.
    let mut binds: Vec<(Bind, usize)> = std::mem::take(&mut read.overrides.binds)
        .into_iter()
        .zip(read.lines.iter().filter_map(|(line, at)| match line {
            Line::Bind(_) => Some(*at),
            Line::Unbind(_) => None,
        }))
        .collect();
    let mut unbinds: Vec<(Unbind, usize)> = std::mem::take(&mut read.overrides.unbinds)
        .into_iter()
        .zip(read.lines.iter().filter_map(|(line, at)| match line {
            Line::Unbind(_) => Some(*at),
            Line::Bind(_) => None,
        }))
        .collect();
    binds.sort_by_key(|(_, at)| *at);
    unbinds.sort_by_key(|(_, at)| *at);
    read.lines.clear();
    for (index, (bind, at)) in binds.into_iter().enumerate() {
        read.lines.push((Line::Bind(index), at));
        read.overrides.binds.push(bind);
    }
    for (index, (unbind, at)) in unbinds.into_iter().enumerate() {
        read.lines.push((Line::Unbind(index), at));
        read.overrides.unbinds.push(unbind);
    }
    read.invalid.sort_unstable();
    read.invalid.dedup();
    read
}

/// Reads `[keys.unbind]`: for each scope, the default keys that do nothing.
/// A key that does not spell is ignored by its line, and the others of its
/// list stand.
fn read_unbinds(item: &toml::Spanned<DeValue<'_>>, working: &str, read: &mut Read) {
    let line = |span: std::ops::Range<usize>| line_of(working, span.start);
    let Some(scopes) = item.get_ref().as_table() else {
        read.invalid.push(line(item.span()));
        return;
    };
    for (key, value) in scopes {
        let (Some(scope), Some(list)) = (Scope::from_table(name(key)), value.get_ref().as_array())
        else {
            read.invalid.push(line(key.span()));
            continue;
        };
        let table = intern(name(key));
        for element in list.iter() {
            let Some(keys) = element.get_ref().as_str().and_then(spelled) else {
                read.invalid.push(line(element.span()));
                continue;
            };
            read.lines.push((
                Line::Unbind(read.overrides.unbinds.len()),
                line(element.span()),
            ));
            read.overrides.unbinds.push(Unbind { scope, table, keys });
        }
    }
}

/// A key as TOML writes it: quoted, with what needs it escaped.
fn quoted(text: &str) -> String {
    let mut out = String::from("\"");
    for char in text.chars() {
        match char {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            char if char.is_control() => out.push_str(&format!("\\u{:04X}", u32::from(char))),
            char => out.push(char),
        }
    }
    out.push('"');
    out
}

/// A line of `[keys.<table>]`, with its note.
fn bind_line(bind: &Bind, newline: &str) -> String {
    let note = bind
        .note
        .as_ref()
        .map_or_else(String::new, |note| format!("  # {note}"));
    format!(
        "{} = {}{note}{newline}",
        quoted(bind.keys),
        quoted(bind.command.id())
    )
}

/// The list of an unbind's keys, as TOML writes it.
fn list(keys: &[&str]) -> String {
    let quoted: Vec<String> = keys.iter().map(|keys| quoted(keys)).collect();
    format!("[{}]", quoted.join(", "))
}

/// The tables of `overrides` as a file of them alone writes them: a table
/// for each scope in the order the lines come, then `[keys.unbind]`.
pub(super) fn canonical(overrides: &Overrides, newline: &str) -> String {
    let mut out = String::new();
    let mut tables: Vec<&'static str> = Vec::new();
    for bind in &overrides.binds {
        if !tables.contains(&bind.table) {
            tables.push(bind.table);
        }
    }
    for table in tables {
        if !out.is_empty() {
            out.push_str(newline);
        }
        out.push_str(&format!("[keys.{table}]{newline}"));
        for bind in overrides.binds.iter().filter(|bind| bind.table == table) {
            out.push_str(&bind_line(bind, newline));
        }
    }
    let lists = unbind_lists(overrides);
    if !lists.is_empty() {
        if !out.is_empty() {
            out.push_str(newline);
        }
        out.push_str(&format!("[keys.{UNBIND}]{newline}"));
        for (table, keys) in lists {
            out.push_str(&format!("{table} = {}{newline}", list(&keys)));
        }
    }
    out
}

/// The unbinds of `overrides` by scope, each scope under the first table
/// name its keys were given.
fn unbind_lists(overrides: &Overrides) -> Vec<(&'static str, Vec<&'static str>)> {
    let mut lists: Vec<(Scope, &'static str, Vec<&'static str>)> = Vec::new();
    for unbind in &overrides.unbinds {
        match lists.iter_mut().find(|(scope, ..)| *scope == unbind.scope) {
            Some((.., keys)) => {
                if !keys.contains(&unbind.keys) {
                    keys.push(unbind.keys);
                }
            }
            None => lists.push((unbind.scope, unbind.table, vec![unbind.keys])),
        }
    }
    lists
        .into_iter()
        .map(|(_, table, keys)| (table, keys))
        .collect()
}

/// `text` with `tables` added at its end, a blank line before them (none
/// under the header line alone, as the other tables have it).
fn appended(text: &str, tables: &str, newline: &str) -> String {
    if tables.is_empty() {
        return text.to_owned();
    }
    let trimmed = text.trim_end_matches(['\r', '\n']);
    let mut out = trimmed.to_owned();
    if !out.is_empty() {
        out.push_str(newline);
        if trimmed != HEADER.trim_end() {
            out.push_str(newline);
        }
    }
    out.push_str(tables);
    out
}

/// `text` with every table of the keys taken out (`[keys.*]` and a
/// `[keys]` of its own), with what is in them, and nothing else changed:
/// what Reset all does. A text with the keys written another way (inline,
/// dotted) loses them where they stand on lines of their own, and keeps
/// them otherwise.
pub(super) fn without_keys(text: &str) -> String {
    let (working, _) = readable(text);
    let (document, _) = DeTable::parse_recoverable(&working);
    let document = document.into_inner();
    let Ok(Some(keys)) = item_at(&document, "keys") else {
        return text.to_owned();
    };
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut ranges: Vec<std::ops::Range<usize>> = Vec::new();
    let headed = |span: &std::ops::Range<usize>| working[span.clone()].starts_with('[');
    // From a header to the next header that was read: its keys, its
    // comments and the lines the reader ignored in it.
    let block = |start: usize| {
        let from = line_start(&working, start);
        let mut end = line_end(&working, start);
        while end < working.len() {
            let next = line_end(&working, end);
            if working[end..next]
                .trim_start_matches([' ', '\t'])
                .starts_with('[')
            {
                break;
            }
            end = next;
        }
        from..end
    };
    if headed(&keys.span()) {
        ranges.push(block(keys.span().start));
    } else if !(keys.get_ref().as_table().is_some_and(|items| {
        !items.is_empty()
            && items
                .iter()
                .all(|(_, item)| is_subtable(&working, item.get_ref(), item.span()))
    })) && own_lines(&working, keys.span())
    {
        // `keys = { … }` on lines of its own.
        let span = keys.span();
        let start = working[..span.start].rfind('\n').map_or(0, |at| at + 1);
        ranges.push(start..line_end(&working, span.end));
    }
    if let Some(items) = keys.get_ref().as_table() {
        for (_, item) in items {
            if headed(&item.span()) {
                ranges.push(block(item.span().start));
            }
        }
    }
    if ranges.is_empty() {
        log::warn!("settings: the keys are not in tables of their own, and stay");
        return text.to_owned();
    }
    ranges.sort_by_key(|range| range.start);
    let mut out = text.to_owned();
    for range in ranges.iter().rev() {
        out.replace_range(range.clone(), "");
    }
    let kept = out.trim_end_matches(['\r', '\n']).len();
    out.truncate(kept);
    if !out.is_empty() {
        out.push_str(newline);
    }
    out
}

/// A key of a table of the file, as the writer meets it.
struct Found {
    /// The span of its name and of its value, in the working text.
    key: std::ops::Range<usize>,
    value: std::ops::Range<usize>,
    /// What it reads as: its keys and its command, where both read.
    read: Option<(&'static str, Command)>,
    note: Option<String>,
}

/// A table of the keys in the file: its name in the file and under
/// `keys`, whether it has a header of its own, and its keys.
struct Table {
    path: String,
    scope: Option<Scope>,
    headed: bool,
    header_end: usize,
    found: Vec<Found>,
}

/// `text` with the keys' tables saying what `overrides` says, and nothing
/// else changed: a line that stays keeps its place and its comment, one
/// whose command or note changed is written again in its place, one that
/// goes takes its line with it, a new one goes at the end of its scope's
/// table (or of a new table at the end of the file, under its table's
/// name), and a table left empty goes. A line the reader ignored stays as
/// it is. Keys written inline or dotted, which lines cannot be added to,
/// get the keys' tables written whole.
pub(super) fn write(text: &str, overrides: &Overrides) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    if text.len() > MAX_BYTES || text.lines().count() > MAX_LINES {
        return text.to_owned();
    }
    let (working, _) = readable(text);
    let (document, _) = DeTable::parse_recoverable(&working);
    let document = document.into_inner();
    let keys = match item_at(&document, "keys") {
        Ok(Some(keys)) => keys,
        Ok(None) => return appended(text, &canonical(overrides, newline), newline),
        Err(_) => {
            log::warn!("settings: keys is not a table, the keys are not written");
            return text.to_owned();
        }
    };
    let whole = || {
        let without = without_keys(text);
        appended(&without, &canonical(overrides, newline), newline)
    };
    let Some(items) = keys.get_ref().as_table() else {
        log::warn!("settings: keys is not a table, the keys are not written");
        return text.to_owned();
    };
    let mut tables: Vec<Table> = Vec::new();
    for (key, item) in items {
        let table = name(key);
        let headed = working[item.span()].starts_with('[');
        let Some(entries) = item.get_ref().as_table() else {
            continue;
        };
        if !headed {
            return whole();
        }
        let scope = (table != UNBIND)
            .then(|| Scope::from_table(table))
            .flatten();
        let found = entries
            .iter()
            .map(|(entry, value)| {
                let read =
                    spelled(name(entry)).zip(value.get_ref().as_str().and_then(Command::from_id));
                let note = comment_after(&working, value.span().end);
                Found {
                    key: entry.span(),
                    value: value.span(),
                    read,
                    note: (!note.is_empty()).then_some(note),
                }
            })
            .collect();
        tables.push(Table {
            path: format!("keys.{table}"),
            scope,
            headed,
            header_end: item.span().end,
            found,
        });
    }
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut touched: Vec<String> = Vec::new();
    let mut matched = vec![false; overrides.binds.len()];
    // A line's whole, from its start to past its break.
    let whole_line =
        |found: &Found| line_start(&working, found.key.start)..line_end(&working, found.value.end);
    for table in tables.iter().filter(|table| table.scope.is_some()) {
        let scope = table.scope.expect("filtered");
        for found in &table.found {
            let Some((keys, command)) = found.read else {
                // A line the reader ignored stays as the person wrote it.
                continue;
            };
            let wanted = overrides.binds.iter().enumerate().find(|(index, bind)| {
                !matched[*index] && bind.scope == scope && same(bind.keys, keys)
            });
            match wanted {
                Some((index, bind)) => {
                    matched[index] = true;
                    if bind.command == command && bind.note == found.note {
                        continue;
                    }
                    if !own_lines(&working, found.key.start..found.value.end) {
                        return whole();
                    }
                    // In its place, its name as the person quoted it.
                    let note = bind
                        .note
                        .as_ref()
                        .map_or_else(String::new, |note| format!("  # {note}"));
                    let written = format!(
                        "{} = {}{note}{newline}",
                        &working[found.key.clone()],
                        quoted(bind.command.id())
                    );
                    edits.push((whole_line(found), written));
                }
                None => {
                    if !own_lines(&working, found.key.start..found.value.end) {
                        return whole();
                    }
                    edits.push((whole_line(found), String::new()));
                    touched.push(table.path.clone());
                }
            }
        }
    }
    // The new lines: at the end of their scope's table, or of a new one.
    let mut new_tables: Vec<(&'static str, Vec<&Bind>)> = Vec::new();
    for (index, bind) in overrides.binds.iter().enumerate() {
        if matched[index] {
            continue;
        }
        let table = tables
            .iter()
            .find(|table| table.scope == Some(bind.scope) && table.headed);
        match table {
            Some(table) => {
                let last = table
                    .found
                    .iter()
                    .map(|found| found.value.end)
                    .max()
                    .unwrap_or(table.header_end);
                let at = line_end(&working, last);
                let mut added = bind_line(bind, newline);
                if !working[..at].ends_with('\n') {
                    added.insert_str(0, newline);
                }
                edits.push((at..at, added));
            }
            None => match new_tables
                .iter_mut()
                .find(|(table, _)| *table == bind.table)
            {
                Some((_, binds)) => binds.push(bind),
                None => new_tables.push((bind.table, vec![bind])),
            },
        }
    }
    // The unbinds, a list for each scope.
    let lists = unbind_lists(overrides);
    let unbind = tables
        .iter()
        .find(|table| table.path == format!("keys.{UNBIND}"));
    let mut listed: Vec<Scope> = Vec::new();
    let mut new_lists: Vec<(&'static str, Vec<&'static str>)> = Vec::new();
    if let Some(table) = unbind {
        let entries = items
            .iter()
            .find(|(key, _)| name(key) == UNBIND)
            .and_then(|(_, item)| item.get_ref().as_table());
        for (key, value) in entries.into_iter().flatten() {
            let Some(scope) = Scope::from_table(name(key)) else {
                continue;
            };
            let Some(array) = value.get_ref().as_array() else {
                continue;
            };
            listed.push(scope);
            // The keys of the list that did not read stay in it.
            let (read, unread): (Vec<_>, Vec<_>) = array
                .iter()
                .partition(|element| element.get_ref().as_str().and_then(spelled).is_some());
            let have: Vec<&'static str> = read
                .iter()
                .filter_map(|element| element.get_ref().as_str().and_then(spelled))
                .collect();
            let want: Vec<&'static str> = lists
                .iter()
                .filter(|(table, _)| Scope::from_table(table) == Some(scope))
                .flat_map(|(_, keys)| keys.iter().copied())
                .collect();
            let alike = have.len() == want.len()
                && have
                    .iter()
                    .all(|keys| want.iter().any(|wanted| same(keys, wanted)));
            if alike {
                continue;
            }
            let whole_span = key.span().start..value.span().end;
            if !own_lines(&working, whole_span.clone()) {
                return whole();
            }
            if want.is_empty() && unread.is_empty() {
                let lines =
                    line_start(&working, whole_span.start)..line_end(&working, whole_span.end);
                edits.push((lines, String::new()));
                touched.push(table.path.clone());
                continue;
            }
            let mut written: Vec<String> = want.iter().map(|keys| quoted(keys)).collect();
            written.extend(
                unread
                    .iter()
                    .map(|element| working[element.span()].to_owned()),
            );
            edits.push((value.span(), format!("[{}]", written.join(", "))));
        }
        for (table_name, keys) in &lists {
            let scope = Scope::from_table(table_name);
            if scope.is_some_and(|scope| listed.contains(&scope)) {
                continue;
            }
            let last = table
                .found
                .iter()
                .map(|found| found.value.end)
                .max()
                .unwrap_or(table.header_end);
            let at = line_end(&working, last);
            let mut added = format!("{table_name} = {}{newline}", list(keys));
            if !working[..at].ends_with('\n') {
                added.insert_str(0, newline);
            }
            edits.push((at..at, added));
        }
    } else {
        new_lists = lists;
    }
    let mut out = text.to_owned();
    let mut order: Vec<usize> = (0..edits.len()).collect();
    order.sort_by(|a, b| (edits[*b].0.start, *b).cmp(&(edits[*a].0.start, *a)));
    for index in order {
        let (range, with) = &edits[index];
        out.replace_range(range.clone(), with);
    }
    for path in touched {
        out = without_empty_table(&out, &path);
    }
    let mut added = String::new();
    for (table, binds) in new_tables {
        if !added.is_empty() {
            added.push_str(newline);
        }
        added.push_str(&format!("[keys.{table}]{newline}"));
        for bind in binds {
            added.push_str(&bind_line(bind, newline));
        }
    }
    if !new_lists.is_empty() {
        if !added.is_empty() {
            added.push_str(newline);
        }
        added.push_str(&format!("[keys.{UNBIND}]{newline}"));
        for (table, keys) in new_lists {
            added.push_str(&format!("{table} = {}{newline}", list(&keys)));
        }
    }
    if added.is_empty() {
        // A text whose last table went keeps one line break at its end.
        let kept = out.trim_end_matches(['\r', '\n']).len();
        if kept < out.len() {
            out.truncate(kept);
            if !out.is_empty() {
                out.push_str(newline);
            }
        }
        return out;
    }
    appended(&out, &added, newline)
}

/// Whether two spellings name the same keys.
fn same(a: &'static str, b: &'static str) -> bool {
    match (spell(a), spell(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::Settings;

    fn bind(scope: Scope, table: &str, keys: &str, command: Command) -> Bind {
        Bind {
            scope,
            table: intern(table),
            keys: intern(keys),
            command,
            note: None,
        }
    }

    fn unbind(scope: Scope, table: &str, keys: &str) -> Unbind {
        Unbind {
            scope,
            table: intern(table),
            keys: intern(keys),
        }
    }

    /// `text` with the settings it reads as changed by `change`, written
    /// back as the app writes a change.
    fn written(text: &str, change: impl FnOnce(&mut Overrides)) -> String {
        let mut settings = Settings::from_toml(text).settings;
        change(&mut settings.keys);
        settings.write_keys_into(text)
    }

    const FILE: &str = "\
# written by tabletist, safe to edit by hand
[data]
page_size = 500

# mine
[keys.grid]
\"D\" = \"delete_row\"  # after swap · was set_default
\"dd\" = \"set_default\"

[keys.editor]
\"ctrl+alt+r\" = \"run_all\"

[keys.unbind]
grid = [\"gd\"]
";

    #[test]
    fn the_keys_tables_are_read_with_the_line_of_each() {
        let loaded = Settings::from_toml(FILE);
        assert!(loaded.invalid.is_empty(), "{:?}", loaded.invalid);
        assert_eq!(loaded.settings.page_size, 500);
        let keys = &loaded.settings.keys;
        assert_eq!(
            keys.binds,
            [
                Bind {
                    note: Some("after swap · was set_default".to_owned()),
                    ..bind(Scope::Grid, "grid", "D", Command::DeleteRow)
                },
                bind(Scope::Grid, "grid", "dd", Command::SetDefault),
                bind(Scope::SqlEditor, "editor", "ctrl+alt+r", Command::RunAll),
            ]
        );
        assert_eq!(keys.unbinds, [unbind(Scope::Grid, "grid", "gd")]);
        assert_eq!(
            loaded.key_lines,
            [
                (Line::Bind(0), 7),
                (Line::Bind(1), 8),
                (Line::Bind(2), 11),
                (Line::Unbind(0), 14),
            ]
        );
    }

    #[test]
    fn a_line_this_version_cannot_read_is_ignored_by_its_number() {
        let text = "\
[keys.grid]
\"x\" = \"set_null\"
\"super+t\" = \"set_null\"
\"ctrl+nosuchkey\" = \"set_null\"
\"y\" = \"no_such_command\"
\"z\" = 5

[keys.nowhere]
\"q\" = \"set_null\"

[keys.unbind]
grid = [\"gd\", \"super+x\", \"gs\"]
sql = \"=\"
";
        let loaded = Settings::from_toml(text);
        assert_eq!(loaded.invalid, [3, 4, 5, 6, 8, 12, 13]);
        assert_eq!(
            loaded.settings.keys.binds,
            [bind(Scope::Grid, "grid", "x", Command::SetNull)]
        );
        // The list's other keys stand.
        assert_eq!(
            loaded.settings.keys.unbinds,
            [
                unbind(Scope::Grid, "grid", "gd"),
                unbind(Scope::Grid, "grid", "gs"),
            ]
        );
    }

    #[test]
    fn a_new_line_goes_into_a_new_table_named_for_the_look() {
        let text = "# written by tabletist, safe to edit by hand\n[data]\npage_size = 500\n";
        let out = written(text, |keys| {
            keys.binds.push(bind(
                Scope::SqlEditor,
                "editor",
                "cmd+alt+f",
                Command::Format,
            ));
        });
        assert_eq!(
            out,
            "# written by tabletist, safe to edit by hand\n[data]\npage_size = 500\n\n\
             [keys.editor]\n\"cmd+alt+f\" = \"format\"\n"
        );
        // Under the header alone, as the other tables go.
        let out = written(HEADER, |keys| {
            keys.binds
                .push(bind(Scope::Grid, "table", "cmd+j", Command::SetNull));
        });
        assert_eq!(
            out,
            "# written by tabletist, safe to edit by hand\n[keys.table]\n\"cmd+j\" = \"set_null\"\n"
        );
        // It reads back as it was written.
        assert_eq!(
            Settings::from_toml(&out).settings.keys.binds,
            [bind(Scope::Grid, "table", "cmd+j", Command::SetNull)]
        );
    }

    #[test]
    fn a_line_that_stays_is_untouched_and_one_that_goes_takes_its_line() {
        // Nothing changed: nothing written.
        assert_eq!(written(FILE, |_| {}), FILE);
        // The line of Run all goes, and its table with it.
        let out = written(FILE, |keys| {
            keys.binds.retain(|bind| bind.command != Command::RunAll);
        });
        assert_eq!(
            out,
            FILE.replace("[keys.editor]\n\"ctrl+alt+r\" = \"run_all\"\n\n", "")
        );
        // A new line of the grid goes after its last one; the comment
        // above the table and the note beside a line stay.
        let out = written(FILE, |keys| {
            keys.binds
                .push(bind(Scope::Grid, "grid", "gx", Command::SetNull));
        });
        assert_eq!(
            out,
            FILE.replace(
                "\"dd\" = \"set_default\"\n",
                "\"dd\" = \"set_default\"\n\"gx\" = \"set_null\"\n"
            )
        );
        // A line given another command is written again in its place.
        let out = written(FILE, |keys| {
            keys.binds[1].command = Command::SetNull;
        });
        assert_eq!(
            out,
            FILE.replace("\"dd\" = \"set_default\"", "\"dd\" = \"set_null\"")
        );
    }

    #[test]
    fn a_line_the_reader_ignored_stays_through_a_write() {
        let text = "[keys.grid]\n\"y\" = \"no_such_command\"\n\"x\" = \"set_null\"\n";
        let out = written(text, |keys| keys.binds.clear());
        assert_eq!(out, "[keys.grid]\n\"y\" = \"no_such_command\"\n");
    }

    #[test]
    fn the_unbinds_are_written_as_a_list_for_each_scope() {
        let out = written(FILE, |keys| {
            keys.unbinds.push(unbind(Scope::Grid, "grid", "gs"));
        });
        assert_eq!(
            out,
            FILE.replace("grid = [\"gd\"]", "grid = [\"gd\", \"gs\"]")
        );
        let out = written(FILE, |keys| {
            keys.unbinds.push(unbind(Scope::SqlEditor, "sql", "ctrl+e"));
        });
        assert_eq!(
            out,
            FILE.replace("grid = [\"gd\"]\n", "grid = [\"gd\"]\nsql = [\"ctrl+e\"]\n")
        );
        // The last unbind goes with its table.
        let out = written(FILE, |keys| keys.unbinds.clear());
        assert_eq!(out, FILE.replace("\n[keys.unbind]\ngrid = [\"gd\"]\n", ""));
        // A key of the list that did not read stays in it.
        let text = "[keys.unbind]\ngrid = [\"gd\", \"super+x\"]\n";
        let out = written(text, |keys| keys.unbinds.clear());
        assert_eq!(out, "[keys.unbind]\ngrid = [\"super+x\"]\n");
    }

    #[test]
    fn what_is_written_whole_reads_back_the_same() {
        let settings = Settings::from_toml(FILE).settings;
        let text = settings.to_toml();
        let read = Settings::from_toml(&text);
        assert!(read.invalid.is_empty());
        assert_eq!(read.settings.keys, settings.keys);
        assert!(
            text.contains("[keys.grid]\n\"D\" = \"delete_row\"  # after swap · was set_default\n")
        );
    }

    #[test]
    fn keys_written_inline_get_the_tables_written_whole() {
        let text = "[data]\npage_size = 500\n\n[keys]\ngrid = { \"x\" = \"set_null\" }\n";
        let out = written(text, |keys| {
            keys.binds
                .push(bind(Scope::Grid, "grid", "gx", Command::SetNull));
        });
        assert_eq!(
            out,
            "[data]\npage_size = 500\n\n[keys.grid]\n\"x\" = \"set_null\"\n\"gx\" = \"set_null\"\n"
        );
    }

    #[test]
    fn reset_all_takes_every_keys_table_and_nothing_else() {
        let text = format!(
            "{FILE}\n[editor.run]\nlimit = 10\n\n[keys.navigate]\n# a comment of mine\n\"y\" = \"no_such_command\"\n"
        );
        let out = Settings::without_keys(&text);
        assert_eq!(
            out,
            "# written by tabletist, safe to edit by hand\n[data]\npage_size = 500\n\n# mine\n\
             [editor.run]\nlimit = 10\n"
        );
        let loaded = Settings::from_toml(&out);
        assert!(loaded.settings.keys.is_empty());
        assert_eq!(loaded.settings.page_size, 500);
        assert_eq!(loaded.settings.sql_limit, 10);
        // A file with no keys stays as it is.
        assert_eq!(
            Settings::without_keys("[data]\npage_size = 500\n"),
            "[data]\npage_size = 500\n"
        );
    }

    #[test]
    fn a_file_with_crlf_endings_gets_crlf_key_lines() {
        let text = "[data]\r\npage_size = 500\r\n";
        let out = written(text, |keys| {
            keys.binds
                .push(bind(Scope::Grid, "grid", "gx", Command::SetNull));
        });
        assert_eq!(
            out,
            "[data]\r\npage_size = 500\r\n\r\n[keys.grid]\r\n\"gx\" = \"set_null\"\r\n"
        );
    }
}
