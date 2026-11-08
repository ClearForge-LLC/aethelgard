//! AET-WO-1600 spike: a stand-in tool run by the real CLI inside the real
//! sandbox, built as `examples/spike_1600_probe.rs`. The build deletes it.
//!
//! As `kicad-cli` (`check drc`): it tries each S1 channel it can reach without
//! `unsafe` and reports every attempt as a finding, `name=ok`, `name=errno=N`
//! or `name=fail:<what the helper said>`. System V IPC and `unshare` go
//! through util-linux's `ipcmk`, `ipcrm` and `unshare`; the pidfd calls go
//! through `rustix`; `clone3` is exercised by spawning a thread and a process,
//! which glibc does with `clone3` and retries with `clone` only on `ENOSYS`.
//!
//! As `renode` (`simulate`): it prints its capability sets, `NoNewPrivs` and
//! seccomp state from `/proc/self/status` (Landlock lets Renode read `/proc`)
//! as one `[ERROR]` line and exits 1, so the CLI quotes the line in its
//! `ToolError`.

use std::fs;
use std::io;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::thread;

use rustix::process::{
    Pid, PidfdFlags, PidfdGetfdFlags, Signal, pidfd_getfd, pidfd_open, pidfd_send_signal,
};
use serde_json::json;

pub fn main() {
    let args: Vec<String> = std::env::args().collect();
    let me = Path::new(args.first().map_or("", String::as_str))
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_owned();
    if me == "renode" {
        renode(&args);
    } else {
        kicad(&args);
    }
}

fn renode(args: &[String]) {
    if args.get(1).map(String::as_str) == Some("--version") {
        println!("Renode v1.17.0");
        return;
    }
    let status = fs::read_to_string("/proc/self/status").unwrap_or_default();
    let field = |key: &str| {
        status
            .lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix(':'))
            .map(|v| v.split_whitespace().next().unwrap_or("").to_owned())
            .unwrap_or_else(|| "?".to_owned())
    };
    println!(
        "[ERROR] spike-1600 uid={} inh={} prm={} eff={} bnd={} amb={} nnp={} seccomp={} filters={}",
        field("Uid"),
        field("CapInh"),
        field("CapPrm"),
        field("CapEff"),
        field("CapBnd"),
        field("CapAmb"),
        field("NoNewPrivs"),
        field("Seccomp"),
        field("Seccomp_filters"),
    );
    exit(1);
}

fn errno<T, E>(r: Result<T, E>) -> String
where
    io::Error: From<E>,
{
    match r.map_err(io::Error::from) {
        Ok(_) => "ok".to_owned(),
        Err(e) => match e.raw_os_error() {
            Some(n) => format!("errno={n}"),
            None => format!("error={e}"),
        },
    }
}

fn printable(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c.is_ascii_graphic() || c == ' ' {
                c
            } else {
                '?'
            }
        })
        .take(120)
        .collect()
}

/// Runs a helper; `ok` on exit 0, else `fail:` and its first stderr line.
fn helper(program: &str, args: &[&str]) -> (String, String) {
    match Command::new(program)
        .args(args)
        // Not Stdio::null: Landlock gives this persona no /dev/null.
        .stdin(Stdio::piped())
        .output()
    {
        Ok(out) if out.status.success() => (
            "ok".to_owned(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ),
        Ok(out) => {
            let said = String::from_utf8_lossy(&out.stderr);
            let line = said.lines().next().unwrap_or("").trim().to_owned();
            let how = match (out.status.code(), out.status.signal()) {
                (Some(c), _) => format!("exit={c}"),
                (None, Some(s)) => format!("signal={s}"),
                _ => "unknown".to_owned(),
            };
            (format!("fail:{how}:{}", printable(&line)), String::new())
        }
        Err(e) => (
            format!("spawn-{}", errno::<(), io::Error>(Err(e))),
            String::new(),
        ),
    }
}

/// `ipcmk <make>`, then, if it made an object, `ipcrm <remove> <id>`, so a
/// probe on a host leaves nothing behind in the host's IPC namespace.
fn sysv(make: &[&str], remove: &str) -> String {
    let (made, stdout) = helper("/usr/bin/ipcmk", make);
    if made != "ok" {
        return made;
    }
    let id = stdout.rsplit(':').next().unwrap_or("").trim().to_owned();
    let (removed, _) = helper("/usr/bin/ipcrm", &[remove, &id]);
    format!("ok;rm={removed}")
}

fn attempts() -> Vec<(&'static str, String)> {
    let mut v = vec![
        (
            "thread_spawn",
            errno(thread::Builder::new().spawn(|| ()).map(|h| h.join())),
        ),
        ("process_spawn", helper("/usr/bin/true", &[]).0),
        ("shmget", sysv(&["-M", "4096"], "-m")),
        ("msgget", sysv(&["-Q"], "-q")),
        ("semget", sysv(&["-S", "1"], "-s")),
        (
            "unshare_user",
            helper("/usr/bin/unshare", &["--user", "/usr/bin/true"]).0,
        ),
    ];
    // The pidfd calls, on this probe's own child.
    match Command::new("/usr/bin/sleep")
        .arg("30")
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            let pid = Pid::from_child(&child);
            let fd = pidfd_open(pid, PidfdFlags::empty());
            match &fd {
                Ok(fd) => {
                    v.push(("pidfd_open", "ok".to_owned()));
                    v.push((
                        "pidfd_send_signal",
                        errno(pidfd_send_signal(fd, Signal::CONT)),
                    ));
                    v.push((
                        "pidfd_getfd",
                        errno(pidfd_getfd(fd, 0, PidfdGetfdFlags::empty())),
                    ));
                }
                Err(e) => {
                    v.push(("pidfd_open", format!("errno={}", e.raw_os_error())));
                    v.push(("pidfd_send_signal", "untried".to_owned()));
                    v.push(("pidfd_getfd", "untried".to_owned()));
                }
            }
            let _ = child.kill();
            let _ = child.wait();
        }
        Err(e) => v.push(("pidfd_child", errno::<(), io::Error>(Err(e)))),
    }
    v
}

fn kicad(args: &[String]) {
    if args.get(1).map(String::as_str) == Some("version") {
        println!("10.0.7");
        return;
    }
    let report = args
        .windows(2)
        .find(|w| w[0] == "-o")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| exit(99));
    let violations: Vec<_> = attempts()
        .iter()
        .map(|(name, r)| json!({"type": "probe", "severity": "error", "description": format!("{name}={r}"), "items": []}))
        .collect();
    let doc = json!({"$schema": "https://schemas.kicad.org/drc.v1.json", "coordinate_units": "mm", "date": "x",
        "ignored_checks": [], "included_severities": ["error", "warning"], "kicad_version": "10.0.7", "source": "b",
        "violations": violations, "unconnected_items": [], "schematic_parity": []});
    fs::write(report, doc.to_string()).unwrap_or_else(|_| exit(96));
    exit(5);
}
