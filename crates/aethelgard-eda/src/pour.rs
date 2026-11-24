//! The copper a buck's thermal pad joins, measured on the drawn board
//! (`AET-WO-4300k` (a), ruled (ii-a)): a **lower bound** per layer, computed
//! from the board as read, never from the emitter's choices or a declared
//! number.
//!
//! On each layer the pad's heat can reach (the outer layers' thermal pours
//! and every `GND` plane), the board is cut into [`CELL_NM`] cells. A cell
//! counts only when it is wholly clear of:
//! - every other net's copper on that layer (a pad's box, a track's
//!   capsule, a via's disk; on an inner plane, every through hole of another
//!   net) grown by the board's clearance;
//! - every `GND` pad the fill meets by a thermal relief (all but the bucks'
//!   own, joined solid), grown by the relief's gap: its spokes are not
//!   counted;
//! - the board's edge, by [`crate::route::EDGE_NM`];
//!
//! each grown further by half the fill's minimum width
//! ([`crate::route::MIN_THICKNESS_NM`]), so a neck the fill would drop is
//! never crossed. Only the cells connected to the pad count: on `F.Cu` from
//! the pad itself, elsewhere from its thermal vias. A pour two pads share is
//! split by the nearer pad's centre, and a cell as near to both counts for
//! neither, so no copper counts twice (ruled (a)). Every choice errs low.

use std::collections::BTreeMap;

use crate::pcb::Rect;
use crate::route::{self, Board, Routed};

/// The raster's cell, nm.
pub const CELL_NM: i64 = 50_000;

/// One buck's measured copper: each joined layer's area, mm², and the
/// least, which the junction screen reads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joined {
    pub reference: String,
    pub layers: Vec<(String, u64)>,
    pub min_mm2: u64,
}

/// A grown obstacle: a rectangle, a capsule, or a disk.
enum Shape {
    Rect(Rect),
    Capsule((i64, i64), (i64, i64), i64),
    Disk((i64, i64), i64),
}

fn segment_distance_sq(p: (i64, i64), a: (i64, i64), b: (i64, i64)) -> i128 {
    let (px, py) = (i128::from(p.0), i128::from(p.1));
    let (ax, ay) = (i128::from(a.0), i128::from(a.1));
    let (bx, by) = (i128::from(b.0), i128::from(b.1));
    let (dx, dy) = (bx - ax, by - ay);
    let len = dx * dx + dy * dy;
    let t_num = ((px - ax) * dx + (py - ay) * dy).clamp(0, len.max(0));
    // The nearest point a + t·d, t = t_num / len, kept exact in i128.
    if len == 0 {
        return (px - ax).pow(2) + (py - ay).pow(2);
    }
    let qx = ax * len + t_num * dx;
    let qy = ay * len + t_num * dy;
    let ex = px * len - qx;
    let ey = py * len - qy;
    (ex * ex + ey * ey) / (len * len)
}

impl Shape {
    fn bounds(&self) -> Rect {
        match *self {
            Shape::Rect(r) => r,
            Shape::Capsule(a, b, r) => Rect {
                x0: a.0.min(b.0) - r,
                y0: a.1.min(b.1) - r,
                x1: a.0.max(b.0) + r,
                y1: a.1.max(b.1) + r,
            },
            Shape::Disk(c, r) => Rect {
                x0: c.0 - r,
                y0: c.1 - r,
                x1: c.0 + r,
                y1: c.1 + r,
            },
        }
    }

    fn covers(&self, p: (i64, i64)) -> bool {
        match *self {
            Shape::Rect(r) => p.0 >= r.x0 && p.0 <= r.x1 && p.1 >= r.y0 && p.1 <= r.y1,
            Shape::Capsule(a, b, r) => {
                segment_distance_sq(p, a, b) <= i128::from(r) * i128::from(r)
            }
            Shape::Disk(c, r) => {
                let (dx, dy) = (i128::from(p.0 - c.0), i128::from(p.1 - c.1));
                dx * dx + dy * dy <= i128::from(r) * i128::from(r)
            }
        }
    }
}

fn grow(r: Rect, d: i64) -> Rect {
    Rect {
        x0: r.x0 - d,
        y0: r.y0 - d,
        x1: r.x1 + d,
        y1: r.y1 + d,
    }
}

/// The free cells of one layer, as a grid over the outline.
struct Grid {
    x0: i64,
    y0: i64,
    w: usize,
    h: usize,
    free: Vec<bool>,
}

impl Grid {
    fn centre(&self, i: usize, j: usize) -> (i64, i64) {
        (
            self.x0 + CELL_NM * i64::try_from(i).unwrap_or(0) + CELL_NM / 2,
            self.y0 + CELL_NM * i64::try_from(j).unwrap_or(0) + CELL_NM / 2,
        )
    }

    fn cell(&self, p: (i64, i64)) -> Option<(usize, usize)> {
        let i = usize::try_from((p.0 - self.x0).div_euclid(CELL_NM)).ok()?;
        let j = usize::try_from((p.1 - self.y0).div_euclid(CELL_NM)).ok()?;
        (i < self.w && j < self.h).then_some((i, j))
    }

    fn block(&mut self, s: &Shape) {
        let b = s.bounds();
        let (Some((i0, j0)), Some((i1, j1))) = (
            self.cell((b.x0.max(self.x0), b.y0.max(self.y0))),
            self.cell((
                b.x1.min(self.x0 + CELL_NM * i64::try_from(self.w).unwrap_or(0) - 1),
                b.y1.min(self.y0 + CELL_NM * i64::try_from(self.h).unwrap_or(0) - 1),
            )),
        ) else {
            return;
        };
        for j in j0..=j1 {
            for i in i0..=i1 {
                if s.covers(self.centre(i, j)) {
                    self.free[j * self.w + i] = false;
                }
            }
        }
    }
}

/// Each declared buck's joined copper on `board` as `routed` read it.
/// `pads` are the bucks' thermal pads, `(reference, pad number)`; a layer
/// joins when it is an outer layer the stack pours or a plane of the pad's
/// net. Returns one entry per pad, in `pads`' order.
pub fn joined(board: &Board, routed: &Routed, pads: &[(String, String)]) -> Vec<Joined> {
    let stack = &board.stack;
    let half_diagonal = CELL_NM * 3 / 4;
    let neck = route::MIN_THICKNESS_NM / 2;
    let margin = stack.clearance_nm + neck + half_diagonal;
    // Each pad: its centre, its net and its rectangle.
    let found: Vec<Option<((i64, i64), String, Rect)>> = pads
        .iter()
        .map(|(reference, number)| {
            board
                .parts
                .iter()
                .filter(|p| &p.reference == reference)
                .flat_map(|p| p.pads.iter())
                .find(|p| &p.number == number)
                .and_then(|p| {
                    let net = p.net.clone()?;
                    Some((p.rect.centre(), net, p.rect))
                })
        })
        .collect();
    let mut layers: Vec<(String, String)> = stack.pours.clone();
    layers.extend(stack.planes.iter().cloned());
    let o = board.outline;
    let edge = route::EDGE_NM + neck + half_diagonal;
    let w = usize::try_from((o.x1 - o.x0) / CELL_NM).unwrap_or(0);
    let h = usize::try_from((o.y1 - o.y0) / CELL_NM).unwrap_or(0);
    let mut areas: BTreeMap<usize, Vec<(String, u64)>> = BTreeMap::new();
    for (layer, fill_net) in &layers {
        let mut g = Grid {
            x0: o.x0,
            y0: o.y0,
            w,
            h,
            free: vec![true; w * h],
        };
        // The edge.
        for j in 0..h {
            for i in 0..w {
                let c = g.centre(i, j);
                if c.0 - o.x0 < edge || o.x1 - c.0 < edge || c.1 - o.y0 < edge || o.y1 - c.1 < edge
                {
                    g.free[j * w + i] = false;
                }
            }
        }
        let outer = layer == "F.Cu" || layer == "B.Cu";
        for part in &board.parts {
            for k in &part.keepouts {
                if k.layer.name() == layer.as_str() {
                    g.block(&Shape::Rect(grow(k.rect, neck + half_diagonal)));
                }
            }
            for p in &part.pads {
                let on_layer = p.drill_nm.is_some() || layer == "F.Cu";
                if !on_layer {
                    continue;
                }
                let own = p.net.as_deref() == Some(fill_net.as_str());
                let solid = pads
                    .iter()
                    .any(|(r, n)| *r == part.reference && *n == p.number)
                    || p.join == Some(crate::stack::Join::Solid);
                if own && solid {
                    continue;
                }
                if own {
                    // A relief: its gap and spokes are not counted.
                    g.block(&Shape::Rect(grow(
                        p.rect,
                        route::THERMAL_GAP_NM + neck + half_diagonal,
                    )));
                } else if outer || p.drill_nm.is_some() {
                    g.block(&Shape::Rect(grow(p.rect, margin)));
                }
            }
        }
        if outer {
            for s in &routed.segments {
                if s.layer.name() == layer.as_str() && s.net != *fill_net {
                    let r = stack.rule(&s.net).width_nm / 2 + margin;
                    g.block(&Shape::Capsule(s.start, s.end, r));
                }
            }
        }
        for v in &routed.vias {
            if v.net != *fill_net {
                g.block(&Shape::Disk(v.at, stack.rule(&v.net).via_nm / 2 + margin));
            }
        }
        for a in &routed.arrays {
            if a.net != *fill_net {
                for &at in &a.at {
                    g.block(&Shape::Disk(at, a.via_nm / 2 + margin));
                }
            }
        }
        // Each pad's component, seeded at the pad (F.Cu) or its thermal vias.
        for (k, f) in found.iter().enumerate() {
            let Some((centre, net, rect)) = f else {
                continue;
            };
            if net != fill_net {
                continue;
            }
            let mut seeds: Vec<(i64, i64)> = Vec::new();
            if layer == "F.Cu" {
                seeds.push(*centre);
                seeds.extend([(rect.x0, rect.y0), (rect.x1, rect.y1)]);
            }
            for a in routed
                .arrays
                .iter()
                .filter(|a| a.reference == pads[k].0 && a.pad == pads[k].1)
            {
                seeds.extend(a.at.iter().copied());
            }
            let mut seen = vec![false; w * h];
            let mut stack_cells: Vec<usize> = seeds
                .iter()
                .filter_map(|&s| g.cell(s))
                .map(|(i, j)| j * w + i)
                .filter(|&c| g.free[c])
                .collect();
            for &c in &stack_cells {
                seen[c] = true;
            }
            let mut count: u64 = 0;
            while let Some(c) = stack_cells.pop() {
                let (i, j) = (c % w, c / w);
                // Nearer this pad's centre than every other pad's, strictly.
                let p = g.centre(i, j);
                let d = |q: (i64, i64)| {
                    let (dx, dy) = (i128::from(p.0 - q.0), i128::from(p.1 - q.1));
                    dx * dx + dy * dy
                };
                let mine = d(*centre);
                let nearer = found
                    .iter()
                    .enumerate()
                    .filter(|(m, o)| *m != k && o.is_some())
                    .all(|(_, o)| d(o.as_ref().map(|x| x.0).unwrap_or(*centre)) > mine);
                if nearer {
                    count += 1;
                }
                let mut push = |n: usize| {
                    if g.free[n] && !seen[n] {
                        seen[n] = true;
                        stack_cells.push(n);
                    }
                };
                if i > 0 {
                    push(c - 1);
                }
                if i + 1 < w {
                    push(c + 1);
                }
                if j > 0 {
                    push(c - w);
                }
                if j + 1 < h {
                    push(c + w);
                }
            }
            // cells × (0.05 mm)² = mm² / 400, rounded down.
            areas
                .entry(k)
                .or_default()
                .push((layer.clone(), count / 400));
        }
    }
    pads.iter()
        .enumerate()
        .map(|(k, (reference, _))| {
            let layers = areas.remove(&k).unwrap_or_default();
            let min_mm2 = layers.iter().map(|(_, a)| *a).min().unwrap_or(0);
            Joined {
                reference: reference.clone(),
                layers,
                min_mm2,
            }
        })
        .collect()
}
