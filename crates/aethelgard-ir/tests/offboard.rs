//! `AET-WO-4300i`: specification 28's off-board pins. Each refusal of
//! `validate`'s step 26 by name; `mcu.reserved_pins` in every solver branch
//! it changed; the FT-first order at a connector in every branch, each shown
//! against the same specification with the net taken off its connector (the
//! control), so the test fails if the order is removed; the motor pull's
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
    let e = refused(&v);
    assert_eq!(
        (e.kind(), e.id(), e.path()),
        ("PowerPartUnknown", Some("STM32H743ZITx"), Some("mcu.part")),
        "{e}"
    );
}

/// The motor channels: `TIM2_CH1` on `PA5` (`TT_ha`) comes before
/// `TIM2_CH2` on `PA0` (`FT_a`); `SERVO1` cannot share TIM1's key.
#[test]
fn a_connector_facing_motor_takes_an_ft_channel_first() {
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

/// The serial candidates: `PA4` and `PA7` (`TT_a`) sort before `PB14` and
/// `PB15`.
#[test]
fn a_connector_facing_port_takes_ft_pins_first() {
    let t = d1_table(&[
        ("PA4", 4, &[("USART1_TX", 4)], &[]),
        ("PA7", 8, &[("USART1_RX", 4)], &[]),
    ]);
    // A serial port always reaches a connector (`SerialPortNotAtConnector`),
    // so the key order alone would give `PA4` and `PA7`.
    let s = solved(&d1(), &t).unwrap();
    assert_eq!(
        (pin_of(&s, "CMD_TX"), pin_of(&s, "CMD_RX")),
        ("PB14", "PB15")
    );
}

/// A pin device's signal: `PC4` (`TT_a`) is the next free key before `PD0`.
#[test]
fn a_connector_facing_device_signal_takes_an_ft_pin_first() {
    let t = d1_table(&[("PC4", 45, &[], &[])]);
    let mut v = d1();
    face(&mut v, "DRV_NFAULT", false);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "DRV_NFAULT"), "PD0");
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "DRV_NFAULT"), "PC4");
}

/// A pin device's ADC input: `PB1` (`TT_a`) has the lowest position.
#[test]
fn a_connector_facing_adc_input_takes_an_ft_pin_first() {
    let t = d1_table(&[]);
    let mut v = d1();
    face(&mut v, "DRV_IPROPI", false);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "DRV_IPROPI"), "PC0");
    assert_eq!(pin_of(&solved(&d1(), &t).unwrap(), "DRV_IPROPI"), "PB1");
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

/// A chip select: `PA4` (`TT_a`) is the first free key, `PD0` the first FT.
#[test]
fn a_connector_facing_chip_select_takes_an_ft_pin_first() {
    let t = s02_table(&[("PA4", 4, &[], &[]), ("PD0", 50, &[], &[])]);
    assert_eq!(pin_of(&solved(&s02(), &t).unwrap(), "FLASH_CS"), "PA4");
    let mut v = s02();
    face(&mut v, "FLASH_CS", true);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "FLASH_CS"), "PD0");
}

/// A bus line: `PA4` (`TT_a`) carries `SPI1_MISO` before `PA6`.
#[test]
fn a_connector_facing_bus_line_takes_an_ft_pin_first() {
    let t = s02_table(&[("PA4", 4, &[("SPI1_MISO", 5)], &[]), ("PD0", 50, &[], &[])]);
    assert_eq!(pin_of(&solved(&s02(), &t).unwrap(), "SPI_MISO"), "PA4");
    let mut v = s02();
    face(&mut v, "SPI_MISO", true);
    assert_eq!(pin_of(&solved(&v, &t).unwrap(), "SPI_MISO"), "PA6");
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
