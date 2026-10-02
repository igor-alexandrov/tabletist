//! Turning values into text for the grid, the row panel and the clipboard.

use std::borrow::Cow;
use std::fmt::Write as _;
use std::time::Duration;

use tabletist_db::{Value, ValueKind};

/// Characters a grid cell shows before cutting the value off.
pub const CELL_MAX_CHARS: usize = 256;
/// Bytes of a binary value the row panel shows as hex: two lines of twelve.
pub const HEX_PREVIEW: usize = 24;
/// Lines of a long value the row panel lays out before "Show all". It
/// shows the first three of them (see `row_panel`); this bounds the layout.
pub const COLLAPSE_LINES: usize = 20;
/// Characters of a long value the row panel lays out before "Show all", so
/// a huge single-line value is never laid out whole.
pub const COLLAPSE_CHARS: usize = 4_000;

/// One short line for a grid cell.
pub fn cell_text(value: &Value) -> Cow<'_, str> {
    match value {
        Value::Null => Cow::Borrowed("NULL"),
        Value::Bool(flag) => Cow::Borrowed(if *flag { "true" } else { "false" }),
        Value::Int(number) => Cow::Owned(number.to_string()),
        Value::Float(number) => Cow::Owned(number.to_string()),
        Value::Text(text) => one_line(text),
        Value::Bytes(bytes) => {
            Cow::Owned(uuid(bytes).unwrap_or_else(|| format!("BLOB · {}", human_size(bytes.len()))))
        }
    }
}

/// Sixteen bytes as the UUID they hold: `0199a3f2-7c1e-7abc-8def-0123456789ab`.
/// SQLite and MySQL have no UUID type, so a UUID key is kept in a `blob(16)`
/// or `binary(16)` column and arrives as bytes. The text is the bytes' hex
/// with four hyphens, so sixteen bytes that are no UUID lose nothing by it.
/// `None` for any other length.
pub fn uuid(bytes: &[u8]) -> Option<String> {
    let bytes: &[u8; 16] = bytes.try_into().ok()?;
    let mut text = String::with_capacity(36);
    for (index, byte) in bytes.iter().enumerate() {
        if matches!(index, 4 | 6 | 8 | 10) {
            text.push('-');
        }
        let _ = write!(text, "{byte:02x}");
    }
    Some(text)
}

/// Text on one line: line breaks and tabs become spaces, hidden characters
/// are written out. For text that is a part of something else (a
/// document's first string); a cell's own text keeps its breaks in sight
/// (see [`cell_line`]).
pub fn one_line(text: &str) -> Cow<'_, str> {
    bounded_line(text, CELL_MAX_CHARS, Breaks::Space)
}

/// The marks a cell writes where text would show nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Marks {
    /// One for each character of a value that is only whitespace.
    pub space: char,
    /// In place of a line break.
    pub line: char,
}

impl Marks {
    /// The design's marks. IBM Plex and JetBrains Mono have neither: they
    /// come from a font of the system's, as they do in a browser.
    pub const DESIGN: Self = Self {
        space: '␣',
        line: '↵',
    };
    /// Marks every bundled face draws, where no font has the design's.
    pub const PLAIN: Self = Self {
        space: '·',
        line: '¶',
    };

    /// The design's marks when `has_glyph` finds both, else the plain ones.
    pub fn pick(mut has_glyph: impl FnMut(char) -> bool) -> Self {
        if has_glyph(Self::DESIGN.space) && has_glyph(Self::DESIGN.line) {
            Self::DESIGN
        } else {
            Self::PLAIN
        }
    }
}

/// A cell's text on one line: a line break becomes `marks.line` (a Windows
/// line end is one break), a tab a space, hidden characters are written out.
pub fn cell_line(text: &str, marks: Marks) -> Cow<'_, str> {
    bounded_line(text, CELL_MAX_CHARS, Breaks::Mark(marks.line))
}

/// What a cell shows for text with nothing to see: `''` for an empty
/// string, a mark for each character of one that is only whitespace. `None`
/// for text that shows as itself.
pub fn blank_text(text: &str, marks: Marks) -> Option<String> {
    if text.is_empty() {
        return Some("''".to_owned());
    }
    let mut shown = String::new();
    let mut characters = text.chars().peekable();
    let mut count = 0;
    while let Some(character) = characters.next() {
        if !character.is_whitespace() {
            return None;
        }
        // A cell's worth is all that is looked at, as for any text.
        if count == CELL_MAX_CHARS {
            shown.push('…');
            break;
        }
        count += 1;
        match character {
            '\r' if characters.peek() == Some(&'\n') => count -= 1,
            '\n' | '\r' => shown.push(marks.line),
            _ => shown.push(marks.space),
        }
    }
    Some(shown)
}

/// Characters of a name (schema, table, column, database) the UI lays out.
/// SQLite names have no length limit, and names are laid out every frame.
pub const NAME_MAX_CHARS: usize = 256;

/// A name that came from the server, as the UI shows it: at most
/// NAME_MAX_CHARS characters, with every hidden character (a line break
/// included) written out as `<U+202E>`, so `users_\u{202E}atad` can not pass
/// for `users_data`, nor `users\u{200B}` for `users`. Copying keeps the name
/// as it is.
pub fn display_safe(name: &str) -> Cow<'_, str> {
    bounded_line(name, NAME_MAX_CHARS, Breaks::Escape)
}

/// `text` with every hidden character written out, as in `display_safe`,
/// and not cut: for text already cut to a size.
pub fn escape_hidden(text: &str) -> Cow<'_, str> {
    bounded_line(text, usize::MAX, Breaks::Escape)
}

/// An object as its tab and the row panel name it: the name, or
/// `schema.name` when `qualified`, both parts as `display_safe` shows them.
pub fn object_title(object: &tabletist_db::ObjectRef, qualified: bool) -> String {
    let name = display_safe(&object.name);
    if qualified {
        format!("{}.{name}", display_safe(&object.schema))
    } else {
        name.into_owned()
    }
}

/// What [`bounded_line`] does with a line break.
#[derive(Clone, Copy)]
enum Breaks {
    /// Written out, as any hidden character (a name has none of its own).
    Escape,
    /// A space, and a tab too.
    Space,
    /// This mark; a tab is a space.
    Mark(char),
}

/// The first `max` characters of `text` with hidden characters written out
/// and "…" when cut; line breaks and tabs as `breaks` says. Borrows when
/// there is nothing to change.
fn bounded_line(text: &str, max: usize, breaks: Breaks) -> Cow<'_, str> {
    // Only the first `max` characters are ever shown, so never look past
    // them: a cell may hold megabytes and is drawn every frame.
    let (end, too_long) = match text.char_indices().nth(max) {
        Some((index, _)) => (index, true),
        None => (text.len(), false),
    };
    let head = &text[..end];
    if !too_long && !head.chars().any(is_hidden) {
        return Cow::Borrowed(text);
    }
    let mut line = String::with_capacity(head.len() + 16);
    let mut characters = head.chars().peekable();
    while let Some(character) = characters.next() {
        match (breaks, character) {
            (Breaks::Space, '\n' | '\r' | '\t') | (Breaks::Mark(_), '\t') => line.push(' '),
            // A Windows line end is one break: its LF writes the mark.
            (Breaks::Mark(_), '\r') if characters.peek() == Some(&'\n') => {}
            (Breaks::Mark(mark), '\n' | '\r') => line.push(mark),
            _ if is_hidden(character) => push_escaped(&mut line, character),
            _ => line.push(character),
        }
    }
    if too_long {
        line.push('…');
    }
    Cow::Owned(line)
}

/// Whether `character` draws nothing, or changes how the text around it is
/// drawn: controls (C0, DEL and C1) and the invisible Unicode format
/// characters (general category Cf). egui draws none of these, and its bidi
/// pass obeys the direction ones, so U+202E shows the text after it
/// backwards.
///
/// The Cf list is written out rather than taken from a crate: it is short and
/// changes rarely. It leaves out the Cf characters that do draw a mark (the
/// Arabic number signs U+0600 to U+0605, U+06DD, U+070F, U+0890, U+0891,
/// U+08E2, U+110BD, U+110CD) and adds U+2028 and U+2029, the line and
/// paragraph separators, which draw nothing either.
fn is_hidden(character: char) -> bool {
    if character.is_ascii() {
        return character.is_ascii_control();
    }
    character.is_control()
        || matches!(
            character,
            '\u{00AD}' // soft hyphen
                | '\u{061C}' // Arabic letter mark
                | '\u{180E}' // Mongolian vowel separator
                | '\u{200B}'..='\u{200F}' // zero width space, joiners, LRM, RLM
                | '\u{2028}'..='\u{202E}' // separators, bidi embeddings and overrides
                | '\u{2060}'..='\u{2064}' // word joiner, invisible operators
                | '\u{2066}'..='\u{206F}' // bidi isolates, deprecated format characters
                | '\u{FEFF}' // byte order mark
                | '\u{FFF9}'..='\u{FFFB}' // interlinear annotation
                | '\u{13430}'..='\u{1343F}' // Egyptian hieroglyph format controls
                | '\u{1BCA0}'..='\u{1BCA3}' // shorthand format controls
                | '\u{1D173}'..='\u{1D17A}' // musical symbol format controls
                | '\u{E0001}' // language tag
                | '\u{E0020}'..='\u{E007F}' // tag characters
        )
}

fn push_escaped(out: &mut String, character: char) {
    let _ = write!(out, "<U+{:04X}>", u32::from(character));
}

/// Whether a column of this type and kind holds PostgreSQL arrays: a result
/// names their type `_text`, the catalog `text[]`. No other database has
/// them.
pub fn is_array(type_name: &str, kind: ValueKind) -> bool {
    kind == ValueKind::Other
        && (type_name.ends_with("[]")
            || type_name
                .strip_prefix('_')
                .is_some_and(|element| !element.is_empty()))
}

/// A column's type as people write it: an array's `_text` is `text[]`.
pub fn type_label(type_name: &str, kind: ValueKind) -> Cow<'_, str> {
    match type_name.strip_prefix('_') {
        Some(element) if is_array(type_name, kind) => Cow::Owned(format!("{element}[]")),
        _ => Cow::Borrowed(type_name),
    }
}

/// A PostgreSQL array's elements.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayItems<'a> {
    /// Each element as its text: a quoted one without its quotes, an array
    /// inside the array as it is written.
    pub items: Vec<Cow<'a, str>>,
    /// The text ended before the array did (a cell's text is cut): there
    /// are more elements than these.
    pub cut: bool,
}

/// The elements of an array as PostgreSQL writes one: `{en,fr}`,
/// `{"two words",NULL}`, `{{1,2},{3,4}}`. `None` for text that is no array,
/// one with its bounds written before it (`[0:1]={a,b}`) included.
pub fn array_items(text: &str) -> Option<ArrayItems<'_>> {
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'{') {
        return None;
    }
    let mut items = Vec::new();
    if text == "{}" {
        return Some(ArrayItems { items, cut: false });
    }
    // The delimiters are ASCII, so stepping by bytes never splits a
    // character, and every slice below starts and ends on one.
    let mut at = 1;
    loop {
        let start = at;
        let item = match bytes.get(at) {
            None => return Some(ArrayItems { items, cut: true }),
            Some(b'"') => {
                at += 1;
                let from = at;
                // Copied only when an escape makes it differ from the text.
                let mut unescaped: Option<String> = None;
                loop {
                    let Some(character) = text[at..].chars().next() else {
                        return Some(ArrayItems { items, cut: true });
                    };
                    at += character.len_utf8();
                    match character {
                        '"' => break,
                        '\\' => {
                            let Some(next) = text[at..].chars().next() else {
                                return Some(ArrayItems { items, cut: true });
                            };
                            unescaped
                                .get_or_insert_with(|| text[from..at - 1].to_owned())
                                .push(next);
                            at += next.len_utf8();
                        }
                        _ => {
                            if let Some(copy) = &mut unescaped {
                                copy.push(character);
                            }
                        }
                    }
                }
                unescaped.map_or(Cow::Borrowed(&text[from..at - 1]), Cow::Owned)
            }
            Some(b'{') => {
                let (mut depth, mut quoted) = (0, false);
                loop {
                    match bytes.get(at) {
                        None => return Some(ArrayItems { items, cut: true }),
                        Some(b'\\') if quoted => at += 1,
                        Some(b'"') => quoted = !quoted,
                        Some(b'{') if !quoted => depth += 1,
                        Some(b'}') if !quoted => depth -= 1,
                        Some(_) => {}
                    }
                    at += 1;
                    if depth == 0 {
                        break;
                    }
                }
                Cow::Borrowed(&text[start..at.min(text.len())])
            }
            Some(_) => {
                while bytes
                    .get(at)
                    .is_some_and(|byte| !matches!(byte, b',' | b'}'))
                {
                    at += 1;
                }
                if at == bytes.len() {
                    return Some(ArrayItems { items, cut: true });
                }
                if at == start {
                    return None;
                }
                Cow::Borrowed(&text[start..at])
            }
        };
        items.push(item);
        match bytes.get(at) {
            Some(b',') => at += 1,
            Some(b'}') if at + 1 == bytes.len() => return Some(ArrayItems { items, cut: false }),
            None => return Some(ArrayItems { items, cut: true }),
            Some(_) => return None,
        }
    }
}

/// The colour a `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` value names, as
/// red, green, blue and alpha (opaque when the value has none).
pub fn hex_color(text: &str) -> Option<[u8; 4]> {
    let digits = text.strip_prefix('#')?;
    if !matches!(digits.len(), 3 | 4 | 6 | 8)
        || !digits.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return None;
    }
    // `#f80` is `#ff8800`: a short form's digits each stand for two.
    let (len, scale) = if digits.len() <= 4 { (1, 17) } else { (2, 1) };
    let channel = |index: usize| {
        let at = index * len;
        digits.get(at..at + len).map_or(Some(255), |hex| {
            Some(u8::from_str_radix(hex, 16).ok()? * scale)
        })
    };
    Some([channel(0)?, channel(1)?, channel(2)?, channel(3)?])
}

/// The colour a text value names, for a swatch beside it.
pub fn color(value: &Value) -> Option<egui::Color32> {
    match value {
        Value::Text(text) => {
            hex_color(text).map(|[r, g, b, a]| egui::Color32::from_rgba_unmultiplied(r, g, b, a))
        }
        _ => None,
    }
}

/// A timestamp or time shown to the second: `2026-01-12 09:14:03.482915`
/// becomes `2026-01-12 09:14:03`, keeping any zone after the fraction.
pub fn to_the_second(text: &str) -> Cow<'_, str> {
    // The fraction follows a time's `hh:mm:ss`.
    let Some(dot) = text.find('.') else {
        return Cow::Borrowed(text);
    };
    let before = &text[..dot];
    let is_time = before.len() >= 8
        && before.as_bytes()[before.len() - 3] == b':'
        && before[before.len() - 2..]
            .bytes()
            .all(|b| b.is_ascii_digit());
    if !is_time {
        return Cow::Borrowed(text);
    }
    let rest = &text[dot + 1..];
    let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return Cow::Borrowed(text);
    }
    Cow::Owned(format!("{before}{}", &rest[digits..]))
}

/// `text` cut in the middle with "…" to fit `max` points as measured by
/// `width`, keeping both ends (a file's name and its extension).
pub fn ellipsize_middle(text: &str, max: f32, width: impl Fn(&str) -> f32) -> String {
    if width(text) <= max {
        return text.to_owned();
    }
    let chars: Vec<char> = text.chars().collect();
    let candidate = |kept: usize| -> String {
        let tail = kept / 2;
        let head = kept - tail;
        chars[..head]
            .iter()
            .chain(std::iter::once(&'…'))
            .chain(chars[chars.len() - tail..].iter())
            .collect()
    };
    let (mut fits, mut too_many) = (0, chars.len());
    while too_many - fits > 1 {
        let middle = (fits + too_many) / 2;
        if width(&candidate(middle)) <= max {
            fits = middle;
        } else {
            too_many = middle;
        }
    }
    candidate(fits)
}

/// The whole value as text, for the clipboard. Binary becomes `0x` hex,
/// and sixteen bytes the UUID the grid shows.
pub fn plain_text(value: &Value) -> String {
    match value {
        Value::Bytes(bytes) => uuid(bytes).unwrap_or_else(|| {
            let mut hex = String::with_capacity(2 + bytes.len() * 2);
            hex.push_str("0x");
            for byte in bytes.iter() {
                let _ = write!(hex, "{byte:02x}");
            }
            hex
        }),
        Value::Text(text) => text.to_string(),
        other => cell_text(other).into_owned(),
    }
}

#[cfg(test)]
thread_local! {
    /// How many times this thread ran `full_text`.
    pub static FULL_TEXTS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The whole value as the row panel shows it as text. JSON the panel can
/// parse is drawn as a tree instead (`json_view`). Sixteen bytes are the
/// UUID the grid shows; any other binary value is its first bytes
/// ([`hex_preview`]). Text is borrowed, not copied.
pub fn full_text(value: &Value) -> Cow<'_, str> {
    #[cfg(test)]
    FULL_TEXTS.with(|count| count.set(count.get() + 1));
    match value {
        Value::Text(text) => Cow::Borrowed(text),
        Value::Bytes(bytes) => Cow::Owned(uuid(bytes).unwrap_or_else(|| hex_preview(bytes))),
        other => Cow::Owned(plain_text(other)),
    }
}

/// One field of the row panel as text, ready to lay out.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FieldText {
    /// What the panel shows first: the whole value, or its start when long.
    pub short: String,
    /// The whole value, when `short` is only its start.
    pub full: Option<String>,
    /// The value's size, for "Show all".
    pub size: String,
    /// How many characters a text value has, for its label.
    pub characters: Option<usize>,
}

/// Formats a value for the row panel. Slow for a big value (it reads all of
/// it), so the app calls it once per selected row, not every frame.
pub fn field_text(value: &Value) -> FieldText {
    let text = full_text(value);
    // A binary value is its first bytes and nothing to unfold: the whole
    // of it is a file's worth, and copying or saving gives it. Sixteen
    // bytes read as the UUID they hold, as text does.
    if let Value::Bytes(bytes) = value
        && uuid(bytes).is_none()
    {
        return FieldText {
            short: text.into_owned(),
            full: None,
            size: human_size(bytes.len()),
            characters: None,
        };
    }
    let long = text.len() > COLLAPSE_CHARS || text.lines().nth(COLLAPSE_LINES).is_some();
    let size = human_size(text.len());
    let characters = matches!(value, Value::Text(_)).then(|| text.chars().count());
    if !long {
        return FieldText {
            short: for_display(&text),
            full: None,
            size,
            characters,
        };
    }
    let start: String = text
        .lines()
        .take(COLLAPSE_LINES)
        .collect::<Vec<_>>()
        .join("\n")
        .chars()
        .take(COLLAPSE_CHARS)
        .collect();
    FieldText {
        short: for_display(&start),
        full: Some(for_display(&text)),
        size,
        characters,
    }
}

/// The first [`HEX_PREVIEW`] bytes as `ff d8 ff ...`, twelve to a line.
pub fn hex_preview(bytes: &[u8]) -> String {
    let mut out = String::new();
    for (index, byte) in bytes.iter().take(HEX_PREVIEW).enumerate() {
        if index > 0 {
            out.push(if index % 12 == 0 { '\n' } else { ' ' });
        }
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// What a binary value starts as, by the bytes files of that kind begin
/// with. A guess, and said as one ("looks like JPEG").
pub fn sniff(bytes: &[u8]) -> Option<&'static str> {
    const SIGNS: [(&[u8], &str); 8] = [
        (b"\xff\xd8\xff", "JPEG"),
        (b"\x89PNG\r\n\x1a\n", "PNG"),
        (b"GIF87a", "GIF"),
        (b"GIF89a", "GIF"),
        (b"%PDF-", "PDF"),
        (b"PK\x03\x04", "ZIP"),
        (b"\x1f\x8b", "gzip"),
        (b"SQLite format 3\0", "SQLite"),
    ];
    if bytes.len() >= 12 && bytes.starts_with(b"RIFF") && &bytes[8..12] == b"WEBP" {
        return Some("WebP");
    }
    SIGNS
        .iter()
        .find(|(sign, _)| bytes.starts_with(sign))
        .map(|(_, name)| *name)
}

/// What a grid cell says of a binary value: its type and its size, never
/// its bytes. `bytea · 48.2 KB`, or the terminal's `bytea 48.2K`.
pub fn binary_label(type_name: &str, bytes: usize, terminal: bool) -> String {
    let name = if type_name.is_empty() {
        "binary"
    } else {
        type_name
    };
    let name = display_safe(name);
    if terminal {
        format!("{name} {}", terse_size(bytes))
    } else {
        format!("{name} · {}", human_size(bytes))
    }
}

/// [`human_size`] as the terminal writes it: `0B`, `48.2K`, `2.1M`.
pub fn terse_size(bytes: usize) -> String {
    human_size(bytes)
        .replace(" KB", "K")
        .replace(" MB", "M")
        .replace(" B", "B")
}

pub fn human_size(bytes: usize) -> String {
    const KB: f64 = 1024.0;
    let size = bytes as f64;
    if size < KB {
        format!("{bytes} B")
    } else if size < KB * KB {
        format!("{:.1} KB", size / KB)
    } else {
        format!("{:.1} MB", size / (KB * KB))
    }
}

/// `1.2K`, `3.4M`, `7B`, dropping a trailing `.0`.
pub fn compact_count(n: u64) -> String {
    let (value, suffix) = match n {
        0..=999 => return n.to_string(),
        1_000..=999_999 => (n as f64 / 1e3, "K"),
        1_000_000..=999_999_999 => (n as f64 / 1e6, "M"),
        _ => (n as f64 / 1e9, "B"),
    };
    // Round half away from zero first: `{:.1}` alone rounds 1.25 to 1.2.
    let text = format!("{:.1}", (value * 10.0).round() / 10.0);
    format!("{}{suffix}", text.trim_end_matches(".0"))
}

/// `1–300 of ~1.2M`, or `None` when the page is empty.
pub fn range_label(
    offset: u64,
    shown: usize,
    has_more: bool,
    estimate: Option<u64>,
    exact: Option<u64>,
) -> Option<String> {
    if shown == 0 {
        return None;
    }
    let first = offset + 1;
    let last = offset + shown as u64;
    let total = if let Some(exact) = exact {
        format!(" of {}", group_digits(exact))
    } else if !has_more {
        format!(" of {last}")
    } else if let Some(estimate) = estimate.filter(|estimate| *estimate >= last) {
        format!(" of ~{}", compact_count(estimate))
    } else {
        String::new()
    };
    Some(format!("{first}–{last}{total}"))
}

/// `1234567` as `1,234,567`.
pub fn group_digits(number: u64) -> String {
    let digits = number.to_string();
    let mut out = String::new();
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// What went wrong, in words for someone who is not reading driver
/// messages; the exact error stays available (as hover text) for the rest.
/// The server's own words (a query error, a host key prompt) are shown as
/// they came; every sentence of ours is translated.
pub fn describe_error(locale: impl fastframe_i18n::Locale, error: &tabletist_db::Error) -> String {
    use crate::i18n::gettext;
    use tabletist_db::{Error, SshStage};
    let sentence = match error {
        Error::Connect(_) => gettext(
            locale,
            "Could not reach the server. Check the host and port, and that the server is running.",
        ),
        Error::Auth(_) => gettext(
            locale,
            "The server refused the login. Check the user and password.",
        ),
        Error::Timeout => gettext(locale, "The server did not answer in time."),
        Error::ConnectionLost(_) => gettext(locale, "The connection was lost."),
        Error::Tls(_) => gettext(
            locale,
            "The secure connection failed. Try another TLS mode, or check the certificate.",
        ),
        Error::Ssh { stage, message } => match stage {
            SshStage::Connect => gettext(locale, "The SSH server could not be reached."),
            SshStage::Auth | SshStage::Secret => gettext(locale, "The SSH login failed."),
            SshStage::Forward => gettext(locale, "The SSH server could not reach the database."),
            // The host key messages already say what to do.
            SshStage::HostKeyUnknown { .. } | SshStage::HostKeyMismatch { .. } => {
                return message.clone();
            }
        },
        Error::Query { message, .. } => return message.clone(),
        other => return other.to_string(),
    };
    sentence.into_owned()
}

/// The most characters of what a database said that a message or an error
/// card shows.
pub const MESSAGE_MAX_CHARS: usize = 2_000;

/// What a database said, cut to what a message shows and ending in "…"
/// when cut. A message can hold megabytes (PostgreSQL repeats a literal it
/// cannot read) and is written and laid out every frame, so nothing here
/// looks past the cut. The start says what went wrong.
pub fn capped(text: &str) -> Cow<'_, str> {
    match text.char_indices().nth(MESSAGE_MAX_CHARS) {
        Some((end, _)) => format!("{}…", &text[..end]).into(),
        None => text.into(),
    }
}

/// Whether `error` is the read-only session refusing a write: Tabletist's
/// own guard, or the server's refusal. PostgreSQL and MySQL say SQLSTATE
/// 25006; SQLite says SQLITE_READONLY (8) and no more. Its extended codes
/// keep the 8 in their low byte and are not a refused write: a journal to
/// recover, a lock or a directory it cannot have, which a SELECT can meet.
pub fn refuses_writes(error: &tabletist_db::Error, driver: tabletist_db::Driver) -> bool {
    use tabletist_db::{Driver, Error};
    match error {
        Error::Refused { .. } => true,
        Error::Query {
            code: Some(code), ..
        } => match driver {
            Driver::Sqlite => code == "8",
            Driver::Postgres | Driver::MySql => code == "25006",
        },
        _ => false,
    }
}

/// A row as tab-separated values on one line.
pub fn tsv_row(row: &[Value]) -> String {
    row.iter()
        .map(|value| plain_text(value).replace(['\t', '\n', '\r'], " "))
        .collect::<Vec<_>>()
        .join("\t")
}

pub fn elapsed(duration: Duration) -> String {
    if duration < Duration::from_secs(1) {
        format!("{} ms", duration.as_millis())
    } else {
        format!("{:.1} s", duration.as_secs_f64())
    }
}

/// Longest line the row panel lays out.
pub const DISPLAY_LINE_CHARS: usize = 500;
/// Most text the row panel shows after "Show all".
pub const DISPLAY_MAX_BYTES: usize = 256 * 1024;

/// Text as the row panel lays it out: at most DISPLAY_MAX_BYTES, with no
/// line longer than DISPLAY_LINE_CHARS. egui's layout of one very long line
/// is slow enough to freeze the window, so long lines are broken for display
/// only; copying still gives the whole value. Hidden characters are written
/// out as in `display_safe`; line breaks and tabs stay.
pub fn for_display(text: &str) -> String {
    let mut cut = text.len().min(DISPLAY_MAX_BYTES);
    while !text.is_char_boundary(cut) {
        cut -= 1;
    }
    let mut out = String::with_capacity(cut + cut / DISPLAY_LINE_CHARS + 64);
    let mut escape = String::new();
    // A written-out character is longer than the one it stands for, so the
    // limit is checked on what is shown as well.
    let mut full = false;
    'lines: for (index, line) in text[..cut].split('\n').enumerate() {
        if index > 0 {
            out.push('\n');
        }
        // A CR before the LF is part of the line break.
        let line = line.strip_suffix('\r').unwrap_or(line);
        let mut count = 0;
        for character in line.chars() {
            if out.len() >= DISPLAY_MAX_BYTES {
                full = true;
                break 'lines;
            }
            escape.clear();
            if character != '\t' && is_hidden(character) {
                push_escaped(&mut escape, character);
            } else {
                escape.push(character);
            }
            for shown in escape.chars() {
                if count == DISPLAY_LINE_CHARS {
                    out.push('\n');
                    count = 0;
                }
                out.push(shown);
                count += 1;
            }
        }
    }
    if full || cut < text.len() {
        out.push_str("\n…\n(Copy gives the whole value.)");
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_refused_write_is_told_from_other_errors() {
        use super::refuses_writes;
        use tabletist_db::{Driver, Error};
        let coded = |code: &str| Error::Query {
            code: Some(code.into()),
            message: "no".into(),
            detail: None,
            hint: None,
        };
        let refused = Error::Refused {
            line: 1,
            what: "COMMIT".into(),
        };
        for driver in [Driver::Postgres, Driver::MySql, Driver::Sqlite] {
            assert!(refuses_writes(&refused, driver));
            assert!(!refuses_writes(&Error::Timeout, driver));
        }
        // SQLSTATE 25006: read_only_sql_transaction.
        assert!(refuses_writes(&coded("25006"), Driver::Postgres));
        assert!(refuses_writes(&coded("25006"), Driver::MySql));
        assert!(!refuses_writes(&coded("42703"), Driver::Postgres));
        // SQLITE_READONLY is 8: what a write on the read-only session gets.
        assert!(refuses_writes(&coded("8"), Driver::Sqlite));
        assert!(!refuses_writes(&coded("1"), Driver::Sqlite));
        // Its extended codes keep the 8 in their low byte and are other
        // troubles, which a SELECT can meet: RECOVERY, CANTLOCK, ROLLBACK,
        // DBMOVED, CANTINIT and DIRECTORY.
        for extended in ["264", "520", "776", "1032", "1288", "1544"] {
            assert!(
                !refuses_writes(&coded(extended), Driver::Sqlite),
                "{extended}"
            );
        }
        // A SQLSTATE whose number ends in the same byte is not SQLite's code.
        assert!(!refuses_writes(&coded("23048"), Driver::Postgres));
    }

    #[test]
    fn a_message_is_cut_where_a_database_says_too_much() {
        use super::{MESSAGE_MAX_CHARS, capped};
        assert_eq!(capped("no such column: x"), "no such column: x");
        let exact = "é".repeat(MESSAGE_MAX_CHARS);
        assert_eq!(capped(&exact), exact.as_str());
        // Cut between characters, never inside one.
        let long = "é".repeat(MESSAGE_MAX_CHARS + 1);
        assert_eq!(capped(&long), format!("{exact}…"));
    }

    #[test]
    fn timestamps_drop_their_fraction_and_keep_their_zone() {
        use super::to_the_second;
        assert_eq!(
            to_the_second("2026-01-12 09:14:03.482915"),
            "2026-01-12 09:14:03"
        );
        assert_eq!(
            to_the_second("2026-01-12 09:14:03.4+02"),
            "2026-01-12 09:14:03+02"
        );
        assert_eq!(to_the_second("09:14:03.5"), "09:14:03");
        assert_eq!(to_the_second("2026-01-12"), "2026-01-12");
        assert_eq!(to_the_second("3.14"), "3.14");
    }

    #[test]
    fn hex_colours_are_recognised_in_every_length() {
        use super::hex_color;
        assert_eq!(hex_color("#3a7bd5"), Some([0x3a, 0x7b, 0xd5, 255]));
        assert_eq!(hex_color("#3a7bd580"), Some([0x3a, 0x7b, 0xd5, 0x80]));
        assert_eq!(hex_color("#FFF"), Some([255, 255, 255, 255]));
        assert_eq!(hex_color("#f80"), Some([0xff, 0x88, 0x00, 255]));
        assert_eq!(hex_color("#f808"), Some([0xff, 0x88, 0x00, 0x88]));
        for not in [
            "",
            "#",
            "3a7bd5",
            "#3a",
            "#3a7bd",
            "#3a7bd5f",
            "#3a7bd5ff0",
            "#ggg",
            " #fff",
            "#fff ",
            "#é12",
        ] {
            assert_eq!(hex_color(not), None, "{not:?}");
        }
    }

    #[test]
    fn the_middle_gives_way_first() {
        let width = |text: &str| text.chars().count() as f32;
        assert_eq!(super::ellipsize_middle("short", 10.0, width), "short");
        assert_eq!(
            super::ellipsize_middle("abcdefghij.jpg", 9.0, width),
            "abcd….jpg"
        );
    }

    use super::*;
    use std::time::Duration;
    use tabletist_db::Value;

    fn text(value: &str) -> Value {
        Value::Text(value.into())
    }

    #[test]
    fn scalars_render_plainly_and_null_says_so() {
        assert_eq!(cell_text(&Value::Null), "NULL");
        assert_eq!(cell_text(&Value::Bool(true)), "true");
        assert_eq!(cell_text(&Value::Int(-42)), "-42");
        assert_eq!(cell_text(&Value::Float(99.5)), "99.5");
        assert_eq!(cell_text(&text("Zoë 🚀")), "Zoë 🚀");
    }

    #[test]
    fn long_and_multiline_text_is_one_short_line() {
        let long = "x".repeat(10_000_000);
        let value = text(&long);
        let cell = cell_text(&value);
        assert_eq!(cell.chars().count(), CELL_MAX_CHARS + 1);
        assert!(cell.ends_with('…'));
        assert_eq!(cell_text(&text("a\nb\tc\r\nd")), "a b c  d");
    }

    #[test]
    fn a_cell_keeps_its_line_breaks_in_sight() {
        let marks = Marks::DESIGN;
        assert_eq!(
            cell_line("Night Train\nPart One", marks),
            "Night Train↵Part One"
        );
        // A Windows line end is one break; a tab is a space.
        assert_eq!(cell_line("a\r\nb\rc\td", marks), "a↵b↵c d");
        assert_eq!(cell_line("a\nb", Marks::PLAIN), "a¶b");
        // Other hidden characters are still written out, and plain text is
        // not copied.
        assert_eq!(cell_line("a\u{200B}b", marks), "a<U+200B>b");
        assert!(matches!(cell_line("plain", marks), Cow::Borrowed(_)));
        let long = "y".repeat(CELL_MAX_CHARS + 10);
        assert!(cell_line(&long, marks).ends_with('…'));
    }

    #[test]
    fn text_with_nothing_to_see_says_what_it_is() {
        let marks = Marks::DESIGN;
        assert_eq!(blank_text("", marks).as_deref(), Some("''"));
        assert_eq!(blank_text("  ", marks).as_deref(), Some("␣␣"));
        assert_eq!(blank_text(" \t\r\n", marks).as_deref(), Some("␣␣↵"));
        assert_eq!(blank_text("  ", Marks::PLAIN).as_deref(), Some("··"));
        // Leading spaces are the text's own: nothing stands in for them.
        assert_eq!(blank_text("   Leading spaces kept", marks), None);
        assert_eq!(blank_text("a", marks), None);
        // A megabyte of spaces is cut like any text, without reading it all.
        let spaces = " ".repeat(1_000_000);
        let shown = blank_text(&spaces, marks).unwrap();
        assert_eq!(shown.chars().count(), CELL_MAX_CHARS + 1);
        assert!(shown.ends_with('…'));
    }

    #[test]
    fn the_marks_fall_back_when_a_font_lacks_the_designs() {
        assert_eq!(Marks::pick(|_| true), Marks::DESIGN);
        assert_eq!(Marks::pick(|character| character != '↵'), Marks::PLAIN);
        assert_eq!(Marks::pick(|_| false), Marks::PLAIN);
    }

    #[test]
    fn arrays_are_told_by_their_type() {
        assert!(is_array("_text", ValueKind::Other));
        assert!(is_array("int4[]", ValueKind::Other));
        // A type the app knows is never an array, whatever its name.
        assert!(!is_array("_text", ValueKind::Text));
        assert!(!is_array("_", ValueKind::Other));
        assert!(!is_array("int4range", ValueKind::Other));
        assert_eq!(type_label("_text", ValueKind::Other), "text[]");
        assert_eq!(type_label("text[]", ValueKind::Other), "text[]");
        assert_eq!(type_label("int8", ValueKind::Numeric), "int8");
    }

    fn items(text: &str) -> Option<(Vec<String>, bool)> {
        array_items(text).map(|array| {
            let items = array.items.iter().map(|item| item.to_string()).collect();
            (items, array.cut)
        })
    }

    #[test]
    fn an_arrays_elements_are_read_as_postgres_writes_them() {
        let some =
            |list: &[&str], cut| Some((list.iter().map(|item| (*item).to_owned()).collect(), cut));
        assert_eq!(items("{en,fr}"), some(&["en", "fr"], false));
        assert_eq!(items("{}"), some(&[], false));
        assert_eq!(items("{1}"), some(&["1"], false));
        // Quotes go, and what they escape stays.
        assert_eq!(
            items(r#"{"two words",NULL,"a \"b\" c\\d","x,y}"}"#),
            some(&["two words", "NULL", r#"a "b" c\d"#, "x,y}"], false)
        );
        assert_eq!(items("{\"Ærø\",東京}"), some(&["Ærø", "東京"], false));
        // An array in an array stays as it is written.
        assert_eq!(
            items(r#"{{1,2},{"a}",b}}"#),
            some(&["{1,2}", r#"{"a}",b}"#], false)
        );
    }

    #[test]
    fn a_cut_array_keeps_the_elements_that_are_whole() {
        let some = |list: &[&str]| {
            Some((
                list.iter()
                    .map(|item| (*item).to_owned())
                    .collect::<Vec<_>>(),
                true,
            ))
        };
        assert_eq!(items("{en,fr,d…"), some(&["en", "fr"]));
        assert_eq!(items("{en,fr,"), some(&["en", "fr"]));
        assert_eq!(items("{en,fr"), some(&["en"]));
        assert_eq!(items(r#"{en,"fr…"#), some(&["en"]));
        assert_eq!(items("{{1,2},{3…"), some(&["{1,2}"]));
        assert_eq!(items("{"), some(&[]));
    }

    #[test]
    fn text_that_is_no_array_is_left_alone() {
        for text in ["", "en,fr", "[0:1]={a,b}", "{a}b", "{,a}", "{a,,b}", "{a}}"] {
            assert_eq!(items(text), None, "{text:?}");
        }
    }

    #[test]
    fn short_text_is_not_copied() {
        let value = text("borrowed");
        assert!(matches!(cell_text(&value), std::borrow::Cow::Borrowed(_)));
    }

    #[test]
    fn blobs_show_their_size_in_the_grid() {
        assert_eq!(cell_text(&Value::Bytes(vec![0; 3].into())), "BLOB · 3 B");
        assert_eq!(
            cell_text(&Value::Bytes(vec![0; 1536].into())),
            "BLOB · 1.5 KB"
        );
    }

    #[test]
    fn sixteen_bytes_read_as_a_uuid_everywhere() {
        let key = Value::Bytes(
            vec![
                0x01, 0x99, 0xa3, 0xf2, 0x7c, 0x1e, 0x7a, 0xbc, 0x8d, 0xef, 0x01, 0x23, 0x45, 0x67,
                0x89, 0xab,
            ]
            .into(),
        );
        let uuid = "0199a3f2-7c1e-7abc-8def-0123456789ab";
        assert_eq!(cell_text(&key), uuid);
        assert_eq!(plain_text(&key), uuid);
        assert_eq!(full_text(&key), uuid);
        assert_eq!(tsv_row(&[Value::Int(1), key]), format!("1\t{uuid}"));
        // Any other length stays a blob.
        for len in [15, 17] {
            let blob = Value::Bytes(vec![0xab; len].into());
            assert_eq!(cell_text(&blob), format!("BLOB · {len} B"));
            assert!(plain_text(&blob).starts_with("0xabab"));
            assert!(full_text(&blob).starts_with("ab ab ab"));
        }
        // The row panel shows the UUID as it shows text, with nothing to
        // unfold.
        let key = Value::Bytes(vec![0x01; 16].into());
        let field = field_text(&key);
        assert_eq!(field.short, "01010101-0101-0101-0101-010101010101");
        assert_eq!(field.full, None);
    }

    #[test]
    fn plain_text_is_the_whole_value() {
        let long = "y".repeat(1000);
        assert_eq!(plain_text(&text(&long)), long);
        assert_eq!(
            plain_text(&Value::Bytes(vec![0x00, 0xff, 0x10].into())),
            "0x00ff10"
        );
        assert_eq!(plain_text(&Value::Null), "NULL");
    }

    #[test]
    fn a_long_field_keeps_its_start_and_its_whole_text() {
        let lines: Vec<String> = (0..50).map(|n| format!("line {n}")).collect();
        let field = field_text(&text(&lines.join("\n")));
        assert_eq!(field.short.lines().count(), COLLAPSE_LINES);
        assert_eq!(
            field.full.as_deref().map(|full| full.lines().count()),
            Some(50)
        );
        let short = field_text(&text("hi"));
        assert_eq!(
            (short.short.as_str(), short.full, short.size.as_str()),
            ("hi", None, "2 B")
        );
        assert_eq!(short.characters, Some(2));
        assert_eq!(field_text(&Value::Int(42)).characters, None);
    }

    #[test]
    fn a_long_line_is_long_too_and_counted_in_characters() {
        let title = "é".repeat(5_000);
        let field = field_text(&text(&title));
        // Its start, broken into lines the panel can lay out.
        let start = field.short.chars().filter(|c| *c != '\n').count();
        assert_eq!(start, COLLAPSE_CHARS);
        assert_eq!(field.characters, Some(5_000));
        assert!(field.full.is_some());
    }

    #[test]
    fn a_binary_field_is_its_first_bytes() {
        let jpeg: Vec<u8> = [0xff, 0xd8, 0xff, 0xe0, 0x00, 0x10]
            .into_iter()
            .chain(std::iter::repeat_n(0xab, 49_352))
            .collect();
        let field = field_text(&Value::Bytes(jpeg.clone().into()));
        assert_eq!(
            field.short,
            "ff d8 ff e0 00 10 ab ab ab ab ab ab\nab ab ab ab ab ab ab ab ab ab ab ab"
        );
        assert_eq!((field.full, field.size.as_str()), (None, "48.2 KB"));
        assert_eq!(hex_preview(&[0x00, 0x0f]), "00 0f");
        assert_eq!(hex_preview(&[]), "");
        // A huge value is never read past its first bytes.
        let started = std::time::Instant::now();
        let huge = Value::Bytes(vec![0xab; 10 * 1024 * 1024].into());
        assert_eq!(field_text(&huge).short.len(), HEX_PREVIEW * 3 - 1);
        assert!(started.elapsed() < Duration::from_secs(1));
    }

    #[test]
    fn a_binary_value_is_guessed_by_how_it_starts() {
        assert_eq!(sniff(&[0xff, 0xd8, 0xff, 0xe0]), Some("JPEG"));
        assert_eq!(sniff(b"\x89PNG\r\n\x1a\n...."), Some("PNG"));
        assert_eq!(sniff(b"GIF89a.."), Some("GIF"));
        assert_eq!(sniff(b"%PDF-1.7"), Some("PDF"));
        assert_eq!(sniff(b"PK\x03\x04"), Some("ZIP"));
        assert_eq!(sniff(b"RIFF\x10\0\0\0WEBPVP8 "), Some("WebP"));
        assert_eq!(sniff(b"RIFF\x10\0\0\0WAVE"), None);
        assert_eq!(sniff(b""), None);
        assert_eq!(sniff(b"plain text"), None);
    }

    #[test]
    fn a_binary_cell_says_its_type_and_size() {
        assert_eq!(binary_label("bytea", 49_358, false), "bytea · 48.2 KB");
        assert_eq!(binary_label("bytea", 49_358, true), "bytea 48.2K");
        assert_eq!(binary_label("BLOB", 0, false), "BLOB · 0 B");
        assert_eq!(binary_label("BLOB", 0, true), "BLOB 0B");
        assert_eq!(binary_label("", 3, false), "binary · 3 B");
        assert_eq!(terse_size(5 * 1024 * 1024), "5.0M");
    }

    #[test]
    fn sizes_and_counts_are_compact() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(2048), "2.0 KB");
        assert_eq!(human_size(5 * 1024 * 1024), "5.0 MB");
        assert_eq!(compact_count(999), "999");
        assert_eq!(compact_count(1_234), "1.2K");
        assert_eq!(compact_count(1_000), "1K");
        assert_eq!(compact_count(1_250_000), "1.3M");
        assert_eq!(compact_count(7_000_000_000), "7B");
    }

    #[test]
    fn range_labels_describe_the_page() {
        assert_eq!(
            range_label(0, 300, true, Some(1_200_000), None).as_deref(),
            Some("1–300 of ~1.2M")
        );
        assert_eq!(
            range_label(0, 300, true, None, None).as_deref(),
            Some("1–300")
        );
        assert_eq!(
            range_label(300, 50, false, Some(10), None).as_deref(),
            Some("301–350 of 350")
        );
        assert_eq!(
            range_label(0, 5, false, None, None).as_deref(),
            Some("1–5 of 5")
        );
        assert_eq!(range_label(0, 0, false, None, None), None);
    }

    #[test]
    fn tsv_rows_keep_one_line_per_row() {
        let row = vec![Value::Int(1), text("a\tb\nc"), Value::Null];
        assert_eq!(tsv_row(&row), "1\ta b c\tNULL");
    }

    #[test]
    fn elapsed_times_read_naturally() {
        assert_eq!(elapsed(Duration::from_millis(84)), "84 ms");
        assert_eq!(elapsed(Duration::from_millis(1234)), "1.2 s");
    }

    #[test]
    fn a_huge_cell_is_cut_without_scanning_the_whole_value() {
        let huge = text(&"z".repeat(10 * 1024 * 1024));
        let started = std::time::Instant::now();
        for _ in 0..20 {
            assert!(cell_text(&huge).ends_with('…'));
        }
        assert!(
            started.elapsed() < std::time::Duration::from_millis(20),
            "20 visible cells took {:?}",
            started.elapsed()
        );
    }

    #[test]
    fn displayed_text_has_short_lines_and_a_bounded_size() {
        let shown = for_display(&"q".repeat(1_000_000));
        assert!(
            shown
                .lines()
                .all(|line| line.chars().count() <= DISPLAY_LINE_CHARS)
        );
        // The limit, the display-only line breaks, and the closing note.
        let bound = DISPLAY_MAX_BYTES + DISPLAY_MAX_BYTES / DISPLAY_LINE_CHARS + 64;
        assert!(shown.len() <= bound, "{}", shown.len());
        assert!(shown.ends_with("(Copy gives the whole value.)"));
        assert_eq!(for_display("short\nlines"), "short\nlines");
    }

    #[test]
    fn a_bidi_override_is_written_out_not_obeyed() {
        assert_eq!(display_safe("users_\u{202E}atad"), "users_<U+202E>atad");
        assert_eq!(
            cell_text(&text("Total: \u{202E}00.0001")),
            "Total: <U+202E>00.0001"
        );
        for character in ['\u{202A}', '\u{202D}', '\u{2066}', '\u{2069}', '\u{061C}'] {
            let shown = display_safe(&format!("a{character}b")).into_owned();
            assert_eq!(shown, format!("a<U+{:04X}>b", u32::from(character)));
        }
    }

    #[test]
    fn invisible_characters_make_names_look_different() {
        assert_ne!(display_safe("users\u{200B}"), display_safe("users"));
        assert_eq!(display_safe("us\u{200D}ers"), "us<U+200D>ers");
        assert_eq!(display_safe("\u{FEFF}users"), "<U+FEFF>users");
        assert_eq!(display_safe("soft\u{00AD}hyphen"), "soft<U+00AD>hyphen");
        assert_eq!(display_safe("tag\u{E0041}"), "tag<U+E0041>");
    }

    #[test]
    fn controls_in_a_name_are_written_out_and_cells_keep_their_spaces() {
        assert_eq!(
            display_safe("a\nb\tc\u{7}\u{7F}\u{85}"),
            "a<U+000A>b<U+0009>c<U+0007><U+007F><U+0085>"
        );
        assert_eq!(cell_text(&text("a\nb\u{0}c")), "a b<U+0000>c");
    }

    #[test]
    fn mixed_scripts_and_emoji_are_left_alone() {
        for name in ["Zoë 🚀", "مرحبا", "日本語", "naïve_café", "a-b.c"] {
            assert!(matches!(display_safe(name), Cow::Borrowed(_)), "{name}");
        }
        assert_eq!(
            display_safe("שלום\u{202C}Zoë\u{2067}"),
            "שלום<U+202C>Zoë<U+2067>"
        );
    }

    #[test]
    fn a_plain_name_is_not_copied() {
        assert!(matches!(display_safe("users"), Cow::Borrowed(_)));
        assert!(matches!(display_safe(""), Cow::Borrowed(_)));
    }

    #[test]
    fn a_long_name_is_cut_without_scanning_it_all() {
        let long = "n".repeat(10 * 1024 * 1024);
        let started = std::time::Instant::now();
        for _ in 0..20 {
            let shown = display_safe(&long);
            assert_eq!(shown.chars().count(), NAME_MAX_CHARS + 1);
            assert!(shown.ends_with('…'));
        }
        assert!(
            started.elapsed() < std::time::Duration::from_millis(20),
            "20 names took {:?}",
            started.elapsed()
        );
        // Hidden characters past the cut are never looked at.
        let tail = format!("{}\u{202E}", "n".repeat(NAME_MAX_CHARS));
        assert!(!display_safe(&tail).contains("<U+"));
    }

    #[test]
    fn the_row_panel_writes_out_hidden_characters_and_keeps_lines() {
        assert_eq!(
            for_display("a\u{202E}b\nc\td\r\ne\rf"),
            "a<U+202E>b\nc\td\ne<U+000D>f"
        );
        // Written-out characters still keep to the size limit.
        let shown = for_display(&"\u{0}".repeat(DISPLAY_MAX_BYTES));
        let bound = DISPLAY_MAX_BYTES + DISPLAY_MAX_BYTES / DISPLAY_LINE_CHARS + 64;
        assert!(shown.len() <= bound, "{}", shown.len());
        assert!(shown.ends_with("(Copy gives the whole value.)"));
    }

    #[test]
    fn copying_keeps_hidden_characters() {
        let value = text("users_\u{202E}atad");
        assert_eq!(plain_text(&value), "users_\u{202E}atad");
        assert_eq!(tsv_row(&[value]), "users_\u{202E}atad");
    }

    #[test]
    fn an_exact_total_replaces_the_estimate() {
        assert_eq!(
            range_label(0, 300, true, Some(90_000), Some(123_456)).as_deref(),
            Some("1–300 of 123,456")
        );
        assert_eq!(
            range_label(0, 300, true, Some(90_000), None).as_deref(),
            Some("1–300 of ~90K")
        );
        assert_eq!(group_digits(0), "0");
        assert_eq!(group_digits(999), "999");
        assert_eq!(group_digits(1_234_567), "1,234,567");
    }

    /// A language whose catalog marks every message it translates.
    #[derive(Clone, Copy)]
    struct Marked;

    struct Brackets;

    impl fastframe_i18n::Translator for Brackets {
        fn translate<'a>(&'a self, string: &'a str, _: Option<&'a str>) -> Cow<'a, str> {
            Cow::Owned(format!("[{string}]"))
        }

        fn ntranslate<'a>(
            &'a self,
            _: u64,
            _: &'a str,
            plural: &'a str,
            _: Option<&'a str>,
        ) -> Cow<'a, str> {
            Cow::Owned(format!("[{plural}]"))
        }
    }

    impl fastframe_i18n::Locale for Marked {
        fn catalog(self) -> Option<&'static dyn fastframe_i18n::Translator> {
            Some(&Brackets)
        }
    }

    #[test]
    fn error_sentences_are_translated_and_server_words_are_not() {
        use tabletist_db::{Error, SshStage};
        let ssh = |stage| Error::Ssh {
            stage,
            message: "host key".into(),
        };
        for error in [
            Error::Connect("x".into()),
            Error::Auth("x".into()),
            Error::Timeout,
            Error::ConnectionLost("x".into()),
            Error::Tls("x".into()),
            ssh(SshStage::Connect),
            ssh(SshStage::Auth),
            ssh(SshStage::Secret),
            ssh(SshStage::Forward),
        ] {
            let described = describe_error(Marked, &error);
            assert!(
                described.starts_with('[') && described.ends_with(']'),
                "{error:?}: {described}"
            );
        }
        let unknown = ssh(SshStage::HostKeyUnknown {
            host: "bastion".into(),
            port: 22,
            fingerprint: "SHA256:abc".into(),
        });
        assert_eq!(describe_error(Marked, &unknown), "host key");
        assert_eq!(
            describe_error(Marked, &Error::query("syntax error at \"x\"")),
            "syntax error at \"x\""
        );
    }
}
