//! `AET-WO-1303b`: `extract`'s supply gate, through the CLI. A domain the
//! specification gives a drop limit is judged from the routed board: each
//! MCU supply and return pin's resistance bounded from the regulator, the
//! drop against the limit, every input it reads refused by name when absent,
//! and nothing about a specification without a limit moving, but a declared
//! coupled pair's `pairs` key (D2), which every specification with
//! `diff_pairs` gains.

#![allow(clippy::unwrap_used)]

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use serde_json::{Value, json};

const S03: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-03.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1303b-{label}-{}", std::process::id()));
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

/// The gate's inputs, as a specification declares them.
fn gated(mut v: Value, limit_mv: u32) -> Value {
    let st = &mut v["board"]["stackup"];
    st["outer_copper_min_um"] =
        json!({"value": 30, "source": "test: 1 oz foil plus plating, at least"});
    st["plane_copper_min_um"] = json!({"value": 15, "source": "test: 1/2 oz foil, at least"});
    st["plating_min_um"] =
        json!({"value": 18, "source": "test: the fabricator's average hole plating"});
    st["copper_temperature_max_c"] =
        json!({"value": 125, "source": "test: the load's junction maximum"});
    v["power"]["drop_limits"] =
        json!([{"domain": "VDD_3V3", "max_millivolts": limit_mv, "source": "test: the limit"}]);
    v
}

fn write(label: &str, v: &Value) -> PathBuf {
    let path = scratch(label).join("spec.json");
    fs::write(&path, serde_json::to_string_pretty(v).unwrap()).unwrap();
    path
}

/// The board `emit` writes for `spec`, once per specification file.
fn emitted(label: &str, spec: &Path) -> PathBuf {
    let dir = scratch(&format!("{label}-board"));
    let (code, v) = run(&[
        "emit",
        "--file",
        spec.to_str().unwrap(),
        "--out",
        dir.to_str().unwrap(),
    ]);
    assert_eq!(code, 0, "{v}");
    fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.extension().is_some_and(|x| x == "kicad_pcb"))
        .unwrap()
}

/// Sentinel-03's board, emitted once: the gate's inputs never change it.
fn s03_board() -> &'static Path {
    static BOARD: OnceLock<PathBuf> = OnceLock::new();
    BOARD.get_or_init(|| emitted("s03", &root().join(S03)))
}

fn s03() -> Value {
    serde_json::from_str(&fs::read_to_string(root().join(S03)).unwrap()).unwrap()
}

fn extract(board: &Path, spec: &Path) -> (i32, Value) {
    run(&[
        "extract",
        "--board",
        board.to_str().unwrap(),
        "--spec",
        spec.to_str().unwrap(),
    ])
}

fn pins(s: &Value, side: &str) -> Vec<Value> {
    s[side].as_array().unwrap().clone()
}

/// A specification with no drop limit is not judged: no `supplies` key,
/// so its verdict's bytes are as before.
#[test]
fn a_specification_without_a_drop_limit_has_no_supplies() {
    let (code, v) = extract(s03_board(), &root().join(S03));
    assert_eq!(code, 0, "{v}");
    assert!(v.get("supplies").is_none(), "{v}");
}

/// Every input the gate reads is refused by name when the specification
/// limits a domain and lacks it: never defaulted (`RUL-14`).
#[test]
fn each_input_the_gate_reads_is_refused_by_name_when_absent() {
    for key in [
        "outer_copper_min_um",
        "plane_copper_min_um",
        "plating_min_um",
        "copper_temperature_max_c",
    ] {
        let mut v = gated(s03(), 165);
        v["board"]["stackup"].as_object_mut().unwrap().remove(key);
        let (code, r) = extract(s03_board(), &write(key, &v));
        assert_eq!(code, 2, "{key}: {r}");
        assert_eq!(r["error"]["kind"], "SupplyInputUndeclared", "{key}: {r}");
        assert!(
            r["error"]["message"].as_str().unwrap().contains(key),
            "{key}: {r}"
        );
    }
    // every part on the domain that draws current is named by a load
    // entry: the flash's removed, the gate names it
    let mut v = gated(s03(), 165);
    v["power"]["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U4");
    let (code, r) = extract(s03_board(), &write("load", &v));
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "SupplySinkUncounted", "{r}");
    assert!(
        r["error"]["message"].as_str().unwrap().starts_with("U4 "),
        "{r}"
    );
}

/// A limit on a domain the MCU has no pin on judges nothing, so it is
/// refused by name rather than passed.
#[test]
fn a_limit_on_a_domain_the_load_does_not_use_is_refused() {
    let mut v = gated(s03(), 165);
    v["power"]["drop_limits"] =
        json!([{"domain": "VDD_5V", "max_millivolts": 250, "source": "test: the limit"}]);
    let (code, r) = extract(s03_board(), &write("five", &v));
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "SupplyTerminalMissing", "{r}");
}

/// The four-layer board: every MCU supply and return pin bounded, from the
/// buck's inductor pad and to its PGND pin, and the drop within 165 mV.
#[test]
fn the_four_layer_board_bounds_every_supply_pin_within_its_limit() {
    let (code, v) = extract(s03_board(), &write("s03-pass", &gated(s03(), 165)));
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    let s = &v["supplies"][0];
    assert_eq!(s["domain"], "VDD_3V3");
    assert_eq!(s["source"], json!({"reference": "L2", "pad": "2"}));
    assert_eq!(s["return"], json!({"reference": "U11", "pad": "1"}));
    // the MCU's 658 mA from its part row and the fixture's 90 mA of other
    // sinks, each cited, derived or declared
    assert_eq!(s["load_milliamps"], 748);
    let (vdd, vss) = (pins(s, "pins"), pins(s, "returns"));
    assert_eq!((vdd.len(), vss.len()), (16, 10), "{s}");
    for p in vdd.iter().chain(&vss) {
        let r = p["r_bound_microohms"]
            .as_u64()
            .unwrap_or_else(|| panic!("{p}"));
        assert!((10_000..80_000).contains(&r), "{p}");
    }
    let worst = |v: &[Value]| {
        v.iter()
            .map(|p| p["r_bound_microohms"].as_u64().unwrap())
            .max()
            .unwrap()
    };
    let drop = s["drop_bound_microvolts"].as_u64().unwrap();
    assert_eq!(drop, ((worst(&vdd) + worst(&vss)) * 748).div_ceil(1000));
    assert!(drop <= 165_000, "{drop}");
    assert!(s.get("note").is_none(), "{s}");
    let conditions: Vec<&str> = s["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    assert!(
        conditions.iter().any(|c| c.contains("+-0.075 mm per axis")),
        "{conditions:?}"
    );
    assert!(
        conditions
            .iter()
            .any(|c| c.contains("other domains' return currents")),
        "{conditions:?}"
    );
}

/// The same board against a limit its bound exceeds: a finding, and the
/// note that the pitch set the margin.
#[test]
fn a_drop_over_its_limit_fails_with_the_pitch_note() {
    let (code, v) = extract(s03_board(), &write("s03-fail", &gated(s03(), 60)));
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "fail", "{v}");
    let s = &v["supplies"][0];
    assert_eq!(s["verdict"], "fail");
    assert!(s["note"].as_str().unwrap().contains("pitch"), "{s}");
    let kinds: Vec<&str> = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["kind"].as_str().unwrap())
        .collect();
    assert_eq!(kinds, ["supply_drop_over_limit"], "{v}");
}

/// The two-layer power fixture: its supply pins bounded, its return refused
/// by name (`GroundReturnUnread`, ruling (iii)): the ground runs through a
/// zone the emitted board leaves unfilled. No drop, and a finding per pin.
#[test]
fn the_two_layer_power_fixture_refuses_its_ground_return_by_name() {
    let spec = write("s02", &gated(common::power::power_tree(), 165));
    let board = emitted("s02", &spec);
    let (code, v) = extract(&board, &spec);
    assert_eq!(code, 1, "{v}");
    let s = &v["supplies"][0];
    assert!(s["drop_bound_microvolts"].is_null(), "{s}");
    let vdd = pins(s, "pins");
    assert_eq!(vdd.len(), 16);
    assert!(vdd.iter().all(|p| p["r_bound_microohms"].is_u64()), "{s}");
    let vss = pins(s, "returns");
    assert!(!vss.is_empty());
    assert!(
        vss.iter()
            .all(|p| p["refused"]["kind"] == "GroundReturnUnread"),
        "{s}"
    );
    let unbounded = v["findings"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|f| f["kind"] == "supply_pin_unbounded")
        .count();
    assert_eq!(unbounded, vss.len(), "{v}");
}

const PAIR: &str = "crates/aethelgard-cli/tests/fixtures/pair-standin.json";

/// D2 on the pair stand-in: each conductor of a declared pair gets the
/// line's own capacitance bound against its net's declared maximum, and the
/// mismatch is named as unverified. A conductor the extraction does not
/// bound as a line is refused by name.
#[test]
fn a_declared_pair_gets_each_conductors_bound_and_its_mismatch_named() {
    let spec = root().join(PAIR);
    let board = emitted("pair", &spec);
    let (code, v) = extract(&board, &spec);
    assert_eq!(code, 0, "{v}");
    let p = &v["pairs"][0];
    assert_eq!(p["name"], "USB_STANDIN");
    assert!(
        p["mismatch"].as_str().unwrap().starts_with("unverified:"),
        "{p}"
    );
    let conductors = p["conductors"].as_array().unwrap();
    assert_eq!(conductors.len(), 2);
    for c in conductors {
        let line = v["lines"]
            .as_array()
            .unwrap()
            .iter()
            .find(|l| l["net"] == c["net"])
            .unwrap();
        assert_eq!(c["c_bound_femtofarads"], line["c_bound_femtofarads"], "{c}");
        assert_eq!(c["declared_femtofarads"], 30_000, "{c}");
        assert_eq!(c["verdict"], "pass", "{c}");
    }
    let mut s: Value = serde_json::from_str(&fs::read_to_string(&spec).unwrap()).unwrap();
    s["diff_pairs"][0]["nets"] = json!(["SPI_SCK", "SPI_MISO"]);
    let edited = write("pair-miso", &s);
    let (code, r) = extract(&board, &edited);
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "PairConductorUnbounded", "{r}");
}
