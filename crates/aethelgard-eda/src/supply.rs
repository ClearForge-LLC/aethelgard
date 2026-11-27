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
//!   board's outline less [`Model::edge_nm`] and clear of every knockout
//!   (each other net's via and every drilled pad, at
//!   [`Model::knockout_raw`]'s radius) widened by the fill's min-thickness
//!   opening; a cell inside that opening ring counts only where a witness
//!   disc admits it (`witnessed`). A through-hole pad of the net that is
//!   relieved joins no plane: its spokes are not priced (a note);
//! - plane copper outside [`Model::crop_nm`] of the terminals' box is left
//!   out, and so is every plane island no via port reaches, as KiCad drops
//!   islands;
//! - every via of the net on the router's grid, and each through-hole pad
//!   of the net on it (priced as a via with the pad's drill), is
//!   [`flow::via`]: a junction region on each layer whose copper holds it,
//!   priced at the worst hole position within [`Model::hole_offset_nm`] per
//!   axis ([`flow::junction_table`]), and a barrel through the stack. An
//!   outer layer joins only through a port whose rays to the hole lie in
//!   copper and clear of every hole. A via whose pad cannot hold the band
//!   at every hole position, or off the grid, is left out with its hole
//!   kept clear (a note).
//!
//! **Stated conditions:** the verdict carries them, from `conditions`.

use std::collections::{BTreeMap, BTreeSet};

use crate::flow::{
    self, Bound, Cell, FlowError, Hole, Iv, Join, JunctionTable, Network, Region, Sheet,
};
use crate::route::{Board, Layer, Pad, Routed};

/// The physical model's inputs. Lengths in nm unless named otherwise.
#[derive(Clone, Debug)]
pub struct Model {
    /// The cell side, nm ([`PITCH_NM`] in the gate).
    pub h_nm: i64,
    /// Copper's resistivity bound, Ω·mm.
    pub rho: Iv,
    /// Outer and inner copper, minimum and maximum, nm.
    pub outer_nm: (i64, i64),
    pub plane_nm: (i64, i64),
    /// The hole plating's minimum, nm.
    pub plating_nm: i64,
    /// [`flow::junction_core`]'s band, nm.
    pub beta_nm: i64,
    /// The fabricator's hole position tolerance, per axis, nm: every hole
    /// is priced at every position within it.
    pub hole_offset_nm: i64,
    /// The fabricator's finished-hole tolerance, larger and smaller than
    /// drawn, nm: every hole is priced at every size within it.
    pub hole_plus_nm: i64,
    pub hole_minus_nm: i64,
    /// The fabricator's track width tolerance, per mille: a track is priced
    /// at `(1000 − this)/1000` of its drawn width.
    pub track_etch_permille: i64,
    /// A plane's inset from the outline.
    pub edge_nm: i64,
    /// Plane copper further than this from the terminals' box is left out.
    pub crop_nm: i64,
    /// The fill's measured clearance to foreign copper, nm, and the
    /// min-thickness opening added to it ([`Model::knockout_raw`]).
    pub fill_clearance_nm: i64,
    pub opening_nm: i64,
}

impl Model {
    /// The fill's own knockout around copper `d` nm across, before the
    /// min-thickness opening: `d/2` plus the fill clearance, rounded up to
    /// 10 µm, plus [`ARC_NM`].
    pub fn knockout_raw(&self, d: i64) -> i64 {
        let r = d / 2 + d % 2 + self.fill_clearance_nm;
        (r + 9_999) / 10_000 * 10_000 + ARC_NM
    }
}

/// KiCad's arc-approximation error, taken against us on both sides of the
/// opening: a knockout's polygon may reach this much past its circle, and
/// the opening's disc this much short of its radius (KiCad's
/// `ARC_HIGH_DEF`, 0.005 mm; a condition the refill check measures).
pub const ARC_NM: i64 = 5_000;

/// A cell near a fill knockout is copper when it lies within `cover` of a
/// witness point that is at least `clear` from every knockout
/// `(centre, raw radius)` in `near` and inside `inner` (the raw outline
/// shrunk by `clear`). KiCad's opening deflates the raw fill by the
/// min-thickness half-width and inflates it back: a point `clear` (the
/// half-width plus the arc error) inside the raw fill survives the
/// deflate, and the inflate covers every point within `cover` (the
/// half-width less the arc error) of it. Candidates: the cell's centre,
/// pushed by up to `cover` away from each knockout, away from all
/// together, and along the eight compass directions. Exact in integers for
/// the candidate tried; a sufficient test.
fn witnessed(
    cell: (i64, i64, i64),
    near: &[((i64, i64), i64)],
    inner: (i64, i64, i64, i64),
    cover: i64,
    clear: i64,
) -> bool {
    let w = cover;
    let (x0, y0, h) = cell;
    let cc = (x0 as f64 + h as f64 / 2.0, y0 as f64 + h as f64 / 2.0);
    let mut dirs: Vec<(f64, f64)> = Vec::new();
    let (mut sx, mut sy) = (0.0, 0.0);
    for &((kx, ky), _) in near {
        let (dx, dy) = (cc.0 - kx as f64, cc.1 - ky as f64);
        let d = dx.hypot(dy).max(1.0);
        dirs.push((dx / d, dy / d));
        sx += dx / d;
        sy += dy / d;
    }
    let n = sx.hypot(sy);
    if n > 0.0 {
        dirs.push((sx / n, sy / n));
    }
    for k in 0..8 {
        let a = std::f64::consts::FRAC_PI_4 * k as f64;
        dirs.push((a.cos(), a.sin()));
    }
    let corners = [(x0, y0), (x0 + h, y0), (x0, y0 + h), (x0 + h, y0 + h)];
    let w2 = i128::from(w) * i128::from(w);
    std::iter::once((0.0, 0.0))
        .chain(dirs.iter().flat_map(|&(ux, uy)| {
            (1..=8).map(move |k| {
                let t = w as f64 * k as f64 / 8.0;
                (ux * t, uy * t)
            })
        }))
        .any(|(px, py)| {
            let c = ((cc.0 + px).round() as i64, (cc.1 + py).round() as i64);
            c.0 >= inner.0
                && c.1 >= inner.1
                && c.0 <= inner.2
                && c.1 <= inner.3
                && corners
                    .iter()
                    .all(|&(px, py)| sq(i128::from(px - c.0)) + sq(i128::from(py - c.1)) <= w2)
                && near.iter().all(|&((kx, ky), r)| {
                    let reach = i128::from(r) + i128::from(clear);
                    sq(i128::from(c.0 - kx)) + sq(i128::from(c.1 - ky)) >= reach * reach
                })
        })
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

/// A via of the net: its centre, pad and drill. A through-hole pad of the
/// net is one too (the §7.1 row: its barrel takes the pad's drill), its pad
/// the largest disc its copper holds.
#[derive(Clone, Debug)]
pub struct ViaAt {
    pub at: (i64, i64),
    pub via_nm: i64,
    pub drill_nm: i64,
    /// A through-hole pad whose plane join is a thermal relief: its spokes
    /// are not priced, so it joins no plane (`ReliefUnpriced`, noted).
    pub relief: bool,
}

/// One layer's copper cells (junction regions left out), its minimum
/// thickness, nm, and whether it is a plane.
pub struct LayerCopper {
    pub name: String,
    pub cells: Vec<Cell>,
    pub t_min_nm: i64,
    pub plane: bool,
}

/// A via geometry's junction region and its faces priced over every hole
/// position ([`flow::junction_table`]).
pub struct Junction {
    pub region: Region,
    pub table: JunctionTable,
}

/// The tolerance square is cut into this many boxes a side for
/// [`flow::junction_table`].
pub const OFFSET_BOXES: usize = 16;

/// A via the network uses: where it is, its junction (an index into
/// [`NetCopper::junctions`]), and each layer it joins with that layer's
/// port faces.
pub struct ViaUse {
    pub via: ViaAt,
    pub junction: usize,
    pub joins: Vec<(usize, Vec<usize>)>,
}

/// One net's copper, ready for any pair of terminals.
pub struct NetCopper {
    pub net: String,
    /// The grid's origin: every via centre is a cell corner.
    pub origin: (i64, i64),
    pub h_nm: i64,
    /// Each copper layer, top to bottom.
    pub layers: Vec<LayerCopper>,
    pub junctions: Vec<Junction>,
    pub vias: Vec<ViaUse>,
    /// The barrel's rows, top to bottom, each a height range in mm; layer
    /// `k`'s copper is row `2k`.
    pub rows: Vec<(Iv, Iv)>,
    /// What was left out, for the record.
    pub notes: Vec<String>,
    /// Each via geometry left out because its pad cannot hold the band
    /// (pad, drill), with the pad that would, nm.
    pub unfit: Vec<(i64, i64, i64)>,
}

/// Why no bound.
#[derive(Clone, Debug, PartialEq)]
pub enum Refusal {
    /// A named refusal: kind and message.
    Named(&'static str, String),
    Flow(FlowError),
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
            Refusal::Named(
                "TerminalMissing",
                format!("{reference}.{number} is not a pad of `{net}`"),
            )
        })
}

/// The plating shell's outer radius, nm, at the smallest and the largest
/// finished hole: the drawn hole less or plus the fabricator's tolerance,
/// halved, plus the plating's minimum. Plating at least that thick lines
/// the finished wall, and the layer's copper meets it, so the shell at
/// either radius is copper; the actual radius is anywhere between.
fn shell_radii(drill_nm: i64, m: &Model) -> (i64, i64) {
    let lo = (drill_nm - m.hole_minus_nm) / 2 + m.plating_nm;
    let hi = (drill_nm + m.hole_plus_nm + 1) / 2 + m.plating_nm;
    (lo, hi)
}

/// The shell radii, mm, in [`HOLE_SIZE_SLICES`] closed intervals covering
/// `[lo, hi]` nm: each a junction table's own, so the worst over the hole's
/// size is a worst over boxes.
fn shell_slices(lo: i64, hi: i64) -> Vec<Iv> {
    let n = HOLE_SIZE_SLICES as i64;
    (0..n)
        .map(|k| {
            let a = lo + (hi - lo) * k / n;
            let b = lo + ((hi - lo) * (k + 1) + n - 1) / n;
            Iv {
                lo: Iv::ratio(a, 1_000_000).lo,
                hi: Iv::ratio(b, 1_000_000).hi,
            }
        })
        .collect()
}

/// The hole a via's barrel is priced at. The barrel's sheet coefficient
/// `ρ·r/A(r)`, `A = t(2r − t)/2`, falls as `r` grows
/// (`d/dr r/(2r − t) = −t/(2r − t)²`): its worst is at the smallest shell.
/// The columns' widths come from the junction table, over every radius.
fn barrel_hole(drill_nm: i64, m: &Model) -> Hole {
    Hole {
        r_d: Iv::ratio(shell_radii(drill_nm, m).0, 1_000_000),
        beta: Iv::ratio(m.beta_nm, 1_000_000),
        t_p: Iv::ratio(m.plating_nm, 1_000_000),
        rho: m.rho,
    }
}

/// A hole kept clear of the net's copper, nm: the largest finished hole at
/// every position the tolerances allow.
fn hole_clear(drill_nm: i64, m: &Model) -> i64 {
    hole_radius(drill_nm + m.hole_plus_nm, m.hole_offset_nm)
}

/// A via's junction reach, nm: the largest shell radius (the hole drilled
/// `drill_nm` at its largest finished size, plus the plating's minimum),
/// the band `beta_nm`, and the furthest the tolerance moves the hole's
/// centre, `⌈e·√2⌉`, plus 1. Its region is the cells meeting the open disc
/// of this radius ([`Region::new`]). The router keeps a judged net's vias
/// apart by the same regions (`AET-WO-1324`), so the two read one formula.
pub fn junction_reach(
    drill_nm: i64,
    hole_plus_nm: i64,
    plating_nm: i64,
    beta_nm: i64,
    hole_offset_nm: i64,
) -> i64 {
    (drill_nm + hole_plus_nm + 1) / 2 + plating_nm + beta_nm + hole_radius(0, hole_offset_nm) + 1
}

/// The via pad, nm, that holds the band at the largest shell and every
/// position: twice the junction reach, rounded up to 10 µm.
fn pad_needed(drill_nm: i64, m: &Model) -> i64 {
    let r = junction_reach(
        drill_nm,
        m.hole_plus_nm,
        m.plating_nm,
        m.beta_nm,
        m.hole_offset_nm,
    );
    (2 * r + 9_999) / 10_000 * 10_000
}

/// A hole's clearance radius in nm: its drill's radius plus the largest
/// distance the tolerance moves its centre, `e·√2` (per axis), rounded up.
fn hole_radius(drill_nm: i64, e_nm: i64) -> i64 {
    let r = drill_nm / 2 + drill_nm % 2;
    r + crate::stack::diagonal_nm(e_nm)
}

/// The convex hull `hull` (nm) lies inside the union of `shapes` (nm × 4):
/// each square of a cover of the hull's box that meets the hull lies wholly
/// inside one shape, splitting each square into four up to `depth` times.
/// The cover's side is rounded up to a multiple of `2^depth`, so every
/// square is integral and the test exact (at the shapes' own scale, where
/// the stadium test's products stay inside `u128`).
fn hull_in_copper(hull: &[(i64, i64)], shapes: &[Shape], depth: u32) -> bool {
    let x0 = S * i128::from(hull.iter().map(|p| p.0).min().unwrap());
    let y0 = S * i128::from(hull.iter().map(|p| p.1).min().unwrap());
    let w = S * i128::from(hull.iter().map(|p| p.0).max().unwrap()) - x0;
    let hgt = S * i128::from(hull.iter().map(|p| p.1).max().unwrap()) - y0;
    let unit = 1i128 << depth;
    let side = (w.max(hgt).max(1) + unit - 1) / unit * unit;
    let hq: Vec<(i128, i128)> = hull
        .iter()
        .map(|&(x, y)| (S * i128::from(x), S * i128::from(y)))
        .collect();
    fn go(
        hq: &[(i128, i128)],
        shapes: &[Shape],
        x0: i128,
        y0: i128,
        side: i128,
        depth: u32,
    ) -> bool {
        if !flow::overlaps(hq, (x0, y0, x0 + side, y0 + side)) {
            return true;
        }
        if shapes
            .iter()
            .any(|sh| sh.holds_square(x0, y0, x0 + side, y0 + side))
        {
            return true;
        }
        if depth == 0 {
            return false;
        }
        let h = side / 2;
        [(x0, y0), (x0 + h, y0), (x0, y0 + h), (x0 + h, y0 + h)]
            .iter()
            .all(|&(a, b)| go(hq, shapes, a, b, h, depth - 1))
    }
    go(&hq, shapes, x0, y0, side, depth)
}

/// The convex hull (counter-clockwise, nm) meets no open hole disc
/// `(centre, radius)`: each centre lies outside the hull, at least its
/// radius from every edge. Exact in integers.
fn hull_clear_of_holes(hull: &[(i64, i64)], holes: &[((i64, i64), i64)]) -> bool {
    holes.iter().all(|&(c, r)| {
        let r2 = i128::from(r) * i128::from(r);
        let n = hull.len();
        let mut inside = n >= 3;
        for k in 0..n {
            let (a, b) = (hull[k], hull[(k + 1) % n]);
            let (ex, ey) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
            let (px, py) = (i128::from(c.0 - a.0), i128::from(c.1 - a.1));
            let cross = ex * py - ey * px;
            if cross < 0 {
                inside = false;
            }
            let dot = ex * px + ey * py;
            let l2 = ex * ex + ey * ey;
            let near = if dot <= 0 || l2 == 0 {
                px * px + py * py < r2
            } else if dot >= l2 {
                let (qx, qy) = (i128::from(c.0 - b.0), i128::from(c.1 - b.1));
                qx * qx + qy * qy < r2
            } else {
                // cross² < r² l2, in u128: |cross| < 2⁶³ here
                cross.unsigned_abs().pow(2) < (r2 as u128) * (l2 as u128)
            };
            if near {
                return false;
            }
        }
        !inside
    })
}

/// The net's copper on the grid, planes cropped to `crop_nm` around
/// `terminals`.
pub fn copper(
    board: &Board,
    routed: &Routed,
    net: &str,
    terminals: &[&Pad],
    m: &Model,
) -> Result<NetCopper, Refusal> {
    let stack = &board.stack;
    let h = m.h_nm;
    let e = m.hole_offset_nm;
    let rule = stack.rule(net);
    let mut vias: Vec<ViaAt> = routed
        .vias
        .iter()
        .filter(|v| v.net == net)
        .map(|v| ViaAt {
            at: v.at,
            via_nm: rule.via_nm,
            drill_nm: rule.drill_nm,
            relief: false,
        })
        .collect();
    for a in routed.arrays.iter().filter(|a| a.net == net) {
        vias.extend(a.at.iter().map(|&at| ViaAt {
            at,
            via_nm: a.via_nm,
            drill_nm: a.drill_nm,
            relief: false,
        }));
    }
    for p in board.parts.iter().flat_map(|p| p.pads.iter()) {
        if let (Some(drill), Some(n)) = (p.drill_nm, p.net.as_deref())
            && n == net
        {
            let r = &p.rect;
            vias.push(ViaAt {
                at: ((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2),
                via_nm: (r.x1 - r.x0).min(r.y1 - r.y0),
                drill_nm: drill,
                relief: p.join == Some(crate::stack::Join::Relief),
            });
        }
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
    let (mut vias, mut off): (Vec<ViaAt>, Vec<ViaAt>) = vias.into_iter().partition(on_grid);
    let mut notes = Vec::new();
    if !off.is_empty() {
        notes.push(format!(
            "{} of the net's vias are off the {h} nm grid and left out (their holes kept clear): {:?}",
            off.len(),
            off.iter().map(|v| v.at).collect::<Vec<_>>()
        ));
    }
    // each via geometry's junction region, priced once; a geometry whose
    // region cannot hold the band at every hole position, or is not
    // star-shaped from each, is left out like an off-grid via
    let mut junctions: Vec<Junction> = Vec::new();
    let mut by_geometry: BTreeMap<(i64, i64), Option<usize>> = BTreeMap::new();
    let mut unfit: Vec<(i64, i64, i64)> = Vec::new();
    let beta = Iv::ratio(m.beta_nm, 1_000_000);
    let t_p = Iv::ratio(m.plating_nm, 1_000_000);
    let e_mm = Iv::ratio(e, 1_000_000).hi;
    let h_mm = h as f64 / 1e6;
    for v in &vias {
        let key = (v.via_nm, v.drill_nm);
        if by_geometry.contains_key(&key) {
            continue;
        }
        // the shell: the plating's minimum on the finished wall, whose
        // radius is anywhere from the smallest to the largest finished hole
        let (r_lo, r_hi) = shell_radii(v.drill_nm, m);
        // the band's outer circle about any centre the tolerance allows, at
        // the largest shell, r0 + e√2: the region (cells meeting that disc)
        // holds it, and the via pad must too, (R − r0)² ≥ 2e², R − r0 > 0
        let r0 = Iv::ratio(r_hi, 1_000_000).add(beta).hi;
        let reach = junction_reach(v.drill_nm, m.hole_plus_nm, m.plating_nm, m.beta_nm, e);
        let region = Region::new(h, reach);
        let room = (v.via_nm as f64 / 2e6) - r0;
        let fits = v.drill_nm > m.hole_minus_nm
            && room > 0.0
            && Iv::exact(room).sqr().lo >= Iv::exact(e_mm).sqr().scale(2.0).hi;
        let priced = if fits && region.rays_inside(h, e) {
            let faces: Vec<(f64, (i64, i64), f64)> = region
                .faces
                .iter()
                .map(|f| (f.d0 as f64 * h_mm, f.normal, f.u0 as f64 * h_mm))
                .collect();
            match shell_slices(r_lo, r_hi)
                .into_iter()
                .map(|r_d| flow::junction_table(&faces, h_mm, e_mm, OFFSET_BOXES, r_d, beta, t_p))
                .collect::<Result<Vec<_>, _>>()
                .map(flow::JunctionTable::merge)
            {
                Ok(table) => {
                    junctions.push(Junction { region, table });
                    Some(junctions.len() - 1)
                }
                Err(_) => None,
            }
        } else {
            None
        };
        if priced.is_none() {
            notes.push(format!(
                "a {}/{} nm via's junction cannot hold the band at every hole size and position the fabricator's tolerances allow: such vias are left out; a via pad of at least {} mm holds it",
                v.via_nm,
                v.drill_nm,
                pad_needed(v.drill_nm, m) as f64 / 1e6
            ));
            unfit.push((v.via_nm, v.drill_nm, pad_needed(v.drill_nm, m)));
        }
        by_geometry.insert(key, priced);
    }
    let (kept, unpriced): (Vec<ViaAt>, Vec<ViaAt>) = vias
        .into_iter()
        .partition(|v| by_geometry[&(v.via_nm, v.drill_nm)].is_some());
    vias = kept;
    off.extend(unpriced);
    // holes kept clear: an off-grid or unpriced via's, and the net's own
    // through-hole pads', each at its drill plus the tolerance
    let own_drilled: Vec<&Pad> = board
        .parts
        .iter()
        .flat_map(|p| p.pads.iter())
        .filter(|p| p.net.as_deref() == Some(net) && p.drill_nm.is_some())
        .collect();
    let mut holes: Vec<((i64, i64), i64)> = off
        .iter()
        .map(|v| (v.at, hole_clear(v.drill_nm, m)))
        .collect();
    for p in &own_drilled {
        let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
        // a priced pad's hole is inside its own junction region
        if !vias.iter().any(|v| v.at == c) {
            holes.push((c, hole_clear(p.drill_nm.unwrap_or(0), m)));
        }
    }
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
    let centre = |v: &ViaAt| ((v.at.0 - origin.0) / h, (v.at.1 - origin.1) / h);
    // every via's region cells, which no sheet holds; two regions meeting is
    // refused by name
    let mut in_region: BTreeSet<Cell> = BTreeSet::new();
    for v in &vias {
        let c = centre(v);
        let jn = &junctions[by_geometry[&(v.via_nm, v.drill_nm)].unwrap_or(0)];
        for &(i, j) in &jn.region.cells {
            if !in_region.insert((c.0 + i, c.1 + j)) {
                return Err(Refusal::Named(
                    "ViaRegionsOverlap",
                    format!("the via at {:?}'s junction region meets another's", v.at),
                ));
            }
        }
    }
    let mut layers = Vec::new();
    let mut eligible: Vec<BTreeSet<Cell>> = Vec::new();
    let mut outer_shapes: Vec<Vec<Shape>> = Vec::new();
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
                // the narrowest the etch tolerance allows, rounded down
                shapes.push(Shape::Stadium {
                    a,
                    b,
                    hw: S * w * i128::from(1000 - m.track_etch_permille) / 2000,
                });
            }
            for v in &vias {
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
            let holes4: Vec<((i128, i128), i128)> = holes
                .iter()
                .map(|&(c, r)| {
                    (
                        (S * i128::from(c.0), S * i128::from(c.1)),
                        S * i128::from(r),
                    )
                })
                .collect();
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
                if in_region.contains(&(i, j)) {
                    continue;
                }
                let (x0, y0) = corner(i, j);
                let (x1, y1) = (x0 + hs, y0 + hs);
                if !holes4.iter().all(|&(c, r)| clear(x0, y0, x1, y1, c, r)) {
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
            eligible.push(BTreeSet::new());
            outer_shapes.push(shapes);
        } else if plane {
            let inset = (
                o.x0 + m.edge_nm,
                o.y0 + m.edge_nm,
                o.x1 - m.edge_nm,
                o.y1 - m.edge_nm,
            );
            // every knockout: its centre, raw radius and whether the fill's
            // opening applies beside it (a hole of the net's own is not the
            // fill's: no opening, no witness)
            let mut knockouts: Vec<((i64, i64), i64, bool)> = Vec::new();
            for v in routed.vias.iter().filter(|v| v.net != net) {
                knockouts.push((v.at, m.knockout_raw(stack.rule(&v.net).via_nm), true));
            }
            for a in routed.arrays.iter().filter(|a| a.net != net) {
                for &at in &a.at {
                    knockouts.push((at, m.knockout_raw(a.via_nm), true));
                }
            }
            for p in board
                .parts
                .iter()
                .flat_map(|p| p.pads.iter())
                .filter(|p| p.drill_nm.is_some())
                .filter(|p| {
                    // a solidly joined pad of the net that is a priced via
                    // is plane copper, not a knockout
                    let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
                    !(p.net.as_deref() == Some(net)
                        && p.join != Some(crate::stack::Join::Relief)
                        && vias.iter().any(|v| v.at == c))
                })
            {
                let (w, hh) = (
                    (p.rect.x1 - p.rect.x0) as f64,
                    (p.rect.y1 - p.rect.y0) as f64,
                );
                let diag = w.hypot(hh).ceil() as i64 + 1;
                let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
                knockouts.push((c, m.knockout_raw(diag), true));
            }
            for &(c, r) in &holes {
                knockouts.push((c, r, false));
            }
            // a cell meeting a raw knockout is out; one meeting only the
            // opening's ring is copper if witnessed ([`witnessed`])
            let (cover, wit) = (m.opening_nm - ARC_NM, m.opening_nm + ARC_NM);
            let mut out: BTreeSet<(i64, i64)> = BTreeSet::new();
            let mut ring: BTreeMap<(i64, i64), Vec<usize>> = BTreeMap::new();
            for (k, &(c, r, opens)) in knockouts.iter().enumerate() {
                let reach = if opens { r + m.opening_nm } else { r };
                let cc = (S * i128::from(c.0), S * i128::from(c.1));
                for i in cell_of(c.0 - reach)..=cell_of(c.0 + reach) {
                    for j in row_of(c.1 - reach)..=row_of(c.1 + reach) {
                        let (x0, y0) = corner(i, j);
                        if !clear(x0, y0, x0 + hs, y0 + hs, cc, S * i128::from(r)) {
                            out.insert((i, j));
                        } else if !clear(x0, y0, x0 + hs, y0 + hs, cc, S * i128::from(reach)) {
                            ring.entry((i, j)).or_default().push(k);
                        }
                    }
                }
            }
            let raw_edge = m.edge_nm - m.opening_nm;
            let inner = (
                o.x0 + raw_edge + wit,
                o.y0 + raw_edge + wit,
                o.x1 - raw_edge - wit,
                o.y1 - raw_edge - wit,
            );
            let mut admitted = 0usize;
            for &(i, j) in ring.keys() {
                if out.contains(&(i, j)) {
                    continue;
                }
                let (x0, y0) = (origin.0 + i * h, origin.1 + j * h);
                // every fill knockout whose opening could reach a witness
                let near: Vec<((i64, i64), i64)> = knockouts
                    .iter()
                    .filter(|(c, r, _)| {
                        let d = r + 2 * wit + 2 * h;
                        (c.0 - x0).abs() <= d && (c.1 - y0).abs() <= d
                    })
                    .map(|&(c, r, _)| (c, r))
                    .collect();
                let hard = ring[&(i, j)].iter().any(|&k| !knockouts[k].2);
                if hard || !witnessed((x0, y0, h), &near, inner, cover, wit) {
                    out.insert((i, j));
                } else {
                    admitted += 1;
                }
            }
            if admitted > 0 {
                notes.push(format!(
                    "{name}: {admitted} cells inside a knockout's opening ring admitted by a witness disc"
                ));
            }
            let mut ok = BTreeSet::new();
            for i in i0..=i1 {
                for j in j0..=j1 {
                    let (x0, y0) = (origin.0 + i * h, origin.1 + j * h);
                    if x0 >= inset.0
                        && y0 >= inset.1
                        && x0 + h <= inset.2
                        && y0 + h <= inset.3
                        && !out.contains(&(i, j))
                    {
                        ok.insert((i, j));
                        if !in_region.contains(&(i, j)) {
                            cells.push((i, j));
                        }
                    }
                }
            }
            layers.push(LayerCopper {
                name: name.clone(),
                cells,
                t_min_nm: m.plane_nm.0,
                plane: true,
            });
            eligible.push(ok);
            outer_shapes.push(Vec::new());
        } else {
            layers.push(LayerCopper {
                name: name.clone(),
                cells: Vec::new(),
                t_min_nm: m.plane_nm.0,
                plane: false,
            });
            eligible.push(BTreeSet::new());
            outer_shapes.push(Vec::new());
        }
    }
    // each via's joins: on a plane, every region cell plane copper, and a
    // port at each face whose outside cell is plane copper; on F.Cu and
    // B.Cu, a port at each face whose outside cell is copper and whose rays,
    // for every hole position, run inside copper
    let mut uses = Vec::new();
    let mut relieved = 0usize;
    for v in vias {
        let ji = by_geometry[&(v.via_nm, v.drill_nm)].unwrap_or(0);
        let jn = &junctions[ji];
        let c = centre(&v);
        let abs = |q: Cell| (c.0 + q.0, c.1 + q.1);
        let mut joins = Vec::new();
        for (li, layer) in layers.iter().enumerate() {
            let have: BTreeSet<Cell> = layer.cells.iter().copied().collect();
            if layer.plane {
                if v.relief {
                    relieved += 1;
                    continue;
                }
            } else if layer.name != "F.Cu" && layer.name != "B.Cu" {
                continue;
            }
            let mut ports = Vec::new();
            for (k, f) in jn.region.faces.iter().enumerate() {
                if !have.contains(&abs(f.outside)) {
                    continue;
                }
                // a face's column (its rays, and its sector of the band)
                // lies in the hull of the face and the square of hole
                // centres: on a plane the face is a port only when every
                // cell that hull meets is plane copper
                if layer.plane
                    && !jn
                        .region
                        .hull_inside(f, h, e, &|q| eligible[li].contains(&abs(q)))
                {
                    continue;
                }
                if !layer.plane {
                    let (nx, ny) = f.normal;
                    let (tx, ty) = (-ny, nx);
                    let p = |s: i64| {
                        (
                            v.at.0 + (f.d0 * nx + s * tx) * h,
                            v.at.1 + (f.d0 * ny + s * ty) * h,
                        )
                    };
                    let pts = [
                        p(f.u0),
                        p(f.u0 + 1),
                        (v.at.0 - e, v.at.1 - e),
                        (v.at.0 + e, v.at.1 - e),
                        (v.at.0 + e, v.at.1 + e),
                        (v.at.0 - e, v.at.1 + e),
                    ];
                    let hull = flow::convex_hull(&pts);
                    // belt-and-suspenders: a port's outside cell is already
                    // clear of every hole, so a hole crossing its hull would
                    // lie inside the junction region, which DRC's hole-to-hole
                    // spacing forbids (a same-net pad hole 0.41 mm off is
                    // still not crossed: `a_port_whose_rays_cross_a_hole_…`);
                    // the hole test keeps the rays off it without relying on
                    // either, and `a_hole_inside_every_ports_hull_joins_no_face`
                    // reaches it with a hole no board would allow
                    if !hull_in_copper(&hull, &outer_shapes[li], 8)
                        || !hull_clear_of_holes(&hull, &holes)
                    {
                        continue;
                    }
                }
                ports.push(k);
            }
            joins.push((li, ports));
        }
        uses.push(ViaUse {
            via: v,
            junction: ji,
            joins,
        });
    }
    if relieved > 0 {
        notes.push(format!(
            "ReliefUnpriced: {relieved} plane joins of thermally relieved pads are left out, their spokes not priced"
        ));
    }
    // plane islands, as KiCad's fill drops them: a plane cell no port of a
    // via joined to that plane reaches is left out
    for (li, layer) in layers.iter_mut().enumerate() {
        if !layer.plane {
            continue;
        }
        let have: BTreeSet<Cell> = layer.cells.iter().copied().collect();
        let mut seen: BTreeSet<Cell> = BTreeSet::new();
        let mut queue: Vec<Cell> = Vec::new();
        for u in &uses {
            let c = centre(&u.via);
            for (l, ports) in &u.joins {
                if *l == li {
                    for &k in ports {
                        let f = &junctions[u.junction].region.faces[k];
                        let q = (c.0 + f.outside.0, c.1 + f.outside.1);
                        if seen.insert(q) {
                            queue.push(q);
                        }
                    }
                }
            }
        }
        while let Some((i, j)) = queue.pop() {
            for q in [(i + 1, j), (i - 1, j), (i, j + 1), (i, j - 1)] {
                if have.contains(&q) && seen.insert(q) {
                    queue.push(q);
                }
            }
        }
        let before = layer.cells.len();
        layer.cells.retain(|q| seen.contains(q));
        if layer.cells.len() < before {
            notes.push(format!(
                "{}: {} plane cells no via port reaches are left out (islands)",
                layer.name,
                before - layer.cells.len()
            ));
        }
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
            // the declared maximum where there is one (and below the
            // derived one), else the finished maximum less every other
            // row's minimum
            let derived = stack.thickness.max_nm - copper_min - others;
            let i = stack
                .dielectrics
                .iter()
                .position(|d| d.0 == *name && d.1 == stack.copper[k + 1]);
            let declared = i.and_then(|i| stack.dielectric_max_nm.get(i).copied().flatten());
            let hi = declared.map_or(derived, |d| d.min(derived));
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
        junctions,
        vias: uses,
        rows,
        notes,
        unfit,
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
            return Err(Refusal::Named(
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
    // each via's centre and, per joined layer, its port edges
    let mut ports: Vec<(Cell, Vec<Vec<usize>>)> = Vec::new();
    for u in &nc.vias {
        let v = &u.via;
        let jn = &nc.junctions[u.junction];
        let hole = barrel_hole(v.drill_nm, m);
        let centre = ((v.at.0 - nc.origin.0) / h, (v.at.1 - nc.origin.1) / h);
        let mut joins = Vec::new();
        for (li, s) in sheets.iter_mut().enumerate() {
            if let Some(sheet) = s
                && let Some((_, p)) = u.joins.iter().find(|(l, _)| *l == li)
            {
                joins.push((
                    Join {
                        sheet,
                        row: 2 * li,
                        t_min: Iv::ratio(nc.layers[li].t_min_nm, 1_000_000),
                    },
                    p.clone(),
                ));
            }
        }
        let (_, edges) = flow::via(
            &mut net, centre, &jn.region, &jn.table, &hole, joins, &nc.rows, None, None,
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
    let k = b.kinds;
    Ok(PinBound {
        shares: [k[0].hi, k[1].hi, k[2].hi],
        bound: b,
        via_transfer,
    })
}

// ---------------------------------------------------------------------------
// The gate (`AET-WO-1303b`, ruled on PR #222): each domain a specification
// gives a drop limit is judged from the routed board.

/// The cell side, nm: an engine constant (ruling S4), exact in binary in mm,
/// a divisor of the router's 125 µm grid so every via centre is a cell
/// corner. A finer pitch is the known remedy when a bound sets the margin
/// ([`Gate::note`]).
pub const PITCH_NM: i64 = 62_500;
/// Plane copper further than this from the terminals' box is left out
/// (ruling S4): less copper only raises the bound.
pub const CROP_NM: i64 = 5_000_000;
/// [`flow::junction_core`]'s band, nm: thin bands cost little (measured: 30 µm to
/// 1 µm raises a coefficient by 1–2 %), and 10 µm lets the worst-case hole's
/// band fit a 0.6 mm via pad.
pub const BETA_NM: i64 = 10_000;
/// The shell radius's range, from the smallest finished hole to the
/// largest, cut into this many slices: each slice's junction table prices
/// every hole position over that slice's radii, so the via's worst box is a
/// worst over size and position together (ruling 6974760231, (1)).
pub const HOLE_SIZE_SLICES: usize = 4;
/// The plane's fill clearance to foreign copper, measured on the
/// gate-prepared board in KiCad 10.0.7: 0.2025 mm (`AET-WO-1303` P4.7).
pub const FILL_CLEARANCE_NM: i64 = 202_500;
/// The zone's min-thickness opening: half of `min_thickness` 0.25 mm
/// (`AET-WO-1303` F9).
pub const OPENING_NM: i64 = 125_000;
/// A plane's inset from the board's edge: the router's 0.5 mm edge
/// clearance plus the opening.
pub const EDGE_NM: i64 = 625_000;

/// Copper's resistivity at `t_c` °C, at most, Ω·mm, as an interval:
/// IACS annealed copper 1.7241 µΩ·cm at 20 °C (NBS Handbook 100), its
/// temperature coefficient 0.00393 /°C (NBS Handbook 100; IPC-TM-650
/// 2.5.14A), times k = 1.181, IPC-MF-150F §3.8.1.2's largest rated foil
/// resistivity over IACS (0.181/0.15328 ohm·g/m², rounded up). k is cited
/// for deposited foil; applying it to plated copper (barrels, outer
/// plating) is a stated condition (ruling D6).
pub fn rho_max(t_c: u32) -> Iv {
    let rise = i64::from(t_c) - 20;
    Iv::ratio(17_241, 1_000_000_000)
        .mul(Iv::exact(1.0).add(Iv::ratio(393, 100_000).mul(Iv::exact(rise as f64))))
        .mul(Iv::ratio(1181, 1000))
}

/// One pin's result: its bound, µΩ (rounded up), or why it has none.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PinResult {
    Bounded {
        pin: String,
        r_bound_microohms: u64,
    },
    Refused {
        pin: String,
        kind: &'static str,
        message: String,
    },
}

/// One domain's verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Gate {
    pub domain: String,
    pub limit_millivolts: u32,
    pub limit_source: String,
    /// Where the regulator's current enters the net, and its own return pin.
    pub source: (String, String),
    pub ret: (String, String),
    pub load_milliamps: u64,
    /// The load's pins on the domain and on `GND`, by pad.
    pub mcu: String,
    pub pins: Vec<PinResult>,
    pub returns: Vec<PinResult>,
    /// (the largest supply bound + the largest return bound) × the load, µV,
    /// rounded up; `None` when any pin is refused. With a radio module on the
    /// domain, its current is charged by [`Shared`] instead.
    pub drop_bound_microvolts: Option<u64>,
    /// A radio module's current, charged only on the copper it shares with
    /// the load's path (`AET-WO-4411`, ruled 1 (a')); `None` without one.
    pub shared: Option<Shared>,
    /// Every other current on the ground copper the load returns through,
    /// each charged by [`Other`] (`AET-WO-4310` (3), ruled D5); its charges
    /// are in [`Gate::drop_bound_microvolts`].
    pub others: Vec<Other>,
    /// Set when the bound comes within 1.5× of the limit: the pitch set the
    /// margin (ruling S4).
    pub note: Option<String>,
    pub conditions: Vec<String>,
    /// What the copper model left out or admitted, per net, for the record.
    pub notes: Vec<String>,
}

/// A radio module's current on the domain and the bounds that charge it
/// (`AET-WO-4411`, ruled 1 (a')): its pads' own bounds from the source and to
/// the return, the largest over its pads on each net, `None` on a side where
/// any of its pads has none (that side is then charged at the load's own
/// largest bound, as without the module).
///
/// The maximum principle: with the source held as reference, a current
/// drawn at a node raises no node's voltage above that node's own, so the
/// voltage it causes at a load pin `k` is at most `I × min(r_kk, r_mm)`, the
/// lesser of the two driving-point resistances. Each is bounded from above
/// here (Thomson's principle, on copper the gate reads), so the module's
/// share of the load's drop is at most its current times the lesser bound,
/// on each side: the copper it shares with the load's path carries it, and
/// the load's own branch does not.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shared {
    pub reference: String,
    pub milliamps: u64,
    pub supply_bound_microohms: Option<u64>,
    pub return_bound_microohms: Option<u64>,
}

/// Another current in the ground copper the gate's load returns through
/// (`AET-WO-4310` (3), ruled D5): another regulated domain's load, entering
/// at its consumers' ground pads, or a battery-fed regulator's input
/// current, entering at its own ground pin, on its way to the battery.
///
/// The maximum principle, with the gate's return pin `b` as reference: a
/// current `I` entering at `c` raises a load pin `a` by at most
/// `I × min(R(a,b), R(c,b))`, and where it leaves (its regulator, the
/// battery) it can only lower `a`. So it raises the load's return by at most
/// `I × min(the load's largest return bound, the largest bound from its
/// entry pads to b)`, each bounded from above (Thomson), as [`Shared`] does
/// for a module. A pad the gate cannot read leaves the entry bound unknown:
/// it is then charged at the load's own largest return bound, which the
/// principle still bounds, and named.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Other {
    /// What the current is, and where it enters.
    pub what: String,
    pub milliamps: u64,
    /// The largest bound from its entry pads to the return pin that was
    /// needed, µΩ: past the load's own largest, no more are solved. `None`:
    /// a pad the gate cannot read, named in `unread`.
    pub entry_bound_microohms: Option<u64>,
    pub unread: Vec<String>,
    /// `milliamps × min(the load's largest return bound, entry bound)`, µV,
    /// rounded up; `None` when the load's own return is unbounded.
    pub charge_microvolts: Option<u64>,
}

fn refusal(kind: &'static str, message: String) -> crate::extract::Refused {
    crate::extract::Refused {
        kind,
        id: None,
        message,
    }
}

/// Each domain the specification limits, judged on the read board. A
/// specification with no `power.drop_limits` gives none, and nothing about
/// its verdict changes.
pub fn supplies(
    spec: &aethelgard_ir::Specification,
    solved: &aethelgard_ir::SolvedTopology,
    table: &aethelgard_ir::PinTable,
    read: &crate::extract::ReadBoard,
) -> Result<Vec<Gate>, crate::extract::Refused> {
    let Some(power) = &spec.power else {
        return Ok(Vec::new());
    };
    let limits = &power.drop_limits;
    if limits.is_empty() {
        return Ok(Vec::new());
    }
    let st = &spec.board.stackup;
    fn need<'a>(
        v: &'a Option<aethelgard_ir::board::Sourced>,
        path: &str,
    ) -> Result<&'a aethelgard_ir::board::Sourced, crate::extract::Refused> {
        v.as_ref().ok_or_else(|| {
            refusal(
                "SupplyInputUndeclared",
                format!("the supply gate reads `board.stackup.{path}`, and the specification does not declare it: it is never defaulted (RUL-14)"),
            )
        })
    }
    let outer = need(&st.outer_copper_min_um, "outer_copper_min_um")?.value;
    let plane = need(&st.plane_copper_min_um, "plane_copper_min_um")?.value;
    let plating = need(&st.plating_min_um, "plating_min_um")?.value;
    let t_c = need(&st.copper_temperature_max_c, "copper_temperature_max_c")?.value;
    let none = aethelgard_ir::board::FabTolerances {
        hole_position_um: None,
        hole_size_plus_um: None,
        hole_size_minus_um: None,
        track_width_permille: None,
        via_ring_um: None,
    };
    let fab = st.fab_tolerances.as_ref().unwrap_or(&none);
    let position = need(&fab.hole_position_um, "fab_tolerances.hole_position_um")?;
    let plus = need(&fab.hole_size_plus_um, "fab_tolerances.hole_size_plus_um")?;
    let minus = need(&fab.hole_size_minus_um, "fab_tolerances.hole_size_minus_um")?;
    let etch = need(
        &fab.track_width_permille,
        "fab_tolerances.track_width_permille",
    )?;
    let conditions = conditions(position, plus, minus, etch);
    let checked = aethelgard_ir::power::check(spec)
        .map_err(|e| refusal(e.kind(), e.to_string()))?
        .ok_or_else(|| {
            refusal(
                "SupplyInputUndeclared",
                "the specification declares no power tree".into(),
            )
        })?;
    let (parts, _) = crate::schematic::design(spec, solved, table)
        .map_err(|e| refusal(e.kind(), e.to_string()))?;
    let mcu = parts
        .iter()
        .find(|p| p.lib_id.starts_with("MCU_"))
        .map(|p| p.reference.clone())
        .ok_or_else(|| refusal("SupplyTerminalMissing", "the design has no MCU".into()))?;
    let board = &read.board;
    let model = Model {
        h_nm: PITCH_NM,
        rho: rho_max(t_c),
        outer_nm: (i64::from(outer) * 1000, board.stack.copper_max_nm),
        plane_nm: (i64::from(plane) * 1000, board.stack.copper_max_nm),
        plating_nm: i64::from(plating) * 1000,
        beta_nm: BETA_NM,
        hole_offset_nm: i64::from(position.value) * 1000,
        hole_plus_nm: i64::from(plus.value) * 1000,
        hole_minus_nm: i64::from(minus.value) * 1000,
        track_etch_permille: i64::from(etch.value),
        edge_nm: EDGE_NM,
        crop_nm: CROP_NM,
        fill_clearance_nm: FILL_CLEARANCE_NM,
        opening_nm: OPENING_NM,
    };
    let pp = aethelgard_ir::power_parts::table().map_err(|e| refusal(e.kind(), e.to_string()))?;
    fn pads_of<'a>(board: &'a Board, reference: &str) -> Vec<&'a Pad> {
        board
            .parts
            .iter()
            .filter(|p| p.reference == reference)
            .flat_map(|p| p.pads.iter())
            .collect()
    }
    let mut out = Vec::new();
    let the_mcu = mcu;
    for limit in limits {
        let d = &limit.domain;
        // The load: the MCU on its own supply; on a radio module's own rail,
        // the module (`AET-WO-4411`, ruled 2026-11-25T20:10Z), whose pins
        // the gate then judges. Any other domain keeps the MCU, and is
        // refused below when it has no pin there.
        let mcu = spec
            .declared_radio_modules()
            .iter()
            .find(|m| &m.supply == d && m.supply != spec.mcu.supply)
            .map_or_else(|| the_mcu.clone(), |m| m.reference.clone());
        // a limited domain the load has no pin on is refused by name: an
        // empty side would leave no drop and no finding
        for side in [d.as_str(), "GND"] {
            if !pads_of(board, &mcu)
                .iter()
                .any(|p| p.net.as_deref() == Some(side))
            {
                return Err(refusal(
                    "SupplyTerminalMissing",
                    format!(
                        "the load {mcu} has no pin on `{side}`, so the drop limit on `{d}` judges nothing"
                    ),
                ));
            }
        }
        // every part on the domain that can draw current is a declared
        // load naming it (the load ruling): cited, derived or declared,
        // never zeroed; the MCU, child regulators and radio modules come
        // from their rows,
        // and capacitors, inductors and test pads draw none
        let children: BTreeSet<&str> = power
            .regulators
            .iter()
            .filter(|r| &r.input == d)
            .map(|r| r.reference.as_str())
            .collect();
        let rail = crate::schematic::Conn::Rail(d.clone());
        for p in &parts {
            let draws = p.conns.values().any(|c| *c == rail)
                && !p.lib_id.starts_with("MCU_")
                && !p.lib_id.starts_with("Device:C")
                && p.lib_id != "Device:L"
                && p.lib_id != crate::schematic::TEST_POINT
                && !power
                    .regulators
                    .iter()
                    .any(|r| &r.output == d && r.reference == p.reference)
                && !children.contains(p.reference.as_str());
            // A radio module on the domain is counted from its row
            // (`aethelgard_ir::radio::load_ma`, in `checked`'s load); a load
            // naming it is refused (`LoadNamesCountedPart`, `AET-WO-4411` D5).
            let named =
                power.loads.iter().any(|l| {
                    &l.domain == d && l.reference.as_deref() == Some(p.reference.as_str())
                }) || spec
                    .declared_radio_modules()
                    .iter()
                    .any(|m| &m.supply == d && m.reference == p.reference);
            if draws && !named {
                return Err(refusal(
                    "SupplySinkUncounted",
                    format!(
                        "{} ({}) is on `{d}`, and no `power.loads` entry names it: its current is cited from its datasheet, derived, or declared, and never zeroed",
                        p.reference, p.lib_id
                    ),
                ));
            }
        }
        let load = checked
            .domains
            .get(d)
            .map(|c| c.load_milliamps)
            .ok_or_else(|| {
                refusal(
                    "SupplyTerminalMissing",
                    format!("no regulator sources `{d}`"),
                )
            })?;
        let reg = power
            .regulators
            .iter()
            .find(|r| &r.output == d)
            .ok_or_else(|| {
                refusal(
                    "SupplyTerminalMissing",
                    format!("no regulator sources `{d}`"),
                )
            })?;
        let row = pp.get(&reg.part).ok_or_else(|| {
            refusal(
                "SupplyTerminalMissing",
                format!("{} is not in the power parts table", reg.part),
            )
        })?;
        let pin = |name: &str| {
            row.pins.get(name).cloned().ok_or_else(|| {
                refusal(
                    "SupplyTerminalMissing",
                    format!("{} has no pin named {name}", reg.part),
                )
            })
        };
        // the source: a buck's current enters the domain at its inductor's
        // pad on it (the inductor is the part between the SW pin's net and
        // the domain); an LDO's at its OUT pin
        let (source, ret_pin) = match &row.kind {
            aethelgard_ir::power_parts::Kind::Buck(_) => {
                let sw = pin("SW")?;
                let sw_net = pads_of(board, &reg.reference)
                    .into_iter()
                    .find(|p| p.number == sw)
                    .and_then(|p| p.net.clone())
                    .ok_or_else(|| {
                        refusal(
                            "SupplyTerminalMissing",
                            format!("{}'s SW pad has no net", reg.reference),
                        )
                    })?;
                let inductor = board
                    .parts
                    .iter()
                    .filter(|p| p.reference != reg.reference)
                    .find(|p| {
                        p.pads
                            .iter()
                            .any(|q| q.net.as_deref() == Some(sw_net.as_str()))
                            && p.pads.iter().any(|q| q.net.as_deref() == Some(d.as_str()))
                    })
                    .ok_or_else(|| {
                        refusal(
                            "SupplyTerminalMissing",
                            format!("no inductor joins {}'s SW to `{d}`", reg.reference),
                        )
                    })?;
                let pad = inductor
                    .pads
                    .iter()
                    .find(|q| q.net.as_deref() == Some(d.as_str()))
                    .expect("found above");
                // the return: PGND, never the exposed pad, which SNVSAN3F
                // Table 6-1 (p. 5) names AGND and p. 6 rates apart from PGND
                (
                    (inductor.reference.clone(), pad.number.clone()),
                    pin("PGND")?,
                )
            }
            aethelgard_ir::power_parts::Kind::Ldo(_) => {
                ((reg.reference.clone(), pin("OUT")?), pin("GND")?)
            }
            _ => {
                return Err(refusal(
                    "SupplyTerminalMissing",
                    format!("{} is not a regulator the gate models", reg.part),
                ));
            }
        };
        let ret = (reg.reference.clone(), ret_pin);
        let gnd = "GND";
        let source_pad = pad(board, d, &source.0, &source.1).map_err(to_refused)?;
        let ret_pad = pad(board, gnd, &ret.0, &ret.1).map_err(to_refused)?;
        let mcu_pads = |net: &str| -> Vec<&Pad> {
            let mut v: Vec<&Pad> = pads_of(board, &mcu)
                .into_iter()
                .filter(|p| p.net.as_deref() == Some(net))
                .collect();
            v.sort_by_key(|p| {
                (
                    p.number.parse::<u32>().unwrap_or(u32::MAX),
                    p.number.clone(),
                )
            });
            v
        };
        let run = |net: &str,
                   from: &Pad,
                   sinks: &[&Pad]|
         -> Result<(Vec<PinResult>, Vec<String>), crate::extract::Refused> {
            let mut terminals = vec![from];
            terminals.extend(sinks.iter().copied());
            let nc = copper(board, &read.routed, net, &terminals, &model).map_err(to_refused)?;
            let notes = nc.notes.iter().map(|n| format!("`{net}`: {n}")).collect();
            let results = sinks
                .iter()
                .map(|p| match bound(&nc, &model, from, p, 400) {
                    Ok(b) => bounded_pin(p, &b),
                    Err(r) => {
                        let (kind, message) = match r {
                            Refusal::Named(k, m) => (k, m),
                            Refusal::Flow(FlowError::Disconnected) if !nc.unfit.is_empty() => (
                                "SupplyViaPadTooSmall",
                                format!(
                                    "no copper the gate reads joins {}.{} to {}.{}: {}",
                                    mcu,
                                    p.number,
                                    from_ref(board, from),
                                    from.number,
                                    nc.unfit
                                        .iter()
                                        .map(|(pad, drill, need)| format!(
                                            "the gate left out the net's {} / {} mm vias, whose pad cannot keep the band at the fabricator's worst hole: via pad >= {} mm",
                                            *pad as f64 / 1e6,
                                            *drill as f64 / 1e6,
                                            *need as f64 / 1e6
                                        ))
                                        .collect::<Vec<_>>()
                                        .join("; ")
                                ),
                            ),
                            Refusal::Flow(FlowError::Disconnected) => (
                                "SupplyPinUnbounded",
                                format!(
                                    "no copper the gate reads joins {}.{} to {}.{}",
                                    mcu,
                                    p.number,
                                    from_ref(board, from),
                                    from.number
                                ),
                            ),
                            Refusal::Flow(FlowError::Malformed(m)) => ("SupplyPinUnbounded", m),
                        };
                        PinResult::Refused {
                            pin: p.number.clone(),
                            kind,
                            message,
                        }
                    }
                })
                .collect();
            Ok((results, notes))
        };
        let vdd = mcu_pads(d);
        let vss = mcu_pads(gnd);

        let (pins, mut notes) = run(d, source_pad, &vdd)?;
        // the 2-layer return runs through a zone the emitted file leaves
        // unfilled: refused by name (ruling (iii))
        let gnd_plane = board.stack.planes.iter().any(|(_, n)| n == gnd);
        let returns = if gnd_plane {
            let (r, n) = run(gnd, ret_pad, &vss)?;
            notes.extend(n);
            r
        } else {
            vss.iter()
                .map(|p| PinResult::Refused {
                    pin: p.number.clone(),
                    kind: "GroundReturnUnread",
                    message: format!("{mcu}.{}'s return runs through a GND zone the emitted board leaves unfilled: the gate reads no plane for GND", p.number),
                })
                .collect()
        };
        // `AET-WO-4411` (ruled 1 (a')): a radio module's current on the
        // domain, charged on the copper it shares with the load's path; any
        // of its pads unbounded leaves that side charged as before.
        let mut shared = None;
        let modules: Vec<&str> = spec
            .declared_radio_modules()
            .iter()
            .filter(|m| &m.supply == d)
            .map(|m| m.reference.as_str())
            .collect();
        if gnd_plane
            && mcu == the_mcu
            && let [reference] = modules[..]
        {
            let ma = aethelgard_ir::radio::load_ma(spec, d)
                .map_err(|e| refusal(e.kind(), e.to_string()))?;
            let of = |net: &str| -> Vec<&Pad> {
                pads_of(board, reference)
                    .into_iter()
                    .filter(|p| p.net.as_deref() == Some(net))
                    .collect()
            };
            let (vdd_m, gnd_m) = (of(d), of(gnd));
            let largest = |v: &[PinResult]| -> Option<u64> {
                v.iter()
                    .map(|r| match r {
                        PinResult::Bounded {
                            r_bound_microohms, ..
                        } => Some(*r_bound_microohms),
                        PinResult::Refused { .. } => None,
                    })
                    .collect::<Option<Vec<u64>>>()
                    .and_then(|v| v.into_iter().max())
            };
            if !vdd_m.is_empty() && !gnd_m.is_empty() {
                let (a, n1) = run(d, source_pad, &vdd_m)?;
                let (b, n2) = run(gnd, ret_pad, &gnd_m)?;
                notes.extend(n1);
                notes.extend(n2);
                shared = Some(Shared {
                    reference: reference.to_owned(),
                    milliamps: ma,
                    supply_bound_microohms: largest(&a),
                    return_bound_microohms: largest(&b),
                });
            }
        }
        let own = match &shared {
            Some(m) => drop_bound_shared(&pins, &returns, load, m),
            None => drop_bound(&pins, &returns, load),
        };
        // `AET-WO-4310` (3), ruled D5: every other current on this load's
        // ground copper, charged by the maximum principle ([`Other`]).
        let r11 = largest_bound(&returns);
        let mut others = Vec::new();
        if gnd_plane {
            let ground = |reference: &str| -> Vec<&Pad> {
                pads_of(board, reference)
                    .into_iter()
                    .filter(|q| q.net.as_deref() == Some(gnd))
                    .collect()
            };
            // Each other regulated domain's load, at its consumers' pads.
            for r in power.regulators.iter().filter(|r| &r.output != d) {
                let k = &r.output;
                let ma = checked.domains.get(k).map_or(0, |c| c.load_milliamps);
                let rail_k = crate::schematic::Conn::Rail(k.clone());
                let mut names = Vec::new();
                let mut entry: Vec<&Pad> = Vec::new();
                for part in &parts {
                    let on = part.conns.values().any(|c| *c == rail_k)
                        && !part.lib_id.starts_with("Device:C")
                        && part.lib_id != "Device:L"
                        && part.lib_id != crate::schematic::TEST_POINT
                        && part.reference != r.reference
                        && !power
                            .regulators
                            .iter()
                            .any(|q| &q.input == k && q.reference == part.reference);
                    if !on {
                        continue;
                    }
                    let g = ground(&part.reference);
                    if !g.is_empty() {
                        names.push(part.reference.clone());
                        entry.extend(g);
                    }
                }
                let what = if names.is_empty() {
                    format!("`{k}`'s load ({}), which enters the ground copper off the board", r.reference)
                } else {
                    format!("`{k}`'s load ({}), entering at {}'s ground pads", r.reference, names.join(", "))
                };
                others.push(charge(what, ma, &entry, ret_pad, r11, &|ps| {
                    entry_bounds(board, &read.routed, gnd, ret_pad, ps, &model)
                })?);
            }
            // Each other battery-fed regulator's input current, at its own
            // ground pin, on its way to the battery.
            if let Some(side) = aethelgard_ir::power::battery_side(spec)
                .map_err(|e| refusal(e.kind(), e.to_string()))?
            {
                let v_mv = (side.v_open_microvolts / 1000).saturating_sub(checked.input_drop_millivolts).max(1);
                for sink in &side.sinks {
                    if sink.reference == reg.reference {
                        continue;
                    }
                    let Some(q) = power.regulators.iter().find(|q| q.reference == sink.reference) else {
                        continue;
                    };
                    let Some(row) = pp.get(&q.part) else { continue };
                    let gname = match &row.kind {
                        aethelgard_ir::power_parts::Kind::Buck(_) => "PGND",
                        _ => "GND",
                    };
                    let Some(gpin) = row.pins.get(gname) else { continue };
                    let entry: Vec<&Pad> = pad(board, gnd, &q.reference, gpin).into_iter().collect();
                    // mW × 1000 / mV = mA, rounded up.
                    let ma = (sink.input_milliwatts * 1000).div_ceil(v_mv);
                    let what = format!(
                        "{}'s input current at the pack's minimum less the input path's drop ({v_mv} mV), entering at its {gname} pin",
                        q.reference
                    );
                    others.push(charge(what, ma, &entry, ret_pad, r11, &|ps| {
                        entry_bounds(board, &read.routed, gnd, ret_pad, ps, &model)
                    })?);
                }
            }
        }
        // The own bound plus every charge; `None` when any is.
        let drop = own.and_then(|own| {
            others
                .iter()
                .try_fold(own, |acc, o| Some(acc + o.charge_microvolts?))
        });
        let note = pitch_note(drop, limit.max_millivolts);
        out.push(Gate {
            domain: d.clone(),
            limit_millivolts: limit.max_millivolts,
            limit_source: limit.source.clone(),
            source,
            ret,
            load_milliamps: load,
            mcu: mcu.clone(),
            pins,
            returns,
            drop_bound_microvolts: drop,
            shared,
            others,
            note,
            conditions: conditions.clone(),
            notes,
        });
    }
    Ok(out)
}

/// (the largest supply bound + the largest return bound) × the load, µV,
/// rounded up (µΩ × mA = nV); `None` when any pin is refused or a side has
/// no pin.
pub fn drop_bound(pins: &[PinResult], returns: &[PinResult], load_milliamps: u64) -> Option<u64> {
    let worst = |v: &[PinResult]| -> Option<u64> {
        v.iter()
            .map(|r| match r {
                PinResult::Bounded {
                    r_bound_microohms, ..
                } => Some(*r_bound_microohms),
                PinResult::Refused { .. } => None,
            })
            .collect::<Option<Vec<u64>>>()
            .and_then(|v| v.into_iter().max())
    };
    let (a, b) = (worst(pins)?, worst(returns)?);
    let nv = (u128::from(a) + u128::from(b)) * u128::from(load_milliamps);
    u64::try_from(nv.div_ceil(1000)).ok()
}

/// [`drop_bound`] with `m`'s current charged by [`Shared`]: the rest of the
/// load at the largest bounds, `m`'s at the lesser of each side's largest
/// bound and its own, µV, rounded up.
pub fn drop_bound_shared(
    pins: &[PinResult],
    returns: &[PinResult],
    load_milliamps: u64,
    m: &Shared,
) -> Option<u64> {
    let rest = load_milliamps.checked_sub(m.milliamps)?;
    let worst = |v: &[PinResult]| -> Option<u64> {
        v.iter()
            .map(|r| match r {
                PinResult::Bounded {
                    r_bound_microohms, ..
                } => Some(*r_bound_microohms),
                PinResult::Refused { .. } => None,
            })
            .collect::<Option<Vec<u64>>>()
            .and_then(|v| v.into_iter().max())
    };
    let (a, b) = (worst(pins)?, worst(returns)?);
    let own =
        a.min(m.supply_bound_microohms.unwrap_or(a)) + b.min(m.return_bound_microohms.unwrap_or(b));
    let nv = (u128::from(a) + u128::from(b)) * u128::from(rest)
        + u128::from(own) * u128::from(m.milliamps);
    u64::try_from(nv.div_ceil(1000)).ok()
}

/// The note a drop within 1.5× of its limit carries (ruling S4).
/// The largest bounded pin, µΩ; `None` when any pin is refused or none is.
fn largest_bound(v: &[PinResult]) -> Option<u64> {
    v.iter()
        .map(|r| match r {
            PinResult::Bounded {
                r_bound_microohms, ..
            } => Some(*r_bound_microohms),
            PinResult::Refused { .. } => None,
        })
        .collect::<Option<Vec<u64>>>()
        .and_then(|v| v.into_iter().max())
}

/// Bounds from `ret` to each pad of `pads` on `net`'s copper, in parallel.
fn entry_bounds(
    board: &Board,
    routed: &crate::route::Routed,
    net: &str,
    ret: &Pad,
    pads: &[&Pad],
    model: &Model,
) -> Result<Vec<Result<u64, String>>, crate::extract::Refused> {
    let mut terminals = vec![ret];
    terminals.extend(pads.iter().copied());
    let nc = copper(board, routed, net, &terminals, model).map_err(to_refused)?;
    let out = std::thread::scope(|s| {
        let handles: Vec<_> = pads
            .iter()
            .map(|p| {
                let nc = &nc;
                s.spawn(move || match bound(nc, model, ret, p, 400) {
                    Ok(b) => Ok(match bounded_pin(p, &b) {
                        PinResult::Bounded {
                            r_bound_microohms, ..
                        } => r_bound_microohms,
                        PinResult::Refused { .. } => u64::MAX,
                    }),
                    Err(e) => Err(format!("{}: {}", p.number, to_refused(e).message)),
                })
            })
            .collect();
        handles
            .into_iter()
            .map(|h| h.join().unwrap_or_else(|_| Err("a bound panicked".to_owned())))
            .collect()
    });
    Ok(out)
}

/// The threads [`charge`] solves at once.
const ENTRY_THREADS: usize = 6;

/// One [`Other`]: its entry pads solved farthest from `ret` first, a chunk
/// of [`ENTRY_THREADS`] at a time, stopping once one reaches the load's own
/// largest return bound `r11` (the charge is then `ma × r11`, whatever the
/// rest). No pads, or a pad the gate cannot read: charged at `r11`.
fn charge(
    what: String,
    ma: u64,
    entry: &[&Pad],
    ret: &Pad,
    r11: Option<u64>,
    solve: &dyn Fn(&[&Pad]) -> Result<Vec<Result<u64, String>>, crate::extract::Refused>,
) -> Result<Other, crate::extract::Refused> {
    let centre = |p: &Pad| {
        (
            i128::from(p.rect.x0 + p.rect.x1) / 2,
            i128::from(p.rect.y0 + p.rect.y1) / 2,
        )
    };
    let (rx, ry) = centre(ret);
    let mut order: Vec<&Pad> = entry.to_vec();
    order.sort_by_key(|p| {
        let (x, y) = centre(p);
        std::cmp::Reverse((x - rx) * (x - rx) + (y - ry) * (y - ry))
    });
    let mut worst: Option<u64> = None;
    let mut unread = Vec::new();
    if order.is_empty() {
        unread.push("no ground pad on the board".to_owned());
    }
    for chunk in order.chunks(ENTRY_THREADS) {
        if r11.is_some_and(|r| worst.is_some_and(|w| w >= r)) {
            break;
        }
        for r in solve(chunk)? {
            match r {
                Ok(v) if v != u64::MAX => worst = Some(worst.map_or(v, |w| w.max(v))),
                Ok(_) => unread.push("a pad refused".to_owned()),
                Err(e) => unread.push(e),
            }
        }
    }
    let entry_bound = if unread.is_empty() { worst } else { None };
    let charge = r11.map(|r| {
        let ohms = entry_bound.map_or(r, |e| e.min(r));
        // mA × µΩ = nV, rounded up to µV.
        (u128::from(ma) * u128::from(ohms)).div_ceil(1000) as u64
    });
    Ok(Other {
        what,
        milliamps: ma,
        entry_bound_microohms: entry_bound,
        unread,
        charge_microvolts: charge,
    })
}

pub fn pitch_note(drop_microvolts: Option<u64>, limit_millivolts: u32) -> Option<String> {
    let limit_uv = u64::from(limit_millivolts) * 1000;
    drop_microvolts.filter(|&v| u128::from(v) * 3 > u128::from(limit_uv) * 2).map(|_| {
        format!(
            "the bound is within 1.5x of the limit: the gate's {} um pitch sets part of this margin, and a finer pitch is the known remedy",
            PITCH_NM as f64 / 1000.0
        )
    })
}

fn bounded_pin(p: &Pad, b: &PinBound) -> PinResult {
    // µΩ, rounded up past the product's own rounding
    let micro = (b.bound.energy.hi * 1e6).next_up().ceil();
    if !(micro.is_finite() && (0.0..1e15).contains(&micro)) {
        return PinResult::Refused {
            pin: p.number.clone(),
            kind: "SupplyPinUnbounded",
            message: format!(
                "the bound is not a finite resistance: {}",
                b.bound.energy.hi
            ),
        };
    }
    PinResult::Bounded {
        pin: p.number.clone(),
        r_bound_microohms: micro as u64,
    }
}

fn from_ref(board: &Board, pad: &Pad) -> String {
    board
        .parts
        .iter()
        .find(|p| p.pads.iter().any(|q| std::ptr::eq(q, pad)))
        .map_or_else(String::new, |p| p.reference.clone())
}

fn to_refused(r: Refusal) -> crate::extract::Refused {
    match r {
        Refusal::Named(kind, message) => refusal(kind, message),
        Refusal::Flow(f) => refusal("SupplyPinUnbounded", format!("{f:?}")),
    }
}

/// The conditions every supply bound rests on, stated in the verdict.
fn conditions(
    position: &aethelgard_ir::board::Sourced,
    plus: &aethelgard_ir::board::Sourced,
    minus: &aethelgard_ir::board::Sourced,
    etch: &aethelgard_ir::board::Sourced,
) -> Vec<String> {
    vec![
        "supply: R is a certified upper bound (Thomson's principle) on a subset of the net's copper: every flux-carrying cell lies wholly inside copper the gate reads; copper left out only raises it".into(),
        format!("supply: every hole anywhere within +-{} mm per axis of its pad ({})", f64::from(position.value) / 1e3, position.source),
        format!("supply: every finished hole from {} mm under to {} mm over its drawn size ({} / {}), lined by at least the declared plating minimum", f64::from(minus.value) / 1e3, f64::from(plus.value) / 1e3, minus.source, plus.source),
        format!("supply: every track at {} of its drawn width ({})", f64::from(1000 - etch.value.min(1000)) / 1e3, etch.source),
        "supply: pads and via pads at their drawn size: no pad etch tolerance is cited".into(),
        "supply: each terminal pad an equipotential: the current enters and leaves through the whole pad".into(),
        "supply: the source is where the regulator's current enters the domain (a buck's inductor pad); at DC it is the net's highest potential, so the bound covers the regulator's sense point too".into(),
        "supply: the return is the regulator's own ground pin; every other current in the ground copper (each other regulated domain's load at its consumers' ground pads, each other battery-fed regulator's input current at its own ground pin) is charged at most its current times the lesser of the load's largest return bound and the largest bound from its entry pads to that pin (the maximum principle, AET-WO-4310 (3)); where it leaves the copper only lowers the load's return".into(),
        "supply: the load's VDD and VSS pins are tied inside it, so the worst supply pin and the worst return pin bound its drop [inferred]".into(),
        "supply: copper resistivity IACS x (1 + 0.00393 x (T - 20)) x 1.181 (IPC-MF-150F 3.8.1.2's foil maximum, applied to plated copper as a stated condition), at the declared copper temperature maximum".into(),
        format!("supply: plane fill at least the outline less {} mm and each knockout's measured {} mm clearance and {} mm min-thickness opening (KiCad 10.0.7, measured)", EDGE_NM as f64 / 1e6, FILL_CLEARANCE_NM as f64 / 1e6, OPENING_NM as f64 / 1e6),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pcb::Rect;
    use crate::route::{Part, Segment, Via};

    fn pad(number: &str, net: &str, x0: i64, y0: i64, x1: i64, y1: i64) -> Pad {
        Pad {
            number: number.into(),
            net: Some(net.into()),
            rect: Rect { x0, y0, x1, y1 },
            corner_nm: 0,
            drill_nm: None,
            join: None,
        }
    }

    // ---- `AET-WO-4310` (3): another current's charge ([`charge`]) ----

    fn charged(
        entry: &[&Pad],
        r11: Option<u64>,
        answers: &dyn Fn(&str) -> Result<u64, String>,
        calls: &std::cell::Cell<usize>,
    ) -> Other {
        let ret = pad("R", "GND", 0, 0, 1, 1);
        charge("x".into(), 2000, entry, &ret, r11, &|ps| {
            calls.set(calls.get() + 1);
            Ok(ps.iter().map(|p| answers(&p.number)).collect())
        })
        .unwrap()
    }

    #[test]
    fn another_current_is_charged_at_the_lesser_of_its_entry_bound_and_the_loads() {
        let calls = std::cell::Cell::new(0);
        let near = pad("1", "GND", 10, 0, 11, 1);
        let far = pad("2", "GND", 9000, 0, 9001, 1);
        // Entry bounds 30 and 40 mOhm, the load's 65: the lesser, 40, at 2 A.
        let o = charged(&[&near, &far], Some(65_000), &|n| Ok(if n == "2" { 40_000 } else { 30_000 }), &calls);
        assert_eq!(o.entry_bound_microohms, Some(40_000));
        assert_eq!(o.charge_microvolts, Some(80_000));
        // An entry bound past the load's: charged at the load's.
        let o = charged(&[&near], Some(25_000), &|_| Ok(30_000), &calls);
        assert_eq!(o.charge_microvolts, Some(50_000));
        // The load's own return unbounded: no charge, so no drop bound.
        assert_eq!(charged(&[&near], None, &|_| Ok(1), &calls).charge_microvolts, None);
    }

    #[test]
    fn a_pad_the_gate_cannot_read_or_no_pad_is_charged_at_the_loads_bound_and_named() {
        let calls = std::cell::Cell::new(0);
        let p = pad("7", "GND", 10, 0, 11, 1);
        let o = charged(&[&p], Some(65_000), &|_| Err("7: TerminalDrilled".into()), &calls);
        assert_eq!(o.entry_bound_microohms, None);
        assert_eq!(o.charge_microvolts, Some(130_000));
        assert_eq!(o.unread, ["7: TerminalDrilled"]);
        let o = charged(&[], Some(65_000), &|_| Ok(1), &calls);
        assert_eq!(o.charge_microvolts, Some(130_000));
        assert_eq!(o.unread, ["no ground pad on the board"]);
        // One pad read at 10 mOhm and one not: the unread one may be worse,
        // so the charge stays at the load's bound.
        let q = pad("8", "GND", 20, 0, 21, 1);
        let o = charged(
            &[&p, &q],
            Some(65_000),
            &|n| if n == "7" { Err("7: TerminalDrilled".into()) } else { Ok(10_000) },
            &calls,
        );
        assert_eq!(o.entry_bound_microohms, None);
        assert_eq!(o.charge_microvolts, Some(130_000));
    }

    #[test]
    fn entry_pads_are_solved_farthest_first_and_stop_once_one_reaches_the_loads_bound() {
        // Seven pads: the farthest, alone in no chunk of its own, reaches
        // the load's bound, so the second chunk is never solved.
        let pads: Vec<Pad> = (0..7)
            .map(|i| pad(&i.to_string(), "GND", 100 * (i + 1), 0, 100 * (i + 1) + 1, 1))
            .collect();
        let refs: Vec<&Pad> = pads.iter().collect();
        let calls = std::cell::Cell::new(0);
        let o = charged(&refs, Some(50_000), &|n| Ok(if n == "6" { 60_000 } else { 10_000 }), &calls);
        assert_eq!(calls.get(), 1, "one chunk of {ENTRY_THREADS}");
        assert_eq!(o.charge_microvolts, Some(100_000));
        // Nothing reaches it: every chunk is solved.
        let calls = std::cell::Cell::new(0);
        let o = charged(&refs, Some(50_000), &|_| Ok(10_000), &calls);
        assert_eq!(calls.get(), 2);
        assert_eq!(o.charge_microvolts, Some(20_000));
    }

    fn part(reference: &str, pads: Vec<Pad>) -> Part {
        Part {
            reference: reference.into(),
            rotation: 0,
            pads,
            courtyard: vec![],
            keepouts: vec![],
            inner_keepouts: Vec::new(),
        }
    }

    /// A 2-layer board 20 × 6 mm, 0.25 mm tracks.
    fn board(parts: Vec<Part>) -> Board {
        let mut stack = crate::stack::reference();
        stack.width_nm = 250_000;
        Board {
            outline: Rect {
                x0: 0,
                y0: 0,
                x1: 20_000_000,
                y1: 6_000_000,
            },
            parts,
            stack,
        }
    }

    fn model() -> Model {
        Model {
            h_nm: PITCH_NM,
            rho: rho_max(125),
            outer_nm: (30_000, 70_000),
            plane_nm: (15_000, 70_000),
            plating_nm: 18_000,
            beta_nm: BETA_NM,
            // JLCPCB's position tolerance; hole size and etch at 0, so the
            // exact cases stay exact (each has its own test)
            hole_offset_nm: 75_000,
            hole_plus_nm: 0,
            hole_minus_nm: 0,
            track_etch_permille: 0,
            edge_nm: EDGE_NM,
            crop_nm: CROP_NM,
            fill_clearance_nm: FILL_CLEARANCE_NM,
            opening_nm: OPENING_NM,
        }
    }

    fn seg(layer: Layer, a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment {
            net: "VDD".into(),
            layer,
            start: a,
            end: b,
        }
    }

    /// A straight 0.25 mm track between two pads 10 mm apart: its cells
    /// fill the track exactly, so the bound is the bar's exact
    /// `ρL/(Wt)` at the minimum thickness and the resistivity bound.
    #[test]
    fn a_straight_track_is_bounded_at_its_exact_resistance() {
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad(
                    "1", "VDD", 12_000_000, 2_000_000, 13_000_000, 3_000_000,
                )],
            ),
        ]);
        let routed = Routed {
            segments: vec![seg(
                Layer::F,
                (1_500_000, 2_500_000),
                (12_500_000, 2_500_000),
            )],
            ..Routed::default()
        };
        let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
        let m = model();
        let nc = copper(&b, &routed, "VDD", &[src, snk], &m).unwrap();
        let r = bound(&nc, &m, src, snk, 400).unwrap().bound.energy.hi;
        let exact = m.rho.hi * 10.0 / (0.25 * 0.03);
        eprintln!("track: bound {r:.6} exact {exact:.6}");
        assert!(r >= exact, "{r} under the exact {exact}");
        assert!(r <= exact * (1.0 + 1e-6), "{r} over the exact {exact}");
        // the etch: at JLCPCB's 20 %, the track is priced no wider than
        // 0.2 mm, so at least that width's exact resistance
        let etched = Model {
            track_etch_permille: 200,
            ..model()
        };
        let nc = copper(&b, &routed, "VDD", &[src, snk], &etched).unwrap();
        let r = bound(&nc, &etched, src, snk, 400).unwrap().bound.energy.hi;
        let narrow = exact * 0.25 / 0.2;
        assert!(r >= narrow, "{r} under the etched track's exact {narrow}");
    }

    /// Two vias carry the current down to B.Cu and back: the bound holds
    /// above the tracks' bars and both barrels at their shortest, and not
    /// far above.
    #[test]
    fn a_path_through_two_vias_is_bounded_above_its_floor() {
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad(
                    "1", "VDD", 15_000_000, 2_000_000, 16_000_000, 3_000_000,
                )],
            ),
        ]);
        let (v1, v2) = ((6_000_000, 2_500_000), (11_000_000, 2_500_000));
        let routed = Routed {
            segments: vec![
                seg(Layer::F, (1_500_000, 2_500_000), v1),
                seg(Layer::B, v1, v2),
                seg(Layer::F, v2, (15_500_000, 2_500_000)),
            ],
            vias: vec![
                Via {
                    net: "VDD".into(),
                    at: v1,
                },
                Via {
                    net: "VDD".into(),
                    at: v2,
                },
            ],
            ..Routed::default()
        };
        let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
        let m = model();
        let nc = copper(&b, &routed, "VDD", &[src, snk], &m).unwrap();
        let pb = bound(&nc, &m, src, snk, 400).unwrap();
        let r = pb.bound.energy.hi;
        let rho = m.rho.hi;
        // tracks between pad edges and via pads' edges (0.3 mm radius)
        let bars = rho * ((6.0 - 0.3 - 2.0) + (11.0 - 0.3 - 6.3) + (15.0 - 11.3)) / (0.25 * 0.03);
        // each barrel at its shortest, the dielectric's minimum 0.2 mm, its
        // shell the plating's minimum on the finished wall: area
        // π t (2 r + t) for a finished radius r
        let shell = |r: f64| std::f64::consts::PI * 0.018 * (2.0 * r + 0.018);
        let barrel = rho * 0.2 / shell(0.15);
        let floor = bars + 2.0 * barrel;
        eprintln!(
            "two vias: bound {r:.5} floor {floor:.5} (shares {:?})",
            pb.shares
        );
        assert!(r >= floor, "{r} under the floor {floor}");
        assert!(r <= floor * 1.6, "{r} far over the floor {floor}");
        assert!(pb.via_transfer.iter().all(|(_, q)| *q == flow::QUANTA));
        // a hole that may finish 0.08 mm small: each barrel at least the
        // smallest shell's
        let small = Model {
            hole_minus_nm: 80_000,
            ..model()
        };
        let nc = copper(&b, &routed, "VDD", &[src, snk], &small).unwrap();
        let r_small = bound(&nc, &small, src, snk, 400).unwrap().bound.energy.hi;
        let floor_small = bars + 2.0 * rho * 0.2 / shell(0.11);
        assert!(r_small >= floor_small, "{r_small} under {floor_small}");
        assert!(r_small > r, "{r_small} not above {r}");
        // a hole that may finish 0.13 mm large: a 0.6 mm pad cannot keep the
        // band at the largest shell and every position; the via is left out
        // and the pad that would is named, 0.7 mm
        let large = Model {
            hole_plus_nm: 130_000,
            ..model()
        };
        let nc = copper(&b, &routed, "VDD", &[src, snk], &large).unwrap();
        assert!(nc.vias.is_empty());
        assert_eq!(nc.unfit, [(600_000, 300_000, 700_000)]);
    }

    /// A through-hole pad of the net is a via (the §7.1 row): its barrel,
    /// at the pad's drill, carries the current from F.Cu to B.Cu when no
    /// via does.
    #[test]
    fn a_through_hole_pad_carries_the_current_as_a_barrel() {
        let mut th = pad("1", "VDD", 5_500_000, 1_750_000, 7_000_000, 3_250_000);
        th.drill_nm = Some(800_000);
        th.corner_nm = 750_000;
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad(
                    "1", "VDD", 15_000_000, 2_000_000, 16_000_000, 3_000_000,
                )],
            ),
            part("J3", vec![th]),
        ]);
        let (c, v2) = ((6_250_000, 2_500_000), (11_000_000, 2_500_000));
        let routed = Routed {
            segments: vec![
                seg(Layer::F, (1_500_000, 2_500_000), c),
                seg(Layer::B, c, v2),
                seg(Layer::F, v2, (15_500_000, 2_500_000)),
            ],
            vias: vec![Via {
                net: "VDD".into(),
                at: v2,
            }],
            ..Routed::default()
        };
        let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
        let m = model();
        let nc = copper(&b, &routed, "VDD", &[src, snk], &m).unwrap();
        let pb = bound(&nc, &m, src, snk, 400).unwrap();
        let through = pb.via_transfer.iter().find(|(at, _)| *at == c).unwrap();
        assert_eq!(through.1, flow::QUANTA, "{:?}", pb.via_transfer);
    }

    /// A dielectric's declared maximum, below the finished maximum less the
    /// other rows' minima, bounds the barrel's row: the same copper costs
    /// less with it, and never more.
    #[test]
    fn a_declared_dielectric_maximum_bounds_the_barrel_row() {
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad(
                    "1", "VDD", 15_000_000, 2_000_000, 16_000_000, 3_000_000,
                )],
            ),
        ]);
        let (v1, v2) = ((6_000_000, 2_500_000), (11_000_000, 2_500_000));
        let routed = Routed {
            segments: vec![
                seg(Layer::F, (1_500_000, 2_500_000), v1),
                seg(Layer::B, v1, v2),
                seg(Layer::F, v2, (15_500_000, 2_500_000)),
            ],
            vias: [v1, v2]
                .iter()
                .map(|&at| Via {
                    net: "VDD".into(),
                    at,
                })
                .collect(),
            ..Routed::default()
        };
        let m = model();
        let r = |b: &Board| {
            let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
            let nc = copper(b, &routed, "VDD", &[src, snk], &m).unwrap();
            bound(&nc, &m, src, snk, 400).unwrap().bound.energy.hi
        };
        let open = r(&b);
        let mut tight = b.clone();
        let min = tight.stack.dielectrics[0].2;
        tight.stack.dielectric_max_nm = vec![Some(min)];
        let declared = r(&tight);
        assert!(declared < open, "{declared} against {open}");
    }

    /// A hole that may finish at no size at all has no shell to price: the
    /// via is left out by name, not priced.
    #[test]
    fn a_hole_that_may_finish_closed_leaves_the_via_out() {
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad("1", "VDD", 9_000_000, 2_000_000, 10_000_000, 3_000_000)],
            ),
        ]);
        let routed = Routed {
            segments: vec![seg(
                Layer::F,
                (1_500_000, 2_500_000),
                (9_500_000, 2_500_000),
            )],
            vias: vec![Via {
                net: "VDD".into(),
                at: (6_000_000, 2_500_000),
            }],
            ..Routed::default()
        };
        let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
        let mut m = model();
        let drill = b.stack.rule("VDD").drill_nm;
        // a tolerance larger than the hole itself
        m.hole_minus_nm = drill + 100_000;
        let nc = copper(&b, &routed, "VDD", &[src, snk], &m).unwrap();
        assert!(nc.vias.is_empty());
        assert!(
            nc.notes.iter().any(|n| n.contains("cannot hold the band")),
            "{:?}",
            nc.notes
        );
    }

    /// A bound that is not a finite resistance refuses its pin by name.
    #[test]
    fn a_bound_that_is_not_finite_refuses_its_pin() {
        let pb = PinBound {
            bound: Bound {
                energy: Iv {
                    lo: 0.0,
                    hi: f64::INFINITY,
                },
                kinds: [Iv::exact(0.0); 3],
                flux: vec![],
            },
            shares: [0.0; 3],
            via_transfer: vec![],
        };
        let p = pad("1", "VDD", 0, 0, 1, 1);
        assert!(matches!(
            bounded_pin(&p, &pb),
            PinResult::Refused {
                kind: "SupplyPinUnbounded",
                ..
            }
        ));
    }

    /// A hole's clearance is its drill's radius plus the tolerance's
    /// diagonal reach, rounded up.
    #[test]
    fn a_hole_is_cleared_at_every_position_its_tolerance_allows() {
        // 150 000 + ⌈75 000 √2⌉ = 150 000 + 106 067
        assert_eq!(hole_radius(300_000, 75_000), 256_067);
        assert_eq!(hole_radius(300_001, 0), 150_001);
        // and at the largest finished size: 0.43 mm across
        let m = Model {
            hole_plus_nm: 130_000,
            ..model()
        };
        assert_eq!(hole_clear(300_000, &m), 215_000 + 106_067);
        // the shell between the smallest and the largest finished hole,
        // lined by the plating's minimum, and the pad that holds its band
        let m = Model {
            hole_minus_nm: 80_000,
            ..m
        };
        assert_eq!(
            shell_radii(300_000, &m),
            (110_000 + 18_000, 215_000 + 18_000)
        );
        assert_eq!(pad_needed(300_000, &m), 700_000);
        // the barrel at the smallest shell, where its coefficient is worst
        let hole = barrel_hole(300_000, &m);
        assert_eq!(hole.r_d.hi, Iv::ratio(128_000, 1_000_000).hi);
        let coefficient = |r: f64| r / (0.018 * (2.0 * r - 0.018));
        assert!(coefficient(0.128) > coefficient(0.233));
    }

    /// The witness disc: a cell in a knockout's opening ring is copper only
    /// when one disc of the opening's radius, clear of every raw knockout,
    /// covers it. A cell at the raw edge is not; one well outside is.
    #[test]
    fn a_ring_cell_is_copper_only_when_a_witness_disc_covers_it() {
        let (w, clear) = (OPENING_NM - ARC_NM, OPENING_NM + ARC_NM);
        let k = ((0, 0), 500_000);
        let inner = (-10_000_000, -10_000_000, 10_000_000, 10_000_000);
        let h = PITCH_NM;
        // just outside the raw knockout: no disc of radius w both covers it
        // and clears the knockout
        assert!(!witnessed((500_000, 0, h), &[k], inner, w, clear));
        // farther out, within the conservative ring
        assert!(witnessed((560_000, 0, h), &[k], inner, w, clear));
        // the same cell pinched between two knockouts is not
        let k2 = ((1_180_000, 31_250), 500_000);
        assert!(!witnessed((560_000, 0, h), &[k, k2], inner, w, clear));
        // nor next to the board's raw edge
        assert!(!witnessed(
            (560_000, 0, h),
            &[k],
            (0, -10_000_000, 600_000, 10_000_000),
            w,
            clear
        ));
    }

    /// The copper-hull test: a hull inside a disc and a track's stadium
    /// passes; one reaching past both fails.
    #[test]
    fn a_ray_hull_must_lie_inside_copper() {
        let shapes = [
            Shape::Disc {
                c: (0, 0),
                r: S * 350_000,
            },
            Shape::Stadium {
                a: (0, 0),
                b: (S * 2_000_000, 0),
                hw: S * 125_000,
            },
        ];
        let inside = [
            (300_000, -60_000),
            (420_000, -60_000),
            (420_000, 60_000),
            (300_000, 60_000),
            (-75_000, -75_000),
            (-75_000, 75_000),
        ];
        assert!(hull_in_copper(&flow::convex_hull(&inside), &shapes, 8));
        let outside = [
            (300_000, -200_000),
            (420_000, -200_000),
            (420_000, 60_000),
            (-75_000, 75_000),
        ];
        assert!(!hull_in_copper(&flow::convex_hull(&outside), &shapes, 8));
        // and clear of every hole: one inside it, or reaching an edge, fails
        let hull = flow::convex_hull(&inside);
        assert!(hull_clear_of_holes(&hull, &[((300_000, 300_000), 100_000)]));
        assert!(!hull_clear_of_holes(&hull, &[((200_000, 0), 10_000)]));
        assert!(!hull_clear_of_holes(&hull, &[((350_000, 100_000), 50_000)]));
    }

    /// A via whose pad cannot hold the band at every hole position is left
    /// out, named in the notes, its hole kept clear.
    #[test]
    fn a_via_too_small_for_the_worst_hole_is_left_out() {
        let b = board(vec![
            part(
                "J1",
                vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
            ),
            part(
                "J2",
                vec![pad(
                    "1", "VDD", 12_000_000, 2_000_000, 13_000_000, 3_000_000,
                )],
            ),
        ]);
        let mut stack = b.stack.clone();
        stack.classes.push(crate::stack::Class {
            name: "SMALL".into(),
            rule: crate::stack::Rule {
                width_nm: 250_000,
                via_nm: 450_000,
                drill_nm: 300_000,
            },
            via_declared: true,
            nets: vec!["VDD".into()],
        });
        let b = Board { stack, ..b };
        let routed = Routed {
            segments: vec![seg(
                Layer::F,
                (1_500_000, 2_500_000),
                (12_500_000, 2_500_000),
            )],
            vias: vec![Via {
                net: "VDD".into(),
                at: (6_000_000, 2_500_000),
            }],
            ..Routed::default()
        };
        let (src, snk) = (&b.parts[0].pads[0], &b.parts[1].pads[0]);
        let nc = copper(&b, &routed, "VDD", &[src, snk], &model()).unwrap();
        assert!(nc.vias.is_empty());
        assert!(
            nc.notes.iter().any(|n| n.contains("cannot hold the band")),
            "{:?}",
            nc.notes
        );
    }

    /// A 4-layer board, VDD's plane on In1, 0.25 mm tracks.
    fn board4(parts: Vec<Part>) -> Board {
        let mut b = board(parts);
        b.stack.copper = vec![
            "F.Cu".into(),
            "In1.Cu".into(),
            "In2.Cu".into(),
            "B.Cu".into(),
        ];
        b.stack.planes = vec![
            ("In1.Cu".into(), "VDD".into()),
            ("In2.Cu".into(), "GND".into()),
        ];
        b.stack.thickness.min_nm = 1_250_000;
        b.stack.thickness.max_nm = 1_851_000;
        b.stack.dielectric_max_nm = vec![None; 3];
        b.stack.dielectrics = vec![
            ("F.Cu".into(), "In1.Cu".into(), 179_000, "s".into()),
            ("In1.Cu".into(), "In2.Cu".into(), 891_000, "s".into()),
            ("In2.Cu".into(), "B.Cu".into(), 179_000, "s".into()),
        ];
        b
    }

    fn joins_of(nc: &NetCopper, at: (i64, i64)) -> Vec<String> {
        nc.vias
            .iter()
            .find(|u| u.via.at == at)
            .map(|u| {
                u.joins
                    .iter()
                    .map(|(l, _)| nc.layers[*l].name.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// A through-hole pad whose plane join is a thermal relief joins no
    /// plane (`ReliefUnpriced`): its spokes are not priced. Solidly joined,
    /// it joins its plane.
    #[test]
    fn a_relieved_pad_joins_no_plane() {
        for (join, plane) in [
            (crate::stack::Join::Relief, false),
            (crate::stack::Join::Solid, true),
        ] {
            let mut th = pad("1", "VDD", 5_500_000, 1_750_000, 7_000_000, 3_250_000);
            th.drill_nm = Some(800_000);
            th.corner_nm = 750_000;
            th.join = Some(join);
            let b = board4(vec![
                part(
                    "J1",
                    vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
                ),
                part("J3", vec![th]),
            ]);
            let routed = Routed {
                segments: vec![seg(
                    Layer::F,
                    (1_500_000, 2_500_000),
                    (6_250_000, 2_500_000),
                )],
                ..Routed::default()
            };
            let src = &b.parts[0].pads[0];
            let nc = copper(&b, &routed, "VDD", &[src, &b.parts[1].pads[0]], &model()).unwrap();
            let joins = joins_of(&nc, (6_250_000, 2_500_000));
            assert_eq!(
                joins.contains(&"In1.Cu".to_string()),
                plane,
                "{join:?}: {joins:?}"
            );
            assert_eq!(
                nc.notes.iter().any(|n| n.starts_with("ReliefUnpriced")),
                !plane,
                "{:?}",
                nc.notes
            );
        }
    }

    /// A pocket of plane between four foreign knockouts, which a witness
    /// disc admits, is an island KiCad's fill drops: no via port reaches it,
    /// so it is left out too.
    #[test]
    fn a_plane_island_is_left_out() {
        let b = board4(vec![part(
            "J1",
            vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
        )]);
        // four GND vias 1 mm apart about (6, 3) mm, inside the crop: their raw knockouts
        // (0.515 mm) overlap and close a pocket a witness disc admits; and
        // one VDD via
        let c = (6_000_000, 3_000_000);
        let routed = Routed {
            vias: vec![
                Via {
                    net: "GND".into(),
                    at: (c.0 - 500_000, c.1 - 500_000),
                },
                Via {
                    net: "GND".into(),
                    at: (c.0 + 500_000, c.1 - 500_000),
                },
                Via {
                    net: "GND".into(),
                    at: (c.0 - 500_000, c.1 + 500_000),
                },
                Via {
                    net: "GND".into(),
                    at: (c.0 + 500_000, c.1 + 500_000),
                },
                Via {
                    net: "VDD".into(),
                    at: (4_000_000, 3_000_000),
                },
            ],
            segments: vec![
                seg(Layer::F, (1_500_000, 2_500_000), (4_000_000, 2_500_000)),
                seg(Layer::F, (4_000_000, 2_500_000), (4_000_000, 3_000_000)),
            ],
            ..Routed::default()
        };
        let src = &b.parts[0].pads[0];
        let nc = copper(&b, &routed, "VDD", &[src], &model()).unwrap();
        let plane = nc.layers.iter().find(|l| l.name == "In1.Cu").unwrap();
        let h = PITCH_NM;
        let pocket = ((c.0 - nc.origin.0) / h, (c.1 - nc.origin.1) / h);
        assert!(
            !plane
                .cells
                .iter()
                .any(|&q| (q.0 - pocket.0).abs() <= 1 && (q.1 - pocket.1).abs() <= 1),
            "the pocket stayed"
        );
        assert!(
            nc.notes.iter().any(|n| n.contains("islands")),
            "{:?}",
            nc.notes
        );
        // and the plane around the VDD via is there
        assert!(plane.cells.len() > 1000);
    }

    /// A foreign via's knockout reaching into one side of a via's junction
    /// region costs the plane only that side's faces: each face is a port
    /// when the hull of its column lies in plane copper, not only when the
    /// whole region does.
    #[test]
    fn a_knockout_beside_a_via_costs_the_plane_only_its_own_side() {
        let b = board4(vec![part(
            "J1",
            vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
        )]);
        let v = (4_000_000, 3_000_000);
        let routed = Routed {
            vias: vec![
                Via {
                    net: "VDD".into(),
                    at: v,
                },
                // its raw knockout, 0.515 mm, reaches to 0.185 mm of the VDD
                // via: into the junction region's own cells
                Via {
                    net: "GND".into(),
                    at: (v.0, v.1 + 700_000),
                },
            ],
            segments: vec![
                seg(Layer::F, (1_500_000, 2_500_000), (4_000_000, 2_500_000)),
                seg(Layer::F, (4_000_000, 2_500_000), v),
            ],
            ..Routed::default()
        };
        let src = &b.parts[0].pads[0];
        let nc = copper(&b, &routed, "VDD", &[src], &model()).unwrap();
        let plane = nc.layers.iter().position(|l| l.name == "In1.Cu").unwrap();
        let u = nc.vias.iter().find(|u| u.via.at == v).unwrap();
        let ports = &u.joins.iter().find(|(l, _)| *l == plane).unwrap().1;
        let jn = &nc.junctions[u.junction];
        let have: BTreeSet<Cell> = nc.layers[plane].cells.iter().copied().collect();
        let centre = (
            (v.0 - nc.origin.0) / PITCH_NM,
            (v.1 - nc.origin.1) / PITCH_NM,
        );
        let beside = jn
            .region
            .faces
            .iter()
            .filter(|f| have.contains(&(centre.0 + f.outside.0, centre.1 + f.outside.1)))
            .count();
        assert!(!ports.is_empty(), "the plane joins the via");
        assert!(ports.len() < beside, "{} of {beside}", ports.len());
        // no port faces the knockout
        assert!(ports.iter().all(|&k| jn.region.faces[k].normal != (0, 1)));
    }

    /// A hole of the net under a via's junction (an off-grid via of the
    /// net 0.21 mm away, closer than any board allows) lies in every port's
    /// hull: no outer face is joined, though the track's cells beyond it are
    /// copper and clear of the hole.
    #[test]
    fn a_hole_inside_every_ports_hull_joins_no_face() {
        let b = board(vec![part(
            "J1",
            vec![pad("1", "VDD", 1_000_000, 2_500_000, 2_000_000, 3_500_000)],
        )]);
        let v = (6_000_000, 3_000_000);
        let routed = Routed {
            segments: vec![seg(Layer::F, (1_500_000, 3_000_000), v)],
            vias: vec![
                Via {
                    net: "VDD".into(),
                    at: v,
                },
                Via {
                    net: "VDD".into(),
                    at: (v.0 + 210_000, v.1),
                },
            ],
            ..Routed::default()
        };
        let src = &b.parts[0].pads[0];
        let nc = copper(&b, &routed, "VDD", &[src], &model()).unwrap();
        let u = nc.vias.iter().find(|u| u.via.at == v).unwrap();
        let f_cu = nc.layers.iter().position(|l| l.name == "F.Cu").unwrap();
        let ports = &u.joins.iter().find(|(l, _)| *l == f_cu).unwrap().1;
        assert!(
            ports.is_empty(),
            "{} faces joined across the hole",
            ports.len()
        );
    }

    /// No port's rays cross a hole of the net. Copper of one net may
    /// overlap, so an off-grid through-hole pad of the net can sit beside
    /// the net's via, closer than DRC's hole-to-hole spacing allows, its pad
    /// copper all round: every port's hull still clears the hole, at its
    /// largest and anywhere the tolerance puts it.
    #[test]
    fn a_port_whose_rays_cross_a_hole_of_the_net_is_not_joined() {
        let v = (6_000_000, 3_000_000);
        // off the 62.5 um grid, so a hole kept clear, not a via
        let c = (v.0 + 410_000, v.1);
        let mut th = pad(
            "1",
            "VDD",
            c.0 - 500_000,
            c.1 - 500_000,
            c.0 + 500_000,
            c.1 + 500_000,
        );
        th.drill_nm = Some(300_000);
        th.corner_nm = 500_000;
        let b = board(vec![part("J1", vec![th])]);
        let routed = Routed {
            segments: vec![seg(Layer::F, (v.0 - 1_000_000, v.1), v)],
            vias: vec![Via {
                net: "VDD".into(),
                at: v,
            }],
            ..Routed::default()
        };
        let src = &b.parts[0].pads[0];
        let m = model();
        let nc = copper(&b, &routed, "VDD", &[src], &m).unwrap();
        let u = &nc.vias[0];
        let jn = &nc.junctions[u.junction];
        let f_cu = nc.layers.iter().position(|l| l.name == "F.Cu").unwrap();
        let ports = &u.joins.iter().find(|(l, _)| *l == f_cu).unwrap().1;
        assert!(!ports.is_empty(), "the track side joins");
        // every port's hull clears the pad's hole, at its largest and
        // anywhere the tolerance puts it
        let e = m.hole_offset_nm;
        for &k in ports {
            let f = &jn.region.faces[k];
            let (nx, ny) = f.normal;
            let (tx, ty) = (-ny, nx);
            let p = |s: i64| {
                (
                    v.0 + (f.d0 * nx + s * tx) * PITCH_NM,
                    v.1 + (f.d0 * ny + s * ty) * PITCH_NM,
                )
            };
            let pts = [
                p(f.u0),
                p(f.u0 + 1),
                (v.0 - e, v.1 - e),
                (v.0 + e, v.1 - e),
                (v.0 + e, v.1 + e),
                (v.0 - e, v.1 + e),
            ];
            let hull = flow::convex_hull(&pts);
            assert!(
                hull_clear_of_holes(&hull, &[(c, hole_clear(300_000, &m))]),
                "face {k}"
            );
        }
    }

    /// A port whose rays would leave copper is no port: a pad of the net
    /// starting 12.5 µm beyond the via's 0.6 mm pad borders the junction
    /// region, but every ray from its cells to the hole crosses the gap.
    #[test]
    fn a_port_whose_rays_leave_copper_is_not_joined() {
        let v = (6_000_000, 3_000_000);
        let b = board(vec![part(
            "J1",
            vec![pad(
                "1",
                "VDD",
                v.0 + 312_500,
                v.1 - 500_000,
                v.0 + 1_312_500,
                v.1 + 500_000,
            )],
        )]);
        let routed = Routed {
            vias: vec![Via {
                net: "VDD".into(),
                at: v,
            }],
            ..Routed::default()
        };
        let src = &b.parts[0].pads[0];
        let nc = copper(&b, &routed, "VDD", &[src], &model()).unwrap();
        let u = &nc.vias[0];
        let jn = &nc.junctions[u.junction];
        let f_cu = nc.layers.iter().position(|l| l.name == "F.Cu").unwrap();
        let cells: BTreeSet<Cell> = nc.layers[f_cu].cells.iter().copied().collect();
        let centre = (
            (v.0 - nc.origin.0) / PITCH_NM,
            (v.1 - nc.origin.1) / PITCH_NM,
        );
        let candidates = jn
            .region
            .faces
            .iter()
            .filter(|f| cells.contains(&(centre.0 + f.outside.0, centre.1 + f.outside.1)))
            .count();
        let ports = u
            .joins
            .iter()
            .find(|(l, _)| *l == f_cu)
            .map_or(0, |(_, p)| p.len());
        assert!(candidates > 0);
        assert_eq!(
            ports, 0,
            "{ports} of {candidates} faces beside copper joined"
        );
    }

    /// The drop is the worst supply pin plus the worst return pin, times the
    /// load, rounded up; a refused pin leaves none.
    #[test]
    fn the_drop_is_the_worst_pair_times_the_load() {
        let b = |pin: &str, r| PinResult::Bounded {
            pin: pin.into(),
            r_bound_microohms: r,
        };
        let pins = [b("1", 30_000), b("2", 52_055)];
        let rets = [b("3", 47_181), b("4", 33_000)];
        // (52 055 + 47 181) µΩ × 659 mA = 65 396 524 nV: 65 397 µV
        assert_eq!(drop_bound(&pins, &rets, 659), Some(65_397));
        let refused = [PinResult::Refused {
            pin: "5".into(),
            kind: "GroundReturnUnread",
            message: String::new(),
        }];
        assert_eq!(drop_bound(&pins, &refused, 659), None);
        assert_eq!(drop_bound(&pins, &[], 659), None);
        // `AET-WO-4411` (ruled 1 (a')): a module's 500 mA of the 659 at the
        // lesser bound each side; a side with no bound of its own at the
        // load's: (52055 + 47181) x 159 + (40000 + 47181) x 500 nV,
        // 59,369,024, rounded up to 59,370 uV.
        let m = |supply, ret| Shared {
            reference: "U30".into(),
            milliamps: 500,
            supply_bound_microohms: supply,
            return_bound_microohms: ret,
        };
        assert_eq!(
            drop_bound_shared(&pins, &rets, 659, &m(Some(40_000), None)),
            Some(59_370)
        );
        // an own bound above the load's charges the load's, never more
        assert_eq!(
            drop_bound_shared(&pins, &rets, 659, &m(Some(90_000), Some(90_000))),
            drop_bound(&pins, &rets, 659)
        );
        assert_eq!(drop_bound_shared(&pins, &rets, 499, &m(None, None)), None);
        assert_eq!(
            drop_bound_shared(&pins, &refused, 659, &m(None, None)),
            None
        );
        // the note at more than two thirds of the limit, not at it
        assert!(pitch_note(Some(110_001), 165).is_some());
        assert!(pitch_note(Some(110_000), 165).is_none());
        assert!(pitch_note(None, 165).is_none());
    }
}
