//! A stand-in `kicad-cli` for tier 1's socket tests (AET-WO-0300a §1.4). It
//! answers `version`; asked for `pcb drc … -o REPORT BOARD`, it reads its
//! targets from the board file (the copy the CLI made), tries to reach each
//! one, and reports every attempt's result as a finding in a KiCad-shaped
//! report: `name=ok` or `name=errno=N`. Run by the real CLI, it runs inside
//! the real sandbox, so the results are what a hostile tool would get.

#![forbid(unsafe_code)]

use std::io;
use std::net::{TcpStream, UdpSocket};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::net::{SocketAddr, UnixStream};
use std::process::exit;

use serde_json::json;

fn result<T>(r: io::Result<T>) -> String {
    match r {
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
    let report = args
        .windows(2)
        .find(|w| w[0] == "-o")
        .map(|w| w[1].clone())
        .unwrap_or_else(|| exit(99));
    let board = std::fs::read_to_string(args.last().unwrap_or_else(|| exit(99)))
        .unwrap_or_else(|_| exit(98));
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
        ("socketpair", result(UnixStream::pair())),
        ("io_uring_setup", result(io_uring::IoUring::new(2))),
    ];
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
