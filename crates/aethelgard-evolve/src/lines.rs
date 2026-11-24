//! A device's log lines read as the transactions its Renode model logs
//! (`AET-WO-3309`), one reader per form an I2C model the twin uses writes,
//! and each transaction placed in the script step it is.
//!
//! The judge (`renode.rs` `attribute`) places a device's lines among its
//! steps with [`place`], and attribution reads a step's witness with
//! [`transactions`]: both read a line the same way. A line in no form is no
//! transaction, and is never read as part of one.

/// How a device's model logs a transaction: the form of its script's lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Form {
    /// `I2C.BME280` (`BME280.cs` L38, L75): `[NOISY] {device}: Write {bytes}`
    /// opens a transaction, and each `Read {bytes}` after it is that
    /// transaction's. Each byte is hexadecimal, at most two digits, joined by
    /// single spaces.
    WriteRead,
    /// Our `Sensors.BMP581` (`BMP581.cs` `FinishTransmission`), one line per
    /// transaction: `write 0xRR <- 0xBB …` (or `<- (none)`) writes the pointer
    /// and the bytes after it; `read 0xRR -> 0xVV …` writes the pointer and
    /// reads the bytes (`AET-WO-3307`).
    Bmp581,
    /// `Sensors.ICP_101xx` at `Debug` (`twin` `Model::Icp101xx`):
    /// `[DEBUG] {device}: Received command: [0xHH, 0xLL]` writes a command,
    /// and `Reading data with CRC: [0xVV, …], …` after it reads.
    Icp,
}

/// One transaction: the positions of its lines, the bytes written (empty
/// for a read the log shows no pointer for), and the bytes read (`None` for
/// a write alone).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Transaction {
    pub lines: Vec<usize>,
    pub written: Vec<u8>,
    pub read: Option<Vec<u8>>,
}

/// What a `[NOISY]` line of `device`'s model says, after `{device}: `.
pub(crate) fn noisy<'l>(line: &'l str, device: &str) -> Option<&'l str> {
    line.strip_prefix("[NOISY] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")
}

/// A `Write`/`Read` line: whether it reads, and its bytes, at least one.
pub(crate) fn write_read(line: &str, device: &str) -> Option<(bool, Vec<u8>)> {
    let (what, rest) = noisy(line, device)?.split_once(' ')?;
    let read = match what {
        "Write" => false,
        "Read" => true,
        _ => return None,
    };
    let bytes = rest
        .split(' ')
        .map(|b| {
            (!b.is_empty() && b.len() <= 2 && b.bytes().all(|c| c.is_ascii_hexdigit()))
                .then(|| u8::from_str_radix(b, 16).ok())
                .flatten()
        })
        .collect::<Option<_>>()?;
    Some((read, bytes))
}

/// A BMP581 line: the bytes written (the pointer first) and the bytes read.
/// Each byte is `0x` and two upper-case digits, as `Join` writes it; the
/// pointer is the model's, 7 bits; the verb and the arrow pair; a read is at
/// least one byte. Any other line is none.
pub(crate) fn bmp581(line: &str, device: &str) -> Option<(Vec<u8>, Option<Vec<u8>>)> {
    let byte = |t: &str| {
        let d = t.strip_prefix("0x")?;
        (d.len() == 2 && d.bytes().all(|c| matches!(c, b'0'..=b'9' | b'A'..=b'F')))
            .then(|| u8::from_str_radix(d, 16).ok())
            .flatten()
    };
    let (verb, rest) = noisy(line, device)?.split_once(' ')?;
    let (pointer, rest) = rest.split_once(' ')?;
    let (arrow, rest) = rest.split_once(' ')?;
    let pointer = byte(pointer).filter(|&p| p <= 0x7F)?;
    let bytes: Vec<u8> = if rest == "(none)" {
        Vec::new()
    } else {
        rest.split(' ').map(byte).collect::<Option<_>>()?
    };
    match (verb, arrow) {
        ("write", "<-") => Some(([&[pointer], &bytes[..]].concat(), None)),
        ("read", "->") if !bytes.is_empty() => Some((vec![pointer], Some(bytes))),
        _ => None,
    }
}

/// An ICP-101xx line: whether it reads, and its bytes, at least one. Each
/// byte is `0x` and upper-case digits without padding, as the model's
/// `{0:X}` writes it (`twin` `pretty`), in `[…]` joined by `, `.
fn icp(line: &str, device: &str) -> Option<(bool, Vec<u8>)> {
    let rest = line
        .strip_prefix("[DEBUG] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")?;
    let list = |t: &str| -> Option<Vec<u8>> {
        t.strip_prefix('[')?
            .split(", ")
            .map(|b| {
                let d = b.strip_prefix("0x")?;
                (matches!(d.len(), 1 | 2)
                    && !(d.len() == 2 && d.starts_with('0'))
                    && d.bytes().all(|c| matches!(c, b'0'..=b'9' | b'A'..=b'F')))
                .then(|| u8::from_str_radix(d, 16).ok())
                .flatten()
            })
            .collect()
    };
    if let Some(t) = rest.strip_prefix("Received command: ") {
        return list(t.strip_suffix(']')?).map(|b| (false, b));
    }
    let (t, after) = rest
        .strip_prefix("Reading data with CRC: ")?
        .split_once("], current command: ")?;
    after.contains(", requested: ").then_some(())?;
    list(t).map(|b| (true, b))
}

/// The form of `device`'s script: our BMP581's if any line is one, else
/// `Write`/`Read` if any line is one, else the ICP-101xx's if any line is
/// one; `None` when no reader knows any line of it.
pub fn form<S: AsRef<str>>(script: &[S], device: &str) -> Option<Form> {
    let any = |f: &dyn Fn(&str) -> bool| script.iter().any(|l| f(l.as_ref()));
    if any(&|l| bmp581(l, device).is_some()) {
        Some(Form::Bmp581)
    } else if any(&|l| write_read(l, device).is_some()) {
        Some(Form::WriteRead)
    } else if any(&|l| icp(l, device).is_some()) {
        Some(Form::Icp)
    } else {
        None
    }
}

/// `lines` as `form`'s transactions, in order, and the position of each line
/// that is none (a warning, another form's line). A `Write` (or an ICP
/// command) stays open across a line that is none, so a `Read` after a model's
/// warning is still its transaction (`AET-WO-3309` D1); a read with nothing
/// open is a transaction of its own, with no pointer.
pub fn transactions<S: AsRef<str>>(
    lines: &[S],
    device: &str,
    form: Form,
) -> (Vec<Transaction>, Vec<usize>) {
    let mut out: Vec<Transaction> = Vec::new();
    let mut none = Vec::new();
    let mut open = false;
    for (i, line) in lines.iter().enumerate() {
        let line = line.as_ref();
        let part = match form {
            Form::Bmp581 => bmp581(line, device).map(|(written, read)| {
                out.push(Transaction {
                    lines: vec![i],
                    written,
                    read,
                });
            }),
            Form::WriteRead | Form::Icp => {
                let read_or_write = if form == Form::Icp {
                    icp(line, device)
                } else {
                    write_read(line, device)
                };
                read_or_write.map(|(read, bytes)| match (read, open) {
                    (false, _) => {
                        out.push(Transaction {
                            lines: vec![i],
                            written: bytes,
                            read: None,
                        });
                        open = true;
                    }
                    (true, true) => {
                        let t = out.last_mut().expect("an open transaction");
                        t.lines.push(i);
                        t.read.get_or_insert_with(Vec::new).extend(bytes);
                    }
                    (true, false) => {
                        out.push(Transaction {
                            lines: vec![i],
                            written: Vec::new(),
                            read: Some(bytes),
                        });
                        open = true;
                    }
                })
            }
        };
        if part.is_none() {
            none.push(i);
        }
    }
    (out, none)
}

/// The step of `steps` (each its bytes written and its read width) each line
/// of `lines` belongs to, by what it says (`AET-WO-3309` D1, ruled R3); `None`
/// for a line that is no transaction, or a transaction no step is.
/// - A transaction's key is its pointer (the ICP-101xx's two-byte command) and
///   its direction, a read or not. Each goes first to the step of the same
///   key, the k-th such transaction to the k-th such step and any later one to
///   the last: a repeated transaction joins its step's, which then fails.
/// - Then each one left, in order, goes to the first step of its direction,
///   in script order, that holds no transaction yet.
/// - Any other is placed nowhere: it is never another step's.
pub fn place<S: AsRef<str>>(
    steps: &[(&[u8], u8)],
    lines: &[S],
    device: &str,
    form: Form,
) -> Vec<Option<usize>> {
    let n = if form == Form::Icp { 2 } else { 1 };
    let key = |written: &[u8], read: bool| (written.get(..n).map(<[u8]>::to_vec), read);
    let (ts, _) = transactions(lines, device, form);
    let mut to: Vec<Option<usize>> = vec![None; ts.len()];
    let mut taken: Vec<usize> = vec![0; steps.len()];
    for (t, tx) in ts.iter().enumerate() {
        let k = key(&tx.written, tx.read.is_some());
        if k.0.is_none() {
            continue;
        }
        let same: Vec<usize> = (0..steps.len())
            .filter(|&s| key(steps[s].0, steps[s].1 > 0) == k)
            .collect();
        if let Some(&last) = same.last() {
            let s = same
                .iter()
                .copied()
                .find(|&s| taken[s] == 0)
                .unwrap_or(last);
            taken[s] += 1;
            to[t] = Some(s);
        }
    }
    for (t, tx) in ts.iter().enumerate() {
        if to[t].is_some() {
            continue;
        }
        let read = tx.read.is_some();
        if let Some(s) = (0..steps.len()).find(|&s| (steps[s].1 > 0) == read && taken[s] == 0) {
            taken[s] += 1;
            to[t] = Some(s);
        }
    }
    let mut owner = vec![None; lines.len()];
    for (tx, s) in ts.iter().zip(to) {
        for &i in &tx.lines {
            owner[i] = s;
        }
    }
    owner
}
