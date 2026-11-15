//! `AET-WO-3300`, tier 1: every refusal of `verify --fab-out` and
//! `--against-fab` that comes before any gate. Each call runs with an empty
//! `PATH`, so a refusal that waited for a tool would show as exit 4 instead.
//! The package itself is tier 2 (`board_tier2.rs`); the stripping of KiCad's
//! clock lines is pinned by `fab.rs`'s unit tests on committed KiCad output.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

const SPEC: &str = "examples/sentinel-01-minimal.json";
/// The reference's seeded set (`AET-WO-3303` D1, ruled): a tolerated fault on
/// `SENSORS` and on `VDD_3V3`, `FLASH`'s absent device detected by its
/// mechanism, and the no-fault baseline.
const SCENARIOS: [&str; 4] = [
    "scenarios/sentinel-01/seeded/pullup-3k3.json",
    "scenarios/sentinel-01/droop-bor3.json",
    "scenarios/sentinel-01/seeded/spi-absent-u4.json",
    "scenarios/sentinel-01/pullup-2k2.json",
];

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-3300-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let _ = fs::remove_file(&dir);
    dir
}

/// Runs the binary from the repository root with an empty `PATH`.
fn run(args: &[&str]) -> (Option<i32>, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{args:?}: {stdout}");
    (out.status.code(), serde_json::from_str(&stdout).unwrap())
}

fn s(p: &Path) -> &str {
    p.to_str().unwrap()
}

/// §1.2: `--fab-out` must be an empty directory or absent. A used directory,
/// a file and a symlink (even to an empty directory) are refused with exit
/// 3 before any gate, and nothing in or behind them changes.
#[test]
fn an_unusable_fab_out_is_refused_before_any_gate_and_left_untouched() {
    let used = scratch("used");
    fs::create_dir_all(&used).unwrap();
    fs::write(used.join("keep"), "x").unwrap();
    let file = scratch("file");
    fs::write(&file, "x").unwrap();
    let target = scratch("target");
    fs::create_dir_all(&target).unwrap();
    let link = scratch("link");
    std::os::unix::fs::symlink(&target, &link).unwrap();
    for dir in [&used, &file, &link] {
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIOS[0],
            SCENARIOS[1],
            SCENARIOS[2],
            SCENARIOS[3],
            "--fab-out",
            s(dir),
        ]);
        assert_eq!(code, Some(3), "{}: {v}", dir.display());
        assert_eq!(v["ok"], false);
        assert_eq!(v["error"]["kind"], "UnwritableOutput", "{v}");
    }
    assert_eq!(fs::read_dir(&used).unwrap().count(), 1);
    assert_eq!(fs::read(used.join("keep")).unwrap(), b"x");
    assert_eq!(fs::read(&file).unwrap(), b"x");
    assert_eq!(fs::read_dir(&target).unwrap().count(), 0);
    for p in [&used, &target] {
        fs::remove_dir_all(p).unwrap();
    }
    fs::remove_file(&file).unwrap();
    fs::remove_file(&link).unwrap();
}

/// A run that cannot pass writes nothing: an absent `--fab-out` is not
/// created when a gate's tool is missing (exit 4), and the two fab flags
/// are one or the other.
#[test]
fn a_run_that_cannot_pass_creates_nothing_and_the_flags_exclude_each_other() {
    let absent = scratch("absent");
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--fab-out",
        s(&absent),
    ]);
    assert_eq!(code, Some(4), "{v}");
    assert!(
        !absent.exists(),
        "--fab-out was created by a run that stopped"
    );

    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--against-fab",
        s(&absent),
    ]);
    assert_eq!(code, Some(3), "{v}");

    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--fab-out",
        s(&absent),
        "--against-fab",
        s(&absent),
    ]);
    assert_eq!(code, Some(64), "{v}");
    assert!(!absent.exists());
}

/// Adversarial A1: an existing `--fab-out` that another uid could write is
/// refused before any gate: group- or other-writable, or (as root, where a
/// test can make one) owned by another uid.
#[test]
fn a_fab_out_another_uid_could_write_is_refused() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("shared");
    fs::create_dir_all(&dir).unwrap();
    for mode in [0o775, 0o757] {
        fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).unwrap();
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIOS[0],
            SCENARIOS[1],
            SCENARIOS[2],
            SCENARIOS[3],
            "--fab-out",
            s(&dir),
        ]);
        assert_eq!(code, Some(3), "{mode:o}: {v}");
        assert_eq!(v["error"]["kind"], "UnwritableOutput");
    }
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o755)).unwrap();
    if rustix::process::geteuid().is_root() {
        std::os::unix::fs::chown(&dir, Some(65534), None).unwrap();
        let (code, v) = run(&[
            "verify",
            "--spec",
            SPEC,
            "--scenarios",
            SCENARIOS[0],
            SCENARIOS[1],
            SCENARIOS[2],
            SCENARIOS[3],
            "--fab-out",
            s(&dir),
        ]);
        assert_eq!(code, Some(3), "{v}");
        assert_eq!(v["error"]["kind"], "UnwritableOutput");
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// `--against` that differs fails before any gate; with `--fab-out` the
/// verdict says nothing was written, and nothing is.
#[test]
fn a_tree_that_differs_writes_no_package_and_says_so() {
    let tree = scratch("against-empty");
    fs::create_dir_all(&tree).unwrap();
    let out = scratch("against-out");
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--against",
        s(&tree),
        "--fab-out",
        s(&out),
    ]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["against"], "differs");
    assert_eq!(v["fab"]["status"], "not_written");
    assert!(!out.exists());
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--against",
        s(&tree),
        "--against-fab",
        s(&tree),
    ]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["fab"]["status"], "not_derived");
    assert_eq!(v["against_fab"], "not_compared");
    fs::remove_dir_all(&tree).unwrap();
}

/// Ruling D3 on PR #103: `INV-03` counts the fault campaign among the gates
/// of "verified", so each fab flag requires `--scenarios`; without it the
/// call is a usage error, exit 64, and nothing is created.
#[test]
fn each_fab_flag_requires_scenarios() {
    let absent = scratch("no-scenarios");
    for flag in ["--fab-out", "--against-fab"] {
        let (code, v) = run(&["verify", "--spec", SPEC, flag, s(&absent)]);
        assert_eq!(code, Some(64), "{flag}: {v}");
        assert_eq!(v["error"]["kind"], "UsageError", "{flag}: {v}");
        assert!(!absent.exists(), "{flag}");
    }
}

/// `AET-WO-4210` D1, ruled by Scotty: an `eol` part the specification does
/// not acknowledge is refused by `validate`, so on the fab path too, by name,
/// before anything is generated or written. The 1.8 V fixture acknowledges
/// both stand-ins; without its list the first refused device in the
/// specification's order is U6, the ICP-10111.
#[test]
fn an_unacknowledged_eol_part_is_refused_on_the_fab_path_by_name() {
    let fixture = root().join("crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json");
    let mut spec: Value = serde_json::from_slice(&fs::read(&fixture).unwrap()).unwrap();
    spec["accept_lifecycle"] = serde_json::json!([]);
    let dir = scratch("eol-spec");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("unacknowledged.json");
    fs::write(&path, spec.to_string()).unwrap();
    let out = scratch("eol-out");
    let (code, v) = run(&[
        "verify",
        "--spec",
        s(&path),
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        "--fab-out",
        s(&out),
    ]);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "LifecycleNotAccepted", "{v}");
    assert_eq!(v["error"]["id"], "U6", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("ICP10111") && m.contains("`eol`") && m.contains("EOL badge"),
        "{m}"
    );
    assert!(!out.exists(), "nothing is created");
}

/// `AET-WO-3303` §1.1, D1 as ruled: a campaign that covers a declared bus or
/// rail with no fault is refused by each fab flag before any gate
/// (`CampaignCoverageShort`), naming every uncovered one, and nothing is
/// created. The baseline alone covers nothing; a detectable fault covers only
/// beside a baseline; each tolerated fault covers only its own target.
#[test]
fn a_campaign_that_misses_a_bus_or_rail_is_refused_by_name() {
    let out = scratch("coverage");
    // `--against-fab` names an existing directory; it is left empty.
    let existing = scratch("coverage-against");
    fs::create_dir_all(&existing).unwrap();
    let all: &[&str] = &["I2C bus `SENSORS`", "SPI bus `FLASH`", "rail `VDD_3V3`"];
    let cases: [(&[&str], &[&str]); 5] = [
        (&[SCENARIOS[3]], all),
        (&[SCENARIOS[2]], all),
        (
            &[SCENARIOS[0], SCENARIOS[3]],
            &["SPI bus `FLASH`", "rail `VDD_3V3`"],
        ),
        (
            &[SCENARIOS[1], SCENARIOS[3]],
            &["I2C bus `SENSORS`", "SPI bus `FLASH`"],
        ),
        (
            &[SCENARIOS[0], SCENARIOS[1], SCENARIOS[2]],
            &["SPI bus `FLASH`"],
        ),
    ];
    for flag in ["--fab-out", "--against-fab"] {
        for (scenarios, named) in cases {
            let mut args = vec!["verify", "--spec", SPEC, "--scenarios"];
            args.extend(scenarios);
            let dir = if flag == "--fab-out" { &out } else { &existing };
            args.extend([flag, s(dir)]);
            let (code, v) = run(&args);
            assert_eq!(code, Some(3), "{flag} {scenarios:?}: {v}");
            assert_eq!(v["error"]["kind"], "CampaignCoverageShort", "{v}");
            let m = v["error"]["message"].as_str().unwrap();
            for n in named {
                assert!(m.contains(n), "{flag} {scenarios:?}: {m}");
            }
            assert_eq!(m.matches('`').count(), 2 * named.len(), "{m}");
            assert!(!out.exists());
            assert_eq!(fs::read_dir(&existing).unwrap().count(), 0);
        }
    }
    fs::remove_dir_all(&existing).unwrap();
}

/// `AET-WO-3303` D1, ruled (b) and (c): a rail other than the MCU's supply
/// cannot be faulted by any override kind, so a package of the connectors
/// fixture is refused naming it; its SPI bus with no device does not block.
#[test]
fn a_rail_or_bus_no_fault_kind_reaches_is_named() {
    let dir = scratch("coverage-d1");
    fs::create_dir_all(&dir).unwrap();
    fs::copy(
        root().join("crates/aethelgard-cli/tests/fixtures/connectors.json"),
        dir.join("connectors.json"),
    )
    .unwrap();
    // Everything a fault can reach is covered: a tolerated pull-up and droop,
    // U4 absent (detected), and the baseline.
    let write =
        |name: &str, expect: &str, findings: serde_json::Value, overrides: serde_json::Value| {
            let scenario = serde_json::json!({
                "schema_version": 2, "name": name, "base": "connectors.json",
                "expect": expect, "expect_findings": findings, "overrides": overrides,
            });
            let path = dir.join(format!("{name}.json"));
            fs::write(&path, scenario.to_string()).unwrap();
            path
        };
    let files = [
        write(
            "tolerated",
            "pass",
            serde_json::json!([]),
            serde_json::json!([
            {"bus": "SENSORS", "pull_up_ohms": 3300},
            {"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "start_ns": 1000000,
                "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
                "source_milliohms": 500, "load_milliamps": 100, "bor_level": 0}}]),
        ),
        write(
            "absent",
            "fail",
            serde_json::json!(["spi_identity_mismatch", "spi_transaction_unwitnessed"]),
            serde_json::json!([{"spi_device_absent": "U4"}]),
        ),
        write(
            "baseline",
            "pass",
            serde_json::json!([]),
            serde_json::json!([]),
        ),
    ];
    let out = dir.join("pkg");
    let (code, v) = run(&[
        "verify",
        "--spec",
        s(&dir.join("connectors.json")),
        "--scenarios",
        s(&files[0]),
        s(&files[1]),
        s(&files[2]),
        "--fab-out",
        s(&out),
    ]);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "CampaignCoverageShort", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    assert!(m.contains("rail `VDD_5V` (not the MCU's supply"), "{m}");
    // `EXT` has no device: named in the verdict's row, never blocking.
    assert!(!m.contains("EXT"), "{m}");
    assert!(
        !m.contains("SENSORS") && !m.contains("FLASH") && !m.contains("VDD_3V3"),
        "{m}"
    );
    assert!(!out.exists());
    fs::remove_dir_all(&dir).unwrap();
}
