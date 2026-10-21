//! Tier 1 of `AET-WO-0300` §1.6: the tool subcommands against stub tools, with none of the real
//! tools installed. Written against `crates/aethelgard-cli/README.md` only.
//!
//! Each test builds a private "toolbox" directory of `#!/bin/sh` stubs named like the tools and
//! runs the binary with `PATH` pointing at nothing else. The CLI runs a stub with an empty
//! environment inside Landlock, so a stub uses shell builtins or absolute paths and writes only
//! where it is told to (`-o <report>` for KiCad).

use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::{Value, json};

// ---------------------------------------------------------------- scaffolding

/// A unique temporary directory, removed on drop.
struct Dir(PathBuf);

impl Dir {
    fn new(label: &str) -> Dir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aet-t1-{}-{}-{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Dir(fs::canonicalize(path).unwrap())
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.join(name);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&path, bytes).unwrap();
        path
    }

    /// Writes an executable `#!/bin/sh` script.
    fn script(&self, name: &str, body: &str) {
        let path = self.write(name, format!("#!/bin/sh\n{body}\n").as_bytes());
        fs::set_permissions(&path, fs::Permissions::from_mode(0o755)).unwrap();
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn o<S: AsRef<OsStr> + ?Sized>(s: &S) -> &OsStr {
    s.as_ref()
}

/// Runs the binary with an empty environment, `PATH=<path>` and the given extra variables.
fn run(args: &[&OsStr], path: &Path, extra_env: &[(&str, &Path)]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .env_clear()
        .env("PATH", path)
        .envs(extra_env.iter().copied())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

/// Exactly one line of stdout holding one JSON object.
fn report(output: &Output) -> Value {
    let stdout = std::str::from_utf8(&output.stdout).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout: {stdout:?} stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let value: Value = serde_json::from_str(stdout).unwrap();
    assert!(value.is_object(), "{value}");
    value
}

/// A refusal: exit `code`, `ok:false`, `error.kind`, and never a `verdict` key.
fn refused(output: &Output, code: i32, kind: &str) -> Value {
    let report = report(output);
    assert_eq!(output.status.code(), Some(code), "{report}");
    assert_eq!(report["ok"], false, "{report}");
    assert_eq!(report["error"]["kind"], kind, "{report}");
    assert!(report.get("verdict").is_none(), "{report}");
    report
}

fn refused_id(output: &Output, code: i32, kind: &str, id: &str) {
    let report = refused(output, code, kind);
    assert_eq!(report["error"]["id"], id, "{report}");
}

/// A judged verdict: exit 0 for pass, 1 for fail.
fn judged(output: &Output, verdict: &str, check: &str) -> Value {
    let report = report(output);
    let code = if verdict == "pass" { 0 } else { 1 };
    assert_eq!(output.status.code(), Some(code), "{report}");
    assert_eq!(report["ok"], verdict == "pass", "{report}");
    assert_eq!(report["verdict"], verdict, "{report}");
    assert_eq!(report["check"], check, "{report}");
    report
}

fn sha256(bytes: &[u8]) -> String {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child.stdin.take().unwrap().write_all(bytes).unwrap();
    let out = child.wait_with_output().unwrap();
    String::from_utf8(out.stdout).unwrap()[..64].to_owned()
}

fn finding_count(report: &Value) -> usize {
    report["findings"].as_array().unwrap().len()
}

// ---------------------------------------------------------------- stub tools

#[derive(Clone, Copy)]
enum Version {
    Good,
    /// Prints nothing and exits 0.
    Empty,
    /// Prints a line and exits nonzero.
    Fails,
}

impl Version {
    fn shell(self, good_line: &str) -> String {
        match self {
            Version::Good => format!("printf '%s\\n' '{good_line}'; exit 0"),
            Version::Empty => "exit 0".into(),
            Version::Fails => "printf 'boom\\n'; exit 3".into(),
        }
    }
}

/// A `kicad-cli` stub. `pre` is extra shell run before the report is written.
struct Kicad {
    version: Version,
    exit: i32,
    report: Option<Value>,
    raw_report: Option<String>,
    pre: String,
    forbid_input: Option<PathBuf>,
}

impl Kicad {
    fn new(exit: i32, report: Option<Value>) -> Kicad {
        Kicad {
            version: Version::Good,
            exit,
            report,
            raw_report: None,
            pre: String::new(),
            forbid_input: None,
        }
    }

    fn install(&self, tb: &Dir) {
        if let Some(report) = &self.report {
            tb.write("report.json", report.to_string().as_bytes());
        }
        if let Some(raw) = &self.raw_report {
            tb.write("report.json", raw.as_bytes());
        }
        let has_report = self.report.is_some() || self.raw_report.is_some();
        let write_report = if has_report {
            format!("/bin/cat '{}' > \"$out\"", tb.join("report.json").display())
        } else {
            String::new()
        };
        let forbid = self.forbid_input.as_ref().map_or(String::new(), |p| {
            format!("if [ \"$last\" = '{}' ]; then exit 98; fi", p.display())
        });
        let body = format!(
            r#"if [ "$1" = version ]; then {version}; fi
out=; prev=; last=; flag=; fmt=
for a in "$@"; do
  if [ "$prev" = -o ]; then out=$a; fi
  if [ "$prev" = --format ] && [ "$a" = json ]; then fmt=1; fi
  if [ "$a" = --exit-code-violations ]; then flag=1; fi
  prev=$a; last=$a
done
case "$1 $2" in "pcb drc"|"sch erc") ;; *) exit 99;; esac
[ "$flag" = 1 ] || exit 99
[ "$fmt" = 1 ] || exit 99
[ -n "$out" ] || exit 99
case "$last" in /*) ;; *) exit 96;; esac
[ -f "$last" ] || exit 97
{forbid}
{pre}
{write_report}
exit {exit}"#,
            version = self.version.shell("10.0.7"),
            pre = self.pre,
            exit = self.exit,
        );
        tb.script("kicad-cli", &body);
    }
}

fn finding(kind: &str, severity: &str, description: &str) -> Value {
    json!({"type": kind, "severity": severity, "description": description,
           "items": [{"description": "Track [NET_B] on F.Cu, length 20.0000 mm",
                      "pos": {"x": 105.0, "y": 110.3}, "uuid": "11111111-2222-3333-4444-555555555555"}]})
}

fn clearance() -> Value {
    finding(
        "clearance",
        "error",
        "Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)",
    )
}

/// A KiCad 10 DRC report holding `violations`.
fn drc_report(violations: Vec<Value>) -> Value {
    json!({"$schema": "https://schemas.kicad.org/drc.v1.json", "coordinate_units": "mm",
           "date": "2026-10-21T00:00:00+0000", "ignored_checks": [],
           "included_severities": ["error", "warning"], "kicad_version": "10.0.7",
           "source": "board.kicad_pcb", "violations": violations,
           "unconnected_items": [], "schematic_parity": []})
}

/// A KiCad 10 ERC report holding `violations` on its root sheet.
fn erc_report(violations: Vec<Value>) -> Value {
    json!({"$schema": "https://schemas.kicad.org/erc.v1.json", "coordinate_units": "mm",
           "date": "2026-10-21T00:00:00+0000", "ignored_checks": [],
           "included_severities": ["error", "warning"], "kicad_version": "10.0.7",
           "source": "sheet.kicad_sch",
           "sheets": [{"path": "/", "uuid_path": "/x", "violations": violations}]})
}

const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} usart3 (STM32F7_USART)\n";
const LOAD_ERROR: &str = "There was an error executing command 'machine LoadPlatformDescription @/x/p.repl': \nCould not find file '/x/p.repl'.\n";

fn renode_stub(tb: &Dir, version: Version, exit: i32, stdout: &str) {
    tb.write("renode.out", stdout.as_bytes());
    let body = format!(
        "if [ \"$1\" = --version ]; then {}; fi\n/bin/cat '{}'\nexit {exit}",
        version.shell("Renode v1.17.0"),
        tb.join("renode.out").display()
    );
    tb.script("renode", &body);
}

fn renode_ok(tb: &Dir) {
    renode_stub(tb, Version::Good, 0, CPU_LISTING);
}

/// A `cargo` stub answering `cargo kani --version` and `cargo kani --harness <name>`, plus a
/// `cargo-kani` that merely exists.
fn kani_stub(tb: &Dir, version: Version, exit: i32, stdout: &str, stderr: &str) {
    tb.write("kani.out", stdout.as_bytes());
    tb.write("kani.err", stderr.as_bytes());
    let body = format!(
        r#"if [ "$1" = kani ] && [ "$2" = --version ]; then {version}; fi
[ "$1" = kani ] || exit 99
prev=; ok=; h=
for a in "$@"; do
  if [ "$prev" = --harness ] && [ -n "$a" ]; then ok=1; h=$a; fi
  prev=$a
done
[ "$ok" = 1 ] || exit 99
# Like real Kani, name the harness checked before its verdict.
case "$(/bin/cat '{out}')" in *"Checking harness"*) ;; *VERIFICATION*) printf 'Checking harness proofs::%s...\n' "$h";; esac
/bin/cat '{out}'
/bin/cat '{err}' >&2
exit {exit}"#,
        version = version.shell("Kani Rust Verifier 0.68.0 (cargo plugin)"),
        out = tb.join("kani.out").display(),
        err = tb.join("kani.err").display(),
    );
    tb.script("cargo", &body);
    tb.script("cargo-kani", "exit 0");
}

const KANI_PASS: &str = "VERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.\n";
const KANI_FAIL: &str = "Failed Checks: attempt to add with overflow\nVERIFICATION:- FAILED\nComplete - 0 successfully verified harnesses, 1 failures, 1 total.\n";
const KANI_COMPILE: &str = "error[E0277]: cannot add `&str` to `u8`\nerror: could not compile `smoke` (lib) due to 1 previous error\nerror: Failed to execute cargo (exit status: 101). Found 1 compilation errors.\n";
const KANI_NO_HARNESS: &str = "error: Failed to match the following harness(es):\n    nope\nPlease specify the correct harness name.\n";

// ---------------------------------------------------------------- fixtures

/// A directory holding one input file.
struct Input {
    dir: Dir,
    path: PathBuf,
}

fn input(name: &str, bytes: &[u8]) -> Input {
    let dir = Dir::new("input");
    let path = dir.write(name, bytes);
    Input { dir, path }
}

fn board() -> Input {
    input("board.kicad_pcb", b"(kicad_pcb (version 20240108))\n")
}

fn schematic() -> Input {
    input("sheet.kicad_sch", b"(kicad_sch (version 20250114))\n")
}

fn drc(tb: &Dir, input: &Path) -> Output {
    run(&[o("check"), o("drc"), o("--board"), o(input)], &tb.0, &[])
}

fn erc(tb: &Dir, input: &Path) -> Output {
    run(
        &[o("check"), o("erc"), o("--schematic"), o(input)],
        &tb.0,
        &[],
    )
}

fn simulate(tb: &Dir, platform: &OsStr) -> Output {
    run(&[o("simulate"), o("--platform"), platform], &tb.0, &[])
}

/// A crate directory with a manifest and one source file.
fn krate() -> Input {
    let dir = Dir::new("crate");
    dir.write("Cargo.toml", b"[package]\nname = \"smoke\"\n");
    dir.write("src/lib.rs", b"pub fn f() {}\n");
    let path = dir.0.clone();
    Input { dir, path }
}

/// Existing empty directories for `KANI_HOME` and `RUSTUP_HOME`.
struct Homes {
    _dir: Dir,
    kani: PathBuf,
    rustup: PathBuf,
}

fn homes() -> Homes {
    let dir = Dir::new("homes");
    let kani = dir.join("kani");
    let rustup = dir.join("rustup");
    fs::create_dir(&kani).unwrap();
    fs::create_dir(&rustup).unwrap();
    Homes {
        _dir: dir,
        kani,
        rustup,
    }
}

fn prove_with(tb: &Dir, krate: &Path, harness: &str, vars: &[(&str, &Path)]) -> Output {
    run(
        &[
            o("prove"),
            o("--crate"),
            o(krate),
            o("--harness"),
            o(harness),
        ],
        &tb.0,
        vars,
    )
}

fn prove(tb: &Dir, krate: &Path, harness: &str) -> Output {
    let h = homes();
    prove_with(
        tb,
        krate,
        harness,
        &[("KANI_HOME", &h.kani), ("RUSTUP_HOME", &h.rustup)],
    )
}

// ---------------------------------------------------------------- 1. tools absent

#[test]
fn drc_without_kicad_cli_is_tool_missing() {
    let tb = Dir::new("tb");
    let b = board();
    refused_id(&drc(&tb, &b.path), 4, "ToolMissing", "kicad-cli");
}

#[test]
fn erc_without_kicad_cli_is_tool_missing() {
    let tb = Dir::new("tb");
    let s = schematic();
    refused_id(&erc(&tb, &s.path), 4, "ToolMissing", "kicad-cli");
}

#[test]
fn simulate_without_renode_is_tool_missing() {
    let tb = Dir::new("tb");
    let p = input("p.repl", b"cpu: CPU.CortexM @ sysbus\n");
    refused_id(
        &simulate(&tb, p.path.as_os_str()),
        4,
        "ToolMissing",
        "renode",
    );
}

#[test]
fn prove_without_cargo_is_tool_missing() {
    let tb = Dir::new("tb");
    let c = krate();
    refused_id(&prove(&tb, &c.path, "h"), 4, "ToolMissing", "cargo");
}

#[test]
fn prove_without_cargo_kani_is_tool_missing() {
    let tb = Dir::new("tb");
    tb.script("cargo", "exit 0");
    let c = krate();
    refused_id(&prove(&tb, &c.path, "h"), 4, "ToolMissing", "cargo-kani");
}

#[test]
fn a_tool_that_is_not_executable_is_refused_with_4() {
    let tb = Dir::new("tb");
    tb.write("kicad-cli", b"#!/bin/sh\nexit 0\n");
    let b = board();
    let output = drc(&tb, &b.path);
    assert_eq!(output.status.code(), Some(4));
    let report = report(&output);
    assert_eq!(report["ok"], false);
    assert!(report.get("verdict").is_none());
    assert_eq!(report["error"]["id"], "kicad-cli");
}

// ---------------------------------------------------------------- 2. prove environment

#[test]
fn prove_without_kani_home_is_tool_unusable() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let h = homes();
    let output = prove_with(&tb, &c.path, "h", &[("RUSTUP_HOME", &h.rustup)]);
    refused_id(&output, 4, "ToolUnusable", "KANI_HOME");
}

#[test]
fn prove_without_rustup_home_is_tool_unusable() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let h = homes();
    let output = prove_with(&tb, &c.path, "h", &[("KANI_HOME", &h.kani)]);
    refused_id(&output, 4, "ToolUnusable", "RUSTUP_HOME");
}

#[test]
fn prove_with_kani_home_not_a_directory_is_tool_unusable() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let h = homes();
    let missing = h.kani.join("absent");
    let output = prove_with(
        &tb,
        &c.path,
        "h",
        &[("KANI_HOME", &missing), ("RUSTUP_HOME", &h.rustup)],
    );
    refused_id(&output, 4, "ToolUnusable", "KANI_HOME");
}

// ---------------------------------------------------------------- 3. version refusals

#[test]
fn kicad_version_printing_nothing_is_tool_unusable() {
    let tb = Dir::new("tb");
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.version = Version::Empty;
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 4, "ToolUnusable", "kicad-cli");
}

#[test]
fn kicad_version_exiting_nonzero_is_tool_unusable() {
    let tb = Dir::new("tb");
    let mut k = Kicad::new(0, Some(erc_report(vec![])));
    k.version = Version::Fails;
    k.install(&tb);
    let s = schematic();
    refused_id(&erc(&tb, &s.path), 4, "ToolUnusable", "kicad-cli");
}

#[test]
fn renode_version_printing_nothing_is_tool_unusable() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Empty, 0, CPU_LISTING);
    let p = input("p.repl", b"x\n");
    refused_id(
        &simulate(&tb, p.path.as_os_str()),
        4,
        "ToolUnusable",
        "renode",
    );
}

#[test]
fn renode_version_exiting_nonzero_is_tool_unusable() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Fails, 0, CPU_LISTING);
    let p = input("p.repl", b"x\n");
    refused_id(
        &simulate(&tb, p.path.as_os_str()),
        4,
        "ToolUnusable",
        "renode",
    );
}

#[test]
fn kani_version_printing_nothing_is_tool_unusable() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Empty, 0, KANI_PASS, "");
    let c = krate();
    let output = prove(&tb, &c.path, "h");
    let report = refused(&output, 4, "ToolUnusable");
    assert!(
        report["error"]["id"] == "cargo-kani" || report["error"]["id"] == "cargo",
        "{report}"
    );
}

#[test]
fn kani_version_exiting_nonzero_is_tool_unusable() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Fails, 0, KANI_PASS, "");
    let c = krate();
    refused(&prove(&tb, &c.path, "h"), 4, "ToolUnusable");
}

// ---------------------------------------------------------------- 4. input path refusals

/// A toolbox where every tool works, so only the input can be at fault.
fn full_toolbox() -> Dir {
    let tb = Dir::new("tb");
    Kicad::new(0, Some(drc_report(vec![]))).install(&tb);
    tb
}

#[test]
fn nonexistent_board_is_unreadable_input() {
    let tb = full_toolbox();
    let gone = tb.join("no-such.kicad_pcb");
    refused(&drc(&tb, &gone), 3, "UnreadableInput");
}

#[test]
fn input_is_checked_before_the_tool_is_looked_for() {
    let tb = Dir::new("tb");
    let gone = tb.join("no-such.kicad_pcb");
    refused(&drc(&tb, &gone), 3, "UnreadableInput");
}

#[test]
fn nonexistent_schematic_is_unreadable_input() {
    let tb = full_toolbox();
    let gone = tb.join("no-such.kicad_sch");
    refused(&erc(&tb, &gone), 3, "UnreadableInput");
}

#[test]
fn directory_as_board_is_unreadable_input() {
    let tb = full_toolbox();
    let d = Dir::new("d");
    refused(&drc(&tb, &d.0), 3, "UnreadableInput");
}

#[test]
fn directory_as_schematic_is_unreadable_input() {
    let tb = full_toolbox();
    let d = Dir::new("d");
    refused(&erc(&tb, &d.0), 3, "UnreadableInput");
}

#[test]
fn directory_as_platform_is_unreadable_input() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    let d = Dir::new("d");
    refused(&simulate(&tb, d.0.as_os_str()), 3, "UnreadableInput");
}

#[test]
fn regular_file_as_crate_is_unreadable_input() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let f = input("Cargo.toml", b"x");
    refused(&prove(&tb, &f.path, "h"), 3, "UnreadableInput");
}

#[test]
fn fifo_as_board_is_unreadable_input() {
    let tb = full_toolbox();
    let d = Dir::new("d");
    let fifo = d.join("board.kicad_pcb");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    refused(&drc(&tb, &fifo), 3, "UnreadableInput");
}

#[test]
fn symlink_to_dev_null_as_board_is_unreadable_input() {
    let tb = full_toolbox();
    let d = Dir::new("d");
    let link = d.join("board.kicad_pcb");
    symlink("/dev/null", &link).unwrap();
    refused(&drc(&tb, &link), 3, "UnreadableInput");
}

#[test]
fn symlink_to_a_regular_file_is_followed() {
    let tb = full_toolbox();
    let b = board();
    let d = Dir::new("d");
    let link = d.join("link.kicad_pcb");
    symlink(&b.path, &link).unwrap();
    let report = judged(&drc(&tb, &link), "pass", "drc");
    assert_eq!(report["input"]["path"], b.path.to_str().unwrap());
}

#[test]
fn crate_holding_a_symlink_is_unreadable_input() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    symlink("src/lib.rs", c.dir.join("alias.rs")).unwrap();
    refused(&prove(&tb, &c.path, "h"), 3, "UnreadableInput");
}

#[test]
fn crate_holding_a_fifo_is_unreadable_input() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let fifo = c.dir.join("src/pipe");
    assert!(
        Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .unwrap()
            .success()
    );
    refused(&prove(&tb, &c.path, "h"), 3, "UnreadableInput");
}

#[test]
fn crate_holding_a_non_utf8_name_is_unreadable_input() {
    use std::os::unix::ffi::OsStrExt;
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    fs::write(c.dir.0.join(OsStr::from_bytes(b"src/bad\xff.rs")), b"x").unwrap();
    refused(&prove(&tb, &c.path, "h"), 3, "UnreadableInput");
}

// ---------------------------------------------------------------- 5. usage

fn usage(args: &[&str]) {
    let tb = full_toolbox();
    let args: Vec<&OsStr> = args.iter().map(|a| o(*a)).collect();
    refused(&run(&args, &tb.0, &[]), 64, "UsageError");
}

#[test]
fn drc_without_board_is_a_usage_error() {
    usage(&["check", "drc"]);
}

#[test]
fn erc_without_schematic_is_a_usage_error() {
    usage(&["check", "erc"]);
}

#[test]
fn prove_without_harness_is_a_usage_error() {
    usage(&["prove", "--crate", "/tmp"]);
}

#[test]
fn prove_without_crate_is_a_usage_error() {
    usage(&["prove", "--harness", "h"]);
}

#[test]
fn simulate_without_platform_is_a_usage_error() {
    usage(&["simulate"]);
}

#[test]
fn check_without_a_kind_is_a_usage_error() {
    usage(&["check"]);
}

#[test]
fn unknown_flag_on_a_tool_subcommand_is_a_usage_error() {
    usage(&["check", "drc", "--board", "/tmp/x", "--frobnicate"]);
}

#[test]
fn unknown_check_kind_is_a_usage_error() {
    usage(&["check", "lvs", "--board", "/tmp/x"]);
}

// ---------------------------------------------------------------- 6. lying KiCad stubs

fn lying_drc(k: Kicad) {
    let tb = Dir::new("tb");
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn kicad_exit_0_and_no_report_is_tool_error() {
    lying_drc(Kicad::new(0, None));
}

#[test]
fn kicad_exit_0_with_a_violating_report_is_tool_error() {
    lying_drc(Kicad::new(0, Some(drc_report(vec![clearance()]))));
}

#[test]
fn kicad_exit_5_with_an_empty_report_is_tool_error() {
    lying_drc(Kicad::new(5, Some(drc_report(vec![]))));
}

#[test]
fn kicad_exit_5_and_no_report_is_tool_error() {
    lying_drc(Kicad::new(5, None));
}

#[test]
fn kicad_exit_2_with_a_clean_report_is_tool_error() {
    lying_drc(Kicad::new(2, Some(drc_report(vec![]))));
}

#[test]
fn kicad_garbage_report_is_tool_error() {
    let mut k = Kicad::new(0, None);
    k.raw_report = Some("this is not json".into());
    lying_drc(k);
}

#[test]
fn kicad_report_of_the_wrong_shape_is_tool_error() {
    let mut k = Kicad::new(0, None);
    k.raw_report = Some("{}".into());
    lying_drc(k);
}

#[test]
fn kicad_killed_by_sigkill_is_tool_error() {
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.pre = "kill -9 $$".into();
    lying_drc(k);
}

#[test]
fn kicad_stdout_over_16_mib_is_tool_error() {
    let tb = Dir::new("tb");
    tb.write("big.bin", &vec![b'a'; 16 * 1024 * 1024 + 1]);
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.pre = format!("/bin/cat '{}'", tb.join("big.bin").display());
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn kicad_stderr_over_16_mib_is_tool_error() {
    let tb = Dir::new("tb");
    tb.write("big.bin", &vec![b'a'; 16 * 1024 * 1024 + 1]);
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.pre = format!("/bin/cat '{}' >&2", tb.join("big.bin").display());
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn kicad_report_with_error_only_severities_is_tool_error() {
    let mut report = drc_report(vec![]);
    report["included_severities"] = json!(["error"]);
    lying_drc(Kicad::new(0, Some(report)));
}

#[test]
fn kicad_report_with_a_different_version_is_tool_error() {
    let mut report = drc_report(vec![]);
    report["kicad_version"] = json!("9.0.8");
    lying_drc(Kicad::new(0, Some(report)));
}

#[test]
fn erc_exit_0_with_a_violating_sheet_is_tool_error() {
    let tb = Dir::new("tb");
    Kicad::new(
        0,
        Some(erc_report(vec![finding(
            "pin_not_connected",
            "error",
            "Pin not connected",
        )])),
    )
    .install(&tb);
    let s = schematic();
    refused_id(&erc(&tb, &s.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn erc_exit_5_with_an_empty_sheet_is_tool_error() {
    let tb = Dir::new("tb");
    Kicad::new(5, Some(erc_report(vec![]))).install(&tb);
    let s = schematic();
    refused_id(&erc(&tb, &s.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn erc_report_with_a_different_version_is_tool_error() {
    let tb = Dir::new("tb");
    let mut report = erc_report(vec![]);
    report["kicad_version"] = json!("9.0.8");
    Kicad::new(0, Some(report)).install(&tb);
    let s = schematic();
    refused_id(&erc(&tb, &s.path), 5, "ToolError", "kicad-cli");
}

// ---------------------------------------------------------------- 7. honest KiCad stubs

#[test]
fn drc_pass_reports_the_full_verdict() {
    let tb = Dir::new("tb");
    Kicad::new(0, Some(drc_report(vec![]))).install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "pass", "drc");
    assert_eq!(report["findings"], json!([]));
    assert_eq!(report["included_severities"], json!(["error", "warning"]));
    assert_eq!(report["tool"]["name"], "kicad-cli");
    assert_eq!(report["tool"]["version"], "10.0.7");
    assert_eq!(report["tool"]["exit_code"], 0);
    assert!(report["sandbox"]["landlock_abi"].as_u64().unwrap() >= 3);
    assert!(report["sandbox"]["tcp_confined"].is_boolean());
    assert!(report["sandbox"]["scoped"].is_boolean());
}

#[test]
fn drc_fail_normalizes_the_finding() {
    let tb = Dir::new("tb");
    Kicad::new(5, Some(drc_report(vec![clearance()]))).install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "fail", "drc");
    assert_eq!(report["tool"]["exit_code"], 5);
    assert_eq!(report["included_severities"], json!(["error", "warning"]));
    assert_eq!(finding_count(&report), 1);
    let f = &report["findings"][0];
    assert_eq!(f["kind"], "clearance");
    assert_eq!(f["severity"], "error");
    assert_eq!(
        f["message"],
        "Clearance violation ( clearance 0.2000 mm; actual 0.0500 mm)"
    );
    let at = &f["where"];
    assert_eq!(at.as_array().unwrap().len(), 1);
    assert_eq!(
        at[0]["description"],
        "Track [NET_B] on F.Cu, length 20.0000 mm"
    );
    assert_eq!(at[0]["x"].as_f64(), Some(105.0));
    assert_eq!(at[0]["y"].as_f64(), Some(110.3));
    assert_eq!(at[0]["uuid"], "11111111-2222-3333-4444-555555555555");
}

#[test]
fn drc_warning_alone_is_a_fail() {
    let tb = Dir::new("tb");
    let warning = finding("silk_overlap", "warning", "Silkscreen overlap");
    Kicad::new(5, Some(drc_report(vec![warning]))).install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "fail", "drc");
    assert_eq!(finding_count(&report), 1);
    assert_eq!(report["findings"][0]["kind"], "silk_overlap");
    assert_eq!(report["findings"][0]["severity"], "warning");
}

#[test]
fn drc_two_violations_give_two_findings() {
    let tb = Dir::new("tb");
    let second = finding("track_dangling", "warning", "Track has unconnected end");
    Kicad::new(5, Some(drc_report(vec![clearance(), second]))).install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "fail", "drc");
    assert_eq!(finding_count(&report), 2);
}

#[test]
fn drc_unconnected_items_count_as_findings() {
    let tb = Dir::new("tb");
    let mut report = drc_report(vec![]);
    report["unconnected_items"] =
        json!([finding("unconnected_items", "error", "Missing connection")]);
    Kicad::new(5, Some(report)).install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "fail", "drc");
    assert_eq!(finding_count(&report), 1);
}

#[test]
fn erc_pass_reports_the_full_verdict() {
    let tb = Dir::new("tb");
    Kicad::new(0, Some(erc_report(vec![]))).install(&tb);
    let s = schematic();
    let report = judged(&erc(&tb, &s.path), "pass", "erc");
    assert_eq!(report["findings"], json!([]));
    assert_eq!(report["included_severities"], json!(["error", "warning"]));
    assert_eq!(report["tool"]["name"], "kicad-cli");
    assert_eq!(report["tool"]["version"], "10.0.7");
    assert_eq!(report["tool"]["exit_code"], 0);
    assert!(report["sandbox"]["landlock_abi"].as_u64().unwrap() >= 3);
}

#[test]
fn erc_fail_normalizes_the_finding_from_the_sheet() {
    let tb = Dir::new("tb");
    let v = finding("pin_not_connected", "error", "Pin not connected");
    Kicad::new(5, Some(erc_report(vec![v]))).install(&tb);
    let s = schematic();
    let report = judged(&erc(&tb, &s.path), "fail", "erc");
    assert_eq!(finding_count(&report), 1);
    let f = &report["findings"][0];
    assert_eq!(f["kind"], "pin_not_connected");
    assert_eq!(f["severity"], "error");
    assert_eq!(f["message"], "Pin not connected");
    assert_eq!(
        f["where"][0]["uuid"],
        "11111111-2222-3333-4444-555555555555"
    );
}

// ---------------------------------------------------------------- 8. hashes and paths

#[test]
fn drc_input_hash_and_path_are_of_the_original_bytes() {
    let tb = full_toolbox();
    let b = board();
    let report = judged(&drc(&tb, &b.path), "pass", "drc");
    assert_eq!(
        report["input"]["sha256"],
        sha256(&fs::read(&b.path).unwrap())
    );
    assert_eq!(report["input"]["path"], b.path.to_str().unwrap());
}

#[test]
fn input_path_is_canonical_when_given_with_dots() {
    let tb = full_toolbox();
    let b = board();
    let dotted = b
        .dir
        .0
        .join("../")
        .join(b.dir.0.file_name().unwrap())
        .join("./board.kicad_pcb");
    let report = judged(&drc(&tb, &dotted), "pass", "drc");
    assert_eq!(report["input"]["path"], b.path.to_str().unwrap());
}

#[test]
fn erc_input_hash_is_of_the_original_bytes() {
    let tb = Dir::new("tb");
    Kicad::new(0, Some(erc_report(vec![]))).install(&tb);
    let s = schematic();
    let report = judged(&erc(&tb, &s.path), "pass", "erc");
    assert_eq!(
        report["input"]["sha256"],
        sha256(&fs::read(&s.path).unwrap())
    );
    assert_eq!(report["input"]["path"], s.path.to_str().unwrap());
}

/// The README's crate hash: sorted `<sha256>  <relative path>\n` lines, hashed, `target/` skipped.
fn crate_hash(root: &Path) -> String {
    fn walk(root: &Path, dir: &Path, lines: &mut Vec<(String, String)>) {
        for entry in fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let rel = path
                .strip_prefix(root)
                .unwrap()
                .to_str()
                .unwrap()
                .to_owned();
            if path.is_dir() {
                if rel != "target" {
                    walk(root, &path, lines);
                }
            } else {
                lines.push((rel, sha256(&fs::read(&path).unwrap())));
            }
        }
    }
    let mut lines = Vec::new();
    walk(root, root, &mut lines);
    lines.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));
    let text: String = lines.iter().map(|(p, h)| format!("{h}  {p}\n")).collect();
    sha256(text.as_bytes())
}

#[test]
fn prove_hashes_the_crate_tree_without_target() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    c.dir
        .write("src/target/inner.rs", b"// a nested target/ is source\n");
    c.dir.write("target/debug/junk.bin", b"build output");
    let expected = crate_hash(&c.path);
    let report = judged(&prove(&tb, &c.path, "add_in_range"), "pass", "prove");
    assert_eq!(report["input"]["sha256"], expected);
    assert_eq!(report["input"]["path"], c.path.to_str().unwrap());
}

#[test]
fn prove_hash_ignores_the_top_level_target_directory() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let without = judged(&prove(&tb, &c.path, "h"), "pass", "prove");
    c.dir.write("target/debug/junk.bin", b"build output");
    let with = judged(&prove(&tb, &c.path, "h"), "pass", "prove");
    assert_eq!(without["input"]["sha256"], with["input"]["sha256"]);
}

#[test]
fn prove_hash_changes_with_a_file_name() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let before = judged(&prove(&tb, &c.path, "h"), "pass", "prove");
    fs::rename(c.dir.join("src/lib.rs"), c.dir.join("src/other.rs")).unwrap();
    let after = judged(&prove(&tb, &c.path, "h"), "pass", "prove");
    assert_ne!(before["input"]["sha256"], after["input"]["sha256"]);
}

// ---------------------------------------------------------------- 9. Renode

#[test]
fn simulate_with_a_cpu_and_no_error_passes() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    let p = input("p.repl", b"cpu: CPU.CortexM @ sysbus\n");
    let report = judged(&simulate(&tb, p.path.as_os_str()), "pass", "simulate");
    assert_eq!(report["machine"]["cpus"], json!(["cpu"]));
    let peripherals = report["machine"]["peripherals"].as_array().unwrap();
    assert!(peripherals.contains(&json!({"name": "cpu", "type": "CortexM"})));
    assert!(peripherals.contains(&json!({"name": "usart3", "type": "STM32F7_USART"})));
    assert_eq!(report["findings"], json!([]));
    assert_eq!(report["tool"]["name"], "renode");
    assert_eq!(report["tool"]["version"], "Renode v1.17.0");
    assert_eq!(report["tool"]["exit_code"], 0);
    assert_eq!(
        report["input"]["sha256"],
        sha256(b"cpu: CPU.CortexM @ sysbus\n")
    );
    assert!(report["sandbox"]["landlock_abi"].as_u64().unwrap() >= 3);
}

#[test]
fn simulate_with_a_load_error_and_no_cpu_fails() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Good, 0, LOAD_ERROR);
    let p = input("p.repl", b"x\n");
    let report = judged(&simulate(&tb, p.path.as_os_str()), "fail", "simulate");
    assert_eq!(finding_count(&report), 1);
    assert_eq!(report["findings"][0]["kind"], "platform_load_error");
    assert_eq!(report["findings"][0]["severity"], "error");
    assert_eq!(report["findings"][0]["where"], json!([]));
    assert_eq!(report["tool"]["exit_code"], 0);
}

#[test]
fn simulate_with_both_a_cpu_and_an_error_is_tool_error() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Good, 0, &format!("{LOAD_ERROR}{CPU_LISTING}"));
    let p = input("p.repl", b"x\n");
    refused_id(&simulate(&tb, p.path.as_os_str()), 5, "ToolError", "renode");
}

#[test]
fn simulate_with_neither_a_cpu_nor_an_error_is_tool_error() {
    let tb = Dir::new("tb");
    renode_stub(
        &tb,
        Version::Good,
        0,
        "Available peripherals:\n  (machine-0)\n",
    );
    let p = input("p.repl", b"x\n");
    refused_id(&simulate(&tb, p.path.as_os_str()), 5, "ToolError", "renode");
}

#[test]
fn simulate_with_no_output_is_tool_error() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Good, 0, "");
    let p = input("p.repl", b"x\n");
    refused_id(&simulate(&tb, p.path.as_os_str()), 5, "ToolError", "renode");
}

#[test]
fn simulate_exiting_nonzero_with_a_cpu_is_tool_error() {
    let tb = Dir::new("tb");
    renode_stub(&tb, Version::Good, 1, CPU_LISTING);
    let p = input("p.repl", b"x\n");
    refused_id(&simulate(&tb, p.path.as_os_str()), 5, "ToolError", "renode");
}

#[test]
fn simulate_killed_by_sigkill_is_tool_error() {
    let tb = Dir::new("tb");
    tb.script(
        "renode",
        "if [ \"$1\" = --version ]; then printf 'Renode v1.17.0\\n'; exit 0; fi\nkill -9 $$",
    );
    let p = input("p.repl", b"x\n");
    refused_id(&simulate(&tb, p.path.as_os_str()), 5, "ToolError", "renode");
}

#[test]
fn at_platform_resolves_inside_the_renode_directory() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    let repl = tb.write("platforms/x.repl", b"cpu: CPU.CortexM @ sysbus\n");
    let report = judged(&simulate(&tb, o("@platforms/x.repl")), "pass", "simulate");
    assert_eq!(report["input"]["path"], repl.to_str().unwrap());
    assert_eq!(
        report["input"]["sha256"],
        sha256(b"cpu: CPU.CortexM @ sysbus\n")
    );
}

#[test]
fn at_platform_resolves_against_the_real_renode_behind_a_symlink() {
    let real = Dir::new("real");
    renode_ok(&real);
    let repl = real.write("platforms/x.repl", b"cpu: CPU.CortexM @ sysbus\n");
    let bin = Dir::new("bin");
    symlink(real.join("renode"), bin.join("renode")).unwrap();
    let report = judged(&simulate(&bin, o("@platforms/x.repl")), "pass", "simulate");
    assert_eq!(report["input"]["path"], repl.to_str().unwrap());
}

#[test]
fn at_platform_escaping_the_renode_tree_is_unreadable_input() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    refused(&simulate(&tb, o("@../../etc/passwd")), 3, "UnreadableInput");
}

#[test]
fn at_platform_escaping_by_a_symlink_is_unreadable_input() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    let outside = input("secret.repl", b"x\n");
    fs::create_dir(tb.join("platforms")).unwrap();
    symlink(&outside.path, tb.join("platforms/leak.repl")).unwrap();
    refused(
        &simulate(&tb, o("@platforms/leak.repl")),
        3,
        "UnreadableInput",
    );
}

#[test]
fn at_platform_naming_nothing_is_unreadable_input() {
    let tb = Dir::new("tb");
    renode_ok(&tb);
    refused(
        &simulate(&tb, o("@platforms/missing.repl")),
        3,
        "UnreadableInput",
    );
}

// ---------------------------------------------------------------- 10. Kani

#[test]
fn prove_pass_reports_the_harness() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    let report = judged(&prove(&tb, &c.path, "add_in_range"), "pass", "prove");
    assert_eq!(report["harness"], "add_in_range");
    assert_eq!(report["findings"], json!([]));
    assert_eq!(
        report["tool"]["version"],
        "Kani Rust Verifier 0.68.0 (cargo plugin)"
    );
    assert_eq!(report["tool"]["exit_code"], 0);
    assert!(report["sandbox"]["landlock_abi"].as_u64().unwrap() >= 3);
}

fn failing_proof(stdout: &str, stderr: &str, kind: &str) {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 1, stdout, stderr);
    let c = krate();
    let report = judged(&prove(&tb, &c.path, "h"), "fail", "prove");
    assert_eq!(report["harness"], "h");
    assert_eq!(report["tool"]["exit_code"], 1);
    assert_eq!(finding_count(&report), 1);
    assert_eq!(report["findings"][0]["kind"], kind);
    assert_eq!(report["findings"][0]["severity"], "error");
    assert_eq!(report["findings"][0]["where"], json!([]));
}

#[test]
fn prove_failed_verification_is_proof_failed() {
    failing_proof(KANI_FAIL, "", "proof_failed");
}

#[test]
fn prove_compile_error_is_compile_error() {
    failing_proof("", KANI_COMPILE, "compile_error");
}

#[test]
fn prove_missing_harness_is_harness_not_found() {
    failing_proof("", KANI_NO_HARNESS, "harness_not_found");
}

fn untrusted_proof(exit: i32, stdout: &str, stderr: &str) {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, exit, stdout, stderr);
    let c = krate();
    let output = prove(&tb, &c.path, "h");
    let report = refused(&output, 5, "ToolError");
    assert_eq!(report["error"]["id"], "cargo-kani", "{report}");
}

#[test]
fn prove_exit_0_with_a_failed_line_is_tool_error() {
    untrusted_proof(0, KANI_FAIL, "");
}

#[test]
fn prove_exit_1_with_nothing_recognizable_is_tool_error() {
    untrusted_proof(1, "something odd happened\n", "");
}

#[test]
fn prove_exit_2_is_tool_error() {
    untrusted_proof(2, KANI_PASS, "");
}

#[test]
fn prove_exit_0_with_no_output_is_tool_error() {
    untrusted_proof(0, "", "");
}

#[test]
fn prove_successful_line_with_a_failing_summary_is_tool_error() {
    untrusted_proof(
        0,
        "VERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 1 failures, 1 total.\n",
        "",
    );
}

#[test]
fn prove_successful_line_with_zero_verified_harnesses_is_tool_error() {
    untrusted_proof(
        0,
        "VERIFICATION:- SUCCESSFUL\nComplete - 0 successfully verified harnesses, 0 failures, 0 total.\n",
        "",
    );
}

#[test]
fn prove_exit_1_with_a_successful_line_is_tool_error() {
    untrusted_proof(1, KANI_PASS, "");
}

#[test]
fn prove_killed_by_sigkill_is_tool_error() {
    let tb = Dir::new("tb");
    tb.script(
        "cargo",
        "if [ \"$2\" = --version ]; then printf 'Kani Rust Verifier 0.68.0 (cargo plugin)\\n'; exit 0; fi\nkill -9 $$",
    );
    tb.script("cargo-kani", "exit 0");
    let c = krate();
    refused(&prove(&tb, &c.path, "h"), 5, "ToolError");
}

// ---------------------------------------------------------------- 11. the copy is judged

const ORIGINAL: &[u8] = b"(kicad_pcb (version 20240108))\n";

/// A KiCad stub that tries to write to the original input's directory and to the file itself.
fn writing_stub(tb: &Dir, i: &Input) {
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.pre = format!(
        "{{ printf pwned > '{dir}/pwned'; }} 2>/dev/null\n{{ printf pwned >> '{file}'; }} 2>/dev/null\n{{ printf pwned > '{file}'; }} 2>/dev/null\n: ",
        dir = i.dir.0.display(),
        file = i.path.display()
    );
    k.forbid_input = Some(i.path.clone());
    k.install(tb);
}

#[test]
fn a_stub_cannot_modify_the_original_input() {
    let tb = Dir::new("tb");
    let i = input("board.kicad_pcb", ORIGINAL);
    writing_stub(&tb, &i);
    let report = judged(&drc(&tb, &i.path), "pass", "drc");
    assert_eq!(fs::read(&i.path).unwrap(), ORIGINAL);
    assert_eq!(report["input"]["sha256"], sha256(ORIGINAL));
}

#[test]
fn a_stub_cannot_create_files_beside_the_original_input() {
    let tb = Dir::new("tb");
    let i = input("board.kicad_pcb", ORIGINAL);
    writing_stub(&tb, &i);
    judged(&drc(&tb, &i.path), "pass", "drc");
    assert!(!i.dir.join("pwned").exists());
}

#[test]
fn nothing_is_left_beside_the_input_after_a_run() {
    let tb = full_toolbox();
    let i = input("board.kicad_pcb", ORIGINAL);
    judged(&drc(&tb, &i.path), "pass", "drc");
    let names: Vec<_> = fs::read_dir(&i.dir.0)
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names, vec![OsStr::new("board.kicad_pcb")]);
}

#[test]
fn the_tool_is_handed_a_copy_whose_bytes_match_the_hash() {
    // The stub exits 98 if given the original path, and 97 if its argument is not a file.
    let tb = Dir::new("tb");
    let i = input("board.kicad_pcb", ORIGINAL);
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    k.forbid_input = Some(i.path.clone());
    k.install(&tb);
    let report = judged(&drc(&tb, &i.path), "pass", "drc");
    assert_eq!(report["input"]["path"], i.path.to_str().unwrap());
}

// ---------------------------------------------------------------- adversarial-pass fixes (builder)

#[test]
fn a_report_that_is_a_fifo_is_tool_error_not_a_hang() {
    let tb = Dir::new("tb");
    let mut k = Kicad::new(0, None);
    k.pre = "/usr/bin/mkfifo \"$out\"".into();
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn a_report_that_is_a_symlink_is_not_followed() {
    let tb = Dir::new("tb");
    let elsewhere = tb.write("elsewhere.json", drc_report(vec![]).to_string().as_bytes());
    let mut k = Kicad::new(0, None);
    k.pre = format!("/bin/ln -s '{}' \"$out\"", elsewhere.display());
    k.install(&tb);
    let b = board();
    refused_id(&drc(&tb, &b.path), 5, "ToolError", "kicad-cli");
}

#[test]
fn a_harness_name_that_only_matches_as_a_substring_is_not_found() {
    let tb = Dir::new("tb");
    let out = format!("Checking harness proofs::add_in_range...\n{KANI_PASS}");
    kani_stub(&tb, Version::Good, 0, &out, "");
    let c = krate();
    let report = judged(&prove(&tb, &c.path, "add_in"), "fail", "prove");
    assert_eq!(
        report["findings"][0]["kind"], "harness_not_found",
        "{report}"
    );
}

#[test]
fn a_harness_name_matching_two_harnesses_is_ambiguous() {
    let tb = Dir::new("tb");
    let out = "Checking harness a::check...\nVERIFICATION:- SUCCESSFUL\nChecking harness b::check...\nVERIFICATION:- SUCCESSFUL\nComplete - 2 successfully verified harnesses, 0 failures, 2 total.\n";
    kani_stub(&tb, Version::Good, 0, out, "");
    let c = krate();
    let report = judged(&prove(&tb, &c.path, "check"), "fail", "prove");
    assert_eq!(
        report["findings"][0]["kind"], "harness_ambiguous",
        "{report}"
    );
}

#[test]
fn a_harness_that_is_not_a_rust_path_is_a_usage_error() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    let c = krate();
    for bad in [
        "add in",
        "add\nVERIFICATION:- SUCCESSFUL",
        "--help",
        "a::",
        "",
    ] {
        refused(&prove(&tb, &c.path, bad), 64, "UsageError");
    }
}

#[test]
fn a_qualified_harness_is_passed_with_exact() {
    let tb = Dir::new("tb");
    kani_stub(&tb, Version::Good, 0, KANI_PASS, "");
    // The stub refuses unless --exact is present.
    let cargo = tb.join("cargo");
    let body = fs::read_to_string(&cargo).unwrap().replace(
        "[ \"$ok\" = 1 ] || exit 99",
        "[ \"$ok\" = 1 ] || exit 99\ncase \" $* \" in *\" --exact \"*) ;; *) exit 98;; esac",
    );
    fs::write(&cargo, body).unwrap();
    let c = krate();
    judged(
        &prove(&tb, &c.path, "proofs::add_in_range"),
        "pass",
        "prove",
    );
}

#[test]
fn a_non_utf8_input_path_is_unreadable_input() {
    use std::os::unix::ffi::OsStrExt;
    let tb = Dir::new("tb");
    Kicad::new(0, Some(drc_report(vec![]))).install(&tb);
    let dir = Dir::new("in");
    let name = OsStr::from_bytes(b"bad\xff.kicad_pcb");
    let path = dir.0.join(name);
    fs::write(&path, b"x").unwrap();
    refused(&drc(&tb, &path), 3, "UnreadableInput");
}

#[test]
fn a_daemon_in_its_own_session_does_not_outlive_the_call() {
    let tb = Dir::new("tb");
    let mut k = Kicad::new(0, Some(drc_report(vec![])));
    // A daemon in a new session, stdio closed so it holds no pipe open.
    k.pre = "/usr/bin/setsid /bin/sh -c 'exec 1>&- 2>&-; exec /bin/sleep 300' &".into();
    k.install(&tb);
    let b = board();
    let output = drc(&tb, &b.path);
    judged(&output, "pass", "drc");
    // No `sleep 300` from the stub may remain once the call has returned.
    std::thread::sleep(std::time::Duration::from_millis(300));
    let survivors: Vec<String> = fs::read_dir("/proc")
        .unwrap()
        .flatten()
        .filter(|e| {
            fs::read(e.path().join("cmdline"))
                .map(|c| c == b"/bin/sleep\x00300\x00")
                .unwrap_or(false)
        })
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        survivors.is_empty(),
        "daemons survived the call: {survivors:?}"
    );
}

#[test]
fn a_tool_cannot_signal_its_reaper_when_scoped() {
    let tb = Dir::new("tb");
    let mut k = Kicad::new(5, None);
    // Report the outcome of signalling the parent (the reaper) as the finding.
    k.pre = r#"e=$(kill -0 $PPID 2>&1); if [ $? -eq 0 ]; then r=delivered; else r=denied; fi
printf '{"$schema":"https://schemas.kicad.org/drc.v1.json","coordinate_units":"mm","date":"x","ignored_checks":[],"included_severities":["error","warning"],"kicad_version":"10.0.7","source":"b","violations":[{"type":"probe","severity":"error","description":"%s","items":[]}],"unconnected_items":[],"schematic_parity":[]}' "$r" > "$out""#.into();
    k.install(&tb);
    let b = board();
    let report = judged(&drc(&tb, &b.path), "fail", "drc");
    if report["sandbox"]["scoped"] == true {
        let message = report["findings"][0]["message"].as_str().unwrap();
        assert!(message.contains("denied"), "{report}");
    }
}
