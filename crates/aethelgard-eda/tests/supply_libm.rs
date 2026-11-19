//! The supply gate's figures through each platform-libm call it makes, pinned
//! exactly, on the host and in the image alike (`AET-WO-1910` D5 (a), ruled on
//! PR #233, comment 6999515740).
//!
//! `supply_tier1` runs in `tools` alone, so these goldens are what `gates`
//! keeps of its figures on the host's glibc. Each of the five call sites
//! decides something discrete, and the goldens pin what it decided:
//! - `supply.rs`'s `witnessed`: `hypot` twice, then `cos` and `sin` for the
//!   probe directions. Each decides whether a ring cell is copper.
//! - its knockout diagonal: `hypot`, then `ceil`. It decides how far a pad's
//!   knockout reaches.
//! - `flow.rs`'s `Region::new`: `atan2`. It decides the order of a junction
//!   region's faces.
//!
//! Each decision a call makes on this board is pinned. Removing what one call
//! contributes moves the digest, and so does a gross change to it (the
//! FEEDBACK's libm coverage map lists one mutant per site, each red here).
//! A difference of one ulp moves nothing here, nor did it in `supply_tier1`
//! (the adversarial pass). So the goldens pin what each call decides on these
//! boards, and a libm that rounds one of them across a decision.

use aethelgard_eda::flow::{Cell, Region};
use aethelgard_eda::pcb::Rect;
use aethelgard_eda::route::{Board, Layer, Pad, Part, Routed, Segment, Via};
use aethelgard_eda::stack::{Stack, Thickness};
use aethelgard_eda::supply::{
    BETA_NM, CROP_NM, EDGE_NM, FILL_CLEARANCE_NM, Model, NetCopper, OPENING_NM, PITCH_NM, copper,
    rho_max,
};
use sha2::{Digest, Sha256};

fn pad(number: &str, net: &str, x0: i64, y0: i64, x1: i64, y1: i64) -> Pad {
    Pad {
        number: number.into(),
        net: Some(net.into()),
        rect: Rect { x0, y0, x1, y1 },
        corner_nm: 0,
        drill_nm: None,
        join: None,
    }
}

fn part(reference: &str, pads: Vec<Pad>) -> Part {
    Part {
        reference: reference.into(),
        rotation: 0,
        pads,
        courtyard: vec![],
        keepouts: vec![],
    }
}

/// `supply::tests`' 4-layer board: 20 x 6 mm, 0.25 mm tracks, In1.Cu a VDD
/// plane and In2.Cu a GND plane.
fn board4(parts: Vec<Part>) -> Board {
    // `stack::reference()` (test-only in the crate) with `supply::tests`' 4-layer changes.
    let stack = Stack {
        via_nm: aethelgard_eda::route::VIA_NM,
        drill_nm: aethelgard_eda::route::DRILL_NM,
        bounded: vec![],
        notes: vec![],
        copper: vec![
            "F.Cu".into(),
            "In1.Cu".into(),
            "In2.Cu".into(),
            "B.Cu".into(),
        ],
        planes: vec![
            ("In1.Cu".into(), "VDD".into()),
            ("In2.Cu".into(), "GND".into()),
        ],
        thickness: Thickness {
            nominal_nm: 1_600_000,
            min_nm: 1_250_000,
            max_nm: 1_851_000,
            source: "s".into(),
        },
        dielectric_max_nm: vec![None; 3],
        dielectrics: vec![
            ("F.Cu".into(), "In1.Cu".into(), 179_000, "s".into()),
            ("In1.Cu".into(), "In2.Cu".into(), 891_000, "s".into()),
            ("In2.Cu".into(), "B.Cu".into(), 179_000, "s".into()),
        ],
        copper_max_nm: 70_000,
        copper_source: "s".into(),
        permittivity_permille: 5400,
        permittivity_source: "s".into(),
        clearance_nm: 200_000,
        width_nm: 250_000,
        classes: vec![],
        pairs: vec![],
        joins: vec![],
        thermal: vec![],
        imus: vec![],
    };
    Board {
        outline: Rect {
            x0: 0,
            y0: 0,
            x1: 20_000_000,
            y1: 6_000_000,
        },
        parts,
        stack,
    }
}

/// `supply::tests`' model: JLCPCB's position tolerance, hole size and etch at 0.
fn model() -> Model {
    Model {
        h_nm: PITCH_NM,
        rho: rho_max(125),
        outer_nm: (30_000, 70_000),
        plane_nm: (15_000, 70_000),
        plating_nm: 18_000,
        beta_nm: BETA_NM,
        hole_offset_nm: 75_000,
        hole_plus_nm: 0,
        hole_minus_nm: 0,
        track_etch_permille: 0,
        edge_nm: EDGE_NM,
        crop_nm: CROP_NM,
        fill_clearance_nm: FILL_CLEARANCE_NM,
        opening_nm: OPENING_NM,
    }
}

fn sha(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Each layer's cells, each junction region's faces in their order, and each
/// via's joins, as text; and the counts a reviewer reads beside the digest.
fn figures(nc: &NetCopper) -> (String, Vec<(String, usize)>, Vec<usize>) {
    let mut text = String::new();
    let mut counts = Vec::new();
    for l in &nc.layers {
        let mut cells: Vec<Cell> = l.cells.clone();
        cells.sort();
        text.push_str(&format!("{} {} {:?}\n", l.name, l.plane, cells));
        counts.push((l.name.clone(), cells.len()));
    }
    let mut faces = Vec::new();
    for j in &nc.junctions {
        text.push_str(&format!("{:?}\n", j.region.faces));
        faces.push(j.region.faces.len());
    }
    for u in &nc.vias {
        text.push_str(&format!("{:?} {} {:?}\n", u.via.at, u.junction, u.joins));
    }
    (text, counts, faces)
}

/// `supply::tests`' 4-layer board: In1.Cu is VDD's plane, joined by one VDD
/// via, and cut by:
/// - two GND vias 1.25 mm apart, between whose raw knockouts only the
///   compass probes (`cos`, `sin`) witness some cells;
/// - two GND vias 0.68 mm from (6.0, 4.6) mm at 0° and 70°, where one cell
///   only the summed "away" direction (`hypot` at `:122`) witnesses;
/// - a GND through-hole pad, by its diagonal knockout (`hypot` at `:838`).
///
/// Measured in scratch with a mutant each, In1.Cu has 6,113 cells, against
/// 6,112 without the summed direction, 6,109 without the compass probes, and
/// 6,109 without each single knockout's direction (`hypot` at `:117`).
/// `copper` reaches all five calls on this board (measured with a marker
/// per call site; `AET-WO-1910` FEEDBACK, the libm coverage map).
#[test]
fn the_net_copper_through_every_libm_call_is_the_golden() {
    let mut th = pad("1", "GND", 2_750_000, 3_750_000, 3_750_000, 4_750_000);
    th.drill_nm = Some(600_000);
    th.corner_nm = 250_000;
    let b = board4(vec![
        part(
            "J1",
            vec![pad("1", "VDD", 1_000_000, 2_000_000, 2_000_000, 3_000_000)],
        ),
        part("J2", vec![th]),
    ]);
    let c = (6_000_000, 3_000_000);
    let via = |net: &str, at: (i64, i64)| Via {
        net: net.into(),
        at,
    };
    let seg = |a: (i64, i64), z: (i64, i64)| Segment {
        net: "VDD".into(),
        layer: Layer::F,
        start: a,
        end: z,
    };
    let routed = Routed {
        vias: vec![
            via("GND", (c.0 - 625_000, c.1)),
            via("GND", (c.0 + 625_000, c.1)),
            // 680 µm at 0° and at 70° from (6.0, 4.6) mm
            via("GND", (6_680_000, 4_600_000)),
            via("GND", (6_232_573, 5_238_990)),
            via("VDD", (4_000_000, 3_000_000)),
        ],
        segments: vec![
            seg((1_500_000, 2_500_000), (4_000_000, 2_500_000)),
            seg((4_000_000, 2_500_000), (4_000_000, 3_000_000)),
        ],
        ..Routed::default()
    };
    let src = &b.parts[0].pads[0];
    let nc = copper(&b, &routed, "VDD", &[src], &model()).expect("the net's copper is built");
    let (text, counts, faces) = figures(&nc);
    assert_eq!(
        (counts, faces, sha(&text)),
        (
            vec![
                ("F.Cu".to_owned(), 393),
                ("In1.Cu".to_owned(), 6113),
                ("In2.Cu".to_owned(), 0),
                ("B.Cu".to_owned(), 0),
            ],
            vec![40],
            "89bfeec51ddf9ef2".to_owned()
        ),
        "the net's copper, cell by cell and face by face, is the golden: a libm call moved one"
    );
}

/// `Region::new` directly, the `atan2` that orders a junction region's faces,
/// at the radii of the reference's via pads (0.3 and 0.6 mm drills' bands).
#[test]
fn a_junction_regions_face_order_is_the_golden() {
    let mut text = String::new();
    let mut counts = Vec::new();
    for r in [150_000, 300_000, 375_000, 457_500] {
        let region = Region::new(PITCH_NM, r);
        text.push_str(&format!("{r} {:?}\n", region.faces));
        counts.push(region.faces.len());
    }
    assert_eq!(
        (counts, sha(&text)),
        (vec![24, 40, 48, 64], "81e6eb657599cc90".to_owned()),
        "each region's faces, in their counter-clockwise order, are the golden: atan2 moved one"
    );
}
