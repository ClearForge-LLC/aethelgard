// AET-WO-3309 §1.1, scratch: for each run directory, each I2C device's whole log (its witnesses'
// `observed`, concatenated in record order, as `evolve` reads it) re-placed among its steps by each
// candidate rule of (3), and `main`'s `aethelgard_evolve::attribute` on the records so rewritten, in
// memory. Also (1)'s byte counts, (2)'s BMP581 transactions against the script, and (4)'s stitch,
// each read from the archive as it is. One JSON line per run.
//
// Rules (spike/3309/predictions.md): a transaction's key is its pointer and direction (read if it reads).
//   R1: the device's step of the same key (the k-th such transaction to the k-th such step, later ones to
//       the last); else unplaced. R2: R1, else the device's one step of the transaction's direction.
//   Carrier A: an unplaced line goes to the step under way (the last whose first placed line came before
//       it, else the first step), as an `[ERROR]` goes today. Carrier B: it leaves kind 13.
// An `[ERROR]` line goes to the step under way under every rule, as today (D-A1).
use aethelgard_chaos::record::Record;
use aethelgard_evolve::{EventRef, View};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq, Debug)]
enum Form {
    Bmp581,
    WriteRead,
    /// The ICP-101xx's `[DEBUG]` lines (D4(a)): `Received command: [0x78, 0x66]` writes the command,
    /// `Reading data with CRC: [..], …` reads after it.
    Icp,
}

/// An ICP-101xx line: ("w", command bytes) or ("r", bytes read).
fn icp(line: &str, device: &str) -> Option<(&'static str, Vec<u8>)> {
    let rest = line
        .strip_prefix("[DEBUG] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")?;
    let list = |t: &str| -> Option<Vec<u8>> {
        let t = t.strip_prefix('[')?.split(']').next()?;
        t.split(", ")
            .map(|b| u8::from_str_radix(b.strip_prefix("0x")?, 16).ok())
            .collect()
    };
    if let Some(t) = rest.strip_prefix("Received command: ") {
        return Some(("w", list(t)?));
    }
    let t = rest.strip_prefix("Reading data with CRC: ")?;
    Some(("r", list(t)?))
}

/// A BMP581 line as (pointer, read bytes count or None for a write, written byte count incl. pointer, value read/written).
fn bmp(line: &str, device: &str) -> Option<(u8, bool, Vec<u8>, Vec<u8>)> {
    let rest = line
        .strip_prefix("[NOISY] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")?;
    let byte = |t: &str| {
        let d = t.strip_prefix("0x")?;
        (d.len() == 2 && d.bytes().all(|c| matches!(c, b'0'..=b'9' | b'A'..=b'F')))
            .then(|| u8::from_str_radix(d, 16).ok())
            .flatten()
    };
    let (verb, rest) = rest.split_once(' ')?;
    let (pointer, rest) = rest.split_once(' ')?;
    let (arrow, rest) = rest.split_once(' ')?;
    let p = byte(pointer).filter(|&p| p <= 0x7F)?;
    let bytes: Vec<u8> = if rest == "(none)" {
        Vec::new()
    } else {
        rest.split(' ').map(byte).collect::<Option<_>>()?
    };
    match (verb, arrow) {
        ("write", "<-") => Some((p, false, [&[p], &bytes[..]].concat(), Vec::new())),
        ("read", "->") if !bytes.is_empty() => Some((p, true, vec![p], bytes)),
        _ => None,
    }
}

/// A `Write`/`Read` line: ("Write"|"Read", bytes).
fn wr<'l>(line: &'l str, device: &str) -> Option<(&'l str, Vec<u8>)> {
    let rest = line
        .strip_prefix("[NOISY] ")?
        .strip_prefix(device)?
        .strip_prefix(": ")?;
    let (what, rest) = rest.split_once(' ')?;
    if what != "Write" && what != "Read" {
        return None;
    }
    let bytes: Vec<u8> = rest
        .split(' ')
        .map(|b| {
            (!b.is_empty() && b.len() <= 2 && b.bytes().all(|c| c.is_ascii_hexdigit()))
                .then(|| u8::from_str_radix(b, 16).ok())
                .flatten()
        })
        .collect::<Option<_>>()?;
    Some((what, bytes))
}

/// A transaction in `lines`: its line indices, written bytes, read bytes (None: no read).
#[derive(Debug, Clone)]
struct Tx {
    lines: Vec<usize>,
    written: Vec<u8>,
    read: Option<Vec<u8>>,
}

/// `lines` (one device's, no `[ERROR]`) as transactions in `form`, and the indices that are none.
fn txs(lines: &[String], device: &str, form: Form) -> (Vec<Tx>, Vec<usize>) {
    let mut out: Vec<Tx> = Vec::new();
    let mut none = Vec::new();
    let mut open = false;
    for (i, l) in lines.iter().enumerate() {
        match form {
            Form::Bmp581 => match bmp(l, device) {
                Some((_, is_read, w, r)) => out.push(Tx {
                    lines: vec![i],
                    written: w,
                    read: is_read.then_some(r),
                }),
                None => none.push(i),
            },
            Form::Icp => match icp(l, device) {
                Some(("w", b)) => {
                    out.push(Tx {
                        lines: vec![i],
                        written: b,
                        read: None,
                    });
                    open = true;
                }
                Some((_, b)) if open => {
                    let t = out.last_mut().unwrap();
                    t.lines.push(i);
                    t.read.get_or_insert_with(Vec::new).extend(b);
                }
                _ => none.push(i),
            },
            Form::WriteRead => match wr(l, device) {
                Some(("Write", b)) => {
                    out.push(Tx {
                        lines: vec![i],
                        written: b,
                        read: None,
                    });
                    open = true;
                }
                Some((_, b)) => {
                    if open {
                        let t = out.last_mut().unwrap();
                        t.lines.push(i);
                        t.read.get_or_insert_with(Vec::new).extend(b);
                    } else {
                        // A Read with no Write before it: a transaction with no pointer.
                        out.push(Tx {
                            lines: vec![i],
                            written: Vec::new(),
                            read: Some(b),
                        });
                        open = true;
                    }
                }
                None => {
                    none.push(i);
                }
            },
        }
    }
    (out, none)
}

struct Step {
    rec: usize,
    write: Vec<u8>,
    width: u8,
    expected: Vec<String>,
    status_ok: bool,
}

/// Today's count rule, as renode.rs `attribute`'s I2C arm: each step takes as many lines as it
/// expects, the last the rest; `[ERROR]` lines go to the step under way.
fn by_count(steps: &[Step], log: &[String]) -> Vec<Vec<String>> {
    let (errors, lines): (Vec<_>, Vec<_>) = log
        .iter()
        .enumerate()
        .partition(|(_, b)| b.starts_with("[ERROR] "));
    let mut seen = vec![Vec::new(); steps.len()];
    let mut at = 0;
    let mut begun = Vec::new();
    for k in 0..steps.len() {
        let n = if k == steps.len() - 1 {
            lines.len().saturating_sub(at)
        } else {
            steps[k].expected.len()
        };
        let end = (at + n).min(lines.len());
        if let Some(&(i, _)) = lines.get(at).filter(|_| at < end) {
            begun.push((i, k));
        }
        seen[k] = lines[at..end].iter().map(|(_, b)| (*b).clone()).collect();
        at = end;
    }
    for (i, body) in errors {
        let k = begun
            .iter()
            .rev()
            .find(|&&(b, _)| b < i)
            .map_or(0, |&(_, k)| k);
        seen[k].push(body.clone());
    }
    seen
}

/// The candidate placement; returns each step's lines and the unplaced lines (with log index).
fn by_match(
    steps: &[Step],
    log: &[String],
    device: &str,
    form: Form,
    fallback: u8,
    carrier_a: bool,
) -> (Vec<Vec<String>>, Vec<String>) {
    let errors: Vec<usize> = (0..log.len())
        .filter(|&i| log[i].starts_with("[ERROR] "))
        .collect();
    let body: Vec<usize> = (0..log.len()).filter(|i| !errors.contains(i)).collect();
    let lines: Vec<String> = body.iter().map(|&i| log[i].clone()).collect();
    let (t, none) = txs(&lines, device, form);
    // owner of each log index
    let mut owner: Vec<Option<usize>> = vec![None; log.len()];
    // The key: the pointer (the command, two bytes, for the ICP) and the direction.
    let n = if form == Form::Icp { 2 } else { 1 };
    let key = |w: &[u8], read: bool| ((w.len() >= n).then(|| w[..n].to_vec()), read);
    let mut used: std::collections::BTreeMap<(Option<Vec<u8>>, bool), usize> = Default::default();
    let mut placed: Vec<Option<usize>> = vec![None; t.len()];
    // Pass 1: by key.
    for (x, tx) in t.iter().enumerate() {
        let k = key(&tx.written, tx.read.is_some());
        let cands: Vec<usize> = (0..steps.len())
            .filter(|&s| k.0.is_some() && key(&steps[s].write, steps[s].width > 0) == k)
            .collect();
        if !cands.is_empty() {
            let n = used.entry(k).or_insert(0);
            placed[x] = Some(cands[(*n).min(cands.len() - 1)]);
            *n += 1;
        }
    }
    // Pass 2: by direction, in log order. 1: the device's one step of that direction; 2: the first
    // step of that direction, in script order, that holds no transaction yet.
    for x in 0..t.len() {
        if placed[x].is_some() {
            continue;
        }
        let dir = t[x].read.is_some();
        let same: Vec<usize> = (0..steps.len())
            .filter(|&s| (steps[s].width > 0) == dir)
            .collect();
        placed[x] = match fallback {
            1 => (same.len() == 1).then(|| same[0]),
            2 => same.into_iter().find(|s| !placed.contains(&Some(*s))),
            _ => None,
        };
    }
    for (x, tx) in t.iter().enumerate() {
        if let Some(s) = placed[x] {
            for &l in &tx.lines {
                owner[body[l]] = Some(s);
            }
        }
    }
    let _ = none;
    let mut seen = vec![Vec::new(); steps.len()];
    let mut unplaced = Vec::new();
    let mut first: Vec<Option<usize>> = vec![None; steps.len()];
    for i in 0..log.len() {
        if let Some(s) = owner[i] {
            first[s].get_or_insert(i);
        }
    }
    let under_way = |i: usize| {
        (0..steps.len())
            .filter(|&s| first[s].is_some_and(|f| f < i))
            .max_by_key(|&s| first[s])
            .unwrap_or(0)
    };
    for i in 0..log.len() {
        match owner[i] {
            Some(s) => seen[s].push(log[i].clone()),
            None if errors.contains(&i) => seen[under_way(i)].push(log[i].clone()),
            None => {
                unplaced.push(log[i].clone());
                if carrier_a {
                    seen[under_way(i)].push(log[i].clone());
                }
            }
        }
    }
    (seen, unplaced)
}

fn short(a: &aethelgard_evolve::Attribution) -> Value {
    let ev = |v: &[EventRef]| {
        v.iter()
            .map(|e| format!("{}:{}", e.index, e.kind))
            .collect::<Vec<_>>()
            .join(",")
    };
    json!({
        "rule": a.rule.id(),
        "evidence": ev(&a.evidence),
        "symptoms": ev(&a.symptoms),
        "masked": ev(&a.masked),
        "unexplained": a.unexplained,
    })
}

/// Each kind 13's fields as `view` gives them (the wrong ones, and whether any).
fn fields(r: &aethelgard_chaos::archive::Replayed) -> Value {
    let Ok(v) = aethelgard_evolve::view(r) else {
        return json!("view refused");
    };
    let mut out = serde_json::Map::new();
    for (i, rec) in r.state.records.iter().enumerate() {
        if let Record::I2cWitness {
            reference, step, ..
        } = rec
            && let Some(e) = v.event(EventRef {
                index: i as u64,
                kind: 13,
            })
            && let View::Fields { fields, .. } = e.view
        {
            let f: Vec<String> = fields
                .iter()
                .map(|f| {
                    format!(
                        "{}{} solved={:#x} observed={}",
                        if f.wrong { "WRONG " } else { "" },
                        f.register,
                        f.solved,
                        f.observed.map_or("-".into(), |o| format!("{o:#x}"))
                    )
                })
                .collect();
            out.insert(format!("{i}:{reference}.{step}"), json!(f));
        }
    }
    Value::Object(out)
}

fn main() {
    for d in std::env::args().skip(1) {
        let r = match aethelgard_chaos::archive::replay_dir(std::path::Path::new(&d)) {
            Ok(r) => r,
            Err(e) => {
                println!("{}", json!({"run": d, "refused": format!("{e:?}")}));
                continue;
            }
        };
        let recs = &r.state.records;
        let base = aethelgard_evolve::attribute(&r)
            .map(|a| short(&a))
            .unwrap_or_else(|e| json!(format!("{e:?}")));
        let verdict = recs.iter().find_map(|x| match x {
            Record::Verdict { pass, findings } => Some(json!({"pass": pass, "findings": findings})),
            _ => None,
        });
        // Each device: its steps (kind 12 joined to kind 13 by reference and step), in record order.
        let mut devices: Vec<(String, String, Vec<Step>)> = Vec::new();
        for (i, x) in recs.iter().enumerate() {
            if let Record::I2cWitness {
                reference,
                device,
                step,
                expected,
                ..
            } = x
            {
                let check = recs.iter().find_map(|c| match c {
                    Record::I2cCheck {
                        reference: rr,
                        step: s,
                        write,
                        width,
                        status,
                        ..
                    } if rr == reference && s == step => {
                        Some((write.clone(), *width, status.as_deref() == Some("ok")))
                    }
                    _ => None,
                });
                let Some((write, width, ok)) = check else {
                    continue;
                };
                let st = Step {
                    rec: i,
                    write,
                    width,
                    expected: expected.clone(),
                    status_ok: ok,
                };
                match devices.iter_mut().find(|d| d.0 == *reference) {
                    Some(d) => d.2.push(st),
                    None => devices.push((reference.clone(), device.clone(), vec![st])),
                }
            }
        }
        let mut dev_out = Vec::new();
        // (name, fallback, carrier A, D3): D3 0 = no finding for an unplaced line; 1 = one more
        // `i2c_transaction_unwitnessed` per unplaced line; 2 = per device with one; 3 = per device with
        // one and no witness that already fails.
        let spec: [(&str, u8, bool, u8); 9] = [
            ("R1A", 0, true, 0),
            ("R1B", 0, false, 0),
            ("R2A", 1, true, 0),
            ("R2B", 1, false, 0),
            ("R2B-line", 1, false, 1),
            ("R2B-dev", 1, false, 2),
            ("R2B-min", 1, false, 3),
            ("R3B", 2, false, 0),
            ("R3B-min", 2, false, 3),
        ];
        let mut variants: Vec<(&str, Vec<(usize, Vec<String>)>, Vec<(String, String)>)> =
            spec.iter().map(|v| (v.0, vec![], vec![])).collect();
        let mut unread = false;
        for (reference, device, steps) in &devices {
            let mut log = Vec::new();
            let mut today = Vec::new();
            for s in steps {
                match &recs[s.rec] {
                    Record::I2cWitness {
                        observed: Some(o), ..
                    } => {
                        log.extend(o.iter().cloned());
                        today.push(o.clone());
                    }
                    _ => unread = true,
                }
            }
            if today.len() != steps.len() {
                dev_out.push(json!({"device": reference, "unread": true}));
                continue;
            }
            let script: Vec<String> = steps
                .iter()
                .flat_map(|s| s.expected.iter().cloned())
                .collect();
            let form = if script.iter().any(|l| bmp(l, device).is_some()) {
                Form::Bmp581
            } else if script.iter().any(|l| icp(l, device).is_some()) {
                Form::Icp
            } else {
                Form::WriteRead
            };
            let errors = log.iter().filter(|l| l.starts_with("[ERROR] ")).count();
            let count_rederived = by_count(steps, &log) == today;
            // (1): a witness's first Write and Read, as evolve's `logged` reads them, against the solved
            // transaction: value equal, count unequal.
            let mut counts = Vec::new();
            let mut stitch = Vec::new();
            for (k, s) in steps.iter().enumerate() {
                let (t, _) = txs(&today[k], device, form);
                let w = t.iter().position(|x| !x.written.is_empty());
                let rd = t.iter().position(|x| x.read.is_some());
                if let (Some(w), Some(rd)) = (w, rd)
                    && w != rd
                {
                    stitch.push(format!(
                        "{}: Write from tx {w}, Read from tx {rd}",
                        step_name(&recs[s.rec])
                    ));
                }
                if let Some(w) = w {
                    let got = &t[w].written;
                    let pack = |b: &[u8]| b.iter().fold(0u128, |v, &x| (v << 8) | x as u128);
                    if pack(got) == pack(&s.write) && got.len() != s.write.len() {
                        counts.push(format!(
                            "{}: Write {:02X?} vs solved {:02X?}",
                            step_name(&recs[s.rec]),
                            got,
                            s.write
                        ));
                    }
                }
            }
            // (2): the whole log's transactions in the BMP581 form against the script's.
            let surplus = if form == Form::Bmp581 {
                let (got, gnone) = txs(&log, device, form);
                let (want, _) = txs(&script, device, form);
                let wl: Vec<&str> = want.iter().map(|t| script[t.lines[0]].as_str()).collect();
                let mut held = 0;
                let mut extra = Vec::new();
                for t in &got {
                    let l = log[t.lines[0]].as_str();
                    if wl.get(held) == Some(&l) {
                        held += 1;
                    } else {
                        extra.push(l.to_owned());
                    }
                }
                json!({"none_lines": gnone.len(), "held": held, "of": wl.len(), "extra": extra})
            } else {
                Value::Null
            };
            let mut placed = serde_json::Map::new();
            for (vi, &(_, fallback, a, _)) in spec.iter().enumerate() {
                let (seen, unplaced) = by_match(steps, &log, device, form, fallback, a);
                let moved: Vec<Value> = (0..steps.len())
                    .filter(|&k| seen[k] != today[k])
                    .map(|k| json!({"step": step_name(&recs[steps[k].rec]), "today": today[k], "placed": seen[k],
                        "boot_today": witness_ok(&steps[k], &today[k]), "boot_placed": witness_ok(&steps[k], &seen[k])}))
                    .collect();
                placed.insert(
                    variants[vi].0.into(),
                    json!({"moved": moved, "unplaced": unplaced}),
                );
                for k in 0..steps.len() {
                    variants[vi].1.push((steps[k].rec, seen[k].clone()));
                }
                for u in unplaced {
                    variants[vi].2.push((reference.clone(), u));
                }
            }
            dev_out.push(
                json!({"device": reference, "form": format!("{form:?}"), "steps": steps.len(),
                "lines": log.len(), "errors": errors, "count_rederived": count_rederived,
                "whole_equals_script": log == script, "counts": counts, "stitch": stitch,
                "surplus": surplus, "placed": placed}),
            );
        }
        let mut var_out = serde_json::Map::new();
        for (vi, (name, rewrites, unplaced)) in variants.iter().enumerate() {
            let d3 = spec[vi].3;
            let mut r2 = aethelgard_chaos::archive::Replayed {
                header: r.header.clone(),
                manifest: r.manifest.clone(),
                state: r.state.clone(),
            };
            let mut changed = false;
            for (i, lines) in rewrites {
                if let Record::I2cWitness { observed, .. } = &mut r2.state.records[*i]
                    && observed.as_ref() != Some(lines)
                {
                    *observed = Some(lines.clone());
                    changed = true;
                }
            }
            // The boot's verdict as the re-placed witnesses give it: `i2c_transaction_unwitnessed` once per
            // I2C witness whose record claims `ok` and whose lines are not the script's (an `[ERROR]` line
            // names `i2c_step_error` instead, kept as recorded). Every other finding as recorded.
            if changed || (d3 > 0 && !unplaced.is_empty()) {
                let ok = |rr: &str, st: &str| {
                    r2.state.records.iter().any(|c| {
                        matches!(c, Record::I2cCheck { reference, step, status, .. }
                        if reference == rr && step == st && status.as_deref() == Some("ok"))
                    })
                };
                let unwitnessed = r2
                    .state
                    .records
                    .iter()
                    .filter(|w| match w {
                        Record::I2cWitness {
                            reference,
                            step,
                            expected,
                            observed: Some(o),
                            ..
                        } => {
                            ok(reference, step)
                                && o != expected
                                && !o.iter().any(|l| l.starts_with("[ERROR] "))
                        }
                        _ => false,
                    })
                    .count() as u64;
                // D3: the unplaced lines' own findings, counted as `i2c_transaction_unwitnessed`.
                let failing = |rr: &str| {
                    r2.state.records.iter().any(|w| match w {
                        Record::I2cWitness {
                            reference,
                            step,
                            expected,
                            observed: Some(o),
                            ..
                        } => reference == rr && ok(reference, step) && o != expected,
                        _ => false,
                    })
                };
                let mut devs: Vec<&str> = unplaced.iter().map(|(d, _)| d.as_str()).collect();
                devs.dedup();
                let extra = match d3 {
                    1 => unplaced.len() as u64,
                    2 => devs.len() as u64,
                    3 => devs.iter().filter(|d| !failing(d)).count() as u64,
                    _ => 0,
                };
                let unwitnessed = unwitnessed + extra;
                for x in r2.state.records.iter_mut() {
                    if let Record::Verdict { pass, findings } = x {
                        findings.retain(|f| f.0 != "i2c_transaction_unwitnessed");
                        if unwitnessed > 0 {
                            findings.push((
                                "i2c_transaction_unwitnessed".into(),
                                "error".into(),
                                unwitnessed,
                            ));
                            findings.sort();
                        }
                        *pass = findings.iter().all(|f| f.1 == "info");
                    }
                }
            }
            let verdict = r2.state.records.iter().find_map(|x| match x {
                Record::Verdict { pass, findings } => {
                    Some(json!({"pass": pass, "findings": findings}))
                }
                _ => None,
            });
            let a = aethelgard_evolve::attribute(&r2)
                .map(|a| short(&a))
                .unwrap_or_else(|e| json!(format!("{e:?}")));
            var_out.insert(
                (*name).into(),
                json!({"changed": changed, "attribute": a, "unplaced": unplaced, "verdict": verdict,
                "fields": if changed { fields(&r2) } else { Value::Null }}),
            );
        }
        println!(
            "{}",
            json!({"run": d, "attribute": base, "verdict": verdict, "unread": unread, "devices": dev_out,
                "fields": fields(&r), "variants": var_out})
        );
    }
}

fn step_name(r: &Record) -> String {
    match r {
        Record::I2cWitness {
            reference, step, ..
        } => format!("{reference}.{step}"),
        _ => String::new(),
    }
}

/// The boot's witness judgement of a step whose record claims `ok`: exactly the script's lines.
fn witness_ok(s: &Step, seen: &[String]) -> Value {
    if !s.status_ok {
        return json!("not judged");
    }
    json!(seen == s.expected.as_slice())
}
