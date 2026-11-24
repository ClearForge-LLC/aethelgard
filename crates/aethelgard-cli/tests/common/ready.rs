//! `AET-WO-4300k` (T), ruled 2026-11-24T11:25Z: `verify --fab-out` refuses
//! `FabricationNotReady` before any gate on a specification that is not
//! ready, so a `--fab-out` test whose point is a downstream gate runs on a
//! ready one. [`READY`] is the one ready fixture (sentinel-01-minimal made
//! ready by `spike/4300k/build/ready_fixture.py`), with its seeded set
//! rebased on it. [`ready`] makes another fixture ready the same way, for a
//! test whose point is a board the two-layer fixture is not (the four-layer
//! case). Included by `#[path]` beside `mod common;`, which it uses.
#![allow(dead_code)]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::{Value, json};

use crate::common::verify::root;

/// The ready fixture: the reference, its reset supervisor (U12, R61, C62),
/// `fab_tolerances`, a "test fixture" fabricator, every stackup bound cited
/// `{condition: test-fixture}`, and its conditions acknowledged.
pub const READY: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready.json";
/// The reference's seeded set (`AET-WO-3303` D1, ruled), rebased on [`READY`].
pub const READY_SCENARIOS: [&str; 5] = [
    "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready/pullup-3k3.json",
    "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready/droop-bor3.json",
    "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready/spi-absent-u4.json",
    "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready/pullup-2k2.json",
    "crates/aethelgard-cli/tests/fixtures/sentinel-01-ready/serial-rx-open-console.json",
];

/// Every acknowledgement [`ready`] writes gives this reason.
const REASON: &str = "test fixture (AET-WO-4300k (T)): never ordered; a test's readied copy acknowledges each condition aethelgard_cli::readiness::conditions computes for it";

fn fixture() -> Value {
    serde_json::from_slice(&fs::read(root().join(READY)).unwrap()).unwrap()
}

/// Cites `cite` on every object under `v` with a `source` and no `cites`,
/// as `ready_fixture.py`'s `cite` does.
fn cite_all(v: &mut Value, cite: &Value) {
    match v {
        Value::Object(m) => {
            if m.contains_key("source") && !m.contains_key("cites") {
                m.insert("cites".to_owned(), cite.clone());
            }
            m.values_mut().for_each(|x| cite_all(x, cite));
        }
        Value::Array(a) => a.iter_mut().for_each(|x| cite_all(x, cite)),
        _ => {}
    }
}

/// Every `cites` under `v`.
fn cites(v: &Value, out: &mut Vec<Value>) {
    match v {
        Value::Object(m) => {
            if let Some(c) = m.get("cites") {
                out.push(c.clone());
            }
            m.values().for_each(|x| cites(x, out));
        }
        Value::Array(a) => a.iter().for_each(|x| cites(x, out)),
        _ => {}
    }
}

/// `spec` made ready as [`READY`] is: the ready fixture's
/// `mcu.reset_supervisor`, `board.stackup.fabricator` and `fab_tolerances`,
/// the fixture's one condition cited on every stackup bound that cites
/// nothing, then exactly the conditions `aethelgard_cli::readiness::conditions`
/// computes for the result acknowledged, each with [`REASON`]. Computed here,
/// so it never goes stale; the result validates.
pub fn ready(mut spec: Value) -> Value {
    let fixture = fixture();
    let from = &fixture["board"]["stackup"];
    let mut cited = Vec::new();
    for (k, v) in from.as_object().unwrap() {
        if k != "fabricator" {
            cites(v, &mut cited);
        }
    }
    let cite = cited[0].clone();
    assert!(
        cited.iter().all(|c| *c == cite),
        "the ready fixture cites one condition on every bound: {cited:?}"
    );
    spec["mcu"]["reset_supervisor"] = fixture["mcu"]["reset_supervisor"].clone();
    let stack = spec["board"]["stackup"].as_object_mut().unwrap();
    stack.insert("fabricator".to_owned(), from["fabricator"].clone());
    stack.insert("fab_tolerances".to_owned(), from["fab_tolerances"].clone());
    for (k, v) in stack.iter_mut() {
        if k != "fabricator" {
            cite_all(v, &cite);
        }
    }
    let typed: aethelgard_ir::Specification = serde_json::from_value(spec.clone()).unwrap();
    typed.validate().unwrap();
    let conditions = aethelgard_cli::readiness::conditions(&typed).unwrap();
    spec["acknowledge_conditions"] = conditions
        .keys()
        .map(|id| json!({"id": id, "reason": REASON}))
        .collect();
    let typed: aethelgard_ir::Specification = serde_json::from_value(spec.clone()).unwrap();
    typed.validate().unwrap();
    spec
}

/// The reference's console scenario (`AET-WO-2302` D5): the open RX line,
/// detected by its mechanism.
pub const CONSOLE_SCENARIO: &str = "scenarios/sentinel-01/seeded/serial-rx-open-console.json";

/// `spec` with the reference's console (`AET-WO-4300k` Q3 (a), ruled
/// 12:45Z): its `CONSOLE` serial port, its connector under `reference`, and
/// the reference's `acknowledge_unwitnessed`, which are a console-less
/// board's and the console's together (`fab_tier1`'s new-field case). A
/// board with no serial port generates no hwdef (`NoSerialPort`), which the
/// readiness gate refuses; run it with [`CONSOLE_SCENARIO`] so the campaign
/// covers the port.
pub fn with_console(mut spec: Value, reference: &str) -> Value {
    let r: Value = serde_json::from_slice(
        &fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap(),
    )
    .unwrap();
    spec["serial_ports"] = r["serial_ports"].clone();
    let mut conn = r["connectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c.to_string().contains("CONSOLE_TX"))
        .unwrap()
        .clone();
    conn["reference"] = reference.into();
    let conns = spec["connectors"].as_array_mut().unwrap();
    assert!(
        conns.iter().all(|c| c["reference"] != reference),
        "{reference} is free"
    );
    conns.push(conn);
    spec["acknowledge_unwitnessed"] = r["acknowledge_unwitnessed"].clone();
    spec
}

/// `spec` written as `spec.json` in `dir`, beside each of `scenarios` (paths
/// from the repository root) rebased on it under its own file name: a
/// scenario's `base` must resolve inside the current directory
/// (`BaseOutsideRoot`), so `verify` runs from `dir` ([`aethelgard_in`]).
/// Returns the scenarios' file names, in order.
pub fn rebased(dir: &Path, spec: &[u8], scenarios: &[&str]) -> Vec<String> {
    fs::create_dir_all(dir).unwrap();
    fs::write(dir.join("spec.json"), spec).unwrap();
    scenarios
        .iter()
        .map(|path| {
            let mut scenario: Value =
                serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap();
            scenario["base"] = "spec.json".into();
            let name = Path::new(path)
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            fs::write(dir.join(&name), scenario.to_string()).unwrap();
            name
        })
        .collect()
}

/// Runs the binary from `dir`: its exit code and stdout, logged as
/// `common::verify::aethelgard` logs a run.
pub fn aethelgard_in(dir: &Path, args: &[&str]) -> (Option<i32>, String) {
    let start = std::time::Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let line = format!(
        "ANSWER 3301: {} exit={:?} wall_s={:.2}\n",
        args[0],
        out.status.code(),
        start.elapsed().as_secs_f64()
    );
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(line.as_bytes()).unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}
