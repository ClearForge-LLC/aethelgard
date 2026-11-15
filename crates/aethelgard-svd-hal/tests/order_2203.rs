//! `AET-WO-2203` §1.4, the part the regeneration guard cannot see: the
//! committed board's devices (`U2`, `U3`) are already in reference order, so a
//! change that sorted devices by reference would pass it. This specification
//! lists `U10`, `U3`, `U2`, which sorts differently, and pins the sha256 of
//! every output that depends on device order. The digests were taken from the
//! code before the devices moved into `SolvedTopology` (`i2c_targets`).

use aethelgard_ir::Specification;
use aethelgard_svd_hal::board::generate_with;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

fn hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn digests() -> Vec<(&'static str, String)> {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let dev = |r: &str, p: &str, a: u8| serde_json::json!({"reference": r, "part": p, "supply": "VDD_3V3", "bus": "SENSORS", "address": a});
    doc["i2c_devices"] = serde_json::json!([
        dev("U10", "BME280", 0x76),
        dev("U3", "TMP117", 0x48),
        dev("U2", "BME280", 0x77)
    ]);
    let bytes = serde_json::to_vec(&doc).unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &hex(&bytes)).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files = generate_with("order-board", &solved, &table, &answers).unwrap();
    let off = BTreeSet::from(["SENSORS".to_owned()]);
    vec![
        (
            "firmware/src/main.rs",
            hex(files["firmware/src/main.rs"].as_bytes()),
        ),
        (
            "platform",
            hex(aethelgard_twin::platform::emit(&solved).unwrap().as_bytes()),
        ),
        (
            "platform faulted",
            hex(
                aethelgard_twin::platform::emit_faulted(&solved, &off, &BTreeSet::new())
                    .unwrap()
                    .as_bytes(),
            ),
        ),
        (
            "boot image",
            hex(format!("{:?}", aethelgard_twin::boot::expected(&solved).unwrap()).as_bytes()),
        ),
    ]
}

/// Taken from the code before the move (`i2c_targets` in the caller). The
/// "platform faulted" digest was re-taken in `AET-WO-1900` (2300 F13), which
/// reworded the faulted platform's provenance comment and nothing else: the old
/// digest was `35509f30…1073`. The "boot image" digest was re-taken in
/// `AET-WO-2204`, which replaced the bus's `CR2`/`TXDR` witness with each
/// identity read's device path and witness lines (the `Image` the digest
/// prints); the other three are unchanged. The old digest was `704f44d1…8e7c`.
/// The "firmware/src/main.rs" digest was re-taken in `AET-WO-4200`, whose
/// firmware hands the flash's chip select to `SPI1` and reads the flash's
/// identity after the I2C reads (record 2 here); the I2C reads' text and order
/// are unchanged. The old digest was `bd4e06af…c5f1`. The "platform", "platform
/// faulted" and "boot image" digests were re-taken in `AET-WO-4200` too: the
/// platform gains SPI1's multiplexer and the flash after the I2C devices, whose
/// lines and order are unchanged, and the `Image` gains U4's SPI check. The old
/// digests were `0b39d4aa…56d2`, `2ccc0e8d…c75b` and `06c8463c…5f5e`. The
/// firmware and both platform digests were re-taken again at the merge of
/// `AET-WO-4100`, whose specification version 2 moved them on `main` (to
/// `7079aa52…`, `b777f4e5…` and `82de0037…` there). They were re-taken once more
/// in `AET-WO-4300c` (specification version 3, so the specification's sha256;
/// the flash is `W25Q64JVxIQ`; and the firmware's SPI output speeds): the
/// firmware was `c468b3cb…604f`, the platform `9fd77dad…53aa` and the
/// faulted platform `c80833a9…8bec`. The boot image's digest is unchanged.
/// Re-taken in `AET-WO-4300f` (specification version 4, `connectors`), which
/// moved only the specification's sha256: with the old hash put back, each
/// output's digest is the old one exactly. The three were `ecd58f5f…9b7b`,
/// `bc1517cd…7522` and `f684f871…3dba`.
/// Re-taken in `AET-WO-1308` (version 5) the same way: with `main`'s hash
/// passed to `solve`, each was `f3e10417…2985`, `9c02c1e2…12bf4` and
/// `cc3a3e65…2dff9` exactly.
const BEFORE: [(&str, &str); 4] = [
    (
        "firmware/src/main.rs",
        "44f22188f559ecb595304ba8fa905876c7ff1ed563b4a5388f059eeb7797e3af",
    ),
    (
        "platform",
        "bd957c1dea50e5f2261a797fbeeb9d7fecaa4f68af13799482810e8d8c93d4b0",
    ),
    (
        "platform faulted",
        "b962492fa26128b8d84385623e0fb5f6d9e04292dabc524c66c6f8d8e61aee15",
    ),
    (
        "boot image",
        "747bdc045a0c2a41b8e93922bb6e463d82b577a9e061905a708c4893228efbb3",
    ),
];

#[test]
fn out_of_order_devices_keep_the_specification_order() {
    let now = digests();
    assert_eq!(now.len(), BEFORE.len());
    for ((name, digest), (want_name, want)) in now.iter().zip(BEFORE) {
        assert_eq!(*name, want_name);
        assert_eq!(digest, want, "{name} changed");
    }
}
