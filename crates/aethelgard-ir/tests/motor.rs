//! `AET-WO-4300a` §1.1–§1.2 and adversarial item 3: motor outputs declared,
//! validated, placed on timer channels, and their topology checked, on a
//! small synthetic `PinTable` (this crate may depend on no other member).

use std::collections::BTreeMap;

use aethelgard_ir::motor::check_silicon;
use aethelgard_ir::{
    IrValidationError, MotorOutput, MotorProtocol, Pin, PinSource, PinTable, PwmTiming, SolveError,
    SolvedClock, SolvedTopology, Specification, solve, solve_with_limit,
};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// The reference specification's declared board (`examples/sentinel-01-minimal.json`).
const BOARD: &str = r#"{"copper_layers": 2, "layers": [{"name": "F.Cu", "role": "signal", "net": null}, {"name": "B.Cu", "role": "signal", "net": null}], "stackup": {"finished_thickness_um": {"nominal": 1600, "min": 340, "max": 2000, "source": "AET-WO-1305 conditions: 1.6 mm declared; 0.34 mm = 2 x 70 um + 0.2 mm; 2.0 mm = 1.6 mm + 25 %"}, "dielectrics": [{"between": ["F.Cu", "B.Cu"], "min_thickness_um": 200, "source": "implied: the finished minimum 340 um less twice the copper maximum 70 um"}], "copper_max_um": {"value": 70, "source": "AET-WO-1305 condition: twice KiCad's default 35 um (2 oz)"}, "relative_permittivity_max_permille": {"value": 5400, "source": "the IPC-4101 FR-4 maximum, specified at 1 MHz"}}, "rules": {"clearance_um": 200, "track_width_um": 200, "via_drill_um": 300, "via_diameter_um": 600}}"#;

/// TIM1 on PA8..PA11 (and its CH1N on PA7, which is no output), TIM3 CH1 on
/// PA6 beside SPI1_MISO, TIM2 CH1 on PA0 and PA5, and TIM4 CH1 only on PB6.
fn table() -> PinTable {
    let pins: &[(&str, &[(&str, u8)])] = &[
        ("PA0", &[("TIM2_CH1", 1)]),
        ("PA5", &[("SPI1_SCK", 5), ("TIM2_CH1", 1)]),
        ("PA6", &[("SPI1_MISO", 5), ("TIM3_CH1", 2)]),
        ("PA7", &[("SPI1_MOSI", 5), ("TIM1_CH1N", 1)]),
        ("PA8", &[("TIM1_CH1", 1)]),
        ("PA9", &[("TIM1_CH2", 1)]),
        ("PA10", &[("TIM1_CH3", 1)]),
        ("PA11", &[("TIM1_CH4", 1)]),
        ("PA13", &[("TIM1_CH1", 1)]),
        ("PB6", &[("TIM4_CH1", 2)]),
        ("PB7", &[]),
        ("PC14", &[]),
    ];
    PinTable {
        part: "STM32H753ZITx".into(),
        package: "TEST".into(),
        source: PinSource {
            repository: "test".into(),
            commit: "test".into(),
            files: BTreeMap::new(),
        },
        pins: pins
            .iter()
            .enumerate()
            .map(|(i, (name, afs))| {
                let pin = Pin {
                    position: i as u16 + 1,
                    alternate_functions: afs.iter().map(|(s, af)| ((*s).into(), *af)).collect(),
                    ..Default::default()
                };
                ((*name).into(), pin)
            })
            .collect(),
    }
}

/// An output's declared pull-down (specification 28, `AET-WO-4300i`), with
/// its link and fitment (specification 29, `AET-WO-4300j`), its references
/// named after the output.
fn pull(label: &str) -> Value {
    json!({"resistor": {"reference": format!("RPD_{label}"), "part": "CRCW0603_F_K",
        "ohms": 442000, "to": "GND"},
        "link": {"reference": format!("RLK_{label}"), "part": "CRCW0402_Z_0"},
        "fitment": "fitted",
        "source": "AET-WO-4300j D1 (a): cross-rail bound 422561 to 895714 ohm (test)"})
}

fn pwm(label: &str, pin: Option<&str>, rate_hz: u32) -> Value {
    json!({"label": label, "pin": pin, "protocol": "pwm",
           "pwm": {"rate_hz": rate_hz, "pulse_min_us": 1000, "pulse_max_us": 2000,
                   "idle": {"pulse_us": 1000}, "resolution_ps": 1_000_000}, "esc": null,
           "pull": pull(label)})
}

fn dshot(label: &str, pin: Option<&str>) -> Value {
    json!({"label": label, "pin": pin, "protocol": "dshot600", "pwm": null, "esc": null,
           "pull": pull(label)})
}

fn spec_json(motors: Vec<Value>) -> Value {
    json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "accept_lifecycle": [],
        "acknowledge_unwitnessed": [],
        "project_name": "t",
        "board": serde_json::from_str::<Value>(BOARD).unwrap(),
        "mcu": {"part": "STM32H753ZITx", "supply": "VDD_3V3", "hse_hz": 0, "bor_level": 0,
                "core_supply": {"mode": "ldo", "source": "DS12117 Rev 10 sec. 3.5.1 p. 27"},
                "supply_source": {"connector": "J2", "pin": "1"}, "reserved_pins": []},
        "voltage_domains": [{"id": "VDD_3V3", "nominal_millivolts": 3300, "tolerance_permille": 50}],
        "i2c_buses": [], "spi_buses": [], "i2c_devices": [], "spi_devices": [],
        "net_constraints": [],
        "connectors": [
            {"reference": "J1", "part": "SM06B-SRSS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "SWDIO", "offboard": null},
                {"pin": "5", "net": "SWCLK", "offboard": null},
                {"pin": "6", "net": "GND", "offboard": null}]},
            {"reference": "J2", "part": "SM04B-GHS-TB", "standard": null, "pins": [
                {"pin": "1", "net": "VDD_3V3", "offboard": null},
                {"pin": "2", "net": null, "offboard": null},
                {"pin": "3", "net": null, "offboard": null},
                {"pin": "4", "net": "GND", "offboard": null}]}
        ],
        "net_classes": [],
        "diff_pairs": [],
        "motor_outputs": motors,
        "power": null,
        "serial_ports": [],
    })
}

fn spec(motors: Vec<Value>) -> Specification {
    serde_json::from_value(spec_json(motors)).expect("a specification")
}

fn with_spi(motors: Vec<Value>, sck: Option<&str>) -> Specification {
    let mut v = spec_json(motors);
    v["spi_buses"] = json!([{"id": "S", "peripheral": null, "speed_hz": 1000000, "domain": "VDD_3V3",
        "sck": {"net": "S_SCK", "pin": sck}, "mosi": {"net": "S_MOSI", "pin": null},
        "miso": {"net": "S_MISO", "pin": null}}]);
    v["net_constraints"] = json!([
        {"net_name": "S_SCK", "max_capacitance_femtofarads": 30000, "max_rise_time_nanoseconds": 20, "impedance_ohms": null},
        {"net_name": "S_MOSI", "max_capacitance_femtofarads": 30000, "max_rise_time_nanoseconds": 20, "impedance_ohms": null}]);
    serde_json::from_value(v).expect("a specification")
}

fn refused(motors: Vec<Value>) -> IrValidationError {
    spec(motors).validate().expect_err("refused")
}

/// `M1`, a DShot output whose pull declares `ohms`, on `VDD_3V3` at
/// `nominal` mV +/- `tol` permille: what `validate` says (`AET-WO-4300j` D3
/// (a), the bound computed on the board's rail). Every edge below was worked
/// out outside the engine in exact fractions.
fn on_rail(nominal: u32, tol: u32, ohms: u32) -> Result<(), IrValidationError> {
    let mut m = dshot("M1", None);
    m["pull"]["resistor"]["ohms"] = json!(ohms);
    let mut v = spec_json(vec![m]);
    v["voltage_domains"][0]["nominal_millivolts"] = json!(nominal);
    v["voltage_domains"][0]["tolerance_permille"] = json!(tol);
    serde_json::from_value::<Specification>(v)
        .expect("a specification")
        .validate()
}

fn outside_on(nominal: u32, tol: u32, ohms: u32) -> String {
    let e = on_rail(nominal, tol, ohms).expect_err("refused");
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "MotorPullOutsideBound",
            Some("M1"),
            Some("motor_outputs[0].pull.resistor.ohms")
        ),
        "{nominal} +/-{tol}, {ohms}: {e}"
    );
    e.to_string()
}

/// The pull-up works from the board's rail; the ESC's thresholds sit on its
/// own, held as a stand-in at 3300 mV +/-50 (`AET-WO-4300j` F2, ruled
/// 12:25Z); and the band, not the bare value, is judged (F3): the low corner
/// (-1 % - 2 %) clears the floor and the high corner (+1 % + 2 %) the ceiling.
/// On the committed 3300 mV +/-50 the window is 17 325 000 / 41 =
/// 422 560.97... to 6 270 000 / 7 = 895 714.28... ohm, so the band admits
/// 435 630 to 869 625 ohm: 442 000 admitted, 432 000 refused. A board rail of
/// 3300 +/-1 holds a stronger pull-up, so the floor falls to 1 386 000 000 /
/// 4897 = 283 030.43... and the band admits 291 785 to 869 625, the ceiling
/// the ESC's alone: 221 000 is still refused, the value D1 (a) rejected. On
/// 3400 +/-50 the floor is 15 400 000 / 47 = 327 659.57..., and on 1800
/// +/-50 no pull-up reaches the ESC's threshold at all. Every edge was worked
/// out outside the engine in exact fractions.
#[test]
fn the_bound_moves_with_the_rail() {
    on_rail(3300, 50, 442_000).unwrap();
    for ohms in [435_630, 869_625] {
        on_rail(3300, 50, ohms).unwrap_or_else(|e| panic!("{ohms}: {e}"));
    }
    for ohms in [432_000, 435_629, 869_626, 422_561, 895_714] {
        let m = outside_on(3300, 50, ohms);
        assert!(m.contains("422561 to 895714 ohm"), "{m}");
    }
    let m = outside_on(3300, 1, 221_000);
    assert!(m.contains("283031 to 895714 ohm"), "{m}");
    assert!(m.contains("`VDD_3V3` (3300 mV +/-1 permille)"), "{m}");
    assert!(m.contains("peer rail 3300 mV +/-50 permille"), "{m}");
    for ohms in [291_785, 869_625] {
        on_rail(3300, 1, ohms).unwrap_or_else(|e| panic!("{ohms}: {e}"));
    }
    outside_on(3300, 1, 291_784);
    outside_on(3300, 1, 869_626);
    let m = outside_on(3400, 50, 221_000);
    assert!(m.contains("327660 to 895714 ohm"), "{m}");
    let e = on_rail(1800, 50, 442_000).unwrap_err();
    assert_eq!(e.kind(), "MotorPullFloorUnreachable", "{e}");
}

/// `MOTOR1`'s pull net is `MOTOR1_PD` (`AET-WO-4300j` D5), the engine's
/// own: a second output spelling it, in any case, is `ReservedName`.
#[test]
fn a_net_named_motor1_pd_is_refused_in_any_case() {
    for label in ["MOTOR1_PD", "motor1_pd", "Motor1_Pd"] {
        let e = refused(vec![dshot("MOTOR1", None), dshot(label, None)]);
        assert_eq!(e.kind(), "ReservedName", "{label}: {e}");
        assert!(e.to_string().contains("MOTOR1_PD"), "{label}: {e}");
    }
    spec(vec![dshot("MOTOR1", None), dshot("MOTOR2", None)])
        .validate()
        .unwrap();
}

/// A rail whose floor is above its ceiling admits nothing: the ceiling is
/// the ESC's, 6 270 000 / 7 = 895 714.28... ohm on its held 3300 +/-50, and a
/// board on 3000 mV +/-13 gives a floor of exactly 900 000. On 3000 +/-64 the
/// floor is 138 600 000, still a `u32`: empty. On 3000 +/-65 the weakest
/// pull-up cannot reach the threshold even with no pull-down: its
/// denominator is -2 000 000 nV. Each is refused by name, at `mcu.supply`,
/// for every value.
#[test]
fn an_empty_or_unreachable_bound_admits_nothing() {
    for (nominal, tol, kind, values) in [
        (3000, 13, "MotorPullBoundEmpty", [442_000, 895_714, 900_000]),
        (
            3000,
            64,
            "MotorPullBoundEmpty",
            [442_000, 895_714, 138_600_000],
        ),
        (
            3000,
            65,
            "MotorPullFloorUnreachable",
            [442_000, 1, u32::MAX],
        ),
    ] {
        for ohms in values {
            let e = on_rail(nominal, tol, ohms).expect_err("refused");
            assert_eq!(
                (e.kind(), e.id(), e.path()),
                (kind, Some("VDD_3V3"), Some("mcu.supply")),
                "{nominal} +/-{tol}, {ohms}: {e}"
            );
        }
    }
    let e = on_rail(3000, 13, 442_000).unwrap_err();
    assert!(
        e.to_string()
            .contains("900000 ohm, is above its ceiling, 895714 ohm"),
        "{e}"
    );
}

#[test]
fn a_protocol_outside_the_closed_set_is_refused() {
    // Missing keys are `validate.rs`'s `removing_any_key_of_any_object_is_refused`.
    let bad = json!({"label": "M1", "pin": null, "protocol": "dshot1200", "pwm": null});
    assert!(serde_json::from_value::<Specification>(spec_json(vec![bad])).is_err());
}

#[test]
fn a_motor_output_s_timing_is_refused_by_name() {
    let mut m = pwm("M1", None, 400);
    m["pwm"] = Value::Null;
    assert_eq!(refused(vec![m]).kind(), "MotorTimingShape");
    let mut m = dshot("M1", None);
    m["pwm"] = json!({"rate_hz": 400, "pulse_min_us": 1000, "pulse_max_us": 2000,
        "idle": {"pulse_us": 1000}, "resolution_ps": 1_000_000});
    assert_eq!(refused(vec![m]).kind(), "MotorTimingShape");
    assert_eq!(refused(vec![pwm("M1", None, 0)]).kind(), "ZeroMotorRate");
    let mut m = pwm("M1", None, 400);
    m["pwm"]["pulse_min_us"] = json!(2000);
    assert_eq!(refused(vec![m]).kind(), "PulseRange");
    let mut m = pwm("M1", None, 400);
    m["pwm"]["pulse_min_us"] = json!(0);
    assert_eq!(refused(vec![m]).kind(), "PulseRange");
    // 2000 us at 500 Hz is the whole 2 ms period.
    assert_eq!(
        refused(vec![pwm("M1", None, 500)]),
        IrValidationError::PulseExceedsPeriod {
            id: "M1".into(),
            max_us: 2000,
            rate_hz: 500
        }
    );
}

#[test]
fn a_motor_output_s_label_is_a_net_and_its_net_takes_no_constraint() {
    let e = refused(vec![pwm("M1", None, 400), dshot("m1", None)]);
    assert_eq!(e.kind(), "DuplicateNetName");
    assert_eq!(
        refused(vec![pwm("M/1", None, 400)]).kind(),
        "InvalidIdentifier"
    );
    assert_eq!(
        refused(vec![pwm("VDD_3V3", None, 400)]).kind(),
        "NetShadowsDomain"
    );
    let mut v = spec_json(vec![pwm("M1", None, 400)]);
    v["net_constraints"] = json!([{"net_name": "M1", "max_capacitance_femtofarads": 1000,
        "max_rise_time_nanoseconds": 10, "impedance_ohms": null}]);
    let s: Specification = serde_json::from_value(v).unwrap();
    assert_eq!(
        s.validate().unwrap_err(),
        IrValidationError::UnjudgedMotorConstraint { id: "M1".into() }
    );
}

/// `AET-WO-4240a` §1.2: `hse_hz`, the key `mcu.hse` replaced, reads as
/// `null` at 0, the only value it ever admitted; any other value is refused
/// at parsing, as is the key beside `hse`.
#[test]
fn the_version_19_hse_hz_reads_as_null_at_0_and_is_refused_otherwise() {
    let mut v = spec_json(vec![]);
    v["mcu"].as_object_mut().unwrap().remove("hse");
    v["mcu"]["hse_hz"] = json!(0);
    let s: Specification = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(s.mcu.hse, None);
    assert!(serde_json::to_value(&s).unwrap()["mcu"]["hse"].is_null());
    v["mcu"]["hse_hz"] = json!(8_000_000);
    let e = serde_json::from_value::<Specification>(v.clone()).unwrap_err();
    assert!(e.to_string().contains("mcu.hse_hz is 8000000"), "{e}");
    v["mcu"]["hse_hz"] = json!(0);
    v["mcu"]["hse"] = json!(null);
    let e = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(e.to_string().contains("duplicate field `hse`"), "{e}");
}

#[test]
fn four_outputs_at_one_rate_share_one_timer_and_carry_its_solved_values() {
    let s = spec(vec![
        pwm("M1", None, 400),
        pwm("M2", None, 400),
        pwm("M3", None, 400),
        pwm("M4", None, 400),
    ]);
    let t = solve(&s, &table(), HASH).expect("solvable");
    let placed: Vec<(&str, &str, &str)> = t
        .motor_outputs_in_order()
        .into_iter()
        .map(|(net, _, a)| (net, a.pin.as_str(), a.signal.as_str()))
        .collect();
    assert_eq!(
        placed,
        [
            ("M1", "PA8", "TIM1_CH1"),
            ("M2", "PA9", "TIM1_CH2"),
            ("M3", "PA10", "TIM1_CH3"),
            ("M4", "PA11", "TIM1_CH4"),
        ]
    );
    for (_, m, _) in t.motor_outputs_in_order() {
        assert_eq!((m.psc, m.arr, m.ccr), (63, 2499, 1000));
        assert!(check_silicon(&t.clock, 1, m).holds);
    }
    assert_eq!(t.clock, SolvedClock::reset());
    assert_eq!(t.assignments["M1"].alternate_function, 1);
    t.validate().expect("the solver's topology validates");
}

#[test]
fn outputs_at_two_rates_take_two_timers() {
    let s = spec(vec![
        pwm("M1", None, 400),
        dshot("M2", None),
        pwm("M3", None, 50),
    ]);
    let t = solve(&s, &table(), HASH).expect("solvable");
    let timer = |net: &str| t.assignments[net].peripheral.clone();
    assert_eq!(timer("M1"), "TIM1");
    assert_ne!(timer("M2"), "TIM1");
    assert_ne!(timer("M3"), "TIM1");
    assert_ne!(timer("M2"), timer("M3"));
    assert_eq!(
        (
            t.motor_outputs["M2"].psc,
            t.motor_outputs["M2"].arr,
            t.motor_outputs["M2"].ccr
        ),
        (0, 106, 40)
    );
}

/// `AET-WO-1211a`: `resolution_ps` joins the timer-share key; one rate at
/// two ticks cannot share TIM1's counter.
#[test]
fn a_resolution_conflict_on_one_timer_is_refused_by_name() {
    let mut fine = pwm("M2", Some("PA9"), 400);
    fine["pwm"]["resolution_ps"] = json!(500_000);
    let s = spec(vec![pwm("M1", Some("PA8"), 400), fine]);
    assert_eq!(
        solve(&s, &table(), HASH).unwrap_err(),
        SolveError::TimerResolutionConflict {
            timer: "TIM1".into(),
            first: "M1".into(),
            second: "M2".into()
        }
    );
}

#[test]
fn adversarial_3_a_rate_conflict_on_one_timer_is_refused_by_name() {
    // PA8 and PA9 offer only TIM1: two rates cannot share its counter.
    let s = spec(vec![
        pwm("M1", Some("PA8"), 400),
        pwm("M2", Some("PA9"), 50),
    ]);
    assert_eq!(
        solve(&s, &table(), HASH).unwrap_err(),
        SolveError::TimerRateConflict {
            timer: "TIM1".into(),
            first: "M1".into(),
            second: "M2".into()
        }
    );
    // The same rate shares it.
    let s = spec(vec![
        pwm("M1", Some("PA8"), 400),
        pwm("M2", Some("PA9"), 400),
    ]);
    solve(&s, &table(), HASH).expect("one rate");
}

#[test]
fn adversarial_3_a_pin_with_no_timer_channel_is_refused_by_name() {
    let e = solve(&spec(vec![pwm("M1", Some("PB7"), 400)]), &table(), HASH).unwrap_err();
    assert_eq!(e.kind(), "NoTimerChannel");
    assert!(
        e.to_string().contains("pin `PB7` carries no timer channel"),
        "{e}"
    );
    // A complementary output (TIM1_CH1N) is not a channel this engine drives.
    let e = solve(&spec(vec![pwm("M1", Some("PA7"), 400)]), &table(), HASH).unwrap_err();
    assert_eq!(e.kind(), "NoTimerChannel");
    // A reserved pin is refused as any hand pin is (`PC14`, the LSE's; `PA13`
    // is refused before it, at validate, as a pin with a reset pull,
    // `MotorPinResetPull`, `AET-WO-4300i` F2).
    let e = solve(&spec(vec![pwm("M1", Some("PC14"), 400)]), &table(), HASH).unwrap_err();
    assert_eq!(e.kind(), "ReservedPin");
}

#[test]
fn adversarial_3_a_timer_pin_shared_with_spi_is_resolved_or_named() {
    // TIM2_CH1 is on PA0 and PA5; SPI1_SCK only on PA5. The search keeps
    // PA5 for SPI and puts the output on PA0.
    let s = with_spi(
        vec![
            pwm("M1", None, 400),
            pwm("M2", None, 400),
            pwm("M3", None, 400),
            pwm("M4", None, 400),
            dshot("M5", None),
        ],
        None,
    );
    let t = solve(&s, &table(), HASH).expect("solvable");
    assert_eq!(t.assignments["S_SCK"].pin, "PA5");
    assert_eq!(t.assignments["M5"].pin, "PA0");
    // On a table where the output's only channel is SPI1_SCK's only pin,
    // each alone is placeable and the two together are not: named, with
    // both claimants.
    let mut small = table();
    small
        .pins
        .retain(|name, _| ["PA5", "PA6", "PA7"].contains(&name.as_str()));
    let s = with_spi(vec![dshot("M1", None)], None);
    let e = solve(&s, &small, HASH).unwrap_err();
    assert_eq!(e.kind(), "MotorsInfeasible", "{e}");
    assert_eq!(e.claimants(), ["S", "M1"]);
}

#[test]
fn a_rate_below_what_a_16_bit_timer_reaches_is_refused_before_the_search() {
    let e = solve(&spec(vec![pwm("M1", None, 15)]), &table(), HASH).unwrap_err();
    assert_eq!(e.kind(), "MotorTiming");
    assert_eq!(e.claimants(), ["M1"]);
}

fn topology() -> SolvedTopology {
    let s = spec(vec![pwm("M1", None, 400), pwm("M2", None, 400)]);
    solve(&s, &table(), HASH).unwrap()
}

#[test]
fn the_topology_refuses_a_motor_output_that_disagrees_with_its_assignment() {
    let kind = |edit: &dyn Fn(&mut SolvedTopology)| {
        let mut t = topology();
        edit(&mut t);
        t.validate().map_err(|e| e.kind())
    };
    assert_eq!(kind(&|_| {}), Ok(()));
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.remove("M2");
        }),
        Err("MotorOutputMismatch")
    );
    assert_eq!(
        kind(&|t| {
            t.assignments.get_mut("M2").unwrap().signal = "TIM1_CH1".into();
        }),
        Err("MotorOutputMismatch")
    );
    assert_eq!(
        kind(&|t| {
            t.assignments.get_mut("M2").unwrap().signal = "TIM1_CH2N".into();
        }),
        Err("MotorOutputMismatch")
    );
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().index = 0;
        }),
        Err("MotorOutputMismatch")
    );
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().ccr = 2500;
        }),
        Err("MotorOutputMismatch")
    );
    // A pulse idle's CCR may reach ARR (one tick short of the period,
    // `AET-WO-1211a`): the shape holds, and only the silicon check judges it.
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().ccr = 2499;
        }),
        Ok(())
    );
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().pwm = None;
        }),
        Err("MotorOutputMismatch")
    );
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().arr = 2498;
        }),
        Err("TimerShared")
    );
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().protocol = MotorProtocol::Dshot600;
            t.motor_outputs.get_mut("M2").unwrap().pwm = None;
        }),
        Err("TimerShared")
    );
    assert_eq!(
        kind(&|t| {
            t.clock.d2ppre2 = 2;
        }),
        Err("UnsupportedClock")
    );
    assert_eq!(
        kind(&|t| {
            t.clock.hse_hz = 8_000_000;
        }),
        Err("UnsupportedClock")
    );
    assert_eq!(
        kind(&|t| {
            t.clock.source = "PLL1".into();
        }),
        Err("UnsupportedClock")
    );
}

#[test]
fn a_version_4_topology_without_the_new_fields_is_refused_never_given_a_clock() {
    let mut v = serde_json::to_value(topology()).unwrap();
    v["schema_version"] = json!(4);
    v.as_object_mut().unwrap().remove("clock");
    v.as_object_mut().unwrap().remove("motor_outputs");
    // No stand-in clock under any version (`AET-WO-4240v` §1.1): a clock
    // carries its `vos`, and nothing supplies one, so the file is refused
    // at parsing, by the key it lacks.
    let e = serde_json::from_value::<SolvedTopology>(v.clone()).unwrap_err();
    assert!(e.to_string().contains("missing field `clock`"), "{e}");
    // Under the current version the missing keys are refused at parsing.
    v["schema_version"] = json!(aethelgard_ir::SOLVED_SCHEMA_VERSION);
    assert!(serde_json::from_value::<SolvedTopology>(v).is_err());
}

#[test]
fn the_solved_motor_output_round_trips_and_keeps_the_declared_timing() {
    let t = topology();
    let back: SolvedTopology = serde_json::from_str(&serde_json::to_string(&t).unwrap()).unwrap();
    assert_eq!(back, t);
    assert_eq!(
        t.motor_outputs["M1"].pwm,
        Some(PwmTiming {
            rate_hz: 400,
            pulse_min_us: 1000,
            pulse_max_us: 2000,
            idle: aethelgard_ir::motor::Idle::PulseUs(1000),
            resolution_ps: 1_000_000,
        })
    );
    let declared: Vec<MotorOutput> = spec(vec![pwm("M1", None, 400)]).motor_outputs;
    assert_eq!(declared[0].protocol, MotorProtocol::Pwm);
}

#[test]
fn a_device_reference_may_not_be_an_observer_s_name() {
    use aethelgard_ir::{IdentifierKind, check_identifier, is_observer_name};
    for name in ["PA8OBS", "pa8obs", "Pk15obs", "pa0obs"] {
        assert!(
            check_identifier(IdentifierKind::Reference, name).is_err(),
            "{name}"
        );
    }
    for name in [
        "pa16obs", "pl8obs", "pa08obs", "pa8ob", "pa8obs2", "pa_obs", "U2",
    ] {
        assert!(!is_observer_name(&name.to_ascii_lowercase()), "{name}");
        assert!(
            check_identifier(IdentifierKind::Reference, name).is_ok(),
            "{name}"
        );
    }
}

#[test]
fn adversarial_more_rates_than_timers_is_named_not_a_spent_budget() {
    // Adversarial pass (low): thirteen rates on the H753's twelve timers ran
    // the search out of budget. Five rates on this table's four timers are
    // named with no search proving it: within 100 nodes, which the search
    // alone spends (measured), every output a claimant (dropping any fits).
    let rates: Vec<Value> = (0..5)
        .map(|i| pwm(&format!("M{i}"), None, 400 + 10 * i))
        .collect();
    let e = solve_with_limit(&spec(rates), &table(), HASH, 100).unwrap_err();
    assert_eq!(e.kind(), "MotorsInfeasible", "{e}");
    assert_eq!(e.claimants(), ["M0", "M1", "M2", "M3", "M4"]);
}

#[test]
fn a_connector_ends_a_motor_net_with_no_off_board_capacitance() {
    // §1.8 after `4300f`: a motor output leaves the board at a connector
    // pin. It is on no bus, so it declares no off-board capacitance.
    let pin = |n: &str, net: Option<&str>, offboard: Value| json!({"pin": n, "net": net, "offboard": offboard});
    let with = |offboard: Value| {
        let mut s = spec_json(vec![pwm("M1", None, 400)]);
        s["connectors"].as_array_mut().unwrap().push(json!({
            "reference": "J3", "part": "SM04B-GHS-TB", "standard": null,
            "pins": [pin("1", Some("M1"), offboard), pin("2", None, Value::Null),
                     pin("3", None, Value::Null), pin("4", Some("GND"), Value::Null)],
        }));
        serde_json::from_value::<Specification>(s)
            .unwrap()
            .validate()
    };
    assert_eq!(with(Value::Null), Ok(()));
    let e =
        with(json!({"capacitance_femtofarads": 1000, "source": "condition: test"})).unwrap_err();
    assert_eq!(e.kind(), "ConnectorBudgetNotApplicable", "{e}");
}

#[test]
fn a_motor_output_named_gnd_is_refused() {
    // Second adversarial review, F2: a motor output labelled `GND` joined
    // the emitter's ground and left its pin unconnected.
    for label in ["GND", "gnd"] {
        let e = refused(vec![pwm(label, None, 400)]);
        assert_eq!(e.kind(), "NetShadowsDomain", "{label}: {e}");
    }
}

// ---- AET-WO-4400g: the clock decoded from the RCC, and a foreign clock ----

use aethelgard_ir::motor::{Apb, RccDecodeError, RccRegisters};

/// The six registers ArduPilot (ChibiOS `stm32h7_mcuconf.h`, `OSCILLATOR_HZ`
/// 0) leaves, as the twin read them back at a 5 s `--foreign` run's end
/// (`AET-WO-4400g`, measured): `RCC_CR` `0x3F003025` (HSI on, `HSIDIV` 0,
/// the PLLs on), `RCC_CFGR` `0x801B`, `RCC_D1CFGR` `0x48`,
/// `RCC_D2CFGR` `0x440`, `RCC_PLLCKSELR` `0x408040`, `RCC_PLL1DIVR`
/// `0x01090231`.
fn ardupilot() -> RccRegisters {
    RccRegisters::from_values([0x3F00_3025, 0x801B, 0x48, 0x440, 0x40_8040, 0x0109_0231])
}

/// §1.2's exact case: HSI 64 MHz ÷ `DIVM1` 4 × `DIVN1` 50 ÷ `DIVP1` 2 is
/// `sys_ck` 400 MHz; `HPRE` ÷2 gives `rcc_hclk1` 200 MHz; `D2PPRE1` and
/// `D2PPRE2` ÷2 with `TIMPRE` 1 give 200 MHz timers on both buses (Table 57).
#[test]
fn ardupilot_s_rcc_registers_decode_to_400_mhz_and_200_mhz_timers() {
    let c = SolvedClock::from_rcc(&ardupilot(), 0, aethelgard_ir::vos::Vos::Scale1).unwrap();
    assert_eq!(c.source, "PLL1");
    assert_eq!(c.sysclk_hz, 400_000_000);
    assert_eq!(
        (c.d1cpre, c.hpre, c.d1ppre, c.d2ppre1, c.d2ppre2),
        (1, 2, 2, 2, 2)
    );
    assert!(c.timpre);
    assert_eq!(c.hclk_hz(), 200_000_000);
    assert_eq!(c.core_hz(), 400_000_000);
    assert_eq!(c.timer_kernel_hz(Apb::Apb1), 200_000_000);
    assert_eq!(c.timer_kernel_hz(Apb::Apb2), 200_000_000);
}

/// `HSIDIV` divides HSI before it clocks the system or PLL1 (§8.7.2): 01 is
/// 32 MHz; and each APB bus's timers take that bus's prescaler (§8.7.9):
/// `D2PPRE1` ÷2 and `D2PPRE2` ÷4 with `TIMPRE` 0 give `rcc_hclk1` on APB1 and
/// half of it on APB2 (Table 57). Adversarial pass: both mutants survived
/// while every fixture had `HSIDIV` 0 and equal buses.
#[test]
fn hsidiv_and_each_bus_s_prescaler_are_read_apart() {
    let hsi32 = RccRegisters::from_values([0x0000_002D, 0, 0, 0, 0x0202_0200, 0x0101_0280]);
    assert_eq!(
        SolvedClock::from_rcc(&hsi32, 0, aethelgard_ir::vos::Vos::Scale1)
            .unwrap()
            .sysclk_hz,
        32_000_000
    );
    let mut r = ardupilot();
    r.cr |= 0b01 << 3;
    assert_eq!(
        SolvedClock::from_rcc(&r, 0, aethelgard_ir::vos::Vos::Scale1)
            .unwrap()
            .sysclk_hz,
        200_000_000
    );
    let mut r = ardupilot();
    r.cfgr &= !(1 << 15);
    r.d2cfgr = (0b101 << 8) | (0b100 << 4);
    let c = SolvedClock::from_rcc(&r, 0, aethelgard_ir::vos::Vos::Scale1).unwrap();
    assert_eq!((c.d2ppre1, c.d2ppre2), (2, 4));
    assert_eq!(c.timer_kernel_hz(Apb::Apb1), 200_000_000);
    assert_eq!(c.timer_kernel_hz(Apb::Apb2), 100_000_000);
}

/// Each register at its reset value (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.9,
/// §8.7.11, §8.7.13) decodes to the solved reset clock.
#[test]
fn the_reset_registers_decode_to_the_reset_clock() {
    // `RCC_CR` as Renode 1.17.0 reads it at reset: HSION, HSIRDY (which
    // mirrors HSION, `STM32H7_RCC.cs` L147) and HSIDIVF (always 1, L158).
    let r = RccRegisters::from_values([0x0000_0025, 0, 0, 0, 0x0202_0200, 0x0101_0280]);
    // The core's scale is declared, never decoded; at Scale 2 the decode is
    // the generated tree (`AET-WO-4240v`).
    assert_eq!(
        SolvedClock::from_rcc(&r, 0, aethelgard_ir::vos::Vos::Scale2),
        Ok(SolvedClock::reset())
    );
}

/// Every refusal of the decode, by name.
#[test]
fn a_register_the_decode_cannot_read_as_rm0433_states_it_is_refused_by_name() {
    let refused = |edit: &dyn Fn(&mut RccRegisters), hse: u32| {
        let mut r = ardupilot();
        edit(&mut r);
        SolvedClock::from_rcc(&r, hse, aethelgard_ir::vos::Vos::Scale1).unwrap_err()
    };
    let reserved = |field: &'static str, value: u32| RccDecodeError::Reserved { field, value };
    // SWS 100 is reserved; 010 is HSE, which no crystal backs.
    assert_eq!(
        refused(&|r| r.cfgr = 0x8023, 0),
        reserved("RCC_CFGR.SWS", 4)
    );
    assert_eq!(
        refused(
            &|r| {
                r.cfgr = 0x8012;
                r.cr |= 1 << 17;
            },
            0
        ),
        RccDecodeError::HseUndeclared
    );
    // PLLSRC 11 clocks no PLL; 10 is HSE; DIVM1 0 disables the prescaler.
    assert_eq!(
        refused(&|r| r.pllckselr = 0x40_8043, 0),
        reserved("RCC_PLLCKSELR.PLLSRC", 3)
    );
    assert_eq!(
        refused(&|r| r.pllckselr = 0x40_8042, 0),
        RccDecodeError::HseUndeclared
    );
    assert_eq!(
        refused(&|r| r.pllckselr = 0x40_8000, 0),
        reserved("RCC_PLLCKSELR.DIVM1", 0)
    );
    // DIVN1 below 0x003 is a wrong configuration; DIVP1 2 divides by 3, odd.
    assert_eq!(
        refused(&|r| r.pll1divr = 0x0109_0202, 0),
        reserved("RCC_PLL1DIVR.DIVN1", 2)
    );
    assert_eq!(
        refused(&|r| r.pll1divr = 0x0109_0431, 0),
        reserved("RCC_PLL1DIVR.DIVP1", 2)
    );
    // D1CPRE and HPRE of 11xx: Renode divides by 2^(n+1), RM0433 by 64 up.
    let unmodelled = |field: &'static str, value: u32| RccDecodeError::Unmodelled { field, value };
    assert_eq!(
        refused(&|r| r.d1cfgr = 0x4C, 0),
        unmodelled("RCC_D1CFGR.HPRE", 0xC)
    );
    assert_eq!(
        refused(&|r| r.d1cfgr = 0xC48, 0),
        unmodelled("RCC_D1CFGR.D1CPRE", 0xC)
    );
    // With a crystal declared, HSE decodes, once ready (HSERDY, bit 17).
    let mut r = ardupilot();
    r.cfgr = 0x8012;
    r.cr |= 1 << 17;
    assert_eq!(
        SolvedClock::from_rcc(&r, 25_000_000, aethelgard_ir::vos::Vos::Scale1)
            .unwrap()
            .sysclk_hz,
        25_000_000
    );
    // A source whose ready flag is clear (adversarial pass: Renode's SWS
    // mirrors SW): PLL1 off and not ready.
    assert_eq!(
        refused(&|r| r.cr = 0x0000_0025, 0),
        RccDecodeError::NotReady { clock: "PLL1" }
    );
    // DIVM1 at its reset 32, where Renode's prescaler may still be 1.
    assert_eq!(
        refused(&|r| r.pllckselr = 0x0202_0200, 0),
        unmodelled("RCC_PLLCKSELR.DIVM1", 32)
    );
    // A 64 MHz reference (DIVM1 1), and a 4.096 GHz VCO (DIVN1 x64).
    let range = |quantity: &'static str, hz: u64| RccDecodeError::OutOfRange { quantity, hz };
    assert_eq!(
        refused(&|r| r.pllckselr = 0x40_8010, 0),
        range("ref1_ck", 64_000_000)
    );
    assert_eq!(
        refused(
            &|r| {
                r.pllckselr = 0x40_8040;
                r.pll1divr = 0x0109_00FF;
            },
            0
        ),
        range("vco1_ck", 4_096_000_000)
    );
}

/// RM0433 Rev 9 Table 57, every row: `rcc_hclk1` while `D2PPREx` divides by
/// at most 2 (`TIMPRE` 0) or 4 (`TIMPRE` 1), else 2 or 4 × `rcc_pclkx`.
#[test]
fn the_timer_kernel_clock_is_table_57_for_both_timpre() {
    for (timpre, ppre, want) in [
        (false, 1, 64),
        (false, 2, 64),
        (false, 4, 32),
        (false, 8, 16),
        (false, 16, 8),
        (true, 1, 64),
        (true, 2, 64),
        (true, 4, 64),
        (true, 8, 32),
        (true, 16, 16),
    ] {
        let c = SolvedClock {
            timpre,
            d2ppre1: ppre,
            d2ppre2: ppre,
            ..SolvedClock::reset()
        };
        assert_eq!(
            c.timer_kernel_hz(Apb::Apb1),
            want * 1_000_000,
            "{timpre} {ppre}"
        );
        assert_eq!(
            c.timer_kernel_hz(Apb::Apb2),
            want * 1_000_000,
            "{timpre} {ppre}"
        );
    }
}

/// `INV-07` residue (a)(2) (`AET-WO-4400g` §1.6): `validate` refuses any
/// solved clock but the reset one, and says the second reason, so a future
/// clock-programming engine trips here instead of rediscovering (b): Renode's
/// RCC pushes its figure to a timer's counter on each RCC write, while the
/// counter's compare channels keep the frequency they were built at.
#[test]
fn a_clock_programming_firmware_is_refused_because_renode_s_rcc_push_reaches_counters_not_channels()
{
    let mut t = topology();
    // At Scale 0 every clock of ArduPilot's tree is within its maximum
    // (`AET-WO-4240v` §1.3), so the refusal is the tree's, not a limit's.
    // `D3PPRE` is not decoded (left 1), so it is set to ArduPilot's ÷2 here.
    t.clock = SolvedClock::from_rcc(&ardupilot(), 0, aethelgard_ir::vos::Vos::Scale0).unwrap();
    t.clock.d3ppre = 2;
    let e = t.validate().unwrap_err();
    assert_eq!(e.kind(), "UnsupportedClock");
    let m = e.to_string();
    assert!(
        m.contains("counters") && m.contains("compare channels") && m.contains("INV-07"),
        "{m}"
    );
}

fn foreign_clock(timers: u32, usart: u32) -> Value {
    let s = |v: u32| json!({"value": v, "source": "ChibiOS stm32h7_mcuconf.h"});
    let usarts: serde_json::Map<String, Value> = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), s(usart)))
        .collect();
    json!({"apb1_timers_hz": s(timers), "apb2_timers_hz": s(timers), "usarts": usarts, "vos": {"value": "scale1", "source": "ArduPilot Copter-4.7.2 stm32h7_mcuconf.h L109"}})
}

fn with_foreign_clock(c: Value) -> Value {
    let mut v = spec_json(vec![]);
    v["mcu"]["foreign_clock"] = c;
    v
}

/// `mcu.foreign_clock` (`AET-WO-4400g` §1.2): declared, it validates and
/// `solve` copies its values; absent, the topology's is `null`.
#[test]
fn a_declared_foreign_clock_validates_and_solve_copies_its_values() {
    let s: Specification =
        serde_json::from_value(with_foreign_clock(foreign_clock(200_000_000, 72_000_000))).unwrap();
    s.validate().unwrap();
    let t = solve(&s, &table(), HASH).unwrap();
    let c = t.foreign_clock.as_ref().unwrap();
    assert_eq!(
        (c.timer_hz(Apb::Apb1), c.timer_hz(Apb::Apb2)),
        (200_000_000, 200_000_000)
    );
    assert_eq!(c.usarts.len(), 8);
    assert!(c.usarts.values().all(|hz| *hz == 72_000_000));
    t.validate().unwrap();
    let text = serde_json::to_string(&t).unwrap();
    assert_eq!(serde_json::from_str::<SolvedTopology>(&text).unwrap(), t);
    assert!(topology().foreign_clock.is_none());
    let none = serde_json::to_value(topology()).unwrap();
    assert_eq!(none["foreign_clock"], Value::Null);
}

/// A topology carries the declaration's rules too: one written by hand
/// with a zero or a split mux is refused (adversarial pass: dropping the
/// check from `SolvedTopology::validate` survived).
#[test]
fn a_topology_s_foreign_clock_is_held_to_the_declaration_s_rules() {
    let s: Specification =
        serde_json::from_value(with_foreign_clock(foreign_clock(200_000_000, 72_000_000))).unwrap();
    let t = solve(&s, &table(), HASH).unwrap();
    let kind = |edit: &dyn Fn(&mut aethelgard_ir::motor::SolvedForeignClock)| {
        let mut t = t.clone();
        edit(t.foreign_clock.as_mut().unwrap());
        t.validate().map_err(|e| e.kind())
    };
    assert_eq!(kind(&|_| {}), Ok(()));
    assert_eq!(kind(&|c| c.apb1_timers_hz = 0), Err("ForeignClockZero"));
    assert_eq!(
        kind(&|c| {
            c.usarts.remove("UART4");
        }),
        Err("ForeignClockUsarts")
    );
    assert_eq!(
        kind(&|c| {
            c.usarts.insert("USART3".into(), 1);
        }),
        Err("ForeignClockMuxSplit")
    );
}

/// Each refusal of a declaration, by name.
#[test]
fn a_foreign_clock_no_silicon_could_run_is_refused_by_name() {
    let kind = |v: Value| {
        serde_json::from_value::<Specification>(with_foreign_clock(v))
            .unwrap()
            .validate()
            .unwrap_err()
            .kind()
    };
    let mut zero = foreign_clock(200_000_000, 72_000_000);
    zero["apb2_timers_hz"]["value"] = json!(0);
    assert_eq!(kind(zero), "ForeignClockZero");
    let mut missing = foreign_clock(200_000_000, 72_000_000);
    missing["usarts"].as_object_mut().unwrap().remove("UART8");
    assert_eq!(kind(missing), "ForeignClockUsarts");
    let mut unknown = foreign_clock(200_000_000, 72_000_000);
    unknown["usarts"]["LPUART1"] = json!({"value": 1, "source": "s"});
    assert_eq!(kind(unknown), "ForeignClockUsarts");
    let mut split = foreign_clock(200_000_000, 72_000_000);
    split["usarts"]["USART6"]["value"] = json!(64_000_000);
    assert_eq!(kind(split), "ForeignClockMuxSplit");
    // The other mux's six, USART234578SEL (adversarial pass: its loop
    // survived while only USART6 was split).
    let mut split = foreign_clock(200_000_000, 72_000_000);
    split["usarts"]["UART8"]["value"] = json!(64_000_000);
    assert_eq!(kind(split), "ForeignClockMuxSplit");
    let mut sourceless = foreign_clock(200_000_000, 72_000_000);
    sourceless["apb1_timers_hz"]["source"] = json!("");
    assert_eq!(kind(sourceless), "InvalidSource");
    // An explicit null is refused when parsing, as every optional key is.
    assert!(serde_json::from_value::<Specification>(with_foreign_clock(Value::Null)).is_err());
}

/// Specification 17: a version-16 file is refused by its version, with or
/// without `mcu.foreign_clock`, never read as today's board.
#[test]
fn a_version_16_file_is_refused_by_its_version() {
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 30);
    for c in [None, Some(foreign_clock(200_000_000, 72_000_000))] {
        let mut v = match c {
            Some(c) => with_foreign_clock(c),
            None => spec_json(vec![]),
        };
        v["schema_version"] = json!(16);
        let e = serde_json::from_value::<Specification>(v)
            .unwrap()
            .validate()
            .unwrap_err();
        assert_eq!(
            e,
            IrValidationError::UnsupportedSchemaVersion {
                changes: aethelgard_ir::versions::lacked_by(16),
                version: 16,
                expected: 30
            }
        );
    }
}

/// `AET-WO-2304b` §1.2: an output's `esc` is `null` or `{"none": {source}}`
/// with a source; an empty source, an unknown variant and an empty object
/// are refused, never read as a default.
#[test]
fn an_esc_is_null_or_none_with_a_source() {
    let with = |esc: Value| {
        let mut m = dshot("M1", Some("PE9"));
        m["esc"] = esc;
        serde_json::from_value::<Specification>(spec_json(vec![m]))
    };
    assert!(with(Value::Null).unwrap().validate().is_ok());
    assert!(
        with(json!({"none": {"source": "a bench header"}}))
            .unwrap()
            .validate()
            .is_ok()
    );
    assert!(matches!(
        with(json!({"none": {"source": ""}})).unwrap().validate(),
        Err(aethelgard_ir::IrValidationError::InvalidSource { ref path })
            if path == "motor_outputs[0].esc.none.source"
    ));
    for bad in [
        json!({}),
        json!({"part": {"source": "s"}}),
        json!({"none": {"source": "s"}, "x": 1}),
        json!({"none": {}}),
        json!("none"),
    ] {
        assert!(with(bad.clone()).is_err(), "{bad}");
    }
}

/// M2's solved pull, to edit.
fn solved_pull(t: &mut SolvedTopology) -> &mut aethelgard_ir::motor::SolvedMotorPull {
    &mut t.motor_outputs.get_mut("M2").unwrap().pull
}

/// `AET-WO-4300j` adversarial F1: the solved pull's strings reach hwdef's
/// comment line verbatim, and its window and value are written there as the
/// bound checked, so a topology is refused when any of them is not a
/// reference or disagrees with itself.
#[test]
fn the_topology_refuses_a_pull_that_is_not_references_or_disagrees_with_its_window() {
    let kind = |edit: &dyn Fn(&mut SolvedTopology)| {
        let mut t = topology();
        edit(&mut t);
        t.validate().map_err(|e| e.kind())
    };
    assert_eq!(kind(&|_| {}), Ok(()));
    // The injection the adversary planted: a live hwdef line in the link.
    assert_eq!(
        kind(&|t| {
            solved_pull(t).link = "R10\nPA8 TIM1_CH1 TIM1 PWM(1) SPEED_VERYLOW PUSHPULL PULLUP\ndefine HAL_SERIAL_ESC_COMM_ENABLED 1\n#".into();
        }),
        Err("InvalidIdentifier")
    );
    assert_eq!(
        kind(&|t| solved_pull(t).resistor = "R 6".into()),
        Err("InvalidIdentifier")
    );
    // The link the resistor's own reference, in any case; or another
    // output's link.
    assert_eq!(
        kind(&|t| {
            let r = solved_pull(t).resistor.to_lowercase();
            solved_pull(t).link = r;
        }),
        Err("DuplicateReference")
    );
    assert_eq!(
        kind(&|t| {
            let other = t.motor_outputs["M1"].pull.link.clone();
            solved_pull(t).link = other;
        }),
        Err("DuplicateReference")
    );
    // The window: at least 1, ordered, and the value inside it, both edges.
    let mismatch = Err("MotorPullMismatch");
    assert_eq!(kind(&|t| solved_pull(t).floor_ohms = 0), mismatch);
    assert_eq!(
        kind(&|t| {
            let c = solved_pull(t).ceiling_ohms;
            solved_pull(t).floor_ohms = c + 1;
        }),
        mismatch
    );
    assert_eq!(kind(&|t| solved_pull(t).ohms = 40_200), mismatch);
    assert_eq!(
        kind(&|t| {
            let f = solved_pull(t).floor_ohms;
            solved_pull(t).ohms = f - 1;
        }),
        mismatch
    );
    assert_eq!(
        kind(&|t| {
            let c = solved_pull(t).ceiling_ohms;
            solved_pull(t).ohms = c + 1;
        }),
        mismatch
    );
    assert_eq!(
        kind(&|t| {
            let f = solved_pull(t).floor_ohms;
            solved_pull(t).ohms = f;
        }),
        Ok(())
    );
    assert_eq!(
        kind(&|t| {
            let c = solved_pull(t).ceiling_ohms;
            solved_pull(t).ohms = c;
        }),
        Ok(())
    );
}
