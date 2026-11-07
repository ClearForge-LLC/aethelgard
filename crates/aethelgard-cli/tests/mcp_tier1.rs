//! Tier 1 of `AET-WO-1500` §3: `aethelgard mcp` over real stdio, with no real tool installed.
//! Written against `crates/aethelgard-cli/README.md` only.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use common::{Client, WAIT, alive, descendants, eventually, example};
use serde_json::{Value, json};

fn text(response: &Value) -> Value {
    serde_json::from_str(response["result"]["content"][0]["text"].as_str().unwrap()).unwrap()
}

#[test]
fn lists_the_five_tools_and_completes_two_calls() {
    let mut c = Client::spawn(|_| {});
    let init = c.initialize();
    assert_eq!(init["result"]["serverInfo"]["name"], "aethelgard");
    assert!(
        init["result"]["capabilities"]["tools"].is_object(),
        "{init}"
    );
    assert!(
        init["result"]["capabilities"].get("resources").is_none(),
        "{init}"
    );
    assert!(
        init["result"]["capabilities"].get("prompts").is_none(),
        "{init}"
    );

    let list = c.request("tools/list", json!({}));
    let tools = list["result"]["tools"].as_array().unwrap();
    let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
    assert_eq!(
        names,
        ["validate", "check_drc", "check_erc", "prove", "simulate"]
    );
    println!("{}", serde_json::to_string_pretty(&list["result"]).unwrap());
    for t in tools {
        assert_eq!(t["inputSchema"]["type"], "object");
        assert_eq!(t["inputSchema"]["additionalProperties"], false);
        assert!(!t["description"].as_str().unwrap().is_empty());
    }
    let prove = tools.iter().find(|t| t["name"] == "prove").unwrap();
    assert_eq!(
        prove["inputSchema"]["required"],
        json!(["crate", "harness"])
    );
    assert!(prove["inputSchema"]["properties"].get("crate").is_some());
    assert!(
        prove["inputSchema"]["properties"]
            .get("crate_dir")
            .is_none()
    );

    let ok = c.call(
        "validate",
        json!({"file": example("sentinel-01-minimal.json")}),
    );
    let result = &ok["result"];
    assert_eq!(result["isError"], false, "{ok}");
    assert_eq!(result["structuredContent"]["exit_code"], 0);
    assert_eq!(result["structuredContent"]["verdict"]["ok"], true);
    assert_eq!(text(&ok), result["structuredContent"]["verdict"]);

    let zero = c.call(
        "validate",
        json!({"file": example("invalid-zero-domain.json")}),
    );
    let result = &zero["result"];
    assert_eq!(result["isError"], true, "{zero}");
    assert_eq!(result["structuredContent"]["exit_code"], 2);
    let verdict = &result["structuredContent"]["verdict"];
    assert_eq!(verdict["ok"], false);
    assert_eq!(verdict["error"]["kind"], "ZeroNominalVoltage");
    assert_eq!(verdict["error"]["id"], "VDD_3V3");
    assert!(
        result["content"][0]["text"]
            .as_str()
            .unwrap()
            .contains("VDD_3V3"),
        "{zero}"
    );

    // Closing stdin ends the server with exit 0 and nothing else on stdout.
    c.stdin = None;
    eventually("exit after stdin closed", || {
        c.child.try_wait().unwrap().is_some()
    });
    assert_eq!(c.child.wait().unwrap().code(), Some(0));
    assert!(c.recv(Duration::from_millis(200)).is_none());
}

#[test]
fn validate_without_file_is_refused_as_empty_input() {
    let mut c = Client::spawn(|_| {});
    c.initialize();
    let r = c.call("validate", json!({}));
    assert_eq!(r["result"]["isError"], true, "{r}");
    assert_eq!(r["result"]["structuredContent"]["exit_code"], 3);
}

#[test]
fn refusals_spawn_no_child_and_are_invalid_params() {
    let dir = Scratch::new("refuse");
    let marker = dir.0.join("tool-ran");
    dir.stub_kicad(&format!(": > '{}'\nexec /bin/sleep 1", marker.display()));
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "x").unwrap();
    let mut c = dir.client();
    c.initialize();
    let bad = [
        ("nope", json!({})),
        ("check_drc", json!({})),
        ("check_drc", json!({"board": 5})),
        ("check_drc", json!({"board": null})),
        ("check_drc", json!({"board": "a\u{0}b"})),
        ("check_drc", json!({"board": board, "extra": "x"})),
        ("check_drc", json!({"board": board, "help": "x"})),
        ("check_drc", json!({"board": board, "version": "x"})),
        ("check.drc", json!({"board": board})),
        ("check/drc", json!({"board": board})),
        ("check drc", json!({"board": board})),
        ("mcp", json!({})),
        ("help", json!({})),
        ("--aethelgard-internal-reaper", json!({})),
    ];
    for (tool, arguments) in bad {
        let r = c.call(tool, arguments.clone());
        assert_eq!(r["error"]["code"], -32602, "{tool} {arguments}: {r}");
        assert!(r.get("result").is_none(), "{r}");
    }
    assert!(!marker.exists(), "a refused call ran the tool");
    // The server is still alive and answers.
    assert!(c.request("tools/list", json!({}))["result"]["tools"].is_array());
}

#[test]
fn a_dash_value_is_judged_by_the_cli_not_taken_as_a_flag() {
    let mut c = Client::spawn(|_| {});
    c.initialize();
    let r = c.call("validate", json!({"file": "--help"}));
    // `--file=--help` is a path that does not exist: exit 3, never the CLI's help.
    assert_eq!(r["result"]["structuredContent"]["exit_code"], 3, "{r}");
    assert_eq!(
        r["result"]["structuredContent"]["verdict"]["error"]["kind"],
        "UnreadableInput"
    );
}

#[test]
fn an_inherited_descriptor_refuses_at_startup_with_exit_4() {
    let out = Command::new("/bin/bash")
        .args([
            "-c",
            "exec 3</dev/null; exec \"$0\" mcp",
            env!("CARGO_BIN_EXE_aethelgard"),
        ])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(4), "{out:?}");
    assert!(
        out.stdout.is_empty(),
        "stdout must carry protocol messages only"
    );
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(
        err.contains("SandboxUnavailable") && err.contains("descriptors"),
        "{err}"
    );
}

// ------------------------------------------------------- cancellation and shutdown

struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Scratch {
        let p = std::env::temp_dir().join(format!("aet-mcp-{}-{label}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        Scratch(fs::canonicalize(p).unwrap())
    }

    /// A `kicad-cli` that reports a version and then runs `body` as the check.
    fn stub_kicad(&self, body: &str) {
        let tb = self.0.join("toolbox");
        fs::create_dir_all(&tb).unwrap();
        let path = tb.join("kicad-cli");
        fs::write(
            &path,
            format!("#!/bin/sh\nif [ \"$1\" = version ]; then echo 10.0.7; exit 0; fi\n{body}\n"),
        )
        .unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn client(&self) -> Client {
        let tb = self.0.join("toolbox");
        Client::spawn(|cmd| {
            cmd.env_clear().env("PATH", &tb);
        })
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn sleeper_running(root: u32, secs: u32) -> bool {
    let want = format!("/bin/sleep\0{secs}\0").into_bytes();
    descendants(root)
        .iter()
        .any(|p| fs::read(format!("/proc/{p}/cmdline")).is_ok_and(|c| c == want))
}

/// A server mid-call on a tool that blocks for good; returns the client, the call's request id
/// and every descendant of the server, recorded while the tool runs.
fn blocked_call(label: &str, secs: u32) -> (Scratch, Client, u64, Vec<u32>) {
    let dir = Scratch::new(label);
    dir.stub_kicad(&format!("exec /bin/sleep {secs}"));
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "x").unwrap();
    let mut c = dir.client();
    c.initialize();
    let id = c.start(
        "tools/call",
        json!({"name": "check_drc", "arguments": {"board": board}}),
    );
    let pid = c.pid();
    eventually("the tool to be running", || sleeper_running(pid, secs));
    let all = descendants(pid);
    assert!(
        all.len() >= 3,
        "expected the CLI, its reaper and the tool, got {all:?}"
    );
    (dir, c, id, all)
}

fn all_gone(pids: &[u32]) {
    eventually("every recorded descendant to be gone", || {
        pids.iter().all(|p| !alive(*p))
    });
}

fn wait_exit(c: &mut Client) -> Option<i32> {
    eventually("the server to exit", || {
        c.child.try_wait().unwrap().is_some()
    });
    c.child.wait().unwrap().code()
}

fn signal(pid: u32, name: &str) {
    assert!(
        Command::new("kill")
            .args([&format!("-{name}"), &pid.to_string()])
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn a_cancel_notification_ends_the_call_and_everything_under_it() {
    let (_dir, mut c, id, all) = blocked_call("cancel", 301);
    c.send(
        &json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
                   "params": {"requestId": id, "reason": "test"}}),
    );
    all_gone(&all);
    // The server itself is still serving.
    assert!(c.request("tools/list", json!({}))["result"]["tools"].is_array());
    assert!(alive(c.pid()));
}

#[test]
fn sigterm_after_stdin_closed_during_a_call_leaves_nothing() {
    let (_dir, mut c, _id, all) = blocked_call("closed-term", 302);
    c.stdin = None;
    signal(c.pid(), "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
    all_gone(&all);
}

#[test]
fn sigterm_alone_during_a_call_leaves_nothing() {
    let (_dir, mut c, _id, all) = blocked_call("term", 303);
    signal(c.pid(), "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
    all_gone(&all);
}

#[test]
fn sigint_during_a_call_leaves_nothing() {
    let (_dir, mut c, _id, all) = blocked_call("int", 304);
    signal(c.pid(), "INT");
    assert_eq!(wait_exit(&mut c), Some(130));
    all_gone(&all);
}

#[test]
fn a_client_that_disconnects_mid_call_leaves_nothing_after_the_grace() {
    let (_dir, mut c, _id, all) = blocked_call("disconnect", 305);
    c.stdin = None;
    // rmcp waits up to 5 s for running handlers, then cancels them; the server then exits 0.
    assert_eq!(wait_exit(&mut c), Some(0));
    all_gone(&all);
}

#[test]
fn sigterm_with_no_call_exits_143() {
    let mut c = Client::spawn(|_| {});
    c.initialize();
    signal(c.pid(), "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
}

// ------------------------------------------------------------------- no listener

fn crate_root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn the_crate_source_opens_no_listener() {
    let mut files = vec![crate_root().join("src")];
    let mut seen = 0;
    while let Some(p) = files.pop() {
        if p.is_dir() {
            files.extend(fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
        } else if p.extension().is_some_and(|e| e == "rs") {
            seen += 1;
            let src = fs::read_to_string(&p).unwrap();
            for token in ["TcpListener", "UnixListener", "UdpSocket", ".bind("] {
                assert!(!src.contains(token), "{} contains {token}", p.display());
            }
        }
    }
    assert!(seen >= 8, "searched only {seen} files");
}

#[test]
fn rmcp_has_exactly_the_stdio_features_and_no_network_crate_is_locked() {
    let root = crate_root().join("../..");
    let ws: toml::Table = fs::read_to_string(root.join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    let dep = &ws["workspace"]["dependencies"]["rmcp"];
    let mut features: Vec<&str> = dep["features"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f.as_str().unwrap())
        .collect();
    features.sort_unstable();
    assert_eq!(features, ["server", "transport-io"]);
    assert_eq!(dep["default-features"], false.into());
    let own: toml::Table = fs::read_to_string(crate_root().join("Cargo.toml"))
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        own["dependencies"]["rmcp"]["workspace"],
        true.into(),
        "the crate must take rmcp from the workspace entry, with no feature of its own"
    );
    assert!(own["dependencies"]["rmcp"].as_table().unwrap().len() == 1);
    let lock = fs::read_to_string(root.join("Cargo.lock")).unwrap();
    for name in [
        "axum",
        "hyper",
        "reqwest",
        "oauth2",
        "tower-http",
        "tungstenite",
    ] {
        assert!(
            !lock.contains(&format!("name = \"{name}\"")),
            "Cargo.lock holds `{name}`, a network crate"
        );
    }
}

#[test]
fn nothing_but_protocol_reaches_stdout() {
    // Every line is JSON-RPC: `Client::recv` panics on anything else.
    let mut c = Client::spawn(|_| {});
    c.initialize();
    let _ = c.request("tools/list", json!({}));
    let _ = c.request("ping", json!({}));
    let _ = c.request("resources/list", json!({}));
    let _ = c.request("prompts/list", json!({}));
    while let Some(m) = c.recv(Duration::from_millis(300)) {
        assert_eq!(m["jsonrpc"], "2.0");
    }
    let _ = WAIT;
}
