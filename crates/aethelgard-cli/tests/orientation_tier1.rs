//! `AET-WO-4400d`, tier 1: the IMU's orientation, declared once. `solve`
//! copies it (solved 10); the emitted board's IMU footprint holds the part's
//! cited axes (`parts.json` `axes`, by pins) where the specification
//! declares them, for every forward edge and every top-side orientation; and
//! an orientation no footprint on F.Cu reaches is refused by name.
//!
//! The board is read back from the emitted file alone: the footprint's
//! `(at x y θ)` and its pads' own positions, turned as KiCad turns them
//! (`classes_tier1`'s `rotation_is_kicads_as_measured`), never through the
//! placer's code.

use std::path::PathBuf;

use aethelgard_eda::schematic::{self, EmitError};
use aethelgard_eda::strict;
use aethelgard_ir::orientation::{Orientation, Toward};
use aethelgard_ir::{PinTable, SolvedTopology, Specification};
use serde_json::{Value, json};

const FIXTURE: &str = "crates/aethelgard-cli/tests/fixtures/sentinel-02-own.json";

fn root() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../.."))
}

fn fixture() -> Value {
    serde_json::from_str(&std::fs::read_to_string(root().join(FIXTURE)).unwrap()).unwrap()
}

fn solved(v: &Value) -> (Specification, SolvedTopology, PinTable) {
    let spec: Specification = serde_json::from_value(v.clone()).unwrap();
    spec.validate().unwrap();
    let table = aethelgard_svd_hal::pin_table(&spec.mcu.part).unwrap();
    let solved = aethelgard_ir::solve(&spec, &table, &"0".repeat(64)).unwrap();
    (spec, solved, table)
}

/// The fixture with `board.forward` and U5's orientation replaced.
fn declared(forward: &str, x: &str, z: &str) -> Value {
    let mut v = fixture();
    v["board"]["forward"] = json!(forward);
    let u5 = v["spi_devices"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|d| d["reference"] == "U5")
        .unwrap();
    u5["orientation"] = json!({"x": x, "z": z});
    v
}

/// KiCad's turn of a footprint's own coordinates, counter-clockwise on the
/// board with y down (measured, `AET-WO-1308` P5).
fn kicad_turn(theta: u16, (x, y): (f64, f64)) -> (f64, f64) {
    match theta {
        0 => (x, y),
        90 => (y, -x),
        180 => (-x, -y),
        270 => (-y, x),
        t => panic!("a turn of {t}"),
    }
}

/// U5's turn, and its chip's X and Y on the board (unit, y down) as its pads
/// place them: from the pads' mean to the mean of the pins `parts.json`
/// names for each axis.
fn u5_axes(board: &str) -> (u16, (i8, i8), (i8, i8)) {
    let tree = strict::board(board).unwrap();
    let fp = tree
        .root()
        .all("footprint")
        .find(|f| f.field("Reference").unwrap() == Some("U5"))
        .unwrap();
    let at = fp.one("at").unwrap().unwrap();
    let theta: u16 = at.arg(3).unwrap_or("0").parse().unwrap();
    let mut pads = std::collections::BTreeMap::new();
    for pad in fp.all("pad") {
        let a = pad.one("at").unwrap().unwrap();
        let local = (
            a.arg(1).unwrap().parse::<f64>().unwrap(),
            a.arg(2).unwrap().parse::<f64>().unwrap(),
        );
        pads.insert(pad.arg(1).unwrap().to_owned(), kicad_turn(theta, local));
    }
    let mean = |names: &[String]| {
        let n = names.len() as f64;
        let (x, y) = names
            .iter()
            .map(|p| pads[p])
            .fold((0.0, 0.0), |s, p| (s.0 + p.0, s.1 + p.1));
        (x / n, y / n)
    };
    let all: Vec<String> = pads.keys().cloned().collect();
    let c = mean(&all);
    let axes = aethelgard_ir::parts::table()
        .unwrap()
        .get("ICM42688P")
        .unwrap()
        .axes
        .clone()
        .unwrap();
    let unit = |pins: &[String]| {
        let (x, y) = mean(pins);
        let (dx, dy) = (x - c.0, y - c.1);
        let s = |v: f64| if v.abs() < 1e-6 { 0 } else { v.signum() as i8 };
        (s(dx), s(dy))
    };
    (theta, unit(&axes.x_toward_pins), unit(&axes.y_toward_pins))
}

/// Where a declared orientation puts the chip's X and Y on the board file
/// (unit, y down, seen from F.Cu), with the board frame x forward, y right,
/// z down through F.Cu (ruled D3): Y = Z × X in the board frame, then each
/// board vector drawn as `a·forward + b·right`.
fn expected(forward: &str, o: Orientation) -> ((i8, i8), (i8, i8)) {
    let body = |d: Toward| -> [i8; 3] {
        match d {
            Toward::Forward => [1, 0, 0],
            Toward::Aft => [-1, 0, 0],
            Toward::Right => [0, 1, 0],
            Toward::Left => [0, -1, 0],
            Toward::Down => [0, 0, 1],
            Toward::Up => [0, 0, -1],
        }
    };
    let (x, z) = (body(o.x), body(o.z));
    let y = [
        z[1] * x[2] - z[2] * x[1],
        z[2] * x[0] - z[0] * x[2],
        z[0] * x[1] - z[1] * x[0],
    ];
    let f: (i8, i8) = match forward {
        "top" => (0, -1),
        "right" => (1, 0),
        "bottom" => (0, 1),
        "left" => (-1, 0),
        e => panic!("{e}"),
    };
    // Right is forward turned clockwise as seen from F.Cu (y down).
    let r = (-f.1, f.0);
    let draw = |v: [i8; 3]| (v[0] * f.0 + v[1] * r.0, v[0] * f.1 + v[1] * r.1);
    (draw(x), draw(y))
}

/// `solve` copies U5's orientation, and nothing else gains one.
#[test]
fn solve_copies_the_declared_orientation() {
    let (_, solved, _) = solved(&fixture());
    assert_eq!(
        solved.spi_devices["U5"].orientation,
        Some(Orientation {
            x: Toward::Right,
            z: Toward::Up
        })
    );
    assert_eq!(solved.spi_devices["U4"].orientation, None);
    assert_eq!(solved.schema_version, 10);
    // Written only where declared; an explicit null and a version-9 file are
    // refused (solved 10).
    let mut v = serde_json::to_value(&solved).unwrap();
    assert_eq!(v.to_string().matches("\"orientation\"").count(), 1);
    v["spi_devices"]["U5"]["orientation"] = Value::Null;
    assert!(serde_json::from_value::<SolvedTopology>(v).is_err());
    let mut old = solved.clone();
    old.schema_version = 9;
    assert_eq!(
        old.validate().unwrap_err().kind(),
        "UnsupportedSchemaVersion"
    );
    let mut skew = solved;
    skew.spi_devices.get_mut("U5").unwrap().orientation = Some(Orientation {
        x: Toward::Up,
        z: Toward::Down,
    });
    let e = skew.validate().unwrap_err();
    assert_eq!((e.kind(), e.id()), ("ImuAxesNotPerpendicular", Some("U5")));
    // A hand-written orientation on the flash is refused, not ignored.
    let mut stray = skew.clone();
    stray.spi_devices.get_mut("U5").unwrap().orientation = None;
    stray.spi_devices.get_mut("U4").unwrap().orientation = Some(Orientation {
        x: Toward::Right,
        z: Toward::Up,
    });
    let e = stray.validate().unwrap_err();
    assert_eq!((e.kind(), e.id()), ("OrientationOnNonImu", Some("U4")));
}

/// `AET-WO-4400d` §2: the emitted board's IMU footprint holds the part's
/// cited axes where the specification declares them: `sentinel-02-own` at 0
/// degrees (ruled D5), and each other top-side orientation, on each forward
/// edge, at the turn it needs, its keep-out turned with it.
#[test]
fn the_placed_imu_agrees_with_the_declared_orientation() {
    let cases = [
        ("top", "right", 0),
        ("top", "forward", 90),
        ("top", "left", 180),
        ("top", "aft", 270),
        ("right", "right", 270),
        ("bottom", "forward", 270),
        ("left", "aft", 0),
    ];
    for (forward, x, want_theta) in cases {
        let v = declared(forward, x, "up");
        let (spec, solved, table) = solved(&v);
        let o = solved.spi_devices["U5"].orientation.unwrap();
        let board = schematic::emit(&spec, &solved, &table).unwrap().board;
        let (theta, x_on_board, y_on_board) = u5_axes(&board);
        assert_eq!(
            (x_on_board, y_on_board),
            expected(forward, o),
            "{forward} {x}"
        );
        assert_eq!(theta, want_theta, "{forward} {x}");
        // The DRC gate's own reading agrees (`route::declared`).
        let stack = aethelgard_eda::stack::Stack::of(&spec);
        aethelgard_eda::route::declared(&strict::board(&board).unwrap(), &stack)
            .unwrap_or_else(|d| panic!("{forward} {x}: {}: {}", d.kind, d.message));
        // The F.Cu keep-out is the pad ring's, turned: 1.85 x 1.35 mm about
        // the origin at 0, its sides swapped at a quarter turn.
        let tree = strict::board(&board).unwrap();
        let fp = tree
            .root()
            .all("footprint")
            .find(|f| f.field("Reference").unwrap() == Some("U5"))
            .unwrap();
        let at = fp.one("at").unwrap().unwrap();
        let origin: (f64, f64) = (
            at.arg(1).unwrap().parse().unwrap(),
            at.arg(2).unwrap().parse().unwrap(),
        );
        let zone = fp
            .all("zone")
            .find(|z| z.one("layer").unwrap().unwrap().arg(1) == Some("F.Cu"))
            .unwrap();
        let pts: Vec<(f64, f64)> = zone
            .one("polygon")
            .unwrap()
            .unwrap()
            .one("pts")
            .unwrap()
            .unwrap()
            .all("xy")
            .map(|p| {
                (
                    p.arg(1).unwrap().parse::<f64>().unwrap() - origin.0,
                    p.arg(2).unwrap().parse::<f64>().unwrap() - origin.1,
                )
            })
            .collect();
        let span = |f: fn(&(f64, f64)) -> f64| {
            let (lo, hi) = pts
                .iter()
                .map(f)
                .fold((f64::MAX, f64::MIN), |(l, h), v| (l.min(v), h.max(v)));
            ((hi - lo) * 1000.0).round() as i64
        };
        let (w, h) = (span(|p| p.0), span(|p| p.1));
        let quarter = theta % 180 == 90;
        assert_eq!(
            (w, h),
            if quarter { (1350, 1850) } else { (1850, 1350) },
            "{forward} {x}"
        );
        let mid: (f64, f64) = pts
            .iter()
            .fold((0.0, 0.0), |s, p| (s.0 + p.0 / 4.0, s.1 + p.1 / 4.0));
        assert!(
            mid.0.abs() < 1e-6 && mid.1.abs() < 1e-6,
            "{forward} {x}: {mid:?}"
        );
    }
}

/// An orientation no footprint on F.Cu reaches is refused by the placer by
/// name (`ImuRotationUnplaceable`), never rounded to one it does: Z down
/// needs B.Cu, and X up or down stands the part on its edge.
#[test]
fn an_orientation_off_the_top_side_is_refused_by_name() {
    for (x, z) in [
        ("right", "down"),
        ("forward", "down"),
        ("up", "forward"),
        ("down", "left"),
    ] {
        let (spec, solved, table) = solved(&declared("top", x, z));
        let e = schematic::emit(&spec, &solved, &table).unwrap_err();
        assert!(
            matches!(e, EmitError::ImuRotationUnplaceable { .. }),
            "{x} {z}: {e}"
        );
        assert_eq!(
            (e.kind(), e.id()),
            ("ImuRotationUnplaceable", Some("U5")),
            "{e}"
        );
        assert!(
            e.to_string().contains(x) && e.to_string().contains(z),
            "{e}"
        );
    }
}

/// `board` with U5's footprint text edited by `edit`.
fn edited_u5(board: &str, edit: impl Fn(&str) -> String) -> String {
    let parts: Vec<&str> = board.split("\n\t(footprint").collect();
    let u5 = parts
        .iter()
        .position(|fp| fp.contains("(property \"Reference\" \"U5\""))
        .unwrap();
    let mut out = parts[0].to_owned();
    for (i, fp) in parts.iter().enumerate().skip(1) {
        out.push_str("\n\t(footprint");
        out.push_str(&if i == u5 { edit(fp) } else { (*fp).to_owned() });
    }
    out
}

/// A footprint's text with an angle added to its own `(at x y)`.
fn turned(fp: &str, theta: &str) -> String {
    let i = fp.find("\n\t\t(at ").unwrap();
    let j = i + fp[i..].find(')').unwrap();
    format!("{} {theta}{}", &fp[..j], &fp[j..])
}

/// `AET-WO-4400d` §5: the board and the firmware disagree when a footprint is
/// turned by hand after placement. `check drc --spec` holds the board's IMU
/// footprint to the declaration before any tool runs, and refuses one turned,
/// mirrored onto B.Cu, or missing, by name (`ImuOrientationDisagrees`).
#[test]
fn a_hand_turned_imu_footprint_is_refused_by_the_drc_gate() {
    let v = fixture();
    let (spec, solved, table) = solved(&v);
    let board = schematic::emit(&spec, &solved, &table).unwrap().board;
    let stack = aethelgard_eda::stack::Stack::of(&spec);
    let judge = |b: &str| aethelgard_eda::route::declared(&strict::board(b).unwrap(), &stack);
    judge(&board).unwrap();
    // U5 is at 0 degrees: turned a quarter, a half, three quarters.
    for theta in ["90", "180", "270"] {
        let hand = edited_u5(&board, |fp| turned(fp, theta));
        assert_ne!(hand, board);
        let d = judge(&hand).unwrap_err();
        assert_eq!(d.kind, "ImuOrientationDisagrees", "{theta}: {}", d.message);
        assert!(d.message.starts_with("U5: "), "{}", d.message);
    }
    let mirrored = edited_u5(&board, |fp| {
        fp.replacen("(layer \"F.Cu\")", "(layer \"B.Cu\")", 1)
    });
    assert_ne!(mirrored, board);
    assert_eq!(
        judge(&mirrored).unwrap_err().kind,
        "ImuOrientationDisagrees"
    );
    // The board unchanged, judged against a declaration no footprint on
    // F.Cu can meet: Z down, or X up.
    for (x, z) in [("right", "down"), ("up", "forward")] {
        let other: Specification = serde_json::from_value(declared("top", x, z)).unwrap();
        let d = aethelgard_eda::route::declared(
            &strict::board(&board).unwrap(),
            &aethelgard_eda::stack::Stack::of(&other),
        )
        .unwrap_err();
        assert_eq!(d.kind, "ImuOrientationDisagrees", "{x} {z}: {}", d.message);
    }
    let renamed = edited_u5(&board, |fp| fp.replacen("\"U5\"", "\"U50\"", 1));
    assert_eq!(judge(&renamed).unwrap_err().kind, "ImuOrientationDisagrees");
    // Through the CLI: refused by name before any tool is looked for.
    let dir = root()
        .join("target")
        .join(format!("aet-4400d-turned-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let b = dir.join("b.kicad_pcb");
    std::fs::write(&b, edited_u5(&board, |fp| turned(fp, "90"))).unwrap();
    let out = std::process::Command::new(env!("CARGO_BIN_EXE_aethelgard"))
        .args([
            "check",
            "drc",
            "--board",
            b.to_str().unwrap(),
            "--spec",
            FIXTURE,
        ])
        .current_dir(root())
        .env("PATH", "/nonexistent")
        .stdin(std::process::Stdio::null())
        .output()
        .unwrap();
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    assert_eq!(v["error"]["kind"], "ImuOrientationDisagrees", "{v}");
    let _ = std::fs::remove_dir_all(&dir);
}
