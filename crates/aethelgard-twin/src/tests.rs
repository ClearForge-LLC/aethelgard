use aethelgard_ir::SolvedTopology;
use aethelgard_svd_hal::cmsis;

use crate::boot::{self, Register};
use crate::platform;

/// The reference topology, as `aethelgard solve` gives it for
/// `examples/sentinel-01-minimal.json`.
const REFERENCE: &str = r#"{"schema_version":1,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000}}"#;

fn reference() -> SolvedTopology {
    serde_json::from_str(REFERENCE).unwrap()
}

/// The reference with `edit` applied to its JSON text.
fn edited(from: &str, to: &str) -> SolvedTopology {
    assert!(REFERENCE.contains(from), "{from}");
    serde_json::from_str(&REFERENCE.replace(from, to)).unwrap()
}

#[test]
fn the_reference_platform_adds_spi1_and_names_every_solved_peripheral() {
    let text = platform::emit(&reference(), &[]).unwrap();
    let declarations: Vec<&str> = text
        .lines()
        .filter(|l| !l.starts_with("//") && !l.starts_with(' ') && !l.is_empty())
        .collect();
    assert_eq!(
        declarations,
        [
            "using \"platforms/cpus/stm32h753.repl\"",
            "spi1: SPI.STM32H7_SPI @ {"
        ]
    );
    // The spike's measured block (`spike/2100/out/sentinel-01.repl`, loaded
    // by `simulate` at 459005f), byte for byte.
    assert!(text.ends_with(
        "spi1: SPI.STM32H7_SPI @ {\n        sysbus 0x40013000;\n        rcc RegistrationPeripheralAlias.SPI1\n    }\n    IRQ -> nvic@35\n"
    ));
    let names: Vec<(String, bool)> = platform::instances(&reference(), &[])
        .unwrap()
        .into_iter()
        .map(|i| (i.name, i.added))
        .collect();
    assert_eq!(
        names,
        [
            ("gpioPortA".into(), false),
            ("gpioPortB".into(), false),
            ("i2c1".into(), false),
            ("spi1".into(), true)
        ]
    );
}

#[test]
fn two_emissions_are_byte_identical() {
    assert_eq!(
        platform::emit(&reference(), &[]).unwrap(),
        platform::emit(&reference(), &[]).unwrap()
    );
}

#[test]
fn an_added_spi_takes_its_base_and_interrupt_from_the_header() {
    let f = cmsis::register_facts().unwrap();
    let t = edited("\"SPI1", "\"SPI2").clone();
    let text = platform::emit(&t, &[]).unwrap();
    let block = format!(
        "spi2: SPI.STM32H7_SPI @ {{\n        sysbus 0x{:08X};\n        rcc RegistrationPeripheralAlias.SPI2\n    }}\n    IRQ -> nvic@{}\n",
        f.base("SPI2").unwrap(),
        f.irq("SPI2").unwrap()
    );
    assert!(text.ends_with(&block), "{text}");
    // A solved SPI4 is Renode's own `spi4`, never declared twice (§2.9).
    let t = edited("\"SPI1", "\"SPI4");
    let text = platform::emit(&t, &[]).unwrap();
    assert!(!text.contains("spi4:"), "{text}");
}

#[test]
fn what_cannot_be_lowered_is_refused() {
    let kind = |t: SolvedTopology| platform::emit(&t, &[]).unwrap_err().kind();
    assert_eq!(
        kind(edited("\"I2C1\"", "\"USART1\"")),
        "UnsupportedPeripheral"
    );
    assert_eq!(kind(edited("\"I2C1\"", "\"I2C5\"")), "Undefined");
    assert_eq!(kind(edited("\"PA0\"", "\"PZ0\"")), "InvalidPin");
    assert_eq!(
        kind(edited("\"schema_version\":1", "\"schema_version\":2")),
        "InvalidTopology"
    );
    assert_eq!(
        boot::expected(&edited("\"PA0\"", "\"PA16\""), &[])
            .unwrap_err()
            .kind(),
        "InvalidPin"
    );
}

#[test]
fn every_stock_address_is_the_header_s() {
    let f = cmsis::register_facts().unwrap();
    for (peripheral, name, _, renode) in platform::STOCK {
        assert_eq!(f.base(peripheral).unwrap(), renode, "{name}");
    }
}

#[test]
fn the_memory_and_the_boot_word_come_from_the_header() {
    let m = boot::memory().unwrap();
    assert_eq!((m.flash.start, m.flash.end), (0x0800_0000, 0x0820_0000));
    assert_eq!((m.ram.start, m.ram.end), (0x2000_0000, 0x2002_0000));
    assert_eq!(boot::boot_word().unwrap(), 0x2000_0000);
    assert!(m.ram.holds(0x2000_0000, 4));
    assert!(m.ram.holds(0x2001_FFFC, 4));
    assert!(!m.ram.holds(0x2001_FFFD, 4));
    assert!(!m.ram.holds(0x1FFF_FFFF, 1));
    assert!(!m.ram.holds(u64::MAX, 2));
}

fn reg(name: &str, address: u32, mask: u32, value: u32) -> Register {
    Register {
        name: name.into(),
        address,
        mask,
        value,
    }
}

/// The values the spike read back from Renode after the reference firmware's
/// boot from a cold RCC (`spike/2100/out/s3/s3-image-cold.out`); here they are
/// computed from the topology alone.
#[test]
fn the_reference_image_is_the_one_the_spike_read_back() {
    let image = boot::expected(&reference(), &[]).unwrap();
    assert_eq!(
        image.registers,
        [
            reg("GPIOA.MODER", 0x5802_0000, 0x0000_FC03, 0x0000_A801),
            reg("GPIOA.AFRL", 0x5802_0020, 0xFFF0_0000, 0x5550_0000),
            reg("GPIOB.MODER", 0x5802_0400, 0x0000_F000, 0x0000_A000),
            reg("GPIOB.AFRL", 0x5802_0420, 0xFF00_0000, 0x4400_0000),
            reg("RCC.AHB4ENR", 0x5802_44E0, u32::MAX, 0x0000_0003),
            reg("RCC.APB1LENR", 0x5802_44E8, u32::MAX, 0x0020_0000),
            reg("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0x0000_1000),
        ]
    );
    assert_eq!(
        image.cold,
        [
            reg("RCC.AHB4ENR", 0x5802_44E0, u32::MAX, 0),
            reg("RCC.APB1LENR", 0x5802_44E8, u32::MAX, 0),
            reg("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0),
        ]
    );
    assert_eq!(image.unobservable, ["GPIOB.OTYPER", "I2C1.TIMINGR"]);
}

#[test]
fn a_pin_above_seven_lands_in_afrh() {
    let image = boot::expected(&edited("\"PB7\"", "\"PB9\""), &[]).unwrap();
    assert!(
        image
            .registers
            .contains(&reg("GPIOB.AFRH", 0x5802_0424, 0x0000_00F0, 0x0000_0040)),
        "{:?}",
        image.registers
    );
}

fn target(reference: &str, part: &str, address: u8) -> aethelgard_ir::parts::I2cTarget {
    aethelgard_ir::parts::I2cTarget {
        reference: reference.into(),
        part: part.into(),
        peripheral: "I2C1".into(),
        address,
    }
}

fn reference_devices() -> Vec<aethelgard_ir::parts::I2cTarget> {
    vec![target("U2", "BME280", 0x76), target("U3", "TMP117", 0x48)]
}

#[test]
fn a_modelled_device_is_declared_and_an_unmodelled_one_is_listed() {
    let text = platform::emit(&reference(), &reference_devices()).unwrap();
    assert!(text.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"), "{text}");
    assert!(!text.contains("u3:"), "{text}");
    assert!(
        text.contains("Unmodelled I2C devices (no Renode 1.17.0 model): U3 (TMP117) at I2C1 0x48."),
        "{text}"
    );
    let none = platform::emit(&reference(), &[]).unwrap();
    assert!(none.contains("model): none."), "{none}");
    let listed: Vec<(String, String)> = platform::instances(&reference(), &reference_devices())
        .unwrap()
        .into_iter()
        .map(|i| (i.name, i.model))
        .collect();
    assert!(listed.contains(&("u2".into(), "BME280".into())));
    assert!(!listed.iter().any(|(n, _)| n == "u3"));
    assert_eq!(
        text,
        platform::emit(&reference(), &reference_devices()).unwrap()
    );
}

#[test]
fn a_device_that_would_break_the_load_is_refused() {
    let kind = |d: Vec<aethelgard_ir::parts::I2cTarget>| {
        platform::emit(&reference(), &d).unwrap_err().kind()
    };
    // A stock instance's name, a repeated name, and a name Renode cannot take.
    assert_eq!(
        kind(vec![target("I2C1", "BME280", 0x76)]),
        "InvalidDeviceName"
    );
    assert_eq!(
        kind(vec![
            target("U2", "BME280", 0x76),
            target("u2", "BME280", 0x77)
        ]),
        "InvalidDeviceName"
    );
    assert_eq!(
        kind(vec![target("2U", "BME280", 0x76)]),
        "InvalidDeviceName"
    );
    assert_eq!(
        kind(vec![target("U 2", "TMP117", 0x48)]),
        "InvalidDeviceName"
    );
    let mut off = target("U2", "BME280", 0x76);
    off.peripheral = "I2C2".into();
    assert_eq!(kind(vec![off]), "DeviceOffBus");
}

#[test]
fn the_judge_expects_the_part_table_s_values_in_records_after_the_boot_word() {
    let image = boot::expected(&reference(), &reference_devices()).unwrap();
    let word = boot::boot_word().unwrap();
    assert_eq!(image.i2c.checks.len(), 1, "U3 has no model, so no read");
    let c = &image.i2c.checks[0];
    assert_eq!(
        (c.reference.as_str(), c.address, c.register),
        ("U2", 0x76, 0xD0)
    );
    assert_eq!((c.width, c.expected), (1, 0x60));
    assert_eq!((c.status_at, c.value_at), (word + 4, word + 8));
    assert_eq!(image.i2c.unmodelled.len(), 1);
    assert_eq!(image.i2c.unmodelled[0].reference, "U3");

    use boot::Outcome::*;
    assert_eq!(c.judge(boot::STATUS_OK, 0x60), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x61), Mismatch);
    assert_eq!(c.judge(boot::STATUS_TXIS, 0x60), Timeout("TXIS"));
    assert_eq!(c.judge(boot::STATUS_RXNE, 0), Timeout("RXNE"));
    assert_eq!(c.judge(boot::RECORD_UNSET, boot::RECORD_UNSET), Invalid);
    assert_eq!(c.judge(0, 0x60), Invalid);
}

#[test]
fn no_devices_means_no_reads_and_nothing_unmodelled() {
    let image = boot::expected(&reference(), &[]).unwrap();
    assert!(image.i2c.checks.is_empty() && image.i2c.unmodelled.is_empty());
}

#[test]
fn every_modelled_part_has_an_identity_so_no_device_passes_unchecked() {
    // Adversarial 3: a part with a model and no identity would be declared in
    // the platform, never read and never reported. Today none exists; this
    // makes adding one a failing test, not a silent gap.
    for (part, _) in crate::model::MODELS {
        assert!(aethelgard_ir::parts::identity(part).is_some(), "{part}");
    }
}

#[test]
fn a_part_name_that_is_not_exactly_a_table_s_is_unmodelled_not_skipped() {
    // Adversarial 3: near-miss names fall to "unmodelled", which is reported.
    let devices: Vec<_> = [
        "bme280",
        "BME280 ",
        "BME 280",
        "",
        "TMP117",
        "BME280\u{200b}",
    ]
    .iter()
    .enumerate()
    .map(|(i, part)| target(&format!("U{i}"), part, 0x40 + i as u8))
    .collect();
    let plan = boot::i2c(&devices).unwrap();
    assert!(plan.checks.is_empty());
    assert_eq!(plan.unmodelled.len(), devices.len());
}

#[test]
fn a_two_byte_identity_is_judged_whole_and_most_significant_byte_first() {
    let c = boot::I2cCheck {
        reference: "U3".into(),
        peripheral: "I2C1".into(),
        address: 0x48,
        register: 0x0F,
        width: 2,
        expected: 0x0117,
        status_at: 0x2000_0004,
        value_at: 0x2000_0008,
    };
    use boot::Outcome::*;
    assert_eq!(c.judge(boot::STATUS_OK, 0x0117), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x1701), Mismatch);
    assert_eq!(c.judge(boot::STATUS_OK, 0x17), Mismatch);
}

/// The reference boot's controller state, as measured on Renode 1.17.0 after
/// the generated read: `I2C1.CR2` and `I2C1.TXDR`.
const MEASURED_CR2: u32 = 0x0201_04EC;
const MEASURED_TXDR: u32 = 0xD0;

#[test]
fn the_controller_witness_is_computed_from_the_address_and_the_part_table() {
    let image = boot::expected(&reference(), &reference_devices()).unwrap();
    let [w] = image.i2c.witnesses.as_slice() else {
        panic!("one bus, one witness");
    };
    assert_eq!(
        (w.peripheral.as_str(), w.reference.as_str(), w.check),
        ("I2C1", "U2", 0)
    );
    assert_eq!((w.cr2_at, w.txdr_at), (0x4000_5404, 0x4000_5428));
    assert_eq!(w.cr2_mask(), 0x02FF_07FF);
    assert_eq!(w.cr2_expected(), MEASURED_CR2);
    assert_eq!(w.txdr_expected(), MEASURED_TXDR);
    assert!(w.differing(MEASURED_CR2, MEASURED_TXDR).is_empty());
    // START is not stored once set, and bits outside the fields do not count.
    assert!(
        w.differing(MEASURED_CR2 | 1 << 13 | 1 << 11, MEASURED_TXDR | 0x100)
            .is_empty()
    );
}

#[test]
fn each_field_of_the_witness_is_judged_alone() {
    let image = boot::expected(&reference(), &reference_devices()).unwrap();
    let w = &image.i2c.witnesses[0];
    // (the CR2 or TXDR the controller shows, the one field that must differ)
    let cases: [(u32, u32, &str, u32, u32); 5] = [
        (MEASURED_CR2 ^ 0x2, MEASURED_TXDR, "SADD", 0xEC, 0xEE),
        (MEASURED_CR2 & !(1 << 10), MEASURED_TXDR, "RD_WRN", 1, 0),
        (
            MEASURED_CR2 & !(0xFF << 16) | 2 << 16,
            MEASURED_TXDR,
            "NBYTES",
            1,
            2,
        ),
        (MEASURED_CR2 & !(1 << 25), MEASURED_TXDR, "AUTOEND", 1, 0),
        (MEASURED_CR2, 0xD1, "TXDR", 0xD0, 0xD1),
    ];
    for (cr2, txdr, name, expected, got) in cases {
        assert_eq!(w.differing(cr2, txdr), [(name, expected, got)], "{name}");
    }
    // A controller no transaction touched shows all five.
    let names: Vec<&str> = w.differing(0, 0).iter().map(|d| d.0).collect();
    assert_eq!(names, ["SADD", "RD_WRN", "NBYTES", "AUTOEND", "TXDR"]);
}

#[test]
fn only_the_last_read_on_a_bus_is_witnessed() {
    // No second modelled part exists, so the same part twice stands for it.
    let devices = vec![target("U2", "BME280", 0x76), target("U4", "BME280", 0x77)];
    let image = boot::expected(&reference(), &devices).unwrap();
    assert_eq!(image.i2c.checks.len(), 2);
    let [w] = image.i2c.witnesses.as_slice() else {
        panic!()
    };
    assert_eq!((w.reference.as_str(), w.check, w.address), ("U4", 1, 0x77));
}
