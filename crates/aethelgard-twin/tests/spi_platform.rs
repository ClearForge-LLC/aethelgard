//! `AET-WO-4200` tier 1: the SPI half of the platform emitter and of the boot
//! judge's plan, on the reference topology (schema 4: U2 BME280 and U3 TMP117
//! on I2C1, U4 W25Q64JVxIQ on SPI1 with chip select FLASH_CS on PA0). Nothing
//! here starts Renode.

use std::collections::BTreeSet;

use aethelgard_ir::{IrValidationError, SolvedI2cDevice, SolvedSpiDevice, SolvedTopology};
use aethelgard_twin::TwinError;
use aethelgard_twin::boot::{self, RECORD_BYTES, Wire};
use aethelgard_twin::platform;

/// The reference topology, as `src/tests.rs` has it.
const REFERENCE: &str = r#"{"schema_version":20,"specification_sha256":"30673b074ee63ac8373134a838fe733b0759eb98584b455fc586bc9814f60bda","assignments":{"CONSOLE_RX":{"pin":"PA10","peripheral":"USART1","signal":"USART1_RX","alternate_function":7},"CONSOLE_TX":{"pin":"PA9","peripheral":"USART1","signal":"USART1_TX","alternate_function":7},"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":{"pin":"PA0","direction":"output","initial":"high"}},"adc_inputs":{},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{"U4":{"index":0,"part":"W25Q64JVxIQ","bus":"FLASH","peripheral":"SPI1","cs":"FLASH_CS"}},"spi_lines":{"SPI_MOSI":{"bus":"FLASH","signal":"MOSI","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135},"SPI_SCK":{"bus":"FLASH","signal":"SCK","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135}},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false,"vos":"scale2","kernel_switch":{"pllckselr":80,"pllcfgr":131084,"pll1divr":16843279}},"motor_outputs":{},"power":null,"serial_ports":{"CONSOLE":{"index":0,"peripheral":"USART1","tx":"CONSOLE_TX","rx":"CONSOLE_RX","role":"console","framing":{"baud":115200,"data_bits":8,"parity":"none","stop_bits":1,"inverted":false},"negotiated":null,"peer_deviation_ppm":5000,"kernel_hz":64000000,"brr":556,"actual_millibaud":115107913,"error_ppm":-799}},"spi_controllers":{"SPI1":{"kernel":"per_ck","kernel_hz":64000000,"declared_hz":20000000,"mbr":1,"achieved_hz":16000000,"achieved_max_hz":16239744}},"i2c_timings":{"I2C1":{"kernel_hz":64000000,"cr1":0,"timingr":{"Fast400k":281481788,"FastPlus1M":11669037,"Standard100k":1356016192}}},"bor_level":3,"core_supply":"ldo","mcu":"STM32H753ZITx","pin_speeds":{"CONSOLE_RX":0,"CONSOLE_TX":0,"FLASH_CS":0,"I2C_SCL":0,"I2C_SDA":0,"SPI_MISO":0,"SPI_MOSI":1,"SPI_SCK":1},"foreign_clock":null}"#;

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
        orientation: None,
    }
}

/// The reference plus ICM20948 `U5` on SPI1, chip select `IMU_CS` on `PA1`.
fn with_imu() -> SolvedTopology {
    let mut t = reference();
    // The ICM20948 clocks at 7 MHz at most; the bus runs at that rate.
    for line in t.spi_lines.values_mut() {
        line.rate_hz = 7_000_000;
    }
    t.gpio.insert(
        "IMU_CS".to_owned(),
        aethelgard_ir::pin_devices::SolvedGpio::output("PA1", aethelgard_ir::motor::Level::High),
    );
    t.spi_devices
        .insert("U5".to_owned(), spi_device(1, "ICM20948", "IMU_CS"));
    t.derive_controllers();
    t
}

/// The lines of `text` from the multiplexer's declaration to the brown-out
/// level's presentation, which always closes the platform (`AET-WO-4230`
/// D10), without comments or blank lines.
fn spi_declarations(text: &str) -> Vec<&str> {
    let from = text.find("spi1mux: SPI.SPIMultiplexer").expect(text);
    text[from..]
        .lines()
        .take_while(|l| *l != "flashController:")
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
    // U5, a scripted part, makes three steps (`AET-WO-4210`).
    assert_eq!(refs, ["U2", "U4", "U5", "U5", "U5"]);
    let steps: Vec<&str> = plan.checks[2..].iter().map(|c| c.step.name()).collect();
    assert_eq!(steps, ["identity", "configure", "data"]);
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
    // Records follow the boot word's, consecutive: the status, then as many
    // value words as the read needs (the data read's six bytes take two).
    let first = plan.checks[0].status_at;
    let mut at = first;
    for c in &plan.checks {
        assert_eq!(c.status_at, at);
        assert_eq!(c.value_at, c.status_at + 4);
        at += 4 + 4 * boot::value_words(c.width);
    }
    assert_eq!(plan.checks[1].status_at, first + RECORD_BYTES);
    assert_eq!(at - first, 2 * RECORD_BYTES + 8 + 8 + 12);
    assert_eq!(first, boot::boot_word().unwrap() + boot::RECORD_OFFSET);

    // U5's witness is the ICM's four lines, and its skipped line is the one
    // chip-select edge the model logs.
    let u5 = &plan.checks[2];
    assert_eq!(u5.device, "spi1.spi1mux.u5");
    assert_eq!((u5.write.as_slice(), u5.width), (&[0x80][..], 1));
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
        text.contains("U4 (W25Q64JVxIQ) is absent behind its chip select PA0"),
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
    // The W25Q64JVxIQ is SPI-only, declared as an I2C device.
    let mut t = reference();
    t.i2c_devices.insert(
        "U6".to_owned(),
        SolvedI2cDevice {
            index: 2,
            part: "W25Q64JVxIQ".to_owned(),
            bus: "SENSORS".to_owned(),
            peripheral: "I2C1".to_owned(),
            address: 0x50,
        },
    );
    // Since `AET-WO-4300c` §1.5 the topology holds every listed part to its
    // bus kind, so it refuses first; `ModelOffItsBus` stays the emitter's own
    // check behind it.
    let e = platform::emit(&t).unwrap_err();
    assert!(
        matches!(&e, TwinError::InvalidTopology(v) if v.kind() == "PartOnWrongBus" && v.id() == Some("U6")),
        "{e:?}"
    );
    // The BME280 is I2C-only, declared as an SPI device.
    let mut t = reference();
    t.spi_devices
        .insert("U6".to_owned(), spi_device(1, "BME280", "FLASH_CS"));
    let e = platform::emit(&t).unwrap_err();
    assert!(
        matches!(&e, TwinError::InvalidTopology(v) if v.kind() == "PartOnWrongBus"),
        "{e:?}"
    );
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

/// `AET-WO-1206` §1.6: what makes the twin's I2C `ModelOffItsBus` check
/// unreachable, so it was deleted. Every part the table gives a Renode model
/// sits, in its row, on the one bus the twin attaches that model to; the
/// topology refuses a part off its row's bus first (`PartOnWrongBus`).
#[test]
fn every_modelled_part_s_row_is_on_its_model_s_bus() {
    let table = aethelgard_ir::parts::table().unwrap();
    let mut modelled = 0;
    for p in table.parts() {
        if let Some(name) = &p.renode {
            let model = aethelgard_twin::model::Model::from_renode(name).unwrap();
            assert_eq!(model.bus(), p.bus.kind(), "{}", p.part);
            modelled += 1;
        }
    }
    assert!(modelled > 0);
}

/// `AET-WO-1206` §1.6: the SPI `ModelOffItsBus` check is kept, because
/// `platform::spi_wiring` is public and reads a topology it does not
/// validate: a hand-written one with an I2C-only model on SPI reaches it.
/// `validate` and the topology's own check pre-empt it on every other path.
#[test]
fn an_i2c_model_on_spi_reaches_the_kept_guard_through_spi_wiring() {
    let mut t = reference();
    t.spi_devices
        .insert("U6".to_owned(), spi_device(1, "BME280", "FLASH_CS"));
    let e = platform::spi_wiring(&t).unwrap_err();
    assert!(
        matches!(&e, TwinError::ModelOffItsBus { reference, part } if reference == "U6" && part == "BME280"),
        "{e:?}"
    );
}

/// `AET-WO-1211c`: the platform emitter and the boot judge take a topology
/// with a pin device (until `1211c` they refused it, `PinDeviceNotSimulated`),
/// and the image holds a declared input in input mode.
#[test]
fn a_topology_with_a_pin_device_is_emitted_and_its_input_imaged() {
    use aethelgard_ir::pin_devices::{Direction, McuPull, SolvedGpio, SolvedPinDevice};
    let mut t = reference();
    t.pin_devices.insert(
        "U9".to_owned(),
        SolvedPinDevice {
            part: "WAKE_SWITCH".to_owned(),
            bridges: Default::default(),
            supplies: Default::default(),
        },
    );
    t.gpio.insert(
        "WAKE".to_owned(),
        SolvedGpio {
            pin: "PB5".to_owned(),
            direction: Direction::Input,
            initial: None,
            pull: Some(McuPull::Up),
            active: Some(aethelgard_ir::motor::Level::Low),
            pulled: Some(aethelgard_ir::motor::Level::High),
        },
    );
    t.pin_speeds.insert("WAKE".to_owned(), 0);
    t.validate().unwrap();
    // AET-WO-1211c: the platform and the image take a pin device; what of
    // one the twin cannot run is the plan's to refuse (`pins::plan`).
    assert_eq!(
        platform::emit(&t).unwrap(),
        platform::emit_faulted(&t, &set(&[]), &set(&[])).unwrap()
    );
    let image = boot::expected(&t).unwrap();
    let moder = image
        .registers
        .iter()
        .find(|r| r.name == "GPIOB.MODER")
        .expect("PB5's port is configured");
    // PB5 an input: `MODER` 00 (RM0433 Rev 9 §11.4.1), held in the image.
    assert_eq!(
        (moder.mask >> 10 & 0b11, moder.value >> 10 & 0b11),
        (0b11, 0b00)
    );
}
