//! `AET-WO-2300` §1.1 and §5: the scenario format, the campaign records
//! through the archive, `check_run`, `check_archived`, and `run_id`. Applying a
//! scenario to a specification is the CLI's (`src/campaign.rs`'s tests).

mod common;

use std::fs;

use aethelgard_chaos::campaign::{
    HELD_LOW, MAX_EXPECTED, MAX_OVERRIDES, Scenario, ScenarioError, check_run, run_id,
};
use aethelgard_chaos::cbor;
use aethelgard_chaos::record::{Line, Override, Reading, Record};
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
fn schema_version_two_is_unsupported() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["schema_version"] = json!(2);
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
        Record::RiseMeasured {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            rise_30_70_ps: 450_000,
            complete: true,
            ceiling_ps: 300_000,
            exceeds: true,
        },
        Record::RiseMeasured {
            bus: "SENSORS".into(),
            net: "I2C_SCL".into(),
            rise_30_70_ps: 100,
            complete: false,
            ceiling_ps: 300_000,
            exceeds: true,
        },
        Record::LineStuckHigh {
            bus: "SENSORS".into(),
            net: "I2C_SDA".into(),
            microvolts_at_release: -123_456,
            ceiling_ps: 300_000,
        },
        Record::BusFault {
            bus: "SENSORS".into(),
            fault: "stuck".into(),
            source: "analog bridge".into(),
            from: vec!["analog.rise_time_measured > ceiling_ns".into()],
            nets: vec!["I2C_SCL".into(), "I2C_SDA".into()],
            firmware_sees: "timeout".into(),
        },
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(Some(0x12)),
            instructions_expected: 1000,
            instructions_observed: Some(999),
            pc: Some(0x0800_0100),
        },
        Record::BootObserved {
            image_sha256: [5; 32],
            boot_word: reading(None),
            instructions_expected: 1000,
            instructions_observed: None,
            pc: None,
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
            cr2: reading(Some(1)),
            txdr: reading(None),
        },
        Record::Verdict {
            pass: false,
            findings: vec![
                (PAST.into(), "error".into(), 2),
                ("device_unmodelled".into(), "info".into(), 1),
            ],
        },
        Record::Verdict {
            pass: true,
            findings: vec![("device_unmodelled".into(), "info".into(), 1)],
        },
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
    // Kinds 5 through 14 are all present.
    let kinds: std::collections::BTreeSet<u64> = records.iter().map(Record::kind).collect();
    assert_eq!(kinds, (5..=14).collect());
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
    Record::RiseMeasured {
        bus: "B".into(),
        net: "N".into(),
        rise_30_70_ps: rise,
        complete,
        ceiling_ps: 300_000,
        exceeds,
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

/// `[7, 0, "A", "B", <rise>, 1, 300000, 0]`
fn kind7_with(rise: &[u8]) -> Vec<u8> {
    let mut item = vec![0x88, 0x07, 0x00, 0x61, b'A', 0x61, b'B'];
    item.extend(rise);
    item.extend([0x01, 0x1a, 0x00, 0x04, 0x93, 0xe0, 0x00]);
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

const CAUSE: &str = "analog.rise_time_measured > ceiling_ns";

fn good() -> Vec<Record> {
    let rise = |net: &str| Record::RiseMeasured {
        bus: "SENSORS".into(),
        net: net.into(),
        rise_30_70_ps: 450_000,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: true,
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
        rise("I2C_SCL"),
        rise("I2C_SDA"),
        fault(vec![CAUSE], vec!["I2C_SCL", "I2C_SDA"]),
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(Some(0x12)),
            instructions_expected: 10,
            instructions_observed: Some(10),
            pc: Some(0),
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
    r.swap(2, 4); // fault before the measurements
    assert_eq!(check_run(&r).unwrap_err(), order);
    assert_eq!(check_run(&[]).unwrap_err(), order);
}

#[test]
fn a_fault_with_no_analog_record_is_refused() {
    let mut r = good();
    r.retain(|x| !matches!(x, Record::RiseMeasured { .. }));
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
            Record::RiseMeasured { .. } | Record::BusFault { .. } | Record::Verdict { .. }
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
    r.insert(2, fault(vec![HELD_LOW], vec!["I2C_SDA"]));
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
    let f = r[2].clone();
    r.insert(3, f);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low override faulted no bus, or one bus twice"
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
    let rise_with = |bus: &str| Record::RiseMeasured {
        bus: bus.into(),
        net: "N".into(),
        rise_30_70_ps: 100,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
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
