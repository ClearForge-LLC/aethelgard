//! `AET-WO-4220b` §1.1: our own symbols and footprints for the ICM-42688-P
//! and the BMP581, held to citation tables written from the makers' pages,
//! apart from the files (a table derived from a file would only check the
//! file against itself, `RUL-20`). A pad off its cited dimension, a swapped
//! pin or a missing keepout fails here (§5.1).
//!
//! Coordinates are KiCad's: the top view, x to the right, y down, in nm.
//! The makers draw the BMP581's pads and the ICM's lands in a bottom view;
//! each is mirrored here as `AET-WO-4220b.FEEDBACK.md` R6 and R7 read it.

use aethelgard_eda::library;
use aethelgard_eda::sexpr::{self, Sx};

/// One land: its number, centre and size, nm.
type Land = (&'static str, (i64, i64), (i64, i64));

/// TDK DS-000347 v1.9 §10.2 pp. 54-55 (top view: pin 1 at the left column's
/// top; centres `2X 1.1625` and `2X 0.9125`, pitch e 0.5, D1 1.5, E1 1.0)
/// and AN-000393 v2.4 Fig. 1 (land = lead L 0.475 by W 0.25, nominal).
const ICM_LANDS: [Land; 14] = [
    ("1", (-1_162_500, -750_000), (475_000, 250_000)),
    ("2", (-1_162_500, -250_000), (475_000, 250_000)),
    ("3", (-1_162_500, 250_000), (475_000, 250_000)),
    ("4", (-1_162_500, 750_000), (475_000, 250_000)),
    ("5", (-500_000, 912_500), (250_000, 475_000)),
    ("6", (0, 912_500), (250_000, 475_000)),
    ("7", (500_000, 912_500), (250_000, 475_000)),
    ("8", (1_162_500, 750_000), (475_000, 250_000)),
    ("9", (1_162_500, 250_000), (475_000, 250_000)),
    ("10", (1_162_500, -250_000), (475_000, 250_000)),
    ("11", (1_162_500, -750_000), (475_000, 250_000)),
    ("12", (500_000, -912_500), (250_000, 475_000)),
    ("13", (0, -912_500), (250_000, 475_000)),
    ("14", (-500_000, -912_500), (250_000, 475_000)),
];

/// Bosch BST-BMP581-DS004-13 v1.13 §8.1.2 Fig. 30 p. 68, mirrored to the
/// top view (pin 1 at the left column's top), centres from the boxed 1.525,
/// 1 and 0.5; lands §8.2 p. 69: package pad 0.25 by 0.275 plus 25 µm on
/// each side, the 0.325 side radial (Fig. 32's 0.075 from the body edge).
const BMP_LANDS: [Land; 10] = [
    ("1", (-762_500, -250_000), (325_000, 300_000)),
    ("2", (-762_500, 250_000), (325_000, 300_000)),
    ("3", (-500_000, 762_500), (300_000, 325_000)),
    ("4", (0, 762_500), (300_000, 325_000)),
    ("5", (500_000, 762_500), (300_000, 325_000)),
    ("6", (762_500, 250_000), (325_000, 300_000)),
    ("7", (762_500, -250_000), (325_000, 300_000)),
    ("8", (500_000, -762_500), (300_000, 325_000)),
    ("9", (0, -762_500), (300_000, 325_000)),
    ("10", (-500_000, -762_500), (300_000, 325_000)),
];

/// A keepout: its layer, its rectangle `(x0, y0, x1, y1)` in nm, and whether
/// it forbids tracks, vias and copper pour.
type Keep = (&'static str, (i64, i64, i64, i64), [bool; 3]);

/// AN-000393 §2.1: "Avoiding vias, traces, and copper pour under IMU
/// device". `F.Cu`: inside the ring of lands, bounded by their inner edges
/// (1.1625 − 0.2375 and 0.9125 − 0.2375). `B.Cu`: the body (3.0 by 2.5,
/// with `aaa` 0.1 of profile tolerance) at its maximum.
const ICM_KEEPOUTS: [Keep; 2] = [
    (
        "F.Cu",
        (-925_000, -675_000, 925_000, 675_000),
        [true, true, true],
    ),
    (
        "B.Cu",
        (-1_550_000, -1_300_000, 1_550_000, 1_300_000),
        [true, true, false],
    ),
];

/// §8.2: "We do not recommend vias or traces under the BMP581." `F.Cu`:
/// inside the ring (0.7625 − 0.1625); `B.Cu`: the body at its Table 3
/// maximum, 2.10 mm square.
const BMP_KEEPOUTS: [Keep; 2] = [
    (
        "F.Cu",
        (-600_000, -600_000, 600_000, 600_000),
        [true, true, true],
    ),
    (
        "B.Cu",
        (-1_050_000, -1_050_000, 1_050_000, 1_050_000),
        [true, true, false],
    ),
];

/// One pin: number, name and KiCad electrical type.
type Pin = (&'static str, &'static str, &'static str);

/// DS-000347 Table 10 p. 20. RESV pins 2, 3, 10 and 11 ("No Connect or
/// Connect to GND") are `no_connect`; pin 7 ("Connect to GND") is passive;
/// INT2/FSYNC/CLKIN is an input, tied to GND ("Connect to GND if FSYNC not
/// used"), its INT2 output never enabled (no interrupt is armed).
const ICM_PINS: [Pin; 14] = [
    ("1", "AP_SDO/AP_AD0", "bidirectional"),
    ("2", "RESV", "no_connect"),
    ("3", "RESV", "no_connect"),
    ("4", "INT1/INT", "output"),
    ("5", "VDDIO", "power_in"),
    ("6", "GND", "power_in"),
    ("7", "RESV", "passive"),
    ("8", "VDD", "power_in"),
    ("9", "INT2/FSYNC/CLKIN", "input"),
    ("10", "RESV", "no_connect"),
    ("11", "RESV", "no_connect"),
    ("12", "AP_CS", "input"),
    ("13", "AP_SCL/AP_SCLK", "input"),
    ("14", "AP_SDA/AP_SDIO/AP_SDI", "bidirectional"),
];

/// BST-BMP581-DS004-13 Table 28 p. 46. INT is passive: it is tied to GND,
/// "allowed, as long as the IRQ pin is not activated" (note (a)).
const BMP_PINS: [Pin; 10] = [
    ("1", "VDDIO", "power_in"),
    ("2", "SCK", "input"),
    ("3", "VSS", "power_in"),
    ("4", "SDI", "bidirectional"),
    ("5", "SDO", "bidirectional"),
    ("6", "CSB", "input"),
    ("7", "INT", "passive"),
    ("8", "VSS", "power_in"),
    ("9", "VSS", "power_in"),
    ("10", "VDD", "power_in"),
];

const ICM_FOOTPRINT: &str = "Aethelgard_Sensor:TDK_LGA-14_3x2.5mm_P0.5mm";
const BMP_FOOTPRINT: &str = "Aethelgard_Sensor:Bosch_LGA-10_2x2mm_P0.5mm";

fn nm(item: &Sx, i: usize) -> i64 {
    sexpr::nm(item.arg(i).unwrap()).unwrap()
}

/// Each pad of `fp`: number, centre, size (nm), shape, angle and layers.
fn lands(fp: &Sx) -> Vec<(String, (i64, i64), (i64, i64))> {
    fp.children("pad")
        .map(|p| {
            assert_eq!(p.arg(2), Some("smd"), "{p:?}");
            assert_eq!(p.arg(3), Some("rect"), "{p:?}");
            let at = p.child("at").unwrap();
            assert_eq!(at.arg(3), None, "a pad at an angle: {p:?}");
            let size = p.child("size").unwrap();
            let layers: Vec<&str> = p
                .child("layers")
                .unwrap()
                .items()
                .iter()
                .skip(1)
                .filter_map(Sx::text)
                .collect();
            assert_eq!(layers, ["F.Cu", "F.Mask", "F.Paste"], "{p:?}");
            (
                p.arg(1).unwrap().to_owned(),
                (nm(at, 1), nm(at, 2)),
                (nm(size, 1), nm(size, 2)),
            )
        })
        .collect()
}

fn keepouts(fp: &Sx) -> Vec<(String, (i64, i64, i64, i64), [bool; 3])> {
    fp.children("zone")
        .map(|z| {
            let k = z.child("keepout").unwrap();
            let flag = |name: &str| match k.child(name).and_then(|f| f.arg(1)) {
                Some("not_allowed") => true,
                Some("allowed") => false,
                other => panic!("{name}: {other:?}"),
            };
            assert!(!flag("pads") && !flag("footprints"), "{z:?}");
            let pts: Vec<(i64, i64)> = z
                .child("polygon")
                .unwrap()
                .child("pts")
                .unwrap()
                .children("xy")
                .map(|xy| (nm(xy, 1), nm(xy, 2)))
                .collect();
            let xs = pts.iter().map(|p| p.0);
            let ys = pts.iter().map(|p| p.1);
            let rect = (
                xs.clone().min().unwrap(),
                ys.clone().min().unwrap(),
                xs.max().unwrap(),
                ys.max().unwrap(),
            );
            assert_eq!(pts.len(), 4, "{z:?}");
            assert!(
                pts.iter()
                    .all(|&(x, y)| (x == rect.0 || x == rect.2) && (y == rect.1 || y == rect.3)),
                "{z:?}"
            );
            (
                z.child("layer").unwrap().arg(1).unwrap().to_owned(),
                rect,
                [flag("tracks"), flag("vias"), flag("copperpour")],
            )
        })
        .collect()
}

fn want_lands(table: &[Land]) -> Vec<(String, (i64, i64), (i64, i64))> {
    table
        .iter()
        .map(|&(n, at, size)| (n.to_owned(), at, size))
        .collect()
}

fn want_keepouts(table: &[Keep]) -> Vec<(String, (i64, i64, i64, i64), [bool; 3])> {
    table
        .iter()
        .map(|&(l, r, f)| (l.to_owned(), r, f))
        .collect()
}

#[test]
fn the_icm_footprint_is_an_000393_s_land_pattern_on_ds_000347_s_pads() {
    let fp = library::footprint(ICM_FOOTPRINT).unwrap();
    assert_eq!(lands(&fp), want_lands(&ICM_LANDS));
    assert_eq!(keepouts(&fp), want_keepouts(&ICM_KEEPOUTS));
}

#[test]
fn the_bmp581_footprint_is_its_landing_pattern_with_its_keepout() {
    let fp = library::footprint(BMP_FOOTPRINT).unwrap();
    assert_eq!(lands(&fp), want_lands(&BMP_LANDS));
    assert_eq!(keepouts(&fp), want_keepouts(&BMP_KEEPOUTS));
}

/// Bosch §8.2: "at least 200 µm distance between the pads": every pair of
/// the BMP581's lands, and of the ICM's, is at least 200 µm apart (the
/// table's own arithmetic, so a land grown off its cited size fails here
/// as well as above).
#[test]
fn no_two_lands_are_closer_than_200_um() {
    for table in [&ICM_LANDS[..], &BMP_LANDS[..]] {
        for (i, a) in table.iter().enumerate() {
            for b in &table[i + 1..] {
                let gap = |c: i64, d: i64, s: i64, t: i64| (c - d).abs() - (s + t) / 2;
                let gx = gap(a.1.0, b.1.0, a.2.0, b.2.0);
                let gy = gap(a.1.1, b.1.1, a.2.1, b.2.1);
                assert!(gx.max(gy) >= 200_000, "{} {}: {gx} {gy}", a.0, b.0);
            }
        }
    }
}

/// Each keepout on `F.Cu` lies inside the ring of lands: it touches no land,
/// so a track can reach every land from outside, and nothing reaches under
/// the part from inside.
#[test]
fn each_front_keepout_is_inside_its_ring_of_lands() {
    for (lands, keeps) in [
        (&ICM_LANDS[..], &ICM_KEEPOUTS[..]),
        (&BMP_LANDS[..], &BMP_KEEPOUTS[..]),
    ] {
        let (_, (x0, y0, x1, y1), _) = keeps[0];
        for &(n, (x, y), (w, h)) in lands {
            let outside = x + w / 2 <= x0 || x - w / 2 >= x1 || y + h / 2 <= y0 || y - h / 2 >= y1;
            assert!(outside, "land {n} overlaps the keepout");
        }
    }
}

fn pins(lib_id: &str) -> Vec<(String, String, String)> {
    let sym = library::symbol(lib_id).unwrap();
    let mut out: Vec<(String, String, String)> = sym
        .children("symbol")
        .flat_map(|u| u.children("pin"))
        .map(|p| {
            (
                p.child("number").unwrap().arg(1).unwrap().to_owned(),
                p.child("name").unwrap().arg(1).unwrap().to_owned(),
                p.arg(1).unwrap().to_owned(),
            )
        })
        .collect();
    out.sort_by_key(|p| p.0.parse::<u32>().unwrap());
    out
}

fn want_pins(table: &[Pin]) -> Vec<(String, String, String)> {
    table
        .iter()
        .map(|&(n, name, kind)| (n.to_owned(), name.to_owned(), kind.to_owned()))
        .collect()
}

#[test]
fn each_symbol_s_pins_are_its_datasheet_s_table() {
    assert_eq!(pins("Aethelgard_Sensor:ICM-42688-P"), want_pins(&ICM_PINS));
    assert_eq!(pins("Aethelgard_Sensor:BMP581"), want_pins(&BMP_PINS));
}

/// Each symbol names its own footprint, so the board draws the lands above.
#[test]
fn each_symbol_names_its_own_footprint() {
    for (sym, fp) in [
        ("Aethelgard_Sensor:ICM-42688-P", ICM_FOOTPRINT),
        ("Aethelgard_Sensor:BMP581", BMP_FOOTPRINT),
    ] {
        assert_eq!(
            library::symbol(sym).unwrap().property("Footprint"),
            Some(fp)
        );
    }
}
