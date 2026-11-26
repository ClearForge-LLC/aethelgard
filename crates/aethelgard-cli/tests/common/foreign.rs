//! The reference board as a foreign firmware's (`AET-WO-4400g` §1.2): its
//! specification with `mcu.foreign_clock` declared, and the platform emitted
//! for it, which `simulate --foreign` requires. The clock is ArduPilot's tree
//! with no crystal (`AET-WO-4412` D1 (a): `aethelgard_ir::ardupilot_clock`'s
//! HSI row, 200 MHz timers and 72 MHz USARTs at VOS Scale 1), and the RCC
//! registers a stub prints are those ArduPilot leaves on that tree
//! ([`RCC_ARDUPILOT_HSI`]), so the timer check holds.

use std::fs;
use std::path::{Path, PathBuf};

/// The six RCC registers the foreign judge reads back, as unmodified
/// ArduPilot leaves them on the HSI (measured on `sentinel-02-own`,
/// `AET-WO-4400g`): PLL1 from the HSI at 400 MHz, HCLK /2, so the APB timers
/// at 200 MHz. In the order of `aethelgard_ir::motor::RccRegisters::OFFSETS`,
/// as a stub prints them.
pub const RCC_ARDUPILOT_HSI: [&str; 6] = [
    "0x3F003025",
    "0x0000801B",
    "0x00000048",
    "0x00000440",
    "0x00408040",
    "0x01090231",
];

/// The same six at their reset values (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.9,
/// §8.7.11, §8.7.13) as Renode 1.17.0 reads them (`RCC_CR`'s `HSIRDY`
/// mirrors `HSION` and `HSIDIVF` reads 1): 64 MHz everywhere.
pub const RCC_AT_RESET: [&str; 6] = [
    "0x00000025",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x02020200",
    "0x01010280",
];

/// `examples/sentinel-01-minimal.json` with ArduPilot's HSI tree declared.
pub fn spec_json() -> serde_json::Value {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let tree = aethelgard_ir::ardupilot_clock::tree(0).unwrap();
    let hz = |hz: u64| serde_json::json!({"value": hz, "source": "Copter-4.7.2 stm32h7_mcuconf.h, no crystal (aethelgard-ir data/ardupilot-h7-clock.tsv)"});
    let usarts: serde_json::Map<String, serde_json::Value> = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), hz(tree.usarts_hz)))
        .collect();
    v["mcu"]["foreign_clock"] = serde_json::json!({
        "apb1_timers_hz": hz(tree.timers_hz),
        "apb2_timers_hz": hz(tree.timers_hz),
        "usarts": usarts,
        "vos": {"value": "scale1", "source": "Copter-4.7.2 stm32h7_mcuconf.h STM32_VOS_SCALE1 (L109)"},
    });
    v
}

/// Writes [`spec_json`] and its emitted platform into `dir`, and returns
/// their paths: (specification, platform). The generated run's platform,
/// the clock dropped (`AET-WO-4412` D1 (a)), is written beside them as
/// `generated-platform.repl`.
pub fn write(dir: &Path) -> (PathBuf, PathBuf) {
    let bytes = serde_json::to_vec_pretty(&spec_json()).unwrap();
    let spec_path = dir.join("foreign-spec.json");
    fs::write(&spec_path, &bytes).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let hash: String = <sha2::Sha256 as sha2::Digest>::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let topology = aethelgard_ir::solve(&spec, &table, &hash).unwrap();
    let platform = dir.join("foreign-platform.repl");
    fs::write(
        &platform,
        aethelgard_twin::platform::emit(&topology).unwrap(),
    )
    .unwrap();
    fs::write(
        dir.join("generated-platform.repl"),
        aethelgard_twin::platform::emit(&aethelgard_cli::generate::generated(&topology)).unwrap(),
    )
    .unwrap();
    (spec_path, platform)
}

/// `elf` with a symbol table naming ArduPilot's panic, a function at `at`
/// (Thumb bit set, as the linker writes it), and the section headers that
/// name it: what the foreign judge hooks (`AET-WO-1609`). The `PT_LOAD`
/// file bytes are unchanged, so every flash hash a transcript states holds.
pub fn with_panic(mut b: Vec<u8>, at: u32) -> Vec<u8> {
    let name = b"_ZN6AP_HAL5panicEPKcz";
    let mut strings = vec![0u8];
    strings.extend_from_slice(name);
    strings.push(0);
    let mut table = vec![0u8; 16];
    for v in [1u32, at | 1, 4] {
        table.extend_from_slice(&v.to_le_bytes());
    }
    table.extend_from_slice(&[0x12, 0, 1, 0]);
    let symtab = b.len() as u32;
    b.extend_from_slice(&table);
    let strtab = b.len() as u32;
    b.extend_from_slice(&strings);
    let shoff = b.len() as u32;
    for (kind, offset, size, link) in [
        (0u32, 0u32, 0u32, 0u32),
        (2, symtab, table.len() as u32, 2),
        (3, strtab, strings.len() as u32, 0),
    ] {
        let mut h = [0u8; 40];
        for (i, v) in [(1, kind), (4, offset), (5, size), (6, link)] {
            h[4 * i..4 * i + 4].copy_from_slice(&v.to_le_bytes());
        }
        b.extend_from_slice(&h);
    }
    b[32..36].copy_from_slice(&shoff.to_le_bytes());
    b[46..48].copy_from_slice(&40u16.to_le_bytes());
    b[48..50].copy_from_slice(&3u16.to_le_bytes());
    b
}
