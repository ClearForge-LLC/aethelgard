//! The placed board, `.kicad_pcb`: every part of the schematic as its
//! vendored footprint, placed without overlap, with each pad on the net
//! KiCad's own netlist gives its pin (`AET-WO-1301`). No tracks, vias or
//! zones; routing is `AET-WO-1302`.
//!
//! - **Footprints** are the vendored `.kicad_mod` trees, given the
//!   schematic's reference, value and symbol fields, the symbol's
//!   `(path "/<uuid>")`, a net on each pad that has a symbol pin, and UUIDs
//!   derived from content. KiCad's parity check (10.0.7,
//!   `drc_test_provider_schematic_parity.cpp`) matches footprints by
//!   reference and needs the library id, the value, every symbol field but
//!   Reference, Value and Footprint, and each pad's net as the netlist names
//!   it. A pad with no symbol pin carries no net: the check refuses a netted
//!   one. That is WSON-6's exposed pad 7, which TI's TMP117 datasheet
//!   (SNOSD82D §9) lets float.
//! - **Net names** follow KiCad's: a local label on the root sheet is
//!   `/<label>`, a power symbol's net is its value, and an unconnected pin's
//!   is `unconnected-(<ref>-<pin name>-Pad<number>)`
//!   (`SCH_PIN::GetDefaultNetName`).
//! - **Placement** is integer-only and a function of the parts alone
//!   ([`place`]).
//!
//! Every coordinate is nanometres, board Y down; every footprint sits on the
//! front. The placer gives every footprint rotation 0 but an anchored part,
//! a connector on a side edge (`AET-WO-1313`) and an IMU turned to its
//! declared orientation (`AET-WO-4400d`); a placement may carry
//! 90, 180 or 270 (`AET-WO-1308` §1.5, [`write_moved`]), which rotates the
//! footprint's pads, graphics and text positions about its origin
//! ([`route::rotate`]) and adds the rotation to each pad's and property's own
//! angle, which KiCad's file holds absolute (measured): the footprint turns
//! whole.

use std::collections::{BTreeMap, BTreeSet};

use aethelgard_ir::board::OutlineBound;

use crate::library;
use crate::parity;
use crate::route;
use crate::rules::FLOOR_NM;
use crate::schematic::{self, Conn, EmitError, GND, Part};
use crate::sexpr::{self, Sx, mm};

/// KiCad 10's board format, the version the vendored footprints are in and
/// `kicad-cli pcb upgrade` writes (`AET-WO-1300` B7).
pub const VERSION: &str = "20260206";

/// Where the MCU's footprint origin goes.
pub const ORIGIN: (i64, i64) = (100_000_000, 100_000_000);
/// The least distance between two parts' keep-out boxes (courtyard and
/// silkscreen together).
pub const GAP_NM: i64 = 500_000;
/// The least distance between two ICs' keep-out boxes: room for the router
/// between their pads (`AET-WO-1302`). At [`GAP_NM`], the reference board's
/// U1 pad 34 and U4 pad 7 faced each other across one track's width.
pub const IC_GAP_NM: i64 = 1_000_000;

/// From the outermost keep-out box to the board edge.
pub const MARGIN_NM: i64 = 2_000_000;
/// The candidate positions' pitch.
const STEP: i64 = 250_000;
/// Every footprint origin and the outline are on this grid.
const SNAP: i64 = 50_000;
/// How many rings of candidates the placer tries around its target: 1 m.
const RINGS: i64 = 4_000;

// The courtyard keeps the board-edge clearance (`AET-WO-1301` §1.2).
const _: () = assert!(MARGIN_NM >= FLOOR_NM);

/// KiCad 10's two-layer layer table, as `kicad-cli pcb upgrade` writes it.
const LAYERS: [(&str, &str, &str, Option<&str>); 20] = [
    ("0", "F.Cu", "signal", None),
    ("2", "B.Cu", "signal", None),
    ("9", "F.Adhes", "user", Some("F.Adhesive")),
    ("11", "B.Adhes", "user", Some("B.Adhesive")),
    ("13", "F.Paste", "user", None),
    ("15", "B.Paste", "user", None),
    ("5", "F.SilkS", "user", Some("F.Silkscreen")),
    ("7", "B.SilkS", "user", Some("B.Silkscreen")),
    ("1", "F.Mask", "user", None),
    ("3", "B.Mask", "user", None),
    ("17", "Dwgs.User", "user", Some("User.Drawings")),
    ("19", "Cmts.User", "user", Some("User.Comments")),
    ("21", "Eco1.User", "user", Some("User.Eco1")),
    ("23", "Eco2.User", "user", Some("User.Eco2")),
    ("25", "Edge.Cuts", "user", None),
    ("27", "Margin", "user", None),
    ("31", "F.CrtYd", "user", Some("F.Courtyard")),
    ("29", "B.CrtYd", "user", Some("B.Courtyard")),
    ("35", "F.Fab", "user", None),
    ("33", "B.Fab", "user", None),
];

/// An axis-aligned box, `[x0, x1) × [y0, y1)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rect {
    pub x0: i64,
    pub y0: i64,
    pub x1: i64,
    pub y1: i64,
}

impl Rect {
    pub(crate) fn point(x: i64, y: i64) -> Rect {
        Rect {
            x0: x,
            y0: y,
            x1: x,
            y1: y,
        }
    }
    pub(crate) fn union(self, o: Rect) -> Rect {
        Rect {
            x0: self.x0.min(o.x0),
            y0: self.y0.min(o.y0),
            x1: self.x1.max(o.x1),
            y1: self.y1.max(o.y1),
        }
    }
    pub fn grow(self, d: i64) -> Rect {
        Rect {
            x0: self.x0 - d,
            y0: self.y0 - d,
            x1: self.x1 + d,
            y1: self.y1 + d,
        }
    }
    fn shift(self, (dx, dy): (i64, i64)) -> Rect {
        Rect {
            x0: self.x0 + dx,
            y0: self.y0 + dy,
            x1: self.x1 + dx,
            y1: self.y1 + dy,
        }
    }
    /// True when the interiors meet; touching edges do not.
    pub fn overlaps(self, o: Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }
    /// True when `o` lies inside this box, edges included.
    pub fn contains(self, o: Rect) -> bool {
        self.x0 <= o.x0 && self.y0 <= o.y0 && o.x1 <= self.x1 && o.y1 <= self.y1
    }
    pub(crate) fn centre(self) -> (i64, i64) {
        (
            self.x0 + (self.x1 - self.x0) / 2,
            self.y0 + (self.y1 - self.y0) / 2,
        )
    }
}

fn internal(reference: &str, what: impl std::fmt::Display) -> EmitError {
    EmitError::Internal(format!("{reference}: {what}"))
}

/// `(head x y …)`'s first two arguments, in nanometres.
fn xy(item: &Sx, reference: &str) -> Result<(i64, i64), EmitError> {
    let coord = |i| {
        item.arg(i)
            .and_then(sexpr::nm)
            .ok_or_else(|| internal(reference, format!("unreadable `{:?}`", item.head())))
    };
    Ok((coord(1)?, coord(2)?))
}

fn layer(item: &Sx) -> Option<&str> {
    item.child("layer").and_then(|l| l.arg(1))
}

/// The box around a graphic's points: `fp_line`, `fp_rect` or `fp_poly`,
/// and half its stroke on every side; for an `fp_circle` or an `fp_arc`, the box of its whole
/// circle, which holds the arc whatever its sweep (`AET-WO-4300d2`: the
/// XT30PW-M's silkscreen rings its pegs with arcs). Anything else is refused.
fn graphic_box(item: &Sx, reference: &str, theta: u16) -> Result<Rect, EmitError> {
    let mut points = Vec::new();
    match item.head() {
        Some("fp_arc") => {
            let mut at = [(0, 0); 3];
            for (i, end) in ["start", "mid", "end"].into_iter().enumerate() {
                let p = item
                    .child(end)
                    .ok_or_else(|| internal(reference, "an arc without its three points"))?;
                at[i] = xy(p, reference)?;
            }
            let (c, r) = circle(at)
                .ok_or_else(|| internal(reference, "an arc whose three points are in line"))?;
            points.push((c.0 - r, c.1 - r));
            points.push((c.0 + r, c.1 + r));
        }
        // A circle's box, from its centre and a point on it (`AET-WO-1312`: the
        // test pad's courtyard and silkscreen are circles).
        Some("fp_circle") => {
            let at = |end: &str| -> Result<(i64, i64), EmitError> {
                xy(
                    item.child(end).ok_or_else(|| {
                        internal(reference, "a circle without its centre and end")
                    })?,
                    reference,
                )
            };
            let (c, e) = (at("center")?, at("end")?);
            let (dx, dy) = (
                i128::from(e.0) - i128::from(c.0),
                i128::from(e.1) - i128::from(c.1),
            );
            let r = i64::try_from(isqrt_up(dx * dx + dy * dy))
                .map_err(|_| internal(reference, "a circle too large to place"))?;
            points.push((c.0 - r, c.1 - r));
            points.push((c.0 + r, c.1 + r));
        }
        Some("fp_line" | "fp_rect") => {
            for end in ["start", "end"] {
                let at = item
                    .child(end)
                    .ok_or_else(|| internal(reference, "a graphic without its ends"))?;
                points.push(xy(at, reference)?);
            }
        }
        Some("fp_poly") => {
            let pts = item
                .child("pts")
                .ok_or_else(|| internal(reference, "a polygon without points"))?;
            for p in pts.children("xy") {
                points.push(xy(p, reference)?);
            }
        }
        other => {
            return Err(internal(
                reference,
                format!("a courtyard or silkscreen `{other:?}`, which the placer does not measure"),
            ));
        }
    }
    let half = item
        .child("stroke")
        .and_then(|s| s.child("width"))
        .and_then(|w| w.arg(1))
        .and_then(sexpr::nm)
        .unwrap_or(0)
        / 2;
    let first = route::rotate(
        theta,
        *points
            .first()
            .ok_or_else(|| internal(reference, "a graphic with no points"))?,
    );
    Ok(points
        .iter()
        .map(|&p| route::rotate(theta, p))
        .fold(Rect::point(first.0, first.1), |r, (x, y)| {
            r.union(Rect::point(x, y))
        })
        .grow(half))
}

/// The least `d` with `d * d >= n`: an integer square root rounded up, so a
/// distance from it is never understated.
pub(crate) fn isqrt_up(n: i128) -> i128 {
    let mut d = (n as f64).sqrt() as i128;
    while d * d < n {
        d += 1;
    }
    while d > 0 && (d - 1) * (d - 1) >= n {
        d -= 1;
    }
    d
}

/// The circle through three points, in nanometres: its centre rounded to the
/// nearest, and a radius that reaches every point from that centre plus 2 nm,
/// so the rounding can only grow it. `None` when the points are in line.
fn circle(p: [(i64, i64); 3]) -> Option<((i64, i64), i64)> {
    let [(ax, ay), (bx, by), (cx, cy)] = p.map(|(x, y)| (i128::from(x), i128::from(y)));
    let d = 2 * (ax * (by - cy) + bx * (cy - ay) + cx * (ay - by));
    if d == 0 {
        return None;
    }
    let (a2, b2, c2) = (ax * ax + ay * ay, bx * bx + by * by, cx * cx + cy * cy);
    let ux = a2 * (by - cy) + b2 * (cy - ay) + c2 * (ay - by);
    let uy = a2 * (cx - bx) + b2 * (ax - cx) + c2 * (bx - ax);
    // Round half away from zero, whatever the signs.
    let round = |n: i128| {
        let (q, r) = (n / d, n % d);
        if 2 * r.abs() >= d.abs() {
            q + if (r < 0) == (d < 0) { 1 } else { -1 }
        } else {
            q
        }
    };
    let (x, y) = (round(ux), round(uy));
    let far = [(ax, ay), (bx, by), (cx, cy)]
        .iter()
        .map(|&(px, py)| (px - x) * (px - x) + (py - y) * (py - y))
        .max()?;
    // The integer square root, rounded up.
    Some((
        (i64::try_from(x).ok()?, i64::try_from(y).ok()?),
        i64::try_from(isqrt_up(far) + 2).ok()?,
    ))
}

/// A visible silkscreen text's box: every glyph as wide as KiCad 10.0.7's
/// widest, `m`, whose advance is 4/3 of the font width (measured by Gerber
/// plot in the tools image, `AET-WO-1301` adversarial pass F3), plus the
/// stroke all round.
fn text_box(item: &Sx, text: &str, reference: &str, theta: u16) -> Result<Rect, EmitError> {
    let at = item
        .child("at")
        .ok_or_else(|| internal(reference, "a text without `at`"))?;
    // The text turns whole with the footprint ([`turn`]).
    let (x, y) = route::rotate(theta, xy(at, reference)?);
    let font = item
        .child("effects")
        .and_then(|e| e.child("font"))
        .ok_or_else(|| internal(reference, "a text without a font"))?;
    let size = font
        .child("size")
        .ok_or_else(|| internal(reference, "a font without a size"))?;
    let (w, h) = xy(size, reference)?;
    let thick = font
        .child("thickness")
        .and_then(|t| t.arg(1))
        .and_then(sexpr::nm)
        .unwrap_or(h / 8);
    let chars = i64::try_from(text.chars().count()).unwrap_or(i64::MAX / 4);
    let (mut hw, mut hh) = (chars.saturating_mul(w) * 2 / 3 + thick, h / 2 + thick);
    let own = route::angle(at.arg(3)).ok_or_else(|| {
        internal(
            reference,
            format!("a text at angle {}", at.arg(3).unwrap_or("?")),
        )
    })?;
    if matches!((own + theta) % 360, 90 | 270) {
        std::mem::swap(&mut hw, &mut hh);
    }
    Ok(Rect {
        x0: x - hw,
        y0: y - hh,
        x1: x + hw,
        y1: y + hh,
    })
}

/// What the placer needs of a footprint, relative to its origin.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    /// The front courtyard's box.
    pub courtyard: Rect,
    /// The courtyard, the silkscreen graphics and the visible silkscreen
    /// text, the reference as this part writes it.
    pub keep: Rect,
}

/// `fp`'s geometry with `reference` as its reference text.
pub fn geometry(fp: &Sx, reference: &str) -> Result<Geometry, EmitError> {
    geometry_at(fp, reference, 0)
}

/// [`geometry`] of the footprint rotated by `theta` about its origin.
pub fn geometry_at(fp: &Sx, reference: &str, theta: u16) -> Result<Geometry, EmitError> {
    let mut courtyard: Option<Rect> = None;
    let mut silk: Option<Rect> = None;
    let join = |acc: &mut Option<Rect>, r: Rect| *acc = Some(acc.map_or(r, |a| a.union(r)));
    for item in fp.items() {
        let hidden = item.child("hide").and_then(|h| h.arg(1)) == Some("yes");
        match (item.head(), layer(item)) {
            (Some("fp_line" | "fp_rect" | "fp_poly" | "fp_arc" | "fp_circle"), Some("F.CrtYd")) => {
                join(&mut courtyard, graphic_box(item, reference, theta)?);
            }
            (Some("fp_line" | "fp_rect" | "fp_poly" | "fp_arc" | "fp_circle"), Some("F.SilkS")) => {
                join(&mut silk, graphic_box(item, reference, theta)?);
            }
            (Some("property"), Some("F.SilkS")) if !hidden => {
                let text = if item.arg(1) == Some("Reference") {
                    reference
                } else {
                    item.arg(2).unwrap_or("")
                };
                join(&mut silk, text_box(item, text, reference, theta)?);
            }
            (Some("fp_text"), Some("F.SilkS")) if !hidden => {
                join(
                    &mut silk,
                    text_box(item, item.arg(2).unwrap_or(""), reference, theta)?,
                );
            }
            _ => {}
        }
    }
    let courtyard =
        courtyard.ok_or_else(|| internal(reference, "a footprint with no courtyard"))?;
    Ok(Geometry {
        courtyard,
        keep: silk.map_or(courtyard, |s| s.union(courtyard)),
    })
}

/// KiCad's escaping of a net name (`EscapeString`, `CTX_NETNAME`): `/` is
/// `{slash}`, line breaks are dropped.
pub(crate) fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '/' => out.push_str("{slash}"),
            '\n' | '\r' => {}
            c => out.push(c),
        }
    }
    out
}

/// Each pin number's net, as KiCad's netlist names it.
pub(crate) fn pin_nets(part: &Part) -> Result<BTreeMap<String, String>, EmitError> {
    let sym = library::symbol(part.lib_id)?;
    let pins = schematic::pins(&sym).map_err(EmitError::Internal)?;
    let mut at_point: BTreeMap<(i64, i64), usize> = BTreeMap::new();
    for p in &pins {
        *at_point.entry(p.at).or_default() += 1;
    }
    let mut out = BTreeMap::new();
    for p in &pins {
        let net = match part.conns.get(&p.number) {
            // KiCad's parity check matches an `unconnected-` net by prefix,
            // so the gate refuses a schematic naming one (`parity`).
            Some(Conn::Net(name) | Conn::Rail(name)) if name.starts_with(parity::UNCONNECTED) => {
                return Err(EmitError::ReservedName(name.clone()));
            }
            Some(Conn::Net(label)) => format!("/{}", escape(label)),
            Some(Conn::Rail(rail)) => escape(rail),
            Some(Conn::NoConnect) | None => {
                // KiCad names stacked unconnected pins after one of them;
                // which one is not modelled here, so it is refused.
                if at_point[&p.at] > 1 {
                    return Err(internal(
                        &part.reference,
                        format!("pin {} is a stacked unconnected pin", p.number),
                    ));
                }
                let shown = if p.name == "~" { "" } else { p.name.as_str() };
                if !shown.is_empty() && shown != p.number {
                    format!(
                        "unconnected-({}-{}-Pad{})",
                        part.reference,
                        escape(shown),
                        escape(&p.number)
                    )
                } else {
                    format!("unconnected-({}-Pad{})", part.reference, escape(&p.number))
                }
            }
        };
        // belt-and-suspenders: no vendored symbol repeats a pin number (so no
        // test reaches this), but one that did would give two pins one pad.
        if out.insert(p.number.clone(), net).is_some() {
            return Err(internal(
                &part.reference,
                format!("pin number {} twice", p.number),
            ));
        }
    }
    Ok(out)
}

/// One footprint on the board.
#[derive(Clone, Debug)]
pub struct Placed {
    pub reference: String,
    pub origin: (i64, i64),
    /// 0, 90, 180 or 270 degrees ([`route::rotate`]); 0 from the placer.
    pub rotation: u16,
    /// Board coordinates.
    pub courtyard: Rect,
    pub keep: Rect,
}

struct Item<'a> {
    part: &'a Part,
    fp: Sx,
    /// The footprint's turn: an IMU's from its seat (`AET-WO-4400d`), else 0.
    rotation: u16,
    /// At `rotation`.
    geometry: Geometry,
    /// By pad number, the pads that carry a net: (position at `rotation`,
    /// net).
    pads: BTreeMap<String, ((i64, i64), String)>,
}

/// A pad number's order: numerically where it is a number, then as text.
fn pad_order(n: &str) -> (u64, &str) {
    (n.parse().unwrap_or(u64::MAX), n)
}

impl Item<'_> {
    /// The nets this part shares with `other`, ground and unconnected nets
    /// aside: label nets first, then rails, each by name.
    fn shared(&self, other: &Item<'_>) -> Vec<String> {
        let mine = self.nets();
        let mut shared: Vec<String> = other.nets().intersection(&mine).cloned().collect();
        shared.sort_by_key(|n| (!n.starts_with('/'), n.clone()));
        shared
    }

    fn nets(&self) -> BTreeSet<String> {
        self.pads
            .values()
            .map(|(_, n)| n.clone())
            .filter(|n| n != GND && !n.starts_with("unconnected-"))
            .collect()
    }

    /// The lowest-numbered pad on `net`, relative to the origin.
    fn pad_on(&self, net: &str) -> Option<(i64, i64)> {
        self.pads
            .iter()
            .filter(|(_, (_, n))| n == net)
            .min_by_key(|(number, _)| pad_order(number))
            .map(|(_, (at, _))| *at)
    }
}

/// The MCU, by its symbol's library.
fn is_mcu(part: &Part) -> bool {
    part.lib_id.starts_with("MCU_")
}

/// Whether `part` is anchored to the IC whose reference is `ic`.
fn anchored_to(part: &Part, ic: &str) -> bool {
    part.owner
        .as_ref()
        .is_some_and(|o| o.anchor.is_some() && o.ic == ic)
}

fn is_ic(part: &Part) -> bool {
    !part.lib_id.starts_with("Device:") && !is_bench(part)
}

/// A test point or the reset switch (`AET-WO-1312`): never an anchor, and
/// placed after every other part but the connectors, so that declaring one
/// moves no other part.
fn is_bench(part: &Part) -> bool {
    part.lib_id == schematic::TEST_POINT || part.lib_id == schematic::SWITCH
}

/// The integer placer (`AET-WO-1301` §1.2), a function of the parts alone:
/// 1. the MCU's origin goes at [`ORIGIN`];
/// 2. every other IC, in reference order, goes beside the placed IC it
///    shares most nets with;
/// 3. every passive, in reference order, goes beside the IC it shares most
///    nets with, or a decoupling capacitor beside the IC it decouples;
/// 4. the reset switch and every test point, in reference order, the same
///    way (`AET-WO-1312`).
///
/// Ground and unconnected nets are not counted, and ties go to the lower
/// reference. "Beside" means: the first free spot, in rings of [`STEP`] out
/// from the target, where the part's keep-out box clears every placed one
/// by [`GAP_NM`]. The target is the anchor's lowest-numbered pad on the
/// first shared net (label nets before rails), so a decoupling capacitor
/// sits by the power pin its rail names; with no shared net, the anchor's
/// courtyard centre.
///
/// Connectors are left to [`place_connectors`], after every other part, so
/// none is an anchor; the board's bottom edge is returned when there is one.
///
/// Under a declared bound (`AET-WO-1313c`), and only there, three things
/// change, so that no name and no list position decides a placement:
/// - (k) the canonical rank ([`canonical_rank`]) takes the place of
///   reference order everywhere above, and in the connectors' edges;
/// - (m) the target is the centroid of the anchor's pads on every shared
///   net ([`centroid_target`]);
/// - (s) the placed parts are compacted toward the MCU ([`compact`]) before
///   the connectors go on.
fn place(
    items: &[Item<'_>],
    pairs: &[[String; 2]],
    bounded: bool,
    seed: u64,
) -> Result<(Vec<Placed>, Edges), EmitError> {
    // Under a declared bound only, every choice that reference order makes
    // below is the canonical rank's instead (`AET-WO-1313c`, (k)).
    let rank = bounded.then(|| canonical_rank(items, seed));
    let rank = rank.as_deref();
    let order = placing_order(items, rank);
    // An anchored part goes straight after its IC, in drawing order, before
    // any other part can crowd the pins it serves (`AET-WO-4300d2` §1.3). A
    // design with no anchor keeps the order above. Drawing order is the
    // anchor's own sequence, never the part's place in the list
    // (`AET-WO-1313c` F1).
    let anchored = |i: usize| {
        let o = items[i].part.owner.as_ref()?;
        o.anchor.as_ref().map(|a| (o.ic.as_str(), a.seq))
    };
    let mut grouped = Vec::with_capacity(order.len());
    for &i in order.iter().filter(|&&i| anchored(i).is_none()) {
        grouped.push(i);
        let reference = items[i].part.reference.as_str();
        // `seq` is unique among one IC's anchored parts (the emitter counts
        // them), so the reference never decides here: it only keeps a
        // hand-built design that repeats a `seq` deterministic.
        let mut own: Vec<(usize, &str, usize)> = (0..items.len())
            .filter_map(|j| match anchored(j) {
                Some((ic, seq)) if ic == reference => {
                    Some((seq, items[j].part.reference.as_str(), j))
                }
                _ => None,
            })
            .collect();
        own.sort_unstable();
        grouped.extend(own.into_iter().map(|(_, _, j)| j));
    }
    if grouped.len() != order.len() {
        return Err(EmitError::Internal(
            "a part is anchored to an IC that is not drawn".into(),
        ));
    }
    let order = grouped;
    let mut placed: Vec<Option<Placed>> = vec![None; items.len()];
    // What compaction needs of each placed part: the IC it went beside, and
    // the gap it kept from the parts placed before it.
    let mut anchor_of: Vec<Option<usize>> = vec![None; items.len()];
    let mut rule: Vec<Rule> = vec![Rule::default(); items.len()];
    // The approach to each declared pair's MCU pins, kept clear of every
    // part placed after the MCU (`AET-WO-4300g` D5, lever 2; `AET-WO-1310`).
    let mut approach: Vec<Rect> = Vec::new();
    for (step, &i) in order.iter().enumerate() {
        let item = &items[i];
        let g = item.geometry;
        if step == 0 {
            if !is_mcu(item.part) {
                return Err(EmitError::Internal("the design has no MCU".into()));
            }
            placed[i] = Some(Placed {
                reference: item.part.reference.clone(),
                origin: ORIGIN,
                rotation: item.rotation,
                courtyard: g.courtyard.shift(ORIGIN),
                keep: g.keep.shift(ORIGIN),
            });
            approach = approaches(item, g.courtyard.shift(ORIGIN), pairs);
            rule[i] = Rule {
                reserve: None,
                ic: true,
                step,
            };
            continue;
        }
        // The anchor: an IC already placed.
        let candidates =
            (0..items.len()).filter(|&j| j != i && placed[j].is_some() && is_ic(items[j].part));
        let anchor = match item.part.owner.as_ref().map(|o| &o.ic) {
            Some(ic) => (0..items.len())
                .find(|&j| placed[j].is_some() && &items[j].part.reference == ic)
                .ok_or_else(|| {
                    internal(
                        &item.part.reference,
                        format!("decouples {ic}, which is not placed"),
                    )
                })?,
            None => candidates
                .max_by(|&a, &b| {
                    let count = |j: usize| item.shared(&items[j]).len();
                    count(a).cmp(&count(b)).then_with(|| match rank {
                        Some(rank) => rank[b].cmp(&rank[a]),
                        None => items[b].part.reference.cmp(&items[a].part.reference),
                    })
                })
                .ok_or_else(|| internal(&item.part.reference, "no IC to place beside"))?,
        };
        let at = placed[anchor].as_ref().map(|p| p.origin).unwrap_or(ORIGIN);
        // An IC that owns anchored parts keeps its loop's room from every part
        // placed before it (`AET-WO-4300d2` §1.3): the longest side of its
        // largest anchored part, plus [`IC_GAP_NM`], so that part can sit on
        // any side of the pins it serves. ICs packed at [`IC_GAP_NM`] took
        // that room on the Sentinel-02 fixture (measured: FEEDBACK §1.4).
        let reference = item.part.reference.as_str();
        let reserve = items
            .iter()
            .filter(|x| anchored_to(x.part, reference))
            .map(|x| {
                let k = x.geometry.keep;
                (k.x1 - k.x0).max(k.y1 - k.y0) + IC_GAP_NM
            })
            .max();
        anchor_of[i] = Some(anchor);
        rule[i] = Rule {
            reserve,
            ic: is_ic(item.part),
            step,
        };
        let taken: Vec<(Rect, i64)> = (0..items.len())
            .filter_map(|j| {
                placed[j]
                    .as_ref()
                    .map(|p| (p.keep, Rule::gap(rule[i], rule[j])))
            })
            .chain(approach.iter().map(|&r| (r, 0)))
            .collect();
        if let Some(a) = item.part.owner.as_ref().and_then(|o| o.anchor.as_ref()) {
            placed[i] = Some(place_anchored(item, &items[anchor], at, a, &taken)?);
            continue;
        }
        let target = if bounded {
            centroid_target(item, &items[anchor], at)
        } else {
            item.shared(&items[anchor])
                .first()
                .and_then(|net| items[anchor].pad_on(net))
                .map(|(x, y)| (at.0 + x, at.1 + y))
        }
        .unwrap_or_else(|| {
            placed[anchor]
                .as_ref()
                .map_or(ORIGIN, |p| p.courtyard.centre())
        });
        let origin = spot(g, target, &taken)
            .ok_or_else(|| internal(&item.part.reference, "no free spot within a metre"))?;
        placed[i] = Some(Placed {
            reference: item.part.reference.clone(),
            origin,
            rotation: item.rotation,
            courtyard: g.courtyard.shift(origin),
            keep: g.keep.shift(origin),
        });
    }
    if let Some(rank) = rank {
        compact(items, &mut placed, &anchor_of, &rule, &approach, rank)?;
    }
    let plan = if bounded {
        BOUNDED_PLAN
    } else {
        EdgePlan::Bottom { gap: IC_GAP_NM }
    };
    let edges = place_connectors(items, &mut placed, plan, rank)?;
    let placed = placed
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| EmitError::Internal("a part was not placed".into()))?;
    Ok((placed, edges))
}

/// Where the connectors go (`AET-WO-1313` D1). With no bound declared, the
/// one row along the bottom edge, [`IC_GAP_NM`] apart (`AET-WO-4300f`).
/// Under a bound, [`BOUNDED_PLAN`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EdgePlan {
    /// Every connector on the bottom edge, cable down, `gap` apart.
    Bottom { gap: i64 },
    /// The left and right edges, cable out: each connector, in the
    /// canonical rank's order ([`canonical_rank`]), to the side with less
    /// length so far (ties left).
    Sides,
}

/// The plan under a declared bound: (c), ruled. Every cable on the bottom
/// edge instead, if Scotty rules it (Q1), is (d): `EdgePlan::Bottom { gap:
/// GAP_NM }`.
const BOUNDED_PLAN: EdgePlan = EdgePlan::Sides;

/// Each connector's rotation under `plan`, cable side out: 0 on the bottom
/// edge (the cable toward +y), 90 on the right, 270 on the left
/// ([`route::rotate`]); and the gap between neighbours on one edge.
fn connector_edges(
    items: &[Item<'_>],
    plan: EdgePlan,
    rank: Option<&[usize]>,
) -> (Vec<(usize, u16)>, i64) {
    let mut row: Vec<usize> = (0..items.len())
        .filter(|&i| schematic::is_connector(items[i].part))
        .collect();
    match rank {
        Some(rank) => row.sort_by_key(|&i| rank[i]),
        None => row.sort_by(|&a, &b| items[a].part.reference.cmp(&items[b].part.reference)),
    }
    match plan {
        EdgePlan::Bottom { gap } => (row.into_iter().map(|i| (i, 0)).collect(), gap),
        EdgePlan::Sides => {
            let (mut left, mut right) = (0i64, 0i64);
            let sides = row
                .into_iter()
                .map(|i| {
                    let k = items[i].geometry.keep;
                    let length = k.x1 - k.x0 + IC_GAP_NM;
                    if left <= right {
                        left += length;
                        (i, 270)
                    } else {
                        right += length;
                        (i, 90)
                    }
                })
                .collect();
            (sides, IC_GAP_NM)
        }
    }
}

/// The edges the connectors set, each where its row's cable side is.
#[derive(Clone, Copy, Debug, Default)]
struct Edges {
    bottom: Option<i64>,
    left: Option<i64>,
    right: Option<i64>,
}

/// A bound's width and height in nanometres.
fn bound_nm(b: OutlineBound) -> (i64, i64) {
    (i64::from(b.width) * 1000, i64::from(b.height) * 1000)
}

/// `AET-WO-1313` §2.1: whether `bound` can hold `parts` at all, before any
/// placement. Inside the bound less [`MARGIN_NM`] on each side, every
/// keep-out box must fit at one of its right angles, and their areas must
/// fit together. Passing says nothing of whether the placer reaches it:
/// that is [`EmitError::OutlineUnreachable`].
pub(crate) fn bound_holds(parts: &[Part], bound: OutlineBound) -> Result<(), EmitError> {
    let items = items(parts)?;
    let (bw, bh) = bound_nm(bound);
    let inner = (bw - 2 * MARGIN_NM, bh - 2 * MARGIN_NM);
    let refuse = |reference: Option<String>, why: String| EmitError::OutlineTooSmall {
        width_um: bound.width,
        height_um: bound.height,
        reference,
        why,
    };
    let (short, long) = (inner.0.min(inner.1), inner.0.max(inner.1));
    if short <= 0 {
        return Err(refuse(
            None,
            format!("less its {MARGIN_NM} nm margin on each side, nothing fits"),
        ));
    }
    let mut sum: i128 = 0;
    for it in &items {
        let k = it.geometry.keep;
        let (w, h) = (k.x1 - k.x0, k.y1 - k.y0);
        if w.min(h) > short || w.max(h) > long {
            return Err(refuse(
                Some(it.part.reference.clone()),
                format!(
                    "`{}`'s keep-out box, {} x {} nm, does not fit inside {} x {} nm (the bound less its margins) at any right angle",
                    it.part.reference, w, h, inner.0, inner.1
                ),
            ));
        }
        sum += i128::from(w) * i128::from(h);
    }
    let room = i128::from(inner.0) * i128::from(inner.1);
    if sum > room {
        return Err(refuse(
            None,
            format!(
                "the parts' keep-out boxes take {} mm2 together, more than the {} mm2 inside the bound less its margins",
                sum / 1_000_000_000_000,
                room / 1_000_000_000_000
            ),
        ));
    }
    Ok(())
}

/// The order [`place`] takes every part but the connectors in, before an
/// anchored part is moved to follow its IC. With no bound declared: the MCU,
/// the other ICs, the passives, the bench parts, each in reference order.
/// Under a declared bound, the canonical rank ([`canonical_rank`]), which
/// keeps those four classes and puts an IC that owns anchored parts before
/// the other ICs (`AET-WO-1313` D1, (r)).
fn placing_order(items: &[Item<'_>], rank: Option<&[usize]>) -> Vec<usize> {
    let mut order: Vec<usize> = (0..items.len())
        .filter(|&i| !schematic::is_connector(items[i].part))
        .collect();
    match rank {
        Some(rank) => order.sort_by_key(|&i| rank[i]),
        None => order.sort_by(|&a, &b| {
            let key = |i: usize| {
                let p = items[i].part;
                (!is_mcu(p), !is_ic(p), is_bench(p), p.reference.as_str())
            };
            key(a).cmp(&key(b))
        }),
    }
    order
}

/// FNV-1a, 64 bits: the canonical rank's colours. Any fixed hash would do;
/// this one is a few lines and needs no dependency.
const FNV_BASIS: u64 = 0xcbf2_9ce4_8422_2325;

fn fnv(mut h: u64, bytes: &[u8]) -> u64 {
    for b in bytes {
        h ^= u64::from(*b);
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    h
}

/// How many rounds of colour refinement the canonical rank takes: each
/// round sees one net further. Three is the prototype's, which `AET-WO-1313b`
/// measured (§2.2). The colours' values order parts, not only tell them
/// apart, so another count places differently: at four, the Sentinel-03
/// fixture and every size sample move (measured, `AET-WO-1313c` FEEDBACK
/// §3).
const COLOUR_ROUNDS: usize = 3;

/// `AET-WO-1313c`, (k): each item's canonical rank, a total order computed
/// from footprints, ownership, the draw sequence ([`schematic::Anchor`]'s
/// `seq`) and the net graph alone. A net's kind (a label net, a rail,
/// ground, unconnected) is read from the form of its name, never its text;
/// a reference decides only a tie that remains.
///
/// Each item's colour starts as a hash of its symbol and footprint, and is
/// refined [`COLOUR_ROUNDS`] times from the colours on each of its pads'
/// nets, its owner's colour, whether it decouples, and its anchor's
/// sequence and pad pairs (Weisfeiler-Lehman colour refinement). The rank
/// is by: the MCU; the other ICs; the passives; the bench parts, a `GND`
/// test point last; an IC that owns anchored parts first (`AET-WO-1313`
/// (r)); the larger courtyard first; the nearer to the MCU over label nets;
/// the colour; the colours of the parts it owns; and last, the reference.
///
/// `seed` 0 is the rank as `AET-WO-1313c` built it. Any other seed is hashed
/// in ahead of every symbol and footprint ([`layout_of`]'s retries,
/// `AET-WO-1317`): two items' colours are equal under every seed or under
/// none (a 64-bit collision aside), so a seed reorders only what the colours
/// order, and the reference decides the same ties it did.
fn canonical_rank(items: &[Item<'_>], seed: u64) -> Vec<usize> {
    let real = |n: &str| !n.starts_with("unconnected-");
    let mut nets: BTreeMap<&str, Vec<(usize, &str)>> = BTreeMap::new();
    for (i, it) in items.iter().enumerate() {
        for (number, (_, net)) in &it.pads {
            if real(net) {
                nets.entry(net.as_str())
                    .or_default()
                    .push((i, number.as_str()));
            }
        }
    }
    let start = match seed {
        0 => FNV_BASIS,
        s => fnv(FNV_BASIS, &s.to_le_bytes()),
    };
    let base: Vec<u64> = items
        .iter()
        .map(|it| {
            fnv(
                fnv(start, it.part.lib_id.as_bytes()),
                it.part.footprint.as_bytes(),
            )
        })
        .collect();
    let owner_of: Vec<Option<usize>> = items
        .iter()
        .map(|it| {
            let o = it.part.owner.as_ref()?;
            items.iter().position(|x| x.part.reference == o.ic)
        })
        .collect();
    let mut colour = base.clone();
    for _ in 0..COLOUR_ROUNDS {
        // A net's colour: its pads' colours and pad numbers, as a multiset.
        let mut net_colour: BTreeMap<&str, u64> = BTreeMap::new();
        for (name, pads) in &nets {
            let mut v: Vec<(u64, &str)> = pads.iter().map(|&(i, n)| (colour[i], n)).collect();
            v.sort_unstable();
            let mut h = FNV_BASIS;
            for (c, n) in v {
                h = fnv(fnv(fnv(h, &c.to_le_bytes()), n.as_bytes()), b"|");
            }
            net_colour.insert(name, h);
        }
        colour = items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let mut v: Vec<(&str, u64)> = it
                    .pads
                    .iter()
                    .filter(|(_, (_, n))| real(n))
                    .map(|(number, (_, n))| (number.as_str(), net_colour[n.as_str()]))
                    .collect();
                v.sort_unstable();
                let mut h = base[i];
                for (number, c) in v {
                    h = fnv(fnv(h, number.as_bytes()), &c.to_le_bytes());
                }
                // Ownership: the owner's colour, whether it decouples, the
                // anchor's sequence and pad pairs; never the owner's name.
                if let Some(o) = it.part.owner.as_ref() {
                    if let Some(j) = owner_of[i] {
                        h = fnv(h, &colour[j].to_le_bytes());
                    }
                    h = fnv(h, if o.decouples { b"d" } else { b"-" });
                    if let Some(a) = o.anchor.as_ref() {
                        h = fnv(h, &a.seq.to_le_bytes());
                        for (own, theirs) in &a.pads {
                            h = fnv(fnv(fnv(h, own.as_bytes()), b":"), theirs.as_bytes());
                        }
                    }
                }
                h
            })
            .collect();
    }
    // Each item's distance from the MCU in the net graph, over label nets
    // only: a rail or ground reaches every part at once.
    let mut distance = vec![usize::MAX; items.len()];
    if let Some(m) = items.iter().position(|it| is_mcu(it.part)) {
        distance[m] = 0;
        let mut frontier = vec![m];
        let mut d = 0;
        while !frontier.is_empty() {
            d += 1;
            let mut next = Vec::new();
            for &i in &frontier {
                for (_, net) in items[i].pads.values() {
                    if !net.starts_with('/') {
                        continue;
                    }
                    for &(j, _) in &nets[net.as_str()] {
                        if distance[j] == usize::MAX {
                            distance[j] = d;
                            next.push(j);
                        }
                    }
                }
            }
            frontier = next;
        }
    }
    let owns_anchored = |i: usize| {
        items
            .iter()
            .any(|x| anchored_to(x.part, &items[i].part.reference))
    };
    // What each item owns: its owned parts' colours, as a multiset. An IC's
    // own colour never sees them, so two ICs alike in footprint and nets,
    // owning different parts, would otherwise tie to their references.
    let owned: Vec<u64> = (0..items.len())
        .map(|i| {
            let mut v: Vec<u64> = (0..items.len())
                .filter(|&j| owner_of[j] == Some(i))
                .map(|j| colour[j])
                .collect();
            v.sort_unstable();
            v.iter().fold(FNV_BASIS, |h, c| fnv(h, &c.to_le_bytes()))
        })
        .collect();
    let ground_test_point = |i: usize| {
        items[i].part.lib_id == schematic::TEST_POINT
            && items[i].pads.values().all(|(_, n)| n == GND)
    };
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |i: usize| {
            let p = items[i].part;
            let c = items[i].geometry.courtyard;
            (
                !is_mcu(p),
                !is_ic(p),
                is_bench(p),
                ground_test_point(i),
                !owns_anchored(i),
                std::cmp::Reverse(i128::from(c.x1 - c.x0) * i128::from(c.y1 - c.y0)),
                distance[i],
                colour[i],
                owned[i],
            )
        };
        key(a)
            .cmp(&key(b))
            .then_with(|| items[a].part.reference.cmp(&items[b].part.reference))
    });
    let mut rank = vec![0; items.len()];
    for (r, &i) in order.iter().enumerate() {
        rank[i] = r;
    }
    rank
}

/// `AET-WO-1313c`, (m): a part's target under a declared bound, the
/// centroid of its anchor's pads on every net they share, from no pad's
/// number or net's name; `None` when they share none.
fn centroid_target(item: &Item<'_>, anchor: &Item<'_>, at: (i64, i64)) -> Option<(i64, i64)> {
    let shared = item.shared(anchor);
    let pads: Vec<(i64, i64)> = anchor
        .pads
        .values()
        .filter(|(_, n)| shared.contains(n))
        .map(|(p, _)| *p)
        .collect();
    let n = i64::try_from(pads.len()).ok().filter(|&n| n > 0)?;
    let (sx, sy) = pads.iter().fold((0, 0), |s, p| (s.0 + p.0, s.1 + p.1));
    Some((at.0 + sx / n, at.1 + sy / n))
}

/// The gap a placed part kept from the parts placed before it ([`place`]):
/// its loop's reserve if it owns anchored parts, else [`IC_GAP_NM`] between
/// two ICs, else [`GAP_NM`]; and when it was placed.
#[derive(Clone, Copy, Default)]
struct Rule {
    reserve: Option<i64>,
    ic: bool,
    step: usize,
}

impl Rule {
    /// The gap between two placed parts: the one placed later set it.
    fn gap(a: Rule, b: Rule) -> i64 {
        let later = if a.step >= b.step { a } else { b };
        match (later.reserve, a.ic && b.ic) {
            (Some(r), _) => r,
            (None, true) => IC_GAP_NM,
            (None, false) => GAP_NM,
        }
    }
}

/// Moves a placed part by `d`.
fn shift(p: &mut Placed, d: (i64, i64)) {
    p.origin = (p.origin.0 + d.0, p.origin.1 + d.1);
    p.keep = p.keep.shift(d);
    p.courtyard = p.courtyard.shift(d);
}

/// How far a group may step sideways, in [`STEP`]s, to find a free column
/// for its step toward the MCU: 10 mm, the prototype's measured setting
/// (`AET-WO-1313b` §2.2).
const SIDESTEP_STEPS: i64 = 40;

/// `AET-WO-1313c`, (s): sidestep compaction, under a declared bound only.
/// A group is an IC other than the MCU with every non-IC part placed beside
/// it, moved rigidly, so every anchored part keeps its place by its pins.
/// In rank order, each group takes one [`STEP`] toward the MCU's courtyard
/// centre in y, or, when that is blocked, the same step together with the
/// nearest free sideways offset (left first, up to [`SIDESTEP_STEPS`]); or,
/// with no step in y to take, one step in x. A step never overshoots the
/// centre. A move must keep every member:
/// - at the gap the placer kept ([`Rule::gap`]) from every part outside the
///   group;
/// - out of every approach strip ([`approaches`]);
/// - inside the x-range the parts had before compaction, so the core never
///   widens.
///
/// `AET-WO-1313d`: compaction checks its own termination argument as it runs.
/// Each move must lower the moving group's potential strictly, and the moves
/// may not pass [`move_bound`], computed before the first move. Either
/// failure is [`EmitError::CompactionUnbounded`], with nothing written.
fn compact(
    items: &[Item<'_>],
    placed: &mut [Option<Placed>],
    anchor_of: &[Option<usize>],
    rule: &[Rule],
    approach: &[Rect],
    rank: &[usize],
) -> Result<(), EmitError> {
    // How a member moves: [`shift`]; a unit test may put a broken mover in
    // its own thread's place (the seam, test builds only).
    #[cfg(not(test))]
    let mv = shift;
    #[cfg(test)]
    let mv = tests::MOVE.with(std::cell::Cell::get);
    let mcu = items.iter().position(|it| is_mcu(it.part));
    let Some(target) = mcu
        .and_then(|i| placed[i].as_ref())
        .map(|p| p.courtyard.centre())
    else {
        return Ok(());
    };
    // Connectors are not placed yet, so none is a group.
    let mut groups: Vec<Vec<usize>> = (0..items.len())
        .filter(|&i| Some(i) != mcu && is_ic(items[i].part) && placed[i].is_some())
        .map(|i| {
            let mut members = vec![i];
            members.extend(
                (0..items.len()).filter(|&j| anchor_of[j] == Some(i) && !is_ic(items[j].part)),
            );
            members
        })
        .collect();
    groups.sort_by_key(|members| rank[members[0]]);
    let Some(core) = placed.iter().flatten().map(|p| p.keep).reduce(Rect::union) else {
        return Ok(());
    };
    let clear = |placed: &[Option<Placed>], members: &[usize], d: (i64, i64)| {
        members.iter().all(|&m| {
            let Some(p) = placed[m].as_ref() else {
                return true;
            };
            let k = p.keep.shift(d);
            k.x0 >= core.x0
                && k.x1 <= core.x1
                && approach.iter().all(|a| !a.overlaps(k))
                && (0..items.len()).all(|o| {
                    members.contains(&o)
                        || placed[o]
                            .as_ref()
                            .is_none_or(|q| !q.keep.overlaps(k.grow(Rule::gap(rule[m], rule[o]))))
                })
        })
    };
    let toward = |v: i64| {
        if v.abs() >= STEP {
            v.signum() * STEP
        } else {
            0
        }
    };
    let centre = |placed: &[Option<Placed>], members: &[usize]| {
        members
            .iter()
            .filter_map(|&m| placed[m].as_ref().map(|p| p.keep))
            .reduce(Rect::union)
            .map(Rect::centre)
    };
    // This ends: a group's potential, (|dy|, |dx|) from its keep-out box's
    // centre to the target, falls strictly (lexicographically) with every
    // move it makes, since a move in y takes one STEP off |dy| with no
    // overshoot, and a move in x alone takes one STEP off |dx|; |dy| is
    // bounded, and |dx| by the core's fixed x-range. A pass in which no
    // group moves is the last.
    let bound = move_bound(
        groups
            .iter()
            .filter_map(|members| centre(placed, members).map(|c| potential(c, target))),
        core.x1 - core.x0,
    );
    let mut moves = 0u64;
    loop {
        let mut moved = false;
        for members in &groups {
            let Some(c) = centre(placed, members) else {
                continue;
            };
            let sy = toward(target.1 - c.1);
            let mut d = None;
            if sy != 0 {
                d = std::iter::once((0, sy))
                    .chain((1..=SIDESTEP_STEPS).flat_map(|j| [-1, 1].map(|s| (s * j * STEP, sy))))
                    .find(|&d| clear(placed, members, d));
            }
            if d.is_none() {
                let sx = toward(target.0 - c.0);
                if sx != 0 && clear(placed, members, (sx, 0)) {
                    d = Some((sx, 0));
                }
            }
            if let Some(d) = d {
                for &m in members {
                    if let Some(p) = placed[m].as_mut() {
                        mv(p, d);
                    }
                }
                moved = true;
                moves += 1;
                let before = potential(c, target);
                let after = centre(placed, members).map_or(before, |c| potential(c, target));
                let why = if after >= before {
                    Some("the move did not lower its potential")
                } else if moves > bound {
                    Some("the moves passed the bound its termination argument gives")
                } else {
                    None
                };
                if let Some(why) = why {
                    return Err(EmitError::CompactionUnbounded {
                        reference: items[members[0]].part.reference.clone(),
                        dy_nm: after.0,
                        dx_nm: after.1,
                        moves,
                        bound,
                        why,
                    });
                }
            }
        }
        if !moved {
            return Ok(());
        }
    }
}

/// A group's potential, `(|dy|, |dx|)` from its keep-out box's centre `c` to
/// the target, compared lexicographically ([`compact`]).
fn potential(c: (i64, i64), target: (i64, i64)) -> (i64, i64) {
    ((target.1 - c.1).abs(), (target.0 - c.0).abs())
}

/// `AET-WO-1313d`: the most moves [`compact`] can make, from each group's
/// starting potential `(|dy0|, |dx0|)` and the core's width `w`.
///
/// Proof, from the termination argument:
/// - A move in y is taken only when `|dy| >= STEP`, and takes exactly one
///   `STEP` off `|dy|`, so a group makes at most `ny = |dy0| / STEP` of them.
/// - A move in x leaves `|dy|` alone and takes exactly one `STEP` off `|dx|`,
///   which is never below 0. So the moves in x number at most `|dx0| / STEP`
///   plus every step a move in y adds to `|dx|`.
/// - A move in y moves sideways by at most [`SIDESTEP_STEPS`] steps, so it
///   adds at most that many: the moves in x number at most
///   `|dx0| / STEP + ny * SIDESTEP_STEPS`.
/// - Also, the group's centre and the target (the MCU's courtyard centre)
///   both stay inside the core's x-range, which no move leaves, so
///   `|dx| <= w` after every move in y: the moves in x after each of the `ny`
///   number at most `w / STEP`, so they total at most
///   `|dx0| / STEP + ny * (w / STEP)`.
/// - Both bounds hold, so the smaller does. A group makes at most
///   `ny + |dx0| / STEP + ny * min(SIDESTEP_STEPS, w / STEP)` moves, and
///   compaction at most their sum. That is tighter than the work order's
///   `(ny + 1) * (w / STEP + 1)`, by this proof.
///
/// Every term comes from the geometry before the first move or from the move
/// rule itself; none is typed in for the bound.
fn move_bound(starts: impl Iterator<Item = (i64, i64)>, w: i64) -> u64 {
    let n = |v: i64| u64::try_from(v / STEP).unwrap_or(0);
    let per_y = n(w).min(n(SIDESTEP_STEPS * STEP));
    starts.map(|(dy, dx)| n(dy) + n(dx) + n(dy) * per_y).sum()
}

/// How far past the MCU's courtyard a pair's approach is kept clear, and how
/// far to each side of its outermost pins (`AET-WO-4300g` D5, lever 2): engine
/// choices, measured to route `pair-standin` around the MCU's support ring.
const APPROACH_DEPTH_NM: i64 = 5_000_000;
const APPROACH_SIDE_NM: i64 = 500_000;

/// For each side of the MCU that carries a declared pair's pins, the strip in
/// front of them: from the pins out past the courtyard by
/// [`APPROACH_DEPTH_NM`], as wide as the pins plus [`APPROACH_SIDE_NM`] each
/// way. A pair is routed first, whole, coupled on F.Cu; it leaves its pins
/// through this strip, so no part placed after the MCU takes it.
fn approaches(mcu: &Item<'_>, courtyard: Rect, pairs: &[[String; 2]]) -> Vec<Rect> {
    let mut by_side: BTreeMap<u8, Vec<(i64, i64)>> = BTreeMap::new();
    for net in pairs.iter().flatten() {
        for &(at, ref n) in mcu.pads.values() {
            if n != net && n.strip_prefix('/') != Some(net.as_str()) {
                continue;
            }
            let (x, y) = (ORIGIN.0 + at.0, ORIGIN.1 + at.1);
            let d = [
                (x - courtyard.x0, 0u8),
                (courtyard.x1 - x, 1),
                (y - courtyard.y0, 2),
                (courtyard.y1 - y, 3),
            ];
            let side = d.iter().min().map_or(0, |&(_, s)| s);
            by_side.entry(side).or_default().push((x, y));
        }
    }
    by_side
        .into_iter()
        .map(|(side, pins)| {
            let xs = pins.iter().map(|p| p.0);
            let ys = pins.iter().map(|p| p.1);
            let (lx, hx) = (xs.clone().min().unwrap_or(0), xs.max().unwrap_or(0));
            let (ly, hy) = (ys.clone().min().unwrap_or(0), ys.max().unwrap_or(0));
            let s = APPROACH_SIDE_NM;
            match side {
                0 => Rect {
                    x0: courtyard.x0 - APPROACH_DEPTH_NM,
                    y0: ly - s,
                    x1: lx,
                    y1: hy + s,
                },
                1 => Rect {
                    x0: hx,
                    y0: ly - s,
                    x1: courtyard.x1 + APPROACH_DEPTH_NM,
                    y1: hy + s,
                },
                2 => Rect {
                    x0: lx - s,
                    y0: courtyard.y0 - APPROACH_DEPTH_NM,
                    x1: hx + s,
                    y1: ly,
                },
                _ => Rect {
                    x0: lx - s,
                    y0: hy,
                    x1: hx + s,
                    y1: courtyard.y1 + APPROACH_DEPTH_NM,
                },
            }
        })
        .collect()
}

/// An anchored part (`AET-WO-4300d2` §1.3), placed at whichever of the four
/// right angles brings its pads nearest the IC pads they serve: for each
/// angle, the free spot nearest the origin that would put each own pad, on
/// average, on its IC pad; then the angle with the least sum of squared
/// centre distances, ties to the smaller angle.
fn place_anchored(
    item: &Item<'_>,
    ic: &Item<'_>,
    at: (i64, i64),
    a: &schematic::Anchor,
    taken: &[(Rect, i64)],
) -> Result<Placed, EmitError> {
    let reference = &item.part.reference;
    let mut pairs = Vec::new();
    for (own, theirs) in &a.pads {
        let (mine, _) = item
            .pads
            .get(own)
            .ok_or_else(|| internal(reference, format!("anchored by pad {own}, which it lacks")))?;
        let (pin, _) = ic.pads.get(theirs).ok_or_else(|| {
            internal(
                reference,
                format!(
                    "anchored to {} pad {theirs}, which it lacks",
                    ic.part.reference
                ),
            )
        })?;
        pairs.push((*mine, (at.0 + pin.0, at.1 + pin.1)));
    }
    let n = i64::try_from(pairs.len().max(1)).unwrap_or(1);
    let mut best: Option<(i128, Placed)> = None;
    for theta in [0u16, 90, 180, 270] {
        let g = geometry_at(&item.fp, reference, theta)?;
        let turned: Vec<_> = pairs
            .iter()
            .map(|&(mine, pin)| (route::rotate(theta, mine), pin))
            .collect();
        let ideal = turned
            .iter()
            .fold((0, 0), |s, &(m, p)| (s.0 + p.0 - m.0, s.1 + p.1 - m.1));
        let (kx, ky) = g.keep.centre();
        let target = (ideal.0 / n + kx, ideal.1 / n + ky);
        let Some(origin) = spot(g, target, taken) else {
            continue;
        };
        let score: i128 = turned
            .iter()
            .map(|&(m, p)| {
                let dx = i128::from(origin.0 + m.0 - p.0);
                let dy = i128::from(origin.1 + m.1 - p.1);
                dx * dx + dy * dy
            })
            .sum();
        if best.as_ref().is_none_or(|(s, _)| score < *s) {
            best = Some((
                score,
                Placed {
                    reference: reference.clone(),
                    origin,
                    rotation: theta,
                    courtyard: g.courtyard.shift(origin),
                    keep: g.keep.shift(origin),
                },
            ));
        }
    }
    best.map(|(_, p)| p)
        .ok_or_else(|| internal(reference, "no free spot within a metre"))
}

/// The connectors, on the edges `plan` names (`AET-WO-4300f`, ruled D7: a
/// side-entry header at rotation 0 takes its cable toward +y; `AET-WO-1313`:
/// the left and right edges at 270 and 90, cable out). Each edge's row is
/// laid out in that edge's own frame, the board turned so the edge is the
/// bottom: in reference order, in one row centred on every part already
/// placed, each keep-out box `gap` from the next and [`IC_GAP_NM`] from
/// every box inside. The edge is the first [`SNAP`] line that leaves
/// [`MARGIN_NM`] past the other parts and lets every connector sit there:
/// each courtyard's cable side is within one [`SNAP`] of the edge and inside
/// it, and every pad keeps [`route::EDGE_NM`] from it. The housing body
/// (`F.Fab`) sits about 0.8 mm inside, by the courtyard's own clearance.
/// Returns the edges set; none with no connector.
fn place_connectors(
    items: &[Item<'_>],
    placed: &mut [Option<Placed>],
    plan: EdgePlan,
    rank: Option<&[usize]>,
) -> Result<Edges, EmitError> {
    let (sides, gap) = connector_edges(items, plan, rank);
    let mut edges = Edges::default();
    if sides.is_empty() {
        return Ok(edges);
    }
    let core = placed
        .iter()
        .flatten()
        .map(|p| p.keep)
        .reduce(Rect::union)
        .ok_or_else(|| EmitError::Internal("connectors with no other part".into()))?;
    let down = |v: i64| v.div_euclid(SNAP) * SNAP;
    let up = |v: i64| -((-v).div_euclid(SNAP) * SNAP);
    for rot in [0u16, 90, 270] {
        let row: Vec<usize> = sides
            .iter()
            .filter(|&&(_, r)| r == rot)
            .map(|&(i, _)| i)
            .collect();
        if row.is_empty() {
            continue;
        }
        // The frame: a board point p is rotate(360 - rot, p) in it, and a
        // frame point q is rotate(rot, q) on the board.
        let back = (360 - rot) % 360;
        let fcore = {
            let a = route::rotate(back, (core.x0, core.y0));
            let b = route::rotate(back, (core.x1, core.y1));
            Rect {
                x0: a.0.min(b.0),
                y0: a.1.min(b.1),
                x1: a.0.max(b.0),
                y1: a.1.max(b.1),
            }
        };
        let mut edge = up(fcore.y1 + MARGIN_NM);
        for &i in &row {
            let g = items[i].geometry;
            if g.keep.y1 > g.courtyard.y1 {
                return Err(internal(
                    &items[i].part.reference,
                    "silkscreen below the courtyard would cross the board edge",
                ));
            }
            edge = edge.max(up(up(fcore.y1 + IC_GAP_NM - g.keep.y0) + g.courtyard.y1));
        }
        let width = row
            .iter()
            .map(|&i| items[i].geometry.keep.x1 - items[i].geometry.keep.x0 + gap)
            .sum::<i64>()
            - gap;
        let mut x = fcore.centre().0 - width / 2;
        for &i in &row {
            let (item, g) = (&items[i], items[i].geometry);
            let at = (up(x - g.keep.x0), down(edge - g.courtyard.y1));
            // belt-and-suspenders: every vendored connector's pads sit inside its
            // courtyard by more than EDGE_NM, so no shipped part trips this (a
            // mutation that loosens it survives); a footprint added later might.
            for pad in item.fp.children("pad") {
                let r = pad_box(pad, &item.part.reference)?;
                if at.1 + r.y1 + route::EDGE_NM > edge {
                    return Err(internal(
                        &item.part.reference,
                        format!(
                            "pad {} would be nearer the edge than {} nm",
                            pad.arg(1).unwrap_or(""),
                            route::EDGE_NM
                        ),
                    ));
                }
            }
            let origin = route::rotate(rot, at);
            let turned = geometry_at(&item.fp, &item.part.reference, rot)?;
            placed[i] = Some(Placed {
                reference: item.part.reference.clone(),
                origin,
                rotation: rot,
                courtyard: turned.courtyard.shift(origin),
                keep: turned.keep.shift(origin),
            });
            x = at.0 + g.keep.x1 + gap;
        }
        let e = route::rotate(rot, (0, edge));
        match rot {
            0 => edges.bottom = Some(e.1),
            90 => edges.right = Some(e.0),
            _ => edges.left = Some(e.0),
        }
    }
    Ok(edges)
}

/// A pad's box relative to its footprint's origin, from its `at` and `size`.
/// A rotated pad is refused: the box would be wrong.
fn pad_box(pad: &Sx, reference: &str) -> Result<Rect, EmitError> {
    let at = pad
        .child("at")
        .ok_or_else(|| internal(reference, "a pad without `at`"))?;
    if at.arg(3).is_some_and(|a| a != "0") {
        return Err(internal(reference, "a rotated pad"));
    }
    let (x, y) = xy(at, reference)?;
    let size = pad
        .child("size")
        .ok_or_else(|| internal(reference, "a pad without `size`"))?;
    let (w, h) = xy(size, reference)?;
    Ok(Rect {
        x0: x - w / 2,
        y0: y - h / 2,
        x1: x + w - w / 2,
        y1: y + h - h / 2,
    })
}

/// The first origin, in rings out from `target`, where `g`'s keep-out box
/// clears every box in `taken` by [`GAP_NM`]. Within a ring, candidates go
/// by squared distance, then y, then x.
fn spot(g: Geometry, target: (i64, i64), taken: &[(Rect, i64)]) -> Option<(i64, i64)> {
    let (cx, cy) = g.keep.centre();
    let snap = |v: i64| v.div_euclid(SNAP) * SNAP;
    for r in 0..=RINGS {
        let mut ring: Vec<(i64, i64)> = Vec::new();
        for d in -r..=r {
            ring.extend([(d, -r), (d, r)]);
            if d != -r && d != r {
                ring.extend([(-r, d), (r, d)]);
            }
        }
        ring.sort_by_key(|&(dx, dy)| (dx * dx + dy * dy, dy, dx));
        ring.dedup();
        for (dx, dy) in ring {
            let origin = (
                snap(target.0 + dx * STEP - cx),
                snap(target.1 + dy * STEP - cy),
            );
            let keep = g.keep.shift(origin);
            if taken.iter().all(|&(t, gap)| !t.overlaps(keep.grow(gap))) {
                return Some(origin);
            }
        }
    }
    None
}

fn uuid_sx(key: &str) -> Sx {
    Sx::List(vec![Sx::atom("uuid"), Sx::str(schematic::uuid(key))])
}

fn at_sx(head: &str, (x, y): (i64, i64)) -> Sx {
    Sx::node(head, &[&mm(x), &mm(y)])
}

/// Every `(uuid …)` under `sx`, derived from `key` and its order.
fn rekey(sx: &mut Sx, key: &str, n: &mut usize) {
    if sx.head() == Some("uuid") {
        *n += 1;
        *sx = uuid_sx(&format!("{key}/item/{n}"));
        return;
    }
    if let Some(items) = sx.items_mut() {
        for c in items {
            rekey(c, key, n);
        }
    }
}

/// The footprint as the board holds it.
fn footprint(
    project: &str,
    item: &Item<'_>,
    origin: (i64, i64),
    rotation: u16,
) -> Result<Sx, EmitError> {
    let part = item.part;
    let key = format!("{project}/footprint/{}", part.reference);
    let sym = library::symbol(part.lib_id)?;
    let fields: Vec<(&str, &str)> = sym
        .children("property")
        .filter_map(|p| Some((p.arg(1)?, p.arg(2).unwrap_or(""))))
        .filter(|(name, _)| !matches!(*name, "Reference" | "Value" | "Footprint"))
        .collect();
    let lib_props: BTreeSet<&str> = item
        .fp
        .children("property")
        .filter_map(|p| p.arg(1))
        .collect();
    let mut out = vec![
        Sx::atom("footprint"),
        Sx::str(part.footprint.clone()),
        Sx::List(vec![Sx::atom("layer"), Sx::str("F.Cu")]),
        uuid_sx(&key),
        match rotation {
            0 => at_sx("at", origin),
            r => Sx::node("at", &[&mm(origin.0), &mm(origin.1), &r.to_string()]),
        },
    ];
    let body = item.fp.items().get(2..).unwrap_or(&[]);
    let last_prop = body
        .iter()
        .rposition(|c| c.head() == Some("property"))
        .ok_or_else(|| internal(&part.reference, "a footprint without properties"))?;
    let mut n = 0;
    for (index, child) in body.iter().enumerate() {
        let mut c = child.clone();
        match child.head() {
            Some("version" | "generator" | "generator_version" | "layer") => continue,
            // A footprint's zone stands in board coordinates in a board file,
            // where its library file has it about the footprint's origin
            // (measured, `AET-WO-1310` D4: written unshifted, KiCad read the
            // ICM-20948's keepout at the board's origin and reported
            // `lib_footprint_mismatch`). It turns with the footprint by the
            // same `route::rotate` its pads do (`AET-WO-4400d` D2, replacing
            // the refusal of a turned zone; the tier-2
            // `a_turned_keepout_is_where_kicad_draws_it` holds it).
            Some("zone") => to_board(&mut c, origin, rotation, &part.reference)?,
            Some("fp_text") if rotation != 0 => turn(&mut c, rotation, &part.reference)?,
            Some("property") => {
                if rotation != 0 {
                    turn(&mut c, rotation, &part.reference)?;
                }
                let text = match child.arg(1) {
                    Some("Reference") => Some(part.reference.as_str()),
                    Some("Value") => Some(part.value.as_str()),
                    Some(name) => fields.iter().find(|(f, _)| *f == name).map(|(_, v)| *v),
                    None => None,
                };
                if let (Some(text), Some(items)) = (text, c.items_mut()) {
                    items[2] = Sx::str(text);
                }
            }
            Some("pad") => {
                let number = child.arg(1).unwrap_or("");
                if rotation != 0 {
                    turn(&mut c, rotation, &part.reference)?;
                }
                if let Some((_, net)) = item.pads.get(number)
                    && let Some(items) = c.items_mut()
                {
                    let net = Sx::List(vec![Sx::atom("net"), Sx::str(net.clone())]);
                    match items.iter().position(|i| i.head() == Some("uuid")) {
                        Some(u) => items.insert(u, net),
                        None => items.push(net),
                    }
                }
            }
            _ => {}
        }
        rekey(&mut c, &key, &mut n);
        out.push(c);
        if index == last_prop {
            for (i, (name, value)) in fields.iter().enumerate() {
                if lib_props.contains(name) {
                    continue;
                }
                out.push(Sx::List(vec![
                    Sx::atom("property"),
                    Sx::str(*name),
                    Sx::str(*value),
                    Sx::node("at", &["0", "0", "0"]),
                    Sx::List(vec![Sx::atom("layer"), Sx::str("F.Fab")]),
                    Sx::node("hide", &["yes"]),
                    uuid_sx(&format!("{key}/field/{i}")),
                    Sx::List(vec![
                        Sx::atom("effects"),
                        Sx::List(vec![
                            Sx::atom("font"),
                            Sx::node("size", &["1", "1"]),
                            Sx::node("thickness", &["0.15"]),
                        ]),
                    ]),
                ]));
            }
            out.push(Sx::List(vec![
                Sx::atom("path"),
                Sx::str(format!(
                    "/{}",
                    schematic::symbol_uuid(project, &part.reference)
                )),
            ]));
            out.push(Sx::List(vec![Sx::atom("sheetname"), Sx::str("/")]));
            out.push(Sx::List(vec![
                Sx::atom("sheetfile"),
                Sx::str(format!("{project}.kicad_sch")),
            ]));
        }
    }
    Ok(Sx::List(out))
}

use aethelgard_ir::names::reference_key;

/// Each part's footprint, pads and geometry.
fn items(parts: &[Part]) -> Result<Vec<Item<'_>>, EmitError> {
    let mut keys = BTreeMap::new();
    for part in parts {
        if let Some(other) = keys.insert(reference_key(&part.reference), &part.reference) {
            let mut pair = [other.as_str(), part.reference.as_str()];
            pair.sort();
            return Err(EmitError::DuplicateReference(pair.join(" and ")));
        }
    }
    let mut out = Vec::new();
    for part in parts {
        let mut fp = library::footprint(&part.footprint)?;
        // The MCU's own reference too, when its support network rings it: on
        // the silkscreen, its box above the top row of pins pushed that row's
        // capacitors 2.49 mm out (`AET-WO-4300g` §3, measured).
        let rings = parts.iter().any(|x| anchored_to(x, &part.reference));
        if part.owner.as_ref().is_some_and(|o| o.anchor.is_some())
            || (rings && part.lib_id == schematic::MCU)
        {
            reference_on_fab(&mut fp);
        }
        let nets = pin_nets(part)?;
        let rotation = match &part.seat {
            Some(seat) => seat_rotation(&fp, seat, &part.reference)?,
            None => 0,
        };
        let mut pads = BTreeMap::new();
        let mut numbers = BTreeSet::new();
        for pad in fp.children("pad") {
            let number = pad.arg(1).unwrap_or("");
            numbers.insert(number.to_owned());
            if let Some(net) = nets.get(number) {
                let at = pad
                    .child("at")
                    .ok_or_else(|| internal(&part.reference, "a pad without `at`"))?;
                let at = route::rotate(rotation, xy(at, &part.reference)?);
                pads.insert(number.to_owned(), (at, net.clone()));
            }
        }
        if let Some(missing) = nets.keys().find(|n| !numbers.contains(*n)) {
            return Err(internal(
                &part.reference,
                format!("pin {missing} has no pad in {}", part.footprint),
            ));
        }
        out.push(Item {
            geometry: geometry_at(&fp, &part.reference, rotation)?,
            rotation,
            part,
            fp,
            pads,
        });
    }
    Ok(out)
}

/// The turn that puts an IMU's cited axes where its seat declares them
/// (`AET-WO-4400d`), or `ImuRotationUnplaceable`. The part's +Z is out of
/// its top and every footprint sits on F.Cu, so Z must point up. Each axis
/// on the footprint runs from the package's centre (its pads' mean) to the
/// mean of the pins the parts table names for it, and must lie along one
/// footprint axis. In the board file seen from F.Cu (y down), forward is the
/// declared edge, right is forward turned clockwise, and with Z up, Y = Z × X
/// is X turned a quarter counter-clockwise, `route::rotate(90, X)`. The turn
/// is the right angle that takes both footprint axes to those directions.
fn seat_rotation(fp: &Sx, seat: &schematic::Seat, reference: &str) -> Result<u16, EmitError> {
    use aethelgard_ir::orientation::{Edge, Toward};
    let o = seat.orientation;
    let unplaceable = |why: String| EmitError::ImuRotationUnplaceable {
        reference: reference.to_owned(),
        x: o.x.name(),
        z: o.z.name(),
        why,
    };
    if o.z != Toward::Up {
        return Err(unplaceable(
            "the part's +Z is out of its top and this engine places every footprint on F.Cu, so Z points up".into(),
        ));
    }
    let mut centres = BTreeMap::new();
    for pad in fp.children("pad") {
        let at = pad
            .child("at")
            .ok_or_else(|| internal(reference, "a pad without `at`"))?;
        centres.insert(pad.arg(1).unwrap_or("").to_owned(), xy(at, reference)?);
    }
    let mean = |pts: &[(i64, i64)]| {
        let n = i64::try_from(pts.len().max(1)).unwrap_or(1);
        let (x, y) = pts.iter().fold((0, 0), |s, p| (s.0 + p.0, s.1 + p.1));
        (x / n, y / n)
    };
    let all: Vec<(i64, i64)> = centres.values().copied().collect();
    let centre = mean(&all);
    let along = |pins: &[String], axis: &str| -> Result<(i64, i64), EmitError> {
        let mut pts = Vec::new();
        for pin in pins {
            pts.push(*centres.get(pin).ok_or_else(|| {
                unplaceable(format!(
                    "the part's {axis} axis names pin {pin}, which its footprint has no pad for"
                ))
            })?);
        }
        let (x, y) = mean(&pts);
        match (x - centre.0, y - centre.1) {
            (dx, 0) if dx != 0 => Ok((dx.signum(), 0)),
            (0, dy) if dy != 0 => Ok((0, dy.signum())),
            _ => Err(unplaceable(format!(
                "the part's {axis} axis (pins {}) does not lie along one footprint axis",
                pins.join(", ")
            ))),
        }
    };
    let fx = along(&seat.x_toward_pins, "X")?;
    let fy = along(&seat.y_toward_pins, "Y")?;
    let forward = match seat.forward {
        Edge::Top => (0, -1),
        Edge::Right => (1, 0),
        Edge::Bottom => (0, 1),
        Edge::Left => (-1, 0),
    };
    let right = (-forward.1, forward.0);
    let want_x = match o.x {
        Toward::Forward => forward,
        Toward::Aft => (-forward.0, -forward.1),
        Toward::Right => right,
        Toward::Left => (-right.0, -right.1),
        Toward::Up | Toward::Down => {
            return Err(unplaceable(
                "X up or down would stand the part on its edge, which this engine does not place"
                    .into(),
            ));
        }
    };
    let want_y = route::rotate(90, want_x);
    [0u16, 90, 180, 270]
        .into_iter()
        .find(|&t| route::rotate(t, fx) == want_x && route::rotate(t, fy) == want_y)
        .ok_or_else(|| {
            unplaceable(
                "the part's cited axes are not right-handed with +Z out of its footprint's top, so no turn on F.Cu agrees".into(),
            )
        })
}

/// Each `(xy x y)` under `sx`, turned by `rotation` and moved by `origin`:
/// a footprint zone's polygon from the library's coordinates to the board's.
fn to_board(
    sx: &mut Sx,
    origin: (i64, i64),
    rotation: u16,
    reference: &str,
) -> Result<(), EmitError> {
    if sx.head() == Some("xy") {
        let (x, y) = route::rotate(rotation, xy(sx, reference)?);
        *sx = at_sx("xy", (origin.0 + x, origin.1 + y));
        return Ok(());
    }
    for c in sx.items_mut().into_iter().flatten() {
        to_board(c, origin, rotation, reference)?;
    }
    Ok(())
}

/// An anchored part's reference text moves from the silkscreen to `F.Fab`
/// (`AET-WO-4300d2` §1.3): on the silkscreen it would hold the part's keep-out
/// box a text's width from the pin it must sit beside. The board's
/// fabrication notes and the placement file still name every reference.
fn reference_on_fab(fp: &mut Sx) {
    let Some(items) = fp.items_mut() else { return };
    for c in items.iter_mut() {
        if c.head() != Some("property") || c.arg(1) != Some("Reference") {
            continue;
        }
        if let Some(fields) = c.items_mut() {
            for f in fields.iter_mut() {
                if f.head() == Some("layer") {
                    *f = Sx::List(vec![Sx::atom("layer"), Sx::str("F.Fab")]);
                }
            }
        }
    }
}

/// The placed board: what `write` emits, for tests and the outline.
pub struct Layout {
    pub placed: Vec<Placed>,
    pub outline: Rect,
    /// `(reference, pad number) → net` for every pad that carries one.
    pub pad_nets: BTreeMap<(String, String), String>,
}

/// A part the caller places itself (`AET-WO-1308` §1.5): its reference, its
/// origin and its rotation. Every other part is placed by [`place`] first.
pub type Move = (String, (i64, i64), u16);

/// How many reseeded canonical ranks [`layout_of`] tries, after the first
/// placement, when that placement misses a declared bound (`AET-WO-1317`).
/// Eight, frozen before the held-out runs: on the one-part-removed sweep over
/// Sentinel-03's bounded variants it removed as many cases as sixteen (its
/// FEEDBACK §3).
const RESEEDS: u64 = 8;

/// The placement and its outline. Under a declared bound, when the placement
/// misses it (`AET-WO-1317`), the placer tries again with the canonical rank
/// reseeded ([`canonical_rank`]), seeds 1 to [`RESEEDS`] in turn, and keeps
/// the first that fits. A placement that fits the first time is the one
/// returned, so no board that fits today moves. Removing a part re-rolls
/// every colour tie the rank breaks by hash, and a greedy placer turns a
/// re-rolled order into an outline some millimetres wider or narrower
/// (FEEDBACK §2); a reseeded rank re-rolls those ties with nothing removed.
/// A seed is a number, never a name, so no reference decides (`AET-WO-1313c`).
///
/// When no seed fits, the refusal names the least-over attempt (the larger
/// overrun, then the sum, then the earlier seed): the placer's best outline.
/// Any other refusal, on any attempt, is returned as it is.
fn layout_of(
    items: &[Item<'_>],
    moves: &[Move],
    pairs: &[[String; 2]],
    bound: Option<OutlineBound>,
) -> Result<Layout, EmitError> {
    let over = |e: &EmitError| match *e {
        EmitError::OutlineUnreachable {
            over_um_w,
            over_um_h,
            ..
        } => Some((over_um_w.max(over_um_h), over_um_w + over_um_h)),
        _ => None,
    };
    let mut best = match attempt(items, moves, pairs, bound, 0) {
        Err(e) if over(&e).is_some() => e,
        done => return done,
    };
    for seed in 1..=RESEEDS {
        match attempt(items, moves, pairs, bound, seed) {
            Err(e) if over(&e).is_some() => {
                if over(&e) < over(&best) {
                    best = e;
                }
            }
            done => return done,
        }
    }
    Err(best)
}

/// One placement of [`layout_of`], with the canonical rank seeded by `seed`.
fn attempt(
    items: &[Item<'_>],
    moves: &[Move],
    pairs: &[[String; 2]],
    bound: Option<OutlineBound>,
    seed: u64,
) -> Result<Layout, EmitError> {
    let (mut placed, edges) = place(items, pairs, bound.is_some(), seed)?;
    let edge = edges.bottom;
    for (reference, origin, rotation) in moves {
        if !matches!(rotation, 0 | 90 | 180 | 270) {
            return Err(internal(reference, format!("a rotation of {rotation}")));
        }
        let i = items
            .iter()
            .position(|it| &it.part.reference == reference)
            .ok_or_else(|| internal(reference, "moved, but not a part of the design"))?;
        let g = geometry_at(&items[i].fp, reference, *rotation)?;
        placed[i] = Placed {
            reference: reference.clone(),
            origin: *origin,
            rotation: *rotation,
            courtyard: g.courtyard.shift(*origin),
            keep: g.keep.shift(*origin),
        };
    }
    let first = placed
        .first()
        .ok_or_else(|| EmitError::Internal("no parts".into()))?
        .keep;
    let all = placed
        .iter()
        .fold(first, |r, p| r.union(p.keep))
        .grow(MARGIN_NM);
    let mut outline = Rect {
        x0: all.x0.div_euclid(SNAP) * SNAP,
        y0: all.y0.div_euclid(SNAP) * SNAP,
        x1: -((-all.x1).div_euclid(SNAP) * SNAP),
        // With connectors, their courtyards' bottoms are the bottom edge, but
        // a moved part below them still bounds it (`AET-WO-1310`, O1).
        y1: match edge {
            Some(e) => {
                // Only a moved part reaching below the connectors' edge moves it.
                let moved = placed
                    .iter()
                    .filter(|p| moves.iter().any(|(r, _, _)| *r == p.reference))
                    .filter(|p| p.keep.y1 > e)
                    .map(|p| p.keep.y1 + MARGIN_NM)
                    .max()
                    .unwrap_or(e);
                e.max(-((-moved).div_euclid(SNAP) * SNAP))
            }
            None => -((-all.y1).div_euclid(SNAP) * SNAP),
        },
    };
    // Connectors on a side edge set that edge (`AET-WO-1313`).
    if let Some(e) = edges.left {
        outline.x0 = e;
    }
    if let Some(e) = edges.right {
        outline.x1 = e;
    }
    if let Some(b) = bound {
        // belt-and-suspenders: under either edge plan every keep-out box is
        // inside the outline by construction (the core within its margin, each
        // connector row inside its own edge), so no board trips this and its
        // mutant survives (FEEDBACK §5.7, M5); a later plan might not.
        if let Some(p) = placed.iter().find(|p| !outline.contains(p.keep)) {
            return Err(internal(&p.reference, "a keep-out box outside the outline"));
        }
        let (w, h) = (outline.x1 - outline.x0, outline.y1 - outline.y0);
        let (bw, bh) = bound_nm(b);
        if w > bw || h > bh {
            return Err(EmitError::OutlineUnreachable {
                width_um: b.width,
                height_um: b.height,
                reached_um_w: w / 1000,
                reached_um_h: h / 1000,
                over_um_w: (w - bw).max(0) / 1000,
                over_um_h: (h - bh).max(0) / 1000,
            });
        }
    }
    let pad_nets = items
        .iter()
        .flat_map(|i| {
            i.pads
                .iter()
                .map(|(n, (_, net))| ((i.part.reference.clone(), n.clone()), net.clone()))
        })
        .collect();
    Ok(Layout {
        placed,
        outline,
        pad_nets,
    })
}

/// The placement and pad nets `write` uses.
#[cfg(test)]
fn layout(parts: &[Part]) -> Result<Layout, EmitError> {
    layout_of(&items(parts)?, &[], &[], None)
}

/// A pad's or a text's `(at x y [a])` with the footprint's rotation added to
/// its own angle, which KiCad's file holds absolute (measured, `AET-WO-1308`
/// P5), so the footprint turns whole: text kept upright would land on the
/// footprint's own turned silkscreen (measured, KiCad's `silk_overlap`).
/// Only right angles are written; the library's are right angles.
fn turn(item: &mut Sx, rotation: u16, reference: &str) -> Result<(), EmitError> {
    let items = item
        .items_mut()
        .ok_or_else(|| internal(reference, "a pad or text that is not a list"))?;
    let at = items
        .iter_mut()
        .find(|i| i.head() == Some("at"))
        .ok_or_else(|| internal(reference, "a pad or text without `at`"))?;
    let (x, y) = xy(at, reference)?;
    let own = route::angle(at.arg(3))
        .ok_or_else(|| internal(reference, format!("an item at angle {:?}", at.arg(3))))?;
    let turned = (own + rotation) % 360;
    *at = match turned {
        0 => at_sx("at", (x, y)),
        a => Sx::node("at", &[&mm(x), &mm(y), &a.to_string()]),
    };
    Ok(())
}

/// The `.kicad_pcb` text for `parts`, on the board `stack` declares.
#[cfg(test)]
pub(crate) fn write(
    project: &str,
    parts: &[Part],
    stack: &crate::stack::Stack,
) -> Result<String, EmitError> {
    write_moved(project, parts, stack, &[], None)
}

/// [`write`], with each of `moves` placed where it says instead, inside
/// `bound` when one is declared (`AET-WO-1313`), routed as today: the
/// tests' path; `emit` places through `schematic::place` (`AET-WO-1314`).
#[cfg(test)]
pub(crate) fn write_moved(
    project: &str,
    parts: &[Part],
    stack: &crate::stack::Stack,
    moves: &[Move],
    bound: Option<OutlineBound>,
) -> Result<String, EmitError> {
    let (footprints, outline) = placed(project, parts, stack, moves, bound)?;
    write_placed_at(
        project,
        footprints,
        outline,
        stack,
        Some(route::Rung::TODAY),
    )
}

/// The placed, netted footprints of [`write_moved`] and the outline, before
/// routing: shared by every rung of the capacitance ladder (`AET-WO-1314`).
pub(crate) fn placed(
    project: &str,
    parts: &[Part],
    stack: &crate::stack::Stack,
    moves: &[Move],
    bound: Option<OutlineBound>,
) -> Result<(Vec<Sx>, Rect), EmitError> {
    let items = items(parts)?;
    let pairs: Vec<[String; 2]> = stack.pairs.iter().map(|p| p.nets.clone()).collect();
    let layout = layout_of(&items, moves, &pairs, bound)?;
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| items[a].part.reference.cmp(&items[b].part.reference));
    // `place` returns each item's placement at the item's index.
    let mut footprints = Vec::new();
    for i in order {
        let p = &layout.placed[i];
        footprints.push(footprint(project, &items[i], p.origin, p.rotation)?);
    }
    Ok((footprints, layout.outline))
}

/// The board of `footprints`, each already placed and netted, inside
/// `outline`, routed: the second half of [`write`], for a board whose parts
/// the specification cannot yet declare (`AET-WO-1309` FEEDBACK D1).
pub fn write_placed(
    project: &str,
    footprints: Vec<Sx>,
    outline: Rect,
    stack: &crate::stack::Stack,
) -> Result<String, EmitError> {
    write_placed_at(
        project,
        footprints,
        outline,
        stack,
        Some(route::Rung::TODAY),
    )
}

/// [`write_placed`], routed at `rung` (`AET-WO-1314`); with `None`, the
/// placed board alone: the same header, footprints and outline, and no
/// track, via or zone (`schematic::placement`).
pub(crate) fn write_placed_at(
    project: &str,
    footprints: Vec<Sx>,
    outline: Rect,
    stack: &crate::stack::Stack,
    rung: Option<route::Rung>,
) -> Result<String, EmitError> {
    let mut layers = vec![Sx::atom("layers")];
    for (id, name, kind, user) in LAYERS {
        // Inner copper rows follow F.Cu and precede B.Cu, as KiCad saves them.
        if name == "B.Cu" {
            for inner in &stack.copper[1..stack.copper.len() - 1] {
                let id = crate::stack::kicad_id(inner).ok_or_else(|| {
                    EmitError::Internal(format!("no KiCad id for copper layer {inner}"))
                })?;
                layers.push(Sx::List(vec![
                    Sx::atom(id),
                    Sx::str(inner),
                    Sx::atom("signal"),
                ]));
            }
        }
        let mut l = vec![Sx::atom(id), Sx::str(name), Sx::atom(kind)];
        if let Some(u) = user {
            l.push(Sx::str(u));
        }
        layers.push(Sx::List(l));
    }
    let mut out = vec![
        Sx::atom("kicad_pcb"),
        Sx::node("version", &[VERSION]),
        Sx::List(vec![Sx::atom("generator"), Sx::str("aethelgard")]),
        Sx::List(vec![
            Sx::atom("generator_version"),
            Sx::str(env!("CARGO_PKG_VERSION")),
        ]),
        Sx::List(vec![
            Sx::atom("general"),
            Sx::node("thickness", &[&mm(stack.thickness.nominal_nm)]),
            Sx::node("legacy_teardrops", &["no"]),
        ]),
        Sx::List(vec![Sx::atom("paper"), Sx::str("A4")]),
        Sx::List(layers),
    ];
    let mut parts = Vec::new();
    for mut fp in footprints {
        join(&mut fp, stack);
        parts.push(route::part_of(&fp).map_err(EmitError::Internal)?);
        out.push(fp);
    }
    let o = outline;
    let board = route::Board {
        outline: o,
        parts,
        stack: stack.clone(),
    };
    let routed = rung
        .map(|rung| {
            route::route_with(&board, rung).map_err(|e| match (e.exhausted, e.net.is_empty()) {
                (Some((reference, pads)), _) => EmitError::PadEscapeExhausted { reference, pads },
                (None, true) => EmitError::BoardUnroutable(e.why),
                (None, false) => EmitError::Unroutable {
                    net: e.net,
                    why: e.why,
                },
            })
        })
        .transpose()?;
    out.push(Sx::List(vec![
        Sx::atom("gr_rect"),
        at_sx("start", (o.x0, o.y0)),
        at_sx("end", (o.x1, o.y1)),
        Sx::List(vec![
            Sx::atom("stroke"),
            Sx::node("width", &["0.05"]),
            Sx::node("type", &["default"]),
        ]),
        Sx::node("fill", &["no"]),
        Sx::List(vec![Sx::atom("layer"), Sx::str("Edge.Cuts")]),
        uuid_sx(&format!("{project}/outline")),
    ]));
    if let Some(routed) = &routed {
        out.extend(copper(project, routed, o, stack));
    }
    out.push(Sx::node("embedded_fonts", &["no"]));
    Ok(sexpr::write(&Sx::List(out)))
}

/// The declared join on each through-hole pad of a part that has one
/// (`AET-WO-1309`), as the pad's own `zone_connect` ([`Join::token`]). A
/// part with none declared is unchanged.
///
/// [`Join::token`]: crate::stack::Join::token
fn join(fp: &mut Sx, stack: &crate::stack::Stack) {
    let declared = fp
        .property("Reference")
        .and_then(|r| stack.joins.iter().find(|(x, _)| x == r).map(|&(_, j)| j));
    let Some(value) = declared.map(crate::stack::Join::token) else {
        return;
    };
    for pad in fp.items_mut().into_iter().flatten() {
        if pad.head() == Some("pad")
            && pad.arg(2) == Some("thru_hole")
            && let Some(items) = pad.items_mut()
        {
            items.push(Sx::node("zone_connect", &[value]));
        }
    }
}

/// The router's tracks, vias and unfilled zones as board items, each
/// keyed by its content. A zone's outline is the board's; it carries no
/// `connect_pads` clearance, which the gate refuses (`AET-WO-1300` B2), and
/// no fill, which the gate's `--refill-zones` computes.
fn copper(project: &str, routed: &route::Routed, o: Rect, stack: &crate::stack::Stack) -> Vec<Sx> {
    let net = |n: &str| Sx::List(vec![Sx::atom("net"), Sx::str(n)]);
    let layer = |l: route::Layer| Sx::List(vec![Sx::atom("layer"), Sx::str(l.name())]);
    let mut out = Vec::new();
    for s in &routed.segments {
        out.push(Sx::List(vec![
            Sx::atom("segment"),
            at_sx("start", s.start),
            at_sx("end", s.end),
            Sx::node("width", &[&mm(stack.rule(&s.net).width_nm)]),
            layer(s.layer),
            net(&s.net),
            uuid_sx(&format!(
                "{project}/segment/{}/{}/{} {} {} {}",
                s.net,
                s.layer.name(),
                s.start.0,
                s.start.1,
                s.end.0,
                s.end.1
            )),
        ]));
    }
    for v in &routed.vias {
        out.push(Sx::List(vec![
            Sx::atom("via"),
            at_sx("at", v.at),
            Sx::node("size", &[&mm(stack.rule(&v.net).via_nm)]),
            Sx::node("drill", &[&mm(stack.rule(&v.net).drill_nm)]),
            Sx::List(vec![Sx::atom("layers"), Sx::str("F.Cu"), Sx::str("B.Cu")]),
            net(&v.net),
            uuid_sx(&format!("{project}/via/{}/{} {}", v.net, v.at.0, v.at.1)),
        ]));
    }
    for a in &routed.arrays {
        for &at in &a.at {
            out.push(Sx::List(vec![
                Sx::atom("via"),
                at_sx("at", at),
                Sx::node("size", &[&mm(a.via_nm)]),
                Sx::node("drill", &[&mm(a.drill_nm)]),
                Sx::List(vec![Sx::atom("layers"), Sx::str("F.Cu"), Sx::str("B.Cu")]),
                net(&a.net),
                uuid_sx(&format!(
                    "{project}/thermal/{}/{}/{} {}",
                    a.reference, a.pad, at.0, at.1
                )),
            ]));
        }
    }
    for (n, l) in &routed.zones {
        let corner = |x, y| Sx::node("xy", &[&mm(x), &mm(y)]);
        out.push(Sx::List(vec![
            Sx::atom("zone"),
            net(n),
            Sx::List(vec![Sx::atom("layer"), Sx::str(l)]),
            uuid_sx(&format!("{project}/zone/{n}/{l}")),
            Sx::node("hatch", &["edge", "0.5"]),
            Sx::node("min_thickness", &[&mm(route::MIN_THICKNESS_NM)]),
            Sx::node("filled_areas_thickness", &["no"]),
            Sx::List(vec![
                Sx::atom("fill"),
                Sx::node("thermal_gap", &[&mm(route::THERMAL_GAP_NM)]),
                Sx::node("thermal_bridge_width", &[&mm(route::THERMAL_BRIDGE_NM)]),
                Sx::node("island_removal_mode", &["0"]),
            ]),
            Sx::List(vec![
                Sx::atom("polygon"),
                Sx::List(vec![
                    Sx::atom("pts"),
                    corner(o.x0, o.y0),
                    corner(o.x1, o.y0),
                    corner(o.x1, o.y1),
                    corner(o.x0, o.y1),
                ]),
            ]),
        ]));
    }
    out
}

#[cfg(test)]
mod tests {
    /// `AET-WO-4400d`: the seat's turn on the ICM-42688-P's own footprint
    /// for each forward edge and top-side X, and each refusal a footprint
    /// can trip that the embedded parts table cannot: pin groups that are
    /// mirrored (left-handed with +Z out of the top), an axis that is not
    /// along a footprint axis or has no length, a pin with no pad.
    #[test]
    fn a_seat_turns_by_its_cited_pins_or_is_refused_by_name() {
        use aethelgard_ir::orientation::{Edge, Orientation, Toward};
        let fp = crate::library::footprint("Aethelgard_Sensor:TDK_LGA-14_3x2.5mm_P0.5mm").unwrap();
        let pins = |v: &[&str]| v.iter().map(|p| (*p).to_owned()).collect::<Vec<_>>();
        let seat = |forward, x, xp: &[&str], yp: &[&str]| crate::schematic::Seat {
            forward,
            orientation: Orientation { x, z: Toward::Up },
            x_toward_pins: pins(xp),
            y_toward_pins: pins(yp),
        };
        let (xp, yp) = (["8", "9", "10", "11"], ["12", "13", "14"]);
        for (forward, x, want) in [
            (Edge::Top, Toward::Right, 0),
            (Edge::Top, Toward::Forward, 90),
            (Edge::Top, Toward::Left, 180),
            (Edge::Top, Toward::Aft, 270),
            (Edge::Right, Toward::Forward, 0),
            (Edge::Bottom, Toward::Right, 180),
            (Edge::Left, Toward::Forward, 180),
        ] {
            assert_eq!(
                super::seat_rotation(&fp, &seat(forward, x, &xp, &yp), "U5").unwrap(),
                want,
                "{forward:?} {x:?}"
            );
        }
        for (xp, yp, why) in [
            (
                &["12", "13", "14"][..],
                &["8", "9", "10", "11"][..],
                "right-handed",
            ),
            (&["1", "8"][..], &yp[..], "along one footprint axis"),
            (&["8", "12"][..], &yp[..], "along one footprint axis"),
            (&["8", "99"][..], &yp[..], "no pad for"),
        ] {
            let e = super::seat_rotation(&fp, &seat(Edge::Top, Toward::Right, xp, yp), "U5")
                .unwrap_err();
            assert_eq!(
                (e.kind(), e.id()),
                ("ImuRotationUnplaceable", Some("U5")),
                "{e}"
            );
            assert!(e.to_string().contains(why), "{why}: {e}");
        }
    }

    #[test]
    fn an_arcs_circle_holds_its_three_points() {
        // A quarter of the unit circle, in nanometres, and the XT30PW-M's
        // first silkscreen arc (start, mid, end, mm).
        let ((x, y), r) =
            super::circle([(1_000_000, 0), (707_107, 707_107), (0, 1_000_000)]).expect("a circle");
        assert!(x.abs() <= 1 && y.abs() <= 1, "{x} {y}");
        assert!((1_000_000..=1_000_004).contains(&r), "{r}");
        let pts = [
            (-8_150_000, -8_890_000),
            (-9_260_000, -10_000_000),
            (-8_150_000, -11_110_000),
        ];
        let ((x, y), r) = super::circle(pts).expect("a circle");
        for (px, py) in pts {
            let d2 = i128::from(px - x).pow(2) + i128::from(py - y).pow(2);
            assert!(d2 <= i128::from(r).pow(2), "({px},{py}) outside r {r}");
        }
        assert_eq!(super::circle([(0, 0), (1, 1), (2, 2)]), None);
    }

    use super::*;

    /// How a placed part moves by `d`: [`shift`], or a test's broken one.
    type Mover = fn(&mut Placed, (i64, i64));

    thread_local! {
        /// The seam `compact` moves its members through in test builds
        /// (`AET-WO-1313d`): [`shift`] unless a test puts another in its own
        /// thread's place.
        pub(super) static MOVE: std::cell::Cell<Mover> = std::cell::Cell::new(shift);
    }

    /// `AET-WO-1313d`: the bound's arithmetic on two hand-built groups, from
    /// `move_bound`'s proof. STEP is 0.25 mm; SIDESTEP_STEPS is 40.
    /// - Group A starts 1.1 mm off in y (4 moves in y) and 0.6 mm in x (2
    ///   moves in x before its first move in y). In a 20 mm core (80 steps)
    ///   the sidestep caps the x-moves after each y-move at 40:
    ///   4 + 2 + 4 x 40 = 166. In a 5 mm core (20 steps) the width does:
    ///   4 + 2 + 4 x 20 = 86.
    /// - Group B starts 0.1 mm off in y, under a step (none in y), and 2.6 mm
    ///   in x (10): 0 + 10 + 0 = 10.
    #[test]
    fn the_move_bound_follows_the_termination_argument() {
        let mm = |v: f64| (v * 1_000_000.0).round() as i64;
        let a = (mm(1.1), mm(0.6));
        let b = (mm(0.1), mm(2.6));
        assert_eq!(move_bound([a].into_iter(), mm(20.0)), 166);
        assert_eq!(move_bound([a].into_iter(), mm(5.0)), 86);
        assert_eq!(move_bound([b].into_iter(), mm(20.0)), 10);
        assert_eq!(move_bound([a, b].into_iter(), mm(20.0)), 176);
        assert_eq!(move_bound(std::iter::empty(), mm(20.0)), 0);
    }

    /// `AET-WO-1313d`: compaction refuses by name, at once, when its
    /// termination argument fails, instead of running for good.
    /// - A move that leaves the keep-out box behind (`1313c`'s mutant of
    ///   `shift`) does not lower the potential: refused at that move.
    /// - A move of one nanometre lowers it, but far more slowly than the
    ///   argument allows: refused when the moves pass the bound.
    #[test]
    fn a_compaction_whose_termination_argument_fails_is_refused_by_name() {
        let gnd = || Conn::Rail("GND".into());
        let parts = vec![
            mcu(&[("57", net("SDA")), ("58", net("SCL"))]),
            part(
                "Sensor:BME280",
                "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
                "U2",
                &[("1", gnd()), ("3", net("SDA")), ("4", net("SCL"))],
            ),
        ];
        let items = items(&parts).unwrap();
        let ok = layout_of(&items, &[], &[], Some(bound(100, 100)));
        assert!(ok.is_ok(), "the design compacts with shift");
        let keep_left_behind: Mover = |p, d| {
            p.origin = (p.origin.0 + d.0, p.origin.1 + d.1);
            p.courtyard = p.courtyard.shift(d);
        };
        let one_nm: Mover = |p, d| shift(p, (d.0.signum(), d.1.signum()));
        for (mover, why) in [
            (keep_left_behind, "the move did not lower its potential"),
            (one_nm, "the moves passed the bound"),
        ] {
            MOVE.with(|m| m.set(mover));
            let t = std::time::Instant::now();
            let Err(e) = layout_of(&items, &[], &[], Some(bound(100, 100))) else {
                panic!("{why}: compaction was not refused");
            };
            let took = t.elapsed();
            MOVE.with(|m| m.set(shift));
            assert_eq!(
                (e.kind(), e.id()),
                ("CompactionUnbounded", Some("U2")),
                "{e}"
            );
            assert!(e.to_string().contains(why), "{why}: {e}");
            assert!(took < std::time::Duration::from_secs(1), "{why}: {took:?}");
        }
    }

    fn part(
        lib_id: &'static str,
        footprint: &str,
        reference: &str,
        conns: &[(&str, Conn)],
    ) -> Part {
        Part {
            lib_id,
            reference: reference.to_owned(),
            value: "v".to_owned(),
            footprint: footprint.to_owned(),
            conns: conns
                .iter()
                .map(|(n, c)| ((*n).to_owned(), c.clone()))
                .collect(),
            note: None,
            owner: None,
            offboard: BTreeMap::new(),
            seat: None,
        }
    }

    const MCU: &str = "MCU_ST_STM32H7:STM32H753ZITx";
    const LQFP: &str = "Package_QFP:LQFP-144_20x20mm_P0.5mm";
    const R0402: &str = "Resistor_SMD:R_0402_1005Metric";

    fn net(n: &str) -> Conn {
        Conn::Net(n.to_owned())
    }

    /// `AET-WO-4400d`: a seated part's pads and geometry are carried at its
    /// turn, so the parts placed by its pins (its capacitors) target the
    /// turned pins, and its keep-out box is the turned one.
    #[test]
    fn a_seated_part_carries_its_pads_and_geometry_at_its_turn() {
        use aethelgard_ir::orientation::{Edge, Orientation, Toward};
        const FP: &str = "Aethelgard_Sensor:TDK_LGA-14_3x2.5mm_P0.5mm";
        let mut p = part(
            "Aethelgard_Sensor:ICM-42688-P",
            FP,
            "U5",
            &[("8", Conn::Rail("VDD_3V3".into())), ("12", net("IMU_CS"))],
        );
        let pins = |v: &[&str]| v.iter().map(|x| (*x).to_owned()).collect::<Vec<_>>();
        p.seat = Some(crate::schematic::Seat {
            forward: Edge::Top,
            orientation: Orientation {
                x: Toward::Forward,
                z: Toward::Up,
            },
            x_toward_pins: pins(&["8", "9", "10", "11"]),
            y_toward_pins: pins(&["12", "13", "14"]),
        });
        let parts = [p];
        let items = items(&parts).unwrap();
        let fp = crate::library::footprint(FP).unwrap();
        assert_eq!(items[0].rotation, 90);
        let turned = geometry_at(&fp, "U5", 90).unwrap();
        assert_eq!(items[0].geometry.courtyard, turned.courtyard);
        assert_eq!(items[0].geometry.keep, turned.keep);
        assert_ne!(
            items[0].geometry.courtyard,
            geometry(&fp, "U5").unwrap().courtyard
        );
        // Pad 8, (1.1625, 0.75) mm in the library, turned a quarter.
        assert_eq!(items[0].pads["8"].0, (750_000, -1_162_500));
    }

    /// The MCU with its supply pins wired as the schematic emitter wires
    /// them, plus `extra`.
    fn mcu(extra: &[(&str, Conn)]) -> Part {
        let pins = schematic::pins(&library::symbol(MCU).unwrap()).unwrap();
        let mut conns: Vec<(String, Conn)> = Vec::new();
        for p in pins {
            let n = p.name.as_str();
            let c = if n.starts_with("VDD") || matches!(n, "VBAT" | "VREF+" | "PDR_ON") {
                Conn::Rail("VDD".into())
            } else if n.starts_with("VSS") || n == "BOOT0" {
                Conn::Rail("GND".into())
            } else if n == "VCAP" {
                net(&format!("VCAP{}", p.number))
            } else {
                continue;
            };
            conns.push((p.number, c));
        }
        let mut m = part(MCU, LQFP, "U1", &[]);
        m.conns = conns.into_iter().collect();
        m.conns
            .extend(extra.iter().map(|(n, c)| ((*n).to_owned(), c.clone())));
        m
    }

    #[test]
    fn net_names_are_kicads() {
        let tmp = part(
            "Sensor_Temperature:TMP117xxDRV",
            "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
            "U\"3",
            &[
                ("1", net("SCL")),
                ("2", Conn::Rail("GND".into())),
                ("5", Conn::Rail("VDD_3V3".into())),
            ],
        );
        let nets = pin_nets(&tmp).unwrap();
        assert_eq!(nets["1"], "/SCL");
        assert_eq!(nets["2"], "GND");
        assert_eq!(nets["5"], "VDD_3V3");
        assert_eq!(nets["3"], "unconnected-(U\"3-ALERT-Pad3)");
        assert_eq!(nets["6"], "unconnected-(U\"3-SDA-Pad6)");
        assert!(!nets.contains_key("7"), "the exposed pad has no pin");
        assert_eq!(escape("a/b\nc"), "a{slash}bc");
    }

    #[test]
    fn the_exposed_pad_carries_no_net() {
        let parts = [
            mcu(&[]),
            part(
                "Sensor_Temperature:TMP117xxDRV",
                "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
                "U3",
                &[("2", Conn::Rail("GND".into()))],
            ),
        ];
        let l = layout(&parts).unwrap();
        assert_eq!(l.pad_nets[&("U3".to_owned(), "2".to_owned())], "GND");
        assert!(!l.pad_nets.contains_key(&("U3".to_owned(), "7".to_owned())));
    }

    /// Many parts with long references around one MCU: no two keep-out
    /// boxes (so no two courtyards) meet, and every courtyard is inside the
    /// outline by at least the board-edge clearance (`AET-WO-1301` §5.3).
    #[test]
    fn many_parts_with_long_references_never_overlap_or_leave_the_board() {
        let mut parts = vec![mcu(&[])];
        let mut mcu_conns = Vec::new();
        for i in 0..60 {
            let n = format!("N{i}");
            mcu_conns.push((format!("{}", 34 + i), net(&n)));
            parts.push(part(
                "Device:R",
                R0402,
                &format!("R_A_VERY_LONG_REFERENCE_DESIGNATOR_{i:03}"),
                &[("1", net(&n)), ("2", Conn::Rail("GND".into()))],
            ));
        }
        parts[0].conns.extend(mcu_conns);
        let l = layout(&parts).unwrap();
        assert_eq!(l.placed.len(), 61);
        let inner = l.outline.grow(-FLOOR_NM);
        for (i, a) in l.placed.iter().enumerate() {
            assert!(
                inner.contains(a.courtyard),
                "{} leaves the board",
                a.reference
            );
            assert!(a.keep.contains(a.courtyard));
            for b in &l.placed[i + 1..] {
                assert!(
                    !a.keep.grow(GAP_NM).overlaps(b.keep),
                    "{} is within {GAP_NM} nm of {}",
                    a.reference,
                    b.reference
                );
                assert!(!a.courtyard.overlaps(b.courtyard));
            }
        }
    }

    /// The placement is a function of the parts, not of their order.
    #[test]
    fn the_board_does_not_depend_on_the_parts_order() {
        let parts = vec![
            mcu(&[("34", net("A")), ("35", net("B"))]),
            part("Device:R", R0402, "R2", &[("1", net("A")), ("2", net("B"))]),
            part(
                "Device:R",
                R0402,
                "R10",
                &[("1", net("B")), ("2", Conn::Rail("GND".into()))],
            ),
            part(
                "Device:C",
                "Capacitor_SMD:C_0402_1005Metric",
                "C1",
                &[("1", net("A"))],
            ),
        ];
        let forward = write("p", &parts, &crate::stack::reference()).unwrap();
        let mut reversed = parts.clone();
        reversed.reverse();
        assert_eq!(
            forward,
            write("p", &reversed, &crate::stack::reference()).unwrap()
        );
        assert!(forward.starts_with("(kicad_pcb\n\t(version 20260206)"));
    }

    /// Parts anchored to one IC, contending for the spots by the same pin:
    /// each takes its place in the IC's draw sequence ([`Anchor::seq`]),
    /// whatever the order of the parts list (`AET-WO-1313c` F1). The draw
    /// sequence here runs against reference order, so neither the list nor
    /// the references can stand in for it.
    ///
    /// [`Anchor::seq`]: schematic::Anchor
    fn anchored_design() -> Vec<Part> {
        let gnd = || Conn::Rail("GND".into());
        let two = |lib: &'static str, fp: &str, r: &str, n: &str| {
            part(lib, fp, r, &[("1", net(n)), ("2", gnd())])
        };
        let at = |p: Part, pin: &str, seq: usize| anchor_to(p, "U2", &[("1", pin)], None, seq);
        vec![
            mcu(&[("34", net("SDA")), ("35", net("SCL"))]),
            bme("U2"),
            at(two("Device:C", C0402, "C9", "SDA"), "3", 0),
            at(two("Device:C", C0402, "C5", "SDA"), "3", 1),
            at(two("Device:R", R0402, "R1", "SDA"), "3", 2),
            at(two("Device:R", R0402, "R7", "SCL"), "4", 3),
        ]
    }

    /// With anchored parts too, the board is a function of the parts, not of
    /// their order: every rotation of the list, and its reverse.
    #[test]
    fn with_anchored_parts_the_board_does_not_depend_on_the_parts_order() {
        let parts = anchored_design();
        let stack = crate::stack::reference();
        // The first in the draw sequence has first pick by U2's pin 3.
        let l = layout(&parts).unwrap();
        let at = |r: &str| l.placed.iter().find(|p| p.reference == r).unwrap().keep;
        let pin = {
            let u2 = l.placed.iter().find(|p| p.reference == "U2").unwrap();
            let items = items(&parts).unwrap();
            let (pad, _) = items[1].pads["3"];
            (u2.origin.0 + pad.0, u2.origin.1 + pad.1)
        };
        let d2 = |k: Rect| {
            let (x, y) = k.centre();
            (x - pin.0).pow(2) + (y - pin.1).pow(2)
        };
        assert!(
            d2(at("C9")) < d2(at("C5")),
            "C9 (seq 0) did not have first pick"
        );
        same_in_every_order(&parts, |p| write("p", p, &stack).unwrap());
    }

    /// `board` gives the same bytes for every rotation of `parts`, and for
    /// each reversed.
    fn same_in_every_order(parts: &[Part], board: impl Fn(&[Part]) -> String) {
        let forward = board(parts);
        for k in 0..parts.len() {
            let mut turned = parts.to_vec();
            turned.rotate_left(k);
            assert_eq!(forward, board(&turned), "rotated by {k}");
            turned.reverse();
            assert_eq!(forward, board(&turned), "rotated by {k}, reversed");
        }
    }

    /// A decoupling capacitor sits beside its own IC's power pin, not the
    /// MCU's, though both are on its rail.
    #[test]
    fn a_decoupling_capacitor_goes_beside_its_ics_power_pin() {
        let vdd = || Conn::Rail("VDD".into());
        let gnd = || Conn::Rail("GND".into());
        let mut cap = part(
            "Device:C",
            "Capacitor_SMD:C_0402_1005Metric",
            "C1",
            &[("1", vdd()), ("2", gnd())],
        );
        cap.owner = Some(schematic::Owner {
            ic: "U2".into(),
            anchor: None,
            decouples: true,
        });
        let parts = vec![
            mcu(&[("34", net("SCL"))]),
            part(
                "Sensor_Temperature:TMP117xxDRV",
                "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
                "U2",
                &[("1", net("SCL")), ("5", vdd()), ("2", gnd())],
            ),
            cap,
        ];
        let l = layout(&parts).unwrap();
        let at = |r: &str| l.placed.iter().find(|p| p.reference == r).unwrap().keep;
        let d2 = |a: Rect, b: Rect| {
            let ((ax, ay), (bx, by)) = (a.centre(), b.centre());
            (ax - bx).pow(2) + (ay - by).pow(2)
        };
        assert!(d2(at("C1"), at("U2")) < d2(at("C1"), at("U1")));
        assert!(
            d2(at("C1"), at("U2")) < 4_000_000i64.pow(2),
            "C1 is not beside U2"
        );
    }

    /// What the emitter cannot name or measure is refused, never guessed.
    #[test]
    fn what_the_board_cannot_model_is_refused() {
        let err = |r: Result<Layout, EmitError>| match r {
            Err(EmitError::Internal(m)) => m,
            other => panic!("{:?}", other.map(|l| l.placed.len())),
        };
        // An MCU with its supply pins open: KiCad names a stacked group of
        // unconnected pins after one of them, which is not modelled.
        let m = err(layout(&[part(MCU, LQFP, "U1", &[])]));
        assert!(m.contains("stacked unconnected pin"), "{m}");
        // A symbol pin with no pad in the footprint.
        let mut small = mcu(&[]);
        small.reference = "U9".into();
        small.footprint = R0402.into();
        let m = err(layout(&[mcu(&[]), small]));
        assert!(m.contains("has no pad in"), "{m}");
        let fp = |body: &str| sexpr::parse(&format!("(footprint \"x\" {body})")).unwrap();
        for (body, want) in [
            ("", "no courtyard"),
            (
                "(fp_rect (start -1 -1) (end 1 1) (layer \"F.CrtYd\")) (property \"Reference\" \"R\" (at 0 0 45) (layer \"F.SilkS\") (effects (font (size 1 1))))",
                "at angle 45",
            ),
        ] {
            match geometry(&fp(body), "R1") {
                Err(EmitError::Internal(m)) => assert!(m.contains(want), "{body}: {m}"),
                other => panic!("{body}: {other:?}"),
            }
        }
        // A circle is measured as its box (`AET-WO-1312`, the test pad).
        let g = geometry(
            &fp("(fp_circle (center 0 0) (end 0.6 0.8) (layer \"F.CrtYd\"))"),
            "R1",
        )
        .unwrap();
        assert_eq!(
            (g.courtyard.x0, g.courtyard.y1),
            (-1_000_000, 1_000_000),
            "{g:?}"
        );
        // A text turned 90 degrees is measured turned.
        let g = geometry(
            &fp("(fp_rect (start 0 0) (end 0.1 0.1) (layer \"F.CrtYd\")) (property \"Reference\" \"R\" (at 5 5 90) (layer \"F.SilkS\") (effects (font (size 1 1) (thickness 0.1))))"),
            "R1234567890",
        )
        .unwrap();
        assert!(g.keep.y1 - g.keep.y0 > g.keep.x1 - g.keep.x0, "{g:?}");
    }

    /// The reference text is part of the keep-out box, so a long one moves
    /// the neighbours away.
    #[test]
    fn the_reference_text_widens_the_keep_out_box() {
        let fp = library::footprint(R0402).unwrap();
        let short = geometry(&fp, "R1").unwrap();
        let long = geometry(&fp, "R_A_VERY_LONG_REFERENCE").unwrap();
        assert!(
            short.keep.y0 < short.courtyard.y0,
            "the text sits above the courtyard"
        );
        assert!(long.keep.x1 - long.keep.x0 > 20_000_000, "{long:?}");
        assert_eq!(short.courtyard, long.courtyard);
    }

    /// Two parts with one footprint get distinct UUIDs throughout.
    #[test]
    fn every_uuid_on_the_board_is_distinct() {
        let bme = |r: &str| {
            part(
                "Sensor:BME280",
                "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
                r,
                &[
                    ("1", Conn::Rail("GND".into())),
                    ("7", Conn::Rail("GND".into())),
                    ("2", Conn::Rail("VDD".into())),
                    ("6", Conn::Rail("VDD".into())),
                    ("8", Conn::Rail("VDD".into())),
                    ("3", net("SDA")),
                    ("4", net("SCL")),
                    ("5", net("SDO")),
                ],
            )
        };
        let board = write(
            "p",
            &[mcu(&[("34", net("SDA"))]), bme("U2"), bme("U3")],
            &crate::stack::reference(),
        )
        .unwrap();
        let uuids: Vec<&str> = board
            .match_indices("(uuid \"")
            .map(|(i, m)| &board[i + m.len()..i + m.len() + 36])
            .collect();
        assert!(uuids.len() > 60, "{}", uuids.len());
        let distinct: BTreeSet<&str> = uuids.iter().copied().collect();
        assert_eq!(distinct.len(), uuids.len());
    }

    #[test]
    fn every_vendored_footprint_has_a_courtyard_inside_its_keep_out_box() {
        for id in [
            LQFP,
            R0402,
            "Package_SON:WSON-6-1EP_2x2mm_P0.65mm_EP1x1.6mm",
            "Capacitor_SMD:C_0402_1005Metric",
            "Package_SO:SOIC-8_5.3x5.3mm_P1.27mm",
            "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
            "Package_SO:Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm",
            "Package_TO_SOT_SMD:SOT-23-5",
            "Package_SO:TSSOP-14_4.4x5mm_P0.65mm",
            "Inductor_SMD:L_Vishay_IHLP-5050",
            "Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal",
            "Capacitor_SMD:C_0603_1608Metric",
            "Capacitor_SMD:C_1210_3225Metric",
            "Resistor_SMD:R_0603_1608Metric",
            "Sensor_Motion:InvenSense_QFN-24_3x3mm_P0.4mm",
        ] {
            let g = geometry(&library::footprint(id).unwrap(), "R1").unwrap();
            assert!(g.keep.contains(g.courtyard), "{id}");
            assert!(
                g.courtyard.x0 < g.courtyard.x1 && g.courtyard.y0 < g.courtyard.y1,
                "{id}"
            );
        }
    }

    /// A connector of `name` as `design` makes one: pins 1..N on `net`
    /// (the last on GND), the mounting pin on GND.
    fn connector(name: &str, reference: &str, net_name: Option<&str>) -> Part {
        let (sym, fp) = schematic::connector_library(name).unwrap();
        let n = aethelgard_ir::connectors::part(name).unwrap().pins;
        let mut conns: Vec<(String, Conn)> = (1..=n)
            .map(|i| {
                let c = match (net_name, i == n) {
                    (_, true) => Conn::Rail("GND".into()),
                    (Some(x), false) => net(x),
                    (None, false) => Conn::NoConnect,
                };
                (i.to_string(), c)
            })
            .collect();
        conns.push(("MP".into(), Conn::Rail("GND".into())));
        let mut p = part(sym, fp, reference, &[]);
        p.conns = conns.into_iter().collect();
        p
    }

    fn passives() -> Vec<Part> {
        vec![
            mcu(&[("34", net("A")), ("35", net("B"))]),
            part("Device:R", R0402, "R2", &[("1", net("A")), ("2", net("B"))]),
            part(
                "Device:R",
                R0402,
                "R10",
                &[("1", net("B")), ("2", Conn::Rail("GND".into()))],
            ),
        ]
    }

    /// A bound of `width` x `height` whole millimetres.
    fn bound(width: u32, height: u32) -> OutlineBound {
        OutlineBound {
            width: width * 1000,
            height: height * 1000,
        }
    }

    /// `AET-WO-1313` §2.1: a bound that cannot hold one part, or the parts'
    /// keep-out boxes together, is refused by name before any placement.
    #[test]
    fn a_bound_that_cannot_hold_the_parts_is_refused_before_placement() {
        let parts = passives();
        bound_holds(&parts, bound(100, 100)).unwrap();
        // The MCU's keep-out box is about 23.3 mm a side: 25 mm less two
        // 2 mm margins is 21 mm.
        match bound_holds(&parts, bound(25, 100)) {
            Err(EmitError::OutlineTooSmall {
                reference: Some(r), ..
            }) => assert_eq!(r, "U1"),
            other => panic!("{other:?}"),
        }
        // Every part fits alone at 30 x 30 mm, but not with the others.
        let mut many = passives();
        for i in 0..40 {
            many.push(connector("SM04B-GHS-TB", &format!("J{i}"), None));
        }
        let e = bound_holds(&many, bound(30, 30)).unwrap_err();
        assert_eq!(e.kind(), "OutlineTooSmall");
        assert_eq!(e.id(), None, "{e}");
        // Margins alone: nothing fits inside a 4 x 4 mm bound.
        assert_eq!(
            bound_holds(&parts, bound(4, 4)).unwrap_err().kind(),
            "OutlineTooSmall"
        );
    }

    /// Under a bound the connectors go on the left and right edges, cable
    /// side out (`BOUNDED_PLAN`); a bound the placer cannot reach is
    /// refused with the outline it reached, never a partial board.
    #[test]
    fn under_a_bound_connectors_go_on_the_side_edges_or_the_bound_is_refused() {
        let mut parts = passives();
        for (name, r) in [
            ("SM06B-GHS-TB", "J3"),
            ("SM04B-GHS-TB", "J1"),
            ("SM10B-GHS-TB", "J10"),
        ] {
            parts.push(connector(name, r, None));
        }
        let items = items(&parts).unwrap();
        let l = layout_of(&items, &[], &[], Some(bound(100, 100))).unwrap();
        let o = l.outline;
        let at = |r: &str| l.placed.iter().find(|p| p.reference == r).unwrap();
        // The canonical rank, by courtyard (none carries a net): J10, J3,
        // J1, each to the side with less length so far: left, right, right.
        for (r, rotation) in [("J10", 270), ("J3", 90), ("J1", 90)] {
            let c = at(r);
            assert_eq!(c.rotation, rotation, "{r}");
            let gap = if rotation == 270 {
                c.courtyard.x0 - o.x0
            } else {
                o.x1 - c.courtyard.x1
            };
            assert!(
                (0..SNAP).contains(&gap),
                "{r}'s cable side is {gap} nm from its edge"
            );
        }
        for (i, a) in l.placed.iter().enumerate() {
            assert!(o.contains(a.keep), "{} leaves the board", a.reference);
            for b in &l.placed[i + 1..] {
                assert!(
                    !a.keep.overlaps(b.keep),
                    "{} overlaps {}",
                    a.reference,
                    b.reference
                );
            }
        }
        // The same parts with no bound: today's bottom row.
        let today = layout(&parts).unwrap();
        assert!(today.placed.iter().all(|p| p.rotation == 0));
        // A bound the keep-out sum allows but the placer cannot reach.
        let (w, h) = (o.x1 - o.x0, o.y1 - o.y0);
        let tight = bound(
            u32::try_from((w - 1) / 1_000_000).unwrap(),
            u32::try_from(h / 1_000_000 + 1).unwrap(),
        );
        bound_holds(&parts, tight).unwrap();
        match layout_of(&items, &[], &[], Some(tight)) {
            Err(EmitError::OutlineUnreachable {
                reached_um_w,
                over_um_w,
                ..
            }) => {
                assert_eq!(reached_um_w, w / 1000);
                assert!(over_um_w > 0);
            }
            other => panic!("{:?}", other.map(|l| l.outline)),
        }
    }

    fn bme(r: &str) -> Part {
        part(
            "Sensor:BME280",
            "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
            r,
            &[
                ("1", Conn::Rail("GND".into())),
                ("2", Conn::Rail("VDD".into())),
                ("3", net("SDA")),
                ("4", net("SCL")),
            ],
        )
    }

    /// (r): under a bound, an IC that owns an anchored part is placed before
    /// the other ICs, whatever its reference; with no bound, today's
    /// reference order.
    #[test]
    fn under_a_bound_an_ic_owning_anchored_parts_is_placed_first() {
        let mut owned = part("Device:R", R0402, "R1", &[("1", net("SDA"))]);
        owned.owner = Some(schematic::Owner {
            ic: "U9".into(),
            anchor: Some(schematic::Anchor {
                pads: vec![("1".into(), "3".into())],
                loop_rule: None,
                seq: 0,
            }),
            decouples: false,
        });
        let parts = vec![mcu(&[("34", net("SDA"))]), bme("U2"), bme("U9"), owned];
        let items = items(&parts).unwrap();
        let names = |order: Vec<usize>| -> Vec<&str> {
            order
                .into_iter()
                .map(|i| items[i].part.reference.as_str())
                .collect()
        };
        assert_eq!(names(placing_order(&items, None)), ["U1", "U2", "U9", "R1"]);
        let rank = canonical_rank(&items, 0);
        assert_eq!(
            names(placing_order(&items, Some(&rank))),
            ["U1", "U9", "U2", "R1"]
        );
    }

    /// Connectors go at the bottom edge, in reference order.
    #[test]
    fn connectors_go_at_the_bottom_edge_in_reference_order() {
        let mut parts = passives();
        parts.push(connector("SM06B-GHS-TB", "J3", None));
        parts.push(connector("SM04B-GHS-TB", "J1", None));
        parts.push(connector("SM10B-GHS-TB", "J10", None));
        let l = layout(&parts).unwrap();
        let items = items(&parts).unwrap();
        let at = |r: &str| l.placed.iter().find(|p| p.reference == r).unwrap();
        // String order: J1 < J10 < J3.
        let (j1, j10, j3) = (at("J1"), at("J10"), at("J3"));
        assert!(
            j1.keep.x1 < j10.keep.x0 && j10.keep.x1 < j3.keep.x0,
            "connectors are not in reference (string) order left to right"
        );
        assert_eq!(
            j1.origin.1, j3.origin.1,
            "the connectors are not in one row"
        );
        let o = l.outline;
        assert_eq!(o.y1 % SNAP, 0, "the outline's bottom is off the 50 um grid");
        assert_eq!(o.x0 % SNAP, 0, "the outline's left is off the 50 um grid");
        assert_eq!(o.y0 % SNAP, 0, "the outline's top is off the 50 um grid");
        assert_eq!(o.x1 % SNAP, 0, "the outline's right is off the 50 um grid");
        for c in [j1, j10, j3] {
            assert!(
                c.courtyard.y1 <= o.y1,
                "{} courtyard is past the bottom edge",
                c.reference
            );
            assert!(
                c.courtyard.y1 > o.y1 - SNAP,
                "{} courtyard bottom is more than one SNAP above the edge",
                c.reference
            );
            let i = items
                .iter()
                .position(|i| i.part.reference == c.reference)
                .unwrap();
            for pad in items[i].fp.children("pad") {
                let r = pad_box(pad, &c.reference).unwrap();
                assert!(
                    c.origin.1 + r.y1 + route::EDGE_NM <= o.y1,
                    "{} pad {} is nearer the edge than EDGE_NM",
                    c.reference,
                    pad.arg(1).unwrap_or("")
                );
            }
            for other in &l.placed {
                if other.reference == c.reference {
                    continue;
                }
                assert!(
                    !c.keep.grow(IC_GAP_NM).overlaps(other.keep),
                    "{} keep box is within IC_GAP_NM of {}",
                    c.reference,
                    other.reference
                );
            }
        }
        assert!(
            l.placed
                .iter()
                .all(|p| p.keep.y1 <= o.y1 - MARGIN_NM || p.reference.starts_with('J')),
            "a non-connector keep box is within MARGIN_NM of the bottom edge"
        );
    }

    /// A connector moves nothing else.
    #[test]
    fn a_connector_moves_no_other_part() {
        let without = layout(&passives()).unwrap();
        let mut parts = passives();
        parts.push(connector("SM04B-GHS-TB", "J1", Some("A")));
        let with = layout(&parts).unwrap();
        assert_eq!(with.placed.len(), without.placed.len() + 1);
        for p in &without.placed {
            let q = with.placed.iter().find(|q| q.reference == p.reference);
            assert_eq!(
                q.map(|q| q.origin),
                Some(p.origin),
                "adding a connector moved {}",
                p.reference
            );
        }
    }

    /// Every connector part of the IR table fits the edge rule, alone and
    /// together, and the board writes.
    #[test]
    fn every_connector_part_places_and_writes() {
        for row in aethelgard_ir::connectors::PARTS.iter() {
            let mut parts = passives();
            parts.push(connector(row.name, "J1", None));
            let l = layout(&parts).unwrap_or_else(|e| panic!("{} does not place: {e:?}", row.name));
            let j = l.placed.iter().find(|p| p.reference == "J1").unwrap();
            assert!(
                j.courtyard.y1 <= l.outline.y1 && j.courtyard.y1 > l.outline.y1 - SNAP,
                "{} courtyard is not at the edge",
                row.name
            );
            write("p", &parts, &crate::stack::reference())
                .unwrap_or_else(|e| panic!("{} does not write: {e:?}", row.name));
        }
        let all: Vec<Part> = passives()
            .into_iter()
            .chain(
                aethelgard_ir::connectors::PARTS
                    .iter()
                    .enumerate()
                    .map(|(i, r)| connector(r.name, &format!("J{i}"), None)),
            )
            .collect();
        layout(&all).expect("all four connector parts together do not place");
    }

    /// `part` anchored to `ic`'s pads, `seq` in its draw sequence.
    fn anchor_to(
        mut part: Part,
        ic: &str,
        pads: &[(&str, &str)],
        loop_rule: Option<(i64, &'static str)>,
        seq: usize,
    ) -> Part {
        part.owner = Some(schematic::Owner {
            ic: ic.into(),
            anchor: Some(schematic::Anchor {
                pads: pads
                    .iter()
                    .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
                    .collect(),
                loop_rule,
                seq,
            }),
            decouples: false,
        });
        part
    }

    const C0402: &str = "Capacitor_SMD:C_0402_1005Metric";

    /// `AET-WO-1313c`: a design for the order tests under a bound. The MCU
    /// with a part anchored to it; U2 and U3, which own anchored parts (U3's
    /// with a loop rule), their draw sequences against reference order; U4,
    /// which owns none; C20 and C21, which differ only in value, so only
    /// the last tie-break tells them apart; a test point on a signal and one
    /// on `GND`; two connectors on the side edges.
    fn bounded_design() -> Vec<Part> {
        let vdd = || Conn::Rail("VDD".into());
        let gnd = || Conn::Rail("GND".into());
        let two = |lib: &'static str, fp: &str, r: &str, one: Conn, two: Conn| {
            part(lib, fp, r, &[("1", one), ("2", two)])
        };
        let mut c20 = two("Device:C", C0402, "C20", vdd(), gnd());
        c20.value = "100n".into();
        let mut c21 = two("Device:C", C0402, "C21", vdd(), gnd());
        c21.value = "1u".into();
        let tp = |r: &str, c: Conn| {
            let mut p = part(
                schematic::TEST_POINT,
                "TestPoint:TestPoint_Pad_D1.0mm",
                r,
                &[("1", c)],
            );
            p.value = "TestPoint".into();
            p
        };
        vec![
            mcu(&[("34", net("SDA")), ("35", net("SCL")), ("36", net("INT"))]),
            anchor_to(
                two("Device:R", R0402, "R30", net("INT"), vdd()),
                "U1",
                &[("1", "36")],
                None,
                0,
            ),
            bme("U2"),
            anchor_to(
                two("Device:C", C0402, "C9", net("SDA"), gnd()),
                "U2",
                &[("1", "3")],
                None,
                0,
            ),
            anchor_to(
                two("Device:C", C0402, "C5", net("SDA"), gnd()),
                "U2",
                &[("1", "3")],
                None,
                1,
            ),
            anchor_to(
                two("Device:R", R0402, "R7", net("SCL"), vdd()),
                "U2",
                &[("1", "4")],
                None,
                2,
            ),
            bme("U3"),
            anchor_to(
                two("Device:C", C0402, "C12", vdd(), gnd()),
                "U3",
                &[("1", "2")],
                Some((2_000_000, "a test's rule")),
                0,
            ),
            anchor_to(
                two("Device:R", R0402, "R12", net("SDA"), vdd()),
                "U3",
                &[("1", "3")],
                None,
                1,
            ),
            bme("U4"),
            c20,
            c21,
            tp("TP1", net("SDA")),
            tp("TP2", gnd()),
            connector("SM04B-GHS-TB", "J1", Some("SCL")),
            connector("SM06B-GHS-TB", "J2", None),
        ]
    }

    /// Every reference renamed so that each class (its letters and its
    /// length) runs in reverse: the canonical rank's last tie-break, and
    /// every other order references could set, reversed. An owner follows
    /// its IC's new name.
    fn references_reversed(parts: &[Part]) -> Vec<Part> {
        let class = |r: &str| {
            (
                r.chars()
                    .take_while(char::is_ascii_alphabetic)
                    .collect::<String>(),
                r.len(),
            )
        };
        let mut by_class: BTreeMap<(String, usize), Vec<String>> = BTreeMap::new();
        for p in parts {
            by_class
                .entry(class(&p.reference))
                .or_default()
                .push(p.reference.clone());
        }
        let mut to: BTreeMap<String, String> = BTreeMap::new();
        for refs in by_class.values_mut() {
            refs.sort();
            for (a, b) in refs.iter().zip(refs.iter().rev()) {
                to.insert(a.clone(), b.clone());
            }
        }
        parts
            .iter()
            .cloned()
            .map(|mut p| {
                p.reference = to[&p.reference].clone();
                if let Some(o) = p.owner.as_mut() {
                    o.ic = to[&o.ic].clone();
                }
                p
            })
            .collect()
    }

    /// A placement with every name taken out: the outline, and each part's
    /// footprint, origin and rotation, sorted.
    type NameFree = (Rect, Vec<(String, (i64, i64), u16)>);

    fn name_free(parts: &[Part], l: &Layout) -> NameFree {
        let mut v: Vec<_> = l
            .placed
            .iter()
            .map(|p| {
                let part = parts.iter().find(|x| x.reference == p.reference).unwrap();
                (part.footprint.clone(), p.origin, p.rotation)
            })
            .collect();
        v.sort();
        (l.outline, v)
    }

    /// `AET-WO-1313c` §2: under a declared bound the board is a function of
    /// the parts, not of their order: every rotation of the list, and its
    /// reverse, places every part the same, and the reversed list writes the
    /// same bytes. (Writing this bounded board takes over a second in a test
    /// build, the writer's cost on its outline, so the bytes are compared
    /// once.)
    #[test]
    fn under_a_bound_the_board_does_not_depend_on_the_parts_order() {
        let parts = bounded_design();
        let b = Some(bound(100, 100));
        same_in_every_order(&parts, |p| {
            let l = layout_of(&items(p).unwrap(), &[], &[], b).unwrap();
            let mut placed: Vec<_> = l
                .placed
                .iter()
                .map(|q| (&q.reference, q.origin, q.rotation))
                .collect();
            placed.sort();
            format!("{:?} {placed:?}", l.outline)
        });
        let stack = crate::stack::reference();
        let mut reversed = parts.clone();
        reversed.reverse();
        assert_eq!(
            write_moved("p", &parts, &stack, &[], b).unwrap(),
            write_moved("p", &reversed, &stack, &[], b).unwrap()
        );
    }

    /// `AET-WO-1313c` §2: under a declared bound no reference decides where
    /// anything goes. With every reference class reversed, the reversed
    /// tie-break included, the placement is the same once names are taken
    /// out. With no bound, reference order still decides (today's placer,
    /// kept byte for byte), which this design shows, so the check is not
    /// empty.
    #[test]
    fn under_a_bound_no_reference_decides_a_placement() {
        let parts = bounded_design();
        let renamed = references_reversed(&parts);
        assert_ne!(
            parts.iter().map(|p| &p.reference).collect::<Vec<_>>(),
            renamed.iter().map(|p| &p.reference).collect::<Vec<_>>()
        );
        let place =
            |p: &[Part], b| name_free(p, &layout_of(&items(p).unwrap(), &[], &[], b).unwrap());
        let b = Some(bound(100, 100));
        assert_eq!(place(&parts, b), place(&renamed, b));
        assert_ne!(place(&parts, None), place(&renamed, None));
    }

    /// `AET-WO-1313c` §5: after compaction every keep-out box is inside the
    /// outline and keeps the placer's gaps ([`GAP_NM`], [`IC_GAP_NM`]
    /// between two ICs), and no part placed after the MCU enters a declared
    /// pair's approach strip. U2 shares only SDA and SCL with the MCU, on the
    /// pins beside the pair's, so its group, compacted toward the MCU's
    /// centre, would cross the strip in front of the pair's pins.
    #[test]
    fn compaction_keeps_the_placers_gaps_and_the_approach_strips() {
        let gnd = || Conn::Rail("GND".into());
        let sensor = |r: &str| {
            part(
                "Sensor:BME280",
                "Package_LGA:Bosch_LGA-8_2.5x2.5mm_P0.65mm_ClockwisePinNumbering",
                r,
                &[("1", gnd()), ("3", net("SDA")), ("4", net("SCL"))],
            )
        };
        let parts = vec![
            mcu(&[
                ("57", net("SDA")),
                ("58", net("SCL")),
                ("55", net("DP")),
                ("56", net("DN")),
            ]),
            sensor("U2"),
            anchor_to(
                part("Device:C", C0402, "C1", &[("1", net("SDA")), ("2", gnd())]),
                "U2",
                &[("1", "3")],
                Some((2_000_000, "a test's rule")),
                0,
            ),
            sensor("U3"),
        ];
        let pairs = [["DP".to_owned(), "DN".to_owned()]];
        let items = items(&parts).unwrap();
        let l = layout_of(&items, &[], &pairs, Some(bound(100, 100))).unwrap();
        let mcu = l.placed.iter().position(|p| p.reference == "U1").unwrap();
        let strips = approaches(&items[0], l.placed[mcu].courtyard, &pairs);
        assert!(!strips.is_empty());
        for (i, a) in l.placed.iter().enumerate() {
            assert!(
                l.outline.x0 <= a.keep.x0
                    && a.keep.x1 <= l.outline.x1
                    && l.outline.y0 <= a.keep.y0
                    && a.keep.y1 <= l.outline.y1,
                "{} leaves the outline",
                a.reference
            );
            if i != mcu {
                assert!(
                    strips.iter().all(|s| !s.overlaps(a.keep)),
                    "{} enters an approach strip",
                    a.reference
                );
            }
            for (j, b) in l.placed.iter().enumerate().skip(i + 1) {
                let both = is_ic(items[i].part) && is_ic(items[j].part);
                let gap = if both { IC_GAP_NM } else { GAP_NM };
                assert!(
                    !a.keep.grow(gap).overlaps(b.keep),
                    "{} and {} are closer than {gap} nm",
                    a.reference,
                    b.reference
                );
            }
        }
    }
}
