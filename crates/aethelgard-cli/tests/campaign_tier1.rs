//! `AET-WO-2300`, tier 1: `aethelgard campaign` and `aethelgard replay` with
//! none of the real tools installed. Renode and the analog bridge are shell
//! stubs, as in `boot_tier1.rs` (whose harness this file copies, the pattern
//! of this crate's test files). The campaign always boots for 100 ms, so the
//! stub Renode's accepted command line has `RunFor "0.100000000"` and a
//! passing transcript counts 10,000,000 instructions.

use std::ffi::OsStr;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use aethelgard_chaos::cbor;
use aethelgard_chaos::record::Record;
use aethelgard_chaos::{Event, Manifest, TraceWriter, synthetic};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

struct Dir(PathBuf);

impl Dir {
    fn new(label: &str) -> Dir {
        static N: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "aet-2300-t1-{}-{}-{label}",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(&path).unwrap();
        Dir(fs::canonicalize(path).unwrap())
    }

    fn write(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.join(name);
        fs::write(&path, bytes).unwrap();
        path
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn spec_bytes() -> Vec<u8> {
    fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap()
}

/// An ELF32 LE `EM_ARM` `ET_EXEC` with one `PT_LOAD` of `code` at `paddr`.
fn elf_at(paddr: u32, code: &[u8], machine: u16) -> Vec<u8> {
    let mut b = vec![0u8; 52];
    b[..7].copy_from_slice(b"\x7fELF\x01\x01\x01");
    b[16..18].copy_from_slice(&2u16.to_le_bytes());
    b[18..20].copy_from_slice(&machine.to_le_bytes());
    b[20..24].copy_from_slice(&1u32.to_le_bytes());
    b[28..32].copy_from_slice(&52u32.to_le_bytes());
    b[40..42].copy_from_slice(&52u16.to_le_bytes());
    b[42..44].copy_from_slice(&32u16.to_le_bytes());
    b[44..46].copy_from_slice(&1u16.to_le_bytes());
    let len = code.len() as u32;
    for v in [1u32, 84, paddr, paddr, len, len, 5, 4] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(code);
    b
}

fn elf() -> Vec<u8> {
    elf_at(0x0800_0000, &[0u8; 64], 40)
}

/// The BME280's line of the platform listing.
const U2_LINE: &str = "  \u{251c}\u{2500}\u{2500} u2 (BME280)\n";

/// The listing of the emitted platform's CPU and every solved peripheral.
const CPU_LISTING: &str = "Available peripherals:\n  (machine-0)\n  \u{251c}\u{2500}\u{2500} cpu (CortexM)\n  \u{2502}       Slot: 0\n  \u{251c}\u{2500}\u{2500} gpioPortA (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} gpioPortB (STM32_GPIOPort)\n  \u{251c}\u{2500}\u{2500} i2c1 (STM32F7_I2C)\n  \u{251c}\u{2500}\u{2500} spi1 (STM32H7_SPI)\n  \u{251c}\u{2500}\u{2500} u2 (BME280)\n";

/// The reference topology's seven registers as a correct boot leaves them.
const GOOD_REGISTERS: [&str; 7] = [
    "0x0000A801",
    "0x55500000",
    "0x0000A000",
    "0x44000000",
    "0x00000003",
    "0x00200000",
    "0x00001000",
];

/// `CR2` (`SADD` 0x76 << 1, `RD_WRN`, `NBYTES` 1, `AUTOEND`) and `TXDR` (0xD0).
const GOOD_CONTROLLER: [&str; 2] = ["0x020104EC", "0x000000D0"];

/// The record a firmware that read U2's ID register (0xD0) as 0x60 leaves.
const GOOD_RECORD: [&str; 2] = ["0x22020000", "0x00000060"];

/// U2's record when the wait for TXIS expired: the bus never answered.
const TXIS_TIMEOUT: [&str; 2] = ["0x22020001", "0x00000060"];

/// 100 ms of a 100 MIPS core, in the transcript's hex.
const INSTRUCTIONS: &str = "0x0000000000989680";

/// A transcript: the echoed command line, the reads between the markers,
/// then `listing`.
fn transcript(reads: &[&str], listing: &str) -> String {
    let mut s = String::from(
        "(monitor) emulation SetSeed 2100;  echo \"AETHELGARD-BOOT-BEGIN\";  echo \"AETHELGARD-BOOT-END\"\n",
    );
    s.push_str("AETHELGARD-BOOT-BEGIN\n");
    for r in reads {
        s.push_str(r);
        s.push('\n');
    }
    s.push_str("AETHELGARD-BOOT-END\n");
    s.push_str(listing);
    s
}

/// The boot's reads, ending in U2's record and I2C1's controller state.
fn boot_reads(record: [&str; 2]) -> Vec<String> {
    let mut v = vec![
        "0x2100B007".to_owned(),
        INSTRUCTIONS.to_owned(),
        "0x00000064".to_owned(),
        // Inside the synthetic ELF's one executable segment.
        "0x8000010".to_owned(),
    ];
    v.extend(GOOD_REGISTERS.iter().map(|r| (*r).to_owned()));
    v.extend(record.iter().map(|r| (*r).to_owned()));
    v.extend(GOOD_CONTROLLER.iter().map(|r| (*r).to_owned()));
    v
}

fn text(record: [&str; 2], listing: &str) -> String {
    let r = boot_reads(record);
    transcript(&r.iter().map(String::as_str).collect::<Vec<_>>(), listing)
}

/// A boot that passes: the BME280 is on the platform and answers.
fn good() -> String {
    text(GOOD_RECORD, CPU_LISTING)
}

/// A boot on a platform whose faulted bus declares no BME280: the read of
/// U2 times out at TXIS.
fn faulted() -> String {
    text(TXIS_TIMEOUT, &CPU_LISTING.replace(U2_LINE, ""))
}

/// A `renode` stub that answers `--version`, refuses (exit 99) a command
/// line without the boot's fixed parts (the seed and a 100 ms RunFor
/// among them), and prints `stdout`.
fn toolbox(stdout: &str) -> Dir {
    let tb = Dir::new("tb");
    let out = tb.write("renode.out", stdout.as_bytes());
    let body = format!(
        r#"#!/bin/sh
if [ "$1" = --version ]; then printf 'Renode v1.17.0\n'; exit 0; fi
case "$5" in
  "emulation SetSeed 2100; mach create; machine LoadPlatformDescription @"*"; sysbus LoadELF @"*"firmware.elf; sysbus WriteDoubleWord 0x580244e0 0x0; sysbus WriteDoubleWord 0x580244e8 0x0; sysbus WriteDoubleWord 0x580244f0 0x0; sysbus WriteDoubleWord 0x20000000 0x0; sysbus WriteDoubleWord 0x20000004 0xffffffff; sysbus WriteDoubleWord 0x20000008 0xffffffff; cpu VectorTableOffset 0x08000000; emulation RunFor \"0.100000000\"; logLevel 3; echo \"AETHELGARD-BOOT-BEGIN\"; sysbus ReadDoubleWord 0x20000000; cpu ExecutedInstructions; cpu PerformanceInMips; cpu PC; sysbus ReadDoubleWord 0x58020000;"*"; echo \"AETHELGARD-BOOT-END\"; peripherals; quit") ;;
  *) exit 99;;
esac
/bin/cat '{out}'
"#,
        out = out.display()
    );
    let stub = tb.write("renode", body.as_bytes());
    fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    tb
}

/// A bridge stub that measures the 2.2 kOhm lines well inside their 300 ns
/// ceiling and the 47 kOhm lines far past it, deciding by the pull-up the
/// request's netlist carries.
const BRIDGE_BY_PULL_UP: &str = r#"#!/bin/sh
if /bin/grep -q 'line 47000' "$1"; then rise=3982.3; else rise=186.4; fi
printf '%s\n' '{"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":'$rise'}' '{"method":"interpolated","net":"I2C_SDA","ngspice":"44.2","rise_30_70_ns":'$rise'}'
"#;

/// A bridge stub that fails (exit 9) if it is asked about SDA, and answers
/// for SCL alone otherwise.
const BRIDGE_SCL_ONLY: &str = r#"#!/bin/sh
if /bin/grep -q I2C_SDA "$1"; then echo 'asked about I2C_SDA' >&2; exit 9; fi
printf '%s\n' '{"method":"interpolated","net":"I2C_SCL","ngspice":"44.2","rise_30_70_ns":186.4}'
"#;

/// The CLI, hard-linked into a directory of its own beside a stub
/// `aethelgard-bridge` with body `bridge`, or with no bridge at all: the
/// CLI runs the bridge from its own directory (`AET-WO-2200` §1.4).
fn cli(bridge: Option<&'static str>) -> PathBuf {
    static MADE: Mutex<Vec<(Option<&'static str>, PathBuf)>> = Mutex::new(Vec::new());
    let mut made = MADE.lock().unwrap();
    if let Some((_, path)) = made.iter().find(|(b, _)| *b == bridge) {
        return path.clone();
    }
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "aet-2300-cli-{}-{}",
        std::process::id(),
        made.len()
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let me = dir.join("aethelgard");
    if fs::hard_link(env!("CARGO_BIN_EXE_aethelgard"), &me).is_err() {
        fs::copy(env!("CARGO_BIN_EXE_aethelgard"), &me).unwrap();
    }
    if let Some(body) = bridge {
        let stub = dir.join("aethelgard-bridge");
        fs::write(&stub, body).unwrap();
        fs::set_permissions(&stub, fs::Permissions::from_mode(0o755)).unwrap();
    }
    made.push((bridge, me.clone()));
    me
}

fn run_cli(cli: &Path, path: &Path, cwd: &Path, args: &[&OsStr]) -> Output {
    Command::new(cli)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .unwrap()
}

fn os(s: &str) -> &OsStr {
    OsStr::new(s)
}

/// `aethelgard campaign --scenarios <scenarios>... --firmware <fw> --archive <archive>`.
fn campaign(
    cli: &Path,
    tools: &Path,
    cwd: &Path,
    scenarios: &[&str],
    firmware: &Path,
    archive: &Path,
) -> Output {
    let mut args = vec![os("campaign"), os("--scenarios")];
    args.extend(scenarios.iter().map(|s| os(s)));
    args.extend([
        os("--firmware"),
        firmware.as_os_str(),
        os("--archive"),
        archive.as_os_str(),
    ]);
    run_cli(cli, tools, cwd, &args)
}

fn replay(tools: &Path, run: &Path) -> Output {
    run_cli(
        &cli(None),
        tools,
        &root(),
        &[os("replay"), os("--archive"), run.as_os_str()],
    )
}

/// Stdout as JSON lines.
fn lines(out: &Output) -> Vec<Value> {
    let stdout = std::str::from_utf8(&out.stdout).unwrap();
    stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or_else(|e| panic!("{e}: {l}")))
        .collect()
}

/// The one line of a refusal.
fn refusal(out: &Output) -> Value {
    let v = lines(out);
    assert_eq!(v.len(), 1, "{v:?} {:?}", out.stderr);
    v.into_iter().next().unwrap()
}

fn expect_refusal(out: &Output, code: i32, kind: &str) -> Value {
    let v = refusal(out);
    assert_eq!(out.status.code(), Some(code), "{v}");
    assert_eq!(v["error"]["kind"], kind, "{v}");
    v
}

/// A scenario file's text.
fn scenario(name: &str, expect: &str, findings: &[&str], overrides: Value) -> String {
    json!({
        "schema_version": 1,
        "name": name,
        "base": "spec.json",
        "expect": expect,
        "expect_findings": findings,
        "overrides": overrides,
    })
    .to_string()
}

/// A working directory holding the reference specification as `spec.json`
/// and the synthetic firmware; the campaign runs with it as its cwd.
struct Work {
    dir: Dir,
    firmware: PathBuf,
    archive: PathBuf,
}

impl Work {
    fn new() -> Work {
        let dir = Dir::new("work");
        dir.write("spec.json", &spec_bytes());
        let firmware = dir.write("fw.elf", &elf());
        let archive = dir.0.join("archive");
        Work {
            dir,
            firmware,
            archive,
        }
    }

    fn cwd(&self) -> &Path {
        &self.dir.0
    }

    fn campaign(&self, cli: &Path, tools: &Path, scenarios: &[&str]) -> Output {
        campaign(
            cli,
            tools,
            self.cwd(),
            scenarios,
            &self.firmware,
            &self.archive,
        )
    }

    /// The run directories in the archive.
    fn runs(&self) -> Vec<PathBuf> {
        match fs::read_dir(&self.archive) {
            Ok(entries) => entries.map(|e| e.unwrap().path()).collect(),
            Err(_) => Vec::new(),
        }
    }
}

const PULL_UP_47K: &str = r#"[{"bus":"SENSORS","pull_up_ohms":47000}]"#;
const PULL_UP_2K2: &str = r#"[{"bus":"SENSORS","pull_up_ohms":2200}]"#;

fn json_of(s: &str) -> Value {
    serde_json::from_str(s).unwrap()
}

fn names(dir: &Path) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .collect();
    v.sort();
    v
}

fn strings(v: &Value, key: &str) -> Vec<String> {
    v[key]
        .as_array()
        .unwrap()
        .iter()
        .map(|k| k.as_str().unwrap().to_owned())
        .collect()
}

// 1. The reference scenarios.

#[test]
fn the_2k2_reference_scenario_passes_archives_and_replays_identically() {
    let tb = toolbox(&good());
    let work = Work::new();
    let archive = Dir::new("archive");
    let out = campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &root(),
        &["scenarios/sentinel-01/pullup-2k2.json"],
        &work.firmware,
        &archive.0.join("a"),
    );
    let all = lines(&out);
    assert_eq!(out.status.code(), Some(0), "{all:?} {:?}", out.stderr);
    assert_eq!(all.len(), 2, "{all:?}");
    let (line, summary) = (&all[0], &all[1]);
    assert_eq!(line["scenario"], "pullup-2k2");
    assert_eq!(line["file"], "scenarios/sentinel-01/pullup-2k2.json");
    assert_eq!(line["verdict"], "pass");
    assert_eq!(line["expect"], "pass");
    assert_eq!(line["matched"], true);
    assert_eq!(line["findings"], json!([]));
    assert_eq!(line["expect_findings"], json!([]));
    assert_eq!(summary["ok"], true);
    assert_eq!(summary["verdict"], "pass");
    assert_eq!(summary["check"], "campaign");
    assert_eq!(summary["scenarios"], 1);
    assert_eq!(summary["matched"], 1);
    assert_eq!(summary["virtual_time_ns"], 100_000_000);
    assert_eq!(summary["trace_bytes"], line["trace_bytes"]);

    let run = PathBuf::from(line["run"].as_str().unwrap());
    assert!(run.is_dir(), "{}", run.display());
    assert_eq!(names(&run), ["manifest.cbor", "trace.cbor"]);
    assert_eq!(
        run.file_name().unwrap().to_str().unwrap(),
        line["run_id"].as_str().unwrap()
    );
    assert_eq!(
        fs::metadata(run.join("trace.cbor")).unwrap().len(),
        line["trace_bytes"].as_u64().unwrap()
    );

    let first = replay(&tb.0, &run);
    let r = lines(&first);
    assert_eq!(first.status.code(), Some(0), "{r:?}");
    assert_eq!(r.len(), 1, "{r:?}");
    assert_eq!(r[0]["ok"], true);
    assert_eq!(r[0]["check"], "replay");
    assert_eq!(r[0]["scenario"], "pullup-2k2");
    assert_eq!(r[0]["verdict"], "pass");
    assert_eq!(r[0]["matched"], true);
    assert_eq!(r[0]["run_id"], line["run_id"]);
    assert_eq!(r[0]["state_sequence_hash"], line["state_sequence_hash"]);
    assert_eq!(r[0]["event_count"], line["event_count"]);
    assert_eq!(r[0]["trace_bytes"], line["trace_bytes"]);
    let artifacts: Vec<&String> = r[0]["artifacts"].as_object().unwrap().keys().collect();
    assert_eq!(
        artifacts,
        ["base", "firmware", "scenario", "specification", "topology"]
    );

    let second = replay(&tb.0, &run);
    assert_eq!(second.status.code(), Some(0));
    assert_eq!(first.stdout, second.stdout);
}

#[test]
fn the_47k_reference_scenario_fails_as_declared_and_replays_to_the_same_hash() {
    let tb = toolbox(&faulted());
    let work = Work::new();
    let archive = Dir::new("archive");
    let out = campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &root(),
        &["scenarios/sentinel-01/pullup-47k.json"],
        &work.firmware,
        &archive.0.join("a"),
    );
    let all = lines(&out);
    assert_eq!(out.status.code(), Some(0), "{all:?} {:?}", out.stderr);
    assert_eq!(all.len(), 2, "{all:?}");
    let line = &all[0];
    assert_eq!(line["scenario"], "pullup-47k");
    assert_eq!(line["verdict"], "fail");
    assert_eq!(line["expect"], "fail");
    assert_eq!(line["matched"], true);
    assert_eq!(
        strings(line, "findings"),
        ["analog_rise_exceeds_ceiling", "i2c_timeout"]
    );
    assert_eq!(all[1]["ok"], true);

    let run = PathBuf::from(line["run"].as_str().unwrap());
    let r = lines(&replay(&tb.0, &run));
    assert_eq!(r[0]["verdict"], "fail");
    assert_eq!(r[0]["matched"], true);
    assert_eq!(r[0]["state_sequence_hash"], line["state_sequence_hash"]);
}

#[test]
fn the_same_campaign_into_a_fresh_archive_gives_the_same_hash() {
    let tb = toolbox(&good());
    let work = Work::new();
    let mut hashes = Vec::new();
    for _ in 0..2 {
        let archive = Dir::new("archive");
        let out = campaign(
            &cli(Some(BRIDGE_BY_PULL_UP)),
            &tb.0,
            &root(),
            &["scenarios/sentinel-01/pullup-2k2.json"],
            &work.firmware,
            &archive.0.join("a"),
        );
        let all = lines(&out);
        assert_eq!(out.status.code(), Some(0), "{all:?}");
        hashes.push((
            all[0]["run_id"].clone(),
            all[0]["state_sequence_hash"].clone(),
            all[0]["trace_bytes"].clone(),
        ));
    }
    assert_eq!(hashes[0], hashes[1]);
}

// 2. `expect` is checked, and for a failure checked exactly (§5.4).

fn spec_run(
    expect: &str,
    findings: &[&str],
    overrides: &str,
    transcript: &str,
) -> (Output, Vec<Value>, Work) {
    let tb = toolbox(transcript);
    let work = Work::new();
    work.dir.write(
        "s.json",
        scenario("case", expect, findings, json_of(overrides)).as_bytes(),
    );
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    let all = lines(&out);
    (out, all, work)
}

#[test]
fn a_failing_run_that_matches_its_expectation_exits_0() {
    let (out, all, _w) = spec_run(
        "fail",
        &["analog_rise_exceeds_ceiling", "i2c_timeout"],
        PULL_UP_47K,
        &faulted(),
    );
    assert_eq!(out.status.code(), Some(0), "{all:?}");
    assert_eq!(all[0]["matched"], true);
    assert_eq!(all[1]["ok"], true);
}

#[test]
fn a_failing_run_under_an_expect_pass_scenario_exits_1_unmatched() {
    let (out, all, w) = spec_run("pass", &[], PULL_UP_47K, &faulted());
    assert_eq!(out.status.code(), Some(1), "{all:?}");
    assert_eq!(all.len(), 2, "{all:?}");
    assert_eq!(all[0]["verdict"], "fail");
    assert_eq!(all[0]["expect"], "pass");
    assert_eq!(all[0]["matched"], false);
    assert_eq!(all[1]["ok"], false);
    assert_eq!(all[1]["verdict"], "fail");
    assert_eq!(all[1]["scenarios"], 1);
    assert_eq!(all[1]["matched"], 0);
    // The run is archived all the same: the evidence of a mismatch is kept.
    assert_eq!(w.runs().len(), 1);
}

#[test]
fn a_failure_for_a_subset_of_the_expected_findings_is_a_campaign_failure() {
    // The run fails on analog and on i2c_timeout; the scenario names only one.
    let (out, all, _w) = spec_run(
        "fail",
        &["analog_rise_exceeds_ceiling"],
        PULL_UP_47K,
        &faulted(),
    );
    assert_eq!(out.status.code(), Some(1), "{all:?}");
    assert_eq!(all[0]["verdict"], "fail");
    assert_eq!(all[0]["matched"], false);
    assert_eq!(all[1]["ok"], false);
}

#[test]
fn a_failure_for_a_superset_or_a_wrong_finding_kind_is_a_campaign_failure() {
    for expected in [
        &[
            "analog_rise_exceeds_ceiling",
            "i2c_timeout",
            "register_mismatch",
        ][..],
        &["register_mismatch"][..],
        &["i2c_timeout"][..],
    ] {
        let (out, all, _w) = spec_run("fail", expected, PULL_UP_47K, &faulted());
        assert_eq!(out.status.code(), Some(1), "{expected:?}: {all:?}");
        assert_eq!(all[0]["matched"], false, "{expected:?}");
        assert_eq!(all[1]["ok"], false, "{expected:?}");
    }
}

#[test]
fn a_passing_run_under_an_expect_fail_scenario_exits_1() {
    let (out, all, _w) = spec_run("fail", &["i2c_timeout"], PULL_UP_2K2, &good());
    assert_eq!(out.status.code(), Some(1), "{all:?}");
    assert_eq!(all[0]["verdict"], "pass");
    assert_eq!(all[0]["expect"], "fail");
    assert_eq!(all[0]["matched"], false);
}

// 3. `line_held_low`.

#[test]
fn a_held_low_sda_line_is_never_sent_to_the_bridge_and_faults_the_bus() {
    let tb = toolbox(&faulted());
    let work = Work::new();
    work.dir.write(
        "s.json",
        scenario(
            "sda-low",
            "fail",
            &["i2c_line_held_low", "i2c_timeout"],
            json!([{ "bus": "SENSORS", "line_held_low": "sda" }]),
        )
        .as_bytes(),
    );
    // Asked about SDA, the bridge stub exits 9 and the campaign would stop.
    let out = work.campaign(&cli(Some(BRIDGE_SCL_ONLY)), &tb.0, &["s.json"]);
    let all = lines(&out);
    assert_eq!(out.status.code(), Some(0), "{all:?} {:?}", out.stderr);
    assert_eq!(all[0]["verdict"], "fail");
    assert_eq!(all[0]["matched"], true);
    assert_eq!(
        strings(&all[0], "findings"),
        ["i2c_line_held_low", "i2c_timeout"]
    );
    let run = PathBuf::from(all[0]["run"].as_str().unwrap());
    let r = lines(&replay(&tb.0, &run));
    assert_eq!(r[0]["ok"], true);
    assert_eq!(r[0]["matched"], true);
    assert_eq!(r[0]["verdict"], "fail");
    assert_eq!(r[0]["state_sequence_hash"], all[0]["state_sequence_hash"]);
}

#[test]
fn holding_both_lines_low_needs_no_bridge_at_all() {
    let tb = toolbox(&faulted());
    let work = Work::new();
    work.dir.write(
        "s.json",
        scenario(
            "both-low",
            "fail",
            &["i2c_line_held_low", "i2c_timeout"],
            json!([
                { "bus": "SENSORS", "line_held_low": "sda" },
                { "bus": "SENSORS", "line_held_low": "scl" },
            ]),
        )
        .as_bytes(),
    );
    let out = work.campaign(&cli(None), &tb.0, &["s.json"]);
    let all = lines(&out);
    assert_eq!(out.status.code(), Some(0), "{all:?} {:?}", out.stderr);
    assert_eq!(all[0]["matched"], true);
    assert_eq!(
        strings(&all[0], "findings"),
        ["i2c_line_held_low", "i2c_timeout"]
    );
    let run = PathBuf::from(all[0]["run"].as_str().unwrap());
    let r = lines(&replay(&tb.0, &run));
    assert_eq!(r[0]["matched"], true);
}

// 4. Refusals before anything runs.

/// A campaign over scenario files written into a fresh working directory,
/// expecting a refusal and no run archived.
fn refused_before_running(
    files: &[(&str, String)],
    scenarios: &[&str],
    code: i32,
    kind: &str,
) -> Value {
    let tb = toolbox(&good());
    let work = Work::new();
    for (name, body) in files {
        work.dir.write(name, body.as_bytes());
    }
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, scenarios);
    let v = expect_refusal(&out, code, kind);
    assert!(work.runs().is_empty(), "{:?}", work.runs());
    v
}

fn scenario_with_base(base: &str) -> String {
    let mut v = json_of(&scenario("case", "pass", &[], json_of(PULL_UP_2K2)));
    v["base"] = json!(base);
    v.to_string()
}

#[test]
fn an_absolute_base_is_refused_before_a_run() {
    refused_before_running(
        &[("s.json", scenario_with_base("/etc/passwd"))],
        &["s.json"],
        3,
        "BaseOutsideRoot",
    );
}

#[test]
fn a_base_escaping_the_current_directory_by_dotdot_is_refused() {
    let tb = toolbox(&good());
    let outer = Dir::new("outer");
    outer.write("spec.json", &spec_bytes());
    let inner = outer.0.join("work");
    fs::create_dir(&inner).unwrap();
    fs::write(inner.join("s.json"), scenario_with_base("../spec.json")).unwrap();
    let fw = outer.write("fw.elf", &elf());
    let archive = inner.join("archive");
    let out = campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &inner,
        &["s.json"],
        &fw,
        &archive,
    );
    expect_refusal(&out, 3, "BaseOutsideRoot");
    assert!(!archive.exists() || names(&archive).is_empty());
}

#[test]
fn a_base_through_a_symlink_pointing_outside_is_refused() {
    let tb = toolbox(&good());
    let work = Work::new();
    let outside = Dir::new("outside");
    let target = outside.write("spec.json", &spec_bytes());
    std::os::unix::fs::symlink(&target, work.cwd().join("link.json")).unwrap();
    work.dir
        .write("s.json", scenario_with_base("link.json").as_bytes());
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    expect_refusal(&out, 3, "BaseOutsideRoot");
    assert!(work.runs().is_empty());
}

#[test]
fn a_generated_platform_file_is_not_a_base() {
    let repl = fs::read_to_string(root().join("boards/sentinel-01/sentinel-01.repl")).unwrap();
    refused_before_running(
        &[("p.repl", repl), ("s.json", scenario_with_base("p.repl"))],
        &["s.json"],
        3,
        "InvalidJson",
    );
}

#[test]
fn a_solved_topology_is_not_a_base() {
    let work = Work::new();
    let tb = toolbox(&good());
    let solved = run_cli(
        &cli(None),
        &tb.0,
        work.cwd(),
        &[os("solve"), os("--file"), os("spec.json")],
    );
    assert_eq!(solved.status.code(), Some(0), "{solved:?}");
    work.dir.write("topology.json", &solved.stdout);
    work.dir
        .write("s.json", scenario_with_base("topology.json").as_bytes());
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    expect_refusal(&out, 2, "SchemaViolation");
    assert!(work.runs().is_empty());
}

#[test]
fn an_unknown_scenario_key_is_refused() {
    let mut v = json_of(&scenario("case", "pass", &[], json_of(PULL_UP_2K2)));
    v["surprise"] = json!(1);
    refused_before_running(
        &[("s.json", v.to_string())],
        &["s.json"],
        2,
        "SchemaViolation",
    );
}

#[test]
fn an_unknown_override_key_is_refused() {
    let s = scenario(
        "case",
        "pass",
        &[],
        json!([{ "bus": "SENSORS", "pull_up_ohms": 2200, "surprise": 1 }]),
    );
    refused_before_running(&[("s.json", s)], &["s.json"], 2, "SchemaViolation");
}

#[test]
fn an_unknown_bus_is_refused() {
    let s = scenario(
        "case",
        "pass",
        &[],
        json!([{ "bus": "NOPE", "pull_up_ohms": 2200 }]),
    );
    refused_before_running(&[("s.json", s)], &["s.json"], 2, "UnknownBus");
}

#[test]
fn a_zero_pull_up_is_refused_as_an_invalid_specification() {
    let s = scenario(
        "case",
        "pass",
        &[],
        json!([{ "bus": "SENSORS", "pull_up_ohms": 0 }]),
    );
    let tb = toolbox(&good());
    let work = Work::new();
    work.dir.write("s.json", s.as_bytes());
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    let v = refusal(&out);
    assert_eq!(out.status.code(), Some(2), "{v}");
    // The specification's own rule names it, not the campaign's.
    let kind = v["error"]["kind"].as_str().unwrap();
    assert_ne!(kind, "SchemaViolation", "{v}");
    assert!(work.runs().is_empty());
    eprintln!("zero pull-up refused as {kind}");
}

#[test]
fn the_same_scenario_file_twice_is_a_duplicate() {
    refused_before_running(
        &[(
            "s.json",
            scenario("case", "pass", &[], json_of(PULL_UP_2K2)),
        )],
        &["s.json", "s.json"],
        2,
        "DuplicateScenario",
    );
}

#[test]
fn a_second_campaign_into_the_same_archive_is_refused_and_the_run_is_untouched() {
    let tb = toolbox(&good());
    let work = Work::new();
    work.dir.write(
        "s.json",
        scenario("case", "pass", &[], json_of(PULL_UP_2K2)).as_bytes(),
    );
    let c = cli(Some(BRIDGE_BY_PULL_UP));
    let first = work.campaign(&c, &tb.0, &["s.json"]);
    assert_eq!(first.status.code(), Some(0), "{:?}", lines(&first));
    let runs = work.runs();
    assert_eq!(runs.len(), 1);
    let read = |run: &Path| -> Vec<Vec<u8>> {
        ["trace.cbor", "manifest.cbor"]
            .iter()
            .map(|f| fs::read(run.join(f)).unwrap())
            .collect()
    };
    let before = read(&runs[0]);

    let second = work.campaign(&c, &tb.0, &["s.json"]);
    expect_refusal(&second, 3, "RunExists");
    assert_eq!(work.runs(), runs);
    assert_eq!(names(&runs[0]), ["manifest.cbor", "trace.cbor"]);
    assert_eq!(before, read(&runs[0]));
}

#[test]
fn a_bad_second_scenario_stops_the_campaign_before_the_first_runs() {
    let bad = scenario(
        "bad",
        "pass",
        &[],
        json!([{ "bus": "NOPE", "pull_up_ohms": 2200 }]),
    );
    let tb = toolbox(&good());
    let work = Work::new();
    work.dir.write(
        "good.json",
        scenario("good", "pass", &[], json_of(PULL_UP_2K2)).as_bytes(),
    );
    work.dir.write("bad.json", bad.as_bytes());
    let out = work.campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &["good.json", "bad.json"],
    );
    // One line only: the refusal, no scenario line for `good`.
    expect_refusal(&out, 2, "UnknownBus");
    assert!(work.runs().is_empty());
    assert!(!work.archive.exists() || names(&work.archive).is_empty());
}

// 5. `replay` refusals.

/// A committed passing run: its tools, working directory and run directory.
fn a_run() -> (Dir, Work, PathBuf) {
    let tb = toolbox(&good());
    let work = Work::new();
    work.dir.write(
        "s.json",
        scenario("case", "pass", &[], json_of(PULL_UP_2K2)).as_bytes(),
    );
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    assert_eq!(out.status.code(), Some(0), "{:?}", lines(&out));
    let run = work.runs().pop().unwrap();
    (tb, work, run)
}

#[test]
fn replaying_a_missing_directory_is_refused() {
    let tb = toolbox(&good());
    let dir = Dir::new("empty");
    let out = replay(&tb.0, &dir.0.join("absent"));
    let v = refusal(&out);
    assert_eq!(out.status.code(), Some(3), "{v}");
    assert!(v["error"]["kind"].is_string(), "{v}");
}

/// The offset of the first run of `needle` in `hay`.
fn find(hay: &[u8], needle: &[u8]) -> usize {
    hay.windows(needle.len())
        .position(|w| w == needle)
        .expect("the needle is in the trace")
}

/// The expected instruction count's integer payload: 10,000,000 is 0x989680.
const INSTRUCTION_COUNT_CBOR: [u8; 5] = [0x1a, 0x00, 0x98, 0x96, 0x80];

#[test]
fn a_one_byte_change_to_the_trace_is_a_hash_mismatch() {
    let (tb, _work, run) = a_run();
    let path = run.join("trace.cbor");
    let mut trace = fs::read(&path).unwrap();
    let at = find(&trace, &INSTRUCTION_COUNT_CBOR) + 4;
    trace[at] ^= 1;
    fs::write(&path, &trace).unwrap();
    let out = replay(&tb.0, &run);
    expect_refusal(&out, 3, "HashMismatch");
}

#[test]
fn a_one_byte_change_with_the_manifest_resealed_diverges_on_replay() {
    let (tb, _work, run) = a_run();
    let path = run.join("trace.cbor");
    let mut trace = fs::read(&path).unwrap();
    let at = find(&trace, &INSTRUCTION_COUNT_CBOR) + 4;
    trace[at] ^= 1;
    fs::write(&path, &trace).unwrap();

    let manifest_path = run.join("manifest.cbor");
    let old = cbor::decode_exact(&fs::read(&manifest_path).unwrap()).unwrap();
    let old = Manifest::from_value(&old).unwrap();
    let sealed = Manifest {
        trace_bytes: trace.len() as u64,
        trace_sha256: Sha256::digest(&trace).into(),
        event_count: old.event_count,
        state_hash: old.state_hash,
    };
    fs::write(&manifest_path, cbor::to_bytes(&sealed.to_value())).unwrap();
    let out = replay(&tb.0, &run);
    expect_refusal(&out, 3, "ReplayDiverged");
}

#[test]
fn an_extra_file_in_the_run_directory_is_refused() {
    let (tb, _work, run) = a_run();
    fs::write(run.join("notes.txt"), b"x").unwrap();
    let out = replay(&tb.0, &run);
    expect_refusal(&out, 3, "UnexpectedEntry");
}

#[test]
fn a_synthetic_run_is_not_a_campaign_run() {
    let tb = toolbox(&good());
    let dir = Dir::new("synthetic");
    let header = synthetic::header(7);
    let id: String = header.run_id.iter().map(|b| format!("{b:02x}")).collect();
    // Named by its run id, so only what it holds is judged.
    let run = dir.0.join(&id);
    let mut w = TraceWriter::create(&run, &header).unwrap();
    let events: Vec<Event> = synthetic::events(7).take(20).collect();
    for e in &events {
        w.append(e).unwrap();
    }
    w.commit().unwrap();
    let out = replay(&tb.0, &run);
    expect_refusal(&out, 3, "NotACampaignRun");
    // The same run under another name is not that run (adversarial F3).
    let renamed = dir.0.join("run");
    fs::rename(&run, &renamed).unwrap();
    expect_refusal(&replay(&tb.0, &renamed), 3, "RunIdMismatch");
}

// 6. Gaps found by the adversarial pass and the mutation run.

/// As `run_cli`, but a child that has not exited in 20 s is killed and the
/// test fails: a FIFO that is opened for reading must never hang the tool.
fn run_guarded(cli: &Path, path: &Path, cwd: &Path, args: &[&OsStr]) -> Output {
    let mut child = Command::new(cli)
        .args(args)
        .current_dir(cwd)
        .env_clear()
        .env("PATH", path)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let start = std::time::Instant::now();
    loop {
        if child.try_wait().unwrap().is_some() {
            return child.wait_with_output().unwrap();
        }
        if start.elapsed() > std::time::Duration::from_secs(20) {
            let _ = child.kill();
            let _ = child.wait();
            panic!("the tool did not return in 20 s: a FIFO hung it");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

fn mkfifo(path: &Path) {
    let status = Command::new("mkfifo").arg(path).status().unwrap();
    assert!(status.success(), "mkfifo {}", path.display());
}

fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[test]
fn an_absolute_base_inside_the_current_directory_is_still_refused() {
    let tb = toolbox(&good());
    let work = Work::new();
    let inside = work.cwd().join("spec.json");
    assert!(inside.is_file());
    work.dir.write(
        "s.json",
        scenario_with_base(inside.to_str().unwrap()).as_bytes(),
    );
    let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &["s.json"]);
    expect_refusal(&out, 3, "BaseOutsideRoot");
    assert!(work.runs().is_empty());
}

#[test]
fn a_base_that_is_a_directory_is_refused() {
    let tb = toolbox(&good());
    let work = Work::new();
    fs::create_dir(work.cwd().join("subdir")).unwrap();
    for (i, base) in ["subdir", "."].into_iter().enumerate() {
        let name = format!("s{i}.json");
        work.dir.write(&name, scenario_with_base(base).as_bytes());
        let out = work.campaign(&cli(Some(BRIDGE_BY_PULL_UP)), &tb.0, &[&name]);
        expect_refusal(&out, 3, "BaseOutsideRoot");
    }
    assert!(work.runs().is_empty());
}

#[test]
fn a_base_that_is_a_fifo_is_refused_without_hanging() {
    let tb = toolbox(&good());
    let work = Work::new();
    mkfifo(&work.cwd().join("base.fifo"));
    work.dir
        .write("s.json", scenario_with_base("base.fifo").as_bytes());
    let c = cli(Some(BRIDGE_BY_PULL_UP));
    let mut args = vec![os("campaign"), os("--scenarios"), os("s.json")];
    args.extend([
        os("--firmware"),
        work.firmware.as_os_str(),
        os("--archive"),
        work.archive.as_os_str(),
    ]);
    let out = run_guarded(&c, &tb.0, work.cwd(), &args);
    expect_refusal(&out, 3, "BaseOutsideRoot");
    assert!(work.runs().is_empty());
}

#[test]
fn a_campaign_with_one_matching_and_one_unmatched_scenario_exits_1() {
    let tb = toolbox(&good());
    let work = Work::new();
    work.dir.write(
        "ok.json",
        scenario("ok", "pass", &[], json_of(PULL_UP_2K2)).as_bytes(),
    );
    // Passes at 2k2, so its expectation of a failure is not met.
    work.dir.write(
        "bad.json",
        scenario("bad", "fail", &["i2c_timeout"], json_of(PULL_UP_2K2)).as_bytes(),
    );
    let out = work.campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &["ok.json", "bad.json"],
    );
    let all = lines(&out);
    assert_eq!(out.status.code(), Some(1), "{all:?} {:?}", out.stderr);
    assert_eq!(all.len(), 3, "{all:?}");
    assert_eq!(all[0]["scenario"], "ok");
    assert_eq!(all[0]["matched"], true);
    assert_eq!(all[1]["scenario"], "bad");
    assert_eq!(all[1]["matched"], false);
    assert_eq!(all[2]["check"], "campaign");
    assert_eq!(all[2]["scenarios"], 2);
    assert_eq!(all[2]["matched"], 1);
    assert_eq!(all[2]["ok"], false);
    assert_eq!(work.runs().len(), 2);
}

#[test]
fn a_run_directory_that_is_a_symlink_to_a_real_run_is_refused() {
    let (tb, work, run) = a_run();
    let link = work.cwd().join("linked-run");
    std::os::unix::fs::symlink(&run, &link).unwrap();
    // The real run replays; the link to it does not.
    assert_eq!(replay(&tb.0, &run).status.code(), Some(0));
    expect_refusal(&replay(&tb.0, &link), 3, "UnexpectedEntry");
}

#[test]
fn a_trace_that_is_a_symlink_to_an_identical_copy_is_refused() {
    let (tb, _work, run) = a_run();
    let outside = Dir::new("outside");
    let copy = outside.write("trace.cbor", &fs::read(run.join("trace.cbor")).unwrap());
    fs::remove_file(run.join("trace.cbor")).unwrap();
    std::os::unix::fs::symlink(&copy, run.join("trace.cbor")).unwrap();
    expect_refusal(&replay(&tb.0, &run), 3, "UnexpectedEntry");
}

#[test]
fn a_manifest_that_is_a_fifo_is_refused_without_hanging() {
    let (tb, _work, run) = a_run();
    fs::remove_file(run.join("manifest.cbor")).unwrap();
    mkfifo(&run.join("manifest.cbor"));
    let out = run_guarded(
        &cli(None),
        &tb.0,
        &root(),
        &[os("replay"), os("--archive"), run.as_os_str()],
    );
    expect_refusal(&out, 3, "UnexpectedEntry");
}

/// Rewrite the run `run` into a fresh directory named by the original run id,
/// its header and its records edited first, with every record and so the state
/// hash recomputed.
fn reseal(
    run: &Path,
    edit_header: impl FnOnce(&mut aethelgard_chaos::Header),
    edit_records: impl FnOnce(&mut Vec<Record>),
) -> (Dir, PathBuf) {
    let old = aethelgard_chaos::replay_dir(run).unwrap();
    let mut header = old.header.clone();
    edit_header(&mut header);
    let mut records = old.state.records;
    edit_records(&mut records);
    let dir = Dir::new("resealed");
    let new = dir.0.join(hex_of(&old.header.run_id));
    let mut w = TraceWriter::create(&new, &header).unwrap();
    for record in records {
        let timestamp = if record.before_boot() { 0 } else { 100_000_000 };
        w.append(&Event::Record { timestamp, record }).unwrap();
    }
    w.commit().unwrap();
    (dir, new)
}

#[test]
fn a_run_resealed_with_the_same_header_replays_so_the_next_test_is_not_vacuous() {
    let (tb, _work, run) = a_run();
    let (_dir, new) = reseal(&run, |_| {}, |_| {});
    let original = lines(&replay(&tb.0, &run));
    let again = replay(&tb.0, &new);
    let again = lines(&again);
    assert_eq!(again[0]["ok"], true, "{again:?}");
    assert_eq!(
        again[0]["state_sequence_hash"],
        original[0]["state_sequence_hash"]
    );
}

#[test]
fn a_run_with_a_consistently_resealed_but_changed_header_is_not_a_campaign_run() {
    let (tb, _work, run) = a_run();
    let (_dir, new) = reseal(
        &run,
        |h| {
            let firmware = h.artifacts.iter_mut().find(|(n, _)| n == "firmware");
            firmware.unwrap().1[0] ^= 1;
        },
        |_| {},
    );
    // Trace hash, byte count and state hash all agree; only the run id does not.
    expect_refusal(&replay(&tb.0, &new), 3, "NotACampaignRun");
}

// 7. `attribute` (`AET-WO-3100` §1.3).

fn attribute(tools: &Path, run: &Path) -> Output {
    run_cli(
        &cli(None),
        tools,
        &root(),
        &[os("attribute"), os("--archive"), run.as_os_str()],
    )
}

#[test]
fn attribute_names_a_passing_run_none_twice_in_identical_bytes() {
    let (tb, _work, run) = a_run();
    let first = attribute(&tb.0, &run);
    let a = refusal(&first);
    assert_eq!(first.status.code(), Some(0), "{a}");
    assert_eq!(
        (&a["ok"], &a["check"], &a["domain"], &a["rule"]),
        (
            &json!(true),
            &json!("attribute"),
            &json!("none"),
            &json!("N1")
        )
    );
    for key in ["evidence", "symptoms", "masked", "unexplained"] {
        assert_eq!(a[key], json!([]), "{key}");
    }
    let r = refusal(&replay(&tb.0, &run));
    assert_eq!(a["run_id"], r["run_id"]);
    assert_eq!(a["state_sequence_hash"], r["state_sequence_hash"]);
    assert_eq!(attribute(&tb.0, &run).stdout, first.stdout);
}

#[test]
fn attribute_names_the_47k_run_electrical_with_its_rises_and_fault() {
    let tb = toolbox(&faulted());
    let work = Work::new();
    let archive = Dir::new("archive");
    let out = campaign(
        &cli(Some(BRIDGE_BY_PULL_UP)),
        &tb.0,
        &root(),
        &["scenarios/sentinel-01/pullup-47k.json"],
        &work.firmware,
        &archive.0.join("a"),
    );
    assert_eq!(out.status.code(), Some(0), "{:?}", lines(&out));
    let run = PathBuf::from(lines(&out)[0]["run"].as_str().unwrap());
    let got = attribute(&tb.0, &run);
    let a = refusal(&got);
    assert_eq!(got.status.code(), Some(0), "{a}");
    assert_eq!(
        (&a["domain"], &a["rule"]),
        (&json!("electrical"), &json!("E1"))
    );
    let kinds: Vec<u64> = a["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_u64().unwrap())
        .collect();
    assert_eq!(kinds, [7, 7, 9]);
    // Every record decided before the boot is at virtual time 0, the rest at
    // the boot's end.
    let trace = fs::read(run.join("trace.cbor")).unwrap();
    let (_, mut at) = cbor::decode_prefix(&trace, 0).unwrap();
    let mut times = Vec::new();
    while at < trace.len() {
        let (item, next) = cbor::decode_prefix(&trace, at).unwrap();
        times.push(Event::from_value(&item).unwrap().timestamp());
        at = next;
    }
    let records = aethelgard_chaos::replay_dir(&run).unwrap().state.records;
    for (t, r) in times.iter().zip(&records) {
        let want = if r.before_boot() { 0 } else { 100_000_000 };
        assert_eq!(*t, want, "{r:?}");
    }
    // The campaign wrote kind 15 from the solved topology, right after kind 6.
    let Record::I2cBusSolved {
        bus,
        peripheral,
        lines,
        devices,
    } = &records[2]
    else {
        panic!("{:?}", records[2]);
    };
    assert_eq!((bus.as_str(), peripheral.as_str()), ("SENSORS", "I2C1"));
    let lines: Vec<(&str, &str, &str, u8, u64)> = lines
        .iter()
        .map(|l| {
            (
                l.signal.as_str(),
                l.net.as_str(),
                l.pin.as_str(),
                l.alternate_function,
                l.ceiling_ps,
            )
        })
        .collect();
    assert_eq!(
        lines,
        [
            ("I2C1_SCL", "I2C_SCL", "PB6", 4, 300_000),
            ("I2C1_SDA", "I2C_SDA", "PB7", 4, 300_000)
        ]
    );
    assert_eq!(devices, &[("U2".to_owned(), 0x76), ("U3".to_owned(), 0x48)]);
}

#[test]
fn an_indeterminate_run_exits_1_with_its_unexplained_finding_named() {
    let (tb, _work, run) = a_run();
    let (_dir, forged) = reseal(
        &run,
        |_| {},
        |r| {
            let last = r.len() - 1;
            r[last] = Record::Verdict {
                pass: false,
                findings: vec![("renode_error_log".into(), "error".into(), 1)],
            };
        },
    );
    let out = attribute(&tb.0, &forged);
    let a = refusal(&out);
    assert_eq!(out.status.code(), Some(1), "{a}");
    assert_eq!(
        (&a["ok"], &a["domain"], &a["rule"], &a["unexplained"]),
        (
            &json!(false),
            &json!("indeterminate"),
            &json!("I1"),
            &json!(["renode_error_log"])
        )
    );
}

#[test]
fn attribute_refuses_whatever_replay_refuses_with_the_same_bytes_and_code() {
    let (tb, _work, run) = a_run();
    let same = |dir: &Path, kind: &str| {
        let (a, r) = (attribute(&tb.0, dir), replay(&tb.0, dir));
        assert_eq!(a.stdout, r.stdout, "{kind}");
        assert_eq!(a.status.code(), r.status.code(), "{kind}");
        expect_refusal(&a, 3, kind);
    };
    let scratch = Dir::new("refusals");
    same(&scratch.0.join("absent"), "Io");
    // A one-byte change, and a run copied under another name.
    let copy = |label: &str| {
        let to = scratch.0.join(label).join(run.file_name().unwrap());
        fs::create_dir_all(&to).unwrap();
        for name in ["trace.cbor", "manifest.cbor"] {
            fs::copy(run.join(name), to.join(name)).unwrap();
        }
        to
    };
    let flipped = copy("flip");
    let mut trace = fs::read(flipped.join("trace.cbor")).unwrap();
    let at = find(&trace, &INSTRUCTION_COUNT_CBOR) + 4;
    trace[at] ^= 1;
    fs::write(flipped.join("trace.cbor"), &trace).unwrap();
    same(&flipped, "HashMismatch");
    let renamed = scratch.0.join("renamed");
    fs::rename(copy("rename"), &renamed).unwrap();
    same(&renamed, "RunIdMismatch");
    let extra = copy("extra");
    fs::write(extra.join("notes.txt"), b"x").unwrap();
    same(&extra, "UnexpectedEntry");
    // A forged join: kind 15 dropped and the run re-sealed.
    let (_d, no_join) = reseal(
        &run,
        |_| {},
        |r| r.retain(|x| !matches!(x, Record::I2cBusSolved { .. })),
    );
    same(&no_join, "NotACampaignRun");
}

#[test]
fn attribute_needs_nothing_but_the_run_directory() {
    let (tb, _work, run) = a_run();
    let before = attribute(&tb.0, &run);
    // Another working directory, an empty PATH, and the run moved elsewhere.
    let elsewhere = Dir::new("elsewhere");
    let moved = elsewhere.0.join(run.file_name().unwrap());
    fs::create_dir_all(&moved).unwrap();
    for name in ["trace.cbor", "manifest.cbor"] {
        fs::copy(run.join(name), moved.join(name)).unwrap();
    }
    let empty = Dir::new("empty-path");
    let after = run_cli(
        &cli(None),
        &empty.0,
        &empty.0,
        &[os("attribute"), os("--archive"), moved.as_os_str()],
    );
    assert_eq!(after.status.code(), Some(0));
    assert_eq!(after.stdout, before.stdout);
    let _ = tb;
}

/// `AET-WO-1203` §3, the review's A2 inputs: a pull-up below the sink floor is
/// a `solve` refusal, so a scenario's override of it never reaches a run.
#[test]
fn a_pull_up_below_the_sink_floor_in_an_override_is_refused_before_any_run() {
    for ohms in [24, 50, 100] {
        let body = scenario(
            "low",
            "pass",
            &[],
            json!([{"bus": "SENSORS", "pull_up_ohms": ohms}]),
        );
        let v = refused_before_running(
            &[("low.json", body)],
            &["low.json"],
            2,
            "PullUpBelowSinkFloor",
        );
        assert_eq!(v["error"]["id"], "SENSORS", "{v}");
    }
}
