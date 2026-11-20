// AET-WO-3306 §1.1, scratch: for each run directory, each I2C step check (kind 12) and witness
// (kind 13) by index, then each device's whole sequence (its witnesses' `observed`, in record
// order, against their `expected`), classified:
//   equal    — the whole observed sequence is the whole expected one;
//   empty    — nothing observed;
//   prefix   — observed is a strict prefix of expected (a transfer cut short);
//   surplus  — every line is a `Write`/`Read` of the device's, and the expected transactions
//              (a `Write` and the `Read`s after it) are a subsequence of the observed ones, whole:
//              one or more transactions its script does not hold;
//   other    — anything else (a wrong line, a warning, a shorter or reordered sequence);
//   unread   — some witness's observed is unreadable (`None`).
// Only `surplus` can move under the whole-sequence rule (AET-WO-3306 §1.2).
use aethelgard_chaos::record::Record;

/// `lines` as transactions: a `Write` line opens one, and each `Read` line after it belongs to it; a
/// `Read` with no `Write` before it opens one of its own. `None` if any line is not a `Write` or
/// `Read` of `device` (a warning, an error).
fn transactions<'l>(lines: &'l [String], device: &str) -> Option<Vec<&'l [String]>> {
    let p = format!("[NOISY] {device}: ");
    let mut starts = Vec::new();
    for (i, l) in lines.iter().enumerate() {
        let rest = l.strip_prefix(&p)?;
        if rest.starts_with("Write ") || (rest.starts_with("Read ") && i == 0) {
            starts.push(i);
        } else if !rest.starts_with("Read ") {
            return None;
        }
    }
    starts.push(lines.len());
    Some(starts.windows(2).map(|w| &lines[w[0]..w[1]]).collect())
}

fn class(expected: &[String], observed: &[String], device: &str) -> (&'static str, Vec<String>) {
    if observed == expected {
        return ("equal", vec![]);
    }
    if observed.is_empty() {
        return ("empty", vec![]);
    }
    if observed.len() < expected.len() && expected.starts_with(observed) {
        return ("prefix", vec![]);
    }
    let (Some(want), Some(got)) = (transactions(expected, device), transactions(observed, device))
    else {
        return ("other", vec![]);
    };
    // Greedy subsequence over whole transactions: the earliest observed one equal to each expected one.
    let mut extra = Vec::new();
    let mut e = 0;
    for t in &got {
        if e < want.len() && *t == want[e] {
            e += 1;
        } else {
            extra.extend(t.iter().cloned());
        }
    }
    if e == want.len() && !extra.is_empty() {
        ("surplus", extra)
    } else {
        ("other", extra)
    }
}

fn main() {
    for d in std::env::args().skip(1) {
        let r = match aethelgard_chaos::archive::replay_dir(std::path::Path::new(&d)) {
            Ok(r) => r,
            Err(e) => {
                println!("== {d}\nREPLAY-REFUSED {e:?}");
                continue;
            }
        };
        println!("== {d}");
        let mut devices: Vec<(String, String, Vec<String>, Option<Vec<String>>)> = Vec::new();
        for (i, rec) in r.state.records.iter().enumerate() {
            match rec {
                Record::I2cCheck { .. } => println!("[{i}] kind 12: {rec:?}"),
                Record::I2cWitness {
                    reference,
                    device,
                    expected,
                    observed,
                    ..
                } => {
                    println!("[{i}] kind 13: {rec:?}");
                    match devices.iter_mut().find(|d| d.0 == *reference) {
                        Some(d) => {
                            d.2.extend(expected.iter().cloned());
                            d.3 = match (d.3.take(), observed) {
                                (Some(mut a), Some(b)) => {
                                    a.extend(b.iter().cloned());
                                    Some(a)
                                }
                                _ => None,
                            };
                        }
                        None => devices.push((
                            reference.clone(),
                            device.clone(),
                            expected.clone(),
                            observed.clone(),
                        )),
                    }
                }
                _ => {}
            }
        }
        for (reference, device, expected, observed) in &devices {
            match observed {
                None => println!("DEVICE {reference} unread"),
                Some(o) => {
                    let (c, extra) = class(expected, o, device);
                    println!("DEVICE {reference} {c} expected={expected:?} observed={o:?} extra={extra:?}");
                }
            }
        }
    }
}
