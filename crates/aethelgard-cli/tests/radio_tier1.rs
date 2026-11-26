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
    // within one period of the module's own 10 um snap against the supply
    // gate's 62.5 um grid (0.25 mm, `AET-WO-4411` (c)), and its ground
    // pad's centre (0.96, 0.2) mm from its origin is on that grid.
    let gap = x1 - (mx + COURTYARD_X1_NM);
    assert!((2_000_000..2_250_000).contains(&gap), "right gap {gap} nm");
    assert_eq!(((mx + 960_000) % 62_500, (my + 200_000) % 62_500), (0, 0));
    // Every other part lies below the antenna edge's 2 mm margin.
    for (r, _, (_, y)) in fps.iter().filter(|(r, ..)| r != "U30") {
        assert!(*y > y0 + 2_000_000, "{r} at y {y}, the edge at {y0}");
    }
}

/// The module's pads, by number, on the nets the row and the port name.
#[test]
fn the_modules_pads_are_on_their_nets() {
    let tree = strict::board(board()).unwrap();
    let pads = |reference: &str| {
        let fp = tree
            .root()
            .lists()
            .filter(|i| i.head() == Some("footprint"))
            .find(|fp| {
                fp.all("property")
                    .any(|p| p.arg(1) == Some("Reference") && p.arg(2) == Some(reference))
            })
            .unwrap();
        move |pad: &str| {
            fp.all("pad")
                .find(|p| p.arg(1) == Some(pad))
                .and_then(|p| p.one("net").unwrap())
                .and_then(|n| n.arg(1))
                .unwrap_or("")
                .to_owned()
        }
    };
    let net = pads("U30");
    // U0TXD (12) drives U31's B4; U0RXD (11) hears U31's B1Y (ruled
    // 2026-11-26T04:03Z (c1): the port through the translator)
    assert_eq!(net("12"), "/U31_B4");
    assert_eq!(net("11"), "/U31_B1Y");
    // U31, TXU0304PW: A1 hears the port's TX and A4Y drives its RX on the
    // MCU's rail, B1Y and B4 on the module's; OE strapped to VCCA, the rail
    // that rises last (SCES935A 9.1: OE low through power-up); A2 and A3
    // tied to GND, no input floating; B2Y, B3Y and NC (SCES935A p. 4, Table
    // 6-1: no internal connection) open
    let txu = pads("U31");
    for (pad, want) in [
        ("1", "VDD_3V3"),
        ("2", "/WIFI_TX"),
        ("3", "GND"),
        ("4", "GND"),
        ("5", "/WIFI_RX"),
        ("7", "GND"),
        ("8", "VDD_3V3"),
        ("10", "/U31_B4"),
        ("13", "/U31_B1Y"),
        ("14", "VDD_3V3_RF"),
    ] {
        assert_eq!(txu(pad), want, "U31 pad {pad}");
    }
    for pad in ["6", "9", "11", "12"] {
        assert!(
            txu(pad).starts_with("unconnected-"),
            "U31 pad {pad}: {}",
            txu(pad)
        );
    }
    // its own rail, U13's (`AET-WO-4411`, ruled 2026-11-25T20:10Z)
    assert_eq!(net("1"), "VDD_3V3_RF");
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

/// D7: the module's port takes a UART of its own, and no entry of the solved
/// topology of the board without the module (its module and port removed)
/// moves: not only `assignments` (the solver's own choice, UART4 on PA0/PA1,
/// kept every assignment but moved `gpio.FLASH_CS` to PA15). `power` moves by
/// the module's counted 500 mA, which `aethelgard-ir`'s radio tests judge.
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
    // U31 carries the port to the module (`AET-WO-4411` (c1)): it goes too
    let power = spec["power"].as_object_mut().unwrap();
    assert!(power.remove("uart_crossings").is_some());
    power["loads"]
        .as_array_mut()
        .unwrap()
        .retain(|l| l["reference"] != "U31");
    let dir = std::env::temp_dir().join(format!("aet-4411-base-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    let base_spec = dir.join("sentinel-03-without-module.json");
    fs::write(&base_spec, serde_json::to_vec(&spec).unwrap()).unwrap();
    let (code, base) = run(&["solve", "--file", base_spec.to_str().unwrap()]);
    assert_eq!(code, 0, "{base}");
    let (code, radio) = run(&["solve", "--file", SPEC]);
    assert_eq!(code, 0, "{radio}");
    let port = &radio["serial_ports"][label.as_str()];
    let own = [
        label.as_str(),
        port["tx"].as_str().unwrap(),
        port["rx"].as_str().unwrap(),
    ];
    let mut keys = 0;
    for (key, a) in base.as_object().unwrap() {
        if key == "specification_sha256" || key == "power" {
            continue;
        }
        let mut b = radio[key].clone();
        if let Some(o) = b.as_object_mut() {
            o.retain(|k, _| !own.contains(&k.as_str()));
        }
        assert_eq!(&b, a, "`{key}` moved");
        keys += 1;
    }
    assert!(keys > 10, "{keys} keys compared");
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

/// (a), ruled 2026-11-25T21:17Z: the regulator whose output is the module's
/// supply, U13, belongs to the module's group: in the strip the module takes
/// at the board's top (its origin within the module's courtyard height,
/// whose y runs from 13.375 mm above the module's origin to 7.175 mm below),
/// left of the module, and turned 180 degrees so its switch node, whose
/// inductor is its largest anchored part, faces the open board.
#[test]
fn the_modules_regulator_sits_in_the_modules_strip_turned_away() {
    let tree = strict::board(board()).unwrap();
    let fps = footprints(&tree);
    let at = |r: &str| fps.iter().find(|(x, ..)| x == r).unwrap().2;
    let ((mx, my), (ux, uy)) = (at("U30"), at("U13"));
    assert!(
        (my - 13_375_000..=my + 7_175_000).contains(&uy),
        "U13 at {ux} {uy}, U30 at {mx} {my}"
    );
    assert!(ux < mx, "U13 at {ux}, U30 at {mx}");
    let turn = tree
        .root()
        .lists()
        .filter(|i| i.head() == Some("footprint"))
        .find(|fp| {
            fp.all("property")
                .any(|p| p.arg(1) == Some("Reference") && p.arg(2) == Some("U13"))
        })
        .and_then(|fp| fp.one("at").unwrap())
        .and_then(|a| a.arg(3).map(str::to_owned));
    assert_eq!(turn.as_deref(), Some("180"));
}

/// Adversarial F4: a module on the MCU's own rail owns no regulator, so the
/// MCU's (U11, on the `sentinel-03-radio` fixture, whose module is on
/// `VDD_3V3`) is placed with the board, below the module's strip, never in
/// the module's group.
#[test]
fn a_module_on_the_mcus_rail_owns_no_regulator() {
    let path = root().join("crates/aethelgard-cli/tests/fixtures/sentinel-03-radio.json");
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let board = aethelgard_eda::schematic::placement(&spec, &solved, &table).unwrap();
    let tree = strict::board(&board).unwrap();
    let fps = footprints(&tree);
    let at = |r: &str| fps.iter().find(|(x, ..)| x == r).unwrap().2;
    let ((_, my), (_, uy)) = (at("U30"), at("U11"));
    assert!(uy > my + 7_175_000, "U11 at y {uy}, U30 at y {my}");
}

/// The gate's conditions name the translator's B side as unbounded once a
/// UART crossing is declared, as an SPI crossing's (`AET-WO-4411` (c1)).
#[test]
fn a_uart_crossing_names_its_b_side_lines_unbounded() {
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    let lines = |s: &aethelgard_ir::Specification| {
        aethelgard_eda::loops::conditions(s)
            .iter()
            .any(|c| c.starts_with("LINES: each line on a translator's B side"))
    };
    assert!(lines(&spec));
    let mut v: Value = serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    v["power"].as_object_mut().unwrap().remove("uart_crossings");
    assert!(!lines(&serde_json::from_value(v).unwrap()));
}

/// U31's two bypasses are loop parts the gate judges (`AET-WO-4411` (c1)):
/// C93 on its VCCA pin (1) and C94 on its VCCB pin (14), at the translator
/// bypass rule, as an SPI crossing's.
#[test]
fn the_crossings_bypasses_are_judged_loops() {
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(root().join(SPEC)).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let rules = aethelgard_eda::loops::rules(&spec, &solved, &table).unwrap();
    for (part, pin) in [("C93", "1"), ("C94", "14")] {
        let r = rules
            .iter()
            .find(|r| r.part == part)
            .unwrap_or_else(|| panic!("{part} is no loop rule"));
        assert_eq!(r.ic, "U31", "{part}");
        assert!(r.pads.iter().any(|(_, p)| p == pin), "{part}: {:?}", r.pads);
    }
}

/// `drc.radio` reads U30's angle as `strict::number(&at, 3).unwrap_or(0)`:
/// absent is KiCad's unturned. One written but not a number (`"90"`, a
/// string, or `ninety`, a bare symbol) never reaches it: the strict reader
/// refuses the board first (`MalformedBoard`, "where KiCad reads a number";
/// the review's R3, ruled 08:59Z: shown, the line left).
#[test]
fn a_modules_angle_that_is_no_number_is_refused_before_drc_radio_reads_it() {
    let text = board();
    let at = text
        .find("(footprint \"Aethelgard_RF:ESP32-C3-WROOM-02\"")
        .expect("U30's footprint");
    let open = at + text[at..].find("(at ").unwrap();
    let close = open + text[open..].find(')').unwrap();
    let dir = std::env::temp_dir().join(format!("aet-4411-angle-{}", std::process::id()));
    fs::create_dir_all(&dir).unwrap();
    for angle in ["\"90\"", "ninety"] {
        let b = dir.join("b.kicad_pcb");
        fs::write(&b, format!("{} {angle}{}", &text[..close], &text[close..])).unwrap();
        let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
            .args([
                "check",
                "drc",
                "--board",
                b.to_str().unwrap(),
                "--spec",
                SPEC,
            ])
            .current_dir(root())
            .env("PATH", "/nonexistent")
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let v: Value = serde_json::from_slice(&out.stdout).unwrap();
        assert_eq!(v["error"]["kind"], "MalformedBoard", "{angle}: {v}");
        assert!(
            v["error"]["message"]
                .as_str()
                .unwrap()
                .contains("where KiCad reads a number"),
            "{angle}: {v}"
        );
    }
}

/// Every via on the board lies on the 1 µm grid an Excellon drill file
/// writes, U30 pad 19's thermal array among them (`AET-WO-4411`, the
/// review's 10:40Z `bench-package-s03` finding: at the 1.125 mm pitch first
/// ruled, 129.4375 mm was drilled at 129.438; 1.0 mm since 11:51Z).
#[test]
fn every_via_lies_on_the_drill_files_micron_grid() {
    let tree = strict::board(board()).unwrap();
    let vias: Vec<_> = tree
        .root()
        .lists()
        .filter(|i| i.head() == Some("via"))
        .collect();
    assert!(vias.len() > 400, "{}", vias.len());
    for v in vias {
        let at = v.one("at").unwrap().unwrap();
        for i in [1, 2] {
            let nm = strict::number(&at, i).unwrap();
            assert_eq!(
                nm.rem_euclid(1000),
                0,
                "a via at {:?}",
                (at.arg(1), at.arg(2))
            );
        }
    }
}
