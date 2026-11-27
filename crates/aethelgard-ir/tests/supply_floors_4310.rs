//! `AET-WO-4310` (2): every powered part judged against its own rail's
//! monitor (ruled D1 (B)), the MCU at VBOR0's minimum (D2), a translator by
//! its operating range (D3 (ii)), and a UART crossing's lines judged again at
//! the reset's release floor (ruled 2 (a)). Each branch has its planted case
//! and its clean twin.

use aethelgard_ir::power::{self, Refusal};
use aethelgard_ir::power_parts::parse_power_parts;
use aethelgard_ir::supervisor::{self, Judgement};
use serde_json::{Value, json};

#[path = "common/power_table.rs"]
mod power_table;
use power_table::{assert_power, row, sentinel_03, spec, table_json};

fn example(path: &str) -> Value {
    let p = format!("{}/../../{path}", env!("CARGO_MANIFEST_DIR"));
    serde_json::from_str(&std::fs::read_to_string(&p).expect(path)).expect("json")
}

/// Each device's judgement, by its first letter: P pass, F floor, N named,
/// E elsewhere.
fn verdicts(v: &Value) -> Vec<(String, char)> {
    supervisor::judged(&spec(v))
        .unwrap()
        .into_iter()
        .map(|(p, j)| {
            let c = match j {
                Judgement::Pass => 'P',
                Judgement::Floor(_) => 'F',
                Judgement::Named(_) => 'N',
                Judgement::Elsewhere(_) => 'E',
            };
            (p.device, c)
        })
        .collect()
}

fn verdict_of(v: &Value, device: &str) -> char {
    verdicts(v)
        .into_iter()
        .find(|(d, _)| d == device)
        .unwrap_or_else(|| panic!("{device} is not a powered part"))
        .1
}

#[test]
fn sentinel_03_judges_every_powered_part_by_its_rails_monitor() {
    let v = sentinel_03();
    let got = verdicts(&v);
    let want = [
        ("MCU", 'P'),
        ("U3", 'P'),
        ("U7", 'P'),
        ("U4", 'P'),
        ("U5", 'P'),
        ("U30", 'N'),
        ("U12", 'P'),
        ("U31.A", 'P'),
        ("U31.B", 'N'),
        ("U10", 'E'),
        ("U11", 'E'),
        ("U13", 'E'),
    ];
    assert_eq!(
        got,
        want.iter()
            .map(|(d, c)| ((*d).to_owned(), *c))
            .collect::<Vec<_>>()
    );
    assert_eq!(supervisor::floors_above_reset(&spec(&v)).unwrap(), []);
    let named = supervisor::named(&spec(&v)).unwrap();
    assert_eq!(
        named.iter().map(|n| n.device.as_str()).collect::<Vec<_>>(),
        ["U30", "U31.B"]
    );
    for n in &named {
        assert_eq!(n.rail, "VDD_3V3_RF");
        assert!(n.text().contains("held by no monitor"), "{}", n.text());
    }
    // U31's A side by its operating range (SCES935A p. 6, 7.3), not the
    // 3000 mV band its timing row is stated over (ruled D3 (ii)).
    let a = supervisor::powered(&spec(&v))
        .unwrap()
        .into_iter()
        .find(|p| p.device == "U31.A")
        .unwrap();
    let (min, cite) = a.min.unwrap();
    assert_eq!(min, 1080);
    assert!(cite.contains("p. 6, 7.3"), "{cite}");
}

#[test]
fn the_same_module_on_the_mcus_rail_is_a_floor_not_named() {
    let mut v = sentinel_03();
    v["radio_modules"][0]["supply"] = json!("VDD_3V3");
    // Its 3000 mV is above U12's 2870 mV falling minimum.
    assert_eq!(verdict_of(&v, "U30"), 'F');
    let f = &supervisor::floors_above_reset(&spec(&v)).unwrap()[0];
    assert_eq!((f.vcc_min_millivolts, f.reset_min_millivolts), (3000, 2870));
}

#[test]
fn the_mcu_is_judged_at_vbor0s_minimum_so_bor0_raises_no_false_floor() {
    // DS12117 Table 111 note 1: functionality is guaranteed down to VBOR0
    // min once the reset is released, so the MCU's 1620 mV row is not its
    // floor (ruled D2). At BOR level 0 the floor equals its own reset, and
    // the rule is strictly above: it passes.
    let v = example("examples/sentinel-02-bench.json");
    assert_eq!(v["mcu"]["bor_level"], 0);
    assert_eq!(verdict_of(&v, "MCU"), 'P');
    let mcu = supervisor::powered(&spec(&v))
        .unwrap()
        .into_iter()
        .find(|p| p.device == "MCU")
        .unwrap();
    let (min, cite) = mcu.min.unwrap();
    assert_eq!(min, 1580);
    assert!(cite.contains("Table 111 note 1"), "{cite}");
    // Its bus devices stay floors, as before (1710 > 1580).
    let floors: Vec<String> = supervisor::floors_above_reset(&spec(&v))
        .unwrap()
        .into_iter()
        .map(|f| f.device)
        .collect();
    assert_eq!(floors, ["U7", "U5"]);
}

#[test]
fn a_floor_on_the_mcus_rail_is_listed_and_a_clean_twin_passes() {
    // Sentinel-01: the flash's 2700 mV above BOR3's 2540 (4300k (9)).
    let v = example("examples/sentinel-01.json");
    assert_eq!(verdict_of(&v, "U4"), 'F');
    assert!(supervisor::named(&spec(&v)).unwrap().is_empty());
    // `sentinel-01-ready`, with its TLV803S: every part passes.
    let ready = example("crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json");
    assert!(
        supervisor::floors_above_reset(&spec(&ready))
            .unwrap()
            .is_empty()
    );
    assert!(supervisor::named(&spec(&ready)).unwrap().is_empty());
    assert!(verdicts(&ready).iter().all(|(_, c)| *c == 'P'));
}

#[test]
fn a_part_with_no_held_minimum_is_named_never_passed() {
    // A pin device: the pin-parts rows hold no supply minimum.
    let v = example("crates/aethelgard-ir/tests/fixtures/d1-drv8876.json");
    let named = supervisor::named(&spec(&v)).unwrap();
    let u2: Vec<_> = named.iter().filter(|n| n.device == "U2").collect();
    assert!(!u2.is_empty(), "{named:?}");
    assert!(
        u2.iter()
            .all(|n| n.why.contains("no supply minimum is held"))
    );
    // A bus device whose part has no row is named, never skipped.
    let mut s1 = example("examples/sentinel-01.json");
    s1["i2c_devices"][0]["part"] = json!("NO-SUCH-PART");
    assert_eq!(verdict_of(&s1, "U2"), 'N');
}

#[test]
fn a_regulator_fed_from_a_rail_is_judged_and_one_fed_from_the_battery_is_named_elsewhere() {
    // sentinel-02-power: U12 (TLV75518) takes 2227 mV at least on VDD_3V3,
    // under BOR3's 2540: it passes. U5 and U13's B side are on VDD_1V8,
    // which no monitor holds.
    let v = example("crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    assert_eq!(verdict_of(&v, "U12"), 'P');
    assert_eq!(verdict_of(&v, "U13.A"), 'P');
    assert_eq!(verdict_of(&v, "U5"), 'N');
    assert_eq!(verdict_of(&v, "U13.B"), 'N');
    assert_eq!(verdict_of(&v, "U10"), 'E');
    assert_eq!(verdict_of(&v, "U11"), 'E');
    match supervisor::judged(&spec(&v))
        .unwrap()
        .into_iter()
        .find(|(p, _)| p.device == "U10")
        .unwrap()
        .1
    {
        Judgement::Elsewhere(why) => assert!(why.contains("headroom"), "{why}"),
        other => panic!("{other:?}"),
    }
}

// ---- the release floor (ruled 2 (a)) ----

fn against(v: &Value, t: &Value) -> Result<Vec<String>, aethelgard_ir::IrValidationError> {
    let sensors = aethelgard_ir::parts::table().unwrap();
    let table = parse_power_parts(&t.to_string()).expect("the changed table parses");
    power::check_against(&spec(v), &table, sensors, &power::output_delays(sensors))
        .map(|c| c.expect("a tree").conditions.into_iter().collect())
}

fn release_notes(conditions: &[String]) -> Vec<&String> {
    conditions
        .iter()
        .filter(|c| c.contains("at the reset's release floor of 2870 mV, holds"))
        .collect()
}

#[test]
fn sentinel_03s_crossing_holds_at_the_release_floor_with_the_lower_band_named() {
    let c = against(&sentinel_03(), &table_json()).expect("Sentinel-03 passes");
    let notes = release_notes(&c);
    assert_eq!(notes.len(), 2, "{c:?}");
    let tx = notes.iter().find(|n| n.contains("`WIFI_TX`")).unwrap();
    assert!(
        tx.contains("between the 2.3 V and 3 V rows, the worse of the two"),
        "{tx}"
    );
    let rx = notes.iter().find(|n| n.contains("`WIFI_RX`")).unwrap();
    assert!(!rx.contains("worse of the two"), "{rx}");
}

#[test]
fn a_release_floor_under_the_lowest_stated_band_is_refused_by_name() {
    // U12 falling at 2400 mV: 2400 less VDD_3V3's 110 mV is 2290 mV at
    // U31's pins, under the 2.3 V row, where nothing is stated.
    let mut t = table_json();
    row(&mut t, "TLV803SDBZR")["kind"]["supervisor"]["falling_millivolts"]["min"] = json!(2400);
    let e = against(&sentinel_03(), &t).expect_err("refused");
    assert_power(&e, Refusal::LineLevelUnmet, "U31");
    assert!(e.to_string().contains("nothing is stated below it"), "{e}");
}

#[test]
fn without_the_lower_band_the_release_floor_is_refused() {
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["levels"]["lower_band"] = Value::Null;
    let e = against(&sentinel_03(), &t).expect_err("refused");
    assert_power(&e, Refusal::LineLevelUnmet, "U31");
    assert!(e.to_string().contains("3000 mV"), "{e}");
}

#[test]
fn a_lower_band_threshold_the_mcu_cannot_clear_at_the_release_floor_is_refused() {
    // VT+ at 980 per mille of 2.76 V is about 2.70 V, over the MCU's VOH
    // there. The band must still be no better than the stated rows.
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["levels"]["lower_band"]["input_high_min_permille"]
        ["value"] = json!(980);
    let e = against(&sentinel_03(), &t).expect_err("refused");
    assert_power(&e, Refusal::LineLevelUnmet, "U31");
    assert!(
        e.to_string()
            .contains("at the reset's release floor of 2870 mV"),
        "{e}"
    );
}

#[test]
fn a_lower_band_better_than_the_stated_rows_or_not_meeting_them_is_refused() {
    for (field, value) in [
        ("input_high_min_permille", 600),
        ("input_low_max_permille", 300),
    ] {
        let mut t = table_json();
        row(&mut t, "TXU0304PW")["kind"]["translator"]["levels"]["lower_band"][field]["value"] =
            json!(value);
        let e = parse_power_parts(&t.to_string()).expect_err(field);
        assert!(e.to_string().contains("no better than it"), "{field}: {e}");
    }
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["levels"]["lower_band"]["input_high_min_permille"]
        ["value"] = json!(1000);
    let e = parse_power_parts(&t.to_string()).expect_err("VT+ at the whole supply");
    assert!(e.to_string().contains("no better than it"), "{e}");
    let mut t = table_json();
    row(&mut t, "TXU0304PW")["kind"]["translator"]["levels"]["lower_band"]["supply_millivolts"]["max"] =
        json!(2900);
    let e = parse_power_parts(&t.to_string()).expect_err("a gap at 2.9 V");
    assert!(e.to_string().contains("must end where"), "{e}");
}

#[test]
fn the_radio_line_names_a_module_on_a_rail_no_monitor_holds() {
    let lines = aethelgard_ir::radio::not_judged(&spec(&sentinel_03()));
    assert_eq!(lines.len(), 1);
    assert!(
        lines[0].contains("its VDD minimum of 3000 mV")
            && lines[0].contains("held by no monitor: nothing holds `VDD_3V3_RF`'s consumers")
            && !lines[0].contains("above the reset's falling minimum"),
        "{}",
        lines[0]
    );
    // On the MCU's rail it is compared with the reset, as before.
    let mut v = sentinel_03();
    v["radio_modules"][0]["supply"] = json!("VDD_3V3");
    let lines = aethelgard_ir::radio::not_judged(&spec(&v));
    assert!(
        lines[0].contains("above the reset's falling minimum of 2870 mV"),
        "{}",
        lines[0]
    );
}
