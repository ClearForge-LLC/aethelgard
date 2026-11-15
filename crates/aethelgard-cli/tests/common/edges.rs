//! The reference with a part on each board edge (`AET-WO-1308` §1.5), for
//! `classes_tier1` (routed and read back) and `with_tools` (ERC and DRC).

use std::fs;
use std::path::Path;

use aethelgard_eda::schematic::Emission;
use aethelgard_eda::stack::Stack;

/// The reference emitted with U2 on the left edge at 90, U3 on the top at 0,
/// U4 on the right at 270 and R1 on the bottom at 180. U2 and U4 are not
/// symmetric under a quarter turn, so a rotation applied the wrong way round
/// puts copper where KiCad does not. U3 is not turned 180: KiCad keeps its
/// reference upright, which then meets its pin-1 mark (`silk_overlap`,
/// measured; FEEDBACK).
pub fn moved() -> (aethelgard_ir::Specification, Emission) {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let plain = aethelgard_eda::schematic::emit(&spec, &solved, &table).unwrap();
    let tree = aethelgard_eda::strict::board(&plain.board).unwrap();
    let (b, _) = aethelgard_eda::route::read(&tree, &Stack::of(&spec)).unwrap();
    let o = b.outline;
    let (cx, cy) = ((o.x0 + o.x1) / 2, (o.y0 + o.y1) / 2);
    let mm = 1_000_000;
    let moves = vec![
        ("U2".to_owned(), (o.x0 - 2 * mm, cy), 90),
        ("U3".to_owned(), (cx, o.y0 - 2 * mm), 0),
        ("U4".to_owned(), (o.x1 + 4 * mm, cy), 270),
        ("R1".to_owned(), (cx, o.y1 + 2 * mm), 180),
    ];
    let moved = aethelgard_eda::schematic::emit_moved(&spec, &solved, &table, &moves).unwrap();
    (spec, moved)
}
