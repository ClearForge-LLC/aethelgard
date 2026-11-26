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

use common::verify::Shared;
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

/// [`banner`]'s lines; none when it carries none.
fn banner_lines(rel: &str) -> Vec<String> {
    banner(rel)
        .as_array()
        .map(|a| a.iter().map(|l| l.as_str().unwrap().to_owned()).collect())
        .unwrap_or_default()
}

#[test]
fn the_three_specifications_carry_the_banners_they_are_chosen_for() {
    let starts = |rel| -> Vec<String> {
        banner_lines(rel)
            .iter()
            .map(|l| l.split(" (").next().unwrap().to_owned())
            .collect()
    };
    assert_eq!(starts(NRND), ["LIFECYCLE NRND: W25Q64JVxIQ"]);
    assert_eq!(
        starts(EOL),
        ["LIFECYCLE NRND: W25Q64JVxIQ", "LIFECYCLE EOL: ICM20948"]
    );
    assert!(banner_lines(ACTIVE).is_empty());
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

/// One `emit` of a specification into a scratch directory: the verdict, and
/// the schematic's and board's paths. Read, never written, by every test that
/// needs that specification's boards.
struct Emitted {
    _dir: Scratch,
    verdict: Value,
    sch: String,
    pcb: String,
}

/// One `emit` per specification, made once per test binary and read by the
/// `check support` test and the `emit` and `drc`/`support` banner tests
/// (`AET-WO-1916` D2, `1607` F-L). A failed `emit` is recorded once and fails
/// every reader, each naming its own property.
static EMITS: [Shared<Emitted>; 3] = [
    Shared::new("emit of the NRND reference"),
    Shared::new("emit of the EOL power board"),
    Shared::new("emit of the ACTIVE bench board"),
];

/// `rel`'s shared `emit`; `property` is what the calling test checks on it.
fn emitted(rel: &str, property: &str) -> &'static Emitted {
    let i = [NRND, EOL, ACTIVE]
        .iter()
        .position(|r| *r == rel)
        .expect("emit is shared for the three banner specifications only");
    EMITS[i].get(property, || {
        let dir = Scratch::new(&format!("1607-emit-{}", next()));
        let out = dir.0.join("out");
        let (code, verdict) = run(
            &["emit", "--file", &spec(rel), "--out", out.to_str().unwrap()],
            None,
            &[],
        );
        assert_eq!(code, 0, "{verdict}");
        let file = |key: &str| verdict[key]["path"].as_str().unwrap().to_owned();
        let (sch, pcb) = (file("schematic"), file("board"));
        Emitted {
            _dir: dir,
            verdict,
            sch,
            pcb,
        }
    })
}

// ------------------------------------------------------------------- (1)

/// `AET-WO-1607` (1), `1311` F4: `check support` runs KiCad 10.0.7 only, as
/// `check drc` and `check erc` do; any other version, however near, is
/// refused before the netlist is exported. 10.0.7 gets past the version (the
/// stub then has no netlist to give, so the export fails as a tool error).
#[test]
fn check_support_runs_only_the_pinned_kicad() {
    let e = emitted(NRND, "check support runs only the pinned KiCad");
    let support = |tb: &Path| {
        run(
            &[
                "check",
                "support",
                "--schematic",
                &e.sch,
                "--board",
                &e.pcb,
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
        let e = emitted(rel, "the emit verdict carries the banner");
        let v = &e.verdict;
        assert_eq!(v["lifecycle"], banner(rel), "{rel}: {v}");
        assert_eq!(v.get("lifecycle").is_some(), rel != ACTIVE, "{rel}: {v}");
        let bom = v["bill_of_materials"]["path"].as_str().unwrap();
        for file in [&e.sch, &e.pcb, bom] {
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
        let e = emitted(rel, "the drc and support verdicts carry the banner");
        let (sch, pcb) = (e.sch.clone(), e.pcb.clone());
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

/// `AET-WO-1919` (1) (G), ruled at its stop: `prove --spec` generates the
/// crate alone. A specification whose board cannot be routed (a 250 µm
/// clearance, which the LQFP-144's pad gaps break, as `stackups_tier1`
/// plants) but whose crate generates is refused by `generate` and by
/// `verify`, before any tool, and `prove --spec` judges its crate
/// `generated`: routing is not what `prove` judges. Until (G) it was refused
/// `Unroutable`. `generate` writes no crate for it, so the crate is written
/// here from the library's `generate::board_tree`; the crate carries the
/// specification's sha256, so no other crate would be `generated`.
#[test]
fn prove_spec_judges_the_crate_of_a_specification_whose_board_does_not_route() {
    let dir = Scratch::new(&format!("1919-c250-{}", next()));
    let mut v: Value = serde_json::from_slice(&fs::read(spec(NRND)).unwrap()).unwrap();
    v["board"]["rules"]["clearance_um"] = Value::from(250);
    let c250 = dir.0.join("c250.json");
    fs::write(&c250, serde_json::to_vec(&v).unwrap()).unwrap();
    let c250 = c250.to_str().unwrap();
    let out = dir.0.join("gen");
    let (code, g) = run(
        &["generate", "--spec", c250, "--out", out.to_str().unwrap()],
        None,
        &[],
    );
    assert_eq!(
        (code, &g["error"]["kind"]),
        (2, &Value::from("Unroutable")),
        "{g}"
    );
    let (code, g) = run(&["verify", "--spec", c250], None, &[]);
    assert_eq!(
        (code, &g["error"]["kind"]),
        (2, &Value::from("Unroutable")),
        "{g}"
    );
    let bytes = fs::read(c250).unwrap();
    let parsed: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    let table = aethelgard_svd_hal::pin_table(&parsed.mcu.part).unwrap();
    let sha = <sha2::Sha256 as sha2::Digest>::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    let topology = aethelgard_ir::solve(&parsed, &table, &sha).unwrap();
    let crate_dir = dir.0.join("crate");
    let files = aethelgard_cli::generate::board_tree(&parsed, &topology, &table).unwrap();
    for (path, text) in &files {
        let rel = path
            .strip_prefix(&format!("{}/", aethelgard_cli::generate::BOARD_DIR))
            .unwrap();
        let to = crate_dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
    let board = crate_dir.to_str().unwrap().to_owned();
    let proofs = fs::read_to_string(crate_dir.join("src/proofs.rs")).unwrap();
    let harness = aethelgard_first_harness(&proofs);
    let tb = toolbox();
    kani(&tb.0);
    let homes = Scratch::new(&format!("1919-homes-{}", next()));
    let (k, r) = (homes.0.join("kani"), homes.0.join("rustup"));
    fs::create_dir(&k).unwrap();
    fs::create_dir(&r).unwrap();
    let vars = [("KANI_HOME", k.as_path()), ("RUSTUP_HOME", r.as_path())];
    let (code, p) = run(
        &[
            "prove",
            "--crate",
            &board,
            "--harness",
            &harness,
            "--spec",
            c250,
        ],
        Some(&tb.0),
        &vars,
    );
    assert_eq!(code, 0, "{p}");
    assert_eq!(p["harness_provenance"], "generated", "{p}");
}

/// A `cargo` for `verify`'s two lanes (`AET-WO-1919` (1) (X)): `cargo kani`
/// runs `prove`, as [`kani`]'s does, then `prove`; `cargo build` runs
/// `build`; each version question is answered.
fn lanes_cargo(tb: &Path, prove: &str, build: &str) {
    stub::write_executable(
        &tb.join("cargo"),
        format!(
            r#"#!/bin/sh
if [ "$1" = kani ] && [ "$2" = --version ]; then echo 'Kani Rust Verifier 0.68.0 (cargo plugin)'; exit 0; fi
if [ "$1" = --version ]; then echo 'cargo 1.99.1 (58cf8f9c3 2026-10-13)'; exit 0; fi
if [ "$1" = kani ]; then
  prev=; h=
  for a in "$@"; do
    if [ "$prev" = --harness ]; then h=$a; fi
    prev=$a
  done
  {prove}
  printf 'Checking harness %s...\nVERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n' "$h"
  exit 0
fi
if [ "$1" = build ]; then {build}; fi
exit 99
"#
        ),
    );
    stub::write_executable(&tb.join("cargo-kani"), "#!/bin/sh\nexit 0\n");
}

/// `verify` with the stand-in `cargo` of [`lanes_cargo`], on a board not
/// drawn, so its gates are the prove lane and `build`'s lane: its exit code,
/// its one line, and how long it took.
fn lanes_verify(prove: &str, build: &str) -> (i32, Value, std::time::Duration) {
    let tb = toolbox();
    lanes_cargo(&tb.0, prove, build);
    let homes = Scratch::new(&format!("1919-lanes-{}", next()));
    let (k, r) = (homes.0.join("kani"), homes.0.join("rustup"));
    fs::create_dir(&k).unwrap();
    fs::create_dir(&r).unwrap();
    let vars = [("KANI_HOME", k.as_path()), ("RUSTUP_HOME", r.as_path())];
    let start = std::time::Instant::now();
    let (code, v) = run(&["verify", "--spec", &spec(NO_BOARD)], Some(&tb.0), &vars);
    (code, v, start.elapsed())
}

/// `AET-WO-1919` (1) (X), ruled: two gates refuse in different lanes, `build`
/// at once and the first prove gate after 3 s. `build` finished first, but
/// the prove gate comes first in today's order, so its refusal is the one
/// reported, and the only line printed.
#[test]
fn of_two_refusals_in_two_lanes_the_earlier_in_today_s_order_is_reported() {
    let (code, v, _) = lanes_verify("/bin/sleep 3; exit 7", "exit 7");
    assert_eq!(code, 5, "{v}");
    assert_eq!(v["error"]["kind"], "ToolError", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(m.starts_with("gate `prove`: "), "{v}");
}

/// `AET-WO-1919` (1) (X), ruled: the lanes run beside each other. Each
/// prove gate takes 1 s and passes. First `build` refuses at once: `verify`
/// reports it when the prove lane is done, P s in. Then `build` takes P s
/// before it refuses. One after another that run would take 2P; side by side
/// it takes about P. Measured against this host's own P, so a slow runner
/// slows both. `build`'s refusal, the first in today's order once every
/// prove gate passed, is reported both times.
#[test]
fn the_prove_lane_and_build_s_lane_run_beside_each_other() {
    let build_refused = |v: &Value| {
        let m = v["error"]["message"].as_str().unwrap();
        assert!(m.starts_with("gate `build`: "), "{v}");
    };
    let (code, v, p) = lanes_verify("/bin/sleep 1", "exit 7");
    assert_eq!(code, 5, "{v}");
    build_refused(&v);
    let secs = p.as_secs().max(4);
    let (code, v, took) = lanes_verify("/bin/sleep 1", &format!("/bin/sleep {secs}; exit 7"));
    assert_eq!(code, 5, "{v}");
    build_refused(&v);
    eprintln!("ANSWER 1919 lanes: the prove lane {p:?}; with build {secs} s beside it {took:?}");
    assert!(
        took.as_secs_f64() < 1.5 * p.as_secs_f64().max(4.0),
        "P {p:?}, took {took:?}"
    );
}

/// Every live process whose environment names `dir` (this test's toolbox, on
/// its `PATH`): the gates, their reapers and the stand-ins they run, by
/// command line.
fn processes_naming(dir: &Path) -> Vec<String> {
    let needle = dir.as_os_str().as_encoded_bytes();
    let mut found = Vec::new();
    for entry in fs::read_dir("/proc").unwrap().flatten() {
        let pid = entry.file_name();
        if !pid.to_string_lossy().bytes().all(|b| b.is_ascii_digit()) {
            continue;
        }
        let Ok(env) = fs::read(entry.path().join("environ")) else {
            continue;
        };
        if env.windows(needle.len()).any(|w| w == needle) {
            let cmd = fs::read(entry.path().join("cmdline")).unwrap_or_default();
            found.push(String::from_utf8_lossy(&cmd).replace('\0', " "));
        }
    }
    found
}

/// Every path beneath `dir`, sorted.
fn listing(dir: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).into_iter().flatten().flatten() {
            if e.file_type().is_ok_and(|t| t.is_dir()) {
                stack.push(e.path());
            }
            out.push(e.path());
        }
    }
    out.sort();
    out
}

/// `AET-WO-1919` (X), the architect's 06:49Z finding: a `verify` that stops
/// early leaves no lane's gate running and writes nothing after it exits.
/// `build`'s lane is seen running its stand-in (a `sleep` of 30 s), then the
/// first prove gate refuses and `verify` exits with that refusal. `verify`
/// kills and waits for its lanes before it exits, so at once (one scan) no
/// process naming this test's toolbox is left (the gate, its reaper, the
/// stand-in), the call's scratch is empty (the killed gate's own directory
/// removed by its guardian, which `verify` waited for), and it stays so.
/// Before the fix, the gates outlived `verify` by 1.1–1.2 s.
#[test]
fn a_verify_that_stops_early_leaves_no_lane_gate_and_no_late_file() {
    let tb = toolbox();
    lanes_cargo(&tb.0, "/bin/sleep 2; exit 7", "/bin/sleep 30.1919; exit 7");
    let homes = Scratch::new(&format!("1919-orphans-{}", next()));
    let (k, r) = (homes.0.join("kani"), homes.0.join("rustup"));
    fs::create_dir(&k).unwrap();
    fs::create_dir(&r).unwrap();
    let tmp = private_tmp(&format!("1919-orphans-tmp-{}", next()));
    let child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["verify", "--spec", &spec(NO_BOARD)])
        .current_dir(root())
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("PATH", &tb.0)
        .env("TMPDIR", &tmp.0)
        .env("KANI_HOME", &k)
        .env("RUSTUP_HOME", &r)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    while !processes_naming(&tb.0)
        .iter()
        .any(|c| c.contains("30.1919"))
    {
        assert!(start.elapsed().as_secs() < 20, "build's lane never started");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    let out = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(5), "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("gate `prove`: "),
        "{v}"
    );
    let exited = std::time::Instant::now();
    let left = processes_naming(&tb.0);
    let scanned = exited.elapsed();
    eprintln!("ANSWER 1919 scanned {scanned:?} after verify exited: {left:?}");
    assert!(
        left.is_empty(),
        "left running after verify exited: {left:?}"
    );
    assert!(scanned.as_millis() < 500, "{scanned:?}");
    assert_eq!(
        listing(&tmp.0),
        Vec::<PathBuf>::new(),
        "left in the scratch"
    );
    std::thread::sleep(std::time::Duration::from_secs(1));
    assert_eq!(
        listing(&tmp.0),
        Vec::<PathBuf>::new(),
        "a file appeared after verify exited"
    );
}

/// `AET-WO-1919` (X): a `verify` killed outright (SIGKILL, so [`stop_lanes`]
/// never runs) still takes its lanes' gates with it. Each gate's parent-death
/// signal fires when its lane's thread ends with the process, and each gate's
/// reaper then ends its tool. Measured: gone 1.1–1.2 s after the kill.
#[test]
fn a_verify_killed_outright_leaves_no_lane_gate() {
    let tb = toolbox();
    lanes_cargo(
        &tb.0,
        "/bin/sleep 30.1919; exit 7",
        "/bin/sleep 30.1919; exit 7",
    );
    let homes = Scratch::new(&format!("1919-killed-{}", next()));
    let (k, r) = (homes.0.join("kani"), homes.0.join("rustup"));
    fs::create_dir(&k).unwrap();
    fs::create_dir(&r).unwrap();
    let tmp = private_tmp(&format!("1919-killed-tmp-{}", next()));
    let mut child = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["verify", "--spec", &spec(NO_BOARD)])
        .current_dir(root())
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("PATH", &tb.0)
        .env("TMPDIR", &tmp.0)
        .env("KANI_HOME", &k)
        .env("RUSTUP_HOME", &r)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    // Both lanes' stand-ins running.
    while processes_naming(&tb.0)
        .iter()
        .filter(|c| c.contains("30.1919"))
        .count()
        < 2
    {
        assert!(start.elapsed().as_secs() < 20, "the lanes never started");
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    child.kill().unwrap();
    child.wait().unwrap();
    let killed = std::time::Instant::now();
    let mut left = processes_naming(&tb.0);
    while !left.is_empty() && killed.elapsed().as_secs() < 10 {
        std::thread::sleep(std::time::Duration::from_millis(50));
        left = processes_naming(&tb.0);
    }
    eprintln!(
        "ANSWER 1919 killed verify: lanes gone {:?} after",
        killed.elapsed()
    );
    assert!(
        left.is_empty(),
        "left running after verify was killed: {left:?}"
    );
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
    // As `private_tmp` reads it: a home of `/`, or none, holds no scratch apart.
    if let Some(home) = std::env::var_os("HOME")
        .and_then(|h| fs::canonicalize(h).ok())
        .filter(|h| h.parent().is_some())
    {
        assert!(
            !real.starts_with(&home),
            "{} is under {}",
            real.display(),
            home.display()
        );
    }
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

/// `solve` (D1, ruled (b)): stdout stays the `SolvedTopology` alone, which
/// `generate hwdef --topology` reads strictly; the banner is on stderr, one
/// line each, equal to `validate`'s, and stderr is empty with none.
#[test]
fn solve_prints_the_banner_on_stderr_and_the_topology_alone_on_stdout() {
    let dir = Scratch::new("1607-solve");
    for rel in [NRND, EOL, ACTIVE] {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args(["solve", "--file", &spec(rel)])
            .env("PATH", "")
            .output()
            .unwrap();
        assert!(out.status.success(), "{rel}");
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert_eq!(
            stderr.lines().collect::<Vec<_>>(),
            banner_lines(rel),
            "{rel}"
        );
        let topology: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert!(topology.get("lifecycle").is_none(), "{rel}");
        let file = dir.0.join("topology.json");
        fs::write(&file, &out.stdout).unwrap();
        let hwdef = dir.0.join("hwdef.dat");
        let _ = fs::remove_file(&hwdef);
        let (code, v) = run(
            &[
                "generate",
                "hwdef",
                "--topology",
                file.to_str().unwrap(),
                "--out",
                hwdef.to_str().unwrap(),
            ],
            None,
            &[],
        );
        // The strict reader takes it: a pass, or a refusal of the board's own
        // (`SenseNotEmitted`, `NoSerialPort`), never the topology's shape.
        assert!(
            code == 0
                || !["SchemaViolation", "InvalidJson"]
                    .contains(&v["error"]["kind"].as_str().unwrap()),
            "{rel}: {v}"
        );
        // The reference is a board CI generates a hwdef for, from this pipe.
        if rel == NRND {
            assert_eq!(code, 0, "{v}");
        }
    }
}
