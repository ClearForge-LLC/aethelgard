//! `AET-WO-4411`, ruled 2026-11-25T20:10Z and 21:17Z: Sentinel-03's radio
//! module on a rail of its own (U13, `VDD_3V3_RF`), planted. The MCU's
//! regulator starts after the module's (`after`); the MCU's rail stays within
//! the module's VIH over its own; the module rail less its drop limit stays
//! at the module's 3.0 V; the rail's limit is declared; and an `after` that
//! names no open-drain power-good, or loops, is refused by name. Ruled
//! 2026-11-26T04:03Z (c1): the port runs through U31, a TXU0304 on both
//! rails, and each UART line's levels are judged at its receiver's pin.
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

/// F2, planted (ruled 2026-11-26T04:03Z): today's board, U1 driving U30's
/// RXD directly, refused at the module's pin. U1's high reaches `VDD_3V3`'s
/// 3,417,204 uV; U30 takes VDD + 0.3 V, its rail's 3,144,727 uV less the
/// 144 mV drop limit, 3,300,727 uV (WROOM-02 Table 6-3).
#[test]
fn the_f2_board_without_its_crossing_is_refused_at_the_modules_pin() {
    let e = refused(&direct(), "LineLevelUnmet");
    assert!(
        e.contains("`WIFI_TX`")
            && e.contains("U30's RXD")
            && e.contains("3417204 uV")
            && e.contains("3300727 uV"),
        "{e}"
    );
}

/// Sentinel-03 with U31 removed: the module on U1's port directly.
fn direct() -> Value {
    let mut v = base();
    let p = &mut v["power"];
    p.as_object_mut().unwrap().remove("uart_crossings");
    p["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U31");
    v
}

/// [`direct`] with `VDD_3V3_RF`'s drop limit at 20 mV, under the 27.5 mV
/// that keeps U1's 3,417,204 uV within U30's VDD + 0.3 V: its lines pass.
fn passing_direct() -> Value {
    let mut v = direct();
    for l in v["power"]["drop_limits"].as_array_mut().unwrap() {
        if l["domain"] == "VDD_3V3_RF" {
            l["max_millivolts"] = json!(20);
        }
    }
    v
}

fn u31(v: &mut Value) -> &mut Value {
    &mut v["power"]["uart_crossings"][0]
}

/// OE floats nowhere (ruled 04:03Z) and is held on VCCA (04:51Z, 08:59Z
/// R1): left `open`, not declared, on VCCB or wired, refused.
#[test]
fn a_translator_oe_left_floating_is_refused() {
    let mut v = base();
    u31(&mut v)["controls"][0]["strap"] = json!("open");
    assert!(refused(&v, "ControlPinUnwired").contains("OE is strapped `open`"));
    let mut v = base();
    u31(&mut v)["controls"] = json!([]);
    assert!(refused(&v, "ControlPinUnwired").contains("OE is declared neither"));
    // strapped to VCCB, or wired: refused by name (ruled 04:51Z, 08:59Z
    // R1; the row allows VCCB for an SPI crossing)
    let mut v = base();
    u31(&mut v)["controls"][0]["strap"] = json!("vccb");
    let e = refused(&v, "CrossingOeUnheld");
    assert!(e.contains("U31's OE is not strapped to VCCA"), "{e}");
    let mut v = base();
    u31(&mut v)["controls"][0] =
        json!({"pin": "OE", "strap": null, "wire": {"net": "WIFI_TX", "role": "drive"}});
    assert!(refused(&v, "CrossingOeUnheld").contains("not strapped to VCCA"));
}

/// A crossing's control takes a strap or a wire: `after` is a regulator's
/// enable.
#[test]
fn an_after_on_a_uart_crossings_control_is_refused() {
    let mut v = base();
    u31(&mut v)["controls"][0] = json!({"pin": "OE", "strap": null, "wire": null, "after": {"regulator": "U13", "pull_up": {"reference": "R33", "part": "CRCW0603_F_K", "ohms": 100000}, "source": "test: OE after U13's PG"}});
    let e = refused(&v, "PowerValueInvalid");
    assert!(e.contains("power.uart_crossings[0].controls"), "{e}");
}

/// The translator row's reader: at least one supply row, and its input
/// thresholds in order, under 1000 per mille.
#[test]
fn a_translator_row_with_no_supplies_or_thresholds_out_of_order_is_refused() {
    use aethelgard_ir::power_parts::parse_power_parts;
    let read = |edit: &dyn Fn(&mut Value)| {
        let mut t: Value = serde_json::from_str(include_str!("../data/power-parts.json")).unwrap();
        for row in t["parts"].as_array_mut().unwrap() {
            if row["part"] == "TXU0304PW" {
                edit(&mut row["kind"]["translator"]);
            }
        }
        parse_power_parts(&t.to_string()).map(|_| ())
    };
    assert_eq!(read(&|_| {}), Ok(()));
    let e = read(&|t| t["supplies"] = json!([])).unwrap_err();
    assert!(
        e.to_string()
            .contains("`kind.translator.supplies` is empty"),
        "{e}"
    );
    let e = read(&|t| t["levels"]["input_low_max_permille"]["value"] = json!(640)).unwrap_err();
    assert!(e.to_string().contains("low below high"), "{e}");
    let e = read(&|t| {
        t["levels"]["input_high_min_permille"]["value"] = json!(1000);
    })
    .unwrap_err();
    assert!(e.to_string().contains("high below 1000"), "{e}");
}

/// A UART crossing carries a radio module's port, once, from the MCU's
/// supply to the module's.
#[test]
fn a_uart_crossing_off_its_modules_port_or_rails_is_refused() {
    let mut v = base();
    u31(&mut v)["serial_port"] = json!("CONSOLE");
    assert!(refused(&v, "CrossingInvalid").contains("carries no declared radio module"));
    let mut v = base();
    let mut twin = u31(&mut v).clone();
    twin["reference"] = json!("U32");
    for (e, r) in twin["external"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .zip(["C95", "C96"])
    {
        e["reference"] = json!(r);
    }
    v["power"]["uart_crossings"]
        .as_array_mut()
        .unwrap()
        .push(twin);
    assert!(refused(&v, "CrossingInvalid").contains("more than one crossing"));
    let mut v = base();
    u31(&mut v)["b_domain"] = json!("VDD_3V3");
    let e = refused(&v, "CrossingDomainMismatch");
    assert!(e.contains("to U30 on `VDD_3V3_RF`"), "{e}");
}

/// The held table with one translator figure changed, and the base judged
/// against it.
fn with_txu(edit: impl Fn(&mut Value)) -> Result<aethelgard_ir::power::Checked, String> {
    use aethelgard_ir::power;
    use aethelgard_ir::power_parts::parse_power_parts;
    let mut t: Value = serde_json::from_str(include_str!("../data/power-parts.json")).unwrap();
    for row in t["parts"].as_array_mut().unwrap() {
        if row["part"] == "TXU0304PW" {
            edit(&mut row["kind"]["translator"]);
        }
    }
    let table = parse_power_parts(&t.to_string()).unwrap();
    let sensors = aethelgard_ir::parts::table().unwrap();
    let spec: Specification = serde_json::from_value(base()).unwrap();
    power::check_against(&spec, &table, sensors, &power::output_delays(sensors))
        .map(Option::unwrap)
        .map_err(|e| format!("{} {e}", e.kind()))
}

/// Each level at its receiver's pin, through U31, by its own branch: a
/// high under the receiver's threshold (VT+ at 900 per mille: U1's 2,634.7 mV
/// under 0.9 x 3,417.2), a low over it (VOL 800 mV over U30's 0.25 x
/// 3,000.7), a receiver drawing past the driver's stated load (U31's input
/// at 9 mA, past U1's 8 mA row), a rail outside the thresholds' stated
/// supply, a port faster than TMAX, and no B-to-A channel.
#[test]
fn each_line_level_through_the_translator_is_judged_at_its_receivers_pin() {
    let lv =
        |k: &'static str, value: i64| move |t: &mut Value| t["levels"][k]["value"] = json!(value);
    let e = with_txu(lv("input_high_min_permille", 900)).unwrap_err();
    assert!(
        e.starts_with("LineLevelUnmet")
            && e.contains("`WIFI_TX`")
            && e.contains("high is at least"),
        "{e}"
    );
    let e = with_txu(lv("output_low_max_millivolts", 800)).unwrap_err();
    assert!(
        e.starts_with("LineLevelUnmet") && e.contains("`U31_B1Y`") && e.contains("low is up to"),
        "{e}"
    );
    let e = with_txu(lv("input_current_max_nanoamps", 9_000_000)).unwrap_err();
    assert!(
        e.starts_with("LineLevelUnmet") && e.contains("draws up to 9000000 nA"),
        "{e}"
    );
    let e = with_txu(|t| t["levels"]["supply_millivolts"]["min"] = json!(3100)).unwrap_err();
    assert!(
        e.starts_with("LineLevelUnmet") && e.contains("outside the 3100 to 5500 mV"),
        "{e}"
    );
    let e = with_txu(|t| t["supplies"][1]["tmax_b_to_a_bps"]["value"] = json!(50_000)).unwrap_err();
    assert!(
        e.starts_with("CrossingTooFast") && e.contains("57600 baud"),
        "{e}"
    );
    let e = with_txu(|t| {
        for c in t["channels"].as_array_mut().unwrap() {
            c["direction"] = json!("a_to_b");
        }
    })
    .unwrap_err();
    assert!(
        e.starts_with("CrossingInvalid") && e.contains("B-to-A channel"),
        "{e}"
    );
}

/// The crossing closes the two rows its Ioff answers (ruled 04:03Z) and
/// names its own: U31's drop, the firmware's pulls, U30's high-impedance
/// load. Without it, the two rows stand.
#[test]
fn the_crossing_closes_the_power_down_and_ioff_rows_and_names_its_own() {
    let ids = |v: Value| {
        use aethelgard_ir::power;
        let sensors = aethelgard_ir::parts::table().unwrap();
        let spec: Specification = serde_json::from_value(v).unwrap();
        let table = aethelgard_ir::power_parts::table().unwrap();
        power::check_against(&spec, table, sensors, &power::output_delays(sensors))
            .unwrap()
            .unwrap()
            .screened
            .into_keys()
            .collect::<Vec<_>>()
    };
    let with = ids(base());
    for id in [
        "translator_supply_drop:U31",
        "translator_partial_power:U31",
        "translator_threshold_inferred:U31",
        "translator_rate_inferred:U31",
        "uart_internal_pulls:U31",
        "radio_output_load:U30",
        "radio_rail_monitor:U30",
    ] {
        assert!(with.iter().any(|x| x == id), "{id}: {with:?}");
    }
    for id in [
        "radio_power_down_order:U30",
        "radio_rail_ioff:U30",
        "translator_supply_not_counted:U11",
        "translator_supply_not_counted:U13",
    ] {
        assert!(!with.iter().any(|x| x == id), "{id}: {with:?}");
    }
    // U31 named by no load on either rail: its supply is uncounted, named
    let mut v = base();
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U31");
    let uncounted = ids(v);
    for id in [
        "translator_supply_not_counted:U11",
        "translator_supply_not_counted:U13",
    ] {
        assert!(uncounted.iter().any(|x| x == id), "{id}: {uncounted:?}");
    }
    // the planted F2 board is refused before its rows are read; with the
    // module rail's drop limit at 20 mV its levels pass, and they are
    let without = ids(passing_direct());
    for id in ["radio_power_down_order:U30", "radio_rail_ioff:U30"] {
        assert!(without.iter().any(|x| x == id), "{id}: {without:?}");
    }
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
/// with U31, U30's TX ends at U31's B4 on its own rail and U31's A4Y drives
/// `WIFI_RX` from `VDD_3V3`, so no path reaches the MCU from another rail
/// (ruled 04:03Z). Without U31, U30's TX onto U1's `WIFI_RX` is judged at
/// VDD = 0: the rail's declared maximum, 3465 mV, against DS12117 Table
/// 10's 4.0 V; at a 220 per mille tolerance it reaches 4026 mV, refused.
#[test]
fn the_modules_tx_reaches_an_unpowered_mcu_only_without_the_crossing() {
    use aethelgard_ir::off_domain::{Bound, Class, paths};
    let tx = |v: Value| {
        let spec: Specification = serde_json::from_value(v).unwrap();
        paths(&spec)
            .unwrap()
            .into_iter()
            .find(|p| p.class == Class::RadioModuleTx)
    };
    assert_eq!(tx(base()), None);
    let p = tx(direct()).expect("U30's TX is a path onto the MCU");
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
    let mut v = passing_direct();
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

/// The radio row's UART levels are read in order: a line is judged by them.
#[test]
fn a_radio_row_with_its_uart_levels_out_of_order_is_refused() {
    let text = include_str!("../data/radio-parts.json");
    assert!(aethelgard_ir::radio::parse(text).is_ok());
    for (from, to) in [
        (
            "\"vil_max_permille\": { \"value\": 250",
            "\"vil_max_permille\": { \"value\": 750",
        ),
        (
            "\"vol_max_permille\": { \"value\": 100",
            "\"vol_max_permille\": { \"value\": 800",
        ),
    ] {
        assert_eq!(text.matches(from).count(), 1, "{from}");
        let e = aethelgard_ir::radio::parse(&text.replace(from, to)).unwrap_err();
        assert!(e.contains("the UART's levels are out of order"), "{e}");
    }
}

/// S1, ruled 2026-11-26T04:51Z: the 3.3 V supply row (inferred TMAX and
/// thresholds) is a UART crossing's only; an SPI crossing between two 3.3 V
/// rails still reads the first row, and its VCCB band refuses it.
#[test]
fn the_3v3_supply_row_is_read_by_a_uart_crossing_only() {
    use aethelgard_ir::power_parts::{Kind, table};
    let Kind::Translator(t) = &table().unwrap().get("TXU0304PW").unwrap().kind else {
        panic!("TXU0304PW is a translator row");
    };
    let rails = (3135, 3465);
    assert_eq!(
        t.supplies_for(rails, rails, true).vccb_millivolts.range.min,
        3000
    );
    assert_eq!(
        t.supplies_for(rails, rails, false)
            .vccb_millivolts
            .range
            .min,
        1650
    );
}
