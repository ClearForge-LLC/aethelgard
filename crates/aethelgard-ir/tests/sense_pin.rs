//! `AET-WO-4300d2` §1.7: the battery-sense net's ADC pin, solved on a small
//! synthetic `PinTable` (the real table's channels are tested in
//! `aethelgard-svd-hal`; this crate may depend on no other member). The
//! specification is Sentinel-02's power fixture without its crossing.

use std::collections::{BTreeMap, BTreeSet};

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

/// The fixture without its crossing and the IMU behind it (it waits for the ICM-20948's footprint).
fn base() -> Value {
    let mut v: Value =
        serde_json::from_str(include_str!("fixtures/sentinel-02-power.json")).unwrap();
    v["power"]["crossings"] = json!([]);
    // The translator's and its CS pull-up's loads go with the crossing
    // (`AET-WO-1209` D4): a load naming a part the board lacks is refused.
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U13" && l["reference"] != "R40");
    // U5 goes, so its acknowledgement would stand for no part (`4210`); the
    // flash's stays (`AET-WO-4211`).
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

fn spec(v: &Value) -> Specification {
    serde_json::from_value(v.clone()).expect("a specification")
}

/// The buses' pins and the chip selects' free pin (`PA0`, the first key),
/// plus whatever ADC pins a test adds.
fn table(adc: &[Row<'_>]) -> PinTable {
    let mut rows: Vec<Row<'_>> = vec![
        ("PA0", 2, &[], &[]),
        ("PA5", 5, &[("SPI1_SCK", 5)], &[]),
        ("PA6", 6, &[("SPI1_MISO", 5)], &["ADC1_INP3", "ADC2_INP3"]),
        ("PA7", 7, &[("SPI1_MOSI", 5)], &[]),
        ("PB6", 8, &[("I2C1_SCL", 4)], &[]),
        ("PB7", 9, &[("I2C1_SDA", 4)], &[]),
    ];
    rows.extend_from_slice(adc);
    table_of(&rows)
}

fn solved(v: &Value, t: &PinTable) -> Result<SolvedTopology, SolveError> {
    solve(&spec(v), t, HASH)
}

#[test]
fn the_sense_takes_the_lowest_position_adc_pin_not_the_first_key() {
    // `PA1` sorts first and `PB1` has the lowest position.
    let t = table(&[
        ("PA1", 30, &[], &["ADC1_INP17"]),
        ("PB1", 3, &[], &["ADC1_INP5", "ADC2_INP5"]),
    ]);
    let s = solved(&base(), &t).unwrap();
    let sense = &s.power.as_ref().unwrap().sense["VBAT_SENSE"];
    assert_eq!(
        (sense.pin.as_str(), sense.position, sense.channels.clone()),
        (
            "PB1",
            3,
            vec!["ADC1_INP5".to_owned(), "ADC2_INP5".to_owned()]
        )
    );
    // An analog pin has no alternate function: it is not an assignment.
    assert!(!s.assignments.contains_key("VBAT_SENSE"));
    assert!(!s.gpio.contains_key("VBAT_SENSE"));
}

#[test]
fn the_sense_is_never_a_pin_another_net_took() {
    // `PA6` (position 6, the lowest ADC pin) is the flash's MISO, and `PA0`
    // (position 2) is the first key, so the chip select takes it.
    let t = table(&[
        ("PA0", 2, &[], &["ADC1_INP16"]),
        ("PC0", 40, &[], &["ADC1_INP10"]),
    ]);
    let s = solved(&base(), &t).unwrap();
    let taken: BTreeSet<&str> = s
        .assignments
        .values()
        .map(|a| a.pin.as_str())
        .chain(s.gpio.values().map(|g| g.pin.as_str()))
        .collect();
    assert!(taken.contains("PA6") && taken.contains("PA0"), "{taken:?}");
    let sense = &s.power.as_ref().unwrap().sense["VBAT_SENSE"];
    assert_eq!(sense.pin, "PC0");
    assert!(!taken.contains(sense.pin.as_str()));
}

#[test]
fn a_hand_pin_is_not_taken_by_the_sense() {
    let mut v = base();
    v["i2c_buses"][0]["sda"]["pin"] = json!("PB7");
    let t = table(&[("PC0", 40, &[], &["ADC1_INP10"])]);
    let mut rows = t.pins.clone();
    rows.get_mut("PB7")
        .unwrap()
        .adc_channels
        .insert("ADC1_INP7".into());
    rows.get_mut("PB7").unwrap().position = 1;
    let t = PinTable { pins: rows, ..t };
    let s = solved(&v, &t).unwrap();
    assert_eq!(s.assignments["I2C_SDA"].pin, "PB7");
    assert_eq!(s.power.unwrap().sense["VBAT_SENSE"].pin, "PC0");
}

#[test]
fn no_free_adc_pin_is_refused_by_net() {
    // Only `PA6`, the flash's MISO, carries an ADC input.
    let err = solved(&base(), &table(&[])).unwrap_err();
    assert_eq!(
        err,
        SolveError::NoAdcPin {
            net: "VBAT_SENSE".into()
        }
    );
    assert_eq!((err.kind(), err.id()), ("NoAdcPin", Some("VBAT_SENSE")));
    assert_eq!(err.claimants(), vec!["VBAT_SENSE".to_owned()]);
}

#[test]
fn the_reserved_pins_are_never_the_sense() {
    let t = table(&[("PA13", 1, &[], &["ADC1_INP9"])]);
    assert!(matches!(
        solved(&base(), &t),
        Err(SolveError::NoAdcPin { .. })
    ));
}

#[test]
fn the_solution_is_deterministic_and_round_trips() {
    let t = table(&[
        ("PB1", 3, &[], &["ADC1_INP5"]),
        ("PC0", 40, &[], &["ADC1_INP10"]),
    ]);
    let a = solved(&base(), &t).unwrap();
    let b = solved(&base(), &t).unwrap();
    assert_eq!(a, b);
    let text = serde_json::to_string(&a).unwrap();
    assert_eq!(text, serde_json::to_string(&b).unwrap());
    let back: SolvedTopology = serde_json::from_str(&text).unwrap();
    assert_eq!(back, a);
}

#[test]
fn a_tree_without_a_sense_states_an_empty_sense_and_a_topology_without_the_key_is_refused() {
    let t = table(&[("PB1", 3, &[], &["ADC1_INP5"])]);
    let mut v = base();
    v["power"]["sense"] = json!([]);
    let s = solved(&v, &t).unwrap();
    assert!(s.power.as_ref().unwrap().sense.is_empty());
    let mut j: Value = serde_json::to_value(&s).unwrap();
    assert_eq!(j["power"]["sense"], json!({}), "{j}");
    // INV-06: a topology that omits the key is refused, never defaulted.
    j["power"].as_object_mut().unwrap().remove("sense");
    let err = serde_json::from_value::<aethelgard_ir::SolvedTopology>(j).unwrap_err();
    assert!(err.to_string().contains("sense"), "{err}");
}

#[test]
fn the_table_serializes_no_channels_where_a_pin_has_none() {
    let t = table(&[]);
    let j = serde_json::to_value(&t).unwrap();
    assert!(j["pins"]["PA0"].get("adc_channels").is_none());
    assert_eq!(
        j["pins"]["PA6"]["adc_channels"],
        json!(["ADC1_INP3", "ADC2_INP3"])
    );
}

#[test]
fn an_adc_channel_name_is_checked_like_a_signal() {
    let mut t = table(&[]);
    t.pins
        .get_mut("PA0")
        .unwrap()
        .adc_channels
        .insert("ADC1 INP3".into());
    assert!(t.validate().is_err());
}

/// `AET-WO-1211a`: the sense is also the ADC input on its own pin, with an
/// ADC1 or ADC3 channel; a topology whose two disagree is refused.
#[test]
fn the_sense_is_its_adc_input_and_a_disagreeing_topology_is_refused() {
    let t = table(&[("PB1", 3, &[], &["ADC1_INP5", "ADC2_INP5"])]);
    let s = solved(&base(), &t).unwrap();
    let a = &s.adc_inputs["VBAT_SENSE"];
    assert_eq!(
        (a.pin.as_str(), a.peripheral.as_str(), a.channel.as_str()),
        ("PB1", "ADC1", "INP5")
    );
    let mut bad = s.clone();
    bad.adc_inputs.get_mut("VBAT_SENSE").unwrap().pin = "PC5".into();
    assert_eq!(bad.validate().unwrap_err().kind(), "SenseAdcMismatch");
    let mut bad = s.clone();
    bad.adc_inputs.clear();
    assert_eq!(bad.validate().unwrap_err().kind(), "SenseAdcMismatch");
}

/// A pin with an ADC2 input alone is passed; with only those free, the
/// sense is `AdcUnmodelled` (`1211` Q8).
#[test]
fn the_sense_skips_an_adc2_only_pin_and_is_unmodelled_with_only_those() {
    let t = table(&[
        ("PB2", 3, &[], &["ADC2_INP2"]),
        ("PC0", 40, &[], &["ADC3_INP10"]),
    ]);
    let s = solved(&base(), &t).unwrap();
    assert_eq!(s.power.as_ref().unwrap().sense["VBAT_SENSE"].pin, "PC0");
    let t = table(&[("PB2", 3, &[], &["ADC2_INP2"])]);
    // `PA6`, the only other ADC pin, is the flash's MISO.
    assert_eq!(solved(&base(), &t).unwrap_err().kind(), "AdcUnmodelled");
}
