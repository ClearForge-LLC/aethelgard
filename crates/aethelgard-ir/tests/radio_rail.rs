//! `AET-WO-4411`, ruled 2026-11-25T20:10Z and 21:17Z: Sentinel-03's radio
//! module on a rail of its own (U13, `VDD_3V3_RF`), planted. The MCU's
//! regulator starts after the module's (`after`); the MCU's rail stays within
//! the module's VIH over its own; the module rail less its drop limit stays
//! at the module's 3.0 V; the rail's limit is declared; and an `after` that
//! names no open-drain power-good, or loops, is refused by name.
use aethelgard_ir::Specification;
use serde_json::{Value, json};

#[path = "common/power_table.rs"]
mod power_table;

fn base() -> Value {
    power_table::sentinel_03()
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

/// Step 25 (`AET-WO-1216b`, ruled D4), raised by the 21:17Z ruling's (iii):
/// `VDD_3V3_RF` leads `VDD_3V3` at every power-up, so U30's TX onto U1's
/// `WIFI_RX` is judged at VDD = 0, never silently passed: the rail's
/// declared maximum, 3465 mV, against DS12117 Table 10's 4.0 V; at a 220
/// per mille tolerance it reaches 4026 mV, and step 25 refuses it before the
/// module's own range does (step 27).
#[test]
fn the_modules_tx_onto_an_unpowered_mcu_is_judged_at_vdd_0() {
    use aethelgard_ir::off_domain::{Bound, Class, paths};
    let spec: Specification = serde_json::from_value(base()).unwrap();
    let p = paths(&spec)
        .unwrap()
        .into_iter()
        .find(|p| p.class == Class::RadioModuleTx)
        .expect("U30's TX is a path onto the MCU");
    assert_eq!(p.net, "WIFI_RX");
    assert!(p.judged(), "{p:?}");
    assert!(
        matches!(
            p.bound,
            Bound::Stated {
                millivolts: 3465,
                ..
            }
        ),
        "{p:?}"
    );
    let mut v = base();
    for d in v["voltage_domains"].as_array_mut().unwrap() {
        if d["id"] == "VDD_3V3_RF" {
            d["tolerance_permille"] = json!(220);
        }
    }
    let e = refused(&v, "OffDomainAboveLimit");
    assert!(e.contains("U30's TX") && e.contains("4026 mV"), "{e}");
}

/// The two `after` branches the specification alone cannot reach: a
/// regulator whose row's `PG` is no open drain (the held table, changed),
/// and a `PG` that already holds another enable.
#[test]
fn an_after_on_a_pg_that_is_no_open_drain_or_already_holds_one_is_refused() {
    use aethelgard_ir::power;
    use aethelgard_ir::power_parts::parse_power_parts;
    let mut t: Value = serde_json::from_str(include_str!("../data/power-parts.json")).unwrap();
    for row in t["parts"].as_array_mut().unwrap() {
        if row["part"] == "LMR33620ADDA" {
            for c in row["kind"]["buck"]["controls"].as_array_mut().unwrap() {
                if c["pin"] == "PG" {
                    c["straps"] = json!([]);
                }
            }
        }
    }
    let table = parse_power_parts(&t.to_string()).unwrap();
    let sensors = aethelgard_ir::parts::table().unwrap();
    let spec: Specification = serde_json::from_value(base()).unwrap();
    let e =
        power::check_against(&spec, &table, sensors, &power::output_delays(sensors)).unwrap_err();
    assert_eq!(e.kind(), "ControlAfterInvalid", "{e}");
    assert!(e.to_string().contains("no open-drain `PG`"), "{e}");

    let mut v = base();
    let en = control(&mut v, "U10", "EN");
    en["strap"] = Value::Null;
    en["after"] = json!({"regulator": "U13", "pull_up": {"reference": "R33", "part": "CRCW0603_F_K", "ohms": 100000}, "source": "test: a second enable on U13's PG"});
    assert!(refused(&v, "ControlAfterInvalid").contains("already holds another enable"));
}

/// A deadlock through a supply (adversarial F1): U13 fed from `VDD_5V`
/// (U10's), U10's EN after U11's PG, and U11's after U13's: U13 waits on
/// U10, U10 on U11, U11 on U13, and none starts.
#[test]
fn an_after_that_waits_on_its_own_follower_through_a_supply_is_refused() {
    let mut v = base();
    regulator(&mut v, "U13")["input"] = json!("VDD_5V");
    let en = control(&mut v, "U10", "EN");
    en["strap"] = Value::Null;
    en["after"] = json!({"regulator": "U11", "pull_up": {"reference": "R33", "part": "CRCW0603_F_K", "ohms": 100000}, "source": "test: a supply loop"});
    regulator(&mut v, "U11")["controls"]
        .as_array_mut()
        .unwrap()
        .retain(|c| c["pin"] != "PG");
    let e = refused(&v, "ControlAfterInvalid");
    assert!(e.contains("loop of `after`s and supplies"), "{e}");
}
