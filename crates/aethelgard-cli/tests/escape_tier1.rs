//! `AET-WO-1307`, tier 1 (no tool runs): escape planning and a net order
//! that is a fact about the board. The router routes the reference, `D1` and
//! `D3` to one geometry under every one of 20 fixed renamings of every net
//! and reference, with the parts and their pads permuted too (ruled K3: the
//! guarantee is the router's, at board level); the reproducers of `1102` A6
//! and `2204` A5 route; the reader refuses a zone that is not the board's
//! outline (K6); and the router's exact judge refuses a via in a part's
//! courtyard, but in the dense part's own ring interior (K1).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aethelgard_eda::route::{self, Board, Routed, Via};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::strict;
use serde_json::Value;

const SPEC: &str = "examples/sentinel-01-minimal.json";
const FOUR: &str = "crates/aethelgard-cli/tests/fixtures/four-layer.json";
const D1: &str = "crates/aethelgard-cli/tests/fixtures/d1-24-i2c.json";
const D3: &str = "crates/aethelgard-cli/tests/fixtures/d3-24-i2c-12-spi.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1307-{label}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(args: &[&str]) -> (i32, Value) {
    let out = Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args(args)
        .current_dir(root())
        .stdin(Stdio::null())
        .output()
        .unwrap();
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1, "{stdout}");
    (
        out.status.code().unwrap(),
        serde_json::from_str(&stdout).unwrap(),
    )
}

/// `emit` of the specification at `spec` into `dir`: the exit code, the
/// verdict and the board text.
fn emit(dir: &Path, spec: &str) -> (i32, Value, String) {
    let out = dir.join("out");
    let (code, v) = run(&["emit", "--file", spec, "--out", out.to_str().unwrap()]);
    let board = v["board"]["path"]
        .as_str()
        .map(|p| fs::read_to_string(p).unwrap())
        .unwrap_or_default();
    (code, v, board)
}

fn stack_of(spec: &str) -> Stack {
    let bytes = fs::read(root().join(spec)).unwrap();
    Stack::of(&serde_json::from_slice::<aethelgard_ir::Specification>(&bytes).unwrap())
}

/// The emitted board of `spec`, as the router reads it, and its copper.
fn emitted(label: &str, spec: &str) -> (Board, Routed, String) {
    let dir = scratch(label);
    let (code, v, text) = emit(&dir, spec);
    assert_eq!(code, 0, "{label}: {v}");
    fs::remove_dir_all(&dir).unwrap();
    let (board, routed) = route::read(&strict::board(&text).unwrap(), &stack_of(spec)).unwrap();
    (board, routed, text)
}

/// SplitMix64: a fixed, toolchain-independent hash for the renamings.
fn mix(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn hash(seed: u64, s: &str) -> u64 {
    s.bytes().fold(mix(seed), |h, b| mix(h ^ u64::from(b)))
}

/// Renaming `seed` of `board`: every net (ground at 2 layers aside, whose
/// name is what makes it stitched there; an `unconnected-` net keeps its
/// prefix) and every reference gets a seeded prefix, so the names sort in a
/// new order; the parts, and each part's pads, are put in a seeded order.
/// Returns the board and the map from new net names to old.
fn renamed(board: &Board, seed: u64) -> (Board, BTreeMap<String, String>) {
    let new_net = |n: &str| {
        if (!board.stack.has_planes() && n == route::GROUND) || n.starts_with("unconnected-") {
            n.to_owned()
        } else {
            format!("{:04x}_{n}", hash(seed, n) & 0xffff)
        }
    };
    let mut back = BTreeMap::new();
    let mut parts = board.parts.clone();
    for part in &mut parts {
        part.reference = format!(
            "Q{:04x}_{}",
            hash(seed ^ 1, &part.reference) & 0xffff,
            part.reference
        );
        for pad in &mut part.pads {
            if let Some(n) = &pad.net {
                let m = new_net(n);
                back.insert(m.clone(), n.clone());
                pad.net = Some(m);
            }
        }
        part.pads.sort_by_key(|p| hash(seed ^ 2, &p.number));
    }
    parts.sort_by_key(|p| hash(seed ^ 3, &p.reference));
    let mut stack = board.stack.clone();
    for (_, n) in &mut stack.planes {
        let m = new_net(n);
        back.insert(m.clone(), n.clone());
        *n = m;
    }
    (
        Board {
            outline: board.outline,
            parts,
            stack,
        },
        back,
    )
}

/// `routed` with its nets named back by `back`, in the reader's order.
fn named_back(routed: &Routed, back: &BTreeMap<String, String>) -> Routed {
    let mut out = routed.clone();
    for s in &mut out.segments {
        s.net = back[&s.net].clone();
    }
    for v in &mut out.vias {
        v.net = back[&v.net].clone();
    }
    for (n, _) in &mut out.zones {
        *n = back[n].clone();
    }
    out.segments.sort();
    out.vias.sort();
    out.zones.sort();
    out.ripups = 0;
    out
}

/// WO §2.2 and §4, ruled K3: `spec`'s emitted board, renamed by each of
/// `seeds` with its parts and pads permuted, routes, and to the board's own
/// copper exactly once its names are put back.
fn one_geometry(label: &str, spec: &str, seeds: std::ops::Range<u64>) {
    let (board, emitted, _) = emitted(label, spec);
    let mut own = emitted.clone();
    own.zones.sort();
    for seed in seeds {
        let (b, back) = renamed(&board, seed);
        assert_ne!(b.parts, board.parts, "{label} seed {seed}: nothing renamed");
        let r = route::route(&b).unwrap_or_else(|e| panic!("{label} seed {seed}: {e:?}"));
        assert_eq!(named_back(&r, &back), own, "{label} seed {seed}");
    }
}

#[test]
fn twenty_renamings_route_the_reference_to_its_own_copper() {
    one_geometry("ref", SPEC, 0..20);
}

#[test]
fn twenty_renamings_route_d1_to_its_own_copper() {
    one_geometry("d1", D1, 0..20);
}

/// `D3` routes at 4 layers (WO §2.1; it failed closed on `VDD_3V3` before
/// the inward escape, K1), and every renaming routes to its copper. Its first
/// emit is also F2's reproducer: with a search's sources unpriced, as before
/// this work order, the router's exact judge refuses D3's own copper (a GND
/// and a `VDD_3V3` F.Cu end 0.395 mm apart; FEEDBACK). DRC with parity is
/// tier 2's (`with_tools.rs`).
#[test]
fn renamings_0_to_9_route_d3_to_its_own_copper() {
    one_geometry("d3a", D3, 0..10);
}

#[test]
fn renamings_10_to_19_route_d3_to_its_own_copper() {
    one_geometry("d3b", D3, 10..20);
}

/// Ruled K3: the exact reproducers of `1102` A6 (`FLASH_CS` renamed `nCS`,
/// `cs`, `Z`, `a`, `~`, `_X`, `U`; the six bus nets renamed `N0`…`N5` in the
/// reference's own order, §4 deviation 4) and of `2204` A5 (the references
/// `Peripherals` and `Sysbus_` on the first I2C and the first SPI device):
/// each gave `Unroutable` on `main` before this work order, and each routes.
#[test]
fn the_reproducers_of_1102_a6_and_2204_a5_route() {
    let base = fs::read_to_string(root().join(SPEC)).unwrap();
    let dir = scratch("repro");
    let mut specs: Vec<(String, String)> = Vec::new();
    for n in ["nCS", "cs", "Z", "a", "~", "_X", "U"] {
        let to = serde_json::to_string(n).unwrap();
        specs.push((format!("A6 {n}"), base.replace("\"FLASH_CS\"", &to)));
    }
    let mut six = base.clone();
    for (i, n) in [
        "I2C_SDA", "I2C_SCL", "SPI_SCK", "SPI_MOSI", "SPI_MISO", "FLASH_CS",
    ]
    .iter()
    .enumerate()
    {
        assert!(six.contains(&format!("\"{n}\"")), "{n}");
        six = six.replace(&format!("\"{n}\""), &format!("\"N{i}\""));
    }
    specs.push(("A6 N0-N5".into(), six));
    for key in ["i2c_devices", "spi_devices"] {
        for n in ["Peripherals", "Sysbus_"] {
            let mut v: Value = serde_json::from_str(&base).unwrap();
            v[key][0]["reference"] = Value::from(n);
            specs.push((format!("A5 {key} {n}"), v.to_string()));
        }
    }
    for (i, (label, text)) in specs.iter().enumerate() {
        let case = dir.join(format!("case{i}"));
        fs::create_dir_all(&case).unwrap();
        let path = case.join("spec.json");
        fs::write(&path, text).unwrap();
        let (code, v, _) = emit(&case, path.to_str().unwrap());
        assert_eq!(code, 0, "{label}: {v}");
    }
    fs::remove_dir_all(&dir).unwrap();
}

/// `nm` as KiCad writes millimetres.
fn mm(nm: i64) -> String {
    format!("{}.{:06}", nm / 1_000_000, nm % 1_000_000)
}

/// The text of `board` with the first zone's `pts` block replaced by
/// `points`, or with `extra` added after its polygon.
fn with_zone(board: &str, points: Option<&[(i64, i64)]>, extra: &str) -> String {
    let zone = board.find("\t(zone\n").unwrap();
    let pts = zone + board[zone..].find("\t\t\t(pts\n").unwrap();
    let end = pts + board[pts..].find("\t\t\t)\n").unwrap() + "\t\t\t)\n".len();
    let mut out = board[..pts].to_owned();
    match points {
        Some(points) => {
            out.push_str("\t\t\t(pts\n");
            for (x, y) in points {
                out.push_str(&format!("\t\t\t\t(xy {} {})\n", mm(*x), mm(*y)));
            }
            out.push_str("\t\t\t)\n");
        }
        None => out.push_str(&board[pts..end]),
    }
    let close = end + board[end..].find("\t\t)\n").unwrap() + "\t\t)\n".len();
    out.push_str(&board[end..close]);
    out.push_str(extra);
    out.push_str(&board[close..]);
    out
}

/// Ruled K6 (4100 F7): each zone has exactly one `polygon` whose four points
/// are the `Edge.Cuts` rectangle's corners, in any cyclic order and
/// direction; a smaller, larger, offset, holed or extra-vertex outline, or a
/// diagonal walk of the corners, is refused by name; a `filled_polygon` is
/// not read. On the 4-layer fixture's GND plane, and the reference's B.Cu
/// zone.
#[test]
fn a_zone_that_is_not_the_board_outline_is_refused_by_name() {
    for (spec, refusal) in [
        (FOUR, "zone `GND` on In1.Cu: its outline is not the board's"),
        (SPEC, "zone `GND` on B.Cu: its outline is not the board's"),
    ] {
        let (board, _, text) = emitted("zone", spec);
        let o = board.outline;
        let (x0, y0, x1, y1) = (o.x0, o.y0, o.x1, o.y1);
        const MM: i64 = 1_000_000;
        let read = |t: &str| route::read(&strict::board(t).unwrap(), &stack_of(spec));
        // The emitter's own order, and every rotation and the reverse walk.
        let ring = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)];
        for start in 0..4 {
            for reverse in [false, true] {
                let mut pts: Vec<_> = (0..4).map(|i| ring[(start + i) % 4]).collect();
                if reverse {
                    pts.reverse();
                }
                read(&with_zone(&text, Some(&pts), "")).unwrap();
            }
        }
        let poly = |head: &str, pts: [(i64, i64); 3]| {
            let mut t = format!("\t\t({head}\n\t\t\t(pts\n");
            for (x, y) in pts {
                t.push_str(&format!("\t\t\t\t(xy {} {})\n", mm(x), mm(y)));
            }
            t + "\t\t\t)\n\t\t)\n"
        };
        let fill = poly("filled_polygon", [(x0, y0), (x1, y0), (x1, y1)]);
        read(&with_zone(&text, None, &fill)).unwrap();
        let wrong: [(&str, Vec<(i64, i64)>); 7] = [
            (
                "smaller",
                vec![(x0 + MM, y0), (x1, y0), (x1, y1), (x0 + MM, y1)],
            ),
            (
                "larger",
                vec![(x0, y0 - MM), (x1, y0 - MM), (x1, y1), (x0, y1)],
            ),
            (
                "offset",
                vec![(x0 + MM, y0), (x1 + MM, y0), (x1 + MM, y1), (x0 + MM, y1)],
            ),
            ("diagonal", vec![(x0, y0), (x1, y1), (x1, y0), (x0, y1)]),
            (
                "extra vertex",
                vec![(x0, y0), ((x0 + x1) / 2, y0), (x1, y0), (x1, y1), (x0, y1)],
            ),
            ("three corners", vec![(x0, y0), (x1, y0), (x1, y1)]),
            (
                "a corner twice",
                vec![(x0, y0), (x1, y0), (x1, y1), (x1, y0)],
            ),
        ];
        for (label, pts) in &wrong {
            let e = read(&with_zone(&text, Some(pts), "")).unwrap_err();
            assert_eq!(e, refusal, "{label}");
        }
        let hole = poly(
            "polygon",
            [
                (x0 + 5 * MM, y0 + 5 * MM),
                (x0 + 6 * MM, y0 + 5 * MM),
                (x0 + 6 * MM, y0 + 6 * MM),
            ],
        );
        assert_eq!(
            read(&with_zone(&text, None, &hole)).unwrap_err(),
            refusal,
            "holed"
        );
    }
}

/// Ruled K1: a via inside the dense part's own ring interior is admitted;
/// a via whose copper enters any part's courtyard elsewhere is refused by
/// `route::check`, the exact judge the router and `extract` run, naming the
/// part. The rule is copper, exactly: a via tangent to the courtyard passes,
/// 1 nm nearer fails.
#[test]
fn a_via_in_a_courtyard_is_refused_but_in_the_dense_parts_own_ring() {
    let (board, _, _) = emitted("courtyard", SPEC);
    let part = |r: &str| board.parts.iter().find(|p| p.reference == r).unwrap();
    let only = |at: (i64, i64)| Routed {
        vias: vec![Via {
            net: "GND".into(),
            at,
        }],
        ..Routed::default()
    };
    // U1, the LQFP-144, is the dense part; U4, the SOIC-8, is not.
    let ring = route::ring_interior(part("U1")).expect("U1 is the dense part");
    assert_eq!(route::ring_interior(part("U4")), None);
    let (cx, cy) = ((ring.x0 + ring.x1) / 2, (ring.y0 + ring.y1) / 2);
    route::check(&board, &only((cx, cy))).unwrap();
    // U4's courtyard: its body's centre, and its right edge at the pads' row.
    let u4 = part("U4");
    let xs = u4.courtyard.iter().flat_map(|&(a, b)| [a.0, b.0]);
    let (left, right) = (xs.clone().min().unwrap(), xs.max().unwrap());
    let ys = u4.courtyard.iter().flat_map(|&(a, b)| [a.1, b.1]);
    let mid = (ys.clone().min().unwrap() + ys.max().unwrap()) / 2;
    let centre = ((left + right) / 2, mid);
    let e = route::check(&board, &only(centre)).unwrap_err();
    assert!(e.contains("is in U4's courtyard"), "{e}");
    let tangent = (right + route::VIA_NM / 2, mid);
    route::check(&board, &only(tangent)).unwrap();
    let e = route::check(&board, &only((tangent.0 - 1, mid))).unwrap_err();
    assert!(e.contains("is in U4's courtyard"), "{e}");
}
