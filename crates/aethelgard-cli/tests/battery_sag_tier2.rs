//! `AET-WO-2304b` §2, tier 2: the two tree boards' campaigns in the tools
//! image, with the real bridge (`--chain`), ngspice and Renode, on firmware
//! `generate` and `build` make from each specification.
//!
//! 1. Sentinel-03's set (`scenarios/sentinel-03/`) meets every expectation,
//!    written before any run: a baseline, a tolerated battery sag, a
//!    survived droop below `VDD_3V3`'s band, `VDD_5V` out of band (CRSF and
//!    MAVLINK open, one E3), each serial port open, SENSORS held low, and
//!    FLASH and IMU absent.
//! 2. The sag's line carries kind 31's chain (the solved minimum within
//!    1 mV of the static point, U10's floor binding) and §1.11's
//!    conditions; the rail's carries §1.7's, verbatim.
//! 3. Every run replays to its own state-sequence hash, and a second
//!    campaign writes every trace byte for byte again (`INV-04`).
//! 4. The power fixture's set (baseline, sag, `VDD_1V8` out of band, U5
//!    absent) meets every expectation; it is for the campaign only.

#![cfg(feature = "with-tools")]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

mod common;

use common::verify::root;

fn aethelgard(args: &[&std::ffi::OsStr]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        !stdout.is_empty(),
        "aethelgard printed nothing: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    (out.status.code(), stdout)
}

fn os(s: &str) -> &std::ffi::OsStr {
    std::ffi::OsStr::new(s)
}

fn scratch(label: &str) -> PathBuf {
    let d = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-2304b-{}-{label}", std::process::id()));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).unwrap();
    d
}

/// `generate` then `build` on `spec`: the ELF.
fn firmware(spec: &str, label: &str) -> PathBuf {
    let dir = scratch(label);
    let (code, out) = aethelgard(&[
        os("generate"),
        os("--spec"),
        os(spec),
        os("--out"),
        dir.join("gen").as_os_str(),
    ]);
    assert_eq!(code, Some(0), "{out}");
    let elf = dir.join("fw.elf");
    let (code, out) = aethelgard(&[
        os("build"),
        os("--crate"),
        dir.join("gen/board").as_os_str(),
        os("--out"),
        elf.as_os_str(),
    ]);
    assert_eq!(code, Some(0), "{out}");
    elf
}

/// The campaign over every scenario in `dir`, into `archive`: its lines.
fn campaign(dir: &str, elf: &Path, archive: &Path) -> Vec<Value> {
    let mut files: Vec<PathBuf> = fs::read_dir(root().join(dir))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let mut args = vec![os("campaign"), os("--scenarios")];
    args.extend(files.iter().map(|f| f.as_os_str()));
    args.extend([
        os("--firmware"),
        elf.as_os_str(),
        os("--archive"),
        archive.as_os_str(),
    ]);
    let (code, out) = aethelgard(&args);
    let lines: Vec<Value> = out
        .lines()
        .map(|l| serde_json::from_str(l).unwrap())
        .collect();
    assert_eq!(code, Some(0), "{out}");
    assert_eq!(lines.len(), files.len() + 1, "{out}");
    for l in &lines[..files.len()] {
        assert_eq!(l["matched"], true, "{l}");
    }
    lines[..files.len()].to_vec()
}

/// Each run replays to its own hash; a second campaign writes every trace
/// again byte for byte.
fn replays_and_repeats(dir: &str, elf: &Path, first: &[Value], archive: &Path, label: &str) {
    for l in first {
        let run = l["run"].as_str().unwrap();
        let (code, out) = aethelgard(&[os("replay"), os("--archive"), os(run)]);
        assert_eq!(code, Some(0), "{out}");
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["state_sequence_hash"], l["state_sequence_hash"], "{v}");
    }
    let again = scratch(label).join("archive");
    let second = campaign(dir, elf, &again);
    for (a, b) in first.iter().zip(&second) {
        assert_eq!(
            a["state_sequence_hash"], b["state_sequence_hash"],
            "{a} {b}"
        );
        let id = a["run_id"].as_str().unwrap();
        let read = |d: &Path| fs::read(d.join(id).join("trace.cbor")).unwrap();
        assert!(read(archive) == read(&again), "{id}'s trace differs");
    }
}

fn line<'a>(lines: &'a [Value], scenario: &str) -> &'a Value {
    lines
        .iter()
        .find(|l| l["scenario"] == scenario)
        .unwrap_or_else(|| panic!("no run of {scenario}"))
}

#[test]
fn sentinel_03_s_campaign_meets_every_expectation_replays_and_repeats() {
    let spec = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";
    let elf = firmware(spec, "s03");
    let archive = scratch("s03-a").join("archive");
    let lines = campaign("scenarios/sentinel-03", &elf, &archive);
    let sag = line(&lines, "battery-sag-tolerated");
    let chain = &sag["battery_sag"];
    let (s, m) = (
        chain["static_microvolts"].as_u64().unwrap(),
        chain["solved_minimum_microvolts"].as_u64().unwrap(),
    );
    assert_eq!(s, 8_088_035, "{chain}");
    assert!(m.abs_diff(s) <= 1_000 && m >= 7_000_000, "{chain}");
    assert_eq!(chain["binding"], "U10", "{chain}");
    assert_eq!(chain["binding_floor_microvolts"], 7_000_000, "{chain}");
    let conditions = |l: &Value| -> Vec<String> {
        l["conditions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap().to_owned())
            .collect()
    };
    let c = conditions(sag);
    for want in [
        "The battery's load is the tree's declared static load; no motor current is commanded, so no reboot loop is shown or excluded.",
        "A battery sag is admitted only while every battery-fed regulator stays at or above its stated regulation floor (D-B9); no brownout is shown or excluded.",
    ] {
        assert!(c.iter().any(|x| x == want), "{c:?}");
    }
    assert_eq!(
        c.iter().filter(|x| x.starts_with("Motor output")).count(),
        4,
        "{c:?}"
    );
    assert!(
        c.iter()
            .any(|x| x.starts_with("The healthy path R is declared 1 mΩ")),
        "{c:?}"
    );
    assert_eq!(
        conditions(line(&lines, "vdd5v-out-of-band")),
        [
            "`VDD_5V` below its band is rendered as its consumers' absence (CRSF and MAVLINK RX open; `EXT` has no device and is not rendered). How a DS-009 peer behaves below 4.75 V is not declared; absent is a modelling assumption (AET-WO-2304 D4)."
        ]
    );
    // A run with no sag and no rail names no condition.
    assert!(line(&lines, "baseline").get("conditions").is_none());
    replays_and_repeats("scenarios/sentinel-03", &elf, &lines, &archive, "s03-b");
}

#[test]
fn the_power_fixture_s_campaign_meets_every_expectation_replays_and_repeats() {
    let spec = "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json";
    let elf = firmware(spec, "p02");
    let archive = scratch("p02-a").join("archive");
    let lines = campaign("scenarios/sentinel-02-power", &elf, &archive);
    assert_eq!(
        line(&lines, "battery-sag-tolerated")["battery_sag"]["binding"],
        "U10"
    );
    replays_and_repeats(
        "scenarios/sentinel-02-power",
        &elf,
        &lines,
        &archive,
        "p02-b",
    );
}
