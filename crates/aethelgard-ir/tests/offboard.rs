//! `AET-WO-4300i`: specification 28's off-board pins. Each refusal of
//! `validate`'s step 26 by name; `mcu.reserved_pins` in every solver branch
//! it changed; FT pins only at a connector in every branch (adversarial pass
//! F1): each branch never takes a TT pin there, and is refused naming the
//! pins it withheld when only TT pins are free, each shown against the same
//! specification with the net taken off its connector (the control), which
//! takes the TT pin; no motor output on a reset pull (F2), and an
//! idle-high one pulled down like any other (F3 reverted); the motor pull's
//! parse and its row; `silicon::implied_pupd` on a motor net; and the
//! power-parts row's `io.weak_pull_down_ohms`.
//!
//! D1 is `tests/fixtures/d1-drv8876.json` on `pin_devices_solve.rs`'s small
//! synthetic table: `DRV_EN` and `DRV_PH` drive the bridge only, `SERVO1`
//! reaches `J4.1`, and the console `CMD` reaches `J1.1` and `J1.2`. The
//! Sentinel-02 base is `sense_pin.rs`'s.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::{
    IrValidationError, Pin, PinSource, PinTable, SolveError, SolvedTopology, Specification, solve,
};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// `(pin, position, [(signal, af)], [adc channel])`.
type Row<'a> = (&'a str, u16, &'a [(&'a str, u8)], &'a [&'a str]);

fn table_of(rows: &[Row<'_>]) -> PinTable {
    PinTable {
        part: "STM32H753ZITx".into(),
        package: "TEST".into(),
        source: PinSource {
            repository: "test".into(),
            commit: "test".into(),
            files: BTreeMap::new(),
        },
        pins: rows
            .iter()
            .map(|(name, position, afs, adc)| {
                let pin = Pin {
                    position: *position,
                    alternate_functions: afs.iter().map(|(s, af)| ((*s).into(), *af)).collect(),
                    adc_channels: adc.iter().map(|c| (*c).to_owned()).collect(),
                };
                ((*name).into(), pin)
            })
            .collect(),
    }
}

/// D1's console, its timer channels, plain pins, one ADC1 pin (`PB1`,
/// `TT_a`, the lowest position) and one ADC3 pin (`PC0`, `FT_a`), and
/// `extra`. `PE9` is `TT_ha`; every other pin here is FT.
fn d1_table(extra: &[Row<'_>]) -> PinTable {
    let mut rows: Vec<Row<'_>> = vec![
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PB14", 20, &[("USART1_TX", 4)], &[]),
        ("PB15", 21, &[("USART1_RX", 4)], &[]),
        ("PE9", 30, &[("TIM1_CH1", 1)], &[]),
        ("PE11", 31, &[("TIM1_CH2", 1)], &[]),
        ("PE13", 32, &[("TIM1_CH3", 1)], &[]),
        ("PC6", 33, &[("TIM3_CH1", 2)], &[]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
        ("PD0", 50, &[], &[]),
        ("PD1", 51, &[], &[]),
        ("PD2", 52, &[], &[]),
    ];
    rows.extend_from_slice(extra);
    table_of(&rows)
}

fn d1() -> Value {
    serde_json::from_str(include_str!("fixtures/d1-drv8876.json")).unwrap()
}

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

fn solved(v: &Value, t: &PinTable) -> Result<SolvedTopology, SolveError> {
    solve(&spec(v), t, HASH)
}

fn refused(v: &Value) -> IrValidationError {
    spec(v).validate().expect_err("refused")
}

/// The solved pin of `net`, wherever the solver put it.
fn pin_of<'a>(s: &'a SolvedTopology, net: &str) -> &'a str {
    s.assignments
        .get(net)
        .map(|a| a.pin.as_str())
        .or_else(|| s.gpio.get(net).map(|g| g.pin.as_str()))
        .or_else(|| s.adc_inputs.get(net).map(|a| a.pin.as_str()))
        .or_else(|| {
            s.power
                .as_ref()
                .and_then(|p| p.sense.get(net))
                .map(|x| x.pin.as_str())
        })
        .unwrap_or_else(|| panic!("`{net}` is not solved"))
}

/// `net` taken off every connector pin that names it: the control.
fn unface(v: &mut Value, net: &str) {
    for c in v["connectors"].as_array_mut().unwrap() {
        for p in c["pins"].as_array_mut().unwrap() {
            if p["net"] == net {
                p["net"] = Value::Null;
            }
        }
    }
}

/// A new four-pin connector `J90` with `net` on its pin 1 (and `GND` on 4),
/// `offboard` declared when `line` (an I2C or SPI line or a chip select).
fn face(v: &mut Value, net: &str, line: bool) {
    let offboard = if line {
        json!({"capacitance_femtofarads": 30000, "source": "condition: test"})
    } else {
        Value::Null
    };
    v["connectors"].as_array_mut().unwrap().push(json!(
        {"reference": "J90", "part": "SM04B-GHS-TB", "standard": null, "pins": [
            {"pin": "1", "net": net, "offboard": offboard},
            {"pin": "2", "net": null, "offboard": null},
            {"pin": "3", "net": null, "offboard": null},
            {"pin": "4", "net": "GND", "offboard": null}]}
    ));
}

fn reserve(v: &mut Value, pin: &str, reason: &str) {
    v["mcu"]["reserved_pins"]
        .as_array_mut()
        .unwrap()
        .push(json!({"pin": pin, "reason": reason}));
}

// ------------------------------------------------------------ the motor pull

#[test]
fn the_fixture_validates_and_solves() {
    let s = solved(&d1(), &d1_table(&[])).unwrap();
    s.validate().unwrap();
    assert_eq!(pin_of(&s, "DRV_EN"), "PE9");
    assert_eq!(pin_of(&s, "SERVO1"), "PC6");
}

#[test]
fn a_pull_that_is_not_to_gnd_is_refused_by_name() {
    let mut v = d1();
    v["motor_outputs"][0]["pull"]["resistor"]["to"] = json!("VDD_3V3");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "MotorPullNotDown",
            Some("DRV_EN"),
            Some("motor_outputs[0].pull.resistor.to")
        ),
        "{e}"
    );
}

#[test]
fn a_pull_whose_part_is_not_a_resistor_row_is_refused() {
    // A capacitor row, then no row at all.
    for part in ["GCM188R71H104KA57", "NOT_A_PART"] {
        let mut v = d1();
        v["motor_outputs"][2]["pull"]["resistor"]["part"] = json!(part);
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.id(), e.path()),
            (
                "PowerPartUnknown",
                Some("SERVO1"),
                Some("motor_outputs[2].pull.resistor.part")
            ),
            "{part}: {e}"
        );
        assert!(e.to_string().contains("is not a resistor row"), "{e}");
    }
}

#[test]
fn a_pull_outside_rpd_is_refused_and_its_edges_are_held() {
    for ohms in [29_999, 50_001] {
        let mut v = d1();
        v["motor_outputs"][1]["pull"]["resistor"]["ohms"] = json!(ohms);
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.id(), e.path()),
            (
                "MotorPullOutsideRpd",
                Some("DRV_PH"),
                Some("motor_outputs[1].pull.resistor.ohms")
            ),
            "{ohms}: {e}"
        );
        let m = e.to_string();
        assert!(m.contains("30000 to 50000"), "{m}");
        assert!(
            m.contains("Table 49 p. 137") && m.contains("Table 146 p. 241"),
            "{m}"
        );
    }
    for ohms in [30_000, 50_000] {
        let mut v = d1();
        v["motor_outputs"][1]["pull"]["resistor"]["ohms"] = json!(ohms);
        spec(&v)
            .validate()
            .unwrap_or_else(|e| panic!("{ohms}: {e}"));
    }
}

#[test]
fn a_missing_or_null_pull_is_refused_at_parse() {
    let mut v = d1();
    v["motor_outputs"][0]
        .as_object_mut()
        .unwrap()
        .remove("pull");
    let e = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(e.to_string().contains("missing field `pull`"), "{e}");
    let mut v = d1();
    v["motor_outputs"][0]["pull"] = Value::Null;
    assert!(serde_json::from_value::<Specification>(v).is_err());
    // No `mcu` arm (D0): the pin device's other arm is an unknown key here.
    let mut v = d1();
    v["motor_outputs"][0]["pull"]["mcu"] = json!("down");
    let e = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(e.to_string().contains("unknown field `mcu`"), "{e}");
}

#[test]
fn a_blank_source_or_reference_is_refused_by_the_house_rules() {
    for (edit, kind, path) in [
        (
            (|v: &mut Value| v["motor_outputs"][0]["pull"]["source"] = json!("")) as fn(&mut Value),
            "InvalidSource",
            "motor_outputs[0].pull.source",
        ),
        (
            |v: &mut Value| v["motor_outputs"][0]["pull"]["source"] = json!(" padded"),
            "InvalidSource",
            "motor_outputs[0].pull.source",
        ),
        (
            |v: &mut Value| v["motor_outputs"][0]["pull"]["resistor"]["reference"] = json!(""),
            "InvalidIdentifier",
            "motor_outputs[0].pull.resistor.reference",
        ),
        (
            |v: &mut Value| v["motor_outputs"][0]["pull"]["resistor"]["part"] = json!(""),
            "InvalidIdentifier",
            "motor_outputs[0].pull.resistor.part",
        ),
        (
            |v: &mut Value| v["motor_outputs"][0]["pull"]["resistor"]["to"] = json!(""),
            "InvalidIdentifier",
            "motor_outputs[0].pull.resistor.to",
        ),
    ] {
        let mut v = d1();
        edit(&mut v);
        let e = refused(&v);
        assert_eq!((e.kind(), e.path()), (kind, Some(path)), "{e}");
    }
}

#[test]
fn a_pull_reference_another_part_holds_is_refused() {
    // `R3` is the nFAULT pull-up's reference.
    let mut v = d1();
    v["motor_outputs"][2]["pull"]["resistor"]["reference"] = json!("R3");
    assert_eq!(refused(&v).kind(), "DuplicateReference");
    // Two outputs on one reference.
    let mut v = d1();
    v["motor_outputs"][2]["pull"]["resistor"]["reference"] = json!("R4");
    assert_eq!(refused(&v).kind(), "DuplicateReference");
}

#[test]
fn a_motor_net_has_no_internal_pull() {
    let s = solved(&d1(), &d1_table(&[])).unwrap();
    for net in ["DRV_EN", "DRV_PH", "SERVO1"] {
        assert_eq!(aethelgard_ir::silicon::implied_pupd(&s, net), 0, "{net}");
    }
}

// ------------------------------------------------------- declared reservations

#[test]
fn a_reservation_of_an_unknown_pin_is_refused_by_solve() {
    let mut v = d1();
    reserve(&mut v, "PZ9", "test");
    spec(&v).validate().unwrap();
    let e = solved(&v, &d1_table(&[])).unwrap_err();
    assert_eq!(
        e,
        SolveError::UnknownPin {
            pin: "PZ9".into(),
            claimant: "mcu.reserved_pins[0]".into()
        }
    );
}

#[test]
fn a_pin_reserved_twice_is_refused() {
    let mut v = d1();
    reserve(&mut v, "PD2", "first");
    reserve(&mut v, "PD2", "second");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "ReservedPinTwice",
            Some("PD2"),
            Some("mcu.reserved_pins[1].pin")
        ),
        "{e}"
    );
}

#[test]
fn a_blank_reserved_pin_is_refused() {
    let mut v = d1();
    reserve(&mut v, "", "test");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.path()),
        ("InvalidIdentifier", Some("mcu.reserved_pins[0].pin")),
        "{e}"
    );
}

#[test]
fn a_blank_reason_is_refused() {
    for reason in ["", " ", "trailing "] {
        let mut v = d1();
        reserve(&mut v, "PD2", reason);
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.path()),
            ("InvalidSource", Some("mcu.reserved_pins[0].reason")),
            "{reason:?}: {e}"
        );
    }
}

#[test]
fn a_pin_the_engine_reserves_is_refused() {
    for pin in aethelgard_ir::RESERVED_PINS {
        let mut v = d1();
        reserve(&mut v, pin, "test");
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.id(), e.path()),
            (
                "ReservedPinEngineReserved",
                Some(pin),
                Some("mcu.reserved_pins[0].pin")
            ),
            "{e}"
        );
    }
    // `PH0` only beside a declared HSE: with `mcu.hse` null it is the
    // board's to reserve.
    let mut v = d1();
    reserve(&mut v, "PH0", "test");
    spec(&v).validate().unwrap();
}

#[test]
fn a_hand_pin_on_a_declared_reservation_names_its_reason() {
    for (edit, claimant) in [
        (
            (|v: &mut Value| v["motor_outputs"][2]["pin"] = json!("PC6")) as fn(&mut Value),
            "motor_outputs[2]",
        ),
        (
            |v: &mut Value| v["pin_devices"][0]["signals"][1]["pin"] = json!("PC6"),
            "pin_devices[0].signals[1]",
        ),
    ] {
        let mut v = d1();
        edit(&mut v);
        reserve(&mut v, "PC6", "kept for the bench probe");
        let e = solved(&v, &d1_table(&[])).unwrap_err();
        assert_eq!(e.kind(), "ReservedPin", "{e}");
        assert_eq!(e.claimants(), [claimant.to_owned()]);
        assert_eq!(
            e.to_string(),
            format!(
                "`{claimant}` names pin `PC6`, which is reserved (`mcu.reserved_pins`: kept for the bench probe)"
            )
        );
    }
}

/// Each branch: the baseline takes the pin, and with it reserved the same
/// claimant takes another. `PD5` is a second `USART1_TX`.
#[test]
fn the_solver_never_offers_a_declared_reservation() {
    let t = d1_table(&[("PD5", 70, &[("USART1_TX", 4)], &[])]);
    let base = solved(&d1(), &t).unwrap();
    for (pin, net) in [
        // the timer channels (`Model::new`'s reservation)
        ("PE9", "DRV_EN"),
        // the serial candidates (`Model::new`'s)
        ("PB14", "CMD_TX"),
        // a pin device's ADC input and signal (`solve`'s, through `place`)
        ("PB1", "DRV_IPROPI"),
        ("PD0", "DRV_NFAULT"),
    ] {
        assert_eq!(pin_of(&base, net), pin, "{net}: the baseline");
        let mut v = d1();
        reserve(&mut v, pin, "test");
        let s = solved(&v, &t).unwrap_or_else(|e| panic!("{pin}: {e}"));
        assert_ne!(pin_of(&s, net), pin, "{net}");
        let taken: BTreeSet<&str> = s
            .assignments
            .values()
            .map(|a| a.pin.as_str())
            .chain(s.gpio.values().map(|g| g.pin.as_str()))
            .chain(s.adc_inputs.values().map(|a| a.pin.as_str()))
            .collect();
        assert!(!taken.contains(pin), "{pin}: {taken:?}");
    }
}

// ----------------------------------------------------------- FT at connectors

#[test]
fn a_hand_tt_pin_at_a_connector_is_refused_by_name() {
    let mut v = d1();
    v["motor_outputs"][2]["pin"] = json!("PE9");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "ConnectorPinNotFt",
            Some("SERVO1"),
            Some("motor_outputs[2].pin")
        ),
        "{e}"
    );
    let m = e.to_string();
    assert!(
        m.contains("`SERVO1` reaches connector pin J4.1 on `PE9`, which is TT_ha"),
        "{m}"
    );
    // The same pin on a net with no connector end validates.
    let mut v = d1();
    v["motor_outputs"][0]["pin"] = json!("PE9");
    spec(&v).validate().unwrap();
    // Every hand claimant at a connector, a serial line among them.
    let mut v = d1();
    v["serial_ports"][0]["rx"]["pin"] = json!("PA7");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "ConnectorPinNotFt",
            Some("CMD_RX"),
            Some("serial_ports[0].rx.pin")
        )
    );
}

/// The family is the structure before its first `_`, so a bare `FT` (`PH1`,
/// DS12117 Table 8) is FT.
#[test]
fn a_hand_ft_pin_of_bare_structure_at_a_connector_validates() {
    let mut v = d1();
    v["motor_outputs"][2]["pin"] = json!("PH1");
    spec(&v).validate().unwrap();
}

#[test]
fn a_hand_pin_at_a_connector_with_no_held_structure_is_refused() {
    let mut v = d1();
    v["motor_outputs"][2]["pin"] = json!("PI0");
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        (
            "McuPinStructureUnheld",
            Some("SERVO1"),
            Some("motor_outputs[2].pin")
        ),
        "{e}"
    );
}

#[test]
fn an_mcu_with_no_row_is_refused_where_the_row_is_needed() {
    // Without the pin device, whose step 24 names the part first.
    let mut v = d1();
    v["pin_devices"] = json!([]);
    unface(&mut v, "M1_OUT1");
    unface(&mut v, "M1_OUT2");
    v["mcu"]["part"] = json!("STM32H743ZITx");
    let unknown = |v: &Value| {
        let e = refused(v);
        assert_eq!(
            (e.kind(), e.id(), e.path()),
            ("PowerPartUnknown", Some("STM32H743ZITx"), Some("mcu.part")),
            "{e}"
        );
    };
    unknown(&v);
    // With no motor output and no hand pin, the console's solved lines still
    // reach `J1` (adversarial pass F8): the solver judges them on the row.
    v["motor_outputs"] = json!([]);
    unface(&mut v, "SERVO1");
    unknown(&v);
    // With no claimant at a connector, nothing needs the row.
    v["serial_ports"] = json!([]);
    unface(&mut v, "CMD_TX");
    unface(&mut v, "CMD_RX");
    spec(&v).validate().unwrap();
}

/// What every refusal for want of a pin at a connector says of the pins it
/// withheld (`AET-WO-4300i` D1, adversarial pass F1).
const WITHHELD: &str = "withheld as not FT at a connector (`AET-WO-4300i` D1)";

/// The motor channels: `TIM2_CH1` on `PA5` (`TT_ha`) comes before
/// `TIM2_CH2` on `PA0` (`FT_a`); `SERVO1` cannot share TIM1's key. At its
/// connector it is never offered `PA5`; off it (the control), it takes it.
#[test]
fn a_connector_facing_motor_is_offered_ft_channels_only() {
    let t = d1_table(&[
        ("PA5", 6, &[("TIM2_CH1", 1)], &[]),
        ("PA0", 7, &[("TIM2_CH2", 1)], &[]),
    ]);
    let s = solved(&d1(), &t).unwrap();
    assert_eq!(pin_of(&s, "SERVO1"), "PA0");
    let mut v = d1();
    unface(&mut v, "SERVO1");
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "SERVO1"), "PA5");
}

/// `SERVO1`'s only free channels are `PA5` and `PE9`, both `TT_ha`: the
/// bridge's outputs hold `PE11` and `PE13` by hand, and `PC6` is reserved.
#[test]
fn a_connector_facing_motor_whose_only_channels_are_tt_is_refused() {
    let t = d1_table(&[("PA5", 6, &[("TIM2_CH1", 1)], &[])]);
    let mut v = d1();
    v["motor_outputs"][0]["pin"] = json!("PE11");
    v["motor_outputs"][1]["pin"] = json!("PE13");
    reserve(&mut v, "PC6", "test");
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(
        (e.kind(), e.id()),
        ("NoTimerChannel", Some("SERVO1")),
        "{e}"
    );
    let m = e.to_string();
    assert!(
        m.contains(&format!("`PA5` (TT_ha), `PE9` (TT_ha) {WITHHELD}")),
        "{m}"
    );
    unface(&mut v, "SERVO1");
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "SERVO1"), "PA5");
}

/// The serial candidates: `PA4` and `PA7` (`TT_a`) sort before `PB14` and
/// `PB15`. A serial port always reaches a connector
/// (`SerialPortNotAtConnector`), so the key order alone would give `PA4`
/// and `PA7`, and there is no off-connector control.
#[test]
fn a_connector_facing_port_is_offered_ft_pins_only() {
    let t = d1_table(&[
        ("PA4", 4, &[("USART1_TX", 4)], &[]),
        ("PA7", 8, &[("USART1_RX", 4)], &[]),
    ]);
    let s = solved(&d1(), &t).unwrap();
    assert_eq!(
        (pin_of(&s, "CMD_TX"), pin_of(&s, "CMD_RX")),
        ("PB14", "PB15")
    );
}

/// With `PB14` (then `PB15`) reserved, the port's only TX (then RX) pin is
/// `PA4` (then `PA7`), `TT_a`: refused, naming it. Unreserved, it solves.
#[test]
fn a_connector_facing_port_whose_only_pin_is_tt_is_refused() {
    let t = d1_table(&[
        ("PA4", 4, &[("USART1_TX", 4)], &[]),
        ("PA7", 8, &[("USART1_RX", 4)], &[]),
    ]);
    for (ft, tt, sig) in [("PB14", "PA4", "TX"), ("PB15", "PA7", "RX")] {
        let mut v = d1();
        reserve(&mut v, ft, "test");
        let e = solved(&v, &t).unwrap_err();
        assert_eq!(
            (e.kind(), e.id()),
            ("NoSerialPeripheral", Some("CMD")),
            "{e}"
        );
        let m = e.to_string();
        assert!(
            m.contains(&format!(
                "USART1: the table has no unreserved pin for `USART1_{sig}`; `{tt}` (TT_a) {WITHHELD}"
            )),
            "{m}"
        );
    }
    solved(&d1(), &t).unwrap();
}

/// A pin device's signal: `PC4` (`TT_a`) is the next free key before `PD0`.
#[test]
fn a_connector_facing_device_signal_is_offered_ft_pins_only() {
    let t = d1_table(&[("PC4", 45, &[], &[])]);
    let mut v = d1();
    face(&mut v, "DRV_NFAULT", false);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "DRV_NFAULT"), "PD0");
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "DRV_NFAULT"), "PC4");
}

/// With `PD0` to `PD2` and `PE13` reserved, `DRV_NFAULT`'s only free pin is `PC4`
/// (`TT_a`): at a connector it is refused, naming it; off it, it takes it.
#[test]
fn a_connector_facing_device_signal_whose_only_pin_is_tt_is_refused() {
    let t = d1_table(&[("PC4", 45, &[], &[])]);
    let mut v = d1();
    for p in ["PD0", "PD1", "PD2", "PE13"] {
        reserve(&mut v, p, "test");
    }
    let control = v.clone();
    face(&mut v, "DRV_NFAULT", false);
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(
        (e.kind(), e.claimants()),
        ("NoFreePin", vec!["pin_devices[0].signals[1]".to_owned()]),
        "{e}"
    );
    let m = e.to_string();
    assert!(m.contains(&format!("`PC4` (TT_a) {WITHHELD}")), "{m}");
    assert_eq!(pin_of(&solved(&control, &t).unwrap(), "DRV_NFAULT"), "PC4");
}

/// A pin with no held structure counts as non-FT: `PI0` (no LQFP144 pin,
/// so DS12117 Table 8 holds none for it) is `DRV_NFAULT`'s only free pin.
#[test]
fn a_pin_with_no_held_structure_is_withheld_at_a_connector() {
    let t = d1_table(&[("PI0", 46, &[], &[])]);
    let mut v = d1();
    for p in ["PD0", "PD1", "PD2", "PE13"] {
        reserve(&mut v, p, "test");
    }
    let control = v.clone();
    face(&mut v, "DRV_NFAULT", false);
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(e.kind(), "NoFreePin", "{e}");
    let m = e.to_string();
    assert!(
        m.contains(&format!("`PI0` (no held structure) {WITHHELD}")),
        "{m}"
    );
    assert_eq!(pin_of(&solved(&control, &t).unwrap(), "DRV_NFAULT"), "PI0");
}

/// A pin device's ADC input: `PB1` (`TT_a`) has the lowest position.
#[test]
fn a_connector_facing_adc_input_is_offered_ft_pins_only() {
    let t = d1_table(&[]);
    let mut v = d1();
    face(&mut v, "DRV_IPROPI", false);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "DRV_IPROPI"), "PC0");
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "DRV_IPROPI"), "PB1");
}

/// With `PC0` reserved, `DRV_IPROPI`'s only ADC pin is `PB1` (`TT_a`).
#[test]
fn a_connector_facing_adc_input_whose_only_pin_is_tt_is_refused() {
    let t = d1_table(&[]);
    let mut v = d1();
    reserve(&mut v, "PC0", "test");
    let control = v.clone();
    face(&mut v, "DRV_IPROPI", false);
    let e = solved(&v, &t).unwrap_err();
    assert_eq!((e.kind(), e.id()), ("NoAdcPin", Some("DRV_IPROPI")), "{e}");
    let m = e.to_string();
    assert!(m.contains(&format!("`PB1` (TT_a) {WITHHELD}")), "{m}");
    assert_eq!(pin_of(&solved(&control, &t).unwrap(), "DRV_IPROPI"), "PB1");
}

/// Sentinel-02's power fixture without its crossing and IMU, as
/// `sense_pin.rs` builds it.
fn s02() -> Value {
    let mut v: Value =
        serde_json::from_str(include_str!("fixtures/sentinel-02-power.json")).unwrap();
    v["power"]["crossings"] = json!([]);
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U13" && l["reference"] != "R40");
    v["accept_lifecycle"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["part"] != "ICM20948");
    for key in ["spi_devices", "spi_buses", "net_constraints"] {
        let kept: Vec<Value> = v[key]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| {
                !(x["reference"] == "U5"
                    || x["id"] == "IMU"
                    || x["net_name"].as_str().is_some_and(|n| n.starts_with("IMU")))
            })
            .cloned()
            .collect();
        v[key] = Value::Array(kept);
    }
    v
}

/// The flash's SPI1 and the I2C bus, and `extra`; an ADC pin (`PF11`) for
/// the battery sense when nothing else is.
fn s02_table(extra: &[Row<'_>]) -> PinTable {
    let mut rows: Vec<Row<'_>> = vec![
        ("PA5", 5, &[("SPI1_SCK", 5)], &[]),
        ("PA6", 6, &[("SPI1_MISO", 5)], &[]),
        ("PA7", 7, &[("SPI1_MOSI", 5)], &[]),
        ("PB6", 8, &[("I2C1_SCL", 4)], &[]),
        ("PB7", 9, &[("I2C1_SDA", 4)], &[]),
        ("PF11", 40, &[], &["ADC1_INP2"]),
    ];
    rows.extend_from_slice(extra);
    table_of(&rows)
}

/// Each of `nets` on a pin of a new four-pin connector `J91` (pins 1 to 3,
/// `GND` on 4), each with an off-board load, as a bus line declares one.
fn face_lines(v: &mut Value, nets: &[&str]) {
    let mut pins: Vec<Value> = (0..3)
        .map(|k| match nets.get(k) {
            Some(net) => json!({"pin": (k + 1).to_string(), "net": net,
                "offboard": {"capacitance_femtofarads": 30000, "source": "condition: test"}}),
            None => json!({"pin": (k + 1).to_string(), "net": null, "offboard": null}),
        })
        .collect();
    pins.push(json!({"pin": "4", "net": "GND", "offboard": null}));
    v["connectors"]
        .as_array_mut()
        .unwrap()
        .push(json!({"reference": "J91", "part": "SM04B-GHS-TB", "standard": null, "pins": pins}));
}

/// A chip select: `PA4` (`TT_a`) is the first free key, `PD0` the first FT.
#[test]
fn a_connector_facing_chip_select_is_offered_ft_pins_only() {
    let t = s02_table(&[("PA4", 4, &[], &[]), ("PD0", 50, &[], &[])]);
    assert_eq!(pin_of(&solved(&s02(), &t).unwrap(), "FLASH_CS"), "PA4");
    let mut v = s02();
    face(&mut v, "FLASH_CS", true);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "FLASH_CS"), "PD0");
}

/// The chip select's free pins are `PA4` and `PB1` (`TT_a`), with `PF11`
/// reserved: at a connector it is refused, naming both; off it, it takes
/// `PA4`, and the battery sense `PB1`.
#[test]
fn a_connector_facing_chip_select_whose_only_pins_are_tt_is_refused() {
    let t = s02_table(&[("PA4", 4, &[], &[]), ("PB1", 3, &[], &["ADC1_INP5"])]);
    let mut v = s02();
    reserve(&mut v, "PF11", "test");
    let control = v.clone();
    face(&mut v, "FLASH_CS", true);
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(
        (e.kind(), e.claimants()),
        ("NoFreePin", vec!["spi_devices[0].cs".to_owned()]),
        "{e}"
    );
    let m = e.to_string();
    assert!(
        m.contains(&format!("`PA4` (TT_a), `PB1` (TT_a) {WITHHELD}")),
        "{m}"
    );
    assert_eq!(pin_of(&solved(&control, &t).unwrap(), "FLASH_CS"), "PA4");
}

/// A bus line: `PA4` (`TT_a`) carries `SPI1_MISO` before `PA6`.
#[test]
fn a_connector_facing_bus_line_is_offered_ft_pins_only() {
    let t = s02_table(&[("PA4", 4, &[("SPI1_MISO", 5)], &[]), ("PD0", 50, &[], &[])]);
    assert_eq!(pin_of(&solved(&s02(), &t).unwrap(), "SPI_MISO"), "PA4");
    let mut v = s02();
    face(&mut v, "SPI_MISO", true);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "SPI_MISO"), "PA6");
}

/// The adversarial pass's probe `p2` (F1): the flash's lines at a
/// connector, SPI1's only SCK on `PA5` (`TT_ha`), SPI2 on FT pins
/// (`PD0` to `PD2`, `FT_h`). The first instance cannot serve it, so it
/// moves to the next, where every line is FT; off the connector (the
/// control) it stays on SPI1. With no SPI2, it is refused, naming `PA5`.
#[test]
fn a_connector_facing_bus_takes_another_instance_rather_than_a_tt_pin() {
    let spi2: [Row<'_>; 3] = [
        ("PD0", 50, &[("SPI2_SCK", 5)], &[]),
        ("PD1", 51, &[("SPI2_MISO", 5)], &[]),
        ("PD2", 52, &[("SPI2_MOSI", 5)], &[]),
    ];
    let mut extra: Vec<Row<'_>> = spi2.to_vec();
    extra.push(("PE2", 60, &[], &[]));
    let t = s02_table(&extra);
    let lines = ["SPI_SCK", "SPI_MOSI", "SPI_MISO"];
    let base = solved(&s02(), &t).unwrap();
    assert_eq!(base.assignments["SPI_SCK"].peripheral, "SPI1");
    let mut v = s02();
    face_lines(&mut v, &lines);
    let s = solved(&v, &t).unwrap();
    let got: Vec<(&str, &str)> = lines
        .iter()
        .map(|n| (pin_of(&s, n), s.assignments[*n].peripheral.as_str()))
        .collect();
    assert_eq!(got, [("PD0", "SPI2"), ("PD2", "SPI2"), ("PD1", "SPI2")]);
    let t = s02_table(&[("PE2", 60, &[], &[])]);
    let e = solved(&v, &t).unwrap_err();
    assert_eq!((e.kind(), e.id()), ("NoInstance", Some("FLASH")), "{e}");
    let m = e.to_string();
    assert!(
        m.contains(&format!(
            "SPI1: the table has no unreserved pin for `SPI1_SCK`; `PA5` (TT_ha) {WITHHELD}"
        )),
        "{m}"
    );
}

/// The chip select and the battery sense read `solve`'s reservation.
#[test]
fn the_chip_select_and_the_sense_never_take_a_declared_reservation() {
    let t = s02_table(&[
        ("PA0", 2, &[], &[]),
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PD0", 50, &[], &[]),
    ]);
    let base = solved(&s02(), &t).unwrap();
    for (pin, net) in [("PA0", "FLASH_CS"), ("PB1", "VBAT_SENSE")] {
        assert_eq!(pin_of(&base, net), pin, "{net}: the baseline");
        let mut v = s02();
        reserve(&mut v, pin, "test");
        let s = solved(&v, &t).unwrap();
        assert_ne!(pin_of(&s, net), pin, "{net}");
    }
}

// ------------------------------------------------- the motor's reset pulls

/// RM0433 Rev 9 p. 542: `PA15` and `PB4` reset pulled up, `PA14` down. A
/// motor output pinned on one is refused (`AET-WO-4300i` F2).
#[test]
fn a_motor_pinned_on_a_reset_pull_is_refused_by_name() {
    for pin in ["PB4", "PA15", "PA14"] {
        let mut v = d1();
        v["motor_outputs"][0]["pin"] = json!(pin);
        let e = refused(&v);
        assert_eq!(
            (e.kind(), e.id(), e.path()),
            (
                "MotorPinResetPull",
                Some("DRV_EN"),
                Some("motor_outputs[0].pin")
            ),
            "{pin}: {e}"
        );
        assert!(e.to_string().contains("RM0433 Rev 9 p. 542"), "{e}");
    }
    let mut v = d1();
    v["motor_outputs"][0]["pin"] = json!("PE11");
    spec(&v).validate().unwrap();
}

/// `TIM2_CH1` sorts before TIM3's channel on `PC6`, so `SERVO1` takes it on
/// `PA0` (the control); on `PA15`, an FT pin pulled up from reset, it is
/// never offered, and `SERVO1` keeps `PC6`.
#[test]
fn the_solver_never_offers_a_motor_a_reset_pull() {
    let t = d1_table(&[("PA0", 7, &[("TIM2_CH1", 1)], &[])]);
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "SERVO1"), "PA0");
    let t = d1_table(&[("PA15", 7, &[("TIM2_CH1", 1)], &[])]);
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "SERVO1"), "PC6");
}

/// An idle-high output validates and solves with its pull to `GND` and no
/// internal pull (F3 reverted by the PM, 2026-11-23): D0 holds every motor
/// line low from power-up, whatever its idle.
#[test]
fn a_motor_idling_high_validates_pulled_down() {
    let mut v = d1();
    v["motor_outputs"][2]["pwm"]["idle"] = json!({"level": "high"});
    assert_eq!(v["motor_outputs"][2]["pull"]["resistor"]["to"], "GND");
    let s = solved(&v, &d1_table(&[])).unwrap();
    assert_eq!(aethelgard_ir::silicon::implied_pupd(&s, "SERVO1"), 0);
}

// ------------------------------------------------------ the power-parts row

#[test]
fn the_mcu_row_holds_rpd_cited_and_refuses_it_malformed() {
    use aethelgard_ir::power_parts::{Kind, POWER_PARTS_JSON, parse_power_parts, table};
    let Some(Kind::Mcu(m)) = table().unwrap().get("STM32H753ZITx").map(|r| &r.kind) else {
        panic!("the MCU row");
    };
    let w = &m.io.weak_pull_down_ohms;
    assert_eq!((w.min, w.typical, w.max), (30_000, 40_000, 50_000));
    assert_eq!(w.at.document, "DS12117");
    assert!(w.at.section.contains("RPD"), "{}", w.at.section);
    let row = |t: &Value| {
        t["parts"]
            .as_array()
            .unwrap()
            .iter()
            .position(|p| p["part"] == "STM32H753ZITx")
            .unwrap()
    };
    let at = json!({"document": "DS12117", "section": "test"});
    for bad in [
        json!({"min": 0, "typical": 40000, "max": 50000, "at": at}),
        json!({"min": 45000, "typical": 40000, "max": 50000, "at": at}),
        json!({"min": 30000, "typical": 55000, "max": 50000, "at": at}),
        json!({"min": 30000, "typical": 40000, "max": 50000,
               "at": {"document": "NOT_HELD", "section": "test"}}),
        json!({"min": 30000, "typical": 40000, "max": 50000, "at": at, "extra": 1}),
        json!({"min": 30000, "max": 50000, "at": at}),
    ] {
        let mut t: Value = serde_json::from_str(POWER_PARTS_JSON).unwrap();
        let r = row(&t);
        t["parts"][r]["kind"]["mcu"]["io"]["weak_pull_down_ohms"] = bad.clone();
        assert!(parse_power_parts(&t.to_string()).is_err(), "{bad}");
    }
    let mut t: Value = serde_json::from_str(POWER_PARTS_JSON).unwrap();
    let r = row(&t);
    t["parts"][r]["kind"]["mcu"]["io"]
        .as_object_mut()
        .unwrap()
        .remove("weak_pull_down_ohms");
    assert!(parse_power_parts(&t.to_string()).is_err());
}

/// A specification without `mcu.reserved_pins`, or with it `null`, is
/// refused at parse: nothing defaults (`INV-06`).
#[test]
fn reserved_pins_is_a_required_key() {
    let mut v = d1();
    v["mcu"].as_object_mut().unwrap().remove("reserved_pins");
    let e = serde_json::from_value::<Specification>(v).unwrap_err();
    assert!(
        e.to_string().contains("missing field `reserved_pins`"),
        "{e}"
    );
    let mut v = d1();
    v["mcu"]["reserved_pins"] = Value::Null;
    assert!(serde_json::from_value::<Specification>(v).is_err());
    let mut v = d1();
    v["mcu"]["reserved_pins"] = json!([{"pin": "PD2"}]);
    assert!(serde_json::from_value::<Specification>(v).is_err());
}
