//! `AET-WO-1209` D3, as ruled: the CLI's `validate` holds each
//! `power.loads` reference to the parts the emitter draws for that
//! specification (`LoadNamesUndrawnPart`), beyond the form step 20a holds.
//! The measured case: the test helpers kept a removed crossing's CS pull-up
//! (`R40`), an `R<n>` the pattern passes and the emitter no longer draws.
//!
//! `AET-WO-1209b`: and the drawn part has a pin on the load's rail
//! (`LoadPartOffRail`), such as Sentinel-03's straps renumbered onto
//! BOOT0's pull-down.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

/// `aethelgard validate` on `v` through stdin: its exit code and its one line.
fn validate(v: &Value) -> (Option<i32>, Value) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .arg("validate")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(v.to_string().as_bytes())
        .unwrap();
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn with_load(mut v: Value, reference: &str) -> Value {
    v["power"]["loads"].as_array_mut().unwrap().push(json!({
        "domain": "VDD_3V3", "milliamps": 1,
        "source": "test: AET-WO-1209", "reference": reference
    }));
    v
}

#[test]
fn a_load_naming_a_resistor_the_emitter_does_not_draw_is_refused() {
    // The helper's board, as it stands, validates: its loads name drawn parts.
    let (code, v) = validate(&common::power::power_tree());
    assert_eq!(code, Some(0), "{v}");
    // The removed crossing's CS pull-up, and a number the emitter never
    // reaches: both pass step 20a's pattern, and neither is drawn.
    let at = common::power::power_tree()["power"]["loads"]
        .as_array()
        .unwrap()
        .len();
    for r in ["R40", "R99"] {
        let (code, v) = validate(&with_load(common::power::power_tree(), r));
        assert_eq!(code, Some(2), "{v}");
        let path = format!("power.loads[{at}].reference");
        assert_eq!(v["error"]["kind"], "LoadNamesUndrawnPart", "{v}");
        assert_eq!(v["error"]["id"], r, "{v}");
        assert_eq!(v["error"]["path"], path.as_str(), "{v}");
        assert_eq!(
            v["error"]["message"],
            format!(
                "{path} on `VDD_3V3` names `{r}`, which the emitter does not draw for this specification"
            ),
            "{v}"
        );
    }
    // With its crossing, the fixture draws `R40`: the same load passes.
    let full: Value = serde_json::from_str(include_str!(
        "../../aethelgard-ir/tests/fixtures/sentinel-02-power.json"
    ))
    .unwrap();
    let (code, v) = validate(&full);
    assert_eq!(code, Some(0), "{v}");
    // The emitter's own `U<n>` and `SW<n>` are drawn: a load naming the MCU,
    // or the reset switch when one is asked for, is never refused as undrawn.
    // The MCU's current is its table row's, so the IR's step 20a refuses a
    // load naming it as counted (`AET-WO-1209c`). The switch is on `NRST`
    // and `GND` only, so a load naming it on a rail is refused as off it
    // (`AET-WO-1209b`).
    let (code, v) = validate(&with_load(full.clone(), "U1"));
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "LoadNamesCountedPart", "{v}");
    let mut v = full.clone();
    v["mcu"]["reset_switch"] = json!(true);
    let (code, v) = validate(&with_load(v, "SW1"));
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "LoadPartOffRail", "{v}");
    assert_eq!(v["error"]["id"], "SW1", "{v}");
}

/// `validate` on a copy of `v` whose load `at` names `reference` instead.
fn renamed(mut v: Value, at: usize, reference: &str) -> (Option<i32>, Value) {
    v["power"]["loads"][at]["reference"] = json!(reference);
    validate(&v)
}

#[test]
fn a_load_naming_a_part_with_no_pin_on_its_rail_is_refused() {
    let s03: Value =
        serde_json::from_str(include_str!("../../../examples/sentinel-03.json")).unwrap();
    // As committed: R6 and R7, the flash's WP and HOLD pull-ups, are on
    // `VDD_3V3`, where their loads are.
    let (code, v) = validate(&s03);
    assert_eq!(code, Some(0), "{v}");
    let at = s03["power"]["loads"]
        .as_array()
        .unwrap()
        .iter()
        .position(|l| l["reference"] == "R7")
        .unwrap();
    assert_eq!(s03["power"]["loads"][at]["domain"], "VDD_3V3");
    // Renumbered one strap on: `R8` is BOOT0's pull-down, drawn on `BOOT0`
    // and `GND`. A part that exists is not enough.
    let (code, v) = renamed(s03.clone(), at, "R8");
    assert_eq!(code, Some(2), "{v}");
    let path = format!("power.loads[{at}].reference");
    assert_eq!(v["error"]["kind"], "LoadPartOffRail", "{v}");
    assert_eq!(v["error"]["id"], "R8", "{v}");
    assert_eq!(v["error"]["path"], path.as_str(), "{v}");
    assert_eq!(
        v["error"]["message"],
        format!(
            "{path} on `VDD_3V3` names `R8`, which the emitter draws with no pin on `VDD_3V3`: its pins are on `BOOT0`, `GND`"
        ),
        "{v}"
    );
    // A part on another rail is refused the same way: the 5 V divider's top
    // resistor, drawn on `VDD_5V` and `U10_FB`.
    let (code, v) = renamed(s03, at, "R10");
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "LoadPartOffRail", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .ends_with("its pins are on `U10_FB`, `VDD_5V`"),
        "{v}"
    );
}

/// §5: a part that reaches a rail only through another part is not on it.
/// Sentinel-02's ICM-20948 (`U5`) sits behind the translator `U13` on
/// `VDD_1V8`, which the LDO `U12` makes from `VDD_3V3`: its current is on
/// `VDD_1V8`, and a load naming it on `VDD_3V3` is refused. The translator,
/// drawn on both rails, may be named on either.
#[test]
fn a_part_reached_through_another_part_is_not_on_that_rail() {
    let full: Value = serde_json::from_str(include_str!(
        "../../aethelgard-ir/tests/fixtures/sentinel-02-power.json"
    ))
    .unwrap();
    let (code, v) = validate(&with_load(full.clone(), "U5"));
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "LoadPartOffRail", "{v}");
    assert_eq!(v["error"]["id"], "U5", "{v}");
    let mut v = full;
    v["power"]["loads"].as_array_mut().unwrap().push(json!({
        "domain": "VDD_1V8", "milliamps": 1,
        "source": "test: AET-WO-1209b", "reference": "U13"
    }));
    let (code, v) = validate(&v);
    assert_eq!(code, Some(0), "{v}");
}
