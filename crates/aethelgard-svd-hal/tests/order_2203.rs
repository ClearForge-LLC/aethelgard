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
    let answers = |part: &str| aethelgard_twin::model::model(part).is_some();
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
            hex(aethelgard_twin::platform::emit_faulted(&solved, &off)
                .unwrap()
                .as_bytes()),
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
/// digest was `35509f30…1073`.
const BEFORE: [(&str, &str); 4] = [
    (
        "firmware/src/main.rs",
        "bd4e06afea31d18fd46e29503fd0076ea679cc246ae5aea636ff8755266dc5f1",
    ),
    (
        "platform",
        "0b39d4aa8dcb25de02663eb4364e195b4a0de2efb736ebb92b84f65e9c0e56d2",
    ),
    (
        "platform faulted",
        "2ccc0e8d5a63073ebf96e75f7a7955887b03a123fc0e05757ba178b48af6c75b",
    ),
    (
        "boot image",
        "704f44d1fd5c60f828991d81aed84c16f7e687af04d86ee378b93b5d04c78e7c",
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
