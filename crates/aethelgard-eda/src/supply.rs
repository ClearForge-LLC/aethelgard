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

use crate::flow::{
    self, Bound, Cell, FlowError, Hole, Iv, Join, JunctionTable, Network, Region, Sheet,
};
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
    /// The fabricator's hole position tolerance, per axis, nm: every hole
    /// is priced at every position within it.
    pub hole_offset_nm: i64,
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
        self.knockout_raw(d) + self.opening_nm
    }

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

/// A cell near a fill knockout is copper when one disc of radius `w` lies
/// in the raw fill (clear of every knockout `(centre, raw radius)` in
/// `near`, and inside `inner`, the raw outline shrunk by `w`) and covers
/// the cell's four corners: KiCad's opening (deflate, then inflate, by the
/// min-thickness half-width) keeps every point such a disc covers.
/// Candidates: the cell's centre, pushed by up to `w` away from each
/// knockout, away from all together, and along the eight compass
/// directions. Exact in integers for the candidate tried; a sufficient
/// test.
fn witnessed(
    cell: (i64, i64, i64),
    near: &[((i64, i64), i64)],
    inner: (i64, i64, i64, i64),
    w: i64,
) -> bool {
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
                    let reach = i128::from(r) + i128::from(w);
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

/// A via of the net: its centre, pad and drill.
#[derive(Clone, Debug)]
pub struct ViaAt {
    pub at: (i64, i64),
    pub via_nm: i64,
    pub drill_nm: i64,
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

/// A hole's clearance radius in nm: its drill's radius plus the largest
/// distance the tolerance moves its centre, `e·√2` (per axis), rounded up.
fn hole_radius(drill_nm: i64, e_nm: i64) -> i64 {
    let r = drill_nm / 2 + drill_nm % 2;
    let mut d = (e_nm as f64 * std::f64::consts::SQRT_2).ceil() as i64;
    while 2 * i128::from(e_nm) * i128::from(e_nm) > i128::from(d) * i128::from(d) {
        d += 1;
    }
    r + d
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
        if !overlaps128(hq, (x0, y0, x0 + side, y0 + side)) {
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

/// [`flow::overlaps`] in `i128` coordinates.
fn overlaps128(poly: &[(i128, i128)], (x0, y0, x1, y1): (i128, i128, i128, i128)) -> bool {
    let bx = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let span = |pts: &[(i128, i128)], ax: (i128, i128)| {
        let v: Vec<i128> = pts.iter().map(|q| ax.0 * q.0 + ax.1 * q.1).collect();
        (*v.iter().min().unwrap(), *v.iter().max().unwrap())
    };
    let mut axes: Vec<(i128, i128)> = vec![(1, 0), (0, 1)];
    for k in 0..poly.len() {
        let (a, b) = (poly[k], poly[(k + 1) % poly.len()]);
        axes.push((b.1 - a.1, -(b.0 - a.0)));
    }
    axes.iter().all(|&ax| {
        let (p0, p1) = span(poly, ax);
        let (q0, q1) = span(&bx, ax);
        p1 > q0 && q1 > p0
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
    let r_d = |drill: i64| Iv::ratio(drill, 2_000_000);
    let beta = Iv::ratio(m.beta_nm, 1_000_000);
    let t_p = Iv::ratio(m.plating_nm, 1_000_000);
    let e_mm = Iv::ratio(e, 1_000_000).hi;
    let h_mm = h as f64 / 1e6;
    for v in &vias {
        let key = (v.via_nm, v.drill_nm);
        if by_geometry.contains_key(&key) {
            continue;
        }
        // the band's outer circle about any centre the tolerance allows, r0
        // + e√2: the region (cells meeting that disc) holds it, and the via
        // pad must too, (R − r0)² ≥ 2e², R − r0 > 0
        let r0 = r_d(v.drill_nm).add(beta).hi;
        let reach = (v.drill_nm / 2 + v.drill_nm % 2) + m.beta_nm + hole_radius(0, e) + 1;
        let region = Region::new(h, reach);
        let room = (v.via_nm as f64 / 2e6) - r0;
        let fits = room > 0.0 && Iv::exact(room).sqr().lo >= Iv::exact(e_mm).sqr().scale(2.0).hi;
        let priced = if fits && region.rays_inside(h, e) {
            let faces: Vec<(f64, (i64, i64), f64)> = region
                .faces
                .iter()
                .map(|f| (f.d0 as f64 * h_mm, f.normal, f.u0 as f64 * h_mm))
                .collect();
            match flow::junction_table(&faces, h_mm, e_mm, OFFSET_BOXES, r_d(v.drill_nm), beta, t_p)
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
                "a {}/{} nm via's junction cannot hold the band at every hole position the ±{e} nm tolerance allows: such vias are left out",
                v.via_nm, v.drill_nm
            ));
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
        .map(|v| (v.at, hole_radius(v.drill_nm, e)))
        .collect();
    for p in &own_drilled {
        let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
        holes.push((c, hole_radius(p.drill_nm.unwrap_or(0), e)));
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
                return Err(named(
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
                shapes.push(Shape::Stadium {
                    a,
                    b,
                    hw: S * w / 2,
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
            let wit = m.opening_nm - ARC_NM;
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
                if hard || !witnessed((x0, y0, h), &near, inner, wit) {
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
    for v in vias {
        let ji = by_geometry[&(v.via_nm, v.drill_nm)].unwrap_or(0);
        let jn = &junctions[ji];
        let c = centre(&v);
        let abs = |q: Cell| (c.0 + q.0, c.1 + q.1);
        let mut joins = Vec::new();
        for (li, layer) in layers.iter().enumerate() {
            let have: BTreeSet<Cell> = layer.cells.iter().copied().collect();
            if layer.plane {
                if !jn
                    .region
                    .cells
                    .iter()
                    .all(|&q| eligible[li].contains(&abs(q)))
                {
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
                    if !hull_in_copper(&flow::convex_hull(&pts), &outer_shapes[li], 8) {
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
        junctions,
        vias: uses,
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
    // each via's centre and, per joined layer, its port edges
    let mut ports: Vec<(Cell, Vec<Vec<usize>>)> = Vec::new();
    for u in &nc.vias {
        let v = &u.via;
        let jn = &nc.junctions[u.junction];
        let hole = Hole {
            r_d: Iv::ratio(v.drill_nm, 2_000_000),
            beta: Iv::ratio(m.beta_nm, 1_000_000),
            t_p: Iv::ratio(m.plating_nm, 1_000_000),
            rho: m.rho,
        };
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
/// [`flow::junction`]'s band, nm: thin bands cost little (measured: 30 µm to
/// 1 µm raises a coefficient by 1–2 %), and 10 µm lets the worst-case hole's
/// band fit a 0.6 mm via pad.
pub const BETA_NM: i64 = 10_000;
/// The fabricator's hole position tolerance, per axis, nm: JLCPCB's
/// capabilities page, "Hole Position Tolerance ±0.075 mm" (read, 1303 S4;
/// conditions.json). Every hole is priced at every position within it
/// (ruling S1).
pub const HOLE_OFFSET_NM: i64 = 75_000;
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
    /// rounded up; `None` when any pin is refused.
    pub drop_bound_microvolts: Option<u64>,
    /// Set when the bound comes within 1.5× of the limit: the pitch set the
    /// margin (ruling S4).
    pub note: Option<String>,
    pub conditions: Vec<String>,
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
    let Some(limits) = &power.drop_limits else {
        return Ok(Vec::new());
    };
    if limits.is_empty() {
        return Ok(Vec::new());
    }
    let st = &spec.board.stackup;
    let need = |v: &Option<aethelgard_ir::board::Sourced>, path: &str| {
        v.as_ref().map(|s| s.value).ok_or_else(|| {
            refusal(
                "SupplyInputUndeclared",
                format!("the supply gate reads `board.stackup.{path}`, and the specification does not declare it: it is never defaulted (RUL-14)"),
            )
        })
    };
    let outer = need(&st.outer_copper_min_um, "outer_copper_min_um")?;
    let plane = need(&st.plane_copper_min_um, "plane_copper_min_um")?;
    let plating = need(&st.plating_min_um, "plating_min_um")?;
    let t_c = need(&st.copper_temperature_max_c, "copper_temperature_max_c")?;
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
        hole_offset_nm: HOLE_OFFSET_NM,
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
    for limit in limits {
        let d = &limit.domain;
        if !power.loads.iter().any(|l| &l.domain == d) {
            return Err(refusal(
                "SupplyLoadUndeclared",
                format!(
                    "the supply gate reads `power.loads`' entry for `{d}`, and there is none: the load's other sinks are declared there, the MCU's maximum is counted from its part row"
                ),
            ));
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
         -> Result<Vec<PinResult>, crate::extract::Refused> {
            let mut terminals = vec![from];
            terminals.extend(sinks.iter().copied());
            let nc = copper(board, &read.routed, net, &terminals, &model).map_err(to_refused)?;
            Ok(sinks
                .iter()
                .map(|p| match bound(&nc, &model, from, p, 400) {
                    Ok(b) => bounded_pin(p, &b),
                    Err(r) => {
                        let (kind, message) = match r {
                            Refusal::Named(k, m) => (k, m),
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
                .collect())
        };
        let vdd = mcu_pads(d);
        let vss = mcu_pads(gnd);
        let pins = run(d, source_pad, &vdd)?;
        // the 2-layer return runs through a zone the emitted file leaves
        // unfilled: refused by name (ruling (iii))
        let gnd_plane = board.stack.planes.iter().any(|(_, n)| n == gnd);
        let returns = if gnd_plane {
            run(gnd, ret_pad, &vss)?
        } else {
            vss.iter()
                .map(|p| PinResult::Refused {
                    pin: p.number.clone(),
                    kind: "GroundReturnUnread",
                    message: format!("{mcu}.{}'s return runs through a GND zone the emitted board leaves unfilled: the gate reads no plane for GND", p.number),
                })
                .collect()
        };
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
        // µΩ × mA = nV; rounded up to µV
        let drop = match (worst(&pins), worst(&returns)) {
            (Some(a), Some(b)) => {
                Some(((u128::from(a) + u128::from(b)) * u128::from(load)).div_ceil(1000) as u64)
            }
            _ => None,
        };
        let limit_uv = u64::from(limit.max_millivolts) * 1000;
        let note = drop.filter(|&v| v * 3 > limit_uv * 2).map(|_| {
            format!("the bound is within 1.5x of the limit: the gate's {} um pitch sets part of this margin, and a finer pitch is the known remedy", PITCH_NM as f64 / 1000.0)
        });
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
            note,
            conditions: conditions(),
        });
    }
    Ok(out)
}

fn bounded_pin(p: &Pad, b: &PinBound) -> PinResult {
    PinResult::Bounded {
        pin: p.number.clone(),
        r_bound_microohms: (b.bound.energy.hi * 1e6).ceil() as u64,
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
fn conditions() -> Vec<String> {
    vec![
        "supply: R is a certified upper bound (Thomson's principle) on a subset of the net's copper: every flux-carrying cell lies wholly inside copper the gate reads; copper left out only raises it".into(),
        format!("supply: every hole anywhere within +-{} mm per axis of its pad (JLCPCB Hole Position Tolerance)", HOLE_OFFSET_NM as f64 / 1e6),
        "supply: each terminal pad an equipotential: the current enters and leaves through the whole pad".into(),
        "supply: the source is where the regulator's current enters the domain (a buck's inductor pad); at DC it is the net's highest potential, so the bound covers the regulator's sense point too".into(),
        "supply: the return is the regulator's own ground pin; other domains' return currents in the ground copper are not bounded".into(),
        "supply: the load's VDD and VSS pins are tied inside it, so the worst supply pin and the worst return pin bound its drop [inferred]".into(),
        "supply: copper resistivity IACS x (1 + 0.00393 x (T - 20)) x 1.181 (IPC-MF-150F 3.8.1.2's foil maximum, applied to plated copper as a stated condition), at the declared copper temperature maximum".into(),
        format!("supply: plane fill at least the outline less {} mm and each knockout's measured {} mm clearance and {} mm min-thickness opening (KiCad 10.0.7, measured)", EDGE_NM as f64 / 1e6, FILL_CLEARANCE_NM as f64 / 1e6, OPENING_NM as f64 / 1e6),
    ]
}
