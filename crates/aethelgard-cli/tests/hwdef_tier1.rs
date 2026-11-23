//! `AET-WO-4400b`: `generate hwdef` end to end on the CI boards, the
//! committed topologies held to what `solve` gives today, the refusals a
//! solved specification can reach, and D2's equality: the board crate's own
//! speed choice equals the speed `solve` stored. No tool runs: each call has
//! an empty `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[path = "common/spec.rs"]
mod spec;

use serde_json::Value;

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

/// The CI boards: (fixture name in `aethelgard-hwdef/tests/fixtures`,
/// specification). `four-port` (`AET-WO-4408`) is the one with SERIAL1–3.
const BOARDS: [(&str, &str); 5] = [
    ("sentinel-01", "examples/sentinel-01-minimal.json"),
    (
        "sentinel-02",
        "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json",
    ),
    (
        "serial-dshot-9600",
        "crates/aethelgard-cli/tests/fixtures/serial-dshot-9600.json",
    ),
    (
        "four-port",
        "crates/aethelgard-cli/tests/fixtures/four-port.json",
    ),
    // AET-WO-1917: Sentinel-01's full specification (`4408` Q5).
    ("sentinel-01-full", "examples/sentinel-01.json"),
];

fn scratch(name: &str) -> PathBuf {
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-4400b-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (Option<i32>, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    (out.status.code(), String::from_utf8(out.stdout).unwrap())
}

fn solve(spec: &str) -> String {
    let (code, out) = run(&["solve", "--file", spec]);
    assert_eq!(code, Some(0), "{out}");
    out
}

/// Writes `topology` and generates from it; returns (exit, report, file).
fn generate(dir: &Path, name: &str, topology: &str) -> (Option<i32>, Value, Option<String>) {
    let t = dir.join(format!("{name}.topology.json"));
    fs::write(&t, topology).unwrap();
    let out = dir.join(format!("{name}.hwdef.dat"));
    let (code, stdout) = run(&[
        "generate",
        "hwdef",
        "--topology",
        t.to_str().unwrap(),
        "--out",
        out.to_str().unwrap(),
    ]);
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        code,
        serde_json::from_str(&stdout).unwrap(),
        fs::read_to_string(&out).ok(),
    )
}

/// Each board's topology is exactly the committed fixture, and the file the
/// CLI writes is exactly the committed one, with its sha256 in the report.
#[test]
fn each_ci_board_generates_its_committed_file() {
    let dir = scratch("boards");
    for (name, spec) in BOARDS {
        let fixtures = root().join("crates/aethelgard-hwdef/tests/fixtures");
        let topology = solve(spec);
        assert_eq!(
            topology,
            fs::read_to_string(fixtures.join(format!("{name}.topology.json"))).unwrap(),
            "{name}: the committed topology is stale; regenerate it with `aethelgard solve`"
        );
        let (code, v, text) = generate(&dir, name, &topology);
        assert_eq!(code, Some(0), "{v}");
        let text = text.unwrap();
        assert_eq!(
            text,
            fs::read_to_string(fixtures.join(format!("{name}.hwdef.dat"))).unwrap(),
            "{name}"
        );
        assert_eq!(v["ok"], true);
        assert_eq!(v["hwdef_sha256"].as_str().unwrap().len(), 64);
        assert!(
            v["not_emitted"]
                .as_array()
                .unwrap()
                .iter()
                .any(|n| n.as_str().unwrap().starts_with("U3 (TMP117)"))
        );
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// The file is never overwritten.
#[test]
fn an_existing_output_is_refused() {
    let dir = scratch("exists");
    let topology = solve(BOARDS[0].1);
    fs::write(dir.join("x.hwdef.dat"), "keep").unwrap();
    let (code, v, text) = generate(&dir, "x", &topology);
    assert_eq!(code, Some(3), "{v}");
    assert_eq!(v["error"]["kind"], "UnwritableOutput");
    assert_eq!(text.as_deref(), Some("keep"));
    fs::remove_dir_all(&dir).unwrap();
}

/// D1, D6: what a solved specification can reach, each refused by name with
/// exit 2 and no file: four serial ports with no console, and an SPI bus
/// below ArduPilot's slowest rate on its cited 80 MHz kernel (312.5 kHz):
/// the solver reaches 250 kHz on HSI, ArduPilot's divider stops at 312.5 kHz.
#[test]
fn what_ardupilot_cannot_run_is_refused_by_name() {
    let dir = scratch("refused");
    let topology = solve("crates/aethelgard-cli/tests/fixtures/serial-four.json");
    let (code, v, text) = generate(&dir, "four", &topology);
    assert_eq!(
        (code, v["error"]["kind"].as_str()),
        (Some(2), Some("NoConsole")),
        "{v}"
    );
    assert!(text.is_none());

    // Adversarial A5: CRSF and SBUS both become RC input, and ArduPilot
    // attaches one RCIN port and drops the next (AP_SerialManager.cpp:536-541).
    let mut four: Value = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/serial-four.json")).unwrap(),
    )
    .unwrap();
    for p in four["serial_ports"].as_array_mut().unwrap() {
        if p["role"] == "mavlink" {
            p["role"] = "console".into();
        }
    }
    let path = dir.join("four-console.json");
    fs::write(&path, four.to_string()).unwrap();
    let (code, v, text) = generate(&dir, "rc", &solve(path.to_str().unwrap()));
    assert_eq!(
        (code, v["error"]["kind"].as_str()),
        (Some(2), Some("RcInputTwice")),
        "{v}"
    );
    assert!(text.is_none());

    let mut spec: Value =
        serde_json::from_slice(&fs::read(root().join(BOARDS[0].1)).unwrap()).unwrap();
    spec["spi_buses"][0]["speed_hz"] = 300_000.into();
    let path = dir.join("slow.json");
    fs::write(&path, spec.to_string()).unwrap();
    let topology = solve(path.to_str().unwrap());
    let t: Value = serde_json::from_str(&topology).unwrap();
    assert_eq!(t["spi_controllers"]["SPI1"]["achieved_hz"], 250_000);
    let (code, v, text) = generate(&dir, "slow", &topology);
    assert_eq!(
        (code, v["error"]["kind"].as_str()),
        (Some(2), Some("SpiBelowArduPilotDivider")),
        "{v}"
    );
    assert_eq!(v["error"]["id"], "SPI1");
    assert!(text.is_none());
    fs::remove_dir_all(&dir).unwrap();
}

/// D2: for every committed specification that solves, the board crate
/// generator's own speed for each pin (`svd-hal`'s `pin_speeds`) equals the
/// one `solve` stored, which `generate hwdef` and `check pinout` read.
#[test]
fn the_board_crate_s_speeds_equal_the_stored_ones() {
    let mut checked = 0;
    let dirs = ["examples", "crates/aethelgard-cli/tests/fixtures"];
    for dir in dirs {
        for entry in fs::read_dir(root().join(dir)).unwrap() {
            let path = entry.unwrap().path();
            if path.extension().is_none_or(|e| e != "json") {
                continue;
            }
            let Some(spec) = spec::committed(&path, &fs::read(&path).unwrap()) else {
                continue;
            };
            let Ok(table) = aethelgard_svd_hal::pin_table(&spec.mcu.part) else {
                continue;
            };
            let Ok(t) = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)) else {
                continue;
            };
            let chosen: std::collections::BTreeMap<String, u8> =
                aethelgard_svd_hal::board::pin_speeds(&t, &table)
                    .unwrap()
                    .into_iter()
                    .map(|p| (p.net, p.ospeedr))
                    .collect();
            assert_eq!(chosen, t.pin_speeds, "{}", path.display());
            checked += 1;
        }
    }
    assert!(checked >= 12, "only {checked} specifications solved");
}

/// F3 (ruled on PR #189): an RC-input port takes no `DEFAULT_SERIALn_BAUD`
/// or `_OPTIONS`, since ArduPilot's RC input sets its UART itself; the header
/// names the declared baud and the one ArduPilot runs (CRSF 416666,
/// `AP_RCProtocol.cpp:351`). Every other port keeps its baud.
#[test]
fn an_rc_port_takes_no_baud_and_the_header_names_what_runs() {
    let dir = scratch("rc");
    let mut four: Value = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/serial-four.json")).unwrap(),
    )
    .unwrap();
    for p in four["serial_ports"].as_array_mut().unwrap() {
        match p["role"].as_str().unwrap() {
            "mavlink" => p["role"] = "console".into(),
            // One RC port only: SBUS becomes a plain 8N1 port.
            "rc_sbus" => {
                p["role"] = "gps".into();
                p["framing"] = serde_json::json!({"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false});
            }
            _ => {}
        }
    }
    let path = dir.join("one-rc.json");
    fs::write(&path, four.to_string()).unwrap();
    let (code, v, text) = generate(&dir, "one-rc", &solve(path.to_str().unwrap()));
    assert_eq!(code, Some(0), "{v}");
    let text = text.unwrap();
    let rc = text
        .lines()
        .find(|l| l.starts_with("# Not emitted: CRSF (rc_crsf) on SERIAL"))
        .unwrap_or_else(|| panic!("{text}"));
    assert!(rc.contains("runs CRSF at 416666 baud"), "{rc}");
    assert!(rc.contains("not the declared 420000 baud"), "{rc}");
    let n = &rc["# Not emitted: CRSF (rc_crsf) on SERIAL".len()..][..1];
    assert!(text.contains(&format!(
        "define DEFAULT_SERIAL{n}_PROTOCOL SerialProtocol_RCIN"
    )));
    assert!(
        !text.contains(&format!("define DEFAULT_SERIAL{n}_BAUD")),
        "{text}"
    );
    assert!(
        !text.contains(&format!("define DEFAULT_SERIAL{n}_OPTIONS")),
        "{text}"
    );
    assert_eq!(
        text.lines()
            .filter(|l| l.starts_with("define DEFAULT_SERIAL") && l.contains("_BAUD "))
            .count(),
        3,
        "{text}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// `AET-WO-4404` (§7.1 L760): through `generate hwdef`, a chip select named
/// with each of ArduPilot's alternate-function prefixes
/// (`chibios_hwdef.py:33`) is refused by name with exit 2 and no file; one
/// in lower case generates, as ArduPilot builds it (FEEDBACK §2.3).
#[test]
fn a_chip_select_named_like_an_alternate_function_is_refused() {
    let dir = scratch("af-label");
    let topology = solve(BOARDS[0].1);
    assert!(topology.contains("\"FLASH_CS\""));
    for prefix in [
        "USART", "UART", "SPI", "I2C", "SDIO", "SDMMC", "OTG", "JT", "TIM", "CAN", "QUADSPI",
        "OCTOSPI", "ETH", "MCO",
    ] {
        let net = format!("{prefix}_FLASH_CS");
        let (code, v, text) = generate(
            &dir,
            prefix,
            &topology.replace("\"FLASH_CS\"", &format!("\"{net}\"")),
        );
        assert_eq!(
            (code, v["error"]["kind"].as_str(), v["error"]["id"].as_str()),
            (Some(2), Some("LabelTakesAfPrefix"), Some(net.as_str())),
            "{v}"
        );
        assert!(text.is_none(), "{prefix}");
    }
    let (code, v, text) = generate(
        &dir,
        "lower",
        &topology.replace("\"FLASH_CS\"", "\"spi_cs\""),
    );
    assert_eq!(code, Some(0), "{v}");
    assert!(text.unwrap().contains("PA0 spi_cs CS "));
    fs::remove_dir_all(&dir).unwrap();
}

/// `AET-WO-4404`: eight ports reach SERIAL7, whose options ArduPilot never
/// reads (`AP_SerialManager.cpp:357`), so none is emitted and the header
/// names the omission; with any SERIAL1 or later, the header names the
/// pull-ups `set_options` adds at run time. Sentinel-01, a console only,
/// carries neither sentence.
#[test]
fn serial7_takes_no_options_and_the_header_names_the_pulls() {
    let dir = scratch("eight");
    let mut spec: Value = serde_json::from_slice(
        &fs::read(root().join("crates/aethelgard-cli/tests/fixtures/serial-four.json")).unwrap(),
    )
    .unwrap();
    let ports = spec["serial_ports"].as_array_mut().unwrap();
    for p in ports.iter_mut() {
        match p["role"].as_str().unwrap() {
            "mavlink" => p["role"] = "console".into(),
            "rc_sbus" => {
                p["role"] = "gps".into();
                p["framing"] = serde_json::json!({"baud": 115200, "data_bits": 8, "parity": "none", "stop_bits": 1, "inverted": false});
            }
            _ => {}
        }
    }
    let gps = ports.iter().find(|p| p["role"] == "gps").unwrap().clone();
    for i in 0..4 {
        let mut p = gps.clone();
        p["label"] = format!("EXTRA{i}").into();
        p["tx"] = serde_json::json!({"net": format!("EXTRA{i}_TX"), "pin": null});
        p["rx"] = serde_json::json!({"net": format!("EXTRA{i}_RX"), "pin": null});
        ports.push(p);
    }
    // Each TX and RX ends at a connector pin (AET-WO-4300b): J1's shape.
    let connectors = spec["connectors"].as_array_mut().unwrap();
    let j1 = connectors
        .iter()
        .find(|c| c["reference"] == "J1")
        .unwrap()
        .clone();
    for i in 0..4 {
        let mut c = j1.clone();
        c["reference"] = format!("J{}", 10 + i).into();
        c["pins"][1]["net"] = format!("EXTRA{i}_TX").into();
        c["pins"][2]["net"] = format!("EXTRA{i}_RX").into();
        connectors.push(c);
    }
    let path = dir.join("eight.json");
    fs::write(&path, spec.to_string()).unwrap();
    let (code, v, text) = generate(&dir, "eight", &solve(path.to_str().unwrap()));
    assert_eq!(code, Some(0), "{v}");
    let text = text.unwrap();
    assert!(
        text.contains("\nSERIAL_ORDER USART1 USART2 USART3 UART4 UART5 USART6 UART7 UART8 "),
        "{text}"
    );
    assert!(
        text.contains("\ndefine DEFAULT_SERIAL7_PROTOCOL SerialProtocol_GPS "),
        "{text}"
    );
    assert!(
        text.contains("\ndefine DEFAULT_SERIAL7_BAUD 115200 "),
        "{text}"
    );
    assert!(!text.contains("DEFAULT_SERIAL7_OPTIONS 0"), "{text}");
    assert!(text.contains("\n# Not emitted: DEFAULT_SERIAL7_OPTIONS for EXTRA3: ArduPilot reads a literal 0 for SERIAL7_OPTIONS (AP_SerialManager.cpp:357).\n"), "{text}");
    assert!(
        text.contains("\ndefine DEFAULT_SERIAL6_OPTIONS 0 "),
        "{text}"
    );
    let pulls = "# Serial pulls: FLOATING holds from reset";
    assert!(text.contains(pulls), "{text}");
    let console_only = fs::read_to_string(
        root().join("crates/aethelgard-hwdef/tests/fixtures/sentinel-01.hwdef.dat"),
    )
    .unwrap();
    assert!(!console_only.contains(pulls) && !console_only.contains("_OPTIONS"));
    fs::remove_dir_all(&dir).unwrap();
}
