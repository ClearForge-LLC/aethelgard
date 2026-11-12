//! Tier 1 for AET-WO-1600 §1.3: the non-socket channels to other processes,
//! and the capability drop, probed by tier 1's stand-in tool
//! (`examples/sandbox_probe.rs`, `mode=channels`) under the real CLI, so the
//! real trampoline, Landlock, filter and drop.
//!
//! Each family the ruling on PR #51 names as reachable without `unsafe` is
//! tried outside first (the control), then inside, where the filter refuses it
//! with EPERM (1). The families with no safe path (`setns`, `clone` with a
//! namespace bit, `ptrace`, `process_vm_*`, `kcmp`, the keyring, `bpf`,
//! `perf_event_open`, `userfaultfd`) are covered by the filter's exhaustive
//! interpreter test and its pinned sequence in `src/run.rs`. A missing helper
//! fails by name; nothing here is skipped.

use std::collections::BTreeMap;
use std::fs::{self, File};
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use rustix::process::{
    Pid, PidfdFlags, PidfdGetfdFlags, Resource, Signal, getrlimit, pidfd_getfd, pidfd_open,
    pidfd_send_signal, prlimit,
};
use rustix::thread::{sched_getaffinity, sched_setaffinity};
use serde_json::Value;

const EPERM: &str = "fail:Operation not permitted";

fn probe() -> PathBuf {
    let probe = Path::new(env!("CARGO_BIN_EXE_aethelgard"))
        .parent()
        .unwrap()
        .join("examples/sandbox_probe");
    assert!(
        probe.exists(),
        "{} is missing: `cargo test` builds it",
        probe.display()
    );
    probe
}

/// A scratch directory removed on drop.
struct Dir(PathBuf);
impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A process to point pid-taking calls at.
fn sleeper() -> Child {
    Command::new("/usr/bin/sleep")
        .arg("30")
        .stdin(Stdio::null())
        .spawn()
        .unwrap()
}

/// Runs the probe as `kicad-cli` under `check drc`; its findings by name.
fn inside() -> BTreeMap<String, String> {
    let dir = Dir(std::env::temp_dir().join(format!(
        "aet-channels-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    )));
    let toolbox = dir.0.join("toolbox");
    fs::create_dir_all(&toolbox).unwrap();
    symlink(probe(), toolbox.join("kicad-cli")).unwrap();
    let board = dir.0.join("channels.kicad_pcb");
    // A process outside the sandbox, for the calls that take a pid.
    let mut victim = sleeper();
    // A minimal board, because `check drc` parses the copy before any tool
    // runs (AET-WO-1300 B5); the probe reads its lines inside it.
    fs::write(
        &board,
        format!("(kicad_pcb\nmode=channels\nvictim={}\n)\n", victim.id()),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["check", "drc", "--board"])
        .arg(&board)
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("PATH", &toolbox)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let _ = victim.kill();
    let _ = victim.wait();
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    assert_eq!(out.status.code(), Some(1), "{v}");
    v["findings"]
        .as_array()
        .unwrap_or_else(|| panic!("no findings: {v}"))
        .iter()
        .map(|f| {
            let m = f["message"].as_str().unwrap();
            let (k, r) = m.split_once('=').unwrap();
            (k.to_owned(), r.to_owned())
        })
        .collect()
}

/// Runs a helper outside the sandbox; `Ok(stdout)` or `Err(stderr)`.
fn outside(program: &str, args: &[&str]) -> Result<String, String> {
    assert!(Path::new(program).exists(), "{program} is missing");
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).into_owned())
    }
}

fn in_a_container() -> bool {
    Path::new("/.dockerenv").exists() || Path::new("/run/.containerenv").exists()
}

#[test]
fn system_v_ipc_is_refused_inside_and_works_outside() {
    for (make, remove) in [
        (&["-M", "4096"][..], "-m"),
        (&["-Q"][..], "-q"),
        (&["-S", "1"][..], "-s"),
    ] {
        let made = outside("/usr/bin/ipcmk", make).unwrap();
        let id = made.rsplit(':').next().unwrap().trim();
        outside("/usr/bin/ipcrm", &[remove, id]).unwrap();
    }
    let got = inside();
    for call in ["shmget", "msgget", "semget"] {
        assert_eq!(got[call], EPERM, "{call}: {got:?}");
    }
}

#[test]
fn unshare_is_refused_inside() {
    // Docker's own profile refuses unshare without CAP_SYS_ADMIN, so inside
    // the image the control cannot succeed; on a host it must.
    let control = outside("/usr/bin/unshare", &["--user", "/usr/bin/true"]);
    if !in_a_container() {
        control.unwrap();
    }
    let got = inside();
    assert_eq!(got["unshare_user"], EPERM, "{got:?}");
}

#[test]
fn pidfd_calls_are_refused_inside_and_reach_the_kernel_outside() {
    // Outside: a pidfd for a child opens; on a plain descriptor the other two
    // reach the kernel and get EBADF (9), not EPERM.
    let mut child = sleeper();
    pidfd_open(Pid::from_child(&child), PidfdFlags::empty()).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    let plain = File::open("/etc/hostname")
        .or_else(|_| File::open("/etc/passwd"))
        .unwrap();
    assert_eq!(
        pidfd_send_signal(&plain, Signal::CONT)
            .unwrap_err()
            .raw_os_error(),
        9
    );
    // Docker's default profile refuses pidfd_getfd without CAP_SYS_PTRACE
    // (the spike's S5), so inside the image the control gets EPERM too; on a
    // host it must reach the kernel.
    let getfd = pidfd_getfd(&plain, 0, PidfdGetfdFlags::empty()).unwrap_err();
    if !in_a_container() {
        assert_eq!(getfd.raw_os_error(), 9);
    }
    let got = inside();
    for call in ["pidfd_open", "pidfd_send_signal", "pidfd_getfd"] {
        assert_eq!(got[call], "errno=1", "{call}: {got:?}");
    }
}

/// `clone3` returns ENOSYS, so glibc falls back to `clone`: a tool can still
/// start threads and children (EPERM would stop both; measured in the spike).
#[test]
fn threads_and_children_still_start_inside() {
    let got = inside();
    assert_eq!(got["thread_spawn"], "ok", "{got:?}");
    assert_eq!(got["process_spawn"], "ok", "{got:?}");
}

/// The test process's own sets, from /proc/self/status, as hex masks.
fn own_caps() -> BTreeMap<String, u64> {
    fs::read_to_string("/proc/self/status")
        .unwrap()
        .lines()
        .filter_map(|l| {
            let (k, v) = l.split_once(':')?;
            let k = k.strip_prefix("Cap")?;
            Some((k.to_owned(), u64::from_str_radix(v.trim(), 16).ok()?))
        })
        .collect()
}

/// The tool holds no capability and runs with `no_new_privs`. Where the caller
/// holds `CAP_SETPCAP` (root in the image: this test in the `tools` job), the
/// caller holds capabilities before the drop and the tool's bounding set is
/// empty after it, which pins the ruled order: bounding before capset.
#[test]
fn the_tool_holds_no_capability() {
    const CAP_SETPCAP: u64 = 1 << 8;
    let before = own_caps();
    let got = inside();
    let caps: BTreeMap<&str, &str> = got["caps"]
        .split(' ')
        .map(|kv| kv.split_once(':').unwrap())
        .collect();
    for set in ["eff", "prm", "inh", "amb"] {
        assert_eq!(caps[set], "0", "{set}: {got:?}, the caller held {before:?}");
    }
    assert_eq!(caps["nnp"], "true", "{got:?}");
    if before["Eff"] & CAP_SETPCAP != 0 {
        assert_ne!(before["Prm"], 0, "{before:?}");
        assert_eq!(caps["bnd"], "0", "the bounding set survived: {got:?}");
    } else {
        // A user cannot empty it, and under no_new_privs need not (F3).
        assert_eq!(
            u64::from_str_radix(caps["bnd"], 16).unwrap(),
            before["Bnd"],
            "{got:?}"
        );
    }
}

/// Another process's limits and CPU affinity are refused; the tool's own are
/// not. `prlimit64` on another pid can kill it (`RLIMIT_CPU`) past Landlock's
/// signal scoping (the adversarial pass, F10).
#[test]
fn another_process_limits_and_affinity_are_refused_and_the_tools_own_are_not() {
    let mut child = sleeper();
    let pid = Some(Pid::from_child(&child));
    prlimit(pid, Resource::Nofile, getrlimit(Resource::Nofile)).unwrap();
    sched_setaffinity(pid, &sched_getaffinity(None).unwrap()).unwrap();
    let _ = child.kill();
    let _ = child.wait();
    let got = inside();
    for (call, want) in [
        ("prlimit_self", "ok"),
        ("prlimit_victim", "errno=1"),
        ("setaffinity_self", "ok"),
        ("setaffinity_victim", "errno=1"),
    ] {
        assert_eq!(got[call], want, "{call}: {got:?}");
    }
}
