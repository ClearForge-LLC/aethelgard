//! `AET-WO-4411`, tier 1 (no tool runs): Sentinel-03, which carries an
//! ESP32-C3-WROOM-02 on a UART of its own (ruled D6), emitted, its module at
//! the board's top-right corner with the antenna area wholly off the board
//! (ruled D2), its pads on the nets its row and port name, its keepout on
//! each copper layer (D4), and no pin moved by the module's port against the
//! same board without it (D7). ERC, DRC and the planted boards (the
//! `tests/fixtures/sentinel-03-radio.json` fixture) in the image are
//! `board_tier2.rs`'s.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::OnceLock;

use aethelgard_eda::strict;
use serde_json::Value;

const SPEC: &str = "examples/sentinel-03.json";
const PCB: &str = "sentinel-03.kicad_pcb";
/// The module's antenna edge and courtyard, about its origin, in nm: the
/// row's antenna area (y1) and the derived footprint's courtyard.
const EDGE_NM: i64 = -7_100_000;
const COURTYARD_X1_NM: i64 = 9_750_000;

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    (
        out.status.code().unwrap(),
        serde_json::from_str(stdout.lines().last().unwrap_or("{}")).unwrap(),
    )
}

/// The fixture's board, emitted once.
fn board() -> &'static str {
    static TEXT: OnceLock<String> = OnceLock::new();
    TEXT.get_or_init(|| {
        let dir = std::env::temp_dir().join(format!("aet-4411-emit-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        let (code, v) = run(&["emit", "--file", SPEC, "--out", dir.to_str().unwrap()]);
        assert_eq!(code, 0, "{v}");
        fs::read_to_string(dir.join(PCB)).unwrap()
    })
}

/// The outline's `(x0, y0, x1, y1)` in nm.
fn outline(tree: &strict::Tree) -> (i64, i64, i64, i64) {
    let rect = tree
        .root()
        .lists()
        .find(|i| i.head() == Some("gr_rect"))
        .unwrap();
    let xy = |h: &str| {
        let n = rect.one(h).unwrap().unwrap();
        (
            strict::number(&n, 1).unwrap(),
            strict::number(&n, 2).unwrap(),
        )
    };
    let ((x0, y0), (x1, y1)) = (xy("start"), xy("end"));
    (x0.min(x1), y0.min(y1), x0.max(x1), y0.max(y1))
}

/// Each footprint's reference, library id and origin in nm.
fn footprints(tree: &strict::Tree) -> Vec<(String, String, (i64, i64))> {
    tree.root()
        .lists()
        .filter(|i| i.head() == Some("footprint"))
        .map(|fp| {
            let reference = fp
                .all("property")
                .find(|p| p.arg(1) == Some("Reference"))
                .and_then(|p| p.arg(2))
                .unwrap()
                .to_owned();
            let at = fp.one("at").unwrap().unwrap();
            (
                reference,
                fp.arg(1).unwrap().to_owned(),
                (
                    strict::number(&at, 1).unwrap(),
                    strict::number(&at, 2).unwrap(),
                ),
            )
        })
        .collect()
}

#[test]
fn the_antenna_overhangs_the_top_right_corner() {
    let tree = strict::board(board()).unwrap();
    let (x0, y0, x1, y1) = outline(&tree);
    let fps = footprints(&tree);
    let (_, id, (mx, my)) = fps.iter().find(|(r, ..)| r == "U30").unwrap();
    assert_eq!(id, "Aethelgard_RF:ESP32-C3-WROOM-02");
    // The board's top edge is the antenna edge: the antenna area is off it.
    assert_eq!(
        y0,
        my + EDGE_NM,
        "outline {x0} {y0} {x1} {y1}, module at {mx} {my}"
    );
    // The module's courtyard keeps the edge margin (2 mm) at the right,
    // within one snap (0.05 mm).
    let gap = x1 - (mx + COURTYARD_X1_NM);
    assert!((2_000_000..=2_050_000).contains(&gap), "right gap {gap} nm");
    // Every other part lies below the antenna edge's 2 mm margin.
    for (r, _, (_, y)) in fps.iter().filter(|(r, ..)| r != "U30") {
        assert!(*y > y0 + 2_000_000, "{r} at y {y}, the edge at {y0}");
    }
}

/// The module's pads, by number, on the nets the row and the port name.
#[test]
fn the_modules_pads_are_on_their_nets() {
    let tree = strict::board(board()).unwrap();
    let fp = tree
        .root()
        .lists()
        .filter(|i| i.head() == Some("footprint"))
        .find(|fp| {
            fp.all("property")
                .any(|p| p.arg(1) == Some("Reference") && p.arg(2) == Some("U30"))
        })
        .unwrap();
    let net = |pad: &str| {
        fp.all("pad")
            .find(|p| p.arg(1) == Some(pad))
            .and_then(|p| p.one("net").unwrap())
            .and_then(|n| n.arg(1))
            .unwrap_or("")
            .to_owned()
    };
    // U0TXD (12) drives the port's RX net; U0RXD (11) hears its TX net.
    assert_eq!(net("12"), "/WIFI_RX");
    assert_eq!(net("11"), "/WIFI_TX");
    assert_eq!(net("1"), "VDD_3V3");
    assert_eq!(net("2"), "/U30_EN");
    assert_eq!(net("9"), "GND");
    assert_eq!(net("19"), "GND");
    // IO2 pulled up by R91; IO8 and IO9 open.
    assert_eq!(net("16"), "/U30_IO2");
    assert!(net("7").starts_with("unconnected-"), "{}", net("7"));
    assert!(net("8").starts_with("unconnected-"), "{}", net("8"));
}

/// D4: one keepout per copper layer, each the antenna area, off the board.
#[test]
fn the_keepout_is_on_each_copper_layer_off_the_board() {
    let tree = strict::board(board()).unwrap();
    let (_, y0, _, _) = outline(&tree);
    let fp = tree
        .root()
        .lists()
        .filter(|i| i.head() == Some("footprint"))
        .find(|fp| fp.arg(1) == Some("Aethelgard_RF:ESP32-C3-WROOM-02"))
        .unwrap();
    let mut layers = Vec::new();
    for z in fp.all("zone") {
        assert!(z.one("keepout").unwrap().is_some());
        layers.push(z.one("layer").unwrap().unwrap().arg(1).unwrap().to_owned());
        let pts = z
            .one("polygon")
            .unwrap()
            .unwrap()
            .one("pts")
            .unwrap()
            .unwrap();
        for xy in pts.lists() {
            assert!(
                strict::number(&xy, 2).unwrap() <= y0,
                "a keepout corner on the board"
            );
        }
    }
    layers.sort();
    assert_eq!(layers, ["B.Cu", "F.Cu", "In1.Cu", "In2.Cu"]);
}

/// D7: the module's port takes a UART of its own, and no pin of the board
/// without the module (its module and port removed) moves.
#[test]
fn the_modules_port_moves_no_pin() {
    let mut spec: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    let module = spec
        .as_object_mut()
        .unwrap()
        .remove("radio_modules")
        .unwrap();
    let label = module[0]["serial_port"].as_str().unwrap().to_owned();
    let ports = spec["serial_ports"].as_array_mut().unwrap();
    let n = ports.len();
    ports.retain(|p| p["label"] != label.as_str());
    assert_eq!(ports.len(), n - 1, "no port `{label}`");
    let dir = std::env::temp_dir().join(format!("aet-4411-base-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let base_spec = dir.join("sentinel-03-without-module.json");
    fs::write(&base_spec, serde_json::to_vec(&spec).unwrap()).unwrap();
    let (code, base) = run(&["solve", "--file", base_spec.to_str().unwrap()]);
    assert_eq!(code, 0, "{base}");
    let (code, radio) = run(&["solve", "--file", SPEC]);
    assert_eq!(code, 0, "{radio}");
    let (a, b) = (&base["assignments"], &radio["assignments"]);
    for (net, pin) in a.as_object().unwrap() {
        assert_eq!(&b[net], pin, "{net} moved");
    }
    let wifi = &radio["serial_ports"][label.as_str()]["peripheral"];
    assert!(wifi.is_string(), "{label}: {wifi}");
    let taken: Vec<&Value> = base["serial_ports"]
        .as_object()
        .unwrap()
        .values()
        .map(|p| &p["peripheral"])
        .collect();
    assert!(!taken.contains(&wifi), "{wifi} is already a port's");
}
