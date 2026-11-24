//! Each connector's opening, judged on the board as read (`AET-WO-4300k`
//! (4), A1): the side its row's drawing puts the mating opening on
//! ([`aethelgard_ir::connectors::opening`], cited by page), turned by the
//! footprint's rotation, must face out of the board at its edge: the
//! courtyard's extreme on that side within [`TOLERANCE_NM`] of the outline's
//! edge on the same side. A connector drawn facing the board, or along an
//! edge, is refused by name. It reads the board, never the placer's plan.

use aethelgard_ir::connectors::{self, Toward};

use crate::route::{self, Board};

/// How far inside the outline a courtyard's opening side may sit: the
/// placer puts it within one snap (0.05 mm) of the edge; twice that is
/// allowed here.
pub const TOLERANCE_NM: i64 = 100_000;

/// One connector the board draws facing the wrong way.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Inward {
    pub reference: String,
    pub message: String,
}

/// Every connector of `board` whose part `value_of` names a row with an
/// opening: how many were judged, and the first that does not open off the
/// board.
pub fn judge(board: &Board, value_of: impl Fn(&str) -> Option<String>) -> (usize, Option<Inward>) {
    let mut judged = 0;
    for part in &board.parts {
        let Some(value) = value_of(&part.reference) else {
            continue;
        };
        let Some(o) = connectors::opening(&value) else {
            continue;
        };
        judged += 1;
        let local = match o.toward {
            Toward::PlusY => (0, 1),
            Toward::MinusY => (0, -1),
        };
        let d = route::rotate(part.rotation, local);
        let xs = part.courtyard.iter().flat_map(|(a, b)| [a.0, b.0]);
        let ys = part.courtyard.iter().flat_map(|(a, b)| [a.1, b.1]);
        let (Some(x0), Some(x1), Some(y0), Some(y1)) =
            (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
        else {
            return (
                judged,
                Some(Inward {
                    reference: part.reference.clone(),
                    message: format!(
                        "{} ({value}) has no courtyard to judge its opening by",
                        part.reference
                    ),
                }),
            );
        };
        let o = board.outline;
        let (gap, edge) = match d {
            (0, 1) => (o.y1 - y1, "bottom"),
            (0, -1) => (y0 - o.y0, "top"),
            (1, 0) => (o.x1 - x1, "right"),
            _ => (x0 - o.x0, "left"),
        };
        if !(0..=TOLERANCE_NM).contains(&gap) {
            return (
                judged,
                Some(Inward {
                    reference: part.reference.clone(),
                    message: format!(
                        "{} ({value}) at rotation {} opens toward the board's {edge} edge, and its courtyard is {gap} nm from it, not within {TOLERANCE_NM} nm: its mating opening ({}) does not face out of the board",
                        part.reference,
                        part.rotation,
                        o_cite(&value)
                    ),
                }),
            );
        }
    }
    (judged, None)
}

fn o_cite(value: &str) -> String {
    connectors::opening(value).map_or_else(String::new, |o| {
        format!("{}, {}", o.cited.document, o.cited.section)
    })
}
