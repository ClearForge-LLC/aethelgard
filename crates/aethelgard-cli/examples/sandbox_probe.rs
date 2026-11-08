//! A stand-in `kicad-cli` for tier 1's socket tests (AET-WO-0300a §1.4). It
//! answers `version`; asked for `pcb drc … -o REPORT BOARD`, it reads its
//! targets from the board file (the copy the CLI made), tries to reach each
//! one, and reports every attempt's result as a finding in a KiCad-shaped
//! report: `name=ok` or `name=errno=N`. Run by the real CLI, it runs inside
//! the real sandbox, so the results are what a hostile tool would get.
//!
//! A board whose first line is `mode=channels` asks instead for AET-WO-1600's
//! non-socket channels and the capability state (`channels()`).

#![forbid(unsafe_code)]

use std::fs::File;
use std::io;
use std::net::{TcpStream, UdpSocket};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixDatagram, UnixStream};
use std::path::Path;
use std::process::{Command, Stdio, exit};
use std::thread;

use rustix::process::{
    Pid, PidfdFlags, PidfdGetfdFlags, Resource, Signal, getrlimit, pidfd_getfd, pidfd_open,
    pidfd_send_signal, prlimit,
};
use rustix::thread::{
    CapabilitySet, capabilities, capability_is_in_ambient_set, capability_is_in_bounding_set,
    no_new_privs, sched_getaffinity, sched_setaffinity,
};

use serde_json::json;

fn result<T, E>(r: Result<T, E>) -> String
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

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("version") {
        println!("10.0.7");
        return;
    }
    let board_path = args.last().unwrap_or_else(|| exit(99));
    let board = std::fs::read_to_string(board_path).unwrap_or_else(|_| exit(98));
    if board.starts_with("mode=channels") {
        let victim = board
            .lines()
            .find_map(|l| l.strip_prefix("victim=")?.parse::<i32>().ok())
            .and_then(Pid::from_raw)
            .unwrap_or_else(|| exit(94));
        report(
            &report_path(&args),
            &channels(Path::new(board_path), victim),
        );
    }
    let target = |key: &str| {
        board
            .lines()
            .find_map(|l| l.strip_prefix(key)?.strip_prefix('=').map(str::to_owned))
            .unwrap_or_else(|| exit(97))
    };
    let attempts = [
        ("unix_path", result(UnixStream::connect(target("unix")))),
        (
            "unix_abstract",
            result(
                SocketAddr::from_abstract_name(target("abstract").as_bytes())
                    .and_then(|a| UnixStream::connect_addr(&a)),
            ),
        ),
        (
            "udp_sendto",
            result(UdpSocket::bind("127.0.0.1:0").and_then(|s| s.send_to(b"x", "127.0.0.1:9"))),
        ),
        (
            "tcp_connect",
            result(TcpStream::connect((
                "127.0.0.1",
                target("tcp").parse::<u16>().unwrap_or(1),
            ))),
        ),
        // A datagram pair can be pointed at any datagram socket outside.
        (
            "unix_dgram_pair_connect",
            result(
                UnixDatagram::pair()
                    .and_then(|(a, _)| a.connect(target("dgram")).and_then(|()| a.send(b"x"))),
            ),
        ),
        ("socketpair", result(UnixStream::pair())),
        ("io_uring_setup", result(io_uring::IoUring::new(2))),
    ];
    report(&report_path(&args), &attempts);
}

fn report_path(args: &[String]) -> String {
    args.windows(2)
        .find(|w| w[0] == "-o")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| exit(99))
}

/// Writes every attempt as a finding in a KiCad-shaped report and exits 5.
fn report(report: &str, attempts: &[(&str, String)]) -> ! {
    let violations: Vec<_> = attempts
        .iter()
        .map(|(name, r)| json!({"type": "probe", "severity": "error", "description": format!("{name}={r}"), "items": []}))
        .collect();
    let doc = json!({"$schema": "https://schemas.kicad.org/drc.v1.json", "coordinate_units": "mm", "date": "x",
        "ignored_checks": [], "included_severities": ["error", "warning"], "kicad_version": "10.0.7", "source": "b",
        "violations": violations, "unconnected_items": [], "schematic_parity": []});
    std::fs::write(report, doc.to_string()).unwrap_or_else(|_| exit(96));
    exit(5);
}

/// Runs a util-linux helper: `ok`, `missing:PATH` if it is not installed (the
/// test then fails by name, never skips), or `fail:` and the last part of its
/// first stderr line, which is the errno's text (`Operation not permitted`).
fn helper(program: &str, args: &[&str]) -> (String, String) {
    if !Path::new(program).exists() {
        return (format!("missing:{program}"), String::new());
    }
    // Not Stdio::null: Landlock gives this tool no /dev/null.
    match Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .output()
    {
        Ok(out) if out.status.success() => (
            "ok".to_owned(),
            String::from_utf8_lossy(&out.stdout).into_owned(),
        ),
        Ok(out) => {
            let said = String::from_utf8_lossy(&out.stderr);
            let first = said.lines().next().unwrap_or("");
            (
                format!("fail:{}", first.rsplit(": ").next().unwrap_or("").trim()),
                String::new(),
            )
        }
        Err(e) => (
            format!("spawn-{}", result::<(), io::Error>(Err(e))),
            String::new(),
        ),
    }
}

/// `ipcmk`, then `ipcrm` of what it made, so a host keeps nothing.
fn sysv(make: &[&str], remove: &str) -> String {
    let (made, stdout) = helper("/usr/bin/ipcmk", make);
    if made != "ok" {
        return made;
    }
    let id = stdout.rsplit(':').next().unwrap_or("").trim().to_owned();
    match helper("/usr/bin/ipcrm", &[remove, &id]).0.as_str() {
        "ok" => "ok".to_owned(),
        other => format!("ok;ipcrm={other}"),
    }
}

/// Every capability number for which `test` holds, as a hex mask.
fn mask(test: fn(CapabilitySet) -> rustix::io::Result<bool>) -> String {
    let mut m = 0u64;
    for n in 0..64 {
        match test(CapabilitySet::from_bits_retain(1 << n)) {
            Ok(true) => m |= 1 << n,
            Ok(false) => {}
            Err(_) => break,
        }
    }
    format!("{m:x}")
}

/// AET-WO-1600's channels, each by the safe path the ruling on PR #51 names,
/// and the capability state the tool runs with. `victim` is a process outside
/// the sandbox, for the calls that take a pid: each is tried on it and on this
/// probe itself.
fn channels(board: &Path, victim: Pid) -> Vec<(&'static str, String)> {
    // A descriptor that is not a pidfd: allowed, these calls give EBADF.
    let plain = File::open(board).unwrap_or_else(|_| exit(95));
    let mut v = vec![
        (
            "thread_spawn",
            result(thread::Builder::new().spawn(|| ()).map(|h| h.join())),
        ),
        ("process_spawn", helper("/usr/bin/true", &[]).0),
        ("shmget", sysv(&["-M", "4096"], "-m")),
        ("msgget", sysv(&["-Q"], "-q")),
        ("semget", sysv(&["-S", "1"], "-s")),
        (
            "unshare_user",
            helper("/usr/bin/unshare", &["--user", "/usr/bin/true"]).0,
        ),
        (
            "pidfd_send_signal",
            result(pidfd_send_signal(&plain, Signal::CONT)),
        ),
        (
            "pidfd_getfd",
            result(pidfd_getfd(&plain, 0, PidfdGetfdFlags::empty())),
        ),
    ];
    let open = match Command::new("/usr/bin/sleep")
        .arg("30")
        .stdin(Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            let r = result(pidfd_open(Pid::from_child(&child), PidfdFlags::empty()));
            let _ = child.kill();
            let _ = child.wait();
            r
        }
        Err(e) => format!("spawn-{}", result::<(), io::Error>(Err(e))),
    };
    v.push(("pidfd_open", open));
    let nofile = getrlimit(Resource::Nofile);
    v.push((
        "prlimit_self",
        result(prlimit(None, Resource::Nofile, nofile)),
    ));
    v.push((
        "prlimit_victim",
        result(prlimit(Some(victim), Resource::Nofile, nofile)),
    ));
    match sched_getaffinity(None) {
        Ok(cpus) => {
            v.push(("setaffinity_self", result(sched_setaffinity(None, &cpus))));
            v.push((
                "setaffinity_victim",
                result(sched_setaffinity(Some(victim), &cpus)),
            ));
        }
        Err(e) => v.push(("getaffinity_self", format!("errno={}", e.raw_os_error()))),
    }
    let caps = match capabilities(None) {
        Ok(c) => format!(
            "eff:{:x} prm:{:x} inh:{:x} amb:{} bnd:{} nnp:{}",
            c.effective.bits(),
            c.permitted.bits(),
            c.inheritable.bits(),
            mask(capability_is_in_ambient_set),
            mask(capability_is_in_bounding_set),
            no_new_privs()
                .map_or_else(|e| format!("errno={}", e.raw_os_error()), |b| b.to_string()),
        ),
        Err(e) => format!("errno={}", e.raw_os_error()),
    };
    v.push(("caps", caps));
    v
}
