//! `AET-WO-4220`: what the generators emit for a board with the parts of our
//! own, read without any tool: the firmware's cited waits, the platform's
//! pins, and the transfer proofs' bounds.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The fixture's solved topology and its generated board crate's files.
fn board(
    spec: &str,
) -> (
    aethelgard_ir::SolvedTopology,
    std::collections::BTreeMap<String, String>,
) {
    let bytes = fs::read(root().join(spec)).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    spec.validate().unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files = aethelgard_svd_hal::board::generate_with("own-board", &solved, &table, &answers)
        .unwrap()
        .into_iter()
        .collect();
    (solved, files)
}

const OWN: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json";

/// The firmware waits each part's cited time after its last configuration
/// write and before its data read, sized at 64 cycles a microsecond: the
/// BMP581's 10 ms after its one write, the ICM-42688-P's 20 ms after its
/// third. A board with no wait in its scripts is unchanged.
#[test]
fn the_firmware_waits_each_cited_time_before_the_data_read() {
    let (_, files) = board(OWN);
    let main = &files["firmware/src/main.rs"];
    assert_eq!(main.matches("fn settle(spins: u32)").count(), 1, "{main}");
    assert_eq!(main.matches("settle(640000);").count(), 1, "{main}");
    assert_eq!(main.matches("settle(1280000);").count(), 1, "{main}");
    let at = |what: &str| main.find(what).unwrap_or_else(|| panic!("{what}"));
    // The BMP581's configuration, its wait, then its data read.
    assert!(at("&[0x36, 0x40, 0x81]") < at("settle(640000);"));
    assert!(at("settle(640000);") < at("&[0x1d]"));
    // The ICM-42688-P's enable is its last write, then its wait, then its read.
    assert!(at("&[0x4c, 0xc0]") < at("&[0x50, 0x66]"));
    assert!(at("&[0x50, 0x66]") < at("&[0x4e, 0x03]"));
    assert!(at("&[0x4e, 0x03]") < at("settle(1280000);"));
    assert!(at("settle(1280000);") < at("&[0x9d]"));
    for spec in [
        "examples/sentinel-01-minimal.json",
        "crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json",
    ] {
        let (_, files) = board(spec);
        assert!(!files["firmware/src/main.rs"].contains("settle"), "{spec}");
    }
}

/// The emitted platform names both models' pins, and its devices are
/// declared as the models of our own; its proofs bound the IMU's bus by the
/// six-byte data read (`SPI_MAX_FRAMES` is 8: one byte out, six in).
#[test]
fn the_platform_pins_both_models_and_the_proofs_bound_the_data_read() {
    let (solved, files) = board(OWN);
    let text = aethelgard_twin::platform::emit(&solved).unwrap();
    assert_eq!(
        aethelgard_twin::platform::own_models(&text).unwrap(),
        [
            aethelgard_twin::model::Model::Bmp581,
            aethelgard_twin::model::Model::Icm42688p
        ]
    );
    assert!(text.contains("u7: Sensors.BMP581 @ i2c1 0x46"), "{text}");
    assert!(text.contains("u5: Sensors.ICM42688P @ "), "{text}");
    assert!(!text.contains("LIFECYCLE"), "{text}");
    assert!(
        files["src/proofs.rs"].contains("kani::assume(wl <= 2 && rl <= 6);"),
        "{}",
        files["src/proofs.rs"]
    );
}
