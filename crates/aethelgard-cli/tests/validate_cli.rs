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
fn solved_topology_document_exits_2() {
    let solved = serde_json::json!({
        "schema_version": 1,
        "specification_sha256": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
        "assignments": {}
    });
    let output = run(&["validate"], solved.to_string().as_bytes());
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert_eq!(report["error"]["id"], Value::Null);
}

#[test]
fn solved_field_smuggled_into_a_specification_exits_2() {
    let mut doc: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    doc["assignments"] = serde_json::json!({});
    let output = run(&["validate"], doc.to_string().as_bytes());
    assert_refused(&output, 2, "SchemaViolation");
}

#[test]
fn invalid_identifier_exits_2_with_a_path_in_the_message() {
    let mut doc: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    doc["i2c_buses"][0]["sda"]["net"] = "I2C SDA".into();
    let output = run(&["validate"], doc.to_string().as_bytes());
    let report = assert_refused(&output, 2, "InvalidIdentifier");
    assert_eq!(report["error"]["id"], Value::Null);
    let message = report["error"]["message"].as_str().unwrap();
    assert!(message.contains("i2c_buses[0].sda.net"), "{message}");
    assert_eq!(report["error"]["path"], "i2c_buses[0].sda.net");
}

/// `AET-WO-1102` §1.3: every refused character and both clock shapes, in a
/// net (the base set), the project name (a crate name) and a reference (a
/// Renode identifier), each refused by `validate` by its path before any gate
/// runs. `generate` refuses the last two itself, so this is `INV-06`.
#[test]
fn every_refused_character_and_shape_is_refused_by_its_path() {
    let base: Value =
        serde_json::from_slice(&std::fs::read(example("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    let refused = |doc: &Value, path: &str| {
        let output = run(&["validate"], doc.to_string().as_bytes());
        let report = assert_refused(&output, 2, "InvalidIdentifier");
        assert_eq!(report["error"]["path"], path, "{report}");
    };
    let cases: Vec<String> = ["/", "{", "}", "$", "[", "]", "\""]
        .iter()
        .map(|c| format!("AB{c}CD"))
        .chain(["AB2026-11-13CD".to_owned(), "AB12:34:56CD".to_owned()])
        .collect();
    for id in &cases {
        let mut doc = base.clone();
        doc["i2c_buses"][0]["sda"]["net"] = id.as_str().into();
        refused(&doc, "i2c_buses[0].sda.net");
        // A project name and a reference are narrower still, so these fail too.
        let mut doc = base.clone();
        doc["project_name"] = id.to_lowercase().into();
        refused(&doc, "project_name");
        let mut doc = base.clone();
        doc["spi_devices"][0]["reference"] = id.as_str().into();
        refused(&doc, "spi_devices[0].reference");
    }
    // What `generate` refuses and `validate` once passed (3300 F14, F15).
    for (path, edit) in [
        ("project_name", "My_Board"),
        ("project_name", "sentinel_01"),
        ("i2c_devices[0].reference", "U-2"),
        ("i2c_devices[0].reference", "U+2"),
    ] {
        let mut doc = base.clone();
        match path {
            "project_name" => doc["project_name"] = edit.into(),
            _ => doc["i2c_devices"][0]["reference"] = edit.into(),
        }
        refused(&doc, path);
    }
    // And KiCad's own `+3V3` and a net with every other character stay valid.
    let mut doc = base.clone();
    doc["i2c_buses"][0]["sda"]["net"] = "+3V3-(Net#1)!,;'~^|".into();
    doc["net_constraints"][0]["net_name"] = "+3V3-(Net#1)!,;'~^|".into();
    assert_eq!(
        run(&["validate"], doc.to_string().as_bytes()).status.code(),
        Some(0)
    );
}

#[test]
fn path_is_absent_from_every_other_refusal() {
    let output = run(&["validate"], br#"{"project_name":"p"}"#);
    let report = assert_refused(&output, 2, "SchemaViolation");
    assert!(report["error"].get("path").is_none(), "{report}");
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
