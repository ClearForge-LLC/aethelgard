//! AET-WO-1400 spike, S1, tier 2: `aethelgard prove` on the hand-written probe
//! in `spike/s1-probe`, inside the tools image. The correct board proves; each
//! of the three seeded bugs fails on the assertion that names it. Each test
//! also writes an `ANSWER 1400-S1` line to `/dev/stderr`, opened as a file:
//! libtest captures `eprintln!` and `io::stderr()` (measured on 1.99.1), but
//! not a separate handle, so the run's log carries the measurement without a
//! deliberate failure. The build half deletes this file with the probe.

#![cfg(feature = "with-tools")]

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use serde_json::Value;

fn prove(board: &str, code: i32) -> Value {
    let probe = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spike/s1-probe");
    let harness = format!("proofs::{board}::every_valid_sequence");
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "prove",
            "--crate",
            probe.to_str().unwrap(),
            "--harness",
            &harness,
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let wall = start.elapsed().as_secs_f64();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let line = format!(
        "ANSWER 1400-S1: {board}: exit={:?} wall_s={wall:.2} verdict={}\n",
        out.status.code(),
        stdout.trim()
    );
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(line.as_bytes()).unwrap();
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    assert_eq!(v["check"], "prove");
    assert_eq!(v["harness"], harness.as_str());
    assert_eq!(
        v["tool"]["version"],
        "Kani Rust Verifier 0.68.0 (cargo plugin)"
    );
    v
}

fn failed_on(board: &str, message: &str) {
    let v = prove(board, 1);
    assert_eq!(v["verdict"], "fail");
    let findings = v["findings"].as_array().unwrap();
    assert_eq!(findings.len(), 1, "{v}");
    assert_eq!(findings[0]["kind"], "proof_failed");
    assert_eq!(findings[0]["message"], message);
}

#[test]
fn s1_the_correct_board_proves() {
    let v = prove("good", 0);
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["findings"], serde_json::json!([]));
}

#[test]
fn s1_a_wrong_af_number_fails() {
    failed_on("bug_wrong_af", "PB8/PB9 AFRH field is not AF4");
}

#[test]
fn s1_a_missing_clock_enable_fails() {
    failed_on(
        "bug_no_clock",
        "I2C1 written before RCC_APB1LENR.I2C1EN is set",
    );
}

#[test]
fn s1_a_write_past_the_block_fails() {
    failed_on(
        "bug_past_block",
        "register access past the end of its block",
    );
}
