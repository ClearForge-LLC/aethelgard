//! AET-WO-1301 §5, the adversarial pass on the placed board and on the
//! board the parity gate prepares, as regression tests. Each attack was
//! measured against KiCad 10.0.7 in the tools image (`container/IMAGE`,
//! `--network none`) at 6b55cb3. A test marked **red until fixed** pins an
//! attack that succeeded there; it asserts the secure outcome and fails
//! until the code changes.

use aethelgard_eda::board::prepare;
use aethelgard_eda::library;
use aethelgard_eda::pcb;
use aethelgard_eda::rules::SETUP;

/// A footprint with real copper on two nets, given `attr`.
fn board_with(attr: &str) -> String {
    format!(
        "(kicad_pcb\n\t(version 20260206)\n\t(footprint \"Resistor_SMD:R_0402_1005Metric\"\n\t\t(layer \"F.Cu\")\n\t\t(at 115 105)\n\t\t(property \"Reference\" \"R99\"\n\t\t\t(at 0 -1.17 0)\n\t\t\t(layer \"F.SilkS\")\n\t\t)\n\t\t({attr})\n\t\t(pad \"1\" smd roundrect\n\t\t\t(at -0.51 0)\n\t\t\t(size 0.54 0.64)\n\t\t\t(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")\n\t\t\t(net \"VDD_3V3\")\n\t\t)\n\t\t(pad \"2\" smd roundrect\n\t\t\t(at 0.51 0)\n\t\t\t(size 0.54 0.64)\n\t\t\t(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")\n\t\t\t(net \"GND\")\n\t\t)\n\t)\n)\n"
    )
}

/// **Red until fixed.** KiCad's parity check skips a footprint whose
/// attributes include `board_only`: it is never an extra footprint, and a
/// second footprint under an existing reference is never a duplicate
/// (`drc_test_provider_schematic_parity.cpp`). Measured on the emitted
/// reference board: a copy of R5 as R99 with `(attr smd board_only)`, and a
/// second R5 valued `0R` with it, each judged with nothing beyond
/// `unconnected_items` (65 → 67), verdict `fail` only because the board is
/// unrouted. So real copper the schematic does not hold passes parity. The
/// emitter never writes `board_only`; the gate should refuse it as it
/// refuses `allow_missing_courtyard` (`board.rs` `waives`).
#[test]
fn attack_a_board_only_footprint_is_refused() {
    assert!(prepare(&board_with("attr smd"), SETUP).is_ok());
    for attr in [
        "attr smd board_only",
        "attr board_only",
        "attr through_hole board_only exclude_from_bom",
    ] {
        assert!(
            prepare(&board_with(attr), SETUP).is_err(),
            "({attr}) hides a footprint from parity"
        );
    }
}

/// KiCad 10.0.7's stroke font, measured in the tools image by plotting
/// F.SilkS to Gerber (`kicad-cli pcb export gerbers`), size 1 mm, stroke
/// 0.15 mm: `m` is the widest glyph `RUL-14` admits (every byte 0x21-0x7E
/// but `/{}$[]` was plotted ten times over), its advance is 4/3 of the font
/// width, and 64 of them are 85.007 mm of ink, pen included.
const SIXTY_FOUR_M_INK_NM: i64 = 85_007_000;

/// **Red until fixed.** The placer's keep-out box takes every glyph as 1.2
/// font widths, so a 64-byte reference of `m` is 77.1 mm wide to it and
/// 85.0 mm on the board. Measured: the emitted boards of specifications
/// whose device references are `U` + 63 × `m` (and `@`, 1.263 per glyph)
/// report `silk_edge_clearance`, `silk_overlap` (against another part's
/// reference text and silkscreen) and `silk_over_copper` (over the MCU's
/// pads) beyond the unconnected items. The box must hold KiCad's ink.
#[test]
fn attack_the_keep_box_holds_the_widest_reference_kicad_draws() {
    let reference = format!("U{}", "m".repeat(63));
    for id in [
        "Resistor_SMD:R_0402_1005Metric",
        "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
        "Package_SO:SOIC-8_5.3x5.3mm_P1.27mm",
        "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
    ] {
        let g = pcb::geometry(&library::footprint(id).unwrap(), &reference).unwrap();
        assert!(
            g.keep.x1 - g.keep.x0 >= SIXTY_FOUR_M_INK_NM,
            "{id}: the keep box is {} nm wide, KiCad's reference is {SIXTY_FOUR_M_INK_NM}",
            g.keep.x1 - g.keep.x0
        );
    }
}
