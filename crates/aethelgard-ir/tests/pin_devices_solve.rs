//! `AET-WO-1211a`: the solver's half of a pin device, on a small synthetic
//! `PinTable` (this crate may depend on no other member; D1 and D2 on the
//! real table are `aethelgard-cli`'s `pin_devices_solve.rs`). The
//! specification is `sense_pin.rs`'s Sentinel-02 power fixture with a
//! DRV8876 whose inputs are GPIO signals (`1211` Q7) and whose IPROPI is an
//! ADC input.
//!
//! Every new placement comes after every existing one, so a chip select and
//! the battery sense keep their pins; an ADC input takes ADC1, then ADC3,
//! and an ADC2-only pin only when the specification pins it (`1211` Q8).

use std::collections::BTreeMap;

use aethelgard_ir::motor::Level;
use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedAdcInput, SolvedGpio};
use aethelgard_ir::{Pin, PinSource, PinTable, SolveError, SolvedTopology, Specification, solve};
use serde_json::{Value, json};

const HASH: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";

/// `(pin, position, [(signal, af)], [adc channel])`.
type Row<'a> = (&'a str, u16, &'a [(&'a str, u8)], &'a [&'a str]);

fn table(extra: &[Row<'_>]) -> PinTable {
    let mut rows: Vec<Row<'_>> = vec![
        ("PA0", 2, &[], &[]),
        ("PA5", 5, &[("SPI1_SCK", 5)], &[]),
        ("PA6", 6, &[("SPI1_MISO", 5)], &["ADC1_INP3", "ADC2_INP3"]),
        ("PA7", 7, &[("SPI1_MOSI", 5)], &[]),
        ("PB6", 8, &[("I2C1_SCL", 4)], &[]),
        ("PB7", 9, &[("I2C1_SDA", 4)], &[]),
        ("PD0", 50, &[], &[]),
        ("PD1", 51, &[], &[]),
        ("PD2", 52, &[], &[]),
        ("PD3", 53, &[], &[]),
        ("PD4", 54, &[], &[]),
    ];
    rows.extend_from_slice(extra);
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

/// `sense_pin.rs`'s base, with the DRV8876 on its 5 V rail (VM 4.5 to 37 V,
/// SLVSDS7B p. 4; a power tree refuses a domain no regulator sources).
fn base() -> Value {
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
    let out = |pin: &str, net: &str| json!({"device_pin": pin, "net": net, "pin": null, "direction": "output", "initial": "low"});
    v["pin_devices"] = json!([{
        "reference": "U20",
        "part": "DRV8876PWPR",
        "supplies": {"VM": "VDD_5V"},
        "straps": {"PMODE": {"tie": "GND"}, "IMODE": {"tie": "GND"}, "VREF": {"tie": "VDD_3V3"}},
        "signals": [
            out("EN/IN1", "DRV_EN"),
            out("PH/IN2", "DRV_PH"),
            out("nSLEEP", "DRV_NSLEEP"),
            {"device_pin": "nFAULT", "net": "DRV_NFAULT", "pin": null, "direction": "input",
             "active": "low", "pull": {"mcu": "up", "source": "DS12117 Rev 10 Table 49 p. 137"}}
        ],
        "analog": [{
            "device_pin": "IPROPI", "net": "DRV_IPROPI", "pin": null, "channel": null,
            "transfer": {"current_output": {
                "resistor": {"reference": "R90", "part": "CRCW0603_F_K", "ohms": 1500},
                "reservoir": {"reference": "C90", "part": "GCM188R71H104KA57"}}},
            "full_scale": {"milliamps": 1900},
            "adc_resolution_bits": 12
        }],
        "bridges": [],
        "part_limits": {"input_pwm_max_hz": null},
        "temperature_celsius": null
    }]);
    v
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
    }
}

/// The battery sense on `PB1` (lowest position), the IPROPI on the next.
fn two_adc_pins() -> PinTable {
    table(&[
        ("PB1", 3, &[], &["ADC1_INP5", "ADC2_INP5"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ])
}

#[test]
fn the_device_s_signals_and_input_come_after_every_other_pin() {
    spec(&base()).validate().unwrap();
    let t = two_adc_pins();
    let s = solved(&base(), &t).unwrap();
    s.validate().unwrap();
    // The chip select keeps the first key; the ADC input is placed before
    // the signals, which take the next free keys in document order.
    assert_eq!(s.gpio["FLASH_CS"], SolvedGpio::output("PA0", Level::High));
    assert_eq!(s.gpio["DRV_EN"], SolvedGpio::output("PD0", Level::Low));
    assert_eq!(s.gpio["DRV_PH"], SolvedGpio::output("PD1", Level::Low));
    assert_eq!(s.gpio["DRV_NSLEEP"], SolvedGpio::output("PD2", Level::Low));
    assert_eq!(
        s.gpio["DRV_NFAULT"],
        SolvedGpio {
            pin: "PD3".into(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::Up),
        }
    );
    // The sense keeps its pin, now with its peripheral chosen; the IPROPI
    // takes the next ADC pin.
    assert_eq!(s.power.as_ref().unwrap().sense["VBAT_SENSE"].pin, "PB1");
    assert_eq!(s.adc_inputs["VBAT_SENSE"], adc("PB1", "ADC1", "INP5"));
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC0", "ADC3", "INP10"));
    assert_eq!(s.pin_devices["U20"].part, "DRV8876PWPR");
    // A signal's pin has its speed, 0.
    assert_eq!(s.pin_speeds["DRV_NFAULT"], 0);
}

#[test]
fn without_the_device_nothing_moves_but_the_new_keys() {
    let t = two_adc_pins();
    let mut v = base();
    v.as_object_mut().unwrap().remove("pin_devices");
    let s = solved(&v, &t).unwrap();
    assert!(s.pin_devices.is_empty());
    let j = serde_json::to_value(&s).unwrap();
    assert!(j.get("pin_devices").is_none(), "{j}");
    assert_eq!(s.gpio.len(), 1);
    assert_eq!(s.adc_inputs.len(), 1);
}

#[test]
fn an_adc2_only_pin_is_never_chosen_unpinned() {
    // `PB2` is the lowest free ADC pin, ADC2 alone; the solver passes it.
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PB2", 4, &[], &["ADC2_INP2"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ]);
    let s = solved(&base(), &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC0", "ADC3", "INP10"));
}

#[test]
fn only_adc2_pins_left_is_adc_unmodelled() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PB2", 4, &[], &["ADC2_INP2"]),
    ]);
    let e = solved(&base(), &t).unwrap_err();
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
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PB2", 4, &[], &["ADC2_INP2"]),
    ]);
    let mut v = base();
    v["pin_devices"][0]["analog"][0]["pin"] = json!("PB2");
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PB2", "ADC2", "INP2"));
}

#[test]
fn a_pin_with_both_takes_adc1() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC0", 40, &[], &["ADC1_INP10", "ADC2_INP10", "ADC3_INP0"]),
    ]);
    let s = solved(&base(), &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC0", "ADC1", "INP10"));
}

#[test]
fn a_channel_named_off_its_pin_is_refused() {
    let t = two_adc_pins();
    let mut v = base();
    v["pin_devices"][0]["analog"][0]["pin"] = json!("PC0");
    v["pin_devices"][0]["analog"][0]["channel"] = json!({"peripheral": "ADC1", "input": "INP10"});
    let e = solved(&v, &t).unwrap_err();
    assert_eq!(e.kind(), "AdcChannelNotOnPin", "{e}");
    // A pin with no ADC input at all.
    let mut v = base();
    v["pin_devices"][0]["analog"][0]["pin"] = json!("PD4");
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "AdcChannelNotOnPin");
}

#[test]
fn a_named_channel_finds_its_pin() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
        ("PC1", 41, &[], &["ADC3_INP11"]),
    ]);
    let mut v = base();
    v["pin_devices"][0]["analog"][0]["channel"] = json!({"peripheral": "ADC3", "input": "INP11"});
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.adc_inputs["DRV_IPROPI"], adc("PC1", "ADC3", "INP11"));
}

#[test]
fn no_adc_pin_left_is_no_adc_pin() {
    let t = table(&[("PB1", 3, &[], &["ADC1_INP5"])]);
    let e = solved(&base(), &t).unwrap_err();
    assert_eq!(
        e,
        SolveError::NoAdcPin {
            net: "DRV_IPROPI".into()
        }
    );
}

#[test]
fn a_device_s_hand_pin_is_not_given_to_the_chip_select() {
    // `PA0` is the chip select's by key order; pinned to the device's
    // signal, the chip select takes the next free key, `PB1`, so the sense
    // and the IPROPI take the next ADC pins.
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
        ("PC1", 41, &[], &["ADC3_INP11"]),
    ]);
    let mut v = base();
    v["pin_devices"][0]["signals"][2]["pin"] = json!("PA0");
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.gpio["DRV_NSLEEP"].pin, "PA0");
    assert_eq!(s.gpio["FLASH_CS"].pin, "PB1");
    assert_eq!(s.adc_inputs["VBAT_SENSE"].pin, "PC0");
    assert_eq!(s.adc_inputs["DRV_IPROPI"].pin, "PC1");
}

#[test]
fn a_device_s_hand_pin_claimed_twice_or_reserved_is_refused() {
    let t = two_adc_pins();
    let mut v = base();
    v["pin_devices"][0]["signals"][0]["pin"] = json!("PD0");
    v["pin_devices"][0]["signals"][1]["pin"] = json!("PD0");
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "PinClaimedTwice");
    let mut v = base();
    v["pin_devices"][0]["signals"][0]["pin"] = json!("PA13");
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
        ("PA13", 60, &[], &[]),
    ]);
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "ReservedPin");
}

#[test]
fn the_sense_skips_an_adc2_only_pin_and_is_unmodelled_with_only_those() {
    let mut v = base();
    v.as_object_mut().unwrap().remove("pin_devices");
    let t = table(&[
        ("PB2", 3, &[], &["ADC2_INP2"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ]);
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.power.as_ref().unwrap().sense["VBAT_SENSE"].pin, "PC0");
    let t = table(&[("PB2", 3, &[], &["ADC2_INP2"])]);
    // `PA6`, the only other ADC pin, is the flash's MISO.
    assert_eq!(solved(&v, &t).unwrap_err().kind(), "AdcUnmodelled");
}
