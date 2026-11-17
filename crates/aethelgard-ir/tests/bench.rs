//! Bench access (`AET-WO-1312`): test points, `BOOT0` on a connector pin,
//! and the reset switch. Every refusal of step 15a by name, the fields'
//! absence as today's board, and `BOOT0` among the emitter's own nets.

use aethelgard_ir::{IrValidationError as E, Specification, bench, names};
use serde_json::{Value, json};

const BENCH: &str = include_str!("../../aethelgard-cli/tests/fixtures/sentinel-01-bench.json");
const V0: &str = include_str!("../../../examples/sentinel-01-minimal.json");

fn bench_json() -> Value {
    serde_json::from_str(BENCH).expect("the fixture is JSON")
}

fn spec_of(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("the specification reads")
}

fn refusal(v: &Value) -> E {
    spec_of(v)
        .validate()
        .expect_err("the specification is refused")
}

fn assert_refusal(err: &E, kind: &str, id: &str) {
    assert_eq!(err.kind(), kind, "{err}");
    assert_eq!(err.id(), Some(id), "{err}");
}

/// Sets test point `i`'s net and reference.
fn set(v: &mut Value, i: usize, net: &str, reference: &str) {
    v["test_points"][i] = json!({"net": net, "reference": reference});
}

#[test]
fn the_bench_fixture_validates() {
    let spec = spec_of(&bench_json());
    assert_eq!(spec.validate(), Ok(()));
    assert_eq!(spec.declared_test_points().len(), 12);
    assert!(spec.mcu.reset_switch);
    assert!(bench::carries_boot0(&spec));
    assert_eq!(bench::switch_reference(&spec).as_deref(), Some("SW1"));
}

/// Absent is today's board, and an empty list or `false` is not written back,
/// so a specification without them serializes as before (`AET-WO-1312` §1.1).
#[test]
fn absent_fields_are_todays_board_and_are_not_written_back() {
    let v0: Value = serde_json::from_str(V0).unwrap();
    assert!(v0.get("test_points").is_none());
    assert!(v0["mcu"].get("reset_switch").is_none());
    let spec = spec_of(&v0);
    assert!(spec.declared_test_points().is_empty());
    assert!(!spec.mcu.reset_switch);
    assert!(!bench::carries_boot0(&spec));
    assert_eq!(bench::switch_reference(&spec), None);
    assert_eq!(serde_json::to_value(&spec).unwrap(), v0);
    // Written empty or false, they read the same and are dropped on writing.
    let mut v = v0.clone();
    v["test_points"] = json!([]);
    v["mcu"]["reset_switch"] = json!(false);
    assert_eq!(serde_json::to_value(spec_of(&v)).unwrap(), v0);
}

#[test]
fn a_test_point_is_an_object_with_exactly_its_two_keys() {
    let mut v = bench_json();
    v["test_points"][0]["extra"] = json!(1);
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = bench_json();
    v["test_points"][0] = json!(["I2C_SDA", "TP1"]);
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = bench_json();
    v["test_points"][0]
        .as_object_mut()
        .unwrap()
        .remove("reference");
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = bench_json();
    v["mcu"]["reset_switch"] = json!("yes");
    assert!(serde_json::from_value::<Specification>(v).is_err());
}

#[test]
fn a_test_point_names_its_net_and_reference_as_identifiers() {
    let mut v = bench_json();
    set(&mut v, 0, "I2C SDA", "TP1");
    let err = refusal(&v);
    assert_eq!(err.kind(), "InvalidIdentifier", "{err}");
    assert_eq!(err.path(), Some("test_points[0].net"), "{err}");
    // A reference is held as a connector's is (`Reference`), so KiCad's
    // unannotated `TP?` and the like are refused (adversarial info item).
    for bad in ["TP/4", "TP?", "~", "TP.1", "TP-1", "1"] {
        let mut v = bench_json();
        set(&mut v, 3, "SPI_MOSI", bad);
        let err = refusal(&v);
        assert_eq!(err.path(), Some("test_points[3].reference"), "{bad}: {err}");
    }
}

#[test]
fn a_net_no_part_declares_is_refused_by_name() {
    for net in ["NOPE", "i2c_sda", "VDD_1V8", "gnd", "Boot0", "TP1"] {
        let mut v = bench_json();
        set(&mut v, 0, net, "TP1");
        assert_refusal(&refusal(&v), "TestPointNetUnknown", "TP1");
    }
}

#[test]
fn every_net_a_connector_pin_may_name_takes_a_test_point() {
    // The fixture's 12 are every one but SWDIO and SWCLK; those two as well.
    let mut v = bench_json();
    let tps = v["test_points"].as_array_mut().unwrap();
    tps.push(json!({"net": "SWDIO", "reference": "TP13"}));
    tps.push(json!({"net": "SWCLK", "reference": "TP14"}));
    assert_eq!(spec_of(&v).validate(), Ok(()));
}

#[test]
fn a_duplicate_reference_is_refused_by_name() {
    let mut v = bench_json();
    set(&mut v, 1, "I2C_SCL", "TP1");
    assert_refusal(&refusal(&v), "TestPointReferenceTaken", "TP1");
    // KiCad's key: case and leading zeros fold.
    let mut v = bench_json();
    set(&mut v, 1, "I2C_SCL", "tp01");
    assert_refusal(&refusal(&v), "TestPointReferenceTaken", "tp01");
}

#[test]
fn a_reference_another_part_holds_is_refused_by_name() {
    // A connector's, a device's, under KiCad's key.
    for taken in ["J2", "j02", "U2", "U4"] {
        let mut v = bench_json();
        set(&mut v, 0, "I2C_SDA", taken);
        let err = refusal(&v);
        assert_refusal(&err, "TestPointReferenceTaken", taken);
    }
}

#[test]
fn a_reference_the_emitter_numbers_is_refused_by_name() {
    for emitted in ["U1", "R1", "C12", "SW1", "sw2", "R01"] {
        let mut v = bench_json();
        set(&mut v, 0, "I2C_SDA", emitted);
        let err = refusal(&v);
        assert_refusal(&err, "TestPointReferenceTaken", emitted);
        assert!(err.to_string().contains("emitter's own numbering"), "{err}");
    }
}

#[test]
fn the_same_net_twice_is_refused_by_name() {
    let mut v = bench_json();
    set(&mut v, 11, "GND", "TP12");
    let err = refusal(&v);
    assert_refusal(&err, "TestPointNetRepeated", "GND");
    assert!(err.to_string().contains("`TP10` and `TP12`"), "{err}");
}

#[test]
fn boot0_is_the_emitters_net_where_carried_and_no_declared_net_may_spell_it() {
    let spec = spec_of(&bench_json());
    assert!(
        names::own_nets(&spec)
            .unwrap()
            .contains(&"BOOT0".to_owned())
    );
    // Listed only where drawn: the board that carries no BOOT0 wires it.
    let v0 = spec_of(&serde_json::from_str(V0).unwrap());
    assert!(!names::own_nets(&v0).unwrap().contains(&"BOOT0".to_owned()));
    // A domain or a signal spelled like it, in any case.
    let mut v: Value = serde_json::from_str(V0).unwrap();
    v["i2c_buses"][0]["sda"]["net"] = json!("boot0");
    let err = refusal(&v);
    assert!(err.to_string().to_lowercase().contains("boot0"), "{err}");
    let mut v: Value = serde_json::from_str(V0).unwrap();
    v["voltage_domains"][0]["id"] = json!("Boot0");
    assert!(spec_of(&v).validate().is_err());
}

#[test]
fn boot0_on_a_connector_pin_alone_is_carried() {
    let mut v: Value = serde_json::from_str(V0).unwrap();
    v["connectors"][1]["pins"][1]["net"] = json!("BOOT0");
    let spec = spec_of(&v);
    assert_eq!(spec.validate(), Ok(()));
    assert!(bench::carries_boot0(&spec));
}

#[test]
fn the_switch_skips_a_spelled_sw1_and_refuses_another_spelling_of_it() {
    let mut v = bench_json();
    v["connectors"][0]["reference"] = json!("SW1");
    // J1 carried the console; renamed, the serial port's connector check
    // reads the new name from the pins, so only the reference moves.
    let spec = spec_of(&v);
    assert_eq!(bench::switch_reference(&spec).as_deref(), Some("SW2"));
    let mut v = bench_json();
    v["connectors"][0]["reference"] = json!("sw1");
    let err = refusal(&v);
    // Step 20's `EmitterReference`, which prints as the emitter's kind.
    assert_eq!(err.kind(), "DuplicateReference", "{err}");
    assert!(err.to_string().contains("SW1 and sw1"), "{err}");
}

/// Specification 13 (`AET-WO-1312` §1.4): a version-12 file is refused by its
/// version, with or without the new fields, never read as today's board.
#[test]
fn a_version_12_file_is_refused_by_its_version() {
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 13);
    for base in [bench_json(), serde_json::from_str(V0).unwrap()] {
        let mut v = base;
        v["schema_version"] = json!(12);
        assert_eq!(
            refusal(&v),
            E::UnsupportedSchemaVersion {
                version: 12,
                expected: 13
            }
        );
    }
}
