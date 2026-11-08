//! KiCad's S-expressions: a reader for files that may be hostile (a board
//! under judgement) and a writer whose output is a function of the tree.
//!
//! A string is double-quoted; inside it `\"` and `\\` are escapes, and `\n`,
//! `\r`, `\t` are read as controls. That is all the quoting KiCad's reader
//! needs, and all [`quote`] writes (`AET-WO-1300` S5: every character
//! `RUL-14` admits survives it).

use std::fmt::Write as _;

/// Deeper nesting than any KiCad file uses is refused, so a hostile file
/// cannot exhaust the stack of anything that walks the tree.
pub const MAX_DEPTH: usize = 256;

/// More tokens (lists, atoms and strings) than any board this engine emits
/// or judges is refused, which bounds the reader's memory: a 16 MiB board
/// of `(a)` cost 430 MiB with a cap on lists alone (`AET-WO-1300`
/// adversarial pass, F4).
pub const MAX_TOKENS: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sx {
    List(Vec<Sx>),
    Atom(String),
    Str(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SexprError {
    #[error("unbalanced `)` at byte {0}")]
    UnbalancedClose(usize),
    #[error("{0} list(s) left open at the end")]
    Unclosed(usize),
    #[error("unterminated string starting at byte {0}")]
    UnterminatedString(usize),
    #[error("a string starting at byte {0} is not UTF-8")]
    NotUtf8(usize),
    #[error("nesting deeper than {MAX_DEPTH} at byte {0}")]
    TooDeep(usize),
    #[error("expected exactly one top-level list, found {0} items")]
    NotOneList(usize),
    #[error("more than {MAX_TOKENS} tokens")]
    TooManyTokens,
    #[error(
        "byte {0} is a control or non-ASCII byte outside a string, which KiCad reads differently"
    )]
    ForbiddenByte(usize),
    #[error("a string starting at byte {0} holds a raw line break or control byte")]
    ControlInString(usize),
    #[error("the line at byte {0} starts with `#`, which KiCad skips as a comment")]
    CommentLine(usize),
    #[error("a `\"` at byte {0} touches an atom, which KiCad reads as one token")]
    QuoteInAtom(usize),
}

impl Sx {
    pub fn atom(a: impl Into<String>) -> Sx {
        Sx::Atom(a.into())
    }
    pub fn str(s: impl Into<String>) -> Sx {
        Sx::Str(s.into())
    }
    /// `(head args...)` with every arg an atom.
    pub fn node(head: &str, args: &[&str]) -> Sx {
        let mut v = vec![Sx::atom(head)];
        v.extend(args.iter().map(|a| Sx::atom(*a)));
        Sx::List(v)
    }
    pub fn head(&self) -> Option<&str> {
        match self {
            Sx::List(v) => match v.first() {
                Some(Sx::Atom(a)) => Some(a),
                _ => None,
            },
            _ => None,
        }
    }
    pub fn items(&self) -> &[Sx] {
        match self {
            Sx::List(v) => v,
            _ => &[],
        }
    }
    pub fn items_mut(&mut self) -> Option<&mut Vec<Sx>> {
        match self {
            Sx::List(v) => Some(v),
            _ => None,
        }
    }
    /// The text of an atom or a string.
    pub fn text(&self) -> Option<&str> {
        match self {
            Sx::Atom(a) | Sx::Str(a) => Some(a),
            Sx::List(_) => None,
        }
    }
    /// The `i`th element's text.
    pub fn arg(&self, i: usize) -> Option<&str> {
        self.items().get(i).and_then(Sx::text)
    }
    /// Direct children whose head is `head`.
    pub fn children<'a>(&'a self, head: &'a str) -> impl Iterator<Item = &'a Sx> + 'a {
        self.items().iter().filter(move |c| c.head() == Some(head))
    }
    pub fn child<'a>(&'a self, head: &str) -> Option<&'a Sx> {
        self.items().iter().find(|c| c.head() == Some(head))
    }
    /// `(property "Name" "value" ...)`'s value.
    pub fn property(&self, name: &str) -> Option<&str> {
        self.children("property")
            .find(|p| p.arg(1) == Some(name))
            .and_then(|p| p.arg(2))
    }
}

enum Token<'a> {
    Open(usize),
    Close(usize),
    Atom(&'a str),
    Str(String),
}

/// The tokens of `src`, each list bracket with its byte offset.
fn tokens(src: &str) -> impl Iterator<Item = Result<Token<'_>, SexprError>> {
    let bytes = src.as_bytes();
    let mut pos = 0;
    std::iter::from_fn(move || {
        while pos < bytes.len() && matches!(bytes[pos], b' ' | b'\t' | b'\n' | b'\r') {
            pos += 1;
        }
        let start = pos;
        let c = *bytes.get(pos)?;
        pos += 1;
        Some(match c {
            b'(' => Ok(Token::Open(start)),
            b')' => Ok(Token::Close(start)),
            b'"' => {
                let mut s = Vec::new();
                loop {
                    let Some(&b) = bytes.get(pos) else {
                        return Some(Err(SexprError::UnterminatedString(start)));
                    };
                    pos += 1;
                    match b {
                        b'"' => break,
                        b'\\' => {
                            let Some(&e) = bytes.get(pos) else {
                                return Some(Err(SexprError::UnterminatedString(start)));
                            };
                            pos += 1;
                            s.push(match e {
                                b'n' => b'\n',
                                b'r' => b'\r',
                                b't' => b'\t',
                                other => other,
                            });
                        }
                        other => s.push(other),
                    }
                }
                String::from_utf8(s)
                    .map(Token::Str)
                    .map_err(|_| SexprError::NotUtf8(start))
            }
            _ => {
                while pos < bytes.len() && !b"()\" \t\n\r".contains(&bytes[pos]) {
                    pos += 1;
                }
                Ok(Token::Atom(&src[start..pos]))
            }
        })
    })
}

/// Refuses every byte sequence KiCad 10.0.7's reader could read differently
/// from this one (`AET-WO-1300` adversarial pass, measured in the tools
/// image): a control byte outside a string (KiCad treats NUL as whitespace),
/// a non-ASCII byte outside a string, a raw line break or control byte inside
/// a string, a line whose first non-blank byte is `#` (KiCad skips it), and a
/// `"` that touches an atom (KiCad keeps it in the token). What remains is
/// read the same way by both, so a scan of it is a scan of what KiCad reads.
pub fn lexically_plain(src: &str) -> Result<(), SexprError> {
    let bytes = src.as_bytes();
    let separator = |b: u8| matches!(b, b' ' | b'\t' | b'\n' | b'\r' | b'(' | b')');
    let mut line_start = true;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if line_start && b == b'#' {
            return Err(SexprError::CommentLine(i));
        }
        match b {
            b'\n' | b'\r' => line_start = true,
            b' ' | b'\t' => {}
            b'"' => {
                if i > 0 && !separator(bytes[i - 1]) {
                    return Err(SexprError::QuoteInAtom(i));
                }
                let start = i;
                i += 1;
                loop {
                    let Some(&c) = bytes.get(i) else {
                        return Err(SexprError::UnterminatedString(start));
                    };
                    match c {
                        b'"' => break,
                        b'\\' => {
                            i += 1;
                            if bytes.get(i).is_some_and(|&e| e < 0x20 || e == 0x7f) {
                                return Err(SexprError::ControlInString(start));
                            }
                        }
                        c if c < 0x20 && c != b'\t' => {
                            return Err(SexprError::ControlInString(start));
                        }
                        0x7f => return Err(SexprError::ControlInString(start)),
                        _ => {}
                    }
                    i += 1;
                }
                if let Some(&next) = bytes.get(i + 1)
                    && !separator(next)
                {
                    return Err(SexprError::QuoteInAtom(i));
                }
                line_start = false;
            }
            0x21..=0x7e => line_start = false,
            _ => return Err(SexprError::ForbiddenByte(i)),
        }
        i += 1;
    }
    Ok(())
}

/// Reads one top-level list, iteratively, refusing nesting past
/// [`MAX_DEPTH`].
pub fn parse(src: &str) -> Result<Sx, SexprError> {
    lexically_plain(src)?;
    let mut count = 0usize;
    let mut stack: Vec<Vec<Sx>> = vec![Vec::new()];
    for token in tokens(src) {
        let token = token?;
        count += 1;
        if count > MAX_TOKENS {
            return Err(SexprError::TooManyTokens);
        }
        match token {
            Token::Open(at) => {
                if stack.len() > MAX_DEPTH {
                    return Err(SexprError::TooDeep(at));
                }
                stack.push(Vec::new());
            }
            Token::Close(at) => {
                if stack.len() < 2 {
                    return Err(SexprError::UnbalancedClose(at));
                }
                let done = stack.pop().unwrap_or_default();
                if let Some(parent) = stack.last_mut() {
                    parent.push(Sx::List(done));
                }
            }
            Token::Atom(a) => push(&mut stack, Sx::Atom(a.to_owned())),
            Token::Str(s) => push(&mut stack, Sx::Str(s)),
        }
    }
    if stack.len() != 1 {
        return Err(SexprError::Unclosed(stack.len() - 1));
    }
    let mut top = stack.pop().unwrap_or_default();
    match (top.len(), top.pop()) {
        (1, Some(list @ Sx::List(_))) => Ok(list),
        (n, _) => Err(SexprError::NotOneList(n)),
    }
}

fn push(stack: &mut [Vec<Sx>], item: Sx) {
    if let Some(top) = stack.last_mut() {
        top.push(item);
    }
}

/// The byte span `[start, end)` of each direct child list of the top-level
/// list, and of the top-level list itself. Call after [`parse`] succeeded.
/// A byte range `[start, end)` of the source.
pub type Span = (usize, usize);

pub fn top_level_spans(src: &str) -> Result<(Vec<Span>, Span), SexprError> {
    let mut depth = 0usize;
    let mut open = 0;
    let mut root = (0, 0);
    let mut spans = Vec::new();
    for token in tokens(src) {
        match token? {
            Token::Open(at) => {
                depth += 1;
                match depth {
                    1 => root.0 = at,
                    2 => open = at,
                    _ => {}
                }
            }
            Token::Close(at) => {
                match depth {
                    1 => root.1 = at + 1,
                    2 => spans.push((open, at + 1)),
                    0 => return Err(SexprError::UnbalancedClose(at)),
                    _ => {}
                }
                depth -= 1;
            }
            Token::Atom(_) | Token::Str(_) => {}
        }
    }
    Ok((spans, root))
}

/// KiCad's string quoting: `"` and `\` escaped, controls as `\n` `\r` `\t`.
pub fn quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Tab-indented, a list on one line when every element is an atom or a
/// string, the shape KiCad itself writes. A function of the tree alone.
pub fn write(sx: &Sx) -> String {
    let mut out = String::new();
    write_into(sx, 0, &mut out);
    out.push('\n');
    out
}

fn write_into(sx: &Sx, depth: usize, out: &mut String) {
    match sx {
        Sx::Atom(a) => out.push_str(a),
        Sx::Str(s) => out.push_str(&quote(s)),
        Sx::List(v) => {
            out.push('(');
            let flat = v.iter().all(|c| !matches!(c, Sx::List(_)));
            for (i, c) in v.iter().enumerate() {
                if i > 0 {
                    if flat || !matches!(c, Sx::List(_)) {
                        out.push(' ');
                    } else {
                        out.push('\n');
                        for _ in 0..=depth {
                            out.push('\t');
                        }
                    }
                }
                write_into(c, depth + 1, out);
            }
            if !flat && v.len() > 1 && matches!(v.last(), Some(Sx::List(_))) {
                out.push('\n');
                for _ in 0..depth {
                    out.push('\t');
                }
            }
            out.push(')');
        }
    }
}

/// Millimetres as text from nanometres: no exponent, no trailing zeros.
pub fn mm(nm: i64) -> String {
    let sign = if nm < 0 { "-" } else { "" };
    let a = nm.unsigned_abs();
    let (whole, frac) = (a / 1_000_000, a % 1_000_000);
    if frac == 0 {
        return format!("{sign}{whole}");
    }
    let mut f = String::new();
    let _ = write!(f, "{frac:06}");
    while f.ends_with('0') {
        f.pop();
    }
    format!("{sign}{whole}.{f}")
}

/// Millimetre text to nanometres, exactly; at most six decimals.
pub fn nm(text: &str) -> Option<i64> {
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    if frac.len() > 6 || (whole.is_empty() && frac.is_empty()) {
        return None;
    }
    let w: i64 = if whole.is_empty() {
        0
    } else {
        whole.parse().ok()?
    };
    let f: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<6}").parse().ok()?
    };
    let v = w.checked_mul(1_000_000)?.checked_add(f)?;
    Some(if neg { -v } else { v })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting_round_trips_every_identifier_character() {
        let all: String = (0x21u8..=0x7e).map(char::from).collect();
        let text = format!("(x {})", quote(&all));
        assert_eq!(parse(&text).unwrap().arg(1), Some(all.as_str()));
    }

    #[test]
    fn refuses_what_is_not_one_balanced_list() {
        assert!(matches!(parse("(a))"), Err(SexprError::UnbalancedClose(3))));
        assert!(matches!(parse("(a"), Err(SexprError::Unclosed(1))));
        assert!(matches!(
            parse("(a \"b)"),
            Err(SexprError::UnterminatedString(3))
        ));
        assert!(matches!(parse("(a) (b)"), Err(SexprError::NotOneList(2))));
        assert!(matches!(parse("a"), Err(SexprError::NotOneList(1))));
        assert!(matches!(parse(""), Err(SexprError::NotOneList(0))));
    }

    #[test]
    fn refuses_nesting_past_the_cap_without_recursing() {
        let deep = format!("{}{}", "(".repeat(MAX_DEPTH + 1), ")".repeat(MAX_DEPTH + 1));
        assert!(matches!(parse(&deep), Err(SexprError::TooDeep(_))));
        let ok = format!("{}{}", "(".repeat(MAX_DEPTH), ")".repeat(MAX_DEPTH));
        assert!(parse(&ok).is_ok());
        let huge = "(".repeat(1_000_000);
        assert!(matches!(parse(&huge), Err(SexprError::TooDeep(_))));
    }

    #[test]
    fn spans_cover_each_top_level_child() {
        let src = "(k (a 1) \"s\" (b (c \")\")))";
        let (spans, root) = top_level_spans(src).unwrap();
        let got: Vec<&str> = spans.iter().map(|&(s, e)| &src[s..e]).collect();
        assert_eq!(got, ["(a 1)", "(b (c \")\"))"]);
        assert_eq!(&src[root.0..root.1], src);
    }

    #[test]
    fn millimetres_are_exact() {
        assert_eq!(mm(2_540_000), "2.54");
        assert_eq!(mm(-50_000), "-0.05");
        assert_eq!(mm(0), "0");
        assert_eq!(nm("2.54"), Some(2_540_000));
        assert_eq!(nm("-0.000001"), Some(-1));
        assert_eq!(nm("1.0000001"), None);
        assert_eq!(nm("x"), None);
        assert_eq!(nm("."), None);
    }
}
