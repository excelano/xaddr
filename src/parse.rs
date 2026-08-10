// The address grammar: text in, an unresolved spec out. No table required.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)
//
// Parsing is deliberately separate from resolution so a caller can validate as the user types,
// before there is anything to look the address up against.

use crate::error::{Error, Result};
use crate::letters::letter_to_col;
use core::ops::Range;

/// How a positional names its column: by letter, or by header name.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ColRef {
    /// `C`, `AF` — already the 0-based index, since letters need no table to resolve.
    Letters(usize),
    /// `[first name]` — needs a header row.
    Name(String),
}

/// One address atom, and which axes it pins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pos {
    /// `C`, `[first name]` — a whole column, every row.
    Column(ColRef),
    /// `5` — a whole row, every column. 0-based here, 1-based as written.
    Row(usize),
    /// `$` — the last row of the table, whatever it turns out to be.
    LastRow,
    /// `C5`, `[price]12` — a single cell. `row` is 0-based.
    Cell { col: ColRef, row: usize },
}

/// One comma-separated element of a spec.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Item {
    Single(Pos),
    /// `A:C`, `[first]:`, `:5`. An absent side runs to the table's edge; both absent is a
    /// syntax error, so at least one is always present.
    Range {
        start: Option<Pos>,
        end: Option<Pos>,
    },
}

/// A parsed address: an ordered list of items. Order is preserved and duplicates are kept —
/// the caller decides whether repetition is meaningful.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Spec {
    pub(crate) items: Vec<Item>,
    /// Byte span of each item in the source, parallel to `items`, for error reporting.
    pub(crate) spans: Vec<Range<usize>>,
}

impl Spec {
    pub fn items(&self) -> &[Item] {
        &self.items
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Parse an address spec. The table is not consulted — `[name]` stays a name and out-of-range
/// letters stay indices until [`Spec::columns`](crate::Spec::columns) or
/// [`Spec::cells`](crate::Spec::cells) resolves them.
pub fn parse(spec: &str) -> Result<Spec> {
    if spec.trim().is_empty() {
        return Err(Error::syntax("empty address", 0..spec.len()));
    }

    let mut items = Vec::new();
    let mut spans = Vec::new();
    for span in split_top_level(spec, ',')? {
        let span = trim_span(spec, span);
        if span.is_empty() {
            return Err(Error::syntax(
                "empty item in the address list — two commas with nothing between them",
                span,
            ));
        }
        items.push(parse_item(spec, span.clone())?);
        spans.push(span);
    }
    Ok(Spec { items, spans })
}

/// Parse one address off the front of `s`, returning it and how many bytes it consumed.
///
/// For parsers that embed addresses in a larger language, where an address has no delimiter
/// and simply ends when the next character cannot continue it — `A:C s/x/y/` is an address
/// followed by a command, and `[dept]~/ops/` is an address followed by an operator. Such a
/// parser cannot slice the address out before parsing it without knowing the grammar, which is
/// the thing it is delegating; this hands back the extent instead.
///
/// The returned [`Spec`] holds exactly one item, so it resolves like any other. Commas are
/// *not* consumed: a host language with its own list or union syntax keeps that for itself.
///
/// ```
/// let (spec, used) = xaddr::parse_prefix("A:C s/x/y/").unwrap();
/// assert_eq!(used, 3);
/// assert_eq!(&"A:C s/x/y/"[used..], " s/x/y/");
/// assert_eq!(spec.items().len(), 1);
/// ```
pub fn parse_prefix(s: &str) -> Result<(Spec, usize)> {
    let n = item_extent(s);
    if n == 0 {
        return Err(Error::syntax(
            "expected an address — a cell (C5), column (C or [name]), row (5), or $",
            0..0,
        ));
    }
    let span = 0..n;
    let item = parse_item(s, span.clone())?;
    Ok((
        Spec {
            items: vec![item],
            spans: vec![span],
        },
        n,
    ))
}

/// How many bytes at the front of `s` belong to one address: an optional leading `:`, a
/// positional, and at most one `:` with an optional positional after it. Mirrors what the
/// item parser will accept, so the slice it measures is the slice that parses.
fn item_extent(s: &str) -> usize {
    if s.as_bytes().first() == Some(&b':') {
        return pos_extent(s, 1).unwrap_or(1);
    }
    let Some(after_start) = pos_extent(s, 0) else {
        return 0;
    };
    if s.as_bytes().get(after_start) == Some(&b':') {
        let after_colon = after_start + 1;
        return pos_extent(s, after_colon).unwrap_or(after_colon);
    }
    after_start
}

/// The end of the positional beginning at `start`, or `None` if none begins there.
///
/// Byte indexing is safe throughout: every byte tested is ASCII, and the continuation bytes of
/// a multi-byte character inside a `[name]` are all `>= 0x80`, so none can be mistaken for a
/// delimiter. An unterminated `[` runs to the end of the string and is left for the parser to
/// report, which keeps the "what is an address" answer in one place.
fn pos_extent(s: &str, start: usize) -> Option<usize> {
    let b = s.as_bytes();
    let mut i = start;
    match *b.get(i)? {
        b'$' => Some(i + 1),
        b'[' => {
            i += 1;
            while i < b.len() {
                if b[i] == b']' {
                    if b.get(i + 1) == Some(&b']') {
                        i += 2;
                        continue;
                    }
                    i += 1;
                    break;
                }
                i += 1;
            }
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            Some(i)
        }
        c if c.is_ascii_alphabetic() => {
            while i < b.len() && b[i].is_ascii_alphabetic() {
                i += 1;
            }
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            Some(i)
        }
        c if c.is_ascii_digit() => {
            while i < b.len() && b[i].is_ascii_digit() {
                i += 1;
            }
            Some(i)
        }
        _ => None,
    }
}

fn parse_item(spec: &str, span: Range<usize>) -> Result<Item> {
    let parts: Vec<Range<usize>> = split_top_level(&spec[span.clone()], ':')?
        .into_iter()
        .map(|r| span.start + r.start..span.start + r.end)
        .collect();

    match parts.len() {
        0 | 1 => Ok(Item::Single(parse_pos(spec, span)?)),
        2 => {
            let lo = trim_span(spec, parts[0].clone());
            let hi = trim_span(spec, parts[1].clone());
            if lo.is_empty() && hi.is_empty() {
                return Err(Error::syntax(
                    "a range needs at least one end — write `A:` from A to the last column, \
                     `:C` from the first through C",
                    span,
                ));
            }
            let start = if lo.is_empty() {
                None
            } else {
                Some(parse_pos(spec, lo)?)
            };
            let end = if hi.is_empty() {
                None
            } else {
                Some(parse_pos(spec, hi)?)
            };
            Ok(Item::Range { start, end })
        }
        _ => Err(Error::syntax(
            "too many `:` in one address — a range has two ends",
            span,
        )),
    }
}

fn parse_pos(spec: &str, span: Range<usize>) -> Result<Pos> {
    let s = &spec[span.clone()];

    if s == "$" {
        return Ok(Pos::LastRow);
    }

    // `[name]` or `[name]12`
    if let Some(rest) = s.strip_prefix('[') {
        let (name, consumed) = parse_name(rest, span.start + 1)?;
        let tail = &rest[consumed..];
        let col = ColRef::Name(name);
        return finish_pos(col, tail, span.start + 1 + consumed);
    }

    // A run of letters, then optionally digits: `C`, `AF`, `C5`.
    let letters: String = s.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
    if !letters.is_empty() {
        let col = letter_to_col(&letters).ok_or_else(|| {
            Error::syntax(
                format!("column {letters} is past the largest column this addresses"),
                span.start..span.start + letters.len(),
            )
        })?;
        return finish_pos(
            ColRef::Letters(col),
            &s[letters.len()..],
            span.start + letters.len(),
        );
    }

    // Bare digits: a row, 1-based as written.
    if s.chars().all(|c| c.is_ascii_digit()) {
        let n: usize = s
            .parse()
            .map_err(|_| Error::syntax(format!("row {s} is too large a number"), span.clone()))?;
        if n == 0 {
            return Err(Error::syntax("row numbers start at 1, not 0", span));
        }
        return Ok(Pos::Row(n - 1));
    }

    Err(Error::syntax(
        format!(
            "unrecognized address {s:?} — use a column letter (C, AF), a bracketed name \
             ([first name]), a row number (5), a cell (C5), or $ for the last row"
        ),
        span,
    ))
}

/// Attach an optional row number to a column reference, giving a column or a cell.
fn finish_pos(col: ColRef, tail: &str, tail_at: usize) -> Result<Pos> {
    if tail.is_empty() {
        return Ok(Pos::Column(col));
    }
    if !tail.chars().all(|c| c.is_ascii_digit()) {
        return Err(Error::syntax(
            format!("unexpected {tail:?} after a column — a column takes only a row number"),
            tail_at..tail_at + tail.len(),
        ));
    }
    let n: usize = tail
        .parse()
        .map_err(|_| Error::syntax("row number is too large", tail_at..tail_at + tail.len()))?;
    if n == 0 {
        return Err(Error::syntax(
            "row numbers start at 1, not 0",
            tail_at..tail_at + tail.len(),
        ));
    }
    Ok(Pos::Cell { col, row: n - 1 })
}

/// Read a bracketed name body (everything after the opening `[`), returning the name and how
/// many bytes of `body` it consumed including the closing `]`. `]]` is an escaped literal `]`.
fn parse_name(body: &str, at: usize) -> Result<(String, usize)> {
    let chars: Vec<char> = body.chars().collect();
    let mut name = String::new();
    let mut i = 0;
    let mut bytes = 0;
    while i < chars.len() {
        if chars[i] == ']' {
            if chars.get(i + 1) == Some(&']') {
                name.push(']');
                bytes += 2;
                i += 2;
                continue;
            }
            return Ok((name, bytes + 1));
        }
        name.push(chars[i]);
        bytes += chars[i].len_utf8();
        i += 1;
    }
    Err(Error::syntax(
        "unterminated [name] — a bracketed column name needs a closing ]",
        at - 1..at + bytes,
    ))
}

/// Split on a delimiter that is not inside `[...]`, returning byte spans. `]]` inside a name
/// stays inside it.
///
/// Names do not nest, so this tracks *inside a name or not* rather than a depth. A `[` inside
/// a name is an ordinary character — a column really can be called `notes [draft]`, and
/// counting depth would leave that name looking unterminated at the end of the address.
fn split_top_level(s: &str, delim: char) -> Result<Vec<Range<usize>>> {
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut in_name = false;
    let chars: Vec<(usize, char)> = s.char_indices().collect();
    let mut i = 0;
    while i < chars.len() {
        let (at, c) = chars[i];
        if in_name {
            if c == ']' {
                if chars.get(i + 1).map(|(_, c)| *c) == Some(']') {
                    i += 2;
                    continue;
                }
                in_name = false;
            }
        } else if c == '[' {
            in_name = true;
        } else if c == delim {
            spans.push(start..at);
            start = at + c.len_utf8();
        }
        i += 1;
    }
    if in_name {
        return Err(Error::syntax(
            "unterminated [name] — a bracketed column name needs a closing ]",
            0..s.len(),
        ));
    }
    spans.push(start..s.len());
    Ok(spans)
}

/// Narrow a span past leading and trailing ASCII whitespace.
fn trim_span(s: &str, span: Range<usize>) -> Range<usize> {
    let slice = &s[span.clone()];
    let lead = slice.len() - slice.trim_start().len();
    let trail = slice.len() - slice.trim_end().len();
    if lead + trail >= slice.len() {
        return span.start + lead..span.start + lead;
    }
    span.start + lead..span.end - trail
}
