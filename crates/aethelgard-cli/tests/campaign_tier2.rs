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
