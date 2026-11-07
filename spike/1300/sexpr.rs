//! A small S-expression reader and writer for KiCad files (spike only).
//!
//! KiCad's files are lists of atoms and double-quoted strings. A string
//! escapes `"` and `\` with a backslash, and `\n`, `\r`, `\t` as usual; that
//! is the whole quoting rule this writer applies (`quote`). Reading keeps the
//! atom/string distinction so a library item can be copied back verbatim.

use std::fmt::Write as _;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Sx {
    List(Vec<Sx>),
    Atom(String),
    Str(String),
}

impl Sx {
    pub fn list(items: Vec<Sx>) -> Sx {
        Sx::List(items)
    }
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

pub fn parse(src: &str) -> Result<Sx, String> {
    let bytes = src.as_bytes();
    let mut pos = 0;
    let mut stack: Vec<Vec<Sx>> = vec![Vec::new()];
    while pos < bytes.len() {
        let c = bytes[pos];
        match c {
            b'(' => {
                stack.push(Vec::new());
                pos += 1;
            }
            b')' => {
                let done = stack.pop().ok_or("unbalanced )")?;
                stack
                    .last_mut()
                    .ok_or_else(|| format!("unbalanced ) at byte {pos}"))?
                    .push(Sx::List(done));
                pos += 1;
            }
            b'"' => {
                pos += 1;
                let mut s = Vec::new();
                loop {
                    let Some(&b) = bytes.get(pos) else {
                        return Err("unterminated string".into());
                    };
                    pos += 1;
                    match b {
                        b'"' => break,
                        b'\\' => {
                            let Some(&e) = bytes.get(pos) else {
                                return Err("unterminated escape".into());
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
                let s = String::from_utf8(s).map_err(|e| e.to_string())?;
                stack.last_mut().ok_or("no list")?.push(Sx::Str(s));
            }
            b' ' | b'\t' | b'\n' | b'\r' => pos += 1,
            _ => {
                let start = pos;
                while pos < bytes.len() && !b"()\" \t\n\r".contains(&bytes[pos]) {
                    pos += 1;
                }
                stack
                    .last_mut()
                    .ok_or("no list")?
                    .push(Sx::Atom(src[start..pos].to_owned()));
            }
        }
    }
    let mut top = stack.pop().ok_or("empty")?;
    if !stack.is_empty() {
        return Err("unbalanced (".into());
    }
    if top.len() != 1 {
        return Err(format!("{} top-level items, expected 1", top.len()));
    }
    Ok(top.remove(0))
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

/// Tab-indented, one list per line unless every element is an atom or
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
    let whole = a / 1_000_000;
    let frac = a % 1_000_000;
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

/// Millimetre text to nanometres, exactly (KiCad files carry at most six
/// decimals).
pub fn nm(text: &str) -> Result<i64, String> {
    let (neg, body) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    let (whole, frac) = body.split_once('.').unwrap_or((body, ""));
    if frac.len() > 6 {
        return Err(format!("`{text}` has more than six decimals"));
    }
    let w: i64 = if whole.is_empty() {
        0
    } else {
        whole
            .parse()
            .map_err(|_| format!("`{text}` is not a number"))?
    };
    let f: i64 = if frac.is_empty() {
        0
    } else {
        format!("{frac:0<6}")
            .parse()
            .map_err(|_| format!("`{text}` is not a number"))?
    };
    let v = w * 1_000_000 + f;
    Ok(if neg { -v } else { v })
}
