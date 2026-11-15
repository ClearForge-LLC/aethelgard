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
