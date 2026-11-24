//! The 4-layer fixture's package (`AET-WO-4100` §2.6, §4 and §5.4): one
//! `--fab-out` package of `four-layer.json` with its seeded set, written
//! once and read by two tests (`AET-WO-1910` D3, ruled on PR #233, comment
//! 6999515740; `AET-WO-1909` A's pattern). Both tests were in
//! `board_tier2.rs` and each wrote the same package, the same invocation
//! with only its directory differing. Each test compares its own copy, so one
//! test's changed bytes never reach the other. The two comparisons run one
//! at a time: each `verify` peaks at about 3.5 GB.

#![cfg(feature = "with-tools")]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::verify::{Shared, aethelgard, copy_tree, one, package_files, scratch};
use serde_json::Value;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

const FOUR: &str = "crates/aethelgard-cli/tests/fixtures/four-layer.json";
/// The 4-layer fixture's seeded set (`AET-WO-3303` D1, ruled): a tolerated
/// pull-up and droop, the absent flash detected, and the 2.2 kΩ baseline.
const FOUR_SCENARIOS: [&str; 4] = [
    "crates/aethelgard-cli/tests/fixtures/four-layer-3k3.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-droop.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-spi-absent.json",
    "crates/aethelgard-cli/tests/fixtures/four-layer-pullup.json",
];

/// `verify` of the 4-layer fixture with its seeded set and `flag <dir>`.
fn four_verify(dir: &Path, flag: &str) -> (Option<i32>, String) {
    let mut args = vec!["verify", "--spec", FOUR, "--scenarios"];
    args.extend(FOUR_SCENARIOS);
    args.extend([flag, dir.to_str().unwrap()]);
    aethelgard(&args)
}

/// The package `--fab-out` wrote: its directory, the verdict and its files.
struct Package {
    dir: PathBuf,
    verdict: Value,
    files: BTreeMap<String, Vec<u8>>,
}

static PACKAGE: Shared<Package> =
    Shared::new("verify four-layer --fab-out, the package to compare");

/// A copy of the shared package under `label`, for one test to compare.
fn package_copy(property: &str, label: &str) -> (&'static Package, PathBuf) {
    let p = PACKAGE.get(property, || {
        let dir = scratch("four-layer-fab").join("pkg");
        let (code, out) = four_verify(&dir, "--fab-out");
        let verdict = one(&out);
        assert_eq!(
            code,
            Some(0),
            "--fab-out writes the 4-layer package to compare: {verdict}"
        );
        // The four-layer board's verdict carries the table (`AET-WO-1609`).
        common::verify::ran(&verdict);
        let files = package_files(&dir);
        Package {
            dir,
            verdict,
            files,
        }
    });
    let copy = scratch(label).join("pkg");
    copy_tree(&p.dir, &copy);
    (p, copy)
}

/// `AET-WO-4100` §2.6 and §4: the 4-layer fixture (OSH Park's stackup, ruled
/// D2) passes every gate, DRC with schematic parity among them, by the
/// specification's rules; `--fab-out` writes both inner Gerbers, each
/// carrying its plane's region and no clock line; `FAB-NOTES.md` states the
/// layer order, each plane's net and every dielectric with its source; and
/// `--against-fab` derives the package again identical.
#[test]
fn four_layer_fixture_passes_every_gate_and_its_package_holds() {
    let (p, dir) = package_copy(
        "the 4-layer fixture passes every gate and its package holds",
        "four-same",
    );
    let v = &p.verdict;
    assert_eq!(v["verdict"], "pass", "the 4-layer verdict is pass: {v}");
    for g in v["gates"].as_array().unwrap() {
        assert_eq!(g["verdict"], "pass", "every gate passes: {g}");
    }
    let package = &p.files;
    for (name, net) in [
        ("sentinel-01-4l-In1_Cu.g1", "GND"),
        ("sentinel-01-4l-In2_Cu.g2", "VDD_3V3"),
    ] {
        let text = String::from_utf8(package[name].clone()).unwrap();
        assert!(
            text.contains(&format!("%TO.N,{net}*%\nG36*\n")),
            "{name} carries no {net} region"
        );
        assert!(
            !text.contains("CreationDate") && !text.contains("Created by"),
            "{name} carries no clock line"
        );
    }
    assert!(
        package.keys().all(|n| !n.contains("gbrjob")),
        "the package holds no Gerber job file"
    );
    let notes = String::from_utf8(package["FAB-NOTES.md"].clone()).unwrap();
    for want in [
        "- 4 copper layers, top to bottom, joined only by through vias:\n",
        "  - In1.Cu: plane, net GND\n",
        "  - In2.Cu: plane, net VDD_3V3\n",
        "  - F.Cu to In1.Cu: 0.179 mm (OSH Park 4-layer",
        "  - In1.Cu to In2.Cu: 0.891 mm (OSH Park 4-layer",
        "rounded down; every maximum, rounded up.",
        "Finished board from 1.25 mm to 1.851 mm thick.",
        "Finished copper at most 70 um thick on each layer.",
        "- `sentinel-01-4l-In1_Cu.g1`: Gerber, inner copper layer 1",
    ] {
        assert!(notes.contains(want), "FAB-NOTES.md states {want}");
    }
    let manifest: Value = serde_json::from_slice(&package["MANIFEST.json"]).unwrap();
    assert!(
        manifest["files"].get("sentinel-01-4l-In2_Cu.g2").is_some(),
        "the manifest names the inner Gerber"
    );
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, again) = four_verify(&dir, "--against-fab");
    let w = one(&again);
    assert_eq!(
        code,
        Some(0),
        "--against-fab on the written package passes: {w}"
    );
    assert_eq!(
        w["against_fab"], "identical",
        "the derived package is identical: {w}"
    );
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}

/// `AET-WO-4100` §5.4: a package missing an inner Gerber, and one whose
/// `FAB-NOTES.md` states a stackup the specification does not, fail
/// `--against-fab`, naming each file.
#[test]
fn against_fab_names_a_missing_inner_layer_and_a_stackup_that_lies() {
    let (_, dir) = package_copy(
        "--against-fab names a missing inner layer and a stackup that lies",
        "four-lie",
    );
    fs::remove_file(dir.join("sentinel-01-4l-In2_Cu.g2")).unwrap();
    let notes = fs::read_to_string(dir.join("FAB-NOTES.md")).unwrap();
    let lie = notes.replace("F.Cu to In1.Cu: 0.179 mm", "F.Cu to In1.Cu: 0.25 mm");
    assert_ne!(lie, notes, "the stackup line to change is in FAB-NOTES.md");
    fs::write(dir.join("FAB-NOTES.md"), lie).unwrap();
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, again) = four_verify(&dir, "--against-fab");
    let w = one(&again);
    assert_eq!(
        code,
        Some(1),
        "--against-fab on a changed package fails: {w}"
    );
    assert_eq!(
        w["against_fab"], "differs",
        "the derived package differs: {w}"
    );
    let named: Vec<String> = w["fab"]["differences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["message"].as_str().unwrap().to_owned())
        .collect();
    assert!(
        named
            .iter()
            .any(|m| m.starts_with("sentinel-01-4l-In2_Cu.g2")),
        "the missing inner Gerber is named: {named:?}"
    );
    assert!(
        named.iter().any(|m| m.starts_with("FAB-NOTES.md")),
        "the lying FAB-NOTES.md is named: {named:?}"
    );
    assert_eq!(
        named.len(),
        2,
        "exactly the two changes are named: {named:?}"
    );
    fs::remove_dir_all(dir.parent().unwrap()).unwrap();
}
