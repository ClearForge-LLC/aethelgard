//! `AET-WO-2300` §1.1 and §5: the scenario format, the campaign records
//! through the archive, `check_run`, `check_archived`, and `run_id`. Applying a
//! scenario to a specification is the CLI's (`src/campaign.rs`'s tests).

mod common;

use std::fs;

use aethelgard_chaos::campaign::{
    HELD_LOW, MAX_EXPECTED, MAX_OVERRIDES, Scenario, ScenarioError, check_run, run_id,
};
use aethelgard_chaos::cbor;
use aethelgard_chaos::record::{BusLine, Line, Override, Reading, Record};
use aethelgard_chaos::synthetic;
use aethelgard_chaos::{ArchiveError as E, Event, Manifest, TraceWriter, replay_dir};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use common::*;

const PAST: &str = "analog_rise_exceeds_ceiling";

// ---- helpers ----

fn root(rel: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn doc(overrides: Value, expect: &str, findings: Value) -> Vec<u8> {
    serde_json::to_vec(&json!({
        "schema_version": 1,
        "name": "t",
        "base": "b.json",
        "expect": expect,
        "expect_findings": findings,
        "overrides": overrides,
    }))
    .unwrap()
}

fn pass_doc(overrides: Value) -> Vec<u8> {
    doc(overrides, "pass", json!([]))
}

fn parse(v: &Value) -> Result<Scenario, ScenarioError> {
    Scenario::parse(&serde_json::to_vec(v).unwrap())
}

fn refused_kind(bytes: &[u8]) -> &'static str {
    Scenario::parse(bytes).expect_err("must be refused").kind
}

fn scenario_with(overrides: Value) -> Scenario {
    Scenario::parse(&pass_doc(overrides)).unwrap()
}

// ---- 1. the reference scenarios ----

#[test]
fn reference_scenario_2k2_parses() {
    let bytes = fs::read(root("scenarios/sentinel-01/pullup-2k2.json")).unwrap();
    let s = Scenario::parse(&bytes).unwrap();
    assert_eq!(s.name, "pullup-2k2");
    assert!(s.expect_pass);
    assert!(s.expect_findings.is_empty());
    assert_eq!(
        s.overrides,
        vec![Override::PullUpOhms {
            bus: "SENSORS".into(),
            ohms: 2200
        }]
    );
    assert_eq!(s.sha256, <[u8; 32]>::from(Sha256::digest(&bytes)));
}

#[test]
fn reference_scenario_47k_parses() {
    let bytes = fs::read(root("scenarios/sentinel-01/pullup-47k.json")).unwrap();
    let s = Scenario::parse(&bytes).unwrap();
    assert_eq!(s.name, "pullup-47k");
    assert!(!s.expect_pass);
    assert_eq!(
        s.expect_findings,
        vec![PAST.to_owned(), "i2c_timeout".into()]
    );
    assert!(s.expect_findings.windows(2).all(|w| w[0] < w[1]));
    assert_eq!(
        s.overrides,
        vec![Override::PullUpOhms {
            bus: "SENSORS".into(),
            ohms: 47000
        }]
    );
    assert_eq!(s.sha256, <[u8; 32]>::from(Sha256::digest(&bytes)));
}

#[test]
fn expected_findings_are_sorted_whatever_the_file_order() {
    let s = Scenario::parse(&doc(json!([]), "fail", json!(["z_kind", "a_kind"]))).unwrap();
    assert_eq!(
        s.expect_findings,
        vec!["a_kind".to_owned(), "z_kind".into()]
    );
}

// ---- 2. parse refusals ----

#[test]
fn unknown_top_level_key_is_refused() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["extra"] = json!(1);
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
}

#[test]
fn a_positional_array_is_refused() {
    let k = refused_kind(br#"[1,"t","b","pass",[],[]]"#);
    assert_eq!(k, "SchemaViolation");
    // And an override as an array.
    let k = refused_kind(&pass_doc(json!([["SENSORS", 2200]])));
    assert_eq!(k, "SchemaViolation");
}

#[test]
fn a_duplicate_top_level_key_is_refused() {
    let raw = br#"{"schema_version":1,"name":"t","name":"u","base":"b","expect":"pass","expect_findings":[],"overrides":[]}"#;
    assert_eq!(refused_kind(raw), "SchemaViolation");
}

#[test]
fn a_duplicate_key_inside_an_override_is_refused() {
    let raw = br#"{"schema_version":1,"name":"t","base":"b","expect":"pass","expect_findings":[],"overrides":[{"bus":"A","bus":"B","pull_up_ohms":1}]}"#;
    assert_eq!(refused_kind(raw), "SchemaViolation");
}

#[test]
fn schema_version_three_is_unsupported() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["schema_version"] = json!(3);
    assert_eq!(parse(&v).unwrap_err().kind, "UnsupportedSchemaVersion");
}

#[test]
fn a_name_with_a_space_is_refused() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["name"] = json!("has space");
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
}

#[test]
fn a_65_byte_name_is_refused_and_64_accepted() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["name"] = json!("n".repeat(65));
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
    v["name"] = json!("n".repeat(64));
    assert!(parse(&v).is_ok());
}

#[test]
fn pass_with_findings_is_a_contradiction() {
    let e = Scenario::parse(&doc(json!([]), "pass", json!(["x"]))).unwrap_err();
    assert_eq!(e.kind, "ExpectContradiction");
}

#[test]
fn fail_without_findings_is_a_contradiction() {
    let e = Scenario::parse(&doc(json!([]), "fail", json!([]))).unwrap_err();
    assert_eq!(e.kind, "ExpectContradiction");
}

#[test]
fn a_finding_kind_listed_twice_is_refused() {
    let e = Scenario::parse(&doc(json!([]), "fail", json!(["x", "x"]))).unwrap_err();
    assert_eq!(e.kind, "SchemaViolation");
    assert!(e.message.contains("twice"), "{}", e.message);
}

#[test]
fn an_override_with_an_extra_key_is_refused_even_when_null() {
    for extra in [json!(null), json!(1)] {
        let o = json!([{ "bus": "SENSORS", "pull_up_ohms": 2200, "net": extra }]);
        assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
    }
    let o = json!([{ "bus": "SENSORS", "pull_up_ohms": 2200, "line_held_low": null }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
    let o = json!([{ "bus": "SENSORS", "pull_up_ohms": 2200, "bogus": null }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
}

#[test]
fn an_override_mixing_bus_and_net_is_refused() {
    let o = json!([{ "bus": "SENSORS", "net": "I2C_SDA", "pull_up_ohms": 2200 }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
    let o = json!([{ "bus": "SENSORS", "net": "I2C_SDA", "max_capacitance_femtofarads": 1 }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
    // An empty override, and one with only a value.
    assert_eq!(refused_kind(&pass_doc(json!([{}]))), "SchemaViolation");
    let o = json!([{ "pull_up_ohms": 2200 }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
}

#[test]
fn line_held_low_must_be_sda_or_scl() {
    for bad in [json!("SDA"), json!("x"), json!(0), json!(null)] {
        let o = json!([{ "bus": "SENSORS", "line_held_low": bad }]);
        assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation", "{bad}");
    }
    for (name, line) in [("sda", Line::Sda), ("scl", Line::Scl)] {
        let s = scenario_with(json!([{ "bus": "SENSORS", "line_held_low": name }]));
        assert_eq!(
            s.overrides,
            vec![Override::LineHeldLow {
                bus: "SENSORS".into(),
                line
            }]
        );
    }
}

#[test]
fn pull_up_ohms_must_be_an_unsigned_32_bit_integer() {
    for bad in ["-1", "2200.0", "4294967296", "\"2200\"", "null", "1e3"] {
        let raw = format!(
            r#"{{"schema_version":1,"name":"t","base":"b","expect":"pass","expect_findings":[],"overrides":[{{"bus":"SENSORS","pull_up_ohms":{bad}}}]}}"#
        );
        assert_eq!(refused_kind(raw.as_bytes()), "SchemaViolation", "{bad}");
    }
    let s = scenario_with(json!([{ "bus": "SENSORS", "pull_up_ohms": 4294967295u64 }]));
    assert_eq!(
        s.overrides,
        vec![Override::PullUpOhms {
            bus: "SENSORS".into(),
            ohms: u32::MAX
        }]
    );
}

#[test]
fn a_negative_or_float_capacitance_is_refused() {
    for bad in ["-1", "1.5", "18446744073709551616"] {
        let raw = format!(
            r#"{{"schema_version":1,"name":"t","base":"b","expect":"pass","expect_findings":[],"overrides":[{{"net":"N","max_capacitance_femtofarads":{bad}}}]}}"#
        );
        assert_eq!(refused_kind(raw.as_bytes()), "SchemaViolation", "{bad}");
    }
}

#[test]
fn two_overrides_of_the_same_bus_pull_up_are_refused() {
    let o = json!([
        { "bus": "SENSORS", "pull_up_ohms": 2200 },
        { "bus": "SENSORS", "pull_up_ohms": 4700 },
    ]);
    let e = Scenario::parse(&pass_doc(o)).unwrap_err();
    assert_eq!(e.kind, "DuplicateOverride");
    assert_eq!(e.id.as_deref(), Some("SENSORS"));
}

#[test]
fn two_overrides_of_the_same_net_capacitance_are_refused() {
    let o = json!([
        { "net": "N", "max_capacitance_femtofarads": 1 },
        { "net": "N", "max_capacitance_femtofarads": 2 },
    ]);
    assert_eq!(refused_kind(&pass_doc(o)), "DuplicateOverride");
}

#[test]
fn different_things_on_one_bus_are_not_duplicates() {
    let o = json!([
        { "bus": "SENSORS", "pull_up_ohms": 2200 },
        { "bus": "SENSORS", "line_held_low": "sda" },
        { "bus": "SENSORS", "line_held_low": "scl" },
    ]);
    assert_eq!(scenario_with(o).overrides.len(), 3);
}

#[test]
fn thirty_three_overrides_are_refused_and_thirty_two_accepted() {
    let mk = |n: usize| -> Value {
        (0..n)
            .map(|i| json!({ "bus": format!("B{i}"), "pull_up_ohms": 1 }))
            .collect()
    };
    assert_eq!(MAX_OVERRIDES, 32);
    assert!(Scenario::parse(&pass_doc(mk(32))).is_ok());
    assert_eq!(refused_kind(&pass_doc(mk(33))), "SchemaViolation");
}

#[test]
fn seventeen_expected_kinds_are_refused_and_sixteen_accepted() {
    let mk = |n: usize| -> Value { (0..n).map(|i| json!(format!("k{i:02}"))).collect() };
    assert_eq!(MAX_EXPECTED, 16);
    assert!(Scenario::parse(&doc(json!([]), "fail", mk(16))).is_ok());
    let e = Scenario::parse(&doc(json!([]), "fail", mk(17))).unwrap_err();
    assert_eq!(e.kind, "SchemaViolation");
}

#[test]
fn bytes_that_are_not_json_are_invalid_json() {
    for raw in [&b""[..], b"not json", b"{", b"\xff\xfe"] {
        assert_eq!(refused_kind(raw), "InvalidJson", "{raw:?}");
    }
}

#[test]
fn a_valid_scenario_followed_by_trailing_bytes_is_invalid_json() {
    let mut raw = pass_doc(json!([]));
    raw.extend(b" trailing");
    assert_eq!(refused_kind(&raw), "InvalidJson");
}

#[test]
fn an_empty_base_and_a_bad_id_are_refused() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["base"] = json!("");
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
    let o = json!([{ "bus": "has space", "pull_up_ohms": 1 }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
    let o = json!([{ "bus": "B".repeat(65), "pull_up_ohms": 1 }]);
    assert_eq!(refused_kind(&pass_doc(o)), "SchemaViolation");
}

// ---- 4. every record variant through the archive ----

fn reading(observed: Option<u32>) -> Reading {
    Reading {
        address: 0x4000_5400,
        mask: 0xff,
        expected: 0x12,
        observed,
    }
}

/// The passing verdict, likewise archived alone.
fn passing_verdict() -> Record {
    Record::Verdict {
        pass: true,
        findings: vec![("device_unmodelled".into(), "info".into(), 1)],
    }
}

/// The boot with every reading absent: a run holds one boot (A11.3), so it is
/// archived in a run of its own.
fn absent_boot() -> Record {
    Record::BootObserved {
        image_sha256: [5; 32],
        boot_word: reading(None),
        instructions_expected: 1000,
        instructions_observed: None,
        pc: None,
        executable: vec![(0, 0x1000_0000)],
    }
}

fn one_of_each() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "pullup-47k".into(),
            sha256: [1; 32],
            expect_pass: false,
            expect_findings: vec![PAST.into(), "i2c_timeout".into()],
            overrides: vec![
                Override::PullUpOhms {
                    bus: "SENSORS".into(),
                    ohms: u32::MAX,
                },
                Override::MaxCapacitance {
                    net: "I2C_SDA".into(),
                    femtofarads: u64::MAX,
                },
                Override::LineHeldLow {
                    bus: "SENSORS".into(),
                    line: Line::Scl,
                },
            ],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        Record::RiseComputed {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            rise_30_70_ps: 450_000,
            complete: true,
            ceiling_ps: 300_000,
            exceeds: true,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
        Record::RiseComputed {
            bus: "SENSORS".into(),
            net: "I2C_SCL".into(),
            rise_30_70_ps: 100,
            complete: false,
            ceiling_ps: 300_000,
            exceeds: true,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: -123_456,
            ceiling_ps: 300_000,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
        Record::BusFault {
            bus: "SENSORS".into(),
            fault: "stuck".into(),
            source: "analog bridge".into(),
            from: vec!["analog.rise_time_computed > ceiling_ns".into()],
            nets: vec!["I2C_SCL".into(), "I2C_SDA".into()],
            firmware_sees: "timeout".into(),
        },
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(Some(0x12)),
            instructions_expected: 1000,
            instructions_observed: Some(999),
            pc: Some(0x0800_0100),
            executable: vec![(0, 0x1000_0000)],
        },
        Record::RegisterObserved {
            name: "RCC_CR".into(),
            reading: reading(Some(0)),
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            register: 0x75,
            expected: 0x71,
            observed: Some(0x71),
            status: None,
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            register: 0x75,
            expected: 0x71,
            observed: None,
            status: Some("timeout_txis".into()),
        },
        Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: "U2".into(),
            device: "i2c1.u2".into(),
            expected: vec![
                "[NOISY] i2c1.u2: Write 75".into(),
                "[NOISY] i2c1.u2: Read 71".into(),
            ],
            observed: None,
            absent: vec![0x77],
        },
        Record::Verdict {
            pass: false,
            findings: vec![
                (PAST.into(), "error".into(), 2),
                ("device_unmodelled".into(), "info".into(), 1),
            ],
        },
        solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[("U2", 0x76)]),
        solved_bus("S2", "I2C2", ["N_SCL", "N_SDA"], &[]),
    ]
}

fn as_events(records: Vec<Record>) -> Vec<Event> {
    records
        .into_iter()
        .enumerate()
        .map(|(i, record)| Event::Record {
            timestamp: i as u64 * 10,
            record,
        })
        .collect()
}

#[test]
fn every_record_variant_round_trips_through_the_archive() {
    let records = one_of_each();
    let (dir, manifest) = write_run("records-roundtrip", 7, &as_events(records.clone()));
    let replayed = replay_dir(&dir).unwrap();
    assert_eq!(replayed.state.records, records);
    assert_eq!(replayed.manifest.state_hash, manifest.state_hash);
    assert_eq!(replayed.manifest, manifest);
    assert_eq!(replayed.manifest.event_count, records.len() as u64);
    // Kinds 5 through 15 are all present.
    let kinds: std::collections::BTreeSet<u64> = records.iter().map(Record::kind).collect();
    assert_eq!(kinds, (5..=15).collect());
    for (name, lone) in [
        ("records-roundtrip-absent-boot", vec![absent_boot()]),
        ("records-roundtrip-passing-verdict", vec![passing_verdict()]),
    ] {
        let (dir, _) = write_run(name, 7, &as_events(lone.clone()));
        assert_eq!(replay_dir(&dir).unwrap().state.records, lone);
    }
}

#[test]
fn a_changed_record_changes_the_state_hash() {
    let a = one_of_each();
    let mut b = one_of_each();
    let Record::Solved {
        topology_sha256, ..
    } = &mut b[1]
    else {
        panic!()
    };
    topology_sha256[31] ^= 1;
    let (_, ma) = write_run("hash-a", 7, &as_events(a));
    let (_, mb) = write_run("hash-b", 7, &as_events(b));
    assert_ne!(ma.state_hash, mb.state_hash);
}

// ---- 5. the writer refuses a bad record ----

fn append_one(name: &str, record: Record) -> Result<(), E> {
    let dir = scratch(name).join("run");
    let mut w = TraceWriter::create(&dir, &synthetic::header(1))?;
    w.append(&Event::Record {
        timestamp: 0,
        record,
    })
}

fn refused(name: &str, record: Record) -> &'static str {
    match append_one(name, record) {
        Err(E::InvalidEvent { index: 0, reason }) => reason,
        other => panic!("expected InvalidEvent, got {other:?}"),
    }
}

fn rise(complete: bool, rise: u64, exceeds: bool) -> Record {
    Record::RiseComputed {
        bus: "B".into(),
        net: "N".into(),
        rise_30_70_ps: rise,
        complete,
        ceiling_ps: 300_000,
        exceeds,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    }
}

#[test]
fn rise_exceeds_must_agree_with_the_ceiling() {
    let why = "exceeds is not the rise against the ceiling";
    assert_eq!(refused("rise-1", rise(true, 450_000, false)), why);
    assert_eq!(refused("rise-2", rise(true, 100, true)), why);
    assert_eq!(refused("rise-3", rise(false, 100, false)), why);
    assert_eq!(refused("rise-4", rise(false, 450_000, false)), why);
    // Equal to the ceiling does not exceed it.
    assert_eq!(refused("rise-5", rise(true, 300_000, true)), why);
    assert!(append_one("rise-ok1", rise(true, 300_000, false)).is_ok());
    assert!(append_one("rise-ok2", rise(true, 300_001, true)).is_ok());
    assert!(append_one("rise-ok3", rise(false, 100, true)).is_ok());
}

fn fault(from: Vec<&str>, nets: Vec<&str>) -> Record {
    Record::BusFault {
        bus: "SENSORS".into(),
        fault: "stuck".into(),
        source: "s".into(),
        from: from.into_iter().map(Into::into).collect(),
        nets: nets.into_iter().map(Into::into).collect(),
        firmware_sees: String::new(),
    }
}

#[test]
fn a_bus_fault_with_unsorted_or_duplicate_nets_is_refused() {
    let why = "fault nets are not sorted unique names";
    assert_eq!(refused("bf-1", fault(vec!["c"], vec!["B", "A"])), why);
    assert_eq!(refused("bf-2", fault(vec!["c"], vec!["A", "A"])), why);
    assert_eq!(refused("bf-3", fault(vec!["c"], vec![])), why);
    assert!(append_one("bf-ok", fault(vec!["c"], vec!["A", "B"])).is_ok());
}

#[test]
fn a_bus_fault_with_no_cause_is_refused() {
    let why = "fault causes are not 1 to 64 printable ASCII bytes each";
    assert_eq!(refused("bf-from-1", fault(vec![], vec!["A"])), why);
    assert_eq!(refused("bf-from-2", fault(vec![""], vec!["A"])), why);
}

fn verdict(pass: bool, findings: &[(&str, &str, u64)]) -> Record {
    Record::Verdict {
        pass,
        findings: findings
            .iter()
            .map(|(k, s, n)| ((*k).into(), (*s).into(), *n))
            .collect(),
    }
}

#[test]
fn a_passing_verdict_beside_an_error_finding_is_refused() {
    let why = "pass is not the absence of a finding above info";
    assert_eq!(refused("v-1", verdict(true, &[("a", "error", 1)])), why);
    assert_eq!(refused("v-2", verdict(true, &[("a", "warning", 1)])), why);
    assert_eq!(refused("v-3", verdict(false, &[("a", "info", 1)])), why);
    assert_eq!(refused("v-4", verdict(false, &[])), why);
    assert!(append_one("v-ok1", verdict(true, &[])).is_ok());
    assert!(append_one("v-ok2", verdict(true, &[("a", "info", 1)])).is_ok());
}

#[test]
fn unsorted_or_duplicate_verdict_findings_are_refused() {
    let why = "findings are not sorted and unique by kind and severity";
    let fs = [("b", "error", 1), ("a", "error", 1)];
    assert_eq!(refused("vs-1", verdict(false, &fs)), why);
    let fs = [("a", "error", 1), ("a", "error", 2)];
    assert_eq!(refused("vs-2", verdict(false, &fs)), why);
    // The same kind at two severities is allowed, in severity order.
    let ok = [("a", "error", 1), ("a", "info", 1)];
    assert!(append_one("vs-ok", verdict(false, &ok)).is_ok());
}

#[test]
fn a_verdict_finding_with_a_count_of_zero_is_refused() {
    let why = "a finding is not a kind, a severity and a count above 0";
    assert_eq!(refused("v0", verdict(false, &[("a", "error", 0)])), why);
}

fn big_scenario(overrides: usize) -> Record {
    Record::Scenario {
        name: "n".repeat(64),
        sha256: [0; 32],
        expect_pass: true,
        expect_findings: vec![],
        overrides: (0..overrides)
            .map(|i| Override::MaxCapacitance {
                net: format!("{i:0>64}"),
                femtofarads: u64::MAX,
            })
            .collect(),
    }
}

#[test]
fn a_scenario_record_over_4096_bytes_is_refused_by_the_writer() {
    let why = refused("big", big_scenario(60));
    assert!(why.contains("4096"), "{why}");
}

#[test]
fn the_largest_scenario_parse_accepts_fits_the_event_bound() {
    let overrides: Vec<Value> = (0..MAX_OVERRIDES)
        .map(|i| json!({ "net": format!("{i:0>64}"), "max_capacitance_femtofarads": u64::MAX }))
        .collect();
    let kinds: Vec<String> = (0..MAX_EXPECTED).map(|i| format!("{i:x>64}")).collect();
    let mut v = json!({
        "schema_version": 1,
        "name": "n".repeat(64),
        "base": "b",
        "expect": "fail",
        "expect_findings": kinds,
        "overrides": overrides,
    });
    let s = parse(&v).expect("the maximal scenario parses");
    assert_eq!(s.overrides.len(), MAX_OVERRIDES);
    assert_eq!(s.expect_findings.len(), MAX_EXPECTED);
    let event = Event::Record {
        timestamp: u64::MAX,
        record: s.record(),
    };
    let len = cbor::to_bytes(&event.to_value()).len();
    assert!(len <= 4096, "maximal scenario record is {len} bytes");
    assert!(append_one("max-scenario", s.record()).is_ok());
    // Pull-up overrides are shorter; u32 ohms with 64-byte buses fit too.
    v["overrides"] = (0..MAX_OVERRIDES)
        .map(|i| json!({ "bus": format!("{i:0>64}"), "pull_up_ohms": u32::MAX }))
        .collect();
    let s = parse(&v).unwrap();
    assert!(append_one("max-scenario-pu", s.record()).is_ok());
}

// ---- 6. crafted kind-7 events ----

fn raw_with_event(name: &str, item: Vec<u8>) -> Result<(), E> {
    let trace = {
        let mut t = header_bytes();
        t.extend(item);
        t
    };
    let dir = scratch(name).join("run");
    fs::create_dir(&dir).unwrap();
    fs::write(dir.join("trace.cbor"), &trace).unwrap();
    let manifest = Manifest {
        trace_bytes: trace.len() as u64,
        trace_sha256: Sha256::digest(&trace).into(),
        event_count: 1,
        state_hash: [0; 32],
    };
    fs::write(
        dir.join("manifest.cbor"),
        cbor::to_bytes(&manifest.to_value()),
    )
    .unwrap();
    replay_dir(&dir).map(|_| ())
}

/// `[7, 0, "A", "B", <rise>, 1, 300000, 0, [400000, 400000]]`
fn kind7_with(rise: &[u8]) -> Vec<u8> {
    let mut item = vec![0x89, 0x07, 0x00, 0x61, b'A', 0x61, b'B'];
    item.extend(rise);
    item.extend([0x01, 0x1a, 0x00, 0x04, 0x93, 0xe0, 0x00]);
    // The window (`AET-WO-2205` §1.3): 400000 is 0x00061a80.
    item.extend([
        0x82, 0x1a, 0x00, 0x06, 0x1a, 0x80, 0x1a, 0x00, 0x06, 0x1a, 0x80,
    ]);
    item
}

#[test]
fn the_crafted_kind_7_frame_is_otherwise_well_formed() {
    // Guards the two refusals below: the frame is right, only the field differs.
    // rise 65536 > ceiling 300000 is false, so exceeds 0 is right; the state
    // hash is wrong, so the replay gets as far as diverging.
    let ok = kind7_with(&[0x1a, 0x00, 0x01, 0x00, 0x00]);
    let r = raw_with_event("k7-ok", ok);
    assert!(matches!(r, Err(E::ReplayDiverged)), "{r:?}");
}

#[test]
fn a_float_where_the_rise_belongs_is_not_deterministic_cbor() {
    // half-, single- and double-precision 1.0
    for float in [
        vec![0xf9, 0x3c, 0x00],
        vec![0xfa, 0x3f, 0x80, 0, 0],
        vec![0xfb, 0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
    ] {
        let r = raw_with_event("k7-float", kind7_with(&float));
        assert!(matches!(r, Err(E::NotDeterministicCbor(_))), "{r:?}");
    }
}

#[test]
fn a_map_where_the_rise_belongs_is_not_deterministic_cbor() {
    for map in [vec![0xa0], vec![0xa1, 0x00, 0x00]] {
        let r = raw_with_event("k7-map", kind7_with(&map));
        assert!(matches!(r, Err(E::NotDeterministicCbor(_))), "{r:?}");
    }
}

// ---- 7. check_run ----

const CAUSE: &str = "analog.rise_time_computed > ceiling_ns";

fn good() -> Vec<Record> {
    let rise = |net: &str| Record::RiseComputed {
        bus: "SENSORS".into(),
        net: net.into(),
        rise_30_70_ps: 450_000,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: true,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    };
    vec![
        Record::Scenario {
            name: "pullup-47k".into(),
            sha256: [1; 32],
            expect_pass: false,
            expect_findings: vec![PAST.into(), "i2c_timeout".into()],
            overrides: vec![Override::PullUpOhms {
                bus: "SENSORS".into(),
                ohms: 47000,
            }],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[("U2", 0x68)]),
        rise("I2C_SCL"),
        rise("I2C_SDA"),
        fault(vec![CAUSE], vec!["I2C_SCL", "I2C_SDA"]),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(Some(0x12)),
            instructions_expected: 10,
            instructions_observed: Some(10),
            pc: Some(0),
            executable: vec![(0, 0x1000_0000)],
        },
        Record::RegisterObserved {
            name: "R".into(),
            reading: reading(Some(0x12)),
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            register: 0x75,
            expected: 0x71,
            observed: None,
            status: Some("timeout_txis".into()),
        },
        witness("I2C1", "U2", 0x75, 0x71, Some(vec![])),
        verdict(
            false,
            &[
                (PAST, "error", 2),
                ("device_unmodelled", "info", 1),
                ("i2c_timeout", "error", 1),
            ],
        ),
    ]
}

/// Kind 13 for the identity read of `reference` (register `register`, value
/// `value`), as the BME280 model logs it: the device is `i2c1.` and the
/// reference lowercased.
fn witness(
    peripheral: &str,
    reference: &str,
    register: u8,
    value: u8,
    observed: Option<Vec<String>>,
) -> Record {
    let device = format!("i2c1.{}", reference.to_ascii_lowercase());
    Record::I2cWitness {
        peripheral: peripheral.into(),
        reference: reference.into(),
        expected: vec![
            format!("[NOISY] {device}: Write {register:X}"),
            format!("[NOISY] {device}: Read {value:X}"),
        ],
        device,
        observed,
        absent: Vec::new(),
    }
}

/// Kind 15 for `bus` on `peripheral`, SCL then SDA, with a 300,000 ps ceiling.
fn solved_bus(bus: &str, peripheral: &str, nets: [&str; 2], devices: &[(&str, u8)]) -> Record {
    // Each controller on its own pins, so two solved buses never share one.
    let pins = match peripheral {
        "I2C1" => ["PB6", "PB7"],
        "I2C2" => ["PF1", "PF0"],
        _ => ["PH7", "PH8"],
    };
    let line = |n: usize, suffix: &str, pin: &str| BusLine {
        signal: format!("{peripheral}{suffix}"),
        net: nets[n].into(),
        pin: pin.into(),
        alternate_function: 4,
        ceiling_ps: 300_000,
    };
    Record::I2cBusSolved {
        bus: bus.into(),
        peripheral: peripheral.into(),
        lines: vec![line(0, "_SCL", pins[0]), line(1, "_SDA", pins[1])],
        devices: devices.iter().map(|(r, a)| ((*r).into(), *a)).collect(),
    }
}

fn on_bus(mut r: Record, b: &str) -> Record {
    if let Record::BusFault { bus, .. } = &mut r {
        *bus = b.into();
    }
    r
}

fn expecting(records: &mut [Record], pass: bool, kinds: &[&str]) {
    let Record::Scenario {
        expect_pass,
        expect_findings,
        ..
    } = &mut records[0]
    else {
        panic!()
    };
    *expect_pass = pass;
    *expect_findings = kinds.iter().map(|s| (*s).into()).collect();
}

fn position(records: &[Record], pred: impl Fn(&Record) -> bool) -> usize {
    records.iter().position(pred).unwrap()
}

#[test]
fn the_47k_shaped_run_matches_what_it_expects() {
    let s = check_run(&good()).unwrap();
    assert_eq!(s.name, "pullup-47k");
    assert!(!s.pass && !s.expect_pass);
    assert_eq!(s.failing_kinds, vec![PAST.to_owned(), "i2c_timeout".into()]);
    assert!(s.matched);
}

#[test]
fn a_run_expecting_fewer_kinds_does_not_match() {
    let mut r = good();
    expecting(&mut r, false, &[PAST]);
    let s = check_run(&r).unwrap();
    assert!(!s.matched);
    assert_eq!(s.failing_kinds.len(), 2);
}

#[test]
fn a_run_expecting_more_kinds_does_not_match() {
    let mut r = good();
    expecting(&mut r, false, &[PAST, "extra", "i2c_timeout"]);
    assert!(!check_run(&r).unwrap().matched);
}

#[test]
fn a_failing_run_expecting_to_pass_does_not_match() {
    let mut r = good();
    expecting(&mut r, true, &[]);
    let s = check_run(&r).unwrap();
    assert!(!s.matched);
    assert!(!s.pass);
}

#[test]
fn info_findings_never_count_toward_a_match() {
    let mut r = good();
    expecting(&mut r, false, &[PAST, "device_unmodelled", "i2c_timeout"]);
    assert!(!check_run(&r).unwrap().matched);
}

#[test]
fn records_out_of_order_are_refused() {
    let order = "the records are not in the campaign's order";
    let mut r = good();
    r.swap(0, 1);
    assert_eq!(check_run(&r).unwrap_err(), order);
    let mut r = good();
    let last = r.len() - 1;
    r.swap(last - 1, last);
    assert_eq!(check_run(&r).unwrap_err(), order);
    let mut r = good();
    r.swap(3, 5); // fault before the measurements
    assert_eq!(check_run(&r).unwrap_err(), order);
    let mut r = good();
    r.swap(2, 3); // a solved bus after a measurement
    assert_eq!(check_run(&r).unwrap_err(), order);
    assert_eq!(check_run(&[]).unwrap_err(), order);
}

#[test]
fn a_fault_with_no_analog_record_is_refused() {
    let mut r = good();
    r.retain(|x| !matches!(x, Record::RiseComputed { .. }));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a bus fault names a net with no measurement past its ceiling"
    );
}

#[test]
fn a_dropped_bus_fault_is_refused() {
    let mut r = good();
    r.retain(|x| !matches!(x, Record::BusFault { .. }));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a measurement past its ceiling faulted no bus"
    );
}

#[test]
fn a_fault_covering_only_one_of_two_failing_lines_is_refused() {
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r[i] = fault(vec![CAUSE], vec!["I2C_SDA"]);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a measurement past its ceiling faulted no bus"
    );
}

#[test]
fn a_fault_on_the_wrong_bus_is_refused() {
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r[i] = on_bus(fault(vec![CAUSE], vec!["I2C_SCL", "I2C_SDA"]), "OTHER");
    assert_eq!(check_run(&r).unwrap_err(), NO_BUS);
    // With OTHER solved too, a fault on its own lines names no measurement.
    r[i] = on_bus(fault(vec![CAUSE], vec!["O_SCL", "O_SDA"]), "OTHER");
    r.insert(2, solved_bus("OTHER", "I2C2", ["O_SCL", "O_SDA"], &[]));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a bus fault names a net with no measurement past its ceiling"
    );
}

#[test]
fn a_verdict_flipped_to_pass_is_refused() {
    let mut r = good();
    let last = r.len() - 1;
    r[last] = verdict(true, &[("device_unmodelled", "info", 1)]);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "the verdict's analog and held-low findings do not match the records"
    );
}

#[test]
fn an_analog_finding_count_that_does_not_match_is_refused() {
    let why = "the verdict's analog and held-low findings do not match the records";
    for n in [0u64, 1, 3] {
        let mut r = good();
        let last = r.len() - 1;
        let mut findings = vec![(PAST.to_owned(), "error".to_owned(), n)];
        findings.push(("i2c_timeout".into(), "error".into(), 1));
        findings.retain(|f| f.2 > 0);
        r[last] = Record::Verdict {
            pass: false,
            findings,
        };
        assert_eq!(check_run(&r).unwrap_err(), why, "count {n}");
    }
}

#[test]
fn a_finding_at_info_severity_does_not_satisfy_the_analog_count() {
    let mut r = good();
    let last = r.len() - 1;
    r[last] = verdict(false, &[(PAST, "info", 2), ("i2c_timeout", "error", 1)]);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "the verdict's analog and held-low findings do not match the records"
    );
}

#[test]
fn two_verdicts_are_refused() {
    let mut r = good();
    let v = r.last().unwrap().clone();
    r.push(v);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a run holds exactly one scenario, topology, boot and verdict"
    );
}

#[test]
fn a_run_with_no_boot_record_is_refused() {
    let mut r = good();
    r.retain(|x| !matches!(x, Record::BootObserved { .. }));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a run holds exactly one scenario, topology, boot and verdict"
    );
}

#[test]
fn a_run_with_no_verdict_or_no_scenario_or_no_topology_is_refused() {
    let why = "a run holds exactly one scenario, topology, boot and verdict";
    let mut r = good();
    r.pop();
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    r.remove(0);
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    r.remove(1);
    assert_eq!(check_run(&r).unwrap_err(), why);
}

fn held_low_run() -> Vec<Record> {
    let mut r = good();
    r.retain(|x| {
        !matches!(
            x,
            Record::RiseComputed { .. } | Record::BusFault { .. } | Record::Verdict { .. }
        )
    });
    let Record::Scenario {
        overrides,
        expect_findings,
        ..
    } = &mut r[0]
    else {
        panic!()
    };
    *overrides = vec![Override::LineHeldLow {
        bus: "SENSORS".into(),
        line: Line::Sda,
    }];
    *expect_findings = vec!["i2c_line_held_low".into()];
    r.insert(3, fault(vec![HELD_LOW], vec!["I2C_SDA"]));
    r.push(verdict(false, &[("i2c_line_held_low", "error", 1)]));
    r
}

#[test]
fn a_held_low_run_is_ok_and_matches() {
    let s = check_run(&held_low_run()).unwrap();
    assert!(s.matched);
    assert_eq!(s.failing_kinds, vec!["i2c_line_held_low".to_owned()]);
}

#[test]
fn a_held_low_fault_with_no_override_is_refused() {
    let mut r = held_low_run();
    let Record::Scenario { overrides, .. } = &mut r[0] else {
        panic!()
    };
    overrides.clear();
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low fault names no held-low override of its bus"
    );
}

#[test]
fn a_held_low_fault_on_another_bus_is_refused() {
    let mut r = held_low_run();
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r[i] = on_bus(fault(vec![HELD_LOW], vec!["I2C_SDA"]), "OTHER");
    assert_eq!(check_run(&r).unwrap_err(), NO_BUS);
    r[i] = on_bus(fault(vec![HELD_LOW], vec!["O_SDA"]), "OTHER");
    r.insert(2, solved_bus("OTHER", "I2C2", ["O_SCL", "O_SDA"], &[]));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low fault names no held-low override of its bus"
    );
}

#[test]
fn a_held_low_fault_mixed_with_another_cause_is_refused() {
    let mut r = held_low_run();
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r[i] = fault(vec![HELD_LOW, CAUSE], vec!["I2C_SDA"]);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low fault names no held-low override of its bus"
    );
}

#[test]
fn a_held_low_override_with_no_fault_is_refused() {
    let mut r = held_low_run();
    r.retain(|x| !matches!(x, Record::BusFault { .. }));
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low override faulted no bus, or one bus twice"
    );
}

#[test]
fn one_held_low_override_faulting_twice_is_refused() {
    let mut r = held_low_run();
    let f = r[3].clone();
    r.insert(4, f);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low override faulted no bus, or one bus twice"
    );
}

// ---- 7a. the joins of kind 15 (AET-WO-3100 R2) ----

const NO_BUS: &str = "an analog record or a bus fault names a bus no kind 15 solves";

fn at(r: &mut [Record], pred: impl Fn(&Record) -> bool) -> &mut Record {
    let i = position(r, pred);
    &mut r[i]
}

fn is_bus(x: &Record) -> bool {
    matches!(x, Record::I2cBusSolved { .. })
}

#[test]
fn the_good_run_carries_one_solved_bus_and_passes_the_joins() {
    let r = good();
    assert_eq!(r.iter().filter(|x| is_bus(x)).count(), 1);
    assert!(check_run(&r).is_ok());
}

#[test]
fn a_run_without_its_solved_bus_is_refused() {
    let mut r = good();
    r.retain(|x| !is_bus(x));
    assert_eq!(check_run(&r).unwrap_err(), NO_BUS);
}

#[test]
fn two_solved_records_for_one_bus_or_unsorted_buses_are_refused() {
    let why = "the solved buses are not sorted by unique bus";
    let mut r = good();
    let b = r[2].clone();
    r.insert(3, b);
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    r.insert(2, solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[]));
    assert_eq!(check_run(&r).unwrap_err(), why);
    // Sorted, another bus beside it: accepted.
    let mut r = good();
    r.insert(3, solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[]));
    assert!(check_run(&r).is_ok());
}

#[test]
fn a_measurement_or_fault_on_a_net_that_is_not_its_buss_line_is_refused() {
    let why = "an analog record or a bus fault names a net that is not a line of its bus";
    let mut r = good();
    if let Record::I2cBusSolved { lines, .. } = at(&mut r, is_bus) {
        lines[1].net = "ELSEWHERE".into();
    }
    assert_eq!(check_run(&r).unwrap_err(), why);
}

#[test]
fn an_identity_read_of_no_solved_device_or_at_another_address_is_refused() {
    let why = "an identity read names no device of a solved bus at its address";
    let mut r = good();
    if let Record::I2cBusSolved { devices, .. } = at(&mut r, is_bus) {
        devices.clear();
    }
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    if let Record::I2cCheck { address, .. } = at(&mut r, |x| matches!(x, Record::I2cCheck { .. })) {
        *address = 0x69;
    }
    assert_eq!(check_run(&r).unwrap_err(), why);
}

#[test]
fn a_device_on_two_solved_buses_is_refused() {
    let mut r = good();
    r.insert(
        3,
        solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[("U2", 0x68)]),
    );
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a device is on two solved buses"
    );
}

/// The reference run with its witness moved to `peripheral` and `reference`.
fn with_witness(peripheral: &str, reference: &str) -> Vec<Record> {
    let mut r = good();
    for x in &mut r {
        if let Record::I2cWitness {
            peripheral: p,
            reference: n,
            ..
        } = x
        {
            *p = peripheral.into();
            *n = reference.into();
        }
    }
    r
}

#[test]
fn a_witness_must_name_a_solved_device_on_its_own_controller() {
    assert!(check_run(&with_witness("I2C1", "U2")).is_ok());
    assert_eq!(
        check_run(&with_witness("I2C2", "U2")).unwrap_err(),
        "a witness is not on its device's solved bus"
    );
    assert_eq!(
        check_run(&with_witness("I2C1", "U9")).unwrap_err(),
        "a witness names no device of a solved bus"
    );
}

#[test]
fn a_solved_bus_round_trips_and_its_field_rules_are_enforced() {
    let ok = solved_bus(
        "SENSORS",
        "I2C1",
        ["I2C_SCL", "I2C_SDA"],
        &[("U2", 0x76), ("U3", 0x48)],
    );
    assert!(ok.check().is_ok());
    let back = Record::from_fields(ok.kind(), &ok.fields()).unwrap();
    assert_eq!(back, ok);
    assert_eq!(ok.kind(), 15);
    assert!(ok.before_boot());
    type Edit<'a> = &'a dyn Fn(&mut Vec<BusLine>, &mut Vec<(String, u8)>);
    let edit = |f: Edit| {
        let mut r = ok.clone();
        if let Record::I2cBusSolved { lines, devices, .. } = &mut r {
            f(lines, devices);
        }
        r.check().unwrap_err()
    };
    let order = "a solved bus's lines are not its peripheral's SCL, then its SDA";
    assert_eq!(edit(&|l, _| l.swap(0, 1)), order);
    assert_eq!(edit(&|l, _| l.truncate(1)), order);
    assert_eq!(edit(&|l, _| l[0].signal = "I2C2_SCL".into()), order);
    assert_eq!(
        edit(&|l, _| l[1].net = l[0].net.clone()),
        "a solved bus's two lines share a net or a pin"
    );
    assert_eq!(
        edit(&|_, d| d.reverse()),
        "a solved bus's devices are not sorted unique references"
    );
    assert_eq!(
        edit(&|l, _| l[0].pin = "P B6".into()),
        "a line's signal, net or pin is not 1 to 64 printable ASCII bytes"
    );
}

// ---- 8. run_id ----

#[test]
fn run_id_is_a_pure_function_of_its_inputs() {
    let (a, b, c) = ([1u8; 32], [2u8; 32], [3u8; 32]);
    assert_eq!(run_id(&a, &b, &c), run_id(&a, &b, &c));
    assert_ne!(run_id(&a, &b, &c), run_id(&b, &a, &c));
    assert_ne!(run_id(&a, &b, &c), run_id(&a, &c, &b));
}

#[test]
fn any_one_bit_of_any_input_changes_the_run_id() {
    let base = [[0x5au8; 32], [0xa5; 32], [0x3c; 32]];
    let id = |i: &[[u8; 32]; 3]| run_id(&i[0], &i[1], &i[2]);
    let want = id(&base);
    let mut seen = std::collections::BTreeSet::from([want]);
    for which in 0..3 {
        for byte in 0..32 {
            let mut changed = base;
            changed[which][byte] ^= 0x01;
            let got = id(&changed);
            assert_ne!(got, want, "input {which} byte {byte}");
            assert!(seen.insert(got), "collision at input {which} byte {byte}");
        }
    }
}

// ---- 9. gaps found by the adversarial pass and the mutation run ----

#[test]
fn an_expected_finding_kind_that_is_not_a_name_is_refused() {
    for bad in ["has space", "", &"k".repeat(65)] {
        let bytes = doc(json!([]), "fail", json!([bad]));
        assert_eq!(refused_kind(&bytes), "SchemaViolation", "kind {bad:?}");
    }
    assert!(Scenario::parse(&doc(json!([]), "fail", json!(["fine_kind"]))).is_ok());
}

fn record_bytes(record: Record) -> Vec<u8> {
    cbor::to_bytes(
        &Event::Record {
            timestamp: 0,
            record,
        }
        .to_value(),
    )
}

#[test]
fn the_reader_applies_the_record_field_rules_the_writer_does() {
    let rise_with = |bus: &str| Record::RiseComputed {
        bus: bus.into(),
        net: "N".into(),
        rise_30_70_ps: 100,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
    };
    let cases: Vec<(&str, Record)> = vec![
        ("rd-rise-65", rise_with(&"B".repeat(65))),
        ("rd-rise-space", rise_with("has space")),
        (
            "rd-scenario-name",
            Record::Scenario {
                name: "bad name".into(),
                sha256: [0; 32],
                expect_pass: true,
                expect_findings: vec![],
                overrides: vec![],
            },
        ),
        ("rd-fault-from", fault(vec![], vec!["A"])),
        (
            "rd-i2c-ref",
            Record::I2cCheck {
                reference: "bad ref".into(),
                address: 0x68,
                register: 0x75,
                expected: 0x71,
                observed: None,
                status: None,
            },
        ),
        (
            "rd-verdict-unsorted",
            verdict(false, &[("b", "error", 1), ("a", "error", 1)]),
        ),
    ];
    for (name, record) in cases {
        // The writer refuses each of these ...
        assert!(
            matches!(
                append_one(name, record.clone()),
                Err(E::InvalidEvent { .. })
            ),
            "{name}: writer"
        );
        // ... and so does the reader, on the same bytes.
        let r = raw_with_event(&format!("{name}-raw"), record_bytes(record));
        assert!(
            matches!(r, Err(E::MalformedEvent { index: 0, .. })),
            "{name}: reader gave {r:?}"
        );
    }
}

#[test]
fn a_kind_7_event_with_an_extra_trailing_field_is_refused() {
    let mut value = Event::Record {
        timestamp: 0,
        record: rise(true, 100, false),
    }
    .to_value();
    let cbor::Value::Array(items) = &mut value else {
        panic!()
    };
    items.push(cbor::Value::Uint(0));
    let r = raw_with_event("k7-extra", cbor::to_bytes(&value));
    assert!(
        matches!(r, Err(E::MalformedEvent { index: 0, .. })),
        "{r:?}"
    );
}

#[test]
fn a_kind_7_complete_flag_of_2_is_refused() {
    let mut value = Event::Record {
        timestamp: 0,
        record: rise(true, 100, false),
    }
    .to_value();
    let cbor::Value::Array(items) = &mut value else {
        panic!()
    };
    // [kind, timestamp, bus, net, rise, complete, ceiling, exceeds]
    assert_eq!(items[5], cbor::Value::Uint(1));
    items[5] = cbor::Value::Uint(2);
    let r = raw_with_event("k7-flag2", cbor::to_bytes(&value));
    assert!(
        matches!(r, Err(E::MalformedEvent { index: 0, .. })),
        "{r:?}"
    );
}

// ---- 10. check_archived ----

mod archived {
    use super::*;
    use aethelgard_chaos::Header;
    use aethelgard_chaos::campaign::{ARTIFACTS, check_archived};

    const SCEN: [u8; 32] = [1; 32];
    const SPEC: [u8; 32] = [2; 32];
    const TOPO: [u8; 32] = [3; 32];
    const FW: [u8; 32] = [4; 32];
    const BASE: [u8; 32] = [5; 32];

    fn header() -> Header {
        let hashes = [BASE, FW, SCEN, SPEC, TOPO];
        Header {
            run_id: run_id(&SCEN, &SPEC, &FW),
            artifacts: ARTIFACTS
                .iter()
                .zip(hashes)
                .map(|(n, h)| ((*n).to_owned(), h))
                .collect(),
            tools: vec![
                ("aethelgard".into(), "0.1.0".into()),
                ("renode".into(), "1.17.0+20260907gitf1dd1b4af".into()),
            ],
        }
    }

    fn records() -> Vec<Record> {
        vec![
            Record::Scenario {
                name: "ok".into(),
                sha256: SCEN,
                expect_pass: true,
                expect_findings: vec![],
                overrides: vec![],
            },
            Record::Solved {
                specification_sha256: SPEC,
                topology_sha256: TOPO,
            },
            Record::BootObserved {
                image_sha256: FW,
                boot_word: reading(Some(0x12)),
                instructions_expected: 10,
                instructions_observed: Some(10),
                pc: Some(0),
                executable: vec![(0, 0x1000_0000)],
            },
            Record::RegisterObserved {
                name: "R".into(),
                reading: reading(Some(0x12)),
            },
            verdict(true, &[]),
        ]
    }

    fn replayed(name: &str, header: &Header, raw_write: bool) -> aethelgard_chaos::Replayed {
        let dir = scratch(name).join("run");
        let mut w = TraceWriter::create(&dir, header).unwrap();
        if raw_write {
            w.append(&write(0, 0x4000_0000, 1)).unwrap();
        }
        for r in records() {
            let timestamp = if r.kind() < 10 { 0 } else { 100_000_000 };
            w.append(&Event::Record {
                timestamp,
                record: r,
            })
            .unwrap();
        }
        w.commit().unwrap();
        replay_dir(&dir).unwrap()
    }

    fn judged(name: &str, header: &Header, raw_write: bool) -> Result<(), &'static str> {
        let r = replayed(name, header, raw_write);
        check_archived(&r.header, &r.state).map(|_| ())
    }

    #[test]
    fn the_header_names_the_engine_and_renode_and_the_bridge_only_when_it_ran() {
        const TOOLS: &str = "the header's tools are not the ones its records say judged the run";
        let tool = |n: &str| (n.to_owned(), "v".to_owned());
        // `records()` computed no line: no bridge and no ngspice.
        for tools in [
            vec![],
            vec![tool("aethelgard")],
            vec![tool("renode")],
            vec![
                tool("aethelgard"),
                tool("aethelgard-bridge"),
                tool("ngspice"),
                tool("renode"),
            ],
            vec![tool("aethelgard"), tool("qemu"), tool("renode")],
        ] {
            let mut h = header();
            h.tools = tools.clone();
            assert_eq!(
                judged("ca-tools", &h, false).unwrap_err(),
                TOOLS,
                "{tools:?}"
            );
        }
        // Any version passes: replay does not compare them with this host's.
        let mut h = header();
        h.tools = vec![
            ("aethelgard".into(), "99.0.0".into()),
            ("renode".into(), "0.0.1+forged".into()),
        ];
        judged("ca-tools-any", &h, false).unwrap();
        // A computed line brings the bridge and its ngspice.
        let mut lines = records();
        lines.insert(
            2,
            solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[]),
        );
        assert_eq!(
            aethelgard_chaos::campaign::tools_of(&lines),
            ["aethelgard", "renode"]
        );
        lines.insert(
            3,
            Record::RiseComputed {
                bus: "SENSORS".into(),
                net: "I2C_SCL".into(),
                rise_30_70_ps: 1,
                complete: true,
                ceiling_ps: 300_000,
                exceeds: false,
                window: aethelgard_chaos::record::Window {
                    deck_hz: 400_000,
                    declared_hz: 400_000,
                },
            },
        );
        assert_eq!(
            aethelgard_chaos::campaign::tools_of(&lines),
            ["aethelgard", "aethelgard-bridge", "ngspice", "renode"]
        );
    }

    #[test]
    fn a_header_whose_tools_are_unsorted_or_not_printable_is_not_written() {
        for tools in [
            vec![("renode", "v"), ("aethelgard", "v")],
            vec![("aethelgard", "v"), ("aethelgard", "w")],
            vec![("aethelgard", "")],
            vec![("aethelgard", "v\n")],
            vec![("aethelgard", "é")],
            vec![("", "v")],
        ] {
            let mut h = header();
            h.tools = tools
                .iter()
                .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
                .collect();
            let dir = scratch("ca-tools-bad").join("run");
            assert!(
                matches!(TraceWriter::create(&dir, &h), Err(E::InvalidHeader(_))),
                "{tools:?}"
            );
        }
        let mut h = header();
        h.tools = vec![("aethelgard".into(), "v".repeat(257))];
        assert!(TraceWriter::create(&scratch("ca-tools-long").join("run"), &h).is_err());
    }

    #[test]
    fn a_consistent_run_is_ok_and_matched() {
        let r = replayed("ca-ok", &header(), false);
        let summary = check_archived(&r.header, &r.state).unwrap();
        assert!(summary.matched && summary.pass);
    }

    #[test]
    fn a_header_scenario_hash_that_differs_from_the_record_is_refused() {
        let mut h = header();
        h.artifacts[2].1 = [9; 32];
        h.run_id = run_id(&[9; 32], &SPEC, &FW);
        assert_eq!(
            judged("ca-scen", &h, false).unwrap_err(),
            "the header's hashes are not the ones its records name"
        );
    }

    #[test]
    fn a_header_topology_hash_that_differs_from_the_record_is_refused() {
        let mut h = header();
        h.artifacts[4].1 = [9; 32];
        assert_eq!(
            judged("ca-topo", &h, false).unwrap_err(),
            "the header's hashes are not the ones its records name"
        );
    }

    #[test]
    fn a_header_specification_hash_that_differs_from_the_record_is_refused() {
        let mut h = header();
        h.artifacts[3].1 = [9; 32];
        h.run_id = run_id(&SCEN, &[9; 32], &FW);
        assert_eq!(
            judged("ca-spec", &h, false).unwrap_err(),
            "the header's hashes are not the ones its records name"
        );
    }

    #[test]
    fn a_zeroed_run_id_is_refused() {
        let mut h = header();
        h.run_id = [0; 16];
        assert_eq!(
            judged("ca-zero", &h, false).unwrap_err(),
            "the run id is not the one its scenario, specification and firmware give"
        );
    }

    #[test]
    fn a_changed_firmware_artifact_no_longer_matches_the_run_id() {
        let mut h = header();
        h.artifacts[1].1 = [9; 32];
        assert_eq!(
            judged("ca-fw", &h, false).unwrap_err(),
            "the run id is not the one its scenario, specification and firmware give"
        );
    }

    #[test]
    fn a_missing_or_extra_artifact_is_refused() {
        let why = "the header's artifacts are not a campaign run's";
        let mut h = header();
        h.artifacts.remove(0);
        assert_eq!(judged("ca-missing", &h, false).unwrap_err(), why);
        let mut h = header();
        h.artifacts.push(("zzz".into(), [7; 32]));
        assert_eq!(judged("ca-extra", &h, false).unwrap_err(), why);
    }

    #[test]
    fn a_raw_register_write_among_the_records_is_refused() {
        assert_eq!(
            judged("ca-raw", &header(), true).unwrap_err(),
            "a campaign run holds an event that is not a campaign record"
        );
    }
}

// ---- 7b. the adversarial pass and the mutation run (AET-WO-3100) ----

fn bus_fields(r: &mut Record) -> (&mut Vec<BusLine>, &mut Vec<(String, u8)>) {
    match r {
        Record::I2cBusSolved { lines, devices, .. } => (lines, devices),
        _ => panic!("not a solved bus"),
    }
}

#[test]
fn every_kind_15_field_rule_refuses_alone() {
    let ok = solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[("U2", 0x76)]);
    let refused = |f: &dyn Fn(&mut Record)| {
        let mut r = ok.clone();
        f(&mut r);
        r.check().unwrap_err()
    };
    let names = "bus or peripheral name is not 1 to 64 printable ASCII bytes";
    let line_names = "a line's signal, net or pin is not 1 to 64 printable ASCII bytes";
    let order = "a solved bus's lines are not its peripheral's SCL, then its SDA";
    let pin =
        "a line's pin is not P, a port letter and 0 to 15, or its alternate function is over 15";
    let rename = |bus: bool| {
        move |r: &mut Record| {
            if let Record::I2cBusSolved {
                bus: b,
                peripheral: p,
                ..
            } = r
            {
                if bus {
                    *b = "A B".into()
                } else {
                    *p = "A B".into()
                }
            }
        }
    };
    assert_eq!(refused(&rename(true)), names);
    assert_eq!(refused(&rename(false)), names);
    assert_eq!(
        refused(&|r| bus_fields(r).0[0].net = "A B".into()),
        line_names
    );
    assert_eq!(
        refused(&|r| bus_fields(r).0[1].signal = "I2C1 SDA".into()),
        line_names
    );
    let third = |r: &mut Record| {
        let l = bus_fields(r).0[1].clone();
        bus_fields(r).0.push(BusLine {
            net: "X".into(),
            pin: "PB9".into(),
            ..l
        });
    };
    assert_eq!(refused(&third), order);
    assert_eq!(
        refused(&|r| bus_fields(r).0[1].signal = "I2C1_SCL".into()),
        order
    );
    assert_eq!(
        refused(&|r| bus_fields(r).0[1].pin = "PB6".into()),
        "a solved bus's two lines share a net or a pin"
    );
    for bad in ["PB16", "PB06", "PB007", "PZ", "SCL", "Pb6", "PB+5", "XB6"] {
        assert_eq!(
            refused(&|r| bus_fields(r).0[0].pin = bad.into()),
            pin,
            "{bad}"
        );
    }
    assert_eq!(
        refused(&|r| bus_fields(r).0[0].alternate_function = 16),
        pin
    );
    let devices = "a solved bus's devices are not sorted unique references";
    assert_eq!(refused(&|r| bus_fields(r).1[0].0 = "U 2".into()), devices);
    assert_eq!(
        refused(&|r| bus_fields(r).1.push(("U2".into(), 0x77))),
        devices
    );
    // The bounds themselves are accepted.
    let mut edge = ok.clone();
    bus_fields(&mut edge).0[0].pin = "PK15".into();
    bus_fields(&mut edge).0[0].alternate_function = 15;
    bus_fields(&mut edge).0[1].pin = "PA0".into();
    assert!(edge.check().is_ok());
}

#[test]
fn split_pin_takes_only_the_solvers_form() {
    use aethelgard_chaos::record::split_pin;
    assert_eq!(split_pin("PB6"), Some(('B', 6)));
    assert_eq!(split_pin("PA15"), Some(('A', 15)));
    assert_eq!(split_pin("PK0"), Some(('K', 0)));
    for bad in [
        "B6", "Pb6", "PB", "PB123", "PB6x", "", "P", "PB16", "PB06", "PB007", "PB+5", "QB6",
    ] {
        assert_eq!(split_pin(bad), None, "{bad}");
    }
}

#[test]
fn a_malformed_kind_15_payload_is_refused_by_the_reader() {
    use aethelgard_chaos::cbor::Value as C;
    let ok = solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[("U2", 0x76)]);
    let fields = ok.fields();
    let edit = |f: &dyn Fn(&mut Vec<C>)| {
        let mut v = fields.clone();
        f(&mut v);
        Record::from_fields(15, &v)
    };
    assert_eq!(edit(&|_| {}), Some(ok.clone()));
    fn line(v: &mut [C]) -> &mut Vec<C> {
        let C::Array(lines) = &mut v[2] else { panic!() };
        let C::Array(l) = &mut lines[0] else { panic!() };
        l
    }
    fn device(v: &mut [C]) -> &mut Vec<C> {
        let C::Array(ds) = &mut v[3] else { panic!() };
        let C::Array(d) = &mut ds[0] else { panic!() };
        d
    }
    // A line or device that is not an array, a trailing field, a value over 255.
    assert_eq!(
        edit(&|v| if let C::Array(ls) = &mut v[2] {
            ls[0] = C::Uint(1)
        }),
        None
    );
    assert_eq!(
        edit(&|v| if let C::Array(ds) = &mut v[3] {
            ds[0] = C::Uint(1)
        }),
        None
    );
    assert_eq!(edit(&|v| line(v).push(C::Uint(0))), None);
    assert_eq!(edit(&|v| device(v).push(C::Uint(0))), None);
    assert_eq!(edit(&|v| line(v)[3] = C::Uint(256)), None);
    assert_eq!(edit(&|v| device(v)[1] = C::Uint(0x176)), None);
    assert_eq!(
        edit(&|v| {
            line(v).pop();
        }),
        None
    );
    assert_eq!(edit(&|v| v.push(C::Uint(0))), None);
}

#[test]
fn before_boot_is_exactly_the_records_at_virtual_time_0() {
    let before: Vec<u64> = one_of_each()
        .iter()
        .filter(|r| r.before_boot())
        .map(Record::kind)
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    assert_eq!(before, [5, 6, 7, 8, 9, 15]);
}

#[test]
fn a_measurement_on_an_unsolved_bus_or_net_is_refused_even_without_a_fault() {
    let mut r = held_low_run();
    // A rise on a bus no kind 15 solves.
    let at = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r.insert(
        at,
        Record::RiseComputed {
            bus: "OTHER".into(),
            net: "I2C_SCL".into(),
            rise_30_70_ps: 1,
            complete: true,
            ceiling_ps: 300_000,
            exceeds: false,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
    );
    assert_eq!(check_run(&r).unwrap_err(), NO_BUS);
    let mut r = held_low_run();
    r.insert(
        at,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "ELSEWHERE".into(),
            microvolts_at_release: 0,
            ceiling_ps: 300_000,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
    );
    assert_eq!(
        check_run(&r).unwrap_err(),
        "an analog record or a bus fault names a net that is not a line of its bus"
    );
}

#[test]
fn an_analog_ceiling_below_or_above_its_lines_is_refused() {
    for ceiling in [299_999, 300_001] {
        let mut r = good();
        if let Record::I2cBusSolved { lines, .. } = at(&mut r, is_bus) {
            lines[0].ceiling_ps = ceiling;
        }
        assert_eq!(
            check_run(&r).unwrap_err(),
            "an analog record's ceiling is not its solved line's"
        );
    }
}

#[test]
fn every_net_of_a_bus_fault_must_be_a_line_of_its_bus() {
    let mut r = good();
    if let Record::BusFault { nets, .. } = at(&mut r, |x| matches!(x, Record::BusFault { .. })) {
        *nets = vec!["I2C_SCL".into(), "ZZZ".into()];
    }
    assert_eq!(
        check_run(&r).unwrap_err(),
        "an analog record or a bus fault names a net that is not a line of its bus"
    );
}

#[test]
fn two_buses_sharing_a_net_a_pin_or_a_controller_are_refused() {
    let why = "two solved buses share a net, a pin or a controller";
    let mut r = good();
    r.insert(3, solved_bus("Z", "I2C3", ["I2C_SDA", "Z_SDA"], &[]));
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    r.insert(3, solved_bus("Z", "I2C1", ["Z_SCL", "Z_SDA"], &[]));
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    let mut z = solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[]);
    bus_fields(&mut z).0[0].pin = "PB7".into();
    r.insert(3, z);
    assert_eq!(check_run(&r).unwrap_err(), why);
}

#[test]
fn two_devices_at_one_address_on_one_bus_are_refused() {
    let mut r = good();
    if let Record::I2cBusSolved { devices, .. } = at(&mut r, is_bus) {
        devices.push(("U3".into(), 0x68));
    }
    assert_eq!(
        check_run(&r).unwrap_err(),
        "two devices on one solved bus share an address"
    );
}

#[test]
fn a_line_measured_twice_or_a_device_read_or_witnessed_twice_is_refused() {
    let why = "a line measured twice, or a device read or witnessed twice";
    // A second rise for one line, with the verdict's count raised to match.
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::RiseComputed { .. }));
    let dup = r[i].clone();
    r.insert(i, dup);
    let last = r.len() - 1;
    r[last] = verdict(false, &[(PAST, "error", 3), ("i2c_timeout", "error", 1)]);
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::I2cCheck { .. }));
    let dup = r[i].clone();
    r.insert(i, dup);
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = with_witness("I2C1", "U2");
    let i = position(&r, |x| matches!(x, Record::I2cWitness { .. }));
    let dup = r[i].clone();
    r.insert(i, dup);
    assert_eq!(check_run(&r).unwrap_err(), why);
}

#[test]
fn two_buses_on_one_controller_are_refused_even_on_their_own_pins() {
    let mut r = good();
    let mut z = solved_bus("Z", "I2C1", ["Z_SCL", "Z_SDA"], &[]);
    bus_fields(&mut z).0[0].pin = "PH7".into();
    bus_fields(&mut z).0[1].pin = "PH8".into();
    r.insert(3, z);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "two solved buses share a net, a pin or a controller"
    );
}

#[test]
fn a_rise_and_a_stuck_high_on_one_line_are_two_measurements() {
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::RiseComputed { .. }));
    r.insert(
        i + 2,
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SCL".into(),
            microvolts_at_release: 0,
            ceiling_ps: 300_000,
            window: aethelgard_chaos::record::Window {
                deck_hz: 400_000,
                declared_hz: 400_000,
            },
        },
    );
    let last = r.len() - 1;
    r[last] = verdict(
        false,
        &[
            (PAST, "error", 2),
            ("analog_line_stuck_high", "error", 1),
            ("i2c_timeout", "error", 1),
        ],
    );
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a line measured twice, or a device read or witnessed twice"
    );
}

/// `AET-WO-2204`: the witness is per identity read, so a bus with two devices
/// has two witnesses and the "controller witnessed twice" rule is gone.
#[test]
fn two_devices_on_one_controller_have_two_witnesses() {
    let mut r = good();
    if let Record::I2cBusSolved { devices, .. } = at(&mut r, is_bus) {
        devices.push(("U3".into(), 0x48));
    }
    let w = position(&r, |x| matches!(x, Record::I2cWitness { .. }));
    r.insert(
        w,
        Record::I2cCheck {
            reference: "U3".into(),
            address: 0x48,
            register: 0x75,
            expected: 0x71,
            observed: None,
            status: Some("timeout_txis".into()),
        },
    );
    // A second read with no witness of its own is refused.
    assert_eq!(
        check_run(&r).unwrap_err(),
        "an identity read without its witness, or a witness without its read"
    );
    r.insert(w + 2, witness("I2C1", "U3", 0x75, 0x71, Some(vec![])));
    assert!(check_run(&r).is_ok());
}

#[test]
fn each_identity_read_has_a_witness_and_each_witness_a_read() {
    let why = "an identity read without its witness, or a witness without its read";
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::I2cWitness { .. }));
    r.remove(i);
    assert_eq!(check_run(&r).unwrap_err(), why);
    let mut r = good();
    let i = position(&r, |x| matches!(x, Record::I2cCheck { .. }));
    r.remove(i);
    assert_eq!(check_run(&r).unwrap_err(), why);
}

#[test]
fn a_witness_device_must_end_with_its_references_lowercased() {
    let mut r = good();
    if let Record::I2cWitness { device, .. } =
        at(&mut r, |x| matches!(x, Record::I2cWitness { .. }))
    {
        *device = "i2c1.u3".into();
    }
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a witness's model path is not its device's emitted name"
    );
}

// AET-WO-2205: the header's tools, kind 10's segments, kinds 7 and 8's window.

const PC_OUTSIDE: &str =
    "the verdict's pc_outside_firmware does not match the boot's pc and segments";

/// `good()` with its boot's `pc` and segments set, and the verdict's
/// `pc_outside_firmware` count.
fn with_pc(pc: u32, executable: Vec<(u32, u32)>, finding: bool) -> Vec<Record> {
    let mut r = good();
    for x in &mut r {
        if let Record::BootObserved {
            pc: p,
            executable: e,
            ..
        } = x
        {
            (*p, *e) = (Some(pc), executable.clone());
        }
    }
    let last = r.len() - 1;
    let mut findings = vec![
        (PAST, "error", 2),
        ("device_unmodelled", "info", 1),
        ("i2c_timeout", "error", 1),
    ];
    if finding {
        findings.push(("pc_outside_firmware", "error", 1));
        findings.sort();
    }
    r[last] = verdict(false, &findings);
    r
}

#[test]
fn pc_outside_firmware_is_exactly_a_pc_outside_the_archived_segments() {
    let segs = vec![(0x0800_0000, 0x0800_2000), (0x2000_0000, 0x2000_0100)];
    // Inside, no finding; outside, the finding: both agree.
    check_run(&with_pc(0x0800_1000, segs.clone(), false)).unwrap();
    check_run(&with_pc(0x7000_0000, segs.clone(), true)).unwrap();
    // The end of a segment is outside it.
    check_run(&with_pc(0x0800_2000, segs.clone(), true)).unwrap();
    // A finding the segments do not show, and a pc outside with no finding.
    assert_eq!(
        check_run(&with_pc(0x0800_1000, segs.clone(), true)).unwrap_err(),
        PC_OUTSIDE
    );
    assert_eq!(
        check_run(&with_pc(0x7000_0000, segs, false)).unwrap_err(),
        PC_OUTSIDE
    );
    // No segment at all: every pc is outside.
    check_run(&with_pc(0x0800_1000, vec![], true)).unwrap();
}

#[test]
fn kind_10s_segments_and_kind_7s_window_are_checked_on_write_and_read() {
    let boot = |executable: Vec<(u32, u32)>| Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(Some(0x12)),
        instructions_expected: 10,
        instructions_observed: Some(10),
        pc: Some(0),
        executable,
    };
    assert!(boot(vec![(0, 1), (1, 2)]).check().is_ok());
    assert!(
        boot((0..16).map(|k| (2 * k, 2 * k + 1)).collect())
            .check()
            .is_ok()
    );
    for bad in [
        vec![(1, 1)],
        vec![(2, 1)],
        vec![(4, 8), (0, 2)],
        vec![(0, 4), (2, 8)],
        (0..17).map(|k| (2 * k, 2 * k + 1)).collect(),
    ] {
        assert_eq!(
            boot(bad.clone()).check().unwrap_err(),
            "executable segments are not at most 16 sorted, disjoint, nonempty ranges",
            "{bad:?}"
        );
    }
    let rise = |deck_hz, declared_hz| Record::RiseComputed {
        bus: "B".into(),
        net: "N".into(),
        rise_30_70_ps: 1,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
        window: aethelgard_chaos::record::Window {
            deck_hz,
            declared_hz,
        },
    };
    assert!(rise(400_000, 100_000).check().is_ok());
    for (d, b) in [(0, 400_000), (400_000, 0)] {
        assert_eq!(
            rise(d, b).check().unwrap_err(),
            "a window's deck and declared speeds are not both above 0"
        );
    }
    // Both round-trip through the profile.
    for r in [
        rise(400_000, 100_000),
        boot(vec![(0x0800_0000, 0x0800_2000)]),
    ] {
        let ev = Event::Record {
            timestamp: 0,
            record: r,
        };
        assert_eq!(Event::from_value(&ev.to_value()).unwrap(), ev);
    }
}
