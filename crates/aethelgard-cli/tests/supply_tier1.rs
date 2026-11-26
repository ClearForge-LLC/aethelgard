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

const S03: &str = "examples/sentinel-03.json";

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

/// Sentinel-03's own drop limit (`AET-WO-4300k` D6, ruled at most 110 mV):
/// above 113 mV its supervisor's release would sit at or above the rail's
/// minimum at its pin, which `validate` refuses (`SupervisorAboveRail`).
const S03_LIMIT_MV: u32 = 110;

/// The gate's inputs, as a specification declares them.
/// A bound's `cites` is kept: a stackup that declares a fabricator (Sentinel-03
/// since `AET-WO-4300k` (b)) refuses an uncited bound.
fn gated(mut v: Value, limit_mv: u32) -> Value {
    let st = &mut v["board"]["stackup"];
    for (key, value, source) in [
        (
            "outer_copper_min_um",
            30,
            "test: 1 oz foil plus plating, at least",
        ),
        ("plane_copper_min_um", 15, "test: 1/2 oz foil, at least"),
        (
            "plating_min_um",
            18,
            "test: the fabricator's average hole plating",
        ),
        (
            "copper_temperature_max_c",
            125,
            "test: the load's junction maximum",
        ),
    ] {
        let cites = st[key]["cites"].clone();
        st[key] = json!({"value": value, "source": source});
        if !cites.is_null() {
            st[key]["cites"] = cites;
        }
    }
    // The MCU's supply's limit replaced; any other domain's kept (the radio
    // module's own rail's, `AET-WO-4411`).
    let mut limits =
        vec![json!({"domain": "VDD_3V3", "max_millivolts": limit_mv, "source": "test: the limit"})];
    limits.extend(
        v["power"]["drop_limits"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|l| l["domain"] != "VDD_3V3")
            .cloned(),
    );
    v["power"]["drop_limits"] = Value::Array(limits);
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

/// Sentinel-03's board, emitted once from its own specification, which
/// declares no drop limit. The gate's inputs can change a routing: the
/// router keeps a judged net's vias one junction region apart where the
/// gate runs (`AET-WO-1324`). Sentinel-03 gated routes byte-identical
/// (measured, FEEDBACK), so each test here judges this one board.
fn s03_board() -> &'static Path {
    static BOARD: OnceLock<PathBuf> = OnceLock::new();
    BOARD.get_or_init(|| emitted("s03", &root().join(S03)))
}

fn s03() -> Value {
    serde_json::from_str(&fs::read_to_string(root().join(S03)).unwrap()).unwrap()
}

/// s024l: the power fixture (as the CLI tests take it) at Sentinel-03's
/// four layers and stackup, JLCPCB's tolerances included, as the measured
/// stop built it (`spike/1303b/build-fixtures.py`, at `81a8e619`); ruling 6976355662 (A)
/// gates the priced four-layer drop on it.
fn s024l() -> Value {
    let mut v = common::power::power_tree();
    let s3 = s03();
    for k in ["copper_layers", "layers", "stackup"] {
        v["board"][k] = s3["board"][k].clone();
    }
    v
}

/// s024l's board, emitted once.
fn s024l_board() -> &'static Path {
    static BOARD: OnceLock<PathBuf> = OnceLock::new();
    BOARD.get_or_init(|| emitted("s024l", &write("s024l-spec", &s024l())))
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

/// One part's row of the gate's `ran` table (`AET-WO-1609`).
fn part<'v>(v: &'v Value, gate: &str) -> &'v Value {
    v["ran"]
        .as_array()
        .and_then(|rows| rows.iter().find(|r| r["gate"] == gate))
        .unwrap_or_else(|| panic!("no `{gate}` row: {v}"))
}

fn pins(s: &Value, side: &str) -> Vec<Value> {
    s[side].as_array().unwrap().clone()
}

/// `AET-WO-4202` (ruled (a), 21:05Z): each resistor a `power.loads` row of
/// Sentinel-03 names is the part its row describes, read from the emitted
/// copper: one pad on `VDD_3V3`, the other on the net it pulls up. The
/// emitter numbers straps and pull-ups in the order it draws them, so a part
/// leaving renumbers every later one (U2's SDO strap was R5), and a row left
/// on the old number counts another part; the supply gate refuses only a
/// sink no row names, never a row on the wrong part.
#[test]
fn each_load_row_s_resistor_is_the_part_its_row_describes() {
    // The rows' own descriptions: the I2C pull-ups (3.465 V / 2.2 kOhm, then
    // / 10 kOhm), the flash's WP and HOLD pull-ups (its pins' leakage), and
    // the 3.3 V regulator's feedback divider (3.465 V / (105 + 46.4) kOhm),
    // and the reset supervisor's NRST pull-up (`AET-WO-4300k` (9)).
    let described = [
        ("R1", "/I2C_SDA"),
        ("R2", "/I2C_SCL"),
        ("R3", "/EXT_SDA"),
        ("R4", "/EXT_SCL"),
        ("R6", "/U4_WP"),
        ("R7", "/U4_HOLD"),
        ("R20", "/U11_FB"),
        ("R61", "/NRST"),
        // `AET-WO-4411`: U30's EN and IO2 strap pull-ups; U13's feedback
        // divider, and its PG's pull-up, which U11's EN follows.
        ("R90", "/U30_EN"),
        ("R91", "/U30_IO2"),
        ("R30", "/U13_FB"),
        ("R32", "/U13_PG"),
    ];
    let rows: Vec<String> = s03()["power"]["loads"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|l| l["reference"].as_str())
        .filter(|r| r.starts_with('R'))
        .map(str::to_owned)
        .collect();
    assert_eq!(rows, described.map(|(r, _)| r));
    let text = fs::read_to_string(s03_board()).unwrap();
    let tree = aethelgard_eda::strict::board(&text).unwrap();
    for (reference, net) in described {
        let fp = tree
            .root()
            .all("footprint")
            .find(|fp| fp.field("Reference").unwrap() == Some(reference))
            .unwrap_or_else(|| panic!("{reference} is on the board"));
        let mut nets: Vec<&str> = fp
            .all("pad")
            .filter_map(|pad| pad.one("net").unwrap().and_then(|n| n.last()))
            .collect();
        nets.sort_unstable();
        assert_eq!(nets, [net, "VDD_3V3"], "{reference}");
    }
}

/// A specification with no drop limit is not judged: no `supplies` key,
/// so its verdict's bytes are as before. Since `AET-WO-4300k` (2) a power
/// tree requires its MCU supply's limit, so that is a board with no tree
/// (sentinel-01-minimal), and Sentinel-03 without its limit is refused by
/// name before any line is judged.
#[test]
fn a_specification_without_a_drop_limit_has_no_supplies() {
    let spec = root().join("examples/sentinel-01-minimal.json");
    let board = emitted("s01-nolimit", &spec);
    let (code, v) = extract(&board, &spec);
    assert_eq!(code, 0, "{v}");
    assert!(v.get("supplies").is_none(), "{v}");
    // ...and the table says so, by the key that would turn it on
    // (`AET-WO-1609` §2).
    assert_eq!(
        part(&v, "extract.supply"),
        &json!({"gate": "extract.supply", "off": "power.drop_limits"})
    );
    let lines = v["lines"].as_array().unwrap().len() as u64;
    assert_eq!(part(&v, "extract.lines")["judged"], lines, "{v}");
    assert_eq!(part(&v, "extract.pairs")["off"], "diff_pairs");
    assert_eq!(part(&v, "extract.oscillator")["off"], "mcu.hse.crystal");
    let mut s = s03();
    s["power"]["drop_limits"] = json!([]);
    let (code, r) = extract(s03_board(), &write("s03-nolimit", &s));
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "DropLimitUndeclared", "{r}");
    assert_eq!(r["error"]["id"], "VDD_3V3", "{r}");
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
        let mut v = gated(s03(), S03_LIMIT_MV);
        v["board"]["stackup"].as_object_mut().unwrap().remove(key);
        let (code, r) = extract(s03_board(), &write(key, &v));
        assert_eq!(code, 2, "{key}: {r}");
        assert_eq!(r["error"]["kind"], "SupplyInputUndeclared", "{key}: {r}");
        assert!(
            r["error"]["message"].as_str().unwrap().contains(key),
            "{key}: {r}"
        );
    }
    // the gate's own tolerances; the via ring rule's (position, the larger
    // size, the ring) are refused at `emit`, before any board exists
    // (`via_ring_tier1`, `AET-WO-1314`)
    for key in ["hole_size_minus_um", "track_width_permille"] {
        let mut v = gated(s03(), S03_LIMIT_MV);
        v["board"]["stackup"]["fab_tolerances"]
            .as_object_mut()
            .unwrap()
            .remove(key);
        let (code, r) = extract(s03_board(), &write(key, &v));
        assert_eq!(code, 2, "{key}: {r}");
        assert_eq!(r["error"]["kind"], "SupplyInputUndeclared", "{key}: {r}");
        assert!(
            r["error"]["message"]
                .as_str()
                .unwrap()
                .contains(&format!("fab_tolerances.{key}")),
            "{key}: {r}"
        );
    }
    // every part on the domain that draws current is named by a load
    // entry: the flash's removed, the gate names it
    let mut v = gated(s03(), S03_LIMIT_MV);
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
    let mut v = gated(s03(), S03_LIMIT_MV);
    // The MCU's supply keeps its limit (`DropLimitUndeclared` otherwise,
    // `AET-WO-4300k` (2)); the second names a domain with no MCU pin.
    v["power"]["drop_limits"]
        .as_array_mut()
        .unwrap()
        .push(json!({"domain": "VDD_5V", "max_millivolts": 250, "source": "test: the limit"}));
    let (code, r) = extract(s03_board(), &write("five", &v));
    assert_eq!(code, 2, "{r}");
    assert_eq!(r["error"]["kind"], "SupplyTerminalMissing", "{r}");
}

/// The four-layer board: every MCU supply and return pin bounded, from the
/// buck's inductor pad and to its PGND pin, every tolerance priced, and the
/// drop within 165 mV.
#[test]
fn the_four_layer_board_bounds_every_supply_pin_within_its_limit() {
    let (code, v) = extract(s024l_board(), &write("s024l-pass", &gated(s024l(), 165)));
    assert_eq!(code, 0, "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    // The limit declared, the part is judged by its domains (`AET-WO-1609`).
    assert_eq!(
        part(&v, "extract.supply"),
        &json!({"gate": "extract.supply", "judged": 1})
    );
    let s = &v["supplies"][0];
    assert_eq!(s["domain"], "VDD_3V3");
    assert_eq!(s["source"], json!({"reference": "L2", "pad": "2"}));
    assert_eq!(s["return"], json!({"reference": "U11", "pad": "1"}));
    // the MCU's 658 mA from its part row, the 1.8 V LDO's input and the
    // fixture's other sinks, each cited, derived or declared (the removed
    // crossing's U13 and R40 no longer among them, `AET-WO-1209` D4)
    assert_eq!(s["load_milliamps"], 714);
    let (vdd, vss) = (pins(s, "pins"), pins(s, "returns"));
    assert_eq!((vdd.len(), vss.len()), (16, 11), "{s}");
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
    assert_eq!(drop, ((worst(&vdd) + worst(&vss)) * 714).div_ceil(1000));
    assert!(drop <= 165_000, "{drop}");
    assert!(s.get("note").is_none(), "{s}");
    let conditions: Vec<&str> = s["conditions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect();
    // each fabricator's tolerance priced, from its declared source, and the
    // pads' drawn size named: Sentinel-03's stackup, OSH Park's since
    // `AET-WO-4300k` (b), its track-width tolerance a declared condition
    for named in [
        "+-0.051 mm per axis of its pad (OSH Park",
        "from 0.064 mm under to 0.064 mm over its drawn size (OSH Park",
        "every track at 0.8 of its drawn width (a declared condition: OSH Park publishes no track-width tolerance",
        "pads and via pads at their drawn size",
    ] {
        assert!(
            conditions.iter().any(|c| c.contains(named)),
            "{named}: {conditions:?}"
        );
    }
    assert!(
        conditions
            .iter()
            .any(|c| c.contains("other domains' return currents")),
        "{conditions:?}"
    );
}

/// Sentinel-03 on the ring rule's vias (`AET-WO-1314`, ruling 6982875594
/// D5, inverting `1303b`'s (A)): every supply and return pin bounded, no
/// `SupplyViaPadTooSmall`, the drop within its limit. Its own specification
/// declares no drop limit; this one adds it.
#[test]
fn sentinel_03_on_rule_sized_vias_bounds_every_return() {
    let (code, r) = extract(s03_board(), &write("s03-ring", &gated(s03(), S03_LIMIT_MV)));
    assert_eq!(code, 0, "{r}");
    let s = &r["supplies"][0];
    assert_eq!(s["verdict"], "pass", "{s}");
    let (vdd, vss) = (pins(s, "pins"), pins(s, "returns"));
    assert_eq!((vdd.len(), vss.len()), (16, 10), "{s}");
    for p in vdd.iter().chain(&vss) {
        assert!(p["r_bound_microohms"].is_u64(), "{p}");
    }
}

/// A ring under the gate's own need: at a declared ring of 0, Sentinel-03's
/// GND vias keep the hole in the pad but not the band the gate prices, so
/// every return is refused by name with the pad that would hold it (rulings
/// 6974760231 (1), 6982875594 D5): 0.57 mm at OSH Park's tolerances since
/// `AET-WO-4300k` (b) (0.7 mm at JLCPCB's). A class
/// declaring a via under the ring rule is refused at `emit` instead
/// (`ViaRingUnheld`, `via_ring_tier1`).
#[test]
fn a_via_pad_the_worst_hole_does_not_fit_refuses_its_pins_with_the_remedy() {
    let mut v = gated(s03(), S03_LIMIT_MV);
    v["board"]["stackup"]["fab_tolerances"]["via_ring_um"]["value"] = json!(0);
    let spec = write("s03-ring0", &v);
    let board = emitted("s03-ring0", &spec);
    let (code, r) = extract(&board, &spec);
    assert_eq!(code, 1, "{r}");
    let s = &r["supplies"][0];
    let vss = pins(s, "returns");
    assert!(!vss.is_empty());
    for p in &vss {
        assert_eq!(p["refused"]["kind"], "SupplyViaPadTooSmall", "{p}");
        assert!(
            p["refused"]["message"]
                .as_str()
                .unwrap()
                .contains("via pad >= 0.57 mm"),
            "{p}"
        );
    }
}

/// The same board against a limit its bound exceeds: a finding, and the
/// note that the pitch set the margin.
#[test]
fn a_drop_over_its_limit_fails_with_the_pitch_note() {
    let (code, v) = extract(s024l_board(), &write("s024l-fail", &gated(s024l(), 60)));
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
///
/// `AET-WO-4410` Q12 (ruled 2026-11-24): the sense is kept off ADC1, on the
/// pin this test measured (`common::sense::keep_off_adc1`). On the PC0 sense
/// extract refuses the board, which
/// `the_two_layer_power_fixture_on_the_pc0_sense_meets_overlapping_vias` holds.
#[test]
fn the_two_layer_power_fixture_refuses_its_ground_return_by_name() {
    let spec = write(
        "s02",
        &gated(
            common::sense::keep_off_adc1(common::power::power_tree()),
            165,
        ),
    );
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
    assert_eq!(
        part(&v, "extract.pairs"),
        &json!({"gate": "extract.pairs", "judged": 2})
    );
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

/// `AET-WO-4410` Q12, then `AET-WO-1324` (ruled 2026-11-24). With the sense
/// on PC0, the two-layer power fixture's router drew two VDD_3V3 vias whose
/// junction regions met, at (93.25, 87.125) mm and (4, 4) pitches from it,
/// and `extract` refused the board (`ViaRegionsOverlap`). The router now
/// keeps a judged net's vias one junction region apart, so the gate reads
/// the board. Every supply pin is bounded, and every return is refused by
/// its own name, `GroundReturnUnread` (the two-layer ground zone is left
/// unfilled): the refusal the PF3 layout had, recorded and not chased here.
#[test]
fn the_two_layer_power_fixture_on_the_pc0_sense_keeps_its_vias_regions_apart() {
    let spec = write("s02-pc0", &gated(common::power::power_tree(), 165));
    let board = emitted("s02-pc0", &spec);
    let (code, v) = extract(&board, &spec);
    assert_eq!(code, 1, "{v}");
    assert_eq!(v["verdict"], "fail", "{v}");
    let s = &v["supplies"][0];
    let (vdd, vss) = (pins(s, "pins"), pins(s, "returns"));
    assert_eq!((vdd.len(), vss.len()), (16, 11), "{s}");
    assert!(vdd.iter().all(|p| p["r_bound_microohms"].is_u64()), "{s}");
    assert!(
        vss.iter()
            .all(|p| p["refused"]["kind"] == "GroundReturnUnread"),
        "{s}"
    );
    // The gate stops at the returns before it prices GND, so the regions are
    // checked here too, on both judged nets (adversarial A4): each routed
    // via, array via and through-hole pad of the net on the gate's cell grid
    // holds the region `extract` gives it, and no two share a cell.
    use aethelgard_eda::{flow::Region, route, stack::Stack, supply};
    let parsed: aethelgard_ir::Specification =
        serde_json::from_str(&fs::read_to_string(&spec).unwrap()).unwrap();
    let stack = Stack::of(&parsed).unwrap();
    let rule = stack.junction.clone().unwrap();
    let tree = aethelgard_eda::strict::board(&fs::read_to_string(&board).unwrap()).unwrap();
    let (read, routed) = route::read(&tree, &stack).unwrap();
    let h = supply::PITCH_NM;
    let origin = routed
        .vias
        .first()
        .map(|v| (v.at.0.rem_euclid(h), v.at.1.rem_euclid(h)))
        .unwrap();
    for net in ["VDD_3V3", "GND"] {
        let mut holes: Vec<((i64, i64), i64)> = routed
            .vias
            .iter()
            .filter(|v| v.net == net)
            .map(|v| (v.at, read.stack.rule(net).drill_nm))
            .collect();
        for a in routed.arrays.iter().filter(|a| a.net == net) {
            holes.extend(a.at.iter().map(|&at| (at, a.drill_nm)));
        }
        for p in read.parts.iter().flat_map(|p| &p.pads) {
            if let (Some(d), Some(n)) = (p.drill_nm, p.net.as_deref())
                && n == net
            {
                let r = p.rect;
                holes.push((((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2), d));
            }
        }
        let mut owner = std::collections::BTreeMap::new();
        let mut judged = 0;
        for (k, &(at, drill)) in holes.iter().enumerate() {
            let (dx, dy) = (at.0 - origin.0, at.1 - origin.1);
            if dx.rem_euclid(h) != 0 || dy.rem_euclid(h) != 0 {
                continue;
            }
            judged += 1;
            for &(i, j) in &Region::new(h, rule.reach(drill)).cells {
                let cell = (dx / h + i, dy / h + j);
                if let Some(other) = owner.insert(cell, k) {
                    panic!("{net}: {:?} and {at:?} meet at {cell:?}", holes[other].0);
                }
            }
        }
        assert!(judged >= 20, "{net}: {judged}");
    }
}

/// `AET-WO-1324` Q1: the router's junction rule is on exactly where the
/// gate runs: on each limited domain and `GND`, at the specification's
/// tolerances and plating, so its reach is the gate's; off with no limit, or
/// with any stackup or tolerance input the gate reads undeclared
/// (adversarial A3: once on with `outer_copper_min_um` missing).
#[test]
fn the_routers_junction_rule_is_on_where_the_gate_runs() {
    use aethelgard_eda::stack::{JunctionRule, Stack};
    let spec =
        |v: &Value| -> aethelgard_ir::Specification { serde_json::from_value(v.clone()).unwrap() };
    let rule = |v: &Value| JunctionRule::of(&spec(v));
    let on = rule(&gated(s03(), S03_LIMIT_MV)).unwrap();
    // the board's stack carries it
    let stack = Stack::of(&spec(&gated(s03(), S03_LIMIT_MV))).unwrap();
    assert_eq!(stack.junction.as_ref(), Some(&on));
    assert_eq!(on.nets, ["VDD_3V3", "VDD_3V3_RF", "GND"]);
    // OSH Park's tolerances since `AET-WO-4300k` (b) (JLCPCB's were 130 and
    // 75 um).
    assert_eq!(
        (on.hole_plus_nm, on.hole_offset_nm, on.plating_nm),
        (64_000, 51_000, 18_000)
    );
    // (300000 + 64000 + 1) / 2 + 18000 + 10000 + ⌈51000·√2⌉ + 1 = 282126.
    assert_eq!(on.reach(300_000), 282_126);
    // Sentinel-03 declares its own limit since `AET-WO-4300k` D6, so the rule
    // is on for it as committed; with no limit it is off.
    assert_eq!(rule(&s03()).as_ref(), Some(&on));
    let mut v = gated(s03(), S03_LIMIT_MV);
    v["power"]["drop_limits"] = json!([]);
    assert_eq!(rule(&v), None);
    for (path, key) in [
        ("board/stackup", "plating_min_um"),
        ("board/stackup", "outer_copper_min_um"),
        ("board/stackup", "plane_copper_min_um"),
        ("board/stackup", "copper_temperature_max_c"),
        ("board/stackup/fab_tolerances", "hole_size_plus_um"),
        ("board/stackup/fab_tolerances", "hole_position_um"),
        ("board/stackup/fab_tolerances", "hole_size_minus_um"),
        ("board/stackup/fab_tolerances", "track_width_permille"),
    ] {
        let mut v = gated(s03(), 165);
        let at = format!("/{path}");
        v.pointer_mut(&at)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key);
        assert_eq!(rule(&v), None, "{key}");
    }
}
