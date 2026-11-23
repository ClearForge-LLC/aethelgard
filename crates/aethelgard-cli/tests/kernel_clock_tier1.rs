//! `AET-WO-4240e` D5: the verdict judges every enabled peripheral's kernel
//! clock again, on its own (`ledger::silicon`, `verify`'s
//! `silicon_rule_fails`), as `generate` refuses it first. The reference
//! board (`examples/sentinel-01.json`, the bench package's) holds as
//! solved; with `clock.kernel_switch` removed, as its firmware was before
//! this work order, SPI1's switch away from a dead `pll1_q_ck` fails it by
//! name (RM0433 Rev 9 pp. 371–372).

use std::fs;
use std::path::Path;

use aethelgard_cli::ledger;
use aethelgard_ir::{SolvedTopology, Specification};
use sha2::{Digest, Sha256};

fn reference() -> SolvedTopology {
    let bytes =
        fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01.json"))
            .unwrap();
    let spec: Specification = serde_json::from_slice(&bytes).unwrap();
    let sha: String = Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    aethelgard_ir::solve(&spec, &table, &sha).unwrap()
}

#[test]
fn the_verdict_holds_the_switch_and_refuses_the_scar_by_name() {
    let t = reference();
    let s = ledger::silicon(&t);
    assert!(s.kernel_clock.is_empty() && s.holds, "{s:?}");
    assert!(ledger::silicon_failures(&s).is_empty());

    let mut before = t;
    before.clock.kernel_switch = None;
    let s = ledger::silicon(&before);
    assert!(!s.holds);
    let failures = ledger::silicon_failures(&s);
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0].starts_with(
            "aethelgard_ir::kernel_clock::check refuses (KernelClockDeadSwitch): SPI1: SPI123SEL"
        ),
        "{}",
        failures[0]
    );
    let json = serde_json::to_value(&s).unwrap();
    assert_eq!(json["kernel_clock"][0]["kind"], "KernelClockDeadSwitch");
    assert_eq!(json["kernel_clock"][0]["peripheral"], "SPI1");
}
