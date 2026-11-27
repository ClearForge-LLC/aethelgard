//! `AET-WO-4413` (2), ruled D1–D3: a rail monitor holds the parts that name
//! it on its rail, judged by its falling minimum; it is push-pull, bypassed,
//! on a rail other than the MCU's, inside its row, releasing under its rail's
//! minimum at its pin, slow enough for a module's settling time, and named
//! only by a module's enable or a translator's `OE` on its rail. Each branch
//! has its planted case; Sentinel-03 is the clean twin.

use aethelgard_ir::power::Refusal;
use aethelgard_ir::power_parts::parse_power_parts;
use aethelgard_ir::supervisor::{self, Judgement};
use serde_json::{Value, json};

#[path = "common/power_table.rs"]
mod power_table;
use power_table::{assert_power, refused, row, sentinel_03, spec, table_json};

fn monitor(v: &mut Value) -> &mut Value {
    &mut v["power"]["rail_monitors"][0]
}

#[test]
fn sentinel_03_validates_and_u32_holds_its_radio_rail() {
    let v = sentinel_03();
    let s = spec(&v);
    s.validate().unwrap();
    let j = supervisor::judged(&s).unwrap();
    for d in ["U30", "U31.B", "U32"] {
        let (_, got) = j.iter().find(|(p, _)| p.device == d).unwrap();
        assert_eq!(got, &Judgement::Pass, "{d}");
    }
    // The screen it rests on, by its id (acknowledged in the specification).
    let c = supervisor::check(&s, aethelgard_ir::power::check(&s).unwrap().as_ref()).unwrap();
    assert!(
        c.iter()
            .any(|(id, t)| id == "supervisor_hysteresis_typical:U32"
                && t.contains("3157 mV")
                && t.contains("3158 mV")),
        "{c:?}"
    );
}

/// A threshold under the module's 3.0 V: a floor, by name, not a pass.
#[test]
fn a_monitor_falling_under_a_part_s_minimum_is_a_floor() {
    let v = sentinel_03();
    let mut t = table_json();
    row(&mut t, "TPS3839G33DBZR")["kind"]["supervisor"]["falling_millivolts"]["min"] = json!(2990);
    let planted = parse_power_parts(&t.to_string()).unwrap();
    let j = supervisor::judged_against(&spec(&v), &planted).unwrap();
    let (_, got) = j.iter().find(|(p, _)| p.device == "U30").unwrap();
    let Judgement::Floor(f) = got else {
        panic!("{got:?}")
    };
    assert_eq!((f.vcc_min_millivolts, f.reset_min_millivolts), (3000, 2990));
    assert!(f.reset.contains("U32's VIT− minimum"), "{}", f.reset);
    // U31.B's 1080 mV is still under it.
    let (_, b) = j.iter().find(|(p, _)| p.device == "U31.B").unwrap();
    assert_eq!(b, &Judgement::Pass);
}

/// A part on the rail the monitor does not drive stays named: a module
/// whose enable keeps its RC is not held by a monitor beside it.
#[test]
fn a_monitor_credits_only_what_it_drives() {
    let mut v = sentinel_03();
    v["power"]["uart_crossings"][0]["controls"] =
        json!([{"pin": "OE", "strap": "vcca", "wire": null}]);
    let named = supervisor::named(&spec(&v)).unwrap();
    assert_eq!(
        named.iter().map(|n| n.device.as_str()).collect::<Vec<_>>(),
        ["U31.B"]
    );
}

#[test]
fn a_monitor_s_refusals_are_named() {
    // The monitor removed: U31's OE is held by nothing (the power tree's
    // check, first); with U31 strapped again, U30 names no declared monitor.
    let mut v = sentinel_03();
    v["power"]["rail_monitors"] = json!([]);
    assert_power(&refused(&v), Refusal::CrossingOeUnheld, "U31");
    v["power"]["uart_crossings"][0]["controls"] =
        json!([{"pin": "OE", "strap": "vcca", "wire": null}]);
    assert_power(&refused(&v), Refusal::RailMonitorInvalid, "U30");
    // Driving nothing.
    let mut v = sentinel_03();
    v["radio_modules"][0]["enable"] = json!({
        "resistor": {"reference": "R90", "part": "CRCW0603_F_K", "ohms": 10000, "to": "VDD_3V3_RF"},
        "capacitor": {"reference": "C90", "part": "GRM188R71C105KA12"}});
    v["power"]["uart_crossings"][0]["controls"] =
        json!([{"pin": "OE", "strap": "vcca", "wire": null}]);
    assert_power(&refused(&v), Refusal::RailMonitorInvalid, "U32");
    // On the MCU's rail (U31 strapped, so its own check passes first): U30,
    // on `VDD_3V3_RF`, names a monitor of another rail.
    let mut v = sentinel_03();
    monitor(&mut v)["rail"] = json!("VDD_3V3");
    v["power"]["uart_crossings"][0]["controls"] =
        json!([{"pin": "OE", "strap": "vcca", "wire": null}]);
    assert_power(&refused(&v), Refusal::RailMonitorInvalid, "U30");
    // A module on the MCU's own supply, with no crossing, naming a monitor
    // there: refused as on the MCU's rail.
    let mut v = sentinel_03();
    monitor(&mut v)["rail"] = json!("VDD_3V3");
    v["radio_modules"][0]["supply"] = json!("VDD_3V3");
    v["power"]["uart_crossings"] = json!([]);
    let e = refused(&v);
    assert!(format!("{e}").contains("the MCU's supply"), "{e}");
    // Open-drain: it needs a pull-up the field does not declare.
    let mut v = sentinel_03();
    monitor(&mut v)["part"] = json!("TLV803SDBZR");
    assert_power(&refused(&v), Refusal::SupervisorOutOfRange, "U32");
    // Its release at or above the rail's minimum at its pin: 3303 - 146 =
    // 3157, not above 3126 + 31.
    let mut v = sentinel_03();
    v["power"]["drop_limits"][1]["max_millivolts"] = json!(146);
    assert_power(&refused(&v), Refusal::SupervisorAboveRail, "U32");
    // A bypass that is no capacitor row.
    let mut v = sentinel_03();
    monitor(&mut v)["bypass"]["part"] = json!("CRCW0603_F_K");
    assert_power(&refused(&v), Refusal::PowerPartUnknown, "C95");
    // A regulator's enable may not name one.
    let mut v = sentinel_03();
    for r in v["power"]["regulators"].as_array_mut().unwrap() {
        if r["reference"] == "U13" {
            for c in r["controls"].as_array_mut().unwrap() {
                if c["pin"] == "EN" {
                    *c = json!({"pin": "EN", "strap": null, "wire": null, "monitor": "U32"});
                }
            }
        }
    }
    assert_power(&refused(&v), Refusal::RailMonitorInvalid, "U13");
}

/// The module's enable must stay low for its settling time: a delay under
/// it is refused, by the planted row.
#[test]
fn a_monitor_faster_than_the_module_settles_is_refused() {
    let v = sentinel_03();
    let mut t = table_json();
    row(&mut t, "TPS3839G33DBZR")["kind"]["supervisor"]["delay_min_microseconds"]["value"] =
        json!(10);
    let planted = parse_power_parts(&t.to_string()).unwrap();
    let s = spec(&v);
    let checked = aethelgard_ir::power::check(&s).unwrap();
    let e = supervisor::check_against(&s, checked.as_ref(), &planted).unwrap_err();
    assert_power(&e, Refusal::RailMonitorInvalid, "U32");
    assert!(format!("{e}").contains("U30"), "{e}");
}

/// A push-pull part on `NRST`, which the MCU drives too, is refused by name.
#[test]
fn a_push_pull_supervisor_on_nrst_is_refused() {
    let mut v = sentinel_03();
    v["mcu"]["reset_supervisor"]["part"] = json!("TPS3839G33DBZR");
    let e = refused(&v);
    assert!(format!("{e}").contains("push-pull output"), "{e}");
    assert!(
        format!("{e}").contains("`NRST` takes an open-drain one"),
        "{e}"
    );
}

/// A monitor whose VDD range does not cover its rail is refused by name.
#[test]
fn a_monitor_outside_its_supply_range_is_refused() {
    let v = sentinel_03();
    let mut t = table_json();
    row(&mut t, "TPS3839G33DBZR")["kind"]["supervisor"]["vdd_millivolts"]["max"] = json!(3500);
    let planted = parse_power_parts(&t.to_string()).unwrap();
    let s = spec(&v);
    let checked = aethelgard_ir::power::check(&s).unwrap();
    let e = supervisor::check_against(&s, checked.as_ref(), &planted).unwrap_err();
    assert_power(&e, Refusal::SupervisorOutOfRange, "U32");
}

/// The rail set above a consumer's maximum is refused by name: U31's VCCB
/// rows end at 3.6 V, which 3430 mV +/- 5 % passes.
#[test]
fn the_rail_above_a_consumer_s_maximum_is_refused() {
    let mut v = sentinel_03();
    for d in v["voltage_domains"].as_array_mut().unwrap() {
        if d["id"] == "VDD_3V3_RF" {
            d["tolerance_permille"] = json!(50);
        }
    }
    assert_power(&refused(&v), Refusal::DomainOutsideConsumer, "VDD_3V3_RF");
}

/// An enable takes its RC or a monitor, exactly one.
#[test]
fn an_enable_with_both_or_neither_is_refused() {
    for enable in [
        json!({}),
        json!({"monitor": "U32", "capacitor": {"reference": "C90", "part": "GRM188R71C105KA12"}}),
    ] {
        let mut v = sentinel_03();
        v["radio_modules"][0]["enable"] = enable.clone();
        let e = refused(&v);
        assert!(format!("{e}").contains("enable"), "{enable}: {e}");
    }
}

/// U31's OE held by a monitor on its A side's rail is not held: the
/// crossing needs VCCA's strap or VCCB's monitor.
#[test]
fn a_crossing_s_oe_on_a_monitor_of_the_wrong_rail_is_unheld() {
    let mut v = sentinel_03();
    monitor(&mut v)["rail"] = json!("VDD_5V");
    assert_power(&refused(&v), Refusal::CrossingOeUnheld, "U31");
}
