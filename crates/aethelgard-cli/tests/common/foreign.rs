//! The reference board as a foreign firmware's (`AET-WO-4400g` §1.2): its
//! specification with `mcu.foreign_clock` declared, and the platform emitted
//! for it, which `simulate --foreign` requires. The clock is the reset
//! clock's figures, 64 MHz timers and USARTs, which the RCC registers at
//! reset decode to ([`RCC_AT_RESET`]), so the timer check holds.

use std::fs;
use std::path::{Path, PathBuf};

/// The six RCC registers the foreign judge reads back, at their reset values
/// (RM0433 Rev 9 §8.7.2, §8.7.7–§8.7.9, §8.7.11, §8.7.13), in the order of
/// `aethelgard_ir::motor::RccRegisters::OFFSETS`, as a stub prints them.
pub const RCC_AT_RESET: [&str; 6] = [
    "0x00000001",
    "0x00000000",
    "0x00000000",
    "0x00000000",
    "0x02020200",
    "0x01010280",
];

/// `examples/sentinel-01-minimal.json` with every foreign clock at 64 MHz.
pub fn spec_json() -> serde_json::Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let mut v: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let hz = |source: &str| serde_json::json!({"value": 64_000_000, "source": source});
    let usarts: serde_json::Map<String, serde_json::Value> = aethelgard_ir::serial::PERIPHERALS
        .iter()
        .map(|p| ((*p).to_owned(), hz("the reset clock's rcc_pclk (RM0433 Rev 9 §8.7.21)")))
        .collect();
    v["mcu"]["foreign_clock"] = serde_json::json!({
        "apb1_timers_hz": hz("the reset clock (RM0433 Rev 9 Table 57)"),
        "apb2_timers_hz": hz("the reset clock (RM0433 Rev 9 Table 57)"),
        "usarts": usarts,
    });
    v
}

/// Writes [`spec_json`] and its emitted platform into `dir`, and returns
/// their paths: (specification, platform).
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
    fs::write(&platform, aethelgard_twin::platform::emit(&topology).unwrap()).unwrap();
    (spec_path, platform)
}
