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
fn with_load(v: Value, reference: &str) -> Value {
    with_load_on(v, "VDD_1V8", reference)
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
    // (ii) The MCU is a part the board has, refused as counted, never as no
    // part: `a_load_naming_the_mcu_is_refused_naming_its_table_row`.
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

/// `v` with one more load, of 1 mA on `domain`, naming `reference`: `1209b`'s
/// `adv/s02-U12-*` copies are the fixture with such a load naming `U12`.
fn with_load_on(mut v: Value, domain: &str, reference: &str) -> Value {
    v["power"]["loads"].as_array_mut().unwrap().push(json!({
        "domain": domain, "milliamps": 1,
        "source": "test: AET-WO-1209", "reference": reference
    }));
    v
}

fn assert_counted(v: &Value, domain: &str, reference: &str, counted_by: &str) {
    let at = v["power"]["loads"].as_array().unwrap().len() - 1;
    let path = format!("power.loads[{at}].reference");
    let err = spec(v).validate().unwrap_err();
    assert_eq!(
        err,
        E::LoadNamesCountedPart {
            path: path.clone(),
            domain: domain.into(),
            reference: reference.into(),
            counted_by: counted_by.into(),
        }
    );
    assert_eq!(err.kind(), "LoadNamesCountedPart");
    assert_eq!(err.id(), Some(reference));
    assert_eq!(err.path(), Some(path.as_str()));
    assert_eq!(
        err.to_string(),
        format!(
            "{path} on `{domain}` names `{reference}`, whose current is already counted by {counted_by}"
        )
    );
}

/// `AET-WO-1209c`, `1209b` F1: a load naming a declared regulator is refused
/// on its input rail, where its row already counts it, on the rail it makes,
/// of which it is the source, and on any other; the refusal names the row.
#[test]
fn a_load_naming_a_regulator_is_refused_naming_its_input_row() {
    // The LDO `U12` makes `VDD_1V8` from `VDD_3V3`.
    for domain in ["VDD_3V3", "VDD_1V8"] {
        assert_counted(
            &with_load_on(fixture(), domain, "U12"),
            domain,
            "U12",
            "regulator `U12`'s input on `VDD_3V3`",
        );
    }
    // The buck `U10` is fed from the battery, which no load is on.
    assert_counted(
        &with_load_on(fixture(), "VDD_5V", "U10"),
        "VDD_5V",
        "U10",
        "regulator `U10`'s input on `battery`",
    );
}

/// `AET-WO-1209c`: a load naming the MCU is refused, its current being its
/// table row's, by whichever `U<n>` the emitter numbers it.
#[test]
fn a_load_naming_the_mcu_is_refused_naming_its_table_row() {
    assert_eq!(names::mcu_reference(&spec(&fixture())), "U1");
    assert_counted(
        &with_load_on(fixture(), "VDD_3V3", "U1"),
        "VDD_3V3",
        "U1",
        "the MCU's table row",
    );
    // Its I2C device renamed `U1`, the fixture numbers the MCU `U2`: a load
    // naming `U1` names the device and passes, one naming `U2` is refused.
    let mut v: Value = serde_json::from_str(&FIXTURE.replace(r#""U2""#, r#""U1""#)).unwrap();
    assert_eq!(names::mcu_reference(&spec(&v)), "U2");
    spec(&v).validate().unwrap();
    v = with_load_on(v, "VDD_3V3", "U2");
    assert_counted(&v, "VDD_3V3", "U2", "the MCU's table row");
}

/// `AET-WO-1209c` §1.2, §5: only the regulator's own reference is counted
/// elsewhere. Its feedback resistors, external parts, the crossing and its
/// pull-up are loads in their own right, and each is judged, entry by entry
/// in document order, after `LoadNamesNoPart`.
#[test]
fn a_regulator_s_own_parts_pass_and_no_part_is_judged_first() {
    for (domain, r) in [
        ("VDD_5V", "R10"),
        ("VDD_5V", "C10"),
        ("VDD_5V", "L1"),
        ("VDD_3V3", "C30"),
        ("VDD_1V8", "C31"),
        ("VDD_1V8", "U13"),
        ("VDD_1V8", "R40"),
    ] {
        spec(&with_load_on(fixture(), domain, r))
            .validate()
            .unwrap();
    }
    // Entries are judged in document order, each as a part and then as
    // counted: of a load naming no part and one naming a regulator, the
    // earlier is the one refused.
    let v = with_load_on(with_load_on(fixture(), "VDD_3V3", "U99"), "VDD_3V3", "U12");
    assert_eq!(spec(&v).validate().unwrap_err().kind(), "LoadNamesNoPart");
    let v = with_load_on(with_load_on(fixture(), "VDD_3V3", "U12"), "VDD_3V3", "U99");
    assert_eq!(
        spec(&v).validate().unwrap_err().kind(),
        "LoadNamesCountedPart"
    );
}
