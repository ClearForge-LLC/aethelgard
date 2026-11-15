//! `AET-WO-4220`: what the generators emit for a board with the parts of our
//! own, read without any tool: the firmware's cited waits, the platform's
//! pins, and the transfer proofs' bounds.

mod common;

use common::verify::{generated, root};

/// The fixture's solved topology and its generated board crate's files.
fn board(
    spec: &str,
) -> (
    aethelgard_ir::SolvedTopology,
    std::collections::BTreeMap<String, String>,
) {
    generated(&root().join(spec), "own-board")
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

/// The archive names a device's steps as the parts table does, in the same
/// order: chaos restates the configuration names it may not import
/// (`belt-and-suspenders` at `step_index`).
#[test]
fn the_archive_orders_every_step_name_the_parts_table_gives() {
    use aethelgard_chaos::record::step_index;
    use aethelgard_ir::parts::{CONFIGURE_STEPS, StepKind};
    assert_eq!(step_index(StepKind::Identity.name()), Some(0));
    assert_eq!(step_index(StepKind::Configure.name()), Some(1));
    for (k, name) in CONFIGURE_STEPS.iter().enumerate() {
        assert_eq!(step_index(name), Some(k + 1), "{name}");
    }
    assert_eq!(
        step_index(StepKind::Data.name()),
        Some(CONFIGURE_STEPS.len() + 1)
    );
    assert_eq!(step_index("configure 0"), None);
}
