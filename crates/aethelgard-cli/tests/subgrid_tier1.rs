//! `AET-WO-1310`, tier 1 (no tool runs): escapes for pads the 125 µm grid
//! cannot reach at the 200 µm floor, judged by the router's exact `check` and
//! `connectivity`, on boards built from the vendored footprints' geometry.
//! One test per pad class: an escape on the pad's own axis, a run of
//! adjacent pads on lanes, a sealed pad's channel escape (at exactly the
//! width a track needs, and 1 nm under it), a pad the grid already reaches
//! (no escape), a turned part, and the refusal by name of a run that has no
//! escape. Plus the exact rounded-pad clearance `check` now judges by.

use std::fs;
use std::path::Path;

use aethelgard_eda::pcb::Rect;
use aethelgard_eda::route::{self, Board, Pad, Part, Routed};
use aethelgard_eda::stack::Stack;

const MM: i64 = 1_000_000;

fn stack() -> Stack {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    Stack::of(&spec)
}

fn rect(c: (i64, i64), w: i64, h: i64) -> Rect {
    Rect {
        x0: c.0 - w / 2,
        y0: c.1 - h / 2,
        x1: c.0 + w - w / 2,
        y1: c.1 + h - h / 2,
    }
}

fn pad(number: &str, net: Option<String>, r: Rect, corner: i64) -> Pad {
    Pad {
        number: number.into(),
        net,
        rect: r,
        corner_nm: corner,
        drill_nm: None,
        join: None,
    }
}

fn square(points: Rect) -> Vec<route::Edge> {
    let (a, b, c, d) = (
        (points.x0, points.y0),
        (points.x1, points.y0),
        (points.x1, points.y1),
        (points.x0, points.y1),
    );
    vec![(a, b), (b, c), (c, d), (d, a)]
}

/// The ICM-20948's land (vendored `InvenSense_QFN-24_3x3mm_P0.4mm`: 0.55 x
/// 0.2 mm pads, rounding ratio 0.25, centres at ±1.5 mm and 0.4 mm apart)
/// as U1 at `origin`, turned `theta`, each pad `wired` given its own net and
/// a partner pad 3 mm out along its axis, spread twice as wide.
fn qfn24(origin: (i64, i64), theta: u16, wired: &dyn Fn(usize) -> bool) -> Board {
    let along = [-1_000_000, -600_000, -200_000, 200_000, 600_000, 1_000_000];
    let mut spots: Vec<(i64, i64, bool)> = Vec::new();
    for &a in &along {
        spots.push((-1_500_000, a, true));
    }
    for &a in &along {
        spots.push((a, 1_500_000, false));
    }
    for &a in along.iter().rev() {
        spots.push((1_500_000, a, true));
    }
    for &a in along.iter().rev() {
        spots.push((a, -1_500_000, false));
    }
    let mut pads = Vec::new();
    let mut parts = Vec::new();
    for (i, &(x, y, wide)) in spots.iter().enumerate() {
        let (rx, ry) = route::rotate(theta, (x, y));
        let wide = wide != (theta % 180 == 90);
        let (w, h) = if wide {
            (550_000, 200_000)
        } else {
            (200_000, 550_000)
        };
        let net = wired(i).then(|| format!("N{}", i + 1));
        pads.push(pad(
            &(i + 1).to_string(),
            net.clone(),
            rect((origin.0 + rx, origin.1 + ry), w, h),
            50_000,
        ));
        if let Some(net) = net {
            let (ax, ay) = if rx.abs() > ry.abs() {
                (rx.signum(), 0)
            } else {
                (0, ry.signum())
            };
            let p = (
                origin.0 + 2 * rx + ax * 3 * MM,
                origin.1 + 2 * ry + ay * 3 * MM,
            );
            parts.push(Part {
                reference: format!("P{}", i + 1),
                rotation: 0,
                pads: vec![pad("1", Some(net), rect(p, 300_000, 300_000), 0)],
                courtyard: vec![],
                keepouts: vec![],
            });
        }
    }
    let field = rect(origin, 3_550_000, 3_550_000);
    parts.insert(
        0,
        Part {
            reference: "U1".into(),
            rotation: theta,
            pads,
            courtyard: square(field),
            keepouts: vec![],
        },
    );
    Board {
        outline: Rect {
            x0: origin.0 - 10 * MM,
            y0: origin.1 - 10 * MM,
            x1: origin.0 + 10 * MM,
            y1: origin.1 + 10 * MM,
        },
        parts,
        stack: stack(),
    }
}

/// Each segment of `routed` that starts or ends at the centre of U1's pad.
fn from_centre<'a>(b: &Board, routed: &'a Routed, number: &str) -> Vec<&'a route::Segment> {
    let p = b.parts[0].pads.iter().find(|p| p.number == number).unwrap();
    let c = ((p.rect.x0 + p.rect.x1) / 2, (p.rect.y0 + p.rect.y1) / 2);
    routed
        .segments
        .iter()
        .filter(|s| s.start == c || s.end == c)
        .collect()
}

fn judged(b: &Board) -> Routed {
    let r = route::route(b).unwrap_or_else(|e| panic!("{e:?}"));
    route::check(b, &r).unwrap();
    route::connectivity(b, &r).unwrap();
    r
}

/// Whether U1's pad `i` (from 0) has a neighbour on each side along its row:
/// such a pad of the QFN never has a grid node at the floor (FEEDBACK §3); a
/// row's end pad may.
fn inner(i: usize) -> bool {
    (1..=4).contains(&(i % 6))
}

/// A: every other pad of the QFN wired, at several of the placer's 25 µm
/// residues: each inner one leaves from its centre along its own axis, out
/// across the pad's outer end; an end pad may be reached on the grid.
#[test]
fn a_sub_grid_pad_escapes_along_its_own_axis() {
    for (dx, dy) in [(0, 0), (25_000, 50_000), (75_000, 100_000)] {
        let b = qfn24((50 * MM + dx, 50 * MM + dy), 0, &|i| i % 2 == 0);
        let r = judged(&b);
        for (i, p) in b.parts[0]
            .pads
            .iter()
            .enumerate()
            .filter(|(_, p)| p.net.is_some())
        {
            let first = from_centre(&b, &r, &p.number);
            if first.is_empty() && !inner(i) {
                continue;
            }
            assert_eq!(first.len(), 1, "pad {}: {first:?}", p.number);
            let s = first[0];
            let wide = p.rect.x1 - p.rect.x0 > p.rect.y1 - p.rect.y0;
            // Along the pad's long axis: y fixed for a pad long in x.
            assert_eq!(s.start.1 == s.end.1, wide, "pad {}: {s:?}", p.number);
            assert_eq!(s.layer, route::Layer::F);
        }
    }
}

/// A's lane plan: a run of four adjacent wired pads (0.4 mm apart, as on
/// the power fixture's U5) reaches the grid on lanes at least 500 µm apart,
/// each lane a grid line.
#[test]
fn a_run_of_adjacent_pads_reaches_the_grid_on_lanes() {
    let b = qfn24((50 * MM + 50_000, 50 * MM), 0, &|i| (7..11).contains(&i));
    let r = judged(&b);
    let mut lanes: Vec<i64> = (8..12)
        .map(|n| {
            let path: Vec<&route::Segment> = from_centre(&b, &r, &n.to_string());
            assert_eq!(path.len(), 1);
            // The pad is long in y: its escape leaves along y, then jogs in x
            // to its lane, which the next segment of its net runs down.
            let s = path[0];
            let far = if s.start.0 == s.end.0 && s.start != s.end {
                if (s.start.1 - 50 * MM).abs() > (s.end.1 - 50 * MM).abs() {
                    s.start
                } else {
                    s.end
                }
            } else {
                unreachable!("{s:?}")
            };
            r.segments
                .iter()
                .filter(|t| {
                    t.net == s.net
                        && t.start.1 == far.1
                        && t.end.1 == far.1
                        && (t.start == far || t.end == far)
                })
                .map(|t| if t.start == far { t.end.0 } else { t.start.0 })
                .next()
                .unwrap_or(far.0)
        })
        .collect();
    lanes.sort();
    for w in lanes.windows(2) {
        assert!(w[1] - w[0] >= 500_000, "{lanes:?}");
    }
    let x0 = b.outline.x0.div_euclid(125_000) * 125_000;
    assert!(lanes.iter().all(|l| (l - x0) % 125_000 == 0), "{lanes:?}");
}

/// A turned part: the same QFN at 90°, every other pad wired, routes.
#[test]
fn a_turned_part_escapes() {
    let b = qfn24((50 * MM + 25_000, 50 * MM + 75_000), 90, &|i| i % 2 == 1);
    let r = judged(&b);
    for (i, p) in b.parts[0]
        .pads
        .iter()
        .enumerate()
        .filter(|(i, p)| p.net.is_some() && inner(*i))
    {
        assert_eq!(
            from_centre(&b, &r, &p.number).len(),
            1,
            "pad {} ({i})",
            p.number
        );
    }
}

/// A pad the grid reaches gets no escape: on the reference board every
/// track end is a grid node (`AET-WO-1310`: today's boards are routed
/// exactly as before).
#[test]
fn a_pad_the_grid_reaches_gets_no_escape() {
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/sentinel-01-minimal.json");
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    let e = aethelgard_eda::schematic::emit(&spec, &solved, &table).unwrap();
    let (b, r) = route::read(
        &aethelgard_eda::strict::board(&e.board).unwrap(),
        &Stack::of(&spec),
    )
    .unwrap();
    let ceil = |v: i64| -((-v).div_euclid(125_000)) * 125_000;
    let (gx, gy) = (ceil(b.outline.x0), ceil(b.outline.y0));
    for s in &r.segments {
        for (x, y) in [s.start, s.end] {
            assert!((x - gx) % 125_000 == 0 && (y - gy) % 125_000 == 0, "{s:?}");
        }
    }
}

/// A sealed pad (A′): an IC's pad whose way out is a channel between its row
/// and a foreign pad, `gap` wide. At exactly the width a track needs
/// (clearance + width + clearance, 600 µm) it escapes down the channel's
/// centreline; 1 nm narrower it does not, and the board is refused.
fn sealed(gap: i64) -> Board {
    let o = (50 * MM + 43_822, 50 * MM);
    let mut ic = Vec::new();
    for (i, y) in [-625_000, 0, 625_000].iter().enumerate() {
        let net = (i == 1).then(|| "N".to_owned());
        ic.push(pad(
            &(i + 4).to_string(),
            net,
            rect((o.0, o.1 + y), 375_000, 375_000),
            0,
        ));
        ic.push(pad(
            &(i + 1).to_string(),
            None,
            rect((o.0 - 3 * MM, o.1 + y), 375_000, 375_000),
            0,
        ));
    }
    let edge = o.0 + 187_500;
    let wall = rect((edge + gap + 250_000, o.1), 500_000, 900_000);
    let yard = Rect {
        x0: o.0 - 3 * MM - 400_000,
        y0: o.1 - 1_000_000,
        x1: edge + 100_000,
        y1: o.1 + 1_000_000,
    };
    Board {
        outline: Rect {
            x0: o.0 - 10 * MM,
            y0: o.1 - 10 * MM,
            x1: o.0 + 10 * MM,
            y1: o.1 + 10 * MM,
        },
        parts: vec![
            Part {
                reference: "U3".into(),
                rotation: 0,
                pads: ic,
                courtyard: square(yard),
                keepouts: vec![],
            },
            Part {
                reference: "W1".into(),
                rotation: 0,
                pads: vec![pad("1", None, wall, 0)],
                courtyard: vec![],
                keepouts: vec![],
            },
            Part {
                reference: "P1".into(),
                rotation: 0,
                pads: vec![pad(
                    "1",
                    Some("N".into()),
                    rect((o.0 + 6 * MM, o.1 + 4 * MM), 400_000, 400_000),
                    0,
                )],
                courtyard: vec![],
                keepouts: vec![],
            },
        ],
        stack: stack(),
    }
}

#[test]
fn a_sealed_pad_escapes_down_its_channel_at_exactly_the_width_a_track_needs() {
    let b = sealed(600_000);
    let r = judged(&b);
    let edge = 50 * MM + 43_822 + 187_500;
    // The run down the channel: on its centreline, 300 µm from each side.
    assert!(
        r.segments.iter().any(|s| s.start.0 == edge + 300_000
            && s.end.0 == edge + 300_000
            && s.start.1 != s.end.1),
        "{:?}",
        r.segments
    );
    let e = route::route(&sealed(599_999)).unwrap_err();
    assert_eq!(e.net, "N", "{e:?}");
    assert!(e.exhausted.is_none(), "{e:?}");
}

/// A run with no escape is refused by name (`PadEscapeExhausted`), naming the
/// part and the run's pads: here a foreign pad 300 µm in front of the QFN's
/// bottom side closes every lane.
#[test]
fn a_run_with_no_escape_is_refused_by_name() {
    let mut b = qfn24((50 * MM, 50 * MM), 0, &|i| (6..12).contains(&i));
    let field_bottom = 50 * MM + 1_775_000;
    b.parts.push(Part {
        reference: "W1".into(),
        rotation: 0,
        pads: vec![pad(
            "1",
            None,
            rect((50 * MM, field_bottom + 300_000 + 150_000), 5 * MM, 300_000),
            0,
        )],
        courtyard: vec![],
        keepouts: vec![],
    });
    let e = route::route(&b).unwrap_err();
    assert_eq!(
        e.exhausted,
        Some((
            "U1".to_owned(),
            ["7", "8", "9", "10", "11", "12"].map(String::from).to_vec()
        )),
        "{e:?}"
    );
}

/// `check` judges a pad by its copper, a rounded rectangle: the ICM-20948's
/// corner pads 1 and 24 are 0.177 mm apart by their boxes and 0.218 mm as
/// copper, and pass; two square-cornered pads 199 µm apart do not.
#[test]
fn check_judges_pads_as_rounded_copper() {
    let one = |a: Pad, b: Pad| Board {
        outline: rect((50 * MM, 50 * MM), 10 * MM, 10 * MM),
        parts: vec![Part {
            reference: "U1".into(),
            rotation: 0,
            pads: vec![a, b],
            courtyard: vec![],
            keepouts: vec![],
        }],
        stack: stack(),
    };
    let o = (50 * MM, 50 * MM);
    // Pad 1 (long in x, at (-1.5, -1.0)) and pad 24 (long in y, at (-1.0, -1.5)).
    let p1 = pad(
        "1",
        Some("A".into()),
        rect((o.0 - 1_500_000, o.1 - 1_000_000), 550_000, 200_000),
        50_000,
    );
    let p24 = pad(
        "24",
        Some("B".into()),
        rect((o.0 - 1_000_000, o.1 - 1_500_000), 200_000, 550_000),
        50_000,
    );
    route::check(&one(p1, p24), &Routed::default()).unwrap();
    let a = pad("1", Some("A".into()), rect(o, 400_000, 400_000), 0);
    let b = pad(
        "2",
        Some("B".into()),
        rect((o.0 + 599_000, o.1), 400_000, 400_000),
        0,
    );
    assert!(route::check(&one(a, b), &Routed::default()).is_err());
}

/// A's measured reach (FEEDBACK §8): the QFN with every one of its 24 pads
/// wired, a full run on each side, routes at each of the placer's 25 µm
/// residues, turned 0° and 90°; and where it could not, the run would be
/// refused by name, never left to the router.
#[test]
fn a_fully_wired_qfn_routes_at_every_residue() {
    for theta in [0u16, 90] {
        for dx in (0..125_000).step_by(25_000) {
            for dy in (0..125_000).step_by(25_000) {
                let b = qfn24((50 * MM + dx, 50 * MM + dy), theta, &|_| true);
                judged(&b);
            }
        }
    }
}
