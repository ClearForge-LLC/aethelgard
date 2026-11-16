use aethelgard_ir::SolvedTopology;
use aethelgard_svd_hal::cmsis;

use crate::boot::{self, Register};
use crate::platform;

/// The reference topology, as `aethelgard solve` gives it for
/// `examples/sentinel-01-minimal.json`.
pub(crate) const REFERENCE: &str = r#"{"schema_version":8,"specification_sha256":"f3798b1fe1084c029d7e0f259b42adb858242adfa9d311ed07eb378ea5c5b999","assignments":{"CONSOLE_RX":{"pin":"PA10","peripheral":"USART1","signal":"USART1_RX","alternate_function":7},"CONSOLE_TX":{"pin":"PA9","peripheral":"USART1","signal":"USART1_TX","alternate_function":7},"I2C_SCL":{"pin":"PB6","peripheral":"I2C1","signal":"I2C1_SCL","alternate_function":4},"I2C_SDA":{"pin":"PB7","peripheral":"I2C1","signal":"I2C1_SDA","alternate_function":4},"SPI_MISO":{"pin":"PA6","peripheral":"SPI1","signal":"SPI1_MISO","alternate_function":5},"SPI_MOSI":{"pin":"PA7","peripheral":"SPI1","signal":"SPI1_MOSI","alternate_function":5},"SPI_SCK":{"pin":"PA5","peripheral":"SPI1","signal":"SPI1_SCK","alternate_function":5}},"gpio":{"FLASH_CS":"PA0"},"i2c_ceilings":{"SENSORS":400000},"i2c_speeds":{"SENSORS":400000},"analog_lines":{"I2C_SCL":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300},"I2C_SDA":{"bus":"SENSORS","pull_up_ohms":2200,"capacitance_femtofarads":100000,"ceiling_ns":300,"domain_millivolts":3300}},"i2c_devices":{"U2":{"index":0,"part":"BME280","bus":"SENSORS","peripheral":"I2C1","address":118},"U3":{"index":1,"part":"TMP117","bus":"SENSORS","peripheral":"I2C1","address":72}},"spi_devices":{"U4":{"index":0,"part":"W25Q64JVxIQ","bus":"FLASH","peripheral":"SPI1","cs":"FLASH_CS"}},"spi_lines":{"SPI_MOSI":{"bus":"FLASH","signal":"MOSI","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135},"SPI_SCK":{"bus":"FLASH","signal":"SCK","rate_hz":20000000,"capacitance_femtofarads":30000,"rise_time_ns":20,"vdd_min_millivolts":3135}},"clock":{"source":"HSI","sysclk_hz":64000000,"hse_hz":0,"d1cpre":1,"hpre":1,"d1ppre":1,"d2ppre1":1,"d2ppre2":1,"d3ppre":1,"timpre":false},"motor_outputs":{},"power":null,"serial_ports":{"CONSOLE":{"index":0,"peripheral":"USART1","tx":"CONSOLE_TX","rx":"CONSOLE_RX","role":"console","framing":{"baud":115200,"data_bits":8,"parity":"none","stop_bits":1,"inverted":false},"negotiated":null,"peer_deviation_ppm":5000,"kernel_hz":64000000,"brr":556,"actual_millibaud":115107913,"error_ppm":-799}},"spi_controllers":{"SPI1":{"kernel":"per_ck","kernel_hz":64000000,"declared_hz":20000000,"mbr":1,"achieved_hz":16000000,"achieved_max_hz":16239744}},"i2c_timings":{"I2C1":{"kernel_hz":64000000,"cr1":0,"timingr":{"Fast400k":281481788,"FastPlus1M":11669037,"Standard100k":1356016192}}},"bor_level":3}"#;

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

/// The reference with no device and no serial port: the board the `2100`
/// spike and `4300a`'s probes read back, before `AET-WO-4300b`'s console.
pub(crate) fn pre_serial() -> SolvedTopology {
    let mut t = bare();
    t.serial_ports.clear();
    t.assignments.retain(|net, _| !net.starts_with("CONSOLE_"));
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
    t.derive_controllers();
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
            "spi1: SPI.STM32H7_SPI @ {",
            "flashController:"
        ]
    );
    // The spike's measured block (`spike/2100/out/sentinel-01.repl`, loaded
    // by `simulate` at 459005f), byte for byte, then the brown-out level's
    // presentation (`AET-WO-4230` D10).
    let presentation = boot::brown_out(&bare()).unwrap().presentation();
    assert!(text.ends_with(&format!(
        "spi1: SPI.STM32H7_SPI @ {{\n        sysbus 0x40013000;\n        rcc RegistrationPeripheralAlias.SPI1\n    }}\n    IRQ -> nvic@35\n{presentation}"
    )));
    let names: Vec<(String, bool)> = platform::instances(
        &pre_serial(),
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
    let block = block + &boot::brown_out(&t).unwrap().presentation();
    assert!(text.ends_with(&block), "{text}");
    // A solved SPI4 is Renode's own `spi4`, never declared twice (§2.9).
    let t = edited("\"SPI1", "\"SPI4");
    let text = platform::emit(&t).unwrap();
    assert!(!text.contains("spi4:"), "{text}");
}

#[test]
fn what_cannot_be_lowered_is_refused() {
    let kind = |t: SolvedTopology| platform::emit(&t).unwrap_err().kind();
    // No SPI peripheral is solved any more, so no `spi_lines` either. A
    // USART is lowered since `AET-WO-4300b`, and one without its serial
    // port is the topology's refusal; FDCAN1 has no model here.
    let mut can = edited("\"SPI1\"", "\"FDCAN1\"");
    can.spi_lines.clear();
    assert_eq!(kind(can), "UnsupportedPeripheral");
    let mut usart = edited("\"SPI1\"", "\"USART1\"");
    usart.spi_lines.clear();
    assert_eq!(kind(usart), "InvalidTopology");
    // Every I2C1 name, signals too: an analog line must stay an I2C line.
    // Since topology 8 (`AET-WO-4230`) the topology itself refuses it: an
    // I2C peripheral with no kernel clock at reset has no `i2c_timings`
    // entry `solve` could give, so `validate` refuses before the platform
    // is reached (it was the platform's `Undefined`).
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
    assert_eq!(kind(t), "InvalidTopology");
    assert_eq!(kind(edited("\"PA0\"", "\"PZ0\"")), "InvalidPin");
    assert_eq!(
        kind(edited("\"schema_version\":8", "\"schema_version\":9")),
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
/// boot from a cold RCC (`spike/2100/out/s3/s3-image-cold.out`), and since
/// `AET-WO-4300a` (2100 F24) every enable register cold at its reset value,
/// read back by the `cold9` probe (FEEDBACK P6.1); here they are computed
/// from the topology alone.
#[test]
fn the_reference_image_is_the_one_the_spike_read_back() {
    let image = boot::expected(&pre_serial()).unwrap();
    assert_eq!(
        image.registers,
        [
            reg("GPIOA.MODER", 0x5802_0000, 0x0000_FC03, 0x0000_A801),
            reg("GPIOA.AFRL", 0x5802_0020, 0xFFF0_0000, 0x5550_0000),
            reg("GPIOB.MODER", 0x5802_0400, 0x0000_F000, 0x0000_A000),
            reg("GPIOB.AFRL", 0x5802_0420, 0xFF00_0000, 0x4400_0000),
            reg("RCC.AHB3ENR", 0x5802_44D4, u32::MAX, 0),
            reg("RCC.AHB1ENR", 0x5802_44D8, u32::MAX, 0),
            reg("RCC.AHB2ENR", 0x5802_44DC, u32::MAX, 0),
            reg("RCC.AHB4ENR", 0x5802_44E0, u32::MAX, 0x0000_0003),
            reg("RCC.APB3ENR", 0x5802_44E4, u32::MAX, 0),
            reg("RCC.APB1LENR", 0x5802_44E8, u32::MAX, 0x0020_0000),
            reg("RCC.APB1HENR", 0x5802_44EC, u32::MAX, 0),
            reg("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0x0000_1000),
            reg("RCC.APB4ENR", 0x5802_44F4, u32::MAX, 0x0001_0000),
            // `AET-WO-2204` A2 (ruled D6): no interrupt armed, one ISER per
            // 32 lines up to the header's `WAKEUP_PIN_IRQn` = 149.
            reg("SysTick.CSR", 0xE000_E010, 0x0001_0003, 0),
            reg("NVIC.ISER0", 0xE000_E100, u32::MAX, 0),
            reg("NVIC.ISER1", 0xE000_E104, u32::MAX, 0),
            reg("NVIC.ISER2", 0xE000_E108, u32::MAX, 0),
            reg("NVIC.ISER3", 0xE000_E10C, u32::MAX, 0),
            reg("NVIC.ISER4", 0xE000_E110, u32::MAX, 0),
        ]
    );
    assert_eq!(
        image.cold,
        [
            reg("RCC.AHB3ENR", 0x5802_44D4, u32::MAX, 0),
            reg("RCC.AHB1ENR", 0x5802_44D8, u32::MAX, 0),
            reg("RCC.AHB2ENR", 0x5802_44DC, u32::MAX, 0),
            reg("RCC.AHB4ENR", 0x5802_44E0, u32::MAX, 0),
            reg("RCC.APB3ENR", 0x5802_44E4, u32::MAX, 0),
            reg("RCC.APB1LENR", 0x5802_44E8, u32::MAX, 0),
            reg("RCC.APB1HENR", 0x5802_44EC, u32::MAX, 0),
            reg("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0),
            reg("RCC.APB4ENR", 0x5802_44F4, u32::MAX, 0x0001_0000),
        ]
    );
    // Every configured port's `OTYPER` is written whole (`AET-WO-4230` B0):
    // port A's SPI and chip-select pins too.
    assert_eq!(
        image.unobservable,
        ["GPIOA.OTYPER", "GPIOB.OTYPER", "I2C1.TIMINGR"]
    );
}

/// The console (`AET-WO-4300b`): USART1 on PA9/PA10 at AF7, 115200 8N1.
/// Its four registers under Renode's stored bits, its pins' mode and AF, its
/// enable bit (`APB2ENR.USART1EN`, bit 4), and its five tags unobservable;
/// nothing else of the image moves.
#[test]
fn the_console_adds_its_usart_its_pins_and_its_gate() {
    let image = boot::expected(&bare()).unwrap();
    let before = boot::expected(&pre_serial()).unwrap();
    let mut added: Vec<&Register> = image
        .registers
        .iter()
        .filter(|r| !before.registers.contains(r))
        .collect();
    added.sort_by_key(|r| r.address);
    assert_eq!(
        added,
        [
            &reg("USART1.CR1", 0x4001_1000, 0x0400_87ED, 0x0000_000D),
            &reg("USART1.CR2", 0x4001_1004, 0x0000_3000, 0),
            &reg("USART1.CR3", 0x4001_1008, 0x0000_00C0, 0),
            &reg("USART1.BRR", 0x4001_100C, 0x0000_FFFF, 556),
            &reg("GPIOA.MODER", 0x5802_0000, 0x003C_FC03, 0x0028_A801),
            &reg("GPIOA.AFRH", 0x5802_0024, 0x0000_0FF0, 0x0000_0770),
            &reg("RCC.APB2ENR", 0x5802_44F0, u32::MAX, 0x0000_1010),
        ]
    );
    assert_eq!(image.cold, before.cold);
    assert_eq!(
        image.unobservable,
        [
            "GPIOA.OTYPER",
            "GPIOB.OTYPER",
            "I2C1.TIMINGR",
            "USART1.CR1.FIFOEN",
            "USART1.CR1.M0",
            "USART1.CR1.M1",
            "USART1.CR2.RXINV",
            "USART1.CR2.TXINV",
            "USART1.CR3.ONEBIT",
        ]
    );
}

/// An SBUS-like port (8E2, odd parity here for `PS`): `PCE`, `PS`, `STOP`
/// `10`, and the solved `BRR`.
#[test]
fn parity_and_two_stop_bits_reach_cr1_and_cr2() {
    let mut t = bare();
    let p = t.serial_ports.get_mut("CONSOLE").unwrap();
    p.framing.parity = aethelgard_ir::Parity::Odd;
    p.framing.stop_bits = 2;
    let image = boot::expected(&t).unwrap();
    let get = |n: &str| image.registers.iter().find(|r| r.name == n).unwrap().value;
    assert_eq!(get("USART1.CR1"), 0x0000_060D);
    assert_eq!(get("USART1.CR2"), 0x0000_2000);
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
        (c.reference.as_str(), &c.wire, c.write.as_slice()),
        ("U2", &boot::Wire::I2c { address: 0x76 }, &[0xD0][..])
    );
    let f = &image.identity.checks[1];
    assert_eq!(
        (
            f.reference.as_str(),
            f.write.as_slice(),
            f.width,
            f.expected
        ),
        ("U4", &[0x9F][..], 3, 0xEF_4017)
    );
    // After the boot word and the brown-out word (`AET-WO-4230` D10).
    assert_eq!((f.status_at, f.value_at), (word + 16, word + 20));
    assert_eq!(f.judge(boot::STATUS_RXP, 0), boot::Outcome::Timeout("RXP"));
    // B0's EOT wait: an SPI read's status, never an I2C one's.
    assert_eq!(f.judge(boot::STATUS_EOT, 0), boot::Outcome::Timeout("EOT"));
    assert_eq!(c.judge(boot::STATUS_EOT, 0x60), boot::Outcome::Invalid);
    assert_eq!(f.judge(boot::STATUS_TXIS, 0), boot::Outcome::Invalid);
    assert_eq!(c.judge(boot::STATUS_RXP, 0x60), boot::Outcome::Invalid);
    assert_eq!((c.width, c.expected), (1, 0x60));
    assert_eq!((c.status_at, c.value_at), (word + 8, word + 12));
    assert_eq!(image.identity.unmodelled.len(), 1);
    assert_eq!(image.identity.unmodelled[0].reference, "U3");

    use boot::Outcome::*;
    assert_eq!(c.judge(boot::STATUS_OK, 0x60), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x61), Mismatch);
    assert_eq!(c.judge(boot::STATUS_TXIS, 0x60), Timeout("TXIS"));
    assert_eq!(c.judge(boot::STATUS_RXNE, 0), Timeout("RXNE"));
    assert_eq!(
        c.judge(boot::RECORD_UNSET, u64::from(boot::RECORD_UNSET)),
        Invalid
    );
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
    let mut c = boot::StepCheck {
        reference: "U3".into(),
        peripheral: "I2C1".into(),
        wire: boot::Wire::I2c { address: 0x48 },
        step: aethelgard_ir::parts::StepKind::Identity,
        label: "identity",
        write: vec![0x0F],
        width: 2,
        expected: 0x0117,
        mask: 0xFFFF,
        status_at: 0x2000_0004,
        value_at: 0x2000_0008,
        level: (-1, "[NOISY] "),
        device: "i2c1.u3".into(),
        witness: None,
        idle_warning: None,
        skipped: Vec::new(),
        stimulus: Some(Vec::new()),
    };
    use boot::Outcome::*;
    assert_eq!(c.judge(boot::STATUS_OK, 0x0117), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x1701), Mismatch);
    assert_eq!(c.judge(boot::STATUS_OK, 0x17), Mismatch);
    // `AET-WO-4210` F6: a masked identity judges only its fixed bits.
    c.mask = 0x3F;
    c.expected = 0x0008;
    assert_eq!(c.judge(boot::STATUS_OK, 0xFFC8), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 0x0009), Mismatch);
    // No bit above the read's two bytes, whatever the mask fixes.
    assert_eq!(c.judge(boot::STATUS_OK, 0x1_0008), Mismatch);
    // A configuration reads nothing, so its value is 0 (the reduction pass's guard).
    c.step = aethelgard_ir::parts::StepKind::Configure;
    c.width = 0;
    c.mask = 0;
    c.expected = 0;
    assert_eq!(c.judge(boot::STATUS_OK, 0), Pass);
    assert_eq!(c.judge(boot::STATUS_OK, 1), Mismatch);
}

/// `AET-WO-4210` S2: a scripted part is one record per step, in its
/// script's order, each record as wide as its read; a part with no script is
/// its identity alone, so a board without one keeps its layout.
#[test]
fn a_scripted_device_is_one_record_per_step() {
    let words: Vec<u32> = [0u8, 1, 4, 5, 6, 8].map(boot::value_words).to_vec();
    assert_eq!(words, [1, 1, 1, 2, 2, 2]);
    let plan = boot::identity(&with_devices(&[
        ("U2", "BME280", 0x76),
        ("U6", "ICP10111", 0x63),
    ]))
    .unwrap();
    let steps: Vec<(&str, &str, u32)> = plan
        .checks
        .iter()
        .map(|c| {
            (
                c.reference.as_str(),
                c.step.name(),
                c.status_at - plan.checks[0].status_at,
            )
        })
        .collect();
    assert_eq!(
        steps,
        [
            ("U2", "identity", 0),
            ("U6", "identity", 8),
            ("U6", "configure", 16),
            ("U6", "data", 24),
        ]
    );
    let data = &plan.checks[3];
    assert_eq!(data.level, (0, "[DEBUG] "));
    assert_eq!(data.stimulus, Some(vec![("DefaultTemperature", 25)]));
    assert_eq!(
        data.witness.as_deref().unwrap()[1],
        "[DEBUG] i2c1.u6: Reading data with CRC: [0x66, 0x66], current command: 0x7866 (UltraLowNoiseTemperaturePressure), requested: 2 bytes"
    );
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
    let lines = |register: u8, width: u8, value: u64| {
        crate::model::Model::Bme280
            .witness(
                "i2c1.u9",
                &aethelgard_ir::parts::Step {
                    kind: aethelgard_ir::parts::StepKind::Identity,
                    label: "identity",
                    write: vec![register],
                    width,
                    expected: value,
                    mask: u64::MAX,
                    settle_us: 0,
                },
            )
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

/// `AET-WO-4220` D6: a platform may load a model of our own only through a
/// header line naming its pinned file and sha256; anything else is
/// `ModelUnpinned`, by name.
#[test]
fn a_model_of_our_own_loads_only_by_its_pinned_header_line() {
    use crate::model::{Model, OWN, OWN_DIR};
    use crate::platform::{OWN_LINE, own_models};
    let line =
        |file: &str, sha: &str| format!("// - {OWN_LINE}`{OWN_DIR}/{file}` sha256 `{sha}`.\n");
    let (_, icm_file, icm_sha) = OWN.iter().find(|o| o.0 == Model::Icm42688p).unwrap();
    let (_, bmp_file, bmp_sha) = OWN.iter().find(|o| o.0 == Model::Bmp581).unwrap();
    let head = "// @generated by aethelgard-twin (AET-WO-2100).\n";
    let body = "using \"platforms/cpus/stm32h753.repl\"\n\nu5: Sensors.ICM42688P @ spi1mux 0\nu8: Sensors.BMP581 @ i2c1 0x46\n";
    let good = format!(
        "{head}{}{}{body}",
        line(bmp_file, bmp_sha),
        line(icm_file, icm_sha)
    );
    assert_eq!(
        own_models(&good).unwrap(),
        [Model::Bmp581, Model::Icm42688p]
    );
    // A platform of Renode's models names none and needs none.
    assert_eq!(
        own_models(&format!(
            "{head}using \"x\"\n\nu2: I2C.BME280 @ i2c1 0x76\n"
        ))
        .unwrap(),
        []
    );
    let refused = |text: String, why: &str| {
        let err = own_models(&text).unwrap_err();
        assert_eq!(err.kind(), "ModelUnpinned", "{err}");
        assert!(err.to_string().contains(why), "`{why}` not in {err}");
    };
    let other = "1".repeat(64);
    refused(
        format!(
            "{head}{}{}{body}",
            line(bmp_file, &other),
            line(icm_file, icm_sha)
        ),
        "but the twin pins it at",
    );
    refused(
        format!(
            "{head}{}{}{body}",
            line("Other.cs", bmp_sha),
            line(icm_file, icm_sha)
        ),
        "no model of our own the twin pins",
    );
    refused(
        format!(
            "{head}{}{}{body}",
            line(icm_file, icm_sha),
            line(icm_file, icm_sha)
        ),
        "twice",
    );
    // The BMP581 declared, its header line missing.
    refused(
        format!("{head}{}{body}", line(icm_file, icm_sha)),
        "`Sensors.BMP581`, but the header names no pinned copy",
    );
    refused(
        format!("{head}// - {OWN_LINE}{icm_file}\n{body}"),
        "is not `",
    );
    // A header line after the header (below the first non-comment line) is
    // not the header: the device it would pin is refused, and with no such
    // device it pins nothing (the CLI then includes nothing).
    refused(
        format!("{head}{body}{}", line(bmp_file, bmp_sha)),
        "the header names no pinned copy",
    );
    assert_eq!(
        own_models(&format!("{head}using \"x\"\n{}", line(bmp_file, bmp_sha))).unwrap(),
        []
    );
    // Any spelling Renode resolves to one of our classes needs the header
    // (adversarial F1, F7).
    for device in [
        "u5: Antmicro.Renode.Peripherals.Sensors.ICM42688P @ spi1mux 0",
        "u5:Sensors.ICM42688P @ spi1mux 0",
        "u5:\tICM42688P @ spi1mux 0",
    ] {
        refused(
            format!("{head}{}using \"x\"\n{device}\n", line(bmp_file, bmp_sha)),
            "`Sensors.ICM42688P`, but the header names no pinned copy",
        );
    }
}

/// `AET-WO-4220` D6: the emitter names the pin of each model of our own the
/// platform declares, and only those; a board of Renode's models is
/// unchanged.
#[test]
fn the_emitted_header_names_each_own_model_s_pin() {
    use crate::model::{Model, OWN};
    let text = platform::emit(&with_devices(&[("U8", "BMP581", 0x46)])).unwrap();
    let (_, file, sha) = OWN.iter().find(|o| o.0 == Model::Bmp581).unwrap();
    let pin = format!(
        "// - {}`models/renode/{file}` sha256 `{sha}`.",
        platform::OWN_LINE
    );
    assert_eq!(text.lines().filter(|l| *l == pin).count(), 1, "{text}");
    assert!(text.contains("u8: Sensors.BMP581 @ i2c1 0x46"), "{text}");
    assert!(
        text.contains(
            "// U8: Aethelgard's own `Sensors.BMP581` (`models/renode/BMP581.cs`), at 0x46 on i2c1."
        ),
        "{text}"
    );
    assert_eq!(platform::own_models(&text).unwrap(), [Model::Bmp581]);
    let renode = platform::emit(&with_devices(&[("U2", "BME280", 0x76)])).unwrap();
    assert!(!renode.contains(platform::OWN_LINE), "{renode}");
    assert!(platform::own_models(&renode).unwrap().is_empty());
}

/// `AET-WO-4230` D10, §1.6: the platform presents the declared level in
/// `FLASH_OPTSR_CUR` by the block measured in FEEDBACK §2 m1, keys first and
/// a wrong key last, for every level, 0 included; the addresses and the
/// field from the header.
#[test]
fn the_platform_presents_the_declared_brown_out_level() {
    let b = boot::brown_out(&reference()).unwrap();
    assert_eq!(
        (b.level, b.word_at, b.mask, b.shift, b.base),
        (3, boot::boot_word().unwrap() + 4, 0xC, 2, 0x5200_2000)
    );
    assert_eq!(
        (b.optkeyr, b.optcr, b.optsr_cur, b.optsr_prg, b.optstart),
        (0x08, 0x18, 0x1C, 0x20, 0x2)
    );
    assert_eq!(b.presented(), 0x0406_AAFC);
    assert_eq!(
        b.presentation(),
        "\n// The brown-out level the specification declares (mcu.bor_level 3), presented in\n\
         // FLASH_OPTSR_CUR as a factory step leaves it (AET-WO-4230 §1.6): the option keys, the\n\
         // word into FLASH_OPTSR_PRG, OPTSTART, then a wrong key to lock FLASH_OPTCR again.\n\
         flashController:\n    init add:\n\
         \x20       WriteDoubleWord 0x08 0x08192A3B\n\
         \x20       WriteDoubleWord 0x08 0x4C5D6E7F\n\
         \x20       WriteDoubleWord 0x20 0x0406AAFC\n\
         \x20       WriteDoubleWord 0x18 0x00000002\n\
         \x20       WriteDoubleWord 0x08 0x00000000\n"
    );
    for (level, word) in [(0u8, 0x0406_AAF0u32), (1, 0x0406_AAF4), (2, 0x0406_AAF8)] {
        let mut t = reference();
        t.bor_level = level;
        let b = boot::brown_out(&t).unwrap();
        assert!(
            b.presentation()
                .contains(&format!("WriteDoubleWord 0x20 0x{word:08X}\n")),
            "{level}"
        );
        assert!(platform::emit(&t).unwrap().ends_with(&b.presentation()));
    }
    let mut t = reference();
    t.bor_level = 4;
    assert!(boot::brown_out(&t).is_err());
}

/// `AET-WO-4230` D10: word 1's `BOR_LEV` against the declared level; the
/// judge's fill (whose bits 3:2 read 3) is named before it is compared.
#[test]
fn the_brown_out_word_is_judged_against_the_declared_level() {
    use boot::BrownOutOutcome::*;
    let b = boot::brown_out(&reference()).unwrap();
    assert_eq!(b.judge(0x0406_AAFC), Pass);
    assert_eq!(b.judge(0x0000_000C), Pass);
    assert_eq!(b.judge(0x0406_AAF0), Mismatch { recorded: 0 });
    assert_eq!(b.judge(0x0406_AAF8), Mismatch { recorded: 2 });
    assert_eq!(b.judge(boot::RECORD_UNSET), Unset);
    let mut t = reference();
    t.bor_level = 0;
    let z = boot::brown_out(&t).unwrap();
    assert_eq!(z.judge(0x0406_AAF0), Pass);
    assert_eq!(z.judge(0x0406_AAFC), Mismatch { recorded: 3 });
    assert_eq!(z.judge(boot::RECORD_UNSET), Unset);
}

/// `AET-WO-4230` D10, WO §5.3: from the flash controller's access log, one
/// word read of `OPTSR_CUR` in the judged boot, at most one in an earlier
/// one, and no other access at all.
#[test]
fn the_flash_controller_is_read_once_and_never_written() {
    let b = boot::brown_out(&reference()).unwrap();
    let a = |write: bool, bits: u8, offset: u32, value: u64| crate::serial::Access {
        source: boot::FLASH_RENODE.to_owned(),
        write,
        bits,
        offset,
        value,
        refused: false,
    };
    let read = a(false, 32, 0x1C, 0x0406_AAFC);
    assert!(b.judge_accesses(&[vec![read.clone()]]).is_empty());
    // A droop's earlier boot cut before the read.
    assert!(b.judge_accesses(&[vec![], vec![read.clone()]]).is_empty());
    assert_eq!(
        b.judge_accesses(&[vec![]]),
        ["the firmware read FLASH_OPTSR_CUR 0 times in the boot the reads judge, not once"]
    );
    assert_eq!(
        b.judge_accesses(&[vec![read.clone(), read.clone()]]),
        ["the firmware read FLASH_OPTSR_CUR 2 times in the boot the reads judge, not once"]
    );
    assert_eq!(
        b.judge_accesses(&[vec![read.clone(), read.clone()], vec![read.clone()]]),
        ["the firmware read FLASH_OPTSR_CUR 2 times in boot 0, not once"]
    );
    // The m1 sequence, run by a firmware: every write named.
    let program = [
        a(true, 32, 0x08, 0x0819_2A3B),
        a(true, 32, 0x08, 0x4C5D_6E7F),
        a(true, 32, 0x20, 0x0406_AAFC),
        a(true, 32, 0x18, 0x2),
        read.clone(),
    ];
    assert_eq!(
        b.judge_accesses(&[program.to_vec()]),
        [
            "the firmware wrote the flash controller (32 bits at 0x8, value 0x8192A3B)",
            "the firmware wrote the flash controller (32 bits at 0x8, value 0x4C5D6E7F)",
            "the firmware wrote the flash controller (32 bits at 0x20, value 0x406AAFC)",
            "the firmware wrote the flash controller (32 bits at 0x18, value 0x2)"
        ]
    );
    // A byte of OPTSR_CUR, or another register, is not the one word read.
    assert_eq!(
        b.judge_accesses(&[vec![a(false, 8, 0x1C, 0xFC), read.clone()]]),
        [
            "the firmware read the flash controller other than FLASH_OPTSR_CUR whole (8 bits at 0x1C)"
        ]
    );
    assert_eq!(
        b.judge_accesses(&[vec![a(false, 32, 0x20, 0), read]]),
        [
            "the firmware read the flash controller other than FLASH_OPTSR_CUR whole (32 bits at 0x20)"
        ]
    );
}
