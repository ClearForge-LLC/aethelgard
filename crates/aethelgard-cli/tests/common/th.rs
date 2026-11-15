//! `AET-WO-1309` fixtures: through-hole pads and thermal-via arrays, as
//! `eda`-level boards (FEEDBACK D1: no part row a specification can declare
//! carries either yet). Each footprint is authored here from cited
//! geometry, placed by hand, and routed by `pcb::write_placed` against a
//! declared board whose joins and arrays the test sets.
//!
//! - **The XT30 shape.** AMASS's board-type table
//!   (`~/.cache/aet-ds/xt30-amass-board-type-table-screenshot.jpg`) gives the
//!   XT30PW's 2.0/3.0 mm weld legs, 20 A and 500 V DC, and no land pattern.
//!   The pads, drills and courtyard are KiCad 10.0.7's
//!   `Connector_AMASS:AMASS_XT30PW-M_1x02_P2.50mm_Horizontal` (read): pads
//!   1 (roundrect 0.22) and 2, 3.5 mm on 1.9 mm drills, 5.0 mm apart; two
//!   unnamed mechanical pads, 1.4 mm on 1.0 mm drills, at (−8, −10) and
//!   (3, −10); courtyard (−9.65, −13.6) to (4.65, 2.25).
//! - **The HSOP-8 shape.** KiCad 10.0.7's
//!   `Package_SO:Texas_HSOP-8-1EP_3.9x4.9mm_P1.27mm` (read): eight 1.55 x
//!   0.6 mm pads at x ±2.7 mm, 1.27 mm pitch; exposed pad 9, 2.95 x 4.9 mm;
//!   courtyard ±3.75 x ±2.75 mm.
//! - **The 0402 load** is the vendored `R_0402_1005Metric`'s land pattern.

use std::fs;
use std::path::Path;

use aethelgard_eda::sexpr::{self, Sx};
use aethelgard_eda::stack::{Join, Stack, ThermalPad};

/// The declared board: the reference's at 2 layers, the 4-layer fixture's
/// (In1.Cu `GND`, In2.Cu `VDD_3V3`) at 4.
pub fn stack(four: bool) -> Stack {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let path = if four {
        root.join("tests/fixtures/four-layer.json")
    } else {
        root.join("../../examples/sentinel-01-minimal.json")
    };
    let spec: aethelgard_ir::Specification =
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
    Stack::of(&spec)
}

/// LMR33630's thermal-via rule as the fixture declares it: TI SNVSAZ9
/// §10.1.1 (p34) for the count (4 x 3) and the drill (10 mil); the pitch
/// and ring are design choices TI does not state (FEEDBACK F4). Four rows
/// along the pad's 4.9 mm, three columns across its 2.95 mm.
pub fn lmr33630_rule(layers: &[&str]) -> aethelgard_ir::thermal::ThermalVias {
    let ti = "TI SNVSAZ9 (LMR33630) §10.1.1, p34: minimum 4 x 3 array of 10-mil thermal vias";
    aethelgard_ir::thermal::ThermalVias {
        columns: 3,
        rows: 4,
        pitch_um: 1000,
        drill_um: 254,
        ring_um: 150,
        layers: layers.iter().map(|l| (*l).to_owned()).collect(),
        sources: aethelgard_ir::thermal::ThermalSources {
            count: ti.into(),
            pitch: "design choice, not in the datasheet: 1.0 mm spreads 4 x 3 evenly over the pad"
                .into(),
            drill: ti.into(),
            ring: "design choice, not in the datasheet: the board's own via ring".into(),
            layers: "TI SNVSAZ9 §10.1.1, p34: the vias go into the ground planes".into(),
        },
    }
}

/// One pad: `extra` holds its drill, layers and any other tokens.
fn pad(
    number: &str,
    kind: &str,
    at: (f64, f64),
    size: (f64, f64),
    net: Option<&str>,
    extra: &str,
) -> String {
    let net = net.map_or(String::new(), |n| format!(" (net \"{n}\")"));
    format!(
        "(pad \"{number}\" {kind} (at {} {}) (size {} {}) {extra}{net})",
        at.0, at.1, size.0, size.1
    )
}

fn footprint(
    lib: &str,
    reference: &str,
    value: &str,
    at: (f64, f64),
    yard: (f64, f64, f64, f64),
    pads: &[String],
) -> Sx {
    let text = format!(
        "(footprint \"{lib}\" (layer \"F.Cu\") (at {} {}) (property \"Reference\" \"{reference}\" (at 0 0) (layer \"F.Fab\")) (property \"Value\" \"{value}\" (at 0 0) (layer \"F.Fab\")) (fp_rect (start {} {}) (end {} {}) (stroke (width 0.05) (type default)) (fill no) (layer \"F.CrtYd\")) {})",
        at.0,
        at.1,
        yard.0,
        yard.1,
        yard.2,
        yard.3,
        pads.join(" ")
    );
    sexpr::parse(&text).unwrap()
}

/// The XT30 shape at `at`: pin 1 on `plus`, pin 2 on `minus`.
pub fn xt30(reference: &str, at: (f64, f64), plus: &str, minus: &str) -> Sx {
    let th = |number: &str,
              shape: &str,
              at: (f64, f64),
              size: f64,
              drill: f64,
              net: Option<&str>,
              extra: &str| {
        let tokens = format!(
            "(drill {drill}) (layers \"*.Cu\" \"*.Mask\"){extra} (remove_unused_layers no)"
        );
        pad(
            number,
            &format!("thru_hole {shape}"),
            at,
            (size, size),
            net,
            &tokens,
        )
    };
    footprint(
        "XT30PW-M_shape",
        reference,
        "XT30PW-M",
        at,
        (-9.65, -13.6, 4.65, 2.25),
        &[
            th("", "circle", (-8.0, -10.0), 1.4, 1.0, None, ""),
            th("", "circle", (3.0, -10.0), 1.4, 1.0, None, ""),
            th(
                "1",
                "roundrect",
                (0.0, 0.0),
                3.5,
                1.9,
                Some(plus),
                " (roundrect_rratio 0.22)",
            ),
            th("2", "circle", (-5.0, 0.0), 3.5, 1.9, Some(minus), ""),
        ],
    )
}

/// The HSOP-8 shape at `at`, pad `n`'s net `nets[n - 1]` (pad 9 the exposed
/// pad); `own_vias` adds KiCad `_ThermalVias`-style through-hole pads
/// numbered 9 (2 x 4, 0.3 mm drill, 0.6 mm, 1.3 mm pitch: that footprint's).
pub fn hsop8(reference: &str, at: (f64, f64), nets: [Option<&str>; 9], own_vias: bool) -> Sx {
    let ys = [-1.905, -0.635, 0.635, 1.905];
    let smd = "(layers \"F.Cu\" \"F.Mask\" \"F.Paste\")";
    let mut pads: Vec<String> = (0..8)
        .map(|i| {
            let (x, y) = if i < 4 {
                (-2.7, ys[i])
            } else {
                (2.7, ys[7 - i])
            };
            pad(
                &(i + 1).to_string(),
                "smd rect",
                (x, y),
                (1.55, 0.6),
                nets[i],
                smd,
            )
        })
        .collect();
    pads.push(pad(
        "9",
        "smd rect",
        (0.0, 0.0),
        (2.95, 4.9),
        nets[8],
        "(layers \"F.Cu\")",
    ));
    if own_vias {
        for x in [-0.65, 0.65] {
            for y in [-1.95, -0.65, 0.65, 1.95] {
                let tokens = "(drill 0.3) (layers \"*.Cu\") (remove_unused_layers no)";
                pads.push(pad(
                    "9",
                    "thru_hole circle",
                    (x, y),
                    (0.6, 0.6),
                    nets[8],
                    tokens,
                ));
            }
        }
    }
    footprint(
        "HSOP-8_shape",
        reference,
        "LMR33630ADDA",
        at,
        (-3.75, -2.75, 3.75, 2.75),
        &pads,
    )
}

/// An 0402 load at `at`, pad 1 on `a`, pad 2 on `b`.
pub fn r0402(reference: &str, at: (f64, f64), a: &str, b: &str) -> Sx {
    let p = |n: &str, x: f64, net: &str| {
        let tokens = "(layers \"F.Cu\" \"F.Mask\" \"F.Paste\") (roundrect_rratio 0.25)";
        pad(
            n,
            "smd roundrect",
            (x, 0.0),
            (0.54, 0.64),
            Some(net),
            tokens,
        )
    };
    footprint(
        "R_0402_shape",
        reference,
        "10k",
        at,
        (-0.93, -0.47, 0.93, 0.47),
        &[p("1", -0.51, a), p("2", 0.51, b)],
    )
}

pub fn outline(x0: f64, y0: f64, x1: f64, y1: f64) -> aethelgard_eda::pcb::Rect {
    let nm = |v: f64| (v * 1e6).round() as i64;
    aethelgard_eda::pcb::Rect {
        x0: nm(x0),
        y0: nm(y0),
        x1: nm(x1),
        y1: nm(y1),
    }
}

/// The XT30 board: J1 on `VBAT`/`GND` joined as `join`, an 0402 load
/// between them, and at 4 layers two 0402s from `VDD_3V3` to `GND`, so its
/// plane has a routed net to draw from (a net of one pad is not routed).
pub fn xt30_board(four: bool, join: Join) -> (Stack, Vec<Sx>, aethelgard_eda::pcb::Rect) {
    let mut stack = stack(four);
    stack.joins = vec![("J1".to_owned(), join)];
    let mut fps = vec![
        xt30("J1", (110.0, 116.0), "VBAT", "GND"),
        r0402("R1", (122.0, 116.0), "VBAT", "GND"),
    ];
    if four {
        fps.push(r0402("R2", (122.0, 108.0), "VDD_3V3", "GND"));
        fps.push(r0402("R3", (122.0, 104.0), "VDD_3V3", "GND"));
    }
    (stack, fps, outline(98.0, 100.0, 130.0, 121.0))
}

/// The thermal board: U5's exposed pad on `GND` with the declared array
/// (`rule`, or none), pin 2 (VIN) to an 0402 load, pin 1 on `GND`.
pub fn thermal_board(
    four: bool,
    rule: Option<aethelgard_ir::thermal::ThermalVias>,
    own_vias: bool,
) -> (Stack, Vec<Sx>, aethelgard_eda::pcb::Rect) {
    let mut stack = stack(four);
    if let Some(rule) = rule {
        stack.thermal = vec![ThermalPad::of("U5", "9", &rule)];
    }
    let mut nets = [None; 9];
    nets[0] = Some("GND");
    nets[1] = Some("VIN");
    nets[8] = Some("GND");
    let mut fps = vec![
        hsop8("U5", (110.0, 112.0), nets, own_vias),
        r0402("R1", (120.0, 112.0), "VIN", "GND"),
    ];
    if four {
        fps.push(r0402("R2", (120.0, 106.0), "VDD_3V3", "GND"));
        fps.push(r0402("R3", (120.0, 103.0), "VDD_3V3", "GND"));
    }
    (stack, fps, outline(100.0, 100.0, 126.0, 120.0))
}

/// The board `pcb::write_placed` writes for a fixture, routed.
pub fn write(
    (stack, fps, outline): (Stack, Vec<Sx>, aethelgard_eda::pcb::Rect),
) -> Result<(Stack, String), String> {
    aethelgard_eda::pcb::write_placed("th-fixture", fps, outline, &stack)
        .map(|text| (stack, text))
        .map_err(|e| e.to_string())
}

/// The gate's own judges on the written text, as `check drc --spec` runs
/// them: the strict reader, `route::read`, `connectivity` and `check`.
pub fn judge(stack: &Stack, text: &str) -> Result<(), String> {
    let tree = aethelgard_eda::strict::board(text).map_err(|e| e.to_string())?;
    aethelgard_eda::route::declared(&tree, stack).map_err(|d| d.message)?;
    let (b, r) = aethelgard_eda::route::read(&tree, stack)?;
    aethelgard_eda::route::connectivity(&b, &r)?;
    aethelgard_eda::route::check(&b, &r)
}
