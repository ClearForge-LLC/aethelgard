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
//!   between pads admits no track, that is the only way out, except inward:
//!   a dense part's ring interior ([`ring_interior`], `AET-WO-1307` K1) is
//!   open to every net's F.Cu tracks and through vias.
//! - **Courtyards.** No via's copper enters a part's courtyard, except a via
//!   inside that part's own ring interior (K1); [`check`] refuses one.
//! - **Order** (`AET-WO-1307` order B): a fact about the board, not its
//!   names. Nets with two or more pads, `unconnected-…` aside, stitched nets
//!   first, then by pad count ascending, then by their pads' centres; a name
//!   decides only between two nets whose pads coincide, which [`check`]
//!   refuses. The nets are indexed in that order, so every index-ordered
//!   choice (the queue, the blockers taken up, ties) follows it. A net's pads
//!   are ordered by centre (y, then x), and a multi-pad net grows as a tree
//!   from its first, joining the nearest pad next. Ground is not tracked
//!   between pads: each ground pad is joined by F.Cu to a via, or to ground
//!   copper that already has one, and the B.Cu zone joins the vias.
//! - **Rip-up and retry.** When a connection finds no free path, a soft
//!   search that may cross other nets' copper, at a price, names the nets in
//!   the way. They and the stuck net are taken up; the stuck net is routed
//!   again first, then they are. The nodes contested cost every later search
//!   more. At most [`RIPUPS`] nets are taken up in all; past that, or when
//!   even the soft search finds nothing, the net is named and routing fails
//!   closed.
//! - **Ties** go to the lower state index: never to a hash or an address.
//! - **Classes** (`AET-WO-1308`): each net is laid by its rule
//!   ([`crate::stack::Rule`]): its class's width and via, its pair's width,
//!   or the board's. Each distinct rule has its own occupancy maps, every
//!   reach computed at that rule's width and via against each piece of
//!   copper's own, so a board that declares no class has one rule and is
//!   routed exactly as before. Clearance is the declared one for every class
//!   (FEEDBACK F2).
//! - **Pairs** (`AET-WO-1308` §1.3): each declared pair is laid first and
//!   whole by [`Router::route_pair`], on F.Cu with no via, and its copper is
//!   fixed; [`pairs`] judges it from the copper alone.
//! - **Rotation** (`AET-WO-1308` §1.5): a part at a right angle is read by
//!   [`part`] through [`rotate`], as KiCad places it (measured).
//!
//! Before the copper is returned, [`check`] and [`connectivity`] judge it
//! exactly, without the grid's margins.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, VecDeque};

use crate::pcb::Rect;
use crate::rules::FLOOR_NM;
use crate::sexpr::Sx;
use crate::stack::Rule;
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
pub const HOLE_GAP_NM: i64 = aethelgard_ir::thermal::HOLE_GAP_UM as i64 * 1000;
/// The zone's minimum width, as the zone states it.
pub const MIN_THICKNESS_NM: i64 = 250_000;
/// A relieved pad's gap to the fill and its spokes' width, as the zone
/// states them (`thermal_gap`, `thermal_bridge_width`).
pub const THERMAL_GAP_NM: i64 = 500_000;
pub const THERMAL_BRIDGE_NM: i64 = 500_000;
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

/// A pad's copper: its box (a superset of a rounded rectangle) and the
/// rounding radius. A front SMD pad has no drill; a through-hole pad
/// (`AET-WO-1309`) has the same copper on every copper layer, around a round
/// plated hole at its centre.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pad {
    pub number: String,
    pub net: Option<String>,
    pub rect: Rect,
    pub corner_nm: i64,
    pub drill_nm: Option<i64>,
    /// A through-hole pad's own `zone_connect`: how KiCad's fill meets it.
    /// The engine writes the declared join there, because KiCad 10.0.7's
    /// refill takes it from the pad and not from a custom rule (measured,
    /// FEEDBACK B2); `check` requires exactly the declared one.
    pub join: Option<crate::stack::Join>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub reference: String,
    /// The footprint's rotation: 0, 90, 180 or 270 ([`rotate`]).
    pub rotation: u16,
    pub pads: Vec<Pad>,
    /// The front courtyard's outline, as straight edges in board
    /// coordinates; empty for a footprint with none.
    pub courtyard: Vec<Edge>,
}

/// A straight edge of an outline, end to end.
pub type Edge = ((i64, i64), (i64, i64));

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
    /// How far the centre of a track laid by `rule` must stay from foreign
    /// copper on the grid.
    fn track_reach(&self, rule: Rule) -> i64 {
        self.s() + rule.width_nm / 2 + EPS_NM
    }
    fn via_reach(&self, rule: Rule) -> i64 {
        self.s() + rule.via_nm / 2 + EPS_NM
    }
    /// Half a corridor's width: a foreign track centred nearer than this to
    /// the pad's axis blocks the pad's straight escape.
    fn corridor_half(&self, rule: Rule) -> i64 {
        self.s() + rule.width_nm
    }
    /// The rule `net`'s copper is laid and judged by (`AET-WO-1308`).
    fn rule(&self, net: &str) -> Rule {
        self.stack.rule(net)
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
    /// The thermal-via arrays under declared pads (`AET-WO-1309` §1.2).
    pub arrays: Vec<Array>,
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

/// A thermal-via array: the pad it is under, its net and its vias' centres,
/// sorted. Each via is drilled and sized as the pad's declared rule says.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Array {
    pub reference: String,
    pub pad: String,
    pub net: String,
    pub via_nm: i64,
    pub drill_nm: i64,
    pub at: Vec<(i64, i64)>,
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

/// Hole to hole, centre to centre, for holes drilled `a` and `b` nm.
fn hole_pitch(a: i64, b: i64) -> i64 {
    a / 2 + b / 2 + HOLE_GAP_NM
}
/// The same with the grid's margin.
fn hole_reach(a: Rule, b: Rule) -> i64 {
    hole_pitch(a.drill_nm, b.drill_nm) + EPS_NM
}

/// A pad of a net, by part and pad index.
type PadRef = (usize, usize);
/// A grid state: (layer, node).
type Node = (Layer, usize);

/// A fixed piece of a filled net's copper that the fill must reach
/// (`AET-WO-1309`): a through-hole pad, by its declared join, or a thermal
/// array's via.
#[derive(Clone, Copy, Debug)]
enum Anchor {
    Pad(PadRef, crate::stack::Join),
    Via((i64, i64)),
}

/// Why a net stopped: nets whose copper is in the way, or nothing to do.
enum Stuck {
    By(BTreeSet<usize>),
    Unroutable(String),
}

struct Router<'a> {
    board: &'a Board,
    grid: Grid,
    /// The distinct rules nets are laid by, the board's own first, and each
    /// net's index into them. A board that declares no class has one.
    kinds: Vec<Rule>,
    kind: Vec<usize>,
    /// Per rule and layer, who may centre a track laid by that rule at each
    /// node, by pads, pad fields and the edge alone.
    hard_track: Vec<[Vec<u32>; 2]>,
    hard_via: Vec<Vec<u32>>,
    /// The same with every routed net's copper.
    track: Vec<[Vec<u32>; 2]>,
    via: Vec<Vec<u32>>,
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
    /// Whether searches may not change layer: a pair's breakouts (§1.3).
    no_vias: bool,
    /// Fixed copper on every layer, with its net: through-hole pads and
    /// thermal-array vias. It cuts every other net's fill.
    cuts: Vec<(Option<usize>, Shape)>,
    /// Per net, the fixed copper its fill must reach.
    anchors: Vec<Vec<Anchor>>,
    /// Per net, whether its copper is a coupled pair's, which no other net
    /// takes up: a pair is laid first and whole.
    fixed: Vec<bool>,
    // The search's scratch, stamped per search.
    stamp: Vec<u16>,
    cost: Vec<u32>,
    prev: Vec<u8>,
    generation: u16,
}

// ---- through-hole joins and thermal arrays (`AET-WO-1309`) ----

/// Whether `net` is filled: ground's B.Cu zone at 2 layers, a plane at 4.
fn filled(stack: &crate::stack::Stack, net: &str) -> bool {
    if stack.has_planes() {
        stack.plane_of(net).is_some()
    } else {
        net == GROUND
    }
}

/// How `part`'s through-hole pads join a fill of their own net, as the
/// specification declares; a pad of a filled net with none is refused.
/// Each through-hole pad of a part with a declared join carries it as its
/// own `zone_connect`, and no other pad carries one; a pad of a filled net
/// with none declared is refused (`INV-06`).
fn join_of(board: &Board, part: &Part, pad: &Pad) -> Result<Option<crate::stack::Join>, String> {
    if pad.drill_nm.is_none() {
        return Ok(None);
    }
    let what = format!("{} pad {}", part.reference, pad.number);
    let declared = board
        .stack
        .joins
        .iter()
        .find(|(r, _)| *r == part.reference)
        .map(|&(_, j)| j);
    if pad.join != declared {
        return Err(format!(
            "{what} joins a fill {:?} by its own `zone_connect`; the specification declares {declared:?}",
            pad.join
        ));
    }
    let Some(net) = pad.net.as_deref().filter(|n| filled(&board.stack, n)) else {
        return Ok(None);
    };
    declared.map(Some).ok_or_else(|| {
        format!(
            "{what} is a through-hole pad on {net}, which is filled, and the specification declares no join for {}",
            part.reference
        )
    })
}

/// True when a disc of radius `r` at `c` lies inside `pad`'s copper.
fn fits(pad: &Pad, c: (i64, i64), r: i64) -> bool {
    let (corner, at) = (pad.corner_nm, Rect::point(c.0, c.1));
    if corner >= r {
        gap2(pad.inner(), at) <= (corner - r) * (corner - r)
    } else {
        pad.inner().grow(corner - r).contains(at)
    }
}

/// Every declared thermal array at its sites on this board: `columns` x
/// `rows` vias `pitch` apart, centred on the pad, turned with the footprint.
/// Refused by name: a declared pad that is not exactly one SMD pad of one
/// part, a footprint that carries vias of its own under it (KiCad's
/// `_ThermalVias` footprints do, as through-hole pads of the same number),
/// a pad with no net, a rule layer that is not a fill of the pad's net, and
/// a site whose via would leave the pad's copper.
pub fn arrays(board: &Board) -> Result<Vec<Array>, String> {
    let mut out: Vec<Array> = Vec::new();
    for t in &board.stack.thermal {
        let what = format!("{} pad {}", t.reference, t.pad);
        let parts: Vec<&Part> = board
            .parts
            .iter()
            .filter(|p| p.reference == t.reference)
            .collect();
        let [part] = parts.as_slice() else {
            return Err(format!(
                "the specification declares a thermal array under {what}, and the board has {} parts {}",
                parts.len(),
                t.reference
            ));
        };
        let numbered: Vec<&Pad> = part.pads.iter().filter(|p| p.number == t.pad).collect();
        let own: Vec<i64> = numbered.iter().filter_map(|p| p.drill_nm).collect();
        if !own.is_empty() {
            return Err(format!(
                "{what}: the footprint carries {} vias of its own (drills {own:?} nm), which contradict the declared array of {} x {} drilled {} nm",
                own.len(),
                t.columns,
                t.rows,
                t.drill_nm
            ));
        }
        let [pad] = numbered.as_slice() else {
            return Err(format!(
                "{what}: {} SMD pads of that number",
                numbered.len()
            ));
        };
        let net = pad
            .net
            .clone()
            .ok_or_else(|| format!("{what} has no net"))?;
        for l in &t.layers {
            let fill = board.stack.plane_of_layer(l) == Some(net.as_str())
                || (!board.stack.has_planes() && l == "B.Cu" && net == GROUND);
            if !fill {
                return Err(format!(
                    "{what}: the rule's layer {l} is not a fill of {net}"
                ));
            }
        }
        if out
            .iter()
            .any(|a| a.reference == t.reference && a.pad == t.pad)
        {
            return Err(format!("{what}: two arrays are declared under it"));
        }
        let (cx, cy) = pad.rect.centre();
        let offset = |i: usize, n: usize| (2 * i as i64 - (n as i64 - 1)) * t.pitch_nm / 2;
        let mut at = Vec::new();
        for j in 0..t.rows {
            for i in 0..t.columns {
                let (dx, dy) = rotate(part.rotation, (offset(i, t.columns), offset(j, t.rows)));
                let c = (cx + dx, cy + dy);
                if !fits(pad, c, t.via_nm / 2) {
                    return Err(format!(
                        "{what}: the array's via at {c:?} leaves the pad's copper; the rule cannot be met on this footprint"
                    ));
                }
                at.push(c);
            }
        }
        at.sort();
        out.push(Array {
            reference: t.reference.clone(),
            pad: t.pad.clone(),
            net,
            via_nm: t.via_nm,
            drill_nm: t.drill_nm,
            at,
        });
    }
    out.sort();
    Ok(out)
}

/// Where a pad of a filled net meets the fill's sure copper `core`: solid,
/// at a node of its own copper ring outside the hole; relieved, at the ring
/// past its relief gap, every node of which must be sure fill so that a
/// spoke at any angle lands in it (FEEDBACK F5).
fn meets(grid: &Grid, core: &[bool], pad: &Pad, join: crate::stack::Join) -> Option<usize> {
    let (cx, cy) = pad.rect.centre();
    match join {
        crate::stack::Join::Solid => {
            let hole = pad.drill_nm.unwrap_or(0) / 2 + EPS_NM;
            grid.nodes(pad.rect).find(|&n| {
                let (x, y) = grid.at(n);
                core[n]
                    && fits(pad, (x, y), 0)
                    && (x - cx) * (x - cx) + (y - cy) * (y - cy) > hole * hole
            })
        }
        crate::stack::Join::Relief => {
            let lo = THERMAL_GAP_NM + MIN_THICKNESS_NM / 2 + EPS_NM;
            let hi = lo + 2 * PITCH_NM;
            let ring: Vec<usize> = grid
                .nodes(pad.rect.grow(hi))
                .filter(|&n| {
                    let (x, y) = grid.at(n);
                    let d2 = gap2(pad.rect, Rect::point(x, y));
                    d2 >= lo * lo && d2 <= hi * hi
                })
                .collect();
            ring.iter()
                .all(|&n| core[n])
                .then(|| ring.first().copied())?
        }
    }
}

/// The node nearest a point that need not lie on the grid.
fn nearest(grid: &Grid, (x, y): (i64, i64)) -> Option<usize> {
    grid.nodes(Rect::point(x, y).grow(PITCH_NM / 2)).next()
}

/// The dense part's ring interior (`AET-WO-1307` K1, the build's rule): the
/// box bounded by the inner edges of the pads nearest the field's centre lines
/// on each of the four sides. A part qualifies only with pads on all four
/// sides, an interior at least [`DENSE_INTERIOR_NM`] across both ways, and no
/// pad inside it, so a QFN's exposed pad excludes it. Why: at a 0.5 mm pitch
/// a pad ring's adjacent plane pads cannot all drop their vias outward in the
/// one strip their neighbours' escapes must also cross (the spike's S1); the
/// interior under the part's own body is empty copper the escape can use
/// instead.
pub fn ring_interior(part: &Part) -> Option<Rect> {
    let field = part.pads.iter().map(|p| p.rect).reduce(|a, b| a.union(b))?;
    let (cx, cy) = ((field.x0 + field.x1) / 2, (field.y0 + field.y1) / 2);
    let (qw, qh) = ((field.x1 - field.x0) / 4, (field.y1 - field.y0) / 4);
    let mid = |p: &&Pad| p.rect.centre();
    let pads = || part.pads.iter();
    let rows = |y: i64| (y - cy).abs() < qh;
    let cols = |x: i64| (x - cx).abs() < qw;
    let x0 = pads()
        .filter(|p| mid(p).0 < cx && rows(mid(p).1))
        .map(|p| p.rect.x1)
        .max()?;
    let x1 = pads()
        .filter(|p| mid(p).0 > cx && rows(mid(p).1))
        .map(|p| p.rect.x0)
        .min()?;
    let y0 = pads()
        .filter(|p| mid(p).1 < cy && cols(mid(p).0))
        .map(|p| p.rect.y1)
        .max()?;
    let y1 = pads()
        .filter(|p| mid(p).1 > cy && cols(mid(p).0))
        .map(|p| p.rect.y0)
        .min()?;
    let r = Rect { x0, y0, x1, y1 };
    let inside =
        |p: &Pad| p.rect.x0 < r.x1 && p.rect.x1 > r.x0 && p.rect.y0 < r.y1 && p.rect.y1 > r.y0;
    (x1 - x0 >= DENSE_INTERIOR_NM && y1 - y0 >= DENSE_INTERIOR_NM && !pads().any(inside))
        .then_some(r)
}

/// The least free interior, each way, of a dense part (K1).
pub const DENSE_INTERIOR_NM: i64 = 4_000_000;

/// True when `p` is inside the closed outline `edges` or on it, exactly
/// (crossings of a ray to +x, half-open in y).
fn under(edges: &[Edge], p: (i64, i64)) -> bool {
    let mut inside = false;
    for &(a, b) in edges {
        let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        let (px, py) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
        if dx * py == dy * px && span(a, b).contains(Rect::point(p.0, p.1)) {
            return true;
        }
        if (a.1 > p.1) != (b.1 > p.1) && (dy > 0) == (px * dy < py * dx) {
            inside = !inside;
        }
    }
    inside
}

/// True when some edge of `edges` passes nearer than `r` to `p`, exactly.
fn near(edges: &[Edge], p: (i64, i64), r: i64) -> bool {
    let r2 = i128::from(r) * i128::from(r);
    edges.iter().any(|&(a, b)| {
        let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
        let (px, py) = (i128::from(p.0 - a.0), i128::from(p.1 - a.1));
        let (l2, dot) = (dx * dx + dy * dy, px * dx + py * dy);
        if l2 == 0 || dot <= 0 {
            px * px + py * py < r2
        } else if dot >= l2 {
            let (qx, qy) = (px - dx, py - dy);
            qx * qx + qy * qy < r2
        } else {
            let cross = px * dy - py * dx;
            cross * cross < r2 * l2
        }
    })
}

/// True when a via of radius `r` centred at `p` touches `pad`'s copper (its
/// box), exactly.
fn touches(pad: &Pad, p: (i64, i64), r: i64) -> bool {
    gap2(pad.rect, Rect::point(p.0, p.1)) <= r * r
}

/// True when a via of radius `r` centred at `p` puts copper in `part`'s
/// courtyard, outside the part's own ring interior `hole` (K1).
fn in_courtyard(part: &Part, hole: Option<Rect>, p: (i64, i64), r: i64) -> bool {
    !hole.is_some_and(|h| h.contains(Rect::point(p.0, p.1)))
        && (under(&part.courtyard, p) || near(&part.courtyard, p, r))
}

fn owner(net: usize) -> u32 {
    u32::try_from(net + 1).unwrap_or(BLOCKED)
}

fn opens(o: u32, who: u32) -> bool {
    o == FREE || o == who
}

impl<'a> Router<'a> {
    fn new(board: &'a Board, grid: Grid, nets: Vec<String>, arrays: &[Array]) -> Router<'a> {
        let n = grid.len();
        let count = nets.len();
        // One set of maps per distinct rule: each is the board's arithmetic
        // at that rule's width and via, so a board with no class is today's.
        let rules: Vec<Rule> = nets.iter().map(|net| board.rule(net)).collect();
        let mut kinds = vec![board.stack.default_rule()];
        for r in &rules {
            if !kinds.contains(r) {
                kinds.push(*r);
            }
        }
        let kind: Vec<usize> = rules
            .iter()
            .map(|r| kinds.iter().position(|k| k == r).unwrap_or(0))
            .collect();
        let mut track: Vec<[Vec<u32>; 2]> = kinds
            .iter()
            .map(|_| [vec![FREE; n], vec![FREE; n]])
            .collect();
        let mut via: Vec<Vec<u32>> = kinds.iter().map(|_| vec![FREE; n]).collect();
        let mut corridors = vec![Vec::new(); count];
        let mut via_corridors = vec![Vec::new(); count];
        let o = board.outline;
        for node in 0..n {
            let (x, y) = grid.at(node);
            let edge = to_edge(o, (x, y));
            for (k, r) in kinds.iter().enumerate() {
                if edge < EDGE_NM + r.width_nm / 2 + EPS_NM {
                    track[k][0][node] = BLOCKED;
                    track[k][1][node] = BLOCKED;
                }
                if edge < EDGE_NM + r.via_nm / 2 + EPS_NM {
                    via[k][node] = BLOCKED;
                }
            }
        }
        let index: BTreeMap<&str, usize> = nets
            .iter()
            .enumerate()
            .map(|(i, n)| (n.as_str(), i))
            .collect();
        let net_of = |pad: &Pad| pad.net.as_deref().and_then(|n| index.get(n).copied());
        let mut cuts = Vec::new();
        let mut anchors = vec![Vec::new(); count];
        // A drilled hole keeps every via's hole the gap away.
        let hole = |via: &mut Vec<Vec<u32>>, at: (i64, i64), drill: i64| {
            let centre = Shape {
                rect: Rect::point(at.0, at.1),
                r: 0,
            };
            for (k, r) in kinds.iter().enumerate() {
                let reach = hole_pitch(drill, r.drill_nm) + EPS_NM;
                claim(&mut via[k], &grid, centre, reach, BLOCKED);
            }
        };
        for array in arrays {
            let net = index.get(array.net.as_str()).copied();
            let who = net.map_or(BLOCKED, owner);
            for &at in &array.at {
                let shape = Shape {
                    rect: Rect::point(at.0, at.1),
                    r: array.via_nm / 2,
                };
                for (k, &r) in kinds.iter().enumerate() {
                    claim(&mut track[k][1], &grid, shape, board.track_reach(r), who);
                    claim(&mut via[k], &grid, shape, board.via_reach(r), who);
                }
                hole(&mut via, at, array.drill_nm);
                cuts.push((net, shape));
                if let Some(n) = net {
                    anchors[n].push(Anchor::Via(at));
                }
            }
        }
        for (pi, part) in board.parts.iter().enumerate() {
            for (i, pad) in part.pads.iter().enumerate() {
                let who = net_of(pad).map_or(BLOCKED, owner);
                let shape = Shape {
                    rect: pad.rect,
                    r: 0,
                };
                let layers = if pad.drill_nm.is_some() { 2 } else { 1 };
                for (k, &r) in kinds.iter().enumerate() {
                    for t in &mut track[k][..layers] {
                        claim(t, &grid, shape, board.track_reach(r), who);
                    }
                    claim(&mut via[k], &grid, shape, board.via_reach(r), who);
                }
                if let Some(d) = pad.drill_nm {
                    hole(&mut via, pad.rect.centre(), d);
                    cuts.push((net_of(pad), shape));
                    if let (Some(n), Ok(Some(join))) = (net_of(pad), join_of(board, part, pad)) {
                        anchors[n].push(Anchor::Pad((pi, i), join));
                    }
                }
            }
            // No via's copper in the courtyard, but in the part's own ring (K1).
            let hole = ring_interior(part);
            if let Some(yard) = part
                .courtyard
                .iter()
                .map(|&(a, b)| span(a, b))
                .reduce(|a, b| a.union(b))
            {
                for (k, r) in kinds.iter().enumerate() {
                    for node in grid.nodes(yard.grow(r.via_nm / 2)) {
                        if in_courtyard(part, hole, grid.at(node), r.via_nm / 2) {
                            via[k][node] = BLOCKED;
                        }
                    }
                }
            }
            // The pad field: on F.Cu, only inside a pad; no via near it;
            // except in a dense part's own ring interior (K1).
            let Some(field) = part.pads.iter().map(|p| p.rect).reduce(|a, b| a.union(b)) else {
                continue;
            };
            let own = |node: usize| {
                let (x, y) = grid.at(node);
                hole.is_some_and(|h| h.contains(Rect::point(x, y)))
            };
            for node in grid.nodes(field) {
                let (x, y) = grid.at(node);
                if !own(node) && !part.pads.iter().any(|p| p.rect.contains(Rect::point(x, y))) {
                    for t in &mut track {
                        t[0][node] = BLOCKED;
                    }
                }
            }
            for (k, r) in kinds.iter().enumerate() {
                for node in grid.nodes(field.grow(r.via_nm / 2 + board.s())) {
                    if !own(node) {
                        via[k][node] = BLOCKED;
                    }
                }
                // In the ring, a via of any net touches no pad (K1: no via in pad).
                for pad in &part.pads {
                    for node in grid.nodes(pad.rect.grow(r.via_nm / 2)) {
                        if own(node) && touches(pad, grid.at(node), r.via_nm / 2) {
                            via[k][node] = BLOCKED;
                        }
                    }
                }
            }
            for pad in &part.pads {
                if let Some(net) = net_of(pad) {
                    let r = rules[net];
                    let half = board.corridor_half(r);
                    corridors[net].extend(corridor(&grid, field, pad, half, 0));
                    via_corridors[net].extend(corridor(
                        &grid,
                        field,
                        pad,
                        half + r.via_nm / 2,
                        r.via_nm / 2 + board.s(),
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
            kinds,
            kind,
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
            no_vias: false,
            cuts,
            anchors,
            fixed: vec![false; count],
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

    /// The rule `net` is laid by.
    fn rule(&self, net: usize) -> Rule {
        self.kinds[self.kind[net]]
    }

    /// Whether `net` may centre a track at `node` on `layer`: by every
    /// routed net's copper, or (`hard`) by pads, fields and the edge alone.
    fn open(&self, layer: Layer, node: usize, net: usize) -> bool {
        opens(self.track[self.kind[net]][layer.index()][node], owner(net))
    }

    fn hard_open(&self, layer: Layer, node: usize, net: usize) -> bool {
        opens(
            self.hard_track[self.kind[net]][layer.index()][node],
            owner(net),
        )
    }

    fn via_open(&self, node: usize, net: usize) -> bool {
        opens(self.via[self.kind[net]][node], owner(net)) || self.vias.get(&node) == Some(&net)
    }

    fn hard_via_open(&self, node: usize, net: usize) -> bool {
        opens(self.hard_via[self.kind[net]][node], owner(net))
    }

    /// The first pair of `path`'s own new vias closer than the hole gap,
    /// as the indices of the first's lower state and the second's upper
    /// one. The map keeps vias apart from those already laid, not within a
    /// path.
    fn close_vias(&self, net: usize, path: &[Node]) -> Option<(usize, usize)> {
        let reach = hole_reach(self.rule(net), self.rule(net));
        let mut laid: Vec<(usize, (i64, i64))> = Vec::new();
        for (i, w) in path.windows(2).enumerate() {
            let ((la, a), (lb, _)) = (w[0], w[1]);
            if la == lb || self.vias.contains_key(&a) {
                continue;
            }
            let (x, y) = self.grid.at(a);
            if let Some(&(j, _)) = laid.iter().rev().find(|&&(_, (px, py))| {
                gap2(Rect::point(px, py), Rect::point(x, y)) < reach * reach
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
        while let Some((i, j)) = self.close_vias(net, path) {
            let outer = path[i].0;
            // path[i] and path[j] are the hop's ends on the outer layer;
            // path[i + 1..j] is the hop itself, its ends included.
            let flat: Vec<Node> = path[i + 2..j - 1]
                .iter()
                .map(|&(_, n)| (outer, n))
                .collect();
            if flat.iter().all(|&(l, n)| self.open(l, n, net)) {
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
            .filter(|&n| self.hard_open(Layer::F, n, net))
            .collect()
    }

    /// The same as states, on B.Cu as well for a through-hole pad.
    fn ends(&self, pad: &Pad, net: usize) -> Vec<Node> {
        let mut out: Vec<Node> = self
            .terminals(pad, net)
            .into_iter()
            .map(|n| (Layer::F, n))
            .collect();
        if pad.drill_nm.is_some() {
            out.extend(
                self.grid
                    .nodes(pad.inner())
                    .filter(|&n| self.hard_open(Layer::B, n, net))
                    .map(|n| (Layer::B, n)),
            );
        }
        out
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
        // The price of a node: free, or (soft) another net's.
        let price = |open: bool, hard: bool| match (open, soft && hard) {
            (true, _) => Some(0),
            (false, true) => Some(SOFT_COST),
            (false, false) => None,
        };
        let mut heap = BinaryHeap::new();
        // A source is priced as a move into it is (F2): a pad's terminal is
        // open by the pads alone, but a track's end there must also keep clear
        // of the copper already laid.
        for &(l, node) in sources {
            let Some(extra) = price(self.open(l, node, net), self.hard_open(l, node, net)) else {
                continue;
            };
            let s = id(l, node, 4);
            if self.stamp[s] != g || extra < self.cost[s] {
                self.stamp[s] = g;
                self.cost[s] = extra;
                self.prev[s] = SOURCE;
                heap.push(Reverse((extra + h(node), s)));
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
                    let Some(extra) = price(self.open(l, m, net), self.hard_open(l, m, net)) else {
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
                self.via_open(node, net) && self.open(other, node, net),
                self.hard_via_open(node, net),
            );
            if let Some(extra) = via_price.filter(|_| !self.no_vias && !self.banned.contains(&node))
            {
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
                    // Only a pair's fixed copper in the way: a wider free
                    // search may still go round it (or under, by a via).
                    if by.is_empty() && self.fixed.contains(&true) {
                        continue;
                    }
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
        let rule = self.rule(net);
        let mut out = BTreeSet::new();
        // The source is a step too (F2): a path may start where it is not free.
        let first = path.first().map(|&(l, n)| ((l, n), (l, n)));
        let steps = first
            .into_iter()
            .chain(path.windows(2).map(|w| (w[0], w[1])));
        for ((la, a), (lb, b)) in steps {
            let (at, reach, layers) = if la == lb {
                if self.open(lb, b, net) {
                    continue;
                }
                (
                    b,
                    self.board.track_reach(rule),
                    [lb == Layer::F, lb == Layer::B],
                )
            } else {
                if self.via_open(a, net) && self.open(lb, a, net) {
                    continue;
                }
                (a, self.board.via_reach(rule), [true, true])
            };
            // Every node where a blocker's track would stand in the way
            // costs more from now on: the contested node's shadow.
            let hole = (la != lb).then_some(rule);
            let (x, y) = self.grid.at(at);
            let shadow = reach + self.board.w() / 2;
            out.extend(self.contest(Rect::point(x, y), shadow, reach, layers, net, hole));
        }
        out
    }

    /// Raises the history of every node within `shadow` of `here` on
    /// `layers`, and names the nets but `skip` whose copper on those layers
    /// is within `reach` of it, or (`hole`: a via of that rule stands at
    /// `here`) whose via hole is too near.
    fn contest(
        &mut self,
        here: Rect,
        shadow: i64,
        reach: i64,
        layers: [bool; 2],
        skip: usize,
        hole: Option<Rule>,
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
            let r = reach + self.rule(other).width_nm / 2;
            let near = (0..2).filter(|&l| layers[l]).any(|l| {
                e[l].iter()
                    .any(|&(p, q)| gap2(span(self.grid.at(p), self.grid.at(q)), here) < r * r)
            });
            if other != skip && near && !self.fixed[other] {
                out.insert(other);
            }
        }
        for (&v, &other) in &self.vias {
            let (vx, vy) = self.grid.at(v);
            let d2 = gap2(Rect::point(vx, vy), here);
            let copper = reach + self.rule(other).via_nm / 2;
            let hole = hole.map_or(0, |h| hole_reach(h, self.rule(other)));
            if other != skip && !self.fixed[other] && (d2 < copper * copper || d2 < hole * hole) {
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
            r: self.rule(net).width_nm / 2,
        };
        let who = owner(net);
        for (k, &r) in self.kinds.iter().enumerate() {
            let (track_reach, via_reach) = (self.board.track_reach(r), self.board.via_reach(r));
            claim(
                &mut self.track[k][layer.index()],
                &self.grid,
                shape,
                track_reach,
                who,
            );
            claim(&mut self.via[k], &self.grid, shape, via_reach, who);
        }
    }

    fn claim_via(&mut self, net: usize, node: usize) {
        let (x, y) = self.grid.at(node);
        let own = self.rule(net);
        let shape = Shape {
            rect: Rect::point(x, y),
            r: own.via_nm / 2,
        };
        let who = owner(net);
        let centre = Shape {
            rect: Rect::point(x, y),
            r: 0,
        };
        for (k, &r) in self.kinds.iter().enumerate() {
            let (track_reach, via_reach) = (self.board.track_reach(r), self.board.via_reach(r));
            for l in 0..2 {
                claim(&mut self.track[k][l], &self.grid, shape, track_reach, who);
            }
            claim(&mut self.via[k], &self.grid, shape, via_reach, who);
            claim(
                &mut self.via[k],
                &self.grid,
                centre,
                hole_reach(own, r),
                BLOCKED,
            );
        }
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
        let start = self.ends(self.pad(first), net);
        if start.is_empty() {
            return Err(Stuck::Unroutable(format!(
                "{} has no node clear to start a track",
                self.pad_name(first)
            )));
        }
        let mut tree: BTreeSet<Node> = start.into_iter().collect();
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
            let states = self.ends(self.pad(target), net);
            if states.is_empty() {
                return Err(Stuck::Unroutable(format!(
                    "{name} has no node clear to end a track"
                )));
            }
            let set: BTreeSet<Node> = states.iter().copied().collect();
            let ends: Vec<usize> = states.iter().map(|&(_, n)| n).collect();
            let bx = self.grid.span(&ends).unwrap_or_default();
            let bx = (bx.0 as i64, bx.1 as i64, bx.2 as i64, bx.3 as i64);
            let h = move |node: usize| {
                let (x, y) = ((node as i64) % nx, (node as i64) / nx);
                let dx = (bx.0 - x).max(x - bx.1).max(0);
                let dy = (bx.2 - y).max(y - bx.3).max(0);
                u32::try_from(dx + dy).unwrap_or(u32::MAX / 4) * STEP_F
            };
            let goal = |l: Layer, n: usize| set.contains(&(l, n));
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
            tree.extend(states);
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
            // The fill reaches a through-hole pad itself, by its declared
            // join, and a pad over a thermal array through the array's vias
            // (`AET-WO-1309`): neither needs a track to a via.
            let part = &self.board.parts[p.0];
            join_of(self.board, part, self.pad(p)).map_err(Stuck::Unroutable)?;
            if self.pad(p).drill_nm.is_some()
                || self
                    .board
                    .stack
                    .thermal
                    .iter()
                    .any(|t| t.reference == part.reference && t.pad == self.pad(p).number)
            {
                continue;
            }
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
    fn zone_blockers(&mut self, ground: usize) -> Result<Option<BTreeSet<usize>>, String> {
        let core = self.zone_core(ground);
        let mut seeds: Vec<usize> = self
            .vias
            .iter()
            .filter(|&(_, &net)| net == ground)
            .map(|(&n, _)| n)
            .collect();
        seeds.extend(self.anchor_seeds(ground, &core)?);
        let labels = components(&self.grid, &core, &seeds);
        // The piece holding the most ground vias, then the first, is the zone.
        let main = main_label(&labels);
        if labels.iter().all(|l| l.is_some() && *l == main) {
            return Ok(None);
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
            out.extend(self.contest(here, shadow, reach, [false, true], ground, None));
        }
        Ok(Some(out))
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
                .flat_map(|(net, e)| {
                    e[1].iter().map(move |&(a, b)| Shape {
                        rect: span(self.grid.at(a), self.grid.at(b)),
                        r: self.rule(net).width_nm / 2,
                    })
                }),
            self.vias
                .iter()
                .filter(|&(_, &net)| net != ground)
                .map(|(&n, &net)| {
                    let (x, y) = self.grid.at(n);
                    Shape {
                        rect: Rect::point(x, y),
                        r: self.rule(net).via_nm / 2,
                    }
                })
                .chain(
                    self.cuts
                        .iter()
                        .filter(|(n, _)| *n != Some(ground))
                        .map(|&(_, s)| s),
                ),
        )
    }

    /// The nodes where `net`'s fixed copper meets its fill's sure copper
    /// `core`, or the first piece that meets it nowhere.
    fn anchor_seeds(&self, net: usize, core: &[bool]) -> Result<Vec<usize>, String> {
        self.anchors[net]
            .iter()
            .map(|a| match *a {
                Anchor::Via(at) => nearest(&self.grid, at)
                    .ok_or_else(|| format!("a thermal via at {at:?} is off the grid")),
                Anchor::Pad(p, join) => {
                    meets(&self.grid, core, self.pad(p), join).ok_or_else(|| {
                        format!(
                            "{} meets the {} nowhere it is sure to be filled ({join:?})",
                            self.pad_name(p),
                            zone_name(&self.board.stack, &self.nets[net])
                        )
                    })
                }
            })
            .collect()
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

/// A pad's centre as (y, x): the key of every geometric order (order B).
fn yx(pad: &Pad) -> (i64, i64) {
    let (x, y) = pad.rect.centre();
    (y, x)
}

/// Each routed net's pads, in order: by centre, y then x (order B). Two pads
/// of one net with one centre keep the order of `board.parts`; [`check`]
/// refuses two such pads of different nets.
fn net_pads(board: &Board) -> BTreeMap<String, Vec<PadRef>> {
    let mut out: BTreeMap<String, Vec<PadRef>> = BTreeMap::new();
    for (part, p) in board.parts.iter().enumerate() {
        for (i, pad) in p.pads.iter().enumerate() {
            if let Some(net) = &pad.net {
                out.entry(net.clone()).or_default().push((part, i));
            }
        }
    }
    for list in out.values_mut() {
        list.sort_by_key(|&(p, i)| yx(&board.parts[p].pads[i]));
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
    // The stitched nets, the filled ones: ground at 2 layers (the B.Cu zone);
    // at 4, each plane's net. Their stitching is laid before any net is
    // tracked.
    let is_stitched = |n: &str| filled(&board.stack, n);
    // Order B: each net's index is its rank by (stitched first, pad count,
    // its pads' centres), the name last; so is every choice made by index.
    let mut nets: Vec<String> = pads.keys().cloned().collect();
    nets.sort_by_cached_key(|name| {
        let list = &pads[name];
        let at: Vec<(i64, i64)> = list
            .iter()
            .map(|&(p, i)| yx(&board.parts[p].pads[i]))
            .collect();
        (!is_stitched(name), list.len(), at, name.clone())
    });
    let pads: Vec<Vec<PadRef>> = nets.iter().map(|n| pads[n].clone()).collect();
    let order: Vec<usize> = (0..nets.len())
        // belt-and-suspenders: an `unconnected-` net is one pin's alone
        // (`pcb::pin_nets`, and parity refuses two), so the pad count already
        // skips it; this keeps a track from ever joining one.
        .filter(|&i| pads[i].len() > 1 && !nets[i].starts_with(crate::parity::UNCONNECTED))
        .collect();
    let stitched: Vec<usize> = (0..nets.len()).filter(|&i| is_stitched(&nets[i])).collect();
    let arrays = arrays(board).map_err(|why| fail("", why))?;
    let mut r = Router::new(board, grid, nets.clone(), &arrays);
    // Each declared pair first, whole, on the bare board (`AET-WO-1308`):
    // its copper is fixed, and its nets leave the queue. A board with no
    // pair is routed as before.
    let index = |name: &str| nets.iter().position(|n| n == name);
    let mut paired = BTreeSet::new();
    for pair in &board.stack.pairs {
        let (Some(p), Some(n)) = (index(&pair.nets[0]), index(&pair.nets[1])) else {
            return Err(fail(
                &pair.nets[0],
                format!("pair `{}`: a conductor has no pad on the board", pair.name),
            ));
        };
        r.release(p, true);
        r.release(n, true);
        r.route_pair(pair, (p, n), [&pads[p], &pads[n]])
            .map_err(|why| fail(&pair.nets[0], why))?;
        r.fixed[p] = true;
        r.fixed[n] = true;
        paired.extend([p, n]);
    }
    let mut queue: VecDeque<usize> = order
        .iter()
        .copied()
        .filter(|i| !paired.contains(i))
        .collect();
    let mut ripups = 0;
    let zones = loop {
        while let Some(net) = queue.pop_front() {
            r.release(net, true);
            loop {
                let p = &pads[net];
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
            if r.vias.values().any(|v| *v == g) || !r.anchors[g].is_empty() {
                let by = r.zone_blockers(g).map_err(|why| fail(&nets[g], why))?;
                if let Some(by) = by {
                    split = Some((g, by));
                    break;
                }
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
            } else if stitched
                .iter()
                .any(|&g| r.vias.values().any(|&v| v == g) || !r.anchors[g].is_empty())
            {
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
        arrays,
        ripups,
    };
    // belt-and-suspenders: the grid's margins already keep the copper clear
    // and joined (no tested board reaches these); the exact judges stand
    // behind them, so a router defect fails closed rather than ships.
    check(board, &routed).map_err(|why| fail("", why))?;
    connectivity(board, &routed).map_err(|why| fail("", why))?;
    pairs(board, &routed).map_err(|why| fail("", why))?;
    Ok(routed)
}

// ---- coupled pairs (`AET-WO-1308` §1.3) ----

/// How far, along its own copper, each conductor of a pair may run uncoupled
/// at each end, from its pad to the coupled run (FEEDBACK F5).
pub const PAIR_BREAKOUT_NM: i64 = 4_000_000;
/// The largest length difference between a pair's two conductors (F6): an
/// engine condition, not a USB figure. At full speed's 83 ns bit it is about
/// 13 ps of a 6.7 ps/mm line.
pub const PAIR_SKEW_NM: i64 = 2_000_000;

/// Grid directions, as [`Grid::step`] numbers them: +x, −x, +y, −y.
const RIGHT: [usize; 4] = [2, 3, 1, 0];
const LEFT: [usize; 4] = [3, 2, 0, 1];

/// One move of a pair's coupled run, as its positive conductor makes it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Turn {
    Straight,
    Right,
    Left,
}

/// What one pair's two conductors measure, from the board's copper alone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PairReport {
    pub name: String,
    pub nets: [String; 2],
    pub gap_nm: i64,
    /// Per conductor: its length, its vias, and its uncoupled length at the
    /// end nearer its first pad and at the other end.
    pub length_nm: [i64; 2],
    pub vias: [usize; 2],
    pub uncoupled_nm: [[i64; 2]; 2],
    pub layers: Vec<&'static str>,
}

impl PairReport {
    pub fn length_difference_nm(&self) -> i64 {
        (self.length_nm[0] - self.length_nm[1]).abs()
    }
}

impl<'a> Router<'a> {
    /// `n` steps from `node` in direction `d`, or `None` off the grid.
    fn walk(&self, mut node: usize, d: usize, n: usize) -> Option<usize> {
        for _ in 0..n {
            node = self.grid.step(node, d)?;
        }
        Some(node)
    }

    /// Distance in pitches, Manhattan, from `node` to the point `p`.
    fn pitches(&self, node: usize, p: (i64, i64)) -> i64 {
        let (x, y) = self.grid.at(node);
        ((x - p.0).abs() + (y - p.1).abs()) / PITCH_NM
    }

    /// The cost of laying `net`'s copper on F.Cu at `node`, or `None` where
    /// it may not.
    fn pair_cell(&self, net: usize, node: Option<usize>) -> Option<u32> {
        let node = node?;
        self.open(Layer::F, node, net)
            .then(|| STEP_F + self.pending[node] + self.history[0][node])
    }

    /// The pair's coupled run: the positive conductor's start node and
    /// direction and its moves. Searched over states (node, direction, steps
    /// since the last turn) on F.Cu only, in a window around the four pads.
    /// The negative conductor stands `m` pitches to the positive's right; a
    /// turn is a macro of the turn and `m` steps, and two turns are at least
    /// `2m + 1` steps apart, so every leg of both conductors is at least a
    /// pitch long. `ends[e][c]` is end `e`'s pad centre for conductor `c`.
    fn pair_run(
        &self,
        (p, n): (usize, usize),
        m: usize,
        ends: [[(i64, i64); 2]; 2],
        margin: usize,
    ) -> Option<(usize, usize, Vec<Turn>)> {
        let reach = PAIR_BREAKOUT_NM / PITCH_NM / 2;
        let gate = 2 * m + 1;
        let corners: Vec<usize> = ends
            .iter()
            .flatten()
            .filter_map(|&(x, y)| self.grid.nodes(Rect::point(x, y).grow(PITCH_NM)).next())
            .collect();
        let (wx0, wy0, wx1, wy1) = self.window(&corners, Some(margin));
        let (wnx, wny) = (wx1 - wx0 + 1, wy1 - wy0 + 1);
        let states = wnx * wny * 4 * (gate + 1);
        if states > MAX_NODES * 4 {
            return None;
        }
        let local = |node: usize| {
            let (ix, iy) = (node % self.grid.nx, node / self.grid.nx);
            (ix >= wx0 && ix <= wx1 && iy >= wy0 && iy <= wy1)
                .then(|| (iy - wy0) * wnx + (ix - wx0))
        };
        let id = |l: usize, d: usize, s: usize| (l * 4 + d) * (gate + 1) + s;
        let mut cost = vec![u32::MAX; states];
        // Per state, its predecessor and the move into it.
        let mut prev: Vec<(u32, u8)> = vec![(u32::MAX, 0); states];
        let mut heap = BinaryHeap::new();
        let h = |node: usize| {
            u32::try_from((self.pitches(node, ends[1][0]) - reach).max(0)).unwrap_or(u32::MAX / 4)
                * STEP_F
        };
        let negative = |node: usize, d: usize| self.walk(node, RIGHT[d], m);
        // A breakout is priced by its two pads' distance. The run leaves end
        // A heading away from its pads and reaches end B heading towards
        // them (`d`, the run's direction there), and the negative pad stands
        // strictly on the run's right, so the two breakouts need not cross.
        let unit = |d: usize| [(1, 0), (-1, 0), (0, 1), (0, -1)][d];
        let breakout = |node: usize, d: usize, e: usize| -> Option<u32> {
            let neg = negative(node, d)?;
            let (a, b) = (
                self.pitches(node, ends[e][0]),
                self.pitches(neg, ends[e][1]),
            );
            let ((rx, ry), (ux, uy)) = (unit(RIGHT[d]), unit(d));
            let [pp, pn] = ends[e];
            let right = (pn.0 - pp.0) * rx + (pn.1 - pp.1) * ry > 0;
            let (x0, y0) = self.grid.at(node);
            let (x1, y1) = self.grid.at(neg);
            // Twice the run's midpoint less twice the pads'.
            let (mx, my) = (x0 + x1 - pp.0 - pn.0, y0 + y1 - pp.1 - pn.1);
            let ahead = mx * ux + my * uy;
            let heading = if e == 0 { ahead > 0 } else { ahead < 0 };
            (a <= reach && b <= reach && right && heading)
                .then(|| u32::try_from(a + b).unwrap_or(0) * STEP_F)
        };
        for node in self
            .grid
            .nodes(Rect::point(ends[0][0].0, ends[0][0].1).grow(reach * PITCH_NM))
        {
            let Some(l) = local(node) else { continue };
            for d in 0..4 {
                let Some(start) = breakout(node, d, 0) else {
                    continue;
                };
                let (Some(a), Some(b)) = (
                    self.pair_cell(p, Some(node)),
                    self.pair_cell(n, negative(node, d)),
                ) else {
                    continue;
                };
                let s = id(l, d, m);
                let c = start + a + b;
                if c < cost[s] {
                    cost[s] = c;
                    prev[s] = (u32::MAX, 0);
                    heap.push(Reverse((c + h(node), false, s, node)));
                }
            }
        }
        while let Some(Reverse((f, done, s, node))) = heap.pop() {
            let (d, steps) = ((s / (gate + 1)) % 4, s % (gate + 1));
            if done {
                // The cheapest run with both breakouts priced: walk it back.
                let mut turns = Vec::new();
                let mut at = s;
                while prev[at].0 != u32::MAX {
                    turns.push(match prev[at].1 {
                        1 => Turn::Right,
                        2 => Turn::Left,
                        _ => Turn::Straight,
                    });
                    at = prev[at].0 as usize;
                }
                turns.reverse();
                let l = at / (4 * (gate + 1));
                let start = (wy0 + l / wnx) * self.grid.nx + wx0 + l % wnx;
                return Some((start, (at / (gate + 1)) % 4, turns));
            }
            let c = cost[s];
            if f != c + h(node) {
                continue;
            }
            if steps >= m
                && let Some(end) = breakout(node, d, 1)
            {
                heap.push(Reverse((c + end, true, s, node)));
            }
            let mut moves: Vec<(usize, usize, usize, u32, u8)> = Vec::new();
            // Straight: both conductors one step on.
            let next = self.grid.step(node, d);
            if let (Some(to), Some(a), Some(b)) = (
                next,
                self.pair_cell(p, next),
                self.pair_cell(n, next.and_then(|t| negative(t, d))),
            ) {
                moves.push((to, d, (steps + 1).min(gate), a + b, 0));
            }
            if steps >= gate {
                for (kind, d2) in [(1u8, RIGHT[d]), (2u8, LEFT[d])] {
                    let mut extra = BEND * 2;
                    let mut ok = true;
                    for j in 1..=m {
                        match self.pair_cell(p, self.walk(node, d2, j)) {
                            Some(c) => extra += c,
                            None => ok = false,
                        }
                    }
                    if kind == 2 {
                        // The outer conductor runs `m` on, turns, and runs `2m`.
                        let corner = negative(node, d).and_then(|x| self.walk(x, d, m));
                        for j in 1..=m {
                            let cell = negative(node, d).and_then(|x| self.walk(x, d, j));
                            match self.pair_cell(n, cell) {
                                Some(c) => extra += c,
                                None => ok = false,
                            }
                        }
                        for j in 1..=2 * m {
                            match self.pair_cell(n, corner.and_then(|x| self.walk(x, d2, j))) {
                                Some(c) => extra += c,
                                None => ok = false,
                            }
                        }
                    }
                    if let (true, Some(to)) = (ok, self.walk(node, d2, m)) {
                        moves.push((to, d2, m, extra, kind));
                    }
                }
            }
            for (to, d2, s2, step, kind) in moves {
                let Some(l) = local(to) else { continue };
                let t = id(l, d2, s2);
                let tc = c.saturating_add(step);
                if tc < cost[t] {
                    cost[t] = tc;
                    prev[t] = (u32::try_from(s).unwrap_or(u32::MAX), kind);
                    heap.push(Reverse((tc + h(to), false, t, to)));
                }
            }
        }
        None
    }

    /// The two conductors' F.Cu paths for a coupled run.
    fn pair_paths(
        &self,
        m: usize,
        start: usize,
        d0: usize,
        turns: &[Turn],
    ) -> Option<[Vec<Node>; 2]> {
        let mut d = d0;
        let mut pos = vec![start];
        let mut neg = vec![self.walk(start, RIGHT[d], m)?];
        for turn in turns {
            let (p_last, n_last) = (*pos.last()?, *neg.last()?);
            match turn {
                Turn::Straight => {
                    pos.push(self.grid.step(p_last, d)?);
                    neg.push(self.grid.step(n_last, d)?);
                }
                Turn::Right => {
                    let d2 = RIGHT[d];
                    for j in 1..=m {
                        pos.push(self.walk(p_last, d2, j)?);
                    }
                    // The inner conductor turns `m` before where it stood.
                    neg.truncate(neg.len().checked_sub(m)?);
                    d = d2;
                }
                Turn::Left => {
                    let d2 = LEFT[d];
                    for j in 1..=m {
                        pos.push(self.walk(p_last, d2, j)?);
                    }
                    for j in 1..=m {
                        neg.push(self.walk(n_last, d, j)?);
                    }
                    let corner = *neg.last()?;
                    for j in 1..=2 * m {
                        neg.push(self.walk(corner, d2, j)?);
                    }
                    d = d2;
                }
            }
        }
        let f = |v: Vec<usize>| v.into_iter().map(|n| (Layer::F, n)).collect();
        Some([f(pos), f(neg)])
    }

    /// Routes a declared pair whole: its coupled run, then each conductor's
    /// breakout at each end, on F.Cu with no via. A pair that cannot be laid
    /// so is refused by name, never routed apart (§1.3).
    fn route_pair(
        &mut self,
        pair: &crate::stack::Pair,
        (p, n): (usize, usize),
        pads: [&[PadRef]; 2],
    ) -> Result<(), String> {
        let name = &pair.name;
        if pads.iter().any(|list| list.len() != 2) {
            return Err(format!(
                "pair `{name}`: each conductor joins exactly two pads; `{}` has {} and `{}` has {}",
                pair.nets[0],
                pads[0].len(),
                pair.nets[1],
                pads[1].len()
            ));
        }
        let spacing = pair.gap_nm + pair.width_nm;
        if spacing % PITCH_NM != 0 {
            return Err(format!(
                "pair `{name}`: gap plus width, {spacing} nm, is not a whole number of the router's {PITCH_NM} nm pitch"
            ));
        }
        let m = usize::try_from(spacing / PITCH_NM).map_err(|e| e.to_string())?;
        // End A is the positive conductor's first pad, with the negative pad
        // on the same part.
        let (a, b) = (pads[0][0], pads[0][1]);
        let (na, nb) = if pads[1][0].0 == a.0 {
            (pads[1][0], pads[1][1])
        } else {
            (pads[1][1], pads[1][0])
        };
        if na.0 != a.0 || nb.0 != b.0 {
            return Err(format!(
                "pair `{name}`: its conductors do not run between the same two parts"
            ));
        }
        let centre = |r: PadRef| self.pad(r).rect.centre();
        let ends = [[centre(a), centre(na)], [centre(b), centre(nb)]];
        self.no_vias = true;
        let run = WINDOWS
            .iter()
            .find_map(|&margin| self.pair_run((p, n), m, ends, margin));
        let result = (|| {
            let (start, d0, turns) = run.ok_or_else(|| {
                format!(
                    "pair `{name}` cannot be coupled: no run on F.Cu holds both conductors {} nm apart",
                    pair.gap_nm
                )
            })?;
            let [pos, neg] = self
                .pair_paths(m, start, d0, &turns)
                .ok_or_else(|| format!("pair `{name}`: its run leaves the grid"))?;
            self.commit(p, &pos);
            self.commit(n, &neg);
            for (net, pad, target) in [
                (p, a, pos[0].1),
                (n, na, neg[0].1),
                (p, b, pos[pos.len() - 1].1),
                (n, nb, neg[neg.len() - 1].1),
            ] {
                let what = format!("pair `{name}` cannot break out to {}", self.pad_name(pad));
                let ends = self.terminals(self.pad(pad), net);
                let sources: Vec<Node> = ends.iter().map(|&t| (Layer::F, t)).collect();
                let goal = |l: Layer, x: usize| l == Layer::F && x == target;
                let mut window = ends.clone();
                window.push(target);
                let nx = self.grid.nx as i64;
                let (tx, ty) = ((target as i64) % nx, (target as i64) / nx);
                let h = move |x: usize| {
                    let (ix, iy) = ((x as i64) % nx, (x as i64) / nx);
                    u32::try_from((ix - tx).abs() + (iy - ty).abs()).unwrap_or(u32::MAX / 4)
                        * STEP_F
                };
                let path = self
                    .connect(net, &sources, &window, &goal, &h, false, &what)
                    .map_err(|_| what.clone())?;
                self.commit(net, &path);
            }
            Ok(())
        })();
        self.no_vias = false;
        result
    }
}

/// One straight piece of a conductor's path: its layer, from, to.
type Run = (Layer, (i64, i64), (i64, i64));

/// One conductor's copper as one path from pad to pad: its segments in
/// order, each as (layer, from, to), and its vias. Refused when the copper
/// branches, or does not run from one of the net's two pads to the other.
fn conductor(board: &Board, routed: &Routed, net: &str) -> Result<(Vec<Run>, usize), String> {
    let pads: Vec<&Pad> = board
        .parts
        .iter()
        .flat_map(|p| &p.pads)
        .filter(|p| p.net.as_deref() == Some(net))
        .collect();
    let [first, last] = pads[..] else {
        return Err(format!("`{net}` has {} pads, not two", pads.len()));
    };
    let mut first = first;
    let mut last = last;
    if yx(last) < yx(first) {
        std::mem::swap(&mut first, &mut last);
    }
    let mut left: Vec<&Segment> = routed.segments.iter().filter(|s| s.net == net).collect();
    let vias: BTreeSet<(i64, i64)> = routed
        .vias
        .iter()
        .filter(|v| v.net == net)
        .map(|v| v.at)
        .collect();
    let starts: Vec<usize> = (0..left.len())
        .filter(|&i| {
            let s = left[i];
            s.layer == Layer::F
                && [s.start, s.end]
                    .iter()
                    .any(|&e| first.inner().contains(Rect::point(e.0, e.1)))
        })
        .collect();
    let mut out = Vec::new();
    let (mut layer, mut at) = match starts.as_slice() {
        [] => return Err(format!("`{net}`: no track leaves its first pad")),
        _ => {
            // The track whose end in the pad is the path's start: the one the
            // rest of the path does not continue through.
            let s = left[starts[0]];
            let inside = |e: (i64, i64)| first.inner().contains(Rect::point(e.0, e.1));
            (
                s.layer,
                if inside(s.start) && !inside(s.end) {
                    s.start
                } else if inside(s.end) && !inside(s.start) {
                    s.end
                } else {
                    s.start
                },
            )
        }
    };
    // Walk: the next track on this layer from here, or a via to the other.
    let mut used_vias = 0;
    while !left.is_empty() {
        let here: Vec<usize> = (0..left.len())
            .filter(|&i| left[i].layer == layer && (left[i].start == at || left[i].end == at))
            .collect();
        match here.as_slice() {
            [i] => {
                let s = left.remove(*i);
                let to = if s.start == at { s.end } else { s.start };
                out.push((layer, at, to));
                at = to;
            }
            [] if vias.contains(&at) => {
                layer = layer.other();
                used_vias += 1;
                if !(0..left.len())
                    .any(|i| left[i].layer == layer && (left[i].start == at || left[i].end == at))
                {
                    return Err(format!("`{net}`: its via at {at:?} leads nowhere"));
                }
            }
            [] => {
                return Err(format!(
                    "`{net}`: its copper is not one path (it stops at {at:?})"
                ));
            }
            _ => return Err(format!("`{net}`: its copper branches at {at:?}")),
        }
    }
    // Copper that runs back over itself would count its length twice and
    // hide a length difference (adversarial pass A2): no two pieces of one
    // conductor on one layer share more than a point.
    for (i, &(la, a0, a1)) in out.iter().enumerate() {
        for &(lb, b0, b1) in &out[i + 1..] {
            let (ra, rb) = (span(a0, a1), span(b0, b1));
            let shared = Rect {
                x0: ra.x0.max(rb.x0),
                y0: ra.y0.max(rb.y0),
                x1: ra.x1.min(rb.x1),
                y1: ra.y1.min(rb.y1),
            };
            if la == lb
                && shared.x0 <= shared.x1
                && shared.y0 <= shared.y1
                && shared != Rect::point(shared.x0, shared.y0)
            {
                return Err(format!(
                    "`{net}`: its copper runs over itself from {:?} to {:?}",
                    (shared.x0, shared.y0),
                    (shared.x1, shared.y1)
                ));
            }
        }
    }
    if !last.inner().contains(Rect::point(at.0, at.1)) || layer != Layer::F {
        return Err(format!(
            "`{net}`: its path ends at {at:?}, not in its second pad"
        ));
    }
    if used_vias != vias.len() {
        return Err(format!("`{net}`: a via of it is not on its path"));
    }
    Ok((out, vias.len()))
}

/// The arc-length intervals of `a`'s path that run beside a segment of `b`
/// on the same layer, parallel at exactly `spacing` centre to centre, and
/// the side `b` stands on (+1 right of travel, −1 left), each interval's own.
fn beside(a: &[Run], b: &[Run], spacing: i64) -> Vec<(i64, i64, i64)> {
    let mut out = Vec::new();
    let mut s0 = 0;
    for &(la, p, q) in a {
        let len = (q.0 - p.0).abs() + (q.1 - p.1).abs();
        let horizontal = p.1 == q.1;
        let dir = if horizontal {
            (q.0 - p.0).signum()
        } else {
            (q.1 - p.1).signum()
        };
        for &(lb, u, v) in b {
            if lb != la || (u.1 == v.1) != horizontal || len == 0 {
                continue;
            }
            // Offset across, and the overlap along, in a's frame.
            let (off, lo, hi, base) = if horizontal {
                (u.1 - p.1, u.0.min(v.0), u.0.max(v.0), p.0)
            } else {
                (u.0 - p.0, u.1.min(v.1), u.1.max(v.1), p.1)
            };
            if off.abs() != spacing {
                continue;
            }
            let (t0, t1) = ((lo - base) * dir, (hi - base) * dir);
            let (t0, t1) = (t0.min(t1).max(0), t0.max(t1).min(len));
            if t0 < t1 {
                // Right of travel, Y down: +x has +y on its right; +y has −x.
                let side = if horizontal {
                    off.signum() * dir
                } else {
                    -off.signum() * dir
                };
                out.push((s0 + t0, s0 + t1, side));
            }
        }
        s0 += len;
    }
    out.sort();
    out
}

/// The coupled stretch of a path, given what runs beside it: one interval
/// once each outer corner's `2·spacing` is bridged (where the partner turns
/// `spacing` inside it), or the place it breaks.
fn coupled(
    path: &[Run],
    mut runs: Vec<(i64, i64, i64)>,
    spacing: i64,
) -> Result<(i64, i64), String> {
    let sides: BTreeSet<i64> = runs.iter().map(|r| r.2).collect();
    if sides.len() > 1 {
        return Err("its conductors change sides".to_owned());
    }
    // Each vertex's arc position, for the corner bridge.
    let mut corners = BTreeSet::new();
    let mut s = 0;
    for w in path.windows(2) {
        s += (w[0].2.0 - w[0].1.0).abs() + (w[0].2.1 - w[0].1.1).abs();
        let turns = (w[0].1.1 == w[0].2.1) != (w[1].1.1 == w[1].2.1);
        if turns && w[0].0 == w[1].0 {
            corners.insert(s);
        }
    }
    runs.sort();
    let Some(&(first, mut end, _)) = runs.first() else {
        return Err("no stretch runs coupled".to_owned());
    };
    for &(a, b, _) in &runs[1..] {
        let bridged = a == end + 2 * spacing && corners.contains(&(end + spacing));
        if a > end && !bridged {
            let at = point_at(path, end);
            return Err(format!(
                "the coupled run breaks at {at:?}: the gap is not the pair's there"
            ));
        }
        end = end.max(b);
    }
    Ok((first, end))
}

/// The point at arc length `t` along `path`.
fn point_at(path: &[Run], mut t: i64) -> (i64, i64) {
    for &(_, p, q) in path {
        let len = (q.0 - p.0).abs() + (q.1 - p.1).abs();
        if t <= len {
            return (
                p.0 + (q.0 - p.0).signum() * t,
                p.1 + (q.1 - p.1).signum() * t,
            );
        }
        t -= len;
    }
    path.last().map_or((0, 0), |s| s.2)
}

/// Judges every declared pair on the board's copper alone (§1.3): each
/// conductor one path from its pad to its other pad; both on the same
/// layers with the same number of vias; one coupled run each, its gap exactly
/// the declared one, bridged only at a corner; at most [`PAIR_BREAKOUT_NM`]
/// uncoupled at either end; and a length difference within
/// [`PAIR_SKEW_NM`]. The board's labels are trusted only once
/// [`check`] and [`connectivity`] have passed, as every caller runs them.
pub fn pairs(board: &Board, routed: &Routed) -> Result<Vec<PairReport>, String> {
    let mut out = Vec::new();
    for pair in &board.stack.pairs {
        let name = &pair.name;
        let fail = |why: String| format!("pair `{name}`: {why}");
        let (pos, vp) = conductor(board, routed, &pair.nets[0]).map_err(fail)?;
        let (neg, vn) = conductor(board, routed, &pair.nets[1]).map_err(fail)?;
        let spacing = pair.gap_nm + pair.width_nm;
        let length = |p: &[Run]| {
            p.iter()
                .map(|s| (s.2.0 - s.1.0).abs() + (s.2.1 - s.1.1).abs())
                .sum::<i64>()
        };
        let layers = |p: &[Run]| p.iter().map(|s| s.0.name()).collect::<BTreeSet<_>>();
        let (lp, ln) = (length(&pos), length(&neg));
        if layers(&pos) != layers(&neg) {
            return Err(fail(format!(
                "its conductors run on {:?} and {:?}",
                layers(&pos),
                layers(&neg)
            )));
        }
        if vp != vn {
            return Err(fail(format!(
                "`{}` has {vp} vias and `{}` has {vn}",
                pair.nets[0], pair.nets[1]
            )));
        }
        let (a0, a1) = coupled(&pos, beside(&pos, &neg, spacing), spacing)
            .map_err(|e| fail(format!("`{}`: {e}", pair.nets[0])))?;
        let (b0, b1) = coupled(&neg, beside(&neg, &pos, spacing), spacing)
            .map_err(|e| fail(format!("`{}`: {e}", pair.nets[1])))?;
        let uncoupled = [[a0, lp - a1], [b0, ln - b1]];
        if let Some(u) = uncoupled.iter().flatten().find(|&&u| u > PAIR_BREAKOUT_NM) {
            return Err(fail(format!(
                "a conductor runs {u} nm uncoupled at an end; at most {PAIR_BREAKOUT_NM} nm is allowed"
            )));
        }
        if (lp - ln).abs() > PAIR_SKEW_NM {
            return Err(fail(format!(
                "its conductors are {lp} and {ln} nm long, {} nm apart; at most {PAIR_SKEW_NM} nm is allowed",
                (lp - ln).abs()
            )));
        }
        out.push(PairReport {
            name: name.clone(),
            nets: pair.nets.clone(),
            gap_nm: pair.gap_nm,
            length_nm: [lp, ln],
            vias: [vp, vn],
            uncoupled_nm: uncoupled,
            layers: layers(&pos).into_iter().collect(),
        });
    }
    Ok(out)
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
                layers: [true, pad.drill_nm.is_some()],
            });
        }
    }
    for a in &routed.arrays {
        for &at in &a.at {
            out.push(Item {
                what: format!("thermal via {at:?} under {} pad {}", a.reference, a.pad),
                net: Some(&a.net),
                shape: Shape {
                    rect: Rect::point(at.0, at.1),
                    r: a.via_nm / 2,
                },
                layers: [true, true],
            });
        }
    }
    for s in &routed.segments {
        out.push(Item {
            what: format!("track {:?} {:?}-{:?}", s.layer, s.start, s.end),
            net: Some(&s.net),
            shape: Shape {
                rect: span(s.start, s.end),
                r: board.rule(&s.net).width_nm / 2,
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
                r: board.rule(&v.net).via_nm / 2,
            },
            layers: [true, true],
        });
    }
    out
}

/// Every pair of copper items on a shared layer, of different nets (a pad
/// with no net differs from everything), keeps the declared clearance; every
/// track and via keeps [`EDGE_NM`] from the edge; holes keep
/// [`HOLE_GAP_NM`]; tracks run along an axis; no via touches a pad of any
/// net, and no via's copper is in a part's courtyard, but inside that part's
/// own ring interior (`AET-WO-1307` K1).
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
    // Every item after the pads is a track, a via or a thermal via ([`items`]
    // puts the pads first): chosen by position, never by its text, which
    // starts with a part's reference for a pad (`via1`, review 2, A16).
    let pads: usize = board.parts.iter().map(|p| p.pads.len()).sum();
    for item in &all[pads..] {
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
    // Every drilled hole, edge to edge: vias, thermal vias and through-hole
    // pads (`AET-WO-1309`).
    let holes: Vec<((i64, i64), i64)> = routed
        .vias
        .iter()
        .map(|v| (v.at, board.rule(&v.net).drill_nm))
        .chain(
            routed
                .arrays
                .iter()
                .flat_map(|a| a.at.iter().map(|&at| (at, a.drill_nm))),
        )
        .chain(
            board
                .parts
                .iter()
                .flat_map(|p| &p.pads)
                .filter_map(|p| Some((p.rect.centre(), p.drill_nm?))),
        )
        .collect();
    for (i, &(a, da)) in holes.iter().enumerate() {
        for &(b, db) in &holes[i + 1..] {
            let need = hole_pitch(da, db);
            if gap2(Rect::point(a.0, a.1), Rect::point(b.0, b.1)) < need * need {
                return Err(format!("holes at {a:?} and {b:?} are too close"));
            }
        }
    }
    // Each thermal array exactly as its declared rule places it, and none
    // other; each through-hole pad of a filled net with a declared join.
    // The gate writes each declared reference and pad into a KiCad rule
    // between single quotes: only a literal KiCad reads as written may be
    // named there (the `AET-WO-1308` A1 lesson).
    let literal = |s: &str| {
        !s.is_empty()
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'+' | b'.'))
    };
    let names = board.stack.joins.iter().map(|(r, _)| r).chain(
        board
            .stack
            .thermal
            .iter()
            .flat_map(|t| [&t.reference, &t.pad]),
    );
    for name in names {
        if !literal(name) {
            return Err(format!(
                "`{name}` is named in a gate rule, and is not a plain literal"
            ));
        }
    }
    let want = arrays(board)?;
    if routed.arrays != want {
        return Err(array_disagrees(&want, &routed.arrays));
    }
    for part in &board.parts {
        for pad in &part.pads {
            join_of(board, part, pad)?;
        }
    }
    // Every declared join is a part's on the board, with a through-hole pad
    // (`INV-06`; `AET-WO-1309` adversarial F9).
    for (reference, _) in &board.stack.joins {
        let has = board
            .parts
            .iter()
            .filter(|p| p.reference == *reference)
            .flat_map(|p| &p.pads)
            .any(|p| p.drill_nm.is_some());
        if !has {
            return Err(format!(
                "the specification declares a join for {reference}, and the board has no through-hole pad of {reference}"
            ));
        }
    }
    for part in &board.parts {
        for pad in &part.pads {
            if let Some(v) = routed
                .vias
                .iter()
                .find(|v| touches(pad, v.at, board.rule(&v.net).via_nm / 2))
            {
                return Err(format!(
                    "via {:?} ({}) touches {} pad {}",
                    v.at, v.net, part.reference, pad.number
                ));
            }
        }
        let hole = ring_interior(part);
        if let Some(v) = routed
            .vias
            .iter()
            .find(|v| in_courtyard(part, hole, v.at, board.rule(&v.net).via_nm / 2))
        {
            return Err(format!(
                "via {:?} ({}) is in {}'s courtyard",
                v.at, v.net, part.reference
            ));
        }
    }
    Ok(())
}

/// Why the board's thermal arrays are not the declared ones: the first pad
/// whose array differs, by count, drill, size or sites.
fn array_disagrees(want: &[Array], got: &[Array]) -> String {
    let key = |a: &Array| (a.reference.clone(), a.pad.clone());
    for g in got {
        let Some(w) = want.iter().find(|w| key(w) == key(g)) else {
            return format!(
                "{} pad {} carries {} thermal vias, and the specification declares no array there",
                g.reference,
                g.pad,
                g.at.len()
            );
        };
        let what = format!("{} pad {}'s thermal array", w.reference, w.pad);
        if g.at.len() != w.at.len() {
            return format!(
                "{what} has {} vias; the rule declares {}",
                g.at.len(),
                w.at.len()
            );
        }
        if (g.via_nm, g.drill_nm) != (w.via_nm, w.drill_nm) {
            return format!(
                "{what}: vias {}/{} nm; the rule declares {}/{} nm",
                g.via_nm, g.drill_nm, w.via_nm, w.drill_nm
            );
        }
        if let Some(at) = g.at.iter().find(|at| !w.at.contains(at)) {
            return format!("{what}: a via at {at:?} is not at a site the rule's pitch gives");
        }
        if g.net != w.net {
            return format!("{what} is on {}; its pad is on {}", g.net, w.net);
        }
    }
    let w = want
        .iter()
        .find(|w| !got.iter().any(|g| key(g) == key(w)))
        .map_or_else(String::new, |w| format!("{} pad {}", w.reference, w.pad));
    format!("{w} has no thermal array; the specification declares one")
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
    // Items: every pad, then every thermal via, then tracks, then vias.
    let pads: Vec<&Pad> = board.parts.iter().flat_map(|p| &p.pads).collect();
    let pad_count = pads.len();
    // A track's end that joins nothing, reported after every other judgement.
    let mut dangling = None;
    for (si, s) in routed.segments.iter().enumerate() {
        let a = first_seg + si;
        // Each end must join its net's pad, via or track (`AET-WO-1307`
        // §5.2: a stub that joins nothing is an escape lie).
        let mut end_joined = [false; 2];
        for (j, b) in all.iter().enumerate() {
            if j == a || b.net != Some(s.net.as_str()) || !b.layers[s.layer.index()] {
                continue;
            }
            let mut joined = false;
            for (k, &e) in ends(s).iter().enumerate() {
                let here = if j < pad_count {
                    // A pad: the track ends inside its copper.
                    pads[j].inner().contains(Rect::point(e.0, e.1))
                } else {
                    b.shape.holds(e)
                };
                end_joined[k] |= here;
                joined |= here;
            }
            joined = joined
                || (j >= first_seg && j < first_via && {
                    let o = &routed.segments[j - first_seg];
                    ends(o).iter().any(|&e| all[a].shape.holds(e))
                });
            if joined {
                let (ra, rb) = (find(&mut parent, a), find(&mut parent, j));
                parent[ra] = rb;
                if j >= first_via {
                    via_layers.entry(routed.vias[j - first_via].at).or_default()[s.layer.index()] =
                        true;
                }
            }
        }
        if let Some(k) = end_joined.iter().position(|j| !j)
            && dangling.is_none()
        {
            dangling = Some(format!(
                "{}: track {:?} {:?}-{:?} ends at {:?}, joined to nothing",
                s.net,
                s.layer,
                s.start,
                s.end,
                ends(s)[k]
            ));
        }
    }

    // Each thermal via is joined to the pad it is under; the fill must
    // join it too (below).
    let pad_item = |reference: &str, number: &str| {
        board
            .parts
            .iter()
            .flat_map(|p| p.pads.iter().map(move |pad| (p, pad)))
            .position(|(p, pad)| p.reference == reference && pad.number == number)
    };
    let mut k = pad_count;
    let mut thermal: Vec<(usize, &Array, (i64, i64))> = Vec::new();
    for a in &routed.arrays {
        let pad = pad_item(&a.reference, &a.pad).ok_or_else(|| {
            format!(
                "a thermal array under {} pad {}, which the board does not have",
                a.reference, a.pad
            )
        })?;
        for &at in &a.at {
            let (ra, rb) = (find(&mut parent, k), find(&mut parent, pad));
            parent[ra] = rb;
            thermal.push((k, a, at));
            k += 1;
        }
    }
    let mut thermal_joined = vec![false; thermal.len()];
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
                    r: board.rule(&s.net).width_nm / 2,
                }),
            routed
                .vias
                .iter()
                .filter(|v| v.net != *net)
                .map(|v| Shape {
                    rect: Rect::point(v.at.0, v.at.1),
                    r: board.rule(&v.net).via_nm / 2,
                })
                // Fixed copper on every layer cuts every other net's fill.
                .chain(
                    thermal
                        .iter()
                        .filter(|(_, a, _)| a.net != *net)
                        .map(|&(_, a, at)| Shape {
                            rect: Rect::point(at.0, at.1),
                            r: a.via_nm / 2,
                        }),
                )
                .chain(
                    board
                        .parts
                        .iter()
                        .flat_map(|p| &p.pads)
                        .filter(|p| p.drill_nm.is_some() && p.net.as_deref() != Some(net.as_str()))
                        .map(|p| Shape { rect: p.rect, r: 0 }),
                ),
        );
        // The fill's anchors: this net's thermal vias and its through-hole
        // pads by their declared joins, each where it meets sure fill.
        let mut anchors: Vec<(usize, usize)> = Vec::new();
        for (t, &(item, a, at)) in thermal.iter().enumerate() {
            if a.net == *net {
                let node = nearest(&grid, at).ok_or("a thermal via is off the grid")?;
                anchors.push((item, node));
                thermal_joined[t] = true;
            }
        }
        let mut pi = 0;
        for part in &board.parts {
            for pad in &part.pads {
                if pad.net.as_deref() == Some(net.as_str())
                    && let Some(join) = join_of(board, part, pad)?
                {
                    let node = meets(&grid, &core, pad, join).ok_or_else(|| {
                        format!(
                            "{} pad {} meets the {layer} fill of {net} nowhere it is sure to be filled ({join:?})",
                            part.reference, pad.number
                        )
                    })?;
                    anchors.push((pi, node));
                }
                pi += 1;
            }
        }
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
        if !b_cu && stitched.is_empty() && anchors.is_empty() {
            return Err(format!(
                "the {layer} plane of {net} reaches no via: no pad draws from it"
            ));
        }
        let mut all_seeds = seeds.clone();
        all_seeds.extend(anchors.iter().map(|&(_, n)| n));
        let labels = components(&grid, &core, &all_seeds);
        let main = labels.first().copied().flatten();
        for (k, &(item, node)) in anchors.iter().enumerate() {
            let label = labels[stitched.len() + k];
            if label.is_none() || label != main {
                return Err(format!(
                    "the {layer} fill of {net} does not reach {} at {:?}",
                    all[item].what,
                    grid.at(node)
                ));
            }
            let root = stitched.first().map_or(anchors[0].0, |&i| first_via + i);
            let (ra, rb) = (find(&mut parent, item), find(&mut parent, root));
            parent[ra] = rb;
        }
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
    if let Some(t) = thermal_joined.iter().position(|j| !j) {
        return Err(format!(
            "{} joins no fill of its net",
            all[thermal[t].0].what
        ));
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
    dangling.map_or(Ok(()), Err)
}

// ---- reading a board ----

fn nm_at(item: &strict::Node<'_>, i: usize, what: &str) -> Result<i64, String> {
    strict::number(item, i).ok_or_else(|| format!("{what}: unreadable `{:?}`", item.head()))
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

/// A footprint's rotation as KiCad's file states it (`AET-WO-1308` §1.5):
/// 0, 90, 180 or 270 degrees, positive counter-clockwise on the board (Y
/// down). Measured in the tools image (IPC-D-356 export of a rotated
/// SOIC-8): a pad's local `(x, y)` lands at the origin plus
/// `(x cos θ + y sin θ, −x sin θ + y cos θ)`, and a pad's or a text's own
/// angle in the file is absolute, the footprint's rotation included.
pub fn rotate(theta: u16, (x, y): (i64, i64)) -> (i64, i64) {
    match theta {
        90 => (y, -x),
        180 => (-x, -y),
        270 => (-y, x),
        _ => (x, y),
    }
}

/// A footprint's `(at x y θ)` angle: none or one of the four right angles,
/// written as KiCad writes them; any other is refused, never rounded.
pub fn angle(at: Option<&str>) -> Option<u16> {
    match at {
        None | Some("0") => Some(0),
        Some("90") => Some(90),
        Some("180") => Some(180),
        Some("270") => Some(270),
        Some(_) => None,
    }
}

/// A placed footprint's copper pads, in board coordinates. Only front SMD
/// pads and plated through-hole pads (`AET-WO-1309`: `*.Cu`, one round
/// drill at the pad's centre, the same copper on every layer), at a right
/// angle, rectangular or rounded, on a footprint at a right angle
/// ([`rotate`]), are modelled; anything else is refused, never approximated.
/// A through-hole pad that sets its own plane join, relief or padstack is
/// refused: how it meets a fill is the specification's (FEEDBACK F5).
pub fn part(fp: &strict::Node<'_>) -> Result<Part, String> {
    let reference = fp
        .field("Reference")
        .map_err(|e| e.to_string())?
        .unwrap_or("?")
        .to_owned();
    let at = one(fp, "at")?.ok_or_else(|| format!("{reference}: no `at`"))?;
    let theta =
        angle(at.arg(3)).ok_or_else(|| format!("{reference} is rotated by {:?}", at.arg(3)))?;
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
            .filter(|l| strict::copper_pad_layer(l))
            .collect();
        if copper.is_empty() {
            // A hole with no copper still cuts every layer it passes:
            // not modelled, so refused (`AET-WO-1309` adversarial F1).
            if one(&pad, "drill")?.is_some() {
                return Err(format!("{what} is a drilled hole with no copper"));
            }
            continue;
        }
        let (drill_nm, join) = match (pad.arg(2), copper.as_slice()) {
            (Some("smd"), ["F.Cu"]) if one(&pad, "drill")?.is_none() => (None, None),
            (Some("thru_hole"), ["*.Cu"]) => through_hole(&pad, &what)?,
            _ => {
                return Err(format!(
                    "{what} is neither a front SMD pad nor a plated through-hole pad on *.Cu"
                ));
            }
        };
        let pat = one(&pad, "at")?;
        let angle = pat.as_ref().and_then(|a| a.arg(3)).unwrap_or("0");
        let (px, py) = rotate(theta, xy_of(pat, &what)?);
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
                    .and_then(|r| strict::number(&r, 1))
                    .ok_or_else(|| format!("{what}: no `roundrect_rratio`"))?;
                w.min(h) * ratio.clamp(0, 500_000) / 1_000_000
            }
            // KiCad draws a circle as a disk of its x size, which the
            // stadium below holds only when the size is square, as
            // `extract` requires (review 2, A16).
            Some("circle") if w != h => {
                return Err(format!("{what} is a circle whose size is not square"));
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
            drill_nm,
            join,
        });
        if drill_nm.is_some_and(|d| d >= w.min(h)) {
            return Err(format!("{what}: its drill leaves no copper ring"));
        }
    }
    // The front courtyard, as straight edges; an arc or circle in it is not
    // modelled and is refused, never approximated.
    let mut courtyard = Vec::new();
    let at = |p: (i64, i64)| {
        let (x, y) = rotate(theta, p);
        (origin.0 + x, origin.1 + y)
    };
    for g in fp.lists() {
        let head = g.head().unwrap_or("");
        if !head.starts_with("fp_") || one(&g, "layer")?.and_then(|l| l.arg(1)) != Some("F.CrtYd") {
            continue;
        }
        let what = format!("{reference}'s courtyard {head}");
        match head {
            "fp_line" | "fp_rect" => {
                let a = at(xy_of(one(&g, "start")?, &what)?);
                let b = at(xy_of(one(&g, "end")?, &what)?);
                if head == "fp_line" {
                    courtyard.push((a, b));
                } else {
                    let c = [a, (b.0, a.1), b, (a.0, b.1)];
                    courtyard.extend((0..4).map(|i| (c[i], c[(i + 1) % 4])));
                }
            }
            "fp_poly" => {
                let pts = one(&g, "pts")?.ok_or_else(|| format!("{what}: no `pts`"))?;
                let c = pts
                    .lists()
                    .map(|xy| match xy.head() {
                        Some("xy") => xy_of(Some(xy), &what).map(at),
                        h => Err(format!(
                            "{what}: a `{}` in its points is not modelled",
                            h.unwrap_or("()")
                        )),
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                courtyard.extend((0..c.len()).map(|i| (c[i], c[(i + 1) % c.len()])));
            }
            _ => return Err(format!("{what} is not modelled")),
        }
    }
    // The outline is judged by crossings, so it must be one closed outline
    // with no edge twice: every corner met by an even number of edges, and no
    // edge repeated in either direction.
    let mut corners: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for &(a, b) in &courtyard {
        *corners.entry(a).or_default() += 1;
        *corners.entry(b).or_default() += 1;
        if !seen.insert((a.min(b), a.max(b))) {
            return Err(format!("{reference}'s courtyard has an edge twice"));
        }
    }
    if corners.values().any(|n| n % 2 == 1) {
        return Err(format!("{reference}'s courtyard is not a closed outline"));
    }
    if courtyard.is_empty() && !pads.is_empty() {
        return Err(format!("{reference} has copper pads and no courtyard"));
    }
    Ok(Part {
        reference,
        rotation: theta,
        pads,
        courtyard,
    })
}

/// A plated through-hole pad's drill (one round hole at the pad's centre,
/// with every layer's copper kept) and its `zone_connect`: 2 solid, 1
/// relief, or none. Any token by which the pad would set its own relief,
/// padstack or unused-layer removal is refused.
fn through_hole(
    pad: &strict::Node<'_>,
    what: &str,
) -> Result<(Option<i64>, Option<crate::stack::Join>), String> {
    let join = match one(pad, "zone_connect")?.map(|z| (z.arg(1), z.arg(2))) {
        None => None,
        Some((Some(t), None)) if let Some(j) = crate::stack::Join::of_token(t) => Some(j),
        Some(other) => return Err(format!("{what} has `zone_connect` {other:?}")),
    };
    for token in [
        "thermal_gap",
        "thermal_width",
        "thermal_bridge_width",
        "thermal_bridge_angle",
        "padstack",
        "keep_end_layers",
        "clearance",
        "backdrill",
        "tertiary_drill",
        "front_post_machining",
        "back_post_machining",
    ] {
        if one(pad, token)?.is_some() {
            return Err(format!("{what} sets its own `{token}`"));
        }
    }
    if let Some(r) = one(pad, "remove_unused_layers")?
        && (r.arg(1) != Some("no") || r.arg(2).is_some())
    {
        return Err(format!("{what} removes its unused layers"));
    }
    let drill = one(pad, "drill")?.ok_or_else(|| format!("{what} has no drill"))?;
    if drill.arg(2).is_some() || drill.lists().count() != 0 {
        return Err(format!(
            "{what}: only a round drill at its centre is modelled"
        ));
    }
    let d = nm_at(&drill, 1, what)?;
    if d <= 0 {
        return Err(format!("{what}: drill {d} nm"));
    }
    Ok((Some(d), join))
}

/// True when `zone`'s outline is the board's (`AET-WO-1307` K6, 4100 F7):
/// exactly one `polygon` with one `pts` of exactly four `xy` points, which
/// are `outline`'s corners in any cyclic order and direction. A
/// `filled_polygon` (KiCad's saved fill) is not read.
fn zone_is_outline(zone: &strict::Node<'_>, outline: Rect) -> Result<bool, String> {
    let polygons: Vec<_> = zone.all("polygon").collect();
    let [polygon] = polygons.as_slice() else {
        return Ok(false);
    };
    let lists: Vec<_> = polygon.lists().collect();
    let [pts] = lists.as_slice() else {
        return Ok(false);
    };
    if pts.head() != Some("pts") {
        return Ok(false);
    }
    let points = pts
        .lists()
        .map(|xy| match xy.head() {
            Some("xy") if xy.len() == 3 => xy_of(Some(xy), "a zone").map(Some),
            _ => Ok(None),
        })
        .collect::<Result<Option<Vec<_>>, _>>()?;
    let Some(points) = points else {
        return Ok(false);
    };
    let Rect { x0, y0, x1, y1 } = outline;
    let mut corners = vec![(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
    let mut seen = points.clone();
    corners.sort();
    seen.sort();
    // The four corners, each once, and each step along an edge (never a
    // diagonal): the rectangle, walked either way from any corner.
    Ok(seen == corners
        && x0 != x1
        && y0 != y1
        && (0..4).all(|i| {
            let (p, q) = (points[i], points[(i + 1) % 4]);
            (p.0 == q.0) != (p.1 == q.1)
        }))
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
        let is_copper = even || strict::copper_layer(name);
        // `(id "name" type "user name")`: KiCad then takes the user name for
        // the layer, so an item on `"TOP"` would sit on `F.Cu` unseen. KiCad
        // itself writes one on a non-copper row (`(9 "F.Adhes" user
        // "F.Adhesive")`), so only a copper row's, or a user name that is
        // itself a copper layer's, is refused (`AET-WO-3303` A11).
        let user = row.arg(3);
        if !(3..=4).contains(&row.len())
            || (user.is_some() && is_copper)
            || user.is_some_and(strict::copper_pad_layer)
        {
            return Err(no(
                "LayersDisagree",
                format!(
                    "layer row `{id}` is not an id, a canonical name and a type; a copper row has no user name, and KiCad would take one for the layer",
                ),
            ));
        }
        if is_copper {
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
        .and_then(|t| strict::number(&t, 1));
    // Every declared class's and pair's net is on a pad (`AET-WO-1308`
    // adversarial pass A3): a board that renames one, pads and tracks alike,
    // would otherwise escape its class's rules.
    let pad_nets: BTreeSet<&str> = board
        .all("footprint")
        .flat_map(|fp| fp.all("pad").collect::<Vec<_>>())
        .filter_map(|pad| pad.one("net").ok().flatten().and_then(|n| n.arg(1)))
        .collect();
    let declared_nets = stack
        .classes
        .iter()
        .flat_map(|c| c.nets.iter())
        .chain(stack.pairs.iter().flat_map(|p| p.nets.iter()));
    for net in declared_nets {
        if !pad_nets.contains(net.as_str()) {
            return Err(no(
                "ClassNetAbsent",
                format!(
                    "the specification puts `{net}` in a class or pair, and no pad of the board is on it"
                ),
            ));
        }
    }
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
/// through via; the outline the one `Edge.Cuts` rectangle. Whether the layer
/// table, the zones and the thickness are exactly the declared ones is
/// [`declared`]'s, which every caller runs first, and [`connectivity`] judges
/// the zones again on the copper read here.
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
    // Each item's rule is its net's (`AET-WO-1308`). A label that lies about
    // the net is caught by `check` and `connectivity`, which join copper by
    // geometry; every caller runs them on what this returns.
    let rule_of = |item: &strict::Node<'_>| -> Result<Rule, String> {
        Ok(one(item, "net")?
            .and_then(|n| n.arg(1))
            .map_or(stack.default_rule(), |n| stack.rule(n)))
    };
    let named = |item: &strict::Node<'_>, rule: Rule| -> Result<String, String> {
        let net = one(item, "net")?.and_then(|n| n.arg(1)).unwrap_or("?");
        Ok(match stack.owner(net) {
            Some((kind, name, _)) if rule != stack.default_rule() => {
                format!(" on `{net}`, whose {kind} `{name}` declares")
            }
            _ => String::new(),
        })
    };
    // The pads the specification puts a thermal array under (`AET-WO-1309`):
    // a via whose centre is in one is that array's, judged by its rule in
    // `check`, never by its own size.
    let under: Vec<(&str, &str, Rect)> = stack
        .thermal
        .iter()
        .flat_map(|t| {
            parts
                .iter()
                .filter(move |p| p.reference == t.reference)
                .flat_map(|p| &p.pads)
                .filter(move |p| p.number == t.pad && p.drill_nm.is_none())
                .map(move |p| (t.reference.as_str(), t.pad.as_str(), p.rect))
        })
        .collect();
    let mut routed = Routed::default();
    for s in board.all("segment") {
        let width = one(&s, "width")?
            .map(|w| nm_at(&w, 1, "segment"))
            .transpose()?;
        let rule = rule_of(&s)?;
        if width != Some(rule.width_nm) {
            let class = named(&s, rule)?;
            return Err(if class.is_empty() {
                format!("a segment {width:?} wide")
            } else {
                format!("a segment {width:?} wide{class} {} nm", rule.width_nm)
            });
        }
        routed.segments.push(Segment {
            net: net(&s, "a segment")?,
            layer: layer_of(&s, "a segment")?,
            start: xy_of(one(&s, "start")?, "segment")?,
            end: xy_of(one(&s, "end")?, "segment")?,
        });
    }
    for v in board.all("via") {
        // The engine's via holds these and nothing else: a backdrill or a
        // machining token would cut its hole off a layer the judges join it
        // on (`AET-WO-1309` adversarial F2).
        for child in v.lists() {
            let head = child.head().unwrap_or("");
            if !matches!(head, "at" | "size" | "drill" | "layers" | "net" | "uuid") {
                return Err(format!("a via has `{head}`, which the gate does not model"));
            }
        }
        let size = one(&v, "size")?.map(|w| nm_at(&w, 1, "via")).transpose()?;
        let drill = one(&v, "drill")?.map(|w| nm_at(&w, 1, "via")).transpose()?;
        let layers: Vec<&str> = one(&v, "layers")?
            .map(|l| l.texts().collect())
            .unwrap_or_default();
        let at = xy_of(one(&v, "at")?, "via")?;
        if let Some(&(reference, pad, _)) = under
            .iter()
            .find(|(_, _, r)| r.contains(Rect::point(at.0, at.1)))
        {
            let (Some(size), Some(drill)) = (size, drill) else {
                return Err(format!("a thermal via at {at:?} has no size or drill"));
            };
            if v.arg(1).is_some() || layers != ["F.Cu", "B.Cu"] {
                return Err(format!("a thermal via at {at:?} on {layers:?}"));
            }
            let net = net(&v, "a via")?;
            let i = match routed
                .arrays
                .iter()
                .position(|a| a.reference == reference && a.pad == pad)
            {
                Some(i) => i,
                None => {
                    routed.arrays.push(Array {
                        reference: reference.to_owned(),
                        pad: pad.to_owned(),
                        net: net.clone(),
                        via_nm: size,
                        drill_nm: drill,
                        at: Vec::new(),
                    });
                    routed.arrays.len() - 1
                }
            };
            let a = &mut routed.arrays[i];
            if (a.via_nm, a.drill_nm, a.net.as_str()) != (size, drill, net.as_str()) {
                return Err(format!(
                    "the thermal vias under {reference} pad {pad} differ in size, drill or net"
                ));
            }
            a.at.push(at);
            continue;
        }
        let rule = rule_of(&v)?;
        if v.arg(1).is_some()
            || size != Some(rule.via_nm)
            || drill != Some(rule.drill_nm)
            || layers != ["F.Cu", "B.Cu"]
        {
            let class = named(&v, rule)?;
            return Err(if class.is_empty() {
                format!("a via {size:?}/{drill:?} on {layers:?}")
            } else {
                format!(
                    "a via {size:?}/{drill:?} on {layers:?}{class} {}/{} nm",
                    rule.via_nm, rule.drill_nm
                )
            });
        }
        routed.vias.push(Via {
            net: net(&v, "a via")?,
            at,
        });
    }
    let outline = span(a, b);
    for z in board.all("zone") {
        let layer = one(&z, "layer")?
            .and_then(|l| l.arg(1))
            .map(str::to_owned)
            .ok_or("a zone has no layer")?;
        let name = net(&z, "a zone")?;
        // A fill is solid and as the engine writes it: its judges assume
        // sure, solid copper (`AET-WO-1309` adversarial F3), and a relieved
        // pad's gap and spokes are the zone's (measured, B2).
        for child in z.lists() {
            let head = child.head().unwrap_or("");
            if !matches!(
                head,
                "net"
                    | "layer"
                    | "uuid"
                    | "hatch"
                    | "min_thickness"
                    | "filled_areas_thickness"
                    | "fill"
                    | "polygon"
                    // KiCad's saved fill: not read (the gate refills), as before.
                    | "filled_polygon"
            ) {
                return Err(format!(
                    "zone `{name}` on {layer} has `{head}`, which the gate does not model"
                ));
            }
        }
        let fill = one(&z, "fill")?;
        if let Some(f) = &fill {
            for child in f.lists() {
                let head = child.head().unwrap_or("");
                if !matches!(
                    head,
                    "thermal_gap" | "thermal_bridge_width" | "island_removal_mode"
                ) {
                    return Err(format!(
                        "zone `{name}` on {layer}: its fill has `{head}`, which the gate does not model"
                    ));
                }
            }
        }
        let pinned = |item: Option<strict::Node<'_>>, want: &str| {
            item.and_then(|i| i.arg(1).map(str::to_owned)) == Some(want.to_owned())
        };
        if fill.as_ref().is_some_and(|f| f.arg(1).is_some())
            || !pinned(
                one(&z, "min_thickness")?,
                &crate::sexpr::mm(MIN_THICKNESS_NM),
            )
            || !pinned(
                fill.as_ref()
                    .map(|f| one(f, "island_removal_mode"))
                    .transpose()?
                    .flatten(),
                "0",
            )
        {
            return Err(format!(
                "zone `{name}` on {layer}: its fill is not the engine's solid fill (min_thickness {}, island_removal_mode 0)",
                crate::sexpr::mm(MIN_THICKNESS_NM)
            ));
        }
        let at = |token: &str| -> Result<Option<i64>, String> {
            fill.as_ref()
                .map(|f| one(f, token))
                .transpose()?
                .flatten()
                .map(|t| nm_at(&t, 1, "a zone's fill"))
                .transpose()
        };
        if at("thermal_gap")? != Some(THERMAL_GAP_NM)
            || at("thermal_bridge_width")? != Some(THERMAL_BRIDGE_NM)
        {
            return Err(format!(
                "zone `{name}` on {layer}: its relief gap and spoke are not the engine's {THERMAL_GAP_NM}/{THERMAL_BRIDGE_NM} nm"
            ));
        }
        if !zone_is_outline(&z, outline)? {
            return Err(format!(
                "zone `{name}` on {layer}: its outline is not the board's"
            ));
        }
        routed.zones.push((name, layer));
    }
    routed.segments.sort();
    routed.vias.sort();
    for a in &mut routed.arrays {
        a.at.sort();
    }
    routed.arrays.sort();
    Ok((
        Board {
            outline,
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
            drill_nm: None,
            join: None,
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
            rotation: 0,
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
            courtyard: Vec::new(),
        }
    }

    /// A quad pad ring: `per` pads a side at 0.5 mm, 1.5 mm long, around a
    /// free square `inner` across; `sides` of the four are populated, and
    /// `centre` adds a pad in the middle (an exposed pad).
    fn ring(per: i64, inner: i64, sides: usize, centre: bool) -> Part {
        let mut pads = Vec::new();
        let half = inner / 2;
        let span = (per - 1) * MM / 2;
        for side in 0..sides {
            for k in 0..per {
                let along = -span / 2 + k * MM / 2;
                let (cx, cy) = match side {
                    0 => (-half - 750_000, along),
                    1 => (half + 750_000, along),
                    2 => (along, -half - 750_000),
                    _ => (along, half + 750_000),
                };
                let (w, h) = if side < 2 {
                    (750_000, 150_000)
                } else {
                    (150_000, 750_000)
                };
                pads.push(pad(
                    &format!("{}", pads.len() + 1),
                    None,
                    rect(
                        50 * MM + cx - w,
                        50 * MM + cy - h,
                        50 * MM + cx + w,
                        50 * MM + cy + h,
                    ),
                ));
            }
        }
        if centre {
            pads.push(pad("EP", None, rect(49 * MM, 49 * MM, 51 * MM, 51 * MM)));
        }
        Part {
            rotation: 0,
            reference: "U".into(),
            pads,
            courtyard: Vec::new(),
        }
    }

    /// K1's rule: a part with pads on all four sides and a free interior of
    /// at least 4 mm is the dense part; two sides, an exposed pad or a
    /// smaller interior is not.
    #[test]
    fn the_dense_part_is_chosen_by_rule() {
        let r = ring_interior(&ring(9, 5 * MM, 4, false)).unwrap();
        assert_eq!(r, rect(47_500_000, 47_500_000, 52_500_000, 52_500_000));
        // The threshold is 4 mm, as a literal: an interior of 4 mm qualifies,
        // one of 3.999998 mm does not.
        assert_eq!(
            ring_interior(&ring(9, 4 * MM, 4, false)).map(|r| r.x1 - r.x0),
            Some(4 * MM)
        );
        assert_eq!(ring_interior(&ring(9, 4 * MM - 2, 4, false)), None);
        assert_eq!(ring_interior(&ring(9, 5 * MM, 2, false)), None);
        assert_eq!(ring_interior(&ring(9, 5 * MM, 3, false)), None);
        assert_eq!(ring_interior(&ring(9, 5 * MM, 4, true)), None);
    }

    /// K1, adversarial A1: in a dense part's ring a via touches no pad, its
    /// own net's included (no via in pad); exactly, at the pad's box.
    #[test]
    fn a_via_in_the_ring_may_not_touch_even_its_own_nets_pad() {
        let mut part = ring(9, 5 * MM, 4, false);
        for p in &mut part.pads {
            p.net = Some("/P".into());
        }
        let board = Board {
            outline: rect(0, 0, 100 * MM, 100 * MM),
            parts: vec![part],
            stack: crate::stack::reference(),
        };
        let hole = ring_interior(&board.parts[0]).unwrap();
        let via = |x: i64| Routed {
            vias: vec![Via {
                net: "/P".into(),
                at: (x, 50 * MM),
            }],
            ..Routed::default()
        };
        check(&board, &via(hole.x0 + VIA_NM / 2 + 1)).unwrap();
        let e = check(&board, &via(hole.x0 + VIA_NM / 2)).unwrap_err();
        assert!(e.contains("touches U pad"), "{e}");
        // The router never sites one there: the node 0.25 mm in from the pad
        // is closed to vias, while one clear of every pad is open.
        let r = Router::new(
            &board,
            Grid::new(board.outline).unwrap(),
            vec!["/P".into()],
            &[],
        );
        let node = |x: i64| r.grid.nodes(Rect::point(x, 50 * MM)).next().unwrap();
        assert_eq!(r.via[0][node(hole.x0 + 250_000)], BLOCKED);
        assert_eq!(r.via[0][node(50 * MM)], FREE);
    }

    /// The courtyard predicates are exact: on the outline is under it; a via
    /// tangent to it is clear, 1 nm nearer is not.
    #[test]
    fn the_courtyard_test_is_exact() {
        let sq = |a: i64, b: i64| -> Vec<Edge> {
            let c = [(a, a), (b, a), (b, b), (a, b)];
            (0..4).map(|i| (c[i], c[(i + 1) % 4])).collect()
        };
        let e = sq(0, 10 * MM);
        assert!(under(&e, (5 * MM, 5 * MM)));
        assert!(under(&e, (0, 5 * MM)) && under(&e, (10 * MM, 10 * MM)));
        assert!(!under(&e, (-1, 5 * MM)) && !under(&e, (10 * MM + 1, 0)));
        assert!(!near(&e, (10 * MM + 300_000, 5 * MM), 300_000));
        assert!(near(&e, (10 * MM + 299_999, 5 * MM), 300_000));
        // A corner: the distance is to the point, not the box.
        assert!(!near(&e, (10 * MM + 212_133, 10 * MM + 212_133), 300_000));
        assert!(near(&e, (10 * MM + 212_131, 10 * MM + 212_131), 300_000));
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
                rotation: 0,
                reference: "R1".into(),
                pads: vec![pad("1", Some("/A"), rect(4 * MM, 4 * MM, 5 * MM, 5 * MM))],
                courtyard: Vec::new(),
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
        assert!(ok(&vias(HOLE_GAP_NM + DRILL_NM)).is_ok());
        assert!(ok(&vias(HOLE_GAP_NM + DRILL_NM - 1)).is_err());
        let slanted = Routed {
            segments: vec![seg("/A", Layer::F, (2 * MM, 2 * MM), (3 * MM, 3 * MM))],
            ..Routed::default()
        };
        assert!(ok(&slanted).is_err());
    }

    /// A pair `/P`, `/N` (gap 0.3 mm, width 0.2 mm: 0.5 mm apart) between two
    /// parts at x = 2 mm and 12 mm, on a stack declaring it, with each part's
    /// two pads `/P` at y = 5 mm and `/N` at 5.5 mm (`AET-WO-1308`).
    fn paired(n_far: i64) -> Board {
        let mut stack = crate::stack::reference();
        stack.pairs.push(crate::stack::Pair {
            name: "X".into(),
            nets: ["/P".into(), "/N".into()],
            gap_nm: 300_000,
            width_nm: 200_000,
        });
        let at = |x: i64, y: i64| rect(x - 150_000, y - 150_000, x + 150_000, y + 150_000);
        let part = |reference: &str, x: i64, xn: i64| Part {
            rotation: 0,
            reference: reference.into(),
            pads: vec![
                pad("1", Some("/P"), at(x, 5 * MM)),
                pad("2", Some("/N"), at(xn, 5 * MM + MM / 2)),
            ],
            courtyard: Vec::new(),
        };
        Board {
            outline: rect(0, 0, 20 * MM, 10 * MM),
            parts: vec![part("A", 2 * MM, 2 * MM), part("B", 12 * MM, n_far)],
            stack,
        }
    }

    /// The pair laid straight and coupled, `/P` with `tail` (points after its
    /// first pad) before its coupled run from x = `from`.
    fn pair_copper(tail: &[(i64, i64)], from: i64) -> Routed {
        let mut segments = Vec::new();
        let mut at = (2 * MM, 5 * MM);
        for &q in tail
            .iter()
            .chain([(from, 5 * MM), (12 * MM, 5 * MM)].iter())
        {
            if q != at {
                segments.push(seg("/P", Layer::F, at, q));
            }
            at = q;
        }
        segments.push(seg(
            "/N",
            Layer::F,
            (2 * MM, 5 * MM + MM / 2),
            (12 * MM, 5 * MM + MM / 2),
        ));
        segments.sort();
        Routed {
            segments,
            ..Routed::default()
        }
    }

    /// The judge's two bounds, each alone (`AET-WO-1308` §1.3, F6): a loop
    /// on `/P`'s tail 1.2 mm out leaves 3.4 mm uncoupled (under 4 mm) and a
    /// 2.4 mm difference (over 2 mm); a loop 2 mm out leaves 5 mm uncoupled.
    #[test]
    fn the_pair_judge_holds_its_skew_and_breakout_bounds() {
        let board = paired(12 * MM);
        let ok = pairs(&board, &pair_copper(&[], 2 * MM)).unwrap();
        assert_eq!(ok[0].length_difference_nm(), 0);
        assert_eq!(ok[0].uncoupled_nm, [[0, 0], [0, 0]]);
        let (x, y) = (2 * MM, 5 * MM);
        let skew = pair_copper(
            &[(x, y - 1_200_000), (x + MM, y - 1_200_000), (x + MM, y)],
            x + MM,
        );
        let e = pairs(&board, &skew).unwrap_err();
        assert!(e.contains("12400000 and 10000000 nm long"), "{e}");
        let tail = pair_copper(
            &[(x, y - 2 * MM), (x + MM, y - 2 * MM), (x + MM, y)],
            x + MM,
        );
        let e = pairs(&board, &tail).unwrap_err();
        assert!(e.contains("5000000 nm uncoupled"), "{e}");
    }

    /// A pair whose conductors end on different parts is refused by name
    /// before any search (`AET-WO-1308` §1.3).
    #[test]
    fn a_pair_between_different_parts_is_refused() {
        let mut board = paired(12 * MM);
        board.parts.push(Part {
            rotation: 0,
            reference: "C".into(),
            pads: vec![pad(
                "1",
                Some("/N"),
                rect(15 * MM, 5 * MM, 15 * MM + 300_000, 5 * MM + 300_000),
            )],
            courtyard: Vec::new(),
        });
        board.parts[1].pads.pop();
        let e = route(&board).unwrap_err();
        assert!(
            e.why.contains("pair `X`") && e.why.contains("same two parts"),
            "{e:?}"
        );
    }

    /// Holes keep the gap at each via's own drill (`AET-WO-1308`): a class
    /// via drilled 0.4 mm beside a board via drilled 0.3 mm needs 0.6 mm
    /// centre to centre, not the board's 0.55.
    #[test]
    fn the_check_judges_holes_at_each_vias_own_drill() {
        let mut stack = crate::stack::reference();
        stack.classes.push(crate::stack::Class {
            name: "big".into(),
            rule: Rule {
                width_nm: 200_000,
                via_nm: 800_000,
                drill_nm: 400_000,
            },
            via_declared: true,
            nets: vec!["/P".into()],
        });
        let board = Board {
            outline: rect(0, 0, 10 * MM, 10 * MM),
            parts: vec![],
            stack,
        };
        let vias = |gap: i64| Routed {
            vias: vec![
                Via {
                    net: "/P".into(),
                    at: (5 * MM, 5 * MM),
                },
                Via {
                    net: "/P".into(),
                    at: (5 * MM + gap, 5 * MM),
                },
            ],
            ..Routed::default()
        };
        assert!(check(&board, &vias(400_000 + HOLE_GAP_NM)).is_ok());
        let e = check(&board, &vias(400_000 + HOLE_GAP_NM - 1)).unwrap_err();
        assert!(e.contains("too close"), "{e}");
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

    /// Review 2, A16: a circle whose size is not square is refused, since
    /// KiCad draws a disk of its x size (the stadium would under-size it);
    /// a square circle reads.
    #[test]
    fn a_circle_pad_that_is_not_square_is_refused() {
        let fp = |size: &str| {
            crate::sexpr::parse(&format!(
                "(footprint \"x\" (at 1 2) (property \"Reference\" \"R1\") (pad \"1\" smd circle (at 0 0) (size {size}) (layers \"F.Cu\")) (fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\")))"
            ))
            .unwrap()
        };
        assert!(part_of(&fp("1 1")).is_ok());
        for size in ["1 0.6", "0.6 1"] {
            let e = part_of(&fp(size)).unwrap_err();
            assert!(e.contains("not square"), "{size}: {e}");
        }
    }

    /// Review 2, A16: a part named `via1`, `track_x` or `thermal1` is held to
    /// the clearance rules of a pad, not to the edge rule of a track or via.
    #[test]
    fn a_part_named_like_copper_is_judged_as_a_pad() {
        for reference in ["via1", "track_x", "thermal1"] {
            let mut part = resistor(reference, 0, "/A", "/B");
            // Its pads 0.1 mm inside the left edge: clear of nothing a pad
            // must keep, inside the edge margin a track must keep.
            for p in &mut part.pads {
                p.rect.x0 += 860_000;
                p.rect.x1 += 860_000;
            }
            let board = Board {
                outline: rect(0, 0, 10 * MM, 20 * MM),
                parts: vec![part],
                stack: crate::stack::reference(),
            };
            let e = check(&board, &Routed::default());
            assert!(
                !e.as_ref().is_err_and(|m| m.contains("of the edge")),
                "{reference}: {e:?}"
            );
        }
    }

    /// What the router does not model is refused when read, never guessed.
    #[test]
    fn reading_refuses_what_the_router_does_not_model() {
        let yard = "(fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\"))";
        let fp = |at: &str, pad: &str| {
            crate::sexpr::parse(&format!(
                "(footprint \"x\" (at {at}) (property \"Reference\" \"R1\") {pad} {yard})"
            ))
            .unwrap()
        };
        let smd = "(pad \"1\" smd rect (at 0 0) (size 1 1) (layers \"F.Cu\"))";
        assert!(part_of(&fp("1 2", smd)).is_ok());
        // A right angle is modelled (`AET-WO-1308` §1.5); any other is not.
        assert!(part_of(&fp("1 2 90", smd)).is_ok());
        // A plated through-hole pad on every copper layer is modelled
        // (`AET-WO-1309`), with its drill; one that picks its own join,
        // relief, padstack or layers is not.
        let th = |extra: &str| {
            format!(
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers \"*.Cu\") {extra})"
            )
        };
        let read = part_of(&fp("1 2", &th("(remove_unused_layers no)"))).unwrap();
        assert_eq!(read.pads[0].drill_nm, Some(500_000));
        // KiCad 10.0.7's tokens, measured (FEEDBACK B2): 2 solid, 1 relief.
        use crate::stack::Join;
        for (token, join) in [("2", Join::Solid), ("1", Join::Relief)] {
            let read = part_of(&fp("1 2", &th(&format!("(zone_connect {token})")))).unwrap();
            assert_eq!(read.pads[0].join, Some(join));
        }
        for (pad, want) in [
            (th("(padstack (mode front_inner_back))"), "own `padstack`"),
            (
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (layers \"*.Cu\"))".to_owned(),
                "has no drill",
            ),
            (th("(zone_connect 0)"), "has `zone_connect` (Some(\"0\"), None)"),
            (th("(thermal_bridge_angle 45)"), "own `thermal_bridge_angle`"),
            (th("(remove_unused_layers yes)"), "removes its unused layers"),
            (
                "(pad \"1\" thru_hole oval (at 0 0) (size 1 2) (drill oval 0.5 1) (layers \"*.Cu\"))".to_owned(),
                "only a round drill",
            ),
            (
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5 (offset 0.1 0)) (layers \"*.Cu\"))".to_owned(),
                "only a round drill",
            ),
            (
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 1) (layers \"*.Cu\"))".to_owned(),
                "no copper ring",
            ),
            (
                "(pad \"1\" thru_hole circle (at 0 0) (size 1 1) (drill 0.5) (layers \"F.Cu\" \"B.Cu\"))".to_owned(),
                "plated through-hole pad on *.Cu",
            ),
            (
                "(pad \"\" np_thru_hole circle (at 0 0) (size 1 1) (drill 1) (layers \"*.Cu\"))".to_owned(),
                "plated through-hole pad on *.Cu",
            ),
        ] {
            let e = part_of(&fp("1 2", &pad)).unwrap_err();
            assert!(e.contains(want), "{pad}: {e}");
        }
        for (at, pad, want) in [
            ("1 2 45", smd, "rotated by Some(\"45\")"),
            ("1 2 90.0", smd, "rotated by Some(\"90.0\")"),
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
            // A drill on an SMD pad: its offset would move the copper.
            (
                "1 2",
                "(pad \"1\" smd rect (at 0 0) (size 1 1) (drill 0 (offset 0 2.5)) (layers \"F.Cu\"))",
                "front SMD",
            ),
        ] {
            let e = part_of(&fp(at, pad)).unwrap_err();
            assert!(e.contains(want), "{pad}: {e}");
        }
        // K1's courtyard is read exactly or refused (adversarial A2).
        let with = |yard: &str| {
            crate::sexpr::parse(&format!(
                "(footprint \"x\" (at 1 2) (property \"Reference\" \"R1\") {smd} {yard})"
            ))
            .unwrap()
        };
        let line =
            |a: &str, b: &str| format!("(fp_line (start {a}) (end {b}) (layer \"F.CrtYd\"))");
        let square = [
            line("-1 -1", "1 -1"),
            line("1 -1", "1 1"),
            line("1 1", "-1 1"),
            line("-1 1", "-1 -1"),
        ];
        assert_eq!(part_of(&with(&square.concat())).unwrap().courtyard.len(), 4);
        for (yard, want) in [
            (String::new(), "no courtyard"),
            (square[..3].concat(), "not a closed outline"),
            (format!("{}{}", square.concat(), square[0]), "an edge twice"),
            (
                format!("{}{}", square.concat(), line("1 -1", "-1 -1")),
                "an edge twice",
            ),
            (
                "(fp_poly (pts (xy -1 -1) (arc (start 1 -1) (mid 1.4 0) (end 1 1)) (xy -1 1)) (layer \"F.CrtYd\"))".into(),
                "`arc` in its points",
            ),
            (
                "(fp_circle (center 0 0) (end 1 0) (layer \"F.CrtYd\"))".into(),
                "not modelled",
            ),
        ] {
            let e = part_of(&with(&yard)).unwrap_err();
            assert!(e.contains(want), "{yard}: {e}");
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

    fn two_layer_stack() -> crate::stack::Stack {
        crate::stack::Stack {
            copper: vec!["F.Cu".into(), "B.Cu".into()],
            planes: vec![],
            thickness: crate::stack::Thickness {
                nominal_nm: 1_600_000,
                min_nm: 1_500_000,
                max_nm: 1_700_000,
                source: "t".into(),
            },
            dielectrics: vec![],
            copper_max_nm: 0,
            copper_source: String::new(),
            permittivity_permille: 0,
            permittivity_source: String::new(),
            clearance_nm: 0,
            width_nm: 0,
            classes: vec![],
            pairs: vec![],
            joins: vec![],
            thermal: vec![],
        }
    }

    #[test]
    fn a_layer_row_with_a_user_name_or_a_non_canonical_copper_name_is_refused() {
        let stack = two_layer_stack();
        let judged = |layers: &str| {
            declared(
                &strict::board(&format!("(kicad_pcb (layers {layers}))")).unwrap(),
                &stack,
            )
        };
        let canonical = "(0 \"F.Cu\" signal) (2 \"B.Cu\" signal)";
        // Control: the canonical table passes the layer check (a later one
        // refuses this board, which has no zone).
        let e = judged(canonical).unwrap_err();
        assert_ne!(e.kind, "LayersDisagree", "{}", e.message);
        // KiCad's user name for a layer: items on "TOP" would sit on F.Cu.
        let e = judged("(0 \"F.Cu\" signal \"TOP\") (2 \"B.Cu\" signal)").unwrap_err();
        assert_eq!(e.kind, "LayersDisagree");
        assert!(e.message.contains("layer row `0`"), "{}", e.message);
        // KiCad's own user name on a non-copper row stays (`F.Adhesive`) ...
        let e = judged(&format!("{canonical} (9 \"F.Adhes\" user \"F.Adhesive\")")).unwrap_err();
        assert_ne!(e.kind, "LayersDisagree", "{}", e.message);
        // ... unless it is a copper layer's name, or the row has five values.
        for row in [
            "(9 \"F.Adhes\" user \"F.Cu\")",
            "(9 \"F.Adhes\" user \"*.Cu\")",
            "(9 \"F.Adhes\" user \"a\" \"b\")",
        ] {
            let e = judged(&format!("{canonical} {row}")).unwrap_err();
            assert_eq!(e.kind, "LayersDisagree", "{row}");
        }
        // A copper layer is its canonical name, not `ends_with(".Cu")`.
        let e = judged(&format!("{canonical} (1 \"X.Cu\" user)")).unwrap_err();
        assert_ne!(e.kind, "LayersDisagree", "{}", e.message);
        let e = judged("(0 \"F.Cu\" signal) (1 \"B.Cu\" signal)").unwrap_err();
        assert_eq!(e.kind, "LayersDisagree");
    }
}
