//! Tier 1 of `AET-WO-0300c` §1.5: the reaper and the guardian learn the CLI's pid from the CLI
//! (`--cli-pid`), never by sampling their parent, and the guardian starts before its directory.

mod common;

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use common::{alive, eventually};

const BIN: &str = env!("CARGO_BIN_EXE_aethelgard");
const REAPER: &str = "--aethelgard-internal-reaper";
const GUARD: &str = "--aethelgard-internal-guard";
const SYSTEM_DIRS: [&str; 6] = ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];

struct Dir(PathBuf);

impl Dir {
    fn new(label: &str) -> Dir {
        let p = std::env::temp_dir().join(format!("aet-life-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Dir(fs::canonicalize(p).unwrap())
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// The pid of a process that has exited and been reaped.
fn dead_pid() -> u32 {
    let mut child = Command::new("/bin/true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// Runs the reaper with `cli_pid` (or no pid argument at all) on a tool that creates `marker`.
/// Returns its exit code and, read while it is a zombie, the minor page faults of the children
/// it waited for (`cminflt`, `proc(5)`): zero exactly when it started no process at all.
fn reaper(dir: &Path, cli_pid: Option<&str>, marker: &Path) -> (Option<i32>, u64) {
    let mut args: Vec<OsString> = vec![REAPER.into()];
    if let Some(pid) = cli_pid {
        args.extend(["--cli-pid".into(), pid.into()]);
    }
    args.extend(["--status".into(), dir.join("status").into()]);
    for d in SYSTEM_DIRS {
        args.extend(["--ro".into(), d.into()]);
    }
    args.extend(["--rw".into(), dir.into(), "--".into()]);
    args.extend([
        "/bin/sh".into(),
        "-c".into(),
        ": > \"$0\"".into(),
        marker.into(),
    ]);
    let mut child = Command::new(BIN)
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut child_faults = None;
    eventually("the reaper to exit", || {
        let stat = fs::read_to_string(format!("/proc/{}/stat", child.id())).unwrap();
        let rest: Vec<&str> = stat
            .rsplit_once(')')
            .unwrap()
            .1
            .split_whitespace()
            .collect();
        // Field 3 is the state, field 11 cminflt.
        child_faults = (rest[0] == "Z").then(|| rest[8].parse().unwrap());
        child_faults.is_some()
    });
    (child.wait().unwrap().code(), child_faults.unwrap())
}

#[test]
fn a_reaper_started_with_a_dead_clis_pid_exits_70_and_spawns_nothing() {
    let dir = Dir::new("reaper-dead");
    let marker = dir.0.join("ran");
    let (code, child_faults) = reaper(&dir.0, Some(&dead_pid().to_string()), &marker);
    assert_eq!(code, Some(70));
    assert_eq!(child_faults, 0, "the reaper started a process");
    // The trampoline creates the status file first thing; the tool creates the marker.
    assert!(!dir.0.join("status").exists(), "the trampoline started");
    assert!(!marker.exists(), "the tool ran");

    // The control: with its real parent's pid the same reaper runs the tool.
    let (code, child_faults) = reaper(&dir.0, Some(&std::process::id().to_string()), &marker);
    assert_eq!(code, Some(0));
    assert!(
        child_faults > 0,
        "a started trampoline must show in cminflt"
    );
    assert!(marker.exists(), "the tool did not run with a live CLI");
}

#[test]
fn a_missing_or_malformed_cli_pid_is_refused_with_64() {
    let dir = Dir::new("reaper-malformed");
    let marker = dir.0.join("ran");
    let bad = [
        "",
        "0",
        "-1",
        "+1",
        " 1",
        "1x",
        "abc",
        "2147483648",
        "99999999999",
    ];
    for pid in bad.iter().map(|p| Some(*p)).chain([None]) {
        let (code, _) = reaper(&dir.0, pid, &marker);
        assert_eq!(code, Some(64), "reaper, pid {pid:?}");
        assert!(!dir.0.join("status").exists() && !marker.exists());
    }
    // A dead pid where one is well formed: a guardian that wrongly accepted its arguments
    // exits 0 at once rather than waiting on this test.
    let dead = dead_pid().to_string();
    let target = dir.0.join("aethelgard-x");
    let target = target.to_str().unwrap();
    let guards: [&[&str]; 7] = [
        &[target],
        &["--cli-pid", target],
        &["--cli-pid", "abc", target],
        &["--cli-pid", "0", target],
        &["--cli-pid", &dead, target, "extra"],
        &["--pid", &dead, target],
        &["--cli-pid", &dead, "/tmp/not-ours"],
    ];
    for args in guards {
        let out = Command::new(BIN).arg(GUARD).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(64), "guardian {args:?}: {out:?}");
    }
}

#[test]
fn a_guardian_started_before_its_directory_removes_it_when_its_parent_dies() {
    let dir = Dir::new("guard-dies");
    let target = dir.0.join("aethelgard-guarded");
    // The parent starts the guardian, creates the directory only once the guardian is running,
    // lets it see the directory, then dies by SIGKILL.
    let out = Command::new("/bin/sh")
        .args([
            "-c",
            "\"$0\" --aethelgard-internal-guard --cli-pid $$ \"$1\" </dev/null >/dev/null 2>&1 &
             echo $!; sleep 0.3; mkdir \"$1\"; sleep 0.3; kill -9 $$",
            BIN,
        ])
        .arg(&target)
        .output()
        .unwrap();
    assert_eq!(out.status.code(), None, "{out:?}");
    let guardian: u32 = String::from_utf8(out.stdout)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    eventually("the guardian to remove the directory", || !target.exists());
    eventually("the guardian to exit", || !alive(guardian));
}

#[test]
fn a_guardian_waits_for_its_directory_and_exits_0_when_its_parent_removes_it() {
    let dir = Dir::new("guard-clean");
    let target = dir.0.join("aethelgard-guarded");
    let mut guardian = Command::new(BIN)
        .args([GUARD, "--cli-pid", &std::process::id().to_string()])
        .arg(&target)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(
        guardian.try_wait().unwrap().is_none(),
        "the guardian left before its directory existed"
    );
    fs::create_dir(&target).unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(guardian.try_wait().unwrap().is_none(), "left too early");
    fs::remove_dir(&target).unwrap();
    eventually("the guardian to exit", || {
        guardian.try_wait().unwrap().is_some()
    });
    assert_eq!(guardian.wait().unwrap().code(), Some(0));
}
