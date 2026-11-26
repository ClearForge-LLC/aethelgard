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
use crate::stack::Rule;

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
/// The most work one routing may do before it is refused by name
/// (`RouterWorkExceeded`, `AET-WO-1323` D1): each state a search expands, and
/// each node a fill flood visits, counts one. So do each source a search
/// seeds, each (pad, tree node) pair the nearest-pad index takes and each
/// pad it picks from, and each pair the final check judges (`AET-WO-1324`):
/// every part of a routing that grows faster than its states. A count, never
/// a clock, so a board's answer cannot depend on the machine. Sized at
/// `1323` §1.1; measured with `1324`'s charges: the heaviest routing any test
/// makes did 177.1 M (d3 shifted, under U1), the heaviest committed
/// specification 136.2 M (`sentinel-02-power`, 38.9 %); `1307` A4's
/// adversary reaches it in under a minute, where it took 203 s to [`RIPUPS`].
pub const WORK: u64 = 350_000_000;

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

/// The half side of the square about a buck's thermal pad where a passing
/// track is priced on F.Cu (`AET-WO-4411` (b)): its area is `ratio_permille`
/// of the copper the buck's screen needs, so the pour can reach it with the
/// block's own pads cut out.
fn pour_half_nm(need_mm2: i64, ratio_permille: i64) -> i64 {
    let side_um = (need_mm2 * ratio_permille * 1_000).isqrt();
    side_um * 1000 / 2
}

/// The pour ladder's steps after step 0, in the order tried: each the
/// square's area about a buck's thermal pad, per mille of the copper its
/// screen needs. Engine constants, as the capacitance ladder's rungs are.
pub const POUR_STEPS: [i64; 4] = [1_000, 1_250, 1_500, 1_750];

/// A track's cost per F.Cu node about a buck's thermal pad, against
/// [`STEP_F`]'s 2 and [`STEP_B`]'s 6: through it is the dearest way across.
const EXIT_COST: u32 = 40;
/// A net is a buck's own when every pad of it lies this near the buck's
/// courtyard: its loop parts' bounds reach 5 mm.
const LOCAL_NM: i64 = 6_000_000;

const STEP_F: u32 = 2;
const STEP_B: u32 = 6;
const BEND: u32 = 1;
const VIA_COST: u32 = 24;

/// One routing of a board, as the capacitance ladder tries it
/// (`AET-WO-1314`, ruling 6982875594 D3): the price of a via on a net with a
/// declared capacitance bound, and whether those nets are routed right
/// after the stitched ones. [`Rung::TODAY`] is the router as it was; a board
/// without `fab_tolerances` is only ever routed at it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rung {
    pub bounded_via_cost: u32,
    pub bounded_first: bool,
    /// The pour ladder's step (`AET-WO-4411` (b), ruled 2026-11-26T00:22Z):
    /// 0 prices nothing, today's routing; step k prices a passing track
    /// about each buck's thermal pad at [`POUR_STEPS`]`[k - 1]`.
    pub pour_step: usize,
}

impl Rung {
    pub const TODAY: Rung = Rung {
        bounded_via_cost: VIA_COST,
        bounded_first: false,
        pour_step: 0,
    };
}

/// The ladder's rungs, in the order tried: today's routing, a bounded net's
/// via at 48, 96 and 320 (2, 4 and 13 times [`VIA_COST`]), then each of
/// those with the bounded nets routed first. Engine constants, like the
/// step costs: measured at `AET-WO-1314`'s stop, where no single rung held
/// Sentinel-03's bounds at every ring and some rung held them at each.
pub const RUNGS: [Rung; 8] = {
    const fn r(bounded_via_cost: u32, bounded_first: bool) -> Rung {
        Rung {
            bounded_via_cost,
            bounded_first,
            pour_step: 0,
        }
    }
    [
        r(VIA_COST, false),
        r(48, false),
        r(96, false),
        r(320, false),
        r(VIA_COST, true),
        r(48, true),
        r(96, true),
        r(320, true),
    ]
};
/// How many via nodes one connection may ban before it gives up a window.
const BANS: usize = 16;
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
    /// Its keepout zones (`AET-WO-4300d2` D2 (a), `RUL-03`): areas where no
    /// copper of the kinds each forbids may be, routed around and judged.
    pub keepouts: Vec<Keepout>,
    /// Its keepout zones on inner copper layers (`AET-WO-4411`: a radio
    /// module's, one per copper layer as ruled D4), each a rectangle with
    /// what it forbids there: vias, and fills. No track runs on an inner
    /// layer here; those layers are planes.
    pub inner_keepouts: Vec<InnerKeepout>,
}

/// A footprint's keepout on an inner copper layer, in board coordinates.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InnerKeepout {
    pub layer: String,
    pub rect: Rect,
    pub vias: bool,
    pub fill: bool,
}

/// A footprint's keepout zone, in board coordinates: a rectangle on one
/// copper layer, with what it forbids there. A vendored footprint's (the
/// ICM-20948's no-copper area under its die) is the only kind modelled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Keepout {
    pub layer: Layer,
    pub rect: Rect,
    pub tracks: bool,
    pub vias: bool,
    pub pads: bool,
    pub fill: bool,
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
    /// Set when the board is refused because a run of a part's pads has no
    /// escape (`AET-WO-1310` D1): the part's reference and the pads.
    pub exhausted: Option<(String, Vec<String>)>,
    /// Set when the routing is refused for its work (`AET-WO-1323` D1): the
    /// bound it passed.
    pub work_bound: Option<u64>,
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
pub(crate) fn gap2(a: Rect, b: Rect) -> i64 {
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
    /// True when the shape's inside meets `area`'s: copper that only touches
    /// its edge does not. A sharp-cornered pad has no rounding (`r` 0), so
    /// its box must overlap the area's; `!clear(…, 0)` held such a pad
    /// inside a keepout to be outside it (`AET-WO-1311` adversarial pass).
    fn enters(self, area: Rect) -> bool {
        if self.r > 0 {
            return gap2(self.rect, area) < self.r * self.r;
        }
        let (a, b) = (self.rect, area);
        a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 && b.y0 < a.y1
    }
    /// True when `p` is in the shape, edge included.
    fn holds(self, p: (i64, i64)) -> bool {
        gap2(self.rect, Rect::point(p.0, p.1)) <= self.r * self.r
    }
}

impl Pad {
    /// The box in which every point is inside the pad's copper, whatever its
    /// rounding: a track that ends here is joined to the pad.
    pub(crate) fn inner(&self) -> Rect {
        self.rect.grow(-self.corner_nm)
    }

    /// The box a track may end in, joined to the pad. It is [`Pad::inner`]
    /// but for a pad that is all rounding, a circle or an oval, whose inner box
    /// is a point or a line (the XT30PW-M's pad 2 is a 3.5 mm circle round a
    /// 1.9 mm hole, `AET-WO-4300d2`; no pad drawn before it is one). There it is
    /// the box inscribed in the rounding: shrunk by `corner × (1 − 1/√2)`,
    /// rounded so the box stays inside the copper.
    pub(crate) fn reach(&self) -> Rect {
        let short = (self.rect.x1 - self.rect.x0).min(self.rect.y1 - self.rect.y0);
        if 2 * self.corner_nm < short {
            return self.inner();
        }
        let c = i128::from(self.corner_nm);
        // ⌊c / √2⌋ ≤ c × 707_106 / 1_000_000 < c / √2.
        let kept = c * 707_106 / 1_000_000;
        let shrink = i64::try_from(c - kept).unwrap_or(self.corner_nm);
        self.rect.grow(-shrink)
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
    /// The routing's work passed its bound ([`WORK`]).
    Work,
}

/// A judged net's junction regions, as the router keeps them apart
/// (`AET-WO-1324`, ruled 2026-11-24; [`crate::stack::JunctionRule`]).
/// `extract` prices each via of the net, each of its through-hole pads and
/// each of its array vias on the cell grid with a junction region: the
/// cells of side [`crate::supply::PITCH_NM`] meeting the open disc of the
/// junction reach about the hole's centre ([`crate::flow::Region::new`]).
/// It refuses two regions that share a cell (`ViaRegionsOverlap`), since
/// its certified bound carries one flow per piece of copper and a region's
/// field is written for one hole. KiCad's DRC passes two such vias, so this
/// is a limit of the supply model, not a fabrication fault: the router
/// keeps the certified model unchanged by routing around it, judging in the
/// same cells from the same reach, never by a disc (two regions 0.707 mm
/// apart meet where twice the reach is 0.698 mm).
struct Junction {
    /// The grid offsets, in pitches, at which two of the net's vias'
    /// regions meet (the offset `(0, 0)`, one via, left out).
    meet: BTreeSet<(i64, i64)>,
    /// The nodes where a via of the net would meet the region of one of its
    /// through-hole pads or array vias.
    fixed: Vec<bool>,
    /// The same with the net's laid vias too: where it may not stand.
    near: Vec<bool>,
}

/// The cell offsets `d` at which a region `b` centred `d` cells from a
/// region `a`'s centre shares a cell with it: every `ca − cb`. Taken row by
/// row: two runs of cells, `a`'s in row `ja` and `b`'s in row `jb`, give
/// exactly the offsets `(lo_a − hi_b ..= hi_a − lo_b, ja − jb)`; their union
/// per row offset is the set, at a cost of runs squared plus its size, not
/// cells squared (`AET-WO-1324` adversarial A2: a 3 mm plating minimum,
/// which `validate` admits, took 6.6 s cell by cell).
fn meeting(a: &crate::flow::Region, b: &crate::flow::Region) -> BTreeSet<(i64, i64)> {
    // each region's runs: (row, first cell, last cell)
    let runs = |r: &crate::flow::Region| {
        let mut cells: Vec<(i64, i64)> = r.cells.iter().map(|&(i, j)| (j, i)).collect();
        cells.sort_unstable();
        let mut out: Vec<(i64, i64, i64)> = Vec::new();
        for (j, i) in cells {
            match out.last_mut() {
                Some((rj, _, hi)) if *rj == j && *hi + 1 == i => *hi = i,
                _ => out.push((j, i, i)),
            }
        }
        out
    };
    let (ra, rb) = (runs(a), runs(b));
    let mut spans: BTreeMap<i64, Vec<(i64, i64)>> = BTreeMap::new();
    for &(ja, lo_a, hi_a) in &ra {
        for &(jb, lo_b, hi_b) in &rb {
            spans
                .entry(ja - jb)
                .or_default()
                .push((lo_a - hi_b, hi_a - lo_b));
        }
    }
    let mut out = BTreeSet::new();
    for (dj, mut s) in spans {
        s.sort_unstable();
        let mut done = i64::MIN;
        for (lo, hi) in s {
            for di in lo.max(done.saturating_add(1))..=hi {
                out.insert((di, dj));
            }
            done = done.max(hi);
        }
    }
    out
}

// A router node is a cell corner of the supply gate's grid.
const _: () = assert!(PITCH_NM % crate::supply::PITCH_NM == 0);

impl Junction {
    /// `net`'s regions, its vias drilled `drill_nm`, under `rule`.
    fn new(
        board: &Board,
        grid: &Grid,
        arrays: &[Array],
        rule: &crate::stack::JunctionRule,
        net: &str,
        drill_nm: i64,
    ) -> Junction {
        use crate::flow::Region;
        let h = crate::supply::PITCH_NM;
        let k = PITCH_NM / h;
        let region = |drill: i64| Region::new(h, rule.reach(drill));
        let own = region(drill_nm);
        let meet = meeting(&own, &own)
            .into_iter()
            .filter(|&(i, j)| i % k == 0 && j % k == 0 && (i, j) != (0, 0))
            .map(|(i, j)| (i / k, j / k))
            .collect();
        let drilled = board
            .parts
            .iter()
            .flat_map(|p| &p.pads)
            .filter(|p| p.net.as_deref() == Some(net))
            .filter_map(|p| {
                let r = p.rect;
                Some((((r.x0 + r.x1) / 2, (r.y0 + r.y1) / 2), p.drill_nm?))
            });
        let arrayed = arrays
            .iter()
            .filter(|a| a.net == net)
            .flat_map(|a| a.at.iter().map(|&at| (at, a.drill_nm)));
        let mut fixed = vec![false; grid.len()];
        let mut by_drill: BTreeMap<i64, BTreeSet<(i64, i64)>> = BTreeMap::new();
        // One off the cell grid `extract` leaves out (its hole kept clear):
        // every point a cell step from it is off the grid too, so it marks
        // no node.
        for (at, drill) in drilled.chain(arrayed) {
            let d = by_drill
                .entry(drill)
                .or_insert_with(|| meeting(&region(drill), &own));
            for &(i, j) in d.iter() {
                let (x, y) = (at.0 + i * h, at.1 + j * h);
                if let Some(node) = grid_node(grid, x, y) {
                    fixed[node] = true;
                }
            }
        }
        Junction {
            meet,
            near: fixed.clone(),
            fixed,
        }
    }
}

/// The node at `(x, y)`, if one stands there.
fn grid_node(grid: &Grid, x: i64, y: i64) -> Option<usize> {
    let (dx, dy) = (x - grid.x0, y - grid.y0);
    if dx % PITCH_NM != 0 || dy % PITCH_NM != 0 {
        return None;
    }
    node_at(grid, dx / PITCH_NM, dy / PITCH_NM)
}

/// Each remaining pad's nearest node of a growing tree, kept as the tree
/// grows (`AET-WO-1324` (2), ruled (i)): the least `(distance, node)` over
/// the tree's nodes, the distance from the pad's centre in x plus y. The
/// tree only grows inside `route_net`, and the least of a pair is the
/// least of its parts, so this is exactly what scanning every remaining
/// pad against the whole tree at every step chose, ties included; at a
/// cost of each new node against each remaining pad instead of every node.
struct Nearest {
    centres: Vec<(i64, i64)>,
    best: Vec<(i64, usize)>,
}

impl Nearest {
    fn new(centres: Vec<(i64, i64)>) -> Nearest {
        let best = vec![(i64::MAX, 0); centres.len()];
        Nearest { centres, best }
    }

    /// Takes `nodes` into the tree; the work, one unit per (pad, node).
    fn add(&mut self, grid: &Grid, nodes: &[usize]) -> u64 {
        for &n in nodes {
            let (x, y) = grid.at(n);
            for (&(cx, cy), b) in self.centres.iter().zip(&mut self.best) {
                let d = ((x - cx).abs() + (y - cy).abs(), n);
                if d < *b {
                    *b = d;
                }
            }
        }
        (nodes.len() * self.centres.len()) as u64
    }

    /// The pad nearest the tree, the first in order on a tie, as its index
    /// and its nearest node; and the work, one unit per pad.
    fn pick(&self) -> (Option<(usize, usize)>, u64) {
        let k = self
            .best
            .iter()
            .enumerate()
            .min_by_key(|&(k, &(d, _))| (d, k))
            .map(|(k, &(_, n))| (k, n));
        (k, self.centres.len() as u64)
    }

    fn remove(&mut self, k: usize) {
        self.centres.remove(k);
        self.best.remove(k);
    }
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
    /// A buck's thermal exits' cost per F.Cu node, and the nets free of it
    /// (`AET-WO-4411` (b)).
    exit: Vec<u32>,
    exit_free: Vec<bool>,
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
    /// By net: laid with no via (`AET-WO-4240c`, an oscillator net).
    via_free: Vec<bool>,
    /// Each net's price for a via (`AET-WO-1314`).
    via_cost: Vec<u32>,
    /// Fixed copper on every layer, with its net: through-hole pads and
    /// thermal-array vias. It cuts every other net's fill.
    cuts: Vec<(Option<usize>, Shape)>,
    /// Per net, the fixed copper its fill must reach.
    anchors: Vec<Vec<Anchor>>,
    /// Per net, whether its copper is a coupled pair's, which no other net
    /// takes up: a pair is laid first and whole.
    fixed: Vec<bool>,
    /// Each pad's off-grid escape (`AET-WO-1310`), by the pad's box.
    escapes: BTreeMap<BoxKey, Escape>,
    /// The first run of pads that has no escape, if any: the board is refused.
    exhausted: Option<(String, Vec<String>)>,
    /// Per net the supply gate judges, its junction regions
    /// (`AET-WO-1324`); `None` for every other net.
    junction: Vec<Option<Junction>>,
    /// The work done so far, and the most this routing may do ([`WORK`]).
    work: u64,
    limit: u64,
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
    let field = field(part)?;
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
    fn new(
        board: &'a Board,
        grid: Grid,
        nets: Vec<String>,
        arrays: &[Array],
        pour_step: usize,
    ) -> Router<'a> {
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
            // A keepout: no track of any net on its layer, and no via, where
            // the copper would reach into it (`AET-WO-4300d2` D2).
            for k_out in &part.keepouts {
                for (k, r) in kinds.iter().enumerate() {
                    if k_out.tracks {
                        for node in grid.nodes(k_out.rect.grow(r.width_nm / 2 + EPS_NM)) {
                            track[k][k_out.layer.index()][node] = BLOCKED;
                        }
                    }
                    if k_out.vias {
                        for node in grid.nodes(k_out.rect.grow(r.via_nm / 2 + EPS_NM)) {
                            via[k][node] = BLOCKED;
                        }
                    }
                }
            }
            // The pad field: on F.Cu, only inside a pad; no via near it;
            // except in a dense part's own ring interior (K1).
            let Some(field) = field(part) else {
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
        // `AET-WO-4411` (b), ruled 2026-11-25T21:17Z: a buck's junction
        // screen reads the F.Cu pour joined to its thermal pad, which tracks
        // crossing near it cut off, so within [`pour_half_nm`] of the pad's
        // centre a track costs [`EXIT_COST`] a node on F.Cu, at the rung's
        // pour step (none at step 0, today's routing), unless its net is
        // the block's own: every pad of it within [`LOCAL_NM`] of the buck's
        // courtyard (its divider, CVCC, CBOOT, inductor).
        let mut exit = vec![0u32; n];
        let mut exit_free = vec![false; count];
        if let Some(&g) = index.get(aethelgard_ir::GND) {
            exit_free[g] = true;
        }
        let ratio = pour_step.checked_sub(1).and_then(|k| POUR_STEPS.get(k));
        for (reference, number, need_mm2) in ratio.map_or(&[][..], |_| &board.stack.exits[..]) {
            let Some(part) = board.parts.iter().find(|p| &p.reference == reference) else {
                continue;
            };
            let (Some(e), Some(yard)) = (
                part.pads
                    .iter()
                    .find(|p| &p.number == number)
                    .map(|p| p.rect),
                part.courtyard
                    .iter()
                    .map(|&(a, b)| span(a, b))
                    .reduce(|a, b| a.union(b)),
            ) else {
                continue;
            };
            let near = yard.grow(LOCAL_NM);
            for (i, name) in nets.iter().enumerate() {
                let mut pads = board
                    .parts
                    .iter()
                    .flat_map(|q| &q.pads)
                    .filter(|p| p.net.as_deref() == Some(name.as_str()))
                    .peekable();
                if pads.peek().is_some() && pads.all(|p| near.contains(p.rect)) {
                    exit_free[i] = true;
                }
            }
            let c = e.centre();
            let half = pour_half_nm(*need_mm2, ratio.copied().unwrap_or(0));
            let pour = Rect {
                x0: c.0 - half,
                y0: c.1 - half,
                x1: c.0 + half,
                y1: c.1 + half,
            };
            for node in grid.nodes(pour) {
                exit[node] = EXIT_COST;
            }
        }
        let (escapes, exhausted) =
            lay_escapes(board, &grid, &nets, &kinds, &kind, &mut track, &mut via);
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
        let junction = nets
            .iter()
            .zip(&rules)
            .map(|(name, r)| {
                let rule = board.stack.junction.as_ref()?;
                rule.nets
                    .contains(name)
                    .then(|| Junction::new(board, &grid, arrays, rule, name, r.drill_nm))
            })
            .collect();
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
            exit,
            exit_free,
            corridors,
            pending_via,
            via_corridors,
            released: vec![false; count],
            history: [vec![0; n], vec![0; n]],
            banned: BTreeSet::new(),
            no_vias: false,
            via_cost: vec![VIA_COST; count],
            via_free: vec![false; count],
            cuts,
            anchors,
            fixed: vec![false; count],
            escapes,
            exhausted,
            junction,
            work: 0,
            limit: WORK,
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

    /// Whether this routing has done more work than it may.
    fn over(&self) -> bool {
        self.work > self.limit
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
        (opens(self.via[self.kind[net]][node], owner(net))
            && !self.junction[net].as_ref().is_some_and(|j| j.near[node]))
            || self.vias.get(&node) == Some(&net)
    }

    fn hard_via_open(&self, node: usize, net: usize) -> bool {
        opens(self.hard_via[self.kind[net]][node], owner(net))
    }

    /// The first pair of `path`'s own new vias closer than the hole gap,
    /// or, on a net the supply gate judges, whose junction regions meet
    /// (`AET-WO-1324`), as the indices of the first's lower state and the
    /// second's upper one. The map keeps vias apart from those already
    /// laid, not within a path.
    fn close_vias(&self, net: usize, path: &[Node]) -> Option<(usize, usize)> {
        let reach = hole_reach(self.rule(net), self.rule(net));
        let meet = self.junction[net].as_ref().map(|j| &j.meet);
        let mut laid: Vec<(usize, (i64, i64))> = Vec::new();
        for (i, w) in path.windows(2).enumerate() {
            let ((la, a), (lb, _)) = (w[0], w[1]);
            if la == lb || self.vias.contains_key(&a) {
                continue;
            }
            let (x, y) = self.grid.at(a);
            if let Some(&(j, _)) = laid.iter().rev().find(|&&(_, (px, py))| {
                gap2(Rect::point(px, py), Rect::point(x, y)) < reach * reach
                    || meet.is_some_and(|m| m.contains(&((x - px) / PITCH_NM, (y - py) / PITCH_NM)))
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
    /// A `soft` search's path is judged as that search judged it, on the
    /// hard maps (pads, fields and the edge alone: `1310` D3's U1,
    /// `AET-WO-1323` D2), since its foreign copper only names who is in the
    /// way. Otherwise the second via's node is banned and `None` returned.
    fn unhop(&mut self, net: usize, path: &mut Vec<Node>, soft: bool) -> Option<()> {
        while let Some((i, j)) = self.close_vias(net, path) {
            let outer = path[i].0;
            // A via between the two: the second does not return to the
            // layer the first left, so the run between cannot be laid flat
            // (`AET-WO-1324` adversarial A1: flattened, it left a layer
            // change no search had judged, a via laid unchecked).
            if path[j].0 != outer {
                self.banned.insert(path[j].1);
                return None;
            }
            // path[i] and path[j] are the hop's ends on the outer layer;
            // path[i + 1..j] is the hop itself, its ends included.
            let flat: Vec<Node> = path[i + 2..j - 1]
                .iter()
                .map(|&(_, n)| (outer, n))
                .collect();
            let open = |l: Layer, n: usize| {
                if soft {
                    self.hard_open(l, n, net)
                } else {
                    self.open(l, n, net)
                }
            };
            if flat.iter().all(|&(l, n)| open(l, n)) {
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
        // An escaped pad is reached only at its escape's end.
        if let Some(e) = self.escapes.get(&key(pad.rect)).filter(|e| e.net == net) {
            return vec![e.end];
        }
        self.grid
            .nodes(pad.reach())
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
                    .nodes(pad.reach())
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
        if self.over() {
            return None;
        }
        // Each source is priced and seeded: a search from a whole tree is
        // work in the tree's size (`AET-WO-1324` (2)).
        self.work += sources.len() as u64;
        if self.over() {
            return None;
        }
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
            self.work += 1;
            if self.over() {
                return None;
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
                    // on F.Cu only: priced on both outer layers, F.Cu is the
                    // cheaper again and the pad's ends close (measured)
                    let corridor = if l == Layer::F {
                        self.pending[m] + if self.exit_free[net] { 0 } else { self.exit[m] }
                    } else {
                        0
                    };
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
            if let Some(extra) = via_price
                .filter(|_| !self.no_vias && !self.via_free[net] && !self.banned.contains(&node))
            {
                let tc = c
                    + self.via_cost[net]
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
        // The fixed pairs whose copper alone stood in a soft path's way
        // (`1308` A4): named if nothing else is found.
        let mut pinned = BTreeSet::new();
        {
            self.banned.clear();
            for (margin, soft) in tries {
                let w = self.window(ends, margin);
                let mut found = self.search(net, sources, goal, h, b_moves, w, soft);
                if self.over() {
                    return Err(Stuck::Work);
                }
                for ban in 0..=BANS {
                    let Some(mut p) = found.take() else { break };
                    if self.unhop(net, &mut p, soft).is_some() {
                        found = Some(p);
                        break;
                    }
                    if ban < BANS {
                        found = self.search(net, sources, goal, h, b_moves, w, soft);
                        if self.over() {
                            return Err(Stuck::Work);
                        }
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
                        pinned.extend(self.pinned(net, &p));
                        continue;
                    }
                    if by.is_empty() {
                        return Err(Stuck::Unroutable(format!("{what}: no free path")));
                    }
                    return Err(Stuck::By(by));
                }
            }
        }
        if pinned.is_empty() {
            return Err(Stuck::Unroutable(what.to_owned()));
        }
        let pairs: Vec<String> = self
            .board
            .stack
            .pairs
            .iter()
            .filter(|p| {
                p.nets
                    .iter()
                    .any(|n| pinned.iter().any(|&i| &self.nets[i] == n))
            })
            .map(|p| format!("`{}` ({})", p.name, p.nets.join(", ")))
            .collect();
        Err(Stuck::Unroutable(format!(
            "{what}: only the fixed copper of the pair {} stands in the way",
            pairs.join(" and the pair ")
        )))
    }

    /// The steps of a soft `path` that were not free: each one's node, the
    /// reach of `net`'s copper there, its layers, and its via's rule when
    /// the step changes layer.
    fn contested(&self, net: usize, path: &[Node]) -> Vec<(usize, i64, [bool; 2], Option<Rule>)> {
        let rule = self.rule(net);
        let mut out = Vec::new();
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
            out.push((at, reach, layers, (la != lb).then_some(rule)));
        }
        out
    }

    /// The nets whose copper stands where a soft `path` went, each step of
    /// it that was not free marked as contested.
    fn blockers(&mut self, net: usize, path: &[Node]) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        for (at, reach, layers, hole) in self.contested(net, path) {
            // Every node where a blocker's track would stand in the way
            // costs more from now on: the contested node's shadow.
            let (x, y) = self.grid.at(at);
            let shadow = reach + self.board.w() / 2;
            out.extend(self.contest(Rect::point(x, y), shadow, reach, layers, net, hole));
        }
        out
    }

    /// The fixed nets (a pair's) whose copper stands where a soft `path`
    /// went: read only, so naming them changes no search.
    fn pinned(&self, net: usize, path: &[Node]) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        for (at, reach, layers, hole) in self.contested(net, path) {
            let (x, y) = self.grid.at(at);
            out.extend(self.near(Rect::point(x, y), reach, layers, net, hole, true));
        }
        out
    }

    /// Raises the history of every node within `shadow` of `here` on
    /// `layers`, and names the nets ([`Router::near`]) that are not fixed.
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
        self.near(here, reach, layers, skip, hole, false)
    }

    /// The nets but `skip`, fixed or not as `fixed` says, whose copper on
    /// `layers` is within `reach` of `here`, or (`hole`) whose via hole is
    /// too near.
    fn near(
        &self,
        here: Rect,
        reach: i64,
        layers: [bool; 2],
        skip: usize,
        hole: Option<Rule>,
        fixed: bool,
    ) -> BTreeSet<usize> {
        let mut out = BTreeSet::new();
        for (other, e) in self.edges.iter().enumerate() {
            let r = reach + self.rule(other).width_nm / 2;
            let near = (0..2).filter(|&l| layers[l]).any(|l| {
                e[l].iter()
                    .any(|&(p, q)| gap2(span(self.grid.at(p), self.grid.at(q)), here) < r * r)
            });
            if other != skip && near && self.fixed[other] == fixed {
                out.insert(other);
            }
        }
        for (&v, &other) in &self.vias {
            let (vx, vy) = self.grid.at(v);
            let d2 = gap2(Rect::point(vx, vy), here);
            let copper = reach + self.rule(other).via_nm / 2;
            let hole = hole.map_or(0, |h| hole_reach(h, self.rule(other)));
            if other != skip
                && self.fixed[other] == fixed
                && (d2 < copper * copper || d2 < hole * hole)
            {
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
        if let Some(j) = &mut self.junction[net] {
            for &(a, b) in &j.meet {
                if let Some(m) = grid_node(&self.grid, x + a * PITCH_NM, y + b * PITCH_NM) {
                    j.near[m] = true;
                }
            }
        }
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
        self.remap();
    }

    /// The routed maps again, from the bare ones and every net's edges and
    /// vias.
    fn remap(&mut self) {
        self.track = self.hard_track.clone();
        self.via = self.hard_via.clone();
        for j in self.junction.iter_mut().flatten() {
            j.near.clone_from(&j.fixed);
        }
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

    /// Takes off every end of `net`'s copper that lies in none of its pads
    /// and is no escape's far end (a node met by one edge, with no via),
    /// until none is left; true if any was. The maps are the caller's to
    /// rebuild ([`Router::remap`]).
    fn prune(&mut self, net: usize) -> bool {
        let reach: Vec<Rect> = self
            .board
            .parts
            .iter()
            .flat_map(|p| &p.pads)
            .filter(|p| p.net.as_deref() == Some(self.nets[net].as_str()))
            .map(|p| p.reach())
            .collect();
        // An escaped pad is joined at its escape's end, off the pad.
        let escaped: BTreeSet<usize> = self
            .escapes
            .values()
            .filter(|e| e.net == net)
            .map(|e| e.end)
            .collect();
        let mut any = false;
        loop {
            let mut cut = false;
            for layer in &mut self.edges[net] {
                let mut degree: BTreeMap<usize, usize> = BTreeMap::new();
                for &(a, b) in layer.iter() {
                    *degree.entry(a).or_default() += 1;
                    *degree.entry(b).or_default() += 1;
                }
                let ends: BTreeSet<usize> = degree
                    .into_iter()
                    .filter(|&(n, d)| {
                        let (x, y) = self.grid.at(n);
                        d == 1
                            && !self.vias.contains_key(&n)
                            && !escaped.contains(&n)
                            && !reach.iter().any(|r| r.contains(Rect::point(x, y)))
                    })
                    .map(|(n, _)| n)
                    .collect();
                if !ends.is_empty() {
                    layer.retain(|(a, b)| !ends.contains(a) && !ends.contains(b));
                    cut = true;
                }
            }
            if !cut {
                return any;
            }
            any = true;
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
        // Each remaining pad's nearest tree node, kept as the tree grows
        // ([`Nearest`]); its work counts (`AET-WO-1324` (2)).
        let mut nearest = Nearest::new(
            remaining
                .iter()
                .map(|&p| self.pad(p).rect.centre())
                .collect(),
        );
        let first: Vec<usize> = tree.iter().map(|&(_, n)| n).collect();
        self.work += nearest.add(&self.grid, &first);
        while !remaining.is_empty() {
            // The pad nearest the tree, then the first in order.
            let nx = self.grid.nx as i64;
            let (picked, work) = nearest.pick();
            self.work += work;
            if self.over() {
                return Err(Stuck::Work);
            }
            let (k, from) = picked.ok_or_else(|| Stuck::Unroutable("no pad left".into()))?;
            nearest.remove(k);
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
            let mut grown = Vec::new();
            let mut grow = |node: Node| {
                if tree.insert(node) {
                    grown.push(node.1);
                }
            };
            for &node in &path {
                grow(node);
            }
            for &(_, n) in &path {
                if self.vias.get(&n) == Some(&net) {
                    grow((Layer::F, n));
                    grow((Layer::B, n));
                }
            }
            for node in states {
                grow(node);
            }
            self.work += nearest.add(&self.grid, &grown);
            if self.over() {
                return Err(Stuck::Work);
            }
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
        let labels = components(&self.grid, &core, &all, &mut self.work);
        if self.over() {
            return Err(Stuck::Work);
        }
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
        let labels = components(&self.grid, &core, &seeds, &mut self.work);
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
                self.work += 1;
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
        // Each escape's own copper, off the grid (`AET-WO-1310`).
        for e in self.escapes.values() {
            for w in e.path.windows(2) {
                out.push(Segment {
                    net: self.nets[e.net].clone(),
                    layer: Layer::F,
                    start: w[0],
                    end: w[1],
                });
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
fn components(grid: &Grid, core: &[bool], seeds: &[usize], work: &mut u64) -> Vec<Option<usize>> {
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
                *work += 1;
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
    route_with(board, Rung::TODAY)
}

/// [`route`] at one rung of the capacitance ladder.
pub fn route_with(board: &Board, rung: Rung) -> Result<Routed, Unroutable> {
    route_measured(board, rung, WORK).map(|(routed, _)| routed)
}

/// [`route_with`] under `limit` units of work in place of [`WORK`], and the
/// work it did: how a test sizes the bound and plants a board one unit over.
pub fn route_measured(board: &Board, rung: Rung, limit: u64) -> Result<(Routed, u64), Unroutable> {
    let fail = |net: &str, why: String| Unroutable {
        net: net.to_owned(),
        why,
        exhausted: None,
        work_bound: None,
    };
    let over = |net: &str, work: u64| Unroutable {
        net: net.to_owned(),
        why: format!(
            "the router's work passed its bound, {limit} search steps and fill nodes, while routing {net} ({work} done)"
        ),
        exhausted: None,
        work_bound: Some(limit),
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
        // the rung's: a bounded net right after the stitched ones
        let first = rung.bounded_first && board.stack.bounded.iter().any(|b| b == name);
        // An oscillator net beside a crystal (`AET-WO-4240c` §1.6) right
        // after the stitched ones; on a board with none, every net ties here.
        let oscillator = board.stack.oscillator.iter().any(|o| o == name);
        (
            !is_stitched(name),
            !oscillator,
            !first,
            list.len(),
            at,
            name.clone(),
        )
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
    let index = |name: &str| nets.iter().position(|n| n == name);
    let mut r = Router::new(board, grid, nets.clone(), &arrays, rung.pour_step);
    r.limit = limit;
    for (cost, net) in r.via_cost.iter_mut().zip(&nets) {
        if board.stack.bounded.iter().any(|b| b == net) {
            *cost = rung.bounded_via_cost;
        }
    }
    // An oscillator net is laid on F.Cu with no via, however often it is
    // routed again (`AET-WO-4240c` §1.6).
    for (free, net) in r.via_free.iter_mut().zip(&nets) {
        *free = board.stack.oscillator.iter().any(|o| o == net);
    }
    if let Some(o) = board.stack.oscillator.iter().find(|o| index(o).is_none()) {
        return Err(fail(o, "an oscillator net has no pad on the board".into()));
    }
    if let Some((reference, pads)) = r.exhausted.clone() {
        return Err(Unroutable {
            net: String::new(),
            why: format!(
                "{reference} pads {} have no escape: no lane on which this run of adjacent connected pads reaches the grid at its width and clearance",
                pads.join(", ")
            ),
            exhausted: Some((reference, pads)),
            work_bound: None,
        });
    }
    // Each declared pair first, whole, on the bare board (`AET-WO-1308`):
    // its copper is fixed, and its nets leave the queue. A board with no
    // pair is routed as before.
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
        if let Err(why) = r.route_pair(pair, (p, n), [&pads[p], &pads[n]]) {
            return Err(if r.over() {
                over(&pair.nets[0], r.work)
            } else {
                fail(&pair.nets[0], why)
            });
        }
        r.fixed[p] = true;
        r.fixed[n] = true;
        paired.extend([p, n]);
    }
    // The pairs' own copper judged now, before any other net is laid
    // (`AET-WO-1323` D4): a pair the judge refuses is refused by its name,
    // never by what a later net or `connectivity` meets first.
    if !board.stack.pairs.is_empty() {
        let laid = Routed {
            segments: r
                .segments()
                .into_iter()
                .filter(|s| board.stack.pairs.iter().any(|p| p.nets.contains(&s.net)))
                .collect(),
            ..Routed::default()
        };
        pairs(board, &laid).map_err(|why| fail("", why))?;
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
                    Err(Stuck::Work) => return Err(over(&nets[net], r.work)),
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
                if r.over() {
                    return Err(over(&nets[g], r.work));
                }
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
    // The final check's pairs are work too (`AET-WO-1324` Q4): it judges
    // every two items, and every two holes, so its cost is charged before
    // it runs and a board too large for it is refused by name.
    r.work += judge_work(board, &routed);
    if r.over() {
        return Err(Unroutable {
            net: String::new(),
            why: format!(
                "the router's work passed its bound, {limit} search steps and fill nodes, at the final check of the routed board ({} done)",
                r.work
            ),
            exhausted: None,
            work_bound: Some(limit),
        });
    }
    // belt-and-suspenders: the grid's margins already keep the copper clear
    // and joined (no tested board reaches these); the exact judges stand
    // behind them, so a router defect fails closed rather than ships.
    check(board, &routed).map_err(|why| fail("", why))?;
    connectivity(board, &routed).map_err(|why| fail("", why))?;
    pairs(board, &routed).map_err(|why| fail("", why))?;
    Ok((routed, r.work))
}

// ---- escapes for pads the grid cannot reach (`AET-WO-1310`) ----

/// A part's pad field: the box around its pads, which no F.Cu track of
/// another part crosses.
fn field(part: &Part) -> Option<Rect> {
    part.pads.iter().map(|p| p.rect).reduce(Rect::union)
}

/// A pad's exact copper: its rounded rectangle, the inner box grown by the
/// rounding radius (a circle or an oval included). `check` judges pads by it:
/// by their boxes, the ICM-20948's corner pads read closer than they are.
fn copper(pad: &Pad) -> Shape {
    let short = (pad.rect.x1 - pad.rect.x0).min(pad.rect.y1 - pad.rect.y0);
    let r = pad.corner_nm.clamp(0, short / 2);
    Shape {
        rect: pad.rect.grow(-r),
        r,
    }
}

/// A part's reference and the pads of a run with no escape.
type Exhausted = (String, Vec<String>);

/// A pad's box as a map key.
type BoxKey = (i64, i64, i64, i64);
fn key(r: Rect) -> BoxKey {
    (r.x0, r.y0, r.x1, r.y1)
}

/// An escape: its net, the grid node it ends at, and its copper as a
/// polyline from the pad's centre. Its corners are off the grid.
#[derive(Clone, Debug)]
struct Escape {
    net: usize,
    end: usize,
    path: Vec<(i64, i64)>,
}

/// Two escapes' lanes are this far apart, centre to centre: the least
/// multiple of the pitch at which a track on one lane's node keeps the
/// grid's reach from the other's copper (`clearance + width + EPS_NM`).
const LANE_NM: i64 = 500_000;
const _: () = assert!(LANE_NM % PITCH_NM == 0 && LANE_NM >= FLOOR_NM + FLOOR_NM + EPS_NM);
/// How far past its field an escape may run before it reaches the grid.
const ESCAPE_STEPS: i64 = 24;
/// How far a reserve or lane is kept clear of its own part's escapes: past
/// the deepest escape by 2 mm, so that none can run beyond its end and jog
/// back across it (adversarial pass, the 0.45 mm boxing). Another part's
/// escapes keep clear of its first [`NEAR_NM`] only: a longer one would sweep
/// across parts it never meets (measured: it refused the power fixture's U5).
const RESERVE_NM: i64 = ESCAPE_STEPS * PITCH_NM + 2_000_000;
const NEAR_NM: i64 = 2_000_000;
/// Pitches between the jogs of two pads of a run that jog the same way.
const JOG_STEP: i64 = 3;
/// How many grid states a pad's hard region may hold and still be sealed.
const SEALED_STATES: usize = 4_000;

/// Which side of `field` the pad meets: 0 left, 1 right, 2 top, 3 bottom;
/// `None` for a corner or inner pad.
fn side_of(field: Rect, pad: &Pad) -> Option<usize> {
    let r = pad.rect;
    let sides = [
        r.x0 == field.x0,
        r.x1 == field.x1,
        r.y0 == field.y0,
        r.y1 == field.y1,
    ];
    match sides {
        // An end pad of a row meets two sides: it leaves along its long axis
        // (adversarial pass: a dual-row part's end pads).
        [h0, h1, v0, v1] if (h0 ^ h1) && (v0 ^ v1) => {
            let (w, h) = (r.x1 - r.x0, r.y1 - r.y0);
            match w.cmp(&h) {
                std::cmp::Ordering::Greater => Some(usize::from(h1)),
                std::cmp::Ordering::Less => Some(2 + usize::from(v1)),
                std::cmp::Ordering::Equal => None,
            }
        }
        _ => (sides.iter().filter(|s| **s).count() == 1).then(|| sides.iter().position(|s| *s))?,
    }
}

/// The point `d` beyond `(x, y)` in `side`'s outward direction.
fn outward(side: usize, (x, y): (i64, i64), d: i64) -> (i64, i64) {
    match side {
        0 => (x - d, y),
        1 => (x + d, y),
        2 => (x, y - d),
        _ => (x, y + d),
    }
}

/// The node at grid indices `(ix, iy)`, if on the grid.
fn node_at(grid: &Grid, ix: i64, iy: i64) -> Option<usize> {
    let (nx, ny) = (i64::try_from(grid.nx).ok()?, i64::try_from(grid.ny).ok()?);
    ((0..nx).contains(&ix) && (0..ny).contains(&iy)).then(|| (iy * nx + ix) as usize)
}

/// Copper or a reserve another escape keeps clear of: its net, its shape,
/// and whether it stands for a grid track (an escape's lane on from its end
/// node, or a reached pad's straight exit), which keeps the grid's own
/// margin, [`EPS_NM`] more than the clearance, so that its nodes stay open.
type Avoid = (usize, Shape, bool);

/// What an escape's copper must keep clear of: every pad of another net (as
/// copper) and every escape or reserve of another net in `avoid`, exactly
/// (a grid track's with the grid's margin); the board edge; and a keepout
/// forbidding F.Cu tracks.
fn clear_of(
    board: &Board,
    net: Option<&str>,
    own: &Pad,
    avoid: &[Avoid],
    me: usize,
    shape: Shape,
) -> bool {
    let s = board.s();
    let edge = board.outline.grow(-EDGE_NM);
    board
        .parts
        .iter()
        .flat_map(|q| &q.pads)
        .all(|p| (p.net.is_some() && p.net.as_deref() == net) || copper(p).clear(shape, s))
        // No F.Cu track crosses another part's pad field (the module's rule).
        && board.parts.iter().all(|q| {
            q.pads.iter().any(|p| p.rect == own.rect)
                || field(q).is_none_or(|f| {
                    Shape { rect: f, r: 0 }.clear(shape, 0)
                })
        })
        && avoid
            .iter()
            .all(|&(n, o, grid)| n == me || o.clear(shape, if grid { s + EPS_NM } else { s }))
        && edge.contains(shape.rect.grow(shape.r))
        && board.parts.iter().flat_map(|q| &q.keepouts).all(|k| {
            !(k.tracks && k.layer == Layer::F) || Shape { rect: k.rect, r: 0 }.clear(shape, 0)
        })
}

/// Lays every escape before any net is routed, claiming its copper in the
/// maps. Returns them, and the first run of pads with none.
///
/// - **A** (the work order's escape): a routed SMD pad on one side of its
///   part's pad field, with no grid terminal, leaves from its centre along
///   its own axis, across the field's edge, to a grid row (or column), then
///   jogs across to its **lane**. The adjacent such pads of one side form a
///   run, and a run's lanes are [`LANE_NM`] apart, its base chosen to keep
///   the largest jog least; pads jogging the same way jog outer first,
///   [`JOG_STEP`] pitches deeper each. A run is laid whole, or refused.
/// - **A′** (D1): a routed SMD pad whose hard region (pads, fields,
///   courtyards, the edge) is sealed, reaching no via site and no other
///   terminal of its net, leaves along its axis to the centreline of the
///   channel in front of it, down the channel to a grid line, and onto the
///   grid outside the seal.
///
/// Both apply only where today's grid has no way out, so a board that
/// routes without them is routed exactly as before. Parts, sides, runs and
/// pads go in the order of their geometry, never their names.
fn lay_escapes(
    board: &Board,
    grid: &Grid,
    nets: &[String],
    kinds: &[Rule],
    kind: &[usize],
    track: &mut [[Vec<u32>; 2]],
    via: &mut [Vec<u32>],
) -> (BTreeMap<BoxKey, Escape>, Option<Exhausted>) {
    let counts = net_pads(board);
    let index: BTreeMap<&str, usize> = nets
        .iter()
        .enumerate()
        .map(|(i, n)| (n.as_str(), i))
        .collect();
    let thermal = |part: &Part, pad: &Pad| {
        board
            .stack
            .thermal
            .iter()
            .any(|t| t.reference == part.reference && t.pad == pad.number)
    };
    // A pad the router must join by F.Cu: an SMD pad of a net it routes.
    let routed = |part: &Part, pad: &Pad| -> Option<usize> {
        let name = pad.net.as_deref()?;
        (pad.drill_nm.is_none()
            && !thermal(part, pad)
            && !name.starts_with(crate::parity::UNCONNECTED)
            && counts.get(name).map_or(0, Vec::len) >= 2)
            .then(|| index.get(name).copied())?
    };
    let open = |track: &[[Vec<u32>; 2]], net: usize, node: usize| {
        opens(track[kind[net]][0][node], owner(net))
    };
    let claim_path =
        |track: &mut [[Vec<u32>; 2]], via: &mut [Vec<u32>], net: usize, path: &[(i64, i64)]| {
            for w in path.windows(2) {
                let shape = Shape {
                    rect: span(w[0], w[1]),
                    r: kinds[kind[net]].width_nm / 2,
                };
                for (k, &r) in kinds.iter().enumerate() {
                    claim(
                        &mut track[k][0],
                        grid,
                        shape,
                        board.track_reach(r),
                        owner(net),
                    );
                    claim(&mut via[k], grid, shape, board.via_reach(r), owner(net));
                }
            }
        };
    let mut escapes = BTreeMap::new();
    let mut exhausted = None;
    // Copper and reserves an escape keeps clear of: the part they belong to,
    // and each as its own part's escapes see it and as another part's do.
    let mut laid: Vec<(&str, Avoid, Avoid)> = Vec::new();
    let line = |net: usize, at: (i64, i64), side: usize, len: i64| Shape {
        rect: span(at, outward(side, at, len)),
        r: kinds[kind[net]].width_nm / 2,
    };
    let seen_by = |laid: &[(&str, Avoid, Avoid)], me: &str| -> Vec<Avoid> {
        laid.iter()
            .map(|&(q, own, other)| if q == me { own } else { other })
            .collect()
    };
    let mut parts: Vec<&Part> = board.parts.iter().filter(|p| !p.pads.is_empty()).collect();
    let field_of = |p: &Part| field(p).unwrap_or(Rect::point(0, 0));
    parts.sort_by_key(|p| {
        let f = field_of(p);
        (f.y0, f.x0, f.y1, f.x1)
    });
    for part in &parts {
        let field = field_of(part);
        // Candidates: by side, along it. A pad the grid reaches keeps its
        // straight exit clear of every escape.
        let mut sides: [Vec<(i64, &Pad, usize)>; 4] = Default::default();
        for pad in &part.pads {
            let Some(net) = routed(part, pad) else {
                continue;
            };
            let reached: Vec<usize> = grid
                .nodes(pad.reach())
                .filter(|&n| open(track, net, n))
                .collect();
            let side = side_of(field, pad);
            if !reached.is_empty() {
                // A pad the grid reaches leaves straight out along the grid
                // line of its terminal nearest its centre: that track is
                // kept clear, by the grid's margin, of every escape.
                if let Some(side) = side {
                    let (cx, cy) = pad.rect.centre();
                    let near = reached.iter().map(|&n| grid.at(n)).min_by_key(|&(x, y)| {
                        let d = if side < 2 {
                            (y - cy).abs()
                        } else {
                            (x - cx).abs()
                        };
                        (d, x, y)
                    });
                    if let Some(at) = near {
                        laid.push((
                            part.reference.as_str(),
                            (net, line(net, at, side, RESERVE_NM), true),
                            (net, line(net, at, side, NEAR_NM), true),
                        ));
                    }
                }
                continue;
            }
            let Some(side) = side else { continue };
            let (cx, cy) = pad.rect.centre();
            sides[side].push((if side < 2 { cy } else { cx }, pad, net));
        }
        // Every candidate's straight centreline is reserved until it is laid.
        let mut pending: Vec<(Rect, usize, Shape)> = Vec::new();
        for (side, list) in sides.iter().enumerate() {
            for &(_, pad, net) in list {
                let c = pad.rect.centre();
                pending.push((
                    pad.rect,
                    net,
                    Shape {
                        rect: span(
                            c,
                            outward(
                                side,
                                c,
                                RESERVE_NM
                                    + (pad.rect.x1 - pad.rect.x0).max(pad.rect.y1 - pad.rect.y0),
                            ),
                        ),
                        r: kinds[kind[net]].width_nm / 2,
                    },
                ));
            }
        }
        for (side, list) in sides.iter_mut().enumerate() {
            list.sort_by_key(|&(c, _, _)| c);
            let origin = if side < 2 { grid.y0 } else { grid.x0 };
            let mut start = 0;
            while start < list.len() {
                let mut end = start + 1;
                while end < list.len() && list[end].0 - list[end - 1].0 < LANE_NM {
                    end += 1;
                }
                let run = &list[start..end];
                start = end;
                let c0 = run[0].0;
                let g0 = origin + (c0 - origin).div_euclid(PITCH_NM) * PITCH_NM;
                let mut bases: Vec<i64> = (-6..=6).map(|j| g0 + j * PITCH_NM).collect();
                let worst = |l0: i64| {
                    run.iter()
                        .enumerate()
                        .map(|(i, &(c, _, _))| (l0 + i as i64 * LANE_NM - c).abs())
                        .max()
                        .unwrap_or(0)
                };
                bases.sort_by_key(|&l0| (worst(l0), (l0 - c0).abs(), l0));
                let mut done = None;
                'base: for l0 in bases {
                    // The longest jogs go first, shallowest; each shorter one
                    // JOG_STEP pitches deeper. A pad jogs away from the
                    // neighbours still running beside it, so its end node
                    // and lane stay a grid track's reach from them; the run's
                    // least jog, by its pivot, goes last and deepest.
                    let lanes: Vec<i64> = (0..run.len()).map(|i| l0 + i as i64 * LANE_NM).collect();
                    let mut by_jog: Vec<(i64, usize)> = (0..run.len())
                        .map(|i| (-(lanes[i] - run[i].0).abs(), i))
                        .collect();
                    by_jog.sort();
                    let mut order: Vec<(i64, usize)> = Vec::new();
                    let mut rank = 0;
                    for (k, &(jog, i)) in by_jog.iter().enumerate() {
                        if k > 0 && by_jog[k - 1].0 != jog {
                            rank += 1;
                        }
                        order.push((rank, i));
                    }
                    let mut trial = laid.clone();
                    let mut got = Vec::new();
                    for &(rank, i) in &order {
                        let (_, pad, net) = run[i];
                        let mut avoid = seen_by(&trial, &part.reference);
                        avoid.extend(
                            pending
                                .iter()
                                .filter(|(r, _, _)| {
                                    *r != pad.rect
                                        && !got.iter().any(|(q, _): &(Rect, Escape)| q == r)
                                })
                                .map(|&(_, n, sh)| (n, sh, false)),
                        );
                        let Some(e) = escape(
                            board,
                            grid,
                            &track[kind[net]][0],
                            pad,
                            side,
                            net,
                            kinds[kind[net]],
                            lanes[i],
                            JOG_STEP * rank,
                            &avoid,
                        ) else {
                            continue 'base;
                        };
                        let half = kinds[kind[net]].width_nm / 2;
                        for w in e.path.windows(2) {
                            let copper = (
                                net,
                                Shape {
                                    rect: span(w[0], w[1]),
                                    r: half,
                                },
                                false,
                            );
                            trial.push((part.reference.as_str(), copper, copper));
                        }
                        let at = grid.at(e.end);
                        trial.push((
                            part.reference.as_str(),
                            (net, line(net, at, side, RESERVE_NM), true),
                            (net, line(net, at, side, NEAR_NM), true),
                        ));
                        got.push((pad.rect, e));
                    }
                    done = Some((trial, got));
                    break;
                }
                let Some((trial, got)) = done else {
                    // A run of two or more is refused by name; a lone pad with
                    // no way out is left to the router, which names it as it
                    // does today ("has no node clear").
                    if exhausted.is_none() && run.len() > 1 {
                        exhausted = Some((
                            part.reference.clone(),
                            run.iter().map(|(_, p, _)| p.number.clone()).collect(),
                        ));
                    }
                    continue;
                };
                laid = trial;
                for (rect, e) in got {
                    claim_path(track, via, e.net, &e.path);
                    pending.retain(|(r, _, _)| *r != rect);
                    escapes.insert(key(rect), e);
                }
            }
        }
    }
    // A′: sealed pads, in the order of their centres.
    let mut terminals_of: BTreeMap<usize, BTreeMap<usize, Rect>> = BTreeMap::new();
    // A′'s candidates: (centre, pad, net, side, its part's reference).
    type Candidate<'b> = ((i64, i64), &'b Pad, usize, usize, &'b str);
    let mut sealed_pads: Vec<Candidate<'_>> = Vec::new();
    for part in &board.parts {
        let field = field_of(part);
        for pad in &part.pads {
            // Every pad of a net counts as a way to it, a drilled or thermal
            // one included (adversarial pass: a through-hole pad in the pocket),
            // and an escaped pad by its escape's end.
            if let Some(net) = pad.net.as_deref().and_then(|n| index.get(n).copied()) {
                for n in grid.nodes(pad.reach()).filter(|&n| open(track, net, n)) {
                    terminals_of.entry(net).or_default().insert(n, pad.rect);
                }
                if let Some(e) = escapes.get(&key(pad.rect)) {
                    terminals_of.entry(net).or_default().insert(e.end, pad.rect);
                }
            }
            let Some(net) = routed(part, pad) else {
                continue;
            };
            if let (false, Some(side)) = (escapes.contains_key(&key(pad.rect)), side_of(field, pad))
            {
                sealed_pads.push((yx(pad), pad, net, side, part.reference.as_str()));
            }
        }
    }
    sealed_pads.sort_by_key(|&(at, pad, _, _, _)| (at, key(pad.rect)));
    for (_, pad, net, side, reference) in sealed_pads {
        let k = kind[net];
        let starts: Vec<usize> = grid
            .nodes(pad.reach())
            .filter(|&n| open(track, net, n))
            .collect();
        let others = |n: usize| {
            terminals_of
                .get(&net)
                .and_then(|m| m.get(&n))
                .is_some_and(|r| *r != pad.rect)
        };
        let is_sealed = |track: &[[Vec<u32>; 2]], via: &[Vec<u32>], from: &[usize]| {
            sealed(grid, &track[k][0], &via[k], owner(net), from, &others)
        };
        if starts.is_empty() || !is_sealed(track, via, &starts) {
            continue;
        }
        let out = |n: usize| !is_sealed(track, via, &[n]);
        if let Some(e) = channel(
            board,
            grid,
            &track[k][0],
            pad,
            side,
            net,
            kinds[k],
            &seen_by(&laid, reference),
            &out,
        ) {
            let half = kinds[k].width_nm / 2;
            for w in e.path.windows(2) {
                let copper = (
                    net,
                    Shape {
                        rect: span(w[0], w[1]),
                        r: half,
                    },
                    false,
                );
                laid.push((reference, copper, copper));
            }
            claim_path(track, via, net, &e.path);
            escapes.insert(key(pad.rect), e);
        }
    }
    (escapes, exhausted)
}

/// One pad's escape (A): from its centre along its side's outward axis to a
/// grid line at least `depth` pitches past the field, then across to `lane`.
/// The shortest that keeps exact clearance from `avoid` and every foreign
/// pad, whose end node is open, and whose lane on from there keeps the
/// grid's reach from `avoid`.
#[allow(clippy::too_many_arguments)]
fn escape(
    board: &Board,
    grid: &Grid,
    map: &[u32],
    pad: &Pad,
    side: usize,
    net: usize,
    rule: Rule,
    lane: i64,
    depth: i64,
    avoid: &[Avoid],
) -> Option<Escape> {
    let half = rule.width_nm / 2;
    let me = pad.net.as_deref();
    let (cx, cy) = pad.rect.centre();
    let horizontal = side < 2;
    let field_edge = match side {
        0 => pad.rect.x0,
        1 => pad.rect.x1,
        2 => pad.rect.y0,
        _ => pad.rect.y1,
    };
    let (o_along, o_cross) = if horizontal {
        (grid.x0, grid.y0)
    } else {
        (grid.y0, grid.x0)
    };
    let ic = (lane - o_cross).div_euclid(PITCH_NM);
    if o_cross + ic * PITCH_NM != lane {
        return None;
    }
    // The first grid line strictly past the field's edge, outward.
    let first = if side % 2 == 1 {
        (field_edge - o_along).div_euclid(PITCH_NM) + 1
    } else {
        (field_edge - 1 - o_along).div_euclid(PITCH_NM)
    };
    for step in depth..ESCAPE_STEPS {
        let ia = if side % 2 == 1 {
            first + step
        } else {
            first - step
        };
        let node = if horizontal {
            node_at(grid, ia, ic)?
        } else {
            node_at(grid, ic, ia)?
        };
        if !opens(map[node], owner(net)) {
            continue;
        }
        let at = grid.at(node);
        let bend = if horizontal { (at.0, cy) } else { (cx, at.1) };
        let mut path = vec![(cx, cy), bend];
        if bend != at {
            path.push(at);
        }
        let lane_on = Shape {
            rect: span(at, outward(side, at, RESERVE_NM)),
            r: half,
        };
        if path.windows(2).all(|w| {
            clear_of(
                board,
                me,
                pad,
                avoid,
                net,
                Shape {
                    rect: span(w[0], w[1]),
                    r: half,
                },
            )
        }) && avoid
            .iter()
            .all(|&(n, o, _)| n == net || o.clear(lane_on, board.s() + EPS_NM))
        {
            return Some(Escape {
                net,
                end: node,
                path,
            });
        }
    }
    None
}

/// Whether the hard region round `starts` on F.Cu is sealed: it reaches no
/// via site and no node `other` names within [`SEALED_STATES`] states.
fn sealed(
    grid: &Grid,
    track: &[u32],
    via: &[u32],
    who: u32,
    starts: &[usize],
    other: &dyn Fn(usize) -> bool,
) -> bool {
    let mut seen: BTreeSet<usize> = starts.iter().copied().collect();
    let mut queue: VecDeque<usize> = starts.iter().copied().collect();
    while let Some(n) = queue.pop_front() {
        if opens(via[n], who) || other(n) || seen.len() > SEALED_STATES {
            return false;
        }
        for d in 0..4 {
            if let Some(m) = grid.step(n, d)
                && opens(track[m], who)
                && seen.insert(m)
            {
                queue.push_back(m);
            }
        }
    }
    true
}

/// A sealed pad's escape (A′): from its centre outward along its side's axis
/// (never across its own part's field) to the centreline of the channel
/// between it and the nearest foreign copper ahead,
/// down the channel to a grid line, and across to a node `out` accepts. The
/// shortest that keeps exact clearance.
#[allow(clippy::too_many_arguments)]
fn channel(
    board: &Board,
    grid: &Grid,
    map: &[u32],
    pad: &Pad,
    side: usize,
    net: usize,
    rule: Rule,
    avoid: &[Avoid],
    out: &dyn Fn(usize) -> bool,
) -> Option<Escape> {
    let half = rule.width_nm / 2;
    let s = board.s();
    let me = pad.net.as_deref();
    let (cx, cy) = pad.rect.centre();
    let mut best: Option<(i64, Escape)> = None;
    {
        let horizontal = side < 2;
        let sign = if side % 2 == 1 { 1 } else { -1 };
        let edge = match side {
            0 => pad.rect.x0,
            1 => pad.rect.x1,
            2 => pad.rect.y0,
            _ => pad.rect.y1,
        };
        let along_c = if horizontal { cy } else { cx };
        // The nearest foreign copper straight ahead, within a track's reach
        // of the axis.
        let wall = board
            .parts
            .iter()
            .flat_map(|q| &q.pads)
            .filter(|p| !(p.net.is_some() && p.net.as_deref() == me))
            .filter_map(|p| {
                let c = copper(p);
                let (lo, hi, near) = if horizontal {
                    (
                        c.rect.y0 - c.r,
                        c.rect.y1 + c.r,
                        if sign > 0 {
                            c.rect.x0 - c.r
                        } else {
                            c.rect.x1 + c.r
                        },
                    )
                } else {
                    (
                        c.rect.x0 - c.r,
                        c.rect.x1 + c.r,
                        if sign > 0 {
                            c.rect.y0 - c.r
                        } else {
                            c.rect.y1 + c.r
                        },
                    )
                };
                (lo - half - s < along_c && along_c < hi + half + s && (near - edge) * sign > 0)
                    .then_some(near)
            })
            .min_by_key(|&near| (near - edge) * sign)?;
        let a = (edge + wall) / 2;
        let (o_along, o_cross, n_cross) = if horizontal {
            (grid.y0, grid.x0, grid.nx)
        } else {
            (grid.x0, grid.y0, grid.ny)
        };
        let base = (along_c - o_along).div_euclid(PITCH_NM);
        for way in [1i64, -1] {
            for step in 1..ESCAPE_STEPS {
                let it = if way > 0 {
                    base + step
                } else {
                    base + 1 - step
                };
                let t = o_along + it * PITCH_NM;
                let lo = (a - o_cross).div_euclid(PITCH_NM);
                for ic in [lo, lo + 1] {
                    if !(0..n_cross as i64).contains(&ic) {
                        continue;
                    }
                    let g = o_cross + ic * PITCH_NM;
                    let (bend, mid, at, node) = if horizontal {
                        ((a, cy), (a, t), (g, t), node_at(grid, ic, it))
                    } else {
                        ((cx, a), (t, a), (t, g), node_at(grid, it, ic))
                    };
                    let Some(node) = node else { continue };
                    if !opens(map[node], owner(net)) || !out(node) {
                        continue;
                    }
                    let mut path = vec![(cx, cy), bend, mid, at];
                    path.dedup();
                    if path.windows(2).all(|w| {
                        clear_of(
                            board,
                            me,
                            pad,
                            avoid,
                            net,
                            Shape {
                                rect: span(w[0], w[1]),
                                r: half,
                            },
                        )
                    }) {
                        let len: i64 = path
                            .windows(2)
                            .map(|w| (w[1].0 - w[0].0).abs() + (w[1].1 - w[0].1).abs())
                            .sum();
                        if best.as_ref().is_none_or(|(l, _)| len < *l) {
                            best = Some((
                                len,
                                Escape {
                                    net,
                                    end: node,
                                    path,
                                },
                            ));
                        }
                    }
                }
            }
        }
    }
    best.map(|(_, e)| e)
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
            // A breakout may meet its own run past the run's first node and
            // walk back along it to reach it (`1311` F2): the run's head is
            // then a stub. Each conductor's copper loses every such end.
            let cut_p = self.prune(p);
            let cut_n = self.prune(n);
            if cut_p || cut_n {
                self.remap();
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
                    .any(|&e| first.reach().contains(Rect::point(e.0, e.1)))
        })
        .collect();
    let mut out = Vec::new();
    let (mut layer, mut at) = match starts.as_slice() {
        [] => return Err(format!("`{net}`: no track leaves its first pad")),
        _ => {
            // The track whose end in the pad is the path's start: the one the
            // rest of the path does not continue through.
            let s = left[starts[0]];
            let inside = |e: (i64, i64)| first.reach().contains(Rect::point(e.0, e.1));
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
    if !last.reach().contains(Rect::point(at.0, at.1)) || layer != Layer::F {
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

/// One copper item: what it is, its shape, net and layers.
struct Item<'a> {
    what: String,
    kind: Copper<'a>,
    net: Option<&'a str>,
    shape: Shape,
    layers: [bool; 2],
}

/// What kind of copper an [`Item`] is, never read from its text, which for
/// a pad starts with its part's reference (`via1`, review 2, A16).
#[derive(Clone, Copy, PartialEq, Eq)]
enum Copper<'a> {
    /// A pad of the part with this reference.
    Pad(&'a str),
    Thermal,
    Track,
    Via,
}

fn items<'a>(board: &'a Board, routed: &'a Routed) -> Vec<Item<'a>> {
    let mut out = Vec::new();
    for part in &board.parts {
        for pad in &part.pads {
            out.push(Item {
                what: format!("{} pad {}", part.reference, pad.number),
                kind: Copper::Pad(&part.reference),
                net: pad.net.as_deref(),
                shape: copper(pad),
                layers: [true, pad.drill_nm.is_some()],
            });
        }
    }
    for a in &routed.arrays {
        for &at in &a.at {
            out.push(Item {
                what: format!("thermal via {at:?} under {} pad {}", a.reference, a.pad),
                kind: Copper::Thermal,
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
            kind: Copper::Track,
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
            kind: Copper::Via,
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

/// No copper a keepout forbids is inside it (`AET-WO-4300d2` D2 (a),
/// `RUL-03`): a track on its layer, a via or thermal via, another part's pad
/// on its layer, or a fill on its layer. Copper that touches its edge is not
/// inside it.
fn keepouts(board: &Board, routed: &Routed, all: &[Item<'_>]) -> Result<(), String> {
    for part in &board.parts {
        for k in &part.keepouts {
            let l = k.layer.index();
            for item in all.iter().filter(|i| i.layers[l]) {
                let forbidden = match item.kind {
                    Copper::Track => k.tracks,
                    Copper::Via | Copper::Thermal => k.vias,
                    Copper::Pad(owner) => k.pads && owner != part.reference,
                };
                if forbidden && item.shape.enters(k.rect) {
                    return Err(format!(
                        "{} is inside {}'s keepout on {:?}",
                        item.what, part.reference, k.layer
                    ));
                }
            }
            let name = k.layer.name();
            // A fill covers the board, so it meets a keepout that reaches
            // inside the outline (`AET-WO-4411`: a radio module's lies off
            // the board's edge). A thermal pour (`AET-WO-4300k` (a)) may share
            // the layer: KiCad's refill keeps it out of the keepout, the DRC
            // gate judges the filled board, and `pour` counts the keepout as
            // no copper.
            if k.fill
                && k.rect.overlaps(board.outline)
                && routed
                    .zones
                    .iter()
                    .any(|(n, layer)| layer == name && !board.stack.is_pour(n, layer))
            {
                return Err(format!(
                    "a fill on {name} would meet {}'s keepout",
                    part.reference
                ));
            }
        }
        for k in &part.inner_keepouts {
            for item in all
                .iter()
                .filter(|i| matches!(i.kind, Copper::Via | Copper::Thermal))
            {
                if k.vias && item.shape.enters(k.rect) {
                    return Err(format!(
                        "{} is inside {}'s keepout on {}",
                        item.what, part.reference, k.layer
                    ));
                }
            }
            if k.fill
                && k.rect.overlaps(board.outline)
                && routed.zones.iter().any(|(_, layer)| *layer == k.layer)
            {
                return Err(format!(
                    "a fill on {} would meet {}'s keepout",
                    k.layer, part.reference
                ));
            }
        }
    }
    Ok(())
}

/// Every pair of copper items on a shared layer, of different nets (a pad
/// with no net differs from everything), keeps the declared clearance; every
/// track and via keeps [`EDGE_NM`] from the edge; holes keep
/// [`HOLE_GAP_NM`]; tracks run along an axis; no via touches a pad of any
/// net, and no via's copper is in a part's courtyard, but inside that part's
/// own ring interior (`AET-WO-1307` K1).
/// The pairs [`check`] judges: every two copper items, and every two
/// drilled holes (`AET-WO-1324` Q4).
fn judge_work(board: &Board, routed: &Routed) -> u64 {
    let pairs = |n: usize| (n as u64) * (n as u64).saturating_sub(1) / 2;
    let holes = routed.vias.len()
        + routed.arrays.iter().map(|a| a.at.len()).sum::<usize>()
        + board
            .parts
            .iter()
            .flat_map(|p| &p.pads)
            .filter(|p| p.drill_nm.is_some())
            .count();
    pairs(items(board, routed).len()) + pairs(holes)
}

pub fn check(board: &Board, routed: &Routed) -> Result<(), String> {
    let all = items(board, routed);
    keepouts(board, routed, &all)?;
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
    // Every track, via and thermal via: chosen by its kind, never by its
    // text (`via1`, review 2, A16).
    for item in all.iter().filter(|i| !matches!(i.kind, Copper::Pad(_))) {
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
                    // A pad: the track ends inside its copper ([`Pad::reach`]).
                    pads[j].reach().contains(Rect::point(e.0, e.1))
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
    let mut declared: Vec<(String, String)> = if stack.has_planes() {
        stack
            .planes
            .iter()
            .map(|(l, n)| (n.clone(), l.clone()))
            .collect()
    } else {
        vec![(GROUND.to_owned(), "B.Cu".to_owned())]
    };
    // `AET-WO-4300k` (a): the thermal pours are declared zones too; a board
    // the router has not yet poured holds only the planes.
    let planes = declared.len();
    declared.extend(stack.pour_zones());
    let mut zones = routed.zones.clone();
    zones.sort();
    zones.dedup();
    if zones.len() != routed.zones.len() || zones.iter().any(|z| !declared.contains(z)) {
        return Err(format!(
            "zones {:?}; the specification declares {declared:?}",
            routed.zones
        ));
    }
    let poured = zones.iter().filter(|(n, l)| stack.is_pour(n, l)).count();
    if stack.has_planes() && zones.len() - poured != planes {
        return Err(format!(
            "planes {:?}; the specification declares {declared:?}",
            routed.zones
        ));
    }
    // A thermal pour (`AET-WO-4300k` (a)) joins nothing a plane does not:
    // KiCad drops its islands, and it adds only its own net's copper, so it
    // is not flooded here.
    for (net, layer) in zones.iter().filter(|(n, l)| !stack.is_pour(n, l)) {
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
        // The judge's flood is not the router's work.
        let labels = components(&grid, &core, &all_seeds, &mut 0);
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

mod read;
pub use read::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strict::{self, Tree};

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
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
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
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
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
            0,
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
                keepouts: Vec::new(),
                inner_keepouts: Vec::new(),
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
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
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

    /// `1308` A4 (`AET-WO-1311` §1.6): a net whose only way is across a
    /// fixed pair's copper fails closed naming the pair. `/S` must cross the
    /// pair `X` on F.Cu, and a keepout over the whole board forbids every
    /// via and every B.Cu track.
    #[test]
    fn a_net_blocked_only_by_a_fixed_pair_names_the_pair() {
        let mut board = paired(12 * MM);
        let at = |x: i64, y: i64| rect(x - 150_000, y - 150_000, x + 150_000, y + 150_000);
        board.parts.push(Part {
            rotation: 0,
            reference: "S".into(),
            pads: vec![
                pad("1", Some("/S"), at(9 * MM, 2 * MM)),
                pad("2", Some("/S"), at(9 * MM, 8 * MM)),
            ],
            courtyard: Vec::new(),
            keepouts: vec![Keepout {
                layer: Layer::B,
                rect: rect(0, 0, 20 * MM, 10 * MM),
                tracks: true,
                vias: true,
                pads: false,
                fill: false,
            }],
            inner_keepouts: Vec::new(),
        });
        let e = route(&board).unwrap_err();
        assert_eq!(e.net, "/S", "{e:?}");
        assert!(
            e.why
                .contains("only the fixed copper of the pair `X` (/P, /N) stands in the way"),
            "{e:?}"
        );
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

    /// The pair walk takes a track end anywhere in a pad's `reach()`, as the
    /// router and `connectivity` do (`AET-WO-1310` D5; roadmap §7.1, from
    /// `4300d2` reduction 8): on round pads, whose `inner()` is their centre
    /// alone, a coupled pair ending 75 µm off centre is judged, not refused.
    #[test]
    fn the_pair_walk_ends_a_track_anywhere_a_pad_reaches() {
        let mut board = paired(12 * MM);
        for p in board.parts.iter_mut().flat_map(|p| p.pads.iter_mut()) {
            p.corner_nm = 150_000;
        }
        let off = 75_000;
        let routed = Routed {
            segments: vec![
                seg(
                    "/N",
                    Layer::F,
                    (2 * MM + off, 5 * MM + MM / 2),
                    (12 * MM - off, 5 * MM + MM / 2),
                ),
                seg(
                    "/P",
                    Layer::F,
                    (2 * MM + off, 5 * MM),
                    (12 * MM - off, 5 * MM),
                ),
            ],
            ..Routed::default()
        };
        let r = pairs(&board, &routed).unwrap();
        assert_eq!(r[0].length_difference_nm(), 0);
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
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
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

    /// Review 2, A16, in `keepouts` (`AET-WO-1311` §1.7): a pad of a part
    /// named `via1`, `track_x` or `thermal1` is a pad there too. A keepout
    /// forbidding pads (and nothing else) refuses it; one forbidding tracks
    /// and vias (and not pads) lets it be.
    #[test]
    fn a_keepout_judges_a_part_named_like_copper_as_a_pad() {
        let keepout = |pads: bool| Keepout {
            layer: Layer::F,
            rect: rect(3 * MM, 9 * MM, 5 * MM, 11 * MM),
            tracks: !pads,
            vias: !pads,
            pads,
            fill: false,
        };
        for reference in ["via1", "track_x", "thermal1", "R9"] {
            for pads in [true, false] {
                let mut host = resistor("H1", 15 * MM, "/C", "/D");
                host.keepouts = vec![keepout(pads)];
                // Rounded pads: inside the keepout whatever the judge's
                // reading of a sharp corner (the next test's).
                let mut guest = resistor(reference, 4 * MM, "/A", "/B");
                for p in &mut guest.pads {
                    p.corner_nm = 100_000;
                }
                let board = Board {
                    outline: rect(0, 0, 20 * MM, 20 * MM),
                    parts: vec![guest, host],
                    stack: crate::stack::reference(),
                };
                let e = check(&board, &Routed::default());
                assert_eq!(
                    e.as_ref().is_err_and(|m| m.contains("keepout")),
                    pads,
                    "{reference}, a keepout forbidding pads: {pads}: {e:?}"
                );
            }
        }
    }

    /// A sharp-cornered pad (`r` 0) inside another part's keepout is inside
    /// it (`AET-WO-1311` adversarial pass); one that only touches its edge,
    /// or lies beside it, is not.
    #[test]
    fn a_sharp_pad_inside_a_keepout_is_refused() {
        for (left, inside) in [(3 * MM, true), (4_760_000, false), (6 * MM, false)] {
            let mut host = resistor("H1", 15 * MM, "/C", "/D");
            host.keepouts = vec![Keepout {
                layer: Layer::F,
                rect: rect(left, 9 * MM, left + 2 * MM, 11 * MM),
                tracks: false,
                vias: false,
                pads: true,
                fill: false,
            }];
            let board = Board {
                outline: rect(0, 0, 20 * MM, 20 * MM),
                parts: vec![resistor("R1", 4 * MM, "/A", "/B"), host],
                stack: crate::stack::reference(),
            };
            let e = check(&board, &Routed::default());
            assert_eq!(
                e.as_ref().is_err_and(|m| m.contains("inside H1's keepout")),
                inside,
                "a keepout from {left} nm: {e:?}"
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
                "(fp_arc (start -1 0) (mid 0 -1) (end 1 0) (layer \"F.CrtYd\"))".into(),
                "not modelled",
            ),
            (
                "(fp_circle (center 0 0) (end 0 0) (layer \"F.CrtYd\"))".into(),
                "no radius",
            ),
            // Adversarial F5: far coordinates are refused, never overflowed.
            (
                "(fp_circle (center -9000000 0) (end 9000000 0) (layer \"F.CrtYd\"))".into(),
                "too large",
            ),
        ] {
            let e = part_of(&with(&yard)).unwrap_err();
            assert!(e.contains(want), "{yard}: {e}");
        }
        // A whole circle is the square that holds it (`AET-WO-1312`): a via
        // kept out of the square is out of the circle.
        let circle = part_of(&with(
            "(fp_circle (center 0 0) (end 0.6 0.8) (layer \"F.CrtYd\"))",
        ))
        .unwrap();
        let corners: std::collections::BTreeSet<(i64, i64)> =
            circle.courtyard.iter().map(|e| e.0).collect();
        let xs: std::collections::BTreeSet<i64> = corners.iter().map(|c| c.0).collect();
        let ys: std::collections::BTreeSet<i64> = corners.iter().map(|c| c.1).collect();
        // Radius 1 mm about the part's origin: a 2 mm square.
        assert_eq!(corners.len(), 4, "{corners:?}");
        assert_eq!(
            (
                xs.last().unwrap() - xs.first().unwrap(),
                ys.last().unwrap() - ys.first().unwrap()
            ),
            (2_000_000, 2_000_000),
            "{corners:?}"
        );
        let board = |copper: &str| {
            strict::board(&format!(
                "(kicad_pcb (layers (0 \"F.Cu\" signal) (4 \"In1.Cu\" signal) (2 \"B.Cu\" signal) (25 \"Edge.Cuts\" user)) (gr_rect (start 0 0) (end 10 10) (layer \"Edge.Cuts\")) {copper})"
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

    /// `AET-WO-1311` §1.2: `read` takes at most [`MAX_ITEMS`] items, pads
    /// included; one more is refused. Since `AET-WO-1322` the footprint's
    /// four courtyard edges and a keepout count too.
    #[test]
    fn a_board_past_the_item_cap_is_refused() {
        let keepout = "(zone (layer \"F.Cu\") (keepout (tracks not_allowed) (vias not_allowed) (pads not_allowed) (copperpour not_allowed) (footprints allowed)) (polygon (pts (xy 4 4) (xy 6 4) (xy 6 6) (xy 4 6))))";
        let with = |pads: usize, segments: usize, zone: &str| {
            let pad = "(pad \"1\" smd rect (at 0 0) (size 0.5 0.5) (layers \"F.Cu\") (net \"/A\"))";
            let segment =
                "(segment (start 1 1) (end 2 1) (width 0.2) (layer \"F.Cu\") (net \"/A\"))";
            strict::board(&format!(
                "(kicad_pcb (layers (0 \"F.Cu\" signal) (2 \"B.Cu\" signal) (31 \"F.CrtYd\" user) (25 \"Edge.Cuts\" user)) (gr_rect (start 0 0) (end 10 10) (layer \"Edge.Cuts\")) (footprint \"x\" (at 5 5) (property \"Reference\" \"R1\") {} (fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\")) {zone}) {})",
                pad.repeat(pads),
                segment.repeat(segments)
            ))
            .unwrap()
        };
        let board = |pads, segments| with(pads, segments, "");
        assert!(read_ref(&board(2, MAX_ITEMS - 6)).is_ok());
        for (pads, segments) in [(2, MAX_ITEMS - 1), (MAX_ITEMS + 1, 0)] {
            let e = read_ref(&board(pads, segments)).unwrap_err();
            assert!(
                e.contains(&format!("{} copper items", MAX_ITEMS + 1)),
                "{e}"
            );
        }
        let e = read_ref(&board(2, MAX_ITEMS - 5)).unwrap_err();
        assert!(
            e.contains(&format!(
                "{} copper items (pads, tracks and vias) and 4 courtyard edges and keepouts, which `check` also scans: {} items",
                MAX_ITEMS - 3,
                MAX_ITEMS + 1
            )),
            "{e}"
        );
        // A footprint's keepout counts as one.
        assert!(read_ref(&with(2, MAX_ITEMS - 7, keepout)).is_ok());
        let e = read_ref(&with(2, MAX_ITEMS - 6, keepout)).unwrap_err();
        assert!(e.contains("5 courtyard edges and keepouts"), "{e}");
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
            via_nm: crate::route::VIA_NM,
            drill_nm: crate::route::DRILL_NM,
            bounded: vec![],
            notes: vec![],
            copper: vec!["F.Cu".into(), "B.Cu".into()],
            planes: vec![],
            thickness: crate::stack::Thickness {
                nominal_nm: 1_600_000,
                min_nm: 1_500_000,
                max_nm: 1_700_000,
                source: "t".into(),
            },
            dielectrics: vec![],
            dielectric_max_nm: vec![],
            copper_max_nm: 0,
            copper_source: String::new(),
            permittivity_permille: 0,
            permittivity_source: String::new(),
            clearance_nm: 0,
            width_nm: 0,
            classes: vec![],
            pairs: vec![],
            oscillator: vec![],
            joins: vec![],
            thermal: vec![],
            imus: vec![],
            pours: vec![],
            exits: vec![],
            junction: None,
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
        // `AET-WO-1311` adversarial B: a row of a copper type is copper
        // whatever its id and name; KiCad plotted `(37 "Top" signal)` and
        // `(39 "Mid" signal)` as In1.Cu and In2.Cu (measured).
        for rows in [
            "(37 \"Top\" signal) (39 \"Mid\" signal)",
            "(3 \"Top\" power) (5 \"Mid\" mixed)",
            "(4.0 \"Top\" jumper)",
        ] {
            let e = judged(&format!("{canonical} {rows}")).unwrap_err();
            assert_eq!(e.kind, "LayersDisagree", "{rows}: {}", e.message);
        }
    }

    /// `AET-WO-1311` adversarial D and E: `read` refuses a board with no
    /// layer table (the strict reader judges item layers only against one,
    /// and `check support` reads without `declared`), a pad with no
    /// `layers` (KiCad flashed one on F.Cu and B.Cu) and a pad with a
    /// padstack of its own (KiCad flashed its size, not the pad's).
    #[test]
    fn read_refuses_what_kicad_places_otherwise() {
        let e = read_ref(
            &strict::board("(kicad_pcb (gr_rect (start 0 0) (end 10 10) (layer \"Edge.Cuts\")))")
                .unwrap(),
        )
        .unwrap_err();
        assert!(e.contains("no layer table"), "{e}");
        let fp = |pad: &str| {
            crate::sexpr::parse(&format!(
                "(footprint \"x\" (at 1 2) (property \"Reference\" \"R1\") {pad} (fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\")))"
            ))
            .unwrap()
        };
        assert!(
            part_of(&fp(
                "(pad \"1\" smd rect (at 0 0) (size 0.5 0.5) (layers \"F.Cu\"))"
            ))
            .is_ok()
        );
        let e = part_of(&fp("(pad \"1\" smd rect (at 0 0) (size 0.5 0.5))")).unwrap_err();
        assert!(e.contains("names no layers"), "{e}");
        let e = part_of(&fp(
            "(pad \"1\" smd rect (at 0 0) (size 0.5 0.5) (layers \"F.Cu\") (padstack (mode custom) (layer \"F.Cu\" (size 3 3))))",
        ))
        .unwrap_err();
        assert!(e.contains("padstack"), "{e}");
    }

    // ---- `AET-WO-1323` ----

    /// D1: a routing that does `w` units of work routes under a bound of `w`
    /// and is refused by name, with the bound, at `w - 1`. One board routes
    /// signals only; the other stitches ground, so a fill flood counts too.
    /// The final check is charged last (`AET-WO-1324` Q4), so `w - 1` is
    /// refused there, and one unit under the routing's own work is refused
    /// while a net is routed, by that net.
    #[test]
    fn a_routing_one_unit_over_its_bound_is_refused_by_name() {
        let board = |b: &str| Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts: vec![
                resistor("R1", 5 * MM, "/A", b),
                resistor("R2", 15 * MM, b, "/A"),
            ],
            stack: crate::stack::reference(),
        };
        for b in ["/B", GROUND] {
            let board = board(b);
            let (routed, w) = route_measured(&board, Rung::TODAY, WORK).unwrap();
            assert!(w > 0, "{b}");
            let (again, at_w) = route_measured(&board, Rung::TODAY, w).unwrap();
            assert_eq!((&again, at_w), (&routed, w), "{b}");
            let e = route_measured(&board, Rung::TODAY, w - 1).unwrap_err();
            assert_eq!(e.work_bound, Some(w - 1), "{b}: {e:?}");
            assert!(e.why.contains("at the final check"), "{b}: {e:?}");
            let under = w - judge_work(&board, &again) - 1;
            let e = route_measured(&board, Rung::TODAY, under).unwrap_err();
            assert_eq!(e.work_bound, Some(under), "{b}: {e:?}");
            assert!(!e.net.is_empty(), "{b}: {e:?}");
            assert!(
                e.why.contains(&format!("its bound, {under} search steps")),
                "{b}: {e:?}"
            );
        }
    }

    /// `1307` A4's adversary, reconstructed (`spike/1323/adv1307.rs`): a
    /// 249 mm board, four LQFP-144 (the vendored footprint's pads, its
    /// courtyard as the ±11.65 mm square) and 288 two-pad nets, a random
    /// matching of the 576 pads from a fixed seed. It took 203 s to
    /// `RIPUPS`; under [`WORK`] it is refused by name.
    fn adversary(seed: u64) -> Board {
        let mut order: Vec<usize> = (0..576).collect();
        let mut s = seed;
        let mut next = || {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            s >> 33
        };
        for i in (1..order.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            order.swap(i, j);
        }
        let mut nets: Vec<Option<String>> = vec![None; 576];
        for (k, w) in order.chunks(2).enumerate() {
            for &p in w {
                nets[p] = Some(format!("/N{k:03}"));
            }
        }
        let lqfp = |i: usize, cx: i64, cy: i64| {
            let (far, step, first) = (10_662_500, 500_000, -8_750_000);
            let pads = (0..144usize)
                .map(|k| {
                    let along = first + (k % 36) as i64 * step;
                    let (x, y, w, h) = match k / 36 {
                        0 => (-far, along, 1_475_000, 300_000),
                        1 => (along, far, 300_000, 1_475_000),
                        2 => (far, -along, 1_475_000, 300_000),
                        _ => (-along, -far, 300_000, 1_475_000),
                    };
                    let (x, y) = (cx + x, cy + y);
                    Pad {
                        corner_nm: 75_000,
                        ..pad(
                            &(k + 1).to_string(),
                            nets[i * 144 + k].as_deref(),
                            rect(x - w / 2, y - h / 2, x + w / 2, y + h / 2),
                        )
                    }
                })
                .collect();
            let c = 11_650_000;
            let q = [
                (cx - c, cy - c),
                (cx + c, cy - c),
                (cx + c, cy + c),
                (cx - c, cy + c),
            ];
            Part {
                reference: format!("U{}", i + 1),
                rotation: 0,
                pads,
                courtyard: (0..4).map(|k| (q[k], q[(k + 1) % 4])).collect(),
                keepouts: Vec::new(),
                inner_keepouts: Vec::new(),
            }
        };
        let parts = [(62, 62), (187, 62), (62, 187), (187, 187)]
            .iter()
            .enumerate()
            .map(|(i, &(x, y))| lqfp(i, x * MM, y * MM))
            .collect();
        Board {
            outline: rect(0, 0, 249 * MM, 249 * MM),
            parts,
            stack: crate::stack::reference(),
        }
    }

    /// D1, the acceptance: the adversary is refused by the work bound, by
    /// name and with the net it was routing, long before `RIPUPS`.
    #[test]
    fn the_1307_adversary_is_refused_by_the_work_bound() {
        let e = route(&adversary(1307)).unwrap_err();
        assert_eq!(e.work_bound, Some(WORK), "{e:?}");
        assert!(e.net.starts_with("/N"), "{e:?}");
        assert!(
            e.why
                .contains("the router's work passed its bound, 350000000"),
            "{e:?}"
        );
    }

    /// D3 (`1310` A7): a footprint keepout must lie inside the footprint's
    /// courtyard outline or its pad field, edges included, at any turn; one
    /// outside both is refused at read, by the part's name.
    #[test]
    fn a_keepout_outside_its_own_footprint_is_refused_at_read() {
        let zone = |(x0, y0, x1, y1): (f64, f64, f64, f64)| {
            format!(
                "(zone (layer \"F.Cu\") (keepout (tracks not_allowed) (vias not_allowed) (pads not_allowed) (copperpour not_allowed) (footprints allowed)) (polygon (pts (xy {x0} {y0}) (xy {x1} {y0}) (xy {x1} {y1}) (xy {x0} {y1}))))"
            )
        };
        let fp = |at: &str, yard: &str, pad: &str, z: &str| {
            part_of(
                &crate::sexpr::parse(&format!(
                    "(footprint \"x\" (at {at}) (property \"Reference\" \"K1\") {pad} {yard} {z})"
                ))
                .unwrap(),
            )
        };
        let small = "(pad \"1\" smd rect (at 0 0) (size 0.2 0.2) (layers \"F.Cu\"))";
        let wide = "(fp_rect (start -2 -1) (end 2 1) (layer \"F.CrtYd\"))";
        let refused = "K1's keepout on F.Cu lies outside its own courtyard and pad field";
        // On the courtyard's edge, whole: read. Moved off the part: refused.
        assert!(fp("5 5", wide, small, &zone((3.0, 4.0, 7.0, 6.0))).is_ok());
        assert_eq!(
            fp("5 5", wide, small, &zone((13.0, 4.0, 17.0, 6.0))).unwrap_err(),
            refused
        );
        assert_eq!(
            fp("5 5", wide, small, &zone((3.0, 4.0, 7.1, 6.0))).unwrap_err(),
            refused
        );
        // Turned 90°: the courtyard stands 2 wide and 4 tall on the board.
        assert!(fp("5 5 90", wide, small, &zone((4.0, 3.0, 6.0, 7.0))).is_ok());
        assert_eq!(
            fp("5 5 90", wide, small, &zone((3.0, 4.0, 7.0, 6.0))).unwrap_err(),
            refused
        );
        // An L-shaped courtyard: a keepout in either leg is read; one across
        // the notch, inside the L's box, is not.
        let ell = "(fp_poly (pts (xy -2 -2) (xy 2 -2) (xy 2 0) (xy 0 0) (xy 0 2) (xy -2 2)) (layer \"F.CrtYd\"))";
        assert!(fp("5 5", ell, small, &zone((3.0, 3.0, 7.0, 5.0))).is_ok());
        assert!(fp("5 5", ell, small, &zone((3.0, 3.0, 5.0, 7.0))).is_ok());
        assert_eq!(
            fp("5 5", ell, small, &zone((4.0, 4.0, 6.0, 6.0))).unwrap_err(),
            refused
        );
        // Beyond a small courtyard but inside the pad field: read.
        let tiny = "(fp_rect (start -0.2 -0.2) (end 0.2 0.2) (layer \"F.CrtYd\"))";
        let big = "(pad \"1\" smd rect (at 0 0) (size 2 2) (layers \"F.Cu\"))";
        assert!(fp("5 5", tiny, big, &zone((4.2, 4.2, 5.8, 5.8))).is_ok());
        assert_eq!(
            fp("5 5", tiny, small, &zone((4.2, 4.2, 5.8, 5.8))).unwrap_err(),
            refused
        );
    }

    /// `paired(12 mm)` with a part `S` whose two `/S` pads stand at
    /// (x, y1) and (x, y2), in µm (`1311` F2).
    fn with_s(x: i64, y1: i64, y2: i64) -> Board {
        let at = |x: i64, y: i64| rect(x - 150_000, y - 150_000, x + 150_000, y + 150_000);
        let mut board = paired(12 * MM);
        board.parts.push(Part {
            rotation: 0,
            reference: "S".into(),
            pads: vec![
                pad("1", Some("/S"), at(x * 1000, y1 * 1000)),
                pad("2", Some("/S"), at(x * 1000, y2 * 1000)),
            ],
            courtyard: Vec::new(),
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
        });
        board
    }

    /// D4 (`1311` F2): the breakout that met its own run three pitches in
    /// and walked back along it left the run's head a stub, which
    /// `connectivity` refused ("joined to nothing"). Pruned, the board routes.
    #[test]
    fn a_pairs_breakout_leaves_no_stub() {
        let board = with_s(7000, 3000, 8000);
        let routed = route(&board).unwrap();
        connectivity(&board, &routed).unwrap();
        let report = pairs(&board, &routed).unwrap();
        assert_eq!(report.len(), 1);
    }

    /// D4, swept as `1307` B6 swept placements: `S` at every x from 0.5 to
    /// 19.5 mm by 0.5 mm and each (y1, y2) of {1, 2, 3, 4} × {6.5, 7, 8, 9}
    /// mm. None reaches `connectivity`'s "joined to nothing"; each routes or
    /// is refused by a name before it.
    #[test]
    fn no_placement_of_a_crossing_part_leaves_a_pair_stub() {
        let mut routed = 0;
        for xi in 0..39 {
            for y1 in [1000, 2000, 3000, 4000] {
                for y2 in [6500, 7000, 8000, 9000] {
                    let x = 500 + 500 * xi;
                    match route(&with_s(x, y1, y2)) {
                        Ok(_) => routed += 1,
                        Err(e) => assert!(
                            !e.why.contains("joined to nothing"),
                            "{x}, {y1}, {y2}: {e:?}"
                        ),
                    }
                }
            }
        }
        assert!(routed > 0);
    }

    /// D4, the adversarial pass's A1: a pair whose pads the grid cannot
    /// reach is joined at each escape's far end, outside the pad; `prune`
    /// keeps that end, as it keeps an end in a pad. Two SOIC-like parts,
    /// 8 pads a side at 0.4 mm, 0.15 mm wide, `/P` and `/N` two pads apart.
    #[test]
    fn a_pair_on_escaped_pads_keeps_its_breakouts() {
        let side = |k: usize| match k {
            2 => Some("/P"),
            4 => Some("/N"),
            _ => None,
        };
        let soic = |reference: &str, cx: i64, nets: [fn(usize) -> Option<&'static str>; 2]| {
            let mut pads = Vec::new();
            for (col, net) in [-1, 1].into_iter().zip(nets) {
                for k in 0..8 {
                    let (x, y) = (cx + col * 2 * MM, 6 * MM + k as i64 * 400_000 - 1_400_000);
                    let number = (pads.len() + 1).to_string();
                    pads.push(pad(
                        &number,
                        net(k),
                        rect(x - MM / 2, y - 75_000, x + MM / 2, y + 75_000),
                    ));
                }
            }
            Part {
                rotation: 0,
                reference: reference.into(),
                pads,
                courtyard: Vec::new(),
                keepouts: Vec::new(),
                inner_keepouts: Vec::new(),
            }
        };
        let none: fn(usize) -> Option<&'static str> = |_| None;
        let mut stack = crate::stack::reference();
        stack.pairs.push(crate::stack::Pair {
            name: "X".into(),
            nets: ["/P".into(), "/N".into()],
            gap_nm: 300_000,
            width_nm: 200_000,
        });
        let board = Board {
            outline: rect(0, 0, 24 * MM, 12 * MM),
            parts: vec![
                soic("A", 5 * MM, [none, side]),
                soic("B", 19 * MM, [side, none]),
            ],
            stack,
        };
        let routed = route(&board).unwrap();
        connectivity(&board, &routed).unwrap();
    }

    /// D4: the pairs' copper is judged right after the pair loop. Here the
    /// pruned run's gap breaks, which the pair judge names; without the
    /// early judge, `/S` is refused first, for the pair's copper in its way.
    #[test]
    fn a_pair_the_judge_refuses_is_refused_by_its_own_name_first() {
        let e = route(&with_s(10_750, 1000, 6500)).unwrap_err();
        assert_eq!(e.net, "", "{e:?}");
        assert!(
            e.why
                .starts_with("pair `X`: `/P`: the coupled run breaks at"),
            "{e:?}"
        );
    }

    // ---- `AET-WO-1324` ----

    /// Sentinel-02-power's gate inputs: JLCPCB's +0.13 mm hole and
    /// ±0.075 mm position, 18 µm plating; `/S` and ground judged.
    fn jlc() -> crate::stack::JunctionRule {
        crate::stack::JunctionRule {
            nets: vec!["/S".into(), GROUND.into()],
            hole_plus_nm: 130_000,
            hole_offset_nm: 75_000,
            plating_nm: 18_000,
        }
    }

    /// A 20 mm board with two pads of `/S` and two of `/T` at its corners,
    /// plus `extra` parts, judged by `rule`.
    fn judged(rule: Option<crate::stack::JunctionRule>, extra: Vec<Part>) -> Board {
        let mut stack = crate::stack::reference();
        stack.junction = rule;
        let mut parts = vec![
            resistor("R1", 2 * MM, "/S", "/T"),
            resistor("R2", 18 * MM, "/T", "/S"),
        ];
        parts.extend(extra);
        Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts,
            stack,
        }
    }

    fn router<'a>(board: &'a Board, arrays: &[Array]) -> Router<'a> {
        let grid = Grid::new(board.outline).unwrap();
        Router::new(board, grid, vec!["/S".into(), "/T".into()], arrays, 0)
    }

    fn node_at(r: &Router<'_>, x: i64, y: i64) -> usize {
        grid_node(&r.grid, x, y).unwrap()
    }

    /// The regions of two 0.3 mm vias meet where their cells do: at 108
    /// offsets of the router's grid, out to (5, 3), and at (4, 4), 0.707 mm
    /// apart, where twice the reach is only 0.698 mm, so a disc would pass
    /// the very pair `extract` refused on Sentinel-02-power (PC0).
    #[test]
    fn two_junction_regions_meet_where_their_cells_do_not_where_a_disc_does() {
        let rule = jlc();
        assert_eq!(rule.reach(300_000), 349_068);
        let board = judged(Some(rule.clone()), Vec::new());
        let r = router(&board, &[]);
        let meet = &r.junction[0].as_ref().unwrap().meet;
        assert_eq!(meet.len(), 108);
        for at in [(4, 4), (5, 3), (-5, -3), (0, 5), (-5, 0)] {
            assert!(meet.contains(&at), "{at:?}");
        }
        for at in [(0, 0), (4, 5), (5, 5), (6, 0), (0, -6)] {
            assert!(!meet.contains(&at), "{at:?}");
        }
        let d2 = 2 * (4 * PITCH_NM) * (4 * PITCH_NM);
        assert!(4 * rule.reach(300_000).pow(2) < d2);
        // `/T` is not judged, and no net is on a board the gate does not run on
        assert!(r.junction[1].is_none());
        let off = judged(None, Vec::new());
        assert!(router(&off, &[]).junction.iter().all(Option::is_none));
    }

    /// A judged net's via keeps its laid vias' regions: at (4, 4) pitches
    /// from one, where the hole gap alone allows it, it may not stand; at
    /// (6, 0) it may; once the net is taken up, it may again.
    #[test]
    fn a_judged_nets_via_stays_out_of_its_laid_vias_regions() {
        let (x, y) = (10 * MM, 8 * MM);
        let near = (x + 4 * PITCH_NM, y + 4 * PITCH_NM);
        let apart = (x + 6 * PITCH_NM, y);
        let board = judged(Some(jlc()), Vec::new());
        let mut r = router(&board, &[]);
        let a = node_at(&r, x, y);
        r.commit(0, &[(Layer::F, a), (Layer::B, a)]);
        assert!(r.via_open(a, 0));
        assert!(!r.via_open(node_at(&r, near.0, near.1), 0));
        assert!(r.via_open(node_at(&r, apart.0, apart.1), 0));
        r.rip(0);
        assert!(r.via_open(node_at(&r, near.0, near.1), 0));
        // with no rule, the hole gap (0.56 mm) alone admits (4, 4)
        let free = judged(None, Vec::new());
        let mut r = router(&free, &[]);
        let a = node_at(&r, x, y);
        r.commit(0, &[(Layer::F, a), (Layer::B, a)]);
        assert!(r.via_open(node_at(&r, near.0, near.1), 0));
    }

    /// Within one path: two new vias whose regions meet are a close pair,
    /// as two within the hole gap are; with no rule they are not.
    #[test]
    fn a_paths_own_vias_whose_regions_meet_are_a_close_pair() {
        let path = |r: &Router<'_>| {
            let (x, y) = (10 * MM, 8 * MM);
            let mut p = vec![(Layer::F, node_at(r, x, y))];
            for k in 0..=4 {
                p.push((Layer::B, node_at(r, x + k * PITCH_NM, y)));
            }
            for k in 1..=4 {
                p.push((Layer::B, node_at(r, x + 4 * PITCH_NM, y + k * PITCH_NM)));
            }
            p.push((Layer::F, node_at(r, x + 4 * PITCH_NM, y + 4 * PITCH_NM)));
            p
        };
        let board = judged(Some(jlc()), Vec::new());
        let r = router(&board, &[]);
        assert_eq!(r.close_vias(0, &path(&r)), Some((0, 10)));
        assert_eq!(r.close_vias(1, &path(&r)), None);
        let free = judged(None, Vec::new());
        let r = router(&free, &[]);
        assert_eq!(r.close_vias(0, &path(&r)), None);
    }

    /// A through-hole pad and an array via of a judged net hold their own
    /// regions (`extract` prices both as vias): a 1 mm hole's region meets a
    /// via's 1 mm off and not 1.25 mm off; an array via on the grid keeps a
    /// routed via (4, 4) pitches off it, where the hole gap would not.
    #[test]
    fn a_judged_nets_through_hole_pad_and_array_via_hold_their_regions() {
        let (x, y) = (10 * MM, 10 * MM);
        let tht = Part {
            reference: "J1".into(),
            rotation: 0,
            pads: vec![Pad {
                drill_nm: Some(1_000_000),
                ..pad(
                    "1",
                    Some("/S"),
                    rect(x - 850_000, y - 850_000, x + 850_000, y + 850_000),
                )
            }],
            courtyard: Vec::new(),
            keepouts: Vec::new(),
            inner_keepouts: Vec::new(),
        };
        let board = judged(Some(jlc()), vec![tht]);
        let r = router(&board, &[]);
        let j = r.junction[0].as_ref().unwrap();
        assert!(j.fixed[node_at(&r, x + MM, y)]);
        assert!(!j.fixed[node_at(&r, x + MM + MM / 4, y)]);
        assert!(
            !r.junction[1]
                .as_ref()
                .is_some_and(|j| j.fixed.contains(&true))
        );
        let (ax, ay) = (6 * MM, 14 * MM);
        let array = Array {
            reference: "U9".into(),
            pad: "9".into(),
            net: "/S".into(),
            via_nm: 600_000,
            drill_nm: 300_000,
            at: vec![(ax, ay)],
        };
        let board = judged(Some(jlc()), Vec::new());
        let r = router(&board, std::slice::from_ref(&array));
        assert!(!r.via_open(node_at(&r, ax + 4 * PITCH_NM, ay + 4 * PITCH_NM), 0));
        assert!(r.via_open(node_at(&r, ax + 6 * PITCH_NM, ay), 0));
        let free = judged(None, Vec::new());
        let r = router(&free, std::slice::from_ref(&array));
        assert!(r.via_open(node_at(&r, ax + 4 * PITCH_NM, ay + 4 * PITCH_NM), 0));
    }

    /// The index picks what scanning every remaining pad against the whole
    /// tree picked, step by step: the least (distance, node), then the
    /// first pad in order; on a lattice where distances tie, with nodes
    /// added out of order and twice, and the picked pad removed each step.
    #[test]
    fn the_nearest_index_picks_what_the_whole_scan_picked_ties_included() {
        let grid = Grid::new(rect(0, 0, 10 * MM, 10 * MM)).unwrap();
        let mut s = 1324_u64;
        let mut next = |m: u64| {
            s = s
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            (s >> 33) % m
        };
        let mut centres: Vec<(i64, i64)> = (0..40)
            .map(|_| ((next(9) as i64 + 1) * MM, (next(9) as i64 + 1) * MM))
            .collect();
        let mut index = Nearest::new(centres.clone());
        let mut tree: Vec<usize> = Vec::new();
        let mut picks = 0;
        while !centres.is_empty() {
            let batch: Vec<usize> = (0..1 + next(12))
                .map(|_| {
                    // whole millimetres and half: many equal distances
                    let (i, j) = (next(21) as usize * 4, next(21) as usize * 4);
                    j * grid.nx + i
                })
                .collect();
            let work = index.add(&grid, &batch);
            assert_eq!(work, (batch.len() * centres.len()) as u64);
            tree.extend(&batch);
            let scan = centres
                .iter()
                .enumerate()
                .map(|(k, &(cx, cy))| {
                    let near = tree
                        .iter()
                        .map(|&n| {
                            let (x, y) = grid.at(n);
                            ((x - cx).abs() + (y - cy).abs(), n)
                        })
                        .min()
                        .unwrap();
                    (k, near)
                })
                .min_by_key(|&(k, (d, _))| (d, k))
                .map(|(k, (_, n))| (k, n));
            let (got, work) = index.pick();
            assert_eq!(got, scan, "step {picks}");
            assert_eq!(work, centres.len() as u64);
            let k = got.unwrap().0;
            index.remove(k);
            centres.remove(k);
            picks += 1;
        }
        assert_eq!(picks, 40);
    }

    /// A search charges each source it seeds: ten sources and a goal met at
    /// the first state expanded cost eleven units.
    #[test]
    fn a_search_charges_every_source_it_seeds() {
        let board = judged(None, Vec::new());
        let mut r = router(&board, &[]);
        let sources: Vec<Node> = (0..10)
            .map(|k| (Layer::B, node_at(&r, 5 * MM + k * PITCH_NM, 10 * MM)))
            .collect();
        let all = r.window(&[], None);
        let found = r.search(0, &sources, &|_, _| true, &|_| 0, true, all, false);
        assert!(found.is_some());
        assert_eq!(r.work, 11);
    }

    /// The final check is charged its pairs before it runs: ten one-pad
    /// nets lay no copper, and their routing's work is the check's 45 pairs;
    /// one unit less is refused by name at the final check.
    #[test]
    fn the_final_check_is_charged_its_pairs() {
        let parts = (0..10)
            .map(|k| Part {
                reference: format!("P{k}"),
                rotation: 0,
                pads: vec![pad(
                    "1",
                    Some(&format!("/N{k}")),
                    rect(
                        (2 + k) * MM,
                        10 * MM,
                        (2 + k) * MM + 300_000,
                        10 * MM + 300_000,
                    ),
                )],
                courtyard: Vec::new(),
                keepouts: Vec::new(),
                inner_keepouts: Vec::new(),
            })
            .collect();
        let board = Board {
            outline: rect(0, 0, 20 * MM, 20 * MM),
            parts,
            stack: crate::stack::reference(),
        };
        let (routed, w) = route_measured(&board, Rung::TODAY, WORK).unwrap();
        assert_eq!((w, judge_work(&board, &routed)), (45, 45));
        let e = route_measured(&board, Rung::TODAY, 44).unwrap_err();
        assert_eq!(e.work_bound, Some(44), "{e:?}");
        assert!(e.net.is_empty(), "{e:?}");
        assert!(e.why.contains("at the final check"), "{e:?}");
    }

    /// N one-pad parts on a square lattice at 2 mm, every pad on one net
    /// that is not stitched (`AET-WO-1324` (2), `1323` D6's board,
    /// reconstructed).
    fn lattice(n: usize) -> Board {
        let side = (n as f64).sqrt().ceil() as i64;
        let (margin, pitch) = (3 * MM, 2 * MM);
        let parts = (0..n as i64)
            .map(|k| {
                let (x, y) = (margin + (k % side) * pitch, margin + (k / side) * pitch);
                Part {
                    reference: format!("P{k}"),
                    rotation: 0,
                    pads: vec![pad(
                        "1",
                        Some("/BIG"),
                        rect(x - 250_000, y - 250_000, x + 250_000, y + 250_000),
                    )],
                    courtyard: Vec::new(),
                    keepouts: Vec::new(),
                    inner_keepouts: Vec::new(),
                }
            })
            .collect();
        let w = 2 * margin + (side - 1) * pitch;
        Board {
            outline: rect(0, 0, w, w),
            parts,
            stack: crate::stack::reference(),
        }
    }

    /// (2), the acceptance: one net of 400, 900 and 1,600 pads routes, and
    /// the work charged for it is its pairs of (pad, tree node) the index
    /// takes, the sources each search seeds, the states expanded and the
    /// final check's pairs: as measured, growing as N² (the scan it
    /// replaced grew as N³: 24.6 G pairs at 1,600, 130 s in `1323`). The
    /// bound: 1,600 pads charge under a third of [`WORK`], and between 8
    /// and 24 times 400 pads' (N² gives 16; an uncharged scan or seed 4).
    /// Counts only, never a clock.
    #[test]
    fn one_net_of_1600_pads_is_charged_its_quadratic_work() {
        let work: Vec<u64> = [400, 900, 1600]
            .iter()
            .map(|&n| route_measured(&lattice(n), Rung::TODAY, WORK).unwrap().1)
            .collect();
        assert_eq!(work, [6_163_182, 31_193_212, 98_575_542]);
        assert!(3 * work[2] < WORK, "{work:?}");
        assert!(
            work[2] >= 8 * work[0] && work[2] <= 24 * work[0],
            "{work:?}"
        );
    }

    /// A2: the regions' meeting offsets, taken run by run, are exactly every
    /// `ca − cb`, for equal and unequal regions, small and large.
    #[test]
    fn the_meeting_offsets_by_runs_are_every_cell_difference() {
        use crate::flow::Region;
        let h = crate::supply::PITCH_NM;
        let every = |a: &Region, b: &Region| {
            let mut out = BTreeSet::new();
            for &(ai, aj) in &a.cells {
                for &(bi, bj) in &b.cells {
                    out.insert((ai - bi, aj - bj));
                }
            }
            out
        };
        for (ra, rb) in [
            (349_068, 349_068),
            (699_068, 349_068),
            (349_068, 699_068),
            (1_000_000, 250_001),
            (62_500, 62_499),
        ] {
            let (a, b) = (Region::new(h, ra), Region::new(h, rb));
            assert_eq!(meeting(&a, &b), every(&a, &b), "{ra} {rb}");
        }
        // any cell set, not only a disc's: rows with gaps, of unequal reach,
        // whose spans neither nest nor touch
        let ragged = |cells: &[(i64, i64)]| Region {
            cells: cells.iter().copied().collect(),
            faces: Vec::new(),
        };
        let a = ragged(&[(0, 0), (1, 0), (5, 0), (9, 0), (-3, 1), (2, 1), (3, 1)]);
        let b = ragged(&[(0, 0), (7, 0), (8, 0), (-6, 2), (4, 2)]);
        assert_eq!(meeting(&a, &b), every(&a, &b));
        assert_eq!(meeting(&b, &a), every(&b, &a));
    }

    /// A1: a path whose first and third vias' regions meet, with a second via
    /// between, cannot be laid flat (the run between ends on the far layer);
    /// the third via's node is banned, never laid unchecked. Under the hole
    /// gap alone, the same.
    #[test]
    fn a_close_pair_with_a_via_between_is_banned_not_flattened() {
        for (rule, far) in [(Some(jlc()), 5), (None, 4)] {
            let board = judged(rule, Vec::new());
            let mut r = router(&board, &[]);
            let (x, y) = (10 * MM, 8 * MM);
            let n = |r: &Router<'_>, i: i64, j: i64| node_at(r, x + i * PITCH_NM, y + j * PITCH_NM);
            let mut p = vec![(Layer::F, n(&r, 0, 0)), (Layer::B, n(&r, 0, 0))];
            for k in 1..=6 {
                p.push((Layer::B, n(&r, 0, k)));
            }
            p.push((Layer::F, n(&r, 0, 6)));
            for k in 1..=far {
                p.push((Layer::F, n(&r, k, 6)));
            }
            for k in (0..=5).rev() {
                p.push((Layer::F, n(&r, far, k)));
            }
            p.push((Layer::B, n(&r, far, 0)));
            p.push((Layer::B, n(&r, far + 1, 0)));
            let before = p.clone();
            assert_eq!(r.unhop(0, &mut p, false), None, "{far}");
            assert!(r.banned.contains(&n(&r, far, 0)), "{far}");
            assert_eq!(p, before, "{far}");
        }
    }
}
