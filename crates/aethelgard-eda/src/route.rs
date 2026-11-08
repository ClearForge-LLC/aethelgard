//! The router (`AET-WO-1302`): tracks on F.Cu and B.Cu and vias from a
//! deterministic, integer-only grid search, and an unfilled ground zone on
//! B.Cu that the gate's `--refill-zones` fills (`AET-WO-1300` S4.3).
//!
//! - **The grid** is [`PITCH_NM`], which divides the LQFP's 0.5 mm pitch, so
//!   every LQFP pad centre is a node. Moves are axis-aligned, so every track
//!   is a horizontal or vertical segment and every clearance below is an
//!   exact integer distance between boxes.
//! - **Spacing.** A node is open to a net only if a track centred there keeps
//!   [`CLEARANCE_NM`] from all foreign copper (pads with no net included) and
//!   [`EDGE_NM`] from the board edge, plus a margin `EPS_NM` that covers the
//!   chord between two open nodes (asserted below). A via site keeps the same
//!   from both layers' copper, and [`HOLE_GAP_NM`] between holes.
//! - **Escape.** On F.Cu no track crosses a footprint's pad field (the box
//!   around its pads) except inside its own pad, so every pad is left
//!   straight out across the field's edge. At the LQFP, where the 0.2 mm gap
//!   between pads admits no track, that is the only way out.
//! - **Order.** Nets with two or more pads, `unconnected-…` aside, by pad
//!   count ascending, then by name. A multi-pad net grows as a tree from its
//!   first pad (by reference, then pad number), joining the nearest pad next.
//!   Ground is not tracked between pads: each ground pad is joined by F.Cu to
//!   a via, or to ground copper that already has one, and the B.Cu zone
//!   joins the vias.
//! - **Rip-up and retry.** When a connection finds no free path, a soft
//!   search that may cross other nets' copper, at a price, names the nets in
//!   the way. They and the stuck net are taken up; the stuck net is routed
//!   again first, then they are. The nodes contested cost every later search
//!   more. At most [`RIPUPS`] nets are taken up in all; past that, or when
//!   even the soft search finds nothing, the net is named and routing fails
//!   closed.
//! - **Ties** go to the lower state index: never to a hash or an address.
//!
//! Before the copper is returned, [`check`] and [`connectivity`] judge it
//! exactly, without the grid's margins.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use crate::pcb::Rect;
use crate::rules::FLOOR_NM;
use crate::sexpr::{self, Sx};

/// The grid's pitch.
pub const PITCH_NM: i64 = 125_000;
/// Every track's width: the gate's minimum (`rules`).
pub const WIDTH_NM: i64 = FLOOR_NM;
/// Copper to foreign copper: the gate's clearance.
pub const CLEARANCE_NM: i64 = FLOOR_NM;
/// The gate's Default net class via: 0.6 mm, drilled 0.3 mm.
pub const VIA_NM: i64 = 600_000;
pub const DRILL_NM: i64 = 300_000;
/// Copper to the board edge: KiCad 10.0.7's board default under the gate's
/// project (measured: "board setup constraints edge clearance 0.5000 mm").
pub const EDGE_NM: i64 = 500_000;
/// Hole to hole: KiCad's default minimum.
pub const HOLE_GAP_NM: i64 = 250_000;
/// The zone's minimum width, as the zone states it.
pub const MIN_THICKNESS_NM: i64 = 250_000;
/// The net the B.Cu zone carries.
pub const GROUND: &str = crate::schematic::GND;
/// How many nets may be taken up, in all, to make way for others.
pub const RIPUPS: usize = 500;

/// Added to every reach on the grid. Two open nodes a pitch apart are each
/// at least `R + EPS` from an obstacle, so the chord between them stays at
/// least `sqrt((R + EPS)² - (PITCH/2)²)` from it, which is `R` or more when
/// `2·R·EPS ≥ PITCH²/4`. The smallest reach is a track's to a pad.
const EPS_NM: i64 = 10_000;
const _: () = assert!(8 * (CLEARANCE_NM + WIDTH_NM / 2) * EPS_NM >= PITCH_NM * PITCH_NM);
const _: () = assert!(500_000 % PITCH_NM == 0);
const _: () = assert!(PITCH_NM < MIN_THICKNESS_NM);
const _: () = assert!(EDGE_NM >= FLOOR_NM && WIDTH_NM >= FLOOR_NM && CLEARANCE_NM >= FLOOR_NM);

/// The largest grid routed, per layer: 250 mm square at this pitch.
const MAX_NODES: usize = 4_000_000;
/// A search's window around its ends, in pitches; then the whole board.
const WINDOWS: [usize; 2] = [16, 64];

/// A pending escape corridor's depth in pitches (2 mm), and its cost per
/// node per pitch nearer the pad.
const CORRIDOR_DEPTH: i64 = 16;
const CORRIDOR_COST: u32 = 2;
/// Half a corridor's width: a foreign track centred nearer than this to the
/// pad's axis blocks the pad's straight escape.
const CORRIDOR_HALF_NM: i64 = CLEARANCE_NM + WIDTH_NM;
/// How far beyond a pad field no other net's via may stand in a pad's
/// corridor, so that pads side by side stagger their vias outward.
const VIA_ESCAPE_NM: i64 = 1_000_000;

const STEP_F: u32 = 2;
const STEP_B: u32 = 6;
const BEND: u32 = 1;
const VIA_COST: u32 = 24;
/// How many via nodes one connection may ban before it gives up a window.
const BANS: usize = 4;
/// A soft search's price for crossing another net's copper.
const SOFT_COST: u32 = 400;
/// What a contested node costs every later search, each time contested.
const HISTORY_COST: u32 = 40;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Layer {
    F,
    B,
}

impl Layer {
    pub fn name(self) -> &'static str {
        match self {
            Layer::F => "F.Cu",
            Layer::B => "B.Cu",
        }
    }
    fn index(self) -> usize {
        self as usize
    }
    fn other(self) -> Layer {
        match self {
            Layer::F => Layer::B,
            Layer::B => Layer::F,
        }
    }
}

/// A front SMD pad's copper: its box (a superset of a rounded rectangle)
/// and the rounding radius.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pad {
    pub number: String,
    pub net: Option<String>,
    pub rect: Rect,
    pub corner_nm: i64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub reference: String,
    pub pads: Vec<Pad>,
}

/// What the router routes: the outline and every footprint's copper pads.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pub outline: Rect,
    pub parts: Vec<Part>,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Segment {
    pub net: String,
    pub layer: Layer,
    pub start: (i64, i64),
    pub end: (i64, i64),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Via {
    pub net: String,
    pub at: (i64, i64),
}

/// The copper the router adds.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Routed {
    pub segments: Vec<Segment>,
    pub vias: Vec<Via>,
    /// Each zone's net and layer; its outline is the board's.
    pub zones: Vec<(String, Layer)>,
    /// How many nets were taken up and routed again (not read back from a
    /// board).
    pub ripups: usize,
}

impl Routed {
    pub fn length_nm(&self) -> i64 {
        self.segments
            .iter()
            .map(|s| (s.end.0 - s.start.0).abs() + (s.end.1 - s.start.1).abs())
            .sum()
    }
}

/// A net the router could not complete, and why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unroutable {
    pub net: String,
    pub why: String,
}

// ---- geometry ----

fn point(x: i64, y: i64) -> Rect {
    Rect {
        x0: x,
        y0: y,
        x1: x,
        y1: y,
    }
}

fn span(a: (i64, i64), b: (i64, i64)) -> Rect {
    Rect {
        x0: a.0.min(b.0),
        y0: a.1.min(b.1),
        x1: a.0.max(b.0),
        y1: a.1.max(b.1),
    }
}

/// The squared distance between two closed boxes.
fn gap2(a: Rect, b: Rect) -> i64 {
    let dx = (a.x0 - b.x1).max(b.x0 - a.x1).max(0);
    let dy = (a.y0 - b.y1).max(b.y0 - a.y1).max(0);
    dx * dx + dy * dy
}

/// The points within `r` of `rect`: a pad (`r` 0), a track (a segment and
/// half its width) or a via (a point and its radius).
#[derive(Clone, Copy, Debug)]
struct Shape {
    rect: Rect,
    r: i64,
}

impl Shape {
    /// True when the two shapes are at least `gap` apart.
    fn clear(self, o: Shape, gap: i64) -> bool {
        let need = self.r + o.r + gap;
        gap2(self.rect, o.rect) >= need * need
    }
    /// True when `p` is in the shape, edge included.
    fn holds(self, p: (i64, i64)) -> bool {
        gap2(self.rect, point(p.0, p.1)) <= self.r * self.r
    }
}

impl Pad {
    /// The box in which every point is inside the pad's copper, whatever its
    /// rounding: a track that ends here is joined to the pad.
    fn inner(&self) -> Rect {
        let c = self.corner_nm;
        Rect {
            x0: self.rect.x0 + c,
            y0: self.rect.y0 + c,
            x1: self.rect.x1 - c,
            y1: self.rect.y1 - c,
        }
    }
}

// ---- the grid ----

const FREE: u32 = 0;
/// A search state with no predecessor.
const SOURCE: u8 = u8::MAX;
const BLOCKED: u32 = u32::MAX;

#[derive(Clone, Copy)]
struct Grid {
    x0: i64,
    y0: i64,
    nx: usize,
    ny: usize,
}

fn ceil_to(v: i64) -> i64 {
    -((-v).div_euclid(PITCH_NM)) * PITCH_NM
}

impl Grid {
    fn new(outline: Rect) -> Option<Grid> {
        let (x0, y0) = (ceil_to(outline.x0), ceil_to(outline.y0));
        if outline.x1 < x0 || outline.y1 < y0 {
            return None;
        }
        let nx = usize::try_from((outline.x1 - x0) / PITCH_NM + 1).ok()?;
        let ny = usize::try_from((outline.y1 - y0) / PITCH_NM + 1).ok()?;
        (nx.checked_mul(ny)? <= MAX_NODES).then_some(Grid { x0, y0, nx, ny })
    }
    fn len(&self) -> usize {
        self.nx * self.ny
    }
    fn at(&self, n: usize) -> (i64, i64) {
        (
            self.x0 + (n % self.nx) as i64 * PITCH_NM,
            self.y0 + (n / self.nx) as i64 * PITCH_NM,
        )
    }
    /// The node indices whose coordinate along an axis lies in `[lo, hi]`.
    fn range(origin: i64, count: usize, lo: i64, hi: i64) -> std::ops::Range<usize> {
        let first = (ceil_to(lo) - origin).max(0) / PITCH_NM;
        let last = (hi - origin).div_euclid(PITCH_NM);
        let first = usize::try_from(first).unwrap_or(count).min(count);
        let end = usize::try_from(last + 1).unwrap_or(0).min(count);
        first..end.max(first)
    }
    /// Every node in the closed box.
    fn nodes(&self, r: Rect) -> impl Iterator<Item = usize> + '_ {
        let xs = Grid::range(self.x0, self.nx, r.x0, r.x1);
        Grid::range(self.y0, self.ny, r.y0, r.y1)
            .flat_map(move |iy| xs.clone().map(move |ix| iy * self.nx + ix))
    }
    /// The neighbour of `n` in direction `d`: +x, -x, +y, -y.
    fn step(&self, n: usize, d: usize) -> Option<usize> {
        let (ix, iy) = (n % self.nx, n / self.nx);
        match d {
            0 if ix + 1 < self.nx => Some(n + 1),
            1 if ix > 0 => Some(n - 1),
            2 if iy + 1 < self.ny => Some(n + self.nx),
            3 if iy > 0 => Some(n - self.nx),
            _ => None,
        }
    }
}

/// Which nets may put copper at each node: free, one net, or none.
fn claim(map: &mut [u32], grid: &Grid, shape: Shape, reach: i64, owner: u32) {
    let r = shape.r + reach;
    for n in grid.nodes(shape.rect.grow_by(r)) {
        let (x, y) = grid.at(n);
        if gap2(shape.rect, point(x, y)) < r * r {
            map[n] = match map[n] {
                FREE => owner,
                cur if cur == owner => cur,
                _ => BLOCKED,
            };
        }
    }
}

trait Grow {
    fn grow_by(self, d: i64) -> Rect;
}
impl Grow for Rect {
    fn grow_by(self, d: i64) -> Rect {
        Rect {
            x0: self.x0 - d,
            y0: self.y0 - d,
            x1: self.x1 + d,
            y1: self.y1 + d,
        }
    }
}

const TRACK_REACH: i64 = CLEARANCE_NM + WIDTH_NM / 2 + EPS_NM;
const VIA_REACH: i64 = CLEARANCE_NM + VIA_NM / 2 + EPS_NM;
/// Hole to hole, centre to centre, with the margin.
const HOLE_REACH: i64 = DRILL_NM + HOLE_GAP_NM + EPS_NM;

/// A pad of a net, by part and pad index.
type PadRef = (usize, usize);
/// A grid state: (layer, node).
type Node = (Layer, usize);

/// Why a net stopped: nets whose copper is in the way, or nothing to do.
enum Stuck {
    By(BTreeSet<usize>),
    Unroutable(String),
}

struct Router<'a> {
    board: &'a Board,
    grid: Grid,
    /// Per layer, who may centre a track at each node, by pads, pad fields
    /// and the edge alone.
    hard_track: [Vec<u32>; 2],
    hard_via: Vec<u32>,
    /// The same with every routed net's copper.
    track: [Vec<u32>; 2],
    via: Vec<u32>,
    nets: Vec<String>,
    /// Per net, per layer, unit edges `(a, b)` with `a < b`.
    edges: Vec<[BTreeSet<(usize, usize)>; 2]>,
    /// Via nodes and their net.
    vias: BTreeMap<usize, usize>,
    /// The extra cost of a F.Cu node in a pending escape corridor, each
    /// net's share of it, and whether that share is withdrawn.
    pending: Vec<u32>,
    corridors: Vec<Vec<(usize, u32)>>,
    /// The same for a via, whose copper blocks a corridor from further off.
    pending_via: Vec<u32>,
    via_corridors: Vec<Vec<(usize, u32)>>,
    released: Vec<bool>,
    /// Per layer, the cost a node has gathered by being contested.
    history: [Vec<u32>; 2],
    /// Via nodes a connection may not use: each made a via of its own path
    /// too close to another.
    banned: BTreeSet<usize>,
    // The search's scratch, stamped per search.
    stamp: Vec<u16>,
    cost: Vec<u32>,
    prev: Vec<u8>,
    generation: u16,
}

fn owner(net: usize) -> u32 {
    u32::try_from(net + 1).unwrap_or(BLOCKED)
}

fn opens(o: u32, who: u32) -> bool {
    o == FREE || o == who
}

impl<'a> Router<'a> {
    fn new(board: &'a Board, grid: Grid, nets: Vec<String>) -> Router<'a> {
        let n = grid.len();
        let count = nets.len();
        let mut track = [vec![FREE; n], vec![FREE; n]];
        let mut via = vec![FREE; n];
        let mut corridors = vec![Vec::new(); count];
        let mut via_corridors = vec![Vec::new(); count];
        let o = board.outline;
        for node in 0..n {
            let (x, y) = grid.at(node);
            let edge = (x - o.x0).min(o.x1 - x).min(y - o.y0).min(o.y1 - y);
            if edge < EDGE_NM + WIDTH_NM / 2 + EPS_NM {
                track[0][node] = BLOCKED;
                track[1][node] = BLOCKED;
            }
            if edge < EDGE_NM + VIA_NM / 2 + EPS_NM {
                via[node] = BLOCKED;
            }
        }
        for part in &board.parts {
            for pad in &part.pads {
                let who = pad
                    .net
                    .as_ref()
                    .and_then(|net| nets.binary_search(net).ok())
                    .map_or(BLOCKED, owner);
                let shape = Shape {
                    rect: pad.rect,
                    r: 0,
                };
                claim(&mut track[0], &grid, shape, TRACK_REACH, who);
                claim(&mut via, &grid, shape, VIA_REACH, who);
            }
            // The pad field: on F.Cu, only inside a pad; no via near it.
            let Some(field) = part
                .pads
                .iter()
                .map(|p| p.rect)
                .reduce(|a, b| a.union_of(b))
            else {
                continue;
            };
            for node in grid.nodes(field) {
                let (x, y) = grid.at(node);
                if !part.pads.iter().any(|p| p.rect.contains(point(x, y))) {
                    track[0][node] = BLOCKED;
                }
            }
            for node in grid.nodes(field.grow_by(VIA_NM / 2 + CLEARANCE_NM)) {
                via[node] = BLOCKED;
            }
            for pad in &part.pads {
                if let Some(net) = pad.net.as_ref().and_then(|n| nets.binary_search(n).ok()) {
                    // Near the field, no other net's via stands in the
                    // pad's way out.
                    let keep = corridor(&grid, field, pad, CORRIDOR_HALF_NM + VIA_NM / 2, 0);
                    for (node, cost) in keep {
                        let depth = CORRIDOR_DEPTH - i64::from(cost / CORRIDOR_COST);
                        if depth * PITCH_NM < VIA_ESCAPE_NM {
                            via[node] = match via[node] {
                                FREE => owner(net),
                                o if o == owner(net) => o,
                                _ => BLOCKED,
                            };
                        }
                    }
                    corridors[net].extend(corridor(&grid, field, pad, CORRIDOR_HALF_NM, 0));
                    via_corridors[net].extend(corridor(
                        &grid,
                        field,
                        pad,
                        CORRIDOR_HALF_NM + VIA_NM / 2,
                        VIA_NM / 2 + CLEARANCE_NM,
                    ));
                }
            }
        }
        let mut pending = vec![0; n];
        for list in &corridors {
            for &(node, cost) in list {
                pending[node] += cost;
            }
        }
        let mut pending_via = vec![0; n];
        for list in &via_corridors {
            for &(node, cost) in list {
                pending_via[node] += cost;
            }
        }
        Router {
            board,
            grid,
            track: track.clone(),
            via: via.clone(),
            hard_track: track,
            hard_via: via,
            edges: vec![Default::default(); count],
            nets,
            vias: BTreeMap::new(),
            pending,
            corridors,
            pending_via,
            via_corridors,
            released: vec![false; count],
            history: [vec![0; n], vec![0; n]],
            banned: BTreeSet::new(),
            stamp: vec![0; 2 * n * 5],
            cost: vec![0; 2 * n * 5],
            prev: vec![0; 2 * n * 5],
            generation: 0,
        }
    }

    /// The net is being routed: its own corridors cost it nothing.
    fn release(&mut self, net: usize) {
        if !std::mem::replace(&mut self.released[net], true) {
            for &(node, cost) in &self.corridors[net] {
                self.pending[node] -= cost;
            }
            for &(node, cost) in &self.via_corridors[net] {
                self.pending_via[node] -= cost;
            }
        }
    }

    fn unrelease(&mut self, net: usize) {
        if std::mem::replace(&mut self.released[net], false) {
            for &(node, cost) in &self.corridors[net] {
                self.pending[node] += cost;
            }
            for &(node, cost) in &self.via_corridors[net] {
                self.pending_via[node] += cost;
            }
        }
    }

    fn open(&self, layer: Layer, node: usize, who: u32) -> bool {
        opens(self.track[layer.index()][node], who)
    }

    fn via_open(&self, node: usize, net: usize) -> bool {
        opens(self.via[node], owner(net)) || self.vias.get(&node) == Some(&net)
    }

    /// The first pair of `path`'s own new vias closer than the hole gap,
    /// as the indices of the first's lower state and the second's upper
    /// one. The map keeps vias apart from those already laid, not within a
    /// path.
    fn close_vias(&self, path: &[Node]) -> Option<(usize, usize)> {
        let mut laid: Vec<(usize, (i64, i64))> = Vec::new();
        for (i, w) in path.windows(2).enumerate() {
            let ((la, a), (lb, _)) = (w[0], w[1]);
            if la == lb || self.vias.contains_key(&a) {
                continue;
            }
            let (x, y) = self.grid.at(a);
            if let Some(&(j, _)) = laid
                .iter()
                .rev()
                .find(|&&(_, (px, py))| gap2(point(px, py), point(x, y)) < HOLE_REACH * HOLE_REACH)
            {
                return Some((j, i + 1));
            }
            laid.push((i, (x, y)));
        }
        None
    }

    /// Two vias of one path closer than the hole gap cannot straddle
    /// foreign copper, which would hold them 1.2 mm apart; so the hop
    /// between them goes on the layer it left, where that layer is open.
    /// Otherwise the second via's node is banned and `None` returned.
    fn unhop(&mut self, net: usize, path: &mut Vec<Node>) -> Option<()> {
        while let Some((i, j)) = self.close_vias(path) {
            let outer = path[i].0;
            // path[i] and path[j] are the hop's ends on the outer layer;
            // path[i + 1..j] is the hop itself, its ends included.
            let flat: Vec<Node> = path[i + 2..j - 1]
                .iter()
                .map(|&(_, n)| (outer, n))
                .collect();
            if flat.iter().all(|&(l, n)| self.open(l, n, owner(net))) {
                let tail = path.split_off(j);
                path.truncate(i + 1);
                path.extend(flat);
                path.extend(tail);
            } else {
                self.banned.insert(path[j].1);
                return None;
            }
        }
        Some(())
    }

    /// The nodes inside the pad's copper where `net` may end a track, by
    /// pads and the edge alone.
    fn terminals(&self, pad: &Pad, net: usize) -> Vec<usize> {
        self.grid
            .nodes(pad.inner())
            .filter(|&n| opens(self.hard_track[0][n], owner(net)))
            .collect()
    }

    /// The cheapest path from any source to a node `goal` accepts, inside
    /// `window` (grid indices, inclusive), by A* with `h`. A `soft` search
    /// may cross other nets' copper, at a price, to learn which nets are in
    /// the way.
    #[allow(clippy::too_many_arguments)]
    fn search(
        &mut self,
        net: usize,
        sources: &[Node],
        goal: &dyn Fn(Layer, usize) -> bool,
        h: &dyn Fn(usize) -> u32,
        b_moves: bool,
        window: (usize, usize, usize, usize),
        soft: bool,
    ) -> Option<Vec<Node>> {
        let who = owner(net);
        let n = self.grid.len();
        self.generation = self.generation.wrapping_add(1);
        if self.generation == 0 {
            self.stamp.iter_mut().for_each(|s| *s = 0);
            self.generation = 1;
        }
        let g = self.generation;
        // A state is (layer, node, the direction it was entered by; 4 for
        // none). `prev` keeps the predecessor's direction, and 8 if the
        // move was a via.
        let id = |l: Layer, node: usize, d: usize| (l.index() * n + node) * 5 + d;
        let layer_of = |s: usize| if s / 5 >= n { Layer::B } else { Layer::F };
        let mut heap = BinaryHeap::new();
        for &(l, node) in sources {
            let s = id(l, node, 4);
            if self.stamp[s] != g {
                self.stamp[s] = g;
                self.cost[s] = 0;
                self.prev[s] = SOURCE;
                heap.push(Reverse((h(node), s)));
            }
        }
        let (wx0, wy0, wx1, wy1) = window;
        let mut moves: Vec<(usize, usize, u32, u8)> = Vec::with_capacity(5);
        while let Some(Reverse((f, s))) = heap.pop() {
            let (node, d, l) = ((s / 5) % n, s % 5, layer_of(s));
            let c = self.cost[s];
            if f != c + h(node) {
                continue;
            }
            if goal(l, node) {
                let mut path = vec![(l, node)];
                let (mut at, mut al, mut ad) = (node, l, d);
                loop {
                    let p = self.prev[id(al, at, ad)];
                    if p == SOURCE {
                        break;
                    }
                    if p & 8 != 0 {
                        al = al.other();
                    } else {
                        at = self.grid.step(at, ad ^ 1)?;
                    }
                    ad = usize::from(p & 7);
                    path.push((al, at));
                }
                path.reverse();
                return Some(path);
            }
            moves.clear();
            // The price of a node: free, or (soft) another net's.
            let price = |open: bool, hard: bool| match (open, soft && hard) {
                (true, _) => Some(0),
                (false, true) => Some(SOFT_COST),
                (false, false) => None,
            };
            let from = d as u8;
            if l == Layer::F || b_moves {
                for dir in 0..4 {
                    let Some(m) = self.grid.step(node, dir) else {
                        continue;
                    };
                    let (mx, my) = (m % self.grid.nx, m / self.grid.nx);
                    if mx < wx0 || mx > wx1 || my < wy0 || my > wy1 {
                        continue;
                    }
                    let Some(extra) = price(
                        self.open(l, m, who),
                        opens(self.hard_track[l.index()][m], who),
                    ) else {
                        continue;
                    };
                    let step = if l == Layer::F { STEP_F } else { STEP_B };
                    let bend = if d != 4 && d != dir { BEND } else { 0 };
                    let corridor = if l == Layer::F { self.pending[m] } else { 0 };
                    let seen = self.history[l.index()][m];
                    let tc = c + step + bend + corridor + seen + extra;
                    moves.push((id(l, m, dir), m, tc, from));
                }
            }
            let other = l.other();
            let via_price = price(
                self.via_open(node, net) && self.open(other, node, who),
                opens(self.hard_via[node], who),
            );
            if let Some(extra) = via_price.filter(|_| !self.banned.contains(&node)) {
                let tc = c
                    + VIA_COST
                    + self.pending_via[node]
                    + self.history[0][node]
                    + self.history[1][node]
                    + extra;
                moves.push((id(other, node, 4), node, tc, from | 8));
            }
            for &(t, tn, tc, p) in &moves {
                if self.stamp[t] != g || tc < self.cost[t] {
                    self.stamp[t] = g;
                    self.cost[t] = tc;
                    self.prev[t] = p;
                    heap.push(Reverse((tc + h(tn), t)));
                }
            }
        }
        None
    }

    /// The window around `ends`, `margin` pitches wide, or the whole board.
    fn window(&self, ends: &[usize], margin: Option<usize>) -> (usize, usize, usize, usize) {
        let all = (0, 0, self.grid.nx - 1, self.grid.ny - 1);
        let Some(m) = margin else { return all };
        let xs = ends.iter().map(|&e| e % self.grid.nx);
        let ys = ends.iter().map(|&e| e / self.grid.nx);
        let (Some(x0), Some(x1), Some(y0), Some(y1)) =
            (xs.clone().min(), xs.max(), ys.clone().min(), ys.max())
        else {
            return all;
        };
        (
            x0.saturating_sub(m),
            y0.saturating_sub(m),
            (x1 + m).min(all.2),
            (y1 + m).min(all.3),
        )
    }

    /// One connection: searched in growing windows around `ends`; if none is
    /// free, the nets whose copper a soft search had to cross.
    #[allow(clippy::too_many_arguments)]
    fn connect(
        &mut self,
        net: usize,
        sources: &[Node],
        ends: &[usize],
        goal: &dyn Fn(Layer, usize) -> bool,
        h: &dyn Fn(usize) -> u32,
        b_moves: bool,
        what: &str,
    ) -> Result<Vec<Node>, Stuck> {
        // Free paths near the ends first, then who is in the way near them,
        // and only then the whole board.
        let tries = [
            (Some(WINDOWS[0]), false),
            (Some(WINDOWS[1]), false),
            (Some(WINDOWS[1]), true),
            (None, false),
            (None, true),
        ];
        {
            self.banned.clear();
            for (margin, soft) in tries {
                let w = self.window(ends, margin);
                let mut found = self.search(net, sources, goal, h, b_moves, w, soft);
                for ban in 0..=BANS {
                    let Some(mut p) = found.take() else { break };
                    if self.unhop(net, &mut p).is_some() {
                        found = Some(p);
                        break;
                    }
                    if ban < BANS {
                        found = self.search(net, sources, goal, h, b_moves, w, soft);
                    }
                }
                if let Some(p) = found {
                    if !soft {
                        return Ok(p);
                    }
                    let by = self.blockers(net, &p);
                    if by.is_empty() {
                        return Err(Stuck::Unroutable(format!("{what}: no free path")));
                    }
                    return Err(Stuck::By(by));
                }
            }
        }
        Err(Stuck::Unroutable(what.to_owned()))
    }

    /// The nets whose copper stands where a soft `path` went, each step of
    /// it that was not free marked as contested.
    fn blockers(&mut self, net: usize, path: &[Node]) -> BTreeSet<usize> {
        let who = owner(net);
        let mut out = BTreeSet::new();
        for w in path.windows(2) {
            let ((la, a), (lb, b)) = (w[0], w[1]);
            let (at, reach, layers) = if la == lb {
                if self.open(lb, b, who) {
                    continue;
                }
                (b, TRACK_REACH, [lb == Layer::F, lb == Layer::B])
            } else {
                if self.via_open(a, net) && self.open(lb, a, who) {
                    continue;
                }
                (a, VIA_REACH, [true, true])
            };
            let (x, y) = self.grid.at(at);
            let here = point(x, y);
            // Every node where a blocker's track would stand in the way
            // costs more from now on: the contested node's shadow.
            let shadow = reach + WIDTH_NM / 2;
            for m in self.grid.nodes(here.grow_by(shadow)) {
                let (mx, my) = self.grid.at(m);
                if gap2(here, point(mx, my)) < shadow * shadow {
                    for l in (0..2).filter(|&l| layers[l]) {
                        self.history[l][m] += HISTORY_COST;
                    }
                }
            }
            for (other, e) in self.edges.iter().enumerate() {
                if other == net {
                    continue;
                }
                let near = (0..2).filter(|&l| layers[l]).any(|l| {
                    e[l].iter().any(|&(p, q)| {
                        let r = reach + WIDTH_NM / 2;
                        gap2(span(self.grid.at(p), self.grid.at(q)), here) < r * r
                    })
                });
                if near {
                    out.insert(other);
                }
            }
            for (&v, &other) in &self.vias {
                let (vx, vy) = self.grid.at(v);
                let d2 = gap2(point(vx, vy), here);
                let copper = reach + VIA_NM / 2;
                let hole = if la != lb { HOLE_REACH } else { 0 };
                if other != net && (d2 < copper * copper || d2 < hole * hole) {
                    out.insert(other);
                }
            }
        }
        out
    }

    /// Lays `path` down as `net`'s copper.
    fn commit(&mut self, net: usize, path: &[Node]) {
        for w in path.windows(2) {
            let ((la, a), (lb, b)) = (w[0], w[1]);
            if a == b && la == lb {
                continue;
            }
            if la == lb {
                self.edges[net][la.index()].insert((a.min(b), a.max(b)));
                self.claim_edge(net, la, a, b);
            } else if let std::collections::btree_map::Entry::Vacant(e) = self.vias.entry(a) {
                e.insert(net);
                self.claim_via(net, a);
            }
        }
    }

    fn claim_edge(&mut self, net: usize, layer: Layer, a: usize, b: usize) {
        let shape = Shape {
            rect: span(self.grid.at(a), self.grid.at(b)),
            r: WIDTH_NM / 2,
        };
        let who = owner(net);
        claim(
            &mut self.track[layer.index()],
            &self.grid,
            shape,
            TRACK_REACH,
            who,
        );
        claim(&mut self.via, &self.grid, shape, VIA_REACH, who);
    }

    fn claim_via(&mut self, net: usize, node: usize) {
        let (x, y) = self.grid.at(node);
        let shape = Shape {
            rect: point(x, y),
            r: VIA_NM / 2,
        };
        let who = owner(net);
        for l in 0..2 {
            claim(&mut self.track[l], &self.grid, shape, TRACK_REACH, who);
        }
        claim(&mut self.via, &self.grid, shape, VIA_REACH, who);
        let centre = Shape {
            rect: point(x, y),
            r: 0,
        };
        claim(&mut self.via, &self.grid, centre, HOLE_REACH, BLOCKED);
    }

    /// Takes `net`'s copper up and makes its corridors pending again.
    fn rip(&mut self, net: usize) {
        self.edges[net] = Default::default();
        self.vias.retain(|_, n| *n != net);
        self.unrelease(net);
        self.track = self.hard_track.clone();
        self.via = self.hard_via.clone();
        for other in 0..self.edges.len() {
            for l in [Layer::F, Layer::B] {
                let edges: Vec<(usize, usize)> =
                    self.edges[other][l.index()].iter().copied().collect();
                for (a, b) in edges {
                    self.claim_edge(other, l, a, b);
                }
            }
        }
        let vias: Vec<(usize, usize)> = self.vias.iter().map(|(&n, &v)| (n, v)).collect();
        for (node, other) in vias {
            self.claim_via(other, node);
        }
    }

    fn pad(&self, (part, i): PadRef) -> &'a Pad {
        &self.board.parts[part].pads[i]
    }

    fn pad_name(&self, (part, i): PadRef) -> String {
        format!(
            "{} pad {}",
            self.board.parts[part].reference, self.board.parts[part].pads[i].number
        )
    }

    /// A signal net: a tree grown from its first pad.
    fn route_net(&mut self, net: usize, pads: &[PadRef]) -> Result<(), Stuck> {
        let first = pads[0];
        let start = self.terminals(self.pad(first), net);
        if start.is_empty() {
            return Err(Stuck::Unroutable(format!(
                "{} has no node clear to start a track",
                self.pad_name(first)
            )));
        }
        let mut tree: BTreeSet<Node> = start.iter().map(|&n| (Layer::F, n)).collect();
        let mut remaining: Vec<PadRef> = pads[1..].to_vec();
        while !remaining.is_empty() {
            // The pad nearest the tree, then the first in order.
            let nx = self.grid.nx as i64;
            let near = |pad: &Pad| {
                let (cx, cy) = (
                    pad.rect.x0 + (pad.rect.x1 - pad.rect.x0) / 2,
                    pad.rect.y0 + (pad.rect.y1 - pad.rect.y0) / 2,
                );
                tree.iter()
                    .map(|&(_, n)| {
                        let (x, y) = self.grid.at(n);
                        ((x - cx).abs() + (y - cy).abs(), n)
                    })
                    .min()
                    .unwrap_or((i64::MAX, 0))
            };
            let (k, (_, from)) = remaining
                .iter()
                .enumerate()
                .map(|(k, &p)| (k, near(self.pad(p))))
                .min_by_key(|&(k, (d, _))| (d, k))
                .ok_or_else(|| Stuck::Unroutable("no pad left".into()))?;
            let target = remaining.remove(k);
            let name = self.pad_name(target);
            let ends = self.terminals(self.pad(target), net);
            if ends.is_empty() {
                return Err(Stuck::Unroutable(format!(
                    "{name} has no node clear to end a track"
                )));
            }
            let set: BTreeSet<usize> = ends.iter().copied().collect();
            let col = |e: &usize| (e % self.grid.nx) as i64;
            let row = |e: &usize| (e / self.grid.nx) as i64;
            let bx = (
                ends.iter().map(col).min().unwrap_or(0),
                ends.iter().map(col).max().unwrap_or(0),
                ends.iter().map(row).min().unwrap_or(0),
                ends.iter().map(row).max().unwrap_or(0),
            );
            let h = move |node: usize| {
                let (x, y) = ((node as i64) % nx, (node as i64) / nx);
                let dx = (bx.0 - x).max(x - bx.1).max(0);
                let dy = (bx.2 - y).max(y - bx.3).max(0);
                u32::try_from(dx + dy).unwrap_or(u32::MAX / 4) * STEP_F
            };
            let goal = |l: Layer, n: usize| l == Layer::F && set.contains(&n);
            let sources: Vec<Node> = tree.iter().copied().collect();
            let mut window_ends = ends.clone();
            window_ends.push(from);
            let what = format!("cannot reach {name}");
            let path = self.connect(net, &sources, &window_ends, &goal, &h, true, &what)?;
            self.commit(net, &path);
            tree.extend(path.iter().copied());
            for &(_, n) in &path {
                if self.vias.get(&n) == Some(&net) {
                    tree.insert((Layer::F, n));
                    tree.insert((Layer::B, n));
                }
            }
            tree.extend(ends.iter().map(|&n| (Layer::F, n)));
        }
        Ok(())
    }

    /// Ground: each pad joined by F.Cu to a via, or to ground copper that
    /// has one; the zone joins the vias.
    fn route_ground(&mut self, net: usize, pads: &[PadRef]) -> Result<(), Stuck> {
        // A via stands only in the largest piece of the zone as the B.Cu
        // copper already laid leaves it.
        let core = self.zone_core(net);
        let all: Vec<usize> = (0..core.len()).collect();
        let labels = components(&self.grid, &core, &all);
        let mut size: BTreeMap<usize, usize> = BTreeMap::new();
        for l in labels.iter().flatten() {
            *size.entry(*l).or_default() += 1;
        }
        let main = size
            .iter()
            .max_by_key(|&(l, c)| (*c, Reverse(*l)))
            .map(|(l, _)| *l);
        let zone: Vec<bool> = labels.iter().map(|l| l.is_some() && *l == main).collect();
        let mut joined: BTreeSet<usize> = BTreeSet::new();
        for &p in pads {
            let name = self.pad_name(p);
            let ends = self.terminals(self.pad(p), net);
            if ends.is_empty() {
                return Err(Stuck::Unroutable(format!(
                    "{name} has no node clear to start a track"
                )));
            }
            if ends.iter().any(|n| joined.contains(n)) {
                joined.extend(ends);
                continue;
            }
            let own: BTreeSet<usize> = ends.iter().copied().collect();
            let goal = |l: Layer, n: usize| {
                (l == Layer::B && zone[n]) || (joined.contains(&n) && !own.contains(&n))
            };
            let sources: Vec<Node> = ends.iter().map(|&n| (Layer::F, n)).collect();
            let what = format!("cannot take {name} to a via");
            let path = self.connect(net, &sources, &ends, &goal, &|_| 0, false, &what)?;
            self.commit(net, &path);
            joined.extend(path.iter().filter(|(l, _)| *l == Layer::F).map(|&(_, n)| n));
            joined.extend(ends);
        }
        Ok(())
    }

    /// `None` when the zone's fill is one piece holding every ground via;
    /// otherwise the nets whose B.Cu copper fences it apart. Every node on
    /// such a fence costs B.Cu searches more from now on.
    fn zone_blockers(&mut self, ground: usize) -> Option<BTreeSet<usize>> {
        let seeds: Vec<usize> = self
            .vias
            .iter()
            .filter(|&(_, &net)| net == ground)
            .map(|(&n, _)| n)
            .collect();
        let core = self.zone_core(ground);
        let labels = components(&self.grid, &core, &seeds);
        // The piece holding the most ground vias, then the first, is the zone.
        let mut count: BTreeMap<usize, usize> = BTreeMap::new();
        for l in labels.iter().flatten() {
            *count.entry(*l).or_default() += 1;
        }
        let main = count
            .iter()
            .max_by_key(|&(l, c)| (*c, Reverse(*l)))
            .map(|(l, _)| *l);
        if labels.iter().all(|l| l.is_some() && *l == main) {
            return None;
        }
        let mut fence = BTreeSet::new();
        let mut seen = vec![false; self.grid.len()];
        for (k, &seed) in seeds.iter().enumerate() {
            if labels[k].is_some() && labels[k] == main {
                continue;
            }
            // Flood the island; its rim is the fence.
            let mut stack = vec![seed];
            while let Some(n) = stack.pop() {
                if std::mem::replace(&mut seen[n], true) {
                    continue;
                }
                for d in 0..4 {
                    if let Some(m) = self.grid.step(n, d) {
                        if core[m] && core[n] {
                            stack.push(m);
                        } else {
                            fence.insert(m);
                        }
                    }
                }
            }
        }
        let reach = CLEARANCE_NM + MIN_THICKNESS_NM / 2 + EPS_NM + PITCH_NM;
        let mut out = BTreeSet::new();
        let shadow = CLEARANCE_NM + MIN_THICKNESS_NM / 2 + WIDTH_NM / 2;
        for &n in &fence {
            let (x, y) = self.grid.at(n);
            let here = point(x, y);
            for m in self.grid.nodes(here.grow_by(shadow)) {
                let (mx, my) = self.grid.at(m);
                if gap2(here, point(mx, my)) < shadow * shadow {
                    self.history[1][m] += HISTORY_COST;
                }
            }
            for (other, e) in self.edges.iter().enumerate() {
                let r = reach + WIDTH_NM / 2;
                if other != ground
                    && e[1]
                        .iter()
                        .any(|&(p, q)| gap2(span(self.grid.at(p), self.grid.at(q)), here) < r * r)
                {
                    out.insert(other);
                }
            }
            for (&v, &other) in &self.vias {
                let (vx, vy) = self.grid.at(v);
                let r = reach + VIA_NM / 2;
                if other != ground && gap2(point(vx, vy), here) < r * r {
                    out.insert(other);
                }
            }
        }
        Some(out)
    }

    /// The B.Cu nodes the zone's fill is sure to cover: at least the
    /// clearance and half the zone's minimum width from foreign B.Cu copper
    /// and from the edge clearance.
    fn zone_core(&self, ground: usize) -> Vec<bool> {
        zone_core(
            &self.grid,
            self.board.outline,
            self.edges
                .iter()
                .enumerate()
                .filter(|&(net, _)| net != ground)
                .flat_map(|(_, e)| {
                    e[1].iter().map(|&(a, b)| Shape {
                        rect: span(self.grid.at(a), self.grid.at(b)),
                        r: WIDTH_NM / 2,
                    })
                }),
            self.vias
                .iter()
                .filter(|&(_, &net)| net != ground)
                .map(|(&n, _)| {
                    let (x, y) = self.grid.at(n);
                    Shape {
                        rect: point(x, y),
                        r: VIA_NM / 2,
                    }
                }),
        )
    }

    /// The copper as segments: each net's unit edges merged into straight
    /// runs, broken at every junction, bend, via and own pad.
    fn segments(&self) -> Vec<Segment> {
        let mut out = Vec::new();
        for (net, layers) in self.edges.iter().enumerate() {
            let pads: Vec<Rect> = self
                .board
                .parts
                .iter()
                .flat_map(|p| &p.pads)
                .filter(|p| p.net.as_deref() == Some(self.nets[net].as_str()))
                .map(|p| p.rect)
                .collect();
            for (li, edges) in layers.iter().enumerate() {
                let layer = if li == 0 { Layer::F } else { Layer::B };
                let mut degree: BTreeMap<usize, (u8, u8)> = BTreeMap::new();
                for &(a, b) in edges {
                    let horizontal = b == a + 1;
                    for n in [a, b] {
                        let e = degree.entry(n).or_default();
                        if horizontal {
                            e.0 += 1;
                        } else {
                            e.1 += 1;
                        }
                    }
                }
                let breaks = |n: usize| {
                    let (h, v) = degree.get(&n).copied().unwrap_or_default();
                    let (x, y) = self.grid.at(n);
                    h + v != 2
                        || (h > 0 && v > 0)
                        || self.vias.contains_key(&n)
                        || pads.iter().any(|r| r.contains(point(x, y)))
                };
                for &(a, b) in edges {
                    let stride = b - a;
                    // A run starts at `a` when no edge of the same direction
                    // ends there, or `a` breaks.
                    if a >= stride && edges.contains(&(a - stride, a)) && !breaks(a) {
                        continue;
                    }
                    let mut end = b;
                    while !breaks(end) && edges.contains(&(end, end + stride)) {
                        end += stride;
                    }
                    out.push(Segment {
                        net: self.nets[net].clone(),
                        layer,
                        start: self.grid.at(a),
                        end: self.grid.at(end),
                    });
                }
            }
        }
        out.sort();
        out
    }
}

/// A pad's pending escape corridor: when the pad meets exactly one side of
/// its footprint's pad field, the F.Cu nodes straight out from that side,
/// within [`CORRIDOR_HALF_NM`] of the pad's axis, each costing more the
/// nearer it is. A net that must cross a corridor then crosses it far out,
/// leaving the pad's own net room to escape and turn inside it.
fn corridor(grid: &Grid, field: Rect, pad: &Pad, half: i64, beyond: i64) -> Vec<(usize, u32)> {
    let r = pad.rect;
    let sides = [
        r.x0 == field.x0,
        r.x1 == field.x1,
        r.y0 == field.y0,
        r.y1 == field.y1,
    ];
    if sides.iter().filter(|s| **s).count() != 1 {
        return Vec::new();
    }
    let (cx, cy) = (r.x0 + (r.x1 - r.x0) / 2, r.y0 + (r.y1 - r.y0) / 2);
    let reach = CORRIDOR_DEPTH * PITCH_NM + beyond;
    let (h, w) = (half - 1, half - 1);
    let (area, from) = match sides {
        [true, ..] => (
            Rect {
                x0: field.x0 - reach,
                y0: cy - h,
                x1: field.x0 - 1,
                y1: cy + h,
            },
            0,
        ),
        [_, true, ..] => (
            Rect {
                x0: field.x1 + 1,
                y0: cy - h,
                x1: field.x1 + reach,
                y1: cy + h,
            },
            1,
        ),
        [_, _, true, _] => (
            Rect {
                x0: cx - w,
                y0: field.y0 - reach,
                x1: cx + w,
                y1: field.y0 - 1,
            },
            2,
        ),
        _ => (
            Rect {
                x0: cx - w,
                y0: field.y1 + 1,
                x1: cx + w,
                y1: field.y1 + reach,
            },
            3,
        ),
    };
    grid.nodes(area)
        .map(|n| {
            let (x, y) = grid.at(n);
            let depth = match from {
                0 => field.x0 - x,
                1 => x - field.x1,
                2 => field.y0 - y,
                _ => y - field.y1,
            };
            let left = (reach - depth).clamp(0, CORRIDOR_DEPTH * PITCH_NM) / PITCH_NM;
            (n, u32::try_from(left).unwrap_or(0) * CORRIDOR_COST)
        })
        .collect()
}

/// See [`Router::zone_core`].
fn zone_core(
    grid: &Grid,
    outline: Rect,
    tracks: impl Iterator<Item = Shape>,
    vias: impl Iterator<Item = Shape>,
) -> Vec<bool> {
    let reach = CLEARANCE_NM + MIN_THICKNESS_NM / 2 + EPS_NM;
    let mut core = vec![true; grid.len()];
    for (n, c) in core.iter_mut().enumerate() {
        let (x, y) = grid.at(n);
        let edge = (x - outline.x0)
            .min(outline.x1 - x)
            .min(y - outline.y0)
            .min(outline.y1 - y);
        if edge < EDGE_NM + MIN_THICKNESS_NM / 2 + EPS_NM {
            *c = false;
        }
    }
    for shape in tracks.chain(vias) {
        let r = shape.r + reach;
        for n in grid.nodes(shape.rect.grow_by(r)) {
            let (x, y) = grid.at(n);
            if gap2(shape.rect, point(x, y)) < r * r {
                core[n] = false;
            }
        }
    }
    core
}

/// The connected parts of `core` holding each of `seeds`: a label per seed,
/// `None` where the seed is not in the core.
fn components(grid: &Grid, core: &[bool], seeds: &[usize]) -> Vec<Option<usize>> {
    let mut label = vec![usize::MAX; grid.len()];
    let mut out = Vec::new();
    let mut next = 0;
    for &s in seeds {
        if !core[s] {
            out.push(None);
            continue;
        }
        if label[s] == usize::MAX {
            let mut stack = vec![s];
            label[s] = next;
            while let Some(n) = stack.pop() {
                for d in 0..4 {
                    if let Some(m) = grid.step(n, d)
                        && core[m]
                        && label[m] == usize::MAX
                    {
                        label[m] = next;
                        stack.push(m);
                    }
                }
            }
            next += 1;
        }
        out.push(Some(label[s]));
    }
    out
}

trait Union {
    fn union_of(self, o: Rect) -> Rect;
}
impl Union for Rect {
    fn union_of(self, o: Rect) -> Rect {
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
}

/// Each routed net's pads, in order: by reference, then pad number.
fn net_pads(board: &Board) -> BTreeMap<String, Vec<PadRef>> {
    let mut order: Vec<usize> = (0..board.parts.len()).collect();
    order.sort_by(|&a, &b| board.parts[a].reference.cmp(&board.parts[b].reference));
    let mut out: BTreeMap<String, Vec<PadRef>> = BTreeMap::new();
    for part in order {
        let mut pads: Vec<usize> = (0..board.parts[part].pads.len()).collect();
        pads.sort_by(|&a, &b| {
            let n = |i: usize| {
                let s = &board.parts[part].pads[i].number;
                (s.parse::<u64>().unwrap_or(u64::MAX), s.clone())
            };
            n(a).cmp(&n(b))
        });
        for i in pads {
            if let Some(net) = &board.parts[part].pads[i].net {
                out.entry(net.clone()).or_default().push((part, i));
            }
        }
    }
    out
}

/// Routes `board`: every net with two or more pads, `unconnected-…` aside.
pub fn route(board: &Board) -> Result<Routed, Unroutable> {
    let fail = |net: &str, why: String| Unroutable {
        net: net.to_owned(),
        why,
    };
    let grid = Grid::new(board.outline).ok_or_else(|| {
        fail(
            "",
            format!(
                "the board, {} x {} mm, is larger than the router's {MAX_NODES}-node grid",
                (board.outline.x1 - board.outline.x0) / 1_000_000,
                (board.outline.y1 - board.outline.y0) / 1_000_000
            ),
        )
    })?;
    let pads = net_pads(board);
    let nets: Vec<String> = pads.keys().cloned().collect();
    let mut order: Vec<usize> = (0..nets.len())
        .filter(|&i| pads[&nets[i]].len() > 1 && !nets[i].starts_with(crate::parity::UNCONNECTED))
        .collect();
    let ground = nets.binary_search_by(|n| n.as_str().cmp(GROUND)).ok();
    // Ground's stitching is the zone's, laid before any net is tracked.
    order.sort_by_key(|&i| (Some(i) != ground, pads[&nets[i]].len(), nets[i].clone()));
    let mut r = Router::new(board, grid, nets.clone());
    let mut queue: VecDeque<usize> = order.iter().copied().collect();
    let mut ripups = 0;
    let zones = loop {
        while let Some(net) = queue.pop_front() {
            r.release(net);
            loop {
                let p = &pads[&nets[net]];
                let result = if Some(net) == ground {
                    r.route_ground(net, p)
                } else {
                    r.route_net(net, p)
                };
                let by = match result {
                    Ok(()) => break,
                    Err(Stuck::Unroutable(why)) => return Err(fail(&nets[net], why)),
                    Err(Stuck::By(by)) => by,
                };
                ripups += by.len();
                if ripups > RIPUPS {
                    let names: Vec<&str> = by.iter().map(|&b| nets[b].as_str()).collect();
                    return Err(fail(
                        &nets[net],
                        format!("still blocked by {names:?} after {RIPUPS} rip-ups"),
                    ));
                }
                // Take the net and its blockers up; route the net again now,
                // then the blockers, in their order.
                r.rip(net);
                r.release(net);
                queue.retain(|q| !by.contains(q));
                for &b in by.iter().rev() {
                    r.rip(b);
                    queue.push_front(b);
                }
            }
        }
        let Some(g) = ground.filter(|g| r.vias.values().any(|v| v == g)) else {
            break Vec::new();
        };
        let Some(by) = r.zone_blockers(g) else {
            break vec![(nets[g].clone(), Layer::B)];
        };
        if by.is_empty() {
            return Err(fail(
                GROUND,
                "the board edge alone cuts the B.Cu zone apart".into(),
            ));
        }
        // The zone is split: the nets whose B.Cu copper fences an island are
        // taken up, and routed again with the fence's nodes dearer.
        ripups += by.len();
        if ripups > RIPUPS {
            return Err(fail(
                GROUND,
                format!("the B.Cu zone does not join every ground via after {RIPUPS} rip-ups"),
            ));
        }
        // Ground is stitched again after them, into the zone they leave.
        for &b in by.iter().chain(&[g]) {
            r.rip(b);
            queue.push_back(b);
        }
    };
    let routed = Routed {
        segments: r.segments(),
        vias: r
            .vias
            .iter()
            .map(|(&n, &net)| Via {
                net: nets[net].clone(),
                at: grid.at(n),
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        zones,
        ripups,
    };
    check(board, &routed).map_err(|why| fail("", why))?;
    connectivity(board, &routed).map_err(|why| fail("", why))?;
    Ok(routed)
}

// ---- the router's own judgement, exact ----

/// One copper item: its shape, net and layers.
struct Item<'a> {
    what: String,
    net: Option<&'a str>,
    shape: Shape,
    layers: [bool; 2],
}

fn items<'a>(board: &'a Board, routed: &'a Routed) -> Vec<Item<'a>> {
    let mut out = Vec::new();
    for part in &board.parts {
        for pad in &part.pads {
            out.push(Item {
                what: format!("{} pad {}", part.reference, pad.number),
                net: pad.net.as_deref(),
                shape: Shape {
                    rect: pad.rect,
                    r: 0,
                },
                layers: [true, false],
            });
        }
    }
    for s in &routed.segments {
        out.push(Item {
            what: format!("track {:?} {:?}-{:?}", s.layer, s.start, s.end),
            net: Some(&s.net),
            shape: Shape {
                rect: span(s.start, s.end),
                r: WIDTH_NM / 2,
            },
            layers: [s.layer == Layer::F, s.layer == Layer::B],
        });
    }
    for v in &routed.vias {
        out.push(Item {
            what: format!("via {:?}", v.at),
            net: Some(&v.net),
            shape: Shape {
                rect: point(v.at.0, v.at.1),
                r: VIA_NM / 2,
            },
            layers: [true, true],
        });
    }
    out
}

/// Every pair of copper items on a shared layer, of different nets (a pad
/// with no net differs from everything), keeps [`CLEARANCE_NM`]; every
/// track and via keeps [`EDGE_NM`] from the edge; holes keep
/// [`HOLE_GAP_NM`]; tracks run along an axis.
pub fn check(board: &Board, routed: &Routed) -> Result<(), String> {
    let all = items(board, routed);
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let shared = (a.layers[0] && b.layers[0]) || (a.layers[1] && b.layers[1]);
            let same = a.net.is_some() && a.net == b.net;
            if shared && !same && !a.shape.clear(b.shape, CLEARANCE_NM) {
                return Err(format!(
                    "{} ({:?}) is within {CLEARANCE_NM} nm of {} ({:?})",
                    a.what, a.net, b.what, b.net
                ));
            }
        }
    }
    let o = board.outline;
    let inner = Rect {
        x0: o.x0 + EDGE_NM,
        y0: o.y0 + EDGE_NM,
        x1: o.x1 - EDGE_NM,
        y1: o.y1 - EDGE_NM,
    };
    for item in all
        .iter()
        .filter(|i| i.what.starts_with("track") || i.what.starts_with("via"))
    {
        if !inner.contains(item.shape.rect.grow_by(item.shape.r)) {
            return Err(format!("{} is within {EDGE_NM} nm of the edge", item.what));
        }
    }
    for s in &routed.segments {
        if s.start.0 != s.end.0 && s.start.1 != s.end.1 || s.start == s.end {
            return Err(format!(
                "track {:?}-{:?} is not along an axis",
                s.start, s.end
            ));
        }
    }
    for (i, a) in routed.vias.iter().enumerate() {
        for b in &routed.vias[i + 1..] {
            let need = DRILL_NM + HOLE_GAP_NM;
            if gap2(point(a.at.0, a.at.1), point(b.at.0, b.at.1)) < need * need {
                return Err(format!("vias {:?} and {:?} are too close", a.at, b.at));
            }
        }
    }
    Ok(())
}

/// Every routed net's copper is one piece, as KiCad joins it: a track's end
/// inside a pad's copper, a via, or another track of the net; ground vias by
/// the zone, judged by the B.Cu area its fill is sure to cover. Every via has
/// copper on both layers, and the zone's net is ground.
pub fn connectivity(board: &Board, routed: &Routed) -> Result<(), String> {
    let grid = Grid::new(board.outline).ok_or("the board is too large")?;
    let all = items(board, routed);
    let mut parent: Vec<usize> = (0..all.len()).collect();
    fn find(p: &mut [usize], mut i: usize) -> usize {
        while p[i] != i {
            p[i] = p[p[i]];
            i = p[i];
        }
        i
    }
    let ends = |s: &Segment| [s.start, s.end];
    // Per via, whether copper meets it on F.Cu and on B.Cu.
    let mut via_layers: BTreeMap<(i64, i64), [bool; 2]> = BTreeMap::new();
    let first_seg = all.len() - routed.segments.len() - routed.vias.len();
    let first_via = all.len() - routed.vias.len();
    for (si, s) in routed.segments.iter().enumerate() {
        let a = first_seg + si;
        for (j, b) in all.iter().enumerate() {
            if j == a || b.net != Some(s.net.as_str()) || !b.layers[s.layer.index()] {
                continue;
            }
            let joined = if j < first_seg {
                // A pad: the track ends inside its copper.
                let pad = &board.parts.iter().flat_map(|p| &p.pads).nth(j);
                pad.is_some_and(|p| ends(s).iter().any(|&e| p.inner().contains(point(e.0, e.1))))
            } else {
                ends(s).iter().any(|&e| b.shape.holds(e))
                    || (j < first_via && {
                        let o = &routed.segments[j - first_seg];
                        ends(o).iter().any(|&e| all[a].shape.holds(e))
                    })
            };
            if joined {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, j));
                parent[ra] = rb;
                if j >= first_via {
                    via_layers.entry(routed.vias[j - first_via].at).or_default()[s.layer.index()] =
                        true;
                }
            }
        }
    }
    let zone_layer = |net: &str| routed.zones.iter().find(|(n, _)| n == net).map(|(_, l)| *l);
    for (net, layer) in &routed.zones {
        if net != GROUND || *layer != Layer::B {
            return Err(format!("a zone on {net} {}", layer.name()));
        }
    }
    if let Some(Layer::B) = zone_layer(GROUND) {
        let core = zone_core(
            &grid,
            board.outline,
            routed
                .segments
                .iter()
                .filter(|s| s.layer == Layer::B && s.net != GROUND)
                .map(|s| Shape {
                    rect: span(s.start, s.end),
                    r: WIDTH_NM / 2,
                }),
            routed
                .vias
                .iter()
                .filter(|v| v.net != GROUND)
                .map(|v| Shape {
                    rect: point(v.at.0, v.at.1),
                    r: VIA_NM / 2,
                }),
        );
        let ground: Vec<usize> = (0..routed.vias.len())
            .filter(|&i| routed.vias[i].net == GROUND)
            .collect();
        let seeds: Vec<usize> = ground
            .iter()
            .map(|&i| {
                let (x, y) = routed.vias[i].at;
                grid.nodes(point(x, y)).next().unwrap_or(usize::MAX)
            })
            .collect();
        if seeds.contains(&usize::MAX) {
            return Err("a ground via is off the grid".into());
        }
        let labels = components(&grid, &core, &seeds);
        for (k, &i) in ground.iter().enumerate() {
            if labels[k].is_none() || labels[k] != labels[0] {
                return Err(format!(
                    "the zone does not reach via {:?}",
                    routed.vias[i].at
                ));
            }
            via_layers.entry(routed.vias[i].at).or_default()[1] = true;
            let (ra, rb) = (
                find(&mut parent, first_via + i),
                find(&mut parent, first_via + ground[0]),
            );
            parent[ra] = rb;
        }
    }
    for v in &routed.vias {
        if via_layers.get(&v.at) != Some(&[true, true]) {
            return Err(format!("via {:?} is not joined on both layers", v.at));
        }
    }
    let pads = net_pads(board);
    let index: BTreeMap<PadRef, usize> = board
        .parts
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| (0..p.pads.len()).map(move |i| (pi, i)))
        .enumerate()
        .map(|(k, r)| (r, k))
        .collect();
    for (net, list) in &pads {
        if list.len() < 2 || net.starts_with(crate::parity::UNCONNECTED) {
            continue;
        }
        let root = find(&mut parent, index[&list[0]]);
        for p in &list[1..] {
            if find(&mut parent, index[p]) != root {
                let part = &board.parts[p.0];
                return Err(format!(
                    "{net}: {} pad {} is not joined to {} pad {}",
                    part.reference,
                    part.pads[p.1].number,
                    board.parts[list[0].0].reference,
                    board.parts[list[0].0].pads[list[0].1].number
                ));
            }
        }
    }
    Ok(())
}

// ---- reading a board ----

fn nm_at(item: &Sx, i: usize, what: &str) -> Result<i64, String> {
    item.arg(i)
        .and_then(sexpr::nm)
        .ok_or_else(|| format!("{what}: unreadable `{:?}`", item.head()))
}

fn xy_of(item: Option<&Sx>, what: &str) -> Result<(i64, i64), String> {
    let item = item.ok_or_else(|| format!("{what}: missing coordinates"))?;
    Ok((nm_at(item, 1, what)?, nm_at(item, 2, what)?))
}

/// A placed footprint's copper pads, in board coordinates. Only front SMD
/// pads at a right angle, rectangular or rounded, are modelled; anything
/// else is refused, never approximated.
pub fn part_of(fp: &Sx) -> Result<Part, String> {
    let reference = fp.property("Reference").unwrap_or("?").to_owned();
    let at = fp
        .child("at")
        .ok_or_else(|| format!("{reference}: no `at`"))?;
    if at.arg(3).is_some_and(|a| a != "0") {
        return Err(format!("{reference} is rotated"));
    }
    let origin = xy_of(Some(at), &reference)?;
    let mut pads = Vec::new();
    for pad in fp.children("pad") {
        let number = pad.arg(1).unwrap_or("").to_owned();
        let what = format!("{reference} pad {number}");
        let layers: Vec<&str> = pad
            .child("layers")
            .map(|l| l.items()[1..].iter().filter_map(Sx::text).collect())
            .unwrap_or_default();
        let copper: Vec<&str> = layers
            .iter()
            .copied()
            .filter(|l| l.ends_with(".Cu"))
            .collect();
        if copper.is_empty() {
            continue;
        }
        if pad.arg(2) != Some("smd") || copper != ["F.Cu"] {
            return Err(format!("{what} is not a front SMD pad"));
        }
        let pat = pad.child("at");
        let (px, py) = xy_of(pat, &what)?;
        let (mut w, mut h) = xy_of(pad.child("size"), &what)?;
        match pat.and_then(|a| a.arg(3)).unwrap_or("0") {
            "0" | "180" => {}
            "90" | "270" => std::mem::swap(&mut w, &mut h),
            a => return Err(format!("{what} is at angle {a}")),
        }
        let corner = match pad.arg(3) {
            Some("rect") => 0,
            Some("roundrect") => {
                let ratio = pad
                    .child("roundrect_rratio")
                    .and_then(|r| r.arg(1))
                    .and_then(sexpr::nm)
                    .ok_or_else(|| format!("{what}: no `roundrect_rratio`"))?;
                w.min(h) * ratio.clamp(0, 500_000) / 1_000_000
            }
            Some("oval" | "circle") => w.min(h) / 2,
            other => return Err(format!("{what} has shape {other:?}")),
        };
        let (cx, cy) = (origin.0 + px, origin.1 + py);
        pads.push(Pad {
            number,
            net: pad.child("net").and_then(|n| n.arg(1)).map(str::to_owned),
            rect: Rect {
                x0: cx - w / 2,
                y0: cy - h / 2,
                x1: cx + w - w / 2,
                y1: cy + h - h / 2,
            },
            corner_nm: corner,
        });
    }
    Ok(Part { reference, pads })
}

fn layer_of(item: &Sx, what: &str) -> Result<Layer, String> {
    match item.child("layer").and_then(|l| l.arg(1)) {
        Some("F.Cu") => Ok(Layer::F),
        Some("B.Cu") => Ok(Layer::B),
        other => Err(format!("{what} on layer {other:?}")),
    }
}

/// A board as written: its outline and pads, and its copper. Tracks and
/// vias must be the router's own sizes.
pub fn read(board: &Sx) -> Result<(Board, Routed), String> {
    let edge = board
        .children("gr_rect")
        .find(|r| r.child("layer").and_then(|l| l.arg(1)) == Some("Edge.Cuts"))
        .ok_or("no outline")?;
    let (a, b) = (
        xy_of(edge.child("start"), "outline")?,
        xy_of(edge.child("end"), "outline")?,
    );
    let parts = board
        .children("footprint")
        .map(part_of)
        .collect::<Result<Vec<_>, _>>()?;
    let net = |item: &Sx, what: &str| {
        item.child("net")
            .and_then(|n| n.arg(1))
            .map(str::to_owned)
            .ok_or_else(|| format!("{what} has no net"))
    };
    let mut routed = Routed::default();
    for s in board.children("segment") {
        let width = s
            .child("width")
            .map(|w| nm_at(w, 1, "segment"))
            .transpose()?;
        if width != Some(WIDTH_NM) {
            return Err(format!("a segment {width:?} wide"));
        }
        routed.segments.push(Segment {
            net: net(s, "a segment")?,
            layer: layer_of(s, "a segment")?,
            start: xy_of(s.child("start"), "segment")?,
            end: xy_of(s.child("end"), "segment")?,
        });
    }
    for v in board.children("via") {
        let size = v.child("size").map(|w| nm_at(w, 1, "via")).transpose()?;
        let drill = v.child("drill").map(|w| nm_at(w, 1, "via")).transpose()?;
        let layers: Vec<&str> = v
            .child("layers")
            .map(|l| l.items()[1..].iter().filter_map(Sx::text).collect())
            .unwrap_or_default();
        if size != Some(VIA_NM) || drill != Some(DRILL_NM) || layers != ["F.Cu", "B.Cu"] {
            return Err(format!("a via {size:?}/{drill:?} on {layers:?}"));
        }
        routed.vias.push(Via {
            net: net(v, "a via")?,
            at: xy_of(v.child("at"), "via")?,
        });
    }
    for z in board.children("zone") {
        routed
            .zones
            .push((net(z, "a zone")?, layer_of(z, "a zone")?));
    }
    routed.segments.sort();
    routed.vias.sort();
    Ok((
        Board {
            outline: span(a, b),
            parts,
        },
        routed,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x0: i64, y0: i64, x1: i64, y1: i64) -> Rect {
        Rect { x0, y0, x1, y1 }
    }

    fn pad(number: &str, net: Option<&str>, r: Rect) -> Pad {
        Pad {
            number: number.into(),
            net: net.map(str::to_owned),
            rect: r,
            corner_nm: 0,
        }
    }

    const MM: i64 = 1_000_000;

    /// Two resistors whose nets cross (`AET-WO-1300` S4's case): both nets
    /// routed, clear, and joined, one by a via pair.
    #[test]
    fn crossing_nets_are_routed_clear_and_joined() {
        let r = |reference: &str, x: i64, a: &str, b: &str| Part {
            reference: reference.into(),
            pads: vec![
                pad(
                    "1",
                    Some(a),
                    rect(
                        x - 760_000,
                        10 * MM - 320_000,
                        x - 260_000,
                        10 * MM + 320_000,
                    ),
                ),
                pad(
                    "2",
                    Some(b),
                    rect(
                        x + 260_000,
                        10 * MM - 320_000,
                        x + 760_000,
                        10 * MM + 320_000,
                    ),
                ),
            ],
        };
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![r("R1", 5 * MM, "/A", "/B"), r("R2", 15 * MM, "/B", "/A")],
        };
        let routed = route(&board).unwrap();
        assert!(!routed.segments.is_empty());
        check(&board, &routed).unwrap();
        connectivity(&board, &routed).unwrap();
        assert_eq!(routed, route(&board).unwrap(), "deterministic");
    }

    /// The clearance check is exact: a track 1 nm too close to a foreign pad
    /// fails, and at exactly the clearance passes.
    #[test]
    fn the_clearance_check_is_exact() {
        let board = Board {
            outline: rect(0, 0, 10 * MM, 10 * MM),
            parts: vec![Part {
                reference: "R1".into(),
                pads: vec![pad("1", Some("/A"), rect(4 * MM, 4 * MM, 5 * MM, 5 * MM))],
            }],
        };
        let at = |y: i64| Routed {
            segments: vec![Segment {
                net: "/B".into(),
                layer: Layer::F,
                start: (2 * MM, y),
                end: (8 * MM, y),
            }],
            ..Routed::default()
        };
        let y = 5 * MM + CLEARANCE_NM + WIDTH_NM / 2;
        assert!(check(&board, &at(y)).is_ok());
        assert!(check(&board, &at(y - 1)).is_err());
        // A pad with no net is foreign to every net, its own kind included.
        let mut b2 = board.clone();
        b2.parts[0].pads[0].net = None;
        let mut r = at(y - 1);
        r.segments[0].net = "/A".into();
        assert!(check(&b2, &r).is_err());
    }
}
