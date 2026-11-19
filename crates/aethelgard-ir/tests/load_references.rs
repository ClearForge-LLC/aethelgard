//! `AET-WO-1209` §1.2, D3 as ruled: `validate` refuses a `power.loads`
//! reference that names no part the board has (step 20a), by the reference,
//! the rail and the JSON path. Before, such a load passed every gate and
//! counted against its rail (`4202`'s finding; on `main` at `96517013`,
//! `validate` and `emit` both accepted Sentinel-03 with U2 removed and its
//! load kept). Each form a load may name passes.

use aethelgard_ir::{IrValidationError as E, Specification, bench, names};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/sentinel-02-power.json");

fn fixture() -> Value {
    serde_json::from_str(FIXTURE).unwrap()
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).unwrap()
}

/// `v` with one more load, of 1 mA on `VDD_1V8`, naming `reference`.
fn with_load(mut v: Value, reference: &str) -> Value {
    v["power"]["loads"].as_array_mut().unwrap().push(json!({
        "domain": "VDD_1V8", "milliamps": 1,
        "source": "test: AET-WO-1209", "reference": reference
    }));
    v
}

fn assert_refused(v: &Value, path: &str, domain: &str, reference: &str) {
    let err = spec(v).validate().unwrap_err();
    assert_eq!(
        err,
        E::LoadNamesNoPart {
            path: path.into(),
            domain: domain.into(),
            reference: reference.into(),
        }
    );
    assert_eq!(err.kind(), "LoadNamesNoPart");
    assert_eq!(err.id(), Some(reference));
    assert_eq!(err.path(), Some(path));
    let text = err.to_string();
    assert!(
        text.starts_with(&format!("{path} on `{domain}` names `{reference}`")),
        "{text}"
    );
}

#[test]
fn a_load_naming_a_part_the_board_lacks_is_refused_by_reference_and_rail() {
    // `4202`'s case: a device removed, its load kept.
    let mut v = fixture();
    v["i2c_devices"]
        .as_array_mut()
        .unwrap()
        .retain(|d| d["reference"] != "U2");
    assert_refused(&v, "power.loads[4].reference", "VDD_3V3", "U2");
    // The crossing removed, its translator's load kept (the five test
    // helpers before `AET-WO-1209` D4).
    let mut v = fixture();
    v["power"]["crossings"] = json!([]);
    v["accept_lifecycle"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["part"] != "ICM20948");
    for key in ["spi_devices", "spi_buses", "net_constraints"] {
        let kept: Vec<Value> = v[key]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| {
                !(x["reference"] == "U5"
                    || x["id"] == "IMU"
                    || x["net_name"].as_str().is_some_and(|n| n.starts_with("IMU")))
            })
            .cloned()
            .collect();
        v[key] = Value::Array(kept);
    }
    assert_refused(&v, "power.loads[11].reference", "VDD_3V3", "U13");
    // Spellings the emitter never gives: case, a leading zero, `R0`, another
    // `U` than the MCU's, a switch the specification does not ask for, a
    // prefix it never numbers.
    let at = fixture()["power"]["loads"].as_array().unwrap().len();
    for r in ["r1", "R01", "R0", "U99", "SW1", "Q1", "C"] {
        assert_refused(
            &with_load(fixture(), r),
            &format!("power.loads[{at}].reference"),
            "VDD_1V8",
            r,
        );
    }
}

#[test]
fn each_form_of_a_part_the_board_has_passes() {
    // (i) Declared: the committed fixture's loads name devices, a connector,
    // the translator and its CS pull-up, and a feedback resistor.
    spec(&fixture()).validate().unwrap();
    // (ii) The MCU, numbered as the emitter numbers it.
    let mcu = names::mcu_reference(&spec(&fixture()));
    spec(&with_load(fixture(), &mcu)).validate().unwrap();
    // (iii) The reset switch, when the specification asks for one.
    let mut v = fixture();
    v["mcu"]["reset_switch"] = json!(true);
    let switch = bench::switch_reference(&spec(&v)).unwrap();
    spec(&with_load(v, &switch)).validate().unwrap();
    // (iv) A resistor or capacitor the emitter numbers, by its pattern: which
    // exist is the emitter's draw order, which only the CLI's `validate`
    // holds.
    for r in ["R99", "C7"] {
        spec(&with_load(fixture(), r)).validate().unwrap();
    }
}
