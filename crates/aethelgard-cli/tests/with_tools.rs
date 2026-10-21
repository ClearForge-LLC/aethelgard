//! Tier 2: the tool subcommands against the real tools, on the calibration
//! fixtures. Built only with `--features with-tools`, which the `tools` CI job
//! enables inside the image pinned by `container/IMAGE`; the environment there
//! carries `PATH`, `KANI_HOME` and `RUSTUP_HOME`.

#![cfg(feature = "with-tools")]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::Value;
use sha2::{Digest, Sha256};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../container/fixtures")
        .join(name)
}

fn run(args: &[&str], path: Option<&str>) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.args(args).stdin(Stdio::null());
    if let Some(p) = path {
        cmd.env("PATH", p);
    }
    cmd.output().unwrap()
}

/// Exactly one JSON line on stdout, and the exit code.
fn verdict(out: &Output, code: i32) -> Value {
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    v
}

fn sha256_of(path: &Path) -> String {
    Sha256::digest(fs::read(path).unwrap())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

fn kinds(v: &Value) -> Vec<&str> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect()
}

fn assert_judged(v: &Value, check: &str, tool: &str, version: &str) {
    assert_eq!(v["check"], check);
    assert_eq!(v["tool"]["name"], tool);
    assert_eq!(v["tool"]["version"], version);
    assert!(v["sandbox"]["landlock_abi"].as_i64().unwrap() >= 3, "{v}");
    assert!(v["sandbox"]["network_confined"].is_boolean(), "{v}");
}

#[test]
fn drc_clean_board_passes() {
    let board = fixture("drc-clean.kicad_pcb");
    let v = verdict(
        &run(&["check", "drc", "--board", board.to_str().unwrap()], None),
        0,
    );
    assert_eq!(v["ok"], true);
    assert_eq!(v["verdict"], "pass");
    assert_judged(&v, "drc", "kicad-cli", "10.0.7");
    assert_eq!(v["tool"]["exit_code"], 0);
    assert_eq!(v["input"]["sha256"], sha256_of(&board));
    assert_eq!(
        v["input"]["path"],
        fs::canonicalize(&board).unwrap().to_str().unwrap()
    );
    assert_eq!(
        v["included_severities"],
        serde_json::json!(["error", "warning"])
    );
    assert_eq!(v["findings"], serde_json::json!([]));
}

#[test]
fn drc_violation_board_fails_with_exactly_one_clearance_finding() {
    let board = fixture("drc-violation.kicad_pcb");
    let v = verdict(
        &run(&["check", "drc", "--board", board.to_str().unwrap()], None),
        1,
    );
    assert_eq!(v["ok"], false);
    assert_eq!(v["verdict"], "fail");
    assert_judged(&v, "drc", "kicad-cli", "10.0.7");
    assert_eq!(v["tool"]["exit_code"], 5);
    assert_eq!(v["input"]["sha256"], sha256_of(&board));
    assert_eq!(
        kinds(&v).iter().filter(|k| **k == "clearance").count(),
        1,
        "{v}"
    );
    let clearance = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["kind"] == "clearance")
        .unwrap();
    assert_eq!(clearance["severity"], "error");
    let nets: Vec<&str> = clearance["where"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["description"].as_str().unwrap())
        .collect();
    assert!(
        nets.iter().any(|d| d.contains("NET_A")) && nets.iter().any(|d| d.contains("NET_B")),
        "{clearance}"
    );
}

#[test]
fn erc_clean_schematic_passes() {
    let sch = fixture("erc-clean.kicad_sch");
    let v = verdict(
        &run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        ),
        0,
    );
    assert_eq!(v["verdict"], "pass");
    assert_judged(&v, "erc", "kicad-cli", "10.0.7");
    assert_eq!(v["input"]["sha256"], sha256_of(&sch));
    assert_eq!(v["findings"], serde_json::json!([]));
}

#[test]
fn erc_dangling_label_fails_with_one_finding() {
    let sch = fixture("erc-dangling-label.kicad_sch");
    let v = verdict(
        &run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        ),
        1,
    );
    assert_eq!(v["verdict"], "fail");
    assert_judged(&v, "erc", "kicad-cli", "10.0.7");
    assert_eq!(v["tool"]["exit_code"], 5);
    assert_eq!(kinds(&v), ["label_dangling"]);
    assert_eq!(v["findings"][0]["severity"], "error");
}

fn prove(crate_dir: &Path, harness: &str, code: i32) -> Value {
    let v = verdict(
        &run(
            &[
                "prove",
                "--crate",
                crate_dir.to_str().unwrap(),
                "--harness",
                harness,
            ],
            None,
        ),
        code,
    );
    assert_judged(
        &v,
        "prove",
        "cargo-kani",
        "Kani Rust Verifier 0.68.0 (cargo plugin)",
    );
    assert_eq!(v["harness"], harness);
    v
}

#[test]
fn prove_verifies_the_in_range_harness() {
    let v = prove(&fixture("kani-smoke"), "add_in_range", 0);
    assert_eq!(v["verdict"], "pass");
    assert_eq!(v["findings"], serde_json::json!([]));
}

#[test]
fn prove_fails_the_seeded_overflow() {
    let v = prove(&fixture("kani-smoke"), "add_overflows", 1);
    assert_eq!(v["verdict"], "fail");
    assert_eq!(kinds(&v), ["proof_failed"]);
    assert_eq!(v["findings"][0]["message"], "attempt to add with overflow");
}

#[test]
fn prove_reports_a_compile_error() {
    let v = prove(&fixture("kani-compile-error"), "add_in_range", 1);
    assert_eq!(kinds(&v), ["compile_error"]);
}

#[test]
fn prove_reports_a_missing_harness() {
    let v = prove(&fixture("kani-smoke"), "no_such_harness", 1);
    assert_eq!(kinds(&v), ["harness_not_found"]);
}

#[test]
fn prove_leaves_the_crate_untouched() {
    let crate_dir = fixture("kani-smoke");
    let before: Vec<_> = fs::read_dir(&crate_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    prove(&crate_dir, "add_in_range", 0);
    let after: Vec<_> = fs::read_dir(&crate_dir)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(before, after, "cargo kani wrote beside the input");
}

#[test]
fn simulate_loads_the_reference_board() {
    let v = verdict(
        &run(
            &[
                "simulate",
                "--platform",
                "@platforms/boards/nucleo_h753zi.repl",
            ],
            None,
        ),
        0,
    );
    assert_eq!(v["verdict"], "pass");
    assert_judged(&v, "simulate", "renode", "Renode v1.17.0");
    assert_eq!(v["machine"]["cpus"], serde_json::json!(["cpu"]));
    let peripherals = v["machine"]["peripherals"].as_array().unwrap();
    assert!(
        peripherals.contains(&serde_json::json!({"name": "cpu", "type": "CortexM"})),
        "{v}"
    );
    assert!(
        v["input"]["path"]
            .as_str()
            .unwrap()
            .ends_with("/platforms/boards/nucleo_h753zi.repl")
    );
}

#[test]
fn simulate_fails_a_platform_that_does_not_load() {
    let repl = fixture("renode-missing-include.repl");
    let v = verdict(
        &run(&["simulate", "--platform", repl.to_str().unwrap()], None),
        1,
    );
    assert_eq!(v["verdict"], "fail");
    assert_eq!(kinds(&v), ["platform_load_error"]);
    assert_eq!(
        v["tool"]["exit_code"], 0,
        "Renode exits 0 on a failed load; the output decided"
    );
}

/// §3.3: with a tool removed from `PATH`, exit 4 naming exactly that tool and
/// no verdict.
fn assert_tool_missing(out: &Output, tool: &str) {
    let v = verdict(out, 4);
    assert_eq!(v["error"]["kind"], "ToolMissing");
    assert_eq!(v["error"]["id"], tool);
    assert!(v.get("verdict").is_none(), "{v}");
}

/// `PATH` without every directory that holds `tool`. Every one, not the first:
/// with merged /usr, /bin and /usr/bin are the same directory.
fn path_without(tool: &str) -> String {
    std::env::var("PATH")
        .unwrap()
        .split(':')
        .filter(|d| !Path::new(d).join(tool).exists())
        .collect::<Vec<_>>()
        .join(":")
}

#[test]
fn each_tool_removed_from_path_is_named() {
    let nucleo = [
        "simulate",
        "--platform",
        "@platforms/boards/nucleo_h753zi.repl",
    ];
    assert_tool_missing(&run(&nucleo, Some(&path_without("renode"))), "renode");
    let board = fixture("drc-clean.kicad_pcb");
    let drc = ["check", "drc", "--board", board.to_str().unwrap()];
    assert_tool_missing(&run(&drc, Some(&path_without("kicad-cli"))), "kicad-cli");
    let sch = fixture("erc-clean.kicad_sch");
    let erc = ["check", "erc", "--schematic", sch.to_str().unwrap()];
    assert_tool_missing(&run(&erc, Some(&path_without("kicad-cli"))), "kicad-cli");
    let krate = fixture("kani-smoke");
    let prove = [
        "prove",
        "--crate",
        krate.to_str().unwrap(),
        "--harness",
        "add_in_range",
    ];
    assert_tool_missing(&run(&prove, Some(&path_without("cargo"))), "cargo");
    // cargo present, cargo-kani absent: a directory holding only cargo.
    let only_cargo = std::env::temp_dir().join(format!("aet-only-cargo-{}", std::process::id()));
    fs::create_dir_all(&only_cargo).unwrap();
    let _ = symlink(which("cargo"), only_cargo.join("cargo"));
    let path = format!("{}:{}", only_cargo.display(), path_without("cargo-kani"));
    assert_tool_missing(&run(&prove, Some(&path)), "cargo-kani");
    fs::remove_dir_all(&only_cargo).unwrap();
}

fn which(tool: &str) -> String {
    std::env::var("PATH")
        .unwrap()
        .split(':')
        .map(|d| Path::new(d).join(tool))
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("{tool} is not on PATH; tier 2 needs the tools image"))
        .to_str()
        .unwrap()
        .to_owned()
}
