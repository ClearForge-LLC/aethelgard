//! What the tier-2 `verify` tests share (`AET-WO-3301`, `AET-WO-3300`, `AET-WO-1909`):
//! running the binary from the repository root, the committed board, `build`,
//! each gate's read grant, a package's files, and [`Shared`], a run made once
//! for every test that reads it.

use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;
use std::time::Instant;

use serde_json::Value;

pub const SPEC: &str = "examples/sentinel-01-minimal.json";

/// The reference's seeded set for a package (`AET-WO-3303` D1, ruled): a
/// tolerated fault on `SENSORS` (3.3 kΩ) and on `VDD_3V3` (a droop past BOR
/// level 3, recovered), `FLASH`'s absent flash detected by its mechanism, the
/// no-fault baseline, and `CONSOLE`'s open RX line detected by its mechanism
/// (`AET-WO-2302` D5).
#[allow(dead_code)]
pub const SEEDED: [&str; 5] = [
    "scenarios/sentinel-01/seeded/pullup-3k3.json",
    "scenarios/sentinel-01/droop-bor3.json",
    "scenarios/sentinel-01/seeded/spi-absent-u4.json",
    "scenarios/sentinel-01/pullup-2k2.json",
    "scenarios/sentinel-01/seeded/serial-rx-open-console.json",
];

pub fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

pub fn board() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../boards/sentinel-01")
}

/// Runs the binary from the repository root: its exit code and stdout.
pub fn aethelgard(args: &[&str]) -> (Option<i32>, String) {
    let start = Instant::now();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    let line = format!(
        "ANSWER 3301: {} exit={:?} wall_s={:.2}\n",
        args[0],
        out.status.code(),
        start.elapsed().as_secs_f64()
    );
    log.write_all(line.as_bytes()).unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

/// The one JSON line a command printed.
pub fn one(stdout: &str) -> Value {
    assert_eq!(stdout.lines().count(), 1, "one verdict line: {stdout}");
    serde_json::from_str(stdout).unwrap()
}

pub fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let (src, dst) = (entry.path(), to.join(entry.file_name()));
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

pub fn sha(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// A fixture's solved topology and the board crate's files generated from it,
/// as the CLI makes them (`AET-WO-4220`).
pub fn generated(
    spec_path: &Path,
    name: &str,
) -> (aethelgard_ir::SolvedTopology, BTreeMap<String, String>) {
    let bytes = fs::read(spec_path).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&bytes).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &sha(&bytes)).unwrap();
    let answers = |part: &str| matches!(aethelgard_twin::model::model(part), Ok(Some(_)));
    let files = aethelgard_svd_hal::board::generate_with(name, &solved, &table, &answers).unwrap();
    (solved, files)
}

/// Writes a generated crate's files under `dir`, emptied first.
pub fn write_files(dir: &Path, files: &BTreeMap<String, String>) {
    let _ = fs::remove_dir_all(dir);
    for (rel, text) in files {
        let to = dir.join(rel);
        fs::create_dir_all(to.parent().unwrap()).unwrap();
        fs::write(&to, text).unwrap();
    }
}

/// Builds `board` with `aethelgard build`; the verdict and the ELF's bytes.
pub fn build(board: &Path, elf: &Path) -> (Value, Vec<u8>) {
    let (code, stdout) = aethelgard(&[
        "build",
        "--crate",
        board.to_str().unwrap(),
        "--out",
        elf.to_str().unwrap(),
    ]);
    let v = one(&stdout);
    assert_eq!(code, Some(0), "build passes: {v}");
    assert_eq!(v["check"], "build", "build names its check: {v}");
    (v, fs::read(elf).unwrap())
}

/// The paths each gate's tools add to their read grant beyond the system set,
/// as the CLI grants them.
pub fn gate_reads(gate: &str) -> Vec<PathBuf> {
    let var = |name: &str| PathBuf::from(std::env::var_os(name).expect(name));
    let bridge = Path::new(env!("CARGO_BIN_EXE_aethelgard"))
        .parent()
        .unwrap()
        .to_path_buf();
    let mut reads = Vec::new();
    match gate {
        "erc" | "drc" | "support" => reads.extend(super::tool_dirs("kicad-cli")),
        "prove" => {
            reads.extend(super::tool_dirs("cargo"));
            reads.extend(super::tool_dirs("cargo-kani"));
            reads.extend([var("KANI_HOME"), var("RUSTUP_HOME")]);
        }
        "build" => {
            reads.extend(super::tool_dirs("cargo"));
            reads.push(var("RUSTUP_HOME"));
        }
        "simulate" | "campaign" => {
            reads.extend(super::tool_dirs("renode"));
            reads.push(bridge);
        }
        other => panic!("no read grant known for gate {other}"),
    }
    reads
}

/// A package directory's files, by name: every entry must be a file.
pub fn package_files(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(dir)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            assert!(
                e.file_type().unwrap().is_file(),
                "a package holds only files: {:?}",
                e.path()
            );
            (
                e.file_name().into_string().unwrap(),
                fs::read(e.path()).unwrap(),
            )
        })
        .collect()
}

/// A run made once per test binary and read by several tests (`AET-WO-1909`
/// F6). Its first result is kept, failure included: a run that panics is
/// recorded once, never made again, and every test that reads it fails at
/// once, naming the property it checks and the recorded failure.
pub struct Shared<T: 'static> {
    name: &'static str,
    cell: OnceLock<Result<T, String>>,
}

impl<T: 'static> Shared<T> {
    pub const fn new(name: &'static str) -> Self {
        Shared {
            name,
            cell: OnceLock::new(),
        }
    }

    /// The run, made by `make` if no test has made it; `property` is what the
    /// calling test checks on it, named if the run failed.
    pub fn get(&'static self, property: &str, make: impl FnOnce() -> T) -> &'static T {
        let result = self.cell.get_or_init(|| {
            panic::catch_unwind(AssertUnwindSafe(make)).map_err(|e| {
                e.downcast_ref::<String>()
                    .cloned()
                    .or_else(|| e.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                    .unwrap_or_else(|| "a panic without a message".to_owned())
            })
        });
        match result {
            Ok(run) => run,
            Err(failure) => panic!(
                "{property}: the shared run `{}` failed once and is not made again: {failure}",
                self.name
            ),
        }
    }
}

/// A fresh scratch directory under the temporary directory, as the tests
/// before `AET-WO-1909` made theirs.
pub fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1909-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// Every gate `verify` can run (`AET-WO-1609`): each has a row in every
/// verdict's `ran` table, judged, off or not reached.
#[allow(dead_code)]
pub const GATES: [&str; 12] = [
    "erc",
    "drc",
    "support",
    "extract",
    "prove",
    "build",
    "simulate",
    "gpio",
    "adc",
    "bridge",
    "campaign",
    "attribute",
];

/// A verdict's `ran` table, checked whole (`AET-WO-1609` §2): each row is a
/// gate or a part with exactly one of `judged` (a count), `off` or
/// `not_reached` (each a non-empty text); every gate in `gates` has its row
/// (by gate, harness and scenario) with its own `judged` when it ran; every
/// gate of [`GATES`] has at least one row. Returns each row as
/// `(gate, "judged n" | "off: key" | "not reached: why")`, harness and
/// scenario appended after a `#`.
#[allow(dead_code)]
pub fn ran(v: &Value) -> Vec<(String, String)> {
    let rows = v["ran"]
        .as_array()
        .unwrap_or_else(|| panic!("no ran table: {v}"));
    let mut out = Vec::new();
    for r in rows {
        let o = r.as_object().unwrap();
        let states: Vec<String> = ["judged", "off", "not_reached"]
            .iter()
            .filter_map(|k| {
                o.get(*k).map(|x| match *k {
                    "judged" => format!("judged {}", x.as_u64().unwrap_or_else(|| panic!("{r}"))),
                    "off" => format!("off: {}", x.as_str().filter(|t| !t.is_empty()).unwrap()),
                    _ => format!(
                        "not reached: {}",
                        x.as_str().filter(|t| !t.is_empty()).unwrap()
                    ),
                })
            })
            .collect();
        assert_eq!(states.len(), 1, "exactly one state: {r}");
        for k in o.keys() {
            assert!(
                [
                    "gate",
                    "harness",
                    "scenario",
                    "judged",
                    "off",
                    "not_reached"
                ]
                .contains(&k.as_str()),
                "{k} in {r}"
            );
        }
        let mut gate = o["gate"].as_str().unwrap().to_owned();
        for k in ["harness", "scenario"] {
            if let Some(x) = o.get(k) {
                gate = format!("{gate}#{}", x.as_str().unwrap());
            }
        }
        out.push((gate, states[0].clone()));
    }
    for g in v["gates"].as_array().unwrap() {
        let mut name = g["gate"].as_str().unwrap().to_owned();
        for k in ["harness", "scenario"] {
            if let Some(x) = g.get(k) {
                name = format!("{name}#{}", x.as_str().unwrap());
            }
        }
        let row = out
            .iter()
            .find(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("gate {name} has no row: {:?}", out));
        if let Some(n) = g["judged"].as_u64() {
            assert_eq!(row.1, format!("judged {n}"), "{name}");
        }
    }
    for g in GATES {
        assert!(
            out.iter()
                .any(|(n, _)| n == g || n.starts_with(&format!("{g}#"))),
            "no `{g}` row: {out:?}"
        );
    }
    out
}

/// The state of the one row named `gate` in [`ran`]'s rows.
#[allow(dead_code)]
pub fn row<'r>(rows: &'r [(String, String)], gate: &str) -> &'r str {
    let hits: Vec<&(String, String)> = rows.iter().filter(|(n, _)| n == gate).collect();
    assert_eq!(hits.len(), 1, "{gate}: {rows:?}");
    &hits[0].1
}
