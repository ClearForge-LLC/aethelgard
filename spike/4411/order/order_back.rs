//! AET-WO-4411 harness (from 4300j's) (never committed into a crate):
//! `order_2203`'s four texts at specification 31, each with the
//! specification-30 document's sha256 put back for the new one, hashed;
//! printed against `BEFORE`. The specification-30 document is 1b110f35's
//! `examples/sentinel-01-minimal.json`, read from `AET_V30`, with
//! the same device edit. Dropped into `crates/aethelgard-svd-hal/tests/` for
//! the run, then removed.
use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use aethelgard_ir::Specification;
use aethelgard_svd_hal::board::generate_with;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

fn hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
fn back() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let mut doc: serde_json::Value = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let dev = |r: &str, p: &str, a: u8| serde_json::json!({"reference": r, "part": p, "supply": "VDD_3V3", "bus": "SENSORS", "address": a, "pin_capacitance": null});
    doc["i2c_devices"] = serde_json::json!([dev("U10", "BME280", 0x76), dev("U3", "TMP117", 0x48), dev("U2", "BME280", 0x77)]);
    let bytes = serde_json::to_vec(&doc).unwrap();
    let mut old: serde_json::Value = serde_json::from_slice(&fs::read(std::env::var("AET_V30").unwrap()).unwrap()).unwrap();
    old["i2c_devices"] = doc["i2c_devices"].clone();
    let old_sha = hex(&serde_json::to_vec(&old).unwrap());
    let new_sha = hex(&bytes);
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let table = pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &new_sha).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files = generate_with("order-board", &solved, &table, &answers).unwrap();
    let off = BTreeSet::from(["SENSORS".to_owned()]);
    let texts = [
        ("firmware/src/main.rs", files["firmware/src/main.rs"].clone()),
        ("platform", aethelgard_twin::platform::emit(&solved).unwrap()),
        ("platform faulted", aethelgard_twin::platform::emit_faulted(&solved, &off, &BTreeSet::new()).unwrap()),
        ("boot image", format!("{:?}", aethelgard_twin::boot::expected(&solved).unwrap())),
    ];
    println!("new sha {new_sha} old sha {old_sha}");
    for (name, t) in texts {
        println!("{name}\tnow {}\tback {}\tsha lines {}", hex(t.as_bytes()), hex(t.replace(&new_sha, &old_sha).as_bytes()), t.matches(&new_sha).count());
    }
}
