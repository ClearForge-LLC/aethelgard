//! `AET-WO-4300k`: the readiness rules, each refusal fired by one change to
//! `examples/sentinel-03.json` (or, where the change is a datasheet limit, to
//! a copy of the embedded power parts table) and checked by name, each
//! passing case beside it.

use aethelgard_ir::IrValidationError as E;
use aethelgard_ir::power::{self, Refusal};
use aethelgard_ir::power_parts::parse_power_parts;
use serde_json::{Value, json};

fn base() -> Value {
    power_table::sentinel_03()
}

fn checked(v: &Value) -> power::Checked {
    power::check(&spec(v)).expect("checks").expect("a tree")
}

fn table(t: &Value) -> aethelgard_ir::power_parts::PowerPartsTable {
    parse_power_parts(&t.to_string()).expect("the changed table parses")
}

/// The specification checked against `t`: `Ok` carries the screened ids.
fn against(v: &Value, t: &Value) -> Result<Vec<String>, E> {
    let sensors = aethelgard_ir::parts::table().unwrap();
    power::check_against(&spec(v), &table(t), sensors, &power::output_delays(sensors))
        .map(|c| c.expect("a tree").screened.into_keys().collect())
}

#[test]
fn the_base_specification_passes() {
    assert_eq!(spec(&base()).validate(), Ok(()));
}

fn drop_limit(domain: &str, mv: u32) -> Value {
    json!({"domain": domain, "max_millivolts": mv, "source": "test only: a declared limit"})
}

#[test]
fn drop_limits_must_name_the_mcus_supply() {
    // The base names VDD_3V3 (the MCU's supply) and not VDD_5V.
    let c = checked(&base());
    assert!(c.screened.contains_key("drop_not_judged:VDD_5V"));
    assert!(!c.screened.contains_key("drop_not_judged:VDD_3V3"));

    let mut v = base();
    v["power"]["drop_limits"] = json!([]);
    assert_power(&refused(&v), Refusal::DropLimitUndeclared, "VDD_3V3");

    // A limit on another domain is no limit on the MCU's supply.
    v["power"]["drop_limits"] = json!([drop_limit("VDD_5V", 100)]);
    let err = refused(&v);
    assert_power(&err, Refusal::DropLimitUndeclared, "VDD_3V3");
    assert_eq!(err.kind(), "DropLimitUndeclared");

    // Both declared: it passes, and the 5 V domain is judged now.
    let mut v = base();
    v["power"]["drop_limits"]
        .as_array_mut()
        .unwrap()
        .push(drop_limit("VDD_5V", 100));
    assert_eq!(spec(&v).validate(), Ok(()));
    let c = checked(&v);
    assert!(!c.screened.contains_key("drop_not_judged:VDD_5V"));
}

#[test]
fn a_regulated_domain_nothing_loads_is_refused() {
    let mut v = base();
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["domain"] != "VDD_5V");
    assert_power(&refused(&v), Refusal::RegulatedDomainUnloaded, "VDD_5V");
    // One milliamp is a load: the rail is no longer unloaded.
    let mut v = base();
    for l in v["power"]["loads"].as_array_mut().unwrap() {
        if l["domain"] == "VDD_5V" {
            l["milliamps"] = json!(1);
        }
    }
    assert_eq!(spec(&v).validate(), Ok(()));
}

#[test]
fn a_doubled_pack_over_a_regulators_absolute_maximum_is_refused() {
    let mut v = base();
    v["power"]["battery"]["cells"]["max"] = json!(6);
    assert_power(&refused(&v), Refusal::DoubledPackOverAbsoluteMaximum, "U10");
    // Four cells pass (the base).
    assert_eq!(spec(&base()).validate(), Ok(()));

    // The edge: LMR33630's VIN absolute maximum is 38000 mV, so 2 x 4 x
    // 4750 = 38000 passes, and one millivolt of cell is 8 mV of doubled pack.
    // At 4750 mV cells the hot-plug's I2t is over SF-1206F700-2's melting
    // figure since `AET-WO-4411` added U13's input ceramics
    // (`FuseMeltsOnHotPlug`), so the edge is judged against a table whose
    // fuse melts later, which isolates this rule.
    let lenient = || {
        let mut t = table_json();
        row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
            json!(10_000_000);
        t
    };
    let mut v = base();
    v["power"]["battery"]["cell_max_millivolts"]["value"] = json!(4750);
    against(&v, &lenient()).expect("38000 mV doubled passes");
    v["power"]["battery"]["cell_max_millivolts"]["value"] = json!(4751);
    assert_power(
        &against(&v, &lenient()).expect_err("38008 mV"),
        Refusal::DoubledPackOverAbsoluteMaximum,
        "U10",
    );
    // One millivolt of absolute maximum: at 4750 mV cells (38000 doubled),
    // an absolute maximum of 37999 mV is refused.
    let mut v = base();
    v["power"]["battery"]["cell_max_millivolts"]["value"] = json!(4750);
    let mut t = lenient();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["vin_abs_max_millivolts"]["value"] = json!(37_999);
    assert_power(
        &against(&v, &t).expect_err("37999 mV"),
        Refusal::DoubledPackOverAbsoluteMaximum,
        "U10",
    );
}

#[test]
fn a_battery_input_without_protection_is_refused() {
    let mut v = base();
    v["power"]["input"]["protection"] = Value::Null;
    let err = refused(&v);
    assert_power(&err, Refusal::BatteryInputUnprotected, "J4");
    assert_eq!(err.kind(), "BatteryInputUnprotected");
}

/// The number after `marker` in a refusal's text.
fn number_after(err: &E, marker: &str) -> i64 {
    let text = err.to_string();
    let at = text
        .find(marker)
        .unwrap_or_else(|| panic!("`{marker}` in {text}"))
        + marker.len();
    text[at..]
        .chars()
        .take_while(char::is_ascii_digit)
        .collect::<String>()
        .parse()
        .unwrap_or_else(|_| panic!("a number after `{marker}` in {text}"))
}

fn protection_refused(v: &Value, t: &Value, id: &str, clause: &str) -> E {
    let err = against(v, t).expect_err(clause);
    assert_power(&err, Refusal::ProtectionPartOutOfRange, id);
    err
}

#[test]
fn the_base_protection_computes_a_1905_ma_input_current() {
    let c = checked(&base());
    // The protection's note carries the branch's input current (the figure the
    // clauses below are edged against), iterated once (the power review's
    // F4): 1653 mA before `AET-WO-4411`'s radio module; its 500 mA on a rail
    // of its own (U13, 490 mW at 504 mA, ruled 2026-11-25T21:17Z), U11's
    // loss read at its 749 mA load (570 mW) and the pull-ups raise it to
    // 1905 mA, and the drop through 33 mOhm and the fuse's 9375 uOhm to 81 mV.
    assert!(
        c.conditions.iter().any(|t| t.contains(
            "input current is 1905 mA at the pack's minimum of 9000 mV less the input path's drop"
        )),
        "the protection's input-current note is missing or changed"
    );
}

#[test]
fn the_fet_must_stand_off_twice_the_pack() {
    let v = base();
    // 2 x 4 x 4350 = 34800 mV.
    let mut t = table_json();
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["vds_max_millivolts"]["value"] = json!(34800);
    against(&v, &t).expect("at twice the pack passes");
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["vds_max_millivolts"]["value"] = json!(34799);
    let err = protection_refused(&v, &t, "Q1", "vds");
    assert!(err.to_string().contains("34800"), "{err}");
}

#[test]
fn the_zener_must_hold_the_gate_within_the_fets_rating() {
    let v = base();
    // The zener clamps at VZ max, 12600 mV; the pack's 17400 mV is above it.
    let mut t = table_json();
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["vgs_max_millivolts"]["value"] = json!(12600);
    against(&v, &t).expect("at the zener's maximum passes");
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["vgs_max_millivolts"]["value"] = json!(12599);
    let err = protection_refused(&v, &t, "Q1", "vgs");
    assert!(err.to_string().contains("12600"), "{err}");
}

#[test]
fn the_gate_drive_must_reach_a_stated_rds_on() {
    let v = base();
    // The drive is the lesser of the pack's minimum (3 x 3000 = 9000 mV) less
    // the fuse's drop at the input current (1905 mA x 9375 uOhm, 18 mV),
    // 8982 mV, and the zener's VZ minimum (11400 mV).
    let mut t = table_json();
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["rds_on_max"] = json!([
        {"vgs_millivolts": 8982, "max_milliohms": {"value": 33, "at": row(&mut table_json(), "DMP6023LSS-13")["kind"]["pfet"]["rds_on_max"][0]["max_milliohms"]["at"].clone()}}
    ]);
    against(&v, &t).expect("a stated RDS(on) at the drive passes");
    t["parts"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["part"] == "DMP6023LSS-13")
        .unwrap()["kind"]["pfet"]["rds_on_max"][0]["vgs_millivolts"] = json!(8983);
    let err = protection_refused(&v, &t, "Q1", "drive");
    assert!(err.to_string().contains("8982 mV"), "{err}");

    // Or the zener's VZ minimum is the lower drive: under 4500 mV, no row.
    let mut t = table_json();
    row(&mut t, "MMSZ5242BT1G")["kind"]["zener"]["vz_millivolts"]["min"] = json!(4500);
    against(&v, &t).expect("a VZ minimum at the first stated drive passes");
    row(&mut t, "MMSZ5242BT1G")["kind"]["zener"]["vz_millivolts"]["min"] = json!(4499);
    protection_refused(&v, &t, "Q1", "vz min");
}

#[test]
fn the_fet_must_carry_the_input_current() {
    let v = base();
    let mut t = table_json();
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["id_max_milliamps"]["value"] = json!(1905);
    against(&v, &t).expect("the input current itself passes");
    row(&mut t, "DMP6023LSS-13")["kind"]["pfet"]["id_max_milliamps"]["value"] = json!(1904);
    let err = protection_refused(&v, &t, "Q1", "id");
    assert!(err.to_string().contains("1905 mA"), "{err}");
}

#[test]
fn the_fuse_must_be_rated_for_the_pack_the_current_and_the_heat() {
    let v = base();
    // Voltage: the pack's maximum, 17400 mV.
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["voltage_max_millivolts"]["value"] = json!(17400);
    against(&v, &t).expect("a fuse rated at the pack passes");
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["voltage_max_millivolts"]["value"] = json!(17399);
    let err = protection_refused(&v, &t, "F1", "voltage");
    assert!(err.to_string().contains("17400 mV"), "{err}");

    // Current: at least twice the input current, 2 x 1905 = 3810 mA.
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["rated_milliamps"]["value"] = json!(3810);
    against(&v, &t).expect("twice the input current passes");
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["rated_milliamps"]["value"] = json!(3809);
    let err = protection_refused(&v, &t, "F1", "twice");
    assert!(err.to_string().contains("twice"), "{err}");

    // Heat: the rating kept at 85 C is at least the input current.
    // 7000 x 273 / 1000 = 1911 passes; 7000 x 272 / 1000 = 1904 does not.
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["kept_at_85c_permille"]["value"] = json!(273);
    against(&v, &t).expect("the 85 C current at the input current passes");
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["kept_at_85c_permille"]["value"] = json!(272);
    let err = protection_refused(&v, &t, "F1", "85 C");
    assert!(err.to_string().contains("85"), "{err}");
}

#[test]
fn a_fuse_that_melts_on_hot_plug_is_refused_and_the_bound_is_screened() {
    let mut v = base();
    // The base's fuse passes and screens its bound as fuse-only.
    assert!(
        checked(&v)
            .screened
            .contains_key("fuse_hot_plug_bound_fuse_only:F1")
    );
    v["power"]["input"]["protection"]["fuse"]["part"] = json!("SF-1206F400-2");
    assert_power(&refused(&v), Refusal::FuseMeltsOnHotPlug, "F1");

    // The edge: the bound is refused at or above the melting I2t.
    let v = base();
    let mut t = table_json();
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
        json!(1);
    let err = against(&v, &t).expect_err("melts");
    assert_power(&err, Refusal::FuseMeltsOnHotPlug, "F1");
    let bound = number_after(&err, "puts up to ");
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
        json!(bound);
    assert_power(
        &against(&v, &t).expect_err("at the bound melts"),
        Refusal::FuseMeltsOnHotPlug,
        "F1",
    );
    row(&mut t, "SF-1206F700-2")["kind"]["fuse"]["melting_i2t_milliamp2_seconds"]["value"] =
        json!(bound + 1);
    let screened = against(&v, &t).expect("one over the bound passes");
    assert!(screened.contains(&"fuse_hot_plug_bound_fuse_only:F1".to_owned()));
}

#[test]
fn an_esc_fed_through_the_boards_vbat_copper_puts_the_fuse_in_the_motor_path() {
    let mut v = base();
    v["motor_outputs"][0]["esc"] = json!({"board_vbat": {"source": "x"}});
    assert_power(&refused(&v), Refusal::FuseInMotorPath, "F1");
}

#[test]
fn the_inductor_peak_must_stay_under_the_high_side_limit_minimum() {
    let v = base();
    let mut t = table_json();
    row(&mut t, "LMR33630ADDA")["kind"]["buck"]["high_side_limit_milliamps"]["min"] = json!(2100);
    let err = against(&v, &t).expect_err("peak over");
    assert_power(&err, Refusal::PeakCurrentOverLimit, "U10");
    let peak = number_after(&err, "peaks at ");
    assert!(peak > 2000, "{err}");
    let set = |t: &mut Value, mv: i64| {
        row(t, "LMR33630ADDA")["kind"]["buck"]["high_side_limit_milliamps"]["min"] = json!(mv);
    };
    set(&mut t, peak);
    assert_power(
        &against(&v, &t).expect_err("at the limit"),
        Refusal::PeakCurrentOverLimit,
        "U10",
    );
    set(&mut t, peak + 1);
    // U10 now clears; U11's own peak is lower still.
    against(&v, &t).expect("one under the limit passes");
}

#[test]
fn one_cin_short_of_the_effective_minimum_is_refused() {
    let mut v = base();
    v["power"]["regulators"][0]["external"]
        .as_array_mut()
        .unwrap()
        .retain(|e| e["reference"] != "C60");
    assert_power(
        &refused(&v),
        Refusal::EffectiveCapacitanceBelowMinimum,
        "U10",
    );
}

// ---- the reset supervisor (`AET-WO-4300k` (9)) ----

use aethelgard_ir::supervisor;

#[path = "common/power_table.rs"]
mod power_table;
use power_table::{assert_power, refused, row, spec, table_json};

fn without_supervisor(v: &mut Value) {
    v["mcu"].as_object_mut().unwrap().remove("reset_supervisor");
}

fn supervisor_check(v: &Value, t: &Value) -> Result<Vec<(String, String)>, E> {
    let s = spec(v);
    let c = power::check(&s).expect("checks").expect("a tree");
    supervisor::check_against(&s, Some(&c), &table(t))
}

fn assert_supervisor(r: Result<Vec<(String, String)>, E>, kind: Refusal, id: &str) {
    assert_power(&r.expect_err("refused"), kind, id);
}

fn set_falling(t: &mut Value, min: i64) {
    row(t, "TLV803SDBZR")["kind"]["supervisor"]["falling_millivolts"]["min"] = json!(min);
}

#[test]
fn the_base_supervisor_leaves_no_device_above_the_reset() {
    assert_eq!(supervisor::floors_above_reset(&spec(&base())).unwrap(), []);
    // The declared supervisor is judged, and passes.
    supervisor_check(&base(), &table_json()).expect("the base's supervisor passes");
}

#[test]
fn without_a_supervisor_the_flash_is_above_bor3() {
    let mut v = base();
    without_supervisor(&mut v);
    let s = spec(&v);
    let (floor, why) = supervisor::reset_floor(&s).unwrap();
    assert_eq!(floor, 2540);
    assert!(why.contains("BOR level 3"), "{why}");
    let floors = supervisor::floors_above_reset(&s).unwrap();
    assert_eq!(floors.len(), 1, "{floors:?}");
    let f = &floors[0];
    assert_eq!((f.device.as_str(), f.part.as_str()), ("U4", "W25Q64JVxIQ"));
    assert_eq!((f.vcc_min_millivolts, f.reset_min_millivolts), (2700, 2540));
    assert!(
        f.text()
            .contains("2700 mV is above the reset's falling minimum of 2540 mV")
    );
    // With no supervisor, `check` has nothing to judge.
    assert_eq!(supervisor::check(&s, None).unwrap(), []);
}

#[test]
fn a_wide_range_device_set_is_under_bor3() {
    // U3, U5 and U7 hold down to 1.71 to 1.8 V; the flash is the only one
    // above BOR3, so without it (Path A) nothing is.
    let mut v = base();
    without_supervisor(&mut v);
    v["spi_devices"]
        .as_array_mut()
        .unwrap()
        .retain(|d| d["reference"] != "U4");
    let s = spec(&v);
    assert_eq!(supervisor::floors_above_reset(&s).unwrap(), []);
    // The three that remain are on the MCU's supply and counted.
    let all: Vec<_> = s
        .declared_i2c_devices()
        .iter()
        .map(|d| d.reference.clone())
        .chain(s.declared_spi_devices().iter().map(|d| d.reference.clone()))
        .collect();
    assert!(
        ["U3", "U5", "U7"]
            .iter()
            .all(|r| all.iter().any(|a| a == r)),
        "{all:?}"
    );
}

#[test]
fn a_supervisor_falling_below_a_devices_minimum_is_refused() {
    let mut v = base();
    without_supervisor(&mut v);
    let flash = supervisor::floors_above_reset(&spec(&v)).unwrap()[0].vcc_min_millivolts;
    assert_eq!(flash, 2700);
    let mut t = table_json();
    // At the flash's minimum it passes: the rule is strictly above.
    set_falling(&mut t, 2700);
    supervisor_check(&base(), &t).expect("falling minimum equal to the device's passes");
    set_falling(&mut t, 2699);
    assert_supervisor(
        supervisor_check(&base(), &t),
        Refusal::SupervisorBelowDeviceFloor,
        "U12",
    );
    set_falling(&mut t, 2600);
    assert_supervisor(
        supervisor_check(&base(), &t),
        Refusal::SupervisorBelowDeviceFloor,
        "U12",
    );
}

#[test]
fn a_supervisor_that_releases_at_the_rails_minimum_is_refused() {
    // VDD_3V3's solved minimum is 3144 mV; less the drop limit it is the
    // rail's minimum at the pin. The supervisor releases at 2990 + 40 = 3030.
    let limit = |v: &mut Value, mv: u32| v["power"]["drop_limits"][0]["max_millivolts"] = json!(mv);
    let mut v = base();
    // 3144 - 114 = 3030: at the release, refused; one millivolt less, passes.
    limit(&mut v, 113);
    supervisor_check(&v, &table_json()).expect("the release one millivolt under the rail passes");
    limit(&mut v, 114);
    assert_supervisor(
        supervisor_check(&v, &table_json()),
        Refusal::SupervisorAboveRail,
        "U12",
    );
    limit(&mut v, 150);
    // `validate` runs the same judgement.
    assert_power(&refused(&v), Refusal::SupervisorAboveRail, "U12");
    assert_supervisor(
        supervisor_check(&v, &table_json()),
        Refusal::SupervisorAboveRail,
        "U12",
    );
}

#[test]
fn a_supervisors_range_and_pull_up_are_judged() {
    // Pull-up: 10 kOhm to 1 MOhm inclusive.
    let pull = |ohms: u32| {
        let mut v = base();
        v["mcu"]["reset_supervisor"]["pull_up"]["ohms"] = json!(ohms);
        supervisor_check(&v, &table_json())
    };
    pull(10_000).expect("the lower end passes");
    pull(1_000_000).expect("the upper end passes");
    assert_supervisor(pull(5000), Refusal::SupervisorOutOfRange, "R61");
    assert_supervisor(pull(9999), Refusal::SupervisorOutOfRange, "R61");
    assert_supervisor(pull(1_000_001), Refusal::SupervisorOutOfRange, "R61");

    // Supply: the rail's minimum at the pin (3144 - 110 = 3034 mV) is the
    // lowest the supervisor must run from.
    let vdd_min = |mv: i64| {
        let mut t = table_json();
        row(&mut t, "TLV803SDBZR")["kind"]["supervisor"]["vdd_millivolts"]["min"] = json!(mv);
        supervisor_check(&base(), &t)
    };
    vdd_min(3034).expect("a supply minimum at the rail's passes");
    assert_supervisor(vdd_min(3035), Refusal::SupervisorOutOfRange, "U12");
}

#[test]
fn the_supervisor_returns_its_conditions_by_id() {
    let got = supervisor_check(&base(), &table_json()).unwrap();
    let ids: Vec<&str> = got.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "supervisor_delay_unstated:U12",
            "supervisor_hysteresis_typical:U12"
        ]
    );
    // SBVS157E p. 6: the delay is stated from VIT- max + 200 mV, 3190 mV,
    // above the rail's 3034 mV at the pin.
    assert!(got[0].1.contains("at least 3190 mV"), "{}", got[0].1);
    assert!(got[0].1.contains("3034 mV"), "{}", got[0].1);
    assert!(got[1].1.contains("3030 mV"), "{}", got[1].1);
}

#[test]
fn a_part_holds_its_supply_by_the_clock_it_is_read_at() {
    let parts = aethelgard_ir::parts::table().unwrap();
    let flash = parts.get("W25Q64JVxIQ").unwrap();
    assert_eq!(flash.supply_min_at(20_000_000), Some(2700));
    assert_eq!(flash.supply_min_at(104_000_000), Some(2700));
    assert_eq!(flash.supply_min_at(104_000_001), Some(3000));
    assert_eq!(flash.supply_min_at(120_000_000), Some(3000));
    assert_eq!(flash.supply_min_at(133_000_000), Some(3000));
    assert_eq!(flash.supply_min_at(133_000_001), None);
    assert_eq!(flash.supply_min_at(200_000_000), None);
    // A part with no clocked rows holds its supply minimum at any clock.
    let sensor = parts.get("TMP117").unwrap();
    assert_eq!(sensor.supply_min_at(1_000_000), Some(1800));
    assert_eq!(sensor.supply_min_at(u64::MAX), Some(1800));
}

// ---- the fabricator and what each stackup bound cites (`AET-WO-4300k` (b)) ----

fn stackup(v: &mut Value) -> &mut Value {
    &mut v["board"]["stackup"]
}

#[test]
fn a_bound_with_no_citation_is_refused_by_its_path() {
    for (path, edit) in [
        (
            "board.stackup.finished_thickness_um",
            "finished_thickness_um",
        ),
        ("board.stackup.copper_max_um", "copper_max_um"),
        (
            "board.stackup.fab_tolerances.via_ring_um",
            "fab_tolerances.via_ring_um",
        ),
    ] {
        let mut v = base();
        let mut at = stackup(&mut v);
        for k in edit.split('.') {
            at = &mut at[k];
        }
        at.as_object_mut().unwrap().remove("cites");
        match refused(&v) {
            E::StackupSourceUncited { path: p } => assert_eq!(p, path),
            other => panic!("{path}: {other}"),
        }
    }
    let mut v = base();
    stackup(&mut v)["dielectrics"][1]
        .as_object_mut()
        .unwrap()
        .remove("cites");
    let err = refused(&v);
    assert_eq!(err.kind(), "StackupSourceUncited");
    match err {
        E::StackupSourceUncited { path } => assert_eq!(path, "board.stackup.dielectrics[1]"),
        other => panic!("{other}"),
    }
}

#[test]
fn a_cite_of_a_document_the_fabricator_lacks_is_foreign() {
    let mut v = base();
    stackup(&mut v)["finished_thickness_um"]["cites"] = json!({"fabricator": "nope"});
    let err = refused(&v);
    assert_eq!(err.kind(), "StackupSourceForeign");
    match err {
        E::StackupSourceForeign {
            path,
            cited,
            fabricator,
        } => {
            assert_eq!(path, "board.stackup.finished_thickness_um.cites");
            assert_eq!((cited.as_str(), fabricator.as_str()), ("nope", "OSH Park"));
        }
        other => panic!("{other}"),
    }
    // The same cite stands once the fabricator holds a document by that id.
    stackup(&mut v)["fabricator"]["documents"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": "nope", "title": "a second sheet", "url": "https://example.test/a", "read": "2026-11-24"}));
    assert_eq!(spec(&v).validate(), Ok(()));
}

#[test]
fn a_fabricator_cite_with_no_fabricator_is_refused() {
    let mut v = base();
    stackup(&mut v)
        .as_object_mut()
        .unwrap()
        .remove("fabricator");
    let err = refused(&v);
    assert_eq!(err.kind(), "StackupValue");
    match err {
        E::StackupValue { path, rule } => {
            assert_eq!(path, "board.stackup.finished_thickness_um.cites");
            assert!(rule.contains("declares none"), "{rule}");
        }
        other => panic!("{other}"),
    }
}

#[test]
fn the_stackups_conditions_are_listed_by_id() {
    let s = spec(&base());
    let got = aethelgard_ir::board::conditions(&s.board.stackup);
    let ids: Vec<&str> = got.iter().map(|(id, _)| id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "stackup_condition:copper-max-2oz",
            "stackup_condition:permittivity-fr4-max",
            "stackup_condition:outer-copper-min",
            "stackup_condition:plane-copper-min",
            "stackup_condition:plating-min",
            "stackup_condition:copper-temperature-max",
            "stackup_condition:track-width-tolerance",
        ]
    );
    assert!(
        got[0].1.contains("board.stackup.copper_max_um"),
        "{}",
        got[0].1
    );
    // A bound that cites a fabricator's document is no condition.
    let mut v = base();
    stackup(&mut v)["plating_min_um"]["cites"] = json!({"fabricator": "oshpark-four-layer"});
    let s = spec(&v);
    assert_eq!(aethelgard_ir::board::conditions(&s.board.stackup).len(), 6);
}

// ---- acknowledged conditions (`AET-WO-4300k` (d)) ----

fn acknowledged(v: &mut Value, id: &str) {
    v["acknowledge_conditions"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id": id, "reason": "a test reason, no more"}));
}

#[test]
fn a_condition_id_is_kind_colon_reference() {
    let mut v = base();
    acknowledged(&mut v, "kind_2:U1");
    assert_eq!(spec(&v).validate(), Ok(()));
    for bad in [
        "nocolon", "Kind:U1", "kInd:U1", "ki-nd:U1", ":U1", "kind:", "kind:a b",
    ] {
        let mut v = base();
        acknowledged(&mut v, bad);
        let err = refused(&v);
        assert_eq!(err.kind(), "ConditionAcknowledgementMalformed", "{bad}");
        match err {
            E::ConditionAcknowledgementMalformed { path, .. } => {
                assert!(path.starts_with("acknowledge_conditions["), "{path}");
                assert!(path.ends_with(".id"), "{path}");
            }
            other => panic!("{bad}: {other}"),
        }
    }
}

#[test]
fn a_condition_is_acknowledged_once() {
    let mut v = base();
    let first = v["acknowledge_conditions"][0].clone();
    v["acknowledge_conditions"]
        .as_array_mut()
        .unwrap()
        .push(first);
    let n = v["acknowledge_conditions"].as_array().unwrap().len() - 1;
    match refused(&v) {
        E::ConditionAcknowledgementMalformed { path, rule } => {
            assert_eq!(path, format!("acknowledge_conditions[{n}].id"));
            assert!(rule.contains("once"), "{rule}");
        }
        other => panic!("{other}"),
    }
}

// ---- the buck junction screened at the copper extract measures (`AET-WO-4300k` (a)) ----

fn screen(reference: &str, mm2: u64) -> Result<String, E> {
    power::junction_screen(&spec(&base()), reference, mm2)
}

#[test]
fn u10_needs_15_cm2_of_copper_per_layer() {
    // 1170 mW at 36 K/W (the figure's 10 cm2 point) on an 85 C board: 127.1 C,
    // over the part's 125 C; at 30 K/W (15 cm2) it is 120.1 C.
    let err = screen("U10", 1499).expect_err("14.99 cm2 reads as the 10 cm2 point");
    assert_power(&err, Refusal::BuckJunctionOverLimit, "U10");
    assert!(err.to_string().contains("127120 m°C"), "{err}");
    screen("U10", 1500).expect("15 cm2 clears");
    screen("U10", 5000).expect("more copper clears");
}

#[test]
fn u11_needs_7_cm2_to_be_on_the_figure_at_all() {
    let err = screen("U11", 699).expect_err("under the figure's smallest area");
    assert_power(&err, Refusal::BuckJunctionOverLimit, "U11");
    assert!(err.to_string().contains("6.99 cm"), "{err}");
    screen("U11", 700).expect("7 cm2 clears");
}

#[test]
fn the_screen_refuses_what_is_not_a_buck_with_a_loss() {
    assert_eq!(
        screen("R10", 5000).expect_err("no such regulator").kind(),
        "PowerValueInvalid"
    );
}

// ---- A13: the load line and the loss gap ----

#[test]
fn every_device_named_in_loads_leaves_no_uncounted_supply_line() {
    let c = checked(&base());
    let uncounted: Vec<_> = c
        .screened
        .keys()
        .filter(|k| k.starts_with("supply_current_not_counted:"))
        .collect();
    assert!(uncounted.is_empty(), "{uncounted:?}");
}

#[test]
fn a_device_whose_load_names_no_reference_is_the_only_one_listed() {
    let mut v = base();
    for l in v["power"]["loads"].as_array_mut().unwrap() {
        if l["reference"] == "U3" {
            l.as_object_mut().unwrap().remove("reference");
        }
    }
    let c = checked(&v);
    let lines: Vec<_> = c
        .screened
        .iter()
        .filter(|(k, _)| k.starts_with("supply_current_not_counted:"))
        .collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    let (id, text) = lines[0];
    assert_eq!(id, "supply_current_not_counted:VDD_3V3");
    assert!(
        text.contains("the supply currents of U3 are not in its load"),
        "{text}"
    );
    for other in ["U4", "U5", "U7"] {
        assert!(!text.contains(other), "{text}");
    }
}

#[test]
fn a_loss_read_below_the_solved_load_is_screened_as_a_gap() {
    // Since `AET-WO-4411` Sentinel-03 reads U11's loss at its solved load
    // (749 mA, the radio module on its own rail since 2026-11-25T21:17Z), so
    // no gap is screened; one milliamp under it is.
    let c = checked(&base());
    assert!(!c.screened.contains_key("loss_current_gap:U11"));
    assert!(!c.screened.contains_key("loss_current_gap:U10"));
    let solved = c.domains["VDD_3V3"].load_milliamps;
    assert_eq!(solved, 749);
    let mut v = base();
    v["power"]["regulators"][1]["junction"]["loss_at_milliamps"] = json!(solved - 70);
    let gap = &checked(&v).screened["loss_current_gap:U11"];
    assert!(gap.contains("a gap of 70 mA"), "{gap}");
    let at = |mv: u64| {
        let mut v = base();
        v["power"]["regulators"][1]["junction"]["loss_at_milliamps"] = json!(mv);
        checked(&v).screened.contains_key("loss_current_gap:U11")
    };
    assert!(at(solved - 1));
    assert!(!at(solved));
    assert!(!at(solved + 1));
}

// ---- the crystal's unmeasured conditions ----

#[test]
fn a_crystal_leaves_two_unmeasured_conditions_and_none_leaves_none() {
    let s = spec(&base());
    let ids: Vec<String> = aethelgard_ir::clock::conditions(s.mcu.hse.as_ref())
        .into_iter()
        .map(|(id, _)| id)
        .collect();
    assert_eq!(
        ids,
        [
            "hse_safety_factor_unmeasured:HSE",
            "hse_drive_level_unmeasured:HSE"
        ]
    );
    let mut v = base();
    v["mcu"]["hse"] = Value::Null;
    let s = spec(&v);
    assert!(s.mcu.hse.is_none());
    assert_eq!(aethelgard_ir::clock::conditions(s.mcu.hse.as_ref()), []);
}

// ---- the mating opening of every connector row ----

#[test]
fn every_connector_row_has_an_opening_toward_a_side() {
    use aethelgard_ir::connectors::{self, Toward};
    for p in connectors::PARTS.iter() {
        let o = connectors::opening(p.name).unwrap_or_else(|| panic!("{} has none", p.name));
        assert_eq!(o, p.opening);
        assert_eq!(o.toward, Toward::PlusY, "{}", p.name);
        assert!(
            !o.cited.section.is_empty() && o.cited.sha256.len() == 64,
            "{}",
            p.name
        );
    }
    for p in connectors::POWER_IN.iter() {
        let o = connectors::opening(p.name).unwrap_or_else(|| panic!("{} has none", p.name));
        assert_eq!(o, p.opening);
        assert!(
            !o.cited.section.is_empty() && o.cited.sha256.len() == 64,
            "{}",
            p.name
        );
    }
    assert_eq!(
        connectors::opening("XT30PW-M").unwrap().toward,
        Toward::MinusY
    );
    assert!(connectors::opening("NO-SUCH-PART").is_none());
}

/// The power review's F1: an undeclared ESC on a protected input is refused
/// by name (`INV-06`), as `board_vbat` is; `none` with its source passes.
#[test]
fn an_undeclared_esc_on_a_protected_input_is_refused() {
    let mut v = base();
    v["motor_outputs"][2]["esc"] = Value::Null;
    assert_power(&refused(&v), Refusal::EscUndeclared, "MOTOR3");
    v["motor_outputs"][2]["esc"] = json!({"board_vbat": {"source": "test"}});
    assert_power(&refused(&v), Refusal::FuseInMotorPath, "F1");
}

/// The power review's F5: the fuse row holds its derating at 85 °C only, so
/// a hotter board is refused by the fuse (every other part's limit raised in
/// the copy, so the fuse decides); 85 °C passes.
#[test]
fn a_board_above_the_fuses_held_derating_is_refused() {
    let mut t = table_json();
    for r in t["parts"].as_array_mut().unwrap() {
        if r["temperature_celsius"]["max"]
            .as_i64()
            .is_some_and(|max| max < 125)
        {
            r["temperature_celsius"]["max"] = json!(125);
        }
    }
    let mut v = base();
    v["power"]["conditions"]["board_temperature_celsius"]["max"] = json!(86);
    let err = against(&v, &t).expect_err("86 C");
    assert_power(&err, Refusal::ProtectionPartOutOfRange, "F1");
    assert!(err.to_string().contains("at 85 °C only"), "{err}");
    against(&base(), &t).expect("85 C passes");
}

/// The power review's F2, F3, F6 (b) and F7: each assumption the protection
/// and the junction screen rest on is named by id where it holds, and only
/// there.
#[test]
fn the_protections_and_junctions_assumptions_are_named_where_they_hold() {
    let ids = |v: &Value, t: &Value| against(v, t).unwrap();
    let base_ids = ids(&base(), &table_json());
    for id in [
        "input_path_resistance_at_25c:Q1",
        "inrush_ceramic_at_rated:C11",
        "inrush_ceramic_at_rated:C21",
        "theta_ja_copper_mismatch:U10",
        "theta_ja_copper_mismatch:U11",
    ] {
        assert!(base_ids.iter().any(|i| i == id), "{id}: {base_ids:?}");
    }
    // A 10 uF input ceramic holds its tolerance in its DC-bias block, so it
    // is counted at +10 %, not named.
    assert!(!base_ids.iter().any(|i| i == "inrush_ceramic_at_rated:C10"));
    // The gate drive is the pack's (8984 mV), not the zener's.
    assert!(
        !base_ids
            .iter()
            .any(|i| i.starts_with("gate_drive_at_zener:"))
    );
    let mut t = table_json();
    row(&mut t, "MMSZ5242BT1G")["kind"]["zener"]["vz_millivolts"]["min"] = json!(8000);
    assert!(
        ids(&base(), &t)
            .iter()
            .any(|i| i == "gate_drive_at_zener:Q1")
    );
    // The figure's own copper, 70 um outer and 35 um inner, names nothing.
    let mut v = base();
    v["board"]["stackup"]["outer_copper_min_um"]["value"] = json!(70);
    v["board"]["stackup"]["plane_copper_min_um"]["value"] = json!(35);
    assert!(
        !ids(&v, &table_json())
            .iter()
            .any(|i| i.starts_with("theta_ja_copper_mismatch:"))
    );
    v["board"]["stackup"]["plane_copper_min_um"]["value"] = json!(34);
    assert!(
        ids(&v, &table_json())
            .iter()
            .any(|i| i == "theta_ja_copper_mismatch:U10")
    );
    let c = checked(&base());
    assert!(
        c.screened["input_path_resistance_at_25c:Q1"].contains("81 mV at 1905 mA"),
        "{}",
        c.screened["input_path_resistance_at_25c:Q1"]
    );
}
