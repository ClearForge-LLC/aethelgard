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
fn schema_version_seven_is_unsupported() {
    let mut v: Value = serde_json::from_slice(&pass_doc(json!([]))).unwrap();
    v["schema_version"] = json!(7);
    assert_eq!(parse(&v).unwrap_err().kind, "UnsupportedSchemaVersion");
    // Versions 5 and 6 (`AET-WO-2310`) read a version-1 file's content as
    // it always did.
    for version in [5, 6] {
        v["schema_version"] = json!(version);
        assert!(parse(&v).is_ok(), "{version}");
    }
}

/// `AET-WO-2302` D5: `{serial_rx_open: port}` is exactly that, a label of 1
/// to 64 printable ASCII bytes, in version 3 alone; one port opened twice is
/// a duplicate, two ports are two overrides; and kind 5 archives it as
/// `[5, port]`.
#[test]
fn an_open_serial_rx_line_is_exactly_its_port_in_version_3() {
    let v3 = |overrides: Value| {
        let mut v: Value =
            serde_json::from_slice(&doc(overrides, "fail", json!(["serial_echo_missing"])))
                .unwrap();
        v["schema_version"] = json!(3);
        v
    };
    let s = parse(&v3(json!([{ "serial_rx_open": "CONSOLE" }]))).unwrap();
    assert_eq!(
        s.overrides,
        [Override::SerialRxOpen {
            port: "CONSOLE".into()
        }]
    );
    for version in [1, 2] {
        let mut v = v3(json!([{ "serial_rx_open": "CONSOLE" }]));
        v["schema_version"] = json!(version);
        let e = parse(&v).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{version}");
        assert!(e.message.contains("schema_version 3"), "{}", e.message);
    }
    for bad in [
        json!([{ "serial_rx_open": "CONSOLE", "bus": "SENSORS" }]),
        json!([{ "serial_rx_open": "CONSOLE", "spi_device_absent": "U4" }]),
        json!([{ "serial_rx_open": "CONSOLE", "net": null }]),
        json!([{ "serial_rx_open": "" }]),
        json!([{ "serial_rx_open": "has space" }]),
        json!([{ "serial_rx_open": 1 }]),
        json!([{ "serial_rx_open": null }]),
        json!([{ "serial_rx_open": "C".repeat(65) }]),
    ] {
        assert_eq!(
            parse(&v3(bad.clone())).unwrap_err().kind,
            "SchemaViolation",
            "{bad}"
        );
    }
    let twice = v3(json!([{ "serial_rx_open": "GPS" }, { "serial_rx_open": "GPS" }]));
    let e = parse(&twice).unwrap_err();
    assert_eq!(
        (e.kind, e.id.as_deref()),
        ("DuplicateOverride", Some("GPS"))
    );
    let two = parse(&v3(
        json!([{ "serial_rx_open": "GPS" }, { "serial_rx_open": "CRSF" }]),
    ));
    assert_eq!(two.unwrap().overrides.len(), 2);
    // A port and an SPI device of one name are two things.
    assert!(
        parse(&v3(
            json!([{ "serial_rx_open": "U4" }, { "spi_device_absent": "U4" }])
        ))
        .is_ok()
    );
    // Kind 5 carries it as `[5, port]`, and reads it back.
    let record = s.record();
    let fields = record.fields();
    let overrides = fields.last().unwrap();
    assert_eq!(
        cbor::to_bytes(overrides),
        cbor::to_bytes(&cbor::Value::Array(vec![cbor::Value::Array(vec![
            cbor::Value::Uint(5),
            cbor::Value::Text("CONSOLE".into()),
        ])]))
    );
    assert_eq!(Record::from_fields(record.kind(), &fields), Some(record));
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
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: None,
        },
        flash_access: None,
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
            brown_out: aethelgard_chaos::record::Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
        Record::RegisterObserved {
            name: "RCC_CR".into(),
            reading: reading(Some(0)),
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            step: "identity".into(),
            write: vec![0x75],
            width: 1,
            expected: 0x71,
            mask: 0xFF,
            observed: Some(0x71),
            status: None,
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            step: "identity".into(),
            write: vec![0x75],
            width: 1,
            expected: 0x71,
            mask: 0xFF,
            observed: None,
            status: Some("timeout_txis".into()),
        },
        Record::I2cWitness {
            step: "identity".into(),
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
    let why = "a finding is not a kind, a severity of error or info, and a count above 0";
    assert_eq!(refused("v0", verdict(false, &[("a", "error", 0)])), why);
}

/// `AET-WO-3201` B3: a severity is `error` or `info`, exactly. Any other,
/// in another case or with whitespace, is refused by the writer, and by
/// the reader in an archive written around the writer: the check is at
/// read, with no byte of the format changed. Before, `zzz` counted as
/// failing.
#[test]
fn a_severity_outside_error_and_info_is_refused_on_write_and_on_read() {
    let why = "a finding is not a kind, a severity of error or info, and a count above 0";
    for (i, severity) in [
        "zzz", "warning", "Error", "INFO", " error", "error ", "info\t", "",
    ]
    .into_iter()
    .enumerate()
    {
        for pass in [false, true] {
            let v = verdict(pass, &[("a", severity, 1)]);
            assert_eq!(
                refused(&format!("sev-w{i}-{pass}"), v.clone()),
                why,
                "{severity:?}"
            );
            let event = Event::Record {
                timestamp: 0,
                record: v,
            };
            let r = raw_with_event(
                &format!("sev-r{i}-{pass}"),
                cbor::to_bytes(&event.to_value()),
            );
            assert!(
                matches!(r, Err(E::MalformedEvent { index: 0, reason, .. }) if reason == why),
                "{severity:?}: {r:?}"
            );
        }
    }
    for severity in aethelgard_chaos::record::SEVERITIES {
        let v = verdict(severity == "info", &[("a", severity, 1)]);
        assert!(append_one(&format!("sev-ok-{severity}"), v).is_ok());
    }
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
            brown_out: aethelgard_chaos::record::Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
        Record::RegisterObserved {
            name: "R".into(),
            reading: reading(Some(0x12)),
        },
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x68,
            step: "identity".into(),
            write: vec![0x75],
            width: 1,
            expected: 0x71,
            mask: 0xFF,
            // The record's value word, which the judge reads with its status
            // (`AET-WO-2309`: all read or all unread).
            observed: Some(0),
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
        step: "identity".into(),
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

/// A computed rise within its ceiling, of `net` on `bus`.
fn healthy_rise(bus: &str, net: &str) -> Record {
    Record::RiseComputed {
        bus: bus.into(),
        net: net.into(),
        rise_30_70_ps: 150_000,
        complete: true,
        ceiling_ps: 300_000,
        exceeds: false,
        window: aethelgard_chaos::record::Window {
            deck_hz: 400_000,
            declared_hz: 400_000,
        },
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
    *expect_findings = vec!["i2c_line_held_low".into(), "i2c_timeout".into()];
    // The other line is measured, healthy; the held one only faulted
    // (`AET-WO-3201` B1). The read times out, as every committed held-low
    // scenario expects, and the verdict names it (`AET-WO-2312`).
    r.insert(3, healthy_rise("SENSORS", "I2C_SCL"));
    r.insert(4, fault(vec![HELD_LOW], vec!["I2C_SDA"]));
    r.push(verdict(
        false,
        &[
            ("i2c_line_held_low", "error", 1),
            ("i2c_timeout", "error", 1),
        ],
    ));
    r
}

#[test]
fn a_held_low_run_is_ok_and_matches() {
    let s = check_run(&held_low_run()).unwrap();
    assert!(s.matched);
    // Before `AET-WO-2312` the fixture left the read's timeout out of its
    // verdict and replayed; the records show it, so that is refused by it.
    let r = with_findings(held_low_run(), &[("i2c_line_held_low", "error", 1)]);
    assert_eq!(check_run(&r).unwrap_err(), why("i2c_timeout"));
    assert_eq!(
        s.failing_kinds,
        vec!["i2c_line_held_low".to_owned(), "i2c_timeout".to_owned()]
    );
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
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    let f = r[i].clone();
    r.insert(i + 1, f);
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
    // Sorted, another bus beside it, each of its lines measured: accepted.
    let mut r = good();
    r.insert(3, solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[]));
    let at = position(&r, |x| matches!(x, Record::BusFault { .. }));
    for net in ["Z_SDA", "Z_SCL"] {
        r.insert(at, healthy_rise("Z", net));
    }
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
                step: "identity".into(),
                write: vec![0x75],
                width: 1,
                expected: 0x71,
                mask: 0xFF,
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
    use aethelgard_chaos::campaign::{ARTIFACTS, RUST_TOOLCHAIN, check_archived};

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
                (
                    RUST_TOOLCHAIN.into(),
                    "rustc 1.91.0 (f8297e351 2025-10-28)".into(),
                ),
            ],
            tool_sha256: vec![(RUST_TOOLCHAIN.into(), [7; 32])],
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
                brown_out: aethelgard_chaos::record::Reading {
                    address: 0x2000_0004,
                    mask: 0xC,
                    expected: 0,
                    observed: Some(0),
                },
                flash_access: Some(Vec::new()),
            },
            Record::RegisterObserved {
                name: "R".into(),
                reading: reading(Some(0x12)),
            },
            verdict(true, &[]),
        ]
    }

    fn replayed(name: &str, header: &Header, raw_write: bool) -> aethelgard_chaos::Replayed {
        replayed_records(name, header, raw_write, records())
    }

    fn replayed_records(
        name: &str,
        header: &Header,
        raw_write: bool,
        records: Vec<Record>,
    ) -> aethelgard_chaos::Replayed {
        let dir = scratch(name).join("run");
        let mut w = TraceWriter::create(&dir, header).unwrap();
        if raw_write {
            w.append(&write(0, 0x4000_0000, 1)).unwrap();
        }
        for r in records {
            // Kinds 5 and 6 before the boot, as before; a solved bus (kind
            // 15) too, which `records_with_lines` holds.
            let timestamp = if r.before_boot() { 0 } else { 100_000_000 };
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
        // `records()` computed no line: no bridge and no ngspice. Each list
        // ends with the host's toolchain, which every run names since
        // `AET-WO-2310`, so it is wrong only as it was before.
        for mut tools in [
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
            tools.push(tool(RUST_TOOLCHAIN));
            let mut h = header();
            h.tools = tools.clone();
            assert_eq!(
                judged("ca-tools", &h, false).unwrap_err(),
                TOOLS,
                "{tools:?}"
            );
        }
        let mut h = header();
        h.tools = vec![];
        h.tool_sha256 = vec![];
        assert_eq!(judged("ca-tools", &h, false).unwrap_err(), TOOLS, "none");
        // Any version passes: replay does not compare them with this host's.
        let mut h = header();
        h.tools = vec![
            ("aethelgard".into(), "99.0.0".into()),
            ("renode".into(), "0.0.1+forged".into()),
            (RUST_TOOLCHAIN.into(), "rustc 0.0.1-forged".into()),
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
            ["aethelgard", "renode", "rust-toolchain"]
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
            [
                "aethelgard",
                "aethelgard-bridge",
                "ngspice",
                "renode",
                "rust-toolchain"
            ]
        );
        // A rail alone ran the bridge; ngspice is named only by a crossing.
        let rail = Record::RailDeclared {
            droop: aethelgard_chaos::record::RailDroop {
                droop: aethelgard_chaos::record::Droop {
                    domain: "VDD".into(),
                    floor_millivolts: 2_000,
                    from_boot_word_ns: 0,
                    fall_ns: 1,
                    hold_ns: 1,
                    rise_ns: 1,
                    source_milliohms: 1,
                    load_milliamps: 0,
                },
                start_ns: 1,
                bor_level: 0,
            },
            nominal_millivolts: 3_300,
            tolerance_permille: 100,
            decoupling_picofarads: 100_000,
        };
        assert_eq!(
            aethelgard_chaos::campaign::tools_of(std::slice::from_ref(&rail)),
            [
                "aethelgard",
                "aethelgard-bridge",
                "renode",
                "rust-toolchain"
            ]
        );
        let crossing = Record::RailCrossing {
            domain: "VDD".into(),
            level: "VBOR0_falling".into(),
            microvolts: 1_620_000,
            at_ps: 1,
            falling: true,
        };
        assert_eq!(
            aethelgard_chaos::campaign::tools_of(&[rail, crossing]),
            [
                "aethelgard",
                "aethelgard-bridge",
                "ngspice",
                "renode",
                "rust-toolchain"
            ]
        );
    }

    const HASHES: &str =
        "the header's tool hashes are not ngspice's library and the toolchain's sysroot";

    /// `records()` with a healthy computed line on each of a solved bus's
    /// two lines: the bridge and ngspice judged the run.
    fn records_with_lines() -> Vec<Record> {
        let mut r = records();
        r.insert(
            2,
            solved_bus("SENSORS", "I2C1", ["I2C_SCL", "I2C_SDA"], &[]),
        );
        r.insert(3, healthy_rise("SENSORS", "I2C_SCL"));
        r.insert(4, healthy_rise("SENSORS", "I2C_SDA"));
        r
    }

    fn judged_records(
        name: &str,
        header: &Header,
        records: Vec<Record>,
    ) -> Result<(), &'static str> {
        let r = replayed_records(name, header, false, records);
        check_archived(&r.header, &r.state).map(|_| ())
    }

    /// (h) `AET-WO-2310`: every run names the host's toolchain, last; the
    /// header hashes ngspice's library whenever ngspice judged the run and
    /// the toolchain's sysroot unless its version is `absent`, and no other
    /// tool's.
    #[test]
    fn the_header_hashes_ngspice_and_the_toolchain_and_no_other_tool() {
        use aethelgard_chaos::campaign::{NGSPICE, TOOLCHAIN_ABSENT};
        judged("ca-hash-ok", &header(), false).unwrap();
        // The toolchain missing from the tools, its hash with it.
        let mut h = header();
        h.tools.retain(|(n, _)| n != RUST_TOOLCHAIN);
        h.tool_sha256.clear();
        assert_eq!(
            judged("ca-hash-no-toolchain", &h, false).unwrap_err(),
            "the header's tools are not the ones its records say judged the run"
        );
        // The toolchain named with no hash.
        let mut h = header();
        h.tool_sha256.clear();
        assert_eq!(
            judged("ca-hash-toolchain-unhashed", &h, false).unwrap_err(),
            HASHES
        );
        // A host with no toolchain: `absent`, and no hash; with one, refused.
        let mut h = header();
        h.tools[2].1 = TOOLCHAIN_ABSENT.into();
        assert_eq!(
            judged("ca-hash-absent-hashed", &h, false).unwrap_err(),
            HASHES
        );
        h.tool_sha256.clear();
        judged("ca-hash-absent", &h, false).unwrap();
        // A hash on renode, beside the toolchain's.
        let mut h = header();
        h.tool_sha256.insert(0, ("renode".into(), [8; 32]));
        assert_eq!(judged("ca-hash-renode", &h, false).unwrap_err(), HASHES);
        // ngspice judged the run: its library is hashed, or refused.
        let tool = |n: &str| (n.to_owned(), "v".to_owned());
        let mut h = header();
        h.tools = vec![
            tool("aethelgard"),
            tool("aethelgard-bridge"),
            tool(NGSPICE),
            tool("renode"),
            tool(RUST_TOOLCHAIN),
        ];
        assert_eq!(
            judged_records("ca-hash-ngspice-unhashed", &h, records_with_lines()).unwrap_err(),
            HASHES
        );
        h.tool_sha256 = vec![(NGSPICE.into(), [9; 32]), (RUST_TOOLCHAIN.into(), [7; 32])];
        judged_records("ca-hash-ngspice", &h, records_with_lines()).unwrap();
        // ngspice hashed, the toolchain not.
        h.tool_sha256.pop();
        assert_eq!(
            judged_records("ca-hash-ngspice-only", &h, records_with_lines()).unwrap_err(),
            HASHES
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
        assert!(matches!(
            TraceWriter::create(&scratch("ca-tools-long").join("run"), &h),
            Err(E::InvalidHeader(
                "a tool is not a name and 1 to 256 printable ASCII bytes of version"
            ))
        ));
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
    let why = "a line measured twice, or a device step read or witnessed twice";
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
        "a line measured twice, or a device step read or witnessed twice"
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
            step: "identity".into(),
            write: vec![0x75],
            width: 1,
            expected: 0x71,
            mask: 0xFF,
            observed: Some(0),
            status: Some("timeout_txis".into()),
        },
    );
    // A second read with no witness of its own is refused.
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a step's read without its witness, or a witness without its read"
    );
    r.insert(w + 2, witness("I2C1", "U3", 0x75, 0x71, Some(vec![])));
    // Its timeout is the judge's second (`AET-WO-2312`): the fixture's old
    // verdict, one timeout, is refused by it.
    assert_eq!(check_run(&r).unwrap_err(), why("i2c_timeout"));
    let r = with_findings(
        r,
        &[
            (PAST, "error", 2),
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 2),
        ],
    );
    assert!(check_run(&r).is_ok());
}

#[test]
fn each_identity_read_has_a_witness_and_each_witness_a_read() {
    let why = "a step's read without its witness, or a witness without its read";
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
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
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

/// `AET-WO-2205` adversarial pass: a `pc` unread beside a boot that was read,
/// `pc_outside_firmware` at `info`, and `pc_not_on_self_loop` beside a core
/// outside the firmware cannot come from the judge, and are refused.
#[test]
fn the_boot_read_is_whole_and_the_pc_findings_are_the_judges() {
    let segs = vec![(0x0800_0000, 0x0800_2000)];
    // pc unread, the rest read, and the reverse: refused at the record, by
    // the writer and the reader alike.
    let boot = |pc: Option<u32>, instructions: Option<u64>| Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(Some(0x12)),
        instructions_expected: 10,
        instructions_observed: instructions,
        pc,
        executable: segs.clone(),
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0,
            observed: Some(0),
        },
        flash_access: Some(Vec::new()),
    };
    for r in [boot(None, Some(10)), boot(Some(0), None)] {
        assert_eq!(
            r.check().unwrap_err(),
            "the boot word, the instruction count, pc and the brown-out word are not all read or all unread"
        );
    }
    boot(Some(0), Some(10)).check().unwrap();
    // pc_outside_firmware at info, pc inside; and error beside info, pc outside.
    let mut r = with_pc(0x0800_1000, segs.clone(), false);
    let last = r.len() - 1;
    r[last] = verdict(
        false,
        &[
            (PAST, "error", 2),
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 1),
            ("pc_outside_firmware", "info", 1),
        ],
    );
    assert_eq!(check_run(&r).unwrap_err(), PC_OUTSIDE);
    let mut r = with_pc(0x7000_0000, segs.clone(), true);
    let last = r.len() - 1;
    r[last] = verdict(
        false,
        &[
            (PAST, "error", 2),
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 1),
            ("pc_outside_firmware", "error", 1),
            ("pc_outside_firmware", "info", 9),
        ],
    );
    assert_eq!(check_run(&r).unwrap_err(), PC_OUTSIDE);
    // pc outside and the finding only at info: the judge makes it an error.
    let mut r = with_pc(0x7000_0000, segs.clone(), false);
    let last = r.len() - 1;
    r[last] = verdict(
        false,
        &[
            (PAST, "error", 2),
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 1),
            ("pc_outside_firmware", "info", 1),
        ],
    );
    assert_eq!(check_run(&r).unwrap_err(), PC_OUTSIDE);
    // pc_not_on_self_loop: kept inside, refused outside.
    let self_loop = |pc: u32, outside: bool| {
        let mut r = with_pc(pc, segs.clone(), outside);
        let last = r.len() - 1;
        let mut f = vec![
            (PAST, "error", 2),
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 1),
            ("pc_not_on_self_loop", "error", 1),
        ];
        if outside {
            f.push(("pc_outside_firmware", "error", 1));
        }
        f.sort();
        r[last] = verdict(false, &f);
        check_run(&r)
    };
    self_loop(0x0800_1000, false).unwrap();
    assert_eq!(
        self_loop(0x7000_0000, true).unwrap_err(),
        "the verdict names pc_not_on_self_loop for a pc outside the firmware or unread"
    );
}

// ---- AET-WO-4210: kinds 12, 13, 21 and 22 per step of a device's script ----

fn i2c_step_check(step: &str, write: Vec<u8>, width: u8, expected: u64, mask: u64) -> Record {
    Record::I2cCheck {
        reference: "U2".into(),
        address: 0x68,
        step: step.into(),
        write,
        width,
        expected,
        mask,
        // A configure step's record still holds a value word, read with its
        // status (`AET-WO-2309`); the judge leaves it unjudged.
        observed: Some(if width > 0 { expected } else { 0 }),
        status: Some("ok".into()),
    }
}

fn i2c_step_witness(step: &str, lines: &[&str]) -> Record {
    let lines: Vec<String> = lines
        .iter()
        .map(|l| format!("[NOISY] i2c1.u2: {l}"))
        .collect();
    Record::I2cWitness {
        step: step.into(),
        peripheral: "I2C1".into(),
        reference: "U2".into(),
        device: "i2c1.u2".into(),
        expected: lines.clone(),
        observed: Some(lines),
        absent: Vec::new(),
    }
}

/// `good()` with U2's identity read followed by a configure and a data step,
/// each witnessed (the timed-out identity read keeps its own witness).
fn three_steps() -> Vec<Record> {
    let mut r = good();
    let c = position(&r, |x| matches!(x, Record::I2cCheck { .. }));
    r.insert(
        c + 1,
        i2c_step_check("configure", vec![0xF2, 0x01], 0, 0, 0),
    );
    r.insert(c + 2, i2c_step_check("data", vec![0xF7], 2, 0x1234, 0xFFFF));
    let w = position(&r, |x| matches!(x, Record::I2cWitness { .. }));
    r.insert(w + 1, i2c_step_witness("configure", &["Write F2 01"]));
    r.insert(w + 2, i2c_step_witness("data", &["Write F7", "Read 12 34"]));
    r
}

#[test]
fn a_device_with_three_steps_round_trips_and_replays() {
    let r = three_steps();
    assert!(check_run(&r).is_ok(), "{:?}", check_run(&r));
    let (dir, manifest) = write_run("three-steps", 7, &as_events(r.clone()));
    let replayed = replay_dir(&dir).unwrap();
    assert_eq!(replayed.state.records, r);
    assert_eq!(replayed.manifest, manifest);
}

#[test]
fn a_step_given_twice_or_out_of_order_is_refused_by_replay() {
    let twice = "a line measured twice, or a device step read or witnessed twice";
    let order = "a device's steps are not in the order identity, configure, data";
    // A step twice, check or witness.
    for kind_is_check in [true, false] {
        let mut r = three_steps();
        let is = |x: &Record| {
            matches!(x, Record::I2cCheck { step, .. } | Record::I2cWitness { step, .. }
                if step == "data")
                && matches!(x, Record::I2cCheck { .. }) == kind_is_check
        };
        let i = position(&r, is);
        let dup = r[i].clone();
        r.insert(i, dup);
        assert_eq!(check_run(&r).unwrap_err(), twice, "check {kind_is_check}");
    }
    // Data before configure, in the checks and in the witnesses.
    for check in [true, false] {
        let mut r = three_steps();
        let pick = |x: &Record, s: &str| match x {
            Record::I2cCheck { step, .. } => check && step == s,
            Record::I2cWitness { step, .. } => !check && step == s,
            _ => false,
        };
        let (a, b) = (
            position(&r, |x| pick(x, "configure")),
            position(&r, |x| pick(x, "data")),
        );
        r.swap(a, b);
        assert_eq!(check_run(&r).unwrap_err(), order, "check {check}");
    }
    // A device whose first step is not its identity.
    let mut r = three_steps();
    let i = position(
        &r,
        |x| matches!(x, Record::I2cCheck { step, .. } if step == "identity"),
    );
    r.remove(i);
    let i = position(
        &r,
        |x| matches!(x, Record::I2cWitness { step, .. } if step == "identity"),
    );
    r.remove(i);
    assert_eq!(check_run(&r).unwrap_err(), order);
    // A step witnessed that was never read, and the reverse.
    let mut r = three_steps();
    let i = position(
        &r,
        |x| matches!(x, Record::I2cWitness { step, .. } if step == "data"),
    );
    r.remove(i);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a step's read without its witness, or a witness without its read"
    );
}

/// `AET-WO-4220`: a device of several configuration writes names each
/// `configure 1`, `configure 2`, ..., so each read and witness is its own
/// step: they replay in order, and a name repeated, out of order, or unknown
/// is refused as any step is.
#[test]
fn several_configurations_each_by_its_own_name_replay_in_order() {
    let several = || {
        let mut r = good();
        let c = position(&r, |x| matches!(x, Record::I2cCheck { .. }));
        let w = position(&r, |x| matches!(x, Record::I2cWitness { .. }));
        for (k, name) in ["configure 1", "configure 2", "configure 3"]
            .iter()
            .enumerate()
        {
            let v = u8::try_from(k).unwrap();
            r.insert(c + 1 + k, i2c_step_check(name, vec![0xF0 + v, v], 0, 0, 0));
        }
        r.insert(c + 4, i2c_step_check("data", vec![0xF7], 2, 0x1234, 0xFFFF));
        let w = w + 4;
        for (k, name) in ["configure 1", "configure 2", "configure 3"]
            .iter()
            .enumerate()
        {
            r.insert(w + 1 + k, i2c_step_witness(name, &["Write F0 00"]));
        }
        r.insert(w + 4, i2c_step_witness("data", &["Write F7", "Read 12 34"]));
        r
    };
    let r = several();
    assert!(check_run(&r).is_ok(), "{:?}", check_run(&r));
    let (dir, manifest) = write_run("several-configurations", 7, &as_events(r.clone()));
    let replayed = replay_dir(&dir).unwrap();
    assert_eq!(replayed.state.records, r);
    assert_eq!(replayed.manifest, manifest);
    let rename = |r: &mut Vec<Record>, from: &str, to: &str| {
        for x in r.iter_mut() {
            if let Record::I2cCheck { step, .. } | Record::I2cWitness { step, .. } = x
                && step == from
            {
                *step = to.to_owned();
            }
        }
    };
    // Two configurations under one name: each is "read twice".
    let mut r = several();
    rename(&mut r, "configure 2", "configure 1");
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a line measured twice, or a device step read or witnessed twice"
    );
    // Out of order: the third before the second.
    let mut r = several();
    rename(&mut r, "configure 2", "configure x");
    rename(&mut r, "configure 3", "configure 2");
    rename(&mut r, "configure x", "configure 3");
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a device's steps are not in the order identity, configure, data"
    );
    // A name that is no step. A bare `configure` is a step, a single
    // configuration's (index 1), so among numbered ones it is refused by
    // its order instead (`AET-WO-2302b`: asserted by name).
    for (bad, why) in [
        (
            "configure 5",
            "a step is not identity, a configuration or data",
        ),
        (
            "configure 0",
            "a step is not identity, a configuration or data",
        ),
        (
            "configure",
            "a device's steps are not in the order identity, configure, data",
        ),
        (
            "configure  1",
            "a step is not identity, a configuration or data",
        ),
    ] {
        let mut r = several();
        rename(&mut r, "configure 3", bad);
        assert_eq!(check_run(&r).unwrap_err(), why, "{bad}");
    }
}

fn spi_step_check(step: &str, write: Vec<u8>, width: u8, expected: u64, mask: u64) -> Record {
    Record::SpiCheck {
        reference: "U4".into(),
        peripheral: "SPI1".into(),
        select: "PA0".into(),
        address: 0,
        step: step.into(),
        write,
        width,
        expected,
        mask,
        observed: None,
        status: None,
    }
}

fn spi_step_witness(step: &str) -> Record {
    Record::SpiWitness {
        step: step.into(),
        peripheral: "SPI1".into(),
        reference: "U4".into(),
        device: "spi1.spi1mux.u4".into(),
        expected: vec![],
        observed: None,
        unrouted: vec![],
    }
}

/// Each kind-12, -13, -21 and -22 field rule, one refusal each: the writer
/// refuses the record and so does the reader, on the same bytes.
#[test]
fn the_step_fields_of_kinds_12_13_21_and_22_are_refused_by_rule() {
    let i2c = |step: &str, write: Vec<u8>, width: u8, expected: u64, mask: u64| {
        i2c_step_check(step, write, width, expected, mask)
    };
    let spi = spi_step_check;
    let mut bad_i2c_witness = i2c_step_witness("identity", &["Write D0"]);
    if let Record::I2cWitness { step, .. } = &mut bad_i2c_witness {
        *step = "boot".into();
    }
    let mut bad_spi_witness = spi_step_witness("identity");
    if let Record::SpiWitness { step, .. } = &mut bad_spi_witness {
        *step = "".into();
    }
    let cases = vec![
        // The step is one of identity, configure, data.
        ("12-step", i2c("probe", vec![0xD0], 1, 0x60, 0xFF)),
        ("21-step", spi("probe", vec![0x9F], 1, 0x60, 0xFF)),
        ("13-step", bad_i2c_witness),
        ("22-step", bad_spi_witness),
        // The write is 1 to 4 bytes.
        ("12-write-empty", i2c("identity", vec![], 1, 0x60, 0xFF)),
        (
            "12-write-5",
            i2c("identity", vec![1, 2, 3, 4, 5], 1, 0x60, 0xFF),
        ),
        ("21-write-empty", spi("identity", vec![], 1, 0x60, 0xFF)),
        (
            "21-write-5",
            spi("identity", vec![1, 2, 3, 4, 5], 1, 0x60, 0xFF),
        ),
        // The width is 0 to 8, and 0 exactly for configure.
        ("12-width-9", i2c("data", vec![1], 9, 0, 0)),
        ("12-width-0-identity", i2c("identity", vec![1], 0, 0, 0)),
        ("12-width-1-configure", i2c("configure", vec![1], 1, 0, 0)),
        ("21-width-9", spi("data", vec![1], 9, 0, 0)),
        ("21-width-0-data", spi("data", vec![1], 0, 0, 0)),
        ("21-width-1-configure", spi("configure", vec![1], 1, 0, 0)),
        // The mask and the expected value fit the width.
        ("12-mask-wide", i2c("identity", vec![1], 1, 0x60, 0x1FF)),
        ("12-expected-wide", i2c("identity", vec![1], 1, 0x160, 0xFF)),
        ("12-mask-configure", i2c("configure", vec![1], 0, 0, 1)),
        ("21-mask-wide", spi("identity", vec![1], 2, 0x60, 0x1_FFFF)),
        (
            "21-expected-wide",
            spi("identity", vec![1], 2, 0x1_0060, 0xFFFF),
        ),
        ("21-mask-configure", spi("configure", vec![1], 0, 0, 1)),
    ];
    for (name, record) in cases {
        assert!(
            matches!(
                append_one(name, record.clone()),
                Err(E::InvalidEvent { .. })
            ),
            "{name}: writer"
        );
        let r = raw_with_event(&format!("{name}-raw"), record_bytes(record));
        assert!(
            matches!(r, Err(E::MalformedEvent { index: 0, .. })),
            "{name}: reader gave {r:?}"
        );
    }
    // The widest read, eight bytes at the full mask, is accepted.
    for record in [
        i2c("data", vec![1, 2, 3, 4], 8, u64::MAX, u64::MAX),
        spi("data", vec![1], 8, u64::MAX, u64::MAX),
        i2c("configure", vec![1], 0, 0, 0),
    ] {
        assert!(append_one("step-ok", record).is_ok());
    }
}

const BROWN_OUT: &str =
    "the verdict's brown-out and flash controller findings do not match the boot's";

/// `good()` with kind 10's brown-out word observed as `word` (declared
/// level 3 expected), its flash accesses `flash`, its boot word set or not
/// (then named missing), and the verdict naming `findings` beside the run's
/// own.
fn with_brown_out(
    word: u32,
    flash: Option<Vec<String>>,
    booted: bool,
    findings: &[&'static str],
) -> Vec<Record> {
    let mut r = good();
    for x in &mut r {
        if let Record::BootObserved {
            boot_word,
            brown_out,
            flash_access,
            ..
        } = x
        {
            if !booted {
                boot_word.observed = Some(0);
            }
            brown_out.expected = 0xC;
            brown_out.observed = Some(word);
            *flash_access = flash.clone();
        }
    }
    let last = r.len() - 1;
    let mut f = vec![
        (PAST, "error", 2),
        ("device_unmodelled", "info", 1),
        ("i2c_timeout", "error", 1),
    ];
    f.extend(findings.iter().map(|k| (*k, "error", 1)));
    // A boot read without its boot word names it (`AET-WO-3201` B2).
    if !booted {
        f.push(("boot_word_missing", "error", 1));
    }
    f.sort();
    r[last] = verdict(false, &f);
    r
}

/// `AET-WO-4230` D10: replay names `brown_out_level_not_programmed` exactly
/// when kind 10's word is the judge's fill or its `BOR_LEV` is not the
/// declared level, and `flash_controller_access` exactly when the flash
/// controller's accesses were not the one read or could not be read; once
/// the boot word is set, never before.
#[test]
fn the_brown_out_findings_are_exactly_what_kind_10_shows() {
    const B: &str = "brown_out_level_not_programmed";
    const F: &str = "flash_controller_access";
    let ok = || Some(Vec::new());
    check_run(&with_brown_out(0x0406_AAFC, ok(), true, &[])).unwrap();
    check_run(&with_brown_out(0x0406_AAF0, ok(), true, &[B])).unwrap();
    check_run(&with_brown_out(0xFFFF_FFFF, ok(), true, &[B])).unwrap();
    let wrote = Some(vec!["the firmware wrote the flash controller".to_owned()]);
    check_run(&with_brown_out(0x0406_AAFC, wrote.clone(), true, &[F])).unwrap();
    check_run(&with_brown_out(0x0406_AAFC, None, true, &[F])).unwrap();
    check_run(&with_brown_out(0x0406_AAF0, None, true, &[B, F])).unwrap();
    // Before the boot word, neither is judged.
    check_run(&with_brown_out(0xFFFF_FFFF, None, false, &[])).unwrap();
    for (word, flash, booted, f) in [
        (0x0406_AAF0, ok(), true, &[][..]),
        (0xFFFF_FFFF, ok(), true, &[][..]),
        (0x0406_AAFC, ok(), true, &[B][..]),
        (0x0406_AAFC, wrote, true, &[][..]),
        (0x0406_AAFC, None, true, &[][..]),
        (0x0406_AAFC, ok(), true, &[F][..]),
        (0xFFFF_FFFF, None, false, &[B, F][..]),
    ] {
        assert_eq!(
            check_run(&with_brown_out(word, flash, booted, f)).unwrap_err(),
            BROWN_OUT,
            "{word:#x} {f:?}"
        );
    }
}

/// `AET-WO-4230` D10: kind 10's brown-out word is read with the boot, its
/// expectation inside its mask, and its flash accesses absent when nothing
/// was read and bounded like witness lines.
#[test]
fn kind_10s_brown_out_word_and_flash_accesses_are_checked() {
    let boot =
        |observed: Option<u32>, expected: u32, flash: Option<Vec<String>>| Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(observed.map(|_| 0x12)),
            instructions_expected: 10,
            instructions_observed: observed.map(|_| 10),
            pc: observed,
            executable: vec![(0, 0x1000_0000)],
            brown_out: Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected,
                observed,
            },
            flash_access: flash,
        };
    boot(Some(0xC), 0xC, Some(Vec::new())).check().unwrap();
    boot(None, 0xC, None).check().unwrap();
    assert_eq!(
        boot(Some(0xC), 0x10, None).check().unwrap_err(),
        "the brown-out word's expected level lies outside its mask"
    );
    assert_eq!(
        boot(None, 0xC, Some(Vec::new())).check().unwrap_err(),
        "a boot that read nothing holds flash controller accesses"
    );
    for flash in [vec![String::new()], vec!["x".to_owned(); 17]] {
        assert_eq!(
            boot(Some(0xC), 0xC, Some(flash)).check().unwrap_err(),
            "flash controller accesses are not at most 16 lines of 1 to 256 printable ASCII bytes"
        );
    }
    let mut unread = boot(Some(0xC), 0xC, None);
    if let Record::BootObserved { brown_out, .. } = &mut unread {
        brown_out.observed = None;
    }
    assert_eq!(
        unread.check().unwrap_err(),
        "the boot word, the instruction count, pc and the brown-out word are not all read or all unread"
    );
}

// ---- AET-WO-3201: the joins and the boot word replay binds ----

const UNBOUND: &str =
    "a solved line has no computed rise, stuck-high record or held-low fault, or more than one";
const BOOT_WORD: &str = "the verdict's boot_word_missing does not match the boot word";
const BOUND: &str =
    "a boot that read nothing does not name once why (a bound or a load error), or a read one does";

/// `records` with its verdict's findings replaced, `pass` as they say.
fn with_findings(mut records: Vec<Record>, findings: &[(&str, &str, u64)]) -> Vec<Record> {
    let pass = findings.iter().all(|(_, s, _)| *s == "info");
    *records.last_mut().unwrap() = verdict(pass, findings);
    records
}

/// `good()` with its boot record edited by `edit`.
fn good_with_boot(edit: impl Fn(&mut Reading, &mut Option<u64>, &mut Option<u32>)) -> Vec<Record> {
    let mut r = good();
    for x in &mut r {
        if let Record::BootObserved {
            boot_word,
            instructions_observed,
            pc,
            ..
        } = x
        {
            edit(boot_word, instructions_observed, pc);
        }
    }
    r
}

/// B1, the forgery `AET-WO-2205` found: every rise and the bus fault
/// dropped and the verdict rewritten to pass. It replayed as a pass; each
/// line of the solved bus now needs its record.
#[test]
fn a_run_with_its_rises_dropped_is_refused() {
    let mut r = good();
    r.retain(|x| !matches!(x, Record::RiseComputed { .. } | Record::BusFault { .. }));
    let r = with_findings(
        r,
        &[
            ("device_unmodelled", "info", 1),
            ("i2c_timeout", "error", 1),
        ],
    );
    assert_eq!(check_run(&r).unwrap_err(), UNBOUND);
    // One line's rise dropped, the other kept: refused too. The bus fault
    // then names only the kept line, or it is refused first for naming a
    // line with no measurement (`AET-WO-2302b`).
    let mut r = good();
    let i = position(
        &r,
        |x| matches!(x, Record::RiseComputed { net, .. } if net == "I2C_SDA"),
    );
    r.remove(i);
    for x in r.iter_mut() {
        if let Record::BusFault { nets, .. } = x {
            nets.retain(|n| n == "I2C_SCL");
        }
    }
    assert_eq!(check_run(&r).unwrap_err(), UNBOUND);
}

/// B1: a line of one bus is not bound by a rise of another bus's line,
/// nor a held line by a held-low fault of the other line.
#[test]
fn a_line_bound_only_by_another_bus_or_another_line_is_refused() {
    // A second bus whose lines' rises are the first bus's: the first bus's
    // own lines are then unbound.
    let mut r = good();
    r.insert(3, solved_bus("Z", "I2C3", ["Z_SCL", "Z_SDA"], &[]));
    let at = position(&r, |x| matches!(x, Record::BusFault { .. }));
    for net in ["Z_SDA", "Z_SCL"] {
        r.insert(at, healthy_rise("Z", net));
    }
    assert!(check_run(&r).is_ok());
    r.retain(|x| !matches!(x, Record::RiseComputed { bus, .. } if bus == "Z"));
    assert_eq!(check_run(&r).unwrap_err(), UNBOUND);
    // The held SDA line's fault named on SCL, with SCL's rise dropped: the
    // fault binds the override's line, SDA, only.
    let mut r = held_low_run();
    r.retain(|x| !matches!(x, Record::RiseComputed { .. }));
    let i = position(&r, |x| matches!(x, Record::BusFault { .. }));
    r[i] = fault(vec![HELD_LOW], vec!["I2C_SCL"]);
    assert_eq!(
        check_run(&r).unwrap_err(),
        "a held-low fault names no held-low override of its bus"
    );
    // The held line measured as well as faulted: bound twice.
    let mut r = held_low_run();
    r.insert(3, healthy_rise("SENSORS", "I2C_SDA"));
    assert_eq!(check_run(&r).unwrap_err(), UNBOUND);
    // The other line's rise dropped beside a held line.
    let mut r = held_low_run();
    r.retain(|x| !matches!(x, Record::RiseComputed { .. }));
    assert_eq!(check_run(&r).unwrap_err(), UNBOUND);
}

/// B2: a boot that read nothing replayed as a pass with no finding saying
/// why. The judge names the bound it was killed at; that is required, at
/// error, once, and a read boot names none.
#[test]
fn a_boot_that_read_nothing_names_its_bound() {
    // Unread whole, as the judge archives a boot killed before its reads:
    // kind 10's four, its flash accesses, and every register, record and
    // witness line beside it (`AET-WO-2309`: all read or all unread).
    let unread = || {
        let mut r = good();
        unread_boot(&mut r);
        r
    };
    // No record is read, so no timeout is named (`AET-WO-2312`): the
    // fixture's old verdict, a timeout beside its bound, is refused by it.
    let others = [(PAST, "error", 2), ("device_unmodelled", "info", 1)];
    let mut f = others.to_vec();
    f.extend([
        ("i2c_timeout", "error", 1),
        ("memory_cap_exceeded", "error", 1),
    ]);
    f.sort();
    assert_eq!(
        check_run(&with_findings(unread(), &f)).unwrap_err(),
        why("i2c_timeout")
    );
    // Before: the run's other findings alone replayed.
    assert_eq!(
        check_run(&with_findings(unread(), &others)).unwrap_err(),
        BOUND
    );
    // Its analog findings alone, no bound named: refused by name.
    assert_eq!(
        check_run(&with_findings(unread(), &[(PAST, "error", 2)])).unwrap_err(),
        BOUND
    );
    // Even its pass with its rises dropped (the forgery `AET-WO-2205` B2
    // found) is refused: since B1, first because each line needs its record.
    let mut r = unread();
    r.retain(|x| !matches!(x, Record::RiseComputed { .. } | Record::BusFault { .. }));
    assert_eq!(check_run(&with_findings(r, &[])).unwrap_err(), UNBOUND);
    // `AET-WO-1918` (2), ruled (b): a kill at the wall-clock hang guard is
    // refused by the CLI and never archived, so a run naming one as its bound
    // is a forgery, and replay refuses it.
    let mut wall = others.to_vec();
    wall.push(("wall_bound_exceeded", "error", 1));
    wall.sort();
    assert!(check_run(&with_findings(unread(), &wall)).is_err());
    for bound in [
        "cpu_aborted",
        "firmware_load_error",
        "memory_cap_exceeded",
        "platform_load_error",
    ] {
        let mut f = others.to_vec();
        f.push((bound, "error", 1));
        f.sort();
        check_run(&with_findings(unread(), &f)).unwrap();
        // At info, twice, or beside a read boot: refused.
        let mut info = others.to_vec();
        info.push((bound, "info", 1));
        info.sort();
        assert_eq!(
            check_run(&with_findings(unread(), &info)).unwrap_err(),
            BOUND
        );
        let mut twice = others.to_vec();
        twice.push((bound, "error", 2));
        twice.sort();
        assert_eq!(
            check_run(&with_findings(unread(), &twice)).unwrap_err(),
            BOUND
        );
        assert_eq!(check_run(&with_findings(good(), &f)).unwrap_err(), BOUND);
    }
    // An unread boot with boot_word_missing instead of a bound: refused.
    let mut f = others.to_vec();
    f.push(("boot_word_missing", "error", 1));
    f.sort();
    assert_eq!(
        check_run(&with_findings(unread(), &f)).unwrap_err(),
        BOOT_WORD
    );
}

/// B2: a boot that was read names `boot_word_missing` exactly when its
/// word is not the firmware's, once, as an error.
#[test]
fn boot_word_missing_is_named_exactly_when_the_word_is_wrong() {
    let wrong = || good_with_boot(|w, _, _| w.observed = Some(0));
    let others = [
        (PAST, "error", 2),
        ("device_unmodelled", "info", 1),
        ("i2c_timeout", "error", 1),
    ];
    // Before: a wrong word with no finding replayed (attribute: I1).
    assert_eq!(
        check_run(&with_findings(wrong(), &others)).unwrap_err(),
        BOOT_WORD
    );
    let mut f = others.to_vec();
    f.push(("boot_word_missing", "error", 1));
    f.sort();
    check_run(&with_findings(wrong(), &f)).unwrap();
    assert_eq!(
        check_run(&with_findings(good(), &f)).unwrap_err(),
        BOOT_WORD
    );
    // At info beside a boot that has its word: refused too.
    let mut f = others.to_vec();
    f.push(("boot_word_missing", "info", 1));
    f.sort();
    assert_eq!(
        check_run(&with_findings(good(), &f)).unwrap_err(),
        BOOT_WORD
    );
    for bad in [
        ("boot_word_missing", "info", 1),
        ("boot_word_missing", "error", 2),
    ] {
        let mut f = others.to_vec();
        f.push(bad);
        f.sort();
        assert_eq!(
            check_run(&with_findings(wrong(), &f)).unwrap_err(),
            BOOT_WORD
        );
    }
}

/// Adversarial F2: without the boot word the judge names a record only by
/// its timeout; a verdict that judges its value,
/// validity or witness is a rewrite, refused, so it can never move the run
/// from `I1` to a confident rule.
#[test]
fn a_record_judged_without_the_boot_word_is_refused() {
    let unbooted = || good_with_boot(|w, _, _| w.observed = Some(0));
    let base = [
        (PAST, "error", 2),
        ("boot_word_missing", "error", 1),
        ("device_unmodelled", "info", 1),
        ("i2c_timeout", "error", 1),
    ];
    check_run(&with_findings(unbooted(), &base)).unwrap();
    for kind in aethelgard_chaos::campaign::BOOTED_READ_FINDINGS {
        let mut f = base.to_vec();
        f.push((kind, "error", 1));
        f.sort();
        assert_eq!(
            check_run(&with_findings(unbooted(), &f)).unwrap_err(),
            "the verdict judges an identity read's record of a boot without its boot word",
            "{kind}"
        );
        // Booted, the records bind the count (`AET-WO-2312`): no record of
        // good()'s shows a mismatch, so naming one is refused by its kind;
        // an invalid read and an unwitnessed one are reached, not equalled
        // (an unmodelled device's read and a cut log are not archived).
        let mut f: Vec<_> = f
            .into_iter()
            .filter(|x| x.0 != "boot_word_missing")
            .collect();
        f.sort();
        let booted = check_run(&with_findings(good(), &f));
        if kind.ends_with("_mismatch") {
            assert_eq!(
                booted.unwrap_err(),
                aethelgard_chaos::campaign::SHOWN
                    .iter()
                    .find(|(k, _)| *k == kind)
                    .unwrap()
                    .1,
                "{kind}"
            );
        } else {
            assert!(booted.is_ok(), "{kind}");
        }
    }
}

/// One edit of a run, named for the assertion's message.
type Case<'a> = (&'a str, &'a dyn Fn(&mut Record));

const NOT_TOGETHER: &str = "the boot's registers, identity reads and witnesses are not all read or all unread with its boot word";

/// `good()` with U4 on SPI1 (kinds 23, 21 and 22) read as the judge reads it.
fn good_with_spi() -> Vec<Record> {
    let mut r = good();
    let bus = position(&r, is_bus);
    r.insert(
        bus + 1,
        Record::SpiBusSolved {
            peripheral: "SPI1".into(),
            devices: vec![("U4".into(), 0, "PA0".into())],
        },
    );
    let mut check = spi_step_check("identity", vec![0x9F], 3, 0xEF_4017, 0xFF_FFFF);
    if let Record::SpiCheck {
        observed, status, ..
    } = &mut check
    {
        *observed = Some(0xEF_4017);
        *status = Some("ok".into());
    }
    let mut witness = spi_step_witness("identity");
    if let Record::SpiWitness {
        expected, observed, ..
    } = &mut witness
    {
        let line =
            "[NOISY] spi1.spi1mux.u4: Transmitting data 0x9F, current state: RecognizeOperation";
        *expected = vec![line.to_owned()];
        *observed = Some(vec![line.to_owned()]);
    }
    let verdict = r.len() - 1;
    r.insert(verdict, witness);
    r.insert(verdict, check);
    r
}

/// `AET-WO-2309` rule (3), `AET-WO-3201` adversarial 5.8: a read boot whose
/// registers or identity reads are set unread, value or status, is refused
/// by name; so is any one of them read beside an unread boot. A read boot's
/// witness may be unread (its log past the filter's budget); an unread boot's
/// holds no lines and no attribution evidence.
#[test]
fn a_boots_reads_are_all_read_or_all_unread() {
    let base = good_with_spi();
    assert_eq!(check_run(&base).map(|_| ()), Ok(()));
    let edited = |r: &[Record], f: &dyn Fn(&mut Record)| {
        let mut r = r.to_vec();
        r.iter_mut().for_each(f);
        check_run(&r).map(|_| ())
    };
    // The flooded log's shape: read, its witnesses unread. Accepted, with
    // the finding the judge names for U4's ok read whose witness is unread
    // (`AET-WO-2310`: without it, refused; U2's read timed out, so it needs
    // none).
    let flooded = |x: &mut Record| match x {
        Record::I2cWitness { observed, .. } | Record::SpiWitness { observed, .. } => {
            *observed = None
        }
        _ => {}
    };
    assert_eq!(edited(&base, &flooded), Err(UNWITNESSED_SPI));
    let mut named = base.clone();
    let Some(Record::Verdict { findings, .. }) = named.last_mut() else {
        panic!("the verdict is last")
    };
    findings.push(("spi_transaction_unwitnessed".into(), "error".into(), 1));
    assert_eq!(edited(&named, &flooded), Ok(()));
    // 5.8, one record at a time, beside a read boot.
    let unread_in_read: [Case; 5] = [
        ("kind 11", &|x| {
            if let Record::RegisterObserved { reading, .. } = x {
                reading.observed = None;
            }
        }),
        ("kind 12 value", &|x| {
            if let Record::I2cCheck { observed, .. } = x {
                *observed = None;
            }
        }),
        ("kind 12 status", &|x| {
            if let Record::I2cCheck { status, .. } = x {
                *status = None;
            }
        }),
        ("kind 21 value", &|x| {
            if let Record::SpiCheck { observed, .. } = x {
                *observed = None;
            }
        }),
        ("kind 21 status", &|x| {
            if let Record::SpiCheck { status, .. } = x {
                *status = None;
            }
        }),
    ];
    for (what, f) in unread_in_read {
        assert_eq!(edited(&base, f), Err(NOT_TOGETHER), "{what}");
    }
    // The same run read as a boot killed before its reads: accepted whole,
    // then each read thing put back alone is refused.
    let mut killed = base.clone();
    unread_boot(&mut killed);
    let killed = with_findings(
        killed,
        &[
            (PAST, "error", 2),
            ("cpu_aborted", "error", 1),
            ("device_unmodelled", "info", 1),
        ],
    );
    assert_eq!(check_run(&killed).map(|_| ()), Ok(()));
    let read_in_unread: [Case; 9] = [
        ("kind 11", &|x| {
            if let Record::RegisterObserved { reading, .. } = x {
                reading.observed = Some(0x12);
            }
        }),
        ("kind 12 value", &|x| {
            if let Record::I2cCheck { observed, .. } = x {
                *observed = Some(0);
            }
        }),
        ("kind 12 status", &|x| {
            if let Record::I2cCheck { status, .. } = x {
                *status = Some("timeout_txis".into());
            }
        }),
        ("kind 21 value", &|x| {
            if let Record::SpiCheck { observed, .. } = x {
                *observed = Some(0);
            }
        }),
        ("kind 21 status", &|x| {
            if let Record::SpiCheck { status, .. } = x {
                *status = Some("ok".into());
            }
        }),
        ("kind 13 lines", &|x| {
            if let Record::I2cWitness { observed, .. } = x {
                *observed = Some(vec![]);
            }
        }),
        ("kind 13 absent", &|x| {
            if let Record::I2cWitness { absent, .. } = x {
                absent.push(0x75);
            }
        }),
        ("kind 22 lines", &|x| {
            if let Record::SpiWitness { observed, .. } = x {
                *observed = Some(vec![]);
            }
        }),
        ("kind 22 unrouted", &|x| {
            if let Record::SpiWitness { unrouted, .. } = x {
                unrouted.push("[WARNING] spi1.spi1mux: a line".into());
            }
        }),
    ];
    for (what, f) in read_in_unread {
        assert_eq!(edited(&killed, f), Err(NOT_TOGETHER), "{what}");
    }
}

// ---- AET-WO-2310: cpu_halted and the unwitnessed ok read ----

const HALTED: &str = "the verdict's cpu_halted does not match the booted core's instruction count";
// Each bus's own reason since `AET-WO-2312`.
const UNWITNESSED: &str = "the verdict's i2c_transaction_unwitnessed does not reach the I2C reads' unread or differing witnesses";
const UNWITNESSED_SPI: &str = "the verdict's spi_transaction_unwitnessed does not reach the SPI reads' unread or differing witnesses";

/// `good()`'s own findings.
const GOOD_FINDINGS: [(&str, &str, u64); 3] = [
    (PAST, "error", 2),
    ("device_unmodelled", "info", 1),
    ("i2c_timeout", "error", 1),
];

fn plus(extra: &[(&'static str, &'static str, u64)]) -> Vec<(&'static str, &'static str, u64)> {
    let mut f = GOOD_FINDINGS.to_vec();
    f.extend_from_slice(extra);
    f.sort();
    f
}

/// (f) A booted core that counted fewer instructions than the run holds is
/// named `cpu_halted` once, as an error; one that counted them all is not.
#[test]
fn cpu_halted_is_named_once_exactly_when_a_booted_core_counted_fewer_instructions() {
    let halted = || good_with_boot(|_, n, _| *n = Some(9));
    assert_eq!(
        check_run(&with_findings(halted(), &GOOD_FINDINGS)).unwrap_err(),
        HALTED,
        "missing"
    );
    check_run(&with_findings(
        halted(),
        &plus(&[("cpu_halted", "error", 1)]),
    ))
    .unwrap();
    for bad in [("cpu_halted", "info", 1), ("cpu_halted", "error", 2)] {
        assert_eq!(
            check_run(&with_findings(halted(), &plus(&[bad]))).unwrap_err(),
            HALTED,
            "{bad:?}"
        );
    }
    // Observed equal to expected: none, so one is refused.
    check_run(&good()).unwrap();
    assert_eq!(
        check_run(&with_findings(good(), &plus(&[("cpu_halted", "error", 1)]))).unwrap_err(),
        HALTED,
        "present at 10 of 10"
    );
}

fn blank_witness(r: &mut [Record], step_of: &str) {
    for x in r {
        match x {
            Record::I2cWitness { step, observed, .. }
            | Record::SpiWitness { step, observed, .. }
                if step == step_of =>
            {
                *observed = None
            }
            _ => {}
        }
    }
}

/// (g) An ok I2C read whose own witness is unread needs
/// `i2c_transaction_unwitnessed` at least once per such read.
#[test]
fn an_ok_i2c_read_with_its_witness_unread_needs_its_unwitnessed_finding() {
    check_run(&three_steps()).unwrap();
    let mut r = three_steps();
    blank_witness(&mut r, "data");
    assert_eq!(
        check_run(&with_findings(r.clone(), &GOOD_FINDINGS)).unwrap_err(),
        UNWITNESSED
    );
    let one = plus(&[("i2c_transaction_unwitnessed", "error", 1)]);
    check_run(&with_findings(r.clone(), &one)).unwrap();
    // At info it is not named.
    let info = plus(&[("i2c_transaction_unwitnessed", "info", 1)]);
    assert_eq!(
        check_run(&with_findings(r.clone(), &info)).unwrap_err(),
        UNWITNESSED
    );
    // Two such reads need two.
    blank_witness(&mut r, "configure");
    assert_eq!(
        check_run(&with_findings(r.clone(), &one)).unwrap_err(),
        UNWITNESSED
    );
    let two = plus(&[("i2c_transaction_unwitnessed", "error", 2)]);
    check_run(&with_findings(r, &two)).unwrap();
    // The timed-out identity read's witness unread needs none.
    let mut r = three_steps();
    blank_witness(&mut r, "identity");
    check_run(&r).unwrap();
}

/// (g) The same for an ok SPI read and `spi_transaction_unwitnessed`.
#[test]
fn an_ok_spi_read_with_its_witness_unread_needs_its_unwitnessed_finding() {
    let mut spi_only = good_with_spi();
    for x in &mut spi_only {
        if let Record::SpiWitness { observed, .. } = x {
            *observed = None;
        }
    }
    assert_eq!(
        check_run(&with_findings(spi_only.clone(), &GOOD_FINDINGS)).unwrap_err(),
        UNWITNESSED_SPI
    );
    let named = plus(&[("spi_transaction_unwitnessed", "error", 1)]);
    check_run(&with_findings(spi_only.clone(), &named)).unwrap();
    // The I2C finding does not stand for it.
    let other = plus(&[("i2c_transaction_unwitnessed", "error", 1)]);
    assert_eq!(
        check_run(&with_findings(spi_only, &other)).unwrap_err(),
        UNWITNESSED_SPI
    );
}

// ---- AET-WO-2312: every kind the records show is bound by them ----

/// The reason replay refuses a verdict whose count of `kind` its records do
/// not bound.
fn why(kind: &str) -> &'static str {
    aethelgard_chaos::campaign::SHOWN
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, w)| *w)
        .unwrap()
}

/// `good_with_spi()` (booted: U2's I2C read timed out, U4's SPI read ok and
/// witnessed) edited by `edit`, its verdict the analog findings and
/// `findings`.
fn judged(
    edit: &dyn Fn(&mut Vec<Record>),
    findings: &[(&str, &str, u64)],
) -> Result<(), &'static str> {
    let mut r = good_with_spi();
    edit(&mut r);
    let mut f = vec![(PAST, "error", 2), ("device_unmodelled", "info", 1)];
    f.extend_from_slice(findings);
    f.sort();
    check_run(&with_findings(r, &f)).map(|_| ())
}

/// Each of U2's kind 12 and kind 13 by `f`.
fn on_u2(r: &mut [Record], f: &dyn Fn(&mut Record)) {
    r.iter_mut()
        .filter(|x| matches!(x, Record::I2cCheck { .. } | Record::I2cWitness { .. }))
        .for_each(f);
}

fn u2_read(status: &'static str, value: u64) -> impl Fn(&mut Vec<Record>) {
    move |r| {
        on_u2(r, &|x| {
            if let Record::I2cCheck {
                status: s,
                observed,
                ..
            } = x
            {
                (*s, *observed) = (Some(status.into()), Some(value));
            }
        })
    }
}

fn u4(f: impl Fn(&mut Record)) -> impl Fn(&mut Vec<Record>) {
    move |r| {
        r.iter_mut()
            .filter(|x| matches!(x, Record::SpiCheck { .. } | Record::SpiWitness { .. }))
            .for_each(&f)
    }
}

fn unbooted(r: &mut [Record]) {
    if let Record::BootObserved { boot_word, .. } =
        at(r, |x| matches!(x, Record::BootObserved { .. }))
    {
        boot_word.observed = Some(0);
    }
}

const TIMEOUT: (&str, &str, u64) = ("i2c_timeout", "error", 1);

/// Review round 3 A8: an archive whose records show a failure never
/// replays as a pass. Each exact kind, omitted, inflated or named at `info`,
/// is refused by its own reason; a reached kind may pass its records, never
/// fall short.
#[test]
fn each_kind_the_records_show_binds_the_verdicts_count() {
    let none = |_: &mut Vec<Record>| {};
    // The timeout as recorded: named once, as an error.
    assert_eq!(judged(&none, &[TIMEOUT]), Ok(()));
    for f in [
        vec![],
        vec![("i2c_timeout", "error", 2)],
        vec![("i2c_timeout", "info", 1)],
    ] {
        assert_eq!(judged(&none, &f), Err(why("i2c_timeout")), "{f:?}");
    }
    // Every status spelling of a timeout, on each bus (`renode::status_name`).
    for s in [
        "timeout_txis",
        "timeout_tc",
        "timeout_rxne",
        "timeout_stopf",
    ] {
        assert_eq!(judged(&u2_read(s, 0), &[TIMEOUT]), Ok(()), "{s}");
        assert_eq!(judged(&u2_read(s, 0), &[]), Err(why("i2c_timeout")), "{s}");
    }
    for s in ["timeout_rxp", "timeout_eot"] {
        let edit = u4(move |x| {
            if let Record::SpiCheck { status, .. } = x {
                *status = Some(s.into());
            }
        });
        let spi = ("spi_timeout", "error", 1);
        assert_eq!(judged(&edit, &[TIMEOUT, spi]), Ok(()), "{s}");
        assert_eq!(judged(&edit, &[TIMEOUT]), Err(why("spi_timeout")), "{s}");
    }
    // A register wrong under its mask: exactly once; under a zero mask it
    // shows nothing.
    let reg = |mask: u32| {
        move |r: &mut Vec<Record>| {
            if let Record::RegisterObserved { reading, .. } =
                at(r, |x| matches!(x, Record::RegisterObserved { .. }))
            {
                (reading.mask, reading.observed) = (mask, Some(0x13));
            }
        }
    };
    let one = ("register_mismatch", "error", 1);
    assert_eq!(judged(&reg(0xff), &[TIMEOUT, one]), Ok(()));
    assert_eq!(
        judged(&reg(0xff), &[TIMEOUT]),
        Err(why("register_mismatch"))
    );
    assert_eq!(
        judged(&reg(0xff), &[TIMEOUT, ("register_mismatch", "error", 2)]),
        Err(why("register_mismatch"))
    );
    assert_eq!(judged(&reg(0), &[TIMEOUT]), Ok(()));
    assert_eq!(
        judged(&reg(0), &[TIMEOUT, one]),
        Err(why("register_mismatch"))
    );
    // A completed read's value, named by its step, exactly; a configuration
    // reads nothing, so any bit it returns is a mismatch; under a zero mask
    // the value shows nothing.
    let step = |name: &'static str, width: u8, mask: u64, value: u64| {
        move |r: &mut Vec<Record>| {
            // U2's identity read passes and is witnessed; the step under test
            // follows it, as a device's script orders them.
            u2_read("ok", 0x71)(r);
            on_u2(r, &|x| {
                if let Record::I2cWitness {
                    expected, observed, ..
                } = x
                {
                    *observed = Some(expected.clone());
                }
            });
            if name == "identity" {
                on_u2(r, &|x| {
                    if let Record::I2cCheck {
                        mask: m, observed, ..
                    } = x
                    {
                        (*m, *observed) = (mask, Some(value));
                    }
                });
                return;
            }
            let c = position(r, |x| matches!(x, Record::I2cCheck { .. }));
            let mut check = r[c].clone();
            if let Record::I2cCheck {
                step: s,
                width: w,
                mask: m,
                expected,
                observed,
                ..
            } = &mut check
            {
                (*s, *w, *m, *observed) = (name.into(), width, mask, Some(value));
                if width == 0 {
                    *expected = 0;
                }
            }
            r.insert(c + 1, check);
            let w = position(r, |x| matches!(x, Record::I2cWitness { .. }));
            let mut witness = r[w].clone();
            if let Record::I2cWitness { step: s, .. } = &mut witness {
                *s = name.into();
            }
            r.insert(w + 1, witness);
        }
    };
    for (name, width, kind) in [
        ("identity", 1, "i2c_identity_mismatch"),
        ("configure", 0, "i2c_configure_mismatch"),
        ("data", 1, "i2c_data_mismatch"),
    ] {
        let wrong = step(name, width, 0xff, 0x70);
        assert_eq!(judged(&wrong, &[(kind, "error", 1)]), Ok(()), "{kind}");
        assert_eq!(judged(&wrong, &[]), Err(why(kind)), "{kind}");
        assert_eq!(
            judged(&wrong, &[(kind, "error", 2)]),
            Err(why(kind)),
            "{kind}"
        );
    }
    assert_eq!(judged(&step("identity", 1, 0, 0x70), &[]), Ok(()));
    let spi_wrong = u4(|x| {
        if let Record::SpiCheck { observed, .. } = x {
            *observed = Some(0xEF_4018);
        }
    });
    let spi = ("spi_identity_mismatch", "error", 1);
    assert_eq!(judged(&spi_wrong, &[TIMEOUT, spi]), Ok(()));
    assert_eq!(
        judged(&spi_wrong, &[TIMEOUT]),
        Err(why("spi_identity_mismatch"))
    );
    // Before the boot word, a record's value, validity and witness show
    // nothing; its timeout still does (`renode::timeouts_without_the_boot_word`).
    let word = ("boot_word_missing", "error", 1);
    let unbooted_wrong = |r: &mut Vec<Record>| {
        step("identity", 1, 0xff, 0x70)(r);
        unbooted(r);
    };
    assert_eq!(judged(&unbooted_wrong, &[word]), Ok(()));
    let unbooted_invalid = |r: &mut Vec<Record>| {
        u2_read("invalid", 0)(r);
        unbooted(r);
    };
    assert_eq!(judged(&unbooted_invalid, &[word]), Ok(()));
    assert_eq!(
        judged(&|r: &mut Vec<Record>| unbooted(r), &[word]),
        Err(why("i2c_timeout"))
    );
    assert_eq!(
        judged(&|r: &mut Vec<Record>| unbooted(r), &[word, TIMEOUT]),
        Ok(())
    );
    // An invalid record is reached, not equalled: an unmodelled device's
    // invalid read is named by the same kind and not archived.
    let invalid = u2_read("invalid", 0);
    assert_eq!(judged(&invalid, &[]), Err(why("i2c_record_invalid")));
    for n in [1, 2] {
        assert_eq!(
            judged(&invalid, &[("i2c_record_invalid", "error", n)]),
            Ok(())
        );
    }
    // An ok read whose witness is unread is reached; a step with no witness
    // format (`expected` empty) is the judge's `spi_witness_unavailable`.
    let flooded = |expected: bool| {
        u4(move |x| {
            if let Record::SpiWitness {
                observed,
                expected: e,
                ..
            } = x
            {
                *observed = None;
                if !expected {
                    e.clear();
                }
            }
        })
    };
    let un = |n| ("spi_transaction_unwitnessed", "error", n);
    assert_eq!(
        judged(&flooded(true), &[TIMEOUT]),
        Err(why("spi_transaction_unwitnessed"))
    );
    assert_eq!(judged(&flooded(true), &[TIMEOUT, un(1)]), Ok(()));
    assert_eq!(judged(&flooded(true), &[TIMEOUT, un(2)]), Ok(()));
    // With no witness format the judge names `spi_witness_unavailable`,
    // exactly, the log unread or not (adversarial F1).
    let ua = ("spi_witness_unavailable", "error", 1);
    assert_eq!(
        judged(&flooded(false), &[TIMEOUT]),
        Err(why("spi_witness_unavailable"))
    );
    assert_eq!(judged(&flooded(false), &[TIMEOUT, ua]), Ok(()));
    // A core that read fewer instructions than the run holds is named at any
    // boot the judge read, the boot word set or not.
    let halted = |r: &mut Vec<Record>| {
        unbooted(r);
        if let Record::BootObserved {
            instructions_observed,
            ..
        } = at(r, |x| matches!(x, Record::BootObserved { .. }))
        {
            *instructions_observed = Some(7);
        }
    };
    let cpu = ("cpu_halted", "error", 1);
    assert_eq!(judged(&halted, &[word, TIMEOUT, cpu]), Ok(()));
    assert_eq!(judged(&halted, &[word, TIMEOUT]), Err(why("cpu_halted")));
    // The locked access: at most once, from Renode's log.
    let locked = |n| ("locked_peripheral_access", "error", n);
    assert_eq!(judged(&none, &[TIMEOUT, locked(1)]), Ok(()));
    assert_eq!(
        judged(&none, &[TIMEOUT, locked(2)]),
        Err("the verdict names locked_peripheral_access more than once")
    );
}

/// `shown` derives the judge's kinds, each once, in `SHOWN`'s order, with
/// the locked access last; an `RCC.` register read with an enable bit off
/// is its evidence, by name (B6: chaos holds no register map).
#[test]
fn shown_names_every_kind_once_and_finds_the_locked_access_by_name() {
    use aethelgard_chaos::campaign::{Bound, SHOWN, shown};
    let mut r = good_with_spi();
    let s = shown(&r);
    let kinds: Vec<&str> = s.iter().map(|x| x.kind).collect();
    let mut want: Vec<&str> = SHOWN.iter().map(|(k, _)| *k).collect();
    want.push("locked_peripheral_access");
    assert_eq!(kinds, want);
    assert_eq!(s.last().unwrap().bound, Bound::AtMostOnce);
    let i = position(&r, |x| matches!(x, Record::RegisterObserved { .. }));
    for (name, observed, shows) in [
        ("RCC.AHB4ENR", 0x10, true),
        ("RCC.AHB4ENR", 0x13, false),
        ("GPIOA.MODER", 0x10, false),
    ] {
        r[i] = Record::RegisterObserved {
            name: name.into(),
            reading: reading(Some(observed)),
        };
        let locked = shown(&r).pop().unwrap();
        assert_eq!(locked.records == [i], shows, "{name} {observed:#x}");
    }
}

/// U4's SPI witness edited: its expected lines and what was observed.
fn u4_witness(expected: Vec<String>, observed: Option<Vec<String>>) -> impl Fn(&mut Vec<Record>) {
    u4(move |x| {
        if let Record::SpiWitness {
            expected: e,
            observed: o,
            ..
        } = x
        {
            (*e, *o) = (expected.clone(), observed.clone());
        }
    })
}

fn lines(n: usize, text: &str) -> Vec<String> {
    (0..n)
        .map(|i| format!("[NOISY] spi1.spi1mux.u4: {text} {i:03}"))
        .collect()
}

/// `AET-WO-2312` adversarial pass F1-F4: what a witness shows of the judge's
/// three witness findings, bound without refusing what the judge writes.
/// An `[ERROR]` line archived is `step_error`. An `ok` read with no witness
/// format is `witness_unavailable`, and one whose short witness differs is
/// `unwitnessed`, each or, where the archive may have cut an `[ERROR]`
/// line, `step_error`. A status the judge never writes is refused.
#[test]
fn the_witness_findings_and_the_statuses_are_bound_as_the_judge_writes_them() {
    let joint = "the verdict's spi witness findings do not reach the SPI witnesses that show one";
    let spi = |kind: &'static str| (kind, "error", 1);
    // F1: an ok read with no witness format, its log read: refused unless
    // named, as `witness_unavailable` or a cut line's `step_error`.
    let no_format = u4_witness(vec![], Some(lines(1, "Transmitting data 0x9F")));
    assert_eq!(judged(&no_format, &[TIMEOUT]), Err(joint));
    assert_eq!(
        judged(&no_format, &[TIMEOUT, spi("spi_witness_unavailable")]),
        Ok(())
    );
    assert_eq!(
        judged(&no_format, &[TIMEOUT, spi("spi_step_error")]),
        Ok(())
    );
    // F2: a short witness that differs, whole: named, or refused.
    let differs = u4_witness(lines(1, "expected"), Some(lines(1, "other")));
    assert_eq!(judged(&differs, &[TIMEOUT]), Err(joint));
    assert_eq!(
        judged(&differs, &[TIMEOUT, spi("spi_transaction_unwitnessed")]),
        Ok(())
    );
    // F3: an `[ERROR]` line, at any status: `step_error`, by its kind.
    let error = u4_witness(
        lines(1, "expected"),
        Some(vec!["[ERROR] spi1.spi1mux.u4: bus error".into()]),
    );
    assert_eq!(judged(&error, &[TIMEOUT]), Err(why("spi_step_error")));
    assert_eq!(judged(&error, &[TIMEOUT, spi("spi_step_error")]), Ok(()));
    let timed_out_error = |r: &mut Vec<Record>| {
        u2_read("timeout_txis", 0)(r);
        on_u2(r, &|x| {
            if let Record::I2cWitness { observed, .. } = x {
                *observed = Some(vec!["[ERROR] i2c1.u2: bus error".into()]);
            }
        });
    };
    assert_eq!(
        judged(&timed_out_error, &[TIMEOUT]),
        Err(why("i2c_step_error"))
    );
    assert_eq!(
        judged(&timed_out_error, &[TIMEOUT, ("i2c_step_error", "error", 1)]),
        Ok(())
    );
    // Must accept: an `[ERROR]` past the archive's 16 lines, which the judge
    // names `step_error` alone, beside a witness format or none.
    for expected in [lines(1, "expected"), vec![]] {
        let cut = u4_witness(expected, Some(lines(16, "flood")));
        assert_eq!(judged(&cut, &[TIMEOUT, spi("spi_step_error")]), Ok(()));
    }
    // Must accept: a passing log so long that `fit` cut the archived copy
    // of the model's own lines (16 of about 230 bytes: both copies pass
    // 4,096, so `fit` kept one observed line, and the record fits).
    let long = lines(16, &"x".repeat(200));
    let popped = u4_witness(long.clone(), Some(long[..1].to_vec()));
    assert_eq!(judged(&popped, &[TIMEOUT]), Ok(()));
    // Before the boot word, no witness finding is named, and a witness that
    // differs shows none.
    let word = ("boot_word_missing", "error", 1);
    let unbooted_differs = |r: &mut Vec<Record>| {
        differs(r);
        unbooted(r);
    };
    assert_eq!(judged(&unbooted_differs, &[word, TIMEOUT]), Ok(()));
    for kind in ["spi_step_error", "spi_witness_unavailable"] {
        assert_eq!(
            judged(
                &|r: &mut Vec<Record>| unbooted(r),
                &[word, TIMEOUT, spi(kind)]
            ),
            Err("the verdict judges an identity read's record of a boot without its boot word"),
            "{kind}"
        );
    }
    // F4: a status the judge never writes, or another bus's, is refused.
    let status = "a read's status is not one the judge writes for its bus";
    for s in ["bogus", "OK", "timeout", "timeout_rxp", "timeout_eot"] {
        assert_eq!(judged(&u2_read(s, 0), &[TIMEOUT]), Err(status), "{s}");
    }
    let spi_txis = u4(|x| {
        if let Record::SpiCheck { status, .. } = x {
            *status = Some("timeout_txis".into());
        }
    });
    assert_eq!(
        judged(&spi_txis, &[TIMEOUT, spi("spi_timeout")]),
        Err(status)
    );
}
