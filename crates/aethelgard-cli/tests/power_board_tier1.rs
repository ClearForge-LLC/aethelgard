//! `AET-WO-4300d2`: the power tree on the board, judged without KiCad. The
//! Sentinel-02 fixture (without its crossing, which waits for `K6` and the
//! ICM-20948's footprint, FEEDBACK D2) is emitted, read back, and its loop
//! rules measured from the board as `check drc --spec` measures them; then
//! each loop part is moved off its pin and fails its rule (§5.1), and a
//! divider on the wrong pads fails `K5` (§5.2).

mod common;

use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::{loops, route, strict};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};

fn fixture() -> (Specification, SolvedTopology, PinTable) {
    let v = common::power::power_tree();
    let spec: Specification = serde_json::from_value(v).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

fn read(spec: &Specification, board: &str) -> route::Board {
    let tree = strict::board(board).unwrap();
    let (b, r) = route::read(&tree, &Stack::of(spec)).unwrap();
    route::connectivity(&b, &r).unwrap();
    b
}

fn judged(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
    board: &str,
) -> Vec<loops::LoopRecord> {
    let rules = loops::rules(spec, solved, table).unwrap();
    loops::judge(&read(spec, board), &rules).unwrap()
}

#[test]
fn every_loop_rule_holds_on_the_fixtures_own_board() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let records = judged(&spec, &solved, &table, &e.board);
    // Every part the tree anchors: per buck the two input capacitors, CBOOT,
    // CVCC, the divider and the inductor (7 x 2); the LDO's two capacitors;
    // the sense reservoir.
    assert_eq!(records.len(), 17, "{records:#?}");
    for r in &records {
        assert_eq!(r.verdict, "pass", "{r:#?}");
    }
    let b = read(&spec, &e.board);
    loops::source_pads(&b, &solved).unwrap();
    // The reservoir sits by the MCU pin its net was solved onto.
    let sense = &solved.power.as_ref().unwrap().sense["VBAT_SENSE"];
    let c50 = records.iter().find(|r| r.part == "C50").unwrap();
    assert_eq!(c50.pads[0].pin, sense.position.to_string());
    // And that MCU pad carries the sense net (§1.7).
    let mcu = b.parts.iter().find(|p| p.reference == c50.ic).unwrap();
    let pad = mcu
        .pads
        .iter()
        .find(|p| p.number == sense.position.to_string())
        .unwrap();
    assert_eq!(pad.net.as_deref(), Some("/VBAT_SENSE"));
    // Two emissions are byte for byte the same.
    assert_eq!(e, schematic::emit(&spec, &solved, &table).unwrap());
}

/// §5.1: Cin far from VIN, the divider at the rail's far end, the inductor
/// away from SW. Each is moved 25 mm off and fails its own rule, and only it.
#[test]
fn a_loop_part_moved_off_its_pin_fails_its_rule() {
    let (spec, solved, table) = fixture();
    let plain = schematic::emit(&spec, &solved, &table).unwrap();
    let b = read(&spec, &plain.board);
    let o = b.outline;
    for part in ["C11", "R10", "L1", "C30", "C50"] {
        let moves = vec![(part.to_owned(), (o.x1 + 25_000_000, o.y0), 0)];
        let moved = match schematic::emit_moved(&spec, &solved, &table, &moves) {
            Ok(m) => m,
            Err(e) => panic!("{part}: {e}"),
        };
        let records = judged(&spec, &solved, &table, &moved.board);
        let failed: Vec<&str> = records
            .iter()
            .filter(|r| r.verdict == "fail")
            .map(|r| r.part.as_str())
            .collect();
        assert_eq!(failed, [part], "{part}");
    }
}

/// §5.2: the divider on the wrong pads. A board whose domain's source pad is
/// not on the domain is refused by `K5`, whatever the labels' own agreement.
#[test]
fn a_source_or_return_pad_off_its_net_fails_k5() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    for (reference, pad, net) in [("R20", "1", "GND"), ("R21", "2", "VDD_3V3")] {
        let mut b = read(&spec, &e.board);
        let p = b
            .parts
            .iter_mut()
            .find(|p| p.reference == reference)
            .unwrap()
            .pads
            .iter_mut()
            .find(|p| p.number == pad)
            .unwrap();
        p.net = Some(net.to_owned());
        let err = loops::source_pads(&b, &solved).unwrap_err();
        assert!(err.contains(reference) && err.contains("K5"), "{err}");
    }
}

/// A loop rule names a part the board lacks: refused, never skipped.
#[test]
fn a_rule_whose_part_is_missing_is_refused() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let mut b = read(&spec, &e.board);
    b.parts.retain(|p| p.reference != "C13");
    let rules = loops::rules(&spec, &solved, &table).unwrap();
    let err = loops::judge(&b, &rules).unwrap_err();
    assert!(err.contains("C13"), "{err}");
}

/// One translator carries one chip select: a crossing on a bus of two
/// devices is refused by name at the emitter (reached here around
/// `validate`, which refuses every crossing until `K6`).
#[test]
fn a_crossing_on_a_bus_of_two_devices_is_not_drawable() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    let mut v: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let devices = v["spi_devices"].as_array_mut().unwrap();
    let mut second = devices[1].clone();
    second["reference"] = "U6".into();
    second["cs"]["net"] = "IMU2_CS".into();
    devices.push(second);
    let spec: Specification = serde_json::from_value(v).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let (fixture_spec, solved, _) = fixture();
    let _ = fixture_spec;
    match schematic::emit(&spec, &solved, &table) {
        Err(e @ EmitError::PowerNotDrawable { .. }) => {
            assert_eq!(e.kind(), "PowerNotDrawable");
            assert_eq!(e.id(), Some("U13"));
            assert!(e.to_string().contains("2 devices"), "{e}");
        }
        other => panic!("{other:?}"),
    }
}

/// D12: the rail an LDO's `power_out` pin drives carries no `PWR_FLAG` (it
/// would be a second driver), and every other rail keeps its flag.
#[test]
fn only_a_rail_no_power_out_pin_drives_carries_a_flag() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let flags = e.schematic.matches("(lib_id \"power:PWR_FLAG\")").count();
    // GND, VBAT, VDD_3V3 and VDD_5V; not VDD_1V8, which U12's OUT drives.
    assert_eq!(flags, 4, "{flags} flags");
}

/// A buck's switch node is laid at its output's class; nothing else joins
/// a class it did not declare.
#[test]
fn a_bucks_sw_net_takes_its_outputs_class_and_an_ldo_adds_nothing() {
    let mut v = common::power::power_tree();
    v["net_classes"][0]["nets"] = serde_json::json!(["VBAT", "VDD_5V", "VDD_1V8"]);
    let spec: Specification = serde_json::from_value(v).unwrap();
    spec.validate().unwrap();
    let stack = Stack::of(&spec);
    let power = stack.classes.iter().find(|c| c.name == "POWER").unwrap();
    assert_eq!(power.nets, ["VBAT", "VDD_5V", "VDD_1V8", "/U10_SW"]);
    let mcu = stack
        .classes
        .iter()
        .find(|c| c.name == "SUPPLY_MCU")
        .unwrap();
    assert_eq!(mcu.nets, ["VDD_3V3", "/U11_SW"]);
}

/// `extract` (no tool) bounds the fixture's board: its through-hole pads,
/// thermal vias and arc-drawn silkscreen are read, and every line passes.
#[test]
fn extract_bounds_the_power_board() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let dir = std::env::temp_dir().join(format!("aet-4300d2-extract-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let board = dir.join("b.kicad_pcb");
    std::fs::write(&board, &e.board).unwrap();
    let spec_path = dir.join("s.json");
    std::fs::write(
        &spec_path,
        serde_json::to_vec(&common::power::power_tree()).unwrap(),
    )
    .unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(["extract", "--board"])
        .arg(&board)
        .arg("--spec")
        .arg(&spec_path)
        .output()
        .unwrap();
    let v: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(out.status.code(), Some(0), "{v}");
    assert_eq!(v["verdict"], "pass", "{v}");
    std::fs::remove_dir_all(&dir).unwrap();
}

/// K10: the battery's rail is `VBAT` on a board with a power tree, so a net
/// the specification spells `VBAT`, in any case, is refused by name.
#[test]
fn a_net_spelled_vbat_beside_a_power_tree_is_refused() {
    let mut v = common::power::power_tree();
    v["i2c_buses"][0]["sda"]["net"] = "vbat".into();
    for c in v["net_constraints"].as_array_mut().unwrap() {
        if c["net_name"] == "I2C_SDA" {
            c["net_name"] = "vbat".into();
        }
    }
    let spec: Specification = serde_json::from_value(v).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    match schematic::emit(&spec, &solved, &table) {
        Err(e @ EmitError::ReservedName(_)) => assert!(e.to_string().contains("VBAT"), "{e}"),
        other => panic!("{other:?}"),
    }
}
