//! `verify --against-fab` (`AET-WO-3300` §1.3, split by ruling D5 on PR #103):
//! one `--fab-out` package of the reference with the 2.2 kΩ scenario, written
//! once and read by two tests (`AET-WO-1909` A, ruled on PR #115). Each test
//! compares its own copy, so one test's changed bytes never reach the other.
//! The two comparisons run one at a time: each `verify` peaks at about 3.5 GB.

#![cfg(feature = "with-tools")]

mod common;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::verify::{SPEC, Shared, aethelgard, copy_tree, one, package_files, scratch};
use serde_json::Value;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The package `--fab-out` wrote: its directory, the verdict and its files.
struct Package {
    dir: PathBuf,
    verdict: Value,
    files: BTreeMap<String, Vec<u8>>,
}

static PACKAGE: Shared<Package> = Shared::new("verify 2k2 --fab-out, the package to compare");

/// `verify` of the reference with the 2.2 kΩ scenario and `flag <dir>`.
fn fab_verify(dir: &Path, flag: &str) -> (Option<i32>, String) {
    aethelgard(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        flag,
        dir.to_str().unwrap(),
    ])
}

/// A copy of the shared package under `label`, for one test to compare.
fn package_copy(property: &str, label: &str) -> (&'static Package, PathBuf) {
    let p = PACKAGE.get(property, || {
        let dir = scratch("against-fab").join("pkg");
        let (code, out) = fab_verify(&dir, "--fab-out");
        let verdict = one(&out);
        assert_eq!(
            code,
            Some(0),
            "--fab-out writes the package to compare: {verdict}"
        );
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

/// §1.3: `--against-fab` on a package `--fab-out` wrote derives it again
/// and finds it identical, naming the same files.
#[test]
fn verify_against_fab_finds_the_package_it_wrote_identical() {
    let (p, dir) = package_copy(
        "--against-fab finds the package it wrote identical",
        "against-same",
    );
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, out) = fab_verify(&dir, "--against-fab");
    let w = one(&out);
    assert_eq!(
        code,
        Some(0),
        "--against-fab on the written package passes: {w}"
    );
    assert_eq!(
        w["against_fab"], "identical",
        "the derived package is identical: {w}"
    );
    assert_eq!(
        w["fab"]["status"], "derived",
        "the package is derived, not written: {w}"
    );
    assert_eq!(
        w["fab"]["files"], p.verdict["fab"]["files"],
        "the derived package names the files --fab-out wrote: {w}"
    );
    assert!(
        w["fab"].get("differences").is_none(),
        "an identical package lists no differences: {w}"
    );
}

/// §1.3 and §3: one changed byte in every file of a package, and one extra
/// file, fail `--against-fab` with exit 1, naming each.
#[test]
fn verify_against_fab_names_every_changed_and_extra_file() {
    let (p, a) = package_copy(
        "--against-fab names every changed and extra file",
        "against-changed",
    );
    for (name, bytes) in &p.files {
        let mut changed = bytes.clone();
        changed[bytes.len() / 2] ^= 0x01;
        fs::write(a.join(name), changed).unwrap();
    }
    fs::write(a.join("extra.gbr"), "").unwrap();
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, out) = fab_verify(&a, "--against-fab");
    let w = one(&out);
    assert_eq!(code, Some(1), "a changed package fails --against-fab: {w}");
    assert_eq!(w["verdict"], "fail", "the verdict is fail: {w}");
    assert_eq!(
        w["against_fab"], "differs",
        "the derived package differs: {w}"
    );
    let mut found: Vec<(String, String)> = w["fab"]["differences"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["kind"].as_str().unwrap().to_owned(),
                d["message"]
                    .as_str()
                    .unwrap()
                    .split(' ')
                    .next()
                    .unwrap()
                    .to_owned(),
            )
        })
        .collect();
    found.sort();
    let mut want: Vec<(String, String)> = p
        .files
        .keys()
        .map(|n| ("against_differs".to_owned(), n.clone()))
        .collect();
    want.push(("against_extra".to_owned(), "extra.gbr".to_owned()));
    want.sort();
    assert_eq!(
        found, want,
        "every changed file is against_differs and the extra one against_extra, each named"
    );
    assert_eq!(
        found.len(),
        16,
        "fifteen changed files and one extra are sixteen differences"
    );
}
