//! `AET-WO-4240e` D4 (ruled): the scar's fixtures. The 26 committed
//! specifications whose firmware, before this work order, switched
//! `SPI123SEL` away from `pll1_q_ck` with PLL1 never run (the measurement,
//! `spike/4240e/RESULTS.md` §3; RM0433 Rev 9 pp. 371–372). Each topology is
//! solved now and then put back as it was, by removing `clock.kernel_switch`:
//! the rule (`aethelgard_ir::kernel_clock::check`) refuses it by name, with
//! every SPI controller the measurement found unclocked, and generation
//! refuses it with the rule's kind; solved as committed, both pass.

use std::fs;
use std::path::Path;

use aethelgard_ir::kernel_clock::{KernelClockRefusal, check};
use aethelgard_ir::{SolvedTopology, Specification};
use aethelgard_svd_hal::board::generate_with;
use aethelgard_svd_hal::pin_table;
use sha2::{Digest, Sha256};

/// Each specification (from the repository root) and the controllers the
/// measurement found starting with no kernel clock (`spike/4240e/out/measure.json`).
/// Sentinel-03 left the list in `AET-WO-4300k`: its crystal runs PLL1, so it
/// has no switch to remove (`sentinel_03_runs_pll1_from_its_crystal_with_no_switch`).
const SCAR: [(&str, &[&str]); 25] = [
    (
        "crates/aethelgard-cli/tests/fixtures/connectors.json",
        &["SPI1", "SPI2"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json",
        &["SPI1", "SPI2", "SPI3"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/four-layer-power.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/four-layer.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/four-port.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/logger-4600.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/motors-dshot600.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/motors-pwm.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/sentinel-01-bench.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json",
        &["SPI1", "SPI2"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/sentinel-02-shaped.json",
        &["SPI1", "SPI2"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/serial-four.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/solve/stress.json",
        &["SPI1", "SPI2", "SPI3"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/spi-only.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-cli/tests/fixtures/two-bme.json",
        &["SPI1"],
    ),
    (
        "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json",
        &["SPI1", "SPI2"],
    ),
    ("examples/sentinel-01-ext-bmp581.json", &["SPI1"]),
    ("examples/sentinel-01-minimal.json", &["SPI1"]),
    ("examples/sentinel-01.json", &["SPI1"]),
    ("examples/sentinel-02-bench.json", &["SPI1"]),
    ("spike/4300j/open/motors-pwm-open.json", &["SPI1"]),
    ("spike/4300j/open/serial-dshot-9600-open.json", &["SPI1"]),
];

fn solved(path: &str) -> (SolvedTopology, aethelgard_ir::PinTable) {
    let bytes = fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(path),
    )
    .unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = pin_table(&spec.mcu.part).unwrap();
    (aethelgard_ir::solve(&spec, &table, &sha).unwrap(), table)
}

fn answers(part: &str) -> bool {
    matches!(aethelgard_twin::model::model(part), Ok(Some(_)))
}

#[test]
fn every_scar_board_is_clocked_as_solved_and_refused_by_name_as_it_was() {
    for (path, unclocked) in SCAR {
        let (t, table) = solved(path);
        assert!(t.clock.kernel_switch.is_some(), "{path}: no switch");
        assert_eq!(check(&t), Vec::new(), "{path} as solved");
        assert!(
            generate_with("scar", &t, &table, &answers).is_ok(),
            "{path} generates"
        );
        let mut before = t.clone();
        before.clock.kernel_switch = None;
        let refused = check(&before);
        let named: Vec<&str> = refused
            .iter()
            .map(|r| match r {
                KernelClockRefusal::KernelClockDeadSwitch {
                    peripheral,
                    selector,
                    from,
                    to,
                    dead,
                    ..
                } => {
                    assert_eq!(
                        (*selector, *from, *to),
                        ("SPI123SEL", "pll1_q_ck", "per_ck"),
                        "{path}"
                    );
                    assert!(dead.iter().any(|d| d.input == "pll1_q_ck"), "{path}: {r}");
                    peripheral.as_str()
                }
                other => panic!("{path}: {other}"),
            })
            .collect();
        assert_eq!(named, unclocked, "{path}");
        let err = generate_with("scar", &before, &table, &answers)
            .err()
            .unwrap();
        assert_eq!(err.kind(), "KernelClockDeadSwitch", "{path}");
        assert!(err.to_string().starts_with(unclocked[0]), "{path}: {err}");
    }
}

/// `AET-WO-4300k`: Sentinel-03 declares its 16 MHz crystal, so PLL1 runs from
/// HSE, its SPI kernels are clocked with no switch, the rule passes and the
/// board generates; the scar it carried (SPI1, SPI2) is gone.
#[test]
fn sentinel_03_runs_pll1_from_its_crystal_with_no_switch() {
    let (t, table) = solved("examples/sentinel-03.json");
    assert!(t.clock.hse.is_some());
    assert!(t.clock.kernel_switch.is_none());
    assert_eq!(check(&t), Vec::new());
    assert!(generate_with("scar", &t, &table, &answers).is_ok());
}
