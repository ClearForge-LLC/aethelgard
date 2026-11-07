//! The spike's board emitter and a proof-of-concept grid router (S3, S4).
//! Footprints are copied from the library with their pads given nets and
//! every UUID inside re-derived from content, so two emissions are identical.

use std::cmp::Reverse;
use std::collections::{BTreeMap, BTreeSet, BinaryHeap};

use super::sexpr::{self, Sx, mm, nm};
use super::uuid;

#[derive(Clone, Debug)]
pub struct Fp {
    pub lib_id: String,
    pub reference: String,
    pub value: String,
    pub at: (i64, i64),
    /// `/<symbol uuid>`, the schematic link parity uses.
    pub path: Option<String>,
    pub sheetfile: String,
    pub pad_nets: BTreeMap<String, String>,
    /// The library footprint as read.
    pub src: Sx,
    /// A local clearance on every pad (S3's footprint-rule row).
    pub pad_clearance: Option<i64>,
}

#[derive(Clone, Debug)]
pub struct Seg {
    pub a: (i64, i64),
    pub b: (i64, i64),
    pub width: i64,
    pub layer: String,
    pub net: String,
}

#[derive(Clone, Debug)]
pub struct Via {
    pub at: (i64, i64),
    pub size: i64,
    pub drill: i64,
    pub net: String,
}

#[derive(Clone, Debug)]
pub struct Zone {
    pub net: String,
    pub layer: String,
    pub pts: Vec<(i64, i64)>,
}

#[derive(Clone, Debug)]
pub struct Board {
    pub key: String,
    pub version: String,
    pub setup: String,
    pub outline: (i64, i64, i64, i64),
    pub footprints: Vec<Fp>,
    pub segs: Vec<Seg>,
    pub vias: Vec<Via>,
    pub zones: Vec<Zone>,
}

const LAYERS: &str = r#"(layers (0 "F.Cu" signal) (31 "B.Cu" signal) (32 "B.Adhes" user "B.Adhesive") (33 "F.Adhes" user "F.Adhesive") (34 "B.Paste" user) (35 "F.Paste" user) (36 "B.SilkS" user "B.Silkscreen") (37 "F.SilkS" user "F.Silkscreen") (38 "B.Mask" user) (39 "F.Mask" user) (40 "Dwgs.User" user "User.Drawings") (41 "Cmts.User" user "User.Comments") (42 "Eco1.User" user "User.Eco1") (43 "Eco2.User" user "User.Eco2") (44 "Edge.Cuts" user) (45 "Margin" user) (46 "B.CrtYd" user "B.Courtyard") (47 "F.CrtYd" user "F.Courtyard") (48 "B.Fab" user) (49 "F.Fab" user))"#;

fn xy(head: &str, p: (i64, i64)) -> Sx {
    Sx::node(head, &[&mm(p.0), &mm(p.1)])
}

fn uuid_sx(key: &str) -> Sx {
    Sx::list(vec![Sx::atom("uuid"), Sx::str(uuid(key))])
}

fn net_sx(nets: &BTreeMap<String, usize>, name: &str) -> Sx {
    Sx::node("net", &[&nets[name].to_string()])
}

/// Every `(uuid ..)` or `(tstamp ..)` under `sx`, re-derived from `key` and
/// its order of appearance.
fn rekey(sx: &mut Sx, key: &str, n: &mut usize) {
    if matches!(sx.head(), Some("uuid" | "tstamp")) {
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

impl Board {
    pub fn net_names(&self) -> BTreeSet<String> {
        let mut names = BTreeSet::new();
        for f in &self.footprints {
            names.extend(f.pad_nets.values().cloned());
        }
        names.extend(self.segs.iter().map(|s| s.net.clone()));
        names.extend(self.vias.iter().map(|v| v.net.clone()));
        names.extend(self.zones.iter().map(|z| z.net.clone()));
        names
    }

    fn footprint(&self, f: &Fp, nets: &BTreeMap<String, usize>) -> Result<Sx, String> {
        let name = f.src.arg(1).ok_or("footprint without a name")?;
        let key = format!("{}/fp/{}", self.key, f.reference);
        let mut v = vec![
            Sx::atom("footprint"),
            Sx::str(f.lib_id.clone()),
            Sx::list(vec![Sx::atom("layer"), Sx::str("F.Cu")]),
            uuid_sx(&key),
            xy("at", f.at),
        ];
        if let Some(p) = &f.path {
            v.push(Sx::list(vec![Sx::atom("path"), Sx::str(p.clone())]));
            v.push(Sx::list(vec![Sx::atom("sheetname"), Sx::str("/")]));
            v.push(Sx::list(vec![
                Sx::atom("sheetfile"),
                Sx::str(f.sheetfile.clone()),
            ]));
        }
        let mut count = 0;
        for child in &f.src.items()[2..] {
            let head = child.head().unwrap_or("");
            if matches!(
                head,
                "version" | "generator" | "generator_version" | "layer"
            ) {
                continue;
            }
            let mut c = child.clone();
            if head == "property"
                && let Some(items) = c.items_mut()
            {
                match items.get(1).and_then(Sx::text) {
                    Some("Reference") => items[2] = Sx::str(f.reference.clone()),
                    Some("Value") => items[2] = Sx::str(f.value.clone()),
                    _ => {}
                }
            }
            if head == "pad" {
                let num = c.arg(1).unwrap_or("").to_owned();
                if let Some(items) = c.items_mut() {
                    if let Some(net) = f.pad_nets.get(&num) {
                        items.push(Sx::list(vec![
                            Sx::atom("net"),
                            Sx::atom(nets[net].to_string()),
                            Sx::str(net.clone()),
                        ]));
                    }
                    if let Some(cl) = f.pad_clearance {
                        items.push(Sx::node("clearance", &[&mm(cl)]));
                    }
                }
            }
            rekey(&mut c, &key, &mut count);
            v.push(c);
        }
        let _ = name;
        Ok(Sx::list(v))
    }

    pub fn emit(&self) -> Result<String, String> {
        let names = self.net_names();
        let nets: BTreeMap<String, usize> = names
            .iter()
            .enumerate()
            .map(|(i, n)| (n.clone(), i + 1))
            .collect();
        let mut top = vec![
            Sx::atom("kicad_pcb"),
            Sx::node("version", &[&self.version]),
            Sx::list(vec![Sx::atom("generator"), Sx::str("aethelgard")]),
            Sx::list(vec![Sx::atom("generator_version"), Sx::str("0.1")]),
            sexpr::parse("(general (thickness 1.6) (legacy_teardrops no))")?,
            Sx::list(vec![Sx::atom("paper"), Sx::str("A4")]),
            sexpr::parse(LAYERS)?,
            sexpr::parse(&self.setup)?,
            Sx::list(vec![Sx::atom("net"), Sx::atom("0"), Sx::str("")]),
        ];
        for (n, i) in &nets {
            top.push(Sx::list(vec![
                Sx::atom("net"),
                Sx::atom(i.to_string()),
                Sx::str(n.clone()),
            ]));
        }
        let mut fps: Vec<&Fp> = self.footprints.iter().collect();
        fps.sort_by(|a, b| a.reference.cmp(&b.reference));
        for f in fps {
            top.push(self.footprint(f, &nets)?);
        }
        let (x0, y0, x1, y1) = self.outline;
        top.push(Sx::list(vec![
            Sx::atom("gr_rect"),
            xy("start", (x0, y0)),
            xy("end", (x1, y1)),
            sexpr::parse("(stroke (width 0.1) (type default))")?,
            Sx::node("fill", &["none"]),
            Sx::list(vec![Sx::atom("layer"), Sx::str("Edge.Cuts")]),
            uuid_sx(&format!("{}/outline", self.key)),
        ]));
        for (i, s) in self.segs.iter().enumerate() {
            top.push(Sx::list(vec![
                Sx::atom("segment"),
                xy("start", s.a),
                xy("end", s.b),
                Sx::node("width", &[&mm(s.width)]),
                Sx::list(vec![Sx::atom("layer"), Sx::str(s.layer.clone())]),
                net_sx(&nets, &s.net),
                uuid_sx(&format!("{}/seg/{i}", self.key)),
            ]));
        }
        for (i, v) in self.vias.iter().enumerate() {
            top.push(Sx::list(vec![
                Sx::atom("via"),
                xy("at", v.at),
                Sx::node("size", &[&mm(v.size)]),
                Sx::node("drill", &[&mm(v.drill)]),
                Sx::list(vec![Sx::atom("layers"), Sx::str("F.Cu"), Sx::str("B.Cu")]),
                net_sx(&nets, &v.net),
                uuid_sx(&format!("{}/via/{i}", self.key)),
            ]));
        }
        for (i, z) in self.zones.iter().enumerate() {
            let mut pts = vec![Sx::atom("pts")];
            pts.extend(z.pts.iter().map(|p| xy("xy", *p)));
            top.push(Sx::list(vec![
                Sx::atom("zone"),
                net_sx(&nets, &z.net),
                Sx::list(vec![Sx::atom("net_name"), Sx::str(z.net.clone())]),
                Sx::list(vec![Sx::atom("layer"), Sx::str(z.layer.clone())]),
                uuid_sx(&format!("{}/zone/{i}", self.key)),
                sexpr::parse("(hatch edge 0.5)")?,
                sexpr::parse("(connect_pads (clearance 0.2))")?,
                sexpr::parse("(min_thickness 0.25)")?,
                sexpr::parse("(filled_areas_thickness no)")?,
                sexpr::parse("(fill (thermal_gap 0.5) (thermal_bridge_width 0.5))")?,
                Sx::list(vec![Sx::atom("polygon"), Sx::list(pts)]),
            ]));
        }
        Ok(sexpr::write(&Sx::list(top)))
    }

    /// Every pad as copper: (net, layer, centre, half-size), board coordinates.
    pub fn pads(&self) -> Result<Vec<PadCu>, String> {
        let mut out = Vec::new();
        for f in &self.footprints {
            for pad in f.src.children("pad") {
                let num = pad.arg(1).unwrap_or("").to_owned();
                let at = pad.child("at").ok_or("pad without at")?;
                let size = pad.child("size").ok_or("pad without size")?;
                let get = |s: &Sx, i| s.arg(i).ok_or("short").map_err(str::to_owned).and_then(nm);
                let layers: Vec<&str> = pad
                    .child("layers")
                    .map(|l| l.items()[1..].iter().filter_map(Sx::text).collect())
                    .unwrap_or_default();
                for layer in ["F.Cu", "B.Cu"] {
                    if layers.iter().any(|l| *l == layer || *l == "*.Cu") {
                        out.push(PadCu {
                            fp: f.reference.clone(),
                            number: num.clone(),
                            net: f.pad_nets.get(&num).cloned(),
                            layer,
                            centre: (f.at.0 + get(at, 1)?, f.at.1 + get(at, 2)?),
                            half: (get(size, 1)? / 2, get(size, 2)? / 2),
                        });
                    }
                }
            }
        }
        Ok(out)
    }
}

#[derive(Clone, Debug)]
pub struct PadCu {
    pub fp: String,
    pub number: String,
    pub net: Option<String>,
    pub layer: &'static str,
    pub centre: (i64, i64),
    pub half: (i64, i64),
}

pub struct Rules {
    pub width: i64,
    pub clearance: i64,
    pub via: i64,
    pub drill: i64,
    pub edge: i64,
    pub pitch: i64,
}

enum Shape {
    Rect((f64, f64), (f64, f64)),
    Seg((f64, f64), (f64, f64), f64),
    Disc((f64, f64), f64),
}

impl Shape {
    fn dist(&self, p: (f64, f64)) -> f64 {
        match *self {
            Shape::Rect(c, h) => {
                let dx = ((p.0 - c.0).abs() - h.0).max(0.0);
                let dy = ((p.1 - c.1).abs() - h.1).max(0.0);
                dx.hypot(dy)
            }
            Shape::Seg(a, b, hw) => {
                let (vx, vy) = (b.0 - a.0, b.1 - a.1);
                let len2 = vx * vx + vy * vy;
                let t = if len2 == 0.0 {
                    0.0
                } else {
                    (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0)
                };
                ((a.0 + t * vx - p.0).hypot(a.1 + t * vy - p.1) - hw).max(0.0)
            }
            Shape::Disc(c, r) => ((p.0 - c.0).hypot(p.1 - c.1) - r).max(0.0),
        }
    }
}

fn f(p: (i64, i64)) -> (f64, f64) {
    (p.0 as f64, p.1 as f64)
}

const LAYER_NAMES: [&str; 2] = ["F.Cu", "B.Cu"];

/// Routes each named two-pad net, in order, on a grid over both layers, with
/// earlier routes as obstacles. Dijkstra with a fixed tie-break, so the route
/// is a function of the board alone. Returns one line per net.
pub fn route(board: &mut Board, nets: &[&str], r: &Rules) -> Result<Vec<String>, String> {
    let mut report = Vec::new();
    for &net in nets {
        let pads = board.pads()?;
        let own: Vec<&PadCu> = pads
            .iter()
            .filter(|p| p.net.as_deref() == Some(net) && p.layer == "F.Cu")
            .collect();
        if own.len() != 2 {
            return Err(format!("net `{net}` has {} F.Cu pads, not 2", own.len()));
        }
        let mut obstacles: [Vec<Shape>; 2] = [Vec::new(), Vec::new()];
        for p in &pads {
            if p.net.as_deref() != Some(net) {
                let l = LAYER_NAMES.iter().position(|n| *n == p.layer).unwrap();
                obstacles[l].push(Shape::Rect(f(p.centre), f(p.half)));
            }
        }
        for s in &board.segs {
            if s.net != net {
                let l = LAYER_NAMES
                    .iter()
                    .position(|n| *n == s.layer)
                    .ok_or("segment layer")?;
                obstacles[l].push(Shape::Seg(f(s.a), f(s.b), s.width as f64 / 2.0));
            }
        }
        for v in &board.vias {
            if v.net != net {
                for o in &mut obstacles {
                    o.push(Shape::Disc(f(v.at), v.size as f64 / 2.0));
                }
            }
        }
        let (x0, y0, x1, y1) = board.outline;
        let lo_x = (x0 + r.edge + r.via).div_euclid(r.pitch) + 1;
        let hi_x = (x1 - r.edge - r.via).div_euclid(r.pitch);
        let lo_y = (y0 + r.edge + r.via).div_euclid(r.pitch) + 1;
        let hi_y = (y1 - r.edge - r.via).div_euclid(r.pitch);
        let (nx, ny) = ((hi_x - lo_x + 1) as usize, (hi_y - lo_y + 1) as usize);
        let pos =
            |ix: usize, iy: usize| ((lo_x + ix as i64) * r.pitch, (lo_y + iy as i64) * r.pitch);
        let margin = r.pitch as f64 / 2.0 + r.clearance as f64;
        let free = |l: usize, p: (i64, i64), radius: f64| {
            obstacles[l].iter().all(|o| o.dist(f(p)) >= radius + margin)
        };
        let idx = |ix: usize, iy: usize, l: usize| (l * ny + iy) * nx + ix;
        let mut track_ok = vec![false; nx * ny * 2];
        let mut via_ok = vec![false; nx * ny];
        for iy in 0..ny {
            for ix in 0..nx {
                let p = pos(ix, iy);
                for (l, _) in LAYER_NAMES.iter().enumerate() {
                    track_ok[idx(ix, iy, l)] = free(l, p, r.width as f64 / 2.0);
                }
                via_ok[iy * nx + ix] = (0..2).all(|l| free(l, p, r.via as f64 / 2.0));
            }
        }
        let nearest = |pad: &PadCu| -> Result<(usize, usize), String> {
            let ix = (pad.centre.0 as f64 / r.pitch as f64).round() as i64 - lo_x;
            let iy = (pad.centre.1 as f64 / r.pitch as f64).round() as i64 - lo_y;
            if ix < 0 || iy < 0 || ix as usize >= nx || iy as usize >= ny {
                return Err(format!("pad {}.{} is off the grid", pad.fp, pad.number));
            }
            Ok((ix as usize, iy as usize))
        };
        let (s, t) = (nearest(own[0])?, nearest(own[1])?);
        let start = idx(s.0, s.1, 0);
        let goal = idx(t.0, t.1, 0);
        track_ok[start] = true;
        track_ok[goal] = true;
        let mut dist = vec![u64::MAX; nx * ny * 2];
        let mut prev = vec![usize::MAX; nx * ny * 2];
        let mut heap = BinaryHeap::new();
        dist[start] = 0;
        heap.push(Reverse((0u64, start)));
        const VIA_COST: u64 = 40;
        while let Some(Reverse((d, u))) = heap.pop() {
            if d > dist[u] {
                continue;
            }
            if u == goal {
                break;
            }
            let l = u / (nx * ny);
            let iy = (u / nx) % ny;
            let ix = u % nx;
            let mut next = Vec::with_capacity(5);
            if ix > 0 {
                next.push((idx(ix - 1, iy, l), 1));
            }
            if ix + 1 < nx {
                next.push((idx(ix + 1, iy, l), 1));
            }
            if iy > 0 {
                next.push((idx(ix, iy - 1, l), 1));
            }
            if iy + 1 < ny {
                next.push((idx(ix, iy + 1, l), 1));
            }
            if via_ok[iy * nx + ix] {
                next.push((idx(ix, iy, 1 - l), VIA_COST));
            }
            for (v, c) in next {
                if !track_ok[v] {
                    continue;
                }
                let nd = d + c;
                if nd < dist[v] {
                    dist[v] = nd;
                    prev[v] = u;
                    heap.push(Reverse((nd, v)));
                }
            }
        }
        if dist[goal] == u64::MAX {
            return Err(format!("net `{net}` found no route"));
        }
        let mut path = vec![goal];
        while *path.last().unwrap() != start {
            path.push(prev[*path.last().unwrap()]);
        }
        path.reverse();
        let decode = |u: usize| {
            let l = u / (nx * ny);
            (pos(u % nx, (u / nx) % ny), l)
        };
        let mut nodes: Vec<((i64, i64), usize)> = path.into_iter().map(decode).collect();
        let (mut segs, mut vias) = (0, 0);
        let mut push = |board: &mut Board, a: (i64, i64), b: (i64, i64), l: usize| {
            if a != b {
                board.segs.push(Seg {
                    a,
                    b,
                    width: r.width,
                    layer: LAYER_NAMES[l].to_owned(),
                    net: net.to_owned(),
                });
                segs += 1;
            }
        };
        push(board, own[0].centre, nodes[0].0, 0);
        let mut run_start = nodes[0];
        let mut i = 1;
        while i < nodes.len() {
            let (p, l) = nodes[i];
            let (q, lq) = nodes[i - 1];
            if l != lq {
                push(board, run_start.0, q, lq);
                board.vias.push(Via {
                    at: q,
                    size: r.via,
                    drill: r.drill,
                    net: net.to_owned(),
                });
                vias += 1;
                run_start = (q, l);
            } else if i + 1 < nodes.len() {
                let (n, ln) = nodes[i + 1];
                let collinear = ln == l
                    && (n.0 - p.0).signum() == (p.0 - q.0).signum()
                    && (n.1 - p.1).signum() == (p.1 - q.1).signum();
                if !collinear {
                    push(board, run_start.0, p, l);
                    run_start = (p, l);
                }
            }
            i += 1;
        }
        let last = nodes.pop().unwrap();
        push(board, run_start.0, last.0, last.1);
        push(board, last.0, own[1].centre, 0);
        report.push(format!(
            "net {net}: {segs} segments, {vias} vias, cost {}",
            dist[goal]
        ));
    }
    Ok(report)
}
