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
    let text = platform::emit(&reference()).unwrap();
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
    let names: Vec<(String, bool)> = platform::instances(&reference())
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
        platform::emit(&reference()).unwrap(),
        platform::emit(&reference()).unwrap()
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
        boot::expected(&edited("\"PA0\"", "\"PA16\""))
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
    let image = boot::expected(&reference()).unwrap();
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
    let image = boot::expected(&edited("\"PB7\"", "\"PB9\"")).unwrap();
    assert!(
        image
            .registers
            .contains(&reg("GPIOB.AFRH", 0x5802_0424, 0x0000_00F0, 0x0000_0040)),
        "{:?}",
        image.registers
    );
}
