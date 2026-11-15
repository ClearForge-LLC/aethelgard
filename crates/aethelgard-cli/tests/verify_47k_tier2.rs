//! `verify` with the 47 kΩ scenario in a covering campaign, run once with
//! `--fab-out` into an empty directory and read by two tests (`AET-WO-1909`
//! C, ruled on PR #115): `AET-WO-3303` D1's detection beside a failing run
//! (ruled (a)), and `AET-WO-3300`'s nothing written. `--fab-out` changes no gate's input, and
//! the verdict equals the one without it apart from its `fab` key
//! (`AET-WO-1909` FEEDBACK).

#![cfg(feature = "with-tools")]

mod common;

use std::fs;

use common::verify::{SPEC, Shared, aethelgard, one, scratch};
use serde_json::Value;

/// The run's exit code and verdict, and how many entries `--fab-out` holds after it.
struct Run {
    code: Option<i32>,
    verdict: Value,
    entries: usize,
}

static RUN: Shared<Run> = Shared::new("verify 47k --fab-out into an empty directory");

/// The absent flash, declaring one of its two kinds (`AET-WO-3303` D1).
const ONE_KIND: &str = "crates/aethelgard-cli/tests/fixtures/spi-absent-u4-one-kind.json";

fn run(property: &str) -> &'static Run {
    RUN.get(property, || {
        let dir = scratch("verify-47k").join("fab");
        fs::create_dir_all(&dir).unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
        }
        let (code, stdout) = aethelgard(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            "scenarios/sentinel-01/pullup-47k.json",
            "scenarios/sentinel-01/droop-bor3.json",
            ONE_KIND,
            "scenarios/sentinel-01/pullup-2k2.json",
            "--fab-out",
            dir.to_str().unwrap(),
        ]);
        Run {
            code,
            verdict: one(&stdout),
            entries: fs::read_dir(&dir).unwrap().count(),
        }
    })
}

/// `AET-WO-3303` D1, ruled (a): the 47 kΩ pull-up, which no design survives,
/// is detected by its mechanism beside the passing baseline (its `attribute`
/// gate passes and it is no finding), while the absent flash whose scenario
/// declares only one of its two kinds fails the campaign gate: a run that
/// fails otherwise than its scenario says fails `verify`, though the
/// campaign covers every bus and rail as declared.
#[test]
fn verify_fails_a_run_that_fails_otherwise_than_declared_and_detects_47k() {
    let r = run("verify detects 47k and fails the one-kind absent flash");
    let v = &r.verdict;
    assert_eq!(r.code, Some(1), "verify exits 1: {v}");
    assert_eq!(v["verdict"], "fail", "the verdict is fail: {v}");
    let gates = v["gates"].as_array().unwrap();
    let campaign = gates.last().unwrap();
    assert_eq!(
        campaign["gate"], "campaign",
        "the campaign is the last gate: {v}"
    );
    assert_eq!(campaign["verdict"], "fail", "the campaign gate fails: {v}");
    assert!(
        gates.iter().any(|g| g["gate"] == "attribute"
            && g["scenario"] == "pullup-47k"
            && g["verdict"] == "pass"),
        "47k's run is attributed: {v}"
    );
    let findings = campaign["findings"].as_array().unwrap();
    let mut kinds: Vec<&str> = findings
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    kinds.sort_unstable();
    assert_eq!(
        kinds,
        ["spi_identity_mismatch", "spi_transaction_unwitnessed"],
        "only the one-kind scenario fails the gate: {v}"
    );
    assert!(
        findings.iter().all(|f| f["message"]
            .as_str()
            .unwrap()
            .contains("spi-absent-u4-one-kind")),
        "{v}"
    );
    assert!(
        gates[..gates.len() - 1]
            .iter()
            .all(|g| g["verdict"] == "pass"),
        "every gate before the campaign passes: {v}"
    );
}

/// §3 of `AET-WO-3300`: a failing `verify` writes nothing into `--fab-out`;
/// the verdict says so.
#[test]
fn verify_fab_out_writes_nothing_when_a_gate_fails() {
    let r = run("verify --fab-out writes nothing when a gate fails");
    let v = &r.verdict;
    assert_eq!(r.code, Some(1), "a failing verify exits 1: {v}");
    assert_eq!(v["verdict"], "fail", "the verdict is fail: {v}");
    assert_eq!(
        v["fab"]["status"], "not_written",
        "the package is not written: {v}"
    );
    assert_eq!(
        v["fab"]["files"],
        serde_json::json!({}),
        "the verdict names no package file: {v}"
    );
    assert_eq!(r.entries, 0, "a failing run wrote nothing into --fab-out");
}
