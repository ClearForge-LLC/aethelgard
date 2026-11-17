//! `AET-WO-4200`, the board crate's SPI side.
//!
//! - F3 (ruling, "F3 is a generator rule"): an undriven chip select is a
//!   selected part, so a generated firmware drives every chip select of a bus
//!   before that bus starts and before its first transfer. The types already
//!   refuse anything else (`tests/ui/fail/spi_*`); this checks the text the
//!   generator writes.
//! - D5: a second device on the bus, the ICM-20948 of the test fixture, gets
//!   its own chip select and variant, and is read after the flash.

use std::fs;
use std::path::Path;

use aethelgard_ir::{SolvedSpiDevice, SolvedTopology, Specification};
use aethelgard_svd_hal::board::generate_with;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

fn reference() -> (SolvedTopology, aethelgard_ir::PinTable) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let bytes = fs::read(path).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

/// What the CLI passes: the twin's model table.
fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

/// The reference plus `U5`, an ICM-20948 on the flash's bus with its chip
/// select on PA1, built by hand from the reference's solved topology.
fn two_devices() -> (SolvedTopology, aethelgard_ir::PinTable) {
    let (mut solved, table) = reference();
    // The ICM-20948 clocks at 7 MHz at most; the bus runs at that rate.
    for line in solved.spi_lines.values_mut() {
        line.rate_hz = 7_000_000;
    }
    let flash = solved.spi_devices["U4"].clone();
    solved.gpio.insert("IMU_CS".into(), "PA1".into());
    solved.spi_devices.insert(
        "U5".into(),
        SolvedSpiDevice {
            index: 1,
            part: "ICM20948".into(),
            bus: flash.bus,
            peripheral: flash.peripheral,
            cs: "IMU_CS".into(),
            orientation: None,
        },
    );
    solved.derive_controllers();
    solved.validate().unwrap();
    (solved, table)
}

/// The position of `needle` in `text`, which must hold it exactly once.
fn once(text: &str, needle: &str) -> usize {
    assert_eq!(text.matches(needle).count(), 1, "`{needle}` in\n{text}");
    text.find(needle).unwrap()
}

/// F3, in text: for every SPI device, its chip select's `into_output` comes
/// before its bus's `init`, which comes before the bus's first `transfer`.
fn selects_precede_their_bus(solved: &SolvedTopology, main: &str) {
    assert!(!solved.spi_devices.is_empty());
    for (reference, d) in solved.spi_devices_in_order() {
        let pin = solved.gpio[&d.cs].to_ascii_lowercase();
        let bus = d.peripheral.to_ascii_lowercase();
        let output = once(main, &format!("let {pin} = gpio"));
        assert!(main[output..].starts_with(&format!(
            "let {pin} = gpio{}.{pin}.into_output(&mut r);",
            &pin[1..2]
        )));
        let init = once(main, &format!("= {bus}.init(&mut r, "));
        assert!(
            output < init,
            "{reference}: {pin} is driven after {bus} starts"
        );
        // The select is handed to the bus by name.
        let line = &main[init..init + main[init..].find('\n').unwrap()];
        assert!(
            line.contains("Selects { ") && line.contains(pin.as_str()),
            "{line}"
        );
        if let Some(first) = main.find(&format!("{bus}.transfer(")) {
            assert!(init < first, "{bus} transfers before it starts");
        }
    }
}

#[test]
fn every_chip_select_is_driven_before_its_bus_starts_and_transfers() {
    let (solved, table) = reference();
    let files = generate_with("sentinel-01-board", &solved, &table, &answers).unwrap();
    let main = &files["firmware/src/main.rs"];
    selects_precede_their_bus(&solved, main);
    assert!(main.contains("spi1.transfer(&mut r, Spi1Device::U4, &[0x9f], &mut buf1)"));

    let (solved, table) = two_devices();
    let files = generate_with("b", &solved, &table, &answers).unwrap();
    selects_precede_their_bus(&solved, &files["firmware/src/main.rs"]);
}

/// The check fails a firmware that starts the bus first (the negative
/// control): `into_output` moved after `init`.
#[test]
fn a_firmware_that_starts_the_bus_first_fails_the_check() {
    let (solved, table) = reference();
    let files = generate_with("b", &solved, &table, &answers).unwrap();
    let main = &files["firmware/src/main.rs"];
    let output = "    let pa0 = gpioa.pa0.into_output(&mut r);\n";
    let init_at = main.find("    let mut spi1 = spi1.init(").unwrap();
    let init_end = init_at + main[init_at..].find('\n').unwrap() + 1;
    let init = &main[init_at..init_end];
    let moved = main
        .replacen(output, "", 1)
        .replacen(init, &format!("{init}{output}"), 1);
    assert_ne!(&moved, main);
    let caught = std::panic::catch_unwind(|| selects_precede_their_bus(&solved, &moved));
    assert!(caught.is_err());
}

#[test]
fn a_second_device_gets_its_own_select_and_is_read_after_the_flash() {
    let (solved, table) = two_devices();
    let files = generate_with("b", &solved, &table, &answers).unwrap();
    let lib = &files["src/board.rs"];
    let selects = &lib[once(lib, "pub struct Spi1Selects {")..];
    let selects = &selects[..selects.find("\n}\n").unwrap()];
    assert!(selects.contains("pub pa0: PA0<Output>,"));
    assert!(selects.contains("pub pa1: PA1<Output>,"));
    let device = &lib[once(lib, "pub enum Spi1Device {")..];
    let device = &device[..device.find("\n}\n").unwrap()];
    assert!(once(device, "    U4,") < once(device, "    U5,"));
    // Each variant selects its own pin: BSRR reset, then set, of PA0 and PA1.
    once(
        lib,
        "Spi1Device::U4 => (Block::GpioA, 0x00010000, 0x00000001),",
    );
    once(
        lib,
        "Spi1Device::U5 => (Block::GpioA, 0x00020000, 0x00000002),",
    );

    // Records: the BME280's, then U4's, then U5's script, as the judge orders
    // them (`AET-WO-4210`): its identity, its wake, and its 6-byte data read,
    // whose record takes two value words, so each record names its word. The
    // first record is word 2, after the boot word and the brown-out word
    // (`AET-WO-4230` D10).
    let main = &files["firmware/src/main.rs"];
    let plan = aethelgard_twin::boot::identity(&solved).unwrap();
    let refs: Vec<(&str, &str, u32)> = plan
        .checks
        .iter()
        .map(|c| {
            let word = (c.status_at - plan.checks[0].status_at) / 4 + 2;
            (c.reference.as_str(), c.step.name(), word)
        })
        .collect();
    assert_eq!(
        refs,
        [
            ("U2", "identity", 2),
            ("U4", "identity", 4),
            ("U5", "identity", 6),
            ("U5", "configure", 8),
            ("U5", "data", 10),
        ]
    );
    let u4 = once(
        main,
        "    let mut buf1 = [0u8; 3];\n    \
         let (status, value) = match spi1.transfer(&mut r, Spi1Device::U4, &[0x9f], &mut buf1) {",
    );
    let u5 = once(
        main,
        "    let mut buf2 = [0u8; 1];\n    \
         let (status, value) = match spi1.transfer(&mut r, Spi1Device::U5, &[0x80], &mut buf2) {",
    );
    let wake = once(
        main,
        "    let mut buf3 = [0u8; 0];\n    \
         let (status, value) = match spi1.transfer(&mut r, Spi1Device::U5, &[0x06, 0x01], &mut buf3) {",
    );
    let data = once(
        main,
        "    let mut buf4 = [0u8; 6];\n    \
         let (status, value) = match spi1.transfer(&mut r, Spi1Device::U5, &[0xad], &mut buf4) {",
    );
    assert!(u4 < u5 && u5 < wake && wake < data);
    assert!(once(main, "record(4, 1, status, value);") < u5);
    assert!(u5 < once(main, "record(6, 1, status, value);"));
    assert!(wake < once(main, "record(8, 1, status, value);"));
    assert!(data < once(main, "record(10, 2, status, value);"));
    assert!(main.contains("static mut AETHELGARD_BOOT: [u32; 13] = [0; 13];"));
    assert!(main.contains("Spi1Selects { pa0, pa1 }"));
}
