//! `validate`'s step 25 (`AET-WO-1216b`, `1216` ruled D1–D5): every path onto
//! an MCU pin from off `mcu.supply`, judged at VDD = 0 against DS12117
//! Table 10, with the exemption read from drawn wiring only.
//!
//! Some arms are unreachable through `validate` today, because an earlier
//! step refuses their input first: an I2C bus or a crossing's A side off
//! `mcu.supply` (steps 5a, 16), a battery divider above VREF+ (step 16) and
//! an exempt rail above 4.0 V (no held regulator boosts from `mcu.supply`).
//! Those are judged here through [`paths`] on a parsed, unvalidated
//! specification, which is what step 25 reads.

use aethelgard_ir::Specification;
use aethelgard_ir::off_domain::{Bound, Class, Lead, Path, limit_at_zero, paths};
use aethelgard_ir::pin_parts::Basis;
use serde_json::{Value, json};

fn fixture(rel: &str) -> Value {
    let root = concat!(env!("CARGO_MANIFEST_DIR"), "/../../");
    serde_json::from_str(&std::fs::read_to_string(format!("{root}{rel}")).unwrap()).unwrap()
}

fn d1() -> Value {
    fixture("crates/aethelgard-ir/tests/fixtures/d1-drv8876.json")
}

fn s02() -> Value {
    fixture("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json")
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

fn walk(v: &Value) -> Vec<Path> {
    paths(&spec(v)).expect("paths")
}

fn of(v: &Value, class: Class) -> Vec<Path> {
    walk(v).into_iter().filter(|p| p.class == class).collect()
}

fn mv(p: &Path) -> i64 {
    match p.bound {
        Bound::Stated { millivolts, .. } => millivolts,
        Bound::Unbounded(_) => panic!("{p:?} is unbounded"),
    }
}

fn leads(p: &Path) -> bool {
    matches!(p.lead, Lead::Leads(_))
}

fn r2(v: &mut Value, ohms: u32) {
    v["pin_devices"][0]["analog"][0]["transfer"]["current_output"]["resistor"]["ohms"] =
        json!(ohms);
}

// ------------------------------------------------------------ path classes

/// D1 as `1216` measured it, R2 1.5 kΩ: 3 mA into 1555 Ω is 4665 mV from a
/// `VMOTOR` no tree orders, refused at VDD = 0 by name, the figure named as
/// a recommended operating condition (D5).
#[test]
fn a_current_output_above_4v0_is_refused_and_named_recommended_operating() {
    let mut v = d1();
    r2(&mut v, 1500);
    let p = &of(&v, Class::PinDeviceCurrentOutput)[0];
    assert_eq!(
        (p.net.as_str(), mv(p), leads(p)),
        ("DRV_IPROPI", 4665, true)
    );
    assert!(matches!(
        p.bound,
        Bound::Stated {
            basis: Basis::RecommendedOperating,
            ..
        }
    ));
    let e = spec(&v).validate().unwrap_err();
    assert_eq!(e.kind(), "OffDomainAboveLimit", "{e}");
    assert_eq!(e.id(), Some("DRV_IPROPI"));
    assert_eq!(
        e.path(),
        Some("pin_devices[0].analog[0].transfer.current_output.resistor")
    );
    let text = e.to_string();
    for says in [
        "up to 4665 mV",
        "takes at most 4000 mV",
        "Table 10",
        "a recommended operating condition, not a stated maximum",
        "1555 ohm",
        "`VMOTOR`",
    ] {
        assert!(text.contains(says), "`{says}` not in: {text}");
    }
}

/// D3 as ruled: R2 1.07 kΩ is at most 1110 Ω (1 % + 100 ppm/K over D1's
/// −20 to 85 °C from 20 °C + 2 % drift = 36 500 ppm), so the held 3 mA gives
/// 3330 mV and D1 validates. At the DRV8876's 3.5 A peak output current the
/// mirror gives 3.5 mA × 1110 Ω = 3885 mV, also under 4.0 V; **that figure
/// rests on the typical 1000 µA/A ratio**: SLVSDS7B states no AERR above
/// 2 A, so it is no bound the engine holds, and step 25 does not judge it.
#[test]
fn d1_s_r2_of_1k07_clears_4v0_on_the_held_figure_and_at_the_typical_peak() {
    let v = d1();
    let p = &of(&v, Class::PinDeviceCurrentOutput)[0];
    assert_eq!(mv(p), 3330);
    spec(&v).validate().unwrap();
    // The held figure is 3000 uA x R2 at its highest: R2 is 1110 ohm.
    let r_hi = mv(p) * 1000 / 3000;
    assert_eq!(r_hi, 1110);
    assert_eq!((3500 * r_hi + 999) / 1000, 3885, "the typical-ratio peak");
    assert!(3885 < limit_at_zero(&spec(&v), None).unwrap().0);
}

/// 4.0 V to the millivolt: R2 at 1286 Ω is at most 1333 Ω (3999 mV) and
/// validates; 1287 Ω is at most 1334 Ω (4002 mV) and is refused.
#[test]
fn the_4v0_limit_is_judged_to_the_millivolt() {
    let mut v = d1();
    r2(&mut v, 1286);
    assert_eq!(mv(&of(&v, Class::PinDeviceCurrentOutput)[0]), 3999);
    spec(&v).validate().unwrap();
    r2(&mut v, 1287);
    assert_eq!(mv(&of(&v, Class::PinDeviceCurrentOutput)[0]), 4002);
    assert_eq!(
        spec(&v).validate().unwrap_err().kind(),
        "OffDomainAboveLimit"
    );
}

/// A pin-device pull to a rail no tree orders: 5 V ±5 % on D2's nFAULT
/// (PA1, FT) passes A1 in operation and is refused at VDD = 0; a second
/// 3.3 V ±5 % rail (3465 mV) is judged and passes.
#[test]
fn a_pin_device_pull_to_a_leading_rail_is_judged() {
    let at = |id: &str, nominal: u32| {
        let mut v = fixture("crates/aethelgard-ir/tests/fixtures/d2-drv8833.json");
        v["voltage_domains"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": id, "nominal_millivolts": nominal, "tolerance_permille": 50}));
        v["pin_devices"][0]["part_limits"]["input_pwm_max_hz"] =
            json!({"value": 50000, "source": "a declaration for this test, no held document"});
        let s = v["pin_devices"][0]["signals"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["device_pin"] == "nFAULT")
            .unwrap();
        s["pin"] = json!("PA1");
        s["pull"] = json!({
            "resistor": {"reference": "R9", "part": "CRCW0603_F_K", "ohms": 10000, "to": id},
            "source": "AET-WO-1216b test: a declared pull-up"
        });
        v["pin_devices"][0]["temperature_celsius"] =
            d1()["pin_devices"][0]["temperature_celsius"].clone();
        v
    };
    let v = at("VDD_5V", 5000);
    let p = &of(&v, Class::PinDevicePull)[0];
    assert_eq!(
        (mv(p), leads(p), p.pin.as_deref()),
        (5250, true, Some("PA1"))
    );
    assert_eq!(
        spec(&v).validate().unwrap_err().kind(),
        "OffDomainAboveLimit"
    );
    let v = at("VDD_3V3B", 3300);
    assert_eq!(mv(&of(&v, Class::PinDevicePull)[0]), 3465);
    spec(&v).validate().unwrap();
    // At the limit is admitted (Table 11 note 3, D4): 3960 mV + 1 % is
    // exactly 4000 mV.
    let mut v = at("VDD_4V0", 3960);
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .last_mut()
        .unwrap()["tolerance_permille"] = json!(10);
    assert_eq!(mv(&of(&v, Class::PinDevicePull)[0]), 4000);
    spec(&v).validate().unwrap();
}

/// The battery divider: Sentinel-02's 26.1 V through R50/R51 at their
/// worst corners is 2536 mV, judged (the battery always leads) and passed.
#[test]
fn the_battery_divider_is_judged_at_the_battery_s_maximum() {
    let p = &of(&s02(), Class::BatteryDivider)[0];
    assert_eq!((p.net.as_str(), mv(p)), ("VBAT_SENSE", 2536));
    assert!(p.judged());
    assert_eq!(p.rails, ["battery"]);
    spec(&s02()).validate().unwrap();
}

/// An I2C bus off `mcu.supply` (step 5a refuses it first): both lines are
/// listed at the bus domain's maximum, and they lead.
#[test]
fn an_i2c_bus_off_the_supply_lists_both_pulls() {
    let mut v = s02();
    v["i2c_buses"][0]["domain"] = json!("VDD_5V");
    let rows = of(&v, Class::I2cPull);
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|p| mv(p) == 5250 && leads(p)));
    assert_eq!(spec(&v).validate().unwrap_err().kind(), "SupplyMismatch");
    // On `mcu.supply`, as committed, it is no path.
    assert!(of(&s02(), Class::I2cPull).is_empty());
}

/// A crossing's A side off `mcu.supply` (steps 5a and 16 refuse it first):
/// its bus lines and chip select are listed, and the chip-select pull.
#[test]
fn a_crossing_a_side_off_the_supply_lists_its_lines_and_cs_pull() {
    assert!(of(&s02(), Class::CrossingASide).is_empty());
    let mut v = s02();
    v["power"]["crossings"][0]["a_domain"] = json!("VDD_5V");
    assert_eq!(of(&v, Class::CrossingASide).len(), 4);
    assert_eq!(of(&v, Class::CrossingCsPull).len(), 1);
}

/// A control wire on an MCU net (`1211e` F7: it validates) is named,
/// unbounded: no row states a control pin's level onto another net.
#[test]
fn a_control_wire_on_an_mcu_net_is_unbounded() {
    let mut v = s02();
    v["power"]["regulators"][0]["controls"][1] =
        json!({"pin": "PG", "strap": null, "wire": {"net": "VBAT_SENSE", "role": "pg"}});
    let p = &of(&v, Class::ControlWire)[0];
    // The control pin is powered from its part's input, the battery, not
    // from the rail it makes (adversarial F2).
    assert_eq!(
        (p.net.as_str(), p.rails.as_slice()),
        ("VBAT_SENSE", ["battery".to_owned()].as_slice())
    );
    assert!(leads(p) && matches!(p.bound, Bound::Unbounded(_)) && !p.judged());
    // So `mcu.supply`'s own regulator's control is listed too.
    let mut v = s02();
    v["power"]["regulators"][1]["controls"][0] =
        json!({"pin": "EN", "strap": null, "wire": {"net": "VBAT_SENSE", "role": "en"}});
    assert_eq!(
        of(&v, Class::ControlWire)[0].at,
        "power.regulators[1].controls[0].wire"
    );
}

/// Connectors, test points and a foreign clock are off-board ends: listed,
/// leading, unbounded, never judged (D2: they are named in `verify`'s
/// `twin_limits`, never in `validate`'s output).
#[test]
fn off_board_ends_are_unbounded_and_never_judged() {
    let s03 = fixture("crates/aethelgard-cli/tests/fixtures/sentinel-03.json");
    let all = walk(&s03);
    let n = |c: Class| all.iter().filter(|p| p.class == c).count();
    assert_eq!((n(Class::Connector), n(Class::TestPoint)), (16, 19));
    let ext = fixture("crates/aethelgard-cli/tests/fixtures/hse-external-8.json");
    let clock = &of(&ext, Class::ForeignClock)[0];
    assert_eq!(clock.pin.as_deref(), Some("PH0"));
    for p in all.iter().chain([clock]).filter(|p| p.rails.is_empty()) {
        assert!(
            leads(p) && matches!(p.bound, Bound::Unbounded(_)) && !p.judged(),
            "{p:?}"
        );
    }
    spec(&s03).validate().unwrap();
}

// ------------------------------------------------------------ the exemption

/// Sentinel-02 with its I2C bus on `VDD_1V8` (from `VDD_3V3` by U12):
/// downstream of `mcu.supply`, so exempt; a connector carrying `VDD_1V8`
/// voids it (F8), one carrying `mcu.supply` itself does not.
#[test]
fn downstream_is_exempt_unless_a_connector_carries_the_rail() {
    let mut v = s02();
    v["i2c_buses"][0]["domain"] = json!("VDD_1V8");
    let p = &of(&v, Class::I2cPull)[0];
    assert!(!leads(p) && !p.judged(), "{p:?}");
    let mut carried = v.clone();
    carried["connectors"][0]["pins"][1]["net"] = json!("VDD_1V8");
    let p = &of(&carried, Class::I2cPull)[0];
    assert!(leads(p), "{p:?}");
    assert!(matches!(&p.lead, Lead::Leads(w) if w.contains("connector `J2` carries `VDD_1V8`")));
    // A test point is an off-board end as a connector is (adversarial F3).
    let mut probed = v.clone();
    probed["test_points"] = json!([{"net": "VDD_1V8", "reference": "TP1"}]);
    let p = &of(&probed, Class::I2cPull)[0];
    assert!(
        matches!(&p.lead, Lead::Leads(w) if w.contains("test point `TP1` carries `VDD_1V8`")),
        "{p:?}"
    );
}

/// A rail two regulators below `mcu.supply` (`VDD_1V2` from `VDD_1V8` from
/// `VDD_3V3`) is exempt; a connector carrying the rail between (`VDD_1V8`)
/// voids it, since it can feed `VDD_1V2` with `VDD_3V3` down.
#[test]
fn a_connector_on_a_rail_between_voids_the_exemption() {
    let mut v = s02();
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "VDD_1V2", "nominal_millivolts": 1200, "tolerance_permille": 50}));
    let mut u14 = v["power"]["regulators"][2].clone();
    u14["reference"] = json!("U14");
    u14["input"] = json!("VDD_1V8");
    u14["output"] = json!("VDD_1V2");
    v["power"]["regulators"].as_array_mut().unwrap().push(u14);
    v["i2c_buses"][0]["domain"] = json!("VDD_1V2");
    assert!(!leads(&of(&v, Class::I2cPull)[0]));
    v["connectors"][0]["pins"][1]["net"] = json!("VDD_1V8");
    assert!(leads(&of(&v, Class::I2cPull)[0]));
}

/// EN wired to the PG of `mcu.supply`'s own regulator orders a sibling after
/// it; the same wire to the PG of `mcu.supply`'s parent grants nothing
/// (downstream only, ruled 2026-11-22); a connector carrying the gated rail
/// voids it.
#[test]
fn only_a_pg_at_or_below_the_supply_orders_a_sibling() {
    let wire = |v: &mut Value, reg: usize, pin: usize, name: &str| {
        v["power"]["regulators"][reg]["controls"][pin] =
            json!({"pin": name, "strap": null, "wire": {"net": "PG_3V3", "role": "sequence"}});
    };
    let mut v = s02();
    v["i2c_buses"][0]["domain"] = json!("VDD_5V");
    assert!(leads(&of(&v, Class::I2cPull)[0]), "racing siblings lead");
    // U10 (5 V) EN on U11 (3V3 = mcu.supply) PG.
    wire(&mut v, 0, 0, "EN");
    wire(&mut v, 1, 1, "PG");
    let p = &of(&v, Class::I2cPull)[0];
    assert!(
        matches!(&p.lead, Lead::After(w) if w.contains("`U10`'s EN is wired to `PG_3V3`, `U11`'s PG")),
        "{p:?}"
    );
    let mut carried = v.clone();
    carried["connectors"][0]["pins"][1]["net"] = json!("VDD_5V");
    assert!(leads(&of(&carried, Class::I2cPull)[0]));
    // An off-board end on the gate net itself can raise EN with `VDD_3V3`
    // down, so the gate orders nothing (adversarial F1).
    let mut gate = v.clone();
    gate["test_points"] = json!([{"net": "PG_3V3", "reference": "TP1"}]);
    let p = &of(&gate, Class::I2cPull)[0];
    assert!(
        matches!(&p.lead, Lead::Leads(w) if w.contains("test point `TP1` carries `PG_3V3`")),
        "{p:?}"
    );
    // Now feed 3V3 from 5 V (U10 is mcu.supply's parent) and gate a third
    // buck, U15 to `VDD_5VB`, by U10's PG: that orders nothing.
    let mut v = s02();
    v["power"]["regulators"][1]["input"] = json!("VDD_5V");
    v["voltage_domains"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "VDD_5VB", "nominal_millivolts": 5000, "tolerance_permille": 50}));
    let mut u15 = v["power"]["regulators"][0].clone();
    u15["reference"] = json!("U15");
    u15["output"] = json!("VDD_5VB");
    v["power"]["regulators"].as_array_mut().unwrap().push(u15);
    v["i2c_buses"][0]["domain"] = json!("VDD_5VB");
    wire(&mut v, 3, 0, "EN");
    wire(&mut v, 0, 1, "PG");
    assert!(
        leads(&of(&v, Class::I2cPull)[0]),
        "an upstream PG grants nothing"
    );
    // The same EN on U11's PG (mcu.supply's own) orders it.
    v["power"]["regulators"][0]["controls"][1] =
        json!({"pin": "PG", "strap": "open", "wire": null});
    wire(&mut v, 1, 1, "PG");
    assert!(!leads(&of(&v, Class::I2cPull)[0]));
}

/// A device with two supplies is judged on each: D1's driver on
/// Sentinel-02, its current output from `VDD_1V8` (exempt) alone is exempt;
/// with `VDD_5V` (racing) as its other supply it leads; with `mcu.supply` as
/// its only supply it is no path.
#[test]
fn a_device_with_two_supplies_is_judged_on_each() {
    let base = || {
        let mut v = s02();
        let mut d = d1()["pin_devices"][0].clone();
        d["temperature_celsius"] = Value::Null;
        v["pin_devices"] = json!([d]);
        v
    };
    let current = |supplies: Value| {
        let mut v = base();
        v["pin_devices"][0]["supplies"] = supplies;
        of(&v, Class::PinDeviceCurrentOutput)
    };
    assert!(!leads(&current(json!({"VM": "VDD_1V8"}))[0]));
    let both = current(json!({"VM": "VDD_1V8", "VCC": "VDD_5V"}));
    assert!(leads(&both[0]) && both[0].rails == ["VDD_1V8", "VDD_5V"]);
    let other = current(json!({"VM": "VDD_5V", "VCC": "VDD_1V8"}));
    assert!(leads(&other[0]));
    assert!(current(json!({"VM": "VDD_3V3"})).is_empty());
}

/// An exemption is read from wiring only: no field of the specification
/// declares an order, and a rail with no regulator leads whatever else is
/// declared (D1's `VMOTOR` by connector, no tree).
#[test]
fn a_rail_with_no_regulator_always_leads() {
    let p = &of(&d1(), Class::PinDeviceCurrentOutput)[0];
    assert!(
        matches!(&p.lead, Lead::Leads(w) if w.contains("no regulator in the drawn tree outputs `VMOTOR`"))
    );
}

// ------------------------------------------------------------ the limits

/// Table 10 at VDD = 0, never Table 13: BOOT0 9.0 V; NRST, an FT pin, a TT
/// pin and a pin the solver places 4.0 V. With VDD up, FT is Table 10's
/// `Min(VDD + 4.0 V, 6 V)`, not Table 13's 5.5 V, and TT is 4.0 V, not
/// VDD + 0.3 V.
#[test]
fn the_limits_are_table_10_s_at_vdd_zero() {
    let s = spec(&d1());
    for (pin, want) in [
        (Some("BOOT0"), 9000),
        (Some("NRST"), 4000),
        (Some("PA1"), 4000),
        (Some("PA4"), 4000),
        (None, 4000),
    ] {
        let (limit, cite) = limit_at_zero(&s, pin).unwrap();
        assert_eq!(limit, want, "{pin:?}");
        assert!(
            cite.contains("Table 10") && !cite.contains("Table 13"),
            "{cite}"
        );
    }
    let io = match aethelgard_ir::power_parts::table()
        .unwrap()
        .get("STM32H753ZITx")
        .map(|r| &r.kind)
    {
        Some(aethelgard_ir::power_parts::Kind::Mcu(m)) => &m.io,
        _ => panic!("the MCU row"),
    };
    assert_eq!(io.absolute_maximum(Some("PA1"), 3300).0, 6000);
    assert_eq!(io.absolute_maximum(Some("PA4"), 3300).0, 4000);
    assert_eq!(io.absolute_maximum(Some("BOOT0"), 3300).0, 9000);
    // An unplaced signal takes the least row at any supply: TT's 4.0 V.
    assert_eq!(io.absolute_maximum(None, 3300).0, 4000);
}

// ------------------------------------------------------------ the data

/// Table 10's rows are refused when they do not rate Table 13's families in
/// order, and a current output's `output_max_basis` is required and one of
/// two words.
#[test]
fn table_10_and_the_basis_are_refused_malformed() {
    use aethelgard_ir::parts::{PARTS_JSON, parse_parts};
    use aethelgard_ir::power_parts::{POWER_PARTS_JSON, parse_power_parts};
    let mut t: Value = serde_json::from_str(POWER_PARTS_JSON).unwrap();
    let row = t["parts"]
        .as_array()
        .unwrap()
        .iter()
        .position(|p| p["part"] == "STM32H753ZITx")
        .unwrap();
    t["parts"][row]["kind"]["mcu"]["io"]["absolute_maximum"]["families"]
        .as_array_mut()
        .unwrap()
        .reverse();
    let e = parse_power_parts(&t.to_string()).unwrap_err().to_string();
    assert!(e.contains("absolute_maximum.families"), "{e}");
    let mut p: Value = serde_json::from_str(PARTS_JSON).unwrap();
    let row = p["pin_parts"]
        .as_array()
        .unwrap()
        .iter()
        .position(|r| r["pins"].get("IPROPI").is_some())
        .unwrap();
    p["pin_parts"][row]["pins"]["IPROPI"]["output_max_basis"] = json!("typical");
    let e = parse_parts(&p.to_string()).unwrap_err().to_string();
    assert!(e.contains("output_max_basis"), "{e}");
    p["pin_parts"][row]["pins"]["IPROPI"]
        .as_object_mut()
        .unwrap()
        .remove("output_max_basis");
    let e = parse_parts(&p.to_string()).unwrap_err().to_string();
    assert!(e.contains("output_max_basis"), "{e}");
}
