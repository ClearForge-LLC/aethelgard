//! Tier 2: the tool subcommands against the real tools, on the calibration
//! fixtures. Built only with `--features with-tools`, which the `tools` CI job
//! enables inside the image pinned by `container/IMAGE`; the environment there
//! carries `PATH`, `KANI_HOME` and `RUSTUP_HOME`.

#![cfg(feature = "with-tools")]

mod common;

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
    assert!(v["sandbox"]["tcp_confined"].is_boolean(), "{v}");
    assert!(v["sandbox"]["scoped"].is_boolean(), "{v}");
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
fn prove_of_an_unresolvable_dependency_names_the_package() {
    let dir = std::env::temp_dir().join(format!("aet-unresolvable-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(dir.join("src")).unwrap();
    fs::write(
        dir.join("Cargo.toml"),
        "[package]\nname = \"unresolvable\"\nversion = \"0.0.0\"\nedition = \"2021\"\npublish = false\n\n[workspace]\n\n[dependencies]\naet-no-such-package-1502 = \"1\"\n",
    )
    .unwrap();
    fs::write(dir.join("src/lib.rs"), "pub fn f() {}\n").unwrap();
    let out = run(
        &["prove", "--crate", dir.to_str().unwrap(), "--harness", "h"],
        None,
    );
    let _ = fs::remove_dir_all(&dir);
    let v = verdict(&out, 5);
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
    let message = v["error"]["message"].as_str().unwrap();
    assert!(message.contains("; the tool said: error:"), "{v}");
    assert!(message.contains("aet-no-such-package-1502"), "{v}");
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

// ---------------------------------------------------------------- through `aethelgard mcp`

/// One tool call through the MCP server; returns the call's `result`.
fn mcp_call(tool: &str, arguments: Value) -> Value {
    let mut client = common::Client::spawn(|_| {});
    client.initialize();
    let response = client.call(tool, arguments);
    assert!(response.get("error").is_none(), "{response}");
    response["result"].clone()
}

fn mcp_judged(result: &Value, exit_code: i64, verdict: &str) {
    assert_eq!(result["isError"], false, "{result}");
    assert_eq!(
        result["structuredContent"]["exit_code"], exit_code,
        "{result}"
    );
    assert_eq!(
        result["structuredContent"]["verdict"]["verdict"], verdict,
        "{result}"
    );
}

#[test]
fn mcp_check_drc_passes_the_clean_board_and_fails_the_violation_without_is_error() {
    let clean = fixture("drc-clean.kicad_pcb");
    mcp_judged(
        &mcp_call("check_drc", serde_json::json!({"board": clean})),
        0,
        "pass",
    );
    let bad = fixture("drc-violation.kicad_pcb");
    mcp_judged(
        &mcp_call("check_drc", serde_json::json!({"board": bad})),
        1,
        "fail",
    );
}

#[test]
fn mcp_prove_fails_the_seeded_overflow_without_is_error() {
    let krate = fixture("kani-smoke");
    let r = mcp_call(
        "prove",
        serde_json::json!({"crate": krate, "harness": "add_overflows"}),
    );
    mcp_judged(&r, 1, "fail");
}

#[test]
fn mcp_simulate_loads_the_nucleo_board() {
    let r = mcp_call(
        "simulate",
        serde_json::json!({"platform": "@platforms/boards/nucleo_h753zi.repl"}),
    );
    mcp_judged(&r, 0, "pass");
}

// ---- AET-WO-1300 §1.4: the emitter and the gate's own rules, judged by KiCad ----

fn example_spec(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Temp(PathBuf);

impl Temp {
    fn new(tag: &str) -> Temp {
        let d = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!("aet-1300-{tag}"));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        Temp(d)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// One line on the job's stderr, past libtest's capture, so the run log is
/// the record of what KiCad said.
fn record(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1300 {line}\n").as_bytes());
}

fn emit_to(spec: &Path, out: &Path) -> Value {
    let o = run(
        &[
            "emit",
            "--file",
            spec.to_str().unwrap(),
            "--out",
            out.to_str().unwrap(),
        ],
        None,
    );
    verdict(&o, 0)
}

#[test]
fn the_emitted_reference_schematic_passes_erc() {
    let t = Temp::new("erc");
    let r = emit_to(&example_spec("sentinel-01-minimal.json"), &t.0);
    let sch = r["schematic"]["path"].as_str().unwrap().to_owned();
    let out = run(&["check", "erc", "--schematic", &sch], None);
    record(&format!(
        "erc on the emitted sentinel-01: {}",
        String::from_utf8_lossy(&out.stdout).trim()
    ));
    let v = verdict(&out, 0);
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_judged(&v, "erc", "kicad-cli", "10.0.7");
}

/// Every character `RUL-14` still admits in a net or domain that an
/// S-expression must quote, and others, in every identifier class that
/// reaches the schematic (`AET-WO-1102`: the project name and the references
/// are narrower now, `p-x` and `U_2`; `"` is refused everywhere).
#[test]
fn identifiers_with_s_expression_characters_survive_kicad() {
    let t = Temp::new("names");
    let mut spec: Value =
        serde_json::from_slice(&fs::read(example_spec("sentinel-01-minimal.json")).unwrap())
            .unwrap();
    let rename = |v: &mut Value, from: &str, to: &str| {
        let text = serde_json::to_string(v).unwrap();
        *v = serde_json::from_str(
            &text.replace(&format!("\"{from}\""), &serde_json::to_string(to).unwrap()),
        )
        .unwrap();
    };
    let names = [
        ("sentinel-01", "p-x"),
        ("VDD_3V3", "VDD'3V3^"),
        ("I2C_SDA", "I2C(SDA)"),
        ("I2C_SCL", "I2C;SCL~"),
        ("SPI_SCK", "SPI'SCK^"),
        ("SPI_MOSI", "MO|SI!"),
        ("SPI_MISO", "MI+SO?"),
        ("FLASH_CS", "CS@(0)"),
        ("U2", "U_2"),
        ("U3", "U_3"),
        ("U4", "U_4"),
    ];
    for (from, to) in names {
        rename(&mut spec, from, to);
    }
    let spec_path = t.0.join("spec.json");
    fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
    let r = emit_to(&spec_path, &t.0.join("out"));
    let sch = PathBuf::from(r["schematic"]["path"].as_str().unwrap());
    let v = verdict(
        &run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        ),
        0,
    );
    assert_eq!(v["verdict"], "pass", "{v}");
    // Read every name back from KiCad's own netlist of the same file.
    let copy = t.0.join("readback.kicad_sch");
    fs::copy(&sch, &copy).unwrap();
    let net = t.0.join("readback.net");
    let out = Command::new("kicad-cli")
        .args(["sch", "export", "netlist", "-o"])
        .arg(&net)
        .arg(&copy)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let netlist = aethelgard_eda::sexpr::parse(&fs::read_to_string(&net).unwrap()).unwrap();
    let nets: Vec<&str> = netlist
        .child("nets")
        .unwrap()
        .children("net")
        .filter_map(|n| n.child("name").and_then(|x| x.arg(1)))
        .collect();
    let refs: Vec<&str> = netlist
        .child("components")
        .unwrap()
        .children("comp")
        .filter_map(|c| c.child("ref").and_then(|x| x.arg(1)))
        .collect();
    let title = netlist
        .child("design")
        .and_then(|d| d.child("sheet"))
        .and_then(|s| s.child("title_block"))
        .and_then(|t| t.child("title"))
        .and_then(|t| t.arg(1));
    record(&format!(
        "names read back: nets {nets:?} refs {refs:?} title {title:?}"
    ));
    assert_eq!(title, Some("p-x"));
    assert!(nets.contains(&"VDD'3V3^"), "{nets:?}");
    for (_, to) in &names[2..8] {
        // A root-sheet label's net is named `/<label>` (spike S4).
        let want = format!("/{to}");
        assert!(nets.contains(&want.as_str()), "{want} not in {nets:?}");
    }
    for (_, to) in &names[8..] {
        assert!(refs.contains(to), "{to} not in {refs:?}");
    }
}

/// S3's board: two tracks 0.1 mm apart, and its own setup block, project and
/// rules file all saying 0.05 mm.
const LOOSE: &str = "(kicad_pcb\n\t(version 20240108)\n\t(generator \"aethelgard\")\n\t(general (thickness 1.6) (legacy_teardrops no))\n\t(paper \"A4\")\n\t(layers (0 \"F.Cu\" signal) (31 \"B.Cu\" signal) (44 \"Edge.Cuts\" user))\n\t(setup (pad_to_mask_clearance 0) (trace_clearance 0.05) (clearance_min 0.05))\n\t(net 0 \"\")\n\t(net 1 \"NET_A\")\n\t(net 2 \"NET_B\")\n\t(gr_rect (start 100 100) (end 130 120) (stroke (width 0.1) (type default)) (fill none) (layer \"Edge.Cuts\"))\n\t(segment (start 105 110) (end 125 110) (width 0.25) (layer \"F.Cu\") (net 1))\n\t(segment (start 105 110.35) (end 125 110.35) (width 0.25) (layer \"F.Cu\") (net 2))\n)\n";

/// `LOOSE` in KiCad 10.0.7's format (the engine's: copper ids 0 and 2, nets
/// by name), with its declared thickness: with `--spec` the gate first
/// requires the board to be the declared one (`AET-WO-4100`), which `LOOSE`'s
/// KiCad 8 table (B.Cu as 31) is not. The same tracks, 0.1 mm apart.
const LOOSE10: &str = "(kicad_pcb\n\t(version 20260206)\n\t(generator \"aethelgard\")\n\t(general (thickness 1.6) (legacy_teardrops no))\n\t(paper \"A4\")\n\t(layers (0 \"F.Cu\" signal) (2 \"B.Cu\" signal) (25 \"Edge.Cuts\" user))\n\t(setup (pad_to_mask_clearance 0) (trace_clearance 0.05) (clearance_min 0.05))\n\t(gr_rect (start 100 100) (end 130 120) (stroke (width 0.1) (type default)) (fill no) (layer \"Edge.Cuts\"))\n\t(segment (start 105 110) (end 125 110) (width 0.25) (layer \"F.Cu\") (net \"NET_A\"))\n\t(segment (start 105 110.35) (end 125 110.35) (width 0.25) (layer \"F.Cu\") (net \"NET_B\"))\n)\n";

#[test]
fn a_board_whose_own_files_are_looser_is_judged_by_the_gate() {
    let t = Temp::new("loose");
    let board = t.0.join("loose.kicad_pcb");
    fs::write(&board, LOOSE10).unwrap();
    let loose = aethelgard_eda::rules::Rules {
        arrays: Vec::new(),
        via_nm: (
            aethelgard_eda::route::VIA_NM,
            aethelgard_eda::route::DRILL_NM,
        ),
        source: aethelgard_eda::rules::Source::Specification,
        clearance_nm: 50_000,
        min_track_width_nm: None,
        classes: Vec::new(),
    };
    fs::write(t.0.join("loose.kicad_pro"), loose.project("loose")).unwrap();
    fs::write(t.0.join("loose.kicad_dru"), loose.custom_rules()).unwrap();
    let spec = example_spec("sentinel-01-minimal.json");
    for (args, rules) in [
        (vec!["--spec", spec.to_str().unwrap()], "specification"),
        (vec![], "kicad-default"),
    ] {
        let mut all = vec!["check", "drc", "--board", board.to_str().unwrap()];
        all.extend(args);
        let out = run(&all, None);
        record(&format!(
            "drc on the loose board, rules {rules}: {}",
            String::from_utf8_lossy(&out.stdout).trim()
        ));
        let v = verdict(&out, 1);
        assert_eq!(v["rules"], rules, "{v}");
        assert_eq!(
            kinds(&v).iter().filter(|k| **k == "clearance").count(),
            1,
            "{v}"
        );
    }
}

/// D5's row, measured (the spike did not run it): with the gate's setup
/// carrying no clearance token, the project file sets the board minimum,
/// which is all a locally overridden pad is judged against. With the board's
/// own looser project the override escapes the gate's custom rules; with the
/// gate's it is caught. So the project step is load-bearing. Raw
/// `kicad-cli`, because the CLI itself refuses the override (D4).
#[test]
fn with_a_token_free_setup_the_project_step_is_load_bearing() {
    let t = Temp::new("d5");
    let mut fp = aethelgard_eda::library::footprint("Resistor_SMD:R_0402_1005Metric").unwrap();
    use aethelgard_eda::sexpr::Sx;
    if let Some(items) = fp.items_mut() {
        items[1] = Sx::str("Resistor_SMD:R_0402_1005Metric");
        items.retain(|i| {
            !matches!(
                i.head(),
                Some("version" | "generator" | "generator_version")
            )
        });
        items.insert(2, Sx::node("at", &["115", "112"]));
        for item in items.iter_mut() {
            if item.head() == Some("pad")
                && let Some(pad) = item.items_mut()
            {
                pad.push(Sx::List(vec![
                    Sx::atom("net"),
                    Sx::atom("1"),
                    Sx::str("NET_A"),
                ]));
                pad.push(Sx::node("clearance", &["0.05"]));
            }
        }
    }
    // Pad 1 is 0.64 mm tall at y 112: its top edge is at 111.68. A 0.25 mm
    // NET_B track centred 0.225 mm above that leaves a 0.1 mm gap.
    let footprint = aethelgard_eda::sexpr::write(&fp);
    let board = LOOSE
        .replace(
            "(setup (pad_to_mask_clearance 0) (trace_clearance 0.05) (clearance_min 0.05))",
            aethelgard_eda::rules::SETUP,
        )
        .replace(
            "\t(segment (start 105 110) (end 125 110) (width 0.25) (layer \"F.Cu\") (net 1))\n\t(segment (start 105 110.35) (end 125 110.35) (width 0.25) (layer \"F.Cu\") (net 2))\n",
            &format!("{footprint}\t(segment (start 110 111.455) (end 120 111.455) (width 0.25) (layer \"F.Cu\") (net 2))\n"),
        );
    let gate = aethelgard_eda::rules::Rules::from_stack(
        &aethelgard_eda::stack::Stack::of(
            &serde_json::from_slice(&fs::read(example_spec("sentinel-01-minimal.json")).unwrap())
                .unwrap(),
        )
        .unwrap(),
    );
    let own = aethelgard_eda::rules::Rules {
        arrays: Vec::new(),
        via_nm: (
            aethelgard_eda::route::VIA_NM,
            aethelgard_eda::route::DRILL_NM,
        ),
        source: aethelgard_eda::rules::Source::Specification,
        clearance_nm: 50_000,
        min_track_width_nm: None,
        classes: Vec::new(),
    };
    let mut counts = Vec::new();
    for (name, project) in [("board's own project", &own), ("gate's project", &gate)] {
        let dir = t.0.join(name.replace([' ', '\''], "_"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("d5.kicad_pcb"), &board).unwrap();
        fs::write(dir.join("d5.kicad_pro"), project.project("d5")).unwrap();
        fs::write(dir.join("d5.kicad_dru"), gate.custom_rules()).unwrap();
        let report = dir.join("report.json");
        let out = Command::new("kicad-cli")
            .args([
                "pcb",
                "drc",
                "--exit-code-violations",
                "--format",
                "json",
                "-o",
            ])
            .arg(&report)
            .arg(dir.join("d5.kicad_pcb"))
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let doc: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
        let clearance: Vec<&str> = doc["violations"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["type"] == "clearance")
            .map(|f| f["description"].as_str().unwrap())
            .collect();
        record(&format!(
            "D5 row, token-free setup, gate rules file, {name}: exit {:?}, clearance violations {clearance:?}",
            out.status.code()
        ));
        counts.push(clearance.len());
    }
    assert!(
        counts[0] == 0 && counts[1] >= 1,
        "{counts:?}: the project step is not load-bearing; keep clearance_min in the setup block (D5)"
    );
}

// ---- AET-WO-1301: the placed board under schematic parity ----

fn record_1301(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1301 {line}\n").as_bytes());
}

/// `check drc --spec --schematic` on a board and schematic.
fn drc_parity(board: &Path, sch: &Path, spec: &Path) -> Value {
    let out = run(
        &[
            "check",
            "drc",
            "--board",
            board.to_str().unwrap(),
            "--spec",
            spec.to_str().unwrap(),
            "--schematic",
            sch.to_str().unwrap(),
        ],
        None,
    );
    // The CLI exits 0 on a pass and 1 on a fail; anything else is no verdict.
    let code = out.status.code().unwrap();
    assert!(
        code == 0 || code == 1,
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    let v = verdict(&out, code);
    assert_judged(&v, "drc", "kicad-cli", "10.0.7");
    assert_eq!(v["rules"], "specification", "{v}");
    v
}

/// The findings that are not `unconnected_items`, as `kind: message`.
fn beyond_unconnected(v: &Value) -> Vec<String> {
    v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] != "unconnected_items")
        .map(|f| {
            format!(
                "{}: {}",
                f["kind"].as_str().unwrap(),
                f["message"].as_str().unwrap()
            )
        })
        .collect()
}

// ---- AET-WO-1302: P1 gate clause 1 ----

fn record_1302(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1302 {line}\n").as_bytes());
}

/// P1 gate clause 1 (`AET-WO-1302` §1.4, tier 2), as one test. From the
/// checked-in reference specification, one command emits a schematic and a
/// placed and routed board; `check erc` on the schematic exits 0, and
/// `check drc --spec --schematic` on the board, judged by the
/// specification's rules with schematic parity, exits 0 with no finding of
/// any kind, unconnected items included. So does the board of 1300's S5
/// names, whose references and nets KiCad must quote. The §3.2 numbers are
/// recorded for each.
#[test]
fn p1_gate_clause_1_the_reference_board_passes_erc_and_drc_with_no_finding() {
    let t = Temp::new("clause1");
    let reference = example_spec("sentinel-01-minimal.json");
    let mut text = serde_json::to_string(
        &serde_json::from_slice::<Value>(&fs::read(&reference).unwrap()).unwrap(),
    )
    .unwrap();
    for (from, to) in [
        ("sentinel-01", "p-x"),
        ("VDD_3V3", "VDD'3V3^"),
        ("I2C_SDA", "I2C(SDA)"),
        ("FLASH_CS", "CS@(0)"),
        ("U2", "U_2"),
        ("U3", "U_3"),
        ("U4", "U_4"),
    ] {
        text = text.replace(&format!("\"{from}\""), &serde_json::to_string(to).unwrap());
    }
    let s5_path = t.0.join("s5.json");
    fs::write(&s5_path, &text).unwrap();
    for (name, spec) in [("sentinel-01", reference), ("s5-names", s5_path)] {
        let started = std::time::Instant::now();
        let r = emit_to(&spec, &t.0.join(name));
        let emit_ms = started.elapsed().as_millis();
        let board = PathBuf::from(r["board"]["path"].as_str().unwrap());
        let sch = PathBuf::from(r["schematic"]["path"].as_str().unwrap());

        let erc = run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        );
        let erc = verdict(&erc, 0);
        assert_judged(&erc, "erc", "kicad-cli", "10.0.7");
        assert_eq!(erc["verdict"], "pass", "{name}: {erc}");
        assert_eq!(kinds(&erc), Vec::<&str>::new(), "{name}: {erc}");

        let drc = drc_parity(&board, &sch, &spec);
        record_1302(&format!(
            "{name}: erc exit {}, drc exit {} with {} finding(s) {:?}",
            erc["tool"]["exit_code"],
            drc["tool"]["exit_code"],
            kinds(&drc).len(),
            kinds(&drc)
        ));
        assert_eq!(drc["tool"]["exit_code"], 0, "{name}: {drc}");
        assert_eq!(drc["verdict"], "pass", "{name}: {drc}");
        assert_eq!(kinds(&drc), Vec::<&str>::new(), "{name}: {drc}");
        assert_eq!(
            drc["schematic"]["sha256"], r["schematic"]["sha256"],
            "{name}"
        );
        assert_eq!(drc["input"]["sha256"], r["board"]["sha256"], "{name}");

        // §3.2: the numbers, from the board as written; the rip-ups from
        // routing its pads again, which lays the same copper.
        let (pads, routed) = aethelgard_eda::route::read(
            &aethelgard_eda::strict::board(&fs::read_to_string(&board).unwrap()).unwrap(),
            &aethelgard_eda::stack::Stack::of(
                &serde_json::from_slice::<aethelgard_ir::Specification>(&fs::read(&spec).unwrap())
                    .unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        let again = aethelgard_eda::route::route(&pads).unwrap();
        assert_eq!(again.segments, routed.segments, "{name}");
        assert_eq!(again.vias, routed.vias, "{name}");
        record_1302(&format!(
            "{name}: emit {emit_ms} ms (debug build, routing included); track {} um in {} segments; {} vias; {} rip-ups; zones {:?}; grid pitch {} nm",
            routed.length_nm() / 1000,
            routed.segments.len(),
            routed.vias.len(),
            again.ripups,
            routed.zones,
            aethelgard_eda::route::PITCH_NM,
        ));
    }
}

/// KiCad's own netlist of the emitted schematic is the oracle for net names:
/// every pin's net there is its pad's net on the board, and no other pad
/// carries one.
#[test]
fn every_pad_carries_the_net_kicads_netlist_gives_its_pin() {
    use std::collections::BTreeMap;
    let t = Temp::new("netlist");
    let r = emit_to(&example_spec("sentinel-01-minimal.json"), &t.0);
    let copy = t.0.join("readback.kicad_sch");
    fs::copy(r["schematic"]["path"].as_str().unwrap(), &copy).unwrap();
    let net = t.0.join("readback.net");
    let out = Command::new("kicad-cli")
        .args(["sch", "export", "netlist", "--format", "kicadsexpr", "-o"])
        .arg(&net)
        .arg(&copy)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let netlist = aethelgard_eda::sexpr::parse(&fs::read_to_string(&net).unwrap()).unwrap();
    let mut kicad: BTreeMap<(String, String), String> = BTreeMap::new();
    for n in netlist.child("nets").unwrap().children("net") {
        let name = n.child("name").and_then(|x| x.arg(1)).unwrap();
        for node in n.children("node") {
            let at = |k: &str| node.child(k).and_then(|x| x.arg(1)).unwrap().to_owned();
            kicad.insert((at("ref"), at("pin")), name.to_owned());
        }
    }
    let board = aethelgard_eda::sexpr::parse(
        &fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap(),
    )
    .unwrap();
    let mut ours: BTreeMap<(String, String), String> = BTreeMap::new();
    for fp in board.children("footprint") {
        let reference = fp.property("Reference").unwrap();
        for pad in fp.children("pad") {
            if let Some(n) = pad.child("net").and_then(|n| n.arg(1)) {
                ours.insert(
                    (reference.to_owned(), pad.arg(1).unwrap().to_owned()),
                    n.to_owned(),
                );
            }
        }
    }
    record_1301(&format!(
        "netlist oracle: {} pins in KiCad's netlist, {} netted pads on the board",
        kicad.len(),
        ours.len()
    ));
    assert_eq!(ours, kicad);
}

/// Replaces the first item of `board` that `pick` matches, in tree order.
fn mutate(board: &str, pick: &dyn Fn(&Sx, &str) -> Option<Sx>) -> String {
    fn walk(sx: &mut Sx, within: &str, pick: &dyn Fn(&Sx, &str) -> Option<Sx>) -> bool {
        let here = sx
            .property("Reference")
            .filter(|_| sx.head() == Some("footprint"))
            .map(str::to_owned)
            .unwrap_or_else(|| within.to_owned());
        if let Some(new) = pick(sx, &here) {
            *sx = new;
            return true;
        }
        match sx.items_mut() {
            Some(items) => items.iter_mut().any(|c| walk(c, &here, pick)),
            None => false,
        }
    }
    let mut root = aethelgard_eda::sexpr::parse(board).unwrap();
    assert!(walk(&mut root, "", pick), "nothing to mutate");
    aethelgard_eda::sexpr::write(&root)
}

use aethelgard_eda::sexpr::Sx;

/// The board without the router's copper: the placed board these parity
/// rows were measured on (`AET-WO-1301`). Parity does not read copper; on
/// the routed board a mismatched pad also shorts the tracks that reach it
/// (`AET-WO-1302`), which the routed check after the rows asserts.
fn unrouted(board: &str) -> String {
    let mut root = aethelgard_eda::sexpr::parse(board).unwrap();
    if let Some(items) = root.items_mut() {
        items.retain(|c| !matches!(c.head(), Some("segment" | "via" | "zone")));
    }
    aethelgard_eda::sexpr::write(&root)
}

/// §5.2 as a regression: parity fails a board that does not match its
/// schematic, each way it can differ, with exactly the finding KiCad names
/// for it. The last row is ruling (a)'s ground: the exposed pad on GND,
/// which has no symbol pin, is a parity finding.
#[test]
fn parity_fails_a_board_that_does_not_match_its_schematic() {
    let t = Temp::new("mismatch");
    let spec = example_spec("sentinel-01-minimal.json");
    let r = emit_to(&spec, &t.0);
    let routed = fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap();
    let board = unrouted(&routed);
    let sch_text = fs::read_to_string(r["schematic"]["path"].as_str().unwrap()).unwrap();
    let property = |name: &'static str, value: Option<&'static str>| {
        move |sx: &Sx, fp: &str| -> Option<Sx> {
            if fp != "C1" || sx.head() != Some("footprint") {
                return None;
            }
            let mut out = sx.clone();
            let items = out.items_mut()?;
            let i = items
                .iter()
                .position(|c| c.head() == Some("property") && c.arg(1) == Some(name))?;
            match value {
                Some(v) => items[i].items_mut()?[2] = Sx::str(v),
                None => {
                    items.remove(i);
                }
            }
            Some(out)
        }
    };
    let renamed = |sx: &Sx, _: &str| -> Option<Sx> {
        (sx.head() == Some("net") && sx.arg(1) == Some("/I2C_SDA"))
            .then(|| Sx::List(vec![Sx::atom("net"), Sx::str("I2C_SDA")]))
    };
    let grounded_ep = |sx: &Sx, fp: &str| -> Option<Sx> {
        if fp != "U3" || sx.head() != Some("pad") || sx.arg(1) != Some("7") {
            return None;
        }
        let mut out = sx.clone();
        out.items_mut()?
            .push(Sx::List(vec![Sx::atom("net"), Sx::str("GND")]));
        Some(out)
    };
    let rows: [(&str, String, String, &str); 5] = [
        (
            "value swapped",
            mutate(&board, &property("Value", Some("2u3"))),
            sch_text.clone(),
            "footprint_symbol_mismatch",
        ),
        (
            "field missing",
            mutate(&board, &property("ki_keywords", None)),
            sch_text.clone(),
            "footprint_symbol_field_mismatch",
        ),
        (
            "board net renamed",
            mutate(&board, &renamed),
            sch_text.clone(),
            "net_conflict",
        ),
        (
            "schematic net renamed",
            board.clone(),
            sch_text.replace("(label \"I2C_SDA\"", "(label \"I2C_SDX\""),
            "net_conflict",
        ),
        (
            "exposed pad on GND",
            mutate(&board, &grounded_ep),
            sch_text.clone(),
            "net_conflict",
        ),
    ];
    for (i, (name, b, s, kind)) in rows.into_iter().enumerate() {
        let dir = t.0.join(format!("row{i}"));
        fs::create_dir_all(&dir).unwrap();
        let (bp, sp) = (dir.join("b.kicad_pcb"), dir.join("s.kicad_sch"));
        fs::write(&bp, &b).unwrap();
        fs::write(&sp, &s).unwrap();
        let v = drc_parity(&bp, &sp, &spec);
        let found = beyond_unconnected(&v);
        record_1301(&format!("mismatch row {name}: {found:?}"));
        assert!(
            !found.is_empty(),
            "{name}: parity passed a mismatched board"
        );
        assert!(
            found.iter().all(|f| f.starts_with(&format!("{kind}:"))),
            "{name}: {found:?}"
        );
        assert_eq!(v["verdict"], "fail");
    }
    // The routed board fails the same way, its copper adding findings.
    let dir = t.0.join("routed");
    fs::create_dir_all(&dir).unwrap();
    let (bp, sp) = (dir.join("b.kicad_pcb"), dir.join("s.kicad_sch"));
    fs::write(&bp, mutate(&routed, &renamed)).unwrap();
    fs::write(&sp, &sch_text).unwrap();
    let v = drc_parity(&bp, &sp, &spec);
    let found = beyond_unconnected(&v);
    record_1301(&format!(
        "mismatch row board net renamed, routed: {found:?}"
    ));
    assert!(
        found.iter().any(|f| f.starts_with("net_conflict:")),
        "{found:?}"
    );
    assert_eq!(v["verdict"], "fail");
}

// ---- AET-WO-1307: escape planning, tier 2 ----

/// `AET-WO-1307` §2.1 and §4, ruled K4 ("every gate" is DRC with parity) and
/// K5: `D3` (24 I2C devices and 12 SPI flashes) and `D1` route at 4 layers
/// under the inward escape, vias under the LQFP's own body included, and each
/// board's schematic passes ERC and the board passes `check drc --spec
/// --schematic` with no finding of any kind.
#[test]
fn d3_and_d1_pass_erc_and_drc_with_parity_at_four_layers() {
    let t = Temp::new("1307-dense");
    for name in ["d3-24-i2c-12-spi.json", "d1-24-i2c.json"] {
        let spec = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let r = emit_to(&spec, &t.0.join(name));
        let board = PathBuf::from(r["board"]["path"].as_str().unwrap());
        let sch = PathBuf::from(r["schematic"]["path"].as_str().unwrap());
        let erc = run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        );
        let erc = verdict(&erc, 0);
        assert_eq!(kinds(&erc), Vec::<&str>::new(), "{name}: {erc}");
        let drc = drc_parity(&board, &sch, &spec);
        assert_eq!(drc["verdict"], "pass", "{name}: {drc}");
        assert_eq!(kinds(&drc), Vec::<&str>::new(), "{name}: {drc}");
        assert_eq!(drc["input"]["sha256"], r["board"]["sha256"], "{name}");
    }
}

// ---- AET-WO-1308: router classes, coupled pairs and rotation, tier 2 ----

fn record_1308(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1308 {line}\n").as_bytes());
}

/// `AET-WO-1308` §1.6: the 4-layer `power` fixture and the stand-in pair
/// fixture each pass ERC, and `check drc --spec --schematic` with no finding
/// of any kind, judged by the gate's per-class rules; the pair's verdict
/// reports its length difference and each conductor's vias, from copper.
#[test]
fn the_class_and_pair_fixtures_pass_erc_and_drc_and_the_pair_is_reported() {
    let t = Temp::new("1308-fixtures");
    for name in ["four-layer-power.json", "pair-standin.json"] {
        let spec = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name);
        let r = emit_to(&spec, &t.0.join(name));
        let board = PathBuf::from(r["board"]["path"].as_str().unwrap());
        let sch = PathBuf::from(r["schematic"]["path"].as_str().unwrap());
        let erc = run(
            &["check", "erc", "--schematic", sch.to_str().unwrap()],
            None,
        );
        let erc = verdict(&erc, 0);
        assert_eq!(kinds(&erc), Vec::<&str>::new(), "{name}: {erc}");
        let drc = drc_parity(&board, &sch, &spec);
        assert_eq!(drc["verdict"], "pass", "{name}: {drc}");
        assert_eq!(kinds(&drc), Vec::<&str>::new(), "{name}: {drc}");
        record_1308(&format!("{name}: drc pass, pairs {}", drc["pairs"]));
        if name == "pair-standin.json" {
            let p = &drc["pairs"][0];
            assert_eq!(p["name"], "USB_STANDIN", "{drc}");
            assert_eq!(p["vias"], serde_json::json!([0, 0]), "{drc}");
            assert!(
                p["length_difference_nm"].as_i64().unwrap()
                    <= p["length_difference_max_nm"].as_i64().unwrap(),
                "{drc}"
            );
        } else {
            assert!(drc.get("pairs").is_none(), "{drc}");
        }
    }
}

/// `AET-WO-1308` §5.1: a board whose track is not its class's width, either
/// way, fails DRC by the gate's own class rule, never the board's.
#[test]
fn a_track_that_is_not_its_class_width_fails_drc_by_the_gates_rule() {
    let t = Temp::new("1308-lie");
    let spec = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/four-layer-power.json");
    let r = emit_to(&spec, &t.0.join("emit"));
    let text = fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap();
    let i = text.find("(width 0.25)").unwrap();
    for (to, bound) in [
        ("(width 0.2)", "min width 0.2500 mm"),
        ("(width 0.3)", "max width 0.2500 mm"),
    ] {
        let lied = format!("{}{to}{}", &text[..i], &text[i + "(width 0.25)".len()..]);
        let board = t.0.join(format!("lie-{}.kicad_pcb", &to[7..10]));
        fs::write(&board, lied).unwrap();
        let out = run(
            &[
                "check",
                "drc",
                "--board",
                board.to_str().unwrap(),
                "--spec",
                spec.to_str().unwrap(),
            ],
            None,
        );
        let v = verdict(&out, 1);
        let widths: Vec<&Value> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["kind"] == "track_width")
            .collect();
        assert_eq!(widths.len(), 1, "{v}");
        let m = widths[0]["message"].as_str().unwrap();
        assert!(
            m.contains("class_power_track_width") && m.contains(bound),
            "{m}"
        );
        record_1308(&format!("lie {to}: {m}"));
    }
}

/// `AET-WO-1308` §1.5: a part on each board edge, at each of the four right
/// angles, routes, and its schematic passes ERC and its board `check drc
/// --spec --schematic` with no finding of any kind.
#[test]
fn a_part_on_each_edge_passes_erc_and_drc() {
    let t = Temp::new("1308-edges");
    let (spec, emitted) = common::edges::moved();
    let spec_path = t.0.join("spec.json");
    fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
    let board = t.0.join(format!("{}.kicad_pcb", spec.project_name));
    let sch = t.0.join(format!("{}.kicad_sch", spec.project_name));
    fs::write(&board, &emitted.board).unwrap();
    fs::write(&sch, &emitted.schematic).unwrap();
    let erc = run(
        &["check", "erc", "--schematic", sch.to_str().unwrap()],
        None,
    );
    let erc = verdict(&erc, 0);
    assert_eq!(kinds(&erc), Vec::<&str>::new(), "{erc}");
    let drc = drc_parity(&board, &sch, &spec_path);
    assert_eq!(drc["verdict"], "pass", "{drc}");
    assert_eq!(kinds(&drc), Vec::<&str>::new(), "{drc}");
    record_1308("edges: erc and drc pass");
}

// ---- AET-WO-1309: through-hole pads and thermal-via arrays, tier 2 ----

fn record_1309(line: &str) {
    use std::io::Write;
    let _ = std::io::stderr()
        .lock()
        .write_all(format!("AET-WO-1309 {line}\n").as_bytes());
}

/// KiCad's DRC report on `text`, prepared exactly as `check drc` prepares
/// its scratch copy (`kicad.rs`): the strict reading and the gate's setup,
/// the gate's project and `.kicad_dru` from `rules`, the vendored libraries
/// and their tables, and `--refill-zones`. The rules come from a declared
/// board (`Rules::from_stack`), because no specification can yet declare a
/// thermal array or a through-hole join (FEEDBACK D1).
fn gate_drc(dir: &Path, text: &str, rules: &aethelgard_eda::rules::Rules) -> (Option<i32>, Value) {
    fs::create_dir_all(dir).unwrap();
    let board = dir.join("th.kicad_pcb");
    let tree = aethelgard_eda::board::read(text).unwrap();
    let prepared =
        aethelgard_eda::board::prepare_read(&tree, text, aethelgard_eda::rules::SETUP).unwrap();
    fs::write(&board, prepared).unwrap();
    fs::write(dir.join("th.kicad_pro"), rules.project("th")).unwrap();
    fs::write(dir.join("th.kicad_dru"), rules.custom_rules()).unwrap();
    for (path, bytes) in aethelgard_eda::library::files().unwrap() {
        let p = dir.join("lib").join(path);
        fs::create_dir_all(p.parent().unwrap()).unwrap();
        fs::write(p, bytes).unwrap();
    }
    let (sym, fp) = aethelgard_eda::library::tables(dir.join("lib").to_str().unwrap()).unwrap();
    fs::write(dir.join("sym-lib-table"), sym).unwrap();
    fs::write(dir.join("fp-lib-table"), fp).unwrap();
    let report = dir.join("report.json");
    let out = Command::new("kicad-cli")
        .args([
            "pcb",
            "drc",
            "--exit-code-violations",
            "--format",
            "json",
            "--refill-zones",
            "--save-board",
            "-o",
        ])
        .arg(&report)
        .arg(&board)
        .current_dir(dir)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let doc: Value = serde_json::from_slice(&fs::read(&report).unwrap()).unwrap();
    (out.status.code(), doc)
}

/// The radii, from `centre`, of every vertex of `net`'s fill on `layer` in
/// a board KiCad saved after refilling it, within `within` nm.
fn fill_radii(board: &str, layer: &str, net: &str, centre: (i64, i64), within: i64) -> Vec<i64> {
    let tree = aethelgard_eda::sexpr::parse(board).unwrap();
    let mut out = Vec::new();
    for zone in tree.children("zone") {
        if zone.child("net").and_then(|n| n.arg(1)) != Some(net)
            || zone.child("layer").and_then(|l| l.arg(1)) != Some(layer)
        {
            continue;
        }
        for poly in zone.children("filled_polygon") {
            for xy in poly.child("pts").unwrap().children("xy") {
                let (x, y) = (
                    aethelgard_eda::sexpr::nm(xy.arg(1).unwrap()).unwrap(),
                    aethelgard_eda::sexpr::nm(xy.arg(2).unwrap()).unwrap(),
                );
                let r = (((x - centre.0) as f64).hypot((y - centre.1) as f64)).round() as i64;
                if r < within {
                    out.push(r);
                }
            }
        }
    }
    out
}

/// Every DRC item of a report: violations, unconnected items and parity, as
/// `type: description`.
fn drc_items(doc: &Value) -> Vec<String> {
    ["violations", "unconnected_items", "schematic_parity"]
        .iter()
        .flat_map(|k| doc[*k].as_array().cloned().unwrap_or_default())
        .map(|f| {
            format!(
                "{}: {}",
                f["type"].as_str().unwrap_or("?"),
                f["description"].as_str().unwrap_or("?")
            )
        })
        .collect()
}

/// `AET-WO-1309` §1.3: the XT30 fixture at 2 and 4 layers with each join,
/// and the thermal fixture's 4 x 3 array into the 4-layer ground plane (and
/// into the 2-layer ground zone), pass KiCad DRC by the gate's own rules
/// with no finding of any kind, unconnected items included. And the fill
/// KiCad refills meets J1's `GND` pad as declared (FEEDBACK B2): relieved,
/// its edge is the 0.5 mm relief gap from the 3.5 mm pad (2.25 mm from its
/// centre); solid, no edge of the fill lies between the hole and 2.6 mm.
#[test]
fn the_through_hole_and_thermal_fixtures_pass_drc_by_the_gates_rules() {
    use aethelgard_eda::stack::Join;
    let t = Temp::new("1309-fixtures");
    let mut boards = Vec::new();
    for four in [false, true] {
        for join in [Join::Solid, Join::Relief] {
            boards.push((
                format!("xt30-{four}-{join:?}"),
                common::th::xt30_board(four, join),
            ));
        }
        let layer = if four { "In1.Cu" } else { "B.Cu" };
        boards.push((
            format!("thermal-{four}"),
            common::th::thermal_board(four, Some(common::th::lmr33630_rule(&[layer])), false),
        ));
    }
    for (name, fixture) in boards {
        let (stack, text) = common::th::write(fixture).unwrap();
        let rules = aethelgard_eda::rules::Rules::from_stack(&stack);
        let (code, doc) = gate_drc(&t.0.join(&name), &text, &rules);
        let items = drc_items(&doc);
        record_1309(&format!("{name}: exit {code:?}, {items:?}"));
        assert_eq!(code, Some(0), "{name}: {items:?}");
        assert!(items.is_empty(), "{name}: {items:?}");
        if let Some(join) = name.strip_prefix("xt30-") {
            let saved = fs::read_to_string(t.0.join(&name).join("th.kicad_pcb")).unwrap();
            let layer = if join.starts_with("true") {
                "In1.Cu"
            } else {
                "B.Cu"
            };
            // J1 at (110, 116); pad 2, 1.9 mm drill, at x -5.
            let radii = fill_radii(&saved, layer, "GND", (105_000_000, 116_000_000), 2_600_000);
            let ring: Vec<i64> = radii.iter().copied().filter(|&r| r > 1_000_000).collect();
            record_1309(&format!(
                "{name}: fill vertices 1.0-2.6 mm from J1 pad 2: {}",
                ring.len()
            ));
            if join.ends_with("Relief") {
                assert!(
                    !ring.is_empty() && ring.iter().all(|r| (2_240_000..=2_260_000).contains(r)),
                    "{name}: {ring:?}"
                );
            } else {
                assert!(ring.is_empty(), "{name}: {ring:?}");
            }
        }
    }
}

/// `AET-WO-4300d2` §1.6: the drill files `through_hole_tier1.rs` pins are
/// what this image's KiCad writes. Each fixture board, prepared with the
/// gate's setup as `fab` prepares its copy, is exported with `fab`'s own
/// flags (`pcb export drill --excellon-min-header`), into one file whose
/// bytes are the pinned file's, and which holds exactly the tools
/// `fab::drills` derives from the board.
#[test]
fn kicad_writes_the_pinned_drill_files() {
    use aethelgard_eda::stack::Join;
    let t = Temp::new("4300d2-drill");
    let mut own_vias = common::th::thermal_board(false, None, true);
    own_vias.0.joins = vec![("U5".to_owned(), Join::Solid)];
    let boards = [
        ("xt30-4", common::th::xt30_board(true, Join::Solid)),
        (
            "thermal-4",
            common::th::thermal_board(true, Some(common::th::lmr33630_rule(&["In1.Cu"])), false),
        ),
        ("via-and-pad-0.3", own_vias),
    ];
    for (name, fixture) in boards {
        let (stack, text) = common::th::write(fixture).unwrap();
        let dir = t.0.join(name);
        fs::create_dir_all(&dir).unwrap();
        let tree = aethelgard_eda::board::read(&text).unwrap();
        let prepared =
            aethelgard_eda::board::prepare_read(&tree, &text, aethelgard_eda::rules::SETUP)
                .unwrap();
        fs::write(dir.join("input.kicad_pcb"), prepared).unwrap();
        let out = Command::new("kicad-cli")
            .args(["pcb", "export", "drill", "--excellon-min-header", "-o"])
            .arg(format!("{}/", dir.display()))
            .arg(dir.join("input.kicad_pcb"))
            .current_dir(&dir)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert_eq!(out.status.code(), Some(0), "{name}: {out:?}");
        let mut written: Vec<String> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .filter(|n| n.ends_with(".drl"))
            .collect();
        written.sort();
        assert_eq!(written, ["input.drl"], "{name}: one drill file");
        let drl = fs::read(dir.join("input.drl")).unwrap();
        let pinned = fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join(format!("tests/fixtures/fab/kicad-10.0.7-{name}.drl")),
        )
        .unwrap();
        record(&format!("4300d2 {name}: drill file {} bytes", drl.len()));
        assert_eq!(
            String::from_utf8_lossy(&drl),
            String::from_utf8_lossy(&pinned),
            "{name}"
        );
        let strict = aethelgard_eda::strict::board(&text).unwrap();
        let (board, routed) = aethelgard_eda::route::read(&strict, &stack).unwrap();
        let drills = aethelgard_eda::fab::drills(&board, &routed).unwrap();
        assert_eq!(aethelgard_eda::fab::holes(&drl, &drills), Ok(()), "{name}");
    }
}

/// `AET-WO-1309` §2: a thermal via drilled other than its rule fails KiCad
/// DRC by the gate's `thermal_U5_9` rule (KiCad's kind is `drill_out_of_range`);
/// and the board's own 0.254 mm
/// drill, judged with no array declared, is measured against KiCad's board
/// minimum (FEEDBACK P3).
#[test]
fn a_thermal_via_other_than_its_rule_fails_drc_by_the_gates_rule() {
    let t = Temp::new("1309-thermal-lie");
    let (stack, text) = common::th::write(common::th::thermal_board(
        true,
        Some(common::th::lmr33630_rule(&["In1.Cu"])),
        false,
    ))
    .unwrap();
    let rules = aethelgard_eda::rules::Rules::from_stack(&stack);
    let i = text.find("(drill 0.254)").unwrap();
    let lied = format!(
        "{}(drill 0.3){}",
        &text[..i],
        &text[i + "(drill 0.254)".len()..]
    );
    let (code, doc) = gate_drc(&t.0.join("lie"), &lied, &rules);
    let items = drc_items(&doc);
    record_1309(&format!(
        "one thermal via drilled 0.3: exit {code:?}, {items:?}"
    ));
    assert_eq!(code, Some(5), "{items:?}");
    assert!(
        items.iter().any(|i| i.starts_with("drill_out_of_range")
            && i.contains("rule 'thermal_U5_9' max hole 0.2540 mm")),
        "{items:?}"
    );
    let mut none = rules.clone();
    none.arrays.clear();
    let (code, doc) = gate_drc(&t.0.join("unruled"), &text, &none);
    record_1309(&format!(
        "the declared array with no thermal rule written: exit {code:?}, {:?}",
        drc_items(&doc)
    ));
}

// ---- `AET-WO-1311`: a seeded neck in the ground zone, against KiCad's refill ----

/// A 2-layer board, 20 × 10 mm, whose B.Cu ground zone joins its two ground
/// vias (x = 103.5 and 115.5 mm) only through a neck at y = 105 mm: `/A`'s
/// B.Cu walls between x = 110 and 111 mm run from the vias at each edge to
/// the neck, the two `gap_um` apart copper to copper, joined on F.Cu. Every coordinate
/// is in the engine's own grammar and on its 125 µm grid but the walls' ends.
fn necked_board(gap_um: i64) -> String {
    let mm = |um: i64| format!("{}.{:03}", um / 1000, um % 1000);
    let top = mm(105_000 - gap_um / 2 - 100);
    let bottom = mm(105_000 + (gap_um + 1) / 2 + 100);
    assert_eq!(gap_um % 2, 0, "a gap centred on the grid row");
    let part = |r: &str, x: f64, y: f64, n1: &str, n2: &str| {
        format!(
            "\t(footprint \"R\" (layer \"F.Cu\") (at {x} {y}) (property \"Reference\" \"{r}\" (at 0 -1) (layer \"F.Fab\")) (property \"Value\" \"10k\" (at 0 1) (layer \"F.Fab\"))\n\t\t(pad \"1\" smd rect (at -0.5 0) (size 0.5 0.6) (layers \"F.Cu\" \"F.Mask\" \"F.Paste\") (net \"{n1}\"))\n\t\t(pad \"2\" smd rect (at 0.5 0) (size 0.5 0.6) (layers \"F.Cu\" \"F.Mask\" \"F.Paste\") (net \"{n2}\"))\n\t\t(fp_rect (start -0.9 -0.45) (end 0.9 0.45) (stroke (width 0.05) (type default)) (fill no) (layer \"F.CrtYd\")))\n"
        )
    };
    let seg = |a: (&str, &str), b: (&str, &str), layer: &str, net: &str| {
        format!(
            "\t(segment (start {} {}) (end {} {}) (width 0.2) (layer \"{layer}\") (net \"{net}\"))\n",
            a.0, a.1, b.0, b.1
        )
    };
    let via = |x: &str, y: &str, net: &str| {
        format!(
            "\t(via (at {x} {y}) (size 0.6) (drill 0.3) (layers \"F.Cu\" \"B.Cu\") (net \"{net}\"))\n"
        )
    };
    let mut text = String::from(
        "(kicad_pcb\n\t(version 20260206)\n\t(generator \"aethelgard\")\n\t(generator_version \"0.1.0\")\n\t(general (thickness 1.6) (legacy_teardrops no))\n\t(paper \"A4\")\n\t(layers (0 \"F.Cu\" signal) (2 \"B.Cu\" signal) (13 \"F.Paste\" user) (1 \"F.Mask\" user) (25 \"Edge.Cuts\" user) (31 \"F.CrtYd\" user \"F.Courtyard\") (35 \"F.Fab\" user))\n",
    );
    text += &part("R1", 104.0, 103.0, "GND", "/L");
    text += &part("R2", 116.0, 103.0, "GND", "/R");
    text += &part("R3", 110.5, 103.0, "/A", "/A");
    text += "\t(gr_rect (start 100 100) (end 120 110) (stroke (width 0.05) (type default)) (fill no) (layer \"Edge.Cuts\"))\n";
    // Ground: each pad to its via on F.Cu; the zone joins the vias.
    text += &seg(("103.5", "103"), ("103.5", "105"), "F.Cu", "GND");
    text += &seg(("115.5", "103"), ("115.5", "105"), "F.Cu", "GND");
    text += &via("103.5", "105", "GND");
    text += &via("115.5", "105", "GND");
    // `/A`: R3's pads (x = 110 and 111 mm) each on an F.Cu spine to a via
    // at each edge; on B.Cu a U from each edge's two vias to the neck, so
    // the neck is a channel 1 mm long between the two U's bottoms.
    for x in ["110", "111"] {
        text += &seg((x, "100.8"), (x, "103"), "F.Cu", "/A");
        text += &seg((x, "103"), (x, "109.2"), "F.Cu", "/A");
        text += &via(x, "100.8", "/A");
        text += &via(x, "109.2", "/A");
        text += &seg((x, "100.8"), (x, &top), "B.Cu", "/A");
        text += &seg((x, &bottom), (x, "109.2"), "B.Cu", "/A");
    }
    text += &seg(("110", &top), ("111", &top), "B.Cu", "/A");
    text += &seg(("110", &bottom), ("111", &bottom), "B.Cu", "/A");
    text += "\t(zone (net \"GND\") (layer \"B.Cu\") (hatch edge 0.5) (min_thickness 0.25) (filled_areas_thickness no) (fill (thermal_gap 0.5) (thermal_bridge_width 0.5) (island_removal_mode 0)) (polygon (pts (xy 100 100) (xy 120 100) (xy 120 110) (xy 100 110))))\n";
    text += "\t(embedded_fonts no)\n)\n";
    text
}

/// The Grok review's row 3 residual (`AET-WO-1311`): the router's and
/// `extract`'s zone judge (`route::connectivity`, on `zone_core`) and
/// KiCad 10.0.7's own refill (`check drc`, `--refill-zones`) on one seeded
/// neck, across a sweep of widths. Measured: the judge joins the zone from
/// 670 µm (a neck one grid node wide: clearance 0.2 mm + min_thickness/2 +
/// 10 µm from each wall), KiCad from 650 µm (2 × 0.2 + 0.25 mm); at 640 µm
/// KiCad reports the vias unconnected. Wherever the judge joins the zone,
/// KiCad's refill joins it too, never the false pass the review feared;
/// between the two the judge fails closed.
#[test]
fn a_seeded_neck_is_joined_by_kicads_refill_wherever_the_judge_joins_it() {
    let scratch = common::Scratch::new("1311-neck");
    let spec: aethelgard_ir::Specification = serde_json::from_str(
        &fs::read_to_string(example_spec("sentinel-01-minimal.json")).unwrap(),
    )
    .unwrap();
    let stack = aethelgard_eda::stack::Stack::of(&spec).unwrap();
    for (gap, judged, kicad) in [
        (600, false, false),
        (640, false, false),
        (650, false, true),
        (660, false, true),
        (670, true, true),
        (680, true, true),
        (800, true, true),
    ] {
        let text = necked_board(gap);
        let tree = aethelgard_eda::strict::board(&text).unwrap();
        let (b, r) = aethelgard_eda::route::read(&tree, &stack).unwrap();
        let judge = aethelgard_eda::route::connectivity(&b, &r);
        assert_eq!(judge.is_ok(), judged, "{gap} µm: {judge:?}");
        if let Err(e) = judge {
            assert!(e.contains("the zone does not reach via"), "{gap} µm: {e}");
        }
        let path = scratch.0.join(format!("neck-{gap}.kicad_pcb"));
        fs::write(&path, &text).unwrap();
        let out = run(&["check", "drc", "--board", path.to_str().unwrap()], None);
        if kicad {
            let v = verdict(&out, 0);
            assert_eq!(v["verdict"], "pass", "{gap} µm: {v}");
        } else {
            let v = verdict(&out, 1);
            assert_eq!(kinds(&v), ["unconnected_items"], "{gap} µm: {v}");
        }
        assert!(
            !judged || kicad,
            "{gap} µm: the judge joined what KiCad cut"
        );
    }
}

// ---- AET-WO-4400d: a turned IMU and its keep-out, tier 2 ----

/// `AET-WO-4400d` D2 (ruled on PR #207): a footprint's keep-out zones turn
/// with it by the same `route::rotate` its pads do. This test holds what the
/// refusal it replaced guarded: `sentinel-02-own` with U5, the ICM-42688-P
/// and its two keep-outs, declared so that the placer turns it 90, 180 and
/// 270 degrees, routes, and its board passes `check drc --spec --schematic`
/// with no finding of any kind: KiCad holds each footprint to its library
/// (`lib_footprint_mismatch`, which caught an unshifted keep-out in
/// `AET-WO-1310` D4), each keep-out's rules to the copper, and the board to
/// its schematic.
#[test]
fn a_turned_keepout_is_where_kicad_draws_it() {
    let fixture: serde_json::Value = serde_json::from_slice(
        &fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/sentinel-02-own.json"),
        )
        .unwrap(),
    )
    .unwrap();
    for (x, theta) in [("forward", "90"), ("left", "180"), ("aft", "270")] {
        let t = Temp::new(&format!("4400d-turned-{theta}"));
        let mut v = fixture.clone();
        assert_eq!(v["board"]["forward"], "top");
        for d in v["spi_devices"].as_array_mut().unwrap() {
            if d["reference"] == "U5" {
                d["orientation"] = serde_json::json!({"x": x, "z": "up"});
            }
        }
        let spec: aethelgard_ir::Specification = serde_json::from_value(v).unwrap();
        let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
        let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
        let emitted = aethelgard_eda::schematic::emit(&spec, &solved, &table).unwrap();
        let u5 = emitted
            .board
            .split("\n\t(footprint")
            .find(|f| f.contains("(property \"Reference\" \"U5\""))
            .unwrap();
        assert!(u5.contains(&format!(" {theta})")), "U5 not at {theta}");
        assert_eq!(u5.matches("(zone").count(), 2, "U5's two keep-outs");
        let spec_path = t.0.join("spec.json");
        fs::write(&spec_path, serde_json::to_vec(&spec).unwrap()).unwrap();
        let board = t.0.join(format!("{}.kicad_pcb", spec.project_name));
        let sch = t.0.join(format!("{}.kicad_sch", spec.project_name));
        fs::write(&board, &emitted.board).unwrap();
        fs::write(&sch, &emitted.schematic).unwrap();
        let drc = drc_parity(&board, &sch, &spec_path);
        assert_eq!(drc["verdict"], "pass", "{theta}: {drc}");
        assert_eq!(kinds(&drc), Vec::<&str>::new(), "{theta}: {drc}");
    }
}
