//! `AET-WO-3300`, tier 1: every refusal of `verify --fab-out` and
//! `--against-fab` that comes before any gate. Each call runs with an empty
//! `PATH`, so a refusal that waited for a tool would show as exit 4 instead.
//! The package itself is tier 2 (`board_tier2.rs`); the stripping of KiCad's
//! clock lines is pinned by `fab.rs`'s unit tests on committed KiCad output.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const SPEC: &str = "examples/sentinel-01-minimal.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-3300-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_file(&dir);
    dir
}

/// Runs the binary from the repository root with an empty `PATH`.
fn run(args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout}");
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// §1.2: `--fab-out` must be an empty directory or absent. A used directory,
/// a file and a symlink (even to an empty directory) are refused with exit
/// 3 before any gate, and nothing in or behind them changes.
#[test]
fn an_unusable_fab_out_is_refused_before_any_gate_and_left_untouched() {
    let used = scratch("used");
    fs::create_dir_all(&used).unwrap();
    fs::write(used.join("keep"), "x").unwrap();
    let file = scratch("file");
    fs::write(&file, "x").unwrap();
    let target = scratch("target");
    fs::create_dir_all(&target).unwrap();
    let link = scratch("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    for dir in [&used, &file, &link] {
        let (code, v) = run(&["verify", "--spec", SPEC, "--fab-out", s(dir)]);
        assert_eq!(code, Some(3), "{}: {v}", dir.display());
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["kind"], "UnwritableOutput", "{v}");
    }
    assert_eq!(fs::read_dir(&used).unwrap().count(), 1);
    assert_eq!(fs::read(used.join("keep")).unwrap(), b"x");
    assert_eq!(fs::read(&file).unwrap(), b"x");
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    for p in [&used, &target] {
        fs::remove_dir_all(p).unwrap();
    }
    fs::remove_file(&file).unwrap();
    fs::remove_file(&link).unwrap();
}

/// A run that cannot pass writes nothing: an absent `--fab-out` is not
/// created when a gate's tool is missing (exit 4), and the two fab flags
/// are one or the other.
#[test]
fn a_run_that_cannot_pass_creates_nothing_and_the_flags_exclude_each_other() {
    let absent = scratch("absent");
    let (code, v) = run(&["verify", "--spec", SPEC, "--fab-out", s(&absent)]);
    assert_eq!(code, Some(4), "{v}");
    assert!(
        !absent.exists(),
        "--fab-out was created by a run that stopped"
    );

    let (code, v) = run(&["verify", "--spec", SPEC, "--against-fab", s(&absent)]);
    assert_eq!(code, Some(3), "{v}");

    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--fab-out",
        s(&absent),
        "--against-fab",
        s(&absent),
    ]);
    assert_eq!(code, Some(64), "{v}");
    assert!(!absent.exists());
}
