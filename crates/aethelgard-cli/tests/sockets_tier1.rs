//! Tier 1 for AET-WO-0300a §1.4: the S4 attack set, run by a stand-in tool
//! (`examples/sandbox_probe.rs`) under the real CLI and so the real
//! trampoline, Landlock and seccomp filter. Every target is live, proven by
//! reaching it from outside the sandbox first; inside, every `socket()` and
//! `io_uring_setup` is refused with EPERM (1), and `socketpair` still works.

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::os::linux::net::SocketAddrExt;
use std::os::unix::fs::symlink;
use std::os::unix::net::{SocketAddr, UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

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

#[test]
fn the_s4_attack_set_is_refused_inside_the_sandbox() {
    let dir = std::env::temp_dir().join(format!("aet-sockets-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let toolbox = dir.join("toolbox");
    fs::create_dir_all(&toolbox).unwrap();
    symlink(probe(), toolbox.join("kicad-cli")).unwrap();

    // Live targets outside the sandbox: a pathname socket (stands in for a
    // session bus or tmux), an abstract socket, and a TCP listener.
    let path = dir.join("bus.sock");
    let unix = UnixListener::bind(&path).unwrap();
    let name = format!("aet-sockets-{}", std::process::id());
    let abstract_listener =
        UnixListener::bind_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap()).unwrap();
    let tcp = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = tcp.local_addr().unwrap().port();
    // Control: each is reachable from here, so a refusal inside is the sandbox.
    UnixStream::connect(&path).unwrap();
    UnixStream::connect_addr(&SocketAddr::from_abstract_name(name.as_bytes()).unwrap()).unwrap();
    TcpStream::connect(("127.0.0.1", port)).unwrap();

    let board = dir.join("targets.kicad_pcb");
    fs::write(
        &board,
        format!("unix={}\nabstract={name}\ntcp={port}\n", path.display()),
    )
    .unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["check", "drc", "--board"])
        .arg(&board)
        .env_clear()
        .env("PATH", &toolbox)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    assert_eq!(out.status.code(), Some(1), "{v}");
    assert_eq!(v["sandbox"]["sockets_denied"], true, "{v}");
    let results: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["message"].as_str().unwrap())
        .collect();
    assert_eq!(
        results,
        [
            "unix_path=errno=1",
            "unix_abstract=errno=1",
            "udp_sendto=errno=1",
            "tcp_connect=errno=1",
            "socketpair=ok",
            "io_uring_setup=errno=1",
        ],
        "{v}"
    );
    drop((unix, abstract_listener, tcp));
    fs::remove_dir_all(&dir).unwrap();
}
