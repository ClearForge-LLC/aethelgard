//! `AET-WO-2304b`: the battery-sag chain's tier-1 checks, with no tool.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aethelgard_ir::Specification;
use serde_json::Value;

fn root() -> PathBuf {
    std::fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// `aethelgard verify --spec <spec> --scenarios <dir>/*.json --fab-out <tmp>`
/// from the repository's root with an empty `PATH`: what it refuses before
/// any gate runs, and that nothing was written.
fn verify_fab_out(spec: &str, scenarios: &str) -> (Option<i32>, Value) {
    let mut files: Vec<PathBuf> = std::fs::read_dir(root().join(scenarios))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    let out_dir = Path::new(env!("CARGO_TARGET_TMPDIR")).join(format!(
        "aet-2304b-fab-{}-{}",
        std::process::id(),
        scenarios.replace('/', "-")
    ));
    let _ = std::fs::remove_dir_all(&out_dir);
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    cmd.arg("verify").arg("--spec").arg(spec).arg("--scenarios");
    cmd.args(&files);
    let out = cmd
        .arg("--fab-out")
        .arg(&out_dir)
        .current_dir(root())
        .env_clear()
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(!out_dir.exists(), "a package was written");
    let v: Value = serde_json::from_slice(&out.stdout)
        .unwrap_or_else(|e| panic!("{e}: {}", String::from_utf8_lossy(&out.stderr)));
    (out.status.code(), v)
}

/// §2: Sentinel-03's scenarios cover every row before any gate:
/// `VDD_3V3` (a survived droop that crosses BOR 3, `droop-bor3`, since
/// `AET-WO-2305` §1.4 no longer counts `droop-tolerated`, which crosses
/// none), `VDD_5V` (its consumers' absence), `battery` (a tolerated sag),
/// CONSOLE, CRSF, MAVLINK, SENSORS, FLASH and IMU; `EXT` has no device.
/// Coverage is judged first, so the next precondition is what refuses: the
/// ledger's 280 unacknowledged rows (measured on `ee835bde`), a §7 stop
/// outside this work order's fence.
#[test]
fn sentinel_03_s_scenarios_cover_every_row_and_the_package_waits_on_its_ledger() {
    let (code, v) = verify_fab_out(
        "crates/aethelgard-cli/tests/fixtures/sentinel-03.json",
        "scenarios/sentinel-03",
    );
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "UnwitnessedFieldNotAcknowledged", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .starts_with("no package: 280 unwitnessed ledger row(s)"),
        "{v}"
    );
    // `AET-WO-2307` §1.1: the 43 entries they need, measured in `spike/2307/`.
    let entries = v["error"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 43, "{v}");
    assert!(entries.iter().all(|a| a["reason"] == ""), "{v}");
}

/// §2: the power fixture's campaign is for the campaign only. Its
/// `VDD_5V` has no consumer to render, so it stays unmet with its reason,
/// and no package is claimed; `battery` and `VDD_1V8` are met by its set.
#[test]
fn the_power_fixture_s_vdd_5v_stays_unmet_with_its_reason() {
    let (code, v) = verify_fab_out(
        "crates/aethelgard-ir/tests/fixtures/sentinel-02-power.json",
        "scenarios/sentinel-02-power",
    );
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "CampaignCoverageShort", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("rail `VDD_5V` (`VDD_5V` has no consumer a judge observes)"),
        "{m}"
    );
    assert!(!m.contains("battery") && !m.contains("VDD_1V8"), "{m}");
}

fn spec(path: &str) -> Specification {
    let bytes = std::fs::read(format!("{}/{path}", env!("CARGO_MANIFEST_DIR"))).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn decoupling(path: &str) -> BTreeMap<String, u64> {
    let s = spec(path);
    let table = aethelgard_svd_hal::pin_table(&s.mcu.part).unwrap();
    let t = aethelgard_ir::solve(&s, &table, &"0".repeat(64)).unwrap();
    aethelgard_eda::schematic::decoupling_picofarads(&s, &t, &table).unwrap()
}

/// F3 (`AET-WO-2304b` §1.4): a regulator's output capacitors count on its
/// rail at IR's credited figure (no maker curve is held for the four Cout
/// parts, so the fixture's 500 per mille declared derating: 4 × 11 µF a
/// rail), measured; a board without a tree moves no byte (`sentinel-01`,
/// `motors-dshot600`: 8.3 µF, as on `main`).
#[test]
fn a_regulator_s_output_capacitors_count_in_its_rail_s_decoupling() {
    let pins: [(&str, &[(&str, u64)]); 4] = [
        (
            "tests/fixtures/sentinel-03.json",
            &[("VDD_3V3", 54_710_000), ("VDD_5V", 44_000_000)],
        ),
        (
            "../aethelgard-ir/tests/fixtures/sentinel-02-power.json",
            &[
                ("VDD_1V8", 700_000),
                ("VDD_3V3", 52_300_000),
                ("VDD_5V", 44_000_000),
            ],
        ),
        (
            "../../examples/sentinel-01-minimal.json",
            &[("VDD_3V3", 8_300_000)],
        ),
        (
            "tests/fixtures/motors-dshot600.json",
            &[("VDD_3V3", 8_300_000)],
        ),
    ];
    for (path, want) in pins {
        let want: BTreeMap<String, u64> =
            want.iter().map(|(d, pf)| ((*d).to_owned(), *pf)).collect();
        assert_eq!(decoupling(path), want, "{path}");
    }
}
