//! `AET-WO-1211a`: the solver's half of a pin device, on a small synthetic
//! `PinTable` (this crate may depend on no other member; D1 and D2 on the
//! real table are `aethelgard-cli`'s `pin_devices_solve.rs`). The
//! specification is D1 (DRV8876, PH/EN, `tests/fixtures/d1-drv8876.json`);
//! the battery sense's choice is shown on `sense_pin.rs`'s Sentinel-02 base.
//!
//! Every new placement comes after every existing one; an ADC input takes
//! ADC1, then ADC3, and an ADC2-only pin only when the specification pins it
//! (`1211` Q8); no driven input of the device sits on a pin the MCU pulls up
//! from reset (PA13, PA15, PB4).

use std::collections::BTreeMap;

use aethelgard_ir::motor::Level;
use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedAdcInput, SolvedGpio};
use aethelgard_ir::{Pin, PinSource, PinTable, SolveError, SolvedTopology, Specification, solve};
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

/// D1's console, its three timer channels, plain pins, and `extra`.
fn table(extra: &[Row<'_>]) -> PinTable {
    let mut rows: Vec<Row<'_>> = vec![
        ("PB14", 20, &[("USART1_TX", 4)], &[]),
        ("PB15", 21, &[("USART1_RX", 4)], &[]),
        ("PE9", 30, &[("TIM1_CH1", 1)], &[]),
        ("PE11", 31, &[("TIM1_CH2", 1)], &[]),
        ("PE13", 32, &[("TIM1_CH3", 1)], &[]),
        ("PC6", 33, &[("TIM3_CH1", 2)], &[]),
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

fn adc(pin: &str, peripheral: &str, channel: &str) -> SolvedAdcInput {
    SolvedAdcInput {
        pin: pin.into(),
        peripheral: peripheral.into(),
        channel: channel.into(),
        // Every call is D1's input, declared at 12 bits (`AET-WO-1211b` S1).
        bits: Some(12),
        transfer: Some(aethelgard_ir::pin_devices::SolvedTransfer::CurrentOutput {
            device: "U2".into(),
        }),
    }
}

fn analog(v: &mut Value) -> &mut Value {
    &mut v["pin_devices"][0]["analog"][0]
}

fn signal(v: &mut Value, k: usize) -> &mut Value {
    &mut v["pin_devices"][0]["signals"][k]
}

/// One ADC1 pin and one ADC3 pin.
fn two_adc_pins() -> PinTable {
    table(&[
        ("PB1", 3, &[], &["ADC1_INP5", "ADC2_INP5"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ])
}

#[test]
fn the_device_s_input_then_signals_come_after_every_other_pin() {
    let t = two_adc_pins();
    let s = solved(&d1(), &t).unwrap();
    s.validate().unwrap();
    // The motor outputs and the console are placed first; the IPROPI takes
    // the lowest ADC pin, then the signals the next free keys in order.
    assert_eq!(s.assignments["DRV_EN"].peripheral, "TIM1");
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PB1", "ADC1", "INP5"));
    assert_eq!(s.gpio["DRV_NSLEEP"], SolvedGpio::output("PC0", Level::Low));
    assert_eq!(
        s.gpio["DRV_NFAULT"],
        SolvedGpio {
            pin: "PD0".into(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::None),
            // `AET-WO-1211c` D7: active low, and R3 to VDD_3V3 holds it high.
            active: Some(Level::Low),
            pulled: Some(Level::High),
        }
    );
    assert_eq!(s.pin_devices["U2"].part, "DRV8876PWPR");
    assert_eq!(
        s.pin_devices["U2"].supplies,
        [("VM".to_owned(), "VMOTOR".to_owned())].into()
    );
    assert_eq!(s.pin_speeds["DRV_NFAULT"], 0);
}

#[test]
fn an_adc2_only_pin_is_never_chosen_unpinned() {
    // `PB2` is the lowest ADC pin, ADC2 alone; the solver passes it.
    let t = table(&[
        ("PB2", 2, &[], &["ADC2_INP2"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ]);
    let s = solved(&d1(), &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC0", "ADC3", "INP10"));
}

#[test]
fn only_adc2_pins_left_is_adc_unmodelled() {
    let t = table(&[("PB2", 2, &[], &["ADC2_INP2"])]);
    let e = solved(&d1(), &t).unwrap_err();
    assert_eq!(
        e,
        SolveError::AdcUnmodelled {
            net: "DRV_IPROPI".into(),
            pins: vec!["PB2".into()]
        }
    );
    assert_eq!((e.kind(), e.id()), ("AdcUnmodelled", Some("DRV_IPROPI")));
}

#[test]
fn an_adc2_only_pin_pinned_by_hand_is_taken() {
    let t = table(&[("PB2", 2, &[], &["ADC2_INP2"])]);
    let mut v = d1();
    analog(&mut v)["pin"] = json!("PB2");
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PB2", "ADC2", "INP2"));
}

/// `1211` Q8: "unless the specification pins it". Naming an ADC2 channel
/// with no pin is no pin.
#[test]
fn an_adc2_channel_named_without_a_pin_is_adc_unmodelled() {
    let t = table(&[("PB2", 2, &[], &["ADC2_INP2"])]);
    let mut v = d1();
    analog(&mut v)["channel"] = json!({"peripheral": "ADC2", "input": "INP2"});
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "AdcUnmodelled");
}

#[test]
fn a_pin_with_both_takes_adc1() {
    let t = table(&[("PC0", 40, &[], &["ADC1_INP10", "ADC2_INP10", "ADC3_INP0"])]);
    let s = solved(&d1(), &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC0", "ADC1", "INP10"));
}

#[test]
fn a_channel_named_off_its_pin_is_refused() {
    let t = two_adc_pins();
    let mut v = d1();
    analog(&mut v)["pin"] = json!("PC0");
    analog(&mut v)["channel"] = json!({"peripheral": "ADC1", "input": "INP10"});
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(e.kind(), "AdcChannelNotOnPin", "{e}");
    // A pin with no ADC input at all.
    let mut v = d1();
    analog(&mut v)["pin"] = json!("PD2");
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "AdcChannelNotOnPin");
}

#[test]
fn a_named_channel_finds_its_pin() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC1", 41, &[], &["ADC3_INP11"]),
    ]);
    let mut v = d1();
    analog(&mut v)["channel"] = json!({"peripheral": "ADC3", "input": "INP11"});
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC1", "ADC3", "INP11"));
}

#[test]
fn no_adc_pin_left_is_no_adc_pin() {
    let e = solved(&d1(), &table(&[])).unwrap_err();
    assert_eq!(
        e,
        SolveError::NoAdcPin {
            net: "DRV_IPROPI".into()
        }
    );
}

#[test]
fn a_device_s_hand_pin_is_not_given_to_a_motor_output() {
    // `PE9` is TIM1's first channel; pinned to nSLEEP, DRV_EN and DRV_PH
    // take TIM1's other two.
    let t = two_adc_pins();
    let mut v = d1();
    signal(&mut v, 0)["pin"] = json!("PE9");
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.gpio["DRV_NSLEEP"].pin, "PE9");
    assert_ne!(s.assignments["DRV_EN"].pin, "PE9");
    assert_ne!(s.assignments["DRV_PH"].pin, "PE9");
}

#[test]
fn a_device_s_hand_pin_claimed_twice_or_reserved_is_refused() {
    let t = two_adc_pins();
    let mut v = d1();
    signal(&mut v, 0)["pin"] = json!("PD0");
    signal(&mut v, 1)["pin"] = json!("PD0");
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "PinClaimedTwice");
    let mut v = d1();
    signal(&mut v, 0)["pin"] = json!("PA14");
    let t = table(&[("PB1", 3, &[], &["ADC1_INP5"]), ("PA14", 60, &[], &[])]);
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "ReservedPin");
}

/// RM0433 Rev 9 p. 542: PA15 and PB4 reset pulled up. An unpinned output
/// signal passes them though they are the first free keys; pinned there,
/// or a bridge input's motor output there, is refused (`1211` §2.6).
#[test]
fn no_driven_input_sits_on_a_reset_pull_up() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PA15", 61, &[], &[]),
        ("PB4", 62, &[("TIM3_CH1", 2)], &[]),
        ("PC7", 63, &[("TIM3_CH2", 2)], &[]),
    ]);
    let s = solved(&d1(), &t).unwrap();
    // SERVO1, no driver input, takes PB4's TIM3 channel; nSLEEP passes
    // PA15, the first free key, for PC6.
    assert_eq!(s.assignments["SERVO1"].pin, "PB4");
    assert_eq!(s.gpio["DRV_NSLEEP"].pin, "PC6");
    let mut v = d1();
    signal(&mut v, 0)["pin"] = json!("PA15");
    assert_eq!(
        solved(&v, &t).unwrap_err(),
        SolveError::UnheldDuringReset {
            net: "DRV_NSLEEP".into(),
            pin: "PA15".into()
        }
    );
    let mut v = d1();
    let en = v["motor_outputs"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|m| m["label"] == "DRV_EN")
        .unwrap();
    en["pin"] = json!("PB4");
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "UnheldDuringReset");
    // An input (nFAULT) may sit there: the MCU reads it.
    let mut v = d1();
    signal(&mut v, 1)["pin"] = json!("PA15");
    solved(&v, &t).unwrap();
}

// ---------------------------------------------------------------- AET-WO-1217 F4

/// D1 with Sentinel-01's SPI flash on its own bus, so a chip select is
/// placed too.
fn d1_with_flash() -> Value {
    let mut v = d1();
    let s01: Value =
        serde_json::from_str(include_str!("../../../examples/sentinel-01.json")).unwrap();
    for key in ["spi_buses", "spi_devices", "accept_lifecycle"] {
        v[key] = s01[key].clone();
    }
    v["net_constraints"] = json!(
        s01["net_constraints"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|c| c["net_name"].as_str().unwrap().starts_with("SPI_"))
            .collect::<Vec<_>>()
    );
    v
}

/// `PC2` and `PC3` (the `PC2_C` and `PC3_C` pads, the MCU row's
/// `io.analog_pads`) carrying a timer channel, an SPI line, a USART line and
/// an ADC input each, first in every order the solver walks; SPI1 elsewhere.
fn with_pads() -> PinTable {
    table(&[
        (
            "PC2",
            28,
            &[("TIM1_CH1", 1), ("SPI1_MISO", 5), ("USART1_TX", 7)],
            &["ADC3_INP0"],
        ),
        (
            "PC3",
            29,
            &[("SPI1_MOSI", 5), ("USART1_RX", 7)],
            &["ADC3_INP1"],
        ),
        ("PE2", 60, &[("SPI1_SCK", 5)], &[]),
        ("PE5", 61, &[("SPI1_MISO", 5)], &[]),
        ("PE6", 62, &[("SPI1_MOSI", 5)], &[]),
    ])
}

/// F4: no digital claimant (a motor output, a bus line, a chip select, a
/// device's GPIO) is offered an analog pad, and an ADC input still is.
#[test]
fn an_analog_pad_takes_an_adc_input_and_nothing_digital() {
    let s = solved(&d1_with_flash(), &with_pads()).unwrap();
    s.validate().unwrap();
    for (net, pin) in s
        .assignments
        .iter()
        .map(|(n, a)| (n, &a.pin))
        .chain(s.gpio.iter().map(|(n, g)| (n, &g.pin)))
    {
        assert!(pin != "PC2" && pin != "PC3", "{net} on {pin}");
    }
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC2", "ADC3", "INP0"));
    assert_eq!(s.assignments["SPI_MISO"].pin, "PE5");
    assert_eq!(s.assignments["DRV_EN"].pin, "PE9");
}

/// F4: a digital hand pin on an analog pad is refused by name, whoever
/// names it; an ADC input pinned there is placed.
#[test]
fn a_digital_hand_pin_on_an_analog_pad_is_refused() {
    let t = with_pads();
    for (claimant, edit) in [
        (
            "pin_devices[0].signals[0]",
            (|v: &mut Value| signal(v, 0)["pin"] = json!("PC2")) as fn(&mut Value),
        ),
        ("spi_buses[0].miso", |v: &mut Value| {
            v["spi_buses"][0]["miso"]["pin"] = json!("PC2")
        }),
        ("spi_devices[0].cs", |v: &mut Value| {
            v["spi_devices"][0]["cs"]["pin"] = json!("PC3")
        }),
        ("serial_ports[0].tx", |v: &mut Value| {
            v["serial_ports"][0]["tx"]["pin"] = json!("PC2")
        }),
    ] {
        let mut v = d1_with_flash();
        edit(&mut v);
        let e = solved(&v, &t).unwrap_err();
        assert_eq!(e.kind(), "AnalogPadPin", "{claimant}: {e}");
        assert_eq!(e.claimants(), [claimant.to_owned()], "{claimant}: {e}");
    }
    let mut v = d1_with_flash();
    analog(&mut v)["pin"] = json!("PC3");
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC3", "ADC3", "INP1"));
}

/// F4: a topology moved onto an analog pad after `solve` is refused too.
#[test]
fn a_topology_with_a_digital_signal_on_an_analog_pad_is_refused() {
    let mut s = solved(&d1_with_flash(), &with_pads()).unwrap();
    s.gpio.get_mut("DRV_NSLEEP").unwrap().pin = "PC3".into();
    let e = s.validate().unwrap_err();
    assert_eq!(e.kind(), "AnalogPadPin", "{e}");
    assert_eq!(e.id(), Some("PC3"), "{e}");
}

/// F4's data: the MCU row's `io.analog_pads` is Table 8's two `ANA` pins,
/// cited; a pad that is not a held pin, or one named twice, is refused.
#[test]
fn the_analog_pads_are_table_8s_and_refused_malformed() {
    use aethelgard_ir::power_parts::{Kind, POWER_PARTS_JSON, parse_power_parts, table};
    let Some(Kind::Mcu(m)) = table().unwrap().get("STM32H753ZITx").map(|r| &r.kind) else {
        panic!("the MCU row");
    };
    let pads: Vec<&str> = m.io.analog_pads.pins.iter().map(String::as_str).collect();
    assert_eq!(pads, ["PC2", "PC3"]);
    let cite = &m.io.analog_pads.at.section;
    assert!(
        cite.contains("Table 8") && cite.contains("ANA") && cite.contains("Table 11"),
        "{cite}"
    );
    for bad in [json!(["PC2", "PC2"]), json!(["PC2", "PC2_C"])] {
        let mut t: Value = serde_json::from_str(POWER_PARTS_JSON).unwrap();
        let row = t["parts"]
            .as_array()
            .unwrap()
            .iter()
            .position(|p| p["part"] == "STM32H753ZITx")
            .unwrap();
        t["parts"][row]["kind"]["mcu"]["io"]["analog_pads"]["pins"] = bad.clone();
        let e = parse_power_parts(&t.to_string()).unwrap_err().to_string();
        assert!(e.contains("analog_pads"), "{bad}: {e}");
    }
}
