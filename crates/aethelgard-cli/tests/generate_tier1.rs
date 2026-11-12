//! `AET-WO-3301`, tier 1: `generate`, and every refusal of `build`, `verify`
//! and `prove --spec` that comes before any tool is looked for. Each such
//! call runs with an empty `PATH`, so a refusal that waited for a tool would
//! show as exit 4 instead. The gates themselves are tier 2
//! (`board_tier2.rs`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;
use sha2::{Digest, Sha256};

const SPEC: &str = "examples/sentinel-01-minimal.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-3301-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

/// Runs the binary from the repository root; `no_tools` empties `PATH`.
fn run(args: &[&str], no_tools: bool) -> (Option<i32>, Value) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.args(args).current_dir(root()).stdin(Stdio::null());
    if no_tools {
        cmd.env("PATH", "");
    }
    let out = cmd.output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout}");
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// Every regular file under `dir`, relative, with its bytes.
fn tree(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for entry in fs::read_dir(&d).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
            } else {
                let rel = path.strip_prefix(dir).unwrap().to_str().unwrap().to_owned();
                out.insert(rel, fs::read(&path).unwrap());
            }
        }
    }
    out
}

fn sha(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// `boards/sentinel-01` as `board/` and `emit`'s three files as `kicad/`:
/// the tree `generate` writes for the reference, assembled from what is
/// committed and what `emit` writes.
fn reference(label: &str) -> PathBuf {
    let dir = scratch(label);
    for (rel, bytes) in tree(&root().join("boards/sentinel-01")) {
        let to = dir.join("board").join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(to, bytes).unwrap();
    }
    let (code, v) = run(
        &["emit", "--file", SPEC, "--out", s(&dir.join("kicad"))],
        true,
    );
    assert_eq!(code, Some(0), "{v}");
    dir
}

/// §3.1, tier 1: the reference reproduces `boards/sentinel-01/**` and
/// `emit`'s three files, byte for byte, and the report names each file's
/// hash. Nothing else is written.
#[test]
fn generate_reproduces_the_committed_board_and_emits_files_byte_for_byte() {
    let out = scratch("generate");
    let (code, v) = run(&["generate", "--spec", SPEC, "--out", s(&out)], true);
    assert_eq!(code, Some(0), "{v}");
    let written = tree(&out);
    let want = tree(&reference("generate-ref"));
    assert_eq!(
        written.keys().collect::<Vec<_>>(),
        want.keys().collect::<Vec<_>>()
    );
    for (path, bytes) in &want {
        assert!(written[path] == *bytes, "{path} differs");
    }
    let named: BTreeMap<String, String> = v["files"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_owned()))
        .collect();
    let hashed: BTreeMap<String, String> =
        written.iter().map(|(k, b)| (k.clone(), sha(b))).collect();
    assert_eq!(named, hashed);
    assert_eq!(
        v["specification_sha256"],
        sha(&fs::read(root().join(SPEC)).unwrap())
    );
    // The topology's hash is the one `campaign` records: of `solve`'s line.
    let solved = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["solve", "--file", SPEC])
        .current_dir(root())
        .output()
        .unwrap();
    let line = String::from_utf8(solved.stdout).unwrap();
    assert_eq!(v["topology_sha256"], sha(line.trim_end().as_bytes()));
}

#[test]
fn two_generations_are_identical_and_a_used_directory_is_refused_untouched() {
    let (a, b) = (scratch("twice-a"), scratch("twice-b"));
    let (_, va) = run(&["generate", "--spec", SPEC, "--out", s(&a)], true);
    let (_, vb) = run(&["generate", "--spec", SPEC, "--out", s(&b)], true);
    assert_eq!(va, vb);
    assert_eq!(tree(&a), tree(&b));

    // Into a directory that is not empty: refused, nothing written or changed.
    let before = tree(&a);
    let (code, v) = run(&["generate", "--spec", SPEC, "--out", s(&a)], true);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "UnwritableOutput");
    assert_eq!(tree(&a), before);

    // Through a symlink to an empty directory: refused, nothing written there.
    let target = scratch("twice-target");
    fs::create_dir_all(&target).unwrap();
    let link = scratch("twice-link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    let (code, v) = run(&["generate", "--spec", SPEC, "--out", s(&link)], true);
    assert_eq!(code, Some(3), "{v}");
    assert!(tree(&target).is_empty());

    // Onto a file: refused, the file unchanged.
    let file = scratch("twice-file");
    fs::write(&file, "keep").unwrap();
    let (code, _) = run(&["generate", "--spec", SPEC, "--out", s(&file)], true);
    assert_eq!(code, Some(3));
    assert_eq!(fs::read(&file).unwrap(), b"keep");
}

#[test]
fn generate_refuses_what_solve_refuses_and_writes_nothing() {
    let dir = scratch("refuse");
    fs::create_dir_all(&dir).unwrap();
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    spec["voltage_domains"][0]["nominal_millivolts"] = 0.into();
    let bad = dir.join("bad.json");
    fs::write(&bad, spec.to_string()).unwrap();
    let out = dir.join("out");
    let (code, v) = run(&["generate", "--spec", s(&bad), "--out", s(&out)], true);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "ZeroNominalVoltage");
    assert!(!out.exists());
    let (code, v) = run(
        &[
            "generate",
            "--spec",
            s(&dir.join("absent.json")),
            "--out",
            s(&out),
        ],
        true,
    );
    assert_eq!(code, Some(3), "{v}");
    assert!(!out.exists());
}

/// §3.3 and §3.4, the half before any tool: `--against` names every
/// differing, missing and extra file, and no gate runs.
#[test]
fn verify_against_fails_on_one_changed_byte_in_any_file_naming_it() {
    let dir = reference("against");
    let files: Vec<String> = tree(&dir).into_keys().collect();
    assert_eq!(files.len(), 14, "{files:?}");
    for path in &files {
        let file = dir.join(path);
        let bytes = fs::read(&file).unwrap();
        let mut changed = bytes.clone();
        changed[bytes.len() / 2] ^= 0x01;
        fs::write(&file, &changed).unwrap();
        let (code, v) = run(&["verify", "--spec", SPEC, "--against", s(&dir)], true);
        assert_eq!(code, Some(1), "{path}: {v}");
        assert_eq!(v["verdict"], "fail");
        assert_eq!(v["against"], "differs");
        assert_eq!(v["gates"], Value::Array(Vec::new()), "{path}: no gate runs");
        let findings = v["findings"].as_array().unwrap();
        assert_eq!(findings.len(), 1, "{path}: {v}");
        assert_eq!(findings[0]["kind"], "against_differs");
        assert!(
            findings[0]["message"]
                .as_str()
                .unwrap()
                .starts_with(path.as_str()),
            "{v}"
        );
        fs::write(&file, &bytes).unwrap();
    }

    // A missing file and an extra one, each named.
    fs::remove_file(dir.join("board/Cargo.lock")).unwrap();
    fs::write(dir.join("board/src/extra.rs"), "").unwrap();
    let (code, v) = run(&["verify", "--spec", SPEC, "--against", s(&dir)], true);
    assert_eq!(code, Some(1));
    let kinds: Vec<(&str, &str)> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| {
            (
                f["kind"].as_str().unwrap(),
                f["message"].as_str().unwrap().split(' ').next().unwrap(),
            )
        })
        .collect();
    assert_eq!(
        kinds,
        [
            ("against_missing", "board/Cargo.lock"),
            ("against_extra", "board/src/extra.rs")
        ]
    );
}

/// §3.3: a board from another specification, and a hand-written vacuous
/// harness, cannot pass `verify`.
#[test]
fn verify_against_refuses_another_specifications_tree_and_a_hand_written_harness() {
    // Another specification: the reference with the TMP117 removed.
    let dir = scratch("other");
    fs::create_dir_all(&dir).unwrap();
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    let devices = spec["i2c_devices"].as_array_mut().unwrap();
    let before = devices.len();
    devices.retain(|d| d["part"] != "TMP117");
    assert_eq!(devices.len(), before - 1);
    let other = dir.join("other.json");
    fs::write(&other, spec.to_string()).unwrap();
    let tree_dir = dir.join("tree");
    let (code, v) = run(
        &["generate", "--spec", s(&other), "--out", s(&tree_dir)],
        true,
    );
    assert_eq!(code, Some(0), "{v}");
    let (code, v) = run(&["verify", "--spec", SPEC, "--against", s(&tree_dir)], true);
    assert_eq!(code, Some(1), "{v}");
    let named: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap().split(' ').next().unwrap())
        .collect();
    assert!(named.contains(&"kicad/sentinel-01.kicad_sch"), "{v}");
    assert!(named.contains(&"kicad/sentinel-01.bom.csv"), "{v}");

    // The review's A4 harness: `assume` cuts every path, no cover.
    let dir = reference("vacuous");
    fs::write(
        dir.join("board/src/proofs.rs"),
        "#[kani::proof]\nfn every_valid_sequence() {\n    let x: u8 = kani::any();\n    kani::assume(x != x);\n    assert!(1 == 2);\n}\n",
    )
    .unwrap();
    let (code, v) = run(&["verify", "--spec", SPEC, "--against", s(&dir)], true);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["findings"][0]["kind"], "against_differs");
    assert!(
        v["findings"][0]["message"]
            .as_str()
            .unwrap()
            .starts_with("board/src/proofs.rs"),
        "{v}"
    );
}

/// A scenario judges its own base: one whose base is not `--spec` would make
/// the campaign judge another specification, so `verify` refuses it before
/// anything runs.
#[test]
fn verify_refuses_a_scenario_whose_base_is_not_the_specification() {
    let dir = scratch("base");
    fs::create_dir_all(&dir).unwrap();
    // The same specification, reformatted: other bytes, another sha256.
    let spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    fs::write(
        dir.join("base.json"),
        serde_json::to_string_pretty(&spec).unwrap(),
    )
    .unwrap();
    let mut scenario: Value = serde_json::from_slice(
        &fs::read(root().join("scenarios/sentinel-01/pullup-2k2.json")).unwrap(),
    )
    .unwrap();
    scenario["base"] = "base.json".into();
    let file = dir.join("scenario.json");
    fs::write(&file, scenario.to_string()).unwrap();
    let (code, v) = run(&["verify", "--spec", SPEC, "--scenarios", s(&file)], true);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "ScenarioBaseMismatch");
    assert_eq!(v["error"]["id"], "pullup-2k2");
}

#[test]
fn prove_with_spec_refuses_an_invalid_specification_before_any_tool() {
    let dir = scratch("prove-spec");
    fs::create_dir_all(&dir).unwrap();
    let bad = dir.join("bad.json");
    fs::write(&bad, "{\"schema_version\": 1}").unwrap();
    let (code, v) = run(
        &[
            "prove",
            "--crate",
            "boards/sentinel-01",
            "--harness",
            "proofs::every_valid_sequence",
            "--spec",
            s(&bad),
        ],
        true,
    );
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "SchemaViolation");
}

/// `build` refuses, before any tool: an `--out` that exists, a crate with
/// a registry dependency (`RUL-04`), and a directory with no firmware crate.
#[test]
fn build_refuses_an_existing_output_a_dependency_and_a_crate_without_firmware() {
    let dir = reference("build");
    let board = dir.join("board");
    let exists = dir.join("exists.elf");
    fs::write(&exists, "keep").unwrap();
    let (code, v) = run(&["build", "--crate", s(&board), "--out", s(&exists)], true);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "UnwritableOutput");
    assert_eq!(fs::read(&exists).unwrap(), b"keep");

    let out = dir.join("fw.elf");
    let lock = board.join("firmware/Cargo.lock");
    let text = fs::read_to_string(&lock).unwrap();
    fs::write(
        &lock,
        format!("{text}\n[[package]]\nname = \"cortex-m\"\nversion = \"0.7.7\"\nsource = \"registry+https://github.com/rust-lang/crates.io-index\"\n"),
    )
    .unwrap();
    let (code, v) = run(&["build", "--crate", s(&board), "--out", s(&out)], true);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "CrateHasDependency");
    assert!(!out.exists());

    let (code, v) = run(
        &["build", "--crate", s(&board.join("src")), "--out", s(&out)],
        true,
    );
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "NotABoardCrate");
    assert!(!out.exists());
}
