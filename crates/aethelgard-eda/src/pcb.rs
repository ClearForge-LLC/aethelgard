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
//! Every coordinate is nanometres, board Y down; every footprint sits at
//! rotation 0 on the front.

use std::collections::{BTreeMap, BTreeSet};

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
    pub(crate) fn grow(self, d: i64) -> Rect {
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
/// and half its stroke on every side. Anything else is refused.
fn graphic_box(item: &Sx, reference: &str) -> Result<Rect, EmitError> {
    let mut points = Vec::new();
    match item.head() {
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
    let first = *points
        .first()
        .ok_or_else(|| internal(reference, "a graphic with no points"))?;
    Ok(points
        .iter()
        .fold(Rect::point(first.0, first.1), |r, &(x, y)| {
            r.union(Rect::point(x, y))
        })
        .grow(half))
}

/// A visible silkscreen text's box: every glyph as wide as KiCad 10.0.7's
/// widest, `m`, whose advance is 4/3 of the font width (measured by Gerber
/// plot in the tools image, `AET-WO-1301` adversarial pass F3), plus the
/// stroke all round.
fn text_box(item: &Sx, text: &str, reference: &str) -> Result<Rect, EmitError> {
    let at = item
        .child("at")
        .ok_or_else(|| internal(reference, "a text without `at`"))?;
    let (x, y) = xy(at, reference)?;
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
    match at.arg(3).unwrap_or("0") {
        "0" | "180" => {}
        "90" | "270" => std::mem::swap(&mut hw, &mut hh),
        a => return Err(internal(reference, format!("a text at angle {a}"))),
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
    let mut courtyard: Option<Rect> = None;
    let mut silk: Option<Rect> = None;
    let join = |acc: &mut Option<Rect>, r: Rect| *acc = Some(acc.map_or(r, |a| a.union(r)));
    for item in fp.items() {
        let hidden = item.child("hide").and_then(|h| h.arg(1)) == Some("yes");
        match (item.head(), layer(item)) {
            (Some("fp_line" | "fp_rect" | "fp_poly" | "fp_arc" | "fp_circle"), Some("F.CrtYd")) => {
                join(&mut courtyard, graphic_box(item, reference)?);
            }
            (Some("fp_line" | "fp_rect" | "fp_poly" | "fp_arc" | "fp_circle"), Some("F.SilkS")) => {
                join(&mut silk, graphic_box(item, reference)?);
            }
            (Some("property"), Some("F.SilkS")) if !hidden => {
                let text = if item.arg(1) == Some("Reference") {
                    reference
                } else {
                    item.arg(2).unwrap_or("")
                };
                join(&mut silk, text_box(item, text, reference)?);
            }
            (Some("fp_text"), Some("F.SilkS")) if !hidden => {
                join(
                    &mut silk,
                    text_box(item, item.arg(2).unwrap_or(""), reference)?,
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
    /// Board coordinates.
    pub courtyard: Rect,
    pub keep: Rect,
}

struct Item<'a> {
    part: &'a Part,
    fp: Sx,
    geometry: Geometry,
    /// By pad number, the pads that carry a net: (position, net).
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

fn is_ic(part: &Part) -> bool {
    !part.lib_id.starts_with("Device:")
}

/// The integer placer (`AET-WO-1301` §1.2), a function of the parts alone:
/// 1. the MCU's origin goes at [`ORIGIN`];
/// 2. every other IC, in reference order, goes beside the placed IC it
///    shares most nets with;
/// 3. every passive, in reference order, goes beside the IC it shares most
///    nets with, or a decoupling capacitor beside the IC it decouples.
///
/// Ground and unconnected nets are not counted, and ties go to the lower
/// reference. "Beside" means: the first free spot, in rings of [`STEP`] out
/// from the target, where the part's keep-out box clears every placed one
/// by [`GAP_NM`]. The target is the anchor's lowest-numbered pad on the
/// first shared net (label nets before rails), so a decoupling capacitor
/// sits by the power pin its rail names; with no shared net, the anchor's
/// courtyard centre.
fn place(items: &[Item<'_>]) -> Result<Vec<Placed>, EmitError> {
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |i: usize| {
            let p = items[i].part;
            (
                !p.lib_id.starts_with("MCU_"),
                !is_ic(p),
                p.reference.clone(),
            )
        };
        key(a).cmp(&key(b))
    });
    let mut placed: Vec<Option<Placed>> = vec![None; items.len()];
    for (step, &i) in order.iter().enumerate() {
        let item = &items[i];
        let g = item.geometry;
        if step == 0 {
            if !item.part.lib_id.starts_with("MCU_") {
                return Err(EmitError::Internal("the design has no MCU".into()));
            }
            placed[i] = Some(Placed {
                reference: item.part.reference.clone(),
                origin: ORIGIN,
                courtyard: g.courtyard.shift(ORIGIN),
                keep: g.keep.shift(ORIGIN),
            });
            continue;
        }
        // The anchor: an IC already placed.
        let candidates =
            (0..items.len()).filter(|&j| j != i && placed[j].is_some() && is_ic(items[j].part));
        let anchor = match &item.part.decouples {
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
                    count(a)
                        .cmp(&count(b))
                        .then_with(|| items[b].part.reference.cmp(&items[a].part.reference))
                })
                .ok_or_else(|| internal(&item.part.reference, "no IC to place beside"))?,
        };
        let at = placed[anchor].as_ref().map(|p| p.origin).unwrap_or(ORIGIN);
        let target = item
            .shared(&items[anchor])
            .first()
            .and_then(|net| items[anchor].pad_on(net))
            .map(|(x, y)| (at.0 + x, at.1 + y))
            .unwrap_or_else(|| {
                placed[anchor]
                    .as_ref()
                    .map_or(ORIGIN, |p| p.courtyard.centre())
            });
        let taken: Vec<(Rect, i64)> = (0..items.len())
            .filter_map(|j| {
                let both = is_ic(items[j].part) && is_ic(item.part);
                placed[j]
                    .as_ref()
                    .map(|p| (p.keep, if both { IC_GAP_NM } else { GAP_NM }))
            })
            .collect();
        let origin = spot(g, target, &taken)
            .ok_or_else(|| internal(&item.part.reference, "no free spot within a metre"))?;
        placed[i] = Some(Placed {
            reference: item.part.reference.clone(),
            origin,
            courtyard: g.courtyard.shift(origin),
            keep: g.keep.shift(origin),
        });
    }
    placed
        .into_iter()
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| EmitError::Internal("a part was not placed".into()))
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
fn footprint(project: &str, item: &Item<'_>, origin: (i64, i64)) -> Result<Sx, EmitError> {
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
        at_sx("at", origin),
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
            Some("property") => {
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

/// A reference as KiCad compares references: ASCII case folded, and the
/// trailing number without its leading zeros (`U01` is `U1`). Two parts with
/// one key are a duplicate to KiCad (adversarial pass F4).
fn reference_key(reference: &str) -> String {
    let upper = reference.to_ascii_uppercase();
    let stem = upper.trim_end_matches(|c: char| c.is_ascii_digit());
    let digits = upper[stem.len()..].trim_start_matches('0');
    match (upper.len() > stem.len(), digits.is_empty()) {
        (false, _) => upper,
        (true, true) => format!("{stem}0"),
        (true, false) => format!("{stem}{digits}"),
    }
}

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
        let fp = library::footprint(&part.footprint)?;
        let nets = pin_nets(part)?;
        let mut pads = BTreeMap::new();
        let mut numbers = BTreeSet::new();
        for pad in fp.children("pad") {
            let number = pad.arg(1).unwrap_or("");
            numbers.insert(number.to_owned());
            if let Some(net) = nets.get(number) {
                let at = pad
                    .child("at")
                    .ok_or_else(|| internal(&part.reference, "a pad without `at`"))?;
                pads.insert(number.to_owned(), (xy(at, &part.reference)?, net.clone()));
            }
        }
        if let Some(missing) = nets.keys().find(|n| !numbers.contains(*n)) {
            return Err(internal(
                &part.reference,
                format!("pin {missing} has no pad in {}", part.footprint),
            ));
        }
        out.push(Item {
            geometry: geometry(&fp, &part.reference)?,
            part,
            fp,
            pads,
        });
    }
    Ok(out)
}

/// The placed board: what `write` emits, for tests and the outline.
pub struct Layout {
    pub placed: Vec<Placed>,
    pub outline: Rect,
    /// `(reference, pad number) → net` for every pad that carries one.
    pub pad_nets: BTreeMap<(String, String), String>,
}

fn layout_of(items: &[Item<'_>]) -> Result<Layout, EmitError> {
    let placed = place(items)?;
    let first = placed
        .first()
        .ok_or_else(|| EmitError::Internal("no parts".into()))?
        .keep;
    let all = placed
        .iter()
        .fold(first, |r, p| r.union(p.keep))
        .grow(MARGIN_NM);
    let outline = Rect {
        x0: all.x0.div_euclid(SNAP) * SNAP,
        y0: all.y0.div_euclid(SNAP) * SNAP,
        x1: -((-all.x1).div_euclid(SNAP) * SNAP),
        y1: -((-all.y1).div_euclid(SNAP) * SNAP),
    };
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
    layout_of(&items(parts)?)
}

/// The `.kicad_pcb` text for `parts`.
pub(crate) fn write(project: &str, parts: &[Part]) -> Result<String, EmitError> {
    let items = items(parts)?;
    let layout = layout_of(&items)?;
    let mut layers = vec![Sx::atom("layers")];
    for (id, name, kind, user) in LAYERS {
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
            Sx::node("thickness", &["1.6"]),
            Sx::node("legacy_teardrops", &["no"]),
        ]),
        Sx::List(vec![Sx::atom("paper"), Sx::str("A4")]),
        Sx::List(layers),
    ];
    let mut order: Vec<usize> = (0..items.len()).collect();
    order.sort_by(|&a, &b| items[a].part.reference.cmp(&items[b].part.reference));
    // `place` returns each item's placement at the item's index.
    let mut parts = Vec::new();
    for i in order {
        let fp = footprint(project, &items[i], layout.placed[i].origin)?;
        parts.push(route::part_of(&fp).map_err(EmitError::Internal)?);
        out.push(fp);
    }
    let o = layout.outline;
    let board = route::Board { outline: o, parts };
    let routed = route::route(&board).map_err(|e| match e.net.is_empty() {
        true => EmitError::BoardUnroutable(e.why),
        false => EmitError::Unroutable {
            net: e.net,
            why: e.why,
        },
    })?;
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
    out.extend(copper(project, &routed, o));
    out.push(Sx::node("embedded_fonts", &["no"]));
    Ok(sexpr::write(&Sx::List(out)))
}

/// The router's tracks, vias and unfilled zones as board items, each
/// keyed by its content. A zone's outline is the board's; it carries no
/// `connect_pads` clearance, which the gate refuses (`AET-WO-1300` B2), and
/// no fill, which the gate's `--refill-zones` computes.
fn copper(project: &str, routed: &route::Routed, o: Rect) -> Vec<Sx> {
    let net = |n: &str| Sx::List(vec![Sx::atom("net"), Sx::str(n)]);
    let layer = |l: route::Layer| Sx::List(vec![Sx::atom("layer"), Sx::str(l.name())]);
    let mut out = Vec::new();
    for s in &routed.segments {
        out.push(Sx::List(vec![
            Sx::atom("segment"),
            at_sx("start", s.start),
            at_sx("end", s.end),
            Sx::node("width", &[&mm(route::WIDTH_NM)]),
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
            Sx::node("size", &[&mm(route::VIA_NM)]),
            Sx::node("drill", &[&mm(route::DRILL_NM)]),
            Sx::List(vec![Sx::atom("layers"), Sx::str("F.Cu"), Sx::str("B.Cu")]),
            net(&v.net),
            uuid_sx(&format!("{project}/via/{}/{} {}", v.net, v.at.0, v.at.1)),
        ]));
    }
    for (n, l) in &routed.zones {
        let corner = |x, y| Sx::node("xy", &[&mm(x), &mm(y)]);
        out.push(Sx::List(vec![
            Sx::atom("zone"),
            net(n),
            layer(*l),
            uuid_sx(&format!("{project}/zone/{n}/{}", l.name())),
            Sx::node("hatch", &["edge", "0.5"]),
            Sx::node("min_thickness", &[&mm(route::MIN_THICKNESS_NM)]),
            Sx::node("filled_areas_thickness", &["no"]),
            Sx::List(vec![
                Sx::atom("fill"),
                Sx::node("thermal_gap", &["0.5"]),
                Sx::node("thermal_bridge_width", &["0.5"]),
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
    use super::*;

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
            decouples: None,
        }
    }

    const MCU: &str = "MCU_ST_STM32H7:STM32H753ZITx";
    const LQFP: &str = "Package_QFP:LQFP-144_20x20mm_P0.5mm";
    const R0402: &str = "Resistor_SMD:R_0402_1005Metric";

    fn net(n: &str) -> Conn {
        Conn::Net(n.to_owned())
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
        let forward = write("p", &parts).unwrap();
        let mut reversed = parts.clone();
        reversed.reverse();
        assert_eq!(forward, write("p", &reversed).unwrap());
        assert!(forward.starts_with("(kicad_pcb\n\t(version 20260206)"));
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
        cap.decouples = Some("U2".into());
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
                "(fp_circle (center 0 0) (end 1 0) (layer \"F.CrtYd\"))",
                "does not measure",
            ),
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
        let board = write("p", &[mcu(&[("34", net("SDA"))]), bme("U2"), bme("U3")]).unwrap();
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
        ] {
            let g = geometry(&library::footprint(id).unwrap(), "R1").unwrap();
            assert!(g.keep.contains(g.courtyard), "{id}");
            assert!(
                g.courtyard.x0 < g.courtyard.x1 && g.courtyard.y0 < g.courtyard.y1,
                "{id}"
            );
        }
    }
}
