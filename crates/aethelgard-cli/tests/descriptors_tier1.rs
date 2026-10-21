//! Tier 1 for the inherited-descriptor refusal (AET-WO-0300a, ruled on PR #15,
//! B2): a descriptor above 2 without close-on-exec would reach the tool, a
//! channel no sandbox layer sees, so the trampoline refuses the call.

use std::fs;
use std::net::TcpListener;
use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};

use serde_json::Value;

/// Descriptors above 2 this process holds without close-on-exec.
fn inheritable_descriptors() -> Vec<String> {
    fs::read_dir("/proc/self/fd")
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let fd = e.file_name().into_string().ok()?;
            let n: u32 = fd.parse().ok()?;
            let info = fs::read_to_string(format!("/proc/self/fdinfo/{n}")).ok()?;
            let flags = info.lines().find_map(|l| l.strip_prefix("flags:"))?;
            let flags = u32::from_str_radix(flags.trim(), 8).ok()?;
            (n > 2 && flags & 0o2_000_000 == 0)
                .then(|| format!("{fd} -> {:?}", fs::read_link(e.path()).ok()))
        })
        .collect()
}

/// What this test runner hands down to the CLI: nothing (measured in `gates`,
/// `tools` and on Furnace, where this test runs). A runner that leaked a
/// descriptor, a jobserver for example, would make every tool call refuse.
#[test]
fn the_test_runner_leaks_no_descriptor_into_the_cli() {
    let leaked = inheritable_descriptors();
    println!("inheritable descriptors above 2: {leaked:?}");
    assert!(leaked.is_empty(), "the runner leaks {leaked:?}");
}

#[test]
fn an_inherited_socket_refuses_the_call_and_the_tool_never_runs() {
    let dir = std::env::temp_dir().join(format!("aet-descriptors-{}", std::process::id()));
    let toolbox = dir.join("toolbox");
    fs::create_dir_all(&toolbox).unwrap();
    let ran = dir.join("tool-ran");
    let stub = toolbox.join("kicad-cli");
    fs::write(
        &stub,
        format!("#!/bin/sh\n: > '{}'\necho 10.0.7\n", ran.display()),
    )
    .unwrap();
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    let board = dir.join("b.kicad_pcb");
    fs::write(&board, "x").unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();

    // bash opens a connected socket as fd 3 without close-on-exec, then
    // becomes the CLI, which inherits it.
    let out = Command::new("/bin/bash")
        .arg("-c")
        .arg(format!(
            "exec 3<>/dev/tcp/127.0.0.1/{port} && exec \"$0\" \"$@\""
        ))
        .arg(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["check", "drc", "--board"])
        .arg(&board)
        .env_clear()
        .env("PATH", &toolbox)
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stdout)));
    assert_eq!(out.status.code(), Some(4), "{v}");
    assert_eq!(v["error"]["kind"], "SandboxUnavailable", "{v}");
    assert_eq!(v["error"]["id"], "descriptors", "{v}");
    assert!(
        v["error"]["message"].as_str().unwrap().contains("[3]"),
        "{v}"
    );
    assert!(v.get("verdict").is_none(), "{v}");
    assert!(
        !ran.exists(),
        "the tool ran despite the inherited descriptor"
    );
    drop(listener);
    fs::remove_dir_all(&dir).unwrap();
}
