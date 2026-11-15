//! `AET-WO-4300c` §2 as amended: `check speeds` on the four acceptance
//! boards. Every in-scope line (SCK, MOSI) passes at the slowest speed that
//! serves it; every out-of-scope line is named with its reason; the
//! ICM-20948 fixture, which no board can be drawn for, is labelled
//! unverified. No tool runs: each call has an empty `PATH`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde_json::Value;

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn speeds(spec: &Path) -> (Option<i32>, Value) {
    let run = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["check", "speeds", "--spec"])
        .arg(spec)
        .current_dir(root())
        .env("PATH", "")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(run.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (run.status.code(), serde_json::from_str(&stdout).unwrap())
}

/// Each line by its net: (ospeedr, the judged row's load or the reason).
fn by_net(v: &Value) -> Vec<(String, String, String)> {
    v["lines"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            let what = match l.get("judged") {
                Some(j) => {
                    assert_eq!(j["verdict"], "pass", "{l}");
                    assert!(j["fmax_hz"].as_u64() >= j["rate_hz"].as_u64(), "{l}");
                    format!("{} pF", j["row_picofarads"])
                }
                None => l["out_of_scope"].as_str().unwrap().to_owned(),
            };
            (
                l["net"].as_str().unwrap().to_owned(),
                l["ospeedr"].as_str().unwrap().to_owned(),
                what,
            )
        })
        .collect()
}

fn reason(v: &Value, net: &str) -> String {
    by_net(v).into_iter().find(|l| l.0 == net).unwrap().2
}

#[test]
fn the_reference_drives_sck_and_mosi_at_01_and_names_every_other_line() {
    let (code, v) = speeds(&root().join("examples/sentinel-01-minimal.json"));
    assert_eq!(code, Some(0), "{v}");
    assert_eq!(
        (v["verdict"].as_str(), v["check"].as_str()),
        (Some("pass"), Some("speeds"))
    );
    assert_eq!(
        v["capacitance"],
        "declared (net_constraints); extract bounds it on the routed board"
    );
    let lines = by_net(&v);
    let speed = |net: &str| lines.iter().find(|l| l.0 == net).unwrap().1.clone();
    assert_eq!(speed("SPI_SCK"), "01");
    assert_eq!(speed("SPI_MOSI"), "01");
    assert_eq!(reason(&v, "SPI_SCK"), "30 pF");
    for net in ["SPI_MISO", "FLASH_CS", "I2C_SDA", "I2C_SCL"] {
        assert_eq!(speed(net), "00", "{net}");
    }
    assert_eq!(
        reason(&v, "SPI_MISO"),
        "input in master mode: no electrical effect"
    );
    assert!(reason(&v, "FLASH_CS").starts_with("software-driven GPIO output"));
    assert!(reason(&v, "I2C_SDA").contains("6.3.32"));
    assert_eq!(lines.len(), 6);
    let sck = &v["lines"].as_array().unwrap()[1]["judged"];
    assert_eq!(sck["vdd_min_millivolts"], 3135);
    assert_eq!(sck["band_millivolts"], serde_json::json!([2700, 3600]));
    assert_eq!(sck["fmax_hz"], 80_000_000);
    assert!(v["table"]["rev_v"].as_str().unwrap().contains("Table 149"));
    assert!(v["table"]["rev_y"].as_str().unwrap().contains("Table 52"));
}

#[test]
fn the_four_layer_fixture_and_d1_hold_on_every_in_scope_line() {
    for (fixture, lines) in [("four-layer.json", 6), ("d1-24-i2c.json", 12)] {
        let (code, v) = speeds(
            &root()
                .join("crates/aethelgard-cli/tests/fixtures")
                .join(fixture),
        );
        assert_eq!(code, Some(0), "{fixture}: {v}");
        let all = by_net(&v);
        assert_eq!(all.len(), lines, "{fixture}");
        let judged = all.iter().filter(|l| l.2.ends_with(" pF")).count();
        assert_eq!(judged, 2, "{fixture}: SCK and MOSI only");
        assert!(all.iter().all(|l| l.1 == "00" || l.1 == "01"), "{fixture}");
    }
}

/// Ruled D2: the ICM-20948 fixture's lines are declared and labelled
/// unverified until it has a board. At 7 MHz `00` serves SCK and MOSI.
#[test]
fn the_icm_fixture_is_judged_on_its_declaration_and_labelled_unverified() {
    let (code, v) = speeds(&root().join("crates/aethelgard-cli/tests/fixtures/spi-imu.json"));
    assert_eq!(code, Some(0), "{v}");
    let label = v["capacitance"].as_str().unwrap();
    assert!(
        label.starts_with(
            "declared (net_constraints), UNVERIFIED: no board can be drawn (UnknownPart:"
        ),
        "{label}"
    );
    let all = by_net(&v);
    assert!(all.iter().all(|l| l.1 == "00"), "{all:?}");
    assert_eq!(reason(&v, "SPI_SCK"), "30 pF");
    assert!(reason(&v, "IMU_CS").starts_with("software-driven"));
}

/// §1.4: a bus whose rate needs `10` or `11` is refused by name, exit 2.
#[test]
fn a_rate_no_selectable_speed_serves_is_refused() {
    let mut spec: Value = serde_json::from_slice(
        &fs::read(root().join("examples/sentinel-01-minimal.json")).unwrap(),
    )
    .unwrap();
    spec["spi_buses"][0]["speed_hz"] = 100_000_000.into();
    let dir = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("aet-4300c-speeds-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("fast.json");
    fs::write(&path, spec.to_string()).unwrap();
    let (code, v) = speeds(&path);
    assert_eq!(code, Some(2), "{v}");
    assert_eq!(v["error"]["kind"], "RateAbovePinSpeed", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("100000000 Hz is above 80000000 Hz"),
        "{v}"
    );
    fs::remove_dir_all(&dir).unwrap();
}
