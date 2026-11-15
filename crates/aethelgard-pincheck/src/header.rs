//! The cross-check's reader: `VAL_GPIOx_MODER`, `VAL_GPIOx_AFRL` and
//! `VAL_GPIOx_AFRH` from ArduPilot's generated `hwdef.h`, the source the ELF's
//! `gpio_default_config` is compiled from (`board.c:108`, `AET-WO-4400` S2).
//!
//! It is strict: each of the 33 macros is defined exactly once and named
//! nowhere else in the file, and its value is `(T | T | ...)` with exactly one
//! term per pin of the register, each `PIN_MODE_<MODE>(nU)` or
//! `PIN_AFIO_AF(nU, mU)`. Anything else is refused (`INV-06`).

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

/// The value text of `#define <name> <value>`, its continuation lines joined.
fn definition(text: &str, name: &str) -> Result<String, Refusal> {
    let mentions = text
        .match_indices(name)
        .filter(|&(at, _)| {
            let word = |c: char| c.is_ascii_alphanumeric() || c == '_';
            let before = text[..at].chars().next_back();
            let after = text[at + name.len()..].chars().next();
            !before.is_some_and(word) && !after.is_some_and(word)
        })
        .count();
    let lines: Vec<&str> = text.lines().collect();
    let defines: Vec<usize> = (0..lines.len())
        .filter(|&i| {
            lines[i]
                .strip_prefix("#define ")
                .and_then(|rest| rest.strip_prefix(name))
                .is_some_and(|rest| rest.starts_with([' ', '\t']))
        })
        .collect();
    let start = match defines.as_slice() {
        [one] if mentions == 1 => *one,
        [] => return refuse(name, "not defined"),
        [_] => {
            return refuse(
                name,
                format!("named {mentions} times; only its definition may name it"),
            );
        }
        many => return refuse(name, format!("defined {} times", many.len())),
    };
    let mut value = lines[start]["#define ".len() + name.len()..].to_owned();
    let mut at = start;
    while let Some(head) = value.strip_suffix('\\') {
        value = head.to_owned();
        at += 1;
        let Some(next) = lines.get(at) else {
            return refuse(name, "a continuation runs past the end of the file");
        };
        value.push(' ');
        value.push_str(next);
    }
    Ok(value)
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

fn moder(text: &str, port: char) -> Result<u32, Refusal> {
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

fn afr(text: &str, port: char, high: bool) -> Result<u32, Refusal> {
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
