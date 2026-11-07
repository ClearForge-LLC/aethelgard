//! The deterministic CBOR profile of `RUL-06`: RFC 8949 §4.2.1 restricted to
//! unsigned and negative integers, byte strings, UTF-8 text strings and
//! arrays, every length definite and every argument in its shortest form.
//!
//! The defining property: every byte string [`decode_exact`] accepts is the
//! encoding of the value it returns, so a trace has exactly one encoding.

use thiserror::Error;

/// An array holds at most this many elements.
pub const MAX_ARRAY: u64 = 65_536;
/// A byte string or text string holds at most this many bytes.
pub const MAX_STRING: u64 = 1 << 20;
/// Arrays nest at most this deep; the outermost array is depth 1.
pub const MAX_DEPTH: usize = 8;

/// A value of the profile. `Neg(n)` is the integer `-1 - n`, so the whole
/// range of major type 1 is representable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Uint(u64),
    Neg(u64),
    Bytes(Vec<u8>),
    Text(String),
    Array(Vec<Value>),
}

impl Value {
    /// The value of an `i64`, using major type 0 for non-negative numbers.
    pub fn from_i64(n: i64) -> Self {
        if n >= 0 {
            Self::Uint(n.unsigned_abs())
        } else {
            Self::Neg(!n.cast_unsigned())
        }
    }

    /// The value as an `i64`, if it is one.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Self::Uint(n) => i64::try_from(*n).ok(),
            Self::Neg(n) => i64::try_from(*n).ok().map(|n| !n),
            _ => None,
        }
    }
}

/// Why a byte string is outside the profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
pub enum CborErrorKind {
    #[error("input ends inside an item")]
    UnexpectedEnd,
    #[error("major type {0} is outside the profile")]
    UnsupportedMajor(u8),
    #[error("indefinite length is outside the profile")]
    IndefiniteLength,
    #[error("additional information {0} is reserved")]
    ReservedAdditionalInfo(u8),
    #[error("argument is not in its shortest form")]
    NotShortest,
    #[error("array of {0} elements exceeds the bound")]
    ArrayTooLong(u64),
    #[error("string of {0} bytes exceeds the bound")]
    StringTooLong(u64),
    #[error("nesting exceeds the bound")]
    TooDeep,
    #[error("text string is not valid UTF-8")]
    InvalidUtf8,
    #[error("bytes follow the item")]
    TrailingBytes,
}

/// A refusal. `offset` is the position of the first byte of the item that is
/// refused (for trailing bytes, of the first byte after the item).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Error)]
#[error("{kind} (byte offset {offset})")]
pub struct CborError {
    pub offset: usize,
    pub kind: CborErrorKind,
}

fn put_head(out: &mut Vec<u8>, major: u8, arg: u64) {
    let m = major << 5;
    if arg < 24 {
        out.push(m | arg as u8);
    } else if let Ok(a) = u8::try_from(arg) {
        out.extend([m | 24, a]);
    } else if let Ok(a) = u16::try_from(arg) {
        out.push(m | 25);
        out.extend(a.to_be_bytes());
    } else if let Ok(a) = u32::try_from(arg) {
        out.push(m | 26);
        out.extend(a.to_be_bytes());
    } else {
        out.push(m | 27);
        out.extend(arg.to_be_bytes());
    }
}

/// Append the encoding of `value`. Encoding never fails; the decoder's
/// bounds are the reader's, so a caller that exceeds them writes something
/// the reader will refuse.
pub fn encode(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Uint(n) => put_head(out, 0, *n),
        Value::Neg(n) => put_head(out, 1, *n),
        Value::Bytes(b) => {
            put_head(out, 2, b.len() as u64);
            out.extend(b);
        }
        Value::Text(t) => {
            put_head(out, 3, t.len() as u64);
            out.extend(t.as_bytes());
        }
        Value::Array(items) => {
            put_head(out, 4, items.len() as u64);
            for item in items {
                encode(item, out);
            }
        }
    }
}

/// The encoding of `value`.
pub fn to_bytes(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(value, &mut out);
    out
}

fn fail<T>(offset: usize, kind: CborErrorKind) -> Result<T, CborError> {
    Err(CborError { offset, kind })
}

/// Read an initial byte and its argument: `(major, argument, next offset)`.
fn head(bytes: &[u8], start: usize) -> Result<(u8, u64, usize), CborError> {
    let Some(&initial) = bytes.get(start) else {
        return fail(start, CborErrorKind::UnexpectedEnd);
    };
    let major = initial >> 5;
    let info = initial & 0x1f;
    let width = match info {
        0..=23 => return Ok((major, u64::from(info), start + 1)),
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        31 if (2..=4).contains(&major) => return fail(start, CborErrorKind::IndefiniteLength),
        _ => return fail(start, CborErrorKind::ReservedAdditionalInfo(info)),
    };
    let end = start + 1 + width;
    let Some(arg_bytes) = bytes.get(start + 1..end) else {
        return fail(start, CborErrorKind::UnexpectedEnd);
    };
    let arg = arg_bytes
        .iter()
        .fold(0u64, |acc, &b| (acc << 8) | u64::from(b));
    let least = match width {
        1 => 24,
        2 => 1 << 8,
        4 => 1 << 16,
        _ => 1 << 32,
    };
    if arg < least {
        return fail(start, CborErrorKind::NotShortest);
    }
    Ok((major, arg, end))
}

fn item(bytes: &[u8], start: usize, depth: usize) -> Result<(Value, usize), CborError> {
    // Major types 5, 6 and 7 are refused before their argument is looked at,
    // so a map's offset is the map's, whatever follows.
    if let Some(&initial) = bytes.get(start) {
        let major = initial >> 5;
        if major >= 5 {
            return fail(start, CborErrorKind::UnsupportedMajor(major));
        }
    }
    let (major, arg, next) = head(bytes, start)?;
    match major {
        0 => Ok((Value::Uint(arg), next)),
        1 => Ok((Value::Neg(arg), next)),
        2 | 3 => {
            if arg > MAX_STRING {
                return fail(start, CborErrorKind::StringTooLong(arg));
            }
            // arg <= 1 MiB, so the cast and the addition cannot overflow.
            let end = next + arg as usize;
            let Some(body) = bytes.get(next..end) else {
                return fail(start, CborErrorKind::UnexpectedEnd);
            };
            if major == 2 {
                Ok((Value::Bytes(body.to_vec()), end))
            } else {
                match std::str::from_utf8(body) {
                    Ok(text) => Ok((Value::Text(text.to_owned()), end)),
                    Err(_) => fail(start, CborErrorKind::InvalidUtf8),
                }
            }
        }
        _ => {
            if arg > MAX_ARRAY {
                return fail(start, CborErrorKind::ArrayTooLong(arg));
            }
            if depth + 1 > MAX_DEPTH {
                return fail(start, CborErrorKind::TooDeep);
            }
            // Every element takes at least one byte, so a length beyond the
            // remaining input is refused before anything is allocated.
            if arg > (bytes.len() - next) as u64 {
                return fail(start, CborErrorKind::UnexpectedEnd);
            }
            let mut items = Vec::with_capacity(arg as usize);
            let mut at = next;
            for _ in 0..arg {
                let (value, end) = item(bytes, at, depth + 1)?;
                items.push(value);
                at = end;
            }
            Ok((Value::Array(items), at))
        }
    }
}

/// Decode one item starting at `offset`; returns it and the offset after it.
pub fn decode_prefix(bytes: &[u8], offset: usize) -> Result<(Value, usize), CborError> {
    item(bytes, offset, 0)
}

/// Decode `bytes` as exactly one item, with nothing after it.
pub fn decode_exact(bytes: &[u8]) -> Result<Value, CborError> {
    let (value, end) = decode_prefix(bytes, 0)?;
    if end != bytes.len() {
        return fail(end, CborErrorKind::TrailingBytes);
    }
    Ok(value)
}
