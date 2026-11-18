//! Supply and ground resistance from a regulator's pad to a load's pins
//! (`AET-WO-1303b`, ruling D5 (b) on PR #222): one net's copper as
//! [`flow`]'s network, and the certified bound between two of its pads.
//!
//! **What is copper here (an inner approximation: Rayleigh lets any copper
//! be left out, which only raises `R`):**
//! - on F.Cu and B.Cu, a square cell of side `h` counts when it lies wholly
//!   inside one of the net's tracks (a stadium), via pads (discs) or pads
//!   (rounded rectangles), or when each of its four, or sixteen, sub-squares
//!   does; and when it is clear of every drilled hole of the net's own
//!   through-hole pads. Coordinates are integers throughout, so the test is
//!   exact;
//! - on an inner plane of the net, a cell counts when it lies inside the
//!   board's outline less `edge_nm`, and clear of every knockout: each other
//!   net's via and every drilled pad, at [`Model::knockout`]'s radius. A
//!   through-hole pad of the net itself is knocked out too: its join to the
//!   plane is not priced;
//! - plane copper outside [`Model::crop_nm`] of the terminals is left out;
//! - every via of the net on the router's grid, its thermal arrays'
//!   included where they are on it, is [`flow::via_ports`]: a junction
//!   square on each layer whose copper holds it wholly, and a barrel through
//!   the stack. A layer that does not hold the square does not join that
//!   via, and a via off the grid is left out with its hole kept clear (left
//!   out, as above).
//!
//! **Stated conditions** (each named in the measurement's output):
//! - every hole concentric with its pad (ruled for the measured stop only);
//! - each terminal pad an equipotential (the current enters and leaves
//!   through the whole pad);
//! - copper no thinner than [`Model`]'s minima, no thicker than the stack's
//!   maximum, at no more than [`Model::rho`];
//! - the plane's fill no smaller than the inset outline less the knockouts.

use std::collections::{BTreeMap, BTreeSet};

use crate::flow::{self, Bound, Cell, FlowError, Hole, Iv, Join, Network, Sheet};
use crate::route::{Board, Layer, Pad, Routed};

/// The physical model's inputs. Lengths in nm unless named otherwise.
#[derive(Clone, Debug)]
pub struct Model {
    /// The cell side: 31 250 or 62 500 nm (exact binary in mm).
    pub h_nm: i64,
    /// Copper's resistivity bound, Ω·mm.
    pub rho: Iv,
    /// Outer and inner copper, minimum and maximum, nm.
    pub outer_nm: (i64, i64),
    pub plane_nm: (i64, i64),
    /// The hole plating's minimum, nm.
    pub plating_nm: i64,
    /// [`flow::junction`]'s band, nm.
    pub beta_nm: i64,
    /// A plane's inset from the outline.
    pub edge_nm: i64,
    /// Plane copper further than this from the terminals' box is left out.
    pub crop_nm: i64,
    /// The fill's measured clearance to foreign copper, nm, and the
    /// min-thickness opening added to it ([`Model::knockout`]).
    pub fill_clearance_nm: i64,
    pub opening_nm: i64,
}

impl Model {
    /// A plane's knockout radius around copper `d` nm across: `d/2` plus
    /// the fill clearance, rounded up to 10 µm, plus the opening.
    pub fn knockout(&self, d: i64) -> i64 {
        let r = d / 2 + d % 2 + self.fill_clearance_nm;
        (r + 9_999) / 10_000 * 10_000 + self.opening_nm
    }
}

/// A convex piece of copper, in nm × 4 (so a cell splits twice exactly).
#[derive(Clone, Copy, Debug)]
enum Shape {
    Disc {
        c: (i128, i128),
        r: i128,
    },
    Stadium {
        a: (i128, i128),
        b: (i128, i128),
        hw: i128,
    },
    /// A rounded rectangle: its inner rectangle and corner radius.
    Round {
        x0: i128,
        y0: i128,
        x1: i128,
        y1: i128,
        r: i128,
    },
}

const S: i128 = 4;

fn sq(v: i128) -> i128 {
    v * v
}

impl Shape {
    fn pad(p: &Pad) -> Shape {
        let r = &p.rect;
        let short = (r.x1 - r.x0).min(r.y1 - r.y0);
        let c = i128::from(p.corner_nm.clamp(0, short / 2));
        let (x0, y0, x1, y1) = (
            S * i128::from(r.x0),
            S * i128::from(r.y0),
            S * i128::from(r.x1),
            S * i128::from(r.y1),
        );
        Shape::Round {
            x0: x0 + S * c,
            y0: y0 + S * c,
            x1: x1 - S * c,
            y1: y1 - S * c,
            r: S * c,
        }
    }

    fn bbox(&self) -> (i128, i128, i128, i128) {
        match *self {
            Shape::Disc { c, r } => (c.0 - r, c.1 - r, c.0 + r, c.1 + r),
            Shape::Stadium { a, b, hw } => (
                a.0.min(b.0) - hw,
                a.1.min(b.1) - hw,
                a.0.max(b.0) + hw,
                a.1.max(b.1) + hw,
            ),
            Shape::Round { x0, y0, x1, y1, r } => (x0 - r, y0 - r, x1 + r, y1 + r),
        }
    }

    fn holds(&self, p: (i128, i128)) -> bool {
        match *self {
            Shape::Disc { c, r } => sq(p.0 - c.0) + sq(p.1 - c.1) <= sq(r),
            Shape::Stadium { a, b, hw } => {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let (px, py) = (p.0 - a.0, p.1 - a.1);
                let dot = px * dx + py * dy;
                let l2 = dx * dx + dy * dy;
                if dot <= 0 || l2 == 0 {
                    sq(px) + sq(py) <= sq(hw)
                } else if dot >= l2 {
                    sq(p.0 - b.0) + sq(p.1 - b.1) <= sq(hw)
                } else {
                    let cross = px * dy - py * dx;
                    // cross² ≤ hw² l2, in i128: |cross| < 2⁶⁴ here
                    let (c2, h2) = (cross.unsigned_abs().pow(2), (hw * hw) as u128 * l2 as u128);
                    c2 <= h2
                }
            }
            Shape::Round { x0, y0, x1, y1, r } => {
                let dx = (x0 - p.0).max(0).max(p.0 - x1);
                let dy = (y0 - p.1).max(0).max(p.1 - y1);
                sq(dx) + sq(dy) <= sq(r)
            }
        }
    }

    /// The square `[x0, x1] × [y0, y1]` lies wholly inside (convex: its
    /// corners do).
    fn holds_square(&self, x0: i128, y0: i128, x1: i128, y1: i128) -> bool {
        [(x0, y0), (x1, y0), (x0, y1), (x1, y1)]
            .iter()
            .all(|&p| self.holds(p))
    }
}

/// The square is clear of the open disc: its nearest point is at least `r`
/// from `c`.
fn clear(x0: i128, y0: i128, x1: i128, y1: i128, c: (i128, i128), r: i128) -> bool {
    let dx = (x0 - c.0).max(0).max(c.0 - x1);
    let dy = (y0 - c.1).max(0).max(c.1 - y1);
    sq(dx) + sq(dy) >= sq(r)
}

fn in_union(
    shapes: &[Shape],
    cand: &[usize],
    x0: i128,
    y0: i128,
    x1: i128,
    y1: i128,
    depth: u32,
) -> bool {
    if cand.iter().any(|&k| shapes[k].holds_square(x0, y0, x1, y1)) {
        return true;
    }
    if depth == 0 {
        return false;
    }
    let (mx, my) = ((x0 + x1) / 2, (y0 + y1) / 2);
    [
        (x0, y0, mx, my),
        (mx, y0, x1, my),
        (x0, my, mx, y1),
        (mx, my, x1, y1),
    ]
    .iter()
    .all(|&(a, b, c, d)| in_union(shapes, cand, a, b, c, d, depth - 1))
}

/// A via of the net: its centre, pad and drill.
#[derive(Clone, Debug)]
pub struct ViaAt {
    pub at: (i64, i64),
    pub via_nm: i64,
    pub drill_nm: i64,
}

/// One layer's copper cells (junction squares left out), its minimum
/// thickness, nm, and whether it is a plane.
pub struct LayerCopper {
    pub name: String,
    pub cells: Vec<Cell>,
    pub t_min_nm: i64,
    pub plane: bool,
}

/// One net's copper, ready for any pair of terminals.
pub struct NetCopper {
    pub net: String,
    /// The grid's origin: every via centre is a cell corner.
    pub origin: (i64, i64),
    pub h_nm: i64,
    /// Each copper layer, top to bottom.
    pub layers: Vec<LayerCopper>,
    /// Each via, its square's half-side in cells, and the layers (indices
    /// into `layers`) whose copper holds the square.
    pub vias: Vec<(ViaAt, i64, Vec<usize>)>,
    /// The barrel's rows, top to bottom, each a height range in mm; layer
    /// `k`'s copper is row `2k`.
    pub rows: Vec<(Iv, Iv)>,
    /// What was left out, for the record.
    pub notes: Vec<String>,
}

/// Why no bound.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// A named refusal: kind and message.
    Named(&'static str, String),
    Flow(FlowError),
}

fn named(kind: &'static str, m: String) -> Refusal {
    Refusal::Named(kind, m)
}

/// The pad `reference`.`number`, on the net.
pub fn pad<'a>(
    board: &'a Board,
    net: &str,
    reference: &str,
    number: &str,
) -> Result<&'a Pad, Refusal> {
    board
        .parts
        .iter()
        .filter(|p| p.reference == reference)
        .flat_map(|p| p.pads.iter())
        .find(|p| p.number == number && p.net.as_deref() == Some(net))
        .ok_or_else(|| {
            named(
                "TerminalMissing",
                format!("{reference}.{number} is not a pad of `{net}`"),
            )
        })
}

/// The net's copper on the grid, cropped to `crop_nm` around `terminals`.
pub fn copper(
    board: &Board,
    routed: &Routed,
    net: &str,
    terminals: &[&Pad],
    m: &Model,
) -> Result<NetCopper, Refusal> {
    let stack = &board.stack;
    let h = m.h_nm;
    let rule = stack.rule(net);
    let mut vias: Vec<ViaAt> = routed
        .vias
        .iter()
        .filter(|v| v.net == net)
        .map(|v| ViaAt {
            at: v.at,
            via_nm: rule.via_nm,
            drill_nm: rule.drill_nm,
        })
        .collect();
    for a in routed.arrays.iter().filter(|a| a.net == net) {
        vias.extend(a.at.iter().map(|&at| ViaAt {
            at,
            via_nm: a.via_nm,
            drill_nm: a.drill_nm,
        }));
    }
    // the grid's origin: the routed vias' corner (the router's grid); a
    // via off it (a thermal array's) is left out, its hole kept clear
    let origin = routed
        .vias
        .iter()
        .find(|v| v.net == net)
        .or(routed.vias.first())
        .map_or((0, 0), |v| (v.at.0.rem_euclid(h), v.at.1.rem_euclid(h)));
    let on_grid = |v: &ViaAt| {
        (v.at.0 - origin.0).rem_euclid(h) == 0 && (v.at.1 - origin.1).rem_euclid(h) == 0
    };
    let (vias, off): (Vec<ViaAt>, Vec<ViaAt>) = vias.into_iter().partition(on_grid);
    let mut notes = Vec::new();
    if !off.is_empty() {
        notes.push(format!(
            "{} of the net's vias are off the {h} nm grid and left out (their holes kept clear): {:?}",
            off.len(),
            off.iter().map(|v| v.at).collect::<Vec<_>>()
        ));
    }
    let off_holes: Vec<((i128, i128), i128)> = off
        .iter()
        .map(|v| {
            (
                (S * i128::from(v.at.0), S * i128::from(v.at.1)),
                S * i128::from(v.drill_nm) / 2 + S,
            )
        })
        .collect();
    // the crop: the terminals' box, grown
    let mut bx = (i64::MAX, i64::MAX, i64::MIN, i64::MIN);
    for p in terminals {
        bx = (
            bx.0.min(p.rect.x0),
            bx.1.min(p.rect.y0),
            bx.2.max(p.rect.x1),
            bx.3.max(p.rect.y1),
        );
    }
    let o = board.outline;
    let crop = (
        (bx.0 - m.crop_nm).max(o.x0),
        (bx.1 - m.crop_nm).max(o.y0),
        (bx.2 + m.crop_nm).min(o.x1),
        (bx.3 + m.crop_nm).min(o.y1),
    );
    let cell_of = |x: i64| (x - origin.0).div_euclid(h);
    let row_of = |y: i64| (y - origin.1).div_euclid(h);
    let (i0, j0, i1, j1) = (
        cell_of(crop.0),
        row_of(crop.1),
        cell_of(crop.2),
        row_of(crop.3),
    );
    let corner = |i: i64, j: i64| {
        (
            S * i128::from(origin.0 + i * h),
            S * i128::from(origin.1 + j * h),
        )
    };
    let hs = S * i128::from(h);
    // the junction squares: half-side m cells with r_d + β < m h
    let mut squares: Vec<(ViaAt, i64)> = Vec::new();
    for v in vias {
        let need = v.drill_nm / 2 + m.beta_nm;
        let k = need / h + 1;
        squares.push((v, k));
    }
    let mut in_square: BTreeSet<(i64, i64)> = BTreeSet::new();
    for (v, k) in &squares {
        let (ci, cj) = ((v.at.0 - origin.0) / h, (v.at.1 - origin.1) / h);
        for i in ci - k..ci + k {
            for j in cj - k..cj + k {
                if !in_square.insert((i, j)) {
                    return Err(named(
                        "ViaSquaresOverlap",
                        format!("the via at {:?}'s junction square meets another's", v.at),
                    ));
                }
            }
        }
    }
    let own_drilled: Vec<&Pad> = board
        .parts
        .iter()
        .flat_map(|p| p.pads.iter())
        .filter(|p| p.net.as_deref() == Some(net) && p.drill_nm.is_some())
        .collect();
    let mut layers = Vec::new();
    let mut blocked: Vec<BTreeSet<(i64, i64)>> = Vec::new();
    for name in &stack.copper {
        let outer = name == "F.Cu" || name == "B.Cu";
        let plane = stack
            .planes
            .iter()
            .any(|(l, n)| l == name && crate::extract::board_net(n) == net)
            || stack.planes.iter().any(|(l, n)| l == name && n == net);
        let mut cells = Vec::new();
        if outer {
            let layer = if name == "F.Cu" { Layer::F } else { Layer::B };
            let mut shapes = Vec::new();
            let w = i128::from(rule.width_nm);
            for s in routed
                .segments
                .iter()
                .filter(|s| s.net == net && s.layer == layer)
            {
                let a = (S * i128::from(s.start.0), S * i128::from(s.start.1));
                let b = (S * i128::from(s.end.0), S * i128::from(s.end.1));
                shapes.push(Shape::Stadium {
                    a,
                    b,
                    hw: S * w / 2,
                });
            }
            for (v, _) in &squares {
                shapes.push(Shape::Disc {
                    c: (S * i128::from(v.at.0), S * i128::from(v.at.1)),
                    r: S * i128::from(v.via_nm) / 2,
                });
            }
            for p in board.parts.iter().flat_map(|p| p.pads.iter()) {
                if p.net.as_deref() == Some(net) && (p.drill_nm.is_some() || layer == Layer::F) {
                    shapes.push(Shape::pad(p));
                }
            }
            let mut holes: Vec<((i128, i128), i128)> = own_drilled
                .iter()
                .map(|p| {
                    let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
                    (
                        (S * i128::from(c.0), S * i128::from(c.1)),
                        S * i128::from(p.drill_nm.unwrap_or(0)) / 2 + S,
                    )
                })
                .collect();
            holes.extend(off_holes.iter().copied());
            // each shape's cells
            let mut cand: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
            for (k, sh) in shapes.iter().enumerate() {
                let (x0, y0, x1, y1) = sh.bbox();
                let lo_i = ((x0 / S) as i64 - origin.0).div_euclid(h);
                let lo_j = ((y0 / S) as i64 - origin.1).div_euclid(h);
                let hi_i = ((x1 / S) as i64 - origin.0).div_euclid(h);
                let hi_j = ((y1 / S) as i64 - origin.1).div_euclid(h);
                // outer copper is not cropped: it is small
                for i in lo_i..=hi_i {
                    for j in lo_j..=hi_j {
                        cand.entry((i, j)).or_default().push(k);
                    }
                }
            }
            for (&(i, j), ks) in &cand {
                if in_square.contains(&(i, j)) {
                    continue;
                }
                let (x0, y0) = corner(i, j);
                let (x1, y1) = (x0 + hs, y0 + hs);
                if !holes.iter().all(|&(c, r)| clear(x0, y0, x1, y1, c, r)) {
                    continue;
                }
                if in_union(&shapes, ks, x0, y0, x1, y1, 2) {
                    cells.push((i, j));
                }
            }
            layers.push(LayerCopper {
                name: name.clone(),
                cells,
                t_min_nm: m.outer_nm.0,
                plane: false,
            });
            blocked.push(BTreeSet::new());
        } else if plane {
            let inset = (
                o.x0 + m.edge_nm,
                o.y0 + m.edge_nm,
                o.x1 - m.edge_nm,
                o.y1 - m.edge_nm,
            );
            let mut out: BTreeSet<(i64, i64)> = BTreeSet::new();
            let mut knock = |c: (i64, i64), r: i64| {
                let cc = (S * i128::from(c.0), S * i128::from(c.1));
                for i in cell_of(c.0 - r)..=cell_of(c.0 + r) {
                    for j in row_of(c.1 - r)..=row_of(c.1 + r) {
                        let (x0, y0) = corner(i, j);
                        if !clear(x0, y0, x0 + hs, y0 + hs, cc, S * i128::from(r)) {
                            out.insert((i, j));
                        }
                    }
                }
            };
            for v in routed.vias.iter().filter(|v| v.net != net) {
                knock(v.at, m.knockout(stack.rule(&v.net).via_nm));
            }
            for a in routed.arrays.iter().filter(|a| a.net != net) {
                for &at in &a.at {
                    knock(at, m.knockout(a.via_nm));
                }
            }
            for p in board
                .parts
                .iter()
                .flat_map(|p| p.pads.iter())
                .filter(|p| p.drill_nm.is_some())
            {
                let (w, hh) = (
                    (p.rect.x1 - p.rect.x0) as f64,
                    (p.rect.y1 - p.rect.y0) as f64,
                );
                let diag = w.hypot(hh).ceil() as i64 + 1;
                let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
                knock(c, m.knockout(diag));
            }
            for v in &off {
                knock(v.at, v.drill_nm / 2 + 1);
            }
            for i in i0..=i1 {
                for j in j0..=j1 {
                    let (x0, y0) = (origin.0 + i * h, origin.1 + j * h);
                    if x0 >= inset.0
                        && y0 >= inset.1
                        && x0 + h <= inset.2
                        && y0 + h <= inset.3
                        && !out.contains(&(i, j))
                        && !in_square.contains(&(i, j))
                    {
                        cells.push((i, j));
                    }
                }
            }
            if !own_drilled.is_empty() {
                notes.push(format!(
                    "{name}: the net's through-hole pads are knocked out, their join not priced"
                ));
            }
            layers.push(LayerCopper {
                name: name.clone(),
                cells,
                t_min_nm: m.plane_nm.0,
                plane: true,
            });
            blocked.push(out);
        } else {
            layers.push(LayerCopper {
                name: name.clone(),
                cells: Vec::new(),
                t_min_nm: m.plane_nm.0,
                plane: false,
            });
            blocked.push(BTreeSet::new());
        }
    }
    // which layers hold each square wholly: on a plane, the square inside
    // the inset outline and the crop, and none of its cells knocked out; on
    // F.Cu and B.Cu, the square inside the via's own pad
    let mut out_vias = Vec::new();
    for (v, k) in squares {
        let (ci, cj) = ((v.at.0 - origin.0) / h, (v.at.1 - origin.1) / h);
        let mut joined = Vec::new();
        for (li, layer) in layers.iter().enumerate() {
            let name = &layer.name;
            let holds = if layer.plane {
                let inset = (
                    o.x0 + m.edge_nm,
                    o.y0 + m.edge_nm,
                    o.x1 - m.edge_nm,
                    o.y1 - m.edge_nm,
                );
                let (x0, y0) = (origin.0 + (ci - k) * h, origin.1 + (cj - k) * h);
                let (x1, y1) = (origin.0 + (ci + k) * h, origin.1 + (cj + k) * h);
                // the plane joins its own via all round (solid fill): the
                // square holds if it is inside the inset and the crop
                x0 >= inset.0
                    && y0 >= inset.1
                    && x1 <= inset.2
                    && y1 <= inset.3
                    && ci - k >= i0
                    && cj - k >= j0
                    && ci + k <= i1 + 1
                    && cj + k <= j1 + 1
                    && (ci - k..ci + k)
                        .all(|i| (cj - k..cj + k).all(|j| !blocked[li].contains(&(i, j))))
            } else if name == "F.Cu" || name == "B.Cu" {
                // the via pad holds the square when its corner is within
                // the pad's radius
                let a = i128::from(k * h);
                2 * sq(a) <= sq(i128::from(v.via_nm) / 2)
            } else {
                false
            };
            if holds {
                joined.push(li);
            }
        }
        out_vias.push((v, k, joined));
    }
    // the barrel's rows
    let mut rows = Vec::new();
    let mm = |nm: i64| Iv::ratio(nm, 1_000_000);
    let n = stack.copper.len();
    let copper_min: i64 = stack
        .copper
        .iter()
        .map(|l| {
            if l == "F.Cu" || l == "B.Cu" {
                m.outer_nm.0
            } else {
                m.plane_nm.0
            }
        })
        .sum();
    let diel_min = |a: &str, b: &str| -> i64 {
        stack
            .dielectrics
            .iter()
            .find(|d| d.0 == a && d.1 == b)
            .map_or(0, |d| d.2)
    };
    let mins: Vec<i64> = (0..n - 1)
        .map(|k| diel_min(&stack.copper[k], &stack.copper[k + 1]))
        .collect();
    for (k, name) in stack.copper.iter().enumerate() {
        let lo = if name == "F.Cu" || name == "B.Cu" {
            m.outer_nm.0
        } else {
            m.plane_nm.0
        };
        rows.push((mm(lo), mm(stack.copper_max_nm)));
        if k + 1 < n {
            let others: i64 = mins
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != k)
                .map(|(_, v)| *v)
                .sum();
            let hi = stack.thickness.max_nm - copper_min - others;
            let lo = mins[k].max(0);
            if lo == 0 {
                notes.push(format!(
                    "the dielectric under {name} declares no minimum: taken at 0"
                ));
            }
            rows.push((mm(lo.max(1)), mm(hi)));
        }
    }
    Ok(NetCopper {
        net: net.to_owned(),
        origin,
        h_nm: h,
        layers,
        vias: out_vias,
        rows,
        notes,
    })
}

/// The bound between two pads of the net, and for each via the current
/// its barrel carries between layers (quanta of the unit current; the
/// largest net transfer from any one layer).
pub struct PinBound {
    pub bound: Bound,
    /// The bound's share in sheet cells, via junctions and barrels, ohms
    /// (upper ends).
    pub shares: [f64; 3],
    pub via_transfer: Vec<((i64, i64), i64)>,
}

/// The certified bound from `source` to `sink`, both pads of the net on
/// F.Cu, each an equipotential.
pub fn bound(
    nc: &NetCopper,
    m: &Model,
    source: &Pad,
    sink: &Pad,
    max_iter: usize,
) -> Result<PinBound, Refusal> {
    for p in [source, sink] {
        if p.drill_nm.is_some() {
            return Err(named(
                "TerminalDrilled",
                format!(
                    "pad {} is through-hole: its terminal is not modelled",
                    p.number
                ),
            ));
        }
    }
    let h = nc.h_nm;
    let mut net = Network::default();
    let src = net.node();
    let snk = net.node();
    let hs = S * i128::from(h);
    let mut sheets: Vec<Option<Sheet>> = Vec::new();
    for LayerCopper {
        name,
        cells,
        t_min_nm: t_min,
        ..
    } in &nc.layers
    {
        if cells.is_empty() {
            sheets.push(None);
            continue;
        }
        let mut terminals = BTreeMap::new();
        if name == "F.Cu" {
            for (pad, node) in [(source, src), (sink, snk)] {
                let shape = Shape::pad(pad);
                for &(i, j) in cells {
                    let (x0, y0) = (
                        S * i128::from(nc.origin.0 + i * h),
                        S * i128::from(nc.origin.1 + j * h),
                    );
                    if shape.holds_square(x0, y0, x0 + hs, y0 + hs) {
                        terminals.insert((i, j), node);
                    }
                }
            }
        }
        let rho_over_t = m.rho.div(Iv::ratio(*t_min, 1_000_000));
        sheets.push(Some(Sheet::new(&mut net, cells, &terminals, rho_over_t)));
    }
    let h_mm = h as f64 / 1e6;
    // each via's centre and, per joined layer, its port edges
    let mut ports: Vec<(Cell, Vec<Vec<usize>>)> = Vec::new();
    for (v, k, joined) in &nc.vias {
        let hole = Hole {
            r_d: Iv::ratio(v.drill_nm, 2_000_000),
            beta: Iv::ratio(m.beta_nm, 1_000_000),
            t_p: Iv::ratio(m.plating_nm, 1_000_000),
            rho: m.rho,
        };
        let corner = ((v.at.0 - nc.origin.0) / h, (v.at.1 - nc.origin.1) / h);
        let mut joins = Vec::new();
        for (li, s) in sheets.iter_mut().enumerate() {
            if let Some(sheet) = s
                && joined.contains(&li)
            {
                joins.push(Join {
                    sheet,
                    row: 2 * li,
                    t_min: Iv::ratio(nc.layers[li].t_min_nm, 1_000_000),
                });
            }
        }
        let (_, edges) = flow::via_ports(
            &mut net, corner, *k, h_mm, &hole, joins, &nc.rows, None, None,
        );
        ports.push((v.at, edges));
    }
    for s in sheets.into_iter().flatten() {
        s.finish(&mut net);
    }
    let b = net
        .bound(src, snk, max_iter, 1e-10)
        .map_err(Refusal::Flow)?;
    let via_transfer = ports
        .into_iter()
        .map(|(at, per_layer)| {
            let t = per_layer
                .iter()
                .map(|edges| edges.iter().map(|&e| b.flux[e]).sum::<i64>().abs())
                .max()
                .unwrap_or(0);
            (at, t)
        })
        .collect();
    let k = net.energy_by_kind(&b.flux);
    Ok(PinBound {
        shares: [k[0].hi, k[1].hi, k[2].hi],
        bound: b,
        via_transfer,
    })
}
