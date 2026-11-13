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
const SCENARIO: &str = "scenarios/sentinel-01/pullup-2k2.json";

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
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIO,
            "--fab-out",
            s(dir),
        ]);
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
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIO,
        "--fab-out",
        s(&absent),
    ]);
    assert_eq!(code, Some(4), "{v}");
    assert!(
        !absent.exists(),
        "--fab-out was created by a run that stopped"
    );

    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIO,
        "--against-fab",
        s(&absent),
    ]);
    assert_eq!(code, Some(3), "{v}");

    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIO,
        "--fab-out",
        s(&absent),
        "--against-fab",
        s(&absent),
    ]);
    assert_eq!(code, Some(64), "{v}");
    assert!(!absent.exists());
}

/// Adversarial A1: an existing `--fab-out` that another uid could write is
/// refused before any gate: group- or other-writable, or (as root, where a
/// test can make one) owned by another uid.
#[test]
fn a_fab_out_another_uid_could_write_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("shared");
    fs::create_dir_all(&dir).unwrap();
    for mode in [0o775, 0o757] {
        fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).unwrap();
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIO,
            "--fab-out",
            s(&dir),
        ]);
        assert_eq!(code, Some(3), "{mode:o}: {v}");
        assert_eq!(v["error"]["kind"], "UnwritableOutput");
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    if rustix::process::geteuid().is_root() {
        std::os::unix::fs::chown(&dir, Some(65534), None).unwrap();
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIO,
            "--fab-out",
            s(&dir),
        ]);
        assert_eq!(code, Some(3), "{v}");
        assert_eq!(v["error"]["kind"], "UnwritableOutput");
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// `--against` that differs fails before any gate; with `--fab-out` the
/// verdict says nothing was written, and nothing is.
#[test]
fn a_tree_that_differs_writes_no_package_and_says_so() {
    let tree = scratch("against-empty");
    fs::create_dir_all(&tree).unwrap();
    let out = scratch("against-out");
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIO,
        "--against",
        s(&tree),
        "--fab-out",
        s(&out),
    ]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["against"], "differs");
    assert_eq!(v["fab"]["status"], "not_written");
    assert!(!out.exists());
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIO,
        "--against",
        s(&tree),
        "--against-fab",
        s(&tree),
    ]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["fab"]["status"], "not_derived");
    assert_eq!(v["against_fab"], "not_compared");
    fs::remove_dir_all(&tree).unwrap();
}

/// Ruling D3 on PR #103: `INV-03` counts the fault campaign among the gates
/// of "verified", so each fab flag requires `--scenarios`; without it the
/// call is a usage error, exit 64, and nothing is created.
#[test]
fn each_fab_flag_requires_scenarios() {
    let absent = scratch("no-scenarios");
    for flag in ["--fab-out", "--against-fab"] {
        let (code, v) = run(&["verify", "--spec", SPEC, flag, s(&absent)]);
        assert_eq!(code, Some(64), "{flag}: {v}");
        assert_eq!(v["error"]["kind"], "UsageError", "{flag}: {v}");
        assert!(!absent.exists(), "{flag}");
    }
}
