//! `AET-WO-4200` tier 1: the SPI half of the platform emitter and of the boot
//! judge's plan, on the reference topology (schema 3: U2 BME280 and U3 TMP117
//! on I2C1, U4 W25Q64JV on SPI1 with chip select FLASH_CS on PA0). Nothing
//! here starts Renode.

use std::collections::BTreeSet;

use aethelgard_ir::{IrValidationError, SolvedI2cDevice, SolvedSpiDevice, SolvedTopology};
use aethelgard_twin::TwinError;
use aethelgard_twin::boot::{self, RECORD_BYTES, Wire};
use aethelgard_twin::platform;

/// The reference topology, as `src/tests.rs` has it.
const REFERENCE: &str = r#"{"schema_version":3,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{"U4":{"index":0,"part":"W25Q64JV","bus":"FLASH","peripheral":"SPI1","cs":"FLASH_CS"}}}"#;

fn reference() -> SolvedTopology {
    serde_json::from_str(REFERENCE).unwrap()
}

fn set(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|n| (*n).to_owned()).collect()
}

fn spi_device(index: u32, part: &str, cs: &str) -> SolvedSpiDevice {
    SolvedSpiDevice {
        index,
        part: part.to_owned(),
        bus: "FLASH".to_owned(),
        peripheral: "SPI1".to_owned(),
        cs: cs.to_owned(),
    }
}

/// The reference plus ICM20948 `U5` on SPI1, chip select `IMU_CS` on `PA1`.
fn with_imu() -> SolvedTopology {
    let mut t = reference();
    t.gpio.insert("IMU_CS".to_owned(), "PA1".to_owned());
    t.spi_devices
        .insert("U5".to_owned(), spi_device(1, "ICM20948", "IMU_CS"));
    t
}

/// The lines of `text` from the multiplexer's declaration on, without
/// comments or blank lines.
fn spi_declarations(text: &str) -> Vec<&str> {
    let from = text.find("spi1mux: SPI.SPIMultiplexer").expect(text);
    text[from..]
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("//"))
        .collect()
}

fn count(text: &str, needle: &str) -> usize {
    text.matches(needle).count()
}

#[test]
fn the_reference_wires_one_mux_one_chip_select_and_the_flash_in_order() {
    let text = platform::emit(&reference()).unwrap();
    assert_eq!(
        count(&text, "spi1mux: SPI.SPIMultiplexer @ spi1"),
        1,
        "{text}"
    );
    assert_eq!(count(&text, "gpioPortA:"), 1, "{text}");
    assert_eq!(
        spi_declarations(&text),
        [
            "spi1mux: SPI.SPIMultiplexer @ spi1",
            "    init:",
            "        SetActiveLow 0",
            "gpioPortA:",
            "    0 -> spi1mux@0",
            "u4_memory: Memory.MappedMemory",
            "    size: 0x800000",
            "u4: SPI.GenericSpiFlash @ spi1mux 0",
            "    underlyingMemory: u4_memory",
            "    manufacturerId: 0xEF",
            "    memoryType: 0x40",
            "    capacityCode: 0x17",
        ],
        "{text}"
    );
}

#[test]
fn two_spi_devices_share_one_mux_and_one_port_block_and_the_judge_reads_both() {
    let t = with_imu();
    let text = platform::emit(&t).unwrap();
    assert_eq!(
        count(&text, "spi1mux: SPI.SPIMultiplexer @ spi1"),
        1,
        "{text}"
    );
    assert_eq!(count(&text, "gpioPortA:"), 1, "{text}");
    assert_eq!(
        spi_declarations(&text),
        [
            "spi1mux: SPI.SPIMultiplexer @ spi1",
            "    init:",
            "        SetActiveLow 0",
            "        SetActiveLow 1",
            "gpioPortA:",
            "    0 -> spi1mux@0",
            "    1 -> spi1mux@1",
            "u4_memory: Memory.MappedMemory",
            "    size: 0x800000",
            "u4: SPI.GenericSpiFlash @ spi1mux 0",
            "    underlyingMemory: u4_memory",
            "    manufacturerId: 0xEF",
            "    memoryType: 0x40",
            "    capacityCode: 0x17",
            "u5: Sensors.ICM20948 @ spi1mux 1",
        ],
        "{text}"
    );

    let plan = boot::identity(&t).unwrap();
    let refs: Vec<&str> = plan.checks.iter().map(|c| c.reference.as_str()).collect();
    assert_eq!(refs, ["U2", "U4", "U5"]);
    assert_eq!(plan.checks[0].wire, Wire::I2c { address: 0x76 });
    assert_eq!(
        plan.checks[1].wire,
        Wire::Spi {
            address: 0,
            select: "PA0".into()
        }
    );
    assert_eq!(
        plan.checks[2].wire,
        Wire::Spi {
            address: 1,
            select: "PA1".into()
        }
    );
    // Records follow the boot word's, one pair each, consecutive.
    let first = plan.checks[0].status_at;
    for (n, c) in plan.checks.iter().enumerate() {
        assert_eq!(c.status_at, first + RECORD_BYTES * n as u32);
        assert_eq!(c.value_at, c.status_at + 4);
    }
    assert_eq!(first, boot::boot_word().unwrap() + boot::RECORD_OFFSET);

    // U5's witness is the ICM's four lines, and its skipped line is the one
    // chip-select edge the model logs.
    let u5 = &plan.checks[2];
    assert_eq!(u5.device, "spi1.spi1mux.u5");
    assert_eq!((u5.write, u5.width), (0x80, 1));
    let id = u5.expected;
    let d = "spi1.spi1mux.u5";
    assert_eq!(
        u5.witness.as_deref(),
        Some(
            &[
                format!("[NOISY] {d}: Preparing for read at bank #0 from 0x0 (WhoAmI)"),
                format!("[NOISY] {d}: Received byte 0x80, returning 0x0"),
                format!("[NOISY] {d}: Read at bank #0 from 0x0 (WhoAmI), returned 0x{id:X}"),
                format!("[NOISY] {d}: Received byte 0x0, returning 0x{id:X}"),
            ][..]
        )
    );
    assert_eq!(
        u5.skipped,
        ["[NOISY] spi1.spi1mux.u5: Finishing transmission, going idle"]
    );
    // The flash's own witness is 8 lines and skips nothing.
    assert_eq!(plan.checks[1].witness.as_ref().map(Vec::len), Some(8));
    assert!(plan.checks[1].skipped.is_empty());
}

#[test]
fn an_absent_device_leaves_its_model_but_keeps_the_mux_and_the_wiring() {
    let t = reference();
    let text = platform::emit_faulted(&t, &set(&[]), &set(&["U4"])).unwrap();
    assert!(!text.contains("u4_memory"), "{text}");
    assert!(!text.contains("u4:"), "{text}");
    assert_eq!(
        spi_declarations(&text),
        [
            "spi1mux: SPI.SPIMultiplexer @ spi1",
            "    init:",
            "        SetActiveLow 0",
            "gpioPortA:",
            "    0 -> spi1mux@0",
        ],
        "{text}"
    );
    assert!(
        text.contains("U4 (W25Q64JV) is absent behind its chip select PA0"),
        "{text}"
    );
    // The same topology with nobody absent is a different platform.
    assert_ne!(text, platform::emit(&t).unwrap());

    let names: Vec<(String, String)> = platform::instances(&t, &set(&[]), &set(&["U4"]))
        .unwrap()
        .into_iter()
        .map(|i| (i.name, i.model))
        .collect();
    assert!(
        names.contains(&("spi1mux".to_owned(), "SPIMultiplexer".to_owned())),
        "{names:?}"
    );
    assert!(!names.iter().any(|(n, _)| n == "u4"), "{names:?}");
    let all: Vec<String> = platform::instances(&t, &set(&[]), &set(&[]))
        .unwrap()
        .into_iter()
        .map(|i| i.name)
        .collect();
    assert!(all.contains(&"u4".to_owned()), "{all:?}");
}

#[test]
fn an_absent_reference_that_is_no_spi_device_is_refused() {
    let t = reference();
    for r in ["U2", "NOPE", "u4"] {
        let e = platform::emit_faulted(&t, &set(&[]), &set(&[r])).unwrap_err();
        assert_eq!(
            e,
            TwinError::UnknownAbsentDevice {
                reference: r.to_owned()
            }
        );
        let e = platform::instances(&t, &set(&[]), &set(&[r])).unwrap_err();
        assert_eq!(e.kind(), "UnknownAbsentDevice", "{r}");
    }
}

#[test]
fn an_unmodelled_spi_part_is_commented_listed_and_reported_never_wired_to_a_model() {
    let mut t = reference();
    t.spi_devices.get_mut("U4").unwrap().part = "W25Q128".to_owned();
    let text = platform::emit(&t).unwrap();
    assert!(!text.contains("u4:"), "{text}");
    assert!(!text.contains("u4_memory"), "{text}");
    // A comment naming it, and the header's list.
    assert!(
        text.contains("// U4 (W25Q128): no Renode 1.17.0 model; its chip select (PA0) is wired to"),
        "{text}"
    );
    assert!(
        text.contains(
            "// - Unmodelled SPI devices (no Renode 1.17.0 model): U4 (W25Q128) at spi1mux@0."
        ),
        "{text}"
    );
    // Its chip select is still wired.
    assert!(text.contains("    0 -> spi1mux@0"), "{text}");

    let plan = boot::identity(&t).unwrap();
    assert!(plan.checks.iter().all(|c| c.reference != "U4"));
    assert_eq!(plan.unmodelled.len(), 2, "U3 (I2C) and U4: {plan:?}");
    let u4 = plan
        .unmodelled
        .iter()
        .find(|u| u.reference == "U4")
        .unwrap();
    assert_eq!(u4.part, "W25Q128");
    assert_eq!(u4.peripheral, "SPI1");
    assert_eq!(
        u4.wire,
        Wire::Spi {
            address: 0,
            select: "PA0".into()
        }
    );
}

#[test]
fn a_model_on_the_wrong_kind_of_bus_is_refused() {
    // The W25Q64JV is SPI-only, declared as an I2C device.
    let mut t = reference();
    t.i2c_devices.insert(
        "U6".to_owned(),
        SolvedI2cDevice {
            index: 2,
            part: "W25Q64JV".to_owned(),
            bus: "SENSORS".to_owned(),
            peripheral: "I2C1".to_owned(),
            address: 0x50,
        },
    );
    let e = platform::emit(&t).unwrap_err();
    assert_eq!(
        e,
        TwinError::ModelOffItsBus {
            reference: "U6".into(),
            part: "W25Q64JV".into()
        }
    );
    // The BME280 is I2C-only, declared as an SPI device.
    let mut t = reference();
    t.spi_devices
        .insert("U6".to_owned(), spi_device(1, "BME280", "FLASH_CS"));
    let e = platform::emit(&t).unwrap_err();
    assert_eq!(e.kind(), "ModelOffItsBus", "{e:?}");
}

#[test]
fn a_reference_that_collides_with_an_emitted_name_is_the_topologys_refusal() {
    // `spi1mux` is a name the emitter declares for SPI1.
    let mut t = reference();
    let u4 = t.spi_devices.remove("U4").unwrap();
    t.spi_devices.insert("SPI1MUX".to_owned(), u4);
    let e = platform::emit(&t).unwrap_err();
    assert_eq!(e.kind(), "InvalidTopology", "{e:?}");

    // An I2C device named for U4's memory.
    let mut t = reference();
    t.i2c_devices.insert(
        "u4_memory".to_owned(),
        SolvedI2cDevice {
            index: 2,
            part: "BME280".to_owned(),
            bus: "SENSORS".to_owned(),
            peripheral: "I2C1".to_owned(),
            address: 0x77,
        },
    );
    let e = platform::emit(&t).unwrap_err();
    assert!(
        matches!(
            e,
            TwinError::InvalidTopology(IrValidationError::ReferenceCollision { .. })
        ),
        "{e:?}"
    );
}

#[test]
fn a_spi_device_path_is_the_controller_its_mux_and_the_lowercased_reference() {
    assert_eq!(
        platform::spi_device_path("SPI1", "U4").unwrap(),
        "spi1.spi1mux.u4"
    );
    assert_eq!(
        platform::spi_device_path("SPI4", "Imu_2").unwrap(),
        "spi4.spi4mux.imu_2"
    );
    let e = platform::spi_device_path("I2C1", "U4").unwrap_err();
    assert_eq!(
        e,
        TwinError::DeviceOffBus {
            reference: "U4".into(),
            peripheral: "I2C1".into()
        }
    );
}
