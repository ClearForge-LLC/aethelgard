//! `AET-WO-1607`, tier 1, with stub tools: `check support` runs only the
//! pinned KiCad (1); `verify` on a specification that draws no board runs
//! past its board gates (5); and the lifecycle banner on the `emit`,
//! `check drc --spec`, `check support`, `prove --spec` and `check pinout`
//! verdicts (6), each equal to `validate`'s, and absent where `validate`'s is.

mod common;
#[path = "common/netlist.rs"]
mod netlist;
#[path = "../../aethelgard-pincheck/tests/common/mod.rs"]
mod pincheck;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

use common::{Scratch, private_tmp, stub};

/// Sentinel-01's reference: the NRND flash.
const NRND: &str = "examples/sentinel-01-minimal.json";
/// Sentinel-02's power board: the NRND flash and the EOL ICM-20948.
const EOL: &str = "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json";
/// Sentinel-02's bench board: `active` parts only, so no banner.
const ACTIVE: &str = "examples/sentinel-02-bench.json";
/// A twin stand-in, `board.drawn: false`.
const NO_BOARD: &str = "crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn spec(rel: &str) -> String {
    root().join(rel).to_str().unwrap().to_owned()
}

/// Runs the binary from the repository root with `PATH` the toolbox (or
/// nothing), `TMPDIR` a private directory off `$HOME` and `vars`: its exit
/// code and its one JSON line.
fn run(args: &[&str], toolbox: Option<&Path>, vars: &[(&str, &Path)]) -> (i32, Value) {
    let tmp = private_tmp(&format!("1607-{}", next()));
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("PATH", toolbox.map_or(OsStr::new(""), |t| t.as_os_str()))
        .env("TMPDIR", &tmp.0)
        .envs(vars.iter().copied())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "{args:?}: {stdout} {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

/// The oracle: `validate`'s banner, or null when it carries none.
fn banner(rel: &str) -> Value {
    let (code, v) = run(&["validate", "--file", &spec(rel)], None, &[]);
    assert_eq!(code, 0, "{v}");
    v["lifecycle"].clone()
}

#[test]
fn the_three_specifications_carry_the_banners_they_are_chosen_for() {
    let lines = |rel| -> Vec<String> {
        banner(rel)
            .as_array()
            .map(|a| a.iter().map(|l| l.as_str().unwrap().to_owned()).collect())
            .unwrap_or_default()
    };
    let starts = |rel| -> Vec<String> {
        lines(rel)
            .iter()
            .map(|l| l.split(" (").next().unwrap().to_owned())
            .collect()
    };
    assert_eq!(starts(NRND), ["LIFECYCLE NRND: W25Q64JVxIQ"]);
    assert_eq!(
        starts(EOL),
        ["LIFECYCLE NRND: W25Q64JVxIQ", "LIFECYCLE EOL: ICM20948"]
    );
    assert!(lines(ACTIVE).is_empty());
}

/// A toolbox directory of `#!/bin/sh` stubs.
fn toolbox() -> Scratch {
    Scratch::new(&format!("1607-tb-{}", next()))
}

fn next() -> usize {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    N.fetch_add(1, Ordering::Relaxed)
}

/// A stub `kicad-cli` printing `version`, which writes `drc.json` for
/// `pcb drc` and `input.net` for `sch export netlist` into the file after
/// `-o`, from `tb`.
fn kicad(tb: &Path, version: &str) {
    let body = format!(
        r#"#!/bin/sh
if [ "$1" = version ]; then printf '%s\n' '{version}'; exit 0; fi
out=; prev=
for a in "$@"; do
  if [ "$prev" = -o ]; then out=$a; fi
  prev=$a
done
case "$1 $2" in
  "pcb drc") /bin/cat '{tb}/drc.json' > "$out"; exit 0;;
  "sch export") [ -f '{tb}/input.net' ] || exit 3; /bin/cat '{tb}/input.net' > "$out"; exit 0;;
esac
exit 99
"#,
        tb = tb.display()
    );
    stub::write_executable(&tb.join("kicad-cli"), body);
    fs::write(
        tb.join("drc.json"),
        r#"{"$schema":"https://schemas.kicad.org/drc.v1.json","coordinate_units":"mm","date":"x","included_severities":["error","warning"],"kicad_version":"10.0.7","source":"input.kicad_pcb","violations":[],"unconnected_items":[],"schematic_parity":[]}"#,
    )
    .unwrap();
}

/// `emit` of `rel` into a scratch directory: the verdict, and the
/// schematic's and board's paths.
fn emitted(rel: &str) -> (Scratch, Value, String, String) {
    let dir = Scratch::new(&format!("1607-emit-{}", next()));
    let out = dir.0.join("out");
    let (code, v) = run(
        &["emit", "--file", &spec(rel), "--out", out.to_str().unwrap()],
        None,
        &[],
    );
    assert_eq!(code, 0, "{v}");
    let file = |key: &str| v[key]["path"].as_str().unwrap().to_owned();
    let (sch, pcb) = (file("schematic"), file("board"));
    (dir, v, sch, pcb)
}

// ------------------------------------------------------------------- (1)

/// `AET-WO-1607` (1), `1311` F4: `check support` runs KiCad 10.0.7 only, as
/// `check drc` and `check erc` do; any other version, however near, is
/// refused before the netlist is exported. 10.0.7 gets past the version (the
/// stub then has no netlist to give, so the export fails as a tool error).
#[test]
fn check_support_runs_only_the_pinned_kicad() {
    let (_dir, _, sch, pcb) = emitted(NRND);
    let support = |tb: &Path| {
        run(
            &[
                "check",
                "support",
                "--schematic",
                &sch,
                "--board",
                &pcb,
                "--spec",
                &spec(NRND),
            ],
            Some(tb),
            &[],
        )
    };
    for printed in ["10.0.8", "10.0.6", "9.0.7", "10.0.7-1", "10.0.70", ""] {
        let tb = toolbox();
        kicad(&tb.0, printed);
        let (code, v) = support(&tb.0);
        assert_eq!(code, 4, "{printed:?}: {v}");
        assert_eq!(v["error"]["kind"], "ToolUnusable", "{printed:?}: {v}");
        assert_eq!(v["error"]["id"], "kicad-cli", "{printed:?}: {v}");
    }
    let tb = toolbox();
    kicad(&tb.0, "10.0.7");
    let (code, v) = support(&tb.0);
    assert_eq!(code, 5, "{v}");
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("sch export netlist exited 3"),
        "{v}"
    );
}

// ------------------------------------------------------------------- (5)

/// `AET-WO-1607` (5), `1210` F2: plain `verify` on a specification that
/// draws no board is not refused up front; it skips ERC, DRC, `support` and
/// `extract` and runs on to `prove`. With no tool on `PATH`, a drawn board's
/// `verify` stops at its first gate, ERC, for `kicad-cli`; the stand-in's
/// stops at `prove`, for `cargo`. Both fab flags still refuse the stand-in
/// (`emit_tier1`'s stand-in test).
#[test]
fn verify_of_a_board_not_drawn_runs_past_its_board_gates() {
    for (rel, gate, tool) in [(NRND, "erc", "kicad-cli"), (NO_BOARD, "prove", "cargo")] {
        let (code, v) = run(&["verify", "--spec", &spec(rel)], None, &[]);
        assert_eq!(code, 4, "{rel}: {v}");
        assert_eq!(v["error"]["kind"], "ToolMissing", "{rel}: {v}");
        assert_eq!(v["error"]["id"], tool, "{rel}: {v}");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .starts_with(&format!("gate `{gate}`: ")),
            "{rel}: {v}"
        );
    }
}

// ------------------------------------------------------------------- (6)

/// `emit`: the verdict carries the banner; the KiCad files do not (a later
/// work order), so no emitted byte moves.
#[test]
fn the_emit_verdict_carries_the_banner_and_the_files_do_not() {
    for rel in [NRND, EOL, ACTIVE] {
        let (_dir, v, sch, pcb) = emitted(rel);
        assert_eq!(v["lifecycle"], banner(rel), "{rel}: {v}");
        assert_eq!(v.get("lifecycle").is_some(), rel != ACTIVE, "{rel}: {v}");
        let bom = v["bill_of_materials"]["path"].as_str().unwrap();
        for file in [&sch, &pcb, bom] {
            let text = fs::read_to_string(file).unwrap();
            assert!(!text.contains("LIFECYCLE"), "{rel}: {file}");
        }
    }
}

/// `check drc --spec` and `check support` carry the banner of the
/// specification they read; `check drc` with no `--spec` reads none.
#[test]
fn the_drc_and_support_verdicts_carry_the_banner_of_their_spec() {
    for rel in [NRND, EOL, ACTIVE] {
        let (_dir, _, sch, pcb) = emitted(rel);
        let tb = toolbox();
        kicad(&tb.0, "10.0.7");
        fs::write(
            tb.0.join("input.net"),
            netlist::netlist_text(&netlist::pads_of(&fs::read_to_string(&pcb).unwrap())),
        )
        .unwrap();
        let s = spec(rel);
        let drc_spec = ["check", "drc", "--board", &pcb, "--spec", &s];
        let support = [
            "check",
            "support",
            "--schematic",
            &sch,
            "--board",
            &pcb,
            "--spec",
            &s,
        ];
        for args in [&drc_spec[..], &support[..]] {
            let (code, v) = run(args, Some(&tb.0), &[]);
            assert!(code == 0 || code == 1, "{rel} {args:?}: {v}");
            assert_eq!(v["lifecycle"], banner(rel), "{rel} {args:?}: {v}");
        }
        let (code, v) = run(&["check", "drc", "--board", &pcb], Some(&tb.0), &[]);
        assert!(code == 0 || code == 1, "{rel}: {v}");
        assert!(v.get("lifecycle").is_none(), "{rel}: {v}");
    }
}

/// A stub cargo whose `kani` verifies every harness.
fn kani(tb: &Path) {
    stub::write_executable(
        &tb.join("cargo"),
        r#"#!/bin/sh
if [ "$1" = kani ] && [ "$2" = --version ]; then echo 'Kani Rust Verifier 0.68.0 (cargo plugin)'; exit 0; fi
[ "$1" = kani ] || exit 99
prev=; h=
for a in "$@"; do
  if [ "$prev" = --harness ]; then h=$a; fi
  prev=$a
done
printf 'Checking harness %s...\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n' "$h"
"#,
    );
    stub::write_executable(&tb.join("cargo-kani"), "#!/bin/sh\nexit 0\n");
}

/// `prove --spec` carries the banner of the specification it generates the
/// crate from; `prove` with no `--spec` reads none.
#[test]
fn the_prove_verdict_carries_the_banner_of_its_spec() {
    for rel in [NRND, EOL, ACTIVE] {
        let dir = Scratch::new(&format!("1607-gen-{}", next()));
        let out = dir.0.join("gen");
        let (code, v) = run(
            &[
                "generate",
                "--spec",
                &spec(rel),
                "--out",
                out.to_str().unwrap(),
            ],
            None,
            &[],
        );
        assert_eq!(code, 0, "{v}");
        let board = out.join("board");
        let proofs = fs::read_to_string(board.join("src/proofs.rs")).unwrap();
        let harness = aethelgard_first_harness(&proofs);
        let tb = toolbox();
        kani(&tb.0);
        let homes = Scratch::new(&format!("1607-homes-{}", next()));
        let (k, r) = (homes.0.join("kani"), homes.0.join("rustup"));
        fs::create_dir(&k).unwrap();
        fs::create_dir(&r).unwrap();
        let vars = [("KANI_HOME", k.as_path()), ("RUSTUP_HOME", r.as_path())];
        let b = board.to_str().unwrap();
        let s = spec(rel);
        let (code, v) = run(
            &["prove", "--crate", b, "--harness", &harness, "--spec", &s],
            Some(&tb.0),
            &vars,
        );
        assert_eq!(code, 0, "{rel}: {v}");
        assert_eq!(v["harness_provenance"], "generated", "{rel}: {v}");
        assert_eq!(v["lifecycle"], banner(rel), "{rel}: {v}");
        let (code, v) = run(
            &["prove", "--crate", b, "--harness", &harness],
            Some(&tb.0),
            &vars,
        );
        assert_eq!(code, 0, "{rel}: {v}");
        assert!(v.get("lifecycle").is_none(), "{rel}: {v}");
    }
}

/// The first `#[kani::proof]` harness of a generated `proofs.rs`, by path.
fn aethelgard_first_harness(proofs: &str) -> String {
    let mut armed = false;
    for line in proofs.lines().map(str::trim) {
        if line == "#[kani::proof]" {
            armed = true;
        } else if armed && let Some(rest) = line.strip_prefix("fn ") {
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .collect();
            return format!("proofs::{name}");
        }
    }
    panic!("no harness in proofs.rs");
}

/// `check pinout` carries the banner of the topology it reads, solved here
/// from each specification: `validate`'s banner, as the solver keeps every
/// device.
#[test]
fn the_pinout_verdict_carries_the_banner_of_its_topology() {
    let dir = Scratch::new("1607-pinout");
    let elf = dir.0.join("fw.elf");
    fs::write(&elf, pincheck::Elf::default().bytes()).unwrap();
    for rel in [NRND, EOL, ACTIVE] {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(["solve", "--file", &spec(rel)])
            .output()
            .unwrap();
        assert!(out.status.success(), "{rel}");
        let topology = dir.0.join("topology.json");
        fs::write(&topology, &out.stdout).unwrap();
        let (code, v) = run(
            &[
                "check",
                "pinout",
                "--topology",
                topology.to_str().unwrap(),
                "--elf",
                elf.to_str().unwrap(),
            ],
            None,
            &[],
        );
        assert!(code == 0 || code == 1, "{rel}: {v}");
        assert_eq!(v["lifecycle"], banner(rel), "{rel}: {v}");
    }
}

// ------------------------------------------------------------------- (2)

/// `AET-WO-1607` (2), `1603` F2: the tier-1 tests' private scratch, which
/// they pass as `TMPDIR` to `build` and `prove`, resolves outside `$HOME`, so
/// a `~/.cargo/config.toml` is never an ancestor of a crate those commands
/// copy. Root's is `/var/lib/aethelgard`.
#[test]
fn the_private_scratch_is_outside_home() {
    let tmp = private_tmp(&format!("1607-home-{}", next()));
    let real = fs::canonicalize(&tmp.0).unwrap();
    let home = fs::canonicalize(std::env::var_os("HOME").unwrap()).unwrap();
    assert!(
        !real.starts_with(&home),
        "{} is under {}",
        real.display(),
        home.display()
    );
    let base = if rustix::process::geteuid().is_root() {
        PathBuf::from("/var/lib/aethelgard")
    } else {
        PathBuf::from(format!(
            "/run/user/{}/aethelgard-tests",
            rustix::process::geteuid().as_raw()
        ))
    };
    assert_eq!(real.parent(), Some(base.as_path()));
}
