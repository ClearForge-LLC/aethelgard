//! `AET-WO-1308`, tier 1 (no tool runs): net classes, coupled pairs and part
//! rotation in the router. The 4-layer `power` fixture is laid at its class's
//! width and via, and the reader refuses copper that is not its class's; a
//! class too wide for the LQFP is refused with the pin; the stand-in pair is
//! routed coupled and measured from copper, a pair that cannot be coupled is
//! refused by name, and the pair judge refuses each of §5.2's breakages; a part
//! on each board edge, rotated, routes and reads back where KiCad puts it.

mod common;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use aethelgard_eda::route::{self, Layer, Routed, Segment, Via};
use aethelgard_eda::stack::Stack;
use aethelgard_eda::strict;
use serde_json::{Value, json};

const POWER: &str = "crates/aethelgard-cli/tests/fixtures/four-layer-power.json";
const PAIR: &str = "crates/aethelgard-cli/tests/fixtures/pair-standin.json";

fn root() -> PathBuf {
    fs::canonicalize(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")).unwrap()
}

fn scratch(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("aet-1308-{label}-{}", std::process::id()));
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

/// `emit` of `spec` (a path from the root): its exit code, verdict and board.
fn emit(dir: &Path, spec: &str) -> (i32, Value, String) {
    let out = dir.join("out");
    let (code, v) = run(&["emit", "--file", spec, "--out", out.to_str().unwrap()]);
    let board = v["board"]["path"]
        .as_str()
        .map(|p| fs::read_to_string(p).unwrap())
        .unwrap_or_default();
    (code, v, board)
}

fn spec_of(path: &str) -> aethelgard_ir::Specification {
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap();
    spec.validate().unwrap();
    spec
}

/// A specification file written from `v` into `dir`, as a path from the root.
fn write_spec(dir: &Path, name: &str, v: &Value) -> String {
    let path = dir.join(name);
    fs::write(&path, serde_json::to_vec_pretty(v).unwrap()).unwrap();
    path.to_str().unwrap().to_owned()
}

fn json_of(path: &str) -> Value {
    serde_json::from_slice(&fs::read(root().join(path)).unwrap()).unwrap()
}

/// The board read back as every gate reads it, judged by the router's own
/// exact checks.
fn read(board: &str, stack: &Stack) -> (route::Board, Routed) {
    let tree = strict::board(board).unwrap();
    route::declared(&tree, stack).unwrap();
    let (b, r) = route::read(&tree, stack).unwrap();
    route::check(&b, &r).unwrap();
    route::connectivity(&b, &r).unwrap();
    (b, r)
}

#[test]
fn the_power_class_is_laid_at_its_width_and_via_and_nothing_else_is() {
    let dir = scratch("power");
    let (code, v, board) = emit(&dir, POWER);
    assert_eq!(code, 0, "{v}");
    let stack = Stack::of(&spec_of(POWER));
    let (_, routed) = read(&board, &stack);
    let class = ["VDD_3V3", "GND"];
    let (mut ours, mut theirs) = (0, 0);
    for s in &routed.segments {
        if class.contains(&s.net.as_str()) {
            ours += 1;
        } else {
            theirs += 1;
        }
    }
    assert!(ours > 0 && theirs > 0, "{ours} {theirs}");
    // The board's own widths, as written: read() accepted each against its
    // net's rule; here every one is counted from the text.
    let widths = |net: &str| {
        board
            .split("\n\t(segment")
            .skip(1)
            .map(|s| s.split("\n\t)").next().unwrap())
            .filter(|s| s.contains(&format!("(net \"{net}\")")))
            .map(|s| s.contains("(width 0.25)"))
            .collect::<Vec<_>>()
    };
    assert!(widths("VDD_3V3").iter().all(|w| *w));
    assert!(widths("GND").iter().all(|w| *w));
    assert!(widths("/SPI_SCK").iter().all(|w| !*w));
    for v in &routed.vias {
        let via = board
            .split("\n\t(via")
            .skip(1)
            .find(|t| t.contains(&format!("(net \"{}\")", v.net)))
            .unwrap();
        let want = if class.contains(&v.net.as_str()) {
            "(size 0.7)\n\t\t(drill 0.3)"
        } else {
            "(size 0.6)\n\t\t(drill 0.3)"
        };
        assert!(via.contains(want), "{}: {via}", v.net);
    }
    eprintln!(
        "ANSWER 1308 power fixture: {} segments ({ours} classed), {} vias",
        routed.segments.len(),
        routed.vias.len()
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// §5.1 at the reader: a track or via that is not its class's is refused,
/// naming the class, by the specification's rule (never the board's).
#[test]
fn the_reader_refuses_copper_that_is_not_its_class() {
    let dir = scratch("lie");
    let (code, v, board) = emit(&dir, POWER);
    assert_eq!(code, 0, "{v}");
    let stack = Stack::of(&spec_of(POWER));
    let i = board.find("(width 0.25)").unwrap();
    for (from, to, what) in [
        ("(width 0.25)", "(width 0.2)", "segment"),
        ("(width 0.25)", "(width 0.3)", "segment"),
    ] {
        let lied = format!("{}{to}{}", &board[..i], &board[i + from.len()..]);
        let e = route::read(&strict::board(&lied).unwrap(), &stack).unwrap_err();
        assert!(e.contains(what) && e.contains("class `power`"), "{e}");
    }
    let j = board.find("(size 0.7)").unwrap();
    let lied = format!("{}(size 0.6){}", &board[..j], &board[j + 10..]);
    let e = route::read(&strict::board(&lied).unwrap(), &stack).unwrap_err();
    assert!(e.contains("via") && e.contains("class `power`"), "{e}");
    fs::remove_dir_all(&dir).unwrap();
}

/// §5.3: a class too wide to leave the LQFP-144's 0.3 mm pads at 0.5 mm pitch
/// is refused by the router, naming the pin (FEEDBACK F7).
#[test]
fn a_class_too_wide_for_the_lqfp_is_refused_with_the_pin() {
    let dir = scratch("starve");
    let mut v = json_of(POWER);
    v["net_classes"][0]["track_width_um"] = json!(300);
    let path = write_spec(&dir, "wide.json", &v);
    let (code, out, _) = emit(&dir, &path);
    assert_eq!(code, 2, "{out}");
    assert_eq!(out["error"]["kind"], "Unroutable", "{out}");
    let m = out["error"]["message"].as_str().unwrap();
    assert!(m.contains("U1 pad"), "{m}");
    eprintln!("ANSWER 1308 starve 300 um: {m}");
    fs::remove_dir_all(&dir).unwrap();
}

/// §1.3: the stand-in pair routes coupled on F.Cu, and the judge measures it
/// from the board's copper alone.
#[test]
fn the_stand_in_pair_is_routed_coupled_and_measured() {
    let dir = scratch("pair");
    let (code, v, board) = emit(&dir, PAIR);
    assert_eq!(code, 0, "{v}");
    let stack = Stack::of(&spec_of(PAIR));
    let (b, r) = read(&board, &stack);
    let reports = route::pairs(&b, &r).unwrap();
    let [p] = &reports[..] else {
        panic!("one pair")
    };
    assert_eq!(p.name, "USB_STANDIN");
    assert_eq!(p.nets, ["/SPI_SCK".to_owned(), "/SPI_MOSI".to_owned()]);
    assert_eq!(p.vias, [0, 0]);
    assert_eq!(p.layers, ["F.Cu"]);
    assert!(p.length_difference_nm() <= route::PAIR_SKEW_NM, "{p:?}");
    assert!(
        p.uncoupled_nm
            .iter()
            .flatten()
            .all(|&u| u <= route::PAIR_BREAKOUT_NM),
        "{p:?}"
    );
    eprintln!("ANSWER 1308 pair: {p:?}");
    fs::remove_dir_all(&dir).unwrap();
}

/// A pair the router cannot couple is refused by name, never routed apart:
/// its polarity reversed (the breakouts would cross), and a spacing that is
/// not a whole number of grid pitches.
#[test]
fn a_pair_that_cannot_be_coupled_is_refused_by_name() {
    let dir = scratch("uncouplable");
    let mut v = json_of(PAIR);
    v["diff_pairs"][0]["nets"] = json!(["SPI_MOSI", "SPI_SCK"]);
    let path = write_spec(&dir, "reversed.json", &v);
    let (code, out, _) = emit(&dir, &path);
    assert_eq!(code, 2, "{out}");
    assert_eq!(out["error"]["kind"], "Unroutable", "{out}");
    assert!(
        out["error"]["message"]
            .as_str()
            .unwrap()
            .contains("pair `USB_STANDIN`"),
        "{out}"
    );
    let mut v = json_of(PAIR);
    v["diff_pairs"][0]["gap_um"] = json!(310);
    let path = write_spec(&dir, "offgrid.json", &v);
    let (code, out, _) = emit(&dir, &path);
    assert_eq!(code, 2, "{out}");
    let m = out["error"]["message"].as_str().unwrap();
    assert!(
        m.contains("pair `USB_STANDIN`") && m.contains("whole number"),
        "{m}"
    );
    fs::remove_dir_all(&dir).unwrap();
}

/// The conductor's longest straight piece on F.Cu.
fn longest(r: &Routed, net: &str) -> usize {
    (0..r.segments.len())
        .filter(|&i| r.segments[i].net == net && r.segments[i].layer == Layer::F)
        .max_by_key(|&i| {
            let s = &r.segments[i];
            ((s.end.0 - s.start.0).abs() + (s.end.1 - s.start.1).abs(), i)
        })
        .unwrap()
}

/// `segment`'s piece replaced by a jog `out` across it, over its middle third.
fn jog(r: &mut Routed, i: usize, out: (i64, i64)) {
    let s = r.segments.remove(i);
    let third = ((s.end.0 - s.start.0) / 3, (s.end.1 - s.start.1) / 3);
    let a = (s.start.0 + third.0, s.start.1 + third.1);
    let b = (s.end.0 - third.0, s.end.1 - third.1);
    let (a2, b2) = ((a.0 + out.0, a.1 + out.1), (b.0 + out.0, b.1 + out.1));
    for (p, q) in [(s.start, a), (a, a2), (a2, b2), (b2, b), (b, s.end)] {
        r.segments.push(Segment {
            net: s.net.clone(),
            layer: s.layer,
            start: p,
            end: q,
        });
    }
}

/// `segment` `i` cut in `2·hops + 1` equal pieces, every other one moved to
/// B.Cu through a via at each change.
fn hop(r: &mut Routed, i: usize, hops: i64) {
    let s = r.segments.remove(i);
    let n = 2 * hops + 1;
    let at = |k: i64| {
        (
            s.start.0 + (s.end.0 - s.start.0) * k / n,
            s.start.1 + (s.end.1 - s.start.1) * k / n,
        )
    };
    for k in 0..n {
        r.segments.push(Segment {
            net: s.net.clone(),
            layer: if k % 2 == 1 { Layer::B } else { Layer::F },
            start: at(k),
            end: at(k + 1),
        });
        if k > 0 {
            r.vias.push(Via {
                net: s.net.clone(),
                at: at(k),
            });
        }
    }
    r.segments.sort();
    r.vias.sort();
}

/// §5.2: the pair judge refuses each breakage of the routed pair, by name:
/// vias on one conductor only, a gap that widens, a conductor detoured.
#[test]
fn the_pair_judge_refuses_each_breakage() {
    let dir = scratch("break");
    let (code, v, board) = emit(&dir, PAIR);
    assert_eq!(code, 0, "{v}");
    let stack = Stack::of(&spec_of(PAIR));
    let (b, routed) = read(&board, &stack);
    let (pos, neg) = ("/SPI_SCK", "/SPI_MOSI");
    // The negative conductor stands across from the positive: the jog goes
    // away from it, perpendicular to the piece.
    let away = |r: &Routed, i: usize, d: i64| {
        let s = &r.segments[i];
        let other = r.segments.iter().find(|o| {
            o.net != s.net && o.layer == s.layer && (o.start.1 == o.end.1) == (s.start.1 == s.end.1)
        });
        let side = other.map_or(1, |o| {
            if s.start.1 == s.end.1 {
                (s.start.1 - o.start.1).signum()
            } else {
                (s.start.0 - o.start.0).signum()
            }
        });
        if s.start.1 == s.end.1 {
            (0, side * d)
        } else {
            (side * d, 0)
        }
    };
    // Vias on one conductor only: it hops to B.Cu and back, so its layers
    // differ. Then both hop, one twice: the same layers, unequal vias.
    let mut r = routed.clone();
    hop(&mut r, longest(&routed, pos), 1);
    let e = route::pairs(&b, &r).unwrap_err();
    assert!(
        e.contains("pair `USB_STANDIN`") && e.contains("run on"),
        "{e}"
    );
    let mut r = routed.clone();
    let (ip, in_) = (longest(&routed, pos), longest(&routed, neg));
    let (sp, sn) = (routed.segments[ip].clone(), routed.segments[in_].clone());
    r.segments.retain(|s| *s != sp && *s != sn);
    for (s, hops) in [(sp, 1), (sn, 2)] {
        let i = r.segments.len();
        r.segments.push(s);
        hop(&mut r, i, hops);
    }
    let e = route::pairs(&b, &r).unwrap_err();
    assert!(
        e.contains("pair `USB_STANDIN`") && e.contains("has 2 vias") && e.contains("has 4"),
        "{e}"
    );
    // The gap widens by a pitch over a third of the longest piece.
    let mut r = routed.clone();
    let i = longest(&r, neg);
    let out = away(&r, i, route::PITCH_NM);
    jog(&mut r, i, out);
    r.segments.sort();
    let e = route::pairs(&b, &r).unwrap_err();
    assert!(
        e.contains("pair `USB_STANDIN`") && e.contains("breaks"),
        "{e}"
    );
    // One conductor detoured 3 mm out and back.
    let mut r = routed.clone();
    let i = longest(&r, pos);
    let out = away(&r, i, 3_000_000);
    jog(&mut r, i, out);
    r.segments.sort();
    let e = route::pairs(&b, &r).unwrap_err();
    assert!(e.contains("pair `USB_STANDIN`"), "{e}");
    eprintln!("ANSWER 1308 detour: {e}");
    fs::remove_dir_all(&dir).unwrap();
}

/// `route::rotate` is KiCad's rule, measured (FEEDBACK §3): the reference's
/// U4 (SOIC-8) at (82.55, 108.4) rotated 90 put pad 1, library (-3.5875,
/// -1.905), at (80.645, 111.989) in KiCad's IPC-D-356 export (3.1750 in,
/// -4.4090 in, Y up), and at 270 at (84.455, 104.813).
#[test]
fn rotation_is_kicads_as_measured() {
    let origin = (82_550_000, 108_400_000);
    for (theta, want) in [
        (90, (80_645_000, 111_987_500)),
        (270, (84_455_000, 104_812_500)),
    ] {
        let (x, y) = route::rotate(theta, (-3_587_500, -1_905_000));
        let at = (origin.0 + x, origin.1 + y);
        // The export rounds to 0.1 mil (2.54 µm).
        assert!(
            (at.0 - want.0).abs() <= 2_540 && (at.1 - want.1).abs() <= 2_540,
            "{theta}: {at:?}"
        );
    }
    assert_eq!(route::rotate(180, (3, 4)), (-3, -4));
    assert_eq!(route::rotate(0, (3, 4)), (3, 4));
}

#[test]
fn a_part_on_each_edge_is_routed_and_read_back_where_kicad_puts_it() {
    let (spec, emitted) = common::edges::moved();
    let board = emitted.board;
    let stack = Stack::of(&spec);
    let (b, r) = read(&board, &stack);
    assert!(!r.segments.is_empty());
    for (reference, theta) in [("U2", "90"), ("R1", "180"), ("U4", "270")] {
        let fp = board
            .split("\n\t(footprint")
            .find(|f| f.contains(&format!("(property \"Reference\" \"{reference}\"")))
            .unwrap();
        assert!(
            fp.contains(&format!(" {theta})")),
            "{reference}: no angle {theta}"
        );
    }
    // U4 at 270: pad 1, library (-3.5875, -1.905), lands at its origin plus
    // (1.905, -3.5875); its own angle is the rotation (absolute in the file).
    let u4 = b.parts.iter().find(|p| p.reference == "U4").unwrap();
    let fp = board
        .split("\n\t(footprint")
        .find(|f| f.contains("(property \"Reference\" \"U4\""))
        .unwrap();
    let at: Vec<i64> = fp
        .lines()
        .find(|l| l.trim_start().starts_with("(at "))
        .unwrap()
        .trim()
        .trim_start_matches("(at ")
        .trim_end_matches(')')
        .split(' ')
        .take(2)
        .map(|n| (n.parse::<f64>().unwrap() * 1e6).round() as i64)
        .collect();
    let pad1 = u4.pads.iter().find(|p| p.number == "1").unwrap();
    let c = (
        (pad1.rect.x0 + pad1.rect.x1) / 2,
        (pad1.rect.y0 + pad1.rect.y1) / 2,
    );
    assert_eq!(c, (at[0] + 1_905_000, at[1] - 3_587_500));
    assert!(fp.contains("(at -3.5875 -1.905 270)"), "pad 1's own angle");
    // The pad's box turned with it: 1.625 x 0.65 mm, long in y at 270.
    assert_eq!(pad1.rect.y1 - pad1.rect.y0, 1_625_000);
}

/// A net in two classes is refused by `validate` with exit 2, by name.
#[test]
fn validate_refuses_a_net_in_two_classes() {
    let dir = scratch("two");
    let mut v = json_of(POWER);
    v["net_classes"] = json!([
        {"name": "a", "track_width_um": 250, "via": null, "nets": ["VDD_3V3"]},
        {"name": "b", "track_width_um": 250, "via": null, "nets": ["VDD_3V3"]}
    ]);
    let path = write_spec(&dir, "two.json", &v);
    let (code, out) = run(&["validate", "--file", &path]);
    assert_eq!(code, 2, "{out}");
    assert_eq!(out["error"]["kind"], "NetInTwoClasses", "{out}");
    fs::remove_dir_all(&dir).unwrap();
}
