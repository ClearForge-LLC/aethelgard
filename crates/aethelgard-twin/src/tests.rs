use aethelgard_ir::SolvedTopology;
use aethelgard_svd_hal::cmsis;

use crate::boot::{self, Register};
use crate::platform;

/// The reference topology, as `aethelgard solve` gives it for
/// `examples/sentinel-01-minimal.json`.
pub(crate) const REFERENCE: &str = r#"{"schema_version":3,"specification_sha256":"acd60081f2d111b1b1ed0d974078fff45ccf2a967ec5f42317417f8ce68149d1","assignments":{"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{"U4":{"index":0,"part":"W25Q64JV","bus":"FLASH","peripheral":"SPI1","cs":"FLASH_CS"}}}"#;

fn reference() -> SolvedTopology {
    serde_json::from_str(REFERENCE).unwrap()
}

/// The reference with no device, I2C or SPI.
pub(crate) fn bare() -> SolvedTopology {
    let mut t = reference();
    t.i2c_devices.clear();
    t.spi_devices.clear();
    t
}

/// The reference with exactly `devices` as its I2C devices, `(reference,
/// part, address)` in the order given, all on `I2C1`.
pub(crate) fn with_devices(devices: &[(&str, &str, u8)]) -> SolvedTopology {
    let mut t = bare();
    for (index, &(reference, part, address)) in devices.iter().enumerate() {
        t.i2c_devices.insert(
            reference.to_owned(),
            aethelgard_ir::SolvedI2cDevice {
                index: u32::try_from(index).unwrap(),
                part: part.to_owned(),
                bus: "SENSORS".to_owned(),
                peripheral: "I2C1".to_owned(),
                address,
            },
        );
    }
    t
}

/// The reference with `edit` applied to its JSON text.
fn edited(from: &str, to: &str) -> SolvedTopology {
    assert!(REFERENCE.contains(from), "{from}");
    let mut t: SolvedTopology = serde_json::from_str(&REFERENCE.replace(from, to)).unwrap();
    t.i2c_devices.clear();
    t.spi_devices.clear();
    t
}

#[test]
fn the_reference_platform_adds_spi1_and_names_every_solved_peripheral() {
    let text = platform::emit(&bare()).unwrap();
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
    let names: Vec<(String, bool)> = platform::instances(
        &bare(),
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    )
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
        platform::emit(&bare()).unwrap(),
        platform::emit(&bare()).unwrap()
    );
}

#[test]
fn an_added_spi_takes_its_base_and_interrupt_from_the_header() {
    let f = cmsis::register_facts().unwrap();
    let t = edited("\"SPI1", "\"SPI2").clone();
    let text = platform::emit(&t).unwrap();
    let block = format!(
        "spi2: SPI.STM32H7_SPI @ {{\n        sysbus 0x{:08X};\n        rcc RegistrationPeripheralAlias.SPI2\n    }}\n    IRQ -> nvic@{}\n",
        f.base("SPI2").unwrap(),
        f.irq("SPI2").unwrap()
    );
    assert!(text.ends_with(&block), "{text}");
    // A solved SPI4 is Renode's own `spi4`, never declared twice (§2.9).
    let t = edited("\"SPI1", "\"SPI4");
    let text = platform::emit(&t).unwrap();
    assert!(!text.contains("spi4:"), "{text}");
}

#[test]
fn what_cannot_be_lowered_is_refused() {
    let kind = |t: SolvedTopology| platform::emit(&t).unwrap_err().kind();
    assert_eq!(
        kind(edited("\"SPI1\"", "\"USART1\"")),
        "UnsupportedPeripheral"
    );
    // Every I2C1 name, signals too: an analog line must stay an I2C line.
    let t = SolvedTopology {
        assignments: reference()
            .assignments
            .into_iter()
            .map(|(net, mut a)| {
                a.peripheral = a.peripheral.replace("I2C1", "I2C5");
                a.signal = a.signal.replace("I2C1", "I2C5");
                (net, a)
            })
            .collect(),
        ..bare()
    };
    assert_eq!(kind(t), "Undefined");
    assert_eq!(kind(edited("\"PA0\"", "\"PZ0\"")), "InvalidPin");
    assert_eq!(
        kind(edited("\"schema_version\":3", "\"schema_version\":4")),
        "InvalidTopology"
    );
    assert_eq!(
        boot::expected(&{
            let mut t = edited("\"PA0\"", "\"PA16\"");
            t.i2c_devices.clear();
            t
        })
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
    let image = boot::expected(&bare()).unwrap();
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
    let image = boot::expected(&{
        let mut t = edited("\"PB7\"", "\"PB9\"");
        t.i2c_devices.clear();
        t
    })
    .unwrap();
    assert!(
        image
            .registers
            .contains(&reg("GPIOB.AFRH", 0x5802_0424, 0x0000_00F0, 0x0000_0040)),
        "{:?}",
        image.registers
    );
}

#[test]
fn a_modelled_device_is_declared_and_an_unmodelled_one_is_listed() {
    let text = platform::emit(&reference()).unwrap();
    assert!(text.contains("\nu2: I2C.BME280 @ i2c1 0x76\n"), "{text}");
    assert!(!text.contains("u3:"), "{text}");
    assert!(
        text.contains("Unmodelled I2C devices (no Renode 1.17.0 model): U3 (TMP117) at I2C1 0x48."),
        "{text}"
    );
    let none = platform::emit(&bare()).unwrap();
    assert!(none.contains("model): none."), "{none}");
    let listed: Vec<(String, String)> = platform::instances(
        &reference(),
        &std::collections::BTreeSet::new(),
        &std::collections::BTreeSet::new(),
    )
    .unwrap()
    .into_iter()
    .map(|i| (i.name, i.model))
    .collect();
    assert!(listed.contains(&("u2".into(), "BME280".into())));
    assert!(!listed.iter().any(|(n, _)| n == "u3"));
    assert_eq!(text, platform::emit(&reference()).unwrap());
}

#[test]
fn a_device_that_would_break_the_load_is_refused() {
    let kind = |d: &[(&str, &str, u8)]| platform::emit(&with_devices(d)).unwrap_err().kind();
    // A stock instance's name is the topology's refusal, which comes first
    // (`AET-WO-2204` D5: `validate` refuses a reference that is a stock name);
    // a name repeated in another case is the topology's too since
    // `AET-WO-4200` D7 (`SolvedTopology::validate` folds references).
    assert_eq!(kind(&[("I2C1", "BME280", 0x76)]), "InvalidTopology");
    assert_eq!(
        kind(&[("U2", "BME280", 0x76), ("u2", "BME280", 0x77)]),
        "InvalidTopology"
    );
    // A name that is no identifier at all, or that Renode cannot take (a
    // leading digit, `-`), is the topology's refusal, which comes first
    // (`AET-WO-2203`: `validate` checks the keys; `AET-WO-1102`: a reference
    // has Renode's shape there).
    assert_eq!(kind(&[("U 2", "TMP117", 0x48)]), "InvalidTopology");
    assert_eq!(kind(&[("2U", "BME280", 0x76)]), "InvalidTopology");
    assert_eq!(kind(&[("U-2", "BME280", 0x76)]), "InvalidTopology");
}

#[test]
fn the_judge_expects_the_part_table_s_values_in_records_after_the_boot_word() {
    let image = boot::expected(&reference()).unwrap();
    let word = boot::boot_word().unwrap();
    assert_eq!(
        image.identity.checks.len(),
        2,
        "U3 has no model, so no read; U2, then U4 on SPI"
    );
    let c = &image.identity.checks[0];
    assert_eq!(
        (c.reference.as_str(), &c.wire, c.write),
        ("U2", &boot::Wire::I2c { address: 0x76 }, 0xD0)
    );
    let f = &image.identity.checks[1];
    assert_eq!(
        (f.reference.as_str(), f.write, f.width, f.expected),
        ("U4", 0x9F, 3, 0xEF_4017)
    );
    assert_eq!((f.status_at, f.value_at), (word + 12, word + 16));
    assert_eq!(f.judge(boot::STATUS_RXP, 0), boot::Outcome::Timeout("RXP"));
    assert_eq!(f.judge(boot::STATUS_TXIS, 0), boot::Outcome::Invalid);
    assert_eq!(c.judge(boot::STATUS_RXP, 0x60), boot::Outcome::Invalid);
    assert_eq!((c.width, c.expected), (1, 0x60));
    assert_eq!((c.status_at, c.value_at), (word + 4, word + 8));
    assert_eq!(image.identity.unmodelled.len(), 1);
    assert_eq!(image.identity.unmodelled[0].reference, "U3");

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
    let image = boot::expected(&bare()).unwrap();
    assert!(image.identity.checks.is_empty() && image.identity.unmodelled.is_empty());
}

#[test]
fn every_modelled_part_has_an_identity_so_no_device_passes_unchecked() {
    // Adversarial 3: a part with a model and no identity would be declared in
    // the platform, never read and never reported. Today none exists; this
    // makes adding one a failing test, not a silent gap.
    let table = aethelgard_ir::parts::table().unwrap();
    for p in table.parts().iter().filter(|p| p.renode.is_some()) {
        assert!(crate::model::kind(&p.part).unwrap().is_some(), "{}", p.part);
    }
}

#[test]
fn a_part_name_that_is_not_exactly_a_table_s_is_unmodelled_not_skipped() {
    // Adversarial 3: near-miss names fall to "unmodelled", which is reported.
    let parts = [
        "bme280",
        "BME280 ",
        "BME 280",
        "",
        "TMP117",
        "BME280\u{200b}",
    ];
    let names: Vec<String> = (0..parts.len()).map(|i| format!("U{i}")).collect();
    let devices: Vec<(&str, &str, u8)> = parts
        .iter()
        .enumerate()
        .map(|(i, part)| (names[i].as_str(), *part, 0x40 + i as u8))
        .collect();
    let plan = boot::identity(&with_devices(&devices)).unwrap();
    assert!(plan.checks.is_empty());
    assert_eq!(plan.unmodelled.len(), devices.len());
}

#[test]
fn a_two_byte_identity_is_judged_whole_and_most_significant_byte_first() {
    let c = boot::IdentityCheck {
        reference: "U3".into(),
        peripheral: "I2C1".into(),
        wire: boot::Wire::I2c { address: 0x48 },
        write: 0x0F,
        width: 2,
        expected: 0x0117,
        status_at: 0x2000_0004,
        value_at: 0x2000_0008,
        device: "i2c1.u3".into(),
        witness: None,
        skipped: Vec::new(),
    };
    use boot::Outcome::*;
    assert_eq!(c.judge(boot::STATUS_OK, 0x0117), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x1701), Mismatch);
    assert_eq!(c.judge(boot::STATUS_OK, 0x17), Mismatch);
}

#[test]
fn each_identity_read_carries_the_models_log_lines_of_its_device() {
    // U2 is register 0xD0, value 0x60, on I2C1: the BME280 model logs the
    // write and the read at Noisy, in upper-case hexadecimal.
    let image = boot::expected(&reference()).unwrap();
    let [c, flash] = image.identity.checks.as_slice() else {
        panic!("two modelled devices, two reads: U2 on I2C1, then U4 on SPI1");
    };
    assert_eq!(flash.device, "spi1.spi1mux.u4");
    assert_eq!(
        flash.wire,
        boot::Wire::Spi {
            address: 0,
            select: "PA0".into()
        }
    );
    assert_eq!(flash.status_at, c.status_at + boot::RECORD_BYTES);
    assert_eq!(c.device, "i2c1.u2");
    assert_eq!(
        c.witness.as_deref(),
        Some(
            [
                "[NOISY] i2c1.u2: Write D0".to_owned(),
                "[NOISY] i2c1.u2: Read 60".to_owned()
            ]
            .as_slice()
        )
    );
}

#[test]
fn the_witness_lines_are_computed_from_the_part_table_and_the_width() {
    // A two-byte identity reads most significant byte first, each byte in
    // upper-case hexadecimal without zero padding: 0x0A is "A", 0x00 is "0".
    let lines = |register, width, value| {
        crate::model::Model::Bme280
            .witness("i2c1.u9", register, width, value)
            .unwrap()
    };
    assert_eq!(
        lines(0x0F, 2, 0x1234),
        ["[NOISY] i2c1.u9: Write F", "[NOISY] i2c1.u9: Read 12 34"]
    );
    assert_eq!(lines(0x0A, 1, 0x0A)[1], "[NOISY] i2c1.u9: Read A");
    assert_eq!(lines(0xD0, 2, 0x0117)[1], "[NOISY] i2c1.u9: Read 1 17");
    assert_eq!(lines(0xD0, 2, 0x0100)[1], "[NOISY] i2c1.u9: Read 1 0");
}

#[test]
fn every_read_on_a_bus_is_witnessed_by_its_own_device() {
    // No second modelled part exists, so the same part twice stands for it.
    // Where 2202 witnessed only the last read on a bus, every check now has
    // its own device path and lines.
    let devices = [("U2", "BME280", 0x76), ("U4", "BME280", 0x77)];
    let image = boot::expected(&with_devices(&devices)).unwrap();
    let [a, b] = image.identity.checks.as_slice() else {
        panic!("two reads")
    };
    assert_eq!(
        (a.device.as_str(), b.device.as_str()),
        ("i2c1.u2", "i2c1.u4")
    );
    assert_eq!(
        b.witness.as_deref().map(|l| l[0].as_str()),
        Some("[NOISY] i2c1.u4: Write D0")
    );
    assert_ne!(a.witness, b.witness);
}
