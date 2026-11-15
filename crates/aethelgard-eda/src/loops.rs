//! The power tree's layout rules, judged from a placed board (`AET-WO-4300d2`
//! §1.3, §2): each part a regulator's datasheet asks to sit by one of its pins
//! is measured there, copper edge to copper edge, against the bound its rule
//! states. The rules and their bounds are the emitter's anchors
//! (`schematic::power`), derived from the tree's declared ownership, so the
//! gate judges exactly what the placer was asked to meet. Every rule a board
//! cannot show is a named condition ([`conditions`]), never a silent pass.
//!
//! The gap between two pads is exact: a rounded rectangle is its inner box
//! widened by a disc of its corner radius, so the distance between two pads
//! is the distance between their inner boxes less both radii (0 when they
//! touch). The board is read by `route::read` and its labels proved by
//! `route::connectivity` before any rule is judged, so a pad's net here is
//! the one its copper says (`RUL-03`).

use aethelgard_ir::{PinTable, SolvedTopology, Specification};

use crate::route::{Board, Pad};
use crate::schematic::EmitError;

/// One part's place by its IC, as the emitter anchored it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopRule {
    pub part: String,
    pub ic: String,
    /// Each of the part's pads with the IC pad it must lie beside.
    pub pads: Vec<(String, String)>,
    pub bound_nm: i64,
    pub rule: &'static str,
}

/// The rules of `spec`'s power tree: empty when it declares none.
pub fn rules(
    spec: &Specification,
    solved: &SolvedTopology,
    table: &PinTable,
) -> Result<Vec<LoopRule>, EmitError> {
    if spec.power.is_none() {
        return Ok(Vec::new());
    }
    let (parts, _) = crate::schematic::design(spec, solved, table)?;
    Ok(parts
        .into_iter()
        .filter_map(|p| {
            let o = p.owner?;
            let a = o.anchor?;
            let (bound_nm, rule) = a.loop_rule?;
            Some(LoopRule {
                part: p.reference,
                ic: o.ic,
                pads: a.pads,
                bound_nm,
                rule,
            })
        })
        .collect())
}

/// One pad pair's measured gap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PadGap {
    pub pad: String,
    pub pin: String,
    pub gap_um: i64,
}

/// One rule as the board measures it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoopRecord {
    pub part: String,
    pub ic: String,
    pub pads: Vec<PadGap>,
    pub bound_um: i64,
    pub rule: &'static str,
    pub verdict: &'static str,
}

fn pad<'b>(board: &'b Board, reference: &str, number: &str) -> Result<&'b Pad, String> {
    let part = board
        .parts
        .iter()
        .find(|p| p.reference == reference)
        .ok_or_else(|| format!("the board has no part `{reference}`"))?;
    let mut found = part.pads.iter().filter(|p| p.number == number);
    match (found.next(), found.next()) {
        (Some(p), None) => Ok(p),
        (None, _) => Err(format!("`{reference}` has no pad `{number}`")),
        (Some(_), Some(_)) => Err(format!("`{reference}` has two pads `{number}`")),
    }
}

/// The copper-edge distance between two pads, in nanometres, exact for
/// rectangles, rounded rectangles and circles.
pub fn gap_nm(a: &Pad, b: &Pad) -> i64 {
    let d2 = i128::from(crate::route::gap2(a.inner(), b.inner()));
    let d = i64::try_from(crate::pcb::isqrt_up(d2)).unwrap_or(i64::MAX);
    (d - a.corner_nm - b.corner_nm).max(0)
}

/// Each rule measured on `board`. A part or pad the rule names that the board
/// lacks is refused, never skipped.
pub fn judge(board: &Board, rules: &[LoopRule]) -> Result<Vec<LoopRecord>, String> {
    let mut out = Vec::new();
    for r in rules {
        let mut pads = Vec::new();
        let mut worst = 0;
        for (own, pin) in &r.pads {
            let g = gap_nm(pad(board, &r.part, own)?, pad(board, &r.ic, pin)?);
            worst = worst.max(g);
            pads.push(PadGap {
                pad: own.clone(),
                pin: pin.clone(),
                gap_um: g.div_euclid(1000) + i64::from(g.rem_euclid(1000) != 0),
            });
        }
        out.push(LoopRecord {
            part: r.part.clone(),
            ic: r.ic.clone(),
            pads,
            bound_um: r.bound_nm / 1000,
            rule: r.rule,
            verdict: if worst <= r.bound_nm { "pass" } else { "fail" },
        });
    }
    Ok(out)
}

/// `K5` on the board: each domain's source pad is on the domain's net and its
/// return pad on `GND`, as `SolvedTopology` states them for `AET-WO-1303`.
/// A board whose divider is turned, or on another net, is refused here.
pub fn source_pads(board: &Board, solved: &SolvedTopology) -> Result<(), String> {
    let Some(p) = &solved.power else {
        return Ok(());
    };
    for (domain, d) in &p.domains {
        for (which, at, want) in [
            ("source", &d.source, crate::pcb::escape(domain)),
            ("return", &d.ret, crate::schematic::GND.to_owned()),
        ] {
            let got = pad(board, &at.reference, &at.pad)?.net.clone();
            if got.as_deref() != Some(want.as_str()) {
                return Err(format!(
                    "`{domain}`'s {which} pad, {} pad {}, is on {}, not `{want}` (K5)",
                    at.reference,
                    at.pad,
                    got.map_or_else(|| "no net".to_owned(), |n| format!("`{n}`")),
                ));
            }
        }
    }
    Ok(())
}

/// What the gate does not judge on a board with a power tree, each named once
/// in the order the datasheets give them (`AET-WO-4300d2` §1.3, §1.4; FAB-NOTES
/// carries them too, the `4300f` F2 rule). Empty for a specification with no
/// tree.
pub fn conditions(spec: &Specification) -> Vec<String> {
    let Some(p) = &spec.power else {
        return Vec::new();
    };
    let mut out = vec![
        "LAYOUT: each loop part's distance to its pin is judged against an engine bound; SNVSAN3F, SBVS320D, SCES935A and AN2834 state \"close\" or \"as close as possible\" and no distance".to_owned(),
        "LAYOUT: not judged: the input loop's area (SNVSAN3F p. 33, Figure 10-1), the SW node's copper area (item 9), the feedback trace's distance from SW (item 4), the PGND side of the ground planes (p. 34, 10.1.1)".to_owned(),
        "LAYOUT: not judged: the width of the traces SNVSAN3F 10.1 asks to be short and wide (item 2, CVCC; item 3, CBOOT; item 7, VIN, VOUT and GND), the copper weight and heat-sinking area (item 8), and the LDO's copper planes and thermal vias (SBVS320D p. 21, 7.4.1)".to_owned(),
        "LAYOUT: not judged: the ground side of a part whose rule names one pin (the LDO's capacitors, each translator bypass, the sense reservoir, CVCC)".to_owned(),
        "LAYOUT: not judged: the placement of each buck's output capacitors, each sense divider's resistors and each crossing's pull-up".to_owned(),
        "CLASSES: each declared class's width is not judged against the current it carries: no current-to-width source is held; a buck's SW net is laid at its output domain's class".to_owned(),
        "SILKSCREEN: each loop part's reference is on F.Fab, not the silkscreen".to_owned(),
    ];
    if spec.board.copper_layers == 2 {
        out.push(
            "LAYOUT: not met: SNVSAN3F p. 33, 10.1 item 5 asks for a ground plane on an inner layer; this board has two copper layers".to_owned(),
        );
    }
    if !p.crossings.is_empty() {
        out.push(
            "LINES: each line on a translator's B side is not bounded; the translator drives it"
                .to_owned(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::gap_nm;
    use crate::pcb::Rect;
    use crate::route::Pad;

    fn pad(x0: i64, y0: i64, x1: i64, y1: i64, corner_nm: i64) -> Pad {
        Pad {
            number: "1".into(),
            net: None,
            rect: Rect { x0, y0, x1, y1 },
            corner_nm,
            drill_nm: None,
            join: None,
        }
    }

    #[test]
    fn the_gap_is_exact_for_squares_rounded_boxes_and_circles() {
        // Two squares 1 mm apart along x.
        assert_eq!(
            gap_nm(&pad(0, 0, 1000, 1000, 0), &pad(2000, 0, 3000, 1000, 0)),
            1000
        );
        // Touching or overlapping: 0.
        assert_eq!(
            gap_nm(&pad(0, 0, 1000, 1000, 0), &pad(500, 0, 3000, 1000, 0)),
            0
        );
        // Two circles of radius 500 whose centres are (3000, 4000) apart, 5000
        // on the diagonal: 5000 - 500 - 500.
        assert_eq!(
            gap_nm(
                &pad(0, 0, 1000, 1000, 500),
                &pad(3000, 4000, 4000, 5000, 500)
            ),
            4000
        );
        // A rounded corner is farther than its box: corners 300, boxes 1 mm
        // 1.6 mm apart on both axes inside their roundings: 1.6 × √2 mm less
        // the two radii, rounded up.
        assert_eq!(
            gap_nm(
                &pad(0, 0, 1000, 1000, 300),
                &pad(2000, 2000, 3000, 3000, 300)
            ),
            1663
        );
    }
}
