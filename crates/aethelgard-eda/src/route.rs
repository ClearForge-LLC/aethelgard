//! The router (`AET-WO-1302`): tracks on F.Cu and B.Cu and vias from a
//! deterministic, integer-only grid search. At 2 copper layers ground is an
//! unfilled zone on B.Cu that the gate's `--refill-zones` fills (`AET-WO-1300`
//! S4.3). At 4 (`AET-WO-4100`, S-P-P-S) each inner layer is an unfilled plane
//! of one declared net, ground and one domain: each pad of a plane net is
//! joined by F.Cu to a through via in the plane, as ground's pads are to the
//! zone, and B.Cu carries tracks only. Track width and clearance are the
//! specification's (`board.rules`), carried by [`Board::stack`].
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
use crate::strict::{self, Tree};

/// The grid's pitch.
pub const PITCH_NM: i64 = 125_000;
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
/// The net the B.Cu zone carries at 2 layers, and one plane at 4.
pub const GROUND: &str = crate::schematic::GND;
/// How many nets may be taken up, in all, to make way for others.
pub const RIPUPS: usize = 500;

/// Added to every reach on the grid. Two open nodes a pitch apart are each
/// at least `R + EPS` from an obstacle, so the chord between them stays at
/// least `sqrt((R + EPS)² - (PITCH/2)²)` from it, which is `R` or more when
/// `2·R·EPS ≥ PITCH²/4`. The smallest reach is a track's to a pad.
const EPS_NM: i64 = 10_000;
// At the floor; a declared width or clearance is never below it (`validate`).
const _: () = assert!(8 * (FLOOR_NM + FLOOR_NM / 2) * EPS_NM >= PITCH_NM * PITCH_NM);
const _: () = assert!(500_000 % PITCH_NM == 0);
const _: () = assert!(PITCH_NM < MIN_THICKNESS_NM);
const _: () = assert!(EDGE_NM >= FLOOR_NM);

/// The largest grid routed, per layer: 250 mm square at this pitch.
const MAX_NODES: usize = 4_000_000;
/// A search's window around its ends, in pitches; then the whole board.
const WINDOWS: [usize; 2] = [16, 64];

/// A pending escape corridor's depth in pitches (2 mm), and its cost per
/// node per pitch nearer the pad.
const CORRIDOR_DEPTH: i64 = 16;
const CORRIDOR_COST: u32 = 2;

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

/// What the router routes: the outline, every footprint's copper pads, and
/// the board the specification declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Board {
    pub outline: Rect,
    pub parts: Vec<Part>,
    pub stack: crate::stack::Stack,
}

impl Board {
    fn w(&self) -> i64 {
        self.stack.width_nm
    }
    fn s(&self) -> i64 {
        self.stack.clearance_nm
    }
    /// How far a track's centre must stay from foreign copper on the grid.
    fn track_reach(&self) -> i64 {
        self.s() + self.w() / 2 + EPS_NM
    }
    fn via_reach(&self) -> i64 {
        self.s() + VIA_NM / 2 + EPS_NM
    }
    /// Half a corridor's width: a foreign track centred nearer than this to
    /// the pad's axis blocks the pad's straight escape.
    fn corridor_half(&self) -> i64 {
        self.s() + self.w()
    }
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
    /// Each zone's net and KiCad layer name; its outline is the board's.
    pub zones: Vec<(String, String)>,
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
        gap2(self.rect, Rect::point(p.0, p.1)) <= self.r * self.r
    }
}

impl Pad {
    /// The box in which every point is inside the pad's copper, whatever its
    /// rounding: a track that ends here is joined to the pad.
    fn inner(&self) -> Rect {
        self.rect.grow(-self.corner_nm)
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
    /// The columns and rows `nodes` span: (first column, last, first row,
    /// last).
    fn span(&self, nodes: &[usize]) -> Option<(usize, usize, usize, usize)> {
        let xs = nodes.iter().map(|&e| e % self.nx);
        let ys = nodes.iter().map(|&e| e / self.nx);
        Some((xs.clone().min()?, xs.max()?, ys.clone().min()?, ys.max()?))
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
    for n in grid.nodes(shape.rect.grow(r)) {
        let (x, y) = grid.at(n);
        if gap2(shape.rect, Rect::point(x, y)) < r * r {
            map[n] = match map[n] {
                FREE => owner,
                cur if cur == owner => cur,
                _ => BLOCKED,
            };
        }
    }
}

/// Hole to hole, centre to centre, with the margin.
const HOLE_PITCH: i64 = DRILL_NM + HOLE_GAP_NM;
const HOLE_REACH: i64 = HOLE_PITCH + EPS_NM;

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
            let edge = to_edge(o, (x, y));
            if edge < EDGE_NM + board.w() / 2 + EPS_NM {
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
                claim(&mut track[0], &grid, shape, board.track_reach(), who);
                claim(&mut via, &grid, shape, board.via_reach(), who);
            }
            // The pad field: on F.Cu, only inside a pad; no via near it.
            let Some(field) = part.pads.iter().map(|p| p.rect).reduce(|a, b| a.union(b)) else {
                continue;
            };
            for node in grid.nodes(field) {
                let (x, y) = grid.at(node);
                if !part.pads.iter().any(|p| p.rect.contains(Rect::point(x, y))) {
                    track[0][node] = BLOCKED;
                }
            }
            for node in grid.nodes(field.grow(VIA_NM / 2 + board.s())) {
                via[node] = BLOCKED;
            }
            for pad in &part.pads {
                if let Some(net) = pad.net.as_ref().and_then(|n| nets.binary_search(n).ok()) {
                    corridors[net].extend(corridor(&grid, field, pad, board.corridor_half(), 0));
                    via_corridors[net].extend(corridor(
                        &grid,
                        field,
                        pad,
                        board.corridor_half() + VIA_NM / 2,
                        VIA_NM / 2 + board.s(),
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

    /// Withdraws (`true`: the net is being routed, so its own corridors cost
    /// it nothing) or restores (`false`: taken up) the net's corridors.
    fn release(&mut self, net: usize, on: bool) {
        if std::mem::replace(&mut self.released[net], on) == on {
            return;
        }
        for (list, map) in [
            (&self.corridors[net], &mut self.pending),
            (&self.via_corridors[net], &mut self.pending_via),
        ] {
            for &(node, cost) in list {
                map[node] = if on {
                    map[node] - cost
                } else {
                    map[node] + cost
                };
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
            if let Some(&(j, _)) = laid.iter().rev().find(|&&(_, (px, py))| {
                gap2(Rect::point(px, py), Rect::point(x, y)) < HOLE_REACH * HOLE_REACH
            }) {
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
        let Some((x0, x1, y0, y1)) = self.grid.span(ends) else {
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
                (
                    b,
                    self.board.track_reach(),
                    [lb == Layer::F, lb == Layer::B],
                )
            } else {
                if self.via_open(a, net) && self.open(lb, a, who) {
                    continue;
                }
                (a, self.board.via_reach(), [true, true])
            };
            // Every node where a blocker's track would stand in the way
            // costs more from now on: the contested node's shadow.
            let hole = if la != lb { HOLE_REACH } else { 0 };
            let (x, y) = self.grid.at(at);
            let shadow = reach + self.board.w() / 2;
            out.extend(self.contest(Rect::point(x, y), shadow, reach, layers, net, hole));
        }
        out
    }

    /// Raises the history of every node within `shadow` of `here` on
    /// `layers`, and names the nets but `skip` whose copper on those layers
    /// is within `reach` of it, or whose via hole is within `hole`.
    fn contest(
        &mut self,
        here: Rect,
        shadow: i64,
        reach: i64,
        layers: [bool; 2],
        skip: usize,
        hole: i64,
    ) -> BTreeSet<usize> {
        for m in self.grid.nodes(here.grow(shadow)) {
            let (mx, my) = self.grid.at(m);
            if gap2(here, Rect::point(mx, my)) < shadow * shadow {
                for l in (0..2).filter(|&l| layers[l]) {
                    self.history[l][m] += HISTORY_COST;
                }
            }
        }
        let mut out = BTreeSet::new();
        for (other, e) in self.edges.iter().enumerate() {
            let r = reach + self.board.w() / 2;
            let near = (0..2).filter(|&l| layers[l]).any(|l| {
                e[l].iter()
                    .any(|&(p, q)| gap2(span(self.grid.at(p), self.grid.at(q)), here) < r * r)
            });
            if other != skip && near {
                out.insert(other);
            }
        }
        for (&v, &other) in &self.vias {
            let (vx, vy) = self.grid.at(v);
            let d2 = gap2(Rect::point(vx, vy), here);
            let copper = reach + VIA_NM / 2;
            if other != skip && (d2 < copper * copper || d2 < hole * hole) {
                out.insert(other);
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
            r: self.board.w() / 2,
        };
        let who = owner(net);
        let (track_reach, via_reach) = (self.board.track_reach(), self.board.via_reach());
        claim(
            &mut self.track[layer.index()],
            &self.grid,
            shape,
            track_reach,
            who,
        );
        claim(&mut self.via, &self.grid, shape, via_reach, who);
    }

    fn claim_via(&mut self, net: usize, node: usize) {
        let (x, y) = self.grid.at(node);
        let shape = Shape {
            rect: Rect::point(x, y),
            r: VIA_NM / 2,
        };
        let who = owner(net);
        let (track_reach, via_reach) = (self.board.track_reach(), self.board.via_reach());
        for l in 0..2 {
            claim(&mut self.track[l], &self.grid, shape, track_reach, who);
        }
        claim(&mut self.via, &self.grid, shape, via_reach, who);
        let centre = Shape {
            rect: Rect::point(x, y),
            r: 0,
        };
        claim(&mut self.via, &self.grid, centre, HOLE_REACH, BLOCKED);
    }

    /// Takes `net`'s copper up and makes its corridors pending again.
    fn rip(&mut self, net: usize) {
        self.edges[net] = Default::default();
        self.vias.retain(|_, n| *n != net);
        self.release(net, false);
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
                let (cx, cy) = pad.rect.centre();
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
            let bx = self.grid.span(&ends).unwrap_or_default();
            let bx = (bx.0 as i64, bx.1 as i64, bx.2 as i64, bx.3 as i64);
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

    /// Ground, and at 4 layers each plane net: each pad joined by F.Cu to a
    /// via, or to the net's copper that has one; the zone or plane joins the
    /// vias.
    fn route_ground(&mut self, net: usize, pads: &[PadRef]) -> Result<(), Stuck> {
        // A via stands only in the largest piece of the zone as the B.Cu
        // copper already laid leaves it.
        let core = self.zone_core(net);
        let all: Vec<usize> = (0..core.len()).collect();
        let labels = components(&self.grid, &core, &all);
        let main = main_label(&labels);
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

    /// `None` when the zone's (or plane's) fill is one piece holding every via
    /// of `ground`, the stitched net; otherwise the nets whose copper fences
    /// it apart. Every node on such a fence costs B.Cu searches more from now
    /// on.
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
        let main = main_label(&labels);
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
        let reach = self.board.s() + MIN_THICKNESS_NM / 2 + EPS_NM + PITCH_NM;
        let shadow = self.board.s() + MIN_THICKNESS_NM / 2 + self.board.w() / 2;
        let mut out = BTreeSet::new();
        for &n in &fence {
            let (x, y) = self.grid.at(n);
            let here = Rect::point(x, y);
            out.extend(self.contest(here, shadow, reach, [false, true], ground, 0));
        }
        Some(out)
    }

    /// The nodes the stitched net's fill is sure to cover: at least the
    /// clearance and half the zone's minimum width from foreign copper on its
    /// layer and from the edge clearance. At 2 layers that is the B.Cu zone,
    /// cut by foreign B.Cu tracks and vias; at 4 it is the net's plane, which
    /// carries no track, so only foreign through vias cut it.
    fn zone_core(&self, ground: usize) -> Vec<bool> {
        let planes = self.board.stack.has_planes();
        zone_core(
            &self.grid,
            self.board.outline,
            self.board.s(),
            self.edges
                .iter()
                .enumerate()
                .filter(|&(net, _)| net != ground && !planes)
                .flat_map(|(_, e)| {
                    e[1].iter().map(|&(a, b)| Shape {
                        rect: span(self.grid.at(a), self.grid.at(b)),
                        r: self.board.w() / 2,
                    })
                }),
            self.vias
                .iter()
                .filter(|&(_, &net)| net != ground)
                .map(|(&n, _)| {
                    let (x, y) = self.grid.at(n);
                    Shape {
                        rect: Rect::point(x, y),
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
                        || pads.iter().any(|r| r.contains(Rect::point(x, y)))
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
    let (cx, cy) = r.centre();
    let reach = CORRIDOR_DEPTH * PITCH_NM + beyond;
    let h = half - 1;
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
                x0: cx - h,
                y0: field.y0 - reach,
                x1: cx + h,
                y1: field.y0 - 1,
            },
            2,
        ),
        _ => (
            Rect {
                x0: cx - h,
                y0: field.y1 + 1,
                x1: cx + h,
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

/// How far a point inside `o` is from its nearest side.
fn to_edge(o: Rect, (x, y): (i64, i64)) -> i64 {
    (x - o.x0).min(o.x1 - x).min(y - o.y0).min(o.y1 - y)
}

/// See [`Router::zone_core`]. `clearance` is the declared one.
fn zone_core(
    grid: &Grid,
    outline: Rect,
    clearance: i64,
    tracks: impl Iterator<Item = Shape>,
    vias: impl Iterator<Item = Shape>,
) -> Vec<bool> {
    let reach = clearance + MIN_THICKNESS_NM / 2 + EPS_NM;
    let mut core = vec![true; grid.len()];
    for (n, c) in core.iter_mut().enumerate() {
        let (x, y) = grid.at(n);
        let edge = to_edge(outline, (x, y));
        if edge < EDGE_NM + MIN_THICKNESS_NM / 2 + EPS_NM {
            *c = false;
        }
    }
    for shape in tracks.chain(vias) {
        let r = shape.r + reach;
        for n in grid.nodes(shape.rect.grow(r)) {
            let (x, y) = grid.at(n);
            if gap2(shape.rect, Rect::point(x, y)) < r * r {
                core[n] = false;
            }
        }
    }
    core
}

/// The label most often given, the lowest on a tie.
fn main_label(labels: &[Option<usize>]) -> Option<usize> {
    let mut count: BTreeMap<usize, usize> = BTreeMap::new();
    for l in labels.iter().flatten() {
        *count.entry(*l).or_default() += 1;
    }
    count
        .iter()
        .max_by_key(|&(l, c)| (*c, Reverse(*l)))
        .map(|(l, _)| *l)
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

/// What a stitched net's fill is called in a refusal: `B.Cu zone`, or
/// `In1.Cu plane`.
fn zone_name(stack: &crate::stack::Stack, net: &str) -> String {
    match stack.plane_of(net) {
        Some(layer) => format!("{layer} plane"),
        None => "B.Cu zone".to_owned(),
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
        // belt-and-suspenders: an `unconnected-` net is one pin's alone
        // (`pcb::pin_nets`, and parity refuses two), so the pad count already
        // skips it; this keeps a track from ever joining one.
        .filter(|&i| pads[&nets[i]].len() > 1 && !nets[i].starts_with(crate::parity::UNCONNECTED))
        .collect();
    // The stitched nets: ground at 2 layers (the B.Cu zone); at 4, each plane's
    // net. Their stitching is laid before any net is tracked.
    let stitched: Vec<usize> = if board.stack.has_planes() {
        board
            .stack
            .planes
            .iter()
            .filter_map(|(_, n)| nets.binary_search(n).ok())
            .collect()
    } else {
        nets.binary_search_by(|n| n.as_str().cmp(GROUND))
            .ok()
            .into_iter()
            .collect()
    };
    order.sort_by_key(|&i| {
        (
            !stitched.contains(&i),
            pads[&nets[i]].len(),
            nets[i].clone(),
        )
    });
    let mut r = Router::new(board, grid, nets.clone());
    let mut queue: VecDeque<usize> = order.iter().copied().collect();
    let mut ripups = 0;
    let zones = loop {
        while let Some(net) = queue.pop_front() {
            r.release(net, true);
            loop {
                let p = &pads[&nets[net]];
                let result = if stitched.contains(&net) {
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
                r.release(net, true);
                queue.retain(|q| !by.contains(q));
                for &b in by.iter().rev() {
                    r.rip(b);
                    queue.push_front(b);
                }
            }
        }
        // Each stitched net's fill must be one piece holding its vias.
        let mut split = None;
        for &g in &stitched {
            if r.vias.values().any(|v| *v == g)
                && let Some(by) = r.zone_blockers(g)
            {
                split = Some((g, by));
                break;
            }
        }
        let Some((g, by)) = split else {
            break if board.stack.has_planes() {
                board
                    .stack
                    .planes
                    .iter()
                    .map(|(l, n)| (n.clone(), l.clone()))
                    .collect()
            } else if stitched.iter().any(|g| r.vias.values().any(|v| v == g)) {
                vec![(GROUND.to_owned(), "B.Cu".to_owned())]
            } else {
                Vec::new()
            };
        };
        let fill = zone_name(&board.stack, &nets[g]);
        if by.is_empty() {
            return Err(fail(
                &nets[g],
                format!("the board edge alone cuts the {fill} apart"),
            ));
        }
        // The fill is split: the nets whose copper fences an island are taken
        // up, and routed again with the fence's nodes dearer.
        ripups += by.len();
        if ripups > RIPUPS {
            return Err(fail(
                &nets[g],
                format!(
                    "the {fill} does not join every {} via after {RIPUPS} rip-ups",
                    nets[g]
                ),
            ));
        }
        // The net is stitched again after them, into the fill they leave.
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
    // belt-and-suspenders: the grid's margins already keep the copper clear
    // and joined (no tested board reaches these); the exact judges stand
    // behind them, so a router defect fails closed rather than ships.
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
                r: board.w() / 2,
            },
            layers: [s.layer == Layer::F, s.layer == Layer::B],
        });
    }
    for v in &routed.vias {
        out.push(Item {
            what: format!("via {:?}", v.at),
            net: Some(&v.net),
            shape: Shape {
                rect: Rect::point(v.at.0, v.at.1),
                r: VIA_NM / 2,
            },
            layers: [true, true],
        });
    }
    out
}

/// Every pair of copper items on a shared layer, of different nets (a pad
/// with no net differs from everything), keeps the declared clearance; every
/// track and via keeps [`EDGE_NM`] from the edge; holes keep
/// [`HOLE_GAP_NM`]; tracks run along an axis.
pub fn check(board: &Board, routed: &Routed) -> Result<(), String> {
    let all = items(board, routed);
    for (i, a) in all.iter().enumerate() {
        for b in &all[i + 1..] {
            let shared = (a.layers[0] && b.layers[0]) || (a.layers[1] && b.layers[1]);
            let same = a.net.is_some() && a.net == b.net;
            if shared && !same && !a.shape.clear(b.shape, board.s()) {
                return Err(format!(
                    "{} ({:?}) is within {} nm of {} ({:?})",
                    a.what,
                    a.net,
                    board.s(),
                    b.what,
                    b.net
                ));
            }
        }
    }
    let o = board.outline;
    let inner = o.grow(-EDGE_NM);
    for item in all
        .iter()
        .filter(|i| i.what.starts_with("track") || i.what.starts_with("via"))
    {
        if !inner.contains(item.shape.rect.grow(item.shape.r)) {
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
            let need = HOLE_PITCH;
            if gap2(Rect::point(a.at.0, a.at.1), Rect::point(b.at.0, b.at.1)) < need * need {
                return Err(format!("vias {:?} and {:?} are too close", a.at, b.at));
            }
        }
    }
    Ok(())
}

/// Every routed net's copper is one piece, as KiCad joins it: a track's end
/// inside a pad's copper, a via, or another track of the net; a stitched
/// net's vias by its fill, judged by the area the fill is sure to cover: at 2
/// layers ground's B.Cu zone, cut by foreign B.Cu copper and vias; at 4 each
/// declared plane, cut by foreign vias. The zones are exactly the declared
/// ones: at 2 layers none or ground's on B.Cu, at 4 every plane and nothing
/// else. Every via is joined on at least two of the copper layers it
/// crosses: tracks on F.Cu and B.Cu, or its net's fill.
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
    // Per via, which copper meets it: F.Cu, B.Cu, its net's zone or plane.
    let mut via_layers: BTreeMap<(i64, i64), [bool; 3]> = BTreeMap::new();
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
                pad.is_some_and(|p| {
                    ends(s)
                        .iter()
                        .any(|&e| p.inner().contains(Rect::point(e.0, e.1)))
                })
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

    let stack = &board.stack;
    let declared: Vec<(String, String)> = if stack.has_planes() {
        stack
            .planes
            .iter()
            .map(|(l, n)| (n.clone(), l.clone()))
            .collect()
    } else {
        vec![(GROUND.to_owned(), "B.Cu".to_owned())]
    };
    let mut zones = routed.zones.clone();
    zones.sort();
    zones.dedup();
    if zones.len() != routed.zones.len() || zones.iter().any(|z| !declared.contains(z)) {
        return Err(format!(
            "zones {:?}; the specification declares {declared:?}",
            routed.zones
        ));
    }
    if stack.has_planes() && zones.len() != declared.len() {
        return Err(format!(
            "planes {:?}; the specification declares {declared:?}",
            routed.zones
        ));
    }
    for (net, layer) in &zones {
        let b_cu = layer == "B.Cu";
        let core = zone_core(
            &grid,
            board.outline,
            board.s(),
            routed
                .segments
                .iter()
                .filter(|s| b_cu && s.layer == Layer::B && s.net != *net)
                .map(|s| Shape {
                    rect: span(s.start, s.end),
                    r: board.w() / 2,
                }),
            routed.vias.iter().filter(|v| v.net != *net).map(|v| Shape {
                rect: Rect::point(v.at.0, v.at.1),
                r: VIA_NM / 2,
            }),
        );
        let stitched: Vec<usize> = (0..routed.vias.len())
            .filter(|&i| routed.vias[i].net == *net)
            .collect();
        let seeds: Vec<usize> = stitched
            .iter()
            .map(|&i| {
                let (x, y) = routed.vias[i].at;
                grid.nodes(Rect::point(x, y)).next().unwrap_or(usize::MAX)
            })
            .collect();
        if seeds.contains(&usize::MAX) {
            return Err(format!("a {net} via is off the grid"));
        }
        if !b_cu && stitched.is_empty() {
            return Err(format!(
                "the {layer} plane of {net} reaches no via: no pad draws from it"
            ));
        }
        let labels = components(&grid, &core, &seeds);
        for (k, &i) in stitched.iter().enumerate() {
            if labels[k].is_none() || labels[k] != labels[0] {
                let fill = if b_cu {
                    "zone".to_owned()
                } else {
                    format!("{layer} plane of {net}")
                };
                return Err(format!(
                    "the {fill} does not reach via {:?}",
                    routed.vias[i].at
                ));
            }
            // Ground's zone is B.Cu copper: a B.Cu stub and the zone are one
            // layer, as before planes. A plane is a layer of its own.
            via_layers.entry(routed.vias[i].at).or_default()[if b_cu { 1 } else { 2 }] = true;
            let (ra, rb) = (
                find(&mut parent, first_via + i),
                find(&mut parent, first_via + stitched[0]),
            );
            parent[ra] = rb;
        }
    }
    for v in &routed.vias {
        let met = via_layers
            .get(&v.at)
            .map_or(0, |m| m.iter().filter(|&&b| b).count());
        if met < 2 {
            return Err(format!(
                "via {:?} is not joined on two of the layers it crosses",
                v.at
            ));
        }
    }
    let index: BTreeMap<PadRef, usize> = board
        .parts
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| (0..p.pads.len()).map(move |i| (pi, i)))
        .enumerate()
        .map(|(k, r)| (r, k))
        .collect();
    // The nets the router routes: two or more pads, `unconnected-` aside.
    let pads: BTreeMap<String, Vec<PadRef>> = net_pads(board)
        .into_iter()
        .filter(|(net, list)| list.len() > 1 && !net.starts_with(crate::parity::UNCONNECTED))
        .collect();
    for (net, list) in &pads {
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
    // Every track and via is a routed net's, joined to that net's pads:
    // no copper on a net no two pads carry, and none left floating.
    for (k, item) in all.iter().enumerate().skip(first_seg) {
        let net = item.net.unwrap_or("");
        let list = pads.get(net).ok_or_else(|| {
            format!(
                "{} is on `{net}`, which the router does not route",
                item.what
            )
        })?;
        if find(&mut parent, k) != find(&mut parent, index[&list[0]]) {
            return Err(format!(
                "{} of `{net}` is joined to none of its pads",
                item.what
            ));
        }
    }
    Ok(())
}

// ---- reading a board ----

fn nm_at(item: &strict::Node<'_>, i: usize, what: &str) -> Result<i64, String> {
    item.arg(i)
        .and_then(sexpr::nm)
        .ok_or_else(|| format!("{what}: unreadable `{:?}`", item.head()))
}

fn xy_of(item: Option<strict::Node<'_>>, what: &str) -> Result<(i64, i64), String> {
    let item = item.ok_or_else(|| format!("{what}: missing coordinates"))?;
    Ok((nm_at(&item, 1, what)?, nm_at(&item, 2, what)?))
}

/// The child `head` of `item`, read by [`strict::Node::one`]: two are refused.
fn one<'a>(item: &strict::Node<'a>, head: &str) -> Result<Option<strict::Node<'a>>, String> {
    item.one(head).map_err(|e| e.to_string())
}

/// A placed footprint this engine built (`pcb`), judged by the strict
/// reader, then read by [`part`].
pub fn part_of(fp: &Sx) -> Result<Part, String> {
    let tree = strict::footprint(fp).map_err(|e| e.to_string())?;
    let fp = one(&tree.root(), "footprint")?.ok_or("no footprint")?;
    part(&fp)
}

/// A placed footprint's copper pads, in board coordinates. Only front SMD
/// pads at a right angle, rectangular or rounded, are modelled; anything
/// else is refused, never approximated.
pub fn part(fp: &strict::Node<'_>) -> Result<Part, String> {
    let reference = fp
        .field("Reference")
        .map_err(|e| e.to_string())?
        .unwrap_or("?")
        .to_owned();
    let at = one(fp, "at")?.ok_or_else(|| format!("{reference}: no `at`"))?;
    if at.arg(3).is_some_and(|a| a != "0") {
        return Err(format!("{reference} is rotated"));
    }
    let origin = xy_of(Some(at), &reference)?;
    let mut pads = Vec::new();
    for pad in fp.all("pad") {
        let number = pad.arg(1).unwrap_or("").to_owned();
        let what = format!("{reference} pad {number}");
        let layers: Vec<&str> = one(&pad, "layers")?
            .map(|l| l.texts().collect())
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
        let pat = one(&pad, "at")?;
        let angle = pat.as_ref().and_then(|a| a.arg(3)).unwrap_or("0");
        let (px, py) = xy_of(pat, &what)?;
        let (mut w, mut h) = xy_of(one(&pad, "size")?, &what)?;
        match angle {
            "0" | "180" => {}
            "90" | "270" => std::mem::swap(&mut w, &mut h),
            a => return Err(format!("{what} is at angle {a}")),
        }
        let corner = match pad.arg(3) {
            Some("rect") => 0,
            Some("roundrect") => {
                let ratio = one(&pad, "roundrect_rratio")?
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
            net: one(&pad, "net")?.and_then(|n| n.arg(1)).map(str::to_owned),
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

/// A board that does not agree with the specification's declared board,
/// named: `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees` or
/// `ViaNotThrough`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Disagrees {
    pub kind: &'static str,
    pub message: String,
}

/// Refuses a board whose copper layers, zones or declared thickness are not
/// the specification's (`AET-WO-4100`): the layer table's copper entries
/// must be exactly the declared layers, each with KiCad's id and `signal`;
/// every via the engine's untyped through via (`F.Cu` to `B.Cu`, no `blind`
/// or `micro`: the specification declares through vias only);
/// every zone a declared one (ground's B.Cu zone at 2 layers, the planes at
/// 4, each plane present); and `(general (thickness …))` the declared
/// nominal. The board is judged by the specification, never by its own
/// claims.
pub fn declared(tree: &Tree, stack: &crate::stack::Stack) -> Result<(), Disagrees> {
    let no = |kind: &'static str, message: String| Disagrees { kind, message };
    let board = tree.root();
    let table = one(&board, "layers")
        .map_err(|m| no("LayersDisagree", m))?
        .ok_or_else(|| no("LayersDisagree", "the board has no layer table".into()))?;
    let mut copper = Vec::new();
    for row in table.lists() {
        let name = row.arg(1).unwrap_or("");
        let id = row.head().unwrap_or("");
        // KiCad 10 numbers copper 0, 2, 4 … 62 (even) and every other layer
        // odd; a row is copper by its id whatever its name says, or by name.
        let even = id.parse::<u32>().is_ok_and(|n| n % 2 == 0 && n <= 62);
        if even || name.ends_with(".Cu") {
            copper.push((
                id.to_owned(),
                name.to_owned(),
                row.arg(2).unwrap_or("").to_owned(),
            ));
        }
    }
    let want: Vec<(String, String, String)> = stack
        .copper
        .iter()
        .map(|n| {
            (
                crate::stack::kicad_id(n).unwrap_or("?").to_owned(),
                n.clone(),
                "signal".to_owned(),
            )
        })
        .collect();
    let mut sorted = copper.clone();
    sorted.sort();
    let mut expected = want.clone();
    expected.sort();
    if sorted != expected {
        return Err(no(
            "LayersDisagree",
            format!(
                "the board's copper layers are {copper:?}; the specification declares {want:?}"
            ),
        ));
    }
    for v in board.all("via") {
        let layers: Vec<&str> = one(&v, "layers")
            .map_err(|m| no("ViaNotThrough", m))?
            .map(|l| l.texts().collect())
            .unwrap_or_default();
        if v.arg(1).is_some() || layers != ["F.Cu", "B.Cu"] {
            return Err(no(
                "ViaNotThrough",
                format!(
                    "a via typed {:?} on {layers:?}; the specification declares through vias, F.Cu to B.Cu",
                    v.arg(1)
                ),
            ));
        }
    }
    let mut zones = Vec::new();
    for z in board.all("zone") {
        let net = one(&z, "net")
            .map_err(|m| no("PlanesDisagree", m))?
            .and_then(|n| n.arg(1))
            .unwrap_or("")
            .to_owned();
        let layer = one(&z, "layer")
            .map_err(|m| no("PlanesDisagree", m))?
            .and_then(|l| l.arg(1))
            .unwrap_or("")
            .to_owned();
        zones.push((net, layer));
    }
    let planes: Vec<(String, String)> = stack
        .planes
        .iter()
        .map(|(l, n)| (n.clone(), l.clone()))
        .collect();
    let ok = if stack.has_planes() {
        let mut z = zones.clone();
        z.sort();
        let mut p = planes.clone();
        p.sort();
        z == p
    } else {
        zones.len() <= 1 && zones.iter().all(|z| z.0 == GROUND && z.1 == "B.Cu")
    };
    if !ok {
        return Err(no(
            "PlanesDisagree",
            format!(
                "the board's zones are {zones:?}; the specification declares {planes:?} (and ground's B.Cu zone at 2 layers)"
            ),
        ));
    }
    let thickness = board
        .one("general")
        .and_then(|g| g.map(|g| g.one("thickness")).transpose())
        .map_err(|e| no("ThicknessDisagrees", e.to_string()))?
        .flatten()
        .and_then(|t| t.arg(1))
        .and_then(sexpr::nm);
    if thickness != Some(stack.thickness.nominal_nm) {
        return Err(no(
            "ThicknessDisagrees",
            format!(
                "the board declares thickness {thickness:?} nm; the specification declares {} nm",
                stack.thickness.nominal_nm
            ),
        ));
    }
    Ok(())
}

fn layer_of(item: &strict::Node<'_>, what: &str) -> Result<Layer, String> {
    match one(item, "layer")?.and_then(|l| l.arg(1)) {
        Some("F.Cu") => Ok(Layer::F),
        Some("B.Cu") => Ok(Layer::B),
        other => Err(format!("{what} on layer {other:?}")),
    }
}

/// A board as the strict reader read it: its outline and pads, and its
/// copper, judged against the specification's declared board `stack`.
/// Tracks must be the declared width and on F.Cu or B.Cu; vias the engine's
/// through via; zones on a declared layer (the planes at 4 layers, ground's
/// B.Cu zone at 2); the outline the one `Edge.Cuts` rectangle. Whether the
/// layer table, the zones and the thickness are exactly the declared ones is
/// [`declared`]'s.
pub fn read(tree: &Tree, stack: &crate::stack::Stack) -> Result<(Board, Routed), String> {
    let board = tree.root();
    let mut edges = Vec::new();
    for r in board.all("gr_rect") {
        if one(&r, "layer")?.and_then(|l| l.arg(1)) == Some("Edge.Cuts") {
            edges.push(r);
        }
    }
    let edge = match edges.as_slice() {
        [edge] => edge,
        [] => return Err("no outline".to_owned()),
        _ => return Err(format!("{} outlines", edges.len())),
    };
    let (a, b) = (
        xy_of(one(edge, "start")?, "outline")?,
        xy_of(one(edge, "end")?, "outline")?,
    );
    let parts = board
        .all("footprint")
        .map(|fp| part(&fp))
        .collect::<Result<Vec<_>, _>>()?;
    let net = |item: &strict::Node<'_>, what: &str| {
        one(item, "net")?
            .and_then(|n| n.arg(1))
            .map(str::to_owned)
            .ok_or_else(|| format!("{what} has no net"))
    };
    let mut routed = Routed::default();
    for s in board.all("segment") {
        let width = one(&s, "width")?
            .map(|w| nm_at(&w, 1, "segment"))
            .transpose()?;
        if width != Some(stack.width_nm) {
            return Err(format!("a segment {width:?} wide"));
        }
        routed.segments.push(Segment {
            net: net(&s, "a segment")?,
            layer: layer_of(&s, "a segment")?,
            start: xy_of(one(&s, "start")?, "segment")?,
            end: xy_of(one(&s, "end")?, "segment")?,
        });
    }
    for v in board.all("via") {
        let size = one(&v, "size")?.map(|w| nm_at(&w, 1, "via")).transpose()?;
        let drill = one(&v, "drill")?.map(|w| nm_at(&w, 1, "via")).transpose()?;
        let layers: Vec<&str> = one(&v, "layers")?
            .map(|l| l.texts().collect())
            .unwrap_or_default();
        if v.arg(1).is_some()
            || size != Some(VIA_NM)
            || drill != Some(DRILL_NM)
            || layers != ["F.Cu", "B.Cu"]
        {
            return Err(format!("a via {size:?}/{drill:?} on {layers:?}"));
        }
        routed.vias.push(Via {
            net: net(&v, "a via")?,
            at: xy_of(one(&v, "at")?, "via")?,
        });
    }
    for z in board.all("zone") {
        let layer = one(&z, "layer")?
            .and_then(|l| l.arg(1))
            .map(str::to_owned)
            .ok_or("a zone has no layer")?;
        let allowed = if stack.has_planes() {
            stack.planes.iter().any(|(l, _)| *l == layer)
        } else {
            layer == "B.Cu"
        };
        if !allowed {
            return Err(format!("a zone on layer {layer:?}"));
        }
        routed.zones.push((net(&z, "a zone")?, layer));
    }
    routed.segments.sort();
    routed.vias.sort();
    Ok((
        Board {
            outline: span(a, b),
            parts,
            stack: stack.clone(),
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

    fn read_ref(tree: &Tree) -> Result<(Board, Routed), String> {
        read(tree, &crate::stack::reference())
    }

    /// An 0402-sized part at (x, 10 mm), pad 1 on `a` and pad 2 on `b`.
    fn resistor(reference: &str, x: i64, a: &str, b: &str) -> Part {
        let y = 10 * MM;
        Part {
            reference: reference.into(),
            pads: vec![
                pad(
                    "1",
                    Some(a),
                    rect(x - 760_000, y - 320_000, x - 260_000, y + 320_000),
                ),
                pad(
                    "2",
                    Some(b),
                    rect(x + 260_000, y - 320_000, x + 760_000, y + 320_000),
                ),
            ],
        }
    }

    /// Two resistors whose nets cross (`AET-WO-1300` S4's case): both nets
    /// routed, clear, and joined, one by a via pair.
    #[test]
    fn crossing_nets_are_routed_clear_and_joined() {
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![
                resistor("R1", 5 * MM, "/A", "/B"),
                resistor("R2", 15 * MM, "/B", "/A"),
            ],
            stack: crate::stack::reference(),
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
            stack: crate::stack::reference(),
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
        let y = 5 * MM + crate::rules::FLOOR_NM + crate::rules::FLOOR_NM / 2;
        assert!(check(&board, &at(y)).is_ok());
        assert!(check(&board, &at(y - 1)).is_err());
        // A pad with no net is foreign to every net, its own kind included.
        let mut b2 = board.clone();
        b2.parts[0].pads[0].net = None;
        let mut r = at(y - 1);
        r.segments[0].net = "/A".into();
        assert!(check(&b2, &r).is_err());
    }

    fn seg(net: &str, layer: Layer, a: (i64, i64), b: (i64, i64)) -> Segment {
        Segment {
            net: net.into(),
            layer,
            start: a,
            end: b,
        }
    }

    /// The edge, the hole gap and the axis are judged too.
    #[test]
    fn the_check_judges_the_edge_holes_and_axis() {
        let board = Board {
            outline: rect(0, 0, 10 * MM, 10 * MM),
            parts: vec![],
            stack: crate::stack::reference(),
        };
        let ok = |r: &Routed| check(&board, r);
        let near_edge = EDGE_NM + crate::rules::FLOOR_NM / 2;
        let one = |y| Routed {
            segments: vec![seg("/A", Layer::B, (2 * MM, y), (8 * MM, y))],
            ..Routed::default()
        };
        assert!(ok(&one(near_edge)).is_ok());
        assert!(ok(&one(near_edge - 1)).is_err());
        let vias = |gap: i64| Routed {
            vias: vec![
                Via {
                    net: "/A".into(),
                    at: (5 * MM, 5 * MM),
                },
                Via {
                    net: "/A".into(),
                    at: (5 * MM + gap, 5 * MM),
                },
            ],
            ..Routed::default()
        };
        assert!(ok(&vias(HOLE_PITCH)).is_ok());
        assert!(ok(&vias(HOLE_PITCH - 1)).is_err());
        let slanted = Routed {
            segments: vec![seg("/A", Layer::F, (2 * MM, 2 * MM), (3 * MM, 3 * MM))],
            ..Routed::default()
        };
        assert!(ok(&slanted).is_err());
    }

    /// A ground via the zone cannot reach, a zone taken away, and a via with
    /// copper on one layer only, each leave the copper unjoined.
    #[test]
    fn connectivity_needs_the_zone_and_both_layers_of_a_via() {
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![
                resistor("R1", 5 * MM, GROUND, "/A"),
                resistor("R2", 15 * MM, GROUND, "/A"),
            ],
            stack: crate::stack::reference(),
        };
        let routed = route(&board).unwrap();
        assert_eq!(routed.zones, vec![(GROUND.to_owned(), "B.Cu".to_owned())]);
        connectivity(&board, &routed).unwrap();
        let mut no_zone = routed.clone();
        no_zone.zones.clear();
        assert!(connectivity(&board, &no_zone).is_err());
        // A B.Cu box of another net round the first ground via.
        let v = routed.vias.iter().find(|v| v.net == GROUND).unwrap().at;
        let d = MM;
        let mut fenced = routed.clone();
        let (x0, y0, x1, y1) = (v.0 - d, v.1 - d, v.0 + d, v.1 + d);
        for (a, b) in [
            ((x0, y0), (x1, y0)),
            ((x1, y0), (x1, y1)),
            ((x0, y1), (x1, y1)),
            ((x0, y0), (x0, y1)),
        ] {
            fenced.segments.push(seg("/Z", Layer::B, a, b));
        }
        let e = connectivity(&board, &fenced).unwrap_err();
        assert!(e.contains("the zone does not reach"), "{e}");
        // A signal via with copper on F.Cu only.
        let mut dangling = routed.clone();
        dangling.vias.push(Via {
            net: "/A".into(),
            at: (10 * MM, 3 * MM),
        });
        dangling
            .segments
            .push(seg("/A", Layer::F, (10 * MM, 3 * MM), (10 * MM, 4 * MM)));
        assert!(connectivity(&board, &dangling).is_err());
    }

    /// Adversarial A3: at 2 layers ground's zone is B.Cu copper, so a ground
    /// via met only by a B.Cu stub and the zone is joined on one layer, and
    /// refused, as before planes.
    #[test]
    fn a_ground_via_met_only_on_b_cu_is_not_joined_on_two_layers() {
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![
                resistor("R1", 5 * MM, GROUND, "/A"),
                resistor("R2", 15 * MM, GROUND, "/A"),
            ],
            stack: crate::stack::reference(),
        };
        let mut routed = route(&board).unwrap();
        connectivity(&board, &routed).unwrap();
        let at = (10 * MM, 16 * MM);
        routed.vias.push(Via {
            net: GROUND.into(),
            at,
        });
        routed
            .segments
            .push(seg(GROUND, Layer::B, at, (10 * MM, 17 * MM)));
        let e = connectivity(&board, &routed).unwrap_err();
        assert!(e.contains("not joined on two"), "{e}");
    }

    /// Adversarial A4: a declared plane whose net no pad draws from has no
    /// via; KiCad would report its copper isolated. It is refused here, by
    /// the router's own judge, before any tool runs.
    #[test]
    fn a_plane_no_via_reaches_is_refused() {
        let mut stack = crate::stack::reference();
        stack.copper = vec![
            "F.Cu".into(),
            "In1.Cu".into(),
            "In2.Cu".into(),
            "B.Cu".into(),
        ];
        stack.planes = vec![
            ("In1.Cu".into(), GROUND.into()),
            ("In2.Cu".into(), "VDD_UNUSED".into()),
        ];
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![
                resistor("R1", 5 * MM, GROUND, "/A"),
                resistor("R2", 15 * MM, GROUND, "/A"),
            ],
            stack,
        };
        let e = route(&board).unwrap_err();
        assert!(e.why.contains("reaches no via"), "{e:?}");
    }

    /// What the router does not model is refused when read, never guessed.
    #[test]
    fn reading_refuses_what_the_router_does_not_model() {
        let fp = |at: &str, pad: &str| {
            crate::sexpr::parse(&format!(
                "(footprint \"x\" (at {at}) (property \"Reference\" \"R1\") {pad})"
            ))
            .unwrap()
        };
        let smd = "(pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\"))";
        assert!(part_of(&fp("1 2", smd)).is_ok());
        for (at, pad, want) in [
            ("1 2 90", smd, "rotated"),
            (
                "1 2",
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers \"*.Cu\"))",
                "front SMD",
            ),
            (
                "1 2",
                "(pad \"1\" smd rect (at 0 0 45) (size 1 1) (layers \"F.Cu\"))",
                "angle 45",
            ),
            (
                "1 2",
                "(pad \"1\" smd custom (at 0 0) (size 1 1) (layers \"F.Cu\"))",
                "shape",
            ),
            (
                "1 2",
                "(pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"B.Cu\"))",
                "front SMD",
            ),
        ] {
            let e = part_of(&fp(at, pad)).unwrap_err();
            assert!(e.contains(want), "{pad}: {e}");
        }
        let board = |copper: &str| {
            strict::board(&format!(
                "(kicad_pcb (gr_rect (start 0 0) (end 10 10) (layer \"Edge.Cuts\")) {copper})"
            ))
            .unwrap()
        };
        assert!(
            read_ref(&board(
                "(segment (start 1 1) (end 2 1) (width 0.2) (layer \"F.Cu\") (net \"/A\"))"
            ))
            .is_ok()
        );
        // A second outline is refused, not resolved to the first.
        assert!(
            read_ref(&board(
                "(gr_rect (start 0 0) (end 50 50) (layer \"Edge.Cuts\"))"
            ))
            .unwrap_err()
            .contains("2 outlines")
        );
        for copper in [
            "(segment (start 1 1) (end 2 1) (width 0.25) (layer \"F.Cu\") (net \"/A\"))",
            "(segment (start 1 1) (end 2 1) (width 0.2) (layer \"In1.Cu\") (net \"/A\"))",
            "(via (at 1 1) (size 0.5) (drill 0.3) (layers \"F.Cu\" \"B.Cu\") (net \"/A\"))",
            "(via (at 1 1) (size 0.6) (drill 0.3) (layers \"F.Cu\") (net \"/A\"))",
            "(segment (start 1 1) (end 2 1) (width 0.2) (layer \"F.Cu\"))",
        ] {
            assert!(read_ref(&board(copper)).is_err(), "{copper}");
        }
    }

    /// A board past the grid's cap is refused before anything is allocated.
    #[test]
    fn a_board_too_large_is_refused() {
        let board = Board {
            outline: rect(0, 0, 300 * MM, 300 * MM),
            parts: vec![],
            stack: crate::stack::reference(),
        };
        let e = route(&board).unwrap_err();
        assert!(e.why.contains("larger than"), "{e:?}");
    }

    /// A zone of any net but ground, or on F.Cu, is refused.
    #[test]
    fn only_a_ground_zone_on_b_cu_is_judged_joined() {
        let board = Board {
            outline: rect(0, 0, 10 * MM, 10 * MM),
            parts: vec![],
            stack: crate::stack::reference(),
        };
        for zone in [
            ("VDD".to_owned(), "B.Cu".to_owned()),
            (GROUND.to_owned(), "F.Cu".to_owned()),
        ] {
            let r = Routed {
                zones: vec![zone.clone()],
                ..Routed::default()
            };
            assert!(connectivity(&board, &r).is_err(), "{zone:?}");
        }
    }
}
