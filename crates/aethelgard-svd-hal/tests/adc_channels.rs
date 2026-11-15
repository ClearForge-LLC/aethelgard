//! `AET-WO-4300d2` §1.7: the embedded table carries each I/O pin's
//! single-ended ADC inputs, read from the same vendored ST XML as the pins.

use std::collections::BTreeSet;

fn channels(pin: &str) -> BTreeSet<String> {
    aethelgard_svd_hal::pin_table("STM32H753ZITx").unwrap().pins[pin]
        .adc_channels
        .clone()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

#[test]
fn the_pins_carry_the_inputs_the_xml_gives_them() {
    assert_eq!(channels("PA6"), set(&["ADC1_INP3", "ADC2_INP3"]));
    assert_eq!(channels("PF3"), set(&["ADC3_INP5"]));
    // `PB8` has no analog input.
    assert!(channels("PB8").is_empty());
}

#[test]
fn only_positive_single_ended_inputs_are_carried() {
    let table = aethelgard_svd_hal::pin_table("STM32H753ZITx").unwrap();
    let all: BTreeSet<&String> = table.pins.values().flat_map(|p| &p.adc_channels).collect();
    assert!(!all.is_empty());
    assert!(
        all.iter()
            .all(|c| c.starts_with("ADC") && c.contains("_INP"))
    );
    // The XML lists `ADC3_INN5` on `PF3` too; it is not carried.
    assert!(!channels("PF3").contains("ADC3_INN5"));
    // Counted in the XML: 28 of its I/O pins carry an `ADCn_INPm` signal.
    assert_eq!(
        table
            .pins
            .values()
            .filter(|p| !p.adc_channels.is_empty())
            .count(),
        28
    );
}
