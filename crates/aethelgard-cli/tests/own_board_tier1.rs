//! `AET-WO-4220b`, tier 1 (no tool runs): the Sentinel-02-shaped fixture's
//! two sensors on its emitted board, drawn from our own library and wired as
//! their datasheets say, and `extract` counting each one's declared pin
//! capacitance as declared, never as a maximum (§1.2, §5.2). Its ERC, DRC
//! with parity and `extract` in the image are `board_tier2.rs`'s.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::Value;

const SPEC: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json";
const PCB: &str = "sentinel-02-own.kicad_pcb";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-4220b-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

/// The fixture's board, emitted once, at a path that outlives the tests.
fn board() -> &'static Path {
    static PATH: OnceLock<PathBuf> = OnceLock::new();
    PATH.get_or_init(|| {
        let dir = scratch("emit");
        let (code, v) = run(&["emit", "--file", SPEC, "--out", dir.to_str().unwrap()]);
        assert_eq!(code, 0, "{v}");
        dir.join(PCB)
    })
}

fn spec() -> Value {
    serde_json::from_str(&fs::read_to_string(root().join(SPEC)).unwrap()).unwrap()
}

/// The fixture with `edit` applied, written beside the scratch board.
fn spec_with(label: &str, edit: impl Fn(&mut Value)) -> PathBuf {
    let mut v = spec();
    edit(&mut v);
    let path = std::env::temp_dir().join(format!(
        "aet-4220b-spec-{label}-{}.json",
        std::process::id()
    ));
    fs::write(&path, serde_json::to_string_pretty(&v).unwrap()).unwrap();
    path
}

fn extract(spec: &str) -> (i32, Value) {
    run(&[
        "extract",
        "--board",
        board().to_str().unwrap(),
        "--spec",
        spec,
    ])
}

/// Each pad of `reference` on the board, by number, with its net.
fn pads(reference: &str) -> BTreeMap<String, String> {
    let text = fs::read_to_string(board()).unwrap();
    let tree = aethelgard_eda::strict::board(&text).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_value(spec()).unwrap();
    let (b, _) =
        aethelgard_eda::route::read(&tree, &aethelgard_eda::stack::Stack::of(&spec)).unwrap();
    b.parts
        .iter()
        .find(|p| p.reference == reference)
        .unwrap()
        .pads
        .iter()
        .map(|p| (p.number.clone(), p.net.clone().unwrap()))
        .collect()
}

fn want(table: &[(&str, &str)]) -> BTreeMap<String, String> {
    table
        .iter()
        .map(|&(p, n)| (p.to_owned(), n.to_owned()))
        .collect()
}

/// The pads KiCad leaves on no net carry its `unconnected-` prefix; every
/// other is the datasheet's connection.
fn wired(reference: &str) -> BTreeMap<String, String> {
    pads(reference)
        .into_iter()
        .map(|(p, n)| {
            let n = if n.starts_with("unconnected-") {
                "open".into()
            } else {
                n
            };
            (p, n)
        })
        .collect()
}

/// DS-000347 Table 10 and Fig. 7 (4-wire SPI): AP_SDO is MISO, AP_CS,
/// AP_SCLK and AP_SDI the bus's CS, SCK and MOSI; VDD and VDDIO on the
/// supply; GND, RESV pin 7 and FSYNC to GND; RESV 2, 3, 10, 11 and INT1 open.
#[test]
fn the_icm_is_wired_as_ds_000347_says() {
    assert_eq!(
        wired("U5"),
        want(&[
            ("1", "/IMU_MISO"),
            ("2", "open"),
            ("3", "open"),
            ("4", "open"),
            ("5", "VDD_3V3"),
            ("6", "GND"),
            ("7", "GND"),
            ("8", "VDD_3V3"),
            ("9", "GND"),
            ("10", "open"),
            ("11", "open"),
            ("12", "/IMU_CS"),
            ("13", "/IMU_SCK"),
            ("14", "/IMU_MOSI"),
        ])
    );
}

/// BST-BMP581-DS004-13 Table 28 and Fig. 26 (I2C): VDDIO and VDD on the
/// supply; VSS to GND; SCK is SCL and SDI is SDA; CSB to VDDIO; INT to GND;
/// SDO on its own strap, which address 0x46 takes to GND (§5.6, p. 38).
#[test]
fn the_bmp581_is_wired_as_its_datasheet_says() {
    assert_eq!(
        wired("U7"),
        want(&[
            ("1", "VDD_3V3"),
            ("2", "/I2C_SCL"),
            ("3", "GND"),
            ("4", "/I2C_SDA"),
            ("5", "/U7_SDO"),
            ("6", "VDD_3V3"),
            ("7", "GND"),
            ("8", "GND"),
            ("9", "GND"),
            ("10", "VDD_3V3"),
        ])
    );
    let text = fs::read_to_string(board()).unwrap();
    let tree = aethelgard_eda::strict::board(&text).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_value(spec()).unwrap();
    let (b, _) =
        aethelgard_eda::route::read(&tree, &aethelgard_eda::stack::Stack::of(&spec)).unwrap();
    let strap: Vec<_> = b
        .parts
        .iter()
        .filter(|p| p.pads.iter().any(|q| q.net.as_deref() == Some("/U7_SDO")))
        .filter(|p| p.reference != "U7")
        .map(|p| {
            let mut nets: Vec<_> = p.pads.iter().filter_map(|q| q.net.clone()).collect();
            nets.sort();
            (p.reference.chars().next().unwrap(), nets)
        })
        .collect();
    assert_eq!(strap, [('R', vec!["/U7_SDO".to_owned(), "GND".into()])]);
}

fn line<'a>(v: &'a Value, net: &str) -> &'a Value {
    v["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["net"] == net)
        .unwrap_or_else(|| panic!("{net}: {v}"))
}

fn strings(v: &Value) -> Vec<&str> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|s| s.as_str().unwrap())
        .collect()
}

/// §1.2: each declared pin is counted once on each line it sits on, in the
/// line's sum and in its own term, never among `pins` (the maxima); the
/// verdict states each declaring device's value, condition and source, and
/// that it is not a maximum. A line with no declared pin is as before.
#[test]
fn extract_counts_each_declared_pin_as_declared_never_as_a_maximum() {
    let (code, v) = extract(SPEC);
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    for (net, pin) in [
        ("IMU_SCK", "U5.13"),
        ("IMU_MOSI", "U5.14"),
        ("I2C_SDA", "U7.4"),
        ("I2C_SCL", "U7.2"),
    ] {
        let l = line(&v, net);
        assert_eq!(strings(&l["declared_pins"]), [pin], "{l}");
        assert_eq!(l["terms"]["declared_pins"], 10_000, "{l}");
        let declarer = &pin[..2];
        assert!(
            strings(&l["pins"]).iter().all(|p| !p.starts_with(declarer)),
            "{l}"
        );
        let t = &l["terms"];
        let sum: u64 = ["trace", "vias", "pads", "pins", "declared_pins"]
            .iter()
            .map(|k| t[*k].as_u64().unwrap())
            .sum();
        assert_eq!(l["c_bound_femtofarads"], sum, "{l}");
    }
    // The flash's lines cite a maximum and declare nothing: unchanged.
    for net in ["SPI_SCK", "SPI_MOSI"] {
        let l = line(&v, net);
        assert!(l.get("declared_pins").is_none(), "{l}");
        assert!(l["terms"].get("declared_pins").is_none(), "{l}");
        assert_eq!(
            strings(&l["pins"]),
            [format!("U4.{}", if net == "SPI_SCK" { 6 } else { 5 })]
        );
    }
    // The standing I2C and SPI conditions no longer cover U7's and U5's
    // pins (adversarial A2, H1).
    let conditions = strings(&v["conditions"]);
    use aethelgard_eda::extract as x;
    assert!(
        conditions.contains(&x::I2C_PIN_CONDITION_BESIDE_DECLARED)
            && !conditions.contains(&x::I2C_PIN_CONDITION)
            && conditions.contains(&x::SPI_CONDITION_BESIDE_DECLARED)
            && !conditions.contains(&x::SPI_CONDITION),
        "{v}"
    );
    let declared: Vec<&str> = strings(&v["conditions"])
        .into_iter()
        .filter(|c| c.contains("a value the specification declares"))
        .collect();
    assert_eq!(declared.len(), 2, "{v}");
    for (c, who, source) in [
        (declared[0], "U7 (BMP581): ", "UM10204 rev. 7.0 Table 10"),
        (
            declared[1],
            "U5 (ICM42688P): ",
            "DS-000347 v1.9, 3.3.2 Table 4",
        ),
    ] {
        assert!(c.starts_with(who), "{c}");
        assert!(c.contains("counted at 10000 fF"), "{c}");
        assert!(c.contains("not a maximum the datasheet prints"), "{c}");
        assert!(
            c.contains("the bound holds only under its condition"),
            "{c}"
        );
        assert!(c.contains(source), "{c}");
    }
}

/// §5.2: without its declaration the ICM, whose datasheet prints a typical
/// only, is refused by name: no typical value stands in for a maximum.
#[test]
fn without_its_declaration_the_icm_is_refused() {
    let spec = spec_with("no-icm", |v| {
        for d in v["spi_devices"].as_array_mut().unwrap() {
            if d["part"] == "ICM42688P" {
                d["pin_capacitance"] = Value::Null;
            }
        }
    });
    let (code, v) = extract(spec.to_str().unwrap());
    assert_eq!(code, 2, "{v}");
    assert_eq!(v["error"]["kind"], "UnboundedPinCapacitance", "{v}");
    assert!(
        v["error"]["message"]
            .as_str()
            .unwrap()
            .contains("U5 (ICM42688P)"),
        "{v}"
    );
    fs::remove_file(spec).unwrap();
}

/// §5.2: without its declaration the BMP581 is counted at UM10204's `Ci`
/// with every other I2C pin, as before `AET-WO-4220b`: in `pins`, with no
/// declared term and no condition of its own. The sum is the same 10 pF.
#[test]
fn without_its_declaration_the_bmp581_is_an_i2c_pin_at_um10204_s_ci() {
    let spec = spec_with("no-bmp", |v| {
        for d in v["i2c_devices"].as_array_mut().unwrap() {
            if d["part"] == "BMP581" {
                d["pin_capacitance"] = Value::Null;
            }
        }
    });
    let (code, v) = extract(spec.to_str().unwrap());
    assert_eq!(code, 0, "{v}");
    let (_, with) = extract(SPEC);
    for net in ["I2C_SDA", "I2C_SCL"] {
        let l = line(&v, net);
        assert!(l.get("declared_pins").is_none(), "{l}");
        assert!(
            strings(&l["pins"]).iter().any(|p| p.starts_with("U7.")),
            "{l}"
        );
        assert_eq!(
            l["c_bound_femtofarads"],
            line(&with, net)["c_bound_femtofarads"]
        );
    }
    let conditions = strings(&v["conditions"]);
    assert!(conditions.iter().all(|c| !c.starts_with("U7 ")), "{v}");
    assert!(
        conditions.contains(&aethelgard_eda::extract::I2C_PIN_CONDITION),
        "{v}"
    );
    fs::remove_file(spec).unwrap();
}
