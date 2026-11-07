//! Tier 1 of `AET-WO-0300c` §1.5: the reaper and the guardian learn the CLI's pid from the CLI
//! (`--cli-pid`), never by sampling their parent, and the guardian starts before its directory.
//! A CLI told its parent's pid checks it once its parent-death signal is armed.

mod common;

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use common::{Scratch, alive, eventually, example, stat_fields};

const BIN: &str = env!("CARGO_BIN_EXE_aethelgard");
const REAPER: &str = "--aethelgard-internal-reaper";
const GUARD: &str = "--aethelgard-internal-guard";
const SYSTEM_DIRS: [&str; 6] = ["/usr", "/lib", "/lib64", "/bin", "/sbin", "/etc"];
const PARENT_PID_VAR: &str = "AETHELGARD_INTERNAL_PARENT_PID";

/// The pid of a process that has exited and been reaped.
fn dead_pid() -> u32 {
    let mut child = Command::new("/bin/true").spawn().unwrap();
    let pid = child.id();
    child.wait().unwrap();
    pid
}

/// Waits for `child` to exit. Returns its exit code and, read while it is a zombie, the minor
/// page faults of the children it waited for (`cminflt`, `proc(5)`): zero exactly when it
/// started no process at all.
fn exit_and_child_faults(mut child: Child) -> (Option<i32>, u64) {
    let mut child_faults = None;
    eventually("the process to exit", || {
        let f = stat_fields(child.id()).unwrap();
        // Field 3 is the state, field 11 cminflt.
        child_faults = (f[0] == "Z").then(|| f[8].parse().unwrap());
        child_faults.is_some()
    });
    (child.wait().unwrap().code(), child_faults.unwrap())
}

/// Runs the reaper with `pid_args` before its other arguments, on a tool that creates `marker`.
fn reaper(dir: &Path, pid_args: &[&str], marker: &Path) -> (Option<i32>, u64) {
    let mut args: Vec<OsString> = vec![REAPER.into()];
    args.extend(pid_args.iter().map(OsString::from));
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
    let child = Command::new(BIN)
        .args(args)
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    exit_and_child_faults(child)
}

#[test]
fn a_reaper_started_with_a_dead_clis_pid_exits_70_and_spawns_nothing() {
    let dir = Scratch::new("reaper-dead");
    let marker = dir.0.join("ran");
    let (code, child_faults) = reaper(&dir.0, &["--cli-pid", &dead_pid().to_string()], &marker);
    assert_eq!(code, Some(70));
    assert_eq!(child_faults, 0, "the reaper started a process");
    // The trampoline creates the status file first thing; the tool creates the marker.
    assert!(!dir.0.join("status").exists(), "the trampoline started");
    assert!(!marker.exists(), "the tool ran");

    // The control: with its real parent's pid the same reaper runs the tool.
    let me = std::process::id().to_string();
    let (code, child_faults) = reaper(&dir.0, &["--cli-pid", &me], &marker);
    assert_eq!(code, Some(0));
    assert!(
        child_faults > 0,
        "a started trampoline must show in cminflt"
    );
    assert!(marker.exists(), "the tool did not run with a live CLI");
}

#[test]
fn a_missing_or_malformed_cli_pid_is_refused_with_64() {
    let dir = Scratch::new("reaper-malformed");
    let marker = dir.0.join("ran");
    let me = std::process::id().to_string();
    let bad = [
        "",
        "0",
        "01",
        "-1",
        "+1",
        " 1",
        "1x",
        "abc",
        "2147483648",
        "99999999999",
    ];
    let mut cases: Vec<Vec<&str>> = bad.iter().map(|p| vec!["--cli-pid", p]).collect();
    cases.push(vec![]);
    cases.push(vec!["--cli-pid"]);
    cases.push(vec!["--cli-pid", &me, "--cli-pid", &me]);
    for args in &cases {
        let (code, _) = reaper(&dir.0, args, &marker);
        assert_eq!(code, Some(64), "reaper {args:?}");
        assert!(!dir.0.join("status").exists() && !marker.exists());
    }
    // A dead pid where one is well formed: a guardian that wrongly accepted its arguments
    // exits 0 at once rather than waiting on this test.
    let dead = dead_pid().to_string();
    let target = dir.0.join("aethelgard-x");
    let target = target.to_str().unwrap();
    // `file_name` reads `aethelgard-link` past a trailing `/` or `/.`, through which
    // `remove_dir_all` would empty the directory the link points at.
    let victim = dir.0.join("victim");
    fs::create_dir(&victim).unwrap();
    fs::write(victim.join("keep"), "x").unwrap();
    symlink(&victim, dir.0.join("aethelgard-link")).unwrap();
    let slash = format!("{}/aethelgard-link/", dir.0.display());
    let dot = format!("{}/aethelgard-link/.", dir.0.display());
    let guards: [&[&str]; 9] = [
        &[target],
        &["--cli-pid", target],
        &["--cli-pid", "abc", target],
        &["--cli-pid", &dead, target, "extra"],
        &["--pid", &dead, target],
        &["--cli-pid", &dead, "/tmp/not-ours"],
        &["--cli-pid", &dead, &slash],
        &["--cli-pid", &dead, &dot],
        &["--cli-pid", &dead, &format!("{target}/..")],
    ];
    for args in guards {
        let out = Command::new(BIN).arg(GUARD).args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(64), "guardian {args:?}: {out:?}");
    }
    assert!(
        victim.join("keep").exists(),
        "a guardian emptied a linked directory"
    );
}

/// The adversarial pass's finding 1: a parent that dies before the CLI arms its parent-death
/// signal never sends it. Told its parent's pid, as `mcp` tells it, the CLI then exits 70
/// before it starts anything or creates a directory.
#[test]
fn a_cli_told_a_dead_parents_pid_exits_70_and_starts_nothing() {
    let dir = Scratch::new("cli-dead-parent");
    let tb = dir.0.join("toolbox");
    let tmp = dir.0.join("tmp");
    fs::create_dir_all(&tb).unwrap();
    fs::create_dir(&tmp).unwrap();
    let marker = dir.0.join("ran");
    let stub = tb.join("kicad-cli");
    fs::write(
        &stub,
        format!("#!/bin/sh\n: > '{}'\necho 10.0.7\n", marker.display()),
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "x").unwrap();
    let cli = |parent: &str| {
        let mut cmd = Command::new(BIN);
        cmd.args(["check", "drc", "--board"])
            .arg(&board)
            .env_clear()
            .env("PATH", &tb)
            .env("TMPDIR", &tmp)
            .env(PARENT_PID_VAR, parent)
            .stdin(Stdio::null());
        cmd
    };
    let child = cli(&dead_pid().to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let (code, child_faults) = exit_and_child_faults(child);
    assert_eq!(code, Some(70));
    assert_eq!(child_faults, 0, "the CLI started a process");
    assert!(!marker.exists(), "the tool ran");
    assert_eq!(
        fs::read_dir(&tmp).unwrap().count(),
        0,
        "a directory was made"
    );

    for bad in ["", "0", "01", "x", "-1"] {
        let out = cli(bad).output().unwrap();
        assert_eq!(out.status.code(), Some(64), "{bad:?}: {out:?}");
        let refusal: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(refusal["error"]["kind"], "UsageError", "{refusal}");
    }

    // The control: the right pid lets the call through.
    let out = Command::new(BIN)
        .arg("validate")
        .arg("--file")
        .arg(example("sentinel-01-minimal.json"))
        .env(PARENT_PID_VAR, std::process::id().to_string())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
}

#[test]
fn a_guardian_started_before_its_directory_removes_it_when_its_parent_dies() {
    let dir = Scratch::new("guard-dies");
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
    let dir = Scratch::new("guard-clean");
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

/// Ruled on PR #34: without the variable the CLI arms no parent-death signal, so a CLI started
/// from a thread that ends while it runs (any multi-threaded caller) completes normally.
#[test]
fn a_cli_without_the_variable_outlives_the_thread_that_started_it() {
    let dir = Scratch::new("cli-thread");
    let tb = dir.0.join("toolbox");
    let tmp = dir.0.join("tmp");
    fs::create_dir_all(&tb).unwrap();
    fs::create_dir(&tmp).unwrap();
    // Slow to report its version, then a clean DRC report: a pass.
    let report = r#"{"$schema":"https://schemas.kicad.org/drc.v1.json","coordinate_units":"mm","date":"x","ignored_checks":[],"included_severities":["error","warning"],"kicad_version":"10.0.7","source":"b","violations":[],"unconnected_items":[],"schematic_parity":[]}"#;
    let stub = tb.join("kicad-cli");
    fs::write(
        &stub,
        format!(
            "#!/bin/sh\nif [ \"$1\" = version ]; then sleep 1; echo 10.0.7; exit 0; fi\n\
             out=; prev=\nfor a in \"$@\"; do [ \"$prev\" = -o ] && out=$a; prev=$a; done\n\
             printf '%s' '{report}' > \"$out\"\n"
        ),
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "x").unwrap();
    let mut cli = std::thread::spawn(move || {
        let cli = Command::new(BIN)
            .args(["check", "drc", "--board"])
            .arg(&board)
            .env_clear()
            .env("PATH", &tb)
            .env("TMPDIR", &tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        // Long enough for a CLI to have armed the signal, had it armed one.
        std::thread::sleep(Duration::from_millis(300));
        cli
    })
    .join()
    .unwrap();
    // The thread that started it has ended; the CLI is still in its version check.
    assert!(
        cli.try_wait().unwrap().is_none(),
        "the CLI ended with its thread"
    );
    let out = cli.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let verdict: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(verdict["verdict"], "pass", "{verdict}");
}

/// A writer that races the guardian: from a thread, it keeps recreating a few files in `dir`
/// (the guardian's own removal of them is lost at once), so a removal that reads the directory
/// and then removes it loses to a new entry. It stops when `stop` is set.
fn racing_writer(dir: &Path, stop: Arc<AtomicBool>) -> std::thread::JoinHandle<()> {
    let files: Vec<_> = ["a", "b", "c", "d"].iter().map(|n| dir.join(n)).collect();
    for f in &files {
        fs::File::create(f).unwrap();
    }
    std::thread::spawn(move || {
        while !stop.load(Ordering::Relaxed) {
            for f in &files {
                let _ = fs::File::create(f);
            }
        }
    })
}

/// Starts a guardian whose CLI is already gone (the pid it is given is not its parent's) over a
/// directory a writer is racing. Returns the guardian and the writer's stop flag and handle.
fn guardian_over_racing_writer(
    target: &Path,
) -> (Child, Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    fs::create_dir(target).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let writer = racing_writer(target, stop.clone());
    let guardian = Command::new(BIN)
        .args([GUARD, "--cli-pid", &dead_pid().to_string()])
        .arg(target)
        .stdin(Stdio::null())
        .spawn()
        .unwrap();
    (guardian, stop, writer)
}

/// `AET-WO-0300d` §1.1, §1.3: a writer that stops after a few hundred milliseconds does not
/// leave the directory behind. Run on `main`, the single unretried removal lost this race.
#[test]
fn a_guardian_removes_its_directory_though_a_writer_races_it() {
    let dir = Scratch::new("guard-race");
    let mut left = 0;
    for n in 0..10 {
        let target = dir.0.join(format!("aethelgard-race-{n}"));
        let (mut guardian, stop, writer) = guardian_over_racing_writer(&target);
        std::thread::sleep(Duration::from_millis(500));
        stop.store(true, Ordering::Relaxed);
        writer.join().unwrap();
        eventually("the guardian to exit", || {
            guardian.try_wait().unwrap().is_some()
        });
        let code = guardian.wait().unwrap().code();
        let present = target.exists();
        if present {
            left += 1;
            let _ = fs::remove_dir_all(&target);
        }
        eprintln!("try {n}: exit {code:?} left={present}");
        assert_eq!(code, Some(0), "try {n}");
    }
    assert_eq!(left, 0, "directories left behind, of 10 tries");
}

/// `AET-WO-0300d` §1.2: a writer that never stops costs the guardian its whole bound, and it
/// then says so: exit 70, directory left.
#[test]
fn a_guardian_that_cannot_empty_its_directory_exits_70_within_11_seconds() {
    let dir = Scratch::new("guard-bound");
    let target = dir.0.join("aethelgard-bound");
    let (mut guardian, stop, writer) = guardian_over_racing_writer(&target);
    let started = Instant::now();
    eventually("the guardian to give up", || {
        guardian.try_wait().unwrap().is_some()
    });
    let took = started.elapsed();
    stop.store(true, Ordering::Relaxed);
    writer.join().unwrap();
    assert_eq!(guardian.wait().unwrap().code(), Some(70));
    assert!(took >= Duration::from_secs(9), "gave up early: {took:?}");
    assert!(took <= Duration::from_secs(11), "gave up late: {took:?}");
    assert!(target.exists(), "exit 70 must leave the directory");
}
