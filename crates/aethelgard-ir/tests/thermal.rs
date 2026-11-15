//! `thermal::ThermalVias` (`AET-WO-1309` §1.2, FEEDBACK D1 and F4): each
//! refusal is named, in order, and the LMR33630's rule passes.

use aethelgard_ir::board::Board;
use aethelgard_ir::thermal::{HOLE_GAP_UM, ThermalSources, ThermalVias};
use aethelgard_ir::{IrValidationError, Specification};
use serde_json::{Value, json};

/// The reference specification's 2-layer board (rules 300/600 um vias).
fn two_layer() -> Board {
    let v: Value =
        serde_json::from_str(include_str!("../../../examples/sentinel-01-minimal.json")).unwrap();
    let spec: Specification = serde_json::from_value(v).unwrap();
    serde_json::from_value(serde_json::to_value(&spec).unwrap()["board"].clone()).unwrap()
}

/// The same board with four layers, `In1.Cu` a GND plane and `In2.Cu` a signal layer.
fn four_layer() -> Board {
    let mut v = serde_json::to_value(two_layer()).unwrap();
    v["copper_layers"] = json!(4);
    v["layers"] = json!([
        {"name": "F.Cu", "role": "signal", "net": null},
        {"name": "In1.Cu", "role": "plane", "net": "GND"},
        {"name": "In2.Cu", "role": "signal", "net": null},
        {"name": "B.Cu", "role": "signal", "net": null},
    ]);
    serde_json::from_value(v).unwrap()
}

fn source(text: &str) -> String {
    text.to_owned()
}

/// TI LMR33630 SNVSAZ9 §10.1.1's rule as the fixture will declare it.
fn lmr33630(layers: &[&str]) -> ThermalVias {
    ThermalVias {
        columns: 3,
        rows: 4,
        pitch_um: 1000,
        drill_um: 254,
        ring_um: 150,
        layers: layers.iter().map(|l| (*l).to_owned()).collect(),
        sources: ThermalSources {
            count: source("SNVSAZ9 10.1.1 p34"),
            pitch: source("SNVSAZ9 10.1.1 p34, evenly distributed"),
            drill: source("SNVSAZ9 10.1.1 p34, 10 mil"),
            ring: source("the board rules' own ring"),
            layers: source("SNVSAZ9 10.1.1 p34, the ground planes"),
        },
    }
}

fn refusal(v: &ThermalVias, board: &Board) -> IrValidationError {
    v.validate("LMR33630", board).expect_err("refused")
}

/// The LMR33630's rule passes on a 4-layer board whose In1.Cu is the GND plane.
#[test]
fn the_lmr33630_rule_passes_on_a_four_layer_board() {
    let v = lmr33630(&["In1.Cu"]);
    v.validate("LMR33630", &four_layer()).unwrap();
    assert_eq!(v.diameter_um(), 554);
}

/// The same rule passes on a 2-layer board, joining the B.Cu ground zone.
#[test]
fn the_lmr33630_rule_passes_on_a_two_layer_board() {
    lmr33630(&["B.Cu"])
        .validate("LMR33630", &two_layer())
        .unwrap();
}

/// A zero field is refused by name, columns first, then rows, pitch and drill.
#[test]
fn a_zero_field_is_refused_in_order() {
    let b = four_layer();
    let mut v = lmr33630(&["In1.Cu"]);
    v.drill_um = 0;
    let e = refusal(&v, &b);
    assert!(
        matches!(&e, IrValidationError::ThermalViasZero { part, field }
        if part == "LMR33630" && *field == "drill_um")
    );
    assert_eq!(e.kind(), "ThermalViasZero");
    v.pitch_um = 0;
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasZero {
            field: "pitch_um",
            ..
        }
    ));
    v.rows = 0;
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasZero { field: "rows", .. }
    ));
    v.columns = 0;
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasZero {
            field: "columns",
            ..
        }
    ));
}

/// An empty or whitespace-only source is refused by name, in the five-field order.
#[test]
fn an_unsourced_field_is_refused_in_order() {
    let b = four_layer();
    let mut v = lmr33630(&["In1.Cu"]);
    v.sources.layers = "  \t".to_owned();
    let e = refusal(&v, &b);
    assert!(
        matches!(&e, IrValidationError::ThermalViasUnsourced { part, field }
        if part == "LMR33630" && *field == "layers")
    );
    assert_eq!(e.kind(), "ThermalViasUnsourced");
    v.sources.ring = String::new();
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasUnsourced { field: "ring", .. }
    ));
    v.sources.drill = " ".to_owned();
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasUnsourced { field: "drill", .. }
    ));
    v.sources.pitch = String::new();
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasUnsourced { field: "pitch", .. }
    ));
    v.sources.count = String::new();
    assert!(matches!(
        refusal(&v, &b),
        IrValidationError::ThermalViasUnsourced { field: "count", .. }
    ));
}

/// A ring under the board's own via ring (150 um) is refused, and 150 passes.
#[test]
fn a_ring_under_the_boards_own_is_refused() {
    let b = four_layer();
    let mut v = lmr33630(&["In1.Cu"]);
    v.ring_um = 149;
    let e = refusal(&v, &b);
    assert!(
        matches!(&e, IrValidationError::ThermalViaRingUnderRules { part, ring_um: 149, min_um: 150 }
        if part == "LMR33630")
    );
    assert_eq!(e.kind(), "ThermalViaRingUnderRules");
    v.ring_um = 150;
    v.validate("LMR33630", &b).unwrap();
}

/// A pitch under drill plus the hole gap is refused; exactly drill plus gap passes.
#[test]
fn a_pitch_under_the_hole_gap_is_refused() {
    let b = four_layer();
    let mut v = lmr33630(&["In1.Cu"]);
    v.pitch_um = 254 + HOLE_GAP_UM - 1;
    let e = refusal(&v, &b);
    assert!(
        matches!(&e, IrValidationError::ThermalViaPitch { part, pitch_um: 503, min_um: 504 }
        if part == "LMR33630")
    );
    assert_eq!(e.kind(), "ThermalViaPitch");
    v.pitch_um = 254 + HOLE_GAP_UM;
    v.validate("LMR33630", &b).unwrap();
    // A drill near u32::MAX cannot overflow the minimum.
    v.drill_um = u32::MAX;
    v.ring_um = 150;
    v.pitch_um = u32::MAX;
    assert!(
        matches!(refusal(&v, &b), IrValidationError::ThermalViaPitch { min_um, .. }
        if min_um == u64::from(u32::MAX) + 250)
    );
}

/// An empty layer list is refused with layer "" and the right kind.
#[test]
fn an_empty_layer_list_is_refused() {
    let e = refusal(&lmr33630(&[]), &four_layer());
    assert!(
        matches!(&e, IrValidationError::ThermalViaLayers { part, layer, .. }
        if part == "LMR33630" && layer.is_empty())
    );
    assert_eq!(e.kind(), "ThermalViaLayers");
}

/// A layer named twice is refused, naming the layer.
#[test]
fn a_repeated_layer_is_refused() {
    let e = refusal(&lmr33630(&["In1.Cu", "In1.Cu"]), &four_layer());
    assert!(
        matches!(&e, IrValidationError::ThermalViaLayers { layer, rule, .. }
        if layer == "In1.Cu" && rule.contains("twice"))
    );
    assert_eq!(e.kind(), "ThermalViaLayers");
}

/// F.Cu, the pad's own layer, is refused as a target.
#[test]
fn the_pads_own_layer_is_refused() {
    let e = refusal(&lmr33630(&["F.Cu"]), &four_layer());
    assert!(
        matches!(&e, IrValidationError::ThermalViaLayers { layer, rule, .. }
        if layer == "F.Cu" && rule.contains("own layer"))
    );
    assert_eq!(e.kind(), "ThermalViaLayers");
}

/// A signal layer, an unknown layer, or B.Cu on a board whose B.Cu is no zone is refused.
#[test]
fn a_layer_the_vias_cannot_join_is_refused() {
    for (layers, board) in [
        (["In2.Cu"], four_layer()),
        (["In9.Cu"], four_layer()),
        (["B.Cu"], four_layer()),
        (["In1.Cu"], two_layer()),
    ] {
        let e = refusal(&lmr33630(&layers), &board);
        assert!(
            matches!(&e, IrValidationError::ThermalViaLayers { layer, rule, .. }
                if layer == layers[0] && rule.contains("can join")),
            "{layers:?}: {e:?}"
        );
        assert_eq!(e.kind(), "ThermalViaLayers");
    }
}

/// The refusals come in the documented order: zero before source before ring before pitch before layers.
#[test]
fn the_refusals_keep_their_order() {
    let b = four_layer();
    let mut v = lmr33630(&[]);
    v.pitch_um = 10;
    v.ring_um = 10;
    v.sources.count = String::new();
    v.columns = 0;
    assert_eq!(refusal(&v, &b).kind(), "ThermalViasZero");
    v.columns = 3;
    assert_eq!(refusal(&v, &b).kind(), "ThermalViasUnsourced");
    v.sources.count = "x".to_owned();
    assert_eq!(refusal(&v, &b).kind(), "ThermalViaRingUnderRules");
    v.ring_um = 150;
    assert_eq!(refusal(&v, &b).kind(), "ThermalViaPitch");
    v.pitch_um = 1000;
    assert_eq!(refusal(&v, &b).kind(), "ThermalViaLayers");
}
