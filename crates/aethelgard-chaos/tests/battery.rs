//! `AET-WO-2304b`: the battery sag and the rail out of band (scenario 4),
//! the chain's static clauses, kind 31 and its re-judgement in replay.
//! Every refusal test mutates one thing of a valid reference and names the
//! reason it expects.

use aethelgard_chaos::battery::{
    REFUSED, SagChain, binding, check_battery_sag, operating_point_uv, settle_ns,
};
use aethelgard_chaos::campaign::{Scenario, ScenarioError, check_run};
use aethelgard_chaos::record::{BatterySag, Override, Reading, Record, SagSink};
use serde_json::{Value as J, json};

const SPAN_NS: u64 = 100_000_000;

/// Sentinel-03's chain as IR derives it (predictions Q1): 9 V open, a
/// healthy 1 mΩ path, U10 11,645 mW (floors 3,800 and 7,000 mV) and U11
/// 3,107 mW (3,800 and 3,800 mV).
fn s03() -> SagChain {
    SagChain {
        v_open_microvolts: 9_000_000,
        healthy_milliohms: 1,
        sinks: vec![
            sink("U10", 11_645, 3_800, 7_000),
            sink("U11", 3_107, 3_800, 3_800),
        ],
        capacitance_picofarads: 67_440_000,
    }
}

fn sink(name: &str, mw: u64, shutdown: u64, regulation: u64) -> SagSink {
    SagSink {
        name: name.into(),
        input_milliwatts: mw,
        shutdown_millivolts: shutdown,
        regulation_millivolts: regulation,
    }
}

fn sag(r: u32) -> BatterySag {
    BatterySag {
        series_milliohms: r,
        start_ns: 10_000_000,
    }
}

fn refusal(s: &BatterySag, c: &SagChain) -> ScenarioError {
    check_battery_sag(s, c, SPAN_NS).expect_err("refused")
}

// ---- the clauses (§1.6) ----

#[test]
fn the_tolerated_sag_admits_at_its_static_point() {
    assert_eq!(check_battery_sag(&sag(500), &s03(), SPAN_NS), Ok(8_088_035));
    assert_eq!(binding(&s03().sinks).unwrap().name, "U10");
}

/// D-B9's edge (measured; predictions Q2 put it one mΩ lower): 949 mΩ is
/// admitted at 7,000,070 µV, 950 mΩ is below U10's regulation floor, and the
/// message names the sink and both floors.
#[test]
fn below_regulation_is_refused_at_the_edge_naming_the_sink_and_both_floors() {
    assert_eq!(check_battery_sag(&sag(949), &s03(), SPAN_NS), Ok(7_000_070));
    let e = refusal(&sag(950), &s03());
    assert_eq!(e.kind, REFUSED);
    assert!(e.message.starts_with("below regulation"), "{e}");
    assert!(
        e.message.contains("U10") && e.message.contains("7000000 µV"),
        "{e}"
    );
    assert!(e.message.contains("3800000 µV"), "{e}");
}

/// No operating point (F5): past 1,372 mΩ the constant power collapses.
#[test]
fn no_operating_point_is_refused() {
    let mut c = s03();
    // Lower every floor so only the collapse can refuse.
    for k in &mut c.sinks {
        (k.shutdown_millivolts, k.regulation_millivolts) = (1, 1);
    }
    assert!(check_battery_sag(&sag(1_372), &c, SPAN_NS).is_ok());
    let e = refusal(&sag(1_373), &c);
    assert!(e.message.starts_with("no operating point"), "{e}");
}

/// Below shutdown (F5), through a chain no real row gives: a regulation
/// floor below the shutdown floor (the `1315` precedent of a test-only
/// table). On every row held, `regulation_vin_min` is at least
/// `vin_millivolts.min`, so the regulation clause binds first there.
#[test]
fn below_shutdown_is_refused_before_regulation_and_names_both_floors() {
    let mut c = s03();
    c.sinks = vec![sink("U99", 14_752, 7_000, 3_000)];
    assert!(check_battery_sag(&sag(949), &c, SPAN_NS).is_ok());
    let e = refusal(&sag(950), &c);
    assert!(e.message.starts_with("below shutdown"), "{e}");
    assert!(
        e.message.contains("U99") && e.message.contains("3000000 µV"),
        "{e}"
    );
    // Both clauses would refuse: shutdown is judged first, whichever sink.
    c.sinks = vec![sink("A", 1, 1, 7_000), sink("B", 14_751, 7_000, 7_000)];
    assert!(refusal(&sag(950), &c).message.starts_with("below shutdown"));
}

#[test]
fn a_sag_at_or_below_the_healthy_path_is_not_a_fault() {
    let e = refusal(&sag(1), &s03());
    assert!(e.message.starts_with("not a fault"), "{e}");
    assert!(check_battery_sag(&sag(2), &s03(), SPAN_NS).is_ok());
}

#[test]
fn a_sag_whose_chain_settles_past_the_span_is_refused() {
    let c = s03();
    let v = check_battery_sag(&sag(500), &c, SPAN_NS).unwrap();
    let settle = settle_ns(&c, 500, v).unwrap();
    let mut s = sag(500);
    s.start_ns = SPAN_NS - settle;
    assert!(check_battery_sag(&s, &c, SPAN_NS).is_ok());
    s.start_ns += 1;
    let e = refusal(&s, &c);
    assert!(e.message.starts_with("past the span"), "{e}");
}

/// The incremental time constant is longer than R·C under constant power,
/// and grows toward the collapse.
#[test]
fn the_settling_time_is_the_incremental_time_constant() {
    let c = s03();
    // R·C at 500 mΩ, 67.44 µF: 33,720 ns; × V²/(V(2V − V_open)) ≈ 1.25.
    let v = operating_point_uv(9_000_000, 500, 14_752).unwrap();
    let s = settle_ns(&c, 500, v).unwrap();
    assert!((337_200..=10 * 33_720 * 13 / 10).contains(&s), "{s}");
    let near = operating_point_uv(9_000_000, 1_372, 14_752).unwrap();
    assert!(settle_ns(&c, 1_372, near).unwrap() > 10 * s);
    assert_eq!(settle_ns(&c, 500, 4_500_000), None);
}

// ---- the scenario (scenario version 4) ----

fn scenario(version: u32, overrides: J) -> J {
    json!({
        "schema_version": version, "name": "s", "base": "b.json", "expect": "pass",
        "expect_findings": [], "overrides": overrides,
    })
}

fn parse(v: &J) -> Result<Scenario, ScenarioError> {
    Scenario::parse(v.to_string().as_bytes())
}

fn sag_json(r: u64, start: u64) -> J {
    json!({"battery_sag": {"series_milliohms": r, "start_ns": start}})
}

#[test]
fn a_battery_sag_and_a_rail_out_of_band_parse_under_version_4_only() {
    let s = parse(&scenario(4, json!([sag_json(500, 10_000_000)]))).unwrap();
    assert_eq!(s.battery_sag(), Some(&sag(500)));
    let r = parse(&scenario(
        4,
        json!([{"rail_out_of_band": "VDD_5V"}, {"serial_rx_open": "CRSF"}]),
    ))
    .unwrap();
    assert_eq!(r.rail_out_of_band(), Some("VDD_5V"));
    for o in [sag_json(500, 1), json!({"rail_out_of_band": "VDD_5V"})] {
        let e = parse(&scenario(3, json!([o]))).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{e}");
        assert!(e.message.contains("schema_version 4"), "{e}");
    }
    assert_eq!(
        parse(&scenario(6, json!([]))).unwrap_err().kind,
        "UnsupportedSchemaVersion"
    );
}

#[test]
fn a_battery_sag_s_integers_are_bounded_and_its_keys_closed() {
    for (r, start) in [(0, 1), (10_000_001, 1), (1, 0), (1, 100_000_001)] {
        let e = parse(&scenario(4, json!([sag_json(r, start)]))).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{r} {start}: {e}");
    }
    assert!(parse(&scenario(4, json!([sag_json(10_000_000, 100_000_000)]))).is_ok());
    for bad in [
        json!({"battery_sag": {"series_milliohms": 500}}),
        json!({"battery_sag": {"series_milliohms": 500, "start_ns": 1, "x": 1}}),
        json!({"battery_sag": {"series_milliohms": 500.5, "start_ns": 1}}),
        json!({"battery_sag": [500, 1]}),
        json!({"battery_sag": {"series_milliohms": 500, "start_ns": 1}, "bus": "B"}),
        json!({"rail_out_of_band": "VDD_5V", "serial_rx_open": "CRSF"}),
        json!({"rail_out_of_band": ""}),
        json!({"rail_out_of_band": 5}),
    ] {
        let e = parse(&scenario(4, json!([bad.clone()]))).unwrap_err();
        assert_eq!(e.kind, "SchemaViolation", "{bad}: {e}");
    }
}

/// §1.6 combined: two sags, or a sag with a droop, are refused by name.
#[test]
fn a_combined_battery_sag_is_refused_by_name() {
    let droop = json!({"droop": {"domain": "VDD_3V3", "floor_millivolts": 3000,
        "start_ns": 1_000_000, "fall_ns": 20_000, "hold_ns": 600_000, "rise_ns": 20_000,
        "source_milliohms": 500, "load_milliamps": 100, "bor_level": 3}});
    for overrides in [
        json!([sag_json(500, 1), sag_json(600, 1)]),
        json!([sag_json(500, 1), droop]),
    ] {
        let e = parse(&scenario(4, overrides)).unwrap_err();
        assert_eq!(e.kind, REFUSED, "{e}");
        assert!(e.message.starts_with("combined"), "{e}");
    }
}

// ---- kind 31 and replay (`INV-04`) ----

fn declared(s: &BatterySag, c: &SagChain) -> Record {
    let at = check_battery_sag(s, c, SPAN_NS).unwrap();
    Record::BatterySagDeclared {
        sag: s.clone(),
        v_open_microvolts: c.v_open_microvolts,
        healthy_milliohms: c.healthy_milliohms,
        sinks: c.sinks.clone(),
        capacitance_picofarads: c.capacitance_picofarads,
        static_microvolts: at,
        solved_minimum_microvolts: at + 1,
        binding: "U10".into(),
    }
}

fn reading(observed: Option<u32>) -> Reading {
    Reading {
        address: 0x2000_0000,
        mask: 0xFFFF_FFFF,
        expected: 0x12,
        observed,
    }
}

/// A passing run of a tolerated sag: the scenario, the topology, kind 31,
/// the boot and the verdict.
fn run(kind31: Record) -> Vec<Record> {
    vec![
        Record::Scenario {
            name: "battery-sag-tolerated".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![Override::BatterySag(sag(500))],
        },
        Record::Solved {
            specification_sha256: [2; 32],
            topology_sha256: [3; 32],
        },
        kind31,
        Record::BootObserved {
            image_sha256: [4; 32],
            boot_word: reading(Some(0x12)),
            instructions_expected: 10,
            instructions_observed: Some(10),
            pc: Some(0),
            executable: vec![(0, 0x1000_0000)],
            brown_out: Reading {
                address: 0x2000_0004,
                mask: 0xC,
                expected: 0,
                observed: Some(0),
            },
            flash_access: Some(Vec::new()),
        },
        Record::Verdict {
            pass: true,
            findings: vec![],
        },
    ]
}

fn good() -> Record {
    declared(&sag(500), &s03())
}

fn with(f: impl FnOnce(&mut Record)) -> Vec<Record> {
    let mut r = good();
    f(&mut r);
    run(r)
}

#[test]
fn a_tolerated_sag_s_run_replays_and_kind_31_round_trips_its_fields() {
    let r = run(good());
    assert!(
        check_run(&r).is_ok_and(|s| s.matched && s.pass),
        "{:?}",
        check_run(&r)
    );
    let k = good();
    assert_eq!(k.kind(), 31);
    assert_eq!(k.timestamp(SPAN_NS), 0);
    assert_eq!(Record::from_fields(31, &k.fields()), Some(k.clone()));
    assert!(k.check().is_ok());
    // The override's wire form, tag 6, and the rail's, tag 7.
    for o in [
        Override::BatterySag(sag(500)),
        Override::RailOutOfBand {
            domain: "VDD_5V".into(),
        },
    ] {
        let s = Record::Scenario {
            name: "s".into(),
            sha256: [0; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides: vec![o],
        };
        assert_eq!(Record::from_fields(5, &s.fields()), Some(s));
    }
}

/// Replay re-judges kind 31 from its own fields: a record any clause would
/// have refused, a doctored static point or solved minimum, a wrong binding
/// floor, or a sag the scenario does not declare is refused.
#[test]
fn replay_refuses_a_doctored_kind_31() {
    let refused = |r: Vec<Record>| check_run(&r).expect_err("refused");
    let clause = "a battery sag's declaration is one its checks refuse";
    // R at the healthy path: not a fault.
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared {
                healthy_milliohms, ..
            } = r
            {
                *healthy_milliohms = 500;
            }
        })),
        clause
    );
    // A sink's floor raised past the static point: below regulation.
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared { sinks, .. } = r {
                sinks[1].regulation_millivolts = 8_100;
            }
        })),
        clause
    );
    // The power doubled: no operating point at 500 mΩ.
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared { sinks, .. } = r {
                sinks[0].input_milliwatts *= 4;
            }
        })),
        clause
    );
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared {
                static_microvolts, ..
            } = r
            {
                *static_microvolts += 1;
            }
        })),
        "a battery sag's static operating point is not the one its fields give"
    );
    let solved =
        "a battery sag's solved minimum is below a floor, or not its static operating point";
    for doctored in [8_088_035 + 1_001, 8_088_035 - 1_001, 6_999_999] {
        assert_eq!(
            refused(with(|r| {
                if let Record::BatterySagDeclared {
                    solved_minimum_microvolts,
                    ..
                } = r
                {
                    *solved_minimum_microvolts = doctored;
                }
            })),
            solved,
            "{doctored}"
        );
    }
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared { binding, .. } = r {
                *binding = "U11".into();
            }
        })),
        "a battery sag's binding floor is not its highest"
    );
    assert_eq!(
        refused(with(|r| {
            if let Record::BatterySagDeclared { sag, .. } = r {
                sag.series_milliohms = 600;
            }
        })),
        "the declared battery sag is not the scenario's"
    );
    let mut no_record = run(good());
    no_record.remove(2);
    assert_eq!(
        refused(no_record),
        "a battery sag override and its declaration do not come together"
    );
}

/// A rail out of band is rendered by its consumers' absence alone.
#[test]
fn a_rail_out_of_band_without_its_rendering_is_refused_in_replay() {
    let scenario = |overrides: Vec<Override>| {
        let mut r = run(good());
        r.remove(2);
        r[0] = Record::Scenario {
            name: "rail".into(),
            sha256: [1; 32],
            expect_pass: true,
            expect_findings: vec![],
            overrides,
        };
        r
    };
    let rail = Override::RailOutOfBand {
        domain: "VDD_5V".into(),
    };
    let reason = "a rail out of band is not rendered by its consumers' absence alone, or by none";
    assert_eq!(check_run(&scenario(vec![rail.clone()])), Err(reason));
    assert_eq!(
        check_run(&scenario(vec![
            rail,
            Override::SerialRxOpen {
                port: "CRSF".into()
            },
            Override::PullUpOhms {
                bus: "B".into(),
                ohms: 1
            },
        ])),
        Err(reason)
    );
}

// ---- the adversarial pass's findings, each refused (AET-WO-2304b FEEDBACK) ----

/// Adversarial 1 and 3: one rail out of band per scenario, rendered by its
/// consumers' absences alone, refused before any run as in replay.
#[test]
fn a_second_rail_or_a_rail_beside_another_fault_is_refused_by_name() {
    let rail = |d: &str| json!({"rail_out_of_band": d});
    let open = |p: &str| json!({"serial_rx_open": p});
    for overrides in [
        json!([rail("VDD_5V"), open("CRSF"), rail("VDD_3V3")]),
        json!([rail("VDD_5V"), open("CRSF"), {"bus": "SENSORS", "line_held_low": "sda"}]),
        json!([rail("VDD_5V"), open("CRSF"), sag_json(500, 1)]),
        json!([rail("VDD_5V")]),
    ] {
        let e = parse(&scenario(4, overrides.clone())).unwrap_err();
        assert_eq!(e.kind, "RailOutOfBandRefused", "{overrides}: {e}");
    }
    let two = vec![
        Override::RailOutOfBand { domain: "A".into() },
        Override::SerialRxOpen { port: "P".into() },
        Override::RailOutOfBand { domain: "B".into() },
    ];
    assert!(aethelgard_chaos::campaign::check_rail_out_of_band(&two).is_err());
}

/// One doctoring of kind 31.
type Edit = fn(&mut Record);

/// Adversarial 4: replay re-judges kind 31's shape and the scenario's
/// "combined" clause, so a record no scenario or deck could have made is
/// refused.
#[test]
fn replay_refuses_a_kind_31_out_of_shape_or_combined() {
    let clause = "a battery sag's declaration is one its checks refuse";
    let cases: [(&str, Edit); 5] = [
        ("start 0", |r| {
            if let Record::BatterySagDeclared { sag, .. } = r {
                sag.start_ns = 0;
            }
        }),
        ("R past 10 kΩ", |r| {
            if let Record::BatterySagDeclared { sag, .. } = r {
                sag.series_milliohms = 20_000_000;
            }
        }),
        ("500 V", |r| {
            if let Record::BatterySagDeclared {
                v_open_microvolts, ..
            } = r
            {
                *v_open_microvolts = 500_000_000;
            }
        }),
        ("0 pF", |r| {
            if let Record::BatterySagDeclared {
                capacitance_picofarads,
                ..
            } = r
            {
                *capacitance_picofarads = 0;
            }
        }),
        ("floors 0", |r| {
            if let Record::BatterySagDeclared { sinks, .. } = r {
                for k in sinks {
                    (k.shutdown_millivolts, k.regulation_millivolts) = (0, 0);
                }
            }
        }),
    ];
    for (what, f) in cases {
        // The scenario's sag matches the record's, so the clause is reached.
        let mut r = good();
        f(&mut r);
        let mut records = run(r.clone());
        if let (Record::Scenario { overrides, .. }, Record::BatterySagDeclared { sag, .. }) =
            (&mut records[0], &r)
        {
            overrides[0] = Override::BatterySag(sag.clone());
        }
        assert_eq!(check_run(&records), Err(clause), "{what}");
    }
    let mut combined = run(good());
    if let Record::Scenario { overrides, .. } = &mut combined[0] {
        overrides.push(Override::BatterySag(sag(600)));
    }
    assert_eq!(
        check_run(&combined),
        Err("a battery sag is combined with another sag or a droop")
    );
}

/// Adversarial 9b: at a zero discriminant the roots meet; that is the
/// collapse, refused as no operating point.
#[test]
fn a_zero_discriminant_is_no_operating_point() {
    assert_eq!(operating_point_uv(2_000_000, 1_000, 1_000), None);
    let c = SagChain {
        v_open_microvolts: 2_000_000,
        healthy_milliohms: 1,
        sinks: vec![sink("U1", 1_000, 1, 1)],
        capacitance_picofarads: 1_000_000,
    };
    let e = refusal(&sag(1_000), &c);
    assert!(e.message.starts_with("no operating point"), "{e}");
}
