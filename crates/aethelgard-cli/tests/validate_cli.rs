//! The exit-code and stdout contract documented in this crate's `README.md`.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Output, Stdio};

use serde_json::Value;

fn example(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

fn run(args: &[&str], stdin: &[u8]) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    // The binary may refuse before reading everything; a broken pipe here is fine.
    let _ = child.stdin.take().unwrap().write_all(stdin);
    child.wait_with_output().unwrap()
}

/// Asserts stdout is exactly one line holding one JSON object, and returns it.
fn report(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "stdout: {stdout:?}");
    serde_json::from_str(stdout).unwrap()
}

fn assert_refused(output: &Output, code: i32, kind: &str) -> Value {
    assert_eq!(output.status.code(), Some(code));
    let report = report(output);
    assert_eq!(report["ok"], false);
    assert_eq!(report["error"]["kind"], kind, "{report}");
    report
}

#[test]
fn valid_example_exits_0() {
    let input = std::fs::read(example("sentinel-01-minimal.json")).unwrap();
    let output = run(&["validate"], &input);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        report(&output),
        serde_json::json!({
            "ok": true,
            "project_name": "sentinel-01",
            "voltage_domains": 1,
            "net_constraints": 2
        })
    );
}

#[test]
fn file_flag_reads_the_file() {
    let path = example("sentinel-01-minimal.json");
    let output = run(&["validate", "--file", path.to_str().unwrap()], b"");
    assert_eq!(output.status.code(), Some(0));
}

#[test]
fn zero_domain_example_exits_2_naming_the_domain() {
    let input = std::fs::read(example("invalid-zero-domain.json")).unwrap();
    let report = assert_refused(&run(&["validate"], &input), 2, "ZeroNominalVoltage");
    assert_eq!(report["error"]["id"], "VDD_3V3");
}

#[test]
fn wrong_shape_exits_2() {
    let output = run(&["validate"], br#"{"project_name":"p"}"#);
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert_eq!(report["error"]["id"], Value::Null);
}

#[test]
fn not_json_exits_3() {
    assert_refused(&run(&["validate"], b"not json"), 3, "InvalidJson");
}

#[test]
fn empty_input_exits_3() {
    assert_refused(&run(&["validate"], b""), 3, "InvalidJson");
}

#[test]
fn unreadable_file_exits_3() {
    let output = run(&["validate", "--file", "/nonexistent/aethelgard.json"], b"");
    assert_refused(&output, 3, "UnreadableInput");
}

#[test]
fn oversized_input_exits_3() {
    let input = vec![b' '; 16 * 1024 * 1024 + 1];
    assert_refused(&run(&["validate"], &input), 3, "InputTooLarge");
}

#[test]
fn usage_error_exits_64_with_json() {
    assert_refused(&run(&["frobnicate"], b""), 64, "UsageError");
    assert_refused(&run(&[], b""), 64, "UsageError");
}

#[test]
fn version_prints_name_and_version() {
    let output = run(&["--version"], b"");
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, b"aethelgard 0.1.0\n");
}
