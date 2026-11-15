//! `AET-WO-4300d2`: the power tree on the board, judged without KiCad. The
//! Sentinel-02 fixture (without its crossing, which waits for the
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
/// `validate`).
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
    let (_, solved, _) = fixture();
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

/// §5.3 (adversarial pass A1): a thermal array short of its rule, or a via
/// off its site, is refused by `route::check`, which `check drc --spec` runs on
/// a power tree's board; the array as emitted passes.
#[test]
fn a_starved_thermal_array_is_refused() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let tree = strict::board(&e.board).unwrap();
    let (b, r) = route::read(&tree, &Stack::of(&spec)).unwrap();
    route::check(&b, &r).unwrap();
    assert_eq!(r.arrays.len(), 2);
    let mut short = r.clone();
    short.arrays[0].at.pop();
    assert!(route::check(&b, &short).is_err());
    let mut moved = r.clone();
    moved.arrays[0].at[0].0 += 300_000;
    assert!(route::check(&b, &moved).is_err());
}

// ---- The crossing and the IMU behind it (`AET-WO-4300d2` D2 (a), K6, D6, D7) ----

/// The vendored ICM-20948 footprint, as its bytes.
fn icm_footprint() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(
        "../aethelgard-kicad-lib/data/kicad-10.0.7/footprints/Sensor_Motion.pretty/InvenSense_QFN-24_3x3mm_P0.4mm.kicad_mod",
    );
    std::fs::read_to_string(path).unwrap()
}

fn full() -> (Specification, SolvedTopology, PinTable) {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../aethelgard-ir/tests/fixtures/sentinel-02-power.json");
    let spec: Specification = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

/// K6 and D2 (a): the committed fixture, crossing and all, validates and is
/// drawn. Its board does not route yet (D7: the ICM-20948's 0.2 mm pads at
/// 0.4 mm pitch, under the 200 um floor on the 125 um grid), refused by name;
/// and `extract` refuses the IMU's MCU-side lines by name (D6 (b)): the
/// translator's data pins have no stated maximum capacitance.
#[test]
fn the_crossing_is_drawn_and_its_limits_are_refused_by_name() {
    let (spec, solved, table) = full();
    match schematic::emit(&spec, &solved, &table) {
        Err(e @ EmitError::Unroutable { .. }) => assert!(e.to_string().contains("U5"), "{e}"),
        other => panic!("{other:?}"),
    }
    match aethelgard_eda::extract::lines(&spec, &solved, &table) {
        Err(r) => {
            assert_eq!(r.kind, "TranslatorPinUnbounded", "{r:?}");
            assert!(r.message.contains("U13"), "{r:?}");
        }
        Ok(_) => panic!("the IMU's lines were bounded"),
    }
}

/// D2 (a): the ICM-20948's keepout is read from its vendored footprint as a
/// rectangle on F.Cu forbidding tracks, vias, pads and fill; a track or a via
/// inside it is refused by `route::check`, and one beside it is not.
#[test]
fn a_keepout_is_read_routed_around_and_judged() {
    let mut text = icm_footprint();
    text = text.replacen(
        "(footprint \"InvenSense_QFN-24_3x3mm_P0.4mm\"",
        "(footprint \"InvenSense_QFN-24_3x3mm_P0.4mm\" (at 100 100)",
        1,
    );
    text = text.replacen("\"REF**\"", "\"U5\"", 1);
    let part = route::part_of(&aethelgard_eda::sexpr::parse(&text).unwrap()).unwrap();
    assert_eq!(part.keepouts.len(), 1);
    let k = &part.keepouts[0];
    assert_eq!(k.layer, route::Layer::F);
    assert!(k.tracks && k.vias && k.pads && k.fill);
    assert_eq!(
        (k.rect.x0, k.rect.y0, k.rect.x1, k.rect.y1),
        (99_125_000, 99_205_000, 100_875_000, 100_795_000)
    );
    // The keepout alone: the gate's pad clearance measures pads by their
    // boxes, which read the QFN's corner pads 1 and 24 as 0.177 mm apart
    // (0.218 mm as rounded copper; FEEDBACK D7), so the pads are left out.
    let mut part = part;
    part.pads.clear();
    let (spec, _, _) = fixture();
    let board = route::Board {
        outline: aethelgard_eda::pcb::Rect {
            x0: 90_000_000,
            y0: 90_000_000,
            x1: 110_000_000,
            y1: 110_000_000,
        },
        parts: vec![part],
        stack: {
            // The fixture's board, without its power parts' arrays and joins.
            let mut stack = Stack::of(&spec);
            stack.thermal.clear();
            stack.joins.clear();
            stack
        },
    };
    let track = |y: i64| route::Routed {
        segments: vec![route::Segment {
            net: "/X".into(),
            layer: route::Layer::F,
            start: (95_000_000, y),
            end: (96_000_000, y),
        }],
        ..Default::default()
    };
    route::check(&board, &track(95_000_000)).unwrap();
    let mut inside = track(95_000_000);
    inside.segments[0].start = (100_000_000, 100_000_000);
    inside.segments[0].end = (100_000_000, 100_500_000);
    let err = route::check(&board, &inside).unwrap_err();
    assert!(err.contains("keepout"), "{err}");
    let via = route::Routed {
        vias: vec![route::Via {
            net: "/X".into(),
            at: (100_000_000, 100_000_000),
        }],
        ..Default::default()
    };
    let err = route::check(&board, &via).unwrap_err();
    assert!(err.contains("keepout"), "{err}");
    // A footprint zone that is not a keepout is refused, never dropped.
    let copper = text.replacen("(keepout", "(fill_keepout_removed", 1);
    let err = route::part_of(&aethelgard_eda::sexpr::parse(&copper).unwrap()).unwrap_err();
    assert!(err.contains("not a keepout"), "{err}");
}

/// `RUL-03` as amended for D2 (a): the strict reader admits a keepout's own
/// `(connect_pads (clearance N))` inside a footprint, and nothing more: the
/// same clearance on a zone that is not a footprint's keepout, or a keepout's
/// `connect_pads` carrying anything else, is still a local clearance.
#[test]
fn the_reader_admits_a_footprints_keepout_clearance_and_nothing_more() {
    let (spec, solved, table) = fixture();
    let e = schematic::emit(&spec, &solved, &table).unwrap();
    let lib = icm_footprint()
        .replacen(
            "(footprint \"InvenSense_QFN-24_3x3mm_P0.4mm\"",
            "(footprint \"Sensor_Motion:InvenSense_QFN-24_3x3mm_P0.4mm\" (at 50 50)",
            1,
        )
        .replacen("\"REF**\"", "\"U99\"", 1);
    let with = |fp: &str| {
        let end = e.board.trim_end().rfind(')').unwrap();
        format!("{}{fp}\n)\n", &e.board[..end])
    };
    let prepare = |text: &str| {
        let tree = aethelgard_eda::board::read(text).unwrap();
        aethelgard_eda::board::prepare_read(&tree, text, "(setup)")
    };
    prepare(&with(&lib)).unwrap();
    let extra = lib.replacen("(clearance 0)", "(clearance 0) (unrelated 1)", 1);
    assert_eq!(prepare(&with(&extra)).unwrap_err().kind(), "LocalClearance");
    let not_keepout = lib.replacen("(keepout", "(kept", 1);
    assert_eq!(
        prepare(&with(&not_keepout)).unwrap_err().kind(),
        "LocalClearance"
    );
}

/// D2 (a): the router routes around a keepout. Two pads of one net face each
/// other across it, so the straight track would cross it; the routed board
/// passes `route::check`, which refuses any track inside the keepout.
#[test]
fn the_router_goes_around_a_keepout() {
    let mm = 1_000_000;
    let pad = |reference: &str, x: i64| route::Part {
        reference: reference.into(),
        rotation: 0,
        pads: vec![route::Pad {
            number: "1".into(),
            net: Some("/X".into()),
            rect: aethelgard_eda::pcb::Rect {
                x0: x - mm / 2,
                y0: 10 * mm - mm / 2,
                x1: x + mm / 2,
                y1: 10 * mm + mm / 2,
            },
            corner_nm: 0,
            drill_nm: None,
            join: None,
        }],
        courtyard: Vec::new(),
        keepouts: Vec::new(),
    };
    let wall = route::Part {
        reference: "K1".into(),
        rotation: 0,
        pads: Vec::new(),
        courtyard: Vec::new(),
        keepouts: vec![route::Keepout {
            layer: route::Layer::F,
            rect: aethelgard_eda::pcb::Rect {
                x0: 9 * mm,
                y0: 7 * mm,
                x1: 11 * mm,
                y1: 13 * mm,
            },
            tracks: true,
            vias: true,
            pads: true,
            fill: true,
        }],
    };
    let (spec, _, _) = fixture();
    let mut stack = Stack::of(&spec);
    stack.thermal.clear();
    stack.joins.clear();
    stack.classes.clear();
    let board = route::Board {
        outline: aethelgard_eda::pcb::Rect {
            x0: 0,
            y0: 0,
            x1: 20 * mm,
            y1: 20 * mm,
        },
        parts: vec![pad("R1", 5 * mm), pad("R2", 15 * mm), wall],
        stack,
    };
    let routed = route::route(&board).unwrap();
    route::check(&board, &routed).unwrap();
    assert!(!routed.segments.is_empty());
}
