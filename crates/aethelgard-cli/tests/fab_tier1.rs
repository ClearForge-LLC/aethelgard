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
/// mechanism, the no-fault baseline, and `CONSOLE`'s open RX line detected
/// by its mechanism (`AET-WO-2302` D5).
const SCENARIOS: [&str; 5] = [
    "scenarios/sentinel-01/seeded/pullup-3k3.json",
    "scenarios/sentinel-01/droop-bor3.json",
    "scenarios/sentinel-01/seeded/spi-absent-u4.json",
    "scenarios/sentinel-01/pullup-2k2.json",
    "scenarios/sentinel-01/seeded/serial-rx-open-console.json",
];

/// `AET-WO-4300k` (T), ruled 11:25Z: a `--fab-out` test whose point lies
/// past the readiness gate runs on the ready fixture (`common/ready.rs`);
/// the reference itself is refused `FabricationNotReady`.
mod common;
#[path = "common/ready.rs"]
mod ready;
use ready::{READY, READY_SCENARIOS};

/// `verify --spec READY --scenarios READY_SCENARIOS... rest...`, as [`run`].
fn ready_run(rest: &[&str]) -> (Option<i32>, Value) {
    let mut args = vec!["verify", "--spec", READY, "--scenarios"];
    args.extend(READY_SCENARIOS);
    args.extend(rest);
    run(&args)
}

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// A test's scratch, under the repository whatever `CARGO_TARGET_DIR` is
/// (`AET-WO-1607` (3), `1211e` F11): `verify` runs from the repository root,
/// and a scenario's `base` must resolve inside it (`BaseOutsideRoot`). In
/// `target/`, which git ignores.
fn scratch(label: &str) -> PathBuf {
    let dir = root()
        .join("target/aet-tests")
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
            SCENARIOS[4],
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
    let (code, v) = ready_run(&["--fab-out", s(&absent)]);
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
        SCENARIOS[4],
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
        SCENARIOS[4],
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
            SCENARIOS[4],
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
            SCENARIOS[4],
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
    // `AET-WO-2308` L4: an empty directory in the tree is named too.
    fs::create_dir_all(tree.join("stray")).unwrap();
    let out = scratch("against-out");
    let (code, v) = ready_run(&["--against", s(&tree), "--fab-out", s(&out)]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["against"], "differs");
    assert!(
        v["findings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["kind"] == "against_extra"
                && f["message"] == "stray/ is in the directory but is not generated"),
        "{v}"
    );
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
        SCENARIOS[4],
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

/// `AET-WO-2308` (1), shape (b): a twin stand-in drawn on a board (the
/// 1.8 V fixture drawn, with its one drawable stand-in, U5's ICM20948) is
/// refused by each fab flag by name, exit 2, before any gate and before
/// anything is written: the emitter's BOM carries no label for it. It comes
/// after the campaign's coverage check (a board refused for its campaign
/// keeps that verdict), so four scenarios cover the board's bus, SPI device
/// and rail; they are read before any gate, never run. Without a fab flag
/// the same specification is not refused for it. The fixture's ICP-10111 has
/// no symbol and its second ICM20948 does not route, so both are dropped;
/// every device declares a pin capacitance (the parts table cites none).
#[test]
fn a_drawn_twin_stand_in_is_refused_by_each_fab_flag_by_name() {
    let fixture = root().join("crates/aethelgard-cli/tests/fixtures/imu-baro-1v8.json");
    let mut spec: Value = serde_json::from_slice(&fs::read(&fixture).unwrap()).unwrap();
    spec["board"]["drawn"] = true.into();
    let keep = |list: &mut Value, drop: &str| {
        let kept: Vec<Value> = list
            .as_array()
            .unwrap()
            .iter()
            .filter(|d| d["reference"] != drop && d["part"] != drop)
            .cloned()
            .collect();
        *list = Value::Array(kept);
    };
    keep(&mut spec["i2c_devices"], "ICP10111");
    keep(&mut spec["accept_lifecycle"], "ICP10111");
    keep(&mut spec["spi_devices"], "U7");
    for list in ["i2c_devices", "spi_devices"] {
        for d in spec[list].as_array_mut().unwrap() {
            d["pin_capacitance"] = serde_json::json!({"declared": {
                "femtofarads": 10000,
                "condition": "AET-WO-2308 fixture: a bound for the test, not a part's",
                "source": "AET-WO-2308 fixture, not a design"
            }});
        }
    }
    let dir = scratch("standin");
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("spec.json"), spec.to_string()).unwrap();
    // Planted at the boot word (`AET-WO-2310`): tier 1's verify judges
    // coverage before any boot, so the instant is never placed.
    let droop = serde_json::json!({"droop": {"domain": "VDD_1V8", "floor_millivolts": 1650,
        "from_boot_word_ns": 0, "fall_ns": 20000, "hold_ns": 200_000, "rise_ns": 20000,
        "source_milliohms": {"value": 500, "source": {"stimulus": "a test sag"}}, "load_milliamps": {"value": 100, "source": {"stimulus": "a test sag"}}}});
    for (name, version, expect, findings, overrides) in [
        (
            "baseline",
            1,
            "pass",
            serde_json::json!([]),
            serde_json::json!([]),
        ),
        (
            "pullup",
            1,
            "pass",
            serde_json::json!([]),
            serde_json::json!([{"bus": "SENSORS", "pull_up_ohms": 3300}]),
        ),
        (
            "absent",
            2,
            "fail",
            serde_json::json!(["spi_identity_mismatch", "spi_transaction_unwitnessed"]),
            serde_json::json!([{"spi_device_absent": "U5"}]),
        ),
        (
            "droop",
            7,
            "pass",
            serde_json::json!([]),
            serde_json::json!([droop]),
        ),
    ] {
        let mut v = serde_json::json!({"schema_version": version, "name": name, "base": "spec.json",
            "expect": expect, "expect_findings": findings, "overrides": overrides});
        if name == "droop" {
            v["expect_counts"] = serde_json::json!({"brownout_reset": 0,
                "brownout_unrecovered": 0, "rail_below_tolerance": 1});
        }
        fs::write(dir.join(format!("{name}.json")), v.to_string()).unwrap();
    }
    let verify = |extra: &[&str]| {
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args([
                "verify",
                "--spec",
                "spec.json",
                "--scenarios",
                "baseline.json",
                "pullup.json",
                "absent.json",
                "droop.json",
            ])
            .args(extra)
            .current_dir(&dir)
            .env("PATH", "")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let stdout = String::from_utf8(out.stdout).unwrap();
        assert_eq!(stdout.lines().count(), 1, "{extra:?}: {stdout}");
        let v: Value = serde_json::from_str(&stdout).unwrap();
        (out.status.code(), v)
    };
    fs::create_dir_all(dir.join("against")).unwrap();
    for (flag, target) in [("--fab-out", "pkg"), ("--against-fab", "against")] {
        let (code, v) = verify(&[flag, target]);
        assert_eq!(code, Some(2), "{flag}: {v}");
        assert_eq!(v["error"]["kind"], "StandInInFabPackage", "{flag}: {v}");
        assert_eq!(v["error"]["id"], "U5", "{flag}: {v}");
        let m = v["error"]["message"].as_str().unwrap();
        assert!(
            m.contains("U5 (ICM20948)") && m.contains("never for a purchase"),
            "{m}"
        );
    }
    assert!(!dir.join("pkg").exists(), "nothing is created");
    assert_eq!(fs::read_dir(dir.join("against")).unwrap().count(), 0);
    // Without a fab flag the run goes on to the gates, and with an empty
    // `PATH` the first tool is missing: never this refusal.
    let (_, v) = verify(&[]);
    assert_ne!(v["error"]["kind"], "StandInInFabPackage", "{v}");
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
        SCENARIOS[4],
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
    let all: &[&str] = &[
        "I2C bus `SENSORS`",
        "SPI bus `FLASH`",
        "serial port `CONSOLE`",
        "rail `VDD_3V3`",
    ];
    let cases: [(&[&str], &[&str]); 7] = [
        (&[SCENARIOS[3]], all),
        (&[SCENARIOS[2]], all),
        (
            &[SCENARIOS[0], SCENARIOS[3]],
            &["SPI bus `FLASH`", "serial port `CONSOLE`", "rail `VDD_3V3`"],
        ),
        (
            &[SCENARIOS[1], SCENARIOS[3]],
            &[
                "I2C bus `SENSORS`",
                "SPI bus `FLASH`",
                "serial port `CONSOLE`",
            ],
        ),
        (
            &[SCENARIOS[0], SCENARIOS[1], SCENARIOS[2]],
            &["SPI bus `FLASH`", "serial port `CONSOLE`"],
        ),
        // `AET-WO-2302` D5: the console is covered only by its own open RX
        // line detected beside the baseline.
        (
            &[SCENARIOS[0], SCENARIOS[1], SCENARIOS[2], SCENARIOS[3]],
            &["serial port `CONSOLE`"],
        ),
        (
            &[SCENARIOS[0], SCENARIOS[1], SCENARIOS[2], SCENARIOS[4]],
            &["SPI bus `FLASH`", "serial port `CONSOLE`"],
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
                "schema_version": 7, "name": name, "base": "connectors.json",
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
            // Never placed: tier 1's verify stops at coverage (`AET-WO-2310`).
            {"droop": {"domain": "VDD_3V3", "floor_millivolts": 2400, "from_boot_word_ns": 0,
                "fall_ns": 20000, "hold_ns": 200000, "rise_ns": 20000,
                "source_milliohms": {"value": 500, "source": {"stimulus": "a test sag"}}, "load_milliamps": {"value": 100, "source": {"stimulus": "a test sag"}}}}]),
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
    assert!(
        m.contains("rail `VDD_5V` (`VDD_5V` has no consumer a judge observes)"),
        "{m}"
    );
    // `EXT` has no device: named in the verdict's row, never blocking.
    assert!(!m.contains("EXT"), "{m}");
    assert!(
        !m.contains("SENSORS") && !m.contains("FLASH") && !m.contains("VDD_3V3"),
        "{m}"
    );
    assert!(!out.exists());
    fs::remove_dir_all(&dir).unwrap();
}

/// `spec` written as `spec.json` in a scratch directory, beside the
/// reference's seeded set rebased on it, so the campaign's preconditions are
/// met and the run reaches the acknowledgement check; then `verify` with
/// `flag`, from that directory, with an empty `PATH`.
fn rebased_verify(label: &str, spec: &Value, flag: &str) -> (Option<i32>, Value, PathBuf) {
    let dir = scratch(label);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("spec.json"), spec.to_string()).unwrap();
    let mut args = vec![
        "verify".to_owned(),
        "--spec".to_owned(),
        "spec.json".to_owned(),
        "--scenarios".to_owned(),
    ];
    for (i, path) in SCENARIOS.iter().enumerate() {
        let mut scenario: Value =
            serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap();
        scenario["base"] = "spec.json".into();
        let name = format!("scenario-{i}.json");
        fs::write(dir.join(&name), scenario.to_string()).unwrap();
        args.push(name);
    }
    // `--fab-out` absent, `--against-fab` an existing empty directory.
    let out_dir = dir.join("pkg");
    if flag == "--against-fab" {
        fs::create_dir_all(&out_dir).unwrap();
    }
    args.extend([flag.to_owned(), "pkg".to_owned()]);
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(&args)
        .current_dir(&dir)
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        out.status.code(),
        serde_json::from_str(&stdout).unwrap(),
        out_dir,
    )
}

fn reference() -> Value {
    serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap()
}

/// `AET-WO-4230` R1 (the architect's ruling on PR #161): each fab flag refuses
/// a specification that leaves any unwitnessed ledger row unacknowledged
/// (`UnwitnessedFieldNotAcknowledged`, exit 2), naming every such row id,
/// before any gate and before anything is written. Here a whole entry is
/// dropped (`GPIOB.OTYPER`, 16 rows) and one field of another
/// (`GPIOA.MODER`'s `MODER0`, which is what a field the generator newly
/// writes in an acknowledged register looks like to the check).
/// `AET-WO-2307`: `error.entries` holds the two entries the specification
/// lacks, `GPIOA.MODER` completed and `GPIOB.OTYPER` new, each with reason
/// "" (F1 as ruled) and its fields sorted; the message states the reason rule.
#[test]
fn an_unacknowledged_unwitnessed_row_is_refused_by_each_fab_flag_naming_every_one() {
    let mut spec = reference();
    let held = |register: &str| {
        let mut a = spec["acknowledge_unwitnessed"]
            .as_array()
            .unwrap()
            .iter()
            .find(|a| a["register"] == register)
            .unwrap()
            .clone();
        a["fields"]
            .as_array_mut()
            .unwrap()
            .sort_by(|x, y| x.as_str().unwrap().cmp(y.as_str().unwrap()));
        a
    };
    let mut want = Value::Array(vec![held("GPIOA.MODER"), held("GPIOB.OTYPER")]);
    want[0]["reason"] = "".into();
    want[1]["reason"] = "".into();
    let entries = spec["acknowledge_unwitnessed"].as_array_mut().unwrap();
    entries.retain(|a| a["register"] != "GPIOB.OTYPER");
    let moder = entries
        .iter_mut()
        .find(|a| a["register"] == "GPIOA.MODER")
        .unwrap();
    moder["fields"]
        .as_array_mut()
        .unwrap()
        .retain(|f| f != "MODER0");
    for flag in ["--fab-out", "--against-fab"] {
        let (code, v, out) = rebased_verify("unacknowledged", &spec, flag);
        assert_eq!(code, Some(2), "{flag}: {v}");
        assert_eq!(v["error"]["kind"], "UnwitnessedFieldNotAcknowledged", "{v}");
        let m = v["error"]["message"].as_str().unwrap();
        let mut named: Vec<String> = (0..16).map(|i| format!("GPIOB.OTYPER.OT{i}")).collect();
        named.push("GPIOA.MODER.MODER0".into());
        assert!(m.contains("17 unwitnessed ledger row(s)"), "{m}");
        for id in &named {
            assert!(
                m.contains(&format!("{id},")) || m.contains(&format!("{id}. ")),
                "{id}: {m}"
            );
        }
        assert_eq!(
            v["error"]["id"], "GPIOB.OTYPER.OT0",
            "the first in the ledger's order: {v}"
        );
        assert_eq!(v["error"]["entries"], want, "{flag}");
        assert!(
            m.contains("`entries` holds the 2 entry(ies)")
                && m.ends_with("1 to 200 bytes of printable ASCII with no space at either end; `validate` refuses an empty one (`InvalidSource`)"),
            "{m}"
        );
        if flag == "--fab-out" {
            assert!(!out.exists(), "nothing is created");
        } else {
            assert_eq!(fs::read_dir(&out).unwrap().count(), 0);
        }
    }
}

/// `AET-WO-1215` beside `AET-WO-2307`: the two refusals that carry
/// `error.entries` print it in one shape, from one command: the same keys in
/// `error`, and `entries` a non-empty array of objects, each refusal's own
/// element (D7 as ruled).
#[test]
fn both_refusals_with_entries_print_one_shape() {
    let mut unacknowledged = reference();
    unacknowledged["acknowledge_unwitnessed"]
        .as_array_mut()
        .unwrap()
        .retain(|a| a["register"] != "GPIOB.OTYPER");
    let mut older = reference();
    older["schema_version"] = 20.into();
    let shape = |v: &Value| {
        let e = v["error"].as_object().unwrap();
        let entries = e["entries"].as_array().unwrap();
        assert!(
            !entries.is_empty() && entries.iter().all(Value::is_object),
            "{v}"
        );
        e.keys().cloned().collect::<Vec<_>>()
    };
    let (code, a, _) = rebased_verify("shape-ack", &unacknowledged, "--fab-out");
    assert_eq!(code, Some(2), "{a}");
    assert_eq!(a["error"]["kind"], "UnwitnessedFieldNotAcknowledged", "{a}");
    let (code, b, _) = rebased_verify("shape-version", &older, "--fab-out");
    assert_eq!(code, Some(2), "{b}");
    assert_eq!(b["error"]["kind"], "UnsupportedSchemaVersion", "{b}");
    assert_eq!(shape(&a), ["entries", "id", "kind", "message"]);
    assert_eq!(shape(&a), shape(&b));
}

/// R1, a new field in an acknowledged register: the reference with the
/// 4-layer fixture's acknowledgements, which are the reference's board
/// without its console port. Every entry is current (none stale), but the
/// console's pins add fields to acknowledged registers (`GPIOA.MODER`'s
/// `MODER9`, `MODER10`, `GPIOA.OSPEEDR`'s `OSPEED9`, `OSPEED10`) beside the
/// registers no entry names (`GPIOA.AFRH`, the USART's): each is named.
#[test]
fn a_new_field_in_an_acknowledged_register_is_refused_on_the_fab_path() {
    let four: Value = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/four-layer.json")).unwrap(),
    )
    .unwrap();
    let mut spec = reference();
    spec["acknowledge_unwitnessed"] = four["acknowledge_unwitnessed"].clone();
    let (code, v, out) = rebased_verify("new-field", &spec, "--fab-out");
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "UnwitnessedFieldNotAcknowledged", "{v}");
    let m = v["error"]["message"].as_str().unwrap();
    for id in [
        "GPIOA.MODER.MODER9",
        "GPIOA.MODER.MODER10",
        "GPIOA.OSPEEDR.OSPEED9",
        "GPIOA.OSPEEDR.OSPEED10",
        "GPIOA.AFRH.AFSEL9",
        "USART1.CR2.STOP",
    ] {
        assert!(m.contains(id), "{id}: {m}");
    }
    // 47 more unwitnessed rows than the 4-layer board's 117.
    assert!(m.contains("47 unwitnessed ledger row(s)"), "{m}");
    assert!(!m.contains("GPIOA.MODER.MODER0"), "{m}");
    assert!(!out.exists());
}

/// The reference, acknowledged, passes the check: with `--against` an empty
/// directory the run reaches the tree comparison and fails there, writing
/// nothing, so no package precondition before it refused. Under `--fab-out`
/// on the ready fixture (`AET-WO-4300k` (T)), the readiness gate included;
/// under `--against-fab` on the reference itself, which it does not judge.
#[test]
fn the_acknowledged_reference_passes_the_check_before_any_gate() {
    let dir = scratch("acknowledged");
    let empty = dir.join("empty");
    fs::create_dir_all(&empty).unwrap();
    let out = dir.join("pkg");
    let (code, v) = ready_run(&["--against", s(&empty), "--fab-out", s(&out)]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["against"], "differs", "{v}");
    assert_eq!(v["fab"]["status"], "not_written", "{v}");
    assert_eq!(v["ledger"]["counts"]["unacknowledged"], 0, "{v}");
    assert!(!out.exists());
    let pkg = dir.join("committed");
    fs::create_dir_all(&pkg).unwrap();
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        SCENARIOS[4],
        "--against",
        s(&empty),
        "--against-fab",
        s(&pkg),
    ]);
    assert_eq!(code, Some(1), "{v}");
    assert_eq!(v["against"], "differs", "{v}");
    assert_eq!(v["fab"]["status"], "not_derived", "{v}");
    assert_eq!(v["ledger"]["counts"]["unacknowledged"], 0, "{v}");
    fs::remove_dir_all(&dir).unwrap();
}

/// `AET-WO-4300k` (1), (c) as ruled: `--fab-out` on a specification the
/// readiness gate refuses names the first gap (the reference declares no
/// `fab_tolerances`, `AET-WO-4300k` (2)), exit 2, after every other refusal
/// before a gate and before `--against`, and creates nothing.
#[test]
fn fab_out_on_a_specification_not_ready_is_refused_by_the_key() {
    let dir = scratch("not-ready");
    let empty = dir.join("empty");
    fs::create_dir_all(&empty).unwrap();
    let out = dir.join("pkg");
    let (code, v) = run(&[
        "verify",
        "--spec",
        SPEC,
        "--scenarios",
        SCENARIOS[0],
        SCENARIOS[1],
        SCENARIOS[2],
        SCENARIOS[3],
        SCENARIOS[4],
        "--against",
        s(&empty),
        "--fab-out",
        s(&out),
    ]);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "FabricationNotReady", "{v}");
    assert_eq!(v["error"]["id"], "board.stackup.fab_tolerances", "{v}");
    assert!(!out.exists());
    fs::remove_dir_all(&dir).unwrap();
}

/// `AET-WO-4300k`, ruled 11:25Z (R): `--rebind` needs `--fab-out` (usage,
/// 64); a directory that holds no committed package (`RebindNoPackage`) and
/// another board's package (`RebindOtherBoard`) are refused before any
/// gate, exit 3, creating nothing. The reference re-binding to the bench's
/// committed package is the second; the committed package re-bound to its own
/// board passes these checks and the readiness gate it does not judge,
/// reaching the first gate's tool.
#[test]
fn a_rebind_needs_fab_out_a_committed_package_and_its_board() {
    let dir = scratch("rebind");
    let out = dir.join("pkg");
    let committed = "fab/sentinel-01-bench/package";
    let args = |spec: &'static str, scenarios: [&'static str; 5], rest: Vec<String>| {
        let mut a: Vec<String> = ["verify", "--spec", spec, "--scenarios"]
            .map(str::to_owned)
            .to_vec();
        a.extend(scenarios.map(str::to_owned));
        a.extend(rest);
        a
    };
    let call = |a: Vec<String>| {
        let r: Vec<&str> = a.iter().map(String::as_str).collect();
        run(&r)
    };
    let (code, v) = call(args(
        SPEC,
        SCENARIOS,
        vec!["--rebind".into(), committed.into()],
    ));
    assert_eq!(code, Some(64), "{v}");
    let (code, v) = call(args(
        SPEC,
        SCENARIOS,
        vec![
            "--fab-out".into(),
            s(&out).into(),
            "--rebind".into(),
            "scenarios/sentinel-01".into(),
        ],
    ));
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "RebindNoPackage", "{v}");
    let (code, v) = call(args(
        SPEC,
        SCENARIOS,
        vec![
            "--fab-out".into(),
            s(&out).into(),
            "--rebind".into(),
            committed.into(),
        ],
    ));
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "RebindOtherBoard", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("is the package of `sentinel-01-bench`, not of `sentinel-01`"),
        "{v}"
    );
    assert!(!out.exists());
    let (code, v) = call(args(
        "examples/sentinel-01.json",
        BENCH_SCENARIOS,
        vec![
            "--fab-out".into(),
            s(&out).into(),
            "--rebind".into(),
            committed.into(),
        ],
    ));
    assert_eq!(code, Some(4), "{v}");
    assert_eq!(v["error"]["kind"], "ToolMissing", "{v}");
    assert!(!out.exists());
    let _ = fs::remove_dir_all(&dir);
}

/// The bench's scenarios, in the order its MANIFEST records them (`ci.yml`).
const BENCH_SCENARIOS: [&str; 5] = [
    "scenarios/sentinel-01-bench/pullup-3k3.json",
    "scenarios/sentinel-01-bench/droop-bor3.json",
    "scenarios/sentinel-01-bench/spi-absent-u4.json",
    "scenarios/sentinel-01-bench/pullup-2k2.json",
    "scenarios/sentinel-01-bench/serial-rx-open-console.json",
];

/// `AET-WO-4300k` (R): the tool cannot tell a committed package from any
/// directory an earlier `--fab-out` of the same board wrote, so CI is the
/// control: `--rebind` appears in `ci.yml` exactly once, pinned to
/// Sentinel-01's committed package (the gate review's F2).
#[test]
fn ci_rebinds_only_sentinel_01_s_committed_package() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let uses: Vec<&str> = ci
        .lines()
        .filter(|l| l.contains("--rebind") && !l.trim_start().starts_with('#'))
        .collect();
    assert_eq!(uses.len(), 1, "{uses:?}");
    assert!(
        uses[0]
            .trim_end()
            .ends_with("--rebind fab/sentinel-01-bench/package"),
        "{uses:?}"
    );
}

/// The text of `job`'s block in `ci.yml`: from its `  job:` line to the next
/// job's.
fn ci_job(ci: &str, job: &str) -> String {
    let start = ci
        .find(&format!("\n  {job}:\n"))
        .unwrap_or_else(|| panic!("job {job}"));
    let rest = &ci[start + 1..];
    let end = rest[1..]
        .find("\n  ")
        .map(|i| {
            // the next line indented exactly two spaces and starting a key
            let mut at = i + 1;
            loop {
                let line = &rest[at + 1..];
                if line.starts_with("  ") && !line.starts_with("   ") && !line.starts_with("  #") {
                    break at;
                }
                match rest[at + 1..].find("\n  ") {
                    Some(n) => at += n + 1,
                    None => break rest.len(),
                }
            }
        })
        .unwrap_or(rest.len());
    rest[..end].to_owned()
}

/// `AET-WO-4300k` (8), ruled 16:50Z (a): Sentinel-03's package job runs its
/// verify on one specification, `--against-fab` on the committed package and
/// `--fab-out` into its own artifact's path, each once, and no other job
/// names Sentinel-03's package.
#[test]
fn ci_derives_sentinel_03_s_package_in_its_own_job_on_its_one_path() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let job = ci_job(&ci, "bench-package-s03");
    assert!(job.contains("timeout-minutes: 90"), "{job}");
    assert!(
        job.contains("target/debug/aethelgard verify --spec examples/sentinel-03.json"),
        "{job}"
    );
    let uses = |needle: &str| {
        ci.lines()
            .filter(|l| !l.trim_start().starts_with('#') && l.contains(needle))
            .count()
    };
    for line in [
        "$VERIFY_S03 --against-fab fab/sentinel-03/package",
        "$VERIFY_S03 --fab-out target/sentinel-03-package",
    ] {
        assert_eq!(uses(line), 1, "{line}");
        assert!(job.contains(line), "{line} is in bench-package-s03");
    }
    assert_eq!(uses("fab/sentinel-03/package"), 1);
    assert_eq!(uses("examples/sentinel-03.json"), 1);
}

/// `AET-WO-4300k`, ruled 16:50Z (a): the required `tools` check counts
/// Sentinel-03's package job. Its "the routing decided" step, run with each
/// planted result: on a code pull request anything but `success` (skipped,
/// cancelled, failed, or missing) is red, as a shard's is; on a docs-only
/// one it must be skipped.
#[test]
fn the_tools_check_fails_a_code_pull_request_without_sentinel_03_s_package_job() {
    let ci = fs::read_to_string(root().join(".github/workflows/ci.yml")).unwrap();
    let tools = ci_job(&ci, "tools");
    assert!(
        tools.contains("needs: [route, tools-build, tools-shard, bench-package-s03]"),
        "{tools}"
    );
    let at = tools.find("- name: the routing decided").unwrap();
    let body: Vec<&str> = tools[at..]
        .lines()
        .skip(2)
        .take_while(|l| l.starts_with("          ") || l.trim().is_empty())
        .map(|l| l.get(10..).unwrap_or(""))
        .collect();
    let script = body.join("\n");
    let decide = |class: &str, build: &str, shard: &str, s03: &str| {
        let s = script
            .replace("${{ needs.route.result }}", "success")
            .replace("${{ needs.tools-build.result }}", build)
            .replace("${{ needs.tools-shard.result }}", shard)
            .replace("${{ needs.bench-package-s03.result }}", s03);
        assert!(!s.contains("${{"), "{s}");
        Command::new("bash")
            .args(["-e", "-c", &s])
            .env("CLASS", class)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    };
    assert!(decide("code", "success", "success", "success"));
    for planted in ["skipped", "cancelled", "failure", ""] {
        assert!(
            !decide("code", "success", "success", planted),
            "{planted:?}"
        );
    }
    assert!(!decide("code", "success", "failure", "success"));
    assert!(decide("docs", "skipped", "skipped", "skipped"));
    assert!(!decide("docs", "skipped", "skipped", "success"));
}

/// `AET-WO-1918` (3), ruled (ii): `against_fab_tier2` writes its package
/// once per binary. One `--fab-out` call writes it, inside `package_copy`;
/// every test reads it through `package_copy`; and `ci/colocate.tsv` keeps
/// the binary in one shard, so no shard writes it a second time.
#[test]
fn against_fab_tier2_writes_its_one_package_once_per_binary() {
    let text = fs::read_to_string(root().join("crates/aethelgard-cli/tests/against_fab_tier2.rs"))
        .unwrap();
    assert_eq!(
        text.matches("\"--fab-out\")").count(),
        1,
        "one --fab-out of the package"
    );
    let copy = text.find("fn package_copy(").unwrap();
    let write = text.find("fab_verify(&dir, \"--fab-out\")").unwrap();
    assert!(write > copy && write < copy + text[copy..].find("\n}\n").unwrap());
    let tests: Vec<&str> = text.split("#[test]\nfn ").skip(1).collect();
    assert_eq!(tests.len(), 4);
    for t in tests {
        let body = &t[..t.find("\n}\n").unwrap()];
        assert!(
            body.contains("package_copy("),
            "{}",
            &t[..t.find('(').unwrap()]
        );
    }
    let colocate = fs::read_to_string(root().join("ci/colocate.tsv")).unwrap();
    assert!(
        colocate
            .lines()
            .any(|l| l.split('\t').next() == Some("tests/against_fab_tier2.rs")),
        "ci/colocate.tsv keeps against_fab_tier2 in one shard"
    );
}
