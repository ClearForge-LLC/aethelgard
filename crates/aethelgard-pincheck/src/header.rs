//! The cross-check's reader: `VAL_GPIOx_MODER`, `VAL_GPIOx_AFRL` and
//! `VAL_GPIOx_AFRH` from ArduPilot's generated `hwdef.h`, the source the ELF's
//! `gpio_default_config` is compiled from (`board.c:108`, `AET-WO-4400` S2).
//!
//! It is strict: each of the 33 macros is defined exactly once and named
//! nowhere else in the file, and its value is `(T | T | ...)` with exactly one
//! term per pin of the register, each `PIN_MODE_<MODE>(nU)` or
//! `PIN_AFIO_AF(nU, mU)`. Anything else is refused (`INV-06`).
//!
//! It reads what the preprocessor sees (`AET-WO-1603` §1.4): continuations
//! spliced, comments removed, `#if 0` and `#if 1` evaluated exactly, and any
//! other conditional around one of the 33 refused, never read through.

use crate::{PORTS, Refusal};

/// `MODER`, `AFRL` and `AFRH` of one port, as `hwdef.h` defines them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct HeaderPort {
    pub moder: u32,
    pub afrl: u32,
    pub afrh: u32,
}

fn refuse<T>(macro_name: &str, message: impl Into<String>) -> Result<T, Refusal> {
    Err(Refusal::Header {
        name: macro_name.to_owned(),
        message: message.into(),
    })
}

/// Where a line of `hwdef.h` stands for the preprocessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    /// Compiled: outside every conditional, or in a branch known taken.
    Live,
    /// Skipped: in `#if 0`, or in a branch after one known taken.
    Dead,
    /// In a branch whose condition is not exactly `0` or `1`.
    Unknown,
}

/// One `#if` group: what its current branch is, and whether an earlier
/// branch is known taken (`Some(true)`), known not (`Some(false)`), or
/// neither (`None`).
struct Group {
    branch: State,
    taken: Option<bool>,
    seen_else: bool,
}

/// A branch condition, exactly: `0` is dead, `1` live, anything else unknown.
fn condition(arg: &str) -> (State, Option<bool>) {
    match arg {
        "0" => (State::Dead, Some(false)),
        "1" => (State::Live, Some(true)),
        _ => (State::Unknown, None),
    }
}

fn malformed<T>(line: usize, message: impl Into<String>) -> Result<T, Refusal> {
    refuse(&format!("line {line}"), message)
}

/// What the preprocessor sees of `text` (`AET-WO-1603` §1.4), one logical
/// line each, with the physical line it starts on: continuations spliced (a
/// backslash ending a line; one followed by blanks, which GCC splices with a
/// warning, is refused), then comments replaced by a space, then each line placed by the
/// conditionals around it. A directive is `#` or `%:`, blanks allowed on
/// either side. A dead line's text is dropped.
fn logical(text: &str) -> Result<Vec<(usize, State, String)>, Refusal> {
    // Splice.
    let mut spliced: Vec<(usize, String)> = Vec::new();
    let mut open: Option<(usize, String)> = None;
    for (i, raw) in text.split('\n').enumerate() {
        let (start, mut acc) = open.take().unwrap_or((i + 1, String::new()));
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        // What the reader does not model is refused, never read
        // (adversarial A3): a byte outside printable ASCII and tab (a lone CR
        // ends a line for GCC, and Rust trims Unicode blanks GCC keeps), a
        // trigraph, a C++ raw string, and `__has_include`'s header-name.
        if let Some(c) = line
            .chars()
            .find(|&c| c != '\t' && !(' '..='~').contains(&c))
        {
            return malformed(
                i + 1,
                format!("the character {c:?}: hwdef.h is printable ASCII, tabs and line ends"),
            );
        }
        if let Some(what) = ["??", "R\"", "__has_include"]
            .into_iter()
            .find(|w| line.contains(w))
        {
            return malformed(
                i + 1,
                format!(
                    "`{what}`: a trigraph, raw string or header-name the reader does not model"
                ),
            );
        }
        let trimmed = line.trim_end_matches([' ', '\t']);
        if trimmed.len() < line.len() && trimmed.ends_with('\\') {
            return malformed(
                i + 1,
                "a backslash followed by blanks: GCC splices it with a warning, the cross-check refuses it",
            );
        }
        match line.strip_suffix('\\') {
            Some(head) => {
                acc.push_str(head);
                open = Some((start, acc));
            }
            None => {
                acc.push_str(line);
                spliced.push((start, acc));
            }
        }
    }
    if let Some((start, _)) = open {
        return malformed(start, "a continuation runs past the end of the file");
    }
    // Strip comments; a string or character literal ends at its line's end.
    // A comment is one space, so a line a comment spans continues to its
    // end: whatever follows `*/` belongs to the line the comment began on.
    let mut in_block: Option<usize> = None;
    let mut pending: Option<(usize, String)> = None;
    let mut stripped = Vec::with_capacity(spliced.len());
    for (line_n, line) in spliced {
        let (n, mut out) = pending
            .take()
            .unwrap_or_else(|| (line_n, String::with_capacity(line.len())));
        let mut quote: Option<char> = None;
        let mut chars = line.chars().peekable();
        while let Some(c) = chars.next() {
            if in_block.is_some() {
                if c == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    in_block = None;
                    out.push(' ');
                }
                continue;
            }
            match quote {
                Some(q) => {
                    out.push(c);
                    if c == '\\' {
                        out.extend(chars.next());
                    } else if c == q {
                        quote = None;
                    }
                }
                None if c == '/' && chars.peek() == Some(&'*') => {
                    chars.next();
                    in_block = Some(n);
                }
                None if c == '/' && chars.peek() == Some(&'/') => {
                    out.push(' ');
                    break;
                }
                None => {
                    if c == '"' || c == '\'' {
                        quote = Some(c);
                    }
                    out.push(c);
                }
            }
        }
        if in_block.is_some() {
            pending = Some((n, out));
        } else {
            stripped.push((n, out));
        }
    }
    if let Some(n) = in_block {
        return malformed(n, "a comment is never closed");
    }
    // Place each line.
    let mut groups: Vec<Group> = Vec::new();
    let state = |groups: &[Group]| {
        if groups.iter().any(|g| g.branch == State::Dead) {
            State::Dead
        } else if groups.iter().any(|g| g.branch == State::Unknown) {
            State::Unknown
        } else {
            State::Live
        }
    };
    let mut out = Vec::with_capacity(stripped.len());
    for (n, line) in stripped {
        let directive = line
            .trim_start()
            .strip_prefix('#')
            .or_else(|| line.trim_start().strip_prefix("%:"))
            .map(str::trim_start);
        let word = directive.map(|d| {
            let end = d
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(d.len());
            (&d[..end], d[end..].trim())
        });
        let here = match word {
            Some(("if" | "ifdef" | "ifndef", arg)) => {
                let here = state(&groups);
                let (branch, taken) = match word {
                    Some(("if", _)) => condition(arg),
                    _ => (State::Unknown, None),
                };
                groups.push(Group {
                    branch,
                    taken,
                    seen_else: false,
                });
                here
            }
            Some((kind @ ("elif" | "elifdef" | "elifndef" | "else" | "endif"), arg)) => {
                let Some(g) = groups.pop() else {
                    return malformed(n, format!("#{kind} without #if"));
                };
                let here = state(&groups);
                if kind == "endif" {
                    out.push((n, here, line));
                    continue;
                }
                if g.seen_else {
                    return malformed(n, format!("#{kind} after #else"));
                }
                let (cond, cond_taken) = match kind {
                    "else" => (State::Live, Some(true)),
                    "elif" => condition(arg),
                    _ => (State::Unknown, None),
                };
                let (branch, taken) = match g.taken {
                    Some(true) => (State::Dead, Some(true)),
                    Some(false) => (cond, cond_taken),
                    None if cond == State::Dead => (State::Dead, None),
                    None => (State::Unknown, None),
                };
                groups.push(Group {
                    branch,
                    taken,
                    seen_else: kind == "else",
                });
                here
            }
            // Another file, or another line number, is a view the reader
            // does not follow (adversarial A8).
            Some((kind @ ("include" | "include_next" | "import" | "line"), _)) => {
                return malformed(n, format!("#{kind}: the reader reads this file alone"));
            }
            _ => state(&groups),
        };
        out.push((n, here, line));
    }
    if !groups.is_empty() {
        return malformed(text.lines().count(), "an #if is never closed by #endif");
    }
    Ok(out)
}

/// The text after `name` when `line` is `#define <name>` followed by a blank
/// or the end, with any blanks GCC allows around `#` and `define`
/// (`AET-WO-4400b2` adversarial A1: ArduPilot writes a `define` line's own
/// spacing through).
fn defines<'a>(line: &'a str, name: &str) -> Option<&'a str> {
    let blank = [' ', '\t'];
    let rest = line
        .trim_start_matches(blank)
        .strip_prefix('#')?
        .trim_start_matches(blank)
        .strip_prefix("define")?;
    if !rest.starts_with(blank) {
        return None;
    }
    let after = rest.trim_start_matches(blank).strip_prefix(name)?;
    (after.is_empty() || after.starts_with(blank)).then_some(after)
}

/// The value text of `#define <name> <value>`, as the preprocessor sees it
/// (`lines`, from [`logical`]): defined exactly once in a live line, named in
/// no other line it compiles, and named in no line it may or may not
/// compile (`HwdefConditionalUnsupported`, `AET-WO-1603` §1.4).
fn definition(lines: &[(usize, State, String)], name: &str) -> Result<String, Refusal> {
    let mentions = |text: &str| {
        text.match_indices(name)
            .filter(|&(at, _)| {
                let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
                let before = text[..at].chars().next_back();
                let after = text[at + name.len()..].chars().next();
                !before.is_some_and(word) && !after.is_some_and(word)
            })
            .count()
    };
    if let Some((n, _, _)) = lines
        .iter()
        .find(|(_, s, l)| *s == State::Unknown && mentions(l) > 0)
    {
        return Err(Refusal::Conditional {
            name: name.to_owned(),
            line: *n,
        });
    }
    let live: Vec<&str> = lines
        .iter()
        .filter(|(_, s, _)| *s == State::Live)
        .map(|(_, _, l)| l.as_str())
        .collect();
    let named: usize = live.iter().map(|l| mentions(l)).sum();
    let defines: Vec<&str> = live
        .iter()
        .filter_map(|l| defines(l, name))
        .filter(|rest| !rest.is_empty())
        .collect();
    match defines.as_slice() {
        [one] if named == 1 => Ok((*one).to_owned()),
        [] => refuse(name, "not defined"),
        [_] => refuse(
            name,
            format!("named {named} times; only its definition may name it"),
        ),
        many => refuse(name, format!("defined {} times", many.len())),
    }
}

/// `n` from `nU`, at most `max`.
fn number(s: &str, max: u32) -> Option<u32> {
    let digits = s.trim().strip_suffix('U')?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) || digits.len() > 2 {
        return None;
    }
    digits.parse().ok().filter(|&n| n <= max)
}

/// The terms of `(T | T | ...)`.
fn terms<'a>(name: &str, value: &'a str) -> Result<Vec<&'a str>, Refusal> {
    let Some(inner) = value
        .trim()
        .strip_prefix('(')
        .and_then(|v| v.strip_suffix(')'))
    else {
        return refuse(name, "the value is not one parenthesised expression");
    };
    Ok(inner.split('|').map(str::trim).collect())
}

fn moder(text: &[(usize, State, String)], port: char) -> Result<u32, Refusal> {
    let name = format!("VAL_GPIO{port}_MODER");
    let value = definition(text, &name)?;
    let mut seen = [false; 16];
    let mut out = 0u32;
    for term in terms(&name, &value)? {
        let parsed = [
            ("INPUT", 0u32),
            ("OUTPUT", 1),
            ("ALTERNATE", 2),
            ("ANALOG", 3),
        ]
        .into_iter()
        .find_map(|(mode, bits)| {
            let arg = term
                .strip_prefix("PIN_MODE_")?
                .strip_prefix(mode)?
                .strip_prefix('(')?
                .strip_suffix(')')?;
            Some((number(arg, 15)?, bits))
        });
        let Some((pin, bits)) = parsed else {
            return refuse(
                &name,
                format!("`{term}` is not PIN_MODE_<MODE>(nU) for a pin 0-15"),
            );
        };
        if std::mem::replace(&mut seen[pin as usize], true) {
            return refuse(&name, format!("pin {pin} is given twice"));
        }
        out |= bits << (2 * pin);
    }
    if let Some(pin) = seen.iter().position(|s| !s) {
        return refuse(&name, format!("pin {pin} is not given"));
    }
    Ok(out)
}

fn afr(text: &[(usize, State, String)], port: char, high: bool) -> Result<u32, Refusal> {
    let name = format!("VAL_GPIO{port}_{}", if high { "AFRH" } else { "AFRL" });
    let value = definition(text, &name)?;
    let base = if high { 8 } else { 0 };
    let mut seen = [false; 8];
    let mut out = 0u32;
    for term in terms(&name, &value)? {
        let parsed = term
            .strip_prefix("PIN_AFIO_AF(")
            .and_then(|t| t.strip_suffix(')'))
            .and_then(|args| args.split_once(','))
            .and_then(|(pin, af)| Some((number(pin, 15)?, number(af, 15)?)));
        let Some((pin, af)) = parsed.filter(|&(pin, _)| (base..base + 8).contains(&pin)) else {
            return refuse(
                &name,
                format!(
                    "`{term}` is not PIN_AFIO_AF(nU, mU) for a pin {base}-{}",
                    base + 7
                ),
            );
        };
        let slot = (pin - base) as usize;
        if std::mem::replace(&mut seen[slot], true) {
            return refuse(&name, format!("pin {pin} is given twice"));
        }
        out |= af << (4 * slot);
    }
    if let Some(slot) = seen.iter().position(|s| !s) {
        return refuse(&name, format!("pin {} is not given", base as usize + slot));
    }
    Ok(out)
}

/// Reads the three registers of every port.
pub fn read(text: &str) -> Result<[HeaderPort; PORTS.len()], Refusal> {
    let text = logical(text)?;
    let text = text.as_slice();
    let mut out = [HeaderPort::default(); PORTS.len()];
    for (slot, &port) in out.iter_mut().zip(PORTS.iter()) {
        *slot = HeaderPort {
            moder: moder(text, port)?,
            afrl: afr(text, port, false)?,
            afrh: afr(text, port, true)?,
        };
    }
    Ok(out)
}

/// `hwdef.h` as the preprocessor sees it, for the identity judge
/// (`AET-WO-4400b2`): the same strict reading as the cross-check's.
pub struct Preprocessed(Vec<(usize, State, String)>);

/// Reads `text` as [`read`] does: continuations spliced, comments removed,
/// `#if 0` and `#if 1` evaluated, any other conditional kept as unknown.
pub fn preprocess(text: &str) -> Result<Preprocessed, Refusal> {
    Ok(Preprocessed(logical(text)?))
}

impl Preprocessed {
    /// The value of `#define <name>`, or `None` when no compiled line defines
    /// it. Refused when it is defined twice, or named inside a conditional
    /// the reader does not evaluate.
    pub fn value(&self, name: &str) -> Result<Option<String>, Refusal> {
        let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
        let mentions = |text: &str| {
            text.match_indices(name).any(|(at, _)| {
                !text[..at].chars().next_back().is_some_and(word)
                    && !text[at + name.len()..].chars().next().is_some_and(word)
            })
        };
        if let Some((n, _, _)) = self
            .0
            .iter()
            .find(|(_, s, l)| *s == State::Unknown && mentions(l))
        {
            return Err(Refusal::Conditional {
                name: name.to_owned(),
                line: *n,
            });
        }
        let defines: Vec<&str> = self
            .0
            .iter()
            .filter(|(_, s, _)| *s == State::Live)
            .filter_map(|(_, _, l)| defines(l, name))
            .collect();
        match defines.as_slice() {
            [] => Ok(None),
            [one] => Ok(Some(one.trim().to_owned())),
            many => refuse(name, format!("defined {} times", many.len())),
        }
    }

    /// Every definition of `name` the preprocessor may compile: in a live
    /// line, or in a branch whose condition the reader does not evaluate
    /// (ArduPilot's `HAL_I2Cn_CONFIG` has one per DMA branch). A caller reads
    /// only what every one of them agrees on.
    pub fn candidates(&self, name: &str) -> Vec<String> {
        self.0
            .iter()
            .filter(|(_, s, _)| *s != State::Dead)
            .filter_map(|(_, _, l)| defines(l, name))
            .map(|rest| rest.trim().to_owned())
            .collect()
    }
}
