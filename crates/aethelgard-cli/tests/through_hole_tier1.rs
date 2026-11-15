//! `AET-WO-1309`: through-hole pads and thermal-via arrays, judged by the
//! gate's own functions on `eda`-level fixtures (FEEDBACK D1). KiCad's
//! verdict on the same boards is `with_tools.rs`'s.

mod common;

use aethelgard_eda::stack::Join;
use common::th;

/// The XT30 shape routes at 2 and 4 layers with either join, and the board
/// read back passes the gate's reader, connectivity and check.
#[test]
fn the_xt30_board_routes_and_passes_the_gates_judges() {
    for four in [false, true] {
        for join in [Join::Solid, Join::Relief] {
            let (stack, text) = th::write(th::xt30_board(four, join))
                .unwrap_or_else(|e| panic!("four {four}, {join:?}: {e}"));
            th::judge(&stack, &text).unwrap_or_else(|e| panic!("four {four}, {join:?}: {e}"));
            assert_eq!(text.matches("thru_hole").count(), 4);
        }
    }
}

/// The thermal fixture routes into the 4-layer ground plane with its 4 x 3
/// array at the rule's sites, and passes the gate's judges read back.
#[test]
fn the_thermal_board_carries_the_declared_array_and_passes() {
    let (stack, text) = th::write(th::thermal_board(
        true,
        Some(th::lmr33630_rule(&["In1.Cu"])),
        false,
    ))
    .unwrap();
    th::judge(&stack, &text).unwrap();
    let thermal = text.matches("(drill 0.254)").count();
    assert_eq!(thermal, 12, "{text}");
}

fn thermal(
    rule: aethelgard_ir::thermal::ThermalVias,
) -> Result<(aethelgard_eda::stack::Stack, String), String> {
    th::write(th::thermal_board(true, Some(rule), false))
}

/// `AET-WO-1309` §1.2: a footprint that carries vias of its own under the
/// declared pad (KiCad's `_ThermalVias` shape: 8 through-hole pads numbered
/// 9, 0.3 mm) is refused by name, naming what contradicts the rule.
#[test]
fn a_footprint_with_its_own_vias_under_a_declared_pad_is_refused() {
    let e = th::write(th::thermal_board(
        true,
        Some(th::lmr33630_rule(&["In1.Cu"])),
        true,
    ))
    .unwrap_err();
    assert!(
        e.contains("U5 pad 9: the footprint carries 8 vias of its own")
            && e.contains("3 x 4 drilled 254000 nm"),
        "{e}"
    );
}

/// `AET-WO-1309` §1.2: a rule whose vias cannot sit inside the pad's copper
/// (a 1.5 mm pitch across the 2.95 mm pad) is refused by name.
#[test]
fn a_rule_the_pad_cannot_hold_is_refused() {
    let mut rule = th::lmr33630_rule(&["In1.Cu"]);
    rule.pitch_um = 1500;
    let e = thermal(rule).unwrap_err();
    assert!(
        e.contains("U5 pad 9: the array's via at") && e.contains("leaves the pad's copper"),
        "{e}"
    );
}

/// `AET-WO-1309` §1.2: every via joins the pad's own plane; a rule naming
/// another net's plane is refused by name.
#[test]
fn a_rule_layer_that_is_not_the_pads_fill_is_refused() {
    let e = thermal(th::lmr33630_rule(&["In2.Cu"])).unwrap_err();
    assert!(
        e.contains("the rule's layer In2.Cu is not a fill of GND"),
        "{e}"
    );
}

/// `AET-WO-1309` §2 and §5.2: the gate judges the array from the declared
/// rule, never from the board. A board with a via missing, one moved off
/// its site, every drill changed, an array laid by another rule, or the
/// array with no rule declared at all, is refused by name.
#[test]
fn the_gate_judges_the_array_from_the_rule_not_the_board() {
    let (stack, text) = thermal(th::lmr33630_rule(&["In1.Cu"])).unwrap();
    th::judge(&stack, &text).unwrap();
    // The array's vias, each a `(via` item holding the rule's drill.
    let sites: Vec<usize> = text
        .match_indices("(via")
        .map(|(i, _)| i)
        .filter(|&i| {
            text[i..]
                .split("(uuid")
                .next()
                .is_some_and(|v| v.contains("(drill 0.254)"))
        })
        .collect();
    assert_eq!(sites.len(), 12);
    // One via missing: 11 of 12.
    let at = sites[0];
    let end = at + text[at..].find("\n\t(").unwrap_or(text.len() - at);
    let missing = format!("{}{}", &text[..at], &text[end + 1..]);
    let e = th::judge(&stack, &missing).unwrap_err();
    assert!(
        e.contains("thermal array has 11 vias; the rule declares 12"),
        "{e}"
    );
    // Every drill 0.3 mm.
    let drilled = text.replace("(drill 0.254)", "(drill 0.3)");
    let e = th::judge(&stack, &drilled).unwrap_err();
    assert!(
        e.contains("vias 554000/300000 nm; the rule declares 554000/254000 nm"),
        "{e}"
    );
    // An array laid by a 3 x 2 rule, judged by the 4 x 3 one.
    let mut two = th::lmr33630_rule(&["In1.Cu"]);
    two.rows = 2;
    let (_, fewer) = thermal(two).unwrap();
    let e = th::judge(&stack, &fewer).unwrap_err();
    assert!(
        e.contains("thermal array has 6 vias; the rule declares 12"),
        "{e}"
    );
    // An array laid by a 1.1 mm pitch, judged by the 1.0 mm one.
    let mut wide = th::lmr33630_rule(&["In1.Cu"]);
    wide.pitch_um = 1100;
    let (_, moved) = thermal(wide).unwrap();
    let e = th::judge(&stack, &moved).unwrap_err();
    assert!(e.contains("is not at a site the rule's pitch gives"), "{e}");
    // No array declared (the CLI's `Stack::of` today): the vias are not the
    // board's own via, and are refused.
    let mut bare = stack.clone();
    bare.thermal.clear();
    let e = th::judge(&bare, &text).unwrap_err();
    assert!(e.contains("a via Some(554000)/Some(254000)"), "{e}");
}

/// `AET-WO-1309` F5 (`INV-06`): a through-hole pad on a filled net with no
/// declared join is refused by name, by the router and by the gate.
#[test]
fn a_through_hole_pad_on_a_filled_net_needs_a_declared_join() {
    for four in [false, true] {
        let (stack, fps, outline) = th::xt30_board(four, aethelgard_eda::stack::Join::Solid);
        let (_, text) = th::write((stack.clone(), fps.clone(), outline)).unwrap();
        let mut none = stack;
        none.joins.clear();
        let e = th::write((none.clone(), fps, outline)).unwrap_err();
        assert!(
            e.contains("J1 pad 2 is a through-hole pad on GND"),
            "{four}: {e}"
        );
        // The board's pads carry the join it was laid with; judged by a
        // specification that declares none, they disagree with it.
        let e = th::judge(&none, &text).unwrap_err();
        assert!(
            e.contains("joins a fill Some(Solid) by its own `zone_connect`; the specification declares None"),
            "{four}: {e}"
        );
        // And a join declared other than the board's.
        let mut relief = none.clone();
        relief.joins = vec![("J1".into(), aethelgard_eda::stack::Join::Relief)];
        let e = th::judge(&relief, &text).unwrap_err();
        assert!(e.contains("declares Some(Relief)"), "{four}: {e}");
    }
}

/// `AET-WO-1309`: a declared reference the gate would have to quote in a
/// KiCad rule, and cannot as a literal, is refused by name.
#[test]
fn a_name_the_gates_rules_cannot_quote_is_refused() {
    let (mut stack, text) =
        th::write(th::xt30_board(false, aethelgard_eda::stack::Join::Solid)).unwrap();
    stack
        .joins
        .push(("J1') || ('".to_owned(), aethelgard_eda::stack::Join::Solid));
    let e = th::judge(&stack, &text).unwrap_err();
    assert!(e.contains("is not a plain literal"), "{e}");
}

/// `AET-WO-1309` F6: extraction reads a board with through-hole pads and a
/// thermal array (it refused any drilled pad before), and bounds a
/// through-hole pad as a via is bounded, around its whole box: at least the
/// bound of a via of the same diameter, whose disk the box holds.
#[test]
fn extraction_reads_through_hole_pads_and_arrays() {
    for fixture in [
        th::xt30_board(true, aethelgard_eda::stack::Join::Solid),
        th::thermal_board(true, Some(th::lmr33630_rule(&["In1.Cu"])), false),
    ] {
        let (stack, text) = th::write(fixture).unwrap();
        let read = aethelgard_eda::extract::read(&text, &stack).unwrap_or_else(|e| panic!("{e:?}"));
        assert!(
            read.board
                .parts
                .iter()
                .any(|p| p.pads.iter().any(|p| p.drill_nm.is_some()))
                || !read.routed.arrays.is_empty()
        );
    }
    let ph = aethelgard_eda::extract::Physics::of(&th::stack(true));
    let pad = aethelgard_eda::pcb::Rect {
        x0: 0,
        y0: 0,
        x1: 600_000,
        y1: 600_000,
    };
    let (box_ff, disk_ff) = (
        aethelgard_eda::extract::through_hole_ff(&ph, pad).unwrap(),
        aethelgard_eda::extract::via_ff(&ph).unwrap(),
    );
    assert!(box_ff >= disk_ff, "{box_ff} < {disk_ff}");
}

// ---- each guard of the gate's own judges, killed one by one (mutation pass) ----

use aethelgard_eda::pcb::Rect;
use aethelgard_eda::route::{self, Array, Board, Layer, Pad, Part, Routed, Segment, Via};

const MM: i64 = 1_000_000;

fn square(
    number: &str,
    net: Option<&str>,
    (x, y): (i64, i64),
    side: i64,
    drill: Option<i64>,
) -> Pad {
    Pad {
        number: number.into(),
        net: net.map(str::to_owned),
        rect: Rect {
            x0: x - side / 2,
            y0: y - side / 2,
            x1: x + side / 2,
            y1: y + side / 2,
        },
        corner_nm: if drill.is_some() { side / 2 } else { 0 },
        drill_nm: drill,
        join: None,
    }
}

fn part(reference: &str, pads: Vec<Pad>) -> Part {
    Part {
        reference: reference.into(),
        rotation: 0,
        pads,
        courtyard: Vec::new(),
    }
}

/// A 2-layer board 40 x 10 mm: GND pads P1 (x 10) and P2 (x 30), each
/// joined on F.Cu to a GND via beside it, and the B.Cu ground zone. With
/// `wall`, a column of unnetted 3.5 mm through-hole pads at x 20 cuts the
/// zone in two.
fn walled(wall: bool) -> (Board, Routed) {
    let mut parts = vec![
        part(
            "P1",
            vec![square("1", Some("GND"), (10 * MM, 5 * MM), MM, None)],
        ),
        part(
            "P2",
            vec![square("1", Some("GND"), (30 * MM, 5 * MM), MM, None)],
        ),
    ];
    if wall {
        let holes = (0..4)
            .map(|i| square("", None, (20 * MM, i * 3_500_000), 3_500_000, Some(MM)))
            .collect();
        parts.push(part("W", holes));
    }
    let seg = |a: (i64, i64), b: (i64, i64)| Segment {
        net: "GND".into(),
        layer: Layer::F,
        start: a,
        end: b,
    };
    let routed = Routed {
        segments: vec![
            seg((10 * MM, 5 * MM), (12 * MM, 5 * MM)),
            seg((28 * MM, 5 * MM), (30 * MM, 5 * MM)),
        ],
        vias: vec![
            Via {
                net: "GND".into(),
                at: (12 * MM, 5 * MM),
            },
            Via {
                net: "GND".into(),
                at: (28 * MM, 5 * MM),
            },
        ],
        zones: vec![("GND".into(), "B.Cu".into())],
        ..Routed::default()
    };
    let board = Board {
        outline: Rect {
            x0: 0,
            y0: 0,
            x1: 40 * MM,
            y1: 10 * MM,
        },
        parts,
        stack: th::stack(false),
    };
    (board, routed)
}

/// A wall of foreign through-hole pads cuts the ground zone: the two halves'
/// vias are not one fill (the cut by foreign through-hole pads).
#[test]
fn foreign_through_hole_pads_cut_a_fill() {
    let (board, routed) = walled(false);
    route::connectivity(&board, &routed).unwrap();
    let (board, routed) = walled(true);
    let e = route::connectivity(&board, &routed).unwrap_err();
    assert!(e.contains("does not reach via"), "{e}");
}

/// A thermal array the fill cannot reach (beyond the wall) is refused, and
/// so is one on a net that has no fill at all.
#[test]
fn a_thermal_array_must_reach_its_nets_fill() {
    let (mut board, mut routed) = walled(true);
    // P2's pad becomes an exposed pad with one thermal via; its F.Cu stub
    // and via go.
    board.parts[1].pads[0].number = "9".into();
    board.parts[1].pads[0].rect = Rect {
        x0: 29 * MM,
        y0: 4 * MM,
        x1: 31 * MM,
        y1: 6 * MM,
    };
    routed.segments.pop();
    routed.vias.pop();
    let array = |net: &str| Array {
        reference: "P2".into(),
        pad: "9".into(),
        net: net.into(),
        via_nm: 554_000,
        drill_nm: 254_000,
        at: vec![(30 * MM, 5 * MM)],
    };
    routed.arrays = vec![array("GND")];
    let e = route::connectivity(&board, &routed).unwrap_err();
    assert!(e.contains("does not reach thermal via"), "{e}");
    // On a net with no fill: nothing joins the via but its pad.
    board.parts[1].pads[0].net = Some("VBAT".into());
    routed.arrays = vec![array("VBAT")];
    let e = route::connectivity(&board, &routed).unwrap_err();
    assert!(e.contains("joins no fill of its net"), "{e}");
}

/// A relieved pad meets its fill only where the whole ring past its gap is
/// sure fill: a foreign hole close by, cutting part of the ring, refuses it.
#[test]
fn a_relieved_pad_needs_its_whole_ring_filled() {
    let (mut board, routed) = walled(false);
    board.stack.joins = vec![("J".into(), aethelgard_eda::stack::Join::Relief)];
    let mut j = square("1", Some("GND"), (20 * MM, 5 * MM), 2 * MM, Some(MM));
    j.join = Some(aethelgard_eda::stack::Join::Relief);
    board.parts.push(part("J", vec![j]));
    route::connectivity(&board, &routed).unwrap();
    // An unnetted 1 mm hole 0.9 mm from J's edge: its cut reaches into J's
    // relief ring on that side only.
    board.parts.push(part(
        "H",
        vec![square("", None, (22_400_000, 5 * MM), MM, Some(500_000))],
    ));
    let e = route::connectivity(&board, &routed).unwrap_err();
    assert!(
        e.contains("J pad 1 meets the B.Cu fill of GND nowhere"),
        "{e}"
    );
}

/// Two arrays declared under one pad are refused by name.
#[test]
fn two_arrays_under_one_pad_are_refused() {
    let (mut stack, fps, outline) =
        th::thermal_board(true, Some(th::lmr33630_rule(&["In1.Cu"])), false);
    stack.thermal.push(stack.thermal[0].clone());
    let e = th::write((stack, fps, outline)).unwrap_err();
    assert!(
        e.contains("U5 pad 9: two arrays are declared under it"),
        "{e}"
    );
}

/// The reader pins the zone's relief gap and spoke to the engine's, and
/// refuses an array whose vias differ among themselves (the last resized).
#[test]
fn the_reader_pins_relief_and_reads_an_array_whole() {
    let (stack, text) =
        th::write(th::xt30_board(true, aethelgard_eda::stack::Join::Relief)).unwrap();
    let wider = text.replacen("(thermal_gap 0.5)", "(thermal_gap 0.6)", 1);
    assert_ne!(wider, text);
    let e = th::judge(&stack, &wider).unwrap_err();
    assert!(
        e.contains("relief gap and spoke are not the engine's"),
        "{e}"
    );
    let (stack, text) = thermal(th::lmr33630_rule(&["In1.Cu"])).unwrap();
    let last = text.rfind("(size 0.554)").unwrap();
    let resized = format!(
        "{}(size 0.6){}",
        &text[..last],
        &text[last + "(size 0.554)".len()..]
    );
    let e = th::judge(&stack, &resized).unwrap_err();
    assert!(
        e.contains("the thermal vias under U5 pad 9 differ in size, drill or net"),
        "{e}"
    );
}

/// Extraction prices a through-hole pad by its through-hole bound, an SMD
/// pad by its pad bound.
#[test]
fn extraction_prices_each_pad_by_its_kind() {
    let ph = aethelgard_eda::extract::Physics::of(&th::stack(true));
    let th_pad = square("1", Some("A"), (0, 0), 3_500_000, Some(1_900_000));
    let smd = square("1", Some("A"), (0, 0), 3_500_000, None);
    assert_eq!(
        aethelgard_eda::extract::pad_bound(&ph, &th_pad),
        aethelgard_eda::extract::through_hole_ff(&ph, th_pad.rect)
    );
    assert_eq!(
        aethelgard_eda::extract::pad_bound(&ph, &smd),
        aethelgard_eda::extract::pad_ff(&ph, smd.rect)
    );
    assert_ne!(
        aethelgard_eda::extract::pad_bound(&ph, &th_pad),
        aethelgard_eda::extract::pad_bound(&ph, &smd)
    );
}

// ---- the adversarial pass (FEEDBACK §5): each finding's refusal ----

/// `text` with `new` put in once, right after the first `after`.
fn after(text: &str, after: &str, new: &str) -> String {
    let i = text.find(after).unwrap_or_else(|| panic!("no `{after}`")) + after.len();
    format!("{}{new}{}", &text[..i], &text[i..])
}

/// Copper and fills the gate does not model are refused by name, never
/// judged as if they were not there: a drilled hole with no copper inside
/// the thermal pad (F1), a backdrill on a thermal via or a through-hole
/// pad (F2), a hatched or otherwise non-engine fill or zone (F3).
#[test]
fn what_the_judges_do_not_model_is_refused() {
    let (stack, text) = thermal(th::lmr33630_rule(&["In1.Cu"])).unwrap();
    let (xstack, xtext) =
        th::write(th::xt30_board(true, aethelgard_eda::stack::Join::Relief)).unwrap();
    let cases = [
        (
            &stack,
            text.replacen(
                "(pad \"1\" smd rect",
                "(pad \"\" np_thru_hole circle (at 1.2 2.2) (size 0.4 0.4) (drill 0.4) (layers \"*.Mask\"))\n\t\t(pad \"1\" smd rect",
                1,
            ),
            "U5 pad  is a drilled hole with no copper",
        ),
        (
            &stack,
            after(
                &text,
                "(drill 0.254)",
                " (backdrill (size 0.5) (layers \"B.Cu\" \"In1.Cu\"))",
            ),
            "a via has `backdrill`",
        ),
        (
            &xstack,
            after(
                &xtext,
                "(drill 1.9)",
                " (backdrill (size 2.0) (layers \"B.Cu\" \"In1.Cu\"))",
            ),
            "sets its own `backdrill`",
        ),
        (
            &stack,
            after(&text, "(island_removal_mode 0)", " (mode hatch)"),
            "its fill has `mode`",
        ),
        (
            &stack,
            text.replacen("(island_removal_mode 0)", "(island_removal_mode 1)", 1),
            "is not the engine's solid fill",
        ),
        (
            &stack,
            text.replacen("(min_thickness 0.25)", "(min_thickness 0.2)", 1),
            "is not the engine's solid fill",
        ),
        (
            &stack,
            after(&text, "(min_thickness 0.25)", " (priority 1)"),
            "has `priority`, which the gate does not model",
        ),
    ];
    for (stack, board, want) in cases {
        assert_ne!(board, text);
        let e = th::judge(stack, &board).unwrap_err();
        assert!(e.contains(want), "{want}: {e}");
    }
}

/// A declared join for a part the board does not have, or one with no
/// through-hole pad, is refused (`INV-06`; F9).
#[test]
fn a_join_for_a_part_with_no_through_hole_pad_is_refused() {
    let (mut stack, text) =
        th::write(th::xt30_board(false, aethelgard_eda::stack::Join::Solid)).unwrap();
    for absent in ["R1", "J9"] {
        stack
            .joins
            .push((absent.into(), aethelgard_eda::stack::Join::Solid));
        let e = th::judge(&stack, &text).unwrap_err();
        assert!(
            e.contains(&format!(
                "declares a join for {absent}, and the board has no through-hole pad"
            )),
            "{e}"
        );
        stack.joins.pop();
    }
}

/// Extraction reads a footprint turned to 270 degrees (its angle is not a
/// coordinate; F7).
#[test]
fn extraction_reads_a_part_at_270_degrees() {
    let (stack, mut fps, outline) = th::xt30_board(true, aethelgard_eda::stack::Join::Solid);
    let r1 = fps
        .iter_mut()
        .find(|f| f.property("Reference") == Some("R1"))
        .unwrap();
    let at = r1
        .items_mut()
        .unwrap()
        .iter_mut()
        .find(|i| i.head() == Some("at"))
        .unwrap();
    at.items_mut()
        .unwrap()
        .push(aethelgard_eda::sexpr::Sx::atom("270"));
    let (stack, text) = th::write((stack, fps, outline)).unwrap();
    assert!(text.contains("(at 122 116 270)"), "R1 is not turned");
    aethelgard_eda::extract::read(&text, &stack).unwrap_or_else(|e| panic!("{e:?}"));
}

/// The array's declared pad must be exactly one SMD pad of one part, with a
/// net; and a declared array the board lacks is refused.
#[test]
fn a_declared_array_needs_its_pad_and_its_vias() {
    let rule = th::lmr33630_rule(&["In1.Cu"]);
    for (reference, pad, want) in [
        ("U9", "9", "the board has 0 parts U9"),
        ("U5", "10", "U5 pad 10: 0 SMD pads of that number"),
    ] {
        let (mut stack, fps, outline) = th::thermal_board(true, None, false);
        stack.thermal = vec![aethelgard_eda::stack::ThermalPad::of(reference, pad, &rule)];
        let e = th::write((stack, fps, outline)).unwrap_err();
        assert!(e.contains(want), "{e}");
    }
    let (mut stack, mut fps, outline) = th::thermal_board(true, Some(rule.clone()), false);
    let mut nets = [None; 9];
    nets[0] = Some("GND");
    nets[1] = Some("VIN");
    fps[0] = th::hsop8("U5", (110.0, 112.0), nets, false);
    stack.thermal = vec![aethelgard_eda::stack::ThermalPad::of("U5", "9", &rule)];
    let e = th::write((stack, fps, outline)).unwrap_err();
    assert!(e.contains("U5 pad 9 has no net"), "{e}");
    // Laid with no array, judged by a specification that declares one.
    let (_, bare) = th::write(th::thermal_board(true, None, false)).unwrap();
    let (stack, _) = thermal(rule).unwrap();
    let e = th::judge(&stack, &bare).unwrap_err();
    assert!(
        e.contains("U5 pad 9 has no thermal array; the specification declares one"),
        "{e}"
    );
}
