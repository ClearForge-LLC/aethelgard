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

/// Every character `RUL-14` still admits that an S-expression must quote,
/// and others, in every identifier class that reaches the schematic.
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
        ("sentinel-01", "p\"(x)\\#"),
        ("VDD_3V3", "VDD\"3V3\\"),
        ("I2C_SDA", "I2C(SDA)"),
        ("I2C_SCL", "I2C;SCL~"),
        ("SPI_SCK", "SPI'SCK^"),
        ("SPI_MOSI", "MO|SI,"),
        ("SPI_MISO", "MI*SO?"),
        ("FLASH_CS", "CS\"(0)"),
        ("U2", "U(2)"),
        ("U3", "U\"3"),
        ("U4", "U\\4"),
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
    assert_eq!(title, Some("p\"(x)\\#"));
    assert!(nets.contains(&"VDD\"3V3\\"), "{nets:?}");
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

#[test]
fn a_board_whose_own_files_are_looser_is_judged_by_the_gate() {
    let t = Temp::new("loose");
    let board = t.0.join("loose.kicad_pcb");
    fs::write(&board, LOOSE).unwrap();
    let loose = aethelgard_eda::rules::Rules {
        source: aethelgard_eda::rules::Source::Specification,
        clearance_nm: 50_000,
        min_track_width_nm: None,
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
    let gate = aethelgard_eda::rules::Rules::from_specification(
        &serde_json::from_slice(&fs::read(example_spec("sentinel-01-minimal.json")).unwrap())
            .unwrap(),
    );
    let own = aethelgard_eda::rules::Rules {
        source: aethelgard_eda::rules::Source::Specification,
        clearance_nm: 50_000,
        min_track_width_nm: None,
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
        ("sentinel-01", "p\"(x)\\#"),
        ("VDD_3V3", "VDD\"3V3\\"),
        ("I2C_SDA", "I2C(SDA)"),
        ("FLASH_CS", "CS\"(0)"),
        ("U2", "U(2)"),
        ("U3", "U\"3"),
        ("U4", "U\\4"),
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
            &aethelgard_eda::sexpr::parse(&fs::read_to_string(&board).unwrap()).unwrap(),
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
