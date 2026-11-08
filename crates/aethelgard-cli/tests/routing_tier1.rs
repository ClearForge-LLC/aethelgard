//! `aethelgard emit`'s routed board, without KiCad (`AET-WO-1302` §1.4,
//! tier 1). Every check reads the board back from the file `emit` wrote,
//! never the router's own memory. That KiCad's gate passes it is tier 2
//! (`with_tools.rs`).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use aethelgard_eda::route::{self, Layer};
use aethelgard_eda::sexpr;
use serde_json::Value;

fn example(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .join(name)
}

struct Dir(PathBuf);

impl Dir {
    fn new(tag: &str) -> Dir {
        let d = std::env::temp_dir().join(format!(
            "aet-1302-{tag}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&d).unwrap();
        Dir(d)
    }
}

impl Drop for Dir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn cli() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_aethelgard"));
    c.stdin(Stdio::null());
    c
}

fn json_line(out: &Output, code: i32) -> Value {
    let stdout = String::from_utf8(out.stdout.clone()).unwrap();
    assert_eq!(
        stdout.lines().count(),
        1,
        "stdout {stdout} stderr {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let v: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(out.status.code(), Some(code), "{v}");
    v
}

/// The reference specification's emitted board, as text.
fn emitted(d: &Dir) -> String {
    let r = json_line(
        &cli()
            .args(["emit", "--file"])
            .arg(example("sentinel-01-minimal.json"))
            .arg("--out")
            .arg(d.0.join("o"))
            .output()
            .unwrap(),
        0,
    );
    fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap()
}

fn read(board: &str) -> (route::Board, route::Routed) {
    route::read(&sexpr::parse(board).unwrap()).unwrap()
}

/// Two processes, in different environments, working directories and out
/// paths, write the same routed board byte for byte; and the router, run
/// again on the board's own pads, lays exactly the copper the file holds.
#[test]
fn the_routed_board_is_byte_identical_and_a_function_of_its_pads() {
    let d = Dir::new("twice");
    let spec = example("sentinel-01-minimal.json");
    let cwd = d.0.join("cwd");
    fs::create_dir(&cwd).unwrap();
    let mut boards = Vec::new();
    for (out, env, dir) in [
        ("a", vec![("LC_ALL", "C")], d.0.clone()),
        (
            "b/deeper",
            vec![
                ("LC_ALL", "tr_TR.UTF-8"),
                ("TZ", "Pacific/Kiritimati"),
                ("SOURCE_DATE_EPOCH", "1"),
                ("RUST_MIN_STACK", "1048576"),
            ],
            cwd.clone(),
        ),
    ] {
        let o = d.0.join(out);
        let r = json_line(
            &cli()
                .env_clear()
                .envs(env)
                .current_dir(dir)
                .args(["emit", "--file", spec.to_str().unwrap(), "--out"])
                .arg(&o)
                .output()
                .unwrap(),
            0,
        );
        boards.push(fs::read_to_string(r["board"]["path"].as_str().unwrap()).unwrap());
    }
    assert_eq!(boards[0], boards[1], "two emissions of the board differ");
    let (board, written) = read(&boards[0]);
    assert!(!written.segments.is_empty() && !written.vias.is_empty());
    let again = route::route(&board).unwrap();
    assert_eq!(again.segments, written.segments);
    assert_eq!(again.vias, written.vias);
    assert_eq!(again.zones, written.zones);
}

/// The router's own clearance check, over every pair of copper items on the
/// board as written: pads (those with no net included), tracks and vias.
#[test]
fn every_pair_of_copper_items_keeps_the_clearance() {
    let d = Dir::new("clear");
    let (board, routed) = read(&emitted(&d));
    route::check(&board, &routed).unwrap();
}

/// No track or via comes within the clearance of a pad whose net is one of
/// KiCad's `unconnected-…`: such a pad is joined to nothing.
#[test]
fn no_copper_touches_an_unconnected_pad() {
    let d = Dir::new("unconnected");
    let (board, routed) = read(&emitted(&d));
    let lone: Vec<&route::Pad> = board
        .parts
        .iter()
        .flat_map(|p| &p.pads)
        .filter(|p| {
            p.net
                .as_deref()
                .is_some_and(|n| n.starts_with("unconnected-"))
        })
        .collect();
    assert!(lone.len() > 100, "{}", lone.len());
    assert!(
        routed
            .segments
            .iter()
            .all(|s| !s.net.starts_with("unconnected-"))
    );
    assert!(
        routed
            .vias
            .iter()
            .all(|v| !v.net.starts_with("unconnected-"))
    );
    // One pad on its own with every track and via: the full check.
    for pad in lone {
        let alone = route::Board {
            outline: board.outline,
            parts: vec![route::Part {
                reference: "lone".into(),
                pads: vec![pad.clone()],
            }],
        };
        route::check(&alone, &routed).unwrap();
    }
}

/// Every routed net's copper is one piece, by the router's own
/// connectivity: tracks end inside pads, on vias or on tracks, and the
/// ground vias are joined by the B.Cu zone, whose net is ground.
#[test]
fn every_nets_copper_is_connected() {
    let d = Dir::new("connected");
    let (board, routed) = read(&emitted(&d));
    route::connectivity(&board, &routed).unwrap();
    assert_eq!(routed.zones, vec![("GND".to_owned(), Layer::B)]);
    // A track taken away leaves a net in pieces, and the check says so.
    let mut cut = routed.clone();
    let i = cut
        .segments
        .iter()
        .position(|s| s.net == "/SPI_SCK")
        .unwrap();
    cut.segments.remove(i);
    assert!(route::connectivity(&board, &cut).is_err());
}

/// Every pad is left straight out: no F.Cu track runs inside a footprint's
/// pad field (the box round its pads) except over a pad of its own net. At
/// the LQFP that is the only way out between pads 0.2 mm apart.
#[test]
fn every_pad_is_left_straight_out_of_its_field() {
    let d = Dir::new("escape");
    let (board, routed) = read(&emitted(&d));
    let step = route::PITCH_NM;
    let mut inside = 0;
    for part in &board.parts {
        let Some(field) =
            part.pads
                .iter()
                .map(|p| p.rect)
                .reduce(|a, b| aethelgard_eda::pcb::Rect {
                    x0: a.x0.min(b.x0),
                    y0: a.y0.min(b.y0),
                    x1: a.x1.max(b.x1),
                    y1: a.y1.max(b.y1),
                })
        else {
            continue;
        };
        for s in routed.segments.iter().filter(|s| s.layer == Layer::F) {
            let (dx, dy) = (
                (s.end.0 - s.start.0).signum(),
                (s.end.1 - s.start.1).signum(),
            );
            let mut at = s.start;
            loop {
                let here = aethelgard_eda::pcb::Rect {
                    x0: at.0,
                    y0: at.1,
                    x1: at.0,
                    y1: at.1,
                };
                if field.contains(here) {
                    inside += 1;
                    assert!(
                        part.pads
                            .iter()
                            .any(|p| p.rect.contains(here)
                                && p.net.as_deref() == Some(s.net.as_str())),
                        "{} crosses {}'s pad field at {at:?}",
                        s.net,
                        part.reference
                    );
                }
                if at == s.end {
                    break;
                }
                at = (at.0 + dx * step, at.1 + dy * step);
            }
        }
    }
    assert!(inside > 0, "no track reaches a pad");
}

/// Adversarial pass A1 (fixed): the router's own judges refuse copper KiCad
/// rejects but they once passed. Measured at d6c3cbd, each passed `check`
/// and `connectivity`:
/// - a closed F.Cu loop of `/I2C_SDA` touching nothing (KiCad:
///   `unconnected_items`);
/// - a stub on an `unconnected-` pad (KiCad: `track_dangling`);
/// - a track on a net no pad carries (KiCad: `track_dangling`).
///
/// They also refuse what KiCad does not see (A2): every `VDD_3V3` segment
/// and via relabelled `GND`.
#[test]
fn the_routers_judges_refuse_orphan_and_relabelled_copper() {
    let d = Dir::new("orphan");
    let board = emitted(&d);
    let seg = |a: &str, b: &str, net: &str, i: usize| {
        format!(
            "\t(segment (start {a}) (end {b}) (width 0.2) (layer \"F.Cu\") (net \"{net}\") (uuid \"00000000-0000-4000-8000-00000000000{i}\"))\n"
        )
    };
    let ring = [
        seg("74 78", "76 78", "/I2C_SDA", 1),
        seg("76 78", "76 79", "/I2C_SDA", 2),
        seg("76 79", "74 79", "/I2C_SDA", 3),
        seg("74 79", "74 78", "/I2C_SDA", 4),
    ]
    .concat();
    let stub = seg(
        "90.1125 85.15",
        "89.6 85.15",
        "unconnected-(U3-ALERT-Pad3)",
        5,
    );
    let stray = seg("74 79", "76 79", "/NOPE", 6);
    let at = board.rfind("\t(embedded_fonts no)").unwrap();
    let mut relabelled = String::new();
    let mut copper = false;
    for line in board.lines() {
        match line {
            "\t(segment" | "\t(via" => copper = true,
            "\t)" => copper = false,
            _ => {}
        }
        relabelled.push_str(if copper && line == "\t\t(net \"VDD_3V3\")" {
            "\t\t(net \"GND\")"
        } else {
            line
        });
        relabelled.push('\n');
    }
    let (b, r) = read(&board);
    route::check(&b, &r).unwrap();
    route::connectivity(&b, &r).unwrap();
    for (name, edited) in [
        ("loop", format!("{}{ring}{}", &board[..at], &board[at..])),
        ("stub", format!("{}{stub}{}", &board[..at], &board[at..])),
        ("stray", format!("{}{stray}{}", &board[..at], &board[at..])),
        ("relabelled", relabelled),
    ] {
        assert_ne!(edited, board, "{name}");
        let (b, r) = read(&edited);
        let judged = route::check(&b, &r).and_then(|_| route::connectivity(&b, &r));
        assert!(judged.is_err(), "{name}: accepted");
    }
}
