//! `verify --against-fab` (`AET-WO-3300` §1.3, split by ruling D5 on PR #103):
//! one `--fab-out` package of the ready fixture (`AET-WO-4300k` (T), ruled
//! 11:25Z: the reference made ready, `common/ready.rs`) with its seeded set,
//! written once and read by two tests (`AET-WO-1909` A, ruled on PR #115). Each test
//! compares its own copy, so one test's changed bytes never reach the other.
//! The two comparisons run one at a time: each `verify` peaks at about 3.5 GB.

#![cfg(feature = "with-tools")]

mod common;
#[path = "common/ready.rs"]
mod ready;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use common::verify::{Shared, aethelgard, copy_tree, one, package_files, scratch};
use ready::{READY, READY_SCENARIOS};
use serde_json::Value;

static ONE_AT_A_TIME: Mutex<()> = Mutex::new(());

/// The package `--fab-out` wrote: its directory, the verdict and its files.
struct Package {
    dir: PathBuf,
    verdict: Value,
    files: BTreeMap<String, Vec<u8>>,
}

static PACKAGE: Shared<Package> =
    Shared::new("verify --fab-out of the ready fixture, the package to compare");

/// `verify` of the ready fixture with its seeded set and `flag <dir>`.
fn fab_verify(dir: &Path, flag: &str) -> (Option<i32>, String) {
    let mut args = vec!["verify", "--spec", READY, "--scenarios"];
    args.extend(READY_SCENARIOS);
    args.extend([flag, dir.to_str().unwrap()]);
    aethelgard(&args)
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

/// §1.3 and §3: one changed byte in every file of a package, one extra
/// file and one empty directory (`AET-WO-2308` L4), fail `--against-fab`
/// with exit 1, naming each.
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
    fs::create_dir_all(a.join("empty")).unwrap();
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
    want.push(("against_extra".to_owned(), "empty/".to_owned()));
    want.sort();
    assert_eq!(
        found, want,
        "every changed file is against_differs, the extra file and the empty directory against_extra, each named"
    );
    assert_eq!(
        found.len(),
        19,
        "seventeen changed files, one extra and one empty directory are nineteen differences"
    );
}

/// `verify` of the ready fixture with its seeded set, `--fab-out out
/// --rebind committed` (`AET-WO-4300k`, ruled 11:25Z (R)).
fn rebind_verify(out: &Path, committed: &Path) -> (Option<i32>, String) {
    let mut args = vec!["verify", "--spec", READY, "--scenarios"];
    args.extend(READY_SCENARIOS);
    args.extend([
        "--fab-out",
        out.to_str().unwrap(),
        "--rebind",
        committed.to_str().unwrap(),
    ]);
    aethelgard(&args)
}

/// `AET-WO-4300k`, ruled 11:25Z (R): re-binding the package `--fab-out` wrote
/// writes the same package, every gate run, and names readiness not judged.
#[test]
fn a_rebind_of_the_package_it_wrote_writes_the_same_package() {
    let (p, committed) = package_copy("a re-bind writes the same package", "rebind-same");
    let out = scratch("rebind-out").join("pkg");
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, o) = rebind_verify(&out, &committed);
    let w = one(&o);
    assert_eq!(code, Some(0), "{w}");
    assert_eq!(w["fab"]["status"], "written", "{w}");
    assert_eq!(
        w["fab"]["fabrication_ready"],
        format!("readiness not judged: a re-bind of {}", committed.display()),
        "{w}"
    );
    assert_eq!(package_files(&out), p.files);
}

/// `AET-WO-4300k`, ruled 11:25Z (R): with one byte of one Gerber flipped in
/// the committed copy, every gate passes and the re-bind is refused
/// `RebindMoved` (exit 3), naming the file, and nothing is written.
#[test]
fn a_rebind_refuses_a_package_with_one_moved_byte() {
    let (p, moved) = package_copy("a re-bind refuses a moved byte", "rebind-moved");
    let gerber = p
        .files
        .keys()
        .find(|n| n.ends_with("-F_Cu.gtl"))
        .unwrap()
        .clone();
    let mut bytes = p.files[&gerber].clone();
    let at = bytes.len() / 2;
    bytes[at] ^= 0x01;
    fs::write(moved.join(&gerber), bytes).unwrap();
    let out = scratch("rebind-out-moved").join("pkg");
    let _turn = ONE_AT_A_TIME.lock().unwrap_or_else(|e| e.into_inner());
    let (code, o) = rebind_verify(&out, &moved);
    let w = one(&o);
    assert_eq!(code, Some(3), "{w}");
    assert_eq!(w["error"]["kind"], "RebindMoved", "{w}");
    assert!(
        w["error"]["message"]
            .as_str()
            .unwrap()
            .contains(&format!("{gerber} differs")),
        "{w}"
    );
    assert!(!out.exists(), "a refused re-bind writes nothing");
}
