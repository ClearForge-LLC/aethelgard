//! Reading a board (`route::read` and what it needs): the strict reader's
//! tree turned into the router's [`Board`] and [`Routed`], refusing what the
//! router does not model. Split out of `route.rs` (roadmap §6, split on
//! touch; `AET-WO-1311`).

use std::collections::BTreeSet;

use super::*;

/// The most items [`read`] takes (`AET-WO-1311` §1.2): copper items (pads,
/// tracks and vias), and since `AET-WO-1322` (`1311` adversarial F) every
/// courtyard edge and footprint keepout, which `check` also scans for every
/// via or item. `check` compares every pair of copper items and
/// `connectivity` every track with every item, so both cost the square of
/// the count; measured (release, Furnace, 2026-11-16): 0.9 s together at
/// 20 000 copper items, 5.8 s at 30 000. The largest board the fixtures
/// emit holds 3 351 copper items, 800 courtyard edges and no keepout
/// (`d3-24-i2c-12-spi`, measured 2026-11-23 by `spike/1322/count.py`).
pub const MAX_ITEMS: usize = 20_000;
use crate::sexpr::Sx;
use crate::strict::{self, Tree};

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
    // The reference and pad numbers as a refusal writes them: bounded and
    // escaped when long or unprintable (`AET-WO-1322`; `1311` adversarial J).
    let named = strict::shown(&reference).into_owned();
    let at = one(fp, "at")?.ok_or_else(|| format!("{named}: no `at`"))?;
    let theta = angle(at.arg(3))
        .ok_or_else(|| format!("{named} is rotated by {}", strict::echo(&at.arg(3))))?;
    let origin = xy_of(Some(at), &named)?;
    let mut pads = Vec::new();
    for pad in fp.all("pad") {
        let number = pad.arg(1).unwrap_or("").to_owned();
        let what = format!("{named} pad {}", strict::shown(&number));
        // A pad with no `layers`, or with a padstack of its own, puts copper
        // where KiCad says, not where this reads it (`AET-WO-1311`
        // adversarial E: KiCad flashed a pad with no `layers` on F.Cu and
        // B.Cu, and a padstack's own size over the pad's): refused.
        if one(&pad, "padstack")?.is_some() {
            return Err(format!(
                "{what} has its own `padstack`, which the gate does not model"
            ));
        }
        let layers: Vec<&str> = one(&pad, "layers")?
            .map(|l| l.texts().collect())
            .ok_or_else(|| format!("{what} names no layers"))?;
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
            other => return Err(format!("{what} has shape {}", strict::echo(&other))),
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
    // The front courtyard, as straight edges; an arc in it is not modelled
    // and is refused, never approximated. A whole circle (`AET-WO-1312`: the
    // test pad's) is taken as the square that holds it: the courtyard only
    // keeps vias out, and a via outside the square is outside the circle, so
    // the rule is never weakened, only kept wider.
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
        let what = format!("{named}'s courtyard {head}");
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
            "fp_circle" => {
                let c = xy_of(one(&g, "center")?, &what)?;
                let e = xy_of(one(&g, "end")?, &what)?;
                // A circle beyond a metre is refused before any arithmetic, so
                // nothing below can overflow (adversarial F5: it panicked).
                if [c.0, c.1, e.0, e.1].iter().any(|v| v.abs() > 1_000_000_000) {
                    return Err(format!("{what} is too large"));
                }
                let (dx, dy) = (
                    i128::from(e.0) - i128::from(c.0),
                    i128::from(e.1) - i128::from(c.1),
                );
                let r = i64::try_from(crate::pcb::isqrt_up(dx * dx + dy * dy))
                    .map_err(|_| format!("{what} is too large"))?;
                if r == 0 {
                    return Err(format!("{what} has no radius"));
                }
                let q = [
                    (c.0 - r, c.1 - r),
                    (c.0 + r, c.1 - r),
                    (c.0 + r, c.1 + r),
                    (c.0 - r, c.1 + r),
                ]
                .map(at);
                courtyard.extend((0..4).map(|i| (q[i], q[(i + 1) % 4])));
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
            return Err(format!("{named}'s courtyard has an edge twice"));
        }
    }
    if corners.values().any(|n| n % 2 == 1) {
        return Err(format!("{named}'s courtyard is not a closed outline"));
    }
    if courtyard.is_empty() && !pads.is_empty() {
        return Err(format!("{named} has copper pads and no courtyard"));
    }
    let mut keepouts = Vec::new();
    for z in fp.all("zone") {
        keepouts.push(keepout(&z, &named)?);
    }
    let part = Part {
        reference,
        rotation: theta,
        pads,
        courtyard,
        keepouts,
    };
    // A footprint's keepout is its own part's (`AET-WO-1323` D3, `1310`
    // A7): wholly inside its courtyard outline or its pad field, edges
    // included. One moved off its part would have `check` judge copper by a
    // zone that is not where the part is.
    for k in &part.keepouts {
        if !field(&part).is_some_and(|f| f.contains(k.rect)) && !held(&part.courtyard, k.rect) {
            return Err(format!(
                "{named}'s keepout on {} lies outside its own courtyard and pad field",
                k.layer.name()
            ));
        }
    }
    Ok(part)
}

/// Whether the box `r` lies wholly inside the closed outline `edges`, its
/// boundary included: no edge enters `r`'s open interior, and `r`'s centre
/// is inside (both at doubled coordinates, so the centre is exact).
fn held(edges: &[Edge], r: Rect) -> bool {
    let twice: Vec<Edge> = edges
        .iter()
        .map(|&(a, b)| ((2 * a.0, 2 * a.1), (2 * b.0, 2 * b.1)))
        .collect();
    !edges.iter().any(|&e| enters(e, r)) && under(&twice, (r.x0 + r.x1, r.y0 + r.y1))
}

/// Whether the segment `(a, b)` passes through the open interior of `r`: no
/// separating axis among x, y and the segment's normal (a segment along
/// `r`'s side, or touching only a corner, does not).
fn enters((a, b): Edge, r: Rect) -> bool {
    if a.0.max(b.0) <= r.x0 || a.0.min(b.0) >= r.x1 || a.1.max(b.1) <= r.y0 || a.1.min(b.1) >= r.y1
    {
        return false;
    }
    let (dx, dy) = (i128::from(b.0 - a.0), i128::from(b.1 - a.1));
    let side = |(x, y): (i64, i64)| dx * i128::from(y - a.1) - dy * i128::from(x - a.0);
    let c = [(r.x0, r.y0), (r.x1, r.y0), (r.x1, r.y1), (r.x0, r.y1)].map(side);
    c.iter().any(|&v| v < 0) && c.iter().any(|&v| v > 0)
}

/// A footprint zone, which only a keepout may be: a rectangle on one copper
/// layer, turned with the footprint, each of its five kinds `allowed` or
/// `not_allowed`, no net. Anything else is refused by name, never dropped
/// (`RUL-03`, amended for `AET-WO-4300d2` D2).
/// `named` is the part's reference as a refusal writes it.
fn keepout(z: &strict::Node<'_>, named: &str) -> Result<Keepout, String> {
    let what = format!("{named}'s zone");
    let k = one(z, "keepout")?.ok_or_else(|| format!("{what} is not a keepout"))?;
    for token in ["net", "net_name"] {
        if one(z, token)?.is_some() {
            return Err(format!("{what} is a keepout with a `{token}`"));
        }
    }
    if one(z, "layers")?.is_some() {
        return Err(format!("{what} names more than one layer"));
    }
    let layer = match one(z, "layer")?.and_then(|l| l.arg(1)) {
        Some("F.Cu") => Layer::F,
        Some("B.Cu") => Layer::B,
        other => {
            return Err(format!(
                "{what} is on {}, not F.Cu or B.Cu",
                strict::echo(&other)
            ));
        }
    };
    let flag = |name: &str| -> Result<bool, String> {
        match one(&k, name)?.map(|n| (n.arg(1), n.arg(2))) {
            Some((Some("not_allowed"), None)) => Ok(true),
            Some((Some("allowed"), None)) => Ok(false),
            other => Err(format!(
                "{what}'s keepout `{name}` is {}",
                strict::echo(&other)
            )),
        }
    };
    let (tracks, vias, pads, fill) = (
        flag("tracks")?,
        flag("vias")?,
        flag("pads")?,
        flag("copperpour")?,
    );
    flag("footprints")?;
    let mut polys = z.all("polygon");
    let (Some(poly), None) = (polys.next(), polys.next()) else {
        return Err(format!("{what} has no polygon, or more than one"));
    };
    let pts = one(&poly, "pts")?.ok_or_else(|| format!("{what}: no `pts`"))?;
    let corners = pts
        .lists()
        // In a board file a footprint's zone stands in board coordinates
        // (`AET-WO-1310` D4, measured: KiCad reads it so).
        .map(|xy| match xy.head() {
            Some("xy") => xy_of(Some(xy), &what),
            h => Err(format!("{what}: a `{}` in its points", h.unwrap_or("()"))),
        })
        .collect::<Result<Vec<_>, _>>()?;
    let rect = corners
        .iter()
        .map(|&(x, y)| Rect::point(x, y))
        .reduce(Rect::union)
        .ok_or_else(|| format!("{what} has no points"))?;
    let on_box =
        |&(x, y): &(i64, i64)| (x == rect.x0 || x == rect.x1) && (y == rect.y0 || y == rect.y1);
    if corners.len() != 4 || !corners.iter().all(on_box) || rect.x0 == rect.x1 || rect.y0 == rect.y1
    {
        return Err(format!("{what} is not a rectangle at a right angle"));
    }
    Ok(Keepout {
        layer,
        rect,
        tracks,
        vias,
        pads,
        fill,
    })
}

/// A plated through-hole pad's drill (one round hole at the pad's centre,
/// with every layer's copper kept) and its `zone_connect`: 2 solid, 1
/// relief, or none. Any token by which the pad would set its own relief or
/// unused-layer removal is refused (a padstack, on any pad, by [`part`]).
fn through_hole(
    pad: &strict::Node<'_>,
    what: &str,
) -> Result<(Option<i64>, Option<crate::stack::Join>), String> {
    let join = match one(pad, "zone_connect")?.map(|z| (z.arg(1), z.arg(2))) {
        None => None,
        Some((Some(t), None)) if let Some(j) = crate::stack::Join::of_token(t) => Some(j),
        Some(other) => {
            return Err(format!(
                "{what} has `zone_connect` {}",
                strict::echo(&other)
            ));
        }
    };
    for token in [
        "thermal_gap",
        "thermal_width",
        "thermal_bridge_width",
        "thermal_bridge_angle",
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
/// named: `LayersDisagree`, `PlanesDisagree`, `ThicknessDisagrees`,
/// `ViaNotThrough` or `ImuOrientationDisagrees`.
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
        // KiCad takes a row of a copper type as copper whatever its id and
        // name (`AET-WO-1311` adversarial B: `(37 "Top" signal)` plotted as
        // In1.Cu, measured).
        let typed = matches!(row.arg(2), Some("signal" | "power" | "mixed" | "jumper"));
        let is_copper = even || typed || strict::copper_layer(name);
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
                "the board's copper layers are {}; the specification declares {want:?}",
                strict::echo(&copper)
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
                    "a via typed {} on {}; the specification declares through vias, F.Cu to B.Cu",
                    strict::echo(&v.arg(1)),
                    strict::echo(&layers)
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
    // `AET-WO-4300k` (a): the thermal pours are declared zones too.
    let pours: Vec<(String, String)> = stack
        .pours
        .iter()
        .map(|(l, n)| (n.clone(), l.clone()))
        .collect();
    let (zones, extra): (Vec<_>, Vec<_>) = zones.into_iter().partition(|z| !pours.contains(z));
    let mut found = extra.clone();
    found.sort();
    let mut want = pours.clone();
    want.sort();
    let ok = found == want
        && if stack.has_planes() {
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
                "the board's zones are {}; the specification declares {planes:?} (and ground's B.Cu zone at 2 layers)",
                strict::echo(&zones)
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
    for (reference, seat) in &stack.imus {
        imu_agrees(&board, reference, seat)
            .map_err(|m| no("ImuOrientationDisagrees", format!("{reference}: {m}")))?;
    }
    Ok(())
}

/// `AET-WO-4400d` §5: a declared IMU's footprint holds the part's cited axes
/// where the specification declares them, read from the board alone: the
/// footprint on F.Cu, its pads where [`part`] reads them, each axis from the
/// pads' mean to the mean of the pins the parts table names for it. The
/// expected directions are worked from the declaration here, in the board
/// frame (x forward, y right, z down through F.Cu, ruled D3), Y = Z × X,
/// each drawn as `a·forward + b·right` on the board seen from F.Cu, where
/// right is forward turned clockwise; nothing is taken from the placer. A
/// declaration with Z not up, or X up or down, draws an X or a Y no footprint
/// on F.Cu has, so it disagrees with every board.
fn imu_agrees(
    board: &strict::Node<'_>,
    reference: &str,
    seat: &crate::schematic::Seat,
) -> Result<(), String> {
    use aethelgard_ir::orientation::{Edge, Toward};
    let mut found = Vec::new();
    for f in board.all("footprint") {
        if f.field("Reference").map_err(|e| e.to_string())? == Some(reference) {
            found.push(f);
        }
    }
    let fp = match found.as_slice() {
        [fp] => fp.clone(),
        [] => return Err("the board has no footprint of that reference".into()),
        _ => {
            return Err(format!(
                "the board has {} footprints of that reference",
                found.len()
            ));
        }
    };
    let layer = one(&fp, "layer")?.and_then(|l| l.arg(1).map(str::to_owned));
    if layer.as_deref() != Some("F.Cu") {
        return Err(format!(
            "its footprint is on {}; a part on the back is mirrored, and the declaration is for F.Cu",
            strict::echo(&layer)
        ));
    }
    let placed = part(&fp)?;
    let centre = |pads: &[&Rect]| -> (i64, i64) {
        let n = i64::try_from(pads.len().max(1)).unwrap_or(1);
        let (x, y) = pads.iter().fold((0, 0), |s, r| {
            (s.0 + (r.x0 + r.x1) / 2, s.1 + (r.y0 + r.y1) / 2)
        });
        (x / n, y / n)
    };
    let all: Vec<&Rect> = placed.pads.iter().map(|p| &p.rect).collect();
    let c = centre(&all);
    let axis = |pins: &[String]| -> Result<(i64, i64), String> {
        let named: Vec<&Rect> = placed
            .pads
            .iter()
            .filter(|p| pins.contains(&p.number))
            .map(|p| &p.rect)
            .collect();
        if named.len() != pins.len() {
            return Err(format!("its footprint lacks a pad of {}", pins.join(", ")));
        }
        let (x, y) = centre(&named);
        Ok(((x - c.0).signum(), (y - c.1).signum()))
    };
    let board_x = axis(&seat.x_toward_pins)?;
    let board_y = axis(&seat.y_toward_pins)?;
    let body = |d: Toward| -> [i64; 3] {
        match d {
            Toward::Forward => [1, 0, 0],
            Toward::Aft => [-1, 0, 0],
            Toward::Right => [0, 1, 0],
            Toward::Left => [0, -1, 0],
            Toward::Down => [0, 0, 1],
            Toward::Up => [0, 0, -1],
        }
    };
    let (x, z) = (body(seat.orientation.x), body(seat.orientation.z));
    let y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    let f: (i64, i64) = match seat.forward {
        Edge::Top => (0, -1),
        Edge::Right => (1, 0),
        Edge::Bottom => (0, 1),
        Edge::Left => (-1, 0),
    };
    let r = (-f.1, f.0);
    let draw = |v: [i64; 3]| (v[0] * f.0 + v[1] * r.0, v[0] * f.1 + v[1] * r.1);
    let (want_x, want_y) = (draw(x), draw(y));
    if (board_x, board_y) != (want_x, want_y) {
        return Err(format!(
            "its footprint puts the chip's X toward {board_x:?} and Y toward {board_y:?} on the board (y down); the specification declares X {} and Z {} with forward the {:?} edge: X toward {want_x:?}, Y toward {want_y:?}",
            seat.orientation.x.name(),
            seat.orientation.z.name(),
            seat.forward
        ));
    }
    Ok(())
}

fn layer_of(item: &strict::Node<'_>, what: &str) -> Result<Layer, String> {
    match one(item, "layer")?.and_then(|l| l.arg(1)) {
        Some("F.Cu") => Ok(Layer::F),
        Some("B.Cu") => Ok(Layer::B),
        other => Err(format!("{what} on layer {}", strict::echo(&other))),
    }
}

/// A board as the strict reader read it: its outline and pads, and its
/// copper, judged against the specification's declared board `stack`.
/// Tracks must be the declared width and on F.Cu or B.Cu; vias the engine's
/// through via; the outline the one `Edge.Cuts` rectangle. Whether the layer
/// table, the zones and the thickness are exactly the declared ones is
/// [`declared`]'s, which every caller but `check support` runs first (so a
/// board with no layer table is refused here too), and [`connectivity`]
/// judges the zones again on the copper read here.
pub fn read(tree: &Tree, stack: &crate::stack::Stack) -> Result<(Board, Routed), String> {
    let board = tree.root();
    // The strict reader judges every item's layer against the table
    // (`strict::board`); a board with none is not judged there, so it is
    // refused here, whichever gate reads it (`AET-WO-1311` adversarial D:
    // `check support` reads a board without `declared`).
    if one(&board, "layers")?.is_none() {
        return Err("the board has no layer table".to_owned());
    }
    let items = board.all("segment").count()
        + board.all("via").count()
        + board
            .all("footprint")
            .map(|fp| fp.all("pad").count())
            .sum::<usize>();
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
    let rest = parts
        .iter()
        .map(|p| p.courtyard.len() + p.keepouts.len())
        .sum::<usize>();
    if items + rest > MAX_ITEMS {
        return Err(format!(
            "the board holds {items} copper items (pads, tracks and vias) and {rest} courtyard edges and keepouts, which `check` also scans: {} items; the judges read at most {MAX_ITEMS}",
            items + rest
        ));
    }
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
                return Err(format!(
                    "a thermal via at {at:?} on {}",
                    strict::echo(&layers)
                ));
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
                format!("a via {size:?}/{drill:?} on {}", strict::echo(&layers))
            } else {
                format!(
                    "a via {size:?}/{drill:?} on {}{class} {}/{} nm",
                    strict::echo(&layers),
                    rule.via_nm,
                    rule.drill_nm
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
        // As a refusal writes it (`AET-WO-1322`).
        let zone = format!(
            "zone `{}` on {}",
            strict::shown(&name),
            strict::shown(&layer)
        );
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
                    "{zone} has `{head}`, which the gate does not model"
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
                        "{zone}: its fill has `{head}`, which the gate does not model"
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
                "{zone}: its fill is not the engine's solid fill (min_thickness {}, island_removal_mode 0)",
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
                "{zone}: its relief gap and spoke are not the engine's {THERMAL_GAP_NM}/{THERMAL_BRIDGE_NM} nm"
            ));
        }
        if !zone_is_outline(&z, outline)? {
            return Err(format!("{zone}: its outline is not the board's"));
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
