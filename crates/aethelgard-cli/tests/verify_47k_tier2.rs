//! `verify` with the 47 kΩ scenario, run once with `--fab-out` into an empty
//! directory and read by two tests (`AET-WO-1909` C, ruled on PR #115):
//! `AET-WO-3301`'s failing verdict with the campaign's findings, and
//! `AET-WO-3300`'s nothing written. `--fab-out` changes no gate's input, and
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

/// §3.2 of `AET-WO-3301`: the 47 kΩ scenario fails `verify`, with the
/// campaign's findings, although that scenario's own expectation (`fail`) is
/// met.
#[test]
fn verify_fails_the_47k_pull_up_with_the_campaigns_findings() {
    let r = run("verify fails the 47k pull-up with the campaign's findings");
    let v = &r.verdict;
    assert_eq!(r.code, Some(1), "verify on 47k exits 1: {v}");
    assert_eq!(v["verdict"], "fail", "the verdict is fail: {v}");
    let gates = v["gates"].as_array().unwrap();
    let campaign = gates.last().unwrap();
    assert_eq!(
        campaign["gate"], "campaign",
        "the campaign is the last gate: {v}"
    );
    assert_eq!(campaign["verdict"], "fail", "the campaign gate fails: {v}");
    assert_eq!(
        campaign["exit_code"], 0,
        "the scenario met its own expectation: {v}"
    );
    let mut kinds: Vec<&str> = campaign["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    kinds.sort_unstable();
    assert_eq!(
        kinds,
        ["analog_rise_exceeds_ceiling", "i2c_timeout"],
        "the campaign gate's findings are exactly the 47k scenario's: {v}"
    );
    assert!(
        gates[..gates.len() - 1]
            .iter()
            .all(|g| g["verdict"] == "pass"),
        "every gate before the campaign passes: {v}"
    );
    assert_eq!(
        v["campaign"][0]["verdict"], "fail",
        "the campaign's run fails: {v}"
    );
}

/// §3 of `AET-WO-3300`: the 47 kΩ scenario fails `verify`, and nothing is
/// written into `--fab-out`; the verdict says so.
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
