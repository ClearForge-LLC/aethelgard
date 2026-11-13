//! `AET-WO-2301`: the declared supply droop. The scenario's `droop` and
//! `expect_counts` (schema version 2), the `check_droop` rule (ruled D3), the
//! records of kinds 16 to 20, `check_run`'s droop rules and `power::schedule`.
//! Every refusal test mutates exactly one thing of a valid reference run and
//! names the reason it expects.

use aethelgard_chaos::campaign::{
    BELOW_TOLERANCE_FINDING, RESET_FINDING, Scenario, UNRECOVERED_FINDING, check_droop, check_run,
    droop_end_ns, rail_levels, tau_ns,
};
use aethelgard_chaos::cbor::Value;
use aethelgard_chaos::power::{self, Crossing};
use aethelgard_chaos::record::{Droop, Override, Reading, Record};
use serde_json::{Value as J, json};

// ---- the reference droop (`droop-bor3`, measured in the pinned image) ----

const NOMINAL_MV: u32 = 3300;
const TOLERANCE: u32 = 50;
const DECOUPLING_PF: u64 = 400_000;
const SPAN_NS: u64 = 10_000_000;

fn reference() -> Droop {
    Droop {
        domain: "VDD_3V3".into(),
        floor_millivolts: 2400,
        start_ns: 1_000_000,
        fall_ns: 20_000,
        hold_ns: 200_000,
        rise_ns: 20_000,
        source_milliohms: 500,
        load_milliamps: 100,
        bor_level: 3,
    }
}

fn droop_json() -> J {
    json!({
        "domain": "VDD_3V3",
        "floor_millivolts": 2400,
        "start_ns": 1_000_000,
        "fall_ns": 20_000,
        "hold_ns": 200_000,
        "rise_ns": 20_000,
        "source_milliohms": 500,
        "load_milliamps": 100,
        "bor_level": 3,
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

fn with_droop(d: J) -> J {
    scenario_json(2, json!([{ "droop": d }]), None)
}

fn droop_refusal(d: J) -> &'static str {
    parse(&with_droop(d)).expect_err("must be refused").kind
}

// ---- 1. parsing ----

/// `AET-WO-2301` §1: a valid version 2 droop scenario parses to the override.
#[test]
fn a_valid_v2_droop_scenario_parses_with_every_field() {
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

#[test]
fn a_droop_under_schema_version_1_is_refused() {
    let v = scenario_json(1, json!([{ "droop": droop_json() }]), None);
    assert_eq!(parse(&v).unwrap_err().kind, "SchemaViolation");
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
        2,
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
    let bounds: [(&str, u64, u64); 8] = [
        ("floor_millivolts", 1, 5_000),
        ("start_ns", 1, 100_000_000),
        ("fall_ns", 1, 10_000_000),
        ("hold_ns", 1, 100_000_000),
        ("rise_ns", 1, 10_000_000),
        ("source_milliohms", 1, 1_000_000),
        ("load_milliamps", 0, 10_000),
        ("bor_level", 0, 3),
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
        let v = scenario_json(2, json!([o]), None);
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
fn schema_version_three_is_unsupported_for_a_droop_too() {
    let v = scenario_json(3, json!([{ "droop": droop_json() }]), None);
    assert_eq!(parse(&v).unwrap_err().kind, "UnsupportedSchemaVersion");
}

// ---- 2. check_droop (ruled D3) ----

fn check(d: &Droop, nominal: u32, pf: u64, span: u64) -> Result<(), String> {
    check_droop(d, nominal, pf, span).map_err(|e| {
        assert_eq!(e.kind, "DroopRefused");
        assert_eq!(e.id.as_deref(), Some(d.domain.as_str()));
        e.message
    })
}

fn refusal(d: &Droop, nominal: u32, pf: u64, span: u64) -> String {
    check(d, nominal, pf, span).expect_err("must be refused")
}

fn with(f: impl FnOnce(&mut Droop)) -> Droop {
    let mut d = reference();
    f(&mut d);
    d
}

/// A droop with no load, so the rail floor is the floor.
fn unloaded(floor_millivolts: u32, bor_level: u8) -> Droop {
    with(|d| {
        d.floor_millivolts = floor_millivolts;
        d.load_milliamps = 0;
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
    let too_heavy = with(|d| d.load_milliamps = 10_000);
    assert_eq!(rail_levels(&too_heavy, NOMINAL_MV), None);
}

#[test]
fn the_reference_droop_passes_the_d3_rule_even_with_its_rail_floor_in_vbor2s_band() {
    // The rail floor, 2.35 V, is inside VBOR2's falling band [2.25, 2.37] V,
    // but VBOR2 is not active at BOR level 3.
    assert_eq!(
        check(&reference(), NOMINAL_MV, DECOUPLING_PF, SPAN_NS),
        Ok(())
    );
}

#[test]
fn a_floor_not_below_the_nominal_is_refused() {
    for floor in [NOMINAL_MV, NOMINAL_MV + 1] {
        let d = with(|d| d.floor_millivolts = floor);
        assert!(refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS).contains("not below"));
    }
}

#[test]
fn a_domain_with_no_decoupling_is_refused() {
    let m = refusal(&reference(), NOMINAL_MV, 0, SPAN_NS);
    assert!(m.contains("no decoupling"), "{m}");
}

#[test]
fn a_load_whose_drop_exceeds_the_floor_is_refused() {
    // 10 A through 500 mΩ is 5 V.
    let d = with(|d| d.load_milliamps = 10_000);
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("I·R drop reaches the floor"), "{m}");
}

#[test]
fn a_hold_shorter_than_ten_time_constants_is_refused_and_exactly_ten_accepted() {
    let short = with(|d| d.hold_ns = 1_999);
    let m = refusal(&short, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("less than 10 τ"), "{m}");
    let exact = with(|d| d.hold_ns = 2_000);
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
        &with(|d| d.floor_millivolts = 2640),
        NOMINAL_MV,
        DECOUPLING_PF,
        SPAN_NS,
    );
    assert!(m.contains("2590000 µV"), "{m}");
    let above = with(|d| d.floor_millivolts = 2840);
    assert_eq!(check(&above, NOMINAL_MV, DECOUPLING_PF, SPAN_NS), Ok(()));
}

#[test]
fn a_resting_rail_not_above_an_active_falling_band_is_refused() {
    // Nominal 2650 mV less 50 mV is 2600 mV, inside VBOR3's falling band.
    let m = refusal(&reference(), 2650, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("not above VBOR3's falling band"), "{m}");
    // The same rest is above BOR0 alone.
    let bor0 = with(|d| d.bor_level = 0);
    assert_eq!(check(&bor0, 2650, DECOUPLING_PF, SPAN_NS), Ok(()));
}

#[test]
fn a_crossed_threshold_whose_rising_band_holds_the_resting_rail_is_refused() {
    // Nominal 2780 mV less 50 mV is 2730 mV: above VBOR3's falling band, inside
    // its rising band [2630, 2780] mV, so the release is not stated.
    let m = refusal(&reference(), 2780, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("not above VBOR3's rising band"), "{m}");
    // Above the rising band it passes.
    assert_eq!(check(&reference(), 2840, DECOUPLING_PF, SPAN_NS), Ok(()));
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
    let d = reference();
    // 1,000,000 + 20,000 + 200,000 + 20,000 + 10 τ (2,000) + tRSTTEMPO
    // (377,000) + nudge (10) + recovery bound (100,000) + two grid steps (20).
    let end = droop_end_ns(&d, DECOUPLING_PF);
    assert_eq!(end, 1_719_030);
    assert_eq!(check(&d, NOMINAL_MV, DECOUPLING_PF, end), Ok(()));
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, end - 1);
    assert!(m.contains("past the boot's"), "{m}");
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

fn recovered() -> Record {
    observation(1_327_980, 0x22020000)
}

fn observation(at_ns: u64, status: u32) -> Record {
    Record::FirmwareObserved {
        at_ns,
        boot_word: reading(0x2000_0000, u32::MAX, 0x2100_B007, Some(0x2100_B007)),
        status: reading(0x2000_0004, u32::MAX, 0x2202_0000, Some(status)),
        value: reading(0x2000_0008, u32::MAX, 0x60, Some(0x60)),
        // The run has no witness record: the controller's readings are vacuous.
        cr2: reading(0, 0, 0, Some(0)),
        txdr: reading(0, 0, 0, Some(0)),
    }
}

fn declared() -> Record {
    Record::RailDeclared {
        droop: reference(),
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
            cr2: reading(10, 11, 12, None),
            txdr: reading(13, 14, 15, Some(16)),
        },
        Record::ExpectedCounts {
            counts: vec![("a".into(), 0), ("b".into(), u64::MAX)],
        },
    ];
    let kinds: Vec<u64> = records.iter().map(Record::kind).collect();
    assert_eq!(kinds, [16, 17, 17, 18, 18, 19, 19, 19, 20]);
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

#[test]
fn a_droop_override_with_a_bad_bor_level_or_a_missing_field_is_not_an_override() {
    let r = droop_records().remove(0);
    let Value::Array(overrides) = r.fields().remove(4) else {
        panic!("the overrides are the fifth field")
    };
    let Value::Array(mut droop) = overrides[0].clone() else {
        panic!()
    };
    let bor = droop.len() - 1;
    droop[bor] = Value::Uint(256);
    let mut fields = r.fields();
    fields[4] = Value::Array(vec![Value::Array(droop.clone())]);
    assert_eq!(Record::from_fields(5, &fields), None, "a u8 overflow");
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
    assert_eq!(recovered().timestamp(end), 1_327_980);
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
        assert!(with_cause(bad).check().is_err(), "{bad:?}");
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
    assert!(with_text(String::new()).check().is_err());
    assert!(with_text("a\nb".into()).check().is_err());
    assert!(with_text("x".repeat(1025)).check().is_err());
    assert!(with_text("x y".repeat(341)).check().is_ok());
    assert!(with_text("x".repeat(1024)).check().is_ok());
}

#[test]
fn expected_counts_must_be_sorted_unique_non_empty_names() {
    let counts = |v: &[(&str, u64)]| Record::ExpectedCounts {
        counts: v.iter().map(|(k, n)| ((*k).into(), *n)).collect(),
    };
    assert!(counts(&[("a", 1), ("b", 2)]).check().is_ok());
    for bad in [
        &[][..],
        &[("b", 1), ("a", 1)],
        &[("a", 1), ("a", 2)],
        &[("a b", 1)],
        &[("", 1)],
    ] {
        assert!(counts(bad).check().is_err(), "{bad:?}");
    }
}

#[test]
fn a_declared_rail_needs_a_floor_below_its_nominal_and_decoupling_and_a_domain_name() {
    let rail = |f: &dyn Fn(&mut Droop), nominal: u32, pf: u64| Record::RailDeclared {
        droop: with(f),
        nominal_millivolts: nominal,
        tolerance_permille: TOLERANCE,
        decoupling_picofarads: pf,
    };
    let keep: &dyn Fn(&mut Droop) = &|_| {};
    assert_eq!(rail(keep, NOMINAL_MV, DECOUPLING_PF).check(), Ok(()));
    assert!(
        rail(keep, 2400, DECOUPLING_PF).check().is_err(),
        "floor = nominal"
    );
    assert!(
        rail(keep, 2399, DECOUPLING_PF).check().is_err(),
        "floor > nominal"
    );
    assert!(rail(keep, NOMINAL_MV, 0).check().is_err(), "no decoupling");
    assert!(
        rail(&|d| d.domain = "bad name".into(), NOMINAL_MV, DECOUPLING_PF)
            .check()
            .is_err()
    );
}

#[test]
fn a_crossing_must_name_a_domain_and_a_level() {
    assert!(crossing("", 1, 1, true).check().is_err());
    let r = Record::RailCrossing {
        domain: "bad domain".into(),
        level: "VBOR3_falling".into(),
        microvolts: 1,
        at_ps: 1,
        falling: true,
    };
    assert!(r.check().is_err());
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
        "a run holds at most one declared rail and one set of expected counts"
    );
}

#[test]
fn a_declared_rail_that_differs_from_the_override_is_refused() {
    let mut r = run();
    let Record::RailDeclared { droop, .. } = &mut r[3] else {
        panic!()
    };
    droop.load_milliamps = 101;
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
    r.insert(k, observation(1_327_980, 0x22020000));
    assert_eq!(refused(&r), MSG_OBS);
}

#[test]
fn an_observation_at_the_wrong_time_is_refused() {
    // The probe is at the release plus the recovery bound: 1,327,980 ns.
    for at in [1_327_970, 1_327_990, 1_227_980, 2_000_000] {
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
    r[k] = observation(1_327_980, 0x22020001);
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
    r[k] = observation(1_327_980, 0x22020001);
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
    r.insert(4, rail);
    assert_eq!(refused(&r), MSG_ORDER, "the rail after a crossing");
}

#[test]
fn a_killed_boot_has_no_observation_and_every_release_counts_as_unrecovered() {
    let mut r = run();
    r.remove(k_obs(&r));
    let b = index(&r, |x| matches!(x, Record::BootObserved { .. }));
    r[b] = boot(None);
    set_verdict(
        &mut r,
        &[
            (RESET_FINDING, "info", 1),
            (UNRECOVERED_FINDING, "error", 1),
            (BELOW_TOLERANCE_FINDING, "info", 1),
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
    *expect_findings = vec![UNRECOVERED_FINDING.into()];
    r[1] = Record::ExpectedCounts {
        counts: vec![(UNRECOVERED_FINDING.into(), 1)],
    };
    let s = judged(&r).unwrap();
    assert!(!s.pass && s.matched);
    assert_eq!(s.failing_kinds, [UNRECOVERED_FINDING]);
    // The same killed boot, claimed recovered, is refused.
    set_verdict(&mut r, &RECOVERED_FINDINGS);
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
        ],
    );
    assert_eq!(refused(&r), MSG_NO_READ);
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
    let cr2 = reading(0x4000_5404, 0x03FF_67FF, 0x0201_04EC, Some(0x0201_04EC));
    let txdr = reading(0x4000_5428, 0xFF, 0xD0, Some(0xD0));
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
    let o = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
    if let Record::FirmwareObserved {
        cr2: c, txdr: t, ..
    } = &mut r[o]
    {
        *c = cr2.clone();
        *t = txdr.clone();
    }
    let v = r.len() - 1;
    r.insert(
        v,
        Record::I2cCheck {
            reference: "U2".into(),
            address: 0x76,
            register: 0xD0,
            expected: 0x60,
            observed: Some(0x60),
            status: Some("ok".into()),
        },
    );
    r.insert(
        v + 1,
        Record::I2cWitness {
            peripheral: "I2C1".into(),
            reference: "U2".into(),
            cr2,
            txdr,
        },
    );
    r
}

/// `AET-WO-2301` adversarial F5, with a check and a witness in the run: the
/// observation's value is the check's expectation, and its `CR2` and `TXDR`
/// are the witness's own address, mask and expectation.
#[test]
fn an_observations_value_and_controller_are_pinned_to_the_check_and_the_witness() {
    assert!(check_run(&run_with_check()).is_ok_and(|s| s.matched));
    let mutations: [fn(&mut Record); 3] = [
        |r| {
            if let Record::FirmwareObserved { value, .. } = r {
                value.expected = 0x61;
                value.observed = Some(0x61);
            }
        },
        |r| {
            if let Record::FirmwareObserved { cr2, .. } = r {
                cr2.mask = 0;
            }
        },
        |r| {
            if let Record::FirmwareObserved { txdr, .. } = r {
                txdr.address = 0x4000_5400;
            }
        },
    ];
    for (i, m) in mutations.iter().enumerate() {
        let mut r = run_with_check();
        let k = index(&r, |x| matches!(x, Record::FirmwareObserved { .. }));
        m(&mut r[k]);
        assert_eq!(
            check_run(&r).map(|_| ()),
            Err("an observation's readings are not the boot's own"),
            "mutation {i}"
        );
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
    let d = with(|d| d.floor_millivolts = 50);
    assert_eq!(rail_levels(&d, NOMINAL_MV), None);
    let m = refusal(&d, NOMINAL_MV, DECOUPLING_PF, SPAN_NS);
    assert!(m.contains("reaches the floor"), "{m}");
}
