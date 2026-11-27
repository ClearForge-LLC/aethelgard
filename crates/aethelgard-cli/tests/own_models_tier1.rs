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

/// The firmware makes every identity and configuration step, then waits
/// ONCE, for the largest cited start-up, then makes every data read
/// (`AET-WO-4230` R6, the architect's ruling on Q-E: the parts power up
/// together, so no per-device sum). The wait is sized at HSI's fastest,
/// 64,958,976 cycles a second (§1.9, amendment 1), rounded up: on the own
/// fixture the ICM-42688-P's 20 ms, which covers the BMP581's 10 ms. A board
/// with no script is unchanged.
#[test]
fn the_firmware_waits_once_for_the_largest_cited_time_before_any_data_read() {
    let (_, files) = board(OWN);
    let main = &files["firmware/src/main.rs"];
    assert_eq!(main.matches("fn settle(spins: u32)").count(), 1, "{main}");
    assert_eq!(main.matches("    settle(").count(), 1, "{main}");
    assert_eq!(main.matches("settle(1299180);").count(), 1, "{main}");
    let at = |what: &str| main.find(what).unwrap_or_else(|| panic!("{what}"));
    // The BMP581's configuration and the ICM-42688-P's (its enable last),
    // then the one wait, then both data reads.
    assert!(at("&[0x36, 0x40, 0x81]") < at("&[0x4c, 0xc0]"));
    assert!(at("&[0x4c, 0xc0]") < at("&[0x50, 0x66]"));
    assert!(at("&[0x50, 0x66]") < at("&[0x4e, 0x03]"));
    assert!(at("&[0x4e, 0x03]") < at("settle(1299180);"));
    assert!(at("settle(1299180);") < at("&[0x1d]"));
    assert!(at("&[0x1d]") < at("&[0x9d]"));
    let (_, files) = board("examples/sentinel-01-minimal.json");
    assert!(!files["firmware/src/main.rs"].contains("settle"));
    // The stand-ins wait too (`AET-WO-4230` §1.9, `SettleBelowStartUp`): twice
    // their typical-only start-ups, 40 ms (two ICM-20948) and 340 us: one
    // wait, the larger.
    let (_, files) = board("crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json");
    let main = &files["firmware/src/main.rs"];
    assert_eq!(main.matches("    settle(").count(), 1, "{main}");
    assert_eq!(main.matches("settle(2598360);").count(), 1, "{main}");
}

/// The emitted platform names both models' pins, and its devices are
/// declared as the models of our own; its proofs bound the IMU's bus by the
/// six-byte data read (`SPI_MAX_FRAMES` is 8: one byte out, six in).
#[test]
fn the_platform_pins_both_models_and_the_proofs_bound_the_data_read() {
    let (solved, files) = board(OWN);
    let text = aethelgard_twin::platform::emit(&solved).unwrap();
    assert_eq!(
        aethelgard_twin::platform::own_models(&text)
            .unwrap()
            .iter()
            .map(|o| o.renode)
            .collect::<Vec<_>>(),
        [
            aethelgard_twin::model::Model::Bmp581.renode(),
            aethelgard_twin::model::Model::Icm42688p.renode()
        ]
    );
    assert!(text.contains("u7: Sensors.BMP581 @ i2c1 0x46"), "{text}");
    assert!(text.contains("u5: Sensors.ICM42688P @ "), "{text}");
    // The one lifecycle line is the flash's (`AET-WO-4211`): both parts of
    // our own are `active`.
    let banner: Vec<&str> = text.lines().filter(|l| l.contains("LIFECYCLE")).collect();
    assert_eq!(banner.len(), 1, "{text}");
    assert!(
        banner[0].starts_with("// - LIFECYCLE NRND: W25Q64JVxIQ (U4) is not recommended"),
        "{text}"
    );
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
