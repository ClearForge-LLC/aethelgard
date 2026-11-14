//! `AET-WO-2300` §3, tier 2: the P2 gate's clauses 2 to 4, inside the tools
//! image, with the real bridge, the real Renode and the reference firmware.
//!
//! 1. The campaign over `pullup-2k2.json` (`expect: pass`) and
//!    `pullup-47k.json` (`expect: fail`) exits 0. The 47 kΩ run's archived
//!    events show `rise_30_70_ps` = 3,982,300 ± 500 against a 300,000 ps
//!    ceiling, the bus fault, and U2's TXIS timeout, as three separate events.
//! 2. `replay` on the 47 kΩ run, twice: identical output, and the hash equals
//!    the committed one. A one-byte change is refused, and so is one whose
//!    manifest is re-sealed around it, by the recomputed state-sequence hash.
//! 3. The same campaign run twice gives identical hashes per scenario.
//!
//! Each run writes `ANSWER 2300` lines to `/dev/stderr`: the trace bytes
//! `OQ-03` asks for.

#![cfg(feature = "with-tools")]

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use aethelgard_chaos::record::Record;
use aethelgard_chaos::{Manifest, cbor, replay_dir};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn answer(line: &str) {
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(format!("ANSWER 2300: {line}\n").as_bytes())
        .unwrap();
}

/// The reference firmware, built as `twin_tier2.rs` builds it: offline,
/// locked, into the top-level `target/`.
fn firmware() -> &'static Path {
    static ELF: OnceLock<PathBuf> = OnceLock::new();
    ELF.get_or_init(|| {
        let out = Command::new(env!("CARGO"))
            .args(["build", "--release", "--offline", "--locked"])
            .current_dir(root().join("boards/sentinel-01/firmware"))
            .env_remove("CARGO_TARGET_DIR")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        root().join(
            "target/sentinel-01-board-firmware/thumbv7em-none-eabihf/release/sentinel-01-board-firmware",
        )
    })
}

fn aethelgard(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.is_empty(),
        "no stdout; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), stdout)
}

fn lines(stdout: &str) -> Vec<Value> {
    stdout
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

struct Campaign {
    code: Option<i32>,
    lines: Vec<Value>,
}

impl Campaign {
    fn scenario(&self, name: &str) -> &Value {
        self.lines
            .iter()
            .find(|l| l["scenario"] == name)
            .unwrap_or_else(|| panic!("no line for {name}: {:?}", self.lines))
    }
}

/// The reference campaign, into a fresh archive named `label`.
fn campaign(label: &str) -> Campaign {
    let archive = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-2300-t2-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&archive);
    let (code, stdout) = aethelgard(&[
        "campaign",
        "--scenarios",
        "scenarios/sentinel-01/pullup-2k2.json",
        "scenarios/sentinel-01/pullup-47k.json",
        "--firmware",
        firmware().to_str().unwrap(),
        "--archive",
        archive.to_str().unwrap(),
    ]);
    let lines = lines(&stdout);
    for l in &lines[..lines.len() - 1] {
        answer(&format!(
            "campaign {label} scenario={} trace_bytes={} event_count={} hash={}",
            l["scenario"], l["trace_bytes"], l["event_count"], l["state_sequence_hash"]
        ));
    }
    answer(&format!(
        "campaign {label} total trace_bytes={} virtual_time_ns={}",
        lines.last().unwrap()["trace_bytes"],
        lines.last().unwrap()["virtual_time_ns"]
    ));
    Campaign { code, lines }
}

fn first() -> &'static Campaign {
    static FIRST: OnceLock<Campaign> = OnceLock::new();
    FIRST.get_or_init(|| campaign("a"))
}

fn run_47k() -> PathBuf {
    PathBuf::from(first().scenario("pullup-47k")["run"].as_str().unwrap())
}

#[test]
fn p2_gate_clause_2_the_campaign_passes_2k2_and_fails_47k_as_each_expects() {
    let c = first();
    assert_eq!(c.code, Some(0), "{:?}", c.lines);
    assert_eq!(c.lines.len(), 3, "{:?}", c.lines);
    let pass = c.scenario("pullup-2k2");
    assert_eq!(
        (&pass["verdict"], &pass["expect"], &pass["matched"]),
        (&"pass".into(), &"pass".into(), &true.into())
    );
    let fail = c.scenario("pullup-47k");
    assert_eq!(
        (&fail["verdict"], &fail["expect"], &fail["matched"]),
        (&"fail".into(), &"fail".into(), &true.into())
    );
    assert_eq!(
        fail["findings"],
        serde_json::json!(["analog_rise_exceeds_ceiling", "i2c_timeout"])
    );
    let summary = c.lines.last().unwrap();
    assert_eq!(summary["ok"], true);
    assert_eq!(summary["matched"], 2);
    assert_eq!(summary["virtual_time_ns"], 100_000_000);
}

#[test]
fn p2_gate_clause_2_the_47k_trace_shows_the_rise_the_fault_and_the_timeout_as_three_events() {
    let replayed = replay_dir(&run_47k()).unwrap();
    let records = &replayed.state.records;
    let rises: Vec<(usize, &Record)> = records
        .iter()
        .enumerate()
        .filter(|(_, r)| matches!(r, Record::RiseMeasured { .. }))
        .collect();
    assert_eq!(rises.len(), 2, "{records:?}");
    for (_, r) in &rises {
        let Record::RiseMeasured {
            bus,
            rise_30_70_ps,
            complete,
            ceiling_ps,
            exceeds,
            ..
        } = r
        else {
            unreachable!()
        };
        assert_eq!(bus, "SENSORS");
        assert!(
            rise_30_70_ps.abs_diff(3_982_300) <= 500,
            "rise {rise_30_70_ps} ps"
        );
        assert_eq!((*complete, *ceiling_ps, *exceeds), (true, 300_000, true));
        answer(&format!("47k rise_30_70_ps={rise_30_70_ps}"));
    }
    let (fault_at, fault) = records
        .iter()
        .enumerate()
        .find(|(_, r)| matches!(r, Record::BusFault { .. }))
        .expect("a bus fault");
    let Record::BusFault {
        bus, nets, from, ..
    } = fault
    else {
        unreachable!()
    };
    assert_eq!(bus, "SENSORS");
    assert_eq!(nets, &["I2C_SCL", "I2C_SDA"]);
    assert_eq!(from, &["analog.rise_time_measured > ceiling_ns"]);
    let (timeout_at, _) = records
        .iter()
        .enumerate()
        .find(|(_, r)| {
            matches!(r, Record::I2cCheck { reference, status: Some(s), .. }
                if reference == "U2" && s == "timeout_txis")
        })
        .expect("U2's TXIS timeout");
    let indices = [rises[0].0, rises[1].0, fault_at, timeout_at];
    assert!(
        indices.windows(2).all(|w| w[0] < w[1]),
        "three separate events, in that order: {indices:?}"
    );
    let Some(Record::Verdict { pass, findings }) = records.last() else {
        panic!("the verdict is last");
    };
    assert!(!pass);
    let failing: Vec<(&str, u64)> = findings
        .iter()
        .filter(|(_, s, _)| s != "info")
        .map(|(k, _, n)| (k.as_str(), *n))
        .collect();
    assert_eq!(
        failing,
        [("analog_rise_exceeds_ceiling", 2), ("i2c_timeout", 1)]
    );
}

#[test]
fn p2_gate_clause_3_replay_twice_is_identical_and_equals_the_committed_hash() {
    let run = run_47k();
    let (c1, out1) = aethelgard(&["replay", "--archive", run.to_str().unwrap()]);
    let (c2, out2) = aethelgard(&["replay", "--archive", run.to_str().unwrap()]);
    assert_eq!((c1, c2), (Some(0), Some(0)), "{out1}");
    assert_eq!(out1, out2, "two replays differ");
    let v = &lines(&out1)[0];
    let manifest = replay_dir(&run).unwrap().manifest;
    let committed: String = manifest
        .state_hash
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(v["state_sequence_hash"], committed.as_str());
    assert_eq!(
        v["state_sequence_hash"],
        first().scenario("pullup-47k")["state_sequence_hash"]
    );
    assert_eq!(
        (&v["verdict"], &v["matched"]),
        (&"fail".into(), &true.into())
    );
    answer(&format!("replay 47k twice identical hash={committed}"));
}

/// A copy of the 47 kΩ run with `edit` applied to its trace.
fn tampered(label: &str, edit: impl Fn(&mut Vec<u8>)) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-2300-t2-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    for name in ["trace.cbor", "manifest.cbor"] {
        fs::copy(run_47k().join(name), dir.join(name)).unwrap();
    }
    let mut trace = fs::read(dir.join("trace.cbor")).unwrap();
    edit(&mut trace);
    fs::write(dir.join("trace.cbor"), &trace).unwrap();
    dir
}

fn refused(dir: &Path) -> (Option<i32>, Value) {
    let (code, out) = aethelgard(&["replay", "--archive", dir.to_str().unwrap()]);
    (code, lines(&out).remove(0))
}

#[test]
fn p2_gate_clause_3_a_one_byte_change_is_refused_even_with_the_manifest_resealed() {
    let flipped = tampered("flip", |t| {
        let at = t.len() / 2;
        t[at] ^= 1;
    });
    let (code, v) = refused(&flipped);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "HashMismatch");

    // 3,982,30x ps is encoded as 0x1a and four bytes. Change its last bit, then
    // seal the manifest around the new bytes, keeping the committed hash: only
    // the recomputed state-sequence hash can now catch it.
    let resealed = tampered("reseal", |t| {
        let at = (0..t.len() - 4)
            .find(|&i| t[i] == 0x1a && t[i + 1..i + 4] == [0x00, 0x3c, 0xc3])
            .expect("a 3,982,xxx ps rise in the trace");
        t[at + 4] ^= 1;
    });
    let trace = fs::read(resealed.join("trace.cbor")).unwrap();
    let old = replay_dir(&run_47k()).unwrap().manifest;
    let manifest = Manifest {
        trace_bytes: trace.len() as u64,
        trace_sha256: Sha256::digest(&trace).into(),
        ..old
    };
    fs::write(
        resealed.join("manifest.cbor"),
        cbor::to_bytes(&manifest.to_value()),
    )
    .unwrap();
    let (code, v) = refused(&resealed);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "ReplayDiverged");
}

#[test]
fn p2_gate_clause_3_the_same_campaign_twice_gives_identical_hashes_per_scenario() {
    let second = campaign("b");
    assert_eq!(second.code, Some(0));
    for name in ["pullup-2k2", "pullup-47k"] {
        let (a, b) = (first().scenario(name), second.scenario(name));
        assert_eq!(a["state_sequence_hash"], b["state_sequence_hash"], "{name}");
        assert_eq!(a["run_id"], b["run_id"], "{name}");
        assert_eq!(a["trace_bytes"], b["trace_bytes"], "{name}");
        let read = |l: &Value| fs::read(Path::new(l["run"].as_str().unwrap()).join("trace.cbor"));
        assert_eq!(read(a).unwrap(), read(b).unwrap(), "{name}: traces differ");
    }
}

// ---- AET-WO-2301: the declared supply droop, Renode's reset recipe, a flooding firmware.
//
// Each of these writes `ANSWER 2301` lines to `/dev/stderr`.

use std::time::Instant;

fn answer_2301(line: &str) {
    let mut log = OpenOptions::new().append(true).open("/dev/stderr").unwrap();
    log.write_all(format!("ANSWER 2301: {line}\n").as_bytes())
        .unwrap();
}

/// Builds the crate in `dir` (its `.cargo/config.toml` names a target directory
/// under the repository's git-ignored `target/`), offline and locked, and
/// returns the ELF named `bin`, found under `target_dir`.
fn build(dir: &Path, target_dir: &str, bin: &str) -> PathBuf {
    let start = Instant::now();
    let out = Command::new(env!("CARGO"))
        .args(["build", "--release", "--offline", "--locked"])
        .current_dir(dir)
        .env_remove("CARGO_TARGET_DIR")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let elf = root()
        .join("target")
        .join(target_dir)
        .join("thumbv7em-none-eabihf/release")
        .join(bin);
    answer_2301(&format!(
        "{bin} build wall_s={:.2} bytes={}",
        start.elapsed().as_secs_f64(),
        fs::metadata(&elf).unwrap().len()
    ));
    elf
}

/// The flooding firmware (`tests/fixtures/flood`).
fn flood_firmware() -> &'static Path {
    static ELF: OnceLock<PathBuf> = OnceLock::new();
    ELF.get_or_init(|| {
        build(
            &root().join("crates/aethelgard-cli/tests/fixtures/flood"),
            "aet-flood-fixture",
            "aet-flood-fixture",
        )
    })
}

fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-2301-t2-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

const DROOPS: [&str; 2] = ["droop-tolerated", "droop-bor3"];

/// The two reference droops, into a fresh archive named `label`: the lines
/// printed, and each scenario's run directory.
fn droop_campaign(label: &str) -> (Option<i32>, Vec<Value>) {
    let archive = scratch(label);
    let scenarios: Vec<String> = DROOPS
        .iter()
        .map(|n| format!("scenarios/sentinel-01/{n}.json"))
        .collect();
    let mut args = vec!["campaign", "--scenarios"];
    args.extend(scenarios.iter().map(String::as_str));
    args.extend(["--firmware", firmware().to_str().unwrap()]);
    args.extend(["--archive", archive.to_str().unwrap()]);
    let start = Instant::now();
    let (code, stdout) = aethelgard(&args);
    answer_2301(&format!(
        "campaign {label} wall_s={:.2}",
        start.elapsed().as_secs_f64()
    ));
    (code, lines(&stdout))
}

fn line<'a>(lines: &'a [Value], name: &str) -> &'a Value {
    lines
        .iter()
        .find(|l| l["scenario"] == name)
        .unwrap_or_else(|| panic!("no line for {name}: {lines:?}"))
}

fn run_dir(line: &Value) -> PathBuf {
    PathBuf::from(line["run"].as_str().unwrap())
}

fn records(run: &Path) -> Vec<Record> {
    replay_dir(run).unwrap().state.records
}

/// `(domain, level, at_ps, falling)` of each kind 17.
fn crossings(records: &[Record]) -> Vec<(String, String, u64, bool)> {
    records
        .iter()
        .filter_map(|r| match r {
            Record::RailCrossing {
                domain,
                level,
                at_ps,
                falling,
                ..
            } => Some((domain.clone(), level.clone(), *at_ps, *falling)),
            _ => None,
        })
        .collect()
}

#[test]
fn the_two_reference_droops_behave_as_predicted_twice_byte_identical_and_replay() {
    let (code_a, a) = droop_campaign("a");
    let (code_b, b) = droop_campaign("b");
    assert_eq!((code_a, code_b), (Some(0), Some(0)), "{a:?} {b:?}");
    for name in DROOPS {
        for l in [line(&a, name), line(&b, name)] {
            assert_eq!(
                (&l["verdict"], &l["expect"], &l["matched"]),
                (&"pass".into(), &"pass".into(), &true.into()),
                "{name}: {l}"
            );
        }
        let read = |l: &Value| fs::read(run_dir(l).join("trace.cbor")).unwrap();
        let (ta, tb) = (read(line(&a, name)), read(line(&b, name)));
        answer_2301(&format!("{name} trace_bytes={}", ta.len()));
        assert_eq!(ta, tb, "{name}: two campaigns' traces differ");
        for l in [line(&a, name), line(&b, name)] {
            let run = run_dir(l);
            let (code, out) = aethelgard(&["replay", "--archive", run.to_str().unwrap()]);
            let v = &lines(&out)[0];
            assert_eq!(code, Some(0), "{name}: {out}");
            assert_eq!(v["matched"], true, "{name}: {out}");
        }
    }

    // droop-tolerated: the rail stays inside its tolerance. The rail is
    // declared (kind 16) but nothing crosses, resets or is observed after.
    let tolerated = records(&run_dir(line(&a, "droop-tolerated")));
    assert!(
        tolerated
            .iter()
            .any(|r| matches!(r, Record::RailDeclared { .. })),
        "kind 16 is archived: {tolerated:?}"
    );
    assert!(
        !tolerated.iter().any(|r| matches!(
            r,
            Record::RailCrossing { .. }
                | Record::ResetRendered { .. }
                | Record::FirmwareObserved { .. }
        )),
        "{tolerated:?}"
    );
    let Some(Record::BootObserved {
        instructions_expected,
        instructions_observed,
        ..
    }) = tolerated
        .iter()
        .find(|r| matches!(r, Record::BootObserved { .. }))
    else {
        panic!("no boot observed: {tolerated:?}");
    };
    assert_eq!(*instructions_expected, 10_000_000);
    assert_eq!(*instructions_observed, Some(10_000_000));
    let Some(Record::Verdict { pass, findings }) = tolerated.last() else {
        panic!("the verdict is last");
    };
    assert!(*pass);
    let droop_findings = |f: &[(String, String, u64)]| {
        f.iter()
            .filter(|(k, _, _)| k != "device_unmodelled")
            .cloned()
            .collect::<Vec<_>>()
    };
    assert!(droop_findings(findings).is_empty(), "{findings:?}");

    // droop-bor3: the supply falls through VBOR3, the reset is asserted and
    // released, and the firmware boots again.
    let bor3 = records(&run_dir(line(&a, "droop-bor3")));
    let found = crossings(&bor3);
    answer_2301(&format!("bor3 crossings {found:?}"));
    let shape: Vec<(&str, u64, bool)> = found
        .iter()
        .map(|(_, level, at, falling)| (level.as_str(), *at, *falling))
        .collect();
    assert_eq!(
        shape,
        [
            ("VBOR3_rising", 1_012_422_222, true),
            ("VBOR3_falling", 1_014_422_222, true),
            ("VBOR3_falling", 1_225_977_778, false),
            ("VBOR3_rising", 1_227_977_778, false),
        ]
    );
    let resets: Vec<(u64, bool, Vec<String>)> = bor3
        .iter()
        .filter_map(|r| match r {
            Record::ResetRendered {
                at_ns,
                released,
                cause,
                ..
            } => Some((*at_ns, *released, cause.clone())),
            _ => None,
        })
        .collect();
    assert_eq!(
        resets,
        [
            (1_014_430, false, vec!["VBOR3".to_owned()]),
            (1_227_980, true, vec!["VBOR3".to_owned()]),
        ]
    );
    let observed: Vec<&Record> = bor3
        .iter()
        .filter(|r| matches!(r, Record::FirmwareObserved { .. }))
        .collect();
    assert_eq!(observed.len(), 1, "{observed:?}");
    let Record::FirmwareObserved {
        at_ns,
        boot_word,
        status,
        value,
        witness,
    } = observed[0]
    else {
        unreachable!()
    };
    assert_eq!(*at_ns, 1_327_980);
    assert_eq!(boot_word.observed, Some(0x2100_b007));
    assert_eq!(status.observed, Some(0x2202_0000));
    assert_eq!(value.observed, Some(0x60));
    // The device's own log shows the transfer since the release, not only
    // the firmware's own record (adversarial F1; `AET-WO-2204`).
    assert_eq!(
        witness.as_deref(),
        Some(
            &[
                "[NOISY] i2c1.u2: Write D0".to_owned(),
                "[NOISY] i2c1.u2: Read 60".to_owned()
            ][..]
        )
    );
    let Some(Record::BootObserved {
        instructions_expected,
        instructions_observed,
        ..
    }) = bor3
        .iter()
        .find(|r| matches!(r, Record::BootObserved { .. }))
    else {
        panic!("no boot observed: {bor3:?}");
    };
    assert_eq!(*instructions_expected, 9_877_201);
    assert_eq!(*instructions_observed, Some(9_877_201));
    let Some(Record::Verdict { pass, findings }) = bor3.last() else {
        panic!("the verdict is last");
    };
    assert!(*pass);
    assert_eq!(
        droop_findings(findings),
        [
            ("brownout_reset".to_owned(), "info".to_owned(), 1),
            ("rail_below_tolerance".to_owned(), "info".to_owned(), 1),
        ]
    );
}

/// Renode, run directly, on a fresh HOME. Returns, per `read` block, the
/// whole-line `0x…` values printed between that block's sentinels.
struct Renode {
    commands: Vec<String>,
    blocks: usize,
}

const BOOT_WORD: u32 = 0x2000_0000;
const STATUS_AT: u32 = 0x2000_0004;
const VALUE_AT: u32 = 0x2000_0008;
const COLD: [u32; 3] = [0x5802_44e0, 0x5802_44e8, 0x5802_44f0];
const UNSET: u32 = 0xffff_ffff;

fn run_for(ns: u64) -> String {
    format!(
        "emulation RunFor \"{}.{:09}\"",
        ns / 1_000_000_000,
        ns % 1_000_000_000
    )
}

fn rearm() -> Vec<String> {
    vec![
        format!("sysbus WriteDoubleWord {BOOT_WORD:#010x} 0x0"),
        format!("sysbus WriteDoubleWord {STATUS_AT:#010x} {UNSET:#010x}"),
        format!("sysbus WriteDoubleWord {VALUE_AT:#010x} {UNSET:#010x}"),
    ]
}

fn cold_writes() -> Vec<String> {
    COLD.iter()
        .map(|a| format!("sysbus WriteDoubleWord {a:#010x} 0x0"))
        .collect()
}

const VTOR: &str = "cpu VectorTableOffset 0x08000000";

impl Renode {
    /// The CLI's own boot prefix, then 10 ms of the reference firmware, then
    /// the re-arm that the bridge does before a reset.
    fn booted() -> Self {
        let mut commands = vec![
            "emulation SetSeed 2100".to_owned(),
            "mach create".to_owned(),
            format!(
                "machine LoadPlatformDescription @{}",
                root().join("boards/sentinel-01/sentinel-01.repl").display()
            ),
            format!("sysbus LoadELF @{}", firmware().display()),
        ];
        commands.extend(cold_writes());
        commands.extend(rearm());
        commands.push(VTOR.to_owned());
        commands.push(run_for(10_000_000));
        commands.extend(rearm());
        Self {
            commands,
            blocks: 0,
        }
    }

    fn add(&mut self, cmds: impl IntoIterator<Item = String>) -> &mut Self {
        self.commands.extend(cmds);
        self
    }

    /// Reads each of `reads` (monitor commands) between sentinels.
    fn read(&mut self, reads: &[&str]) -> &mut Self {
        let n = self.blocks;
        self.blocks += 1;
        self.commands.push("logLevel 3".to_owned());
        self.commands.push(format!("echo \"AET2301-BEGIN-{n}\""));
        self.commands.extend(reads.iter().map(|r| (*r).to_owned()));
        self.commands.push(format!("echo \"AET2301-END-{n}\""));
        self
    }

    fn run(&self, label: &str) -> Vec<Vec<u64>> {
        let home = scratch(&format!("renode-{label}"));
        let mut commands = self.commands.clone();
        commands.push("quit".to_owned());
        let start = Instant::now();
        let out = Command::new("renode")
            .args([
                "--disable-gui",
                "--console",
                "--plain",
                "-e",
                &commands.join("; "),
            ])
            .env("HOME", &home)
            .env("DOTNET_EnableDiagnostics", "0")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let text = String::from_utf8_lossy(&out.stdout).replace('\r', "");
        answer_2301(&format!(
            "renode {label} wall_s={:.2}",
            start.elapsed().as_secs_f64()
        ));
        let all: Vec<&str> = text.lines().collect();
        (0..self.blocks)
            .map(|n| {
                let begin = format!("AET2301-BEGIN-{n}");
                let end = format!("AET2301-END-{n}");
                let b = all
                    .iter()
                    .position(|l| *l == begin)
                    .unwrap_or_else(|| panic!("{label}: no {begin} in {text}"));
                let e = all
                    .iter()
                    .position(|l| *l == end)
                    .unwrap_or_else(|| panic!("{label}: no {end} in {text}"));
                // Only whole lines that are a hex value: anything else Renode
                // prints (a log line split into the block) is not a reading.
                all[b + 1..e]
                    .iter()
                    .filter_map(|l| l.strip_prefix("0x"))
                    .filter_map(|h| u64::from_str_radix(h, 16).ok())
                    .collect()
            })
            .collect()
    }
}

const READ_BOOT: &str = "sysbus ReadDoubleWord 0x20000000";
const READ_STATUS: &str = "sysbus ReadDoubleWord 0x20000004";
const READ_VALUE: &str = "sysbus ReadDoubleWord 0x20000008";
const READ_INSTR: &str = "cpu ExecutedInstructions";
const READ_VTOR: &str = "cpu VectorTableOffset";

/// `machine Reset`, the three cold writes, the vector table offset, halted, and
/// the re-arm: the recipe the CLI renders a reset with.
fn recipe() -> Vec<String> {
    let mut c = vec!["machine Reset".to_owned()];
    c.extend(cold_writes());
    c.push(VTOR.to_owned());
    c.push("cpu IsHalted true".to_owned());
    c.extend(rearm());
    c
}

#[test]
fn renodes_reset_recipe_is_pinned_step_by_step() {
    // a. `machine Reset` alone never reboots: the core is at zero, the vector
    // table offset is cleared, and nothing runs.
    let mut r = Renode::booted();
    r.add(["machine Reset".to_owned(), run_for(1_000_000)])
        .read(&[READ_BOOT, READ_INSTR, READ_VTOR]);
    let a = r.run("a-machine-reset-alone");
    assert_eq!(
        a,
        [vec![0x0, 0x0, 0x0]],
        "boot word, instructions, VTOR after `machine Reset` alone"
    );

    // b. The recipe, held 1 ms, released, nudged by 10 ns: the firmware boots.
    let mut r = Renode::booted();
    r.add(recipe())
        .add([
            run_for(1_000_000),
            "cpu IsHalted false".to_owned(),
            run_for(10),
        ])
        .add([run_for(100_000)])
        .read(&[READ_BOOT, READ_STATUS, READ_VALUE]);
    let b = r.run("b-recipe");
    assert_eq!(
        b,
        [vec![0x2100_b007, 0x2202_0000, 0x60]],
        "boot word, status, value 100 us after the release"
    );

    // c. The first RunFor after a release executes nothing (F5): without the
    // 10 ns nudge, 10 us later no instruction has run; with it, 1000 have.
    let mut r = Renode::booted();
    r.add(recipe())
        .add([run_for(1_000_000), "cpu IsHalted false".to_owned()])
        .add([run_for(10_000)])
        .read(&[READ_INSTR]);
    let no_nudge = r.run("c-no-nudge");
    assert_eq!(no_nudge, [vec![0]], "instructions, no nudge");
    let mut r = Renode::booted();
    r.add(recipe())
        .add([
            run_for(1_000_000),
            "cpu IsHalted false".to_owned(),
            run_for(10),
            run_for(10_000),
        ])
        .read(&[READ_INSTR]);
    let nudged = r.run("c-nudge");
    assert_eq!(nudged, [vec![1000]], "instructions, with the nudge");
}

#[test]
fn a_flooding_firmware_reaches_a_verdict_and_its_message_is_the_same_twice() {
    let elf = flood_firmware();
    let runs: Vec<(Option<i32>, Value)> = (0..2)
        .map(|i| {
            let start = Instant::now();
            let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
                .args([
                    "simulate",
                    "--platform",
                    "boards/sentinel-01/sentinel-01.repl",
                    "--firmware",
                    elf.to_str().unwrap(),
                    "--spec",
                    "examples/sentinel-01-minimal.json",
                    "--for",
                    "10ms",
                ])
                .current_dir(root())
                .stdin(Stdio::null())
                .output()
                .unwrap();
            answer_2301(&format!(
                "flood run {i}: exit={:?} wall_s={:.2} stdout_bytes={}",
                out.status.code(),
                start.elapsed().as_secs_f64(),
                out.stdout.len()
            ));
            let text = String::from_utf8(out.stdout).unwrap();
            let v: Value = serde_json::from_str(text.lines().next().unwrap_or_else(|| {
                panic!(
                    "no stdout; stderr: {}",
                    String::from_utf8_lossy(&out.stderr)
                )
            }))
            .unwrap();
            (out.status.code(), v)
        })
        .collect();
    for (code, v) in &runs {
        assert!(matches!(code, Some(0 | 1)), "{code:?} {v}");
        assert!(v.get("verdict").is_some(), "{v}");
        let kinds: Vec<(&str, &str)> = v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| (f["kind"].as_str().unwrap(), f["severity"].as_str().unwrap()))
            .collect();
        assert!(
            kinds.iter().any(|(k, _)| *k == "locked_peripheral_access"),
            "{kinds:?}"
        );
        assert!(kinds.contains(&("renode_log_flood", "info")), "{kinds:?}");
    }
    let findings = |v: &Value| {
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| {
                (
                    f["kind"].clone(),
                    f["severity"].clone(),
                    f["message"].clone(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(findings(&runs[0].1), findings(&runs[1].1));
}
