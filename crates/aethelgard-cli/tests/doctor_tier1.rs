//! Tier 1 of `AET-WO-1604` §1.1: `aethelgard doctor`, with stub tools on a
//! `PATH` of their own and a stub bridge beside a copy of the binary.
//! Written against `crates/aethelgard-cli/README.md` (`## doctor`).

mod common;

use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use common::{Scratch, private_tmp, stub};
use serde_json::{Value, json};

const BLOCKED_BY_SCRATCH: [&str; 8] = [
    "check drc",
    "check erc",
    "check support",
    "build",
    "prove",
    "simulate",
    "campaign",
    "verify",
];

/// The bridge with no request: its identity, its usage, exit 2.
const USAGE_BRIDGE: &str =
    "#!/bin/sh\necho 'usage: aethelgard-bridge <request.json>' >&2\nexit 2\n";

/// A copy of the binary, with `bridge` as the stub beside it when given.
fn cli(bridge: Option<&'static str>, identified: bool) -> PathBuf {
    type Made = Vec<((Option<&'static str>, bool), PathBuf)>;
    static MADE: Mutex<Made> = Mutex::new(Vec::new());
    let mut made = MADE.lock().unwrap();
    if let Some((_, path)) = made.iter().find(|(k, _)| *k == (bridge, identified)) {
        return path.clone();
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "aet-1604-cli-{}-{}",
        std::process::id(),
        made.len()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let me = dir.join("aethelgard");
    if fs::hard_link(env!("CARGO_BIN_EXE_aethelgard"), &me).is_err() {
        stub::write_executable(&me, fs::read(env!("CARGO_BIN_EXE_aethelgard")).unwrap());
    }
    if let Some(body) = bridge {
        let body = if identified {
            common::identified_stub(body)
        } else {
            body.to_owned()
        };
        stub::write_executable(&dir.join("aethelgard-bridge"), body);
    }
    made.push(((bridge, identified), me.clone()));
    me
}

/// Stub tools, each printing the version its subcommand expects unless
/// `except` replaces or (with `None`) removes it.
struct Toolbox {
    dir: Scratch,
    homes: Scratch,
}

fn toolbox(except: &[(&str, Option<&str>)]) -> Toolbox {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = Scratch::new(&format!("doctor-box-{n}"));
    let homes = Scratch::new(&format!("doctor-homes-{n}"));
    // The pinned build, which `doctor` holds Renode to as a boot does
    // (`AET-WO-4400g` §1.5).
    let renode = format!(
        "#!/bin/sh\necho 'Renode v1.17.0'\necho 'build: {}'\n",
        aethelgard_twin::platform::RENODE_BUILD
    );
    let tools = [
        ("kicad-cli", "#!/bin/sh\necho 10.0.7\n"),
        ("renode", renode.as_str()),
        (
            "cargo",
            "#!/bin/sh\nif [ \"$1\" = kani ]; then echo 'cargo-kani 0.66.0'; else echo 'cargo 1.95.0 (stub)'; fi\n",
        ),
        ("cargo-kani", "#!/bin/sh\necho 'cargo-kani 0.66.0'\n"),
    ];
    for (name, body) in tools {
        let body = match except.iter().find(|(n, _)| *n == name) {
            Some((_, None)) => continue,
            Some((_, Some(b))) => *b,
            None => body,
        };
        stub::write_executable(&dir.0.join(name), body);
    }
    fs::create_dir_all(homes.0.join("kani")).unwrap();
    fs::create_dir_all(homes.0.join("rustup")).unwrap();
    Toolbox { dir, homes }
}

fn doctor(cli: &Path, tb: &Toolbox, tmpdir: &Path) -> (Option<i32>, Value) {
    doctor_in(cli, tb, tmpdir, None)
}

/// `doctor` run directly, or (with `shell`) as the program a bash line `exec`s.
fn doctor_in(cli: &Path, tb: &Toolbox, tmpdir: &Path, shell: Option<&str>) -> (Option<i32>, Value) {
    let mut cmd = match shell {
        Some(line) => {
            let mut c = Command::new("/bin/bash");
            c.arg("-c").arg(line).arg(cli).arg("doctor");
            c
        }
        None => {
            let mut c = Command::new(cli);
            c.arg("doctor");
            c
        }
    };
    let out: Output = cmd
        .env_clear()
        .env("HOME", std::env::var_os("HOME").unwrap_or_default())
        .env("PATH", &tb.dir.0)
        .env("TMPDIR", tmpdir)
        .env("KANI_HOME", tb.homes.0.join("kani"))
        .env("RUSTUP_HOME", tb.homes.0.join("rustup"))
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout);
    assert_eq!(text.lines().count(), 1, "one line: {text}");
    let v: Value = serde_json::from_str(&text).unwrap_or_else(|e| panic!("{e}: {text}"));
    (out.status.code(), v)
}

fn tool<'a>(v: &'a Value, name: &str) -> &'a Value {
    v["tools"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["tool"] == name)
        .unwrap_or_else(|| panic!("no {name} in {v}"))
}

fn missing(v: &Value) -> Vec<(String, String)> {
    v["missing"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["kind"].as_str().unwrap().to_owned(),
                m["id"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect()
}

/// The sandbox report every probe here must carry: the one a tool run
/// reports, worked out apart from the CLI for this binary's directory.
fn assert_sandbox(sandbox: &Value, dirs: &[PathBuf]) {
    let (boundary, unmet) = common::expected_boundary(dirs);
    assert!(sandbox["landlock_abi"].as_i64().unwrap() >= 3, "{sandbox}");
    assert_eq!(sandbox["sockets_denied"], true, "{sandbox}");
    assert_eq!(sandbox["uid_boundary"], boundary, "{sandbox}");
    assert_eq!(sandbox["uid_boundary_unmet"], json!(unmet), "{sandbox}");
}

#[test]
fn everything_present_is_exit_0_with_each_version_and_the_sandbox() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[]);
    let tmp = private_tmp("doctor-all");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(v["ok"], true, "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    assert_eq!(v["check"], "doctor", "{v}");
    assert_eq!(v["engine_version"], env!("CARGO_PKG_VERSION"), "{v}");
    assert_eq!(v["missing"], json!([]), "{v}");
    assert_eq!(
        v["scratch"],
        fs::canonicalize(&tmp.0).unwrap().display().to_string(),
        "{v}"
    );
    assert_sandbox(&v["sandbox"], &[cli.parent().unwrap().to_path_buf()]);
    assert_eq!(tool(&v, "kicad-cli")["version"], "10.0.7", "{v}");
    assert_eq!(tool(&v, "renode")["version"], "Renode v1.17.0", "{v}");
    assert_eq!(tool(&v, "cargo")["version"], "cargo 1.95.0 (stub)", "{v}");
    assert_eq!(
        tool(&v, "cargo-kani")["version"],
        "cargo-kani 0.66.0",
        "{v}"
    );
    let bridge = tool(&v, "aethelgard-bridge");
    assert_eq!(bridge["version"], "0.1.0", "{v}");
    assert_eq!(bridge["ngspice_loaded"], true, "{v}");
    for t in v["tools"].as_array().unwrap() {
        assert!(t["path"].is_string(), "{t}");
        assert!(t["sandbox"].is_object(), "{t}");
    }
    let names: Vec<&str> = v["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["tool"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "kicad-cli",
            "renode",
            "aethelgard-bridge",
            "cargo",
            "cargo-kani"
        ]
    );
}

#[test]
fn a_missing_tool_is_named_with_the_subcommands_it_blocks() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[("renode", None)]);
    let tmp = private_tmp("doctor-renode");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["ok"], false, "{v}");
    assert_eq!(v["verdict"], "fail", "{v}");
    assert_eq!(
        v["missing"],
        json!([{
            "kind": "ToolMissing",
            "id": "renode",
            "message": "`renode` is not on PATH",
            "blocks": ["simulate", "campaign", "verify"],
        }]),
        "{v}"
    );
    assert_eq!(tool(&v, "renode"), &json!({"tool": "renode"}), "{v}");
    // Nothing else is touched by one tool's absence.
    assert_eq!(tool(&v, "kicad-cli")["version"], "10.0.7", "{v}");
    assert!(v["sandbox"].is_object(), "{v}");
}

#[test]
fn cargo_missing_is_named_once_for_build_and_prove() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[("cargo", None)]);
    let tmp = private_tmp("doctor-cargo");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        v["missing"],
        json!([{
            "kind": "ToolMissing",
            "id": "cargo",
            "message": "`cargo` is not on PATH",
            "blocks": ["build", "prove", "verify"],
        }]),
        "{v}"
    );
}

#[test]
fn a_missing_variable_is_named_as_the_subcommand_names_it() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[]);
    fs::remove_dir(tb.homes.0.join("kani")).unwrap();
    let tmp = private_tmp("doctor-kani-home");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(missing(&v), [("ToolUnusable".into(), "KANI_HOME".into())]);
    assert_eq!(v["missing"][0]["blocks"], json!(["prove", "verify"]), "{v}");
    assert!(tool(&v, "cargo-kani")["path"].is_string(), "{v}");
    assert!(tool(&v, "cargo-kani").get("version").is_none(), "{v}");
}

#[test]
fn a_tool_off_its_pin_reports_its_version_and_blocks_what_pins_it() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[
        ("kicad-cli", Some("#!/bin/sh\necho 10.0.8\n")),
        ("renode", Some("#!/bin/sh\necho 'Renode v1.16.0'\n")),
    ]);
    let tmp = private_tmp("doctor-pins");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        missing(&v),
        [
            ("ToolUnusable".into(), "kicad-cli".into()),
            ("ToolUnusable".into(), "renode".into())
        ]
    );
    assert_eq!(
        v["missing"][0]["blocks"],
        json!(["check drc", "check erc", "verify"]),
        "{v}"
    );
    assert_eq!(
        v["missing"][1]["blocks"],
        json!(["simulate --firmware", "campaign", "verify"]),
        "{v}"
    );
    assert_eq!(tool(&v, "kicad-cli")["version"], "10.0.8", "{v}");
    assert_eq!(tool(&v, "renode")["version"], "Renode v1.16.0", "{v}");
}

#[test]
fn a_tool_that_will_not_say_its_version_is_unusable() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[("kicad-cli", Some("#!/bin/sh\nexit 3\n"))]);
    let tmp = private_tmp("doctor-silent");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(missing(&v), [("ToolUnusable".into(), "kicad-cli".into())]);
    assert_eq!(
        v["missing"][0]["blocks"],
        json!(["check drc", "check erc", "check support", "verify"]),
        "{v}"
    );
}

#[test]
fn an_unsafe_scratch_blocks_every_tool_subcommand_and_nothing_is_probed() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    // A stub that leaves a mark if anything runs it.
    let marks = Scratch::new("doctor-marks");
    let ran = marks.0.join("ran");
    let body = format!("#!/bin/sh\n: > '{}'\necho 10.0.7\n", ran.display());
    let tb = toolbox(&[("kicad-cli", Some(body.as_str()))]);
    // World-writable: another user could plant configuration a tool reads.
    let (code, v) = doctor(&cli, &tb, Path::new("/tmp"));
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(missing(&v), [("ScratchUnsafe".into(), "/tmp".into())]);
    assert_eq!(v["missing"][0]["blocks"], json!(BLOCKED_BY_SCRATCH), "{v}");
    assert!(v["scratch"].is_null(), "{v}");
    assert!(v["sandbox"].is_null(), "{v}");
    for t in v["tools"].as_array().unwrap() {
        assert!(t["path"].is_string(), "found on PATH: {t}");
        assert!(t.get("version").is_none(), "not probed: {t}");
        assert!(t.get("sandbox").is_none(), "not probed: {t}");
    }
    assert!(!ran.exists(), "a tool ran without a scratch");
}

#[test]
fn a_sandbox_layer_that_cannot_be_installed_is_reported_never_assumed() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let tb = toolbox(&[]);
    let tmp = private_tmp("doctor-descriptor");
    // bash opens fd 3 without close-on-exec and becomes the CLI: every
    // confined run refuses before it installs anything (`descriptors`).
    let (code, v) = doctor_in(
        &cli,
        &tb,
        &tmp.0,
        Some("exec 3</dev/null && exec \"$0\" \"$@\""),
    );
    assert_eq!(code, Some(1), "{v}");
    assert!(v["sandbox"].is_null(), "{v}");
    assert_eq!(
        missing(&v),
        [("SandboxUnavailable".into(), "descriptors".into())]
    );
    assert_eq!(v["missing"][0]["blocks"], json!(BLOCKED_BY_SCRATCH), "{v}");
    for t in v["tools"].as_array().unwrap() {
        assert!(t.get("version").is_none(), "no probe ran: {t}");
    }
}

#[test]
fn the_probe_runs_confined() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    // Outside every grant: the stub prints it as its version if it can.
    let secret = Scratch::new("doctor-secret");
    let file = secret.0.join("secret");
    fs::write(&file, "LEAKED\n").unwrap();
    let body = format!(
        "#!/bin/sh\n/bin/cat '{}' 2>/dev/null || echo 10.0.7\n",
        file.display()
    );
    let tb = toolbox(&[("kicad-cli", Some(body.as_str()))]);
    let tmp = private_tmp("doctor-confined");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(tool(&v, "kicad-cli")["version"], "10.0.7", "{v}");
    assert_sandbox(
        &tool(&v, "kicad-cli")["sandbox"],
        std::slice::from_ref(&tb.dir.0),
    );
}

#[test]
fn a_bridge_that_is_absent_or_not_ours_is_named() {
    let tb = toolbox(&[]);
    let tmp = private_tmp("doctor-bridge");
    let (code, v) = doctor(&cli(None, true), &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        missing(&v),
        [("BridgeUnavailable".into(), "aethelgard-bridge".into())]
    );
    assert_eq!(
        v["missing"][0]["blocks"],
        json!(["simulate --firmware", "campaign", "verify"]),
        "{v}"
    );
    // Another tree's bridge.
    let stale = "#!/bin/sh\necho '{\"bridge\":\"aethelgard-bridge\",\"source_sha256\":\"00\",\"version\":\"0.1.0\"}'\nexit 2\n";
    let (code, v) = doctor(&cli(Some(stale), false), &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        missing(&v),
        [("BridgeMismatch".into(), "aethelgard-bridge".into())]
    );
    assert!(
        tool(&v, "aethelgard-bridge")
            .get("ngspice_loaded")
            .is_none()
    );
}

#[test]
fn a_bridge_the_loader_cannot_start_names_what_it_said() {
    let tb = toolbox(&[]);
    let tmp = private_tmp("doctor-loader");
    // What the loader prints when libngspice.so.0 is absent: nothing on
    // stdout, exit 127.
    let unloaded = "#!/bin/sh\necho 'aethelgard-bridge: error while loading shared libraries: libngspice.so.0: cannot open shared object file: No such file or directory' >&2\nexit 127\n";
    let (code, v) = doctor(&cli(Some(unloaded), false), &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        missing(&v),
        [("ToolUnusable".into(), "aethelgard-bridge".into())]
    );
    let message = v["missing"][0]["message"].as_str().unwrap();
    assert!(message.contains("exited 127"), "{message}");
    assert!(message.contains("libngspice.so.0"), "{message}");
}

#[test]
fn a_bridge_that_runs_without_a_request_is_not_the_bridge() {
    let tb = toolbox(&[]);
    let tmp = private_tmp("doctor-bridge-0");
    let (code, v) = doctor(&cli(Some("#!/bin/sh\nexit 0\n"), true), &tb, &tmp.0);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(
        missing(&v),
        [("ToolUnusable".into(), "aethelgard-bridge".into())]
    );
}

#[test]
fn doctor_takes_no_argument() {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([OsStr::new("doctor"), OsStr::new("--spec"), OsStr::new("x")])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(64));
}

/// Each probe runs in a scratch of its own, as each subcommand's tool does:
/// nothing one tool leaves in its `HOME` reaches the next probe.
#[test]
fn each_probe_has_a_scratch_of_its_own() {
    let cli = cli(Some(USAGE_BRIDGE), true);
    let renode = format!(
        "#!/bin/sh\nif [ -e \"$HOME/planted\" ]; then echo LEAKED; else echo 'Renode v1.17.0'; echo 'build: {}'; fi\n",
        aethelgard_twin::platform::RENODE_BUILD
    );
    let tb = toolbox(&[
        (
            "kicad-cli",
            Some("#!/bin/sh\n: > \"$HOME/planted\"\necho 10.0.7\n"),
        ),
        ("renode", Some(renode.as_str())),
    ]);
    let tmp = private_tmp("doctor-own-scratch");
    let (code, v) = doctor(&cli, &tb, &tmp.0);
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(tool(&v, "renode")["version"], "Renode v1.17.0", "{v}");
}
