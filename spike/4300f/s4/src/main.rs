//! AET-WO-4300f spike S4 (host half): can the repository's router route
//! Sentinel-01 with one hand-placed JST-GH 4-pin connector at the bottom edge?
use aethelgard_eda::pcb::Rect;
use aethelgard_eda::route::{self, Board, Layer};
use aethelgard_eda::sexpr::{self, Sx};
use aethelgard_eda::{schematic, stack, strict};
use sha2::{Digest, Sha256};
use std::time::Instant;

fn home() -> String {
    std::env::var("HOME").unwrap()
}

fn sha(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}

fn mm(nm: i64) -> String {
    sexpr::mm(nm)
}

fn counts(r: &route::Routed) -> String {
    format!(
        "segments={} vias={} zones={} ripups={} length_nm={}",
        r.segments.len(),
        r.vias.len(),
        r.zones.len(),
        r.ripups,
        r.length_nm()
    )
}

fn uuid_sx(key: &str) -> Sx {
    Sx::List(vec![Sx::atom("uuid"), Sx::str(schematic::uuid(key))])
}
fn at_sx(head: &str, (x, y): (i64, i64)) -> Sx {
    Sx::node(head, &[&mm(x), &mm(y)])
}

/// The connector footprint as pcb.rs `footprint()` builds a placed one.
fn connector(lib: &Sx, at: (i64, i64), angle: &str, nets: &[(&str, &str)]) -> Sx {
    let mut out = vec![
        Sx::atom("footprint"),
        Sx::str("Connector_JST:JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal"),
        Sx::List(vec![Sx::atom("layer"), Sx::str("F.Cu")]),
        uuid_sx("s4/footprint/J1"),
    ];
    let mut a = vec![Sx::atom("at"), Sx::atom(mm(at.0)), Sx::atom(mm(at.1))];
    if angle != "0" {
        a.push(Sx::atom(angle));
    }
    out.push(Sx::List(a));
    for child in lib.items().get(2..).unwrap_or(&[]) {
        let mut c = child.clone();
        match child.head() {
            Some("version" | "generator" | "generator_version" | "layer") => continue,
            Some("property") if child.arg(1) == Some("Reference") => {
                c.items_mut().unwrap()[2] = Sx::str("J1");
            }
            Some("pad") => {
                // pads are matched in order: "MP" twice share one entry
                let number = child.arg(1).unwrap_or("");
                if let Some((_, net)) = nets.iter().find(|(n, _)| *n == number) {
                    c.items_mut()
                        .unwrap()
                        .push(Sx::List(vec![Sx::atom("net"), Sx::str(*net)]));
                }
            }
            _ => {}
        }
        out.push(c);
    }
    Sx::List(out)
}

fn ref_of(fp: &Sx) -> &str {
    fp.items()
        .iter()
        .find(|c| c.head() == Some("property") && c.arg(1) == Some("Reference"))
        .and_then(|c| c.arg(2))
        .unwrap_or("")
}

fn main() {
    let h = home();
    let root = std::env::current_dir().unwrap();
    let spec_bytes = std::fs::read(root.join("examples/sentinel-01-minimal.json")).unwrap();
    let spec: aethelgard_ir::Specification = serde_json::from_slice(&spec_bytes).unwrap();
    spec.validate().unwrap();
    let st = stack::Stack::of(&spec);
    println!(
        "stack: layers={} width_nm={} clearance_nm={} planes={:?}",
        st.layers(),
        st.width_nm,
        st.clearance_nm,
        st.planes
    );
    let pcb_path = format!("{h}/.cache/aet-4300f/s4-ref/sentinel-01.kicad_pcb");
    let text = std::fs::read_to_string(&pcb_path).unwrap();
    println!("reference board sha256 {}", sha(text.as_bytes()));
    let tree = strict::board(&text).unwrap();
    println!(
        "route::declared: {:?}",
        route::declared(&tree, &st).map_err(|d| d.message)
    );
    let (board, read_routed) = route::read(&tree, &st).unwrap();
    let nets: std::collections::BTreeSet<&str> = board
        .parts
        .iter()
        .flat_map(|p| p.pads.iter().filter_map(|x| x.net.as_deref()))
        .collect();
    println!(
        "parts={} outline_nm={:?} nets={}",
        board.parts.len(),
        board.outline,
        nets.len()
    );
    println!(
        "nets matching I2C|3V3|GND: {:?}",
        nets.iter()
            .filter(|n| n.contains("I2C") || n.contains("3V3") || **n == "GND")
            .collect::<Vec<_>>()
    );
    println!("read():  {}", counts(&read_routed));
    let t = Instant::now();
    let rr = route::route(&board).expect("reference reroute");
    println!("route(): {}  wall={:?}", counts(&rr), t.elapsed());
    let (mut a, mut b) = (rr.clone(), read_routed.clone());
    a.segments.sort();
    a.vias.sort();
    b.segments.sort();
    b.vias.sort();
    println!(
        "reroute == read (segments, vias, zones): {} {} {}",
        a.segments == b.segments,
        a.vias == b.vias,
        a.zones == b.zones
    );
    println!(
        "reference check: {:?}; connectivity: {:?}",
        route::check(&board, &rr),
        route::connectivity(&board, &rr)
    );
    // round trip of the emitted text through parse/write
    let sx = sexpr::parse(&text).unwrap();
    println!(
        "parse/write round-trip identical to emitted text: {}",
        sexpr::write(&sx) == text
    );

    let lib_text = std::fs::read_to_string(format!(
        "{h}/.cache/aet-4300f/kicad-lib/gitlab-10.0.7/Connector_JST.pretty/JST_GH_SM04B-GHS-TB_1x04-1MP_P1.25mm_Horizontal.kicad_mod"
    ))
    .unwrap();
    let lib = sexpr::parse(&lib_text).unwrap();

    // Placement.
    let o = board.outline;
    let cx = ((o.x0 + o.x1) / 2 + 25_000).div_euclid(50_000) * 50_000;
    // The placer's own legality (pcb.rs `place`/`spot`): J1's keep-out box (courtyard ∪
    // silkscreen ∪ visible silkscreen text, `pcb::geometry`) clears every placed keep-out box
    // by GAP_NM, or IC_GAP_NM when both are ICs (`is_ic`: lib id not `Device:`; a connector
    // is one). The first such origin moving down from the old edge, on the 50 µm snap.
    let shift = |r: Rect, (dx, dy): (i64, i64)| Rect {
        x0: r.x0 + dx,
        y0: r.y0 + dy,
        x1: r.x1 + dx,
        y1: r.y1 + dy,
    };
    let grow = |r: Rect, d: i64| Rect {
        x0: r.x0 - d,
        y0: r.y0 - d,
        x1: r.x1 + d,
        y1: r.y1 + d,
    };
    let mut keeps: Vec<(String, bool, Rect)> = Vec::new();
    for item in sx.items() {
        if item.head() != Some("footprint") {
            continue;
        }
        let lib_id = item.arg(1).unwrap_or("").to_owned();
        let reference = item
            .items()
            .iter()
            .find(|c| c.head() == Some("property") && c.arg(1) == Some("Reference"))
            .and_then(|c| c.arg(2))
            .unwrap_or("?")
            .to_owned();
        let at = item.child("at").expect("footprint at");
        let x = at.arg(1).and_then(sexpr::nm).expect("at x");
        let y = at.arg(2).and_then(sexpr::nm).expect("at y");
        let g = aethelgard_eda::pcb::geometry(item, &reference).expect("geometry");
        keeps.push((
            reference,
            !lib_id.starts_with("Device:"),
            shift(g.keep, (x, y)),
        ));
    }
    let jg = aethelgard_eda::pcb::geometry(&lib, "J1").expect("J1 geometry");
    println!(
        "J1 local courtyard {:?} keep {:?} (keep is courtyard + silkscreen text)",
        jg.courtyard, jg.keep
    );
    let clears = |oy: i64| {
        let k = shift(jg.keep, (cx, oy));
        keeps.iter().all(|(_, ic, r)| {
            let gap = if *ic {
                aethelgard_eda::pcb::IC_GAP_NM
            } else {
                aethelgard_eda::pcb::GAP_NM
            };
            !grow(k, gap).overlaps(*r)
        })
    };
    let mut oy = o.y1 - 1_500_000 - jg.courtyard.y0;
    while !clears(oy) {
        oy += 50_000;
    }
    let origin = (cx, oy);
    {
        // Validation of `keeps`: the emitter's outline is their union grown by MARGIN_NM
        // (pcb.rs `layout_of`), so rebuilding it here must reproduce `board.outline`.
        let u = keeps
            .iter()
            .map(|(_, _, r)| *r)
            .reduce(|a, b| Rect {
                x0: a.x0.min(b.x0),
                y0: a.y0.min(b.y0),
                x1: a.x1.max(b.x1),
                y1: a.y1.max(b.y1),
            })
            .expect("keeps");
        let g = grow(u, aethelgard_eda::pcb::MARGIN_NM);
        println!(
            "keeps union grown by MARGIN {:?} vs emitted outline {:?}: max |delta| {} nm",
            g,
            o,
            [g.x0 - o.x0, g.y0 - o.y0, g.x1 - o.x1, g.y1 - o.y1]
                .iter()
                .map(|d| d.abs())
                .max()
                .unwrap()
        );
        // Smallest clearance (Chebyshev box gap) from J1's keep to any placed keep.
        let k = shift(jg.keep, origin);
        let gap_to = |r: &Rect| {
            let dx = (r.x0 - k.x1).max(k.x0 - r.x1).max(0);
            let dy = (r.y0 - k.y1).max(k.y0 - r.y1).max(0);
            dx.max(dy)
        };
        let min = keeps
            .iter()
            .map(|(n, ic, r)| (gap_to(r), n.clone(), *ic))
            .min()
            .unwrap();
        println!(
            "smallest J1 keep-out clearance over all {} boxes: {} mm to {} (is_ic {})",
            keeps.len(),
            mm(min.0),
            min.1,
            min.2
        );
    }
    let court_top = oy + jg.courtyard.y0;
    let new_y1 = oy + jg.courtyard.y1;
    {
        let k = shift(jg.keep, origin);
        let nearest = keeps
            .iter()
            .filter(|(_, _, r)| r.x1 > k.x0 && r.x0 < k.x1)
            .map(|(n, ic, r)| (k.y0 - r.y1, n.clone(), *ic))
            .min();
        println!(
            "placed by keep-out: J1 keep {:?}; nearest box above (gap, ref, is_ic): {:?}; keep bottom vs new edge: {} mm",
            k,
            nearest.map(|(g, n, ic)| (mm(g), n, ic)),
            mm(new_y1 - k.y1)
        );
    }
    let new_outline = Rect { y1: new_y1, ..o };
    println!(
        "old outline {:?}\nconnector origin=({},{}) courtyard y {}..{} new outline {:?}",
        o,
        mm(origin.0),
        mm(origin.1),
        mm(court_top),
        mm(new_y1),
        new_outline
    );

    // rotation refusal, as read
    let rot = connector(&lib, origin, "90", &[]);
    println!(
        "part_of(J1 at 90 deg): {:?}",
        route::part_of(&rot).map(|p| p.pads.len())
    );

    {
        let max_y = board
            .parts
            .iter()
            .flat_map(|p| p.pads.iter())
            .map(|p| p.rect.y1)
            .max()
            .unwrap();
        println!(
            "existing pads: max pad y1 = {} mm (old outline y1 {} mm, margin {})",
            mm(max_y),
            mm(o.y1),
            mm(o.y1 - max_y)
        );
        let (j0, j1) = (court_top, new_y1);
        let (jx0, jx1) = (origin.0 - 4_730_000, origin.0 + 4_730_000);
        for p in &board.parts {
            for pad in &p.pads {
                let r = pad.rect;
                if r.y1 > j0 && r.x1 > jx0 && r.x0 < jx1 && r.y0 < j1 {
                    println!(
                        "  existing pad {} {} overlaps J1 courtyard box",
                        p.reference, pad.number
                    );
                }
            }
        }
    }
    let base_nets = [
        ("1", "VDD_3V3"),
        ("2", "/I2C_SCL"),
        ("3", "/I2C_SDA"),
        ("4", "GND"),
    ];
    let out_dir = format!("{h}/.cache/aet-4300f/s4-out");
    std::fs::create_dir_all(&out_dir).unwrap();
    for (variant, mp) in [("A", None), ("B", Some("GND"))] {
        println!("\n=== variant {variant} (MP net: {mp:?}) ===");
        let mut nets: Vec<(&str, &str)> = base_nets.to_vec();
        if let Some(n) = mp {
            nets.push(("MP", n));
        }
        let fp = connector(&lib, origin, "0", &nets);
        let part = route::part_of(&fp).expect("part_of J1");
        for p in &part.pads {
            let r = p.rect;
            let d = (r.x0 - new_outline.x0)
                .min(new_outline.x1 - r.x1)
                .min(r.y0 - new_outline.y0)
                .min(new_outline.y1 - r.y1);
            println!(
                "  pad {:>2} net={:?} rect=({},{})-({},{}) edge_dist_mm={} (>= EDGE {}: {})",
                p.number,
                p.net,
                mm(r.x0),
                mm(r.y0),
                mm(r.x1),
                mm(r.y1),
                mm(d),
                mm(route::EDGE_NM),
                d >= route::EDGE_NM
            );
        }
        let mut nb = Board {
            outline: new_outline,
            parts: board.parts.clone(),
            stack: st.clone(),
        };
        nb.parts.push(part);
        let t = Instant::now();
        let res = route::route(&nb);
        let wall = t.elapsed();
        let routed = match res {
            Ok(r) => {
                println!("route: Ok  {}  wall={:?}", counts(&r), wall);
                r
            }
            Err(e) => {
                println!(
                    "route: Err net={:?} why={:?}  wall={:?}",
                    e.net, e.why, wall
                );
                continue;
            }
        };
        println!("check:        {:?}", route::check(&nb, &routed));
        println!("connectivity: {:?}", route::connectivity(&nb, &routed));
        let j1: Vec<&str> = {
            let mut v: Vec<&str> = nets.iter().map(|(_, n)| *n).collect();
            v.sort();
            v.dedup();
            v
        };
        for n in &j1 {
            let segs = routed.segments.iter().filter(|s| s.net == *n).count();
            let vias = routed.vias.iter().filter(|s| s.net == *n).count();
            println!(
                "  net {n}: segments={segs} vias={vias} (zones on net: {})",
                routed.zones.iter().filter(|z| z.0 == *n).count()
            );
        }
        println!(
            "  F.Cu segs={} B.Cu segs={}",
            routed
                .segments
                .iter()
                .filter(|s| s.layer == Layer::F)
                .count(),
            routed
                .segments
                .iter()
                .filter(|s| s.layer == Layer::B)
                .count()
        );

        // Output board.
        let mut items: Vec<Sx> = Vec::new();
        let mut placed = false;
        let all = sx.items().to_vec();
        for (i, c) in all.iter().enumerate() {
            match c.head() {
                Some("footprint") => {
                    if !placed && ref_of(c) > "J1" {
                        items.push(fp.clone());
                        placed = true;
                    }
                    items.push(c.clone());
                }
                Some("gr_rect") => {
                    if !placed {
                        items.push(fp.clone());
                        placed = true;
                    }
                    let n = new_outline;
                    items.push(Sx::List(vec![
                        Sx::atom("gr_rect"),
                        at_sx("start", (n.x0, n.y0)),
                        at_sx("end", (n.x1, n.y1)),
                        Sx::List(vec![
                            Sx::atom("stroke"),
                            Sx::node("width", &["0.05"]),
                            Sx::node("type", &["default"]),
                        ]),
                        Sx::node("fill", &["no"]),
                        Sx::List(vec![Sx::atom("layer"), Sx::str("Edge.Cuts")]),
                        uuid_sx("sentinel-01/outline"),
                    ]));
                    let _ = i;
                }
                Some("segment" | "via" | "zone") => {}
                _ => items.push(c.clone()),
            }
        }
        // copper goes before the trailing embedded_fonts
        let last = items.pop().unwrap();
        assert_eq!(last.head(), Some("embedded_fonts"));
        items.extend(copper("sentinel-01", &routed, new_outline, st.width_nm));
        items.push(last);
        let outp = sexpr::write(&Sx::List(items));
        let path = format!("{out_dir}/sentinel-01-J1-{variant}.kicad_pcb");
        std::fs::write(&path, &outp).unwrap();
        println!("wrote {path} sha256 {}", sha(outp.as_bytes()));
        // read it back with the repo's own reader and compare
        let t2 = strict::board(&outp).unwrap();
        println!(
            "  declared: {:?}",
            route::declared(&t2, &st).map_err(|d| d.message)
        );
        match route::read(&t2, &st) {
            Ok((b2, r2)) => {
                println!(
                    "  read-back: parts={} outline_eq={} board_eq={} segs_eq={} vias_eq={}  check={:?} connectivity={:?}",
                    b2.parts.len(),
                    b2.outline == nb.outline,
                    {
                        let mut x = b2.parts.clone();
                        let mut y = nb.parts.clone();
                        x.sort_by(|a, b| a.reference.cmp(&b.reference));
                        y.sort_by(|a, b| a.reference.cmp(&b.reference));
                        x == y
                    },
                    r2.segments == {
                        let mut s = routed.segments.clone();
                        s.sort();
                        s
                    },
                    r2.vias == {
                        let mut s = routed.vias.clone();
                        s.sort();
                        s
                    },
                    route::check(&b2, &r2),
                    route::connectivity(&b2, &r2)
                );
            }
            Err(e) => println!("  read-back Err: {e}"),
        }
    }
}

/// pcb.rs `copper()`, copied (it is private there).
fn copper(project: &str, routed: &route::Routed, o: Rect, width_nm: i64) -> Vec<Sx> {
    let net = |n: &str| Sx::List(vec![Sx::atom("net"), Sx::str(n)]);
    let layer = |l: Layer| Sx::List(vec![Sx::atom("layer"), Sx::str(l.name())]);
    let mut out = Vec::new();
    for s in &routed.segments {
        out.push(Sx::List(vec![
            Sx::atom("segment"),
            at_sx("start", s.start),
            at_sx("end", s.end),
            Sx::node("width", &[&mm(width_nm)]),
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
            Sx::List(vec![Sx::atom("layer"), Sx::str(l)]),
            uuid_sx(&format!("{project}/zone/{n}/{l}")),
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
