//! `AET-WO-2301`: the declared supply droop. The scenario's `droop` and
//! `expect_counts` (schema version 2), the `check_droop` rule (ruled D3), the
//! records of kinds 16 to 20, `check_run`'s droop rules and `power::schedule`.
//! Every refusal test mutates exactly one thing of a valid reference run and
//! names the reason it expects.

use aethelgard_chaos::campaign::{
    BELOW_TOLERANCE_FINDING, PLANTED_CHANGES, PLANTED_NEEDS_VERSION, RESET_FINDING, Scenario,
    UNRECOVERED_FINDING, check_droop, check_droop_shape, check_run, droop_end_ns, rail_levels,
    tau_ns,
};
use aethelgard_chaos::cbor::Value;
use aethelgard_chaos::power::{self, Crossing};
use aethelgard_chaos::record::{Droop, Override, RailDroop, Reading, Record, resolve_instant};
use serde_json::{Value as J, json};

// ---- the reference droop (`droop-bor3`, measured in the pinned image) ----

const NOMINAL_MV: u32 = 3300;
const TOLERANCE: u32 = 50;
const DECOUPLING_PF: u64 = 400_000;
const SPAN_NS: u64 = 10_000_000;

/// The reference droop's instant from reset, as `droop-bor3` measured it,
/// and its BOR level, the specification's (`AET-WO-2310`: both are the
/// rendered rail's, kind 16's, no longer the scenario's).
const START_NS: u64 = 1_000_000;
const BOR_LEVEL: u8 = 3;
/// The scenario's offset that places [`START_NS`] from the reference boot
/// word, [`BOOT_WORD_NS`]: 1,000,000 − 3,627,610, before the word.
const OFFSET_NS: i64 = -2_627_610;

/// The scenario's droop (scenario 6): its instant signed from the
/// undisturbed boot word, no BOR level.
fn reference() -> Droop {
    Droop {
        domain: "VDD_3V3".into(),
        floor_millivolts: 2400,
        from_boot_word_ns: OFFSET_NS,
        fall_ns: 20_000,
        hold_ns: 200_000,
        rise_ns: 20_000,
        source_milliohms: 500,
        load_milliamps: 100,
    }
}

/// The reference droop as the run rendered it (kind 16): placed at
/// [`START_NS`] from the boot word, at the specification's BOR level 3.
fn reference_rail() -> RailDroop {
    RailDroop {
        droop: reference(),
        start_ns: START_NS,
        bor_level: BOR_LEVEL,
    }
}

fn droop_json() -> J {
    json!({
        "domain": "VDD_3V3",
        "floor_millivolts": 2400,
        "from_boot_word_ns": OFFSET_NS,
        "fall_ns": 20_000,
        "hold_ns": 200_000,
        "rise_ns": 20_000,
        "source_milliohms": 500,
        "load_milliamps": 100,
    })
}

fn scenario_json(version: u32, overrides: J, counts: Option<J>) -> J {
    let mut v = json!({
        "schema_version": version,
        "name": "droop-bor3",
        "base": "b.json",
        "expect": "pass",
        "expect_findings": [],
        "overrides": overrides,
    });
    if let Some(c) = counts {
        v["expect_counts"] = c;
    }
    v
}

fn parse(v: &J) -> Result<Scenario, aethelgard_chaos::campaign::ScenarioError> {
    Scenario::parse(&serde_json::to_vec(v).unwrap())
}

/// A droop is read only at scenario 6 (`AET-WO-2310`).
fn with_droop(d: J) -> J {
    scenario_json(6, json!([{ "droop": d }]), None)
}

fn droop_refusal(d: J) -> &'static str {
    parse(&with_droop(d)).expect_err("must be refused").kind
}

// ---- 1. parsing ----

/// `AET-WO-2301` §1: a valid droop scenario parses to the override; since
/// `AET-WO-2310` a droop is read at version 6 only.
#[test]
fn a_valid_v6_droop_scenario_parses_with_every_field() {
    let s = parse(&with_droop(droop_json())).unwrap();
    assert_eq!(s.overrides, vec![Override::Droop(reference())]);
    assert_eq!(s.droop(), Some(&reference()));
    assert!(s.expect_counts.is_empty());
    assert_eq!(s.counts_record(), None);
}

#[test]
fn expect_counts_is_parsed_and_sorted_whatever_the_file_order() {
    let counts = json!({"rail_below_tolerance": 1, "brownout_reset": 1, "brownout_unrecovered": 0});
    let s = parse(&scenario_json(2, json!([]), Some(counts))).unwrap();
    assert_eq!(
        s.expect_counts,
        vec![
            ("brownout_reset".to_owned(), 1),
            ("brownout_unrecovered".to_owned(), 0),
            ("rail_below_tolerance".to_owned(), 1),
        ]
    );
    assert_eq!(
        s.counts_record(),
        Some(Record::ExpectedCounts {
            counts: s.expect_counts.clone()
        })
    );
}

/// A droop below version 2 was a `SchemaViolation`; since `AET-WO-2310` a
/// droop below version 6 is refused first, by `PlantedEventNeedsVersion6`.
#[test]
fn a_droop_under_schema_version_1_is_refused() {
    let v = scenario_json(1, json!([{ "droop": droop_json() }]), None);
    assert_eq!(parse(&v).unwrap_err().kind, PLANTED_NEEDS_VERSION);
}

/// `AET-WO-2310`: a droop at versions 2 to 5, in either key form, is refused
/// by name, and the refusal says what changed.
#[test]
fn a_droop_under_schema_versions_2_to_5_needs_version_6() {
    let mut old = droop_json();
    let o = old.as_object_mut().unwrap();
    o.remove("from_boot_word_ns");
    o.insert("start_ns".into(), json!(START_NS));
    o.insert("bor_level".into(), json!(BOR_LEVEL));
    for version in 2..=5 {
        for d in [droop_json(), old.clone()] {
            let v = scenario_json(version, json!([{ "droop": d }]), None);
            let e = parse(&v).unwrap_err();
            assert_eq!(e.kind, PLANTED_NEEDS_VERSION, "{version}: {e:?}");
            assert!(e.message.contains(PLANTED_CHANGES), "{e:?}");
            assert!(
                e.message
                    .contains(&format!("schema_version {version} reads no droop")),
                "{e:?}"
            );
        }
    }
}

/// `AET-WO-2310`: at version 6 a droop states neither `start_ns` nor
/// `bor_level`; either is an unknown key.
#[test]
fn a_v6_droop_with_start_ns_or_bor_level_is_refused() {
    for (key, value) in [
        ("start_ns", json!(START_NS)),
        ("bor_level", json!(BOR_LEVEL)),
    ] {
        let mut d = droop_json();
        d[key] = value.clone();
        let e = parse(&with_droop(d)).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{key}");
        assert!(e.message.contains(key), "{key}: {e:?}");
        // Nor in place of the offset.
        let mut d = droop_json();
        d.as_object_mut().unwrap().remove("from_boot_word_ns");
        d[key] = value;
        assert_eq!(droop_refusal(d), "SchemaViolation", "{key} for the offset");
    }
}

/// `AET-WO-2310`: the offset is signed, a multiple of 10, within ±100 ms of
/// the boot word, judged before the boot from the offset alone.
#[test]
fn a_droop_offset_off_the_10_ns_grid_or_past_100_ms_is_refused_before_the_boot() {
    for ok in [-100_000_000i64, -10, 0, 10, 100_000_000] {
        let mut d = droop_json();
        d["from_boot_word_ns"] = json!(ok);
        let s = parse(&with_droop(d)).unwrap_or_else(|e| panic!("{ok}: {e:?}"));
        assert_eq!(s.droop().map(|d| d.from_boot_word_ns), Some(ok));
    }
    for bad in [5i64, -5, 1, -2_627_615, 100_000_010, -100_000_010] {
        let mut d = droop_json();
        d["from_boot_word_ns"] = json!(bad);
        let e = parse(&with_droop(d)).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{bad}");
        assert!(
            e.message
                .contains("judged before the boot, from the offset alone"),
            "{bad}: {e:?}"
        );
    }
}

#[test]
fn expect_counts_under_schema_version_1_is_refused() {
    let v = scenario_json(1, json!([]), Some(json!({"brownout_reset": 1})));
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
}

#[test]
fn a_duplicate_key_in_expect_counts_is_refused() {
    let raw = br#"{"schema_version":2,"name":"t","base":"b","expect":"pass","expect_findings":[],"expect_counts":{"brownout_reset":1,"brownout_reset":2},"overrides":[]}"#;
    assert_eq!(Scenario::parse(raw).unwrap_err().kind, "SchemaViolation");
}

#[test]
fn an_expect_counts_kind_that_is_not_a_name_or_a_count_that_is_not_an_integer_is_refused() {
    for counts in [
        json!({"a b": 1}),
        json!({"": 1}),
        json!({"brownout_reset": -1}),
        json!({"brownout_reset": 1.5}),
        json!([["brownout_reset", 1]]),
    ] {
        let v = scenario_json(2, json!([]), Some(counts.clone()));
        assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation", "{counts}");
    }
}

#[test]
fn two_droop_overrides_are_refused_whatever_their_domains() {
    let mut other = droop_json();
    other["domain"] = json!("VDD_1V8");
    let v = scenario_json(
        6,
        json!([{ "droop": droop_json() }, { "droop": other }]),
        None,
    );
    assert_eq!(parse(&v).unwrap_err().kind, "DuplicateOverride");
}

#[test]
fn a_droop_with_an_unknown_key_is_refused() {
    let mut d = droop_json();
    d["extra"] = json!(1);
    assert_eq!(droop_refusal(d), "SchemaViolation");
}

#[test]
fn a_droop_missing_any_key_is_refused() {
    for key in droop_json().as_object().unwrap().keys() {
        let mut d = droop_json();
        d.as_object_mut().unwrap().remove(key);
        assert_eq!(droop_refusal(d), "SchemaViolation", "missing {key}");
    }
}

/// `-1` is refused for the signed offset too: off its 10 ns grid
/// (`AET-WO-2310`; a negative multiple of 10 is a valid offset).
#[test]
fn a_droop_integer_that_is_a_float_or_negative_or_text_is_refused() {
    for key in droop_json().as_object().unwrap().keys() {
        if key == "domain" {
            continue;
        }
        for bad in [
            json!(1.5),
            json!(2400.0),
            json!(-1),
            json!("2400"),
            json!(null),
        ] {
            let mut d = droop_json();
            d[key] = bad.clone();
            assert_eq!(droop_refusal(d), "SchemaViolation", "{key} = {bad}");
        }
    }
}

/// `AET-WO-2301` §5.4: each bound is checked on its own, both ends.
#[test]
fn each_droop_bound_is_inclusive_and_one_outside_is_refused() {
    // `start_ns` and `bor_level` are no longer keys (`AET-WO-2310`): the
    // offset's bound is its own test, the BOR level the specification's.
    let bounds: [(&str, u64, u64); 6] = [
        ("floor_millivolts", 1, 5_000),
        ("fall_ns", 1, 10_000_000),
        ("hold_ns", 1, 100_000_000),
        ("rise_ns", 1, 10_000_000),
        ("source_milliohms", 1, 1_000_000),
        ("load_milliamps", 0, 10_000),
    ];
    for (key, lo, hi) in bounds {
        for ok in [lo, hi] {
            let mut d = droop_json();
            d[key] = json!(ok);
            assert!(parse(&with_droop(d)).is_ok(), "{key} = {ok} is inside");
        }
        let mut outside = vec![hi + 1];
        if lo > 0 {
            outside.push(lo - 1);
        }
        for bad in outside {
            let mut d = droop_json();
            d[key] = json!(bad);
            assert_eq!(droop_refusal(d), "SchemaViolation", "{key} = {bad}");
        }
    }
}

#[test]
fn a_droop_mixed_with_another_override_key_is_refused() {
    for extra in [
        json!({"bus": "SENSORS"}),
        json!({"net": "N"}),
        json!({"pull_up_ohms": 2200}),
        json!({"line_held_low": "sda"}),
    ] {
        let mut o = json!({ "droop": droop_json() });
        for (k, v) in extra.as_object().unwrap() {
            o[k] = v.clone();
        }
        let v = scenario_json(6, json!([o]), None);
        assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation", "{extra}");
    }
}

#[test]
fn a_droop_domain_must_be_1_to_64_printable_ascii_bytes() {
    for domain in ["", "a b", "tab\t", "caf\u{e9}", &"D".repeat(65)] {
        let mut d = droop_json();
        d["domain"] = json!(domain);
        assert_eq!(droop_refusal(d), "SchemaViolation", "{domain:?}");
    }
    let mut d = droop_json();
    d["domain"] = json!("D".repeat(64));
    assert!(parse(&with_droop(d)).is_ok());
}

#[test]
fn schema_version_seven_is_unsupported_for_a_droop_too() {
    let v = scenario_json(7, json!([{ "droop": droop_json() }]), None);
    assert_eq!(parse(&v).unwrap_err().kind, "UnsupportedSchemaVersion");
}

// ---- 2. check_droop (ruled D3) ----

/// The full rule, and (`AET-WO-2310`) its shape alone, which every refusal
/// but the instant's shares: each refusal of a rendered rail other than past
/// the span is `check_droop_shape`'s at the same level, word for word.
fn check(d: &RailDroop, nominal: u32, pf: u64, span: u64) -> Result<(), String> {
    let full = check_droop(d, nominal, pf, span).map_err(|e| {
        assert_eq!(e.kind, "DroopRefused");
        assert_eq!(e.id.as_deref(), Some(d.droop.domain.as_str()));
        e.message
    });
    let shape = check_droop_shape(&d.droop, d.bor_level, nominal, pf).map_err(|e| e.message);
    match &full {
        Err(m) if m.contains("past the boot's") => assert_eq!(shape, Ok(())),
        _ => assert_eq!(shape, full),
    }
    full
}

fn refusal(d: &RailDroop, nominal: u32, pf: u64, span: u64) -> String {
    check(d, nominal, pf, span).expect_err("must be refused")
}

fn with(f: impl FnOnce(&mut RailDroop)) -> RailDroop {
    let mut d = reference_rail();
    f(&mut d);
    d
}

/// A droop with no load, so the rail floor is the floor.
fn unloaded(floor_millivolts: u32, bor_level: u8) -> RailDroop {
    with(|d| {
        d.droop.floor_millivolts = floor_millivolts;
        d.droop.load_milliamps = 0;
        d.bor_level = bor_level;
    })
}

#[test]
fn the_time_constant_and_rail_levels_of_the_reference_droop() {
    assert_eq!(tau_ns(500, DECOUPLING_PF), 200);
    assert_eq!(tau_ns(1, 1), 1, "rounded up");
    assert_eq!(tau_ns(1, 0), 0);
    assert_eq!(
        rail_levels(&reference(), NOMINAL_MV),
        Some((2_350_000, 3_250_000))
    );
    let too_heavy = with(|d| d.droop.load_milliamps = 10_000);
    assert_eq!(rail_levels(&too_heavy.droop, NOMINAL_MV), None);
}

#[test]
fn the_reference_droop_passes_the_d3_rule_even_with_its_rail_floor_in_vbor2s_band() {
    // The rail floor, 2.35 V, is inside VBOR2's falling band [2.25, 2.37] V,
    // but VBOR2 is not active at BOR level 3.
    assert_eq!(
        check(&reference_rail(), NOMINAL_MV, DECOUPLING_PF, SPAN_NS),
        Ok(())
    );
}

#[test]
fn a_floor_not_below_the_nominal_is_refused() {
    for floor in [NOMINAL_MV, NOMINAL_MV + 1] {
        let d = with(|d| d.droop.floor_millivolts = floor);
        assert!(refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS).contains("not below"));
    }
}

#[test]
fn a_domain_with_no_decoupling_is_refused() {
    let m = refusal(&reference_rail(), NOMINAL_MV, 0, SPAN_NS);
    assert!(m.contains("no decoupling"), "{m}");
}

#[test]
fn a_load_whose_drop_exceeds_the_floor_is_refused() {
    // 10 A through 500 mΩ is 5 V.
    let d = with(|d| d.droop.load_milliamps = 10_000);
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("I·R drop reaches the floor"), "{m}");
}

#[test]
fn a_hold_shorter_than_ten_time_constants_is_refused_and_exactly_ten_accepted() {
    let short = with(|d| d.droop.hold_ns = 1_999);
    let m = refusal(&short, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("less than 10 τ"), "{m}");
    let exact = with(|d| d.droop.hold_ns = 2_000);
    assert_eq!(check(&exact, NOMINAL_MV, DECOUPLING_PF, SPAN_NS), Ok(()));
}

#[test]
fn a_rail_floor_inside_the_active_falling_band_is_refused_at_both_ends() {
    // Unloaded, the rail floor is the floor: VBOR3 falls in [2540, 2680] mV.
    for mv in [2540, 2610, 2680] {
        let m = refusal(&unloaded(mv, 3), NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
        assert!(m.contains("lies inside VBOR3's falling band"), "{mv}: {m}");
    }
    // One mV below, it crosses; above the band it is inside VBOR3's
    // hysteresis region up to the rising band's maximum, 2780 mV
    // (adversarial F4), and one mV above that it does not cross.
    let m = refusal(&unloaded(2681, 3), NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("hysteresis region"), "{m}");
    for mv in [2539, 2781] {
        assert_eq!(
            check(&unloaded(mv, 3), NOMINAL_MV, DECOUPLING_PF, SPAN_NS),
            Ok(()),
            "{mv}"
        );
    }
}

#[test]
fn the_rail_floor_is_the_floor_less_the_loads_drop() {
    // 2640 mV is inside the band; 100 mA through 500 mΩ takes the rail to
    // 2590 mV, still inside; 2840 mV less 50 mV is 2790 mV, above VBOR3's
    // whole hysteresis region.
    let m = refusal(
        &with(|d| d.droop.floor_millivolts = 2640),
        NOMINAL_MV,
        DECOUPLING_PF,
        SPAN_NS,
    );
    assert!(m.contains("2590000 µV"), "{m}");
    let above = with(|d| d.droop.floor_millivolts = 2840);
    assert_eq!(check(&above, NOMINAL_MV, DECOUPLING_PF, SPAN_NS), Ok(()));
}

#[test]
fn a_resting_rail_not_above_an_active_falling_band_is_refused() {
    // Nominal 2650 mV less 50 mV is 2600 mV, inside VBOR3's falling band.
    let m = refusal(&reference_rail(), 2650, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("not above VBOR3's falling band"), "{m}");
    // The same rest is above BOR0 alone.
    let bor0 = with(|d| d.bor_level = 0);
    assert_eq!(check(&bor0, 2650, DECOUPLING_PF, SPAN_NS), Ok(()));
}

#[test]
fn a_crossed_threshold_whose_rising_band_holds_the_resting_rail_is_refused() {
    // Nominal 2780 mV less 50 mV is 2730 mV: above VBOR3's falling band, inside
    // its rising band [2630, 2780] mV, so the release is not stated.
    let m = refusal(&reference_rail(), 2780, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("not above VBOR3's rising band"), "{m}");
    // Above the rising band it passes.
    assert_eq!(
        check(&reference_rail(), 2840, DECOUPLING_PF, SPAN_NS),
        Ok(())
    );
}

/// `AET-WO-2301` adversarial F4: a floor between the falling band and the
/// rising level sits where the rail's ringing dithers across a level ngspice
/// computes, so it is refused as inside the hysteresis region.
#[test]
fn a_floor_between_the_falling_band_and_the_rising_band_is_refused() {
    for mv in [2690, 2700, 2780] {
        let m = refusal(&unloaded(mv, 3), NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
        assert!(m.contains("VBOR3's hysteresis region"), "{mv}: {m}");
    }
}

#[test]
fn a_droop_whose_effects_run_past_the_span_is_refused() {
    let d = reference_rail();
    // 1,000,000 + 20,000 + 200,000 + 20,000 + 10 τ (2,000) + tRSTTEMPO
    // (377,000) + nudge (10) + two grid steps (20); the recovery bound is
    // checked once the boot has measured it (`AET-WO-2305` §1.2).
    let end = droop_end_ns(&d, DECOUPLING_PF);
    assert_eq!(end, 1_619_030);
    assert_eq!(check(&d, NOMINAL_MV, DECOUPLING_PF, end), Ok(()));
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, end - 1);
    assert!(m.contains("past the boot's"), "{m}");
    // The instant is the rendered rail's (`AET-WO-2310`): the end moves with
    // it, whatever the scenario's offset, and the refusal names it.
    let later = with(|d| d.start_ns += 10);
    assert_eq!(droop_end_ns(&later, DECOUPLING_PF), end + 10);
    let m = refusal(&later, NOMINAL_MV, DECOUPLING_PF, end);
    assert!(
        m.contains("its instant 1000010 ns, placed from the undisturbed boot word"),
        "{m}"
    );
    let moved = with(|d| d.droop.from_boot_word_ns += 10);
    assert_eq!(droop_end_ns(&moved, DECOUPLING_PF), end);
}

#[test]
fn vbor2_is_active_only_at_bor_level_2() {
    let d = with(|d| d.bor_level = 2);
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("VBOR2"), "{m}");
    for level in [0, 1, 3] {
        let d = with(|d| d.bor_level = level);
        assert_eq!(
            check(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS),
            Ok(()),
            "{level}"
        );
    }
}

#[test]
fn bor0_is_active_at_every_bor_level() {
    // VBOR0 falls in [1580, 1680] mV.
    for level in 0..=3 {
        let m = refusal(&unloaded(1650, level), NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
        assert!(m.contains("VBOR0"), "level {level}: {m}");
    }
}

// ---- 3. the records of kinds 16 to 20 ----

fn reading(address: u32, mask: u32, expected: u32, observed: Option<u32>) -> Reading {
    Reading {
        address,
        mask,
        expected,
        observed,
    }
}

/// The reference undisturbed boot word, sentinel-01-minimal's as measured
/// for `AET-WO-2310` (the value the work order states), its bound with the
/// one-quantum margin, and the reference release's probe: 1,227,980 + the
/// 10 ns nudge + 3,727,610. The word places the droop (kind 16's 1,000,000 ns
/// is the word plus the scenario's −2,627,610) and bounds the probe both.
const BOOT_WORD_NS: u64 = 3_627_610;
const BOUND_NS: u64 = BOOT_WORD_NS + power::RECOVERY_MARGIN_NS;
const PROBE_NS: u64 = 1_227_980 + power::NUDGE_NS + BOUND_NS;

fn bound() -> Record {
    Record::RecoveryBound {
        boot_word_ns: Some(BOOT_WORD_NS),
        margin_ns: power::RECOVERY_MARGIN_NS,
        bound_ns: Some(BOUND_NS),
    }
}

fn recovered() -> Record {
    observation(PROBE_NS, 0x22020000)
}

fn observation(at_ns: u64, status: u32) -> Record {
    Record::FirmwareObserved {
        at_ns,
        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, Some(0x2100_B007)),
        status: reading(0x2000_0004, u32::MAX, 0x2202_0000, Some(status)),
        value: reading(0x2000_0008, u32::MAX, 0x60, Some(0x60)),
        // The run has no witness record, so the observation shows no lines.
        witness: None,
    }
}

fn declared() -> Record {
    Record::RailDeclared {
        droop: reference_rail(),
        nominal_millivolts: NOMINAL_MV,
        tolerance_permille: TOLERANCE,
        decoupling_picofarads: DECOUPLING_PF,
    }
}

fn crossing(level: &str, microvolts: u64, at_ps: u64, falling: bool) -> Record {
    Record::RailCrossing {
        domain: "VDD_3V3".into(),
        level: level.into(),
        microvolts,
        at_ps,
        falling,
    }
}

fn reset(at_ns: u64, released: bool) -> Record {
    Record::ResetRendered {
        at_ns,
        released,
        cause: vec!["VBOR3".into()],
        rendering: if released {
            power::RELEASE_RENDERING
        } else {
            power::ASSERT_RENDERING
        }
        .into(),
    }
}

fn droop_records() -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "droop-bor3".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![Override::Droop(reference())],
        },
        Record::ExpectedCounts {
            counts: vec![
                (RESET_FINDING.into(), 1),
                (UNRECOVERED_FINDING.into(), 0),
                (BELOW_TOLERANCE_FINDING.into(), 1),
            ],
        },
        declared(),
    ]
}

#[test]
fn every_new_record_kind_round_trips_and_checks() {
    let records = [
        declared(),
        crossing("VBOR3_falling", 2_610_000, 1_014_422_222, true),
        crossing("VBOR3_rising", 2_700_000, 1_227_977_778, false),
        reset(1_014_430, false),
        reset(1_227_980, true),
        recovered(),
        observation(1, 0x22020001),
        Record::FirmwareObserved {
            at_ns: 5,
            boot_word: reading(1, 2, 3, None),
            status: reading(4, 5, 6, None),
            value: reading(7, 8, 9, None),
            witness: None,
        },
        Record::FirmwareObserved {
            at_ns: 6,
            boot_word: reading(1, 2, 3, None),
            status: reading(4, 5, 6, None),
            value: reading(7, 8, 9, None),
            witness: Some(vec!["[NOISY] i2c1.u2: Read 60".into()]),
        },
        Record::ExpectedCounts {
            counts: vec![("a".into(), 0), ("b".into(), u64::MAX)],
        },
        bound(),
        Record::RecoveryBound {
            boot_word_ns: None,
            margin_ns: power::RECOVERY_MARGIN_NS,
            bound_ns: None,
        },
    ];
    let kinds: Vec<u64> = records.iter().map(Record::kind).collect();
    assert_eq!(kinds, [16, 17, 17, 18, 18, 19, 19, 19, 19, 20, 32, 32]);
    for r in &records {
        assert_eq!(r.check(), Ok(()), "{r:?}");
        assert_eq!(Record::from_fields(r.kind(), &r.fields()).as_ref(), Some(r));
    }
}

#[test]
fn the_droop_override_round_trips_through_the_scenario_record() {
    let r = droop_records().remove(0);
    assert_eq!(r.check(), Ok(()));
    assert_eq!(Record::from_fields(5, &r.fields()), Some(r));
}

#[test]
fn a_new_kind_with_a_short_or_long_or_mistyped_payload_is_not_a_record() {
    for r in [
        declared(),
        reset(1, false),
        recovered(),
        droop_records().remove(1),
    ] {
        let fields = r.fields();
        let mut long = fields.clone();
        long.push(Value::Uint(0));
        assert_eq!(Record::from_fields(r.kind(), &long), None);
        assert_eq!(
            Record::from_fields(r.kind(), &fields[..fields.len() - 1]),
            None
        );
        let mut mistyped = fields.clone();
        mistyped[0] = Value::Neg(1);
        assert_eq!(Record::from_fields(r.kind(), &mistyped), None);
    }
    assert_eq!(Record::from_fields(21, &[]), None);
}

/// The BOR level is kind 16's since `AET-WO-2310`, no longer the
/// override's: a u8 overflow there is not a record, and an override with the
/// old tenth field, or missing one, is not an override.
#[test]
fn a_droop_override_with_a_bad_bor_level_or_a_missing_field_is_not_an_override() {
    // The rendered rail's fields: the droop's 8, then start_ns, bor_level.
    let rail = declared().fields();
    let bor = 9;
    assert_eq!(rail[bor], Value::Uint(u64::from(BOR_LEVEL)));
    let mut bad = rail.clone();
    bad[bor] = Value::Uint(256);
    assert_eq!(Record::from_fields(16, &bad), None, "a u8 overflow");
    // The offset is signed on the wire: the reference's is a CBOR negative.
    assert_eq!(rail[2], Value::from_i64(OFFSET_NS));
    assert!(matches!(rail[2], Value::Neg(_)), "{:?}", rail[2]);
    let r = droop_records().remove(0);
    let Value::Array(overrides) = r.fields().remove(4) else {
        panic!("the overrides are the fifth field")
    };
    let Value::Array(mut droop) = overrides[0].clone() else {
        panic!()
    };
    let mut fields = r.fields();
    // A version-18 override's trailing bor_level is one field too many.
    let mut old = droop.clone();
    old.push(Value::Uint(3));
    fields[4] = Value::Array(vec![Value::Array(old)]);
    assert_eq!(Record::from_fields(5, &fields), None, "the old bor_level");
    droop.pop();
    fields[4] = Value::Array(vec![Value::Array(droop)]);
    assert_eq!(Record::from_fields(5, &fields), None, "a missing field");
}

/// A record's archived time (`AET-WO-2301` §2.2): the declared rail and the
/// counts are decided before the boot; the rest carry their own.
#[test]
fn the_new_records_are_stamped_with_their_own_time_or_zero() {
    let end = 9_999_999_999;
    assert_eq!(declared().timestamp(end), 0);
    assert!(declared().before_boot());
    assert_eq!(droop_records().remove(1).timestamp(end), 0);
    assert!(droop_records().remove(1).before_boot());
    assert_eq!(
        crossing("VBOR3_falling", 2_610_000, 1_014_422_222, true).timestamp(end),
        1_014_422
    );
    assert_eq!(reset(1_014_430, false).timestamp(end), 1_014_430);
    assert_eq!(recovered().timestamp(end), PROBE_NS);
    assert!(!recovered().before_boot());
}

#[test]
fn a_reset_with_an_empty_or_unsorted_or_repeated_cause_is_refused() {
    let with_cause = |cause: &[&str]| Record::ResetRendered {
        at_ns: 1,
        released: false,
        cause: cause.iter().map(|s| (*s).into()).collect(),
        rendering: "r".into(),
    };
    assert_eq!(with_cause(&["VBOR0", "VBOR3"]).check(), Ok(()));
    for bad in [
        &[][..],
        &["VBOR3", "VBOR0"],
        &["VBOR3", "VBOR3"],
        &["VBOR 3"],
    ] {
        assert_eq!(
            with_cause(bad).check(),
            Err("a reset's causes are not sorted unique threshold names"),
            "{bad:?}"
        );
    }
}

#[test]
fn a_reset_rendering_must_be_1_to_1024_printable_ascii_bytes() {
    let with_text = |text: String| Record::ResetRendered {
        at_ns: 1,
        released: true,
        cause: vec!["VBOR3".into()],
        rendering: text,
    };
    let refused = Err("a reset's rendering is not 1 to 1024 printable ASCII bytes");
    assert_eq!(with_text(String::new()).check(), refused);
    assert_eq!(with_text("a\nb".into()).check(), refused);
    assert_eq!(with_text("x".repeat(1025)).check(), refused);
    assert!(with_text("x y".repeat(341)).check().is_ok());
    assert!(with_text("x".repeat(1024)).check().is_ok());
}

#[test]
fn expected_counts_must_be_sorted_unique_non_empty_names() {
    let counts = |v: &[(&str, u64)]| Record::ExpectedCounts {
        counts: v.iter().map(|(k, n)| ((*k).into(), *n)).collect(),
    };
    assert!(counts(&[("a", 1), ("b", 2)]).check().is_ok());
    let kinds = "expected counts are not one or more finding kinds";
    let sorted = "expected counts are not sorted unique by kind";
    for (bad, why) in [
        (&[][..], kinds),
        (&[("b", 1), ("a", 1)], sorted),
        (&[("a", 1), ("a", 2)], sorted),
        (&[("a b", 1)], kinds),
        (&[("", 1)], kinds),
    ] {
        assert_eq!(counts(bad).check(), Err(why), "{bad:?}");
    }
}

#[test]
fn a_declared_rail_needs_a_floor_below_its_nominal_and_decoupling_and_a_domain_name() {
    let rail = |f: &dyn Fn(&mut RailDroop), nominal: u32, pf: u64| Record::RailDeclared {
        droop: with(f),
        nominal_millivolts: nominal,
        tolerance_permille: TOLERANCE,
        decoupling_picofarads: pf,
    };
    let keep: &dyn Fn(&mut RailDroop) = &|_| {};
    assert_eq!(rail(keep, NOMINAL_MV, DECOUPLING_PF).check(), Ok(()));
    let floor = Err("a declared rail's floor is not below its nominal, or it has no decoupling");
    assert_eq!(
        rail(keep, 2400, DECOUPLING_PF).check(),
        floor,
        "floor = nominal"
    );
    assert_eq!(
        rail(keep, 2399, DECOUPLING_PF).check(),
        floor,
        "floor > nominal"
    );
    assert_eq!(rail(keep, NOMINAL_MV, 0).check(), floor, "no decoupling");
    assert_eq!(
        rail(
            &|d| d.droop.domain = "bad name".into(),
            NOMINAL_MV,
            DECOUPLING_PF
        )
        .check(),
        Err("a droop's domain is not 1 to 64 printable ASCII bytes")
    );
}

#[test]
fn a_crossing_must_name_a_domain_and_a_level() {
    let refused = Err("a crossing's domain or level is not 1 to 64 printable ASCII bytes");
    assert_eq!(crossing("", 1, 1, true).check(), refused);
    let r = Record::RailCrossing {
        domain: "bad domain".into(),
        level: "VBOR3_falling".into(),
        microvolts: 1,
        at_ps: 1,
        falling: true,
    };
    assert_eq!(r.check(), refused);
}

// ---- 4. check_run's droop rules ----

const MSG_ORDER: &str = "the records are not in the campaign's order";
const MSG_PAIR: &str = "a droop override and its declared rail do not come together";
const MSG_NO_DROOP: &str = "a run without a droop holds a droop's records";
const MSG_RAIL: &str = "the declared rail is not the scenario's droop";
const MSG_DOMAIN: &str = "a crossing names another domain";
const MSG_CROSSINGS: &str = "a threshold's crossings do not follow from the declared rail";
const MSG_LEVEL: &str = "a crossing names a level no active threshold has";
const MSG_RESETS: &str =
    "the rendered resets are not the supervisor's rule applied to the crossings";
const MSG_OBS: &str = "each release has not exactly one observation at the recovery bound";
const MSG_NO_READ: &str = "a boot that read nothing holds an observation";
const MSG_COUNTS: &str = "the verdict's droop findings do not match the droop's records";
const MSG_SEVERITY: &str = "a droop finding has the wrong severity";

fn boot(instructions_observed: Option<u64>) -> Record {
    let none = instructions_observed.is_none();
    Record::BootObserved {
        image_sha256: [4; 32],
        boot_word: reading(
            0x2000_0000,
            u32::MAX,
            0x2100_B007,
            (!none).then_some(0x2100_B007),
        ),
        instructions_expected: 10,
        instructions_observed,
        pc: (!none).then_some(0),
        executable: vec![(0, 0x1000_0000)],
        // BOR level 3 under BOR_LEV's mask: the level kind 16 states
        // (`AET-WO-2310`).
        brown_out: aethelgard_chaos::record::Reading {
            address: 0x2000_0004,
            mask: 0xC,
            expected: 0xC,
            observed: Some(0xC),
        },
        flash_access: Some(Vec::new()),
    }
}

fn verdict(findings: &[(&str, &str, u64)]) -> Record {
    Record::Verdict {
        pass: findings.iter().all(|(_, s, _)| *s == "info"),
        findings: findings
            .iter()
            .map(|(k, s, n)| ((*k).into(), (*s).into(), *n))
            .collect(),
    }
}

const RECOVERED_FINDINGS: [(&str, &str, u64); 2] = [
    (RESET_FINDING, "info", 1),
    (BELOW_TOLERANCE_FINDING, "info", 1),
];

/// The reference run, complete and valid: the scenario, its counts, the
/// topology, the declared rail, the crossings, the resets and the observation
/// in time order, the boot and the verdict.
fn run() -> Vec<Record> {
    let mut v = droop_records();
    v.insert(
        2,
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
    );
    v.extend([
        bound(),
        crossing("VBOR3_rising", 2_700_000, 1_012_422_222, true),
        crossing("VBOR3_falling", 2_610_000, 1_014_422_222, true),
        reset(1_014_430, false),
        crossing("VBOR3_falling", 2_610_000, 1_225_977_778, false),
        crossing("VBOR3_rising", 2_700_000, 1_227_977_778, false),
        reset(1_227_980, true),
        recovered(),
        boot(Some(10)),
        verdict(&RECOVERED_FINDINGS),
    ]);
    v
}

fn index(records: &[Record], pred: impl Fn(&Record) -> bool) -> usize {
    records.iter().position(pred).expect("a record to mutate")
}

fn is_crossing(level: &str, falling: bool) -> impl Fn(&Record) -> bool + '_ {
    move |r| matches!(r, Record::RailCrossing { level: l, falling: f, .. } if l == level && *f == falling)
}

fn is_reset(released: bool) -> impl Fn(&Record) -> bool {
    move |r| matches!(r, Record::ResetRendered { released: x, .. } if *x == released)
}

fn is_observation(r: &Record) -> bool {
    matches!(r, Record::FirmwareObserved { .. })
}

/// Put the droop's crossings, resets and observations back in time order
/// after a mutation that moves one, so the mutation is the only fault.
fn resort(records: &mut [Record]) {
    let key = |r: &Record| match r {
        Record::RailCrossing { at_ps, .. } => (*at_ps, 0),
        Record::ResetRendered { at_ns, .. } => (at_ns * 1000, 1),
        Record::FirmwareObserved { at_ns, .. } => (at_ns * 1000, 2),
        _ => unreachable!(),
    };
    let from = index(records, |r| matches!(r, Record::RailCrossing { .. }));
    let to = from
        + records[from..]
            .iter()
            .take_while(|r| {
                matches!(
                    r,
                    Record::RailCrossing { .. }
                        | Record::ResetRendered { .. }
                        | Record::FirmwareObserved { .. }
                )
            })
            .count();
    records[from..to].sort_by_key(key);
}

fn set_verdict(records: &mut [Record], findings: &[(&str, &str, u64)]) {
    *records.last_mut().unwrap() = verdict(findings);
}

fn judged(records: &[Record]) -> Result<aethelgard_chaos::campaign::RunSummary, &'static str> {
    check_run(records)
}

fn refused(records: &[Record]) -> &'static str {
    judged(records).expect_err("must be refused")
}

#[test]
fn the_reference_droop_run_passes_and_matches_its_counts() {
    let s = judged(&run()).unwrap();
    assert!(s.pass && s.expect_pass && s.matched);
    assert!(s.failing_kinds.is_empty());
}

#[test]
fn a_droop_run_without_expected_counts_matches_too() {
    let mut r = run();
    r.remove(1);
    assert!(judged(&r).unwrap().matched);
}

#[test]
fn a_wrong_expected_count_does_not_match_but_the_run_is_still_judged() {
    for (kind, n) in [
        (RESET_FINDING, 2),
        (UNRECOVERED_FINDING, 1),
        (BELOW_TOLERANCE_FINDING, 0),
    ] {
        let mut r = run();
        r[1] = Record::ExpectedCounts {
            counts: vec![(kind.into(), n)],
        };
        let s = judged(&r).unwrap();
        assert!(s.pass && !s.matched, "{kind} = {n}");
    }
}

#[test]
fn a_droop_record_with_no_droop_override_is_refused() {
    let mut r = run();
    let Record::Scenario { overrides, .. } = &mut r[0] else {
        panic!()
    };
    overrides.clear();
    // Its kind 32 goes with the override (`AET-WO-2310`: a run with none
    // planted holds none; that refusal is its own test).
    r.remove(index(&r, |x| matches!(x, Record::RecoveryBound { .. })));
    assert_eq!(refused(&r), MSG_PAIR);
    // And with the declared rail gone too, the crossings alone are refused.
    r.remove(index(&r, |x| matches!(x, Record::RailDeclared { .. })));
    assert_eq!(refused(&r), MSG_NO_DROOP);
}

#[test]
fn a_run_with_no_droop_and_no_droop_records_is_accepted_with_no_droop_findings() {
    let mut r = run();
    r.retain(|x| {
        !matches!(
            x,
            Record::RailDeclared { .. }
                | Record::RailCrossing { .. }
                | Record::ResetRendered { .. }
                | Record::FirmwareObserved { .. }
                | Record::ExpectedCounts { .. }
                | Record::RecoveryBound { .. }
        )
    });
    let Record::Scenario { overrides, .. } = &mut r[0] else {
        panic!()
    };
    overrides.clear();
    set_verdict(&mut r, &[]);
    assert!(judged(&r).unwrap().matched);
    // A droop finding with nothing behind it is refused.
    set_verdict(&mut r, &[(RESET_FINDING, "info", 1)]);
    assert_eq!(refused(&r), MSG_COUNTS);
}

#[test]
fn a_droop_override_with_no_declared_rail_is_refused() {
    let mut r = run();
    r.remove(index(&r, |x| matches!(x, Record::RailDeclared { .. })));
    assert_eq!(refused(&r), MSG_PAIR);
}

#[test]
fn a_second_declared_rail_is_refused() {
    let mut r = run();
    r.insert(3, declared());
    assert_eq!(
        refused(&r),
        "a run holds at most one declared rail or battery sag and one set of expected counts"
    );
}

#[test]
fn a_declared_rail_that_differs_from_the_override_is_refused() {
    let mut r = run();
    let Record::RailDeclared { droop, .. } = &mut r[3] else {
        panic!()
    };
    droop.droop.load_milliamps = 101;
    assert_eq!(refused(&r), MSG_RAIL);
}

#[test]
fn a_crossing_of_another_domain_is_refused() {
    let mut r = run();
    let i = index(&r, is_crossing("VBOR3_falling", true));
    let Record::RailCrossing { domain, .. } = &mut r[i] else {
        panic!()
    };
    *domain = "VDD_1V8".into();
    assert_eq!(refused(&r), MSG_DOMAIN);
}

#[test]
fn a_falling_crossing_past_its_ramps_window_is_refused() {
    // The window ends at start + fall + 10 τ: 1,022,000 ns.
    let i = |r: &[Record]| index(r, is_crossing("VBOR3_falling", true));
    for (at_ps, ok) in [(1_022_000_000, true), (1_022_000_001, false)] {
        let mut r = run();
        let k = i(&r);
        let Record::RailCrossing { at_ps: t, .. } = &mut r[k] else {
            panic!()
        };
        *t = at_ps;
        resort(&mut r);
        if ok {
            // Later than the table's reset: the resets no longer follow, but
            // the crossings do.
            assert_eq!(refused(&r), MSG_RESETS);
        } else {
            assert_eq!(refused(&r), MSG_CROSSINGS);
        }
    }
}

#[test]
fn a_crossing_before_its_ramp_starts_is_refused() {
    let mut r = run();
    let k = index(&r, is_crossing("VBOR3_rising", true));
    let Record::RailCrossing { at_ps, .. } = &mut r[k] else {
        panic!()
    };
    *at_ps = 1_000_000_000;
    resort(&mut r);
    assert_eq!(refused(&r), MSG_CROSSINGS);
}

#[test]
fn a_rising_crossing_past_its_ramps_window_is_refused() {
    // The window ends at start + fall + hold + rise + 10 τ: 1,242,000 ns.
    let mut r = run();
    let k = index(&r, is_crossing("VBOR3_rising", false));
    let Record::RailCrossing { at_ps, .. } = &mut r[k] else {
        panic!()
    };
    *at_ps = 1_242_000_001;
    resort(&mut r);
    assert_eq!(refused(&r), MSG_CROSSINGS);
}

#[test]
fn a_missing_crossing_is_refused() {
    for (level, falling) in [
        ("VBOR3_rising", true),
        ("VBOR3_falling", true),
        ("VBOR3_falling", false),
        ("VBOR3_rising", false),
    ] {
        let mut r = run();
        r.remove(index(&r, is_crossing(level, falling)));
        assert_eq!(refused(&r), MSG_CROSSINGS, "{level} {falling}");
    }
}

#[test]
fn a_crossing_of_a_level_the_floor_does_not_reach_is_refused() {
    // BOR0's levels, 1.62 V and 1.67 V, are active but the rail never gets there.
    let mut r = run();
    r.insert(
        index(&r, is_crossing("VBOR3_rising", true)),
        crossing("VBOR0_falling", 1_620_000, 1_012_000_000, true),
    );
    assert_eq!(refused(&r), MSG_CROSSINGS);
}

#[test]
fn a_crossing_of_an_inactive_level_is_refused() {
    let mut r = run();
    r.insert(
        index(&r, is_crossing("VBOR3_rising", true)) + 1,
        crossing("VBOR2_falling", 2_310_000, 1_013_000_000, true),
    );
    assert_eq!(refused(&r), MSG_LEVEL);
}

#[test]
fn a_crossing_at_a_non_typical_value_is_refused() {
    for uv in [2_700_001, 2_699_999, 0] {
        let mut r = run();
        let k = index(&r, is_crossing("VBOR3_rising", true));
        let Record::RailCrossing { microvolts, .. } = &mut r[k] else {
            panic!()
        };
        *microvolts = uv;
        assert_eq!(refused(&r), MSG_CROSSINGS, "{uv}");
    }
}

#[test]
fn a_crossing_twice_in_one_direction_is_refused() {
    let mut r = run();
    let k = index(&r, is_crossing("VBOR3_falling", true));
    let dup = crossing("VBOR3_falling", 2_610_000, 1_014_500_000, true);
    r.insert(k + 1, dup);
    resort(&mut r);
    assert_eq!(refused(&r), MSG_CROSSINGS);
}

#[test]
fn a_missing_rendered_reset_is_refused() {
    for released in [false, true] {
        let mut r = run();
        r.remove(index(&r, is_reset(released)));
        assert_eq!(refused(&r), MSG_RESETS, "released {released}");
    }
}

#[test]
fn an_extra_rendered_reset_is_refused() {
    let mut r = run();
    r.push(reset(1_300_000, false));
    // Placed before the boot, in time order.
    let extra = r.pop().unwrap();
    let k = index(&r, is_observation);
    r.insert(k, extra);
    resort(&mut r);
    assert_eq!(refused(&r), MSG_RESETS);
}

#[test]
fn a_reset_at_a_time_that_is_not_the_schedules_is_refused() {
    for (released, at) in [(false, 1_014_440), (false, 1_014_420), (true, 1_227_990)] {
        let mut r = run();
        let k = index(&r, is_reset(released));
        let Record::ResetRendered { at_ns, .. } = &mut r[k] else {
            panic!()
        };
        *at_ns = at;
        resort(&mut r);
        assert_eq!(refused(&r), MSG_RESETS, "{at}");
    }
}

#[test]
fn a_reset_with_another_cause_or_rendering_is_refused() {
    for released in [false, true] {
        let mut r = run();
        let k = index(&r, is_reset(released));
        let Record::ResetRendered { rendering, .. } = &mut r[k] else {
            panic!()
        };
        rendering.push('.');
        assert_eq!(refused(&r), MSG_RESETS, "rendering, released {released}");

        let mut r = run();
        let k = index(&r, is_reset(released));
        let Record::ResetRendered { cause, .. } = &mut r[k] else {
            panic!()
        };
        *cause = vec!["VBOR0".into()];
        assert_eq!(refused(&r), MSG_RESETS, "cause, released {released}");
    }
    // The assert and release renderings are not interchangeable.
    let mut r = run();
    let k = index(&r, is_reset(false));
    let Record::ResetRendered { rendering, .. } = &mut r[k] else {
        panic!()
    };
    *rendering = power::RELEASE_RENDERING.into();
    assert_eq!(refused(&r), MSG_RESETS);
}

#[test]
fn a_release_with_no_observation_is_refused() {
    let mut r = run();
    r.remove(index(&r, is_observation));
    assert_eq!(refused(&r), MSG_OBS);
}

#[test]
fn a_second_observation_of_one_release_is_refused() {
    let mut r = run();
    let k = index(&r, is_observation);
    r.insert(k, observation(PROBE_NS, 0x22020000));
    assert_eq!(refused(&r), MSG_OBS);
}

#[test]
fn an_observation_at_the_wrong_time_is_refused() {
    // The probe is at the release, the nudge and the recovery bound:
    // 4,955,600 ns (`AET-WO-2310`'s boot word; the first two are 2305's).
    assert_eq!(PROBE_NS, 4_955_600);
    for at in [
        1_327_970,
        1_327_990,
        1_227_980,
        2_000_000,
        PROBE_NS - 10,
        PROBE_NS + 10,
    ] {
        let mut r = run();
        let k = index(&r, is_observation);
        r[k] = observation(at, 0x22020000);
        resort(&mut r);
        assert_eq!(refused(&r), MSG_OBS, "{at}");
    }
}

#[test]
fn an_unrecovered_observation_needs_its_error_finding() {
    for (field, bad) in [("status", 0x22020001u32), ("boot_word", 0), ("value", 0x61)] {
        let mut r = run();
        let k = index(&r, is_observation);
        let Record::FirmwareObserved {
            boot_word,
            status,
            value,
            ..
        } = &mut r[k]
        else {
            panic!()
        };
        match field {
            "status" => status.observed = Some(bad),
            "boot_word" => boot_word.observed = Some(bad),
            _ => value.observed = Some(bad),
        }
        assert_eq!(
            refused(&r),
            MSG_COUNTS,
            "{field}: the verdict says recovered"
        );
    }
}

#[test]
fn an_unobserved_reading_is_not_a_recovery() {
    let mut r = run();
    let k = index(&r, is_observation);
    let Record::FirmwareObserved { value, .. } = &mut r[k] else {
        panic!()
    };
    value.observed = None;
    assert_eq!(refused(&r), MSG_COUNTS);
}

#[test]
fn an_unrecovered_observation_with_its_error_finding_is_accepted_and_fails_the_run() {
    let mut r = run();
    let k = k_obs(&r);
    r[k] = observation(PROBE_NS, 0x22020001);
    let Record::Scenario {
        expect_pass,
        expect_findings,
        ..
    } = &mut r[0]
    else {
        panic!()
    };
    *expect_pass = false;
    *expect_findings = vec![UNRECOVERED_FINDING.into()];
    r[1] = Record::ExpectedCounts {
        counts: vec![
            (RESET_FINDING.into(), 1),
            (UNRECOVERED_FINDING.into(), 1),
            (BELOW_TOLERANCE_FINDING.into(), 1),
        ],
    };
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "error", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
        ],
    );
    let s = judged(&r).unwrap();
    assert!(!s.pass && !s.expect_pass && s.matched);
    assert_eq!(s.failing_kinds, [UNRECOVERED_FINDING]);
}

fn k_obs(r: &[Record]) -> usize {
    index(r, is_observation)
}

#[test]
fn an_unrecovered_finding_at_info_is_refused() {
    let mut r = run();
    let k = k_obs(&r);
    r[k] = observation(PROBE_NS, 0x22020001);
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "info", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
        ],
    );
    assert_eq!(refused(&r), MSG_SEVERITY);
}

#[test]
fn a_brownout_reset_finding_at_error_is_refused() {
    let mut r = run();
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "error", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
        ],
    );
    assert_eq!(refused(&r), MSG_SEVERITY);
}

#[test]
fn a_rail_below_tolerance_finding_at_error_is_refused() {
    let mut r = run();
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (BELOW_TOLERANCE_FINDING, "error", 1),
        ],
    );
    assert_eq!(refused(&r), MSG_SEVERITY);
}

#[test]
fn the_verdicts_droop_counts_must_equal_the_records() {
    for findings in [
        &[(BELOW_TOLERANCE_FINDING, "info", 1)][..],
        &[
            (RESET_FINDING, "info", 2),
            (BELOW_TOLERANCE_FINDING, "info", 1),
        ],
        &[(RESET_FINDING, "info", 1)],
        &[
            (RESET_FINDING, "info", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "error", 1),
        ],
    ] {
        let mut r = run();
        set_verdict(&mut r, findings);
        assert_eq!(refused(&r), MSG_COUNTS, "{findings:?}");
    }
}

#[test]
fn a_rail_inside_its_tolerance_has_no_below_tolerance_finding() {
    // Tolerance 300 permille puts the limit at 2310 mV; the rail floor is 2350 mV.
    let mut r = run();
    let Record::RailDeclared {
        tolerance_permille, ..
    } = &mut r[3]
    else {
        panic!()
    };
    *tolerance_permille = 300;
    assert_eq!(refused(&r), MSG_COUNTS);
    set_verdict(&mut r, &[(RESET_FINDING, "info", 1)]);
    r[1] = Record::ExpectedCounts {
        counts: vec![(BELOW_TOLERANCE_FINDING.into(), 0)],
    };
    assert!(judged(&r).unwrap().matched);
}

#[test]
fn droop_records_out_of_time_order_are_refused() {
    // Each adjacent pair of the droop's records, swapped.
    let n = run()
        .iter()
        .filter(|r| {
            matches!(
                r,
                Record::RailCrossing { .. }
                    | Record::ResetRendered { .. }
                    | Record::FirmwareObserved { .. }
            )
        })
        .count();
    let from = index(&run(), |r| matches!(r, Record::RailCrossing { .. }));
    for i in from..from + n - 1 {
        let mut r = run();
        r.swap(i, i + 1);
        assert_eq!(refused(&r), MSG_ORDER, "swap {i}");
    }
}

#[test]
fn a_droop_record_outside_its_rank_is_refused() {
    let mut r = run();
    let obs = r.remove(k_obs(&r));
    let boot_at = index(&r, |x| matches!(x, Record::BootObserved { .. }));
    r.insert(boot_at + 1, obs);
    assert_eq!(refused(&r), MSG_ORDER);
    let mut r = run();
    let rail = r.remove(3);
    let first = index(&r, |x| matches!(x, Record::RailCrossing { .. }));
    r.insert(first + 1, rail);
    assert_eq!(refused(&r), MSG_ORDER, "the rail after a crossing");
    // Kind 32 is decided before the boot, with the rail (`AET-WO-2305`).
    let mut r = run();
    let b = r.remove(index(&r, |x| matches!(x, Record::RecoveryBound { .. })));
    let first = index(&r, |x| matches!(x, Record::RailCrossing { .. }));
    r.insert(first + 1, b);
    assert_eq!(refused(&r), MSG_ORDER, "the bound after a crossing");
}

#[test]
fn a_killed_boot_has_no_observation_and_every_release_counts_as_unrecovered() {
    let mut r = run();
    r.remove(k_obs(&r));
    let b = index(&r, |x| matches!(x, Record::BootObserved { .. }));
    r[b] = boot(None);
    // Killed at a bound, which the verdict names (`AET-WO-3201` B2).
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "error", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
            ("memory_cap_exceeded", "error", 1),
        ],
    );
    let Record::Scenario {
        expect_pass,
        expect_findings,
        ..
    } = &mut r[0]
    else {
        panic!()
    };
    *expect_pass = false;
    *expect_findings = vec![UNRECOVERED_FINDING.into(), "memory_cap_exceeded".into()];
    r[1] = Record::ExpectedCounts {
        counts: vec![(UNRECOVERED_FINDING.into(), 1)],
    };
    let s = judged(&r).unwrap();
    assert!(!s.pass && s.matched);
    assert_eq!(
        s.failing_kinds,
        [UNRECOVERED_FINDING, "memory_cap_exceeded"]
    );
    // The same killed boot, claimed recovered, is refused.
    let mut recovered = RECOVERED_FINDINGS.to_vec();
    recovered.push(("memory_cap_exceeded", "error", 1));
    set_verdict(&mut r, &recovered);
    assert_eq!(refused(&r), MSG_COUNTS);
}

#[test]
fn a_killed_boot_that_holds_an_observation_is_refused() {
    let mut r = run();
    let b = index(&r, |x| matches!(x, Record::BootObserved { .. }));
    r[b] = boot(None);
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "error", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
            ("memory_cap_exceeded", "error", 1),
        ],
    );
    assert_eq!(refused(&r), MSG_NO_READ);
}

// ---- 4b. the planted instant and level (`AET-WO-2310`) ----

const MSG_ONE_BOUND: &str =
    "a planted run does not hold exactly one kind 32, or a run with none planted holds one";
const MSG_NO_WORD: &str = "a planted run's reference holds no undisturbed boot word";
const MSG_INSTANT: &str =
    "the declared rail's instant is not the undisturbed boot word plus the scenario's offset";
const MSG_BOR: &str = "the declared rail's BOR level is not the one kind 10 declares";

fn is_bound(r: &Record) -> bool {
    matches!(r, Record::RecoveryBound { .. })
}

#[test]
fn the_reference_rails_instant_is_the_boot_word_plus_the_offset() {
    assert_eq!(resolve_instant(BOOT_WORD_NS, OFFSET_NS), Some(START_NS));
    assert_eq!(
        aethelgard_chaos::campaign::declared_bor_level(&reading(0, 0xC, 0xC, None)),
        Some(BOR_LEVEL)
    );
}

/// (a) A droop's run holds exactly one kind 32, and it holds the word.
#[test]
fn a_droop_run_without_its_kind_32_or_with_no_word_or_two_is_refused() {
    let mut r = run();
    r.remove(index(&r, is_bound));
    assert_eq!(refused(&r), MSG_ONE_BOUND, "no kind 32");

    let mut r = run();
    let k = index(&r, is_bound);
    r[k] = Record::RecoveryBound {
        boot_word_ns: None,
        margin_ns: power::RECOVERY_MARGIN_NS,
        bound_ns: None,
    };
    assert_eq!(r[k].check(), Ok(()));
    assert_eq!(refused(&r), MSG_NO_WORD, "kind 32 with no word");

    let mut r = run();
    let k = index(&r, is_bound);
    r.insert(k, bound());
    assert_eq!(refused(&r), MSG_ONE_BOUND, "two kind 32");
}

/// (b) Kind 16's instant is the word plus the scenario's offset, to the ns.
#[test]
fn a_declared_rails_instant_10_ns_off_the_word_plus_the_offset_is_refused() {
    for delta in [10i64, -10] {
        let mut r = run();
        let k = index(&r, |x| matches!(x, Record::RailDeclared { .. }));
        let Record::RailDeclared { droop, .. } = &mut r[k] else {
            panic!()
        };
        droop.start_ns = droop.start_ns.checked_add_signed(delta).unwrap();
        assert_eq!(refused(&r), MSG_INSTANT, "{delta}");
    }
    // The scenario's offset moved instead, both records agreeing: refused too.
    let mut r = run();
    for x in &mut r {
        match x {
            Record::Scenario { overrides, .. } => {
                let Override::Droop(d) = &mut overrides[0] else {
                    panic!()
                };
                d.from_boot_word_ns += 10;
            }
            Record::RailDeclared { droop, .. } => droop.droop.from_boot_word_ns += 10,
            _ => {}
        }
    }
    assert_eq!(refused(&r), MSG_INSTANT);
    // And the word moved by 10, the bound with it: the rail is no longer
    // placed from it.
    let mut r = run();
    let k = index(&r, is_bound);
    r[k] = Record::RecoveryBound {
        boot_word_ns: Some(BOOT_WORD_NS + 10),
        margin_ns: power::RECOVERY_MARGIN_NS,
        bound_ns: Some(BOUND_NS + 10),
    };
    assert_eq!(refused(&r), MSG_INSTANT);
}

/// (c) Kind 16's BOR level is the one kind 10's brown-out word declares.
#[test]
fn a_declared_rails_bor_level_other_than_kind_10s_is_refused() {
    let mut r = run();
    let k = index(&r, |x| matches!(x, Record::RailDeclared { .. }));
    let Record::RailDeclared { droop, .. } = &mut r[k] else {
        panic!()
    };
    droop.bor_level = 2;
    assert_eq!(refused(&r), MSG_BOR, "kind 16 at 2");
    // Kind 10 at level 2 (0x8 under 0xC), read as declared: kind 16's 3 is
    // not it.
    let mut r = run();
    let b = index(&r, |x| matches!(x, Record::BootObserved { .. }));
    let Record::BootObserved { brown_out, .. } = &mut r[b] else {
        panic!()
    };
    brown_out.expected = 0x8;
    brown_out.observed = Some(0x8);
    assert_eq!(refused(&r), MSG_BOR, "kind 10 at 2");
}

/// (e) A run with no planted override holds no kind 32.
#[test]
fn a_run_with_nothing_planted_that_holds_a_kind_32_is_refused() {
    let mut r = run();
    r.retain(|x| {
        !matches!(
            x,
            Record::RailDeclared { .. }
                | Record::RailCrossing { .. }
                | Record::ResetRendered { .. }
                | Record::FirmwareObserved { .. }
                | Record::ExpectedCounts { .. }
        )
    });
    let Record::Scenario { overrides, .. } = &mut r[0] else {
        panic!()
    };
    overrides.clear();
    set_verdict(&mut r, &[]);
    assert_eq!(refused(&r), MSG_ONE_BOUND);
    r.remove(index(&r, is_bound));
    assert!(judged(&r).unwrap().matched);
}

// ---- 5. power::schedule beyond its unit tests ----

fn cross(level: &str, at_ns: u64, falling: bool) -> Crossing {
    let t = power::BOR
        .iter()
        .find(|t| level.starts_with(t.name))
        .unwrap();
    Crossing {
        level: level.into(),
        microvolts: if level.ends_with("_falling") {
            t.falling()
        } else {
            t.rising()
        },
        at_ps: at_ns * 1000,
        falling,
    }
}

#[test]
fn two_separate_dips_give_two_resets() {
    let r = power::schedule(
        3,
        &[
            cross("VBOR3_falling", 100_000, true),
            cross("VBOR3_rising", 200_000, false),
            cross("VBOR3_falling", 500_000, true),
            cross("VBOR3_rising", 700_000, false),
        ],
    )
    .unwrap();
    let reset = |a, b| power::Reset {
        asserted_ns: a,
        released_ns: Some(b),
        cause: vec!["VBOR3".into()],
    };
    assert_eq!(r, [reset(100_000, 200_000), reset(500_000, 700_000)]);
}

#[test]
fn the_schedule_is_independent_of_the_crossings_order() {
    let mut c = vec![
        cross("VBOR3_falling", 100_000, true),
        cross("VBOR3_rising", 200_000, false),
        cross("VBOR3_falling", 500_000, true),
        cross("VBOR3_rising", 700_000, false),
    ];
    let forward = power::schedule(3, &c).unwrap();
    c.reverse();
    assert_eq!(power::schedule(3, &c).unwrap(), forward);
}

/// `AET-WO-2301` D4: BOR0 releases `tRSTTEMPO` after its rising crossing, and
/// a fall before that release keeps the one reset, the release moving later.
#[test]
fn a_refall_before_bor0s_temporization_ends_keeps_one_reset_released_later() {
    let t = power::T_RSTTEMPO_NS;
    let r = power::schedule(
        0,
        &[
            cross("VBOR0_falling", 100_000, true),
            cross("VBOR0_rising", 200_000, false),
            // Back below 100 µs after the rise: inside the temporization.
            cross("VBOR0_falling", 300_000, true),
            cross("VBOR0_rising", 400_000, false),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 1, "one reset");
    assert_eq!(r[0].asserted_ns, 100_000);
    assert_eq!(
        r[0].released_ns,
        Some(400_000 + t),
        "released after the last rise"
    );
}

#[test]
fn a_refall_after_bor0s_temporization_ends_is_a_second_reset() {
    let t = power::T_RSTTEMPO_NS;
    let r = power::schedule(
        0,
        &[
            cross("VBOR0_falling", 100_000, true),
            cross("VBOR0_rising", 200_000, false),
            cross("VBOR0_falling", 200_000 + t + 1_000, true),
            cross("VBOR0_rising", 200_000 + t + 100_000, false),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 2);
    assert_eq!(r[0].released_ns, Some(200_000 + t));
}

/// `AET-WO-2301` adversarial F5: replay pins each observation's readings to
/// the boot's own, so a re-sealed archive cannot declare an observation
/// recovered by giving it a mask of 0 or its own expectation.
#[test]
fn an_observation_whose_readings_are_not_the_boots_own_is_refused() {
    let at = |r: &[Record]| index(r, |x| matches!(x, Record::FirmwareObserved { .. }));
    let mutations: [fn(&mut Record); 4] = [
        |r| {
            if let Record::FirmwareObserved { boot_word, .. } = r {
                boot_word.mask = 0;
            }
        },
        |r| {
            if let Record::FirmwareObserved { boot_word, .. } = r {
                boot_word.expected = 0;
                boot_word.observed = Some(0);
            }
        },
        |r| {
            if let Record::FirmwareObserved { status, .. } = r {
                status.mask = 0;
                status.observed = Some(0x2202_0001);
            }
        },
        |r| {
            if let Record::FirmwareObserved { value, .. } = r {
                value.mask = 0xff;
            }
        },
    ];
    for (i, m) in mutations.iter().enumerate() {
        let mut r = run();
        let k = at(&r);
        m(&mut r[k]);
        assert_eq!(
            check_run(&r).map(|_| ()),
            Err("an observation's readings are not the boot's own"),
            "mutation {i}"
        );
    }
}

/// The reference run with the solved bus, U2's identity check and I2C1's
/// witness, and an observation whose controller readings are the witness's.
fn run_with_check() -> Vec<Record> {
    use aethelgard_chaos::record::BusLine;
    let line = |signal: &str, net: &str, pin: &str| BusLine {
        signal: signal.into(),
        net: net.into(),
        pin: pin.into(),
        alternate_function: 4,
        ceiling_ps: 300_000,
    };
    let mut r = run();
    let solved = index(&r, |x| matches!(x, Record::Solved { .. }));
    r.insert(
        solved + 1,
        Record::I2cBusSolved {
            bus: "SENSORS".into(),
            peripheral: "I2C1".into(),
            lines: vec![
                line("I2C1_SCL", "I2C_SCL", "PB6"),
                line("I2C1_SDA", "I2C_SDA", "PB7"),
            ],
            devices: vec![("U2".into(), 0x76)],
        },
    );
    // Each line measured, healthy (`AET-WO-3201` B1).
    for (k, net) in ["I2C_SCL", "I2C_SDA"].into_iter().enumerate() {
        r.insert(
            solved + 2 + k,
            Record::RiseComputed {
                bus: "SENSORS".into(),
                net: net.into(),
                rise_30_70_ps: 150_000,
                complete: true,
                ceiling_ps: 300_000,
                exceeds: false,
                window: aethelgard_chaos::record::Window {
                    deck_hz: 400_000,
                    declared_hz: 400_000,
                },
            },
        );
    }
    let o = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
    if let Record::FirmwareObserved { witness, .. } = &mut r[o] {
        *witness = Some(lines());
    }
    let v = r.len() - 1;
    r.insert(
        v,
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x76,
            step: "identity".into(),
            write: vec![0xD0],
            width: 1,
            expected: 0x60,
            mask: 0xFF,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
    );
    r.insert(
        v + 1,
        Record::I2cWitness {
            step: "identity".into(),
            peripheral: "I2C1".into(),
            reference: "U2".into(),
            device: "i2c1.u2".into(),
            expected: lines(),
            observed: Some(lines()),
            absent: Vec::new(),
        },
    );
    r
}

/// U2's identity read as the BME280 model logs it: register 0xD0, value 0x60.
fn lines() -> Vec<String> {
    vec![
        "[NOISY] i2c1.u2: Write D0".into(),
        "[NOISY] i2c1.u2: Read 60".into(),
    ]
}

/// `AET-WO-2301` adversarial F5, with a check and a witness in the run: the
/// observation's value is the check's expectation, and (`AET-WO-2204`) its
/// witness lines are the first witness record's own expected lines, or none
/// when the run has no witness.
#[test]
fn an_observations_value_and_witness_are_pinned_to_the_check_and_the_witness() {
    assert!(check_run(&run_with_check()).is_ok_and(|s| s.matched));
    let mut r = run_with_check();
    let k = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
    if let Record::FirmwareObserved { value, .. } = &mut r[k] {
        value.expected = 0x61;
        value.observed = Some(0x61);
    }
    assert_eq!(
        check_run(&r).map(|_| ()),
        Err("an observation's readings are not the boot's own"),
    );
    // Lines on an observation of a run with no witness record are refused.
    let mut r = run();
    let k = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
    if let Record::FirmwareObserved { witness, .. } = &mut r[k] {
        *witness = Some(lines());
    }
    assert_eq!(
        check_run(&r).map(|_| ()),
        Err("an observation's readings are not the boot's own"),
    );
    // With a witness record, an observation that shows other lines, none, or
    // an unreadable log is not a recovery: the verdict that says recovered is
    // refused.
    let others: [Option<Vec<String>>; 3] = [
        Some(vec!["[NOISY] i2c1.u2: Write D0".into()]),
        Some(vec![]),
        None,
    ];
    for (i, w) in others.into_iter().enumerate() {
        let mut r = run_with_check();
        let k = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
        if let Record::FirmwareObserved { witness, .. } = &mut r[k] {
            *witness = w;
        }
        assert_eq!(refused(&r), MSG_COUNTS, "variant {i}");
    }
}

/// Mutation pass, guard 22: a re-fall inside BOR0's temporization with no rise
/// after it holds the reset to the run's end; the first rise's pending release
/// must not close it while the rail is low.
#[test]
fn a_refall_inside_trsttempo_with_no_later_rise_is_never_released() {
    let c = |level: &str, uv: u64, at_ns: u64, falling: bool| Crossing {
        level: level.into(),
        microvolts: uv,
        at_ps: at_ns * 1000,
        falling,
    };
    let r = power::schedule(
        0,
        &[
            c("VBOR0_falling", 1_620_000, 100_000, true),
            c("VBOR0_rising", 1_670_000, 200_000, false),
            c("VBOR0_falling", 1_620_000, 300_000, true),
        ],
    )
    .unwrap();
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0].released_ns, None, "{r:?}");
}

/// Mutation pass, guard 26: a load whose I·R drop equals the floor takes the
/// rail to 0 V, which is no rail: refused, as a larger drop is.
#[test]
fn a_drop_exactly_equal_to_the_floor_is_refused() {
    // 100 mA through 500 mΩ is 50 mV: the floor itself.
    let d = with(|d| d.droop.floor_millivolts = 50);
    assert_eq!(rail_levels(&d.droop, NOMINAL_MV), None);
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("reaches the floor"), "{m}");
}

/// `AET-WO-2310`: kind 16's BOR level is the specification's, which the part
/// bounds to 0 to 3; a rail at level 4 is refused by its own record, never
/// read as level 0 by `power::active`.
#[test]
fn a_declared_rail_at_bor_level_4_is_refused_by_its_record() {
    assert_eq!(declared().check(), Ok(()));
    let Record::RailDeclared {
        mut droop,
        nominal_millivolts,
        tolerance_permille,
        decoupling_picofarads,
    } = declared()
    else {
        unreachable!()
    };
    droop.bor_level = 4;
    let four = Record::RailDeclared {
        droop,
        nominal_millivolts,
        tolerance_permille,
        decoupling_picofarads,
    };
    assert_eq!(
        four.check(),
        Err("a declared rail's BOR level is not 0 to 3")
    );
}
