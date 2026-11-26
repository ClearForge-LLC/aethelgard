//! `AET-WO-4411`, ruled 2026-11-25T20:10Z and 21:17Z: Sentinel-03's radio
//! module on a rail of its own (U13, `VDD_3V3_RF`), planted. The MCU's
//! regulator starts after the module's (`after`); the MCU's rail stays within
//! the module's VIH over its own; the module rail less its drop limit stays
//! at the module's 3.0 V; the rail's limit is declared; and an `after` that
//! names no open-drain power-good, or loops, is refused by name.
use aethelgard_ir::Specification;
use serde_json::{Value, json};

const SPEC_PATH: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../examples/sentinel-03.json"
);

fn base() -> Value {
    serde_json::from_str(&std::fs::read_to_string(SPEC_PATH).unwrap()).unwrap()
}

fn kind(v: &Value) -> Result<(), String> {
    let spec: Specification = serde_json::from_value(v.clone()).map_err(|e| e.to_string())?;
    spec.validate().map_err(|e| format!("{} {e}", e.kind()))
}

fn regulator<'a>(v: &'a mut Value, reference: &str) -> &'a mut Value {
    v["power"]["regulators"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["reference"] == reference)
        .unwrap()
}

fn control<'a>(v: &'a mut Value, reference: &str, pin: &str) -> &'a mut Value {
    regulator(v, reference)["controls"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|c| c["pin"] == pin)
        .unwrap()
}

fn refused(v: &Value, want: &str) -> String {
    let e = kind(v).expect_err("the plant is refused");
    assert!(e.starts_with(want), "{e}");
    e
}

#[test]
fn the_base_holds_every_rule() {
    assert_eq!(kind(&base()), Ok(()));
}

/// (iii): without U11's `EN` after U13's `PG`, the MCU's UART can drive the
/// module's RXD while its rail is down.
#[test]
fn a_module_rail_the_mcus_regulator_does_not_follow_is_refused() {
    let mut v = base();
    let en = control(&mut v, "U11", "EN");
    en.as_object_mut().unwrap().remove("after");
    en["strap"] = json!("input");
    // U13's PG, no longer claimed, is declared open
    regulator(&mut v, "U13")["controls"]
        .as_array_mut()
        .unwrap()
        .push(json!({"pin": "PG", "strap": "open", "wire": null}));
    let e = refused(&v, "RadioRailUnsequenced");
    assert!(e.contains("U11's EN does not follow U13's PG"), "{e}");
}

/// The module's VIH maximum, VDD + 0.3 V (WROOM-02 Table 6-3), over its own
/// rail at its lowest: U11's top resistor at 106.0 kOhm keeps `VDD_3V3` at
/// most 300 mV over; at 106.3 kOhm it reaches 3,446,631 uV, 301,904 uV over
/// `VDD_3V3_RF`'s 3,144,727 uV (measured).
#[test]
fn an_mcu_rail_above_the_modules_vih_is_refused() {
    let at = |ohms: u32| {
        let mut v = base();
        regulator(&mut v, "U11")["feedback"]["top"]["ohms"] = json!(ohms);
        v
    };
    assert_eq!(kind(&at(106_000)), Ok(()));
    let e = refused(&at(106_300), "RadioLevelOverSupply");
    assert!(e.contains("3446631 uV") && e.contains("3144727 uV"), "{e}");
}

/// The module's 3.0 V minimum on its own rail (ruled 20:10Z): U13's
/// guaranteed 3,144,727 uV less the declared drop limit. 144 mV leaves
/// 3,000,727 uV; 145 mV leaves 2,999,727 uV, under it.
#[test]
fn a_module_rail_below_its_minimum_after_its_drop_is_refused() {
    let at = |mv: u32| {
        let mut v = base();
        for l in v["power"]["drop_limits"].as_array_mut().unwrap() {
            if l["domain"] == "VDD_3V3_RF" {
                l["max_millivolts"] = json!(mv);
            }
        }
        v
    };
    assert_eq!(kind(&at(144)), Ok(()));
    let e = refused(&at(145), "RadioSupplyBelowMinimum");
    assert!(e.contains("2999727 uV"), "{e}");
}

/// The module is its rail's load, so the rail needs a declared limit.
#[test]
fn a_module_rail_with_no_drop_limit_is_refused() {
    let mut v = base();
    v["power"]["drop_limits"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["domain"] != "VDD_3V3_RF");
    let e = refused(&v, "DropLimitUndeclared");
    assert!(e.contains("VDD_3V3_RF"), "{e}");
}

/// `after` names an open-drain power-good on another declared regulator,
/// sits on an enable, is declared alone, and never loops.
#[test]
fn an_after_that_cannot_start_its_regulator_is_refused() {
    let after = |v: &mut Value, regulator: &str| {
        control(v, "U11", "EN")["after"]["regulator"] = json!(regulator);
    };
    let mut v = base();
    after(&mut v, "U99");
    assert!(refused(&v, "ControlAfterInvalid").contains("no declared regulator"));
    let mut v = base();
    after(&mut v, "U11");
    assert!(refused(&v, "ControlAfterInvalid").contains("the regulator itself"));
    // A strap beside it: exactly one of the three.
    let mut v = base();
    control(&mut v, "U11", "EN")["strap"] = json!("input");
    assert!(
        refused(&v, "PowerValueInvalid").contains("exactly one of `strap`, `wire` and `after`")
    );
    // U13's PG declared in its own controls as well.
    let mut v = base();
    regulator(&mut v, "U13")["controls"]
        .as_array_mut()
        .unwrap()
        .push(json!({"pin": "PG", "strap": "open", "wire": null}));
    assert!(refused(&v, "PowerValueInvalid").contains("`after` declares"));
    // On a pin that is no enable.
    let mut v = base();
    let pg = control(&mut v, "U11", "PG");
    pg["strap"] = Value::Null;
    pg["after"] = json!({"regulator": "U10", "pull_up": {"reference": "R33", "part": "CRCW0603_F_K", "ohms": 100000}, "source": "test: a PG that follows"});
    regulator(&mut v, "U10")["controls"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["pin"] != "PG");
    assert!(refused(&v, "ControlAfterInvalid").contains("names it no enable"));
    // A loop: U13 waits on U11, which waits on U13.
    let mut v = base();
    let en = control(&mut v, "U13", "EN");
    en["strap"] = Value::Null;
    en["after"] = json!({"regulator": "U11", "pull_up": {"reference": "R33", "part": "CRCW0603_F_K", "ohms": 100000}, "source": "test: a loop"});
    control(&mut v, "U11", "PG")["strap"] = Value::Null;
    regulator(&mut v, "U11")["controls"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["pin"] != "PG");
    assert!(refused(&v, "ControlAfterInvalid").contains("loop"));
}
