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
    ];
    PinTable {
        part: "TEST_PART".into(),
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

fn pwm(label: &str, pin: Option<&str>, rate_hz: u32) -> Value {
    json!({"label": label, "pin": pin, "protocol": "pwm",
           "pwm": {"rate_hz": rate_hz, "pulse_min_us": 1000, "pulse_max_us": 2000}})
}

fn dshot(label: &str, pin: Option<&str>) -> Value {
    json!({"label": label, "pin": pin, "protocol": "dshot600", "pwm": null})
}

fn spec_json(motors: Vec<Value>) -> Value {
    json!({
        "schema_version": aethelgard_ir::SPECIFICATION_SCHEMA_VERSION,
        "accept_lifecycle": [],
        "acknowledge_unwitnessed": [],
        "project_name": "t",
        "board": serde_json::from_str::<Value>(BOARD).unwrap(),
        "mcu": {"part": "TEST_PART", "supply": "VDD_3V3", "hse_hz": 0, "bor_level": 0,
                "supply_source": {"connector": "J2", "pin": "1"}},
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
    m["pwm"] = json!({"rate_hz": 400, "pulse_min_us": 1000, "pulse_max_us": 2000});
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

#[test]
fn a_crystal_is_refused_by_name_until_a_crystal_part_exists() {
    let mut v = spec_json(vec![]);
    v["mcu"]["hse_hz"] = json!(8_000_000);
    let s: Specification = serde_json::from_value(v).unwrap();
    assert_eq!(
        s.validate().unwrap_err(),
        IrValidationError::CrystalNotSupported { hse_hz: 8_000_000 }
    );
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
    // A reserved pin is refused as any hand pin is.
    let e = solve(&spec(vec![pwm("M1", Some("PA13"), 400)]), &table(), HASH).unwrap_err();
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
    assert_eq!(
        kind(&|t| {
            t.motor_outputs.get_mut("M2").unwrap().ccr = 2499;
        }),
        Err("MotorOutputMismatch")
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
}

#[test]
fn a_version_4_topology_without_the_new_fields_is_refused_as_a_version() {
    let mut v = serde_json::to_value(topology()).unwrap();
    v["schema_version"] = json!(4);
    v.as_object_mut().unwrap().remove("clock");
    v.as_object_mut().unwrap().remove("motor_outputs");
    let t: SolvedTopology = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(t.validate().unwrap_err().kind(), "UnsupportedSchemaVersion");
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
            pulse_max_us: 2000
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
    let c = SolvedClock::from_rcc(&ardupilot(), 0).unwrap();
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

/// Each register at its reset value (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.9,
/// §8.7.11, §8.7.13) decodes to the solved reset clock.
#[test]
fn the_reset_registers_decode_to_the_reset_clock() {
    let r = RccRegisters::from_values([0x0000_0001, 0, 0, 0, 0x0202_0200, 0x0101_0280]);
    assert_eq!(SolvedClock::from_rcc(&r, 0), Ok(SolvedClock::reset()));
}

/// Every refusal of the decode, by name.
#[test]
fn a_register_the_decode_cannot_read_as_rm0433_states_it_is_refused_by_name() {
    let refused = |edit: &dyn Fn(&mut RccRegisters), hse: u32| {
        let mut r = ardupilot();
        edit(&mut r);
        SolvedClock::from_rcc(&r, hse).unwrap_err()
    };
    let reserved = |field: &'static str, value: u32| RccDecodeError::Reserved { field, value };
    // SWS 100 is reserved; 010 is HSE, which no crystal backs.
    assert_eq!(
        refused(&|r| r.cfgr = 0x8023, 0),
        reserved("RCC_CFGR.SWS", 4)
    );
    assert_eq!(
        refused(&|r| r.cfgr = 0x8012, 0),
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
    // With a crystal declared, HSE decodes.
    let mut r = ardupilot();
    r.cfgr = 0x8012;
    assert_eq!(
        SolvedClock::from_rcc(&r, 25_000_000).unwrap().sysclk_hz,
        25_000_000
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
    t.clock = SolvedClock::from_rcc(&ardupilot(), 0).unwrap();
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
    json!({"apb1_timers_hz": s(timers), "apb2_timers_hz": s(timers), "usarts": usarts})
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
    assert_eq!(aethelgard_ir::SPECIFICATION_SCHEMA_VERSION, 17);
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
                version: 16,
                expected: 17
            }
        );
    }
}
