//! Tier 1 of `AET-WO-1500` §3: `aethelgard mcp` over real stdio, with no real tool installed.
//! Written against `crates/aethelgard-cli/README.md` only.

mod common;

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use common::{Client, Scratch, WAIT, alive, descendants, eventually, example, stat_fields};
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
        [
            "validate",
            "solve",
            "emit",
            "check_drc",
            "check_erc",
            "prove",
            "simulate"
        ]
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
    fs::write(&board, "(kicad_pcb (version 20240108))\n").unwrap();
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

impl Scratch {
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
    fs::write(&board, "(kicad_pcb (version 20240108))\n").unwrap();
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
fn sighup_during_a_call_leaves_nothing() {
    let (_dir, mut c, _id, all) = blocked_call("hup", 307);
    signal(c.pid(), "HUP");
    assert_eq!(wait_exit(&mut c), Some(129));
    all_gone(&all);
}

/// SIGKILL cannot be handled: the CLI dies by its parent-death signal, then its reaper and
/// guardian clean up (AET-WO-0300c §1.3).
#[test]
fn sigkill_of_the_server_during_a_call_leaves_nothing() {
    let (_dir, mut c, _id, all) = blocked_call("kill", 308);
    signal(c.pid(), "KILL");
    assert_eq!(wait_exit(&mut c), None);
    all_gone(&all);
}

/// Ruled on PR #34: `mcp` never arms the parent-death signal on itself, even with the variable
/// set, so a server started from a thread that ends keeps serving.
#[test]
fn the_server_outlives_the_thread_that_started_it() {
    let me = std::process::id().to_string();
    let mut c = std::thread::spawn(move || {
        let c = Client::spawn(|cmd| {
            cmd.env("AETHELGARD_INTERNAL_PARENT_PID", &me);
        });
        // Long enough for the server to have armed the signal, had it armed one.
        std::thread::sleep(Duration::from_millis(300));
        c
    })
    .join()
    .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(alive(c.pid()), "the server died with its thread");
    assert_eq!(c.initialize()["result"]["serverInfo"]["name"], "aethelgard");
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
fn stdin_closed_before_initialize_exits_0() {
    let mut c = Client::spawn(|_| {});
    c.stdin = None;
    assert_eq!(wait_exit(&mut c), Some(0));
}

#[test]
fn sigterm_before_initialize_exits_143() {
    let mut c = Client::spawn(|_| {});
    // Wait for the handler (SigCgt bit for signal 15), so the signal cannot beat it.
    let pid = c.pid();
    eventually("the SIGTERM handler", || {
        fs::read_to_string(format!("/proc/{pid}/status")).is_ok_and(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("SigCgt:"))
                .and_then(|m| u64::from_str_radix(m.trim(), 16).ok())
                .is_some_and(|m| m & (1 << 14) != 0)
        })
    });
    signal(pid, "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
}

#[test]
fn sigterm_with_no_call_exits_143() {
    let mut c = Client::spawn(|_| {});
    c.initialize();
    signal(c.pid(), "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
}

// ------------------------------------------- AET-WO-0300c: the CLI killed at any instant

/// A process's start time in clock ticks since boot, `None` once it is gone or a zombie. With
/// the pid it names one process, so a reused pid is never mistaken for a survivor.
fn start_time(pid: u32) -> Option<u64> {
    // Field 22 of stat.
    stat_fields(pid)
        .filter(|f| f[0] != "Z")?
        .get(19)?
        .parse()
        .ok()
}

/// Every live process whose command line or environment holds `marker`, wherever it has been
/// reparented: the CLI, its guardian, its reaper, the trampoline and the tool all do.
fn marked(marker: &Path) -> Vec<u32> {
    let marker = marker.as_os_str().as_encoded_bytes();
    fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter_map(|e| {
            let pid: u32 = e.file_name().to_str()?.parse().ok()?;
            let holds = |file: &str| {
                fs::read(e.path().join(file))
                    .is_ok_and(|b| b.windows(marker.len()).any(|w| w == marker))
            };
            ((holds("cmdline") || holds("environ")) && alive(pid)).then_some(pid)
        })
        .collect()
}

/// Waits up to 30 s for `left` to return nothing, and returns what it last returned.
fn settle(mut left: impl FnMut() -> Vec<u32>) -> Vec<u32> {
    let deadline = std::time::Instant::now() + Duration::from_secs(30);
    loop {
        let now = left();
        if now.is_empty() || std::time::Instant::now() > deadline {
            return now;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// `AET-WO-1500` finding 8a. Each call is cancelled at once, or after up to 12 ms, so the
/// kill of the CLI sweeps its start-up: before the guardian, between the guardian and its
/// directory, before the reaper looks at its parent. Every descendant ever seen, and every
/// process naming this test's `TMPDIR`, must be gone by the deadline, and so must every
/// `aethelgard-*` directory.
#[test]
fn an_immediate_cancel_storm_leaves_no_process_and_no_directory() {
    use std::collections::HashSet;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    const CALLS: u64 = 500;
    let dir = Scratch::new("storm");
    dir.stub_kicad("exec /bin/sleep 306");
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "(kicad_pcb (version 20240108))\n").unwrap();
    let tmp = dir.0.join("tmp");
    fs::create_dir(&tmp).unwrap();
    let tb = dir.0.join("toolbox");
    let mut c = Client::spawn(|cmd| {
        cmd.env_clear().env("PATH", &tb).env("TMPDIR", &tmp);
    });
    c.initialize();
    let server = c.pid();

    let stop = Arc::new(AtomicBool::new(false));
    let sampler = {
        let stop = Arc::clone(&stop);
        std::thread::spawn(move || {
            let mut seen = HashSet::new();
            while !stop.load(Ordering::Relaxed) {
                for p in descendants(server) {
                    seen.extend(start_time(p).map(|t| (p, t)));
                }
                std::thread::sleep(Duration::from_millis(1));
            }
            seen
        })
    };
    for i in 0..CALLS {
        let id = c.start(
            "tools/call",
            json!({"name": "check_drc", "arguments": {"board": board}}),
        );
        std::thread::sleep(Duration::from_micros((i % 25) * 500));
        c.send(
            &json!({"jsonrpc": "2.0", "method": "notifications/cancelled",
                       "params": {"requestId": id, "reason": "storm"}}),
        );
    }
    // Still serving; then a shutdown ends any call whose cancel came too early to count.
    assert!(c.request("tools/list", json!({}))["result"]["tools"].is_array());
    signal(server, "TERM");
    assert_eq!(wait_exit(&mut c), Some(143));
    stop.store(true, Ordering::Relaxed);
    let seen = sampler.join().unwrap();

    nothing_left(&tmp, &seen, &format!("storm: calls={CALLS}"));
}

/// Waits for every process in `seen` (pid and start time) and every process naming `tmp` to be
/// gone, and for `tmp` to hold no `aethelgard-*` directory; kills what survives, prints the
/// counts after `label`, and fails if anything was left.
fn nothing_left(tmp: &Path, seen: &std::collections::HashSet<(u32, u64)>, label: &str) {
    let survivors = settle(|| {
        let mut left: Vec<u32> = seen
            .iter()
            .filter(|&&(p, t)| start_time(p) == Some(t))
            .map(|&(p, _)| p)
            .collect();
        left.extend(marked(tmp));
        left.sort_unstable();
        left.dedup();
        left
    });
    let described: Vec<String> = survivors
        .iter()
        .map(|p| {
            let cmd = fs::read(format!("/proc/{p}/cmdline")).unwrap_or_default();
            format!("{p}: {}", String::from_utf8_lossy(&cmd).replace('\0', " "))
        })
        .collect();
    for p in &survivors {
        let _ = Command::new("kill")
            .args(["-KILL", &p.to_string()])
            .status();
    }
    let dirs: Vec<String> = fs::read_dir(tmp)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.starts_with("aethelgard-"))
        .collect();
    println!(
        "{label} descendants_seen={} survivors={} leftover_dirs={}",
        seen.len(),
        survivors.len(),
        dirs.len()
    );
    assert!(
        survivors.is_empty() && dirs.is_empty(),
        "survivors {described:#?}, leftover directories {dirs:?}"
    );
}

/// The adversarial pass's finding 1: a server SIGKILLed while its calls are starting, some
/// CLIs not yet past arming their parent-death signal. Each server gets a burst of calls and
/// dies 0 to 9.5 ms later; nothing may outlive them.
#[test]
fn servers_sigkilled_while_calls_start_leave_nothing() {
    const SERVERS: u64 = 100;
    const BURST: usize = 5;
    let dir = Scratch::new("parent-kill");
    dir.stub_kicad("exec /bin/sleep 309");
    let board = dir.0.join("b.kicad_pcb");
    fs::write(&board, "(kicad_pcb (version 20240108))\n").unwrap();
    let tmp = dir.0.join("tmp");
    fs::create_dir(&tmp).unwrap();
    let tb = dir.0.join("toolbox");
    for i in 0..SERVERS {
        let mut c = Client::spawn(|cmd| {
            cmd.env_clear().env("PATH", &tb).env("TMPDIR", &tmp);
        });
        c.initialize();
        for _ in 0..BURST {
            c.start(
                "tools/call",
                json!({"name": "check_drc", "arguments": {"board": board}}),
            );
        }
        std::thread::sleep(Duration::from_micros((i % 20) * 500));
        signal(c.pid(), "KILL");
        assert_eq!(wait_exit(&mut c), None);
    }
    nothing_left(
        &tmp,
        &std::collections::HashSet::new(),
        &format!("parent-kill: servers={SERVERS} burst={BURST}"),
    );
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
