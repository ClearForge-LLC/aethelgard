//! AET-WO-2312 §1.1 probe. Reads archives through chaos's own reader and changes nothing in `crates/`.
//!
//! `census DIR...`: one JSON line per archive, with these fields:
//!   - for each kind, the prototype derivation (`proto::shown`): its records and their count interval;
//!   - `select`'s set, copied from evolve at 7dec1d7b;
//!   - the verdict's count;
//!   - the RCC name-against-address comparison on every kind 11.
//!
//! `make OUT NAME...`: the reviewer's shapes and the cost archives, at this tree's trace version, built through
//! the tests' `archived` header (evolve/tests/attribute.rs:241).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use aethelgard_chaos::campaign::{self, run_id};
use aethelgard_chaos::record::{self, BusLine, Override, Reading, Record};
use aethelgard_chaos::{Event, Header, TraceWriter, replay_dir};
use serde_json::{Value, json};

mod proto {
    //! The prototype of (1): each kind's records as the judge (cli/src/renode.rs at 7dec1d7b) names them,
    //! and the interval the verdict's count must lie in.
    use super::*;

    pub struct Row {
        pub kind: String,
        pub records: Vec<usize>,
        pub least: u64,
        /// `None`: no upper bound.
        pub most: Option<u64>,
    }

    pub fn booted(records: &[Record]) -> bool {
        records.iter().any(|r| {
            matches!(r, Record::BootObserved { boot_word, .. }
                if boot_word.observed == Some(boot_word.expected))
        })
    }

    pub fn read(records: &[Record]) -> bool {
        records.iter().any(
            |r| matches!(r, Record::BootObserved { boot_word, .. } if boot_word.observed.is_some()),
        )
    }

    /// The judge's `Outcome::Pass` for a completed read (twin `StepCheck::judge`, `parts::width_mask`).
    pub fn read_matches(width: u8, expected: u64, mask: u64, observed: Option<u64>) -> bool {
        let read = match width {
            0 => 0,
            8.. => u64::MAX,
            w => (1u64 << (8 * u32::from(w))) - 1,
        };
        observed.is_some_and(|o| o & !read == 0 && o & mask == expected & mask)
    }

    pub fn mismatch_kind(bus: &str, step: &str) -> Option<String> {
        let step = match record::step_index(step)? {
            0 => "identity",
            1..=4 => "configure",
            _ => "data",
        };
        Some(format!("{bus}_{step}_mismatch"))
    }

    /// An `ok` read's own witness, by reference and step: (index, expected, observed).
    fn witness_of<'a>(
        records: &'a [Record],
        bus: &str,
        reference: &str,
        step: &str,
    ) -> Option<(usize, &'a Vec<String>, &'a Option<Vec<String>>)> {
        records.iter().enumerate().find_map(|(i, w)| match (bus, w) {
            (
                "i2c",
                Record::I2cWitness {
                    reference: r,
                    step: s,
                    expected,
                    observed,
                    ..
                },
            )
            | (
                "spi",
                Record::SpiWitness {
                    reference: r,
                    step: s,
                    expected,
                    observed,
                    ..
                },
            ) if r == reference && s == step => Some((i, expected, observed)),
            _ => None,
        })
    }

    pub fn shown(records: &[Record]) -> Vec<Row> {
        let booted = booted(records);
        let read = read(records);
        let mut rows: Vec<Row> = Vec::new();
        let idx = |pred: &dyn Fn(&Record) -> bool| -> Vec<usize> {
            (0..records.len()).filter(|&i| pred(&records[i])).collect()
        };
        fn push_exact(rows: &mut Vec<Row>, kind: &str, v: Vec<usize>) {
            let n = v.len() as u64;
            rows.push(Row { kind: kind.into(), records: v, least: n, most: Some(n) });
        }
        macro_rules! exact {
            ($k:expr, $v:expr $(,)?) => {{
                let v = $v;
                push_exact(&mut rows, $k, v)
            }};
        }
        exact!(campaign::PAST_CEILING_FINDING, idx(&|r| matches!(r, Record::RiseComputed { exceeds: true, .. })));
        exact!(campaign::STUCK_HIGH_FINDING, idx(&|r| matches!(r, Record::LineStuckHigh { .. })));
        exact!(campaign::HELD_LOW_FINDING,
            idx(&|r| matches!(r, Record::BusFault { from, .. } if from.iter().any(|f| f == campaign::HELD_LOW))),
        );
        // Kind 11 under its mask, at any boot (the judge reads every register whenever it reads the boot).
        exact!("register_mismatch",
            idx(&|r| matches!(r, Record::RegisterObserved { reading, .. }
                if reading.observed.is_some_and(|o| (o ^ reading.expected) & reading.mask != 0))),
        );
        for bus in ["i2c", "spi"] {
            let status = |r: &Record| -> Option<String> {
                match (bus, r) {
                    ("i2c", Record::I2cCheck { status, .. }) | ("spi", Record::SpiCheck { status, .. }) => {
                        Some(status.clone().unwrap_or_default())
                    }
                    _ => None,
                }
            };
            // Named at any boot (`timeouts_without_the_boot_word`).
            exact!(&format!("{bus}_timeout"),
                idx(&|r| status(r).is_some_and(|s| s.starts_with("timeout_"))),
            );
            // Named once booted; an unmodelled device's invalid read is named by the same kind and not archived.
            let inv = if booted { idx(&|r| status(r).is_some_and(|s| s == "invalid")) } else { vec![] };
            let n = inv.len() as u64;
            rows.push(Row { kind: format!("{bus}_record_invalid"), records: inv, least: n, most: None });
            for step in ["identity", "configure", "data"] {
                let kind = format!("{bus}_{step}_mismatch");
                let v = if booted {
                    idx(&|r| match r {
                        Record::I2cCheck { status: Some(s), expected, mask, width, observed, step: st, .. }
                        | Record::SpiCheck { status: Some(s), expected, mask, width, observed, step: st, .. }
                            if status(r).is_some() =>
                        {
                            s == "ok"
                                && !read_matches(*width, *expected, *mask, *observed)
                                && mismatch_kind(bus, st).as_deref() == Some(kind.as_str())
                        }
                        _ => false,
                    })
                } else {
                    vec![]
                };
                exact!(&kind, v);
            }
            // The witnessed kind: select's set (an ok read whose witness differs), gated; the bound from below
            // is check_run's (an ok read whose witness is unread), whose count the judge always names.
            let mut differ = Vec::new();
            let mut unread = 0u64;
            if booted {
                for (i, r) in records.iter().enumerate() {
                    let (reference, step) = match (bus, r) {
                        ("i2c", Record::I2cCheck { reference, step, status: Some(s), .. })
                        | ("spi", Record::SpiCheck { reference, step, status: Some(s), .. })
                            if s == "ok" =>
                        {
                            (reference, step)
                        }
                        _ => continue,
                    };
                    let _ = i;
                    if let Some((w, expected, observed)) = witness_of(records, bus, reference, step) {
                        if observed.as_ref() != Some(expected) {
                            differ.push(w);
                        }
                        if observed.is_none() {
                            unread += 1;
                        }
                    }
                }
            }
            differ.sort();
            rows.push(Row { kind: format!("{bus}_transaction_unwitnessed"), records: differ, least: unread, most: None });
        }
        exact!(campaign::BOOT_WORD_FINDING,
            idx(&|r| matches!(r, Record::BootObserved { boot_word, .. }
                if boot_word.observed.is_some_and(|o| o != boot_word.expected))),
        );
        exact!(campaign::PC_OUTSIDE_FINDING,
            idx(&|r| matches!(r, Record::BootObserved { pc: Some(pc), executable, .. } if !record::inside(executable, *pc))),
        );
        // Ungated in the judge (`halted` is computed before `booted` is used); check_run binds it once booted.
        exact!(campaign::HALTED_FINDING,
            idx(&|r| matches!(r, Record::BootObserved { instructions_expected, instructions_observed, .. }
                if instructions_observed.is_some_and(|n| n < *instructions_expected))),
        );
        exact!(campaign::BROWN_OUT_FINDING,
            idx(&|r| matches!(r, Record::BootObserved { boot_word, brown_out, .. }
                if boot_word.observed == Some(boot_word.expected) && campaign::brown_out_fails(brown_out))),
        );
        exact!(campaign::FLASH_ACCESS_FINDING,
            idx(&|r| matches!(r, Record::BootObserved { boot_word, flash_access, .. }
                if boot_word.observed == Some(boot_word.expected) && flash_access.as_deref() != Some(&[]))),
        );
        exact!(campaign::MOTOR_STATIC_FINDING, idx(&|r| matches!(r, Record::MotorTimingCheck { holds: false, .. })));
        exact!(campaign::MOTOR_UNWITNESSED_FINDING,
            idx(&|r| booted && matches!(r, Record::MotorWitness { status, .. } if status == "unwitnessed" || status == "unreadable")),
        );
        exact!(campaign::MOTOR_MISMATCH_FINDING,
            idx(&|r| booted && matches!(r, Record::MotorWitness { status, .. } if status == "mismatch" || status == "batched" || status == "stopped")),
        );
        exact!(campaign::SERIAL_STATIC_FINDING, idx(&|r| matches!(r, Record::SerialFramingCheck { holds: false, .. })));
        exact!(campaign::SERIAL_TAG_FINDING,
            idx(&|r| booted && matches!(r, Record::SerialFramingCheck { tags, .. } if tags.as_ref().is_none_or(|t| !t.holds()))),
        );
        let releases = campaign::released(records);
        let witness_kinds = |w: &Record| -> Vec<&'static str> {
            match w {
                Record::SerialWitness { label, .. } if booted => records
                    .iter()
                    .find(|r| matches!(r, Record::SerialPortSolved { label: l, .. } if l == label))
                    .map(|p| campaign::serial_witness_findings(p, w, releases))
                    .unwrap_or_default(),
                _ => Vec::new(),
            }
        };
        exact!(campaign::SERIAL_PATTERN_FINDING, idx(&|r| witness_kinds(r).contains(&campaign::SERIAL_PATTERN_FINDING)));
        exact!(campaign::SERIAL_ECHO_FINDING, idx(&|r| witness_kinds(r).contains(&campaign::SERIAL_ECHO_FINDING)));
        let forged = idx(&|r| witness_kinds(r).contains(&campaign::SERIAL_FORGED_FINDING));
        let ports = records.iter().any(|r| matches!(r, Record::SerialPortSolved { .. }));
        let n = forged.len() as u64;
        rows.push(Row {
            kind: campaign::SERIAL_FORGED_FINDING.into(),
            records: forged,
            least: n,
            most: if ports { None } else { Some(n) },
        });
        // Named at most once, from Renode's log, which no record holds. The evidence is the RCC registers
        // read with an enable bit off, by name (no `svd-hal`).
        let _ = read;
        rows.push(Row {
            kind: "locked_peripheral_access".into(),
            records: idx(&|r| rcc_off_by_name(r)),
            least: 0,
            most: Some(1),
        });
        rows
    }

    pub fn rcc_off_by_name(r: &Record) -> bool {
        matches!(r, Record::RegisterObserved { name, reading }
            if name.starts_with("RCC.") && reading.observed.is_some_and(|o| reading.expected & reading.mask & !o != 0))
    }
}

/// evolve's `Run::select` at 7dec1d7b, its predicates copied (ungated as there).
fn select_like(records: &[Record], rcc: (u32, u32), kind: &str) -> Option<(Vec<usize>, bool)> {
    let idx = |pred: &dyn Fn(&Record) -> bool| -> Vec<usize> {
        (0..records.len()).filter(|&i| pred(&records[i])).collect()
    };
    let booted = proto::booted(records);
    let check_of = |bus: &str, reference: &str, step: &str| -> Option<&Record> {
        records.iter().find(|r| match (bus, r) {
            ("i2c", Record::I2cCheck { reference: x, step: s, .. })
            | ("spi", Record::SpiCheck { reference: x, step: s, .. }) => x == reference && s == step,
            _ => false,
        })
    };
    let i2c_status = |r: &Record, f: &dyn Fn(&str) -> bool| matches!(r, Record::I2cCheck { status: Some(s), .. } if f(s));
    let releases = campaign::released(records);
    let witness_kinds = |w: &Record| -> Vec<&'static str> {
        match w {
            Record::SerialWitness { label, .. } if booted => records
                .iter()
                .find(|r| matches!(r, Record::SerialPortSolved { label: l, .. } if l == label))
                .map(|p| campaign::serial_witness_findings(p, w, releases))
                .unwrap_or_default(),
            _ => Vec::new(),
        }
    };
    let events = match kind {
        campaign::PAST_CEILING_FINDING => idx(&|r| matches!(r, Record::RiseComputed { exceeds: true, .. })),
        campaign::STUCK_HIGH_FINDING => idx(&|r| matches!(r, Record::LineStuckHigh { .. })),
        campaign::HELD_LOW_FINDING => {
            idx(&|r| matches!(r, Record::BusFault { from, .. } if from.iter().any(|f| f == campaign::HELD_LOW)))
        }
        "i2c_timeout" => idx(&|r| i2c_status(r, &|s| s.starts_with("timeout_"))),
        "i2c_record_invalid" => idx(&|r| i2c_status(r, &|s| s == "invalid")),
        "i2c_identity_mismatch" | "i2c_configure_mismatch" | "i2c_data_mismatch" | "spi_identity_mismatch"
        | "spi_configure_mismatch" | "spi_data_mismatch" => idx(&|r| {
            matches!(r, Record::I2cCheck { status: Some(s), expected, mask, width, observed, step, .. }
                | Record::SpiCheck { status: Some(s), expected, mask, width, observed, step, .. }
                if s == "ok" && !proto::read_matches(*width, *expected, *mask, *observed)
                    && proto::mismatch_kind(if r.kind() == 12 { "i2c" } else { "spi" }, step).as_deref() == Some(kind))
        }),
        "i2c_transaction_unwitnessed" => idx(&|r| match r {
            Record::I2cWitness { reference, step, expected, observed, .. } => {
                matches!(check_of("i2c", reference, step), Some(Record::I2cCheck { status: Some(s), .. }) if s == "ok")
                    && observed.as_deref() != Some(expected.as_slice())
            }
            _ => false,
        }),
        "spi_timeout" => idx(&|r| matches!(r, Record::SpiCheck { status: Some(s), .. } if s.starts_with("timeout_"))),
        "spi_record_invalid" => idx(&|r| matches!(r, Record::SpiCheck { status: Some(s), .. } if s == "invalid")),
        "spi_transaction_unwitnessed" => idx(&|r| match r {
            Record::SpiWitness { reference, step, expected, observed, .. } => {
                matches!(check_of("spi", reference, step), Some(Record::SpiCheck { status: Some(s), .. }) if s == "ok")
                    && observed.as_deref() != Some(expected.as_slice())
            }
            _ => false,
        }),
        campaign::MOTOR_STATIC_FINDING => idx(&|r| matches!(r, Record::MotorTimingCheck { holds: false, .. })),
        campaign::MOTOR_UNWITNESSED_FINDING => idx(&|r| {
            booted && matches!(r, Record::MotorWitness { status, .. } if status == "unwitnessed" || status == "unreadable")
        }),
        campaign::MOTOR_MISMATCH_FINDING => idx(&|r| {
            booted && matches!(r, Record::MotorWitness { status, .. } if status == "mismatch" || status == "batched" || status == "stopped")
        }),
        campaign::SERIAL_STATIC_FINDING => idx(&|r| matches!(r, Record::SerialFramingCheck { holds: false, .. })),
        campaign::SERIAL_TAG_FINDING => idx(&|r| {
            matches!(r, Record::SerialFramingCheck { tags, .. } if booted && tags.as_ref().is_none_or(|t| !t.holds()))
        }),
        campaign::SERIAL_PATTERN_FINDING | campaign::SERIAL_ECHO_FINDING => idx(&|r| witness_kinds(r).contains(&kind)),
        campaign::SERIAL_FORGED_FINDING => {
            return Some((idx(&|r| witness_kinds(r).contains(&kind)), false));
        }
        "register_mismatch" => idx(&|r| matches!(r, Record::RegisterObserved { reading, .. }
            if reading.observed.is_some_and(|o| (o ^ reading.expected) & reading.mask != 0))),
        campaign::BOOT_WORD_FINDING => idx(&|r| matches!(r, Record::BootObserved { boot_word, .. }
            if boot_word.observed.is_some_and(|o| o != boot_word.expected))),
        campaign::PC_OUTSIDE_FINDING => {
            idx(&|r| matches!(r, Record::BootObserved { pc: Some(pc), executable, .. } if !record::inside(executable, *pc)))
        }
        campaign::BROWN_OUT_FINDING => idx(&|r| matches!(r, Record::BootObserved { boot_word, brown_out, .. }
            if boot_word.observed == Some(boot_word.expected) && campaign::brown_out_fails(brown_out))),
        campaign::FLASH_ACCESS_FINDING => idx(&|r| matches!(r, Record::BootObserved { boot_word, flash_access, .. }
            if boot_word.observed == Some(boot_word.expected) && flash_access.as_deref() != Some(&[]))),
        "cpu_halted" => idx(&|r| matches!(r, Record::BootObserved { instructions_expected, instructions_observed, .. }
            if instructions_observed.is_some_and(|n| n < *instructions_expected))),
        "locked_peripheral_access" => {
            return Some((
                idx(&|r| match r {
                    Record::RegisterObserved { reading, .. } => {
                        (rcc.0..rcc.1).contains(&reading.address)
                            && reading.observed.is_some_and(|o| reading.expected & reading.mask & !o != 0)
                    }
                    _ => false,
                }),
                false,
            ));
        }
        _ => return None,
    };
    Some((events, true))
}

fn rcc_range() -> (u32, u32) {
    let f = aethelgard_svd_hal::cmsis::register_facts().expect("register facts");
    let rcc = f.base("RCC").expect("RCC base");
    (rcc, rcc + f.layout("RCC").expect("RCC layout").size)
}

fn census(dir: &Path, rcc: (u32, u32)) -> Value {
    let run = match replay_dir(dir) {
        Ok(r) => r,
        Err(e) => return json!({"archive": dir.display().to_string(), "read_error": e.to_string()}),
    };
    let records = &run.state.records;
    let check = campaign::check_archived(&run.header, &run.state);
    let Some(Record::Verdict { findings, .. }) = records.last() else {
        return json!({"archive": dir.display().to_string(), "no_verdict": true});
    };
    let error = |k: &str| -> u64 { findings.iter().filter(|(x, s, _)| x == k && s != "info").map(|f| f.2).sum() };
    let any = |k: &str| -> u64 { findings.iter().filter(|(x, _, _)| x == k).map(|f| f.2).sum() };
    let mut kinds = serde_json::Map::new();
    let mut disagree = Vec::new();
    let mut outside = Vec::new();
    for row in proto::shown(records) {
        let sel = select_like(records, rcc, &row.kind);
        let (v, a) = (error(&row.kind), any(&row.kind));
        let inside = v >= row.least && row.most.is_none_or(|m| v <= m) && a == v;
        if !inside {
            outside.push(row.kind.clone());
        }
        let sel_set = sel.as_ref().map(|s| s.0.clone());
        if sel_set.as_ref() != Some(&row.records) {
            disagree.push(row.kind.clone());
        }
        if v > 0 || !row.records.is_empty() || sel_set.as_ref().is_some_and(|s| !s.is_empty()) {
            kinds.insert(
                row.kind.clone(),
                json!({"shown": row.records.len(), "least": row.least, "most": row.most,
                       "select": sel_set.map(|s| s.len()), "select_exact": sel.map(|s| s.1),
                       "verdict": v, "any_severity": a}),
            );
        }
    }
    // Every verdict kind the derivation does not name, with its count.
    let named: Vec<String> = proto::shown(records).into_iter().map(|r| r.kind).collect();
    let mut other: BTreeMap<String, u64> = BTreeMap::new();
    for (k, s, n) in findings {
        if s != "info" && !named.contains(k) {
            *other.entry(k.clone()).or_default() += n;
        }
    }
    // RCC by name against by address, on every kind 11.
    let mut rcc_mismatch = Vec::new();
    let mut kind11 = 0;
    for r in records {
        if let Record::RegisterObserved { name, reading } = r {
            kind11 += 1;
            if name.starts_with("RCC.") != (rcc.0..rcc.1).contains(&reading.address) {
                rcc_mismatch.push(format!("{name}@{:#010x}", reading.address));
            }
        }
    }
    let mut statuses: BTreeMap<String, u64> = BTreeMap::new();
    let mut empty_expected = 0;
    let mut error_lines = 0;
    for r in records {
        match r {
            Record::I2cCheck { status, .. } | Record::SpiCheck { status, .. } => {
                *statuses.entry(format!("{}:{}", r.kind(), status.as_deref().unwrap_or("null"))).or_default() += 1;
            }
            Record::I2cWitness { expected, observed, .. } | Record::SpiWitness { expected, observed, .. } => {
                empty_expected += u64::from(expected.is_empty());
                error_lines += u64::from(observed.as_ref().is_some_and(|o| o.iter().any(|l| l.starts_with("[ERROR]"))));
            }
            _ => {}
        }
    }
    let Record::Scenario { name, .. } = &records[0] else { unreachable!() };
    json!({
        "archive": dir.display().to_string(),
        "scenario": name,
        "check": check.as_ref().map(|s| json!({"pass": s.pass, "matched": s.matched})).unwrap_or_else(|e| json!(e)),
        "read": proto::read(records), "booted": proto::booted(records),
        "records": records.len(),
        "kinds": kinds, "outside_interval": outside, "proto_vs_select": disagree, "other_failing": other,
        "kind11": kind11, "rcc_name_vs_address": rcc_mismatch,
        "statuses": statuses, "witness_expected_empty": empty_expected, "witness_error_lines": error_lines,
    })
}

// ---- make: the shapes and the cost archives (the reviewer's probe, r3/chaos/probe, on the tests' header) ----

fn reading(address: u32, mask: u32, expected: u32, observed: u32) -> Reading {
    Reading { address, mask, expected, observed: Some(observed) }
}
fn line(peripheral: &str, suffix: &str, net: &str, pin: &str) -> BusLine {
    BusLine {
        signal: format!("{peripheral}{suffix}"),
        net: net.into(),
        pin: pin.into(),
        alternate_function: 4,
        ceiling_ps: 300_000,
    }
}
fn solved(bus: &str, peripheral: &str, nets: [&str; 2], pins: [&str; 2], devices: &[(&str, u8)]) -> Record {
    Record::I2cBusSolved {
        bus: bus.into(),
        peripheral: peripheral.into(),
        lines: vec![line(peripheral, "_SCL", nets[0], pins[0]), line(peripheral, "_SDA", nets[1], pins[1])],
        devices: devices.iter().map(|(r, a)| ((*r).into(), *a)).collect(),
    }
}
fn rise(bus: &str, net: &str, ps: u64) -> Record {
    Record::RiseComputed {
        bus: bus.into(),
        net: net.into(),
        rise_30_70_ps: ps,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: ps > 300_000,
        window: record::Window { deck_hz: 400_000, declared_hz: 400_000 },
    }
}
fn check(reference: &str, address: u8, status: &str, observed: u64) -> Record {
    Record::I2cCheck {
        reference: reference.into(),
        address,
        step: "identity".into(),
        write: vec![0xD0],
        width: 1,
        expected: 0x60,
        mask: 0xFF,
        observed: Some(observed),
        status: Some(status.into()),
    }
}
fn witness_with(peripheral: &str, reference: &str, expected: &[&str], observed: Option<&[&str]>) -> Record {
    let device = format!("{}.{}", peripheral.to_ascii_lowercase(), reference.to_ascii_lowercase());
    let lines = |v: &[&str]| -> Vec<String> { v.iter().map(|l| format!("[NOISY] {device}: {l}")).collect() };
    Record::I2cWitness {
        step: "identity".into(),
        peripheral: peripheral.into(),
        reference: reference.into(),
        expected: lines(expected),
        observed: observed.map(lines),
        device,
        absent: Vec::new(),
    }
}
const OK: [&str; 2] = ["Write D0", "Read 60"];
fn verdict(findings: &[(&str, u64)]) -> Record {
    let mut findings: Vec<(String, String, u64)> =
        findings.iter().map(|(k, n)| ((*k).into(), "error".into(), *n)).collect();
    findings.push(("device_unmodelled".into(), "info".into(), 1));
    findings.sort();
    Record::Verdict { pass: findings.iter().all(|f| f.1 == "info"), findings }
}
fn register(name: &str, observed: Option<u32>) -> Record {
    let (address, mask, expected) = match name {
        "GPIOA.MODER" => (0x5802_0000, 0x0000_FC03, 0x0000_A801),
        "GPIOB.MODER" => (0x5802_0400, 0x0000_F000, 0x0000_A000),
        "GPIOB.AFRL" => (0x5802_0420, 0xFF00_0000, 0x4400_0000),
        "RCC.AHB4ENR" => (0x5802_44E0, u32::MAX, 0x3),
        "RCC.APB1LENR" => (0x5802_44E8, u32::MAX, 0x0020_0000),
        _ => panic!("{name}"),
    };
    Record::RegisterObserved {
        name: name.into(),
        reading: Reading { address, mask, expected, observed: Some(observed.unwrap_or(expected)) },
    }
}
fn boot(word: u32) -> Record {
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, word),
        instructions_expected: 10_000_000,
        instructions_observed: Some(10_000_000),
        pc: Some(0x0800_0596),
        executable: vec![(0, 0x1000_0000)],
        brown_out: Reading { address: 0x2000_0004, mask: 0xC, expected: 0, observed: Some(0) },
        flash_access: Some(Vec::new()),
    }
}
fn healthy() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "pullup-2k2".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![Override::PullUpOhms { bus: "SENSORS".into(), ohms: 2200 }],
        },
        Record::Solved { specification_sha256: [2; 32], topology_sha256: [3; 32] },
        solved("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], ["PB6", "PB7"], &[("U2", 0x76), ("U3", 0x48)]),
        rise("SENSORS", "I2C_SCL", 186_402),
        rise("SENSORS", "I2C_SDA", 186_402),
        boot(0x2100_B007),
        register("GPIOA.MODER", None),
        register("GPIOB.MODER", None),
        register("GPIOB.AFRL", None),
        register("RCC.AHB4ENR", None),
        register("RCC.APB1LENR", None),
        check("U2", 0x76, "ok", 0x60),
        witness_with("I2C1", "U2", &OK, Some(&OK)),
        verdict(&[]),
    ]
}
fn pos(r: &[Record], pred: impl Fn(&Record) -> bool) -> usize {
    r.iter().position(pred).unwrap()
}

/// The tests' `archived` header (evolve/tests/attribute.rs:241 at 7dec1d7b), written to `out/<run id>`.
fn write(records: &[Record], out: &Path) -> PathBuf {
    let (Record::Scenario { sha256, .. }, Record::Solved { specification_sha256, topology_sha256 }) =
        (&records[0], &records[1])
    else {
        panic!("a run starts with its scenario and topology")
    };
    let firmware = [9u8; 32];
    let id = run_id(sha256, specification_sha256, &firmware);
    let header = Header {
        run_id: id,
        artifacts: vec![
            ("base".into(), [8; 32]),
            ("firmware".into(), firmware),
            ("scenario".into(), *sha256),
            ("specification".into(), *specification_sha256),
            ("topology".into(), *topology_sha256),
        ],
        tools: campaign::tools_of(records).iter().map(|n| ((*n).to_owned(), "test".to_owned())).collect(),
        tool_sha256: campaign::tools_of(records)
            .iter()
            .filter(|n| [campaign::NGSPICE, campaign::RUST_TOOLCHAIN].contains(n))
            .map(|n| ((*n).to_owned(), [7; 32]))
            .collect(),
    };
    let hex: String = id.iter().map(|b| format!("{b:02x}")).collect();
    std::fs::create_dir_all(out).unwrap();
    let dir = out.join(hex);
    let _ = std::fs::remove_dir_all(&dir);
    let mut w = TraceWriter::create(&dir, &header).unwrap();
    for record in records.iter().cloned() {
        let timestamp = if record.before_boot() { 0 } else { 100_000_000 };
        w.append(&Event::Record { timestamp, record }).unwrap();
    }
    w.commit().unwrap();
    dir
}

fn scenario(name: &str) -> Vec<Record> {
    let mut r = healthy();
    match name {
        "healthy" => {}
        "reg_unnamed" => {
            let i = pos(&r, |x| matches!(x, Record::RegisterObserved { name, .. } if name == "GPIOB.AFRL"));
            r[i] = register("GPIOB.AFRL", Some(0x4700_0000));
        }
        "timeout_unnamed" => {
            let i = pos(&r, |x| matches!(x, Record::I2cCheck { .. }));
            r[i] = check("U2", 0x76, "timeout_rxne", 0);
            let i = pos(&r, |x| matches!(x, Record::I2cWitness { .. }));
            r[i] = witness_with("I2C1", "U2", &OK, Some(&[]));
        }
        "idmismatch_unnamed" => {
            let i = pos(&r, |x| matches!(x, Record::I2cCheck { .. }));
            r[i] = check("U2", 0x76, "ok", 0x58);
            let i = pos(&r, |x| matches!(x, Record::I2cWitness { .. }));
            r[i] = witness_with("I2C1", "U2", &OK, Some(&["Write D0", "Read 58"]));
        }
        "halted_unnamed" => {
            let i = pos(&r, |x| matches!(x, Record::BootObserved { .. }));
            if let Record::BootObserved { instructions_observed, .. } = &mut r[i] {
                *instructions_observed = Some(5);
            }
        }
        "big_fail" | "big_pass" => {
            let k: usize = std::env::var("K").map_or(100, |v| v.parse().unwrap());
            let d: usize = std::env::var("D").map_or(128, |v| v.parse().unwrap());
            let fail = name == "big_fail";
            let mut pins = Vec::new();
            for p in 'A'..='Z' {
                for n in 0..16 {
                    pins.push(format!("P{p}{n}"));
                }
            }
            let mut out = vec![r[0].clone(), r[1].clone()];
            let buses: Vec<(String, String, Vec<(String, u8)>)> = (0..k)
                .map(|b| {
                    let refs = (0..d).map(|i| (format!("U{b:03}x{i:04}"), (i + 1) as u8)).collect();
                    (format!("B{b:04}"), format!("I2X{b:04}"), refs)
                })
                .collect();
            for (bi, (id, per, refs)) in buses.iter().enumerate() {
                let devs: Vec<(&str, u8)> = refs.iter().map(|(r, a)| (r.as_str(), *a)).collect();
                out.push(solved(
                    id,
                    per,
                    [&format!("{id}_SCL"), &format!("{id}_SDA")],
                    [&pins[(2 * bi) % pins.len()], &pins[(2 * bi + 1) % pins.len()]],
                    &devs,
                ));
            }
            for (id, _, _) in &buses {
                out.push(rise(id, &format!("{id}_SCL"), 186_402));
                out.push(rise(id, &format!("{id}_SDA"), 186_402));
            }
            out.push(boot(0x2100_B007));
            for (_, _, refs) in &buses {
                for (rf, a) in refs {
                    out.push(check(rf, *a, if fail { "timeout_rxne" } else { "ok" }, if fail { 0 } else { 0x60 }));
                }
            }
            for (_, per, refs) in &buses {
                for (rf, _) in refs {
                    let obs: &[&str] = if fail { &[] } else { &OK };
                    out.push(witness_with(per, rf, &OK, Some(obs)));
                }
            }
            let n = (k * d) as u64;
            out.push(if fail { verdict(&[("i2c_timeout", n)]) } else { verdict(&[]) });
            return out;
        }
        other => panic!("unknown scenario {other}"),
    }
    r
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("census") => {
            let rcc = rcc_range();
            eprintln!("rcc range {:#010x}..{:#010x}", rcc.0, rcc.1);
            for d in &args[2..] {
                println!("{}", census(Path::new(d), rcc));
            }
        }
        Some("make") => {
            let out = PathBuf::from(&args[2]);
            for name in &args[3..] {
                let dir = write(&scenario(name), &out.join(name));
                println!("{name} {}", dir.display());
            }
        }
        _ => panic!("usage: probe census DIR... | probe make OUT NAME..."),
    }
}
