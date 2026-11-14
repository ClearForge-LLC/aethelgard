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
        "erc" | "drc" => reads.extend(super::tool_dirs("kicad-cli")),
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
